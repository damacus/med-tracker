use crate::entities::api_change_event;
use crate::{database_error, ApiError, AuthContext};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseTransaction, DbBackend, Set, Statement};
use serde_json::json;

pub(super) struct SyncRecord<'a> {
    pub record_type: &'a str,
    pub record_id: i64,
    pub portable_id: &'a str,
    pub action: &'a str,
    pub person_portable_id: Option<&'a str>,
}

pub(super) async fn lock_household(
    db: &DatabaseTransaction,
    household_id: i64,
) -> Result<(), ApiError> {
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT id FROM households WHERE id = $1 FOR UPDATE",
        [household_id.into()],
    ))
    .await
    .map_err(database_error)?
    .ok_or_else(ApiError::not_found)?;
    Ok(())
}

pub(super) async fn record_change(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    record: SyncRecord<'_>,
) -> Result<(), ApiError> {
    let now = Utc::now().naive_utc();
    let mut metadata = json!({
        "record_type": record.record_type,
        "record_id": record.record_id,
        "portable_id": record.portable_id
    });
    if let Some(person_portable_id) = record.person_portable_id {
        metadata["person_portable_id"] = json!(person_portable_id);
    }
    api_change_event::ActiveModel {
        household_id: Set(context.membership.household_id),
        household_membership_id: Set(Some(context.membership.id)),
        account_id: Set(Some(context.account_id)),
        action: Set(record.action.to_owned()),
        record_type: Set(record.record_type.to_owned()),
        record_id: Set(record.record_id),
        record_portable_id: Set(Some(record.portable_id.to_owned())),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(metadata),
        occurred_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    Ok(())
}
