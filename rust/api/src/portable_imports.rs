mod access;
mod contracts;
mod identity;
mod links;
mod mapping;
mod pauses;
mod preflight;
mod validation;
mod writing;

use crate::entities::{grant, security_audit_event};
use crate::medication_management::{
    error_response, finish_with_request_id, household_manager, record_version, request_context,
};
use crate::mutation_idempotency::lock_household_and_reauthenticate;
use crate::portable_crypto::{self, PortableCryptoError};
use crate::sync_events::{record_change, SyncRecord};
use crate::{database_error, ApiError, AppState, AuthContext};
use axum::extract::{rejection::JsonRejection, DefaultBodyLimit, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::post;
use axum::{Json, Router};
use chrono::Utc;
use chrono::{DateTime, Datelike, NaiveDate, NaiveTime};
use sea_orm::prelude::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait,
    QueryFilter, Set, Statement, TransactionTrait,
};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use uuid::Uuid;

const CONTROLLER: &str = "api/v1/portable_imports";
const PASSPHRASE_HEADER: &str = "x-medtracker-portable-passphrase";
const BODY_LIMIT: usize = 24 * 1024 * 1024;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/households/{household_id}/portable_imports/dry_run",
            post(dry_run),
        )
        .route(
            "/api/v1/households/{household_id}/portable_imports",
            post(apply),
        )
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
}

async fn dry_run(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    handle(state, household_id, headers, body, true).await
}

async fn apply(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    handle(state, household_id, headers, body, false).await
}

async fn fail(
    db: DatabaseTransaction,
    context: &AuthContext,
    action: &str,
    status: StatusCode,
    message: &str,
) -> Result<Response, ApiError> {
    let code = match status {
        StatusCode::BAD_REQUEST => "bad_request",
        StatusCode::FORBIDDEN => "forbidden",
        _ => "unprocessable_content",
    };
    error_response(
        db,
        context,
        "POST",
        CONTROLLER,
        "HouseholdPolicy",
        action,
        status,
        code,
        message,
        None,
    )
    .await
}

fn passphrase(headers: &HeaderMap) -> Option<String> {
    headers
        .get(PASSPHRASE_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
}

fn crypto_message(error: PortableCryptoError) -> &'static str {
    match error {
        PortableCryptoError::TooLarge => "Portable bundle is too large",
        PortableCryptoError::InvalidEnvelope => "Portable bundle could not be decrypted",
        PortableCryptoError::Unavailable => "Portable decryption is temporarily unavailable",
    }
}

async fn handle(
    state: AppState,
    household_id: i64,
    headers: HeaderMap,
    body: Result<Json<Value>, JsonRejection>,
    is_dry_run: bool,
) -> Result<Response, ApiError> {
    let action = if is_dry_run { "dry_run" } else { "create" };
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let Json(body) = match body {
        Ok(body) => body,
        Err(rejection) => {
            let status = if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
                StatusCode::UNPROCESSABLE_ENTITY
            } else {
                StatusCode::BAD_REQUEST
            };
            let message = if status == StatusCode::UNPROCESSABLE_ENTITY {
                "Portable bundle is too large"
            } else {
                "Invalid JSON request body"
            };
            return fail(db, &context, action, status, message).await;
        }
    };
    let Some(object) = body.as_object() else {
        return fail(
            db,
            &context,
            action,
            StatusCode::BAD_REQUEST,
            "Portable bundle is required",
        )
        .await;
    };
    let Some(bundle) = object.get("bundle") else {
        return fail(
            db,
            &context,
            action,
            StatusCode::BAD_REQUEST,
            "Portable bundle is required",
        )
        .await;
    };
    if object.len() != 1 {
        return fail(
            db,
            &context,
            action,
            StatusCode::UNPROCESSABLE_ENTITY,
            "Unsupported import request fields",
        )
        .await;
    }
    let Some(passphrase) = passphrase(&headers) else {
        return fail(
            db,
            &context,
            action,
            StatusCode::UNPROCESSABLE_ENTITY,
            "Portable passphrase header is required",
        )
        .await;
    };
    db.commit().await.map_err(database_error)?;
    let bundle = bundle.clone();
    let decrypted =
        tokio::task::spawn_blocking(move || portable_crypto::decrypt(&bundle, &passphrase))
            .await
            .map_err(|_| ApiError::internal())?;
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let payload = match decrypted {
        Ok(payload) => payload,
        Err(error) => {
            return fail(
                db,
                &context,
                action,
                StatusCode::UNPROCESSABLE_ENTITY,
                crypto_message(error),
            )
            .await
        }
    };
    let plan = match ImportPlan::parse(payload) {
        Ok(plan) => plan,
        Err(message) => {
            return fail(
                db,
                &context,
                action,
                StatusCode::UNPROCESSABLE_ENTITY,
                &message,
            )
            .await
        }
    };
    db.rollback().await.map_err(database_error)?;
    let db = state.db.begin().await.map_err(database_error)?;
    let (_, context) =
        lock_household_and_reauthenticate(&state, &db, &headers, household_id).await?;
    let mut index = ExistingIndex::load(&db, household_id).await?;
    if !authorized(&db, &context, &plan, &index).await? {
        return fail(
            db,
            &context,
            action,
            StatusCode::FORBIDDEN,
            "You are not authorized to perform this action.",
        )
        .await;
    }
    let result = preflight(&db, &context, &plan, &index).await?;
    if is_dry_run || !result.conflicts.is_empty() || !result.errors.is_empty() {
        let status = if is_dry_run {
            StatusCode::OK
        } else {
            StatusCode::UNPROCESSABLE_ENTITY
        };
        return finish_with_request_id(
            db,
            &context,
            &Uuid::new_v4().to_string(),
            "POST",
            CONTROLLER,
            "HouseholdPolicy",
            action,
            status,
            status.is_success(),
            json!({"data": result.body(false)}),
            None,
        )
        .await;
    }
    let request_id = Uuid::new_v4().to_string();
    if let Err(message) = write_records(&db, &context, &plan, &mut index, &request_id).await {
        db.rollback().await.map_err(database_error)?;
        let (fresh, context) = request_context(&state, &headers, household_id).await?;
        return fail(
            fresh,
            &context,
            action,
            StatusCode::UNPROCESSABLE_ENTITY,
            &message,
        )
        .await;
    }
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(household_id),
        actor_account_id: Set(Some(context.account_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        event_type: Set("portable_data.imported".to_owned()),
        request_id: Set(Some(request_id.clone())),
        ip: Set(None),
        metadata: Set(json!({"record_counts": plan.counts, "dry_run": false})),
        audit_context: Set(
            json!({"request_id": request_id, "actor_membership_id": context.membership.id}),
        ),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .map_err(database_error)?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        "POST",
        CONTROLLER,
        "HouseholdPolicy",
        action,
        StatusCode::CREATED,
        true,
        json!({"data": result.body(true)}),
        None,
    )
    .await
}

use access::authorized;
use contracts::{ImportPlan, BASE_TYPES, V2_TYPES};
use identity::{table, ExistingIndex};
use links::{grant_importer_access, write_health_links, write_location_memberships};
use mapping::{mapped_row, record_person, text};
use pauses::{reconcile_pauses, resume_pause, verify_pause};
use preflight::preflight;
use validation::{decimal, validate_row};
use writing::{verify_immutable, write_records};
