use crate::models::{
    access::{self, TenantTransaction},
    care::{dose_occurrences, doses, treatments},
    entities::{
        account, active_storage_attachment, medication, medication_take, person, person_medication,
        schedule,
    },
    errors::OperationError,
    profile,
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
    super::person_schedules::projection::refresh(tenant).await?;
    let db = tenant.transaction();
    let household = tenant.scope().household_id;
    let people = access::person_queries::household_people(tenant.scope())
        .filter(person::Column::Id.in_subquery(access::participating_people(tenant.membership())))
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
    let linked_people = account::Entity::find()
        .filter(account::Column::PersonId.is_in(people.iter().map(|person| person.id)))
        .all(db)
        .await?
        .into_iter()
        .map(|account| account.person_id)
        .collect::<HashSet<_>>();
    let mut attachments: HashSet<i64> = active_storage_attachment::Entity::find()
        .filter(active_storage_attachment::Column::HouseholdId.eq(household))
        .filter(active_storage_attachment::Column::RecordType.eq("Person"))
        .filter(active_storage_attachment::Column::Name.eq("avatar"))
        .filter(
            active_storage_attachment::Column::RecordId.is_in(
                people
                    .iter()
                    .filter(|person| !linked_people.contains(&person.id))
                    .map(|person| person.id)
                    .collect::<Vec<_>>(),
            ),
        )
        .all(db)
        .await?
        .into_iter()
        .map(|row| row.record_id)
        .collect();
    attachments.extend(
        profile::avatar::account_avatar_people(
            tenant,
            &people.iter().map(|person| person.id).collect::<Vec<_>>(),
        )
        .await?,
    );
    let gravatars = profile::gravatar::for_visible_people(tenant, &people, &attachments).await?;
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
    dose_occurrences::dashboard_capacity(recent_takes.len())?;
    let recent_take_ids: HashSet<_> = recent_takes.iter().map(|take| take.id).collect();
    let blocked = doses::dashboard_timing(tenant, &schedules, &assignments, now, zone).await?;
    let previews: HashMap<_, _> = schedules
        .iter()
        .map(|record| {
            (
                record.id,
                doses::browser_schedule_preview(
                    record.clone(),
                    now.with_timezone(&crate::models::care::person_schedules::execution_zone(
                        record, zone,
                    ))
                    .date_naive(),
                ),
            )
        })
        .collect();
    let mut cards = treatments::projection::serialize_schedules(db, tenant, schedules).await?;
    for card in &mut cards {
        card["source_type"] = json!("schedule");
        if let Some(preview) = card["id"].as_i64().and_then(|id| previews.get(&id)) {
            card["active"] = json!(preview.available);
            card["dose_amount"] = json!(preview.amount);
            card["dose_unit"] = json!(preview.unit);
        }
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
                slots.push(("open", None, None, None));
            }
        } else {
            slots.extend(source_rows.iter().map(|row| {
                (
                    row.outcome.as_str(),
                    row.scheduled_at,
                    row.medication_take.as_ref(),
                    Some(*row),
                )
            }));
        }
        let represented_takes: HashSet<_> = source_rows
            .iter()
            .filter_map(|row| row.medication_take.as_ref().map(|take| take.id))
            .collect();
        slots.extend(
            recent_takes
                .iter()
                .filter(|take| {
                    !represented_takes.contains(&take.id)
                        && if kind == "schedule" {
                            take.schedule_id == Some(source_id)
                        } else {
                            take.person_medication_id == Some(source_id)
                        }
                })
                .map(|take| ("taken", None, Some(take), None)),
        );
        for (outcome, scheduled_at, take, occurrence) in slots {
            dose_occurrences::dashboard_capacity(rows.len().saturating_add(1))?;
            let actual_taken_at = take.and_then(|take| take.taken_at);
            let scheduled = scheduled_at.map(|time| time.and_utc().with_timezone(&zone));
            let display_time = scheduled_at
                .or(actual_taken_at)
                .map(|time| time.and_utc().with_timezone(&zone));
            let status = if outcome != "open" {
                outcome
            } else if occurrence.is_some_and(|row| !row.expected) {
                "unavailable"
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
                .or_else(|| {
                    occurrence
                        .and_then(|row| row.dose.as_ref())
                        .and_then(|dose| dose.amount.clone())
                })
                .unwrap_or_else(|| card["dose_amount"].as_str().unwrap_or("").to_owned());
            let unit = take
                .and_then(|take| take.dose_unit.as_deref())
                .or_else(|| {
                    occurrence
                        .and_then(|row| row.dose.as_ref())
                        .and_then(|dose| dose.unit.as_deref())
                })
                .unwrap_or_else(|| card["dose_unit"].as_str().unwrap_or(""));
            let period = display_time.map_or("anytime", |time| match time.hour() {
                0..=11 => "morning",
                12..=17 => "afternoon",
                _ => "evening",
            });
            let occurrence_key = occurrence.map(|row| row.occurrence_key.as_str());
            let occurrence_etag = occurrence.and_then(|row| row.occurrence_etag.as_deref());
            let occurrence_actionable = occurrence.is_none_or(|row| row.actionable);
            let id = occurrence_key.map(str::to_owned).unwrap_or_else(|| {
                take.map(|take| format!("take-{}", take.id))
                    .unwrap_or_else(|| format!("{kind}-{source_id}-as-needed"))
            });
            let mut record_query = url::form_urlencoded::Serializer::new(String::new());
            record_query
                .append_pair("record_medication_id", &medicine.id.to_string())
                .append_pair("record_source_type", kind)
                .append_pair("record_source_id", &source_id.to_string())
                .append_pair("record_person_id", &person_id.to_string());
            if let Some(key) = occurrence_key {
                record_query.append_pair("record_occurrence_key", key);
            }
            if let Some(etag) = occurrence_etag {
                record_query.append_pair("record_occurrence_etag", etag);
            }
            let record_url = format!("/households/{slug}/dashboard?{}", record_query.finish());
            let mut history_url = format!(
                "/households/{slug}/people/{person_id}/treatments/{}/{source_id}/doses",
                if kind == "schedule" {
                    "schedules"
                } else {
                    "assignments"
                }
            );
            if let Some(occurrence) = occurrence {
                history_url.push_str(&format!(
                    "?start_date={}&end_date={}#dose-occurrence-{}",
                    occurrence.window_start, occurrence.window_end, occurrence.occurrence_key
                ));
            }
            rows.push(json!({"id":id,"medication_take_id":take.map(|take|take.id),"position":occurrence.map(|row|row.position),"occurrence_key":occurrence_key,"occurrence_etag":occurrence_etag,"scheduled_at":scheduled_at.map(|time|time.and_utc().to_rfc3339()),"actual_taken_at":actual_taken_at.map(|time|time.and_utc().to_rfc3339()),"actual_time":actual_taken_at.map(|time|time.and_utc().with_timezone(&zone).format("%H:%M").to_string()),"person_id":person_id,"person_name":person.name,"avatar":attachments.contains(&person_id),"initials":person.name.split_whitespace().filter_map(|word| word.chars().next()).take(2).flat_map(char::to_uppercase).collect::<String>(),"medication":name,"icon":match medicine.dose_unit.as_deref().unwrap_or("").to_ascii_lowercase().as_str() { "tablet"|"capsule"|"gummy"|"pill"=>"pill", "ml"|"drop"|"spray"=>"droplet", "iu"=>"syringe", _=>"medication" },"dose":format!("{} {}",amount, dose_unit(unit,&amount)),"source_id":source_id,"source_type":kind,"medication_id":medicine.id,"stock_ids":card["eligible_stock_medication_ids"],"as_needed":as_needed,"can_record_outcome":occurrence_actionable && !as_needed && card["can_record"] == true && outcome == "open" && scheduled.is_none_or(|time|time.with_timezone(&Utc) <= now),"outcome":outcome,"status":status,"status_label":match status { "available" => &labels["dashboard"]["stats"]["now"], "unavailable" => &labels["dose_records"]["unavailable"], "paused" => &labels["person_medications"]["card"]["paused"], "not_taken" => &labels["dashboard"]["outcomes"]["not_taken"], _ => &labels["dashboard"]["statuses"][status] },"period":period,"time":display_time.map(|time| time.format("%H:%M").to_string()).unwrap_or_else(|| labels["dashboard"]["routine"]["anytime"].as_str().unwrap_or("").to_owned()),"order":display_time.map(|time| time.timestamp()).unwrap_or(i64::MAX),"action_label":labels["person_medications"]["card"][if person.id == tenant.membership().person_id { "take" } else { "give" }],"can_record":occurrence_actionable && card["can_record"] == true && status == "available","record_url":record_url,"person_url":format!("/households/{slug}/people/{person_id}"),"history_url":history_url}));
        }
    }
    for row in &mut rows {
        if let Some(person_id) = row["person_id"].as_i64() {
            row["gravatar_url"] = json!(gravatars.get(&person_id));
        }
    }
    rows.sort_by(|left, right| timeline_order(left).cmp(&timeline_order(right)));
    let next_action = rows
        .iter()
        .find(|row| row["as_needed"] == false && row["can_record"] == true)
        .or_else(|| {
            rows.iter()
                .find(|row| row["as_needed"] == false && row["status"] == "upcoming")
        })
        .cloned()
        .unwrap_or(Value::Null);
    let mut actionable: Vec<_> = rows
        .iter()
        .filter(|row| matches!(row["status"].as_str(), Some("available" | "upcoming")))
        .cloned()
        .collect();
    actionable.sort_by(|left, right| {
        (left["status"] != "available", timeline_order(left))
            .cmp(&(right["status"] != "available", timeline_order(right)))
    });
    let routine: Vec<_> = rows
        .iter()
        .filter(|row| {
            row["as_needed"] == false && row["outcome"] == "open" && row["status"] != "max_reached"
        })
        .collect();
    let completed: Vec<_> = rows
        .iter()
        .filter(|row| {
            row["outcome"] == "taken"
                && row["medication_take_id"]
                    .as_i64()
                    .is_some_and(|id| recent_take_ids.contains(&id))
        })
        .cloned()
        .collect();
    let mut lanes: Vec<_> = people.iter().filter(|person| ids.contains(&person.id)).map(|person| json!({"id":person.id,"name":person.name,"avatar":attachments.contains(&person.id),"initials":person.name.split_whitespace().filter_map(|word| word.chars().next()).take(2).flat_map(char::to_uppercase).collect::<String>(),"rows":rows.iter().filter(|row| row["person_id"] == person.id).collect::<Vec<_>>(),"has_as_needed":rows.iter().any(|row|row["person_id"] == person.id && row["as_needed"] == true && row["outcome"] == "open"),"remaining":routine.iter().filter(|row|row["person_id"] == person.id).count(),"routine_summary":if routine.iter().any(|row|row["person_id"] == person.id) {labels["dashboard"]["routine"]["left_today"]["other"].as_str().unwrap_or("").replace("%{count}",&routine.iter().filter(|row|row["person_id"] == person.id).count().to_string())} else {labels["dashboard"]["routine"]["done_today"].as_str().unwrap_or("").to_owned()}})).collect();
    for lane in &mut lanes {
        if let Some(person_id) = lane["id"].as_i64() {
            lane["gravatar_url"] = json!(gravatars.get(&person_id));
        }
    }
    let inventory: Vec<_> = medicines.iter().map(|medicine| json!({"id":medicine.id,"name":medicine.friendly_name.as_deref().filter(|name| !name.is_empty()).or(medicine.name.as_deref()).unwrap_or(""),"supply":medicine.current_supply.map(|value| format!("{} {}",value.normalize(), if medicine.dose_unit.as_deref() == Some("ml") { "ml" } else if value == sea_orm::prelude::Decimal::ONE { "unit" } else { "units" })),"percentage":medicine.current_supply.zip(medicine.supply_at_last_restock).filter(|(_,baseline)|*baseline > sea_orm::prelude::Decimal::ZERO).map(|(supply,baseline)|((supply/baseline)*sea_orm::prelude::Decimal::from(100)).clamp(sea_orm::prelude::Decimal::ZERO,sea_orm::prelude::Decimal::from(100)).to_string()).unwrap_or_else(||"100".to_owned()),"low":medicine.current_supply.is_some_and(|value| value <= medicine.reorder_threshold)})).collect();
    let stock = inventory
        .iter()
        .find(|medicine| medicine["low"] == true)
        .or_else(|| inventory.first());
    let mut projection = json!({"periods":(["morning","afternoon","evening","anytime"].into_iter().filter(|period| rows.iter().any(|row| row["period"] == *period)).collect::<Vec<_>>()),"people":people.iter().map(|person| json!({"id":person.id,"name":person.name,"initials":initials(&person.name),"avatar":attachments.contains(&person.id),"gravatar_url":gravatars.get(&person.id)})).collect::<Vec<_>>(),"selected":selected,"selected_name":selected.and_then(|id| person_by_id.get(&id)).map(|person| json!(person.name)).unwrap_or_else(|| labels["dashboard"]["person_selector"]["all_family"].clone()),"selected_initials":selected.and_then(|id|person_by_id.get(&id)).map(|person|json!(initials(&person.name))).unwrap_or_else(||labels["dashboard"]["person_selector"]["all_family_initials"].clone()),"rows":rows,"lanes":lanes,"next":actionable.first(),"following":actionable.iter().skip(1).take(4).collect::<Vec<_>>(),"completed":completed,"inventory":inventory,"stock":stock,"tasks_left":routine.len(),"due_now":routine.iter().filter(|row| row["status"] == "available").count(),"next_due":if routine.iter().any(|row| row["status"] == "available") { labels["dashboard"]["stats"]["now"].clone() } else { routine.iter().filter(|row| row["status"] == "upcoming").filter_map(|row| row["time"].as_str()).min().map(|value| json!(value)).unwrap_or_else(|| labels["dashboard"]["stats"]["no_upcoming_doses"].clone()) },"date":today.format("%A, %b %-d").to_string()});
    projection["selected_avatar"] = json!(selected.is_some_and(|id| attachments.contains(&id)));
    projection["next_action"] = next_action;
    projection["selected_gravatar_url"] = json!(selected.and_then(|id| gravatars.get(&id)));
    projection["timeline_rows"] = json!(
        projection["rows"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|row| !(row["as_needed"] == true && row["outcome"] == "open"))
            .collect::<Vec<_>>()
    );
    projection["as_needed_rows"] = json!(
        projection["rows"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|row| row["as_needed"] == true && row["outcome"] == "open")
            .collect::<Vec<_>>()
    );
    projection["now_at"] = json!(now.to_rfc3339());
    projection["now_time"] = json!(now.with_timezone(&zone).format("%H:%M").to_string());
    projection["now_order"] = json!(now.timestamp());
    Ok(projection)
}

fn timeline_order(row: &Value) -> (i64, Option<i64>, &str, Option<i64>, Option<i64>, &str) {
    (
        row["order"].as_i64().unwrap_or(i64::MAX),
        row["person_id"].as_i64(),
        row["source_type"].as_str().unwrap_or_default(),
        row["source_id"].as_i64(),
        row["position"].as_i64(),
        row["id"].as_str().unwrap_or_default(),
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
