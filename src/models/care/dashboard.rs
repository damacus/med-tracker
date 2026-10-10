use crate::models::{
    access::{self, TenantTransaction},
    care::{dose_occurrences, doses, treatments},
    entities::{
        active_storage_attachment, medication, medication_take, person, person_medication, schedule,
    },
    errors::OperationError,
};
use chrono::{Timelike, Utc};
use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

pub async fn read(
    tenant: &TenantTransaction,
    zone: chrono_tz::Tz,
    selected: Option<i64>,
    slug: &str,
    labels: &Value,
) -> Result<Value, OperationError> {
    access::recheck(tenant).await?;
    let db = tenant.transaction();
    let household = tenant.scope().household_id;
    let people = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(household))
        .filter(person::Column::Id.in_subquery(access::granted_people(tenant.membership())))
        .order_by_asc(person::Column::Name)
        .all(db)
        .await?;
    if selected.is_some_and(|id| !people.iter().any(|person| person.id == id)) {
        return Err(OperationError::NotFound);
    }
    let ids: Vec<i64> = people
        .iter()
        .filter(|person| selected.is_none_or(|id| person.id == id))
        .map(|person| person.id)
        .collect();
    let attachments: HashSet<i64> = active_storage_attachment::Entity::find()
        .filter(active_storage_attachment::Column::HouseholdId.eq(household))
        .filter(active_storage_attachment::Column::RecordType.eq("Person"))
        .filter(active_storage_attachment::Column::Name.eq("avatar"))
        .filter(
            active_storage_attachment::Column::RecordId
                .is_in(people.iter().map(|person| person.id).collect::<Vec<_>>()),
        )
        .all(db)
        .await?
        .into_iter()
        .map(|row| row.record_id)
        .collect();
    let schedules = schedule::Entity::find()
        .filter(schedule::Column::HouseholdId.eq(household))
        .filter(schedule::Column::PersonId.is_in(ids.clone()))
        .filter(schedule::Column::RetiredAt.is_null())
        .all(db)
        .await?;
    let assignments = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(household))
        .filter(person_medication::Column::PersonId.is_in(ids.clone()))
        .filter(person_medication::Column::RetiredAt.is_null())
        .all(db)
        .await?;
    let as_needed_schedules: HashSet<_> = schedules
        .iter()
        .filter(|source| dose_occurrences::schedule_as_needed(source))
        .map(|source| source.id)
        .collect();
    let now = Utc::now();
    let today = now.with_timezone(&zone).date_naive();
    let recent_takes: Vec<_> = medication_take::Entity::find()
        .filter(medication_take::Column::HouseholdId.eq(household))
        .filter(
            Condition::any()
                .add(
                    medication_take::Column::ScheduleId
                        .is_in(schedules.iter().map(|row| row.id).collect::<Vec<_>>()),
                )
                .add(
                    medication_take::Column::PersonMedicationId
                        .is_in(assignments.iter().map(|row| row.id).collect::<Vec<_>>()),
                ),
        )
        .filter(medication_take::Column::TakenAt.gte((now - chrono::Duration::days(2)).naive_utc()))
        .filter(medication_take::Column::TakenAt.lte((now + chrono::Duration::days(1)).naive_utc()))
        .all(db)
        .await?
        .into_iter()
        .filter(|take| {
            take.taken_at
                .is_some_and(|time| time.and_utc().with_timezone(&zone).date_naive() == today)
        })
        .collect();
    let blocked = doses::dashboard_timing(tenant, &schedules, &assignments, now, zone).await?;
    let mut cards = treatments::projection::serialize_schedules(db, tenant, schedules).await?;
    for card in &mut cards {
        card["source_type"] = json!("schedule");
    }
    let mut assigned =
        treatments::projection::serialize_assignments(db, tenant, assignments).await?;
    for card in &mut assigned {
        card["source_type"] = json!("person_medication");
    }
    cards.extend(assigned);
    let medicines = access::medication_scope(tenant)
        .filter(
            medication::Column::Id.is_in(
                cards
                    .iter()
                    .filter_map(|card| card["medication_id"].as_i64())
                    .collect::<Vec<_>>(),
            ),
        )
        .all(db)
        .await?;
    let stock = treatments::projection::dashboard_stock(tenant, &cards, &medicines).await?;
    let medicine_by_id: HashMap<_, _> = medicines
        .iter()
        .map(|medicine| (medicine.id, medicine))
        .collect();
    let person_by_id: HashMap<_, _> = people.iter().map(|person| (person.id, person)).collect();
    let projected = dose_occurrences::with_dashboard_timezone(
        zone,
        dose_occurrences::projected_for_dashboard(tenant, &ids, today),
    )
    .await?;
    let mut rows = Vec::new();
    for card in cards {
        let Some(person_id) = card["person_id"].as_i64() else {
            continue;
        };
        let Some(person) = person_by_id.get(&person_id) else {
            continue;
        };
        let Some(medicine) = card["medication_id"]
            .as_i64()
            .and_then(|id| medicine_by_id.get(&id))
        else {
            continue;
        };
        let kind = card["source_type"].as_str().unwrap_or_default();
        let source_id = card["id"].as_i64().unwrap_or_default();
        let source_rows: Vec<_> = projected
            .iter()
            .filter(|row| row.source_type == kind && row.source_id == source_id)
            .collect();
        let as_needed = card["administration_kind"] == "as_needed"
            || as_needed_schedules.contains(&source_id) && kind == "schedule";
        let mut slots = Vec::new();
        if as_needed {
            if card["active"] == true {
                slots.push(("open", None, None));
            }
        } else if let Some(row) = source_rows.iter().find(|row| row.outcome == "open") {
            slots.push(("open", row.scheduled_at, None));
        }
        slots.extend(
            source_rows
                .iter()
                .filter(|row| row.outcome != "open" && row.outcome != "taken")
                .map(|row| (row.outcome.as_str(), row.scheduled_at, None)),
        );
        slots.extend(
            recent_takes
                .iter()
                .filter(|take| {
                    if kind == "schedule" {
                        take.schedule_id == Some(source_id)
                    } else {
                        take.person_medication_id == Some(source_id)
                    }
                })
                .map(|take| ("taken", take.taken_at, Some(take))),
        );
        for (outcome, time, take) in slots {
            let scheduled = time.map(|time| time.and_utc().with_timezone(&zone));
            let status = if outcome != "open" {
                outcome
            } else if card["paused"] == true {
                "paused"
            } else if !stock
                .get(&(kind.to_owned(), source_id))
                .copied()
                .unwrap_or(false)
            {
                "out_of_stock"
            } else if let Some(reason) = blocked.get(&(person_id, medicine.id)) {
                reason
            } else if scheduled.is_some_and(|time| time.with_timezone(&Utc) > now) {
                "upcoming"
            } else {
                "available"
            };
            let name = medicine
                .friendly_name
                .as_deref()
                .filter(|name| !name.is_empty())
                .or(medicine.name.as_deref())
                .unwrap_or("");
            let amount = take
                .and_then(|take| take.dose_amount)
                .map(|amount| amount.normalize().to_string())
                .unwrap_or_else(|| card["dose_amount"].as_str().unwrap_or("").to_owned());
            let unit = take
                .and_then(|take| take.dose_unit.as_deref())
                .unwrap_or_else(|| card["dose_unit"].as_str().unwrap_or(""));
            let period = scheduled.map_or("anytime", |time| match time.hour() {
                0..=11 => "morning",
                12..=17 => "afternoon",
                _ => "evening",
            });
            rows.push(json!({"person_id":person_id,"person_name":person.name,"avatar":attachments.contains(&person_id),"initials":person.name.split_whitespace().filter_map(|word| word.chars().next()).take(2).flat_map(char::to_uppercase).collect::<String>(),"medication":name,"icon":match medicine.dose_unit.as_deref().unwrap_or("").to_ascii_lowercase().as_str() { "tablet"|"capsule"|"gummy"|"pill"=>"pill", "ml"|"drop"|"spray"=>"droplet", "iu"=>"syringe", _=>"medication" },"dose":format!("{} {}",amount, dose_unit(unit,&amount)),"source_id":source_id,"source_type":kind,"medication_id":medicine.id,"stock_ids":card["eligible_stock_medication_ids"],"as_needed":as_needed,"can_record_outcome":!as_needed && card["can_record"] == true && outcome == "open" && scheduled.is_none_or(|time|time.with_timezone(&Utc) <= now),"outcome":outcome,"status":status,"status_label":match status { "available" => &labels["dashboard"]["stats"]["now"], "paused" => &labels["person_medications"]["card"]["paused"], "not_taken" => &labels["dashboard"]["outcomes"]["not_taken"], _ => &labels["dashboard"]["statuses"][status] },"period":period,"time":scheduled.map(|time| time.format("%H:%M").to_string()).unwrap_or_else(|| labels["dashboard"]["routine"]["anytime"].as_str().unwrap_or("").to_owned()),"order":scheduled.map(|time| time.timestamp()).unwrap_or(i64::MAX),"action_label":labels["person_medications"]["card"][if person.account_id == Some(tenant.scope().actor.account_id) { "take" } else { "give" }],"can_record":card["can_record"] == true && matches!(status,"available"|"upcoming"),"record_url":format!("/households/{slug}/dashboard?record_medication_id={}&record_source_type={kind}&record_source_id={source_id}&record_person_id={person_id}",medicine.id),"person_url":format!("/households/{slug}/people/{person_id}"),"history_url":format!("/households/{slug}/people/{person_id}/treatments/{}/{source_id}/doses",if kind == "schedule" { "schedules" } else { "assignments" })}));
        }
    }
    rows.sort_by_key(|row| {
        (
            row["order"].as_i64().unwrap_or(i64::MAX),
            row["person_id"].as_i64(),
            row["source_id"].as_i64(),
        )
    });
    let mut actionable: Vec<_> = rows
        .iter()
        .filter(|row| matches!(row["status"].as_str(), Some("available" | "upcoming")))
        .cloned()
        .collect();
    actionable.sort_by_key(|row| {
        (
            row["status"] != "available",
            row["order"].as_i64().unwrap_or(i64::MAX),
            row["person_id"].as_i64(),
            row["source_id"].as_i64(),
        )
    });
    let routine: Vec<_> = rows
        .iter()
        .filter(|row| {
            row["as_needed"] == false && row["outcome"] == "open" && row["status"] != "max_reached"
        })
        .collect();
    let completed: Vec<_> = rows
        .iter()
        .filter(|row| row["outcome"] == "taken")
        .cloned()
        .collect();
    let lanes: Vec<_> = people.iter().filter(|person| ids.contains(&person.id)).map(|person| json!({"id":person.id,"name":person.name,"avatar":attachments.contains(&person.id),"initials":person.name.split_whitespace().filter_map(|word| word.chars().next()).take(2).flat_map(char::to_uppercase).collect::<String>(),"rows":rows.iter().filter(|row| row["person_id"] == person.id).collect::<Vec<_>>(),"has_as_needed":rows.iter().any(|row|row["person_id"] == person.id && row["as_needed"] == true && row["outcome"] == "open"),"remaining":routine.iter().filter(|row|row["person_id"] == person.id).count(),"routine_summary":if routine.iter().any(|row|row["person_id"] == person.id) {labels["dashboard"]["routine"]["left_today"]["other"].as_str().unwrap_or("").replace("%{count}",&routine.iter().filter(|row|row["person_id"] == person.id).count().to_string())} else {labels["dashboard"]["routine"]["done_today"].as_str().unwrap_or("").to_owned()}})).collect();
    let inventory: Vec<_> = medicines.iter().map(|medicine| json!({"id":medicine.id,"name":medicine.friendly_name.as_deref().filter(|name| !name.is_empty()).or(medicine.name.as_deref()).unwrap_or(""),"supply":medicine.current_supply.map(|value| format!("{} {}",value.normalize(), if medicine.dose_unit.as_deref() == Some("ml") { "ml" } else if value == sea_orm::prelude::Decimal::ONE { "unit" } else { "units" })),"percentage":medicine.current_supply.zip(medicine.supply_at_last_restock).filter(|(_,baseline)|*baseline > sea_orm::prelude::Decimal::ZERO).map(|(supply,baseline)|((supply/baseline)*sea_orm::prelude::Decimal::from(100)).clamp(sea_orm::prelude::Decimal::ZERO,sea_orm::prelude::Decimal::from(100)).to_string()).unwrap_or_else(||"100".to_owned()),"low":medicine.current_supply.is_some_and(|value| value <= medicine.reorder_threshold)})).collect();
    let stock = inventory
        .iter()
        .find(|medicine| medicine["low"] == true)
        .or_else(|| inventory.first());
    Ok(
        json!({"periods":(["morning","afternoon","evening","anytime"].into_iter().filter(|period| rows.iter().any(|row| row["period"] == *period)).collect::<Vec<_>>()),"people":people.iter().map(|person| json!({"id":person.id,"name":person.name,"initials":initials(&person.name)})).collect::<Vec<_>>(),"selected":selected,"selected_name":selected.and_then(|id| person_by_id.get(&id)).map(|person| json!(person.name)).unwrap_or_else(|| labels["dashboard"]["person_selector"]["all_family"].clone()),"selected_initials":selected.and_then(|id|person_by_id.get(&id)).map(|person|json!(initials(&person.name))).unwrap_or_else(||labels["dashboard"]["person_selector"]["all_family_initials"].clone()),"rows":rows,"lanes":lanes,"next":actionable.first(),"following":actionable.iter().skip(1).take(4).collect::<Vec<_>>(),"completed":completed,"inventory":inventory,"stock":stock,"tasks_left":routine.len(),"due_now":routine.iter().filter(|row| row["status"] == "available").count(),"next_due":if routine.iter().any(|row| row["status"] == "available") { labels["dashboard"]["stats"]["now"].clone() } else { routine.iter().filter(|row| row["status"] == "upcoming").filter_map(|row| row["time"].as_str()).min().map(|value| json!(value)).unwrap_or_else(|| labels["dashboard"]["stats"]["no_upcoming_doses"].clone()) },"date":today.format("%A, %b %-d").to_string()}),
    )
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .flat_map(char::to_uppercase)
        .collect()
}

fn dose_unit(unit: &str, amount: &str) -> String {
    if amount != "1" && matches!(unit, "tablet" | "capsule" | "puff" | "drop") {
        format!("{unit}s")
    } else {
        unit.to_owned()
    }
}
