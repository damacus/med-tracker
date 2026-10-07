use super::*;

pub(super) async fn change(
    tenant: &TenantTransaction,
    kind: &str,
    id: i64,
    portable_id: &str,
    action: &str,
    person_portable_id: Option<&str>,
) -> Result<(), OperationError> {
    let now = Utc::now().naive_utc();
    let mut metadata = json!({"record_type":kind,"record_id":id,"portable_id":portable_id});
    if let Some(person) = person_portable_id {
        metadata["person_portable_id"] = json!(person);
    }
    api_change_event::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        household_membership_id: Set(Some(tenant.membership().id)),
        account_id: Set(Some(tenant.scope().actor.account_id)),
        action: Set(action.into()),
        record_type: Set(kind.into()),
        record_id: Set(id),
        record_portable_id: Set(Some(portable_id.into())),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        metadata: Set(metadata),
        occurred_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    Ok(())
}

pub(super) async fn tombstone(
    tenant: &TenantTransaction,
    kind: &str,
    id: i64,
    portable_id: &str,
    person_portable_id: &str,
) -> Result<(), OperationError> {
    let now = Utc::now().naive_utc();
    api_tombstone::ActiveModel {
        household_id:Set(tenant.scope().household_id),household_membership_id:Set(Some(tenant.membership().id)),account_id:Set(Some(tenant.scope().actor.account_id)),
        action:Set("delete".into()),record_type:Set(kind.into()),record_portable_id:Set(portable_id.into()),
        metadata:Set(json!({"record_type":kind,"record_id":id,"portable_id":portable_id,"person_portable_id":person_portable_id})),
        deleted_at:Set(now),created_at:Set(now),updated_at:Set(now),..Default::default()
    }.insert(tenant.transaction()).await?;
    Ok(())
}
