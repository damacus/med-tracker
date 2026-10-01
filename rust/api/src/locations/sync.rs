use super::access::manager;
use super::cascade::{delete_dependents, record_cascade_effects};
use super::persistence::{location, location_change, representation, snapshot};
use super::validation::{attributes, valid_identifier};
use crate::database_error;
use crate::entities::api_tombstone;
use crate::read_entities::stock_location;
use crate::ApiError;
use crate::AuthContext;
use axum::http::StatusCode;
use chrono::Utc;
use sea_orm::ActiveModelTrait;
use sea_orm::ColumnTrait;
use sea_orm::DatabaseTransaction;
use sea_orm::DbErr;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::Set;
use serde_json::json;
use serde_json::Value;

pub(crate) async fn apply_sync_operation(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &crate::sync_batch::SyncOperation,
    request_id: &str,
) -> Result<crate::sync_batch::SyncResult, ApiError> {
    if !manager(context) {
        return Err(ApiError::forbidden());
    }
    let household_id = context.membership.household_id;
    let request = json!({"location": operation.attributes});
    match operation.action.as_str() {
        "create" => {
            let Some((Some(name), description)) = attributes(&request, true) else {
                return Err(sync_error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "unprocessable_content",
                    "Location attributes are invalid",
                ));
            };
            let now = Utc::now().naive_utc();
            let record = stock_location::ActiveModel {
                household_id: Set(household_id),
                name: Set(name),
                description: Set(description.unwrap_or(None)),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(sync_write_error)?;
            location_change(
                db,
                context,
                request_id,
                &record,
                "create",
                "create",
                None,
                Some(snapshot(&record)),
            )
            .await?;
            Ok(sync_result(record, true))
        }
        "update" | "delete" => {
            let id = operation.id.as_deref().ok_or_else(|| {
                sync_error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "unprocessable_content",
                    "Location ID is required",
                )
            })?;
            if !valid_identifier(id) {
                return Err(ApiError::not_found());
            }
            let record = location(db, household_id, id, true)
                .await?
                .ok_or_else(ApiError::not_found)?;
            let if_match = operation
                .if_match
                .as_deref()
                .filter(|tag| !tag.is_empty())
                .ok_or_else(|| {
                    sync_error(
                        StatusCode::PRECONDITION_REQUIRED,
                        "precondition_required",
                        "if_match is required",
                    )
                })?;
            if if_match != representation(record.clone()).1 {
                return Err(sync_error(
                    StatusCode::CONFLICT,
                    "sync_conflict",
                    "Record has changed since it was last read",
                ));
            }
            if operation.action == "update" {
                let Some((name, description)) = attributes(&request, false) else {
                    return Err(sync_error(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "unprocessable_content",
                        "Location attributes are invalid",
                    ));
                };
                if name.as_ref().is_none_or(|name| name == &record.name)
                    && description
                        .as_ref()
                        .is_none_or(|description| description == &record.description)
                {
                    return Ok(sync_result(record, true));
                }
                let before = snapshot(&record);
                let mut active: stock_location::ActiveModel = record.into();
                if let Some(name) = name {
                    active.name = Set(name);
                }
                if let Some(description) = description {
                    active.description = Set(description);
                }
                active.updated_at = Set(Utc::now().naive_utc());
                let changed = active.update(db).await.map_err(sync_write_error)?;
                location_change(
                    db,
                    context,
                    request_id,
                    &changed,
                    "update",
                    "update",
                    Some(before),
                    Some(snapshot(&changed)),
                )
                .await?;
                Ok(sync_result(changed, true))
            } else {
                if !operation.attributes.is_empty() {
                    return Err(sync_error(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "unprocessable_content",
                        "Location attributes are invalid",
                    ));
                }
                let cascade = delete_dependents(db, &record)
                    .await
                    .map_err(sync_write_error)?
                    .ok_or_else(|| {
                        sync_error(
                            StatusCode::UNPROCESSABLE_ENTITY,
                            "unprocessable_content",
                            "Location cannot be deleted while administration history exists",
                        )
                    })?;
                record_cascade_effects(db, context, request_id, &cascade).await?;
                location_change(
                    db,
                    context,
                    request_id,
                    &record,
                    "destroy",
                    "delete",
                    Some(snapshot(&record)),
                    None,
                )
                .await?;
                Ok(sync_result(record, false))
            }
        }
        _ => Err(sync_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "sync_operation_unsupported",
            "Operation is not supported offline",
        )),
    }
}

pub(crate) async fn authorize_sync_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &crate::sync_batch::SyncOperation,
    saved: &Value,
) -> Result<(), ApiError> {
    if !manager(context) {
        return Err(ApiError::forbidden());
    }
    let portable_id = saved
        .get("record_portable_id")
        .and_then(Value::as_str)
        .ok_or_else(ApiError::forbidden)?;
    if operation.action == "delete" {
        let found = api_tombstone::Entity::find()
            .filter(api_tombstone::Column::HouseholdId.eq(context.membership.household_id))
            .filter(api_tombstone::Column::RecordType.eq("Location"))
            .filter(api_tombstone::Column::RecordPortableId.eq(portable_id))
            .one(db)
            .await
            .map_err(database_error)?;
        if found.is_none() {
            return Err(ApiError::forbidden());
        }
    } else if location(db, context.membership.household_id, portable_id, false)
        .await?
        .is_none()
    {
        return Err(ApiError::forbidden());
    }
    Ok(())
}

fn sync_result(record: stock_location::Model, live: bool) -> crate::sync_batch::SyncResult {
    let etag = live.then(|| representation(record.clone()).1);
    crate::sync_batch::SyncResult {
        record_type: "Location",
        record_id: Some(record.id),
        record_portable_id: Some(record.portable_id),
        etag,
        replayed: None,
    }
}

fn sync_error(status: StatusCode, code: &'static str, message: &'static str) -> ApiError {
    ApiError {
        status,
        code,
        message,
        preserve_activity: false,
    }
}

fn sync_write_error(error: DbErr) -> ApiError {
    if matches!(
        error.sql_err(),
        Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
    ) {
        sync_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "unprocessable_content",
            "Location attributes are invalid",
        )
    } else if matches!(
        error.sql_err(),
        Some(sea_orm::SqlErr::ForeignKeyConstraintViolation(_))
    ) {
        sync_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "unprocessable_content",
            "Location cannot be deleted while retained records exist",
        )
    } else {
        database_error(error)
    }
}
