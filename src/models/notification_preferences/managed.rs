use super::*;
use crate::models::entities::security_audit_event;
use sea_orm::{ConnectionTrait, DbBackend, QueryResult, Statement};
use serde::Serialize;

#[derive(Serialize)]
pub struct Person {
    pub id: i64,
    pub name: String,
    pub adult: bool,
    pub enabled: bool,
}

async fn grants(
    tenant: &TenantTransaction,
    lock: bool,
) -> Result<Vec<QueryResult>, OperationError> {
    let mut sql = "SELECT g.id AS grant_id,p.id,p.name,p.person_type,g.missed_dose_notifications_enabled AS enabled FROM person_access_grants g JOIN people p ON p.id=g.person_id AND p.household_id=g.household_id WHERE g.household_id=$1 AND g.household_membership_id=$2 AND g.person_id<>$3 AND g.access_level='manage' AND g.revoked_at IS NULL AND (g.expires_at IS NULL OR g.expires_at>timezone('UTC',clock_timestamp())) ORDER BY p.name,p.id,g.id".to_owned();
    if lock {
        sql.push_str(" FOR UPDATE OF g");
    }
    Ok(tenant
        .transaction()
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [
                tenant.scope().household_id.into(),
                tenant.membership().id.into(),
                tenant.membership().person_id.unwrap_or(0).into(),
            ],
        ))
        .await?)
}

pub async fn read(tenant: &TenantTransaction) -> Result<Vec<Person>, OperationError> {
    let mut people: Vec<Person> = Vec::new();
    for row in grants(tenant, false).await? {
        let id = row.try_get::<i64>("", "id")?;
        let enabled = row.try_get::<bool>("", "enabled")?;
        if let Some(existing) = people.iter_mut().find(|person| person.id == id) {
            existing.enabled |= enabled;
        } else {
            people.push(Person {
                id,
                name: row.try_get("", "name")?,
                adult: row.try_get::<i32>("", "person_type")? == 0,
                enabled,
            });
        }
    }
    Ok(people)
}

pub async fn update(
    tenant: &TenantTransaction,
    account_id: i64,
    changes: Changes,
    selected: &[i64],
) -> Result<(), OperationError> {
    super::update(tenant, account_id, changes).await?;
    let rows = grants(tenant, true).await?;
    let mut adult_ids = Vec::new();
    let mut changed_grants = Vec::new();
    let mut previous = Vec::new();
    for row in rows {
        if row.try_get::<i32>("", "person_type")? != 0 {
            continue;
        }
        let id = row.try_get::<i64>("", "id")?;
        adult_ids.push(id);
        let enabled = row.try_get::<bool>("", "enabled")?;
        if enabled {
            previous.push(id);
        }
        if enabled != selected.contains(&id) {
            changed_grants.push(row.try_get::<i64>("", "grant_id")?);
        }
    }
    if selected.iter().any(|id| !adult_ids.contains(id)) {
        return Err(profile::validation(
            "managed_person_ids",
            "contains a person you cannot manage",
        ));
    }
    if changed_grants.is_empty() {
        return Ok(());
    }
    let result = tenant.transaction().execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE person_access_grants SET missed_dose_notifications_enabled=EXISTS(SELECT 1 FROM jsonb_array_elements_text($3::jsonb) chosen(value) WHERE chosen.value::bigint=person_id), updated_at=timezone('UTC',clock_timestamp()) WHERE household_id=$1 AND household_membership_id=$2 AND id IN(SELECT value::bigint FROM jsonb_array_elements_text($4::jsonb)) AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at>timezone('UTC',clock_timestamp()))",
        [tenant.scope().household_id.into(),tenant.membership().id.into(),json!(selected).into(),json!(changed_grants).into()]
    )).await?;
    if result.rows_affected() != changed_grants.len() as u64 {
        return Err(OperationError::Forbidden);
    }
    previous.sort_unstable();
    previous.dedup();
    security_audit_event::ActiveModel {
        actor_membership_id: Set(Some(tenant.membership().id)),
        actor_account_id: Set(Some(account_id)),
        household_id: Set(tenant.scope().household_id),
        event_type: Set("notification_preferences.managed_people.updated".into()),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        metadata: Set(json!({"previous_person_ids":previous,"person_ids":selected})),
        audit_context: Set(json!({"actor_membership_id":tenant.membership().id,"household_id":tenant.scope().household_id})),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }.insert(tenant.transaction()).await?;
    Ok(())
}
