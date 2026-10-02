use crate::database_error;
use crate::entities::api_tombstone;
use crate::medication_management::record_version;
use crate::read_entities::stock_location;
use crate::read_resources::location_value;
use crate::representation_etag;
use crate::sync_events::record_change;
use crate::sync_events::SyncRecord;
use crate::ApiError;
use crate::AuthContext;
use chrono::Utc;
use sea_orm::ActiveModelTrait;
use sea_orm::ColumnTrait;
use sea_orm::DatabaseTransaction;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::QuerySelect;
use sea_orm::Set;
use serde_json::json;
use serde_json::Value;

pub(super) async fn location(
    db: &DatabaseTransaction,
    household_id: i64,
    id: &str,
    lock: bool,
) -> Result<Option<stock_location::Model>, ApiError> {
    let query =
        stock_location::Entity::find().filter(stock_location::Column::HouseholdId.eq(household_id));
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(stock_location::Column::Id.eq(id))
    } else {
        query.filter(stock_location::Column::PortableId.eq(id))
    };
    let query = if lock { query.lock_exclusive() } else { query };
    query.one(db).await.map_err(database_error)
}

pub(super) fn representation(record: stock_location::Model) -> (Value, String) {
    let body = json!({"data": location_value(record)});
    let etag = representation_etag(&body);
    (body, etag)
}

pub(super) fn snapshot(record: &stock_location::Model) -> Value {
    json!({"id": record.id, "household_id": record.household_id, "portable_id": record.portable_id, "name": record.name, "description": record.description, "created_at": record.created_at, "updated_at": record.updated_at})
}

pub(super) async fn tombstone(
    db: &DatabaseTransaction,
    context: &AuthContext,
    record_type: &str,
    record_id: i64,
    portable_id: &str,
    extra_metadata: Value,
) -> Result<(), ApiError> {
    let now = Utc::now().naive_utc();
    let mut metadata =
        json!({"record_type": record_type, "record_id": record_id, "portable_id": portable_id});
    if let (Some(base), Some(extra)) = (metadata.as_object_mut(), extra_metadata.as_object()) {
        base.extend(extra.clone());
    }
    api_tombstone::ActiveModel {
        household_id: Set(context.membership.household_id),
        household_membership_id: Set(Some(context.membership.id)),
        account_id: Set(Some(context.account_id)),
        action: Set("delete".to_owned()),
        record_type: Set(record_type.to_owned()),
        record_portable_id: Set(portable_id.to_owned()),
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

#[allow(clippy::too_many_arguments)]
pub(super) async fn location_change(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    record: &stock_location::Model,
    version_event: &str,
    sync_action: &str,
    before: Option<Value>,
    after: Option<Value>,
) -> Result<(), ApiError> {
    record_version(
        db,
        context,
        request_id,
        "Location",
        record.id,
        version_event,
        before,
        after,
    )
    .await?;
    if sync_action == "delete" {
        tombstone(
            db,
            context,
            "Location",
            record.id,
            &record.portable_id,
            json!({}),
        )
        .await
    } else {
        record_change(
            db,
            context,
            request_id,
            SyncRecord {
                record_type: "Location",
                record_id: record.id,
                portable_id: &record.portable_id,
                action: sync_action,
                person_portable_id: None,
            },
        )
        .await
    }
}
