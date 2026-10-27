use super::*;
use sea_orm::{
    ConnectionTrait,
    sea_query::{Expr, Func, Query},
};

pub(super) async fn self_changes(
    transaction: &sea_orm::DatabaseTransaction,
    account_id: crate::models::ids::AccountId,
    person: &person::Model,
    request_id: &str,
) -> Result<(), OperationError> {
    let memberships = membership::Entity::find()
        .inner_join(household::Entity)
        .filter(membership::Column::PersonId.eq(person.id))
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .filter(household::Column::Status.eq("active"))
        .filter(household::Column::LifecycleState.eq("active"))
        .order_by_asc(membership::Column::HouseholdId)
        .lock_shared()
        .all(transaction)
        .await?;
    for membership in memberships {
        event_context(transaction, Some(&membership)).await?;
        let now = Utc::now().naive_utc();
        api_change_event::ActiveModel {
            household_id: Set(membership.household_id),
            household_membership_id: Set(Some(membership.id)),
            account_id: Set(Some(account_id.get())),
            action: Set("update".into()),
            record_type: Set("Person".into()),
            record_id: Set(person.id),
            record_portable_id: Set(Some(person.portable_id.clone())),
            request_id: Set(Some(request_id.into())),
            metadata: Set(json!({"record_type":"Person","record_id":person.id,"portable_id":person.portable_id})),
            occurred_at: Set(now), created_at: Set(now), updated_at: Set(now),
            ..Default::default()
        }.insert(transaction).await?;
    }
    event_context(transaction, None).await
}

async fn event_context(
    transaction: &sea_orm::DatabaseTransaction,
    membership: Option<&membership::Model>,
) -> Result<(), OperationError> {
    let context = Query::select()
        .expr(
            Func::cust("pg_catalog.set_config").args([
                Expr::val("med_tracker.current_household_id"),
                Expr::val(
                    membership
                        .map(|row| row.household_id.to_string())
                        .unwrap_or_default(),
                ),
                Expr::val(true),
            ]),
        )
        .expr(Func::cust("pg_catalog.set_config").args([
            Expr::val("med_tracker.current_membership_id"),
            Expr::val(membership.map(|row| row.id.to_string()).unwrap_or_default()),
            Expr::val(true),
        ]))
        .to_owned();
    transaction
        .execute_raw(transaction.get_database_backend().build(&context))
        .await?;
    Ok(())
}

pub(super) fn snapshot(record: &person::Model) -> Value {
    json!({"id":record.id,"portable_id":record.portable_id,"name":record.name,"email":record.email,"date_of_birth":record.date_of_birth,"person_type":record.person_type,"has_capacity":record.has_capacity,"created_at":record.created_at,"updated_at":record.updated_at})
}

fn context(
    tenant: &TenantTransaction,
    event: &str,
    provenance: Option<&CredentialProvenance>,
) -> Value {
    let mut value = crate::models::audit_context::household(tenant, provenance);
    value["policy_class"] = json!("PersonPolicy");
    value["policy_query"] = json!(format!("{event}?"));
    value
}

pub(super) async fn record_version(
    tenant: &TenantTransaction,
    kind: &str,
    id: i64,
    event: &str,
    before: Option<Value>,
    after: Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    let mut fields = std::collections::BTreeSet::new();
    for row in [before.as_ref(), Some(&after)].into_iter().flatten() {
        fields.extend(
            row.as_object()
                .ok_or(OperationError::Unavailable)?
                .keys()
                .cloned(),
        );
    }
    let mut changes = serde_json::Map::new();
    for field in fields {
        let old = before
            .as_ref()
            .and_then(|value| value.get(&field))
            .cloned()
            .unwrap_or(Value::Null);
        let new = after.get(&field).cloned().unwrap_or(Value::Null);
        if old != new {
            changes.insert(field, json!([old, new]));
        }
    }
    version::ActiveModel {
        item_type: Set(kind.into()),
        item_id: Set(id),
        event: Set(event.into()),
        object: Set(before.map(|value| value.to_string())),
        object_changes: Set(Some(Value::Object(changes).to_string())),
        whodunnit: Set(tenant.legacy_user_id().map(|id| id.to_string())),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        household_id: Set(Some(tenant.scope().household_id)),
        actor_membership_id: Set(Some(tenant.membership().id)),
        audit_context: Set(context(tenant, event, provenance)),
        created_at: Set(Some(Utc::now().naive_utc())),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    Ok(())
}

pub(super) async fn change(
    tenant: &TenantTransaction,
    kind: &str,
    id: i64,
    portable: &str,
    event: &str,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    let now = Utc::now().naive_utc();
    let _ = provenance;
    api_change_event::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        household_membership_id: Set(Some(tenant.membership().id)),
        account_id: Set(Some(tenant.scope().actor.account_id)),
        action: Set(event.into()),
        record_type: Set(kind.into()),
        record_id: Set(id),
        record_portable_id: Set(Some(portable.into())),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        metadata: Set(json!({"record_type":kind,"record_id":id,"portable_id":portable})),
        occurred_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    Ok(())
}
