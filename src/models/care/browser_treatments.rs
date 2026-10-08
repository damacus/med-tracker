use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::{people, treatments::projection},
    entities::{dosage, medication, pause_period, person, person_medication, schedule},
    errors::OperationError,
};
use chrono::NaiveDate;
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};
use serde_json::{Value, json};
use std::collections::HashMap;

pub async fn person_cards(
    tenant: &TenantTransaction,
    person_id: i64,
    zone: chrono_tz::Tz,
) -> Result<Vec<Value>, OperationError> {
    access::require_person_access(tenant, person_id, PersonAccess::View).await?;
    let schedules = access::schedule_scope(tenant)
        .filter(schedule::Column::PersonId.eq(person_id))
        .order_by_asc(schedule::Column::Id)
        .all(tenant.transaction())
        .await?;
    let assignments = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(person_medication::Column::PersonId.eq(person_id))
        .filter(person_medication::Column::RetiredAt.is_null())
        .order_by_asc(person_medication::Column::Position)
        .order_by_asc(person_medication::Column::Id)
        .all(tenant.transaction())
        .await?;
    let mut cards =
        projection::serialize_schedules(tenant.transaction(), tenant, schedules).await?;
    for card in &mut cards {
        card["source_kind"] = json!("schedules");
        card["source_label"] = json!("scheduled treatment");
    }
    let mut ongoing =
        projection::serialize_assignments(tenant.transaction(), tenant, assignments).await?;
    for card in &mut ongoing {
        card["source_kind"] = json!("assignments");
        card["source_label"] = json!("ongoing medication");
    }
    cards.append(&mut ongoing);
    let medication_ids: Vec<i64> = cards
        .iter()
        .filter_map(|card| card["medication_id"].as_i64())
        .collect();
    let medications: HashMap<i64, (String, String)> = access::medication_scope(tenant)
        .filter(medication::Column::Id.is_in(medication_ids))
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|record| {
            let name = record
                .friendly_name
                .filter(|value| !value.is_empty())
                .or(record.name)
                .unwrap_or_else(|| "Medication".into());
            let supply = record.current_supply.map_or_else(
                || "Supply not tracked".to_owned(),
                |amount| {
                    let amount = amount.normalize().to_string();
                    let unit = record.dose_unit.as_deref().unwrap_or("units");
                    format!("{} in stock", amount_and_unit(&amount, unit))
                },
            );
            (record.id, (name, supply))
        })
        .collect();
    let schedule_ids: Vec<i64> = cards
        .iter()
        .filter(|card| card["source_kind"] == "schedules")
        .filter_map(|card| card["id"].as_i64())
        .collect();
    let assignment_ids: Vec<i64> = cards
        .iter()
        .filter(|card| card["source_kind"] == "assignments")
        .filter_map(|card| card["id"].as_i64())
        .collect();
    let periods = if schedule_ids.is_empty() && assignment_ids.is_empty() {
        Vec::new()
    } else {
        pause_period::Entity::find()
            .filter(pause_period::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(
                Condition::any()
                    .add(pause_period::Column::ScheduleId.is_in(schedule_ids))
                    .add(pause_period::Column::PersonMedicationId.is_in(assignment_ids)),
            )
            .order_by_desc(pause_period::Column::CreatedAt)
            .order_by_desc(pause_period::Column::Id)
            .all(tenant.transaction())
            .await?
    };
    let mut pause_history: HashMap<(bool, i64), Vec<Value>> = HashMap::new();
    for period in periods {
        let source = period
            .schedule_id
            .map(|id| (true, id))
            .or_else(|| period.person_medication_id.map(|id| (false, id)));
        if let Some(source) = source
            && period.ended_at.is_some()
        {
            pause_history.entry(source).or_default().push(json!({
                "reason": period.reason,
                "note": period.note,
                "started_at": period.started_at.map(|value| value.and_utc().with_timezone(&zone).format("%d %b %Y").to_string()),
                "started_at_iso": period.started_at.map(|value| value.and_utc().to_rfc3339()),
                "ended_at": period.ended_at.map(|value| value.and_utc().with_timezone(&zone).format("%d %b %Y").to_string()),
                "ended_at_iso": period.ended_at.map(|value| value.and_utc().to_rfc3339())
            }));
        }
    }
    cards.retain_mut(|card| {
        let Some((name, supply)) = card["medication_id"]
            .as_i64()
            .and_then(|id| medications.get(&id))
        else {
            return false;
        };
        card["medication_name"] = json!(name);
        card["stock_label"] = json!(supply);
        card["timing_summary"] = json!(timing_summary(card));
        card["dose_label"] = json!(dose_label(card));
        card["limit_summary"] = json!(limit_summary(card));
        card["date_range"] = json!(date_range(card));
        let source = (
            card["source_kind"] == "schedules",
            card["id"].as_i64().unwrap_or_default(),
        );
        card["pause_history"] = json!(pause_history.remove(&source).unwrap_or_default());
        true
    });
    cards.sort_by_key(|card| {
        (
            card["paused"].as_bool().unwrap_or(false),
            card["medication_name"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
            card["source_kind"].as_str().unwrap_or_default().to_owned(),
        )
    });
    Ok(cards)
}

fn timing_summary(card: &Value) -> Option<String> {
    let schedule = match card["schedule_type"].as_str() {
        Some("every_other_day") => "Every other day",
        Some("multiple_daily") => "Several times a day",
        Some("weekly") => "Weekly",
        Some("monthly") => "Monthly",
        Some("specific_dates") => "On specific dates",
        Some("prn") => "As needed",
        Some("tapering") => "Taper plan",
        Some(_) => "Daily",
        None if card["administration_kind"] == "as_needed" => "As needed",
        None => "",
    };
    let frequency = card["frequency"].as_str().filter(|value| !value.is_empty());
    let summary = match (schedule, frequency) {
        ("", None) => return None,
        ("", Some(frequency)) => frequency.to_owned(),
        (schedule, Some(frequency)) if !schedule.eq_ignore_ascii_case(frequency) => {
            format!("{schedule} · {frequency}")
        }
        (schedule, _) => schedule.to_owned(),
    };
    Some(summary)
}

fn dose_label(card: &Value) -> String {
    let Some(amount) = card["dose_amount"].as_str() else {
        return "Dose not set".into();
    };
    let amount = display_decimal(amount);
    let Some(unit) = card["dose_unit"].as_str() else {
        return amount.into();
    };
    amount_and_unit(amount, unit)
}

fn amount_and_unit(amount: &str, unit: &str) -> String {
    let countable = matches!(
        unit,
        "tablet" | "capsule" | "drop" | "puff" | "patch" | "spray" | "unit"
    );
    let suffix = if amount != "1" && countable { "s" } else { "" };
    format!("{amount} {unit}{suffix}")
}

fn date_range(card: &Value) -> Option<String> {
    let start = card["start_date"].as_str().map(display_date);
    let end = card["end_date"].as_str().map(display_date);
    match (start, end) {
        (Some(start), Some(end)) => Some(format!("{start} to {end}")),
        (Some(start), None) => Some(format!("From {start}")),
        (None, Some(end)) => Some(format!("Until {end}")),
        (None, None) => None,
    }
}

fn display_date(value: &str) -> String {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_or_else(
        |_| value.to_owned(),
        |date| date.format("%d %b %Y").to_string(),
    )
}

fn display_decimal(value: &str) -> &str {
    value.strip_suffix(".0").unwrap_or(value)
}

fn limit_summary(card: &Value) -> Option<String> {
    let maximum = card["max_daily_doses"].as_i64();
    let spacing = card["min_hours_between_doses"]
        .as_str()
        .filter(|value| !matches!(*value, "0" | "0.0"));
    if maximum.is_none_or(|value| value <= 0) && spacing.is_none() {
        return None;
    }
    let mut parts = Vec::new();
    if let Some(maximum) = maximum.filter(|value| *value > 0) {
        let period = match card["dose_cycle"].as_str() {
            Some("weekly") => "week",
            Some("monthly") => "month",
            _ => "day",
        };
        let noun = if maximum == 1 { "dose" } else { "doses" };
        parts.push(format!("Maximum {maximum} {noun} a {period}"));
    }
    if let Some(spacing) = spacing {
        let spacing = display_decimal(spacing);
        let noun = if spacing == "1" { "hour" } else { "hours" };
        parts.push(format!("At least {spacing} {noun} between doses"));
    }
    Some(parts.join(" · "))
}

pub async fn index(
    tenant: &TenantTransaction,
    id: &str,
    page: i64,
    zone: chrono_tz::Tz,
) -> Result<Value, OperationError> {
    if page < 1 {
        return Err(OperationError::Validation {
            details: json!({"errors":{"page":["must be positive"]}}),
        });
    }
    let (person_data, _) = people::read(tenant, id, zone).await?;
    let person_id = person_data["data"]["id"]
        .as_str()
        .and_then(|value| value.parse::<i64>().ok())
        .or_else(|| person_data["data"]["id"].as_i64())
        .ok_or(OperationError::Unavailable)?;
    let schedule_query =
        access::schedule_scope(tenant).filter(schedule::Column::PersonId.eq(person_id));
    let assignment_query = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(person_medication::Column::PersonId.eq(person_id))
        .filter(person_medication::Column::RetiredAt.is_null());
    let schedule_count = schedule_query.clone().count(tenant.transaction()).await?;
    let assignment_count = assignment_query.clone().count(tenant.transaction()).await?;
    let offset = page.saturating_sub(1).saturating_mul(20) as u64;
    let schedules = schedule_query
        .order_by_asc(schedule::Column::Id)
        .limit(20)
        .offset(offset)
        .all(tenant.transaction())
        .await?;
    let assignments = assignment_query
        .order_by_asc(person_medication::Column::Id)
        .limit(20)
        .offset(offset)
        .all(tenant.transaction())
        .await?;
    let schedules =
        projection::serialize_schedules(tenant.transaction(), tenant, schedules).await?;
    let assignments =
        projection::serialize_assignments(tenant.transaction(), tenant, assignments).await?;
    let can_manage = access::can_access_person(tenant, person_id, PersonAccess::Manage).await?;
    let medications = super::browser_query::index(tenant).await?;
    Ok(
        json!({"person":person_data["data"],"schedules":schedules,"assignments":assignments,"medications":medications,"can_manage":can_manage,"page":page,"has_next":schedule_count.max(assignment_count)>(page as u64).saturating_mul(20)}),
    )
}

pub async fn options(tenant: &TenantTransaction, person_id: &str) -> Result<Value, OperationError> {
    access::recheck(tenant).await?;
    let query =
        person::Entity::find().filter(person::Column::HouseholdId.eq(tenant.scope().household_id));
    let query = if let Ok(id) = person_id.parse::<i64>() {
        query.filter(person::Column::Id.eq(id))
    } else {
        query.filter(person::Column::PortableId.eq(person_id))
    };
    let row = query
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    access::require_person_access(tenant, row.id, PersonAccess::Manage).await?;
    let medications = access::medication_scope(tenant)
        .order_by_asc(medication::Column::Name)
        .all(tenant.transaction())
        .await?;
    let ids: Vec<i64> = medications.iter().map(|row| row.id).collect();
    let dosages = dosage::Entity::find()
        .filter(dosage::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(dosage::Column::MedicationId.is_in(ids))
        .order_by_asc(dosage::Column::Id)
        .all(tenant.transaction())
        .await?;
    Ok(
        json!({"person_id":row.id,"person_name":row.name,"medications":medications.iter().map(|row| json!({"id":row.id,"name":row.friendly_name.as_deref().filter(|name| !name.is_empty()).or(row.name.as_deref()).unwrap_or("Medication")})).collect::<Vec<_>>(),"dosages":dosages.iter().map(|row|json!({"id":row.id,"medication_id":row.medication_id,"amount":row.amount.normalize().to_string(),"unit":row.unit})).collect::<Vec<_>>()}),
    )
}

pub async fn source_option(
    tenant: &TenantTransaction,
    is_schedule: bool,
    id: i64,
) -> Result<Option<i64>, OperationError> {
    if is_schedule {
        Ok(schedule::Entity::find_by_id(id)
            .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::NotFound)?
            .source_dosage_option_id)
    } else {
        Ok(person_medication::Entity::find_by_id(id)
            .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::NotFound)?
            .source_dosage_option_id)
    }
}
