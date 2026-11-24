use axum::{
    Json,
    extract::{
        Path, State,
        rejection::{JsonRejection, PathRejection},
    },
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::IntoResponse,
};
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};
use serde::Deserialize;
use serde_json::Value;

use crate::models::{
    errors::OperationError,
    identity::{
        account_sessions,
        resource::{self, AuthenticationError},
    },
    ids::AccountId,
    profile::self_account,
};
use crate::views::{
    DataResponse,
    account_sessions::{AccountProfile, Error, ErrorResponse},
};

mod avatar;
mod guardian_declarations;
mod person_schedules;
mod personal_history;
mod shared_schedules;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/v1/auth")
        .add("/households", get(households))
        .add("/profile", get(profile).patch(update_profile))
        .add("/profile/history", get(personal_history::index))
        .add("/schedules", get(person_schedules::index))
        .add("/schedules/adopt", post(person_schedules::adopt))
        .add("/schedules/{id}", patch(person_schedules::update))
        .add("/shared_schedules/{id}", patch(shared_schedules::update))
        .add(
            "/care-sharing/{id}",
            delete(person_schedules::withdraw_sharing),
        )
        .add("/guardian-declarations", get(guardian_declarations::index))
        .add(
            "/guardian-declarations/{id}",
            patch(guardian_declarations::withdraw),
        )
        .add(
            "/profile/avatar",
            get(avatar::show)
                .put(avatar::upload)
                .delete(avatar::remove)
                .layer(axum::extract::DefaultBodyLimit::max(6 * 1024 * 1024)),
        )
        .add("/sessions", get(sessions))
        .add("/sessions/{id}", delete(revoke))
        .add("/logout", delete(logout))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileInput {
    profile: ProfilePatch,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfilePatch {
    #[serde(default, deserialize_with = "present")]
    name: Option<String>,
    #[serde(default, deserialize_with = "present")]
    time_zone: Option<String>,
    #[serde(default, deserialize_with = "present")]
    gravatar_enabled: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    mobile_shortcuts: Option<Vec<String>>,
}

impl ProfilePatch {
    fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.time_zone.is_none()
            && self.gravatar_enabled.is_none()
            && self.mobile_shortcuts.is_none()
    }
}

fn present<'de, D, T>(deserializer: D) -> std::result::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

async fn profile(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
) -> Response {
    profile_action(ctx, headers, request, None).await
}

async fn update_profile(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
    body: std::result::Result<Json<ProfileInput>, JsonRejection>,
) -> Response {
    profile_action(ctx, headers, request, Some(body)).await
}

async fn profile_action(
    ctx: AppContext,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
    body: Option<std::result::Result<Json<ProfileInput>, JsonRejection>>,
) -> Response {
    let request_id = request_id(request);
    let principal =
        match resource::authenticate(&ctx.db, &headers, ctx.config.settings.as_ref()).await {
            Ok(principal) => principal,
            Err(error) => return authentication(error, &request_id),
        };
    let write = body.is_some();
    let changes = match body {
        Some(Ok(Json(input))) if !input.profile.is_empty() => match self_account::Changes::parse(
            input.profile.name.as_deref(),
            input.profile.time_zone.as_deref(),
            input.profile.gravatar_enabled,
            input.profile.mobile_shortcuts.as_deref(),
        ) {
            Ok(changes) => Some(changes),
            Err(_) => {
                return failure(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "unprocessable_content",
                    "Profile contains an invalid field",
                    &request_id,
                );
            }
        },
        Some(_) => {
            return failure(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "Invalid profile body",
                &request_id,
            );
        }
        None => None,
    };
    let transaction = match principal
        .account_transaction(&ctx.db, ctx.config.settings.as_ref(), write)
        .await
    {
        Ok(transaction) => transaction,
        Err(error) => return authentication(error, &request_id),
    };
    let account_id = AccountId::from_database(principal.account_id());
    let result = async {
        if let Some(changes) = changes {
            self_account::update(&transaction, account_id, &request_id, changes).await?;
        }
        self_account::read(&transaction, account_id, ctx.config.settings.as_ref()).await
    }
    .await;
    match result {
        Ok(snapshot) => match transaction.commit().await {
            Ok(()) => reply(StatusCode::OK, Some(representation(snapshot)), &request_id),
            Err(error) => operation(error.into(), &request_id),
        },
        Err(error) => {
            if let Err(error) = transaction.rollback().await {
                return operation(error.into(), &request_id);
            }
            match error {
                OperationError::Validation { .. } => failure(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "unprocessable_content",
                    "Profile contains an invalid field",
                    &request_id,
                ),
                other => operation(other, &request_id),
            }
        }
    }
}

fn representation(snapshot: self_account::Snapshot) -> DataResponse<AccountProfile> {
    DataResponse {
        data: AccountProfile {
            account_id: snapshot.account_id.to_string(),
            name: snapshot.name,
            email: snapshot.email,
            time_zone: snapshot.time_zone.name(),
            gravatar_enabled: snapshot.gravatar_enabled,
            avatar_attached: snapshot.avatar_attached,
            mobile_shortcuts: snapshot.mobile_shortcuts,
        },
    }
}

async fn households(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    let principal =
        match resource::authenticate(&ctx.db, &headers, ctx.config.settings.as_ref()).await {
            Ok(principal) => principal,
            Err(error) => return authentication(error, &request_id),
        };
    match account_sessions::households(&ctx.db, principal.account_id()).await {
        Ok(body) => reply(StatusCode::OK, Some(body), &request_id),
        Err(error) => operation(error, &request_id),
    }
}

async fn sessions(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    let principal =
        match resource::authenticate(&ctx.db, &headers, ctx.config.settings.as_ref()).await {
            Ok(principal) => principal,
            Err(error) => return authentication(error, &request_id),
        };
    match account_sessions::sessions(
        &ctx.db,
        principal.account_id(),
        principal.provenance().method,
        ctx.config.settings.as_ref(),
    )
    .await
    {
        Ok(body) => reply(StatusCode::OK, Some(body), &request_id),
        Err(error) => operation(error, &request_id),
    }
}

async fn revoke(
    State(ctx): State<AppContext>,
    id: Result<Path<String>, PathRejection>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    let principal =
        match resource::authenticate(&ctx.db, &headers, ctx.config.settings.as_ref()).await {
            Ok(principal) => principal,
            Err(error) => return authentication(error, &request_id),
        };
    let Some(id) = id.ok().and_then(|Path(id)| id.parse::<i64>().ok()) else {
        return operation(OperationError::NotFound, &request_id);
    };
    match account_sessions::revoke(
        &ctx.db,
        principal.account_id(),
        principal.provenance().method,
        id,
        &request_id,
        ctx.config.settings.as_ref(),
    )
    .await
    {
        Ok(()) => reply(StatusCode::NO_CONTENT, None::<()>, &request_id),
        Err(error) => operation(error, &request_id),
    }
}

async fn logout(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    match account_sessions::logout(&ctx.db, &headers, &request_id).await {
        Ok(()) => reply(StatusCode::NO_CONTENT, None::<()>, &request_id),
        Err(error) => operation(error, &request_id),
    }
}

fn request_id(request: Option<axum::Extension<LocoRequestId>>) -> String {
    request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |axum::Extension(id)| id.get().to_owned(),
    )
}

fn authentication(error: AuthenticationError, request_id: &str) -> Response {
    match error {
        AuthenticationError::Unauthenticated => failure(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Authentication required",
            request_id,
        ),
        AuthenticationError::Forbidden | AuthenticationError::NotFound => failure(
            StatusCode::FORBIDDEN,
            "forbidden",
            "You are not authorized to perform this action.",
            request_id,
        ),
        AuthenticationError::InsufficientScope { authenticate } => {
            let mut response = failure(
                StatusCode::FORBIDDEN,
                "forbidden",
                "You are not authorized to perform this action.",
                request_id,
            );
            if let Ok(value) = HeaderValue::from_str(&authenticate) {
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, value);
            }
            response
        }
        AuthenticationError::Unavailable => failure(
            StatusCode::SERVICE_UNAVAILABLE,
            "service_unavailable",
            "Service is temporarily unavailable",
            request_id,
        ),
    }
}

fn operation(error: OperationError, request_id: &str) -> Response {
    match error {
        OperationError::NotFound => failure(
            StatusCode::NOT_FOUND,
            "not_found",
            "Record not found",
            request_id,
        ),
        OperationError::Unauthenticated => {
            authentication(AuthenticationError::Unauthenticated, request_id)
        }
        OperationError::Forbidden => authentication(AuthenticationError::Forbidden, request_id),
        OperationError::Unavailable => failure(
            StatusCode::SERVICE_UNAVAILABLE,
            "service_unavailable",
            "Service is temporarily unavailable",
            request_id,
        ),
        _ => failure(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Internal server error",
            request_id,
        ),
    }
}

fn failure(status: StatusCode, code: &str, message: &str, request_id: &str) -> Response {
    reply(
        status,
        Some(ErrorResponse {
            error: Error {
                code,
                message,
                request_id,
                details: None,
            },
        }),
        request_id,
    )
}

fn reply(status: StatusCode, body: Option<impl serde::Serialize>, request_id: &str) -> Response {
    let response = match body {
        Some(body) if (status.is_client_error() || status.is_server_error()) => {
            match serde_json::to_value(&body) {
                Ok(mut value) => {
                    crate::telemetry::ids::enrich_error(&mut value);
                    (status, Json(value)).into_response()
                }
                Err(_) => (status, Json(body)).into_response(),
            }
        }
        Some(body) => (status, Json(body)).into_response(),
        None => status.into_response(),
    };
    response_headers(response, request_id)
}

fn response_headers(mut response: Response, request_id: &str) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if let Ok(value) = HeaderValue::from_str(request_id) {
        response.headers_mut().insert("x-request-id", value);
    }
    if response.status() == StatusCode::UNAUTHORIZED {
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_profile_response_preserves_null_name_and_typed_shortcuts()
    -> Result<(), serde_json::Error> {
        let response = representation(self_account::Snapshot {
            account_id: AccountId::from_database(42),
            name: None,
            saved_display_name: Some("private".into()),
            display_name_enabled: false,
            email: "profile@example.test".into(),
            time_zone: chrono_tz::Europe::London,
            gravatar_enabled: false,
            avatar_attached: true,
            mobile_shortcuts: vec![self_account::Shortcut::MedicineReviews],
        });
        assert_eq!(response.data.account_id, "42");
        let body = serde_json::to_value(response)?;
        assert!(body["data"]["name"].is_null());
        assert_eq!(
            body["data"]["mobile_shortcuts"],
            serde_json::json!(["medicine_reviews"])
        );
        assert!(body["data"].get("saved_display_name").is_none());
        Ok(())
    }

    #[tokio::test]
    async fn unavailable_maps_to_service_unavailable_envelope() {
        for response in [
            authentication(AuthenticationError::Unavailable, "request-id"),
            operation(OperationError::Unavailable, "request-id"),
        ] {
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            let bytes = axum::body::to_bytes(response.into_body(), 4096)
                .await
                .expect("error envelope body");
            let body: Value = serde_json::from_slice(&bytes).expect("error envelope JSON");
            assert_eq!(body["error"]["code"], "service_unavailable");
            assert_eq!(body["error"]["request_id"], "request-id");
        }
    }
}
