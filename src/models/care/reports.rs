use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    authorization,
    care::{dose_occurrences, doses, review_prompts},
    entities::{
        grant, health_event, health_event_medication, medication, medication_take, person,
        person_medication, review_prompt, schedule, security_audit_event,
    },
    errors::OperationError,
};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use chrono_tz::Tz;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DbBackend, EntityTrait, QueryFilter,
    QueryOrder, Set, Statement, prelude::Decimal,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};

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
    let Some(id) = tenant.membership().person_id else {
        return Ok(false);
    };
    let Some(actor) = person::Entity::find_by_id(id)
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
    Ok(actor.household_id == tenant.scope().household_id && (actor.person_type == 0 || by_age))
}

pub async fn accessible_people(
    tenant: &TenantTransaction,
    level: PersonAccess,
) -> Result<Vec<person::Model>, OperationError> {
    access::recheck(tenant).await?;
    if !authorization::ready() {
        return Err(OperationError::Unavailable);
    }
    let people = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .order_by_asc(person::Column::Name)
        .order_by_asc(person::Column::Id)
        .all(tenant.transaction())
        .await?;
    let grants = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(tenant.membership().id))
        .filter(grant::Column::RevokedAt.is_null())
        .all(tenant.transaction())
        .await?;
    let grants: HashMap<i64, grant::Model> =
        grants.into_iter().map(|row| (row.person_id, row)).collect();
    let now = Utc::now().naive_utc();
    Ok(people
        .into_iter()
        .filter(|person| {
            grants.get(&person.id).is_some_and(|grant| {
                authorization::person_access(tenant.membership(), person, grant, level, now)
            })
        })
        .collect())
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

async fn json_rows(
    tenant: &TenantTransaction,
    sql: &str,
    values: Vec<sea_orm::Value>,
) -> Result<Value, OperationError> {
    let row = tenant
        .transaction()
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            values,
        ))
        .await?
        .ok_or(OperationError::Unavailable)?;
    row.try_get("", "payload").map_err(Into::into)
}

pub async fn gp_history(
    tenant: &TenantTransaction,
    person: &person::Model,
    start: NaiveDate,
    end: NaiveDate,
    include_takes: bool,
    generated_at: DateTime<Utc>,
    zone: Tz,
) -> Result<Value, OperationError> {
    let medicines = json_rows(
        tenant,
        "SELECT COALESCE(jsonb_agg(jsonb_build_object('id', m.id::text, 'name', COALESCE(NULLIF(m.friendly_name, ''), m.name)) ORDER BY lower(COALESCE(m.friendly_name, '')), m.id), '[]'::jsonb) AS payload FROM medications m WHERE m.household_id = $1 AND m.id IN (SELECT s.medication_id FROM schedules s WHERE s.person_id = $2 AND s.household_id = $1 AND s.active AND s.retired_at IS NULL AND s.start_date <= $3::date AND s.end_date >= $3::date UNION SELECT pm.medication_id FROM person_medications pm WHERE pm.person_id = $2 AND pm.household_id = $1 AND pm.active AND pm.retired_at IS NULL)",
        vec![person.household_id.into(), person.id.into(), generated_at.with_timezone(&zone).date_naive().to_string().into()],
    )
    .await?;
    let chronology = json_rows(
        tenant,
        "SELECT COALESCE(jsonb_agg(jsonb_build_object('id', he.id::text, 'event_kind', CASE he.event_kind WHEN 0 THEN 'illness' ELSE 'suspected_side_effect' END, 'title', btrim(he.title), 'started_on', he.started_on, 'ended_on', he.ended_on, 'ongoing', he.ended_on IS NULL, 'duration_days', CASE WHEN he.ended_on IS NULL THEN NULL ELSE he.ended_on - he.started_on + 1 END, 'severity', CASE he.severity WHEN 0 THEN 'mild' WHEN 1 THEN 'moderate' WHEN 2 THEN 'severe' ELSE NULL END, 'notes', he.notes, 'action_taken', he.action_taken, 'medical_help_sought', he.medical_help_sought, 'medication_names', (SELECT COALESCE(jsonb_agg(hem.medication_name ORDER BY hem.id), '[]'::jsonb) FROM health_event_medications hem WHERE hem.health_event_id = he.id AND hem.household_id = $1)) ORDER BY he.started_on DESC, he.id DESC), '[]'::jsonb) AS payload FROM health_events he WHERE he.household_id = $1 AND he.person_id = $2 AND he.started_on <= $4::date AND (he.ended_on IS NULL OR he.ended_on >= $3::date)",
        vec![person.household_id.into(), person.id.into(), start.to_string().into(), end.to_string().into()],
    )
    .await?;
    let takes = if include_takes {
        let mut takes = json_rows(
            tenant,
            "SELECT COALESCE(jsonb_agg(jsonb_build_object('taken_at', to_char(t.taken_at, 'YYYY-MM-DD\"T\"HH24:MI:SS') || 'Z', 'medication_name', COALESCE(NULLIF(sm.friendly_name, ''), sm.name, NULLIF(dm.friendly_name, ''), dm.name), 'dose_amount', t.dose_amount::text, 'dose_unit', t.dose_unit, 'source_type', CASE WHEN pm.id IS NOT NULL AND pm.administration_kind = 0 THEN 'routine' WHEN pm.id IS NOT NULL OR s.schedule_type = 4 THEN 'as_needed' ELSE 'scheduled' END, 'location_name', tfl.name) ORDER BY t.taken_at, t.id), '[]'::jsonb) AS payload FROM medication_takes t LEFT JOIN schedules s ON s.id = t.schedule_id AND s.household_id = $1 LEFT JOIN person_medications pm ON pm.id = t.person_medication_id AND pm.household_id = $1 LEFT JOIN medications sm ON sm.id = COALESCE(s.medication_id, pm.medication_id) AND sm.household_id = $1 LEFT JOIN medications dm ON dm.id = t.taken_from_medication_id AND dm.household_id = $1 LEFT JOIN locations tfl ON tfl.id = t.taken_from_location_id AND tfl.household_id = $1 WHERE t.household_id = $1 AND (s.person_id = $2 OR pm.person_id = $2) AND t.taken_at >= $3::timestamp AND t.taken_at < ($4::date + INTERVAL '1 day')",
            vec![person.household_id.into(), person.id.into(), (start - Duration::days(1)).to_string().into(), (end + Duration::days(1)).to_string().into()],
        )
        .await?;
        if let Some(rows) = takes.as_array_mut() {
            rows.retain_mut(|row| {
                let Some(at) = row["taken_at"]
                    .as_str()
                    .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                else {
                    return false;
                };
                let local = at.with_timezone(&zone);
                if !(start..=end).contains(&local.date_naive()) {
                    return false;
                }
                row["taken_at"] = json!(local.to_rfc3339());
                true
            });
        }
        takes
    } else {
        json!([])
    };
    Ok(json!({
        "person": {"id": person.id.to_string(), "name": person.name, "date_of_birth": person.date_of_birth.map(|date| date.to_string())},
        "start_date": start.to_string(),
        "end_date": end.to_string(),
        "generated_at": generated_at.to_rfc3339(),
        "current_medicines": medicines,
        "chronology": chronology,
        "medication_takes": takes,
    }))
}

pub fn gp_history_pdf(mut data: Value, zone: Tz, include_takes: bool) -> Value {
    if let Some(takes) = data["medication_takes"].as_array_mut() {
        for take in takes {
            let Some(local) = take["taken_at"]
                .as_str()
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .map(|at| at.with_timezone(&zone))
            else {
                continue;
            };
            take["taken_at"] = json!(local.format("%Y-%m-%d %H:%M").to_string());
        }
    }
    data["time_zone"] = json!(zone.name());
    data["include_medication_takes"] = json!(include_takes);
    data
}

pub async fn medication_reviews(
    tenant: &TenantTransaction,
    person: &person::Model,
    status: Option<&str>,
    generated_at: DateTime<Utc>,
    today: NaiveDate,
) -> Result<Value, OperationError> {
    review_prompts::refresh(tenant, person.id, today).await?;
    let mut query = review_prompt::Entity::find()
        .filter(review_prompt::Column::HouseholdId.eq(person.household_id))
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
    let prompts: Vec<Value> = rows.iter().map(review_prompts::value).collect();
    Ok(json!({
        "person": {"id": person.id.to_string(), "name": person.name},
        "generated_at": generated_at.to_rfc3339(),
        "prompts": prompts,
    }))
}

pub fn medication_review_pdf(mut data: Value, people: &[person::Model], zone: Tz) -> Value {
    let names: HashMap<i64, &str> = people
        .iter()
        .map(|row| (row.id, row.name.as_str()))
        .collect();
    let mut to_discuss = 0usize;
    let mut reviewed = 0usize;
    let mut represented: HashSet<i64> = HashSet::new();
    if let Some(prompts) = data["prompts"].as_array_mut() {
        for prompt in prompts {
            if let Some(person_id) = prompt["person_id"]
                .as_str()
                .and_then(|value| value.parse::<i64>().ok())
            {
                prompt["person_name"] = json!(names.get(&person_id).copied().unwrap_or(""));
                represented.insert(person_id);
            }
            match prompt["status"].as_str() {
                Some("needs_review") => to_discuss += 1,
                Some(
                    "reviewed_with_practitioner"
                    | "expected_prescribed_combination"
                    | "not_relevant",
                ) => reviewed += 1,
                _ => {}
            }
        }
    }
    data["to_discuss"] = json!(to_discuss);
    data["reviewed"] = json!(reviewed);
    data["people_count"] = json!(represented.len());
    data["time_zone"] = json!(zone.name());
    data
}

pub async fn household_medication_reviews(
    tenant: &TenantTransaction,
    people: &[person::Model],
    status: Option<&str>,
    generated_at: DateTime<Utc>,
    today: NaiveDate,
) -> Result<Value, OperationError> {
    let ids: Vec<i64> = people.iter().map(|row| row.id).collect();
    review_prompts::refresh_many(tenant, &ids, today).await?;
    let mut query = review_prompt::Entity::find()
        .filter(review_prompt::Column::HouseholdId.eq(tenant.scope().household_id))
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
    let prompts: Vec<Value> = rows
        .iter()
        .map(|row| {
            let mut value = review_prompts::value(row);
            value["person_name"] = json!(names.get(&row.person_id).copied().unwrap_or(""));
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
    Ok(json!({
        "person": {"name": people.iter().map(|row| row.name.as_str()).collect::<Vec<_>>().join(", ")},
        "people_count": represented_people,
        "generated_at": generated_at.to_rfc3339(),
        "to_discuss": to_discuss,
        "reviewed": reviewed,
        "prompts": prompts,
    }))
}

fn inventory_alerts(
    schedules: &[schedule::Model],
    medicines: &HashMap<i64, medication::Model>,
    today: NaiveDate,
) -> Vec<Value> {
    let mut alerts: Vec<(i64, Value)> = schedules.iter().filter_map(|source| {
        if !source.active || source.retired_at.is_some() || source.start_date.is_some_and(|date| date > today) || source.end_date.is_some_and(|date| date < today) { return None; }
        let medicine = medicines.get(&source.medication_id)?;
        let supply = medicine.current_supply.unwrap_or(Decimal::ZERO);
        let first = source.start_date.map_or(today, |day| day.max(today));
        let last = source.end_date.map_or(today + Duration::days(30), |day| day.min(today + Duration::days(30)));
        if last < first { return None; }
        let days = (last - first).num_days() + 1;
        let mut consumed = Decimal::ZERO;
        for offset in 0..days {
            let day = first + Duration::days(offset);
            let expected = i64::from(dose_occurrences::expected_schedule_doses_for_report(source, day));
            if expected == 0 { continue; }
            let config = dose_occurrences::schedule_config_on(source, day).unwrap_or(&source.schedule_config);
            let amount = doses::config_decimal(config, &["amount", "dose_amount"]).or(source.dose_amount);
            let unit = doses::config_value(config, &["unit", "dose_unit"]).and_then(Value::as_str)
                .filter(|value| !value.is_empty()).or(source.dose_unit.as_deref());
            let quantity = if matches!(unit, Some("tablet" | "capsule" | "gummy" | "sachet" | "spray" | "drop" | "pad" | "ml")) { amount.unwrap_or(Decimal::ZERO) } else { Decimal::ONE };
            consumed += Decimal::from(expected) * quantity;
        }
        let burn_rate = consumed / Decimal::from(days);
        if burn_rate <= Decimal::ZERO { return None; }
        let days_left = (supply / burn_rate).floor().to_string().parse::<i64>().unwrap_or(0);
        if days_left >= 14 { return None; }
        let name = medicine.friendly_name.as_deref().filter(|name| !name.is_empty())
            .or(medicine.name.as_deref()).unwrap_or("");
        Some((days_left, json!({"medication_name":name,"days_left":days_left,"doses_left":supply.to_string().parse::<f64>().unwrap_or(0.0),"low_stock":days_left <= 3})))
    }).collect();
    alerts.sort_by_key(|(days, _)| *days);
    alerts.into_iter().take(2).map(|(_, value)| value).collect()
}

fn insight_summary(
    daily: &[Value],
    cycles: &[Value],
    takes: &[Value],
    alerts: &[Value],
    schedules: &[schedule::Model],
    medicines: &HashMap<i64, medication::Model>,
    period: InsightPeriod,
) -> Value {
    let InsightPeriod { start, end, today } = period;
    let count = |row: &Value, key: &str| row[key].as_u64().unwrap_or(0);
    let expected: u64 = daily
        .iter()
        .chain(cycles)
        .map(|row| count(row, "expected"))
        .sum();
    let logged: u64 = daily
        .iter()
        .chain(cycles)
        .map(|row| count(row, "actual") + count(row, "not_taken"))
        .sum::<u64>()
        + takes
            .iter()
            .filter(|row| row["source_type"] == "as_needed")
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
        return json!({"state":"learning","evidence_summary":evidence_summary,"insights":[]});
    }
    let mut insights = Vec::new();
    if let Some(alert) = alerts.first() {
        let name = alert["medication_name"].as_str().unwrap_or("");
        let days = alert["days_left"].as_i64().unwrap_or(0);
        insights.push(json!({"title":"Supply needs attention","summary":if days <= 0 { format!("{name} appears to be out of stock.") } else if days == 1 { format!("{name} has about 1 day of supply left.") } else { format!("{name} has about {days} days of supply left.") },"detail":format!("Check the stock record for {name} and plan the next refill."),"severity":if days <= 3 { "urgent" } else { "warning" }}));
    }
    let mut current_missed = 0;
    let mut longest_missed = 0;
    for row in daily {
        current_missed = if count(row, "unexplained_missed") > 0 {
            current_missed + 1
        } else {
            0
        };
        longest_missed = longest_missed.max(current_missed);
    }
    if longest_missed >= 2 {
        insights.push(json!({"title":"Missed-dose pattern","summary":format!("Expected doses were missed for {longest_missed} days in a row."),"detail":"Review the schedule and reminders for this window.","severity":"warning"}));
    }
    let mut cycle_groups: HashMap<i64, Vec<(NaiveDate, NaiveDate, u64)>> = HashMap::new();
    for cycle in cycles {
        let (Some(id), Some(first), Some(last)) = (
            cycle["source_id"].as_i64(),
            cycle["window_starts_on"]
                .as_str()
                .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()),
            cycle["window_ends_on"]
                .as_str()
                .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()),
        ) else {
            continue;
        };
        if last < today {
            cycle_groups.entry(id).or_default().push((
                first,
                last,
                count(cycle, "unexplained_missed"),
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
        insights.push(json!({"title":"Missed routine cycles","summary":format!("{longest_cycles} consecutive cycles have unexplained missed doses."),"detail":"Review the routine dose records for these cycles.","severity":"warning"}));
    }
    let adherence = daily
        .iter()
        .rev()
        .take_while(|row| {
            count(row, "expected") > 0 && count(row, "actual") >= count(row, "expected")
        })
        .count();
    if adherence >= 3 {
        insights.push(json!({"title":"Adherence streak","summary":format!("Every expected dose was logged for {adherence} days."),"detail":"That consistency gives your care routine a solid rhythm.","severity":"positive"}));
    }
    let mut expected_times = Vec::new();
    for source in schedules {
        let mut day = start;
        while day <= end {
            if dose_occurrences::expected_schedule_doses_for_report(source, day) > 0
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
                            && take["source_id"].as_i64() == Some(*source_id)
                            && take["taken_at"]
                                .as_str()
                                .and_then(|value| {
                                    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M")
                                        .ok()
                                })
                                .is_some_and(|at| (at - *expected_at).num_minutes().abs() <= 90)
                    })
                    .is_some_and(|(index, _)| used.insert(index))
            })
            .count();
        let percentage = (on_time as f64 / expected_times.len() as f64 * 100.0).round() as u64;
        if percentage >= 80 {
            insights.push(json!({"title":"Consistent dose timing","summary":format!("{percentage}% of logged doses were close to their scheduled time."),"detail":"Your logged timing has been steady in this report window.","severity":"positive"}));
        }
    }
    let prn = takes
        .iter()
        .filter(|row| row["source_type"] == "as_needed")
        .count();
    if prn > 0 {
        insights.push(json!({"title":"As-needed use logged","summary":format!("{prn} as-needed doses were logged in this window."),"detail":"Review these entries alongside routine doses to keep the full picture clear.","severity":"info"}));
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
        insights.push(json!({"title":"Schedule detail to review","summary":format!("{name} is set for multiple daily doses without specific times."),"detail":"Adding times can make reminders, reporting, and timing insights clearer.","severity":"info"}));
    }
    json!({"state":if insights.is_empty() { "no_action" } else { "insights" },"evidence_summary":evidence_summary,"insights":insights})
}

pub async fn ordinary_history(
    tenant: &TenantTransaction,
    people: &[person::Model],
    start: NaiveDate,
    end: NaiveDate,
    generated_at: DateTime<Utc>,
    zone: Tz,
) -> Result<Value, OperationError> {
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
        .filter(schedule::Column::HouseholdId.eq(household_id))
        .filter(schedule::Column::PersonId.is_in(ids.clone()))
        .all(db)
        .await?;
    let assignments = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(household_id))
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
        .filter(medication::Column::HouseholdId.eq(household_id))
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
        .filter(medication_take::Column::HouseholdId.eq(household_id))
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
        take_rows.push(json!({
            "taken_at": at.format("%Y-%m-%d %H:%M").to_string(),
            "source_id": source_key.map(|(_, id)| id),
            "person_name": names.get(&person_id).copied().unwrap_or(""),
            "medication_name": medication_name(medication_id),
            "dose_amount": take.dose_amount.map(|value| value.to_string()),
            "dose_unit": take.dose_unit,
            "source_type": source_type,
            "location_name": "",
        }));
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
            not_taken.push(json!({
                "scheduled_at": row.scheduled_at.map(|at| at.and_utc().with_timezone(&zone).format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| row.window_start.to_string()),
                "person_name": names.get(&row.person_id).copied().unwrap_or(""),
                "medication_name": medication_name(row.medication_id),
                "reason": row.reason,
                "note": row.note,
            }));
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
    let cycle_summaries: Vec<Value> = cycles.into_iter().map(|((_, source_id, first, last), (person_id, medication_id, expected, actual, not_taken, missed))| json!({
        "window_starts_on": first.to_string(), "window_ends_on": last.to_string(),
        "source_id": source_id,
        "person_name": names.get(&person_id).copied().unwrap_or(""), "medication_name": medication_name(medication_id),
        "expected": expected, "actual": actual, "not_taken": not_taken, "unexplained_missed": missed,
    })).collect();
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
        daily_outcomes.push(json!({"date":day.to_string(),"day_name":day.format("%a").to_string(),"percentage":percentage,"expected":expected,"actual":actual,"not_taken":not_taken,"unexplained_missed":missed}));
        day += Duration::days(1);
    }
    let events = health_event::Entity::find()
        .filter(health_event::Column::HouseholdId.eq(household_id))
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
            .filter(health_event_medication::Column::HouseholdId.eq(household_id))
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
        let value = json!({
            "title": event.title, "started_on": event.started_on.to_string(),
            "ended_on": event.ended_on.map(|date| date.to_string()),
            "severity": event.severity.map(|level| match level { 0 => "mild", 1 => "moderate", 2 => "severe", _ => "" }),
            "notes": event.notes, "action_taken": event.action_taken,
            "medication_names": by_event.remove(&event.id).unwrap_or_default(),
        });
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
    let illness_patterns: Vec<Value> = patterns.into_iter().filter_map(|(title, mut dates)| {
        if dates.len() < 2 { return None; }
        dates.sort();
        let span = (dates.last()? .to_owned() - dates.first()? .to_owned()).num_days();
        Some(json!({"display_title":title,"episode_count":dates.len(),"first_started_on":dates.first()?.to_string(),"most_recent_started_on":dates.last()?.to_string(),"average_interval_days":span / (dates.len() as i64 - 1)}))
    }).collect();
    let inventory_alerts = inventory_alerts(&schedules, &medicines, today);
    let insights = insight_summary(
        &daily_outcomes,
        &cycle_summaries,
        &take_rows,
        &inventory_alerts,
        &schedules,
        &medicines,
        InsightPeriod { start, end, today },
    );
    let total_expected: usize = daily_outcomes
        .iter()
        .map(|row| row["expected"].as_u64().unwrap_or(0) as usize)
        .chain(
            cycle_summaries
                .iter()
                .map(|row| row["expected"].as_u64().unwrap_or(0) as usize),
        )
        .sum();
    let total_actual: usize = daily_outcomes
        .iter()
        .map(|row| row["actual"].as_u64().unwrap_or(0) as usize)
        .chain(
            cycle_summaries
                .iter()
                .map(|row| row["actual"].as_u64().unwrap_or(0) as usize),
        )
        .sum();
    let compliance = if total_expected == 0 {
        100
    } else {
        ((total_actual as f64 / total_expected as f64 * 100.0).round() as usize).min(100)
    };
    let today_taken: Vec<Value> = today_medicines.into_iter().map(|((person_name, person_id), medicines)| json!({
        "person_id":person_id,"person_name":person_name,"medications":medicines.into_iter().map(|(id,name)|json!({"id":id,"name":name})).collect::<Vec<_>>()
    })).collect();
    Ok(json!({
        "people": people.iter().map(|row| row.name.clone()).collect::<Vec<_>>(),
        "start_date": start.to_string(), "end_date": end.to_string(),
        "generated_at": generated_at.to_rfc3339(),
        "medication_takes": take_rows, "not_taken_outcomes": not_taken,
        "daily_outcomes": daily_outcomes, "cycle_summaries": cycle_summaries,
        "today_taken": today_taken, "inventory_alerts": inventory_alerts, "smart_insights": insights,
        "summary": {"expected":total_expected,"actual":total_actual,"compliance":compliance},
        "suspected_side_effects": side_effects, "notable_illnesses": illnesses,
        "illness_patterns": illness_patterns,
    }))
}
