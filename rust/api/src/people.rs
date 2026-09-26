use crate::entities::{account, grant, membership, security_audit_event, user};
use crate::medication_management::{
    error_response, finish, finish_with_request_id, record_version, request_context,
};
use crate::read_entities::{carer_relationship, location_membership, person, stock_location};
use crate::read_resources::{age, serialize_people, today};
use crate::sync_events::{lock_household, record_change, SyncRecord};
use crate::{
    database_error, granted_people, representation_etag, tenant_setting, ApiError, AppState,
    AuthContext, CredentialKind,
};
use axum::extract::{rejection::JsonRejection, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::{NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde_json::{json, Value};
use uuid::Uuid;

struct Attributes {
    name: Option<String>,
    email: Option<Option<String>>,
    date_of_birth: Option<NaiveDate>,
    person_type: Option<i32>,
    has_capacity: Option<bool>,
}

fn valid_identifier(value: &str) -> bool {
    let numeric = value
        .as_bytes()
        .first()
        .is_some_and(|byte| (b'1'..=b'9').contains(byte))
        && value.bytes().all(|byte| byte.is_ascii_digit());
    numeric
        || (value.len() == 36
            && value.bytes().enumerate().all(|(index, byte)| match index {
                8 | 13 | 18 | 23 => byte == b'-',
                19 => matches!(byte, b'8' | b'9' | b'a' | b'b' | b'A' | b'B'),
                _ => byte.is_ascii_hexdigit(),
            }))
}

fn email_valid(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    if local.is_empty()
        || !local
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".!#$%&'*+/=?^_`{|}~-".contains(&byte))
    {
        return false;
    }
    domain.split('.').all(|label| {
        label.len() <= 63
            && label
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_alphanumeric())
            && label
                .bytes()
                .last()
                .is_some_and(|byte| byte.is_ascii_alphanumeric())
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

fn parse_attributes(body: &Value, create: bool) -> Result<Attributes, StatusCode> {
    let outer = body.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    let inner = outer.get("person").ok_or(StatusCode::BAD_REQUEST)?;
    let inner = inner.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    if outer.len() != 1
        || inner.is_empty()
        || inner.keys().any(|key| {
            !matches!(
                key.as_str(),
                "name" | "email" | "date_of_birth" | "person_type" | "has_capacity"
            )
        })
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let name = match inner.get("name") {
        Some(value) => {
            let text = value.as_str().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
            if text.trim().is_empty() {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            Some(text.to_owned())
        }
        None => None,
    };
    let email = match inner.get("email") {
        Some(Value::Null) => Some(None),
        Some(Value::String(value)) => {
            let normalized = value.trim().to_lowercase();
            if !normalized.is_empty() && !email_valid(&normalized) {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            Some((!normalized.is_empty()).then_some(normalized))
        }
        Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        None => None,
    };
    let date_of_birth = match inner.get("date_of_birth") {
        Some(Value::String(value)) => Some(
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?,
        ),
        Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        None => None,
    };
    let person_type = match inner.get("person_type") {
        Some(Value::String(value)) => Some(match value.as_str() {
            "adult" => 0,
            "minor" => 1,
            "dependent_adult" => 2,
            _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        }),
        Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        None => None,
    };
    let has_capacity = match inner.get("has_capacity") {
        Some(Value::Bool(value)) => Some(*value),
        Some(_) => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        None => None,
    };
    if create && (name.is_none() || date_of_birth.is_none()) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    Ok(Attributes {
        name,
        email,
        date_of_birth,
        person_type,
        has_capacity,
    })
}

async fn failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    status: StatusCode,
) -> Result<Response, ApiError> {
    let (code, message) = match status {
        StatusCode::BAD_REQUEST => ("bad_request", "Invalid request body"),
        StatusCode::FORBIDDEN => (
            "forbidden",
            "You are not authorized to perform this action.",
        ),
        StatusCode::NOT_FOUND => ("not_found", "Record not found"),
        _ => ("validation_failed", "Validation failed"),
    };
    let errors =
        (status == StatusCode::UNPROCESSABLE_ENTITY).then(|| json!({"person": ["is invalid"]}));
    error_response(
        db,
        context,
        method,
        "api/v1/people",
        "PersonPolicy",
        action,
        status,
        code,
        message,
        errors,
    )
    .await
}

async fn may_create(
    db: &DatabaseTransaction,
    member: &membership::Model,
) -> Result<bool, ApiError> {
    if matches!(member.role.as_str(), "owner" | "administrator") {
        return Ok(true);
    }
    let active = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(member.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(member.id))
        .filter(grant::Column::AccessLevel.eq("manage"))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        )
        .one(db)
        .await
        .map_err(database_error)?;
    Ok(active.is_some())
}

async fn manageable(
    db: &DatabaseTransaction,
    context: &AuthContext,
    person_id: i64,
) -> Result<bool, ApiError> {
    let active = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::PersonId.eq(person_id))
        .filter(grant::Column::AccessLevel.eq("manage"))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        )
        .one(db)
        .await
        .map_err(database_error)?;
    Ok(active.is_some())
}

fn person_valid(record: &person::Model, has_carer: bool) -> bool {
    if record.name.trim().is_empty() || record.date_of_birth.is_none() {
        return false;
    }
    if record
        .email
        .as_deref()
        .is_some_and(|value| !email_valid(value))
    {
        return false;
    }
    let Some(years) = age(record.date_of_birth, today()) else {
        return false;
    };
    if (years < 18 && record.person_type == 2) || (years >= 18 && record.person_type == 1) {
        return false;
    }
    if !record.has_capacity && !has_carer {
        return false;
    }
    true
}

fn snapshot(record: &person::Model) -> Value {
    json!({"id": record.id, "account_id": record.account_id, "household_id": record.household_id, "portable_id": record.portable_id, "name": record.name, "email": record.email, "date_of_birth": record.date_of_birth, "person_type": record.person_type, "has_capacity": record.has_capacity, "created_at": record.created_at, "updated_at": record.updated_at})
}

fn grant_state(record: &grant::Model) -> Value {
    json!({
        "household_membership_id": record.household_membership_id,
        "person_id": record.person_id,
        "access_level": record.access_level,
        "relationship_type": record.relationship_type,
        "expires_at": record.expires_at,
        "revoked_at": record.revoked_at,
        "carer_relationship_id": record.carer_relationship_id
    })
}

async fn record_grant_event(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    grant: &grant::Model,
    previous_state: Option<Value>,
) -> Result<(), ApiError> {
    let now = Utc::now().naive_utc();
    let (authentication_method, session_reference) = match context.credential_kind {
        CredentialKind::ApiSession => (
            "api_session",
            format!("api_session:{}", context.credential_reference),
        ),
        CredentialKind::ApiAppToken => (
            "api_app_token",
            format!("api_app_token:{}", context.credential_reference),
        ),
        CredentialKind::OauthGrant => (
            "oauth",
            format!("oauth_grant:{}", context.credential_reference),
        ),
        CredentialKind::BrowserSession => (
            "browser_session",
            format!("browser_session:{}", context.credential_reference),
        ),
    };
    security_audit_event::ActiveModel {
        household_id: Set(context.membership.household_id),
        actor_account_id: Set(Some(context.account_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        event_type: Set("household_access.person_grant_changed".to_owned()),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(json!({
            "target_membership_id": grant.household_membership_id,
            "target_grant_id": grant.id,
            "previous_state": previous_state,
            "new_state": grant_state(grant),
            "outcome": "success"
        })),
        audit_context: Set(json!({
            "actor_account_id": context.account_id,
            "actor_user_id": context.user_id,
            "actor_membership_id": context.membership.id,
            "active_role": context.membership.role,
            "permissions_version": context.membership.permissions_version,
            "household_id": context.membership.household_id,
            "authentication_method": authentication_method,
            "session_reference": session_reference,
            "request_id": request_id
        })),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    Ok(())
}

async fn representation(
    db: &DatabaseTransaction,
    record: person::Model,
) -> Result<(Value, String), ApiError> {
    let row = serialize_people(db, vec![record]).await?.remove(0);
    let body = json!({"data": row});
    let etag = representation_etag(&body);
    Ok((body, etag))
}

async fn carer_exists(db: &DatabaseTransaction, person_id: i64) -> Result<bool, ApiError> {
    let relationship = carer_relationship::Entity::find()
        .filter(carer_relationship::Column::PatientId.eq(person_id))
        .filter(carer_relationship::Column::Active.eq(true))
        .one(db)
        .await
        .map_err(database_error)?;
    Ok(relationship.is_some())
}

pub(super) async fn show_me(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let account = account::Entity::find_by_id(context.account_id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    let user = user::Entity::find_by_id(context.user_id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    let record = person::Entity::find_by_id(user.person_id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    tenant_setting(&db, "med_tracker.current_household_id", record.household_id)
        .await
        .map_err(database_error)?;
    let own = serialize_people(&db, vec![record]).await?.remove(0);
    tenant_setting(&db, "med_tracker.current_household_id", household_id)
        .await
        .map_err(database_error)?;
    let status = match account.status {
        1 => "unverified",
        2 => "verified",
        3 => "closed",
        _ => return Err(ApiError::internal()),
    };
    let body = json!({"data": {"id": user.id, "email_address": user.email_address, "membership_role": context.membership.role, "active": user.active, "person": own, "account": {"id": account.id, "email": account.email, "status": status}}});
    finish(
        db,
        &context,
        "GET",
        "api/v1/me",
        "MePolicy",
        "show",
        StatusCode::OK,
        true,
        body,
        None,
    )
    .await
}

pub(super) async fn create(
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
        Err(status) => return failure(db, &context, "POST", "create", status).await,
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

async fn update(
    state: AppState,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
    method: &str,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !valid_identifier(&id) {
        return failure(db, &context, method, "update", StatusCode::BAD_REQUEST).await;
    }
    let query = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(household_id))
        .filter(person::Column::Id.in_subquery(granted_people(&context.membership)));
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(person::Column::Id.eq(id))
    } else {
        query.filter(person::Column::PortableId.eq(&id))
    };
    let Some(found) = query.one(&db).await.map_err(database_error)? else {
        return failure(db, &context, method, "update", StatusCode::NOT_FOUND).await;
    };
    if !manageable(&db, &context, found.id).await? {
        return failure(db, &context, method, "update", StatusCode::FORBIDDEN).await;
    }
    let Json(body) = match payload {
        Ok(body) => body,
        Err(_) => return failure(db, &context, method, "update", StatusCode::BAD_REQUEST).await,
    };
    let attrs = match parse_attributes(&body, false) {
        Ok(attrs) => attrs,
        Err(status) => return failure(db, &context, method, "update", status).await,
    };
    lock_household(&db, household_id).await?;
    let Some(record) = person::Entity::find_by_id(found.id)
        .lock_exclusive()
        .one(&db)
        .await
        .map_err(database_error)?
    else {
        return failure(db, &context, method, "update", StatusCode::NOT_FOUND).await;
    };
    let before = snapshot(&record);
    let mut active: person::ActiveModel = record.clone().into();
    if let Some(value) = attrs.name {
        active.name = Set(value)
    }
    if let Some(value) = attrs.email {
        active.email = Set(value)
    }
    if let Some(value) = attrs.date_of_birth {
        active.date_of_birth = Set(Some(value))
    }
    if let Some(value) = attrs.person_type {
        active.person_type = Set(value)
    }
    if let Some(value) = attrs.has_capacity {
        active.has_capacity = Set(value)
    }
    let Some(years) = age(active.date_of_birth.clone().take().flatten(), today()) else {
        return failure(
            db,
            &context,
            method,
            "update",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await;
    };
    let person_type = active
        .person_type
        .clone()
        .take()
        .ok_or_else(ApiError::internal)?;
    if (years < 18 && person_type == 1) || (years >= 18 && person_type == 2) {
        active.has_capacity = Set(false)
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
        if !person_valid(&record, carer_exists(&db, record.id).await?) {
            return failure(
                db,
                &context,
                method,
                "update",
                StatusCode::UNPROCESSABLE_ENTITY,
            )
            .await;
        }
        let (body, etag) = representation(&db, record).await?;
        return finish(
            db,
            &context,
            method,
            "api/v1/people",
            "PersonPolicy",
            "update",
            StatusCode::OK,
            true,
            body,
            Some(&etag),
        )
        .await;
    }
    active.updated_at = Set(Utc::now().naive_utc());
    let savepoint = db.begin().await.map_err(database_error)?;
    let record = match active.update(&savepoint).await {
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
                method,
                "update",
                StatusCode::UNPROCESSABLE_ENTITY,
            )
            .await;
        }
        Err(error) => return Err(database_error(error)),
    };
    if !person_valid(&record, carer_exists(&savepoint, record.id).await?) {
        savepoint.rollback().await.map_err(database_error)?;
        return failure(
            db,
            &context,
            method,
            "update",
            StatusCode::UNPROCESSABLE_ENTITY,
        )
        .await;
    }
    savepoint.commit().await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "Person",
        record.id,
        "update",
        Some(before),
        Some(snapshot(&record)),
    )
    .await?;
    record_change(
        &db,
        &context,
        &request_id,
        SyncRecord {
            record_type: "Person",
            record_id: record.id,
            portable_id: &record.portable_id,
            action: "update",
            person_portable_id: Some(&record.portable_id),
        },
    )
    .await?;
    let (body, etag) = representation(&db, record).await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        method,
        "api/v1/people",
        "PersonPolicy",
        "update",
        StatusCode::OK,
        true,
        body,
        Some(&etag),
    )
    .await
}

pub(super) async fn patch(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    update(state, household_id, id, headers, payload, "PATCH").await
}

pub(super) async fn put(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    update(state, household_id, id, headers, payload, "PUT").await
}
