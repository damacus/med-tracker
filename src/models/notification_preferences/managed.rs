use super::*;
use crate::models::entities::{carer_relationship, grant, membership, security_audit_event};
use sea_orm::sea_query::{Alias, Expr, Func, LockType, Query};
use sea_orm::{
    Condition, ConnectionTrait, DbBackend, EntityTrait, ExprTrait, QueryFilter, Statement,
};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct Person {
    pub id: i64,
    pub name: String,
    pub adult: bool,
    pub enabled: bool,
}

struct GrantRow {
    grant_id: i64,
    person: person::Model,
    enabled: bool,
}

fn linked_relationship_exists(actor_person_id: i64) -> sea_orm::sea_query::SelectStatement {
    let link = Alias::new("link");
    let declaration = Alias::new("declaration");
    let live_declaration = Query::select()
        .expr(1)
        .from_as(
            (Alias::new("public"), Alias::new("guardian_declarations")),
            declaration.clone(),
        )
        .cond_where(
            Condition::all()
                .add(Expr::col((declaration.clone(), "id")).equals((
                    link.clone(),
                    carer_relationship::Column::GuardianDeclarationId,
                )))
                .add(Expr::col((declaration.clone(), "withdrawn_at")).is_null())
                .add(Expr::col((declaration.clone(), "superseded_at")).is_null()),
        )
        .to_owned();
    Query::select()
        .expr(1)
        .from_as(carer_relationship::Entity, link.clone())
        .cond_where(
            Condition::all()
                .add(
                    Expr::col((link.clone(), carer_relationship::Column::Id))
                        .equals((grant::Entity, grant::Column::CarerRelationshipId)),
                )
                .add(
                    Expr::col((link.clone(), carer_relationship::Column::HouseholdId))
                        .equals((grant::Entity, grant::Column::HouseholdId)),
                )
                .add(
                    Expr::col((link.clone(), carer_relationship::Column::PatientId))
                        .equals((grant::Entity, grant::Column::PersonId)),
                )
                .add(
                    Expr::col((link.clone(), carer_relationship::Column::CarerId))
                        .eq(actor_person_id),
                )
                .add(Expr::col((link.clone(), carer_relationship::Column::Active)).eq(true))
                .add(
                    Condition::any()
                        .add(
                            Expr::col((
                                link.clone(),
                                carer_relationship::Column::GuardianDeclarationId,
                            ))
                            .is_null(),
                        )
                        .add(Expr::exists(live_declaration)),
                ),
        )
        .to_owned()
}

fn active_subject_membership_exists() -> sea_orm::sea_query::SelectStatement {
    let subject = Alias::new("subject");
    Query::select()
        .expr(1)
        .from_as(membership::Entity, subject.clone())
        .cond_where(
            Condition::all()
                .add(
                    Expr::col((subject.clone(), membership::Column::PersonId))
                        .equals((grant::Entity, grant::Column::PersonId)),
                )
                .add(
                    Expr::col((subject.clone(), membership::Column::HouseholdId))
                        .equals((grant::Entity, grant::Column::HouseholdId)),
                )
                .add(Expr::col((subject.clone(), membership::Column::Status)).eq("active"))
                .add(Expr::col((subject.clone(), membership::Column::RevokedAt)).is_null()),
        )
        .to_owned()
}

async fn grants(tenant: &TenantTransaction, lock: bool) -> Result<Vec<GrantRow>, OperationError> {
    let actor_person_id = tenant.membership().person_id;
    let mut query = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(tenant.membership().id))
        .filter(grant::Column::PersonId.ne(actor_person_id))
        .filter(grant::Column::AccessLevel.eq("manage"))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(
                    Expr::col((grant::Entity, grant::Column::ExpiresAt)).gt(Expr::from(
                        Func::cust("pg_catalog.timezone").args(vec![
                            Expr::val("UTC"),
                            Expr::from(Func::cust("pg_catalog.clock_timestamp")),
                        ]),
                    )),
                ),
        )
        .filter(
            Condition::any()
                .add(grant::Column::CarerRelationshipId.is_null())
                .add(Expr::exists(linked_relationship_exists(actor_person_id))),
        )
        .filter(Expr::exists(active_subject_membership_exists()));
    if lock {
        query = query.lock(LockType::Update);
    }
    let rows = query.all(tenant.transaction()).await?;
    let people: HashMap<i64, person::Model> = person::Entity::find()
        .filter(person::Column::Id.is_in(rows.iter().map(|row| row.person_id)))
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|model| (model.id, model))
        .collect();
    let mut grants = Vec::new();
    for row in rows {
        let person = people
            .get(&row.person_id)
            .ok_or(OperationError::Unavailable)?;
        grants.push(GrantRow {
            grant_id: row.id,
            person: person.clone(),
            enabled: row.missed_dose_notifications_enabled,
        });
    }
    grants.sort_by(|left, right| {
        left.person
            .name
            .cmp(&right.person.name)
            .then(left.person.id.cmp(&right.person.id))
            .then(left.grant_id.cmp(&right.grant_id))
    });
    Ok(grants)
}

pub async fn read(tenant: &TenantTransaction) -> Result<Vec<Person>, OperationError> {
    let mut people: Vec<Person> = Vec::new();
    for row in grants(tenant, false).await? {
        if let Some(existing) = people.iter_mut().find(|person| person.id == row.person.id) {
            existing.enabled |= row.enabled;
        } else {
            people.push(Person {
                id: row.person.id,
                name: row.person.name,
                adult: row.person.person_type == 0,
                enabled: row.enabled,
            });
        }
    }
    let declared = crate::models::care::guardian_declarations::manageable_subjects(tenant).await?;
    for subject in person::Entity::find()
        .filter(person::Column::Id.is_in(declared))
        .all(tenant.transaction())
        .await?
    {
        if people.iter().all(|person| person.id != subject.id) {
            people.push(Person {
                id: subject.id,
                name: subject.name,
                adult: subject.person_type == 0,
                enabled: false,
            });
        }
    }
    people.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
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
        if row.person.person_type != 0 {
            continue;
        }
        let id = row.person.id;
        adult_ids.push(id);
        if row.enabled {
            previous.push(id);
        }
        if row.enabled != selected.contains(&id) {
            changed_grants.push(row.grant_id);
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
