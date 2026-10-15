use axum::{
    Json,
    extract::{State, rejection::JsonRejection},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};
use serde::{Deserialize, Serialize};

use crate::models::{
    errors::OperationError,
    identity::resource::{self, AuthenticationError},
    ids::AccountId,
    profile::{self, self_account as account},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Input {
    profile: Patch,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Patch {
    #[serde(default, deserialize_with = "present")]
    name: Option<String>,
    #[serde(default, deserialize_with = "present")]
    time_zone: Option<String>,
    #[serde(default, deserialize_with = "present")]
    gravatar_enabled: Option<bool>,
    #[serde(default, deserialize_with = "present")]
    mobile_shortcuts: Option<Vec<String>>,
}

fn present<'de, D, T>(deserializer: D) -> std::result::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Serialize)]
struct Representation {
    account_id: String,
    name: String,
    email: String,
    time_zone: String,
    gravatar_enabled: bool,
    mobile_shortcuts: Vec<String>,
    avatar_attached: bool,
}

impl From<account::Snapshot> for Representation {
    fn from(snapshot: account::Snapshot) -> Self {
        Self {
            account_id: snapshot.account_id.to_string(),
            name: snapshot.name,
            email: snapshot.email,
            time_zone: snapshot.time_zone,
            gravatar_enabled: snapshot.gravatar_enabled,
            mobile_shortcuts: snapshot.mobile_shortcuts,
            avatar_attached: snapshot.avatar_attached,
        }
    }
}

#[derive(Serialize)]
struct Body {
    data: Representation,
}

pub(super) fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub(super) async fn show(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
) -> Response {
    execute(ctx, headers, request, None).await
}

pub(super) async fn update(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
    body: std::result::Result<Json<Input>, JsonRejection>,
) -> Response {
    execute(ctx, headers, request, Some(body)).await
}

async fn execute(
    ctx: AppContext,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
    body: Option<std::result::Result<Json<Input>, JsonRejection>>,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |axum::Extension(id)| id.get().to_owned(),
    );
    let principal =
        match resource::authenticate(&ctx.db, &headers, ctx.config.settings.as_ref()).await {
            Ok(value) => value,
            Err(error) => return authentication(error, &request_id),
        };
    let write = body.is_some();
    let changes = match body {
        Some(Ok(Json(input))) => match account::ValidatedChanges::parse(
            input.profile.name,
            profile::Changes {
                date_of_birth: None,
                time_zone: input.profile.time_zone,
                gravatar_enabled: input.profile.gravatar_enabled,
                mobile_shortcuts: input.profile.mobile_shortcuts,
            },
        ) {
            Ok(value) => Some(value),
            Err(error) => return operation(error, &request_id),
        },
        Some(Err(error)) => {
            return failure(
                error.status(),
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
        Ok(value) => value,
        Err(error) => return authentication(error, &request_id),
    };
    let account_id = AccountId::from_database(principal.account_id());
    let result = async {
        if let Some(changes) = changes {
            changes.apply(&transaction, account_id, &request_id).await?;
        }
        account::read(&transaction, account_id, ctx.config.settings.as_ref()).await
    }
    .await;
    match result {
        Ok(snapshot) => {
            if let Err(error) = transaction.commit().await {
                return operation(error.into(), &request_id);
            }
            let mut response = Json(Body {
                data: snapshot.into(),
            })
            .into_response();
            if let Ok(value) = HeaderValue::from_str(&request_id) {
                response.headers_mut().insert("x-request-id", value);
            }
            response
        }
        Err(error) => {
            if let Err(rollback) = transaction.rollback().await {
                return operation(rollback.into(), &request_id);
            }
            operation(error, &request_id)
        }
    }
}

pub(super) fn authentication(error: AuthenticationError, request_id: &str) -> Response {
    match error {
        AuthenticationError::Unauthenticated => failure(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Authentication required",
            request_id,
        ),
        AuthenticationError::Forbidden => failure(
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
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Internal server error",
            request_id,
        ),
    }
}

pub(super) fn operation(error: OperationError, request_id: &str) -> Response {
    match error {
        OperationError::Unauthenticated => {
            authentication(AuthenticationError::Unauthenticated, request_id)
        }
        OperationError::Forbidden => authentication(AuthenticationError::Forbidden, request_id),
        OperationError::Validation { .. } => failure(
            StatusCode::UNPROCESSABLE_ENTITY,
            "unprocessable_content",
            "Profile contains an invalid field",
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

pub(super) fn failure(status: StatusCode, code: &str, message: &str, request_id: &str) -> Response {
    #[derive(Serialize)]
    struct Error<'a> {
        code: &'a str,
        message: &'a str,
        request_id: &'a str,
    }
    #[derive(Serialize)]
    struct Envelope<'a> {
        error: Error<'a>,
    }
    let mut response = (
        status,
        Json(Envelope {
            error: Error {
                code,
                message,
                request_id,
            },
        }),
    )
        .into_response();
    if let Ok(value) = HeaderValue::from_str(request_id) {
        response.headers_mut().insert("x-request-id", value);
    }
    if status == StatusCode::UNAUTHORIZED {
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    }
    response
}
