use super::*;

pub(super) async fn change(
    context: &StockContext<'_>,
    kind: &str,
    id: i64,
    portable: &str,
    action: &str,
    extra: Value,
) -> Result<(), OperationError> {
    let tenant = context.tenant;
    let now = Utc::now().naive_utc();
    let mut metadata = json!({"record_type":kind,"record_id":id,"portable_id":portable});
    if let (Some(base), Some(extra)) = (metadata.as_object_mut(), extra.as_object()) {
        base.extend(extra.clone());
    }
    api_change_event::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        household_membership_id: Set(Some(tenant.membership().id)),
        account_id: Set(Some(tenant.scope().actor.account_id)),
        action: Set(action.into()),
        record_type: Set(kind.into()),
        record_id: Set(id),
        record_portable_id: Set(Some(portable.into())),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        metadata: Set(metadata),
        occurred_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await
    .map_err(database_error)?;
    Ok(())
}

pub(super) async fn tombstone(
    db: &DatabaseTransaction,
    context: &StockContext<'_>,
    kind: &str,
    id: i64,
    portable: &str,
    extra: Value,
) -> Result<(), OperationError> {
    let tenant = context.tenant;
    let now = Utc::now().naive_utc();
    let mut metadata = json!({"record_type":kind,"record_id":id,"portable_id":portable});
    if let (Some(base), Some(extra)) = (metadata.as_object_mut(), extra.as_object()) {
        base.extend(extra.clone());
    }
    api_tombstone::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        household_membership_id: Set(Some(tenant.membership().id)),
        account_id: Set(Some(tenant.scope().actor.account_id)),
        action: Set("delete".into()),
        record_type: Set(kind.into()),
        record_portable_id: Set(portable.into()),
        metadata: Set(metadata),
        deleted_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    Ok(())
}
