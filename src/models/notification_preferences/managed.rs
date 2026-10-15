use super::*;
use crate::models::{entities::security_audit_event, ids::PersonId};
use sea_orm::{ConnectionTrait, DbBackend, QueryResult, Statement};
use serde::Serialize;

#[derive(Serialize)]
pub struct Person {
    pub id: PersonId,
    pub name: String,
    pub enabled: bool,
}

async fn participants(tenant: &TenantTransaction) -> Result<Vec<QueryResult>, OperationError> {
    Ok(tenant.transaction().query_all_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT p.id,med_tracker.person_display_name(p.id) AS name,COALESCE(s.enabled,false) AS enabled FROM household_memberships m JOIN people p ON p.id=m.person_id LEFT JOIN household_notification_subscriptions s ON s.household_id=m.household_id AND s.person_id=p.id AND s.account_id=$2 WHERE m.household_id=$1 AND m.status='active' AND m.revoked_at IS NULL AND p.id<>$3 ORDER BY name,p.id",
        [tenant.scope().household_id.into(),tenant.scope().actor.account_id.into(),tenant.person_id().get().into()],
    )).await?)
}

pub async fn read(tenant: &TenantTransaction) -> Result<Vec<Person>, OperationError> {
    participants(tenant)
        .await?
        .into_iter()
        .map(|row| {
            Ok(Person {
                id: PersonId::from_database(row.try_get("", "id")?),
                name: row.try_get("", "name")?,
                enabled: row.try_get("", "enabled")?,
            })
        })
        .collect()
}

pub async fn update(
    tenant: &TenantTransaction,
    account_id: AccountId,
    changes: Changes,
    selected: &[PersonId],
) -> Result<(), OperationError> {
    super::update(tenant, account_id, changes).await?;
    let people = read(tenant).await?;
    if selected
        .iter()
        .any(|id| !people.iter().any(|person| person.id == *id))
    {
        return Err(profile::validation(
            "managed_person_ids",
            "contains a person outside your current household",
        ));
    }
    let changed: Vec<PersonId> = people
        .iter()
        .filter(|person| person.enabled != selected.contains(&person.id))
        .map(|person| person.id)
        .collect();
    if changed.is_empty() {
        return Ok(());
    }
    let mut previous: Vec<PersonId> = people
        .iter()
        .filter(|person| person.enabled)
        .map(|person| person.id)
        .collect();
    let result = tenant.transaction().execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO household_notification_subscriptions(account_id,household_id,person_id,enabled) SELECT $1,$2,m.person_id,EXISTS(SELECT 1 FROM jsonb_array_elements_text($3::jsonb) chosen(value) WHERE chosen.value::bigint=m.person_id) FROM household_memberships m WHERE m.household_id=$2 AND m.status='active' AND m.revoked_at IS NULL AND m.person_id IN(SELECT value::bigint FROM jsonb_array_elements_text($4::jsonb)) ON CONFLICT(account_id,household_id,person_id) DO UPDATE SET enabled=EXCLUDED.enabled,updated_at=timezone('UTC',clock_timestamp())",
        [account_id.get().into(),tenant.scope().household_id.into(),json!(selected).into(),json!(changed).into()],
    )).await?;
    if result.rows_affected() != changed.len() as u64 {
        return Err(OperationError::Forbidden);
    }
    previous.sort_unstable();
    security_audit_event::ActiveModel {
        actor_membership_id: Set(Some(tenant.membership().id)),
        actor_account_id: Set(Some(account_id.get())),
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
