use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::{dose_occurrences, doses, review_prompts},
    entities::{
        health_event, health_event_medication, location, medication, medication_take, person,
        person_medication, review_prompt, schedule, security_audit_event,
    },
    errors::OperationError,
};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike, Utc};
use chrono_tz::Tz;
use loco_rs::model::TenantQueryExt;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    QueryTrait, Set,
    prelude::Decimal,
    sea_query::{Expr, Func},
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};

mod types;
pub use types::*;

type CycleKey<'a> = (&'a str, i64, NaiveDate, NaiveDate);
type CycleCounts = (i64, i64, usize, usize, usize, usize);

struct InsightPeriod {
    start: NaiveDate,
    end: NaiveDate,
    today: NaiveDate,
}

pub async fn actor_adult(
    tenant: &TenantTransaction,
    today: NaiveDate,
) -> Result<bool, OperationError> {
    let id = tenant.membership().person_id;
    let Some(actor) = access::person_queries::household_people(tenant.scope())
        .filter(person::Column::Id.eq(id))
        .one(tenant.transaction())
        .await?
    else {
        return Ok(false);
    };
    let by_age = actor.date_of_birth.is_some_and(|born| {
        let mut age = today.year() - born.year();
        if (today.month(), today.day()) < (born.month(), born.day()) {
            age -= 1;
        }
        age >= 18
    });
    Ok(actor.person_type == 0 || by_age)
}

pub async fn accessible_people(
    tenant: &TenantTransaction,
    level: PersonAccess,
) -> Result<Vec<person::Model>, OperationError> {
    let mut people = access::people_with_access(tenant, level).await?;
    people.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
    Ok(people)
}

pub async fn record_download(
    tenant: &TenantTransaction,
    event_type: &str,
    metadata: Value,
) -> Result<(), OperationError> {
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        actor_account_id: Set(Some(tenant.scope().actor.account_id)),
        actor_membership_id: Set(Some(tenant.membership().id)),
        event_type: Set(event_type.to_owned()),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        metadata: Set(metadata),
        audit_context: Set(json!({"actor_account_id":tenant.scope().actor.account_id,"actor_membership_id":tenant.membership().id,"household_id":tenant.scope().household_id,"request_id":tenant.scope().request_id})),
        created_at: Set(now), updated_at: Set(now), ..Default::default()
    }.insert(tenant.transaction()).await?;
    Ok(())
}

pub async fn gp_history(
    tenant: &TenantTransaction,
    person: &person::Model,
    start: NaiveDate,
    end: NaiveDate,
    include_takes: bool,
    generated_at: DateTime<Utc>,
    zone: Tz,
) -> Result<GpHistoryReport, OperationError> {
    let household_id = tenant.scope().household_id;
    let db = tenant.transaction();
    let today = generated_at.with_timezone(&zone).date_naive();
    let schedule_medicines = schedule::Entity::find()
        .in_tenant(household_id)
        .select_only()
        .column(schedule::Column::MedicationId)
        .filter(schedule::Column::PersonId.eq(person.id))
        .filter(schedule::Column::Active.eq(true))
        .filter(schedule::Column::RetiredAt.is_null())
        .filter(schedule::Column::StartDate.lte(today))
        .filter(schedule::Column::EndDate.gte(today))
        .into_query();
    let assigned_medicines = person_medication::Entity::find()
        .in_tenant(household_id)
        .select_only()
        .column(person_medication::Column::MedicationId)
        .filter(person_medication::Column::PersonId.eq(person.id))
        .filter(person_medication::Column::Active.eq(true))
        .filter(person_medication::Column::RetiredAt.is_null())
        .into_query();
    let current = medication::Entity::find()
        .in_tenant(household_id)
        .filter(
            Condition::any()
                .add(medication::Column::Id.in_subquery(schedule_medicines))
                .add(medication::Column::Id.in_subquery(assigned_medicines)),
        )
        .order_by_asc(Func::lower(Func::coalesce([
            Expr::col(medication::Column::FriendlyName),
            Expr::value(""),
        ])))
        .order_by_asc(medication::Column::Id)
        .all(db)
        .await?;
    let current_medicines = current
        .into_iter()
        .map(|row| GpMedicine {
            id: row.id,
            name: row
                .friendly_name
                .filter(|name| !name.is_empty())
                .or(row.name),
        })
        .collect();
    let events = health_event::Entity::find()
        .in_tenant(household_id)
        .filter(health_event::Column::PersonId.eq(person.id))
        .filter(health_event::Column::StartedOn.lte(end))
        .filter(
            Condition::any()
                .add(health_event::Column::EndedOn.is_null())
                .add(health_event::Column::EndedOn.gte(start)),
        )
        .order_by_desc(health_event::Column::StartedOn)
        .order_by_desc(health_event::Column::Id)
        .all(db)
        .await?;
    let event_ids: Vec<i64> = events.iter().map(|row| row.id).collect();
    let event_medicines = if event_ids.is_empty() {
        Vec::new()
    } else {
        health_event_medication::Entity::find()
            .in_tenant(household_id)
            .filter(health_event_medication::Column::HealthEventId.is_in(event_ids))
            .order_by_asc(health_event_medication::Column::Id)
            .all(db)
            .await?
    };
    let mut names_by_event: HashMap<i64, Vec<String>> = HashMap::new();
    for row in event_medicines {
        names_by_event
            .entry(row.health_event_id)
            .or_default()
            .push(row.medication_name);
    }
    let chronology = events
        .into_iter()
        .map(|row| GpEvent {
            id: row.id,
            event_kind: if row.event_kind == 0 {
                "illness"
            } else {
                "suspected_side_effect"
            },
            title: row.title.trim_matches(' ').to_owned(),
            started_on: row.started_on,
            ended_on: row.ended_on,
            ongoing: row.ended_on.is_none(),
            duration_days: row
                .ended_on
                .map(|end| (end - row.started_on).num_days() + 1),
            severity: match row.severity {
                Some(0) => Some("mild"),
                Some(1) => Some("moderate"),
                Some(2) => Some("severe"),
                _ => None,
            },
            notes: row.notes,
            action_taken: row.action_taken,
            medical_help_sought: row.medical_help_sought,
            medication_names: names_by_event.remove(&row.id).unwrap_or_default(),
            kind_label: None,
            severity_label: None,
            date_range_label: None,
            details: None,
        })
        .collect();
    let medication_takes = if include_takes {
        gp_takes(tenant, person.id, start, end, zone).await?
    } else {
        Vec::new()
    };
    Ok(GpHistoryReport {
        person: GpPerson::from(person),
        start_date: start,
        end_date: end,
        generated_at: ReportUtc(generated_at),
        current_medicines,
        chronology,
        medication_takes,
        time_zone: None,
        include_medication_takes: None,
        generated_label: None,
        reporting_period: None,
        birthday_label: None,
    })
}

pub fn gp_history_pdf(mut data: GpHistoryReport, zone: Tz, include_takes: bool) -> GpHistoryReport {
    for take in &mut data.medication_takes {
        take.taken_at.pdf = true;
    }
    data.time_zone = Some(zone.name().to_owned());
    data.include_medication_takes = Some(include_takes);
    data
}

async fn gp_takes(
    tenant: &TenantTransaction,
    person_id: i64,
    start: NaiveDate,
    end: NaiveDate,
    zone: Tz,
) -> Result<Vec<GpTake>, OperationError> {
    let household_id = tenant.scope().household_id;
    let db = tenant.transaction();
    let schedule_ids = schedule::Entity::find()
        .in_tenant(household_id)
        .select_only()
        .column(schedule::Column::Id)
        .filter(schedule::Column::PersonId.eq(person_id))
        .into_query();
    let assignment_ids = person_medication::Entity::find()
        .in_tenant(household_id)
        .select_only()
        .column(person_medication::Column::Id)
        .filter(person_medication::Column::PersonId.eq(person_id))
        .into_query();
    let source_filter = Condition::any()
        .add(medication_take::Column::ScheduleId.in_subquery(schedule_ids))
        .add(medication_take::Column::PersonMedicationId.in_subquery(assignment_ids));
    let takes = medication_take::Entity::find()
        .in_tenant(household_id)
        .filter(source_filter)
        .filter(
            medication_take::Column::TakenAt.gte(
                (start - Duration::days(1))
                    .and_hms_opt(0, 0, 0)
                    .ok_or(OperationError::Unavailable)?,
            ),
        )
        .filter(
            medication_take::Column::TakenAt.lt((end + Duration::days(2))
                .and_hms_opt(0, 0, 0)
                .ok_or(OperationError::Unavailable)?),
        )
        .order_by_asc(medication_take::Column::TakenAt)
        .order_by_asc(medication_take::Column::Id)
        .all(db)
        .await?;
    let referenced_schedules: Vec<i64> = takes.iter().filter_map(|row| row.schedule_id).collect();
    let referenced_assignments: Vec<i64> = takes
        .iter()
        .filter_map(|row| row.person_medication_id)
        .collect();
    let source_schedules = if referenced_schedules.is_empty() {
        Vec::new()
    } else {
        schedule::Entity::find()
            .in_tenant(household_id)
            .filter(schedule::Column::Id.is_in(referenced_schedules))
            .all(db)
            .await?
    };
    let source_assignments = if referenced_assignments.is_empty() {
        Vec::new()
    } else {
        person_medication::Entity::find()
            .in_tenant(household_id)
            .filter(person_medication::Column::Id.is_in(referenced_assignments))
            .all(db)
            .await?
    };
    let schedule_by_id: HashMap<i64, schedule::Model> = source_schedules
        .into_iter()
        .map(|row| (row.id, row))
        .collect();
    let assignment_by_id: HashMap<i64, person_medication::Model> = source_assignments
        .into_iter()
        .map(|row| (row.id, row))
        .collect();
    let medicine_ids: HashSet<i64> = schedule_by_id
        .values()
        .map(|row| row.medication_id)
        .chain(assignment_by_id.values().map(|row| row.medication_id))
        .chain(takes.iter().filter_map(|row| row.taken_from_medication_id))
        .collect();
    let medicines = if medicine_ids.is_empty() {
        Vec::new()
    } else {
        medication::Entity::find()
            .in_tenant(household_id)
            .filter(medication::Column::Id.is_in(medicine_ids))
            .all(db)
            .await?
    };
    let medicines: HashMap<i64, medication::Model> =
        medicines.into_iter().map(|row| (row.id, row)).collect();
    let location_ids: HashSet<i64> = takes
        .iter()
        .filter_map(|row| row.taken_from_location_id)
        .collect();
    let locations = if location_ids.is_empty() {
        Vec::new()
    } else {
        location::Entity::find()
            .in_tenant(household_id)
            .filter(location::Column::Id.is_in(location_ids))
            .all(db)
            .await?
    };
    let locations: HashMap<i64, String> = locations
        .into_iter()
        .map(|row| (row.id, row.name))
        .collect();
    let display = |id: i64| {
        medicines.get(&id).and_then(|row| {
            row.friendly_name
                .as_ref()
                .filter(|name| !name.is_empty())
                .or(row.name.as_ref())
                .cloned()
        })
    };
    let mut rows = Vec::new();
    for take in takes {
        let Some(taken_at) = take.taken_at.and_then(|at| at.with_nanosecond(0)) else {
            continue;
        };
        let local = taken_at.and_utc().with_timezone(&zone);
        if !(start..=end).contains(&local.date_naive()) {
            continue;
        }
        let schedule = take.schedule_id.and_then(|id| schedule_by_id.get(&id));
        let assignment = take
            .person_medication_id
            .and_then(|id| assignment_by_id.get(&id));
        let source_medicine = schedule
            .map(|row| row.medication_id)
            .or_else(|| assignment.map(|row| row.medication_id));
        let name = source_medicine
            .and_then(display)
            .or_else(|| take.taken_from_medication_id.and_then(display));
        let source_type = if assignment.is_some_and(|row| row.administration_kind == 0) {
            "routine"
        } else if assignment.is_some() || schedule.is_some_and(|row| row.schedule_type == 4) {
            "as_needed"
        } else {
            "scheduled"
        };
        rows.push(GpTake {
            taken_at: GpTakeTime {
                at: local.fixed_offset(),
                pdf: false,
            },
            medication_name: name,
            dose_amount: take.dose_amount,
            dose_unit: take.dose_unit,
            source_type,
            location_name: take
                .taken_from_location_id
                .and_then(|id| locations.get(&id).cloned()),
        });
    }
    Ok(rows)
}

pub async fn medication_reviews(
    tenant: &TenantTransaction,
    person: &person::Model,
    status: Option<&str>,
    generated_at: DateTime<Utc>,
    today: NaiveDate,
) -> Result<MedicationReviewReport, OperationError> {
    review_prompts::refresh(tenant, person.id, today).await?;
    let mut query = review_prompt::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(review_prompt::Column::PersonId.eq(person.id));
    query = if let Some(status) = status {
        query.filter(review_prompt::Column::Status.eq(status))
    } else {
        query.filter(review_prompt::Column::Status.ne("hidden_low_signal"))
    };
    let rows = query
        .order_by_asc(review_prompt::Column::CreatedAt)
        .order_by_asc(review_prompt::Column::Id)
        .all(tenant.transaction())
        .await?;
    Ok(MedicationReviewReport {
        person: ReviewPerson {
            id: Some(person.id),
            name: person.name.clone(),
        },
        generated_at: ReportUtc(generated_at),
        prompts: rows.iter().map(ReviewPromptRow::from).collect(),
        people_count: None,
        to_discuss: None,
        reviewed: None,
        time_zone: None,
        generated_label: None,
    })
}

pub fn medication_review_pdf(
    mut data: MedicationReviewReport,
    people: &[person::Model],
    zone: Tz,
) -> MedicationReviewReport {
    let names: HashMap<i64, &str> = people
        .iter()
        .map(|row| (row.id, row.name.as_str()))
        .collect();
    let mut to_discuss = 0usize;
    let mut reviewed = 0usize;
    let mut represented: HashSet<i64> = HashSet::new();
    for prompt in &mut data.prompts {
        prompt.person_name = Some(
            names
                .get(&prompt.person_id)
                .copied()
                .unwrap_or("")
                .to_owned(),
        );
        represented.insert(prompt.person_id);
        match prompt.status.as_str() {
            "needs_review" => to_discuss += 1,
            "reviewed_with_practitioner" | "expected_prescribed_combination" | "not_relevant" => {
                reviewed += 1
            }
            _ => {}
        }
    }
    data.to_discuss = Some(to_discuss);
    data.reviewed = Some(reviewed);
    data.people_count = Some(represented.len());
    data.time_zone = Some(zone.name().to_owned());
    data
}

pub async fn household_medication_reviews(
    tenant: &TenantTransaction,
    people: &[person::Model],
    status: Option<&str>,
    generated_at: DateTime<Utc>,
    today: NaiveDate,
) -> Result<MedicationReviewReport, OperationError> {
    let ids: Vec<i64> = people.iter().map(|row| row.id).collect();
    review_prompts::refresh_many(tenant, &ids, today).await?;
    let mut query = review_prompt::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(review_prompt::Column::PersonId.is_in(ids));
    query = if let Some(status) = status {
        query.filter(review_prompt::Column::Status.eq(status))
    } else {
        query.filter(review_prompt::Column::Status.ne("hidden_low_signal"))
    };
    let rows = query
        .order_by_asc(review_prompt::Column::CreatedAt)
        .order_by_asc(review_prompt::Column::Id)
        .all(tenant.transaction())
        .await?;
    let names: HashMap<i64, &str> = people
        .iter()
        .map(|row| (row.id, row.name.as_str()))
        .collect();
    let prompts: Vec<ReviewPromptRow> = rows
        .iter()
        .map(|row| {
            let mut value = ReviewPromptRow::from(row);
            value.person_name = Some(names.get(&row.person_id).copied().unwrap_or("").to_owned());
            value
        })
        .collect();
    let to_discuss = rows
        .iter()
        .filter(|row| row.status == "needs_review")
        .count();
    let reviewed = rows
        .iter()
        .filter(|row| {
            matches!(
                row.status.as_str(),
                "reviewed_with_practitioner" | "expected_prescribed_combination" | "not_relevant"
            )
        })
        .count();
    let represented_people = rows
        .iter()
        .map(|row| row.person_id)
        .collect::<HashSet<_>>()
        .len();
    Ok(MedicationReviewReport {
        person: ReviewPerson {
            id: None,
            name: people
                .iter()
                .map(|row| row.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        },
        people_count: Some(represented_people),
        generated_at: ReportUtc(generated_at),
        to_discuss: Some(to_discuss),
        reviewed: Some(reviewed),
        prompts,
        time_zone: None,
        generated_label: None,
    })
}

fn inventory_alerts(
    schedules: &[schedule::Model],
    medicines: &HashMap<i64, medication::Model>,
    today: NaiveDate,
    settings: Option<&serde_json::Value>,
) -> Vec<InventoryAlert> {
    let mut alerts: Vec<InventoryAlert> = schedules
        .iter()
        .filter_map(|source| {
            if !source.active
                || source.retired_at.is_some()
                || source.start_date.is_some_and(|date| date > today)
                || source.end_date.is_some_and(|date| date < today)
            {
                return None;
            }
            let medicine = medicines.get(&source.medication_id)?;
            let supply = medicine.current_supply.unwrap_or(Decimal::ZERO);
            let first = source.start_date.map_or(today, |day| day.max(today));
            let last = source.end_date.map_or(today + Duration::days(30), |day| {
                day.min(today + Duration::days(30))
            });
            if last < first {
                return None;
            }
            let days = (last - first).num_days() + 1;
            let mut consumed = Decimal::ZERO;
            for offset in 0..days {
                let day = first + Duration::days(offset);
                let expected = i64::from(dose_occurrences::expected_schedule_doses_for_report(
                    source, day, settings,
                ));
                if expected == 0 {
                    continue;
                }
                let config = dose_occurrences::schedule_config_on(source, day)
                    .unwrap_or(&source.schedule_config);
                let amount = doses::config_decimal(config, &["amount", "dose_amount"])
                    .or(source.dose_amount);
                let unit = doses::config_value(config, &["unit", "dose_unit"])
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                    .or(source.dose_unit.as_deref());
                let quantity = if matches!(
                    unit,
                    Some(
                        "tablet" | "capsule" | "gummy" | "sachet" | "spray" | "drop" | "pad" | "ml"
                    )
                ) {
                    amount.unwrap_or(Decimal::ZERO)
                } else {
                    Decimal::ONE
                };
                consumed += Decimal::from(expected) * quantity;
            }
            let burn_rate = consumed / Decimal::from(days);
            if burn_rate <= Decimal::ZERO {
                return None;
            }
            let days_left = (supply / burn_rate)
                .floor()
                .to_string()
                .parse::<i64>()
                .unwrap_or(0);
            if days_left >= 14 {
                return None;
            }
            let name = medicine
                .friendly_name
                .as_deref()
                .filter(|name| !name.is_empty())
                .or(medicine.name.as_deref())
                .unwrap_or("");
            Some(InventoryAlert {
                medication_name: name.to_owned(),
                days_left,
                doses_left: supply.to_string().parse::<f64>().unwrap_or(0.0),
                low_stock: days_left <= 3,
            })
        })
        .collect();
    alerts.sort_by_key(|row| row.days_left);
    alerts.into_iter().take(2).collect()
}

#[allow(clippy::too_many_arguments)]
fn insight_summary(
    daily: &[DailyOutcome],
    cycles: &[CycleSummary],
    takes: &[OrdinaryTake],
    alerts: &[InventoryAlert],
    schedules: &[schedule::Model],
    medicines: &HashMap<i64, medication::Model>,
    period: InsightPeriod,
    settings: Option<&serde_json::Value>,
) -> SmartInsights {
    let InsightPeriod { start, end, today } = period;
    let expected: u64 = daily.iter().map(|row| row.expected as u64).sum::<u64>()
        + cycles.iter().map(|row| row.expected as u64).sum::<u64>();
    let logged: u64 = daily
        .iter()
        .map(|row| (row.actual + row.not_taken) as u64)
        .sum::<u64>()
        + cycles
            .iter()
            .map(|row| (row.actual + row.not_taken) as u64)
            .sum::<u64>()
        + takes
            .iter()
            .filter(|row| row.source_type == "as_needed")
            .count() as u64;
    let evidence_events = expected + logged;
    let evidence_summary = format!(
        "{} days reviewed with {} medication {}",
        daily.len(),
        evidence_events,
        if evidence_events == 1 {
            "event"
        } else {
            "events"
        }
    );
    if daily.len() < 7 || evidence_events < 5 {
        return SmartInsights {
            state: "learning",
            evidence_summary,
            insights: Vec::new(),
        };
    }
    let mut insights = Vec::new();
    if let Some(alert) = alerts.first() {
        let name = alert.medication_name.as_str();
        let days = alert.days_left;
        insights.push(Insight {
            title: "Supply needs attention",
            summary: if days <= 0 {
                format!("{name} appears to be out of stock.")
            } else if days == 1 {
                format!("{name} has about 1 day of supply left.")
            } else {
                format!("{name} has about {days} days of supply left.")
            },
            detail: format!("Check the stock record for {name} and plan the next refill."),
            severity: if days <= 3 { "urgent" } else { "warning" },
        });
    }
    let mut current_missed = 0;
    let mut longest_missed = 0;
    for row in daily {
        current_missed = if row.unexplained_missed > 0 {
            current_missed + 1
        } else {
            0
        };
        longest_missed = longest_missed.max(current_missed);
    }
    if longest_missed >= 2 {
        insights.push(Insight {
            title: "Missed-dose pattern",
            summary: format!("Expected doses were missed for {longest_missed} days in a row."),
            detail: "Review the schedule and reminders for this window.".to_owned(),
            severity: "warning",
        });
    }
    let mut cycle_groups: HashMap<i64, Vec<(NaiveDate, NaiveDate, u64)>> = HashMap::new();
    for cycle in cycles {
        let (id, first, last) = (
            cycle.source_id,
            cycle.window_starts_on,
            cycle.window_ends_on,
        );
        if last < today {
            cycle_groups.entry(id).or_default().push((
                first,
                last,
                cycle.unexplained_missed as u64,
            ));
        }
    }
    let mut longest_cycles = 0;
    for group in cycle_groups.values_mut() {
        group.sort_by_key(|row| row.0);
        let (mut previous, mut current) = (None, 0);
        for (first, last, missed) in group {
            if previous.is_some_and(|day| day != *first - Duration::days(1)) {
                current = 0;
            }
            current = if *missed > 0 { current + 1 } else { 0 };
            longest_cycles = longest_cycles.max(current);
            previous = Some(*last);
        }
    }
    if longest_cycles >= 2 {
        insights.push(Insight {
            title: "Missed routine cycles",
            summary: format!("{longest_cycles} consecutive cycles have unexplained missed doses."),
            detail: "Review the routine dose records for these cycles.".to_owned(),
            severity: "warning",
        });
    }
    let adherence = daily
        .iter()
        .rev()
        .take_while(|row| row.expected > 0 && row.actual >= row.expected)
        .count();
    if adherence >= 3 {
        insights.push(Insight {
            title: "Adherence streak",
            summary: format!("Every expected dose was logged for {adherence} days."),
            detail: "That consistency gives your care routine a solid rhythm.".to_owned(),
            severity: "positive",
        });
    }
    let mut expected_times = Vec::new();
    for source in schedules {
        let mut day = start;
        while day <= end {
            if dose_occurrences::expected_schedule_doses_for_report(source, day, settings) > 0
                && let Some(times) = source
                    .schedule_config
                    .get("times")
                    .and_then(Value::as_array)
            {
                for time in times {
                    if let Some(time) = time
                        .as_str()
                        .and_then(|value| chrono::NaiveTime::parse_from_str(value, "%H:%M").ok())
                    {
                        expected_times.push((source.id, day.and_time(time)));
                    }
                }
            }
            day += Duration::days(1);
        }
    }
    if expected_times.len() >= 5 {
        let mut used = HashSet::new();
        let on_time = expected_times
            .iter()
            .filter(|(source_id, expected_at)| {
                takes
                    .iter()
                    .enumerate()
                    .find(|(index, take)| {
                        !used.contains(index)
                            && take.source_id == *source_id
                            && within_ninety_report_minutes(take.taken_at.0, *expected_at)
                    })
                    .is_some_and(|(index, _)| used.insert(index))
            })
            .count();
        let percentage = (on_time as f64 / expected_times.len() as f64 * 100.0).round() as u64;
        if percentage >= 80 {
            insights.push(Insight {
                title: "Consistent dose timing",
                summary: format!(
                    "{percentage}% of logged doses were close to their scheduled time."
                ),
                detail: "Your logged timing has been steady in this report window.".to_owned(),
                severity: "positive",
            });
        }
    }
    let prn = takes
        .iter()
        .filter(|row| row.source_type == "as_needed")
        .count();
    if prn > 0 {
        insights.push(Insight {
            title: "As-needed use logged",
            summary: format!("{prn} as-needed doses were logged in this window."),
            detail: "Review these entries alongside routine doses to keep the full picture clear."
                .to_owned(),
            severity: "info",
        });
    }
    if let Some(source) = schedules.iter().find(|source| {
        source.active
            && source.schedule_type == 1
            && source
                .schedule_config
                .get("times")
                .and_then(Value::as_array)
                .is_none_or(Vec::is_empty)
    }) {
        let name = medicines
            .get(&source.medication_id)
            .and_then(|record| {
                record
                    .friendly_name
                    .as_deref()
                    .filter(|name| !name.is_empty())
                    .or(record.name.as_deref())
            })
            .unwrap_or("");
        insights.push(Insight {
            title: "Schedule detail to review",
            summary: format!("{name} is set for multiple daily doses without specific times."),
            detail: "Adding times can make reminders, reporting, and timing insights clearer."
                .to_owned(),
            severity: "info",
        });
    }
    SmartInsights {
        state: if insights.is_empty() {
            "no_action"
        } else {
            "insights"
        },
        evidence_summary,
        insights,
    }
}

fn within_ninety_report_minutes(
    taken_at: chrono::NaiveDateTime,
    expected_at: chrono::NaiveDateTime,
) -> bool {
    let minute = taken_at.with_second(0).and_then(|at| at.with_nanosecond(0));
    minute.is_some_and(|at| (at - expected_at).num_minutes().abs() <= 90)
}

pub async fn ordinary_history(
    tenant: &TenantTransaction,
    people: &[person::Model],
    start: NaiveDate,
    end: NaiveDate,
    generated_at: DateTime<Utc>,
    zone: Tz,
) -> Result<OrdinaryHistoryReport, OperationError> {
    super::person_schedules::projection::refresh(tenant).await?;
    let current = access::people_with_access(tenant, PersonAccess::View).await?;
    if people
        .iter()
        .any(|person| !current.iter().any(|visible| visible.id == person.id))
    {
        return Err(OperationError::Forbidden);
    }
    let ids: Vec<i64> = people.iter().map(|row| row.id).collect();
    let names: HashMap<i64, &str> = people
        .iter()
        .map(|row| (row.id, row.name.as_str()))
        .collect();
    let db = tenant.transaction();
    let household_id = tenant.scope().household_id;
    let outcomes = dose_occurrences::with_dashboard_timezone(
        zone,
        dose_occurrences::projected_for_report(tenant, &ids, start, end),
    )
    .await?;
    let schedules = schedule::Entity::find()
        .in_tenant(household_id)
        .filter(schedule::Column::PersonId.is_in(ids.clone()))
        .all(db)
        .await?;
    let assignments = person_medication::Entity::find()
        .in_tenant(household_id)
        .filter(person_medication::Column::PersonId.is_in(ids.clone()))
        .all(db)
        .await?;
    let sources: HashMap<(&str, i64), (i64, i64, &str)> = schedules
        .iter()
        .map(|row| {
            (
                ("schedule", row.id),
                (
                    row.person_id,
                    row.medication_id,
                    if row.schedule_type == 4 {
                        "as_needed"
                    } else {
                        "scheduled"
                    },
                ),
            )
        })
        .chain(assignments.iter().map(|row| {
            (
                ("person_medication", row.id),
                (
                    row.person_id,
                    row.medication_id,
                    if row.administration_kind == 0 {
                        "routine"
                    } else {
                        "as_needed"
                    },
                ),
            )
        }))
        .collect();
    let medicines: HashMap<i64, medication::Model> = medication::Entity::find()
        .in_tenant(household_id)
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row))
        .collect();
    let medication_name = |id: i64| {
        medicines
            .get(&id)
            .and_then(|row| {
                row.friendly_name
                    .as_deref()
                    .filter(|name| !name.is_empty())
                    .or(row.name.as_deref())
            })
            .unwrap_or("")
            .to_owned()
    };
    let today = generated_at.with_timezone(&zone).date_naive();
    let take_scan_start = start.min(today) - Duration::days(1);
    let take_scan_end = end.max(today) + Duration::days(2);
    let takes = medication_take::Entity::find()
        .in_tenant(household_id)
        .filter(
            medication_take::Column::TakenAt.gte(
                take_scan_start
                    .and_hms_opt(0, 0, 0)
                    .ok_or(OperationError::Unavailable)?,
            ),
        )
        .filter(
            medication_take::Column::TakenAt.lt(take_scan_end
                .and_hms_opt(0, 0, 0)
                .ok_or(OperationError::Unavailable)?),
        )
        .order_by_asc(medication_take::Column::TakenAt)
        .order_by_asc(medication_take::Column::Id)
        .all(db)
        .await?;
    let mut take_rows = Vec::new();
    let mut today_medicines: BTreeMap<(String, i64), BTreeMap<i64, String>> = BTreeMap::new();
    let mut actual_by_day: HashMap<NaiveDate, usize> = HashMap::new();
    let cycle_sources: HashSet<(&str, i64, NaiveDate, NaiveDate)> = outcomes
        .iter()
        .filter(|row| row.window_end > row.window_start)
        .map(|row| {
            (
                row.source_type,
                row.source_id,
                row.window_start,
                row.window_end,
            )
        })
        .collect();
    for take in &takes {
        let source_key = take.schedule_id.map(|id| ("schedule", id)).or_else(|| {
            take.person_medication_id
                .map(|id| ("person_medication", id))
        });
        let Some((person_id, medication_id, source_type)) =
            source_key.and_then(|key| sources.get(&key).copied())
        else {
            continue;
        };
        let Some(taken_at) = take.taken_at else {
            continue;
        };
        let at = DateTime::<Utc>::from_naive_utc_and_offset(taken_at, Utc).with_timezone(&zone);
        let day = at.date_naive();
        if day == today {
            today_medicines
                .entry((
                    names.get(&person_id).copied().unwrap_or("").to_owned(),
                    person_id,
                ))
                .or_default()
                .insert(medication_id, medication_name(medication_id));
        }
        if day < start || day > end {
            continue;
        }
        let cycle = source_key.is_some_and(|(kind, id)| {
            cycle_sources
                .iter()
                .any(|(source_kind, source_id, first, last)| {
                    *source_kind == kind && *source_id == id && *first <= day && day <= *last
                })
        });
        if !cycle {
            *actual_by_day.entry(day).or_default() += 1;
        }
        take_rows.push(OrdinaryTake {
            taken_at: LocalMinute(at.naive_local()),
            source_id: source_key
                .map(|(_, id)| id)
                .ok_or(OperationError::Unavailable)?,
            person_name: names.get(&person_id).copied().unwrap_or("").to_owned(),
            medication_name: medication_name(medication_id),
            dose_amount: take.dose_amount,
            dose_unit: take.dose_unit.clone(),
            source_type,
            location_name: "",
        });
    }
    let now_utc = generated_at.naive_utc();
    let mut daily: BTreeMap<NaiveDate, (usize, usize, usize)> = BTreeMap::new();
    let mut cycles: BTreeMap<CycleKey<'_>, CycleCounts> = BTreeMap::new();
    let mut not_taken = Vec::new();
    for row in &outcomes {
        if !row.expected && row.outcome == "open" {
            continue;
        }
        let missed = row.outcome == "open"
            && row
                .scheduled_at
                .map_or(row.window_end < today, |time| time < now_utc);
        if row.outcome == "not_taken" {
            not_taken.push(NotTakenOutcome {
                scheduled_at: row
                    .scheduled_at
                    .map_or(NotTakenTime::Window(row.window_start), |at| {
                        NotTakenTime::At(at.and_utc().with_timezone(&zone).fixed_offset())
                    }),
                person_name: names.get(&row.person_id).copied().unwrap_or("").to_owned(),
                medication_name: medication_name(row.medication_id),
                reason: row.reason.clone(),
                note: row.note.clone(),
            });
        }
        if row.window_end > row.window_start {
            let counts = cycles
                .entry((
                    row.source_type,
                    row.source_id,
                    row.window_start,
                    row.window_end,
                ))
                .or_insert((row.person_id, row.medication_id, 0, 0, 0, 0));
            counts.2 += usize::from(row.expected);
            counts.3 += usize::from(row.outcome == "taken");
            counts.4 += usize::from(row.outcome == "not_taken");
            counts.5 += usize::from(missed);
        } else {
            let counts = daily.entry(row.window_start).or_default();
            counts.0 += usize::from(row.expected);
            counts.1 += usize::from(row.outcome == "not_taken");
            counts.2 += usize::from(missed);
        }
    }
    let cycle_summaries: Vec<CycleSummary> = cycles
        .into_iter()
        .map(
            |(
                (_, source_id, first, last),
                (person_id, medication_id, expected, actual, not_taken, missed),
            )| CycleSummary {
                window_starts_on: first,
                window_ends_on: last,
                source_id,
                person_name: names.get(&person_id).copied().unwrap_or("").to_owned(),
                medication_name: medication_name(medication_id),
                expected,
                actual,
                not_taken,
                unexplained_missed: missed,
            },
        )
        .collect();
    let mut daily_outcomes = Vec::new();
    let mut day = start;
    while day <= end {
        let (expected, not_taken, missed) = daily.get(&day).copied().unwrap_or_default();
        let actual = actual_by_day.get(&day).copied().unwrap_or(0);
        let percentage = if expected == 0 {
            100
        } else {
            ((actual as f64 / expected as f64 * 100.0).round() as usize).min(100)
        };
        daily_outcomes.push(DailyOutcome {
            date: day,
            day_name: day.format("%a").to_string(),
            percentage,
            expected,
            actual,
            not_taken,
            unexplained_missed: missed,
        });
        day += Duration::days(1);
    }
    let events = health_event::Entity::find()
        .in_tenant(household_id)
        .filter(health_event::Column::PersonId.is_in(ids))
        .filter(health_event::Column::StartedOn.lte(end))
        .filter(
            Condition::any()
                .add(health_event::Column::EndedOn.is_null())
                .add(health_event::Column::EndedOn.gte(start)),
        )
        .order_by_desc(health_event::Column::StartedOn)
        .order_by_desc(health_event::Column::Id)
        .all(db)
        .await?;
    let event_ids: Vec<i64> = events.iter().map(|row| row.id).collect();
    let event_medicines = if event_ids.is_empty() {
        Vec::new()
    } else {
        health_event_medication::Entity::find()
            .in_tenant(household_id)
            .filter(health_event_medication::Column::HealthEventId.is_in(event_ids))
            .all(db)
            .await?
    };
    let mut by_event: HashMap<i64, Vec<String>> = HashMap::new();
    for row in event_medicines {
        by_event
            .entry(row.health_event_id)
            .or_default()
            .push(row.medication_name);
    }
    let mut side_effects = Vec::new();
    let mut illnesses = Vec::new();
    let mut patterns: HashMap<String, Vec<NaiveDate>> = HashMap::new();
    for event in events {
        let value = OrdinaryEvent {
            title: event.title.clone(),
            started_on: event.started_on,
            ended_on: event.ended_on,
            severity: event.severity.map(|level| match level {
                0 => "mild",
                1 => "moderate",
                2 => "severe",
                _ => "",
            }),
            notes: event.notes,
            action_taken: event.action_taken,
            medication_names: by_event.remove(&event.id).unwrap_or_default(),
        };
        if event.event_kind == 0 {
            patterns
                .entry(event.title.to_lowercase())
                .or_default()
                .push(event.started_on);
            illnesses.push(value);
        } else {
            side_effects.push(value);
        }
    }
    let illness_patterns: Vec<IllnessPattern> = patterns
        .into_iter()
        .filter_map(|(title, mut dates)| {
            if dates.len() < 2 {
                return None;
            }
            dates.sort();
            let span = (dates.last()?.to_owned() - dates.first()?.to_owned()).num_days();
            Some(IllnessPattern {
                display_title: title,
                episode_count: dates.len(),
                first_started_on: *dates.first()?,
                most_recent_started_on: *dates.last()?,
                average_interval_days: span / (dates.len() as i64 - 1),
            })
        })
        .collect();
    let inventory_alerts = inventory_alerts(&schedules, &medicines, today, tenant.settings());
    let insights = insight_summary(
        &daily_outcomes,
        &cycle_summaries,
        &take_rows,
        &inventory_alerts,
        &schedules,
        &medicines,
        InsightPeriod { start, end, today },
        tenant.settings(),
    );
    let total_expected: usize = daily_outcomes.iter().map(|row| row.expected).sum::<usize>()
        + cycle_summaries
            .iter()
            .map(|row| row.expected)
            .sum::<usize>();
    let total_actual: usize = daily_outcomes.iter().map(|row| row.actual).sum::<usize>()
        + cycle_summaries.iter().map(|row| row.actual).sum::<usize>();
    let compliance = if total_expected == 0 {
        100
    } else {
        ((total_actual as f64 / total_expected as f64 * 100.0).round() as usize).min(100)
    };
    let today_taken: Vec<TodayTaken> = today_medicines
        .into_iter()
        .map(|((person_name, person_id), medicines)| TodayTaken {
            person_id,
            person_name,
            medications: medicines
                .into_iter()
                .map(|(id, name)| TodayMedicine { id, name })
                .collect(),
        })
        .collect();
    Ok(OrdinaryHistoryReport {
        people: OrdinaryPeople::Names(people.iter().map(|row| row.name.clone()).collect()),
        start_date: start,
        end_date: end,
        generated_at: ReportUtc(generated_at),
        medication_takes: take_rows,
        not_taken_outcomes: not_taken,
        daily_outcomes,
        cycle_summaries,
        today_taken,
        inventory_alerts,
        smart_insights: insights,
        summary: OrdinarySummary {
            expected: total_expected,
            actual: total_actual,
            compliance,
        },
        suspected_side_effects: side_effects,
        notable_illnesses: illnesses,
        illness_patterns,
        generated_label: None,
        manageable_people: None,
        selected_person_id: None,
        include_medication_takes: None,
    })
}

#[cfg(test)]
mod typed_report_tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn report_rows_keep_typed_dates_and_minute_projected_timing() {
        fn accepts_report(_: &OrdinaryHistoryReport) {}
        let expected = NaiveDate::from_ymd_opt(2026, 7, 1)
            .unwrap()
            .and_hms_opt(11, 0, 0)
            .unwrap();
        let early = NaiveDate::from_ymd_opt(2026, 7, 1)
            .unwrap()
            .and_hms_opt(9, 29, 59)
            .unwrap();
        let boundary = NaiveDate::from_ymd_opt(2026, 7, 1)
            .unwrap()
            .and_hms_opt(9, 30, 59)
            .unwrap();
        assert!(!within_ninety_report_minutes(early, expected));
        assert!(within_ninety_report_minutes(boundary, expected));
        let _ = accepts_report;
    }

    #[test]
    fn gp_wire_keeps_nulls_decimal_spelling_and_utc_offset() {
        let day = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        let at = day.and_hms_opt(9, 0, 0).unwrap().and_utc();
        let report = GpHistoryReport {
            person: GpPerson {
                id: 73,
                name: "Person".into(),
                date_of_birth: None,
            },
            start_date: day,
            end_date: day,
            generated_at: ReportUtc(at),
            current_medicines: vec![GpMedicine { id: 80, name: None }],
            chronology: Vec::new(),
            medication_takes: vec![GpTake {
                taken_at: GpTakeTime {
                    at: at.fixed_offset(),
                    pdf: false,
                },
                medication_name: None,
                dose_amount: Some(Decimal::from_str("1.2500").unwrap()),
                dose_unit: None,
                source_type: "scheduled",
                location_name: None,
            }],
            time_zone: None,
            include_medication_takes: None,
            generated_label: None,
            reporting_period: None,
            birthday_label: None,
        };
        let wire = serde_json::to_value(report).unwrap();
        assert_eq!(wire["generated_at"], "2026-07-01T09:00:00+00:00");
        assert_eq!(wire["person"]["id"], "73");
        assert!(wire["person"]["date_of_birth"].is_null());
        assert!(wire["current_medicines"][0]["name"].is_null());
        assert_eq!(wire["medication_takes"][0]["dose_amount"], "1.2500");
        assert!(wire["medication_takes"][0]["location_name"].is_null());
        assert!(wire.get("time_zone").is_none());
    }
}
