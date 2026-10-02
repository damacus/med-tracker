use super::access::may_create;
use super::persistence::{grant_state, record_grant_event, representation, snapshot};
use super::responses::{failure, failure_with_errors};
use super::validation::{name_errors, parse_attributes};
use crate::database_error;
use crate::entities::grant;
use crate::entities::membership;
use crate::medication_management::finish_with_request_id;
use crate::medication_management::record_version;
use crate::medication_management::request_context;
use crate::read_entities::carer_relationship;
use crate::read_entities::location_membership;
use crate::read_entities::person;
use crate::read_entities::stock_location;
use crate::read_resources::age;
use crate::read_resources::today;
use crate::sync_events::lock_household;
use crate::sync_events::record_change;
use crate::sync_events::SyncRecord;
use crate::ApiError;
use crate::AppState;
use axum::extract::rejection::JsonRejection;
use axum::extract::Path;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::Response;
use axum::Json;
use chrono::Utc;
use sea_orm::ActiveModelTrait;
use sea_orm::ColumnTrait;
use sea_orm::EntityTrait;
use sea_orm::QueryFilter;
use sea_orm::QueryOrder;
use sea_orm::QuerySelect;
use sea_orm::Set;
use sea_orm::TransactionTrait;
use serde_json::json;
use serde_json::Value;
use uuid::Uuid;

pub(crate) async fn create(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !may_create(&db, &context.membership).await? {
        return failure(db, &context, "POST", "create", StatusCode::FORBIDDEN).await;
    }
    let Json(body) = match payload {
        Ok(body) => body,
        Err(_) => return failure(db, &context, "POST", "create", StatusCode::BAD_REQUEST).await,
    };
    let attrs = match parse_attributes(&body, true) {
        Ok(attrs) => attrs,
        Err(status) => {
            return failure_with_errors(
                db,
                &context,
                "POST",
                "create",
                status,
                name_errors(&body, true),
            )
            .await;
        }
    };
    lock_household(&db, household_id).await?;
    let current_membership = membership::Entity::find_by_id(context.membership.id)
        .lock_exclusive()
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    if current_membership.status != "active"
        || current_membership.revoked_at.is_some()
        || !may_create(&db, &current_membership).await?
    {
        return failure(db, &context, "POST", "create", StatusCode::FORBIDDEN).await;
    }
    let now = Utc::now().naive_utc();
    let person_type = attrs.person_type.unwrap_or(0);
    let years = age(attrs.date_of_birth, today()).ok_or_else(ApiError::internal)?;
    let dependent = (years < 18 && person_type == 1) || (years >= 18 && person_type == 2);
    let actor_person_id = current_membership.person_id;
    let has_capacity = if dependent {
        false
    } else {
        attrs.has_capacity.unwrap_or(true)
    };
    let carer_in_household = if let Some(actor_person_id) = actor_person_id {
        person::Entity::find_by_id(actor_person_id)
            .one(&db)
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
        return failure(
            db,
            &context,
            "POST",
            "create",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await;
    }
    let savepoint = db.begin().await.map_err(database_error)?;
    let active = person::ActiveModel {
        account_id: Set(None),
        household_id: Set(household_id),
        portable_id: Set(Uuid::new_v4().to_string()),
        name: Set(attrs.name.expect("create name")),
        email: Set(attrs.email.unwrap_or(None)),
        date_of_birth: Set(attrs.date_of_birth),
        person_type: Set(person_type),
        has_capacity: Set(has_capacity),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    let record = match active.insert(&savepoint).await {
        Ok(record) => record,
        Err(error)
            if matches!(
                error.sql_err(),
                Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
            ) =>
        {
            savepoint.rollback().await.map_err(database_error)?;
            return failure(
                db,
                &context,
                "POST",
                "create",
                StatusCode::UNPROCESSABLE_ENTITY,
            )
            .await;
        }
        Err(error) => return Err(database_error(error)),
    };
    let relationship = if dependent {
        let relationship = carer_relationship::ActiveModel {
            household_id: Set(household_id),
            carer_id: Set(actor_person_id.expect("carer")),
            patient_id: Set(record.id),
            relationship_type: Set(Some("family_member".to_owned())),
            active: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&savepoint)
        .await
        .map_err(database_error)?;
        Some(relationship)
    } else {
        None
    };
    let relationship_id = relationship.as_ref().map(|row| row.id);
    let self_grant_event = if dependent {
        let carer_id = actor_person_id.expect("carer");
        let active = grant::Entity::find()
            .filter(grant::Column::HouseholdMembershipId.eq(current_membership.id))
            .filter(grant::Column::PersonId.eq(carer_id))
            .filter(grant::Column::RevokedAt.is_null())
            .lock_exclusive()
            .one(&savepoint)
            .await
            .map_err(database_error)?;
        let existing = if active.is_some() {
            active
        } else {
            grant::Entity::find()
                .filter(grant::Column::HouseholdMembershipId.eq(current_membership.id))
                .filter(grant::Column::PersonId.eq(carer_id))
                .order_by_desc(grant::Column::Id)
                .lock_exclusive()
                .one(&savepoint)
                .await
                .map_err(database_error)?
        };
        if let Some(existing) = existing {
            let unchanged = existing.access_level == "manage"
                && existing.relationship_type == "self"
                && existing.expires_at.is_none()
                && existing.revoked_at.is_none()
                && existing.carer_relationship_id.is_none();
            if unchanged {
                None
            } else {
                let previous = grant_state(&existing);
                let mut self_grant: grant::ActiveModel = existing.into();
                self_grant.access_level = Set("manage".to_owned());
                self_grant.relationship_type = Set("self".to_owned());
                self_grant.expires_at = Set(None);
                self_grant.revoked_at = Set(None);
                self_grant.carer_relationship_id = Set(None);
                self_grant.updated_at = Set(now);
                let updated = self_grant
                    .update(&savepoint)
                    .await
                    .map_err(database_error)?;
                Some((Some(previous), updated))
            }
        } else {
            let created = grant::ActiveModel {
                household_id: Set(household_id),
                household_membership_id: Set(current_membership.id),
                person_id: Set(carer_id),
                access_level: Set("manage".to_owned()),
                relationship_type: Set("self".to_owned()),
                granted_by_membership_id: Set(Some(current_membership.id)),
                revoked_at: Set(None),
                expires_at: Set(None),
                carer_relationship_id: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&savepoint)
            .await
            .map_err(database_error)?;
            Some((None, created))
        }
    } else {
        None
    };
    let home = stock_location::Entity::find()
        .filter(stock_location::Column::HouseholdId.eq(household_id))
        .filter(stock_location::Column::Name.eq("Home"))
        .one(&savepoint)
        .await
        .map_err(database_error)?;
    let (home, home_created) = match home {
        Some(home) => (home, false),
        None => (
            stock_location::ActiveModel {
                household_id: Set(household_id),
                portable_id: Set(Uuid::new_v4().to_string()),
                name: Set("Home".to_owned()),
                description: Set(Some("Primary home location".to_owned())),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&savepoint)
            .await
            .map_err(database_error)?,
            true,
        ),
    };
    let location_link = location_membership::ActiveModel {
        household_id: Set(household_id),
        location_id: Set(home.id),
        person_id: Set(record.id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&savepoint)
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
        carer_relationship_id: Set(relationship_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&savepoint)
    .await
    .map_err(database_error)?;
    let mut active: membership::ActiveModel = current_membership.clone().into();
    active.permissions_version =
        Set(current_membership.permissions_version + 1 + i32::from(self_grant_event.is_some()));
    active.updated_at = Set(now);
    active.update(&savepoint).await.map_err(database_error)?;
    savepoint.commit().await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "Person",
        record.id,
        "create",
        None,
        Some(snapshot(&record)),
    )
    .await?;
    if home_created {
        record_version(
            &db,
            &context,
            &request_id,
            "Location",
            home.id,
            "create",
            None,
            Some(json!({"id": home.id, "household_id": home.household_id, "portable_id": home.portable_id, "name": home.name, "description": home.description, "created_at": home.created_at, "updated_at": home.updated_at})),
        ).await?;
        record_change(
            &db,
            &context,
            &request_id,
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
        &db,
        &context,
        &request_id,
        "LocationMembership",
        location_link.id,
        "create",
        None,
        Some(json!({"id": location_link.id, "household_id": location_link.household_id, "location_id": location_link.location_id, "person_id": location_link.person_id, "created_at": location_link.created_at, "updated_at": location_link.updated_at})),
    ).await?;
    if let Some(relationship) = &relationship {
        record_version(
            &db,
            &context,
            &request_id,
            "CarerRelationship",
            relationship.id,
            "create",
            None,
            Some(json!({"id": relationship.id, "household_id": relationship.household_id, "carer_id": relationship.carer_id, "patient_id": relationship.patient_id, "relationship_type": relationship.relationship_type, "active": relationship.active, "created_at": relationship.created_at, "updated_at": relationship.updated_at})),
        ).await?;
    }
    if let Some((previous, grant)) = &self_grant_event {
        record_grant_event(&db, &context, &request_id, grant, previous.clone()).await?;
    }
    record_grant_event(&db, &context, &request_id, &patient_grant, None).await?;
    record_change(
        &db,
        &context,
        &request_id,
        SyncRecord {
            record_type: "Person",
            record_id: record.id,
            portable_id: &record.portable_id,
            action: "create",
            person_portable_id: Some(&record.portable_id),
        },
    )
    .await?;
    let (body, etag) = representation(&db, record).await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        "POST",
        "api/v1/people",
        "PersonPolicy",
        "create",
        StatusCode::CREATED,
        true,
        body,
        Some(&etag),
    )
    .await
}
