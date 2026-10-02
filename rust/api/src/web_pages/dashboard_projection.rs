use super::time::dashboard_time;
use super::{field, numeric};
use chrono::{DateTime, Utc};
use medtracker_web::dashboard::{
    calculate_prn, is_same_local_day, DashboardHistory, DashboardPerson, DashboardStock,
    DashboardTask, DashboardTaskRow, PrnInput, TaskState,
};
use serde_json::Value;
use std::collections::HashMap;

fn dashboard_dose(source: &Value) -> String {
    let amount = field(source, "dose_amount");
    let unit = field(source, "dose_unit");
    format!("{amount} {unit}").trim().to_owned()
}

fn schedule_in_local_range(source: &Value, today: chrono::NaiveDate) -> bool {
    let start = chrono::NaiveDate::parse_from_str(field(source, "start_date"), "%Y-%m-%d").ok();
    let end = chrono::NaiveDate::parse_from_str(field(source, "end_date"), "%Y-%m-%d").ok();
    start
        .zip(end)
        .is_some_and(|(start, end)| (start..=end).contains(&today))
}

fn taper_step_on(source: &Value, today: chrono::NaiveDate) -> Option<&Value> {
    source
        .pointer("/schedule_config/taper_steps")
        .and_then(Value::as_array)?
        .iter()
        .find(|step| {
            let start =
                chrono::NaiveDate::parse_from_str(field(step, "start_date"), "%Y-%m-%d").ok();
            let end = chrono::NaiveDate::parse_from_str(field(step, "end_date"), "%Y-%m-%d").ok();
            start
                .zip(end)
                .is_some_and(|(start, end)| (start..=end).contains(&today))
        })
}

fn schedule_applies_on(source: &Value, today: chrono::NaiveDate) -> bool {
    schedule_in_local_range(source, today)
        && (field(source, "schedule_type") != "tapering" || taper_step_on(source, today).is_some())
}

fn source_active_on(kind: &str, source: &Value, today: chrono::NaiveDate) -> bool {
    if kind == "schedules" {
        source.get("paused").and_then(Value::as_bool) == Some(false)
            && schedule_applies_on(source, today)
    } else {
        source.get("active").and_then(Value::as_bool) == Some(true)
    }
}

pub(super) fn dashboard_sources(
    schedules: Vec<Value>,
    assignments: Vec<Value>,
    visible: &[i64],
    today: chrono::NaiveDate,
) -> Vec<(String, Value)> {
    schedules
        .into_iter()
        .map(|value| ("schedules".to_owned(), value))
        .chain(
            assignments
                .into_iter()
                .map(|value| ("person_medications".to_owned(), value)),
        )
        .filter(|(_, source)| numeric(source, "person_id").is_some_and(|id| visible.contains(&id)))
        .filter(|(kind, source)| {
            source_active_on(kind, source, today)
                || (source
                    .get("current_pause_period")
                    .is_some_and(|value| !value.is_null())
                    && (kind != "schedules" || schedule_applies_on(source, today)))
        })
        .collect()
}

fn source_takes(takes: &[Value], kind: &str, id: i64) -> Vec<DateTime<Utc>> {
    let key = if kind == "schedules" {
        "schedule_id"
    } else {
        "person_medication_id"
    };
    takes
        .iter()
        .filter(|row| numeric(row, key) == Some(id))
        .filter_map(|row| dashboard_time(field(row, "taken_at")))
        .collect()
}

fn stock_matches(
    source: &Value,
    permitted_medications: &HashMap<i64, std::collections::HashSet<i64>>,
    medications: &HashMap<i64, &Value>,
    household_manager: bool,
) -> bool {
    let Some(source_medication) =
        numeric(source, "medication_id").and_then(|id| medications.get(&id).copied())
    else {
        return false;
    };
    let allowed_ids = numeric(source, "person_id").and_then(|id| permitted_medications.get(&id));
    medications.iter().any(|(id, row)| {
        (household_manager || allowed_ids.is_some_and(|ids| ids.contains(id)))
            && field(row, "name") == field(source_medication, "name")
            && field(row, "dose_amount") == field(source_medication, "dose_amount")
            && field(row, "dose_unit") == field(source_medication, "dose_unit")
            && row.get("out_of_stock").and_then(Value::as_bool) != Some(true)
    })
}

fn source_config_on(source: &Value, today: chrono::NaiveDate) -> Option<&Value> {
    let config = source.get("schedule_config")?;
    if field(source, "schedule_type") != "tapering" {
        return Some(config);
    }
    taper_step_on(source, today).or(Some(config))
}

fn source_limit(source: &Value, today: chrono::NaiveDate) -> Option<usize> {
    let count = |value: &Value| {
        value
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
    };
    source_config_on(source, today)
        .and_then(|config| {
            ["max_daily_doses", "max_doses", "max"]
                .iter()
                .find_map(|name| config.get(*name).and_then(count))
        })
        .or_else(|| source.get("max_daily_doses").and_then(count))
}

fn source_interval(source: &Value, today: chrono::NaiveDate) -> Option<f64> {
    let hours = |value: &Value| {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
    };
    source_config_on(source, today)
        .and_then(|config| {
            ["min_hours_between_doses", "min_hours", "minimum_hours"]
                .iter()
                .find_map(|name| config.get(*name).and_then(hours))
        })
        .or_else(|| source.get("min_hours_between_doses").and_then(hours))
}

pub(super) struct SourceTaskInput<'a> {
    pub(super) kind: &'a str,
    pub(super) id: i64,
    pub(super) medication_id: i64,
    pub(super) source: &'a Value,
    pub(super) rows: &'a [Value],
    pub(super) now: DateTime<Utc>,
    pub(super) timezone: chrono_tz::Tz,
    pub(super) takes: &'a [Value],
    pub(super) medications: &'a HashMap<i64, &'a Value>,
    pub(super) permitted_medications: &'a HashMap<i64, std::collections::HashSet<i64>>,
    pub(super) household_manager: bool,
}

pub(super) fn project_source_tasks(
    input: SourceTaskInput<'_>,
    person: &mut DashboardPerson,
    metric_tasks: &mut Vec<DashboardTask>,
) {
    let SourceTaskInput {
        kind,
        id,
        medication_id,
        source,
        rows,
        now,
        timezone,
        takes,
        medications,
        permitted_medications,
        household_manager,
    } = input;
    let today = now.with_timezone(&timezone).date_naive();
    let medication = medications.get(&medication_id).copied();
    let medication_name = medication
        .map(|row| field(row, "display_name"))
        .filter(|name| !name.is_empty())
        .or_else(|| medication.map(|row| field(row, "name")))
        .unwrap_or("Medication")
        .to_owned();
    let prn = field(source, "administration_kind") == "as_needed"
        || field(source, "schedule_type") == "prn"
        || field(source, "frequency").eq_ignore_ascii_case("as needed")
        || source
            .pointer("/schedule_config/as_needed")
            .and_then(Value::as_bool)
            == Some(true);
    if !prn
        && rows.is_empty()
        && source
            .get("current_pause_period")
            .is_some_and(|value| !value.is_null())
    {
        person.tasks.push(DashboardTaskRow {
            medication_name: medication_name.clone(),
            dose: dashboard_dose(source),
            time: "—".to_owned(),
            scheduled_at: None,
            state: TaskState::Paused,
            routine: true,
        });
    }
    if prn {
        let source_takes = source_takes(takes, kind, id);
        let projection = calculate_prn(PrnInput {
            now,
            timezone,
            paused: source
                .get("current_pause_period")
                .is_some_and(|value| !value.is_null()),
            active: source_active_on(kind, source, today),
            stock_available: stock_matches(
                source,
                permitted_medications,
                medications,
                household_manager,
            ),
            max_doses: source_limit(source, today),
            min_hours_between_doses: source_interval(source, today),
            dose_cycle: field(source, "dose_cycle"),
            takes: &source_takes,
        });
        let time = projection
            .next_available_at
            .map(|value| value.with_timezone(&timezone).format("%H:%M").to_string())
            .unwrap_or_else(|| "Anytime".to_owned());
        person.tasks.push(DashboardTaskRow {
            medication_name: medication_name.clone(),
            dose: dashboard_dose(source),
            time,
            scheduled_at: projection.next_available_at,
            state: projection.state,
            routine: false,
        });
        metric_tasks.push(DashboardTask {
            state: projection.state,
            scheduled_at: projection.next_available_at,
        });
    }
    for row in rows {
        if row.get("expected").and_then(Value::as_bool) == Some(false)
            && field(row, "outcome") == "open"
        {
            continue;
        }
        let scheduled_at = dashboard_time(field(row, "scheduled_at"));
        let state = match field(row, "outcome") {
            "taken" => TaskState::Taken,
            "not_taken" => TaskState::NotTaken,
            _ if source
                .get("current_pause_period")
                .is_some_and(|value| !value.is_null()) =>
            {
                TaskState::Paused
            }
            _ if medication.is_some_and(|row| {
                row.get("out_of_stock").and_then(Value::as_bool) == Some(true)
            }) =>
            {
                TaskState::OutOfStock
            }
            _ if prn => TaskState::Unknown,
            _ if row.get("due").and_then(Value::as_bool) == Some(true) => TaskState::Available,
            _ => TaskState::Upcoming,
        };
        let time = scheduled_at
            .map(|value| value.with_timezone(&timezone).format("%H:%M").to_string())
            .unwrap_or_else(|| "Anytime".to_owned());
        let task = DashboardTaskRow {
            medication_name: medication_name.clone(),
            dose: dashboard_dose(source),
            time,
            scheduled_at,
            state,
            routine: !prn,
        };
        if state == TaskState::NotTaken {
            person.outcomes.push(task);
        } else if state != TaskState::Taken && !prn {
            person.tasks.push(task);
            metric_tasks.push(DashboardTask {
                state,
                scheduled_at,
            });
        }
    }
}

pub(super) fn dashboard_history(
    takes: &[Value],
    selected: &[i64],
    selectable_people: &[(i64, String)],
    medications: &HashMap<i64, &Value>,
    now: DateTime<Utc>,
    timezone: chrono_tz::Tz,
) -> Vec<DashboardHistory> {
    takes
        .iter()
        .filter(|row| numeric(row, "person_id").is_some_and(|id| selected.contains(&id)))
        .filter_map(|row| {
            let timestamp = dashboard_time(field(row, "taken_at"))?;
            if !is_same_local_day(timestamp, now, timezone) {
                return None;
            }
            let name = selectable_people
                .iter()
                .find(|(id, _)| Some(*id) == numeric(row, "person_id"))?
                .1
                .clone();
            let medication = medications.get(&numeric(row, "medication_id")?)?;
            Some(DashboardHistory {
                person_name: name,
                medication_name: field(medication, "display_name").to_owned(),
                dose: dashboard_dose(row),
                time: timestamp
                    .with_timezone(&timezone)
                    .format("%H:%M")
                    .to_string(),
            })
        })
        .collect()
}

pub(super) fn dashboard_stock(
    mut stock_ids: Vec<i64>,
    medications: &HashMap<i64, &Value>,
) -> Vec<DashboardStock> {
    stock_ids.sort_unstable();
    stock_ids.dedup();
    stock_ids
        .into_iter()
        .filter_map(|id| {
            let row = medications.get(&id)?;
            Some(DashboardStock {
                name: field(row, "display_name").to_owned(),
                amount: field(row, "current_supply").to_owned(),
                unit: field(row, "dose_unit").to_owned(),
                low: row.get("low_stock").and_then(Value::as_bool) == Some(true),
                out: row.get("out_of_stock").and_then(Value::as_bool) == Some(true),
            })
        })
        .collect()
}
