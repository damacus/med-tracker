use super::*;
use crate::models::{
    entities::{account, active_storage_attachment, medication, person_medication, schedule},
    profile,
};
use loco_rs::model::TenantQueryExt;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use std::collections::HashSet;

pub async fn enrich(
    tenant: &TenantTransaction,
    collection: &mut Value,
    labels: &Value,
) -> Result<(), OperationError> {
    access::recheck(tenant).await?;
    let ids: Vec<i64> = collection["data"]
        .as_array()
        .ok_or(OperationError::Unavailable)?
        .iter()
        .filter_map(|row| row["id"].as_i64())
        .collect();
    if ids.is_empty() {
        return Ok(());
    }
    let db = tenant.transaction();
    let people = access::person_queries::household_people(tenant.scope())
        .filter(person::Column::Id.is_in(ids.clone()))
        .all(db)
        .await?;
    let manageable: HashSet<_> = access::people_with_access(tenant, PersonAccess::Manage)
        .await?
        .into_iter()
        .map(|row| row.id)
        .collect();
    let linked: HashSet<_> = account::Entity::find()
        .filter(account::Column::PersonId.is_in(ids.clone()))
        .all(db)
        .await?
        .into_iter()
        .map(|row| row.person_id)
        .collect();
    let mut uploaded: HashSet<_> = active_storage_attachment::Entity::find()
        .filter(active_storage_attachment::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(active_storage_attachment::Column::RecordType.eq("Person"))
        .filter(active_storage_attachment::Column::Name.eq("avatar"))
        .filter(
            active_storage_attachment::Column::RecordId.is_in(
                ids.iter()
                    .filter(|id| !linked.contains(id))
                    .copied()
                    .collect::<Vec<_>>(),
            ),
        )
        .all(db)
        .await?
        .into_iter()
        .map(|row| row.record_id)
        .collect();
    for row in db.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT p.id FROM people p JOIN accounts owner ON owner.person_id=p.id JOIN account_avatars a ON a.account_id=owner.id WHERE p.id=ANY($1::bigint[])",
        [ids.clone().into()])).await? {
        uploaded.insert(row.try_get("", "id")?);
    }
    let gravatars = profile::gravatar::for_visible_people(tenant, &people, &uploaded).await?;
    let schedules = access::schedule_scope(tenant)
        .filter(schedule::Column::PersonId.is_in(ids.clone()))
        .all(db)
        .await?;
    let assignments = person_medication::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(person_medication::Column::PersonId.is_in(ids))
        .filter(person_medication::Column::RetiredAt.is_null())
        .all(db)
        .await?;
    let mut care =
        super::super::treatments::projection::serialize_schedules(db, tenant, schedules).await?;
    care.extend(
        super::super::treatments::projection::serialize_assignments(db, tenant, assignments)
            .await?,
    );
    let visible_medicines: HashSet<_> = access::medication_scope(tenant)
        .filter(
            medication::Column::Id.is_in(
                care.iter()
                    .filter_map(|row| row["medication_id"].as_i64())
                    .collect::<Vec<_>>(),
            ),
        )
        .all(db)
        .await?
        .into_iter()
        .map(|row| row.id)
        .collect();
    let mut counts: HashMap<i64, usize> = HashMap::new();
    for row in care {
        if row["active"] == true
            && row["paused"] != true
            && row["medication_id"]
                .as_i64()
                .is_some_and(|id| visible_medicines.contains(&id))
            && let Some(id) = row["person_id"].as_i64()
        {
            *counts.entry(id).or_default() += 1;
        }
    }
    if let Some(rows) = collection["data"].as_array_mut() {
        for row in rows {
            let id = row["id"].as_i64().ok_or(OperationError::Unavailable)?;
            let count = counts.get(&id).copied().unwrap_or_default();
            row["active_medication_count"] = json!(count);
            row["active_medication_label"] = if count == 0 {
                labels["people"]["card"]["no_active_medications"].clone()
            } else {
                json!(
                    labels["people"]["index"]["active_medications"]
                        [if count == 1 { "one" } else { "other" }]
                    .as_str()
                    .unwrap_or_default()
                    .replace("%{count}", &count.to_string())
                )
            };
            row["can_manage"] = json!(manageable.contains(&id));
            row["avatar_attached"] = json!(uploaded.contains(&id));
            row["gravatar_url"] = json!(gravatars.get(&id));
            row["avatar_initials"] = json!(
                row["name"]
                    .as_str()
                    .unwrap_or_default()
                    .split_whitespace()
                    .filter_map(|word| word.chars().next())
                    .take(2)
                    .flat_map(char::to_uppercase)
                    .collect::<String>()
            );
            row["date_of_birth_display"] = json!(birth_date(row["date_of_birth"].as_str(), labels));
        }
    }
    Ok(())
}

pub(crate) fn birth_date(value: Option<&str>, labels: &Value) -> Option<String> {
    value
        .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
        .map(|date| {
            format!(
                "{} {} {}",
                date.day(),
                labels["reports"]["medication_review"]["months"][date.month0() as usize]
                    .as_str()
                    .unwrap_or_default(),
                date.year()
            )
        })
}
