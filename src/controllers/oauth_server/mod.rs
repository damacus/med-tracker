use crate::models::identity::{self, ExchangeError, oauth};
use axum::{
    body::{Body, to_bytes},
    extract::Request,
    http::{StatusCode, header},
    response::IntoResponse,
};
use loco_rs::prelude::*;
mod authorization_response;
mod consent;
pub use consent::routes as browser_routes;

pub fn routes() -> Routes {
    Routes::new()
        .add("/.well-known/oauth-authorization-server", get(discovery))
        .add("/api/v1/capabilities", get(capabilities))
        .add("/token", post(token))
        .add("/revoke", post(revoke))
}

async fn discovery(State(ctx): State<AppContext>) -> Response {
    let issuer = ctx.config.server.full_url();
    axum::Json(serde_json::json!({"issuer":issuer,"authorization_endpoint":format!("{issuer}/authorize"),"token_endpoint":format!("{issuer}/token"),"revocation_endpoint":format!("{issuer}/revoke"),"response_types_supported":["code"],"response_modes_supported":["query","form_post"],"grant_types_supported":["authorization_code","refresh_token"],"code_challenge_methods_supported":["S256"],"token_endpoint_auth_methods_supported":["none","client_secret_basic","client_secret_post"]})).into_response()
}

async fn capabilities(State(ctx): State<AppContext>) -> Response {
    let lifetime = match identity::AuthenticationLifetime::from_environment() {
        Ok(lifetime) => lifetime,
        Err(error) => return failure(error),
    };
    match oauth::mobile_clients(&ctx.db).await {
        Ok(clients) => {
            let base = ctx.config.server.full_url();
            let base = base.trim_end_matches('/');
            no_cache(axum::Json(serde_json::json!({"data": {
                "format": "medtracker.api.capabilities.v1",
                "api_version": "v1",
                "authentication": {
                    "methods": ["oauth_bearer", "api_app_token"],
                    "hosted_mobile": "rodauth_authorization_code_pkce",
                    "mobile_oauth": {
                        "discovery_url": format!("{base}/.well-known/oauth-authorization-server"),
                        "household_binding": "account",
                        "inactivity_timeout_days": lifetime.inactivity_days(),
                        "maximum_age_days": lifetime.maximum_age_days(),
                        "clients": clients
                    }
                },
        "administration": {"household": true, "fresh_mfa_required": false, "app_tokens": false, "audit_logs": false, "invitations": true, "person_access_grants": true},
        "medication_pause_periods": {"supported": true, "reasons": ["out_of_supply", "temporarily_not_needed", "clinician_advice", "side_effects", "other"], "effective_time": "server_acceptance"},
        "dose_outcomes": {"source_types": ["schedule", "person_medication"], "max_read_days": 31, "actions": ["not_taken", "reopen", "take"], "replacement_requires_version": true},
        "stock_removals": {"actions": ["create", "index"], "submission_id_required": true, "max_page_size": 100},
        "location_management": {"actions": ["create", "update", "destroy"], "version_required": true, "person_memberships": ["create", "destroy"], "memberships_online_only": true},
        "medication_reviews": {"actions": [], "version_required": true, "max_page_size": 100},
        "reports": {"formats": ["json", "pdf"], "health_history": true, "medication_reviews": true, "selected_person_required": true, "health_history_max_span_days": 366},
        "invitations": {"actions": ["accept", "resend"], "online_only": true, "acceptance_session_required": true},
        "portable_formats": [],
        "backups": {"encrypted_migration_bundle": false, "unencrypted_zip": false, "health_data_json": false},
        "fhir": {"version": "R4", "resources": []},
        "sync": {"portable_ids": true, "numeric_ids": "backward_compatible", "mobile_snapshot": true, "dry_run_import": false, "idempotency_keys": true, "etag_conflicts": true, "change_feed": true, "batch_mutations": true, "tombstones": true, "operations": [
            {"resource_type": "medication_take", "actions": ["create"]},
            {"resource_type": "medication_dose_occurrence", "actions": ["create", "update"]},
            {"resource_type": "medication_pause_period", "actions": ["create", "close"]},
            {"resource_type": "medication", "actions": ["create", "update", "delete", "adjust_inventory", "mark_as_ordered", "mark_as_received", "remove_stock"]},
            {"resource_type": "medication_dosage_option", "actions": ["create", "update"]},
            {"resource_type": "person", "actions": ["create", "update"]},
            {"resource_type": "health_event", "actions": ["create", "update", "delete"]},
            {"resource_type": "location", "actions": ["create", "update", "delete"]},
            {"resource_type": "medication_review_prompt", "actions": ["update"]},
            {"resource_type": "schedule", "actions": ["create", "update", "delete", "pause", "resume"]},
            {"resource_type": "person_medication", "actions": ["create", "update", "delete", "pause", "resume", "reorder"]}
        ], "online_only_resources": ["account", "profile", "avatar", "invitation", "household_membership", "person_access_grant", "location_membership", "report", "api_session", "api_app_token"]},
        "client_tools": {"cli": {"supported": false, "status": "available", "binary": "medtracker", "api_boundary": "/api/v1", "distribution": "github_release"}, "mcp_server": {"supported": false, "transport": "streamable_http", "endpoint": "/mcp", "stdio_binary": "medtracker-mcp", "tools": [], "resources": []}, "diagnostics": ["request_id", "retry_after"]}
            }})).into_response())
        }
        Err(error) => failure(error),
    }
}

async fn token(State(ctx): State<AppContext>, request: Request) -> Response {
    match identity::exchange(&ctx.db, request).await {
        Ok(body) => no_cache(axum::Json(body).into_response()),
        Err(error) => failure(error),
    }
}

async fn revoke(State(ctx): State<AppContext>, request: Request) -> Response {
    let request = match revocation_request(request).await {
        Ok(request) => request,
        Err(error) => return failure(error),
    };
    match oauth::revoke(&ctx.db, request).await {
        Ok(()) => no_cache(StatusCode::OK.into_response()),
        Err(error) => failure(error),
    }
}

async fn revocation_request(request: Request) -> std::result::Result<Request, ExchangeError> {
    if request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        != Some("application/json")
    {
        return Ok(request);
    }
    let (mut parts, body) = request.into_parts();
    let bytes = to_bytes(body, 16_384)
        .await
        .map_err(|_| ExchangeError::InvalidRequest)?;
    let fields: std::collections::BTreeMap<String, String> =
        serde_json::from_slice(&bytes).map_err(|_| ExchangeError::InvalidRequest)?;
    let body = serde_urlencoded::to_string(fields).map_err(|_| ExchangeError::InvalidRequest)?;
    parts.headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    parts.headers.remove(header::CONTENT_LENGTH);
    Ok(Request::from_parts(parts, Body::from(body)))
}

pub(super) fn failure(error: ExchangeError) -> Response {
    let (status, body, authenticate) = match error {
        ExchangeError::InvalidRequest => (
            StatusCode::BAD_REQUEST,
            serde_json::json!({"error":"invalid_request"}),
            None,
        ),
        ExchangeError::InvalidClient => (
            StatusCode::UNAUTHORIZED,
            serde_json::json!({"error":"invalid_client"}),
            Some("Basic realm=\"OAuth\"".to_owned()),
        ),
        ExchangeError::InvalidGrant => (
            StatusCode::BAD_REQUEST,
            serde_json::json!({"error":"invalid_grant"}),
            None,
        ),
        ExchangeError::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({"error":"temporarily_unavailable"}),
            None,
        ),
        ExchangeError::Protocol { body, authenticate } => (
            if authenticate.is_some() {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::BAD_REQUEST
            },
            body,
            authenticate,
        ),
    };
    let mut response = (status, axum::Json(body)).into_response();
    if let Some(value) = authenticate.and_then(|value| header::HeaderValue::from_str(&value).ok()) {
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, value);
    }
    no_cache(response)
}

fn no_cache(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response
        .headers_mut()
        .insert(header::PRAGMA, header::HeaderValue::from_static("no-cache"));
    response
}
