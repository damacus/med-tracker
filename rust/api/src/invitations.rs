use crate::entities::{
    api_change_event, grant, household, household_invitation, household_invitation_grant,
    membership, security_audit_event, version,
};
use crate::medication_management::{
    error_response, finish, finish_with_request_id, household_manager, record_version,
    request_context,
};
use crate::mutation_idempotency::{self, Lookup, StoredResponse};
use crate::read_entities::{carer_relationship, person};
use crate::{
    audit, auth_sessions, database_error, restricted_role, tenant_setting, ApiError, AppState,
    AuthContext, CredentialKind,
};
use axum::body::{Body, Bytes};
use axum::extract::{rejection::JsonRejection, Path, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{Duration, NaiveDateTime, Utc};
use lettre::message::{header::ContentType, Mailbox};
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::{Message, SmtpTransport, Transport};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait,
    IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, Statement,
    TransactionTrait,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::time::Duration as StdDuration;
use url::Url;
use uuid::Uuid;

const CONTROLLER: &str = "api/v1/admin/invitations";
const POLICY: &str = "HouseholdInvitationPolicy";

#[derive(Clone)]
pub(super) struct MailConfig {
    smtp_host: Option<String>,
    smtp_port: u16,
    smtp_username: Option<String>,
    smtp_password: Option<String>,
    authentication: String,
    starttls: bool,
    app_url: Url,
    from: Mailbox,
}

fn smtp_send(config: MailConfig, email: String, token: String) -> Result<(), ()> {
    let host = config.smtp_host.ok_or(())?;
    let recipient = email.parse::<Mailbox>().map_err(|_| ())?;
    let url = format!(
        "{}/invitations/accept?token={token}",
        config.app_url.as_str().trim_end_matches('/')
    );
    let message = Message::builder()
        .from(config.from)
        .to(recipient)
        .subject("MedTracker invitation")
        .header(ContentType::TEXT_PLAIN)
        .body(format!("You have been invited to MedTracker.\n\nAccept invitation:\n{url}\n\nThis invitation expires in seven days.\n"))
        .map_err(|_| ())?;
    let mut builder = if config.starttls {
        SmtpTransport::starttls_relay(&host).map_err(|_| ())?
    } else {
        SmtpTransport::builder_dangerous(&host)
    }
    .port(config.smtp_port)
    .timeout(Some(StdDuration::from_secs(5)));
    if let (Some(username), Some(password)) = (config.smtp_username, config.smtp_password) {
        let mechanism = match config.authentication.as_str() {
            "login" => Mechanism::Login,
            "plain" => Mechanism::Plain,
            _ => return Err(()),
        };
        builder = builder
            .credentials(Credentials::new(username, password))
            .authentication(vec![mechanism]);
    }
    builder.build().send(&message).map_err(|_| ())?;
    Ok(())
}

impl MailConfig {
    pub(super) fn from_env() -> Result<Self, String> {
        let smtp_host = std::env::var("SMTP_ADDRESS")
            .ok()
            .filter(|value| !value.is_empty());
        let smtp_port = std::env::var("SMTP_PORT")
            .unwrap_or_else(|_| "587".to_owned())
            .parse::<u16>()
            .map_err(|_| "SMTP_PORT must be a valid port".to_owned())?;
        let smtp_username = std::env::var("SMTP_USER_NAME")
            .ok()
            .filter(|value| !value.is_empty());
        let smtp_password = std::env::var("SMTP_PASSWORD")
            .ok()
            .filter(|value| !value.is_empty());
        if smtp_username.is_some() != smtp_password.is_some() {
            return Err("SMTP credentials must be configured together".to_owned());
        }
        let authentication =
            std::env::var("SMTP_AUTHENTICATION").unwrap_or_else(|_| "plain".to_owned());
        if !matches!(authentication.as_str(), "plain" | "login") {
            return Err("SMTP_AUTHENTICATION must be plain or login".to_owned());
        }
        let starttls = match std::env::var("SMTP_STARTTLS")
            .unwrap_or_else(|_| "true".to_owned())
            .as_str()
        {
            "true" => true,
            "false" => false,
            _ => return Err("SMTP_STARTTLS must be true or false".to_owned()),
        };
        let app_url = Url::parse(
            &std::env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000".to_owned()),
        )
        .map_err(|_| "APP_URL must be an absolute URL".to_owned())?;
        if !matches!(app_url.scheme(), "http" | "https")
            || app_url.host().is_none()
            || !app_url.username().is_empty()
            || app_url.password().is_some()
            || app_url.query().is_some()
            || app_url.fragment().is_some()
        {
            return Err("APP_URL must be an HTTP origin or base path".to_owned());
        }
        let from = std::env::var("MAILER_FROM")
            .unwrap_or_else(|_| "MedTracker <noreply@medtracker.app>".to_owned())
            .parse::<Mailbox>()
            .map_err(|_| "MAILER_FROM must be a mailbox".to_owned())?;
        if smtp_host
            .as_ref()
            .is_some_and(|value| value.contains(['\r', '\n']))
        {
            return Err("Invalid invitation mail configuration".to_owned());
        }
        Ok(Self {
            smtp_host,
            smtp_port,
            smtp_username,
            smtp_password,
            authentication,
            starttls,
            app_url,
            from,
        })
    }
}

fn collection_path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/admin/invitations")
}

fn valid_id(value: &str) -> Option<i64> {
    let mut chars = value.bytes();
    if !matches!(chars.next(), Some(b'1'..=b'9')) || !chars.all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

fn summary(row: &household_invitation::Model, now: NaiveDateTime) -> Value {
    json!({
        "id": row.id,
        "email": row.email,
        "membership_role": row.membership_role,
        "pending": row.accepted_at.is_none() && row.revoked_at.is_none() && row.expires_at > now,
        "accepted_at": row.accepted_at.map(|at| at.and_utc().to_rfc3339()),
        "revoked_at": row.revoked_at.map(|at| at.and_utc().to_rfc3339()),
        "expires_at": row.expires_at.and_utc().to_rfc3339(),
    })
}

fn state_row(row: &household_invitation::Model) -> Value {
    json!({
        "email": row.email,
        "membership_role": row.membership_role,
        "accepted_at": row.accepted_at,
        "revoked_at": row.revoked_at,
        "expires_at": row.expires_at,
        "invited_by_membership_id": row.invited_by_membership_id,
    })
}

async fn audit_event(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    event_type: &str,
    target_id: i64,
) -> Result<(), ApiError> {
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(context.membership.household_id),
        actor_account_id: Set(Some(context.account_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        event_type: Set(event_type.to_owned()),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(
            json!({"target_type":"HouseholdInvitation","target_id":target_id,"outcome":"success"}),
        ),
        audit_context: Set(json!({})),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    Ok(())
}

async fn denied(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
) -> Result<Response, ApiError> {
    error_response(
        db,
        context,
        method,
        CONTROLLER,
        POLICY,
        action,
        StatusCode::FORBIDDEN,
        "forbidden",
        "You are not authorized to perform this action.",
        None,
    )
    .await
}

async fn invalid(
    db: DatabaseTransaction,
    context: &AuthContext,
    method: &str,
    action: &str,
    status: StatusCode,
) -> Result<Response, ApiError> {
    let code = if status == StatusCode::BAD_REQUEST {
        "bad_request"
    } else {
        "validation_failed"
    };
    error_response(
        db,
        context,
        method,
        CONTROLLER,
        POLICY,
        action,
        status,
        code,
        "Invalid invitation",
        None,
    )
    .await
}

fn valid_email(email: &str) -> bool {
    email.len() <= 320
        && !email.is_empty()
        && !email.contains(char::is_whitespace)
        && email.matches('@').count() == 1
        && email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        })
}

fn parse_create(body: &Value) -> Result<(String, String), StatusCode> {
    let outer = body.as_object().ok_or(StatusCode::BAD_REQUEST)?;
    let inner = outer
        .get("household_invitation")
        .and_then(Value::as_object)
        .ok_or(StatusCode::BAD_REQUEST)?;
    if outer.len() != 1
        || inner.len() != 2
        || !inner.contains_key("email")
        || !inner.contains_key("membership_role")
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let email = inner
        .get("email")
        .and_then(Value::as_str)
        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?
        .trim()
        .to_lowercase();
    let role = inner
        .get("membership_role")
        .and_then(Value::as_str)
        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
    if !valid_email(&email) || !matches!(role, "administrator" | "member") {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    Ok((email, role.to_owned()))
}

fn new_token() -> Result<(String, String), ApiError> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| ApiError::internal())?;
    let token = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let digest = hex::encode(Sha256::digest(token.as_bytes()));
    Ok((token, digest))
}

async fn keyed_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    path: &str,
    body: &Value,
    action: &str,
) -> Result<Option<Response>, ApiError> {
    let Some(key) = mutation_idempotency::key(headers) else {
        return Ok(None);
    };
    let digest = mutation_idempotency::digest(method, path, body);
    match mutation_idempotency::lookup(db, context, key, method, path, &digest).await? {
        Lookup::New => Ok(None),
        Lookup::Replay(saved) => {
            let mut saved = *saved;
            let status = StatusCode::from_u16(saved.response_status as u16)
                .map_err(|_| ApiError::internal())?;
            let request_id = audit::record_resource_request(
                db, context, method, CONTROLLER, POLICY, action, status, true,
            )
            .await
            .map_err(database_error)?;
            if saved.response_body.get("error").is_some() {
                saved.response_body["error"]["request_id"] = json!(request_id);
            }
            let mut response = mutation_idempotency::replay(saved)?;
            response.headers_mut().insert(
                "x-request-id",
                HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
            );
            Ok(Some(response))
        }
        Lookup::Conflict => {
            let request_id = audit::record_resource_request(
                db,
                context,
                method,
                CONTROLLER,
                POLICY,
                action,
                StatusCode::CONFLICT,
                true,
            )
            .await
            .map_err(database_error)?;
            let mut response = (StatusCode::CONFLICT, Json(json!({"error":{"code":"idempotency_key_reused","message":"Idempotency key has already been used for a different request","request_id":request_id}}))).into_response();
            response.headers_mut().insert(
                "x-request-id",
                HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
            );
            Ok(Some(response))
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn keyed_failure(
    db: DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    path: &str,
    request: &Value,
    action: &str,
    status: StatusCode,
    code: &str,
    message: &str,
    errors: Option<Value>,
) -> Result<Response, ApiError> {
    let request_id = Uuid::new_v4().to_string();
    let mut response_body =
        json!({"error":{"code":code,"message":message,"request_id":request_id}});
    if let Some(errors) = errors {
        response_body["error"]["errors"] = errors;
    }
    if let Some(key) = mutation_idempotency::key(headers) {
        mutation_idempotency::store(
            &db,
            context,
            StoredResponse {
                key,
                method,
                path,
                digest: &mutation_idempotency::digest(method, path, request),
                status,
                body: response_body.clone(),
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
        action,
        status,
        true,
        response_body,
        None,
    )
    .await
}

pub(super) async fn index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !household_manager(&context) {
        return denied(db, &context, "GET", "index").await;
    }
    let rows = household_invitation::Entity::find()
        .filter(household_invitation::Column::HouseholdId.eq(household_id))
        .order_by_desc(household_invitation::Column::CreatedAt)
        .limit(100)
        .all(&db)
        .await
        .map_err(database_error)?;
    let now = Utc::now().naive_utc();
    let data = rows.iter().map(|row| summary(row, now)).collect::<Vec<_>>();
    finish(
        db,
        &context,
        "GET",
        CONTROLLER,
        POLICY,
        "index",
        StatusCode::OK,
        true,
        json!({"data":data}),
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
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    if !household_manager(&context) {
        return denied(db, &context, "POST", "create").await;
    }
    let Json(body) = match payload {
        Ok(body) => body,
        Err(_) => return invalid(db, &context, "POST", "create", StatusCode::BAD_REQUEST).await,
    };
    let path = collection_path(household_id);
    if let Some(response) =
        keyed_replay(&db, &context, &headers, "POST", &path, &body, "create").await?
    {
        db.commit().await.map_err(database_error)?;
        return Ok(response);
    }
    let (email, role) = match parse_create(&body) {
        Ok(value) => value,
        Err(status) => {
            let invalid_email_value = body
                .get("household_invitation")
                .and_then(Value::as_object)
                .and_then(|attributes| attributes.get("email"))
                .and_then(Value::as_str)
                .map(|value| value.trim().to_lowercase())
                .is_some_and(|value| !valid_email(&value));
            if status == StatusCode::UNPROCESSABLE_ENTITY && invalid_email_value {
                return keyed_failure(
                    db,
                    &context,
                    &headers,
                    "POST",
                    &path,
                    &body,
                    "create",
                    status,
                    "validation_failed",
                    "Validation failed",
                    Some(json!({"email": ["is invalid"]})),
                )
                .await;
            }
            let code = if status == StatusCode::BAD_REQUEST {
                "bad_request"
            } else {
                "validation_failed"
            };
            return keyed_failure(
                db,
                &context,
                &headers,
                "POST",
                &path,
                &body,
                "create",
                status,
                code,
                "Invalid invitation",
                None,
            )
            .await;
        }
    };
    let duplicate = household_invitation::Entity::find()
        .filter(household_invitation::Column::HouseholdId.eq(household_id))
        .filter(household_invitation::Column::Email.eq(&email))
        .filter(household_invitation::Column::AcceptedAt.is_null())
        .filter(household_invitation::Column::RevokedAt.is_null())
        .one(&db)
        .await
        .map_err(database_error)?;
    if duplicate.is_some() {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &body,
            "create",
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({"email": ["has already been taken"]})),
        )
        .await;
    }
    let (_, digest) = new_token()?;
    let now = Utc::now().naive_utc();
    let row = household_invitation::ActiveModel {
        household_id: Set(household_id),
        invited_by_membership_id: Set(context.membership.id),
        email: Set(email),
        membership_role: Set(role),
        token_digest: Set(digest),
        expires_at: Set(now + Duration::days(7)),
        accepted_at: Set(None),
        revoked_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "HouseholdInvitation",
        row.id,
        "create",
        None,
        Some(state_row(&row)),
    )
    .await?;
    audit_event(
        &db,
        &context,
        &request_id,
        "api/admin/invitation/created",
        row.id,
    )
    .await?;
    let response_body = json!({"data":summary(&row, now)});
    if let Some(key) = mutation_idempotency::key(&headers) {
        mutation_idempotency::store(
            &db,
            &context,
            StoredResponse {
                key,
                method: "POST",
                path: &path,
                digest: &mutation_idempotency::digest("POST", &path, &body),
                status: StatusCode::CREATED,
                body: response_body.clone(),
                request_id: &request_id,
                etag: None,
            },
        )
        .await?;
    }
    finish_with_request_id(
        db,
        &context,
        &request_id,
        "POST",
        CONTROLLER,
        POLICY,
        "create",
        StatusCode::CREATED,
        true,
        response_body,
        None,
    )
    .await
}

pub(super) async fn destroy(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    if !household_manager(&context) {
        return denied(db, &context, "DELETE", "destroy").await;
    }
    let path = format!("/api/v1/households/{household_id}/admin/invitations/{id}");
    let request = json!({});
    if let Some(response) = keyed_replay(
        &db, &context, &headers, "DELETE", &path, &request, "destroy",
    )
    .await?
    {
        db.commit().await.map_err(database_error)?;
        return Ok(response);
    }
    let Some(id) = valid_id(&id) else {
        return keyed_failure(
            db,
            &context,
            &headers,
            "DELETE",
            &path,
            &request,
            "destroy",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
            None,
        )
        .await;
    };
    let row = household_invitation::Entity::find_by_id(id)
        .filter(household_invitation::Column::HouseholdId.eq(household_id))
        .one(&db)
        .await
        .map_err(database_error)?;
    let Some(row) = row else {
        return keyed_failure(
            db,
            &context,
            &headers,
            "DELETE",
            &path,
            &request,
            "destroy",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
            None,
        )
        .await;
    };
    let before = state_row(&row);
    let mut active: household_invitation::ActiveModel = row.into();
    let now = Utc::now().naive_utc();
    active.revoked_at = Set(Some(now));
    active.updated_at = Set(now);
    let row = active.update(&db).await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "HouseholdInvitation",
        row.id,
        "update",
        Some(before),
        Some(state_row(&row)),
    )
    .await?;
    audit_event(
        &db,
        &context,
        &request_id,
        "api/admin/invitation/revoked",
        row.id,
    )
    .await?;
    if let Some(key) = mutation_idempotency::key(&headers) {
        mutation_idempotency::store(
            &db,
            &context,
            StoredResponse {
                key,
                method: "DELETE",
                path: &path,
                digest: &mutation_idempotency::digest("DELETE", &path, &request),
                status: StatusCode::NO_CONTENT,
                body: json!({}),
                request_id: &request_id,
                etag: None,
            },
        )
        .await?;
    }
    let mut response = finish_with_request_id(
        db,
        &context,
        &request_id,
        "DELETE",
        CONTROLLER,
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
    Ok(response)
}

fn no_store(response: &mut Response) {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
}

async fn resend_unavailable(
    state: &AppState,
    headers: &HeaderMap,
    household_id: i64,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(state, headers, household_id).await?;
    let mut response = error_response(
        db,
        &context,
        "POST",
        CONTROLLER,
        POLICY,
        "resend",
        StatusCode::SERVICE_UNAVAILABLE,
        "invitation_delivery_unavailable",
        "Invitation delivery is temporarily unavailable",
        None,
    )
    .await?;
    no_store(&mut response);
    Ok(response)
}

pub(super) async fn resend(
    state: State<AppState>,
    path: Path<(i64, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let result = resend_inner(state, path, headers, body).await;
    let mut response = match result {
        Ok(response) => response,
        Err(error) => error.into_response(),
    };
    no_store(&mut response);
    Ok(response)
}

async fn resend_inner(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let (db, _) = request_context(&state, &headers, household_id).await?;
    let (_, context) = mutation_idempotency::lock_household_and_reauthenticate(
        &state,
        &db,
        &headers,
        household_id,
    )
    .await?;
    if !household_manager(&context)
        || !matches!(
            context.credential_kind,
            CredentialKind::ApiSession | CredentialKind::OauthGrant
        )
    {
        return denied(db, &context, "POST", "resend").await;
    }
    let path = format!("/api/v1/households/{household_id}/admin/invitations/{id}/resend");
    let request = if body.is_empty() {
        json!({})
    } else {
        serde_json::from_slice::<Value>(&body).unwrap_or(Value::Null)
    };
    if let Some(response) =
        keyed_replay(&db, &context, &headers, "POST", &path, &request, "resend").await?
    {
        db.commit().await.map_err(database_error)?;
        return Ok(response);
    }
    let Some(id) = valid_id(&id) else {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &request,
            "resend",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
            None,
        )
        .await;
    };
    let Some(row) = household_invitation::Entity::find_by_id(id)
        .filter(household_invitation::Column::HouseholdId.eq(household_id))
        .lock_exclusive()
        .one(&db)
        .await
        .map_err(database_error)?
    else {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &request,
            "resend",
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
            None,
        )
        .await;
    };
    if request != json!({}) {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &request,
            "resend",
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Invalid invitation",
            None,
        )
        .await;
    }
    if row.accepted_at.is_some() || row.revoked_at.is_some() {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &request,
            "resend",
            StatusCode::UNPROCESSABLE_ENTITY,
            "unprocessable_content",
            "Invitation cannot be resent",
            None,
        )
        .await;
    }
    let key = mutation_idempotency::key(&headers);
    let digest = mutation_idempotency::digest("POST", &path, &request);
    let duplicate = household_invitation::Entity::find()
        .filter(household_invitation::Column::HouseholdId.eq(household_id))
        .filter(household_invitation::Column::Email.eq(&row.email))
        .filter(household_invitation::Column::Id.ne(row.id))
        .filter(household_invitation::Column::AcceptedAt.is_null())
        .filter(household_invitation::Column::RevokedAt.is_null())
        .filter(household_invitation::Column::ExpiresAt.gt(Utc::now().naive_utc()))
        .one(&db)
        .await
        .map_err(database_error)?;
    if duplicate.is_some() {
        return keyed_failure(
            db,
            &context,
            &headers,
            "POST",
            &path,
            &request,
            "resend",
            StatusCode::UNPROCESSABLE_ENTITY,
            "unprocessable_content",
            "Invitation cannot be resent",
            None,
        )
        .await;
    }
    let before = state_row(&row);
    let (token, token_digest) = new_token()?;
    let now = Utc::now().naive_utc();
    let mut active: household_invitation::ActiveModel = row.into();
    active.token_digest = Set(token_digest);
    active.expires_at = Set(now + Duration::days(7));
    active.updated_at = Set(now);
    let row = active.update(&db).await.map_err(database_error)?;
    let request_id = Uuid::new_v4().to_string();
    record_version(
        &db,
        &context,
        &request_id,
        "HouseholdInvitation",
        row.id,
        "resend",
        Some(before),
        Some(state_row(&row)),
    )
    .await?;
    audit_event(
        &db,
        &context,
        &request_id,
        "api/admin/invitation/resent",
        row.id,
    )
    .await?;
    let config = (*state.invitation_mail).clone();
    let email = row.email.clone();
    let delivered = tokio::task::spawn_blocking(move || smtp_send(config, email, token)).await;
    if !matches!(delivered, Ok(Ok(()))) {
        db.rollback().await.map_err(database_error)?;
        return resend_unavailable(&state, &headers, household_id).await;
    }
    let body = json!({"data":{"invitation_id":row.id.to_string(),"expires_at":row.expires_at.and_utc().to_rfc3339(),"delivery_status":"queued"}});
    if let Some(key) = key {
        mutation_idempotency::store(
            &db,
            &context,
            StoredResponse {
                key,
                method: "POST",
                path: &path,
                digest: &digest,
                status: StatusCode::OK,
                body: body.clone(),
                request_id: &request_id,
                etag: None,
            },
        )
        .await?;
    }
    let mut response = finish_with_request_id(
        db,
        &context,
        &request_id,
        "POST",
        CONTROLLER,
        POLICY,
        "resend",
        StatusCode::OK,
        true,
        body,
        None,
    )
    .await?;
    no_store(&mut response);
    Ok(response)
}

fn acceptance_error(status: StatusCode, code: &'static str, message: &'static str) -> ApiError {
    ApiError {
        status,
        code,
        message,
        preserve_activity: false,
    }
}

fn invitation_unavailable() -> ApiError {
    acceptance_error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "invitation_unavailable",
        "Invitation is unavailable",
    )
}

fn accepted_body(row: &membership::Model) -> Value {
    json!({"data": {
        "household_id": row.household_id.to_string(),
        "membership_id": row.id.to_string(),
        "person_id": row.person_id.map(|value| value.to_string()),
        "role": row.role,
    }})
}

async fn accepted_retry(
    db: &DatabaseTransaction,
    actor: &auth_sessions::InvitationActor,
    headers: &HeaderMap,
    digest: &str,
) -> Result<Option<membership::Model>, ApiError> {
    let memberships = membership::Entity::find()
        .filter(membership::Column::AccountId.eq(actor.account.id))
        .filter(membership::Column::Status.eq("active"))
        .filter(membership::Column::RevokedAt.is_null())
        .order_by_asc(membership::Column::HouseholdId)
        .order_by_asc(membership::Column::Id)
        .all(db)
        .await
        .map_err(database_error)?;
    for row in memberships {
        tenant_setting(db, "med_tracker.current_household_id", row.household_id)
            .await
            .map_err(database_error)?;
        let home = household::Entity::find_by_id(row.household_id)
            .lock_exclusive()
            .one(db)
            .await
            .map_err(database_error)?;
        if !home.is_some_and(|home| home.status == "active" && home.lifecycle_state == "active") {
            continue;
        }
        let current_actor = auth_sessions::invitation_actor(db, headers).await?;
        if current_actor.account.id != actor.account.id
            || current_actor.session.id != actor.session.id
        {
            return Err(invitation_unavailable());
        }
        let current_membership = membership::Entity::find_by_id(row.id)
            .lock_exclusive()
            .one(db)
            .await
            .map_err(database_error)?;
        let Some(current_membership) = current_membership.filter(|member| {
            member.account_id == actor.account.id
                && member.status == "active"
                && member.revoked_at.is_none()
        }) else {
            continue;
        };
        let accepted = household_invitation::Entity::find()
            .filter(household_invitation::Column::HouseholdId.eq(row.household_id))
            .filter(household_invitation::Column::TokenDigest.eq(digest))
            .filter(household_invitation::Column::Email.eq(&actor.account.email))
            .filter(household_invitation::Column::AcceptedAt.is_not_null())
            .filter(household_invitation::Column::RevokedAt.is_null())
            .one(db)
            .await
            .map_err(database_error)?;
        if accepted.is_some() {
            return Ok(Some(current_membership));
        }
    }
    Ok(None)
}

#[allow(clippy::too_many_arguments)]
async fn record_acceptance_effects(
    db: &DatabaseTransaction,
    request_id: &str,
    actor: &auth_sessions::InvitationActor,
    inviter: &membership::Model,
    member: &membership::Model,
    new_person: &person::Model,
    invitation_before: &household_invitation::Model,
    invitation_after: &household_invitation::Model,
    relations: &[carer_relationship::Model],
    now: NaiveDateTime,
) -> Result<(), ApiError> {
    let household_id = member.household_id;
    let grants = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(member.id))
        .filter(grant::Column::RevokedAt.is_null())
        .all(db)
        .await
        .map_err(database_error)?;
    let mut events = vec![security_audit_event::ActiveModel {
        household_id: Set(household_id),
        actor_account_id: Set(Some(inviter.account_id)),
        actor_membership_id: Set(Some(inviter.id)),
        event_type: Set("household_access.membership_created".to_owned()),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(json!({
            "target_account_id":actor.account.id,
            "target_membership_id":member.id,
            "previous_state":null,
            "new_state":{"role":member.role,"status":member.status,"person_id":member.person_id,"permissions_version":1},
            "outcome":"success"
        })),
        audit_context: Set(json!({})),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }];
    for row in &grants {
        let self_grant = row.person_id == new_person.id;
        events.push(security_audit_event::ActiveModel {
            household_id: Set(household_id),
            actor_account_id: Set(Some(if self_grant {
                actor.account.id
            } else {
                inviter.account_id
            })),
            actor_membership_id: Set(Some(if self_grant { member.id } else { inviter.id })),
            event_type: Set("household_access.person_grant_changed".to_owned()),
            request_id: Set(Some(request_id.to_owned())),
            metadata: Set(json!({
                "target_membership_id":member.id,
                "target_grant_id":row.id,
                "previous_state":null,
                "new_state":{
                    "household_membership_id":member.id,
                    "person_id":row.person_id,
                    "access_level":row.access_level,
                    "relationship_type":row.relationship_type,
                    "expires_at":row.expires_at.map(|at| at.and_utc().to_rfc3339()),
                    "revoked_at":null,
                    "carer_relationship_id":row.carer_relationship_id
                },
                "outcome":"success"
            })),
            audit_context: Set(json!({})),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        });
    }
    security_audit_event::Entity::insert_many(events)
        .exec(db)
        .await
        .map_err(database_error)?;
    let mut versions = vec![version::ActiveModel {
        item_type: Set("Person".to_owned()),
        item_id: Set(new_person.id),
        event: Set("create".to_owned()),
        object: Set(None),
        object_changes: Set(Some(json!({"name":[null,new_person.name],"person_type":[null,new_person.person_type],"has_capacity":[null,new_person.has_capacity]}).to_string())),
        whodunnit: Set(None),
        request_id: Set(Some(request_id.to_owned())),
        household_id: Set(Some(household_id)),
        actor_membership_id: Set(None),
        audit_context: Set(json!({"actor_account_id":actor.account.id})),
        created_at: Set(Some(now)),
        ..Default::default()
    }];
    for relation in relations {
        versions.push(version::ActiveModel {
            item_type: Set("CarerRelationship".to_owned()),
            item_id: Set(relation.id),
            event: Set("create".to_owned()),
            object: Set(None),
            object_changes: Set(Some(json!({"carer_id":[null,relation.carer_id],"patient_id":[null,relation.patient_id],"relationship_type":[null,relation.relationship_type]}).to_string())),
            whodunnit: Set(None),
            request_id: Set(Some(request_id.to_owned())),
            household_id: Set(Some(household_id)),
            actor_membership_id: Set(None),
            audit_context: Set(json!({"actor_account_id":actor.account.id})),
            created_at: Set(Some(now)),
            ..Default::default()
        });
    }
    versions.push(version::ActiveModel {
        item_type: Set("HouseholdInvitation".to_owned()),
        item_id: Set(invitation_before.id),
        event: Set("update".to_owned()),
        object: Set(Some(state_row(invitation_before).to_string())),
        object_changes: Set(Some(
            json!({"accepted_at":[null,invitation_after.accepted_at]}).to_string(),
        )),
        whodunnit: Set(None),
        request_id: Set(Some(request_id.to_owned())),
        household_id: Set(Some(household_id)),
        actor_membership_id: Set(None),
        audit_context: Set(json!({"actor_account_id":actor.account.id})),
        created_at: Set(Some(now)),
        ..Default::default()
    });
    version::Entity::insert_many(versions)
        .exec(db)
        .await
        .map_err(database_error)?;
    api_change_event::ActiveModel {
        household_id: Set(household_id),
        household_membership_id: Set(None),
        account_id: Set(Some(actor.account.id)),
        action: Set("create".to_owned()),
        record_type: Set("Person".to_owned()),
        record_id: Set(new_person.id),
        record_portable_id: Set(Some(new_person.portable_id.clone())),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(json!({"record_type":"Person","record_id":new_person.id,"portable_id":new_person.portable_id})),
        occurred_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }.insert(db).await.map_err(database_error)?;
    Ok(())
}

async fn accept_pending(
    db: &DatabaseTransaction,
    request_id: &str,
    actor: &auth_sessions::InvitationActor,
    headers: &HeaderMap,
    invitation: household_invitation::Model,
) -> Result<membership::Model, ApiError> {
    let household_id = invitation.household_id;
    let expected_digest = invitation.token_digest.clone();
    tenant_setting(db, "med_tracker.current_household_id", household_id)
        .await
        .map_err(database_error)?;
    tenant_setting(db, "med_tracker.current_account_id", actor.account.id)
        .await
        .map_err(database_error)?;
    let home = household::Entity::find_by_id(household_id)
        .lock_exclusive()
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(invitation_unavailable)?;
    if home.status != "active" || home.lifecycle_state != "active" {
        return Err(invitation_unavailable());
    }
    let current_actor = auth_sessions::invitation_actor(db, headers).await?;
    if current_actor.account.id != actor.account.id || current_actor.session.id != actor.session.id
    {
        return Err(invitation_unavailable());
    }
    let invitation = household_invitation::Entity::find_by_id(invitation.id)
        .filter(household_invitation::Column::HouseholdId.eq(household_id))
        .lock_exclusive()
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(invitation_unavailable)?;
    if invitation.email != actor.account.email.to_lowercase()
        || invitation.token_digest != expected_digest
        || invitation.revoked_at.is_some()
        || invitation.expires_at <= Utc::now().naive_utc()
    {
        return Err(invitation_unavailable());
    }
    if invitation.accepted_at.is_some() {
        return membership::Entity::find()
            .filter(membership::Column::HouseholdId.eq(household_id))
            .filter(membership::Column::AccountId.eq(actor.account.id))
            .filter(membership::Column::Status.eq("active"))
            .filter(membership::Column::RevokedAt.is_null())
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(invitation_unavailable);
    }
    let inviter = membership::Entity::find_by_id(invitation.invited_by_membership_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(invitation_unavailable)?;
    if inviter.household_id != household_id
        || inviter.status != "active"
        || inviter.revoked_at.is_some()
        || !matches!(inviter.role.as_str(), "owner" | "administrator")
    {
        return Err(invitation_unavailable());
    }
    let existing = membership::Entity::find()
        .filter(membership::Column::HouseholdId.eq(household_id))
        .filter(membership::Column::AccountId.eq(actor.account.id))
        .one(db)
        .await
        .map_err(database_error)?;
    if existing.is_some() {
        return Err(invitation_unavailable());
    }
    let now = Utc::now().naive_utc();
    let new_person = person::ActiveModel {
        account_id: Set(Some(actor.account.id)),
        household_id: Set(household_id),
        portable_id: Set(Uuid::new_v4().to_string()),
        name: Set(actor.person.name.clone()),
        email: Set(None),
        date_of_birth: Set(actor.person.date_of_birth),
        person_type: Set(actor.person.person_type),
        has_capacity: Set(actor.person.has_capacity),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    let new_membership = membership::ActiveModel {
        account_id: Set(actor.account.id),
        household_id: Set(household_id),
        person_id: Set(Some(new_person.id)),
        permissions_version: Set(1),
        role: Set(invitation.membership_role.clone()),
        status: Set("active".to_owned()),
        revoked_at: Set(None),
        joined_at: Set(Some(now)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    grant::ActiveModel {
        household_id: Set(household_id),
        household_membership_id: Set(new_membership.id),
        person_id: Set(new_person.id),
        access_level: Set("manage".to_owned()),
        relationship_type: Set("self".to_owned()),
        expires_at: Set(None),
        revoked_at: Set(None),
        granted_by_membership_id: Set(Some(new_membership.id)),
        carer_relationship_id: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    let additional = household_invitation_grant::Entity::find()
        .filter(household_invitation_grant::Column::HouseholdInvitationId.eq(invitation.id))
        .filter(household_invitation_grant::Column::HouseholdId.eq(household_id))
        .all(db)
        .await
        .map_err(database_error)?;
    let mut latest_grants = HashMap::new();
    for row in additional {
        let entry = latest_grants
            .entry(row.person_id)
            .or_insert_with(|| row.clone());
        if row.id > entry.id {
            *entry = row;
        }
    }
    let additional = latest_grants.into_values().collect::<Vec<_>>();
    let mut created_relations = Vec::new();
    if !additional.is_empty() {
        let patient_ids = additional
            .iter()
            .map(|row| row.person_id)
            .collect::<HashSet<_>>();
        let patients = person::Entity::find()
            .filter(person::Column::HouseholdId.eq(household_id))
            .filter(person::Column::Id.is_in(patient_ids.iter().copied()))
            .all(db)
            .await
            .map_err(database_error)?;
        if patients.len() != patient_ids.len() {
            return Err(invitation_unavailable());
        }
        let relationships = additional
            .iter()
            .filter_map(|row| {
                let relation_type = match row.relationship_type.as_str() {
                    "parent" => "parent",
                    "family_member" => "family_member",
                    "carer" | "professional" => "professional_carer",
                    _ => return None,
                };
                Some(carer_relationship::ActiveModel {
                    household_id: Set(household_id),
                    carer_id: Set(new_person.id),
                    patient_id: Set(row.person_id),
                    relationship_type: Set(Some(relation_type.to_owned())),
                    active: Set(true),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                })
            })
            .collect::<Vec<_>>();
        if !relationships.is_empty() {
            carer_relationship::Entity::insert_many(relationships)
                .exec(db)
                .await
                .map_err(database_error)?;
        }
        let relationships = carer_relationship::Entity::find()
            .filter(carer_relationship::Column::HouseholdId.eq(household_id))
            .filter(carer_relationship::Column::CarerId.eq(new_person.id))
            .filter(carer_relationship::Column::PatientId.is_in(patient_ids.iter().copied()))
            .all(db)
            .await
            .map_err(database_error)?;
        created_relations = relationships.clone();
        let relation_ids = relationships
            .into_iter()
            .map(|row| (row.patient_id, row.id))
            .collect::<HashMap<_, _>>();
        let grants = additional
            .into_iter()
            .map(|row| {
                let relation_id = if matches!(
                    row.relationship_type.as_str(),
                    "parent" | "family_member" | "carer" | "professional"
                ) {
                    relation_ids.get(&row.person_id).copied()
                } else {
                    None
                };
                let grant_type =
                    if matches!(row.relationship_type.as_str(), "carer" | "professional") {
                        "professional".to_owned()
                    } else {
                        row.relationship_type
                    };
                grant::ActiveModel {
                    household_id: Set(household_id),
                    household_membership_id: Set(new_membership.id),
                    person_id: Set(row.person_id),
                    access_level: Set(row.access_level),
                    relationship_type: Set(grant_type),
                    expires_at: Set(row.expires_at),
                    revoked_at: Set(None),
                    granted_by_membership_id: Set(Some(inviter.id)),
                    carer_relationship_id: Set(relation_id),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                }
            })
            .collect::<Vec<_>>();
        grant::Entity::insert_many(grants)
            .exec(db)
            .await
            .map_err(database_error)?;
    }
    let grant_count = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(new_membership.id))
        .filter(grant::Column::RevokedAt.is_null())
        .count(db)
        .await
        .map_err(database_error)?;
    let mut member_active = new_membership.into_active_model();
    member_active.permissions_version =
        Set(i32::try_from(grant_count).map_err(|_| ApiError::internal())? + 1);
    member_active.updated_at = Set(now);
    let new_membership = member_active.update(db).await.map_err(database_error)?;
    let invitation_before = invitation.clone();
    let mut active: household_invitation::ActiveModel = invitation.into();
    active.accepted_at = Set(Some(now));
    active.updated_at = Set(now);
    let invitation_after = active.update(db).await.map_err(database_error)?;
    record_acceptance_effects(
        db,
        request_id,
        actor,
        &inviter,
        &new_membership,
        &new_person,
        &invitation_before,
        &invitation_after,
        &created_relations,
        now,
    )
    .await?;
    Ok(new_membership)
}

pub(super) async fn accept(
    state: State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let result = accept_inner(state, headers, payload).await;
    let mut response = match result {
        Ok(response) => response,
        Err(error) => error.into_response(),
    };
    no_store(&mut response);
    Ok(response)
}

async fn accept_inner(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let db = state.db.begin().await.map_err(database_error)?;
    restricted_role(&db).await.map_err(database_error)?;
    let actor = auth_sessions::invitation_actor(&db, &headers).await?;
    let Json(body) = payload.map_err(|_| {
        acceptance_error(
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid request body",
        )
    })?;
    let outer = body.as_object().ok_or_else(|| {
        acceptance_error(
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid request body",
        )
    })?;
    let token = outer
        .get("token")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            acceptance_error(
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Invalid request body",
            )
        })?;
    if outer.len() != 1 {
        return Err(acceptance_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Invalid request body",
        ));
    }
    let digest = hex::encode(Sha256::digest(token.as_bytes()));
    let request_id = Uuid::new_v4().to_string();
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT set_config('med_tracker.current_invitation_token_digest', $1, true)",
        [digest.clone().into()],
    ))
    .await
    .map_err(database_error)?;
    let invitation = household_invitation::Entity::find()
        .filter(household_invitation::Column::TokenDigest.eq(&digest))
        .filter(household_invitation::Column::AcceptedAt.is_null())
        .filter(household_invitation::Column::RevokedAt.is_null())
        .filter(household_invitation::Column::ExpiresAt.gt(Utc::now().naive_utc()))
        .one(&db)
        .await
        .map_err(database_error)?;
    let result = if let Some(invitation) = invitation {
        accept_pending(&db, &request_id, &actor, &headers, invitation).await?
    } else {
        accepted_retry(&db, &actor, &headers, &digest)
            .await?
            .ok_or_else(invitation_unavailable)?
    };
    db.commit().await.map_err(database_error)?;
    let mut response = (StatusCode::OK, Json(accepted_body(&result))).into_response();
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
    );
    no_store(&mut response);
    Ok(response)
}
