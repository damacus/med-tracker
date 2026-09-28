use crate::entities::{
    account, active_storage_attachment, active_storage_blob, grant, person, security_audit_event,
};
use crate::medication_management::{
    error_response, finish, finish_with_request_id, record_version, request_context,
};
use crate::mutation_idempotency::{self, Lookup, StoredResponse};
use crate::read_resources::{age, today};
use crate::sync_events::{lock_household, record_change, SyncRecord};
use crate::{authenticate, database_error, ApiError, AppState, AuthContext};
use axum::body::Body;
use axum::extract::multipart::MultipartRejection;
use axum::extract::{rejection::JsonRejection, DefaultBodyLimit, Multipart, Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{extract::Request, Json, Router};
use chrono::{NaiveDate, Utc};
use md5::{Digest, Md5};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, QueryFilter,
    QuerySelect, Set, TransactionTrait,
};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

const CONTROLLER: &str = "api/v1/profiles";
const POLICY: &str = "PersonPolicy";
const AVATAR_CONTROLLER: &str = "api/v1/profile_avatars";
const MAX_AVATAR_BYTES: usize = 5 * 1024 * 1024;
const SHORTCUTS: [&str; 9] = [
    "dashboard",
    "inventory",
    "locations",
    "people",
    "finder",
    "medicine_reviews",
    "reports",
    "profile",
    "administration",
];

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/households/{household_id}/profile",
            get(show).patch(patch).put(put),
        )
        .route(
            "/api/v1/households/{household_id}/profile/avatar",
            get(download).put(upload).delete(remove),
        )
        .layer(DefaultBodyLimit::max(MAX_AVATAR_BYTES + 64 * 1024))
        .layer(middleware::from_fn(no_store))
}

#[derive(Clone)]
pub(super) struct AvatarStorage {
    root: Arc<PathBuf>,
    service_name: String,
}

impl AvatarStorage {
    pub(super) fn from_env() -> Result<Self, String> {
        let root =
            std::env::var("ACTIVE_STORAGE_ROOT").unwrap_or_else(|_| "/app/storage".to_owned());
        let root = PathBuf::from(root);
        if !root.is_absolute() {
            return Err("ACTIVE_STORAGE_ROOT must be absolute".to_owned());
        }
        let explicit = std::env::var("ACTIVE_STORAGE_SERVICE_NAME").ok();
        let existing = std::env::var("ACTIVE_STORAGE_SERVICE").ok();
        let service_name =
            Self::service_name_from_values(explicit.as_deref(), existing.as_deref())?;
        Ok(Self {
            root: Arc::new(root),
            service_name,
        })
    }

    fn service_name_from_values(
        explicit: Option<&str>,
        existing: Option<&str>,
    ) -> Result<String, String> {
        let service_name = explicit.or(existing).unwrap_or("persistent");
        if !matches!(service_name, "test" | "local" | "persistent") {
            return Err(
                "ACTIVE_STORAGE_SERVICE_NAME or ACTIVE_STORAGE_SERVICE must name a disk service"
                    .to_owned(),
            );
        }
        Ok(service_name.to_owned())
    }

    fn path(&self, key: &str) -> Option<PathBuf> {
        if key.len() < 4 || !key.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
            return None;
        }
        Some(self.root.join(&key[..2]).join(&key[2..4]).join(key))
    }
}

async fn no_store(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

struct Attributes {
    date_of_birth: Option<NaiveDate>,
    time_zone: Option<String>,
    gravatar_enabled: Option<bool>,
    mobile_shortcuts: Option<Vec<String>>,
}

fn parse(body: &Value) -> Result<Attributes, (StatusCode, &'static str)> {
    let outer = body
        .as_object()
        .ok_or((StatusCode::BAD_REQUEST, "bad_request"))?;
    let inner = outer
        .get("profile")
        .and_then(Value::as_object)
        .ok_or((StatusCode::BAD_REQUEST, "bad_request"))?;
    if outer.len() != 1
        || inner.keys().any(|key| {
            !matches!(
                key.as_str(),
                "date_of_birth" | "time_zone" | "gravatar_enabled" | "mobile_shortcuts"
            )
        })
    {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "unprocessable_content"));
    }
    let invalid = (StatusCode::UNPROCESSABLE_ENTITY, "unprocessable_content");
    let invalid_value = (StatusCode::UNPROCESSABLE_ENTITY, "validation_failed");
    let date_of_birth = match inner.get("date_of_birth") {
        None => None,
        Some(Value::String(value)) if value.len() == 10 => {
            Some(NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| invalid)?)
        }
        Some(_) => return Err(invalid),
    };
    let time_zone = match inner.get("time_zone") {
        None => None,
        Some(Value::String(value)) => {
            if !value.is_empty() && value.parse::<chrono_tz::Tz>().is_err() {
                return Err(invalid_value);
            }
            Some(value.clone())
        }
        Some(_) => return Err(invalid),
    };
    let gravatar_enabled = match inner.get("gravatar_enabled") {
        None => None,
        Some(Value::Bool(value)) => Some(*value),
        Some(_) => return Err(invalid),
    };
    let mobile_shortcuts = match inner.get("mobile_shortcuts") {
        None => None,
        Some(Value::Array(items)) => {
            if !(1..=3).contains(&items.len()) {
                return Err(invalid_value);
            }
            let mut values = Vec::with_capacity(items.len());
            for item in items {
                let value = item.as_str().ok_or(invalid)?;
                if !SHORTCUTS.contains(&value) || values.iter().any(|seen| seen == value) {
                    return Err(invalid_value);
                }
                values.push(value.to_owned());
            }
            Some(values)
        }
        Some(_) => return Err(invalid),
    };
    Ok(Attributes {
        date_of_birth,
        time_zone,
        gravatar_enabled,
        mobile_shortcuts,
    })
}

async fn self_person(
    db: &DatabaseTransaction,
    context: &AuthContext,
    update: bool,
) -> Result<Option<person::Model>, ApiError> {
    let row = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(context.membership.household_id))
        .filter(person::Column::Id.eq(context.membership.person_id))
        .filter(person::Column::AccountId.eq(context.account_id))
        .one(db)
        .await
        .map_err(database_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let mut query = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::PersonId.eq(row.id))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(grant::Column::ExpiresAt.gt(Utc::now().naive_utc())),
        );
    query = if update {
        query.filter(grant::Column::AccessLevel.eq("manage"))
    } else {
        query.filter(grant::Column::AccessLevel.is_in(["view", "record", "manage"]))
    };
    if query.one(db).await.map_err(database_error)?.is_none() {
        return Err(ApiError::forbidden());
    }
    Ok(Some(row))
}

async fn account_row(
    db: &DatabaseTransaction,
    context: &AuthContext,
) -> Result<account::Model, ApiError> {
    account::Entity::find_by_id(context.account_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)
}

async fn response_value(
    db: &DatabaseTransaction,
    person: &person::Model,
    account: &account::Model,
) -> Result<Value, ApiError> {
    let attachment = active_storage_attachment::Entity::find()
        .filter(active_storage_attachment::Column::HouseholdId.eq(person.household_id))
        .filter(active_storage_attachment::Column::RecordType.eq("Person"))
        .filter(active_storage_attachment::Column::RecordId.eq(person.id))
        .filter(active_storage_attachment::Column::Name.eq("avatar"))
        .one(db)
        .await
        .map_err(database_error)?;
    let preferences = &account.preferences;
    let time_zone = preferences
        .get("time_zone")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or("UTC");
    let gravatar_enabled = match preferences.get("gravatar_enabled") {
        Some(Value::Bool(value)) => *value,
        Some(Value::String(value)) => matches!(value.as_str(), "true" | "1"),
        _ => false,
    };
    let mobile_shortcuts = preferences
        .get("mobile_shortcuts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| {
            json!(["dashboard", "inventory", "finder"])
                .as_array()
                .unwrap()
                .clone()
        });
    Ok(json!({"data": {
        "person_id": person.id.to_string(),
        "account_id": account.id.to_string(),
        "date_of_birth": person.date_of_birth,
        "time_zone": time_zone,
        "gravatar_enabled": gravatar_enabled,
        "mobile_shortcuts": mobile_shortcuts,
        "avatar_attached": attachment.is_some()
    }}))
}

fn person_snapshot(person: &person::Model) -> Value {
    json!({
        "id": person.id,
        "account_id": person.account_id,
        "household_id": person.household_id,
        "portable_id": person.portable_id,
        "name": person.name,
        "email": person.email,
        "date_of_birth": person.date_of_birth,
        "person_type": person.person_type,
        "has_capacity": person.has_capacity,
        "created_at": person.created_at,
        "updated_at": person.updated_at
    })
}

struct ProfileKey<'a> {
    key: &'a str,
    path: String,
    digest: String,
}

async fn profile_failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    status: StatusCode,
    code: &str,
    key: Option<&ProfileKey<'_>>,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let body = json!({"error": {
        "code":code,
        "message":"Profile attributes are invalid",
        "request_id":request_id
    }});
    if let Some(key) = key {
        mutation_idempotency::store(
            &db,
            context,
            StoredResponse {
                key: key.key,
                method,
                path: &key.path,
                digest: &key.digest,
                status,
                body: body.clone(),
                request_id: &request_id,
                etag: None,
            },
        )
        .await?;
    }
    finish_with_request_id(
        db,
        context,
        &request_id,
        method,
        CONTROLLER,
        POLICY,
        "update",
        status,
        false,
        body,
        None,
    )
    .await
}

pub(super) async fn show(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let person = match self_person(&db, &context, false).await? {
        Some(row) => row,
        None => {
            return error_response(
                db,
                &context,
                "GET",
                CONTROLLER,
                POLICY,
                "show",
                StatusCode::NOT_FOUND,
                "not_found",
                "Record not found",
                None,
            )
            .await;
        }
    };
    let account = account_row(&db, &context).await?;
    let body = response_value(&db, &person, &account).await?;
    finish(
        db,
        &context,
        "GET",
        CONTROLLER,
        POLICY,
        "show",
        StatusCode::OK,
        true,
        body,
        None,
    )
    .await
}

async fn update(
    state: AppState,
    household_id: i64,
    headers: HeaderMap,
    input: Result<Json<Value>, JsonRejection>,
    method: &str,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    match self_person(&db, &context, true).await? {
        Some(_) => (),
        None => {
            return error_response(
                db,
                &context,
                method,
                CONTROLLER,
                POLICY,
                "update",
                StatusCode::NOT_FOUND,
                "not_found",
                "Record not found",
                None,
            )
            .await;
        }
    };
    lock_household(&db, household_id).await?;
    let current = authenticate(&state, &db, &headers, household_id).await?;
    let person = self_person(&db, &current, true)
        .await?
        .ok_or_else(ApiError::forbidden)?;
    let body = match input {
        Ok(Json(value)) => value,
        Err(_) => {
            return error_response(
                db,
                &current,
                method,
                CONTROLLER,
                POLICY,
                "update",
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Invalid request body",
                None,
            )
            .await;
        }
    };
    let path = format!("/api/v1/households/{household_id}/profile");
    let key = mutation_idempotency::key(&headers).map(|key| ProfileKey {
        key,
        digest: mutation_idempotency::digest(method, &path, &body),
        path,
    });
    if let Some(key) = key.as_ref() {
        match mutation_idempotency::lookup(&db, &current, key.key, method, &key.path, &key.digest)
            .await?
        {
            Lookup::New => {}
            Lookup::Replay(saved) => {
                let replay_id = Uuid::new_v4().to_string();
                let status = StatusCode::from_u16(saved.response_status as u16)
                    .map_err(|_| ApiError::internal())?;
                crate::audit::record_resource_request_with_id(
                    &db,
                    &current,
                    &replay_id,
                    method,
                    CONTROLLER,
                    POLICY,
                    "update",
                    status,
                    status.is_success(),
                )
                .await
                .map_err(database_error)?;
                db.commit().await.map_err(database_error)?;
                let mut saved = *saved;
                if status.is_client_error() && saved.response_body.get("error").is_some() {
                    saved.response_body["error"]["request_id"] = json!(replay_id);
                }
                let mut response = mutation_idempotency::replay(saved)?;
                response.headers_mut().insert(
                    "x-request-id",
                    HeaderValue::from_str(&replay_id).map_err(|_| ApiError::internal())?,
                );
                return Ok(response);
            }
            Lookup::Conflict => {
                return error_response(
                    db,
                    &current,
                    method,
                    CONTROLLER,
                    POLICY,
                    "update",
                    StatusCode::CONFLICT,
                    "idempotency_key_reused",
                    "Idempotency key has already been used for a different request",
                    None,
                )
                .await;
            }
        }
    }
    let attrs = match parse(&body) {
        Ok(value) => value,
        Err((status, code)) => {
            return profile_failure(db, &current, method, status, code, key.as_ref()).await;
        }
    };
    let mut account = account::Entity::find_by_id(current.account_id)
        .lock_exclusive()
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::unauthorized)?;
    let mut person = person::Entity::find_by_id(person.id)
        .lock_exclusive()
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    if person.account_id != Some(current.account_id)
        || person.household_id != current.membership.household_id
    {
        return Err(ApiError::forbidden());
    }
    let request_id = Uuid::new_v4().to_string();
    let mut prefs = account.preferences.as_object().cloned().unwrap_or_default();
    if let Some(date) = attrs.date_of_birth {
        if person.date_of_birth != Some(date) {
            let years = age(Some(date), today()).ok_or_else(ApiError::internal)?;
            if (years < 18 && person.person_type == 2) || (years >= 18 && person.person_type == 1) {
                return profile_failure(
                    db,
                    &current,
                    method,
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "validation_failed",
                    key.as_ref(),
                )
                .await;
            }
            let before = person_snapshot(&person);
            let mut active: person::ActiveModel = person.into();
            active.date_of_birth = Set(Some(date));
            active.updated_at = Set(Utc::now().naive_utc());
            person = active.update(&db).await.map_err(database_error)?;
            record_version(
                &db,
                &current,
                &request_id,
                "Person",
                person.id,
                "update",
                Some(before),
                Some(person_snapshot(&person)),
            )
            .await?;
            record_change(
                &db,
                &current,
                &request_id,
                SyncRecord {
                    record_type: "Person",
                    record_id: person.id,
                    portable_id: &person.portable_id,
                    action: "update",
                    person_portable_id: Some(&person.portable_id),
                },
            )
            .await?;
        }
    }
    if let Some(value) = attrs.time_zone {
        prefs.insert("time_zone".to_owned(), json!(value));
    }
    if let Some(value) = attrs.gravatar_enabled {
        prefs.insert("gravatar_enabled".to_owned(), json!(value));
    }
    if let Some(value) = attrs.mobile_shortcuts {
        prefs.insert("mobile_shortcuts".to_owned(), json!(value));
    }
    let prefs = Value::Object(prefs);
    if prefs != account.preferences {
        let mut active: account::ActiveModel = account.into();
        active.preferences = Set(prefs);
        active.updated_at = Set(Utc::now().naive_utc());
        account = active.update(&db).await.map_err(database_error)?;
    }
    let response = response_value(&db, &person, &account).await?;
    if let Some(key) = key.as_ref() {
        mutation_idempotency::store(
            &db,
            &current,
            StoredResponse {
                key: key.key,
                method,
                path: &key.path,
                digest: &key.digest,
                status: StatusCode::OK,
                body: response.clone(),
                request_id: &request_id,
                etag: None,
            },
        )
        .await?;
    }
    finish_with_request_id(
        db,
        &current,
        &request_id,
        method,
        CONTROLLER,
        POLICY,
        "update",
        StatusCode::OK,
        true,
        response,
        None,
    )
    .await
}

pub(super) async fn patch(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    input: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    update(state, household_id, headers, input, "PATCH").await
}

pub(super) async fn put(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    input: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    update(state, household_id, headers, input, "PUT").await
}

async fn avatar_failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    status: StatusCode,
    code: &str,
    message: &str,
) -> Result<Response, ApiError> {
    error_response(
        db,
        context,
        method,
        AVATAR_CONTROLLER,
        POLICY,
        match method {
            "GET" => "show",
            "PUT" => "update",
            _ => "destroy",
        },
        status,
        code,
        message,
        None,
    )
    .await
}

async fn avatar_attachment(
    db: &DatabaseTransaction,
    person: &person::Model,
) -> Result<Option<active_storage_attachment::Model>, ApiError> {
    active_storage_attachment::Entity::find()
        .filter(active_storage_attachment::Column::HouseholdId.eq(person.household_id))
        .filter(active_storage_attachment::Column::RecordType.eq("Person"))
        .filter(active_storage_attachment::Column::RecordId.eq(person.id))
        .filter(active_storage_attachment::Column::Name.eq("avatar"))
        .one(db)
        .await
        .map_err(database_error)
}

async fn detach_avatar(
    db: &DatabaseTransaction,
    storage: &AvatarStorage,
    attachment: active_storage_attachment::Model,
) -> Result<Option<PathBuf>, ApiError> {
    let blob = active_storage_blob::Entity::find_by_id(attachment.blob_id)
        .lock_exclusive()
        .one(db)
        .await
        .map_err(database_error)?;
    active_storage_attachment::Entity::delete_by_id(attachment.id)
        .exec(db)
        .await
        .map_err(database_error)?;
    let Some(blob) = blob else {
        return Ok(None);
    };
    if blob.service_name != storage.service_name {
        return Ok(None);
    }
    let savepoint = db.begin().await.map_err(database_error)?;
    let deleted = active_storage_blob::Entity::delete_by_id(blob.id)
        .exec(&savepoint)
        .await;
    if deleted.is_err() {
        savepoint.rollback().await.map_err(database_error)?;
        return Ok(None);
    }
    savepoint.commit().await.map_err(database_error)?;
    Ok(storage.path(&blob.key))
}

async fn remove_retired_file(path: Option<PathBuf>) {
    if let Some(path) = path {
        let _ = tokio::task::spawn_blocking(move || std::fs::remove_file(path)).await;
    }
}

async fn avatar_event(
    db: &DatabaseTransaction,
    context: &AuthContext,
    person: &person::Model,
    action: &str,
    request_id: &str,
) -> Result<(), ApiError> {
    security_audit_event::ActiveModel {
        household_id: Set(person.household_id),
        actor_account_id: Set(Some(context.account_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        event_type: Set(format!("profile.avatar.{action}")),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(json!({"person_id":person.id})),
        audit_context: Set(json!({
            "actor_account_id":context.account_id,
            "actor_membership_id":context.membership.id,
            "household_id":person.household_id,
            "request_id":request_id
        })),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    Ok(())
}

fn avatar_path(storage: &AvatarStorage, key: &str) -> Result<PathBuf, ApiError> {
    storage.path(key).ok_or_else(ApiError::internal)
}

pub(super) async fn download(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let Some(person) = self_person(&db, &context, false).await? else {
        return avatar_failure(
            db,
            &context,
            "GET",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    let Some(attachment) = avatar_attachment(&db, &person).await? else {
        return avatar_failure(
            db,
            &context,
            "GET",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    };
    let blob = active_storage_blob::Entity::find_by_id(attachment.blob_id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    if blob.service_name != state.avatar_storage.service_name {
        return avatar_failure(
            db,
            &context,
            "GET",
            StatusCode::SERVICE_UNAVAILABLE,
            "avatar_unavailable",
            "Avatar is temporarily unavailable",
        )
        .await;
    }
    let path = avatar_path(&state.avatar_storage, &blob.key)?;
    let bytes = match tokio::task::spawn_blocking(move || std::fs::read(path)).await {
        Ok(Ok(value)) => value,
        Ok(Err(_)) | Err(_) => {
            return avatar_failure(
                db,
                &context,
                "GET",
                StatusCode::SERVICE_UNAVAILABLE,
                "avatar_unavailable",
                "Avatar is temporarily unavailable",
            )
            .await;
        }
    };
    let request_id = Uuid::new_v4().to_string();
    crate::audit::record_resource_request_with_id(
        &db,
        &context,
        &request_id,
        "GET",
        AVATAR_CONTROLLER,
        POLICY,
        "show",
        StatusCode::OK,
        true,
    )
    .await
    .map_err(database_error)?;
    db.commit().await.map_err(database_error)?;
    let mut response = (StatusCode::OK, Body::from(bytes)).into_response();
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
    );
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(
            blob.content_type
                .as_deref()
                .unwrap_or("application/octet-stream"),
        )
        .map_err(|_| ApiError::internal())?,
    );
    let filename = blob.filename.replace(['\r', '\n', '"'], "_");
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("inline; filename=\"{filename}\""))
            .map_err(|_| ApiError::internal())?,
    );
    Ok(response)
}

struct UploadedAvatar {
    bytes: Vec<u8>,
    filename: String,
    content_type: String,
}

enum UploadError {
    BadRequest,
    InvalidAttributes,
    ValidationFailed,
}

async fn uploaded_avatar(
    input: Result<Multipart, MultipartRejection>,
) -> Result<UploadedAvatar, UploadError> {
    let mut multipart = input.map_err(|_| UploadError::BadRequest)?;
    let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| UploadError::BadRequest)?
    else {
        return Err(UploadError::BadRequest);
    };
    if field.name() != Some("avatar") {
        return Err(UploadError::BadRequest);
    }
    let filename = field
        .file_name()
        .ok_or(UploadError::InvalidAttributes)?
        .to_owned();
    let content_type = field
        .content_type()
        .ok_or(UploadError::ValidationFailed)?
        .to_owned();
    let bytes = field
        .bytes()
        .await
        .map_err(|_| UploadError::BadRequest)?
        .to_vec();
    if multipart
        .next_field()
        .await
        .map_err(|_| UploadError::BadRequest)?
        .is_some()
    {
        return Err(UploadError::ValidationFailed);
    }
    if filename.is_empty()
        || !matches!(
            content_type.as_str(),
            "image/png" | "image/jpeg" | "image/webp"
        )
        || bytes.len() > MAX_AVATAR_BYTES
    {
        return Err(UploadError::ValidationFailed);
    }
    Ok(UploadedAvatar {
        bytes,
        filename,
        content_type,
    })
}

fn write_avatar(storage: &AvatarStorage, key: &str, bytes: &[u8]) -> std::io::Result<PathBuf> {
    let path = storage
        .path(key)
        .ok_or_else(|| std::io::Error::other("invalid avatar key"))?;
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("avatar path has no parent"))?;
    std::fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".{key}.{}", Uuid::new_v4().simple()));
    let write = std::fs::write(&temporary, bytes).and_then(|_| std::fs::rename(&temporary, &path));
    if write.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    write.map(|()| path)
}

pub(super) async fn upload(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    input: Result<Multipart, MultipartRejection>,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if self_person(&db, &context, true).await?.is_none() {
        return avatar_failure(
            db,
            &context,
            "PUT",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    }
    let upload = match uploaded_avatar(input).await {
        Ok(value) => value,
        Err(UploadError::BadRequest) => {
            return avatar_failure(
                db,
                &context,
                "PUT",
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Invalid request body",
            )
            .await
        }
        Err(UploadError::InvalidAttributes) => {
            return avatar_failure(
                db,
                &context,
                "PUT",
                StatusCode::UNPROCESSABLE_ENTITY,
                "unprocessable_content",
                "Profile attributes are invalid",
            )
            .await
        }
        Err(UploadError::ValidationFailed) => {
            return avatar_failure(
                db,
                &context,
                "PUT",
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Avatar is invalid",
            )
            .await
        }
    };
    lock_household(&db, household_id).await?;
    let current = authenticate(&state, &db, &headers, household_id).await?;
    let person = self_person(&db, &current, true)
        .await?
        .ok_or_else(ApiError::forbidden)?;
    let account = account_row(&db, &current).await?;
    let key = Uuid::new_v4().simple().to_string();
    let request_id = Uuid::new_v4().to_string();
    let storage = state.avatar_storage.clone();
    let key_for_write = key.clone();
    let bytes_for_write = upload.bytes.clone();
    let path = match tokio::task::spawn_blocking(move || {
        write_avatar(&storage, &key_for_write, &bytes_for_write)
    })
    .await
    {
        Ok(Ok(path)) => path,
        Ok(Err(_)) | Err(_) => {
            return avatar_failure(
                db,
                &current,
                "PUT",
                StatusCode::SERVICE_UNAVAILABLE,
                "avatar_unavailable",
                "Avatar is temporarily unavailable",
            )
            .await
        }
    };
    let result = save_avatar(
        &db,
        &current,
        &person,
        &account,
        &state.avatar_storage,
        key,
        upload,
        &request_id,
    )
    .await;
    let response = match result {
        Ok((body, retired)) => finish_with_request_id(
            db,
            &current,
            &request_id,
            "PUT",
            AVATAR_CONTROLLER,
            POLICY,
            "update",
            StatusCode::OK,
            true,
            body,
            None,
        )
        .await
        .map(|response| (response, retired)),
        Err(error) => Err(error),
    };
    match response {
        Ok((response, retired)) => {
            remove_retired_file(retired).await;
            Ok(response)
        }
        Err(error) => {
            remove_retired_file(Some(path)).await;
            Err(error)
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn save_avatar(
    db: &DatabaseTransaction,
    context: &AuthContext,
    person: &person::Model,
    account: &account::Model,
    storage: &AvatarStorage,
    key: String,
    upload: UploadedAvatar,
    request_id: &str,
) -> Result<(Value, Option<PathBuf>), ApiError> {
    let checksum = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        Md5::digest(&upload.bytes),
    );
    let now = Utc::now().naive_utc();
    let blob = active_storage_blob::ActiveModel {
        key: Set(key),
        filename: Set(upload.filename),
        content_type: Set(Some(upload.content_type)),
        byte_size: Set(upload.bytes.len() as i64),
        checksum: Set(Some(checksum)),
        metadata: Set(Some("{}".to_owned())),
        service_name: Set(storage.service_name.clone()),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    let retired = match avatar_attachment(db, person).await? {
        Some(old) => detach_avatar(db, storage, old).await?,
        None => None,
    };
    active_storage_attachment::ActiveModel {
        name: Set("avatar".to_owned()),
        record_type: Set("Person".to_owned()),
        record_id: Set(person.id),
        blob_id: Set(blob.id),
        household_id: Set(person.household_id),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    avatar_event(db, context, person, "updated", request_id).await?;
    Ok((response_value(db, person, account).await?, retired))
}

pub(super) async fn remove(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if self_person(&db, &context, true).await?.is_none() {
        return avatar_failure(
            db,
            &context,
            "DELETE",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
        )
        .await;
    }
    lock_household(&db, household_id).await?;
    let current = authenticate(&state, &db, &headers, household_id).await?;
    let person = self_person(&db, &current, true)
        .await?
        .ok_or_else(ApiError::forbidden)?;
    let retired = match avatar_attachment(&db, &person).await? {
        Some(attachment) => detach_avatar(&db, &state.avatar_storage, attachment).await?,
        None => None,
    };
    let request_id = Uuid::new_v4().to_string();
    avatar_event(&db, &current, &person, "removed", &request_id).await?;
    let mut response = finish_with_request_id(
        db,
        &current,
        &request_id,
        "DELETE",
        AVATAR_CONTROLLER,
        POLICY,
        "destroy",
        StatusCode::NO_CONTENT,
        true,
        json!({}),
        None,
    )
    .await?;
    *response.body_mut() = Body::empty();
    response.headers_mut().remove(header::CONTENT_TYPE);
    remove_retired_file(retired).await;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::AvatarStorage;

    #[test]
    fn existing_storage_service_config_is_used_and_unsupported_services_fail() {
        assert_eq!(
            AvatarStorage::service_name_from_values(None, Some("persistent")).unwrap(),
            "persistent"
        );
        assert_eq!(
            AvatarStorage::service_name_from_values(Some("test"), Some("persistent")).unwrap(),
            "test"
        );
        assert!(AvatarStorage::service_name_from_values(None, Some("amazon")).is_err());
        assert!(AvatarStorage::service_name_from_values(None, Some("mirror")).is_err());
    }
}
