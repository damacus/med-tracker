use super::access::{carer_exists, manageable, may_create};
use super::persistence::{grant_state, record_grant_event, representation, snapshot};
use super::validation::{parse_attributes, person_valid, valid_identifier};
use crate::database_error;
use crate::entities::grant;
use crate::entities::membership;
use crate::granted_people;
use crate::medication_management::record_version;
use crate::read_entities::carer_relationship;
use crate::read_entities::location_membership;
use crate::read_entities::person;
use crate::read_entities::stock_location;
use crate::read_resources::age;
use crate::read_resources::today;
use crate::sync_batch::SyncOperation;
use crate::sync_batch::SyncResult;
use crate::sync_events::record_change;
use crate::sync_events::SyncRecord;
use crate::ApiError;
use crate::AuthContext;
use axum::http::StatusCode;
use chrono::Utc;
use sea_orm::ActiveModelTrait;
use sea_orm::ColumnTrait;
use sea_orm::DatabaseTransaction;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::QueryOrder;
use sea_orm::Set;
use serde_json::json;
use serde_json::Value;
use uuid::Uuid;

fn sync_error(status: StatusCode) -> ApiError {
    let (code, message) = match status {
        StatusCode::BAD_REQUEST => ("bad_request", "Invalid request body"),
        StatusCode::NOT_FOUND => ("not_found", "Record not found"),
        StatusCode::FORBIDDEN => (
            "forbidden",
            "You are not authorized to perform this action.",
        ),
        StatusCode::PRECONDITION_REQUIRED => (
            "precondition_required",
            "A current resource version is required",
        ),
        StatusCode::CONFLICT => ("sync_conflict", "Record has changed since it was last read"),
        _ => ("unprocessable_content", "Person is invalid"),
    };
    ApiError {
        status,
        code,
        message,
        preserve_activity: false,
    }
}

pub(crate) async fn apply_sync_operation(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &SyncOperation,
    request_id: &str,
) -> Result<SyncResult, ApiError> {
    let household_id = context.membership.household_id;
    if operation.action == "create" {
        if !may_create(db, &context.membership).await? {
            return Err(sync_error(StatusCode::FORBIDDEN));
        }
        let attrs =
            parse_attributes(&json!({"person": operation.attributes}), true).map_err(sync_error)?;
        let now = Utc::now().naive_utc();
        let person_type = attrs.person_type.unwrap_or(0);
        let years = age(attrs.date_of_birth, today()).ok_or_else(ApiError::internal)?;
        let dependent = (years < 18 && person_type == 1) || (years >= 18 && person_type == 2);
        let actor_person_id = context.membership.person_id;
        let has_capacity = if dependent {
            false
        } else {
            attrs.has_capacity.unwrap_or(true)
        };
        let carer_in_household = if let Some(id) = actor_person_id {
            person::Entity::find_by_id(id)
                .one(db)
                .await
                .map_err(database_error)?
                .is_some_and(|person| person.household_id == household_id)
        } else {
            false
        };
        if (years < 18 && person_type == 2)
            || (years >= 18 && person_type == 1)
            || (!has_capacity && (!dependent || !carer_in_household))
        {
            return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY));
        }
        let record = person::ActiveModel {
            account_id: Set(None),
            household_id: Set(household_id),
            portable_id: Set(Uuid::new_v4().to_string()),
            name: Set(attrs.name.unwrap()),
            email: Set(attrs.email.unwrap_or(None)),
            date_of_birth: Set(attrs.date_of_birth),
            person_type: Set(person_type),
            has_capacity: Set(has_capacity),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|error| {
            if matches!(
                error.sql_err(),
                Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
            ) {
                sync_error(StatusCode::UNPROCESSABLE_ENTITY)
            } else {
                database_error(error)
            }
        })?;
        let relationship = if dependent {
            Some(
                carer_relationship::ActiveModel {
                    household_id: Set(household_id),
                    carer_id: Set(actor_person_id.unwrap()),
                    patient_id: Set(record.id),
                    relationship_type: Set(Some("family_member".to_owned())),
                    active: Set(true),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                }
                .insert(db)
                .await
                .map_err(database_error)?,
            )
        } else {
            None
        };
        let mut self_grant_event = None;
        if dependent {
            let carer_id = actor_person_id.unwrap();
            let existing = grant::Entity::find()
                .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
                .filter(grant::Column::PersonId.eq(carer_id))
                .order_by_desc(grant::Column::Id)
                .one(db)
                .await
                .map_err(database_error)?;
            let needs_update = existing.as_ref().is_none_or(|grant| {
                grant.access_level != "manage"
                    || grant.relationship_type != "self"
                    || grant.expires_at.is_some()
                    || grant.revoked_at.is_some()
                    || grant.carer_relationship_id.is_some()
            });
            if needs_update {
                let (previous, updated) = if let Some(existing) = existing {
                    let previous = grant_state(&existing);
                    let mut active: grant::ActiveModel = existing.into();
                    active.access_level = Set("manage".to_owned());
                    active.relationship_type = Set("self".to_owned());
                    active.expires_at = Set(None);
                    active.revoked_at = Set(None);
                    active.carer_relationship_id = Set(None);
                    active.updated_at = Set(now);
                    (
                        Some(previous),
                        active.update(db).await.map_err(database_error)?,
                    )
                } else {
                    (
                        None,
                        grant::ActiveModel {
                            household_id: Set(household_id),
                            household_membership_id: Set(context.membership.id),
                            person_id: Set(carer_id),
                            access_level: Set("manage".to_owned()),
                            relationship_type: Set("self".to_owned()),
                            granted_by_membership_id: Set(Some(context.membership.id)),
                            revoked_at: Set(None),
                            expires_at: Set(None),
                            carer_relationship_id: Set(None),
                            created_at: Set(now),
                            updated_at: Set(now),
                            ..Default::default()
                        }
                        .insert(db)
                        .await
                        .map_err(database_error)?,
                    )
                };
                self_grant_event = Some((previous, updated));
            }
        }
        let home = stock_location::Entity::find()
            .filter(stock_location::Column::HouseholdId.eq(household_id))
            .filter(stock_location::Column::Name.eq("Home"))
            .one(db)
            .await
            .map_err(database_error)?;
        let (home, home_created) = if let Some(home) = home {
            (home, false)
        } else {
            (
                stock_location::ActiveModel {
                    household_id: Set(household_id),
                    portable_id: Set(Uuid::new_v4().to_string()),
                    name: Set("Home".to_owned()),
                    description: Set(Some("Primary home location".to_owned())),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                }
                .insert(db)
                .await
                .map_err(database_error)?,
                true,
            )
        };
        let location_link = location_membership::ActiveModel {
            household_id: Set(household_id),
            location_id: Set(home.id),
            person_id: Set(record.id),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(database_error)?;
        let patient_grant = grant::ActiveModel {
            household_id: Set(household_id),
            household_membership_id: Set(context.membership.id),
            person_id: Set(record.id),
            access_level: Set("manage".to_owned()),
            relationship_type: Set("family_member".to_owned()),
            granted_by_membership_id: Set(Some(context.membership.id)),
            revoked_at: Set(None),
            expires_at: Set(None),
            carer_relationship_id: Set(relationship.as_ref().map(|row| row.id)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(database_error)?;
        let mut membership: membership::ActiveModel = context.membership.clone().into();
        membership.permissions_version =
            Set(context.membership.permissions_version + 1 + i32::from(self_grant_event.is_some()));
        membership.updated_at = Set(now);
        membership.update(db).await.map_err(database_error)?;
        record_version(
            db,
            context,
            request_id,
            "Person",
            record.id,
            "create",
            None,
            Some(snapshot(&record)),
        )
        .await?;
        if home_created {
            record_version(db, context, request_id, "Location", home.id, "create", None,
                Some(json!({"id": home.id, "household_id": home.household_id,
                    "portable_id": home.portable_id, "name": home.name, "description": home.description,
                    "created_at": home.created_at, "updated_at": home.updated_at}))).await?;
            record_change(
                db,
                context,
                request_id,
                SyncRecord {
                    record_type: "Location",
                    record_id: home.id,
                    portable_id: &home.portable_id,
                    action: "create",
                    person_portable_id: None,
                },
            )
            .await?;
        }
        record_version(
            db,
            context,
            request_id,
            "LocationMembership",
            location_link.id,
            "create",
            None,
            Some(
                json!({"id": location_link.id, "household_id": location_link.household_id,
                "location_id": location_link.location_id, "person_id": location_link.person_id,
                "created_at": location_link.created_at, "updated_at": location_link.updated_at}),
            ),
        )
        .await?;
        if let Some(relationship) = &relationship {
            record_version(db, context, request_id, "CarerRelationship", relationship.id, "create", None,
                Some(json!({"id": relationship.id, "household_id": relationship.household_id,
                    "carer_id": relationship.carer_id, "patient_id": relationship.patient_id,
                    "relationship_type": relationship.relationship_type, "active": relationship.active,
                    "created_at": relationship.created_at, "updated_at": relationship.updated_at}))).await?;
        }
        if let Some((previous, grant)) = &self_grant_event {
            record_grant_event(db, context, request_id, grant, previous.clone()).await?;
        }
        record_grant_event(db, context, request_id, &patient_grant, None).await?;
        record_change(
            db,
            context,
            request_id,
            SyncRecord {
                record_type: "Person",
                record_id: record.id,
                portable_id: &record.portable_id,
                action: "create",
                person_portable_id: Some(&record.portable_id),
            },
        )
        .await?;
        let (body, etag) = representation(db, record.clone()).await?;
        let _ = body;
        return Ok(SyncResult {
            record_type: "Person",
            record_id: Some(record.id),
            record_portable_id: Some(record.portable_id),
            etag: Some(etag),
            replayed: Some(false),
        });
    }
    let id = operation
        .id
        .as_deref()
        .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
    if !valid_identifier(id) {
        return Err(sync_error(StatusCode::BAD_REQUEST));
    }
    let mut query = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(household_id))
        .filter(person::Column::Id.in_subquery(granted_people(&context.membership)));
    query = if let Ok(id) = id.parse::<i64>() {
        query.filter(person::Column::Id.eq(id))
    } else {
        query.filter(person::Column::PortableId.eq(id))
    };
    let record = query
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
    if !manageable(db, context, record.id).await? {
        return Err(sync_error(StatusCode::FORBIDDEN));
    }
    let (_, current_etag) = representation(db, record.clone()).await?;
    let expected = operation
        .if_match
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| sync_error(StatusCode::PRECONDITION_REQUIRED))?;
    if expected != current_etag {
        return Err(sync_error(StatusCode::CONFLICT));
    }
    let attrs =
        parse_attributes(&json!({"person": operation.attributes}), false).map_err(sync_error)?;
    let before = snapshot(&record);
    let mut active: person::ActiveModel = record.clone().into();
    if let Some(value) = attrs.name {
        active.name = Set(value);
    }
    if let Some(value) = attrs.email {
        active.email = Set(value);
    }
    if let Some(value) = attrs.date_of_birth {
        active.date_of_birth = Set(Some(value));
    }
    if let Some(value) = attrs.person_type {
        active.person_type = Set(value);
    }
    if let Some(value) = attrs.has_capacity {
        active.has_capacity = Set(value);
    }
    let years = age(active.date_of_birth.clone().take().flatten(), today())
        .ok_or_else(|| sync_error(StatusCode::UNPROCESSABLE_ENTITY))?;
    let person_type = active
        .person_type
        .clone()
        .take()
        .ok_or_else(ApiError::internal)?;
    if (years < 18 && person_type == 1) || (years >= 18 && person_type == 2) {
        active.has_capacity = Set(false);
    }
    let unchanged = active
        .name
        .clone()
        .take()
        .unwrap_or_else(|| record.name.clone())
        == record.name
        && active
            .email
            .clone()
            .take()
            .unwrap_or_else(|| record.email.clone())
            == record.email
        && active
            .date_of_birth
            .clone()
            .take()
            .unwrap_or(record.date_of_birth)
            == record.date_of_birth
        && active
            .person_type
            .clone()
            .take()
            .unwrap_or(record.person_type)
            == record.person_type
        && active
            .has_capacity
            .clone()
            .take()
            .unwrap_or(record.has_capacity)
            == record.has_capacity;
    if unchanged {
        if !person_valid(&record, carer_exists(db, record.id).await?) {
            return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY));
        }
        return Ok(SyncResult {
            record_type: "Person",
            record_id: Some(record.id),
            record_portable_id: Some(record.portable_id),
            etag: Some(current_etag),
            replayed: Some(true),
        });
    }
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(db).await.map_err(|error| {
        if matches!(
            error.sql_err(),
            Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
        ) {
            sync_error(StatusCode::UNPROCESSABLE_ENTITY)
        } else {
            database_error(error)
        }
    })?;
    if !person_valid(&updated, carer_exists(db, updated.id).await?) {
        return Err(sync_error(StatusCode::UNPROCESSABLE_ENTITY));
    }
    record_version(
        db,
        context,
        request_id,
        "Person",
        updated.id,
        "update",
        Some(before),
        Some(snapshot(&updated)),
    )
    .await?;
    record_change(
        db,
        context,
        request_id,
        SyncRecord {
            record_type: "Person",
            record_id: updated.id,
            portable_id: &updated.portable_id,
            action: "update",
            person_portable_id: Some(&updated.portable_id),
        },
    )
    .await?;
    let (_, etag) = representation(db, updated.clone()).await?;
    Ok(SyncResult {
        record_type: "Person",
        record_id: Some(updated.id),
        record_portable_id: Some(updated.portable_id),
        etag: Some(etag),
        replayed: Some(false),
    })
}

pub(crate) async fn authorize_sync_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    operation: &SyncOperation,
    saved: &Value,
) -> Result<(), ApiError> {
    if saved.get("action").and_then(Value::as_str) != Some(operation.action.as_str())
        || saved.get("record_type").and_then(Value::as_str) != Some("Person")
    {
        return Err(ApiError::forbidden());
    }
    let portable_id = saved
        .get("record_portable_id")
        .and_then(Value::as_str)
        .ok_or_else(ApiError::forbidden)?;
    let record = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(context.membership.household_id))
        .filter(person::Column::PortableId.eq(portable_id))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::forbidden)?;
    if saved.get("record_id").and_then(Value::as_str) != Some(record.id.to_string().as_str())
        || operation
            .id
            .as_deref()
            .is_some_and(|id| id != record.portable_id && id != record.id.to_string())
        || (operation.action == "create" && !may_create(db, &context.membership).await?)
        || !manageable(db, context, record.id).await?
    {
        return Err(ApiError::forbidden());
    }
    Ok(())
}
