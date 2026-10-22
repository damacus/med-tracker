use std::cell::RefCell;

use async_trait::async_trait;
use axum::{
    Router,
    extract::{Request, State},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use better_auth::integrations::axum::AxumIntegration;
use better_auth_core::{
    AuthError, AuthRequest, AuthResponse, AuthResult,
    plugin::{AuthContext, AuthPlugin, AuthRoute, BeforeRequestAction},
};
use loco_rs::controller::middleware::request_id::LocoRequestId;

use super::{ClinicalAuthSchema, IdentityService, policy::clinical_id};

#[derive(Clone)]
pub(super) struct AuthenticatedRequest {
    pub account_id: i64,
    pub session_token: String,
}

struct RequestContext {
    request_id: String,
    principal: Option<AuthenticatedRequest>,
}

tokio::task_local! {
    static REQUEST: RefCell<RequestContext>;
}

pub(super) fn authenticated() -> AuthResult<AuthenticatedRequest> {
    REQUEST
        .try_with(|context| context.borrow().principal.clone())
        .ok()
        .flatten()
        .ok_or(AuthError::Unauthenticated)
}

pub(super) fn request_id() -> Option<String> {
    REQUEST
        .try_with(|context| context.borrow().request_id.clone())
        .ok()
}

pub(super) struct ClinicalRequest(pub std::sync::Arc<super::IdentityPolicy>);

pub enum BrowserIdentity {
    SignedOut,
    Enrolment {
        user: better_auth_core::wire::UserView,
        session: better_auth_core::wire::SessionView,
    },
    Authenticated {
        user: better_auth_core::wire::UserView,
        session: better_auth_core::wire::SessionView,
    },
}

pub async fn browser_identity(
    service: &IdentityService,
    request: &AuthRequest,
) -> AuthResult<BrowserIdentity> {
    let authoritative = match service
        .context()
        .require_authoritative_session(request)
        .await
    {
        Ok((user, session)) => match service.policy.admit_session(&session).await {
            Ok(()) => Ok((user, session)),
            Err(error) => Err(error),
        },
        Err(error) => Err(error),
    };
    match authoritative {
        Ok((user, session)) => match session
            .additional_fields
            .get("clinical_session_purpose")
            .and_then(better_auth_core::FieldValue::as_str)
        {
            Some("authenticated") => Ok(BrowserIdentity::Authenticated { user, session }),
            Some("enrolment") => Ok(BrowserIdentity::Enrolment { user, session }),
            _ => Err(AuthError::Unauthenticated),
        },
        Err(AuthError::Unauthenticated | AuthError::SessionNotFound) => {
            Ok(BrowserIdentity::SignedOut)
        }
        Err(error) => Err(error),
    }
}

fn has_clinical_assurance(fields: &better_auth_core::FieldMap) -> bool {
    fields
        .get("clinical_session_purpose")
        .and_then(better_auth_core::FieldValue::as_str)
        == Some("authenticated")
}

#[async_trait]
impl AuthPlugin<ClinicalAuthSchema> for ClinicalRequest {
    fn name(&self) -> &'static str {
        "clinical-request"
    }

    fn routes(&self) -> Vec<AuthRoute> {
        Vec::new()
    }

    async fn before_request(
        &self,
        request: &AuthRequest,
        context: &AuthContext<ClinicalAuthSchema>,
    ) -> AuthResult<Option<BeforeRequestAction>> {
        let totp = matches!(
            request.path(),
            "/security/api-key/create"
                | "/security/api-key/revoke"
                | "/security/session/revoke"
                | "/security/operation/start"
                | "/security/totp/start"
                | "/security/totp/finish"
                | "/security/totp/login"
                | "/security/totp/disable/confirm"
                | "/security/passkey/start"
                | "/security/passkey/confirm"
        );
        if !totp
            && !request.path().starts_with("/reset-password/")
            && !matches!(
                request.path(),
                "/onboarding/signup"
                    | "/sign-up/email"
                    | "/verify-email"
                    | "/change-email"
                    | "/change-password"
                    | "/unlink-account"
                    | "/passkey/delete-passkey"
                    | "/verify-password"
                    | "/request-password-reset"
                    | "/reset-password"
                    | "/sign-in/social"
                    | "/link-social"
                    | "/callback/zitadel"
                    | "/two-factor/enable"
                    | "/two-factor/disable"
                    | "/two-factor/verify-totp"
                    | "/two-factor/verify-backup-code"
                    | "/two-factor/generate-backup-codes"
                    | "/send-verification-email"
                    | "/onboarding/confirm-email"
                    | "/sign-in/email"
                    | "/passkey/generate-authenticate-options"
                    | "/passkey/verify-authentication"
                    | "/passkey/generate-register-options"
                    | "/passkey/verify-registration"
                    | "/onboarding/complete"
                    | "/get-session"
                    | "/revoke-session"
                    | "/sign-out"
                    | "/security/password/confirm"
            )
        {
            return Err(AuthError::forbidden(
                "This account operation is unavailable",
            ));
        }
        if matches!(
            request.path(),
            "/sign-up/email" | "/reset-password" | "/change-password"
        ) {
            let body: serde_json::Value = request.body_as_json()?;
            if let Some(password) = body
                .get(
                    if request.path() == "/reset-password" || request.path() == "/change-password" {
                        "newPassword"
                    } else {
                        "password"
                    },
                )
                .and_then(serde_json::Value::as_str)
            {
                super::validate_new_password(password).map_err(AuthError::validation)?;
            }
        }
        let authoritative = match context.require_authoritative_session(request).await {
            Ok((user, session)) => {
                self.0.admit_session(&session).await?;
                let onboarding = request.path().starts_with("/reset-password/")
                    || matches!(
                        request.path(),
                        "/request-password-reset"
                            | "/reset-password"
                            | "/onboarding/complete"
                            | "/passkey/generate-register-options"
                            | "/passkey/verify-registration"
                            | "/get-session"
                            | "/sign-out"
                            | "/verify-email"
                            | "/send-verification-email"
                    );
                if !has_clinical_assurance(&session.additional_fields) && !onboarding {
                    return Err(AuthError::forbidden(
                        "Complete account setup before changing security settings",
                    ));
                }
                Ok((user, session))
            }
            Err(error) => Err(error),
        };
        let principal = match authoritative {
            Ok((user, session)) if has_clinical_assurance(&session.additional_fields) => Some((
                clinical_id(&user.id.display_string()?)?,
                session.token.typed()?.clone(),
            )),
            Ok(_) => None,
            Err(AuthError::Unauthenticated | AuthError::SessionNotFound) => None,
            Err(error) => return Err(error),
        };
        REQUEST
            .try_with(|state| {
                let mut state = state.borrow_mut();
                state.principal =
                    principal.map(|(account_id, session_token)| AuthenticatedRequest {
                        account_id,
                        session_token,
                    });
            })
            .map_err(|_| AuthError::internal("Identity request context missing"))?;
        Ok(None)
    }

    async fn on_request(
        &self,
        _request: &AuthRequest,
        _context: &AuthContext<ClinicalAuthSchema>,
    ) -> AuthResult<Option<AuthResponse>> {
        Ok(None)
    }
}

pub fn router(service: IdentityService) -> Router {
    service
        .auth
        .clone()
        .axum_router_with_state::<IdentityService>()
        .route(
            "/security/session/revoke",
            axum::routing::post(revoke_session),
        )
        .route("/onboarding/signup", axum::routing::post(revoke_session))
        .route(
            "/security/password/confirm",
            axum::routing::post(revoke_session),
        )
        .route(
            "/security/passkey/start",
            axum::routing::post(revoke_session),
        )
        .route(
            "/security/passkey/confirm",
            axum::routing::post(revoke_session),
        )
        .route("/security/totp/start", axum::routing::post(revoke_session))
        .route("/security/totp/finish", axum::routing::post(revoke_session))
        .route("/security/totp/login", axum::routing::post(revoke_session))
        .route(
            "/security/totp/disable/confirm",
            axum::routing::post(revoke_session),
        )
        .with_state(service.clone())
        .layer(middleware::from_fn_with_state(service, request_context))
}

async fn revoke_session(
    State(service): State<IdentityService>,
    axum::Extension(request): axum::Extension<AuthRequest>,
) -> Response {
    let request_id = request_id().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    match dispatch(&service, request, request_id).await {
        Ok(response) => better_auth_core::error::materialize_axum_response(response)
            .unwrap_or_else(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        Err(error) => error.into_response(),
    }
}

async fn request_context(
    State(service): State<IdentityService>,
    mut request: Request,
    next: Next,
) -> Response {
    if let Err(status) = super::normalize_cookie_headers(request.headers_mut()) {
        return (status, [(axum::http::header::CACHE_CONTROL, "no-store")]).into_response();
    }
    let (parts, body) = request.into_parts();
    let Ok(body) = axum::body::to_bytes(body, 65536).await else {
        return axum::http::StatusCode::PAYLOAD_TOO_LARGE.into_response();
    };
    let mut preflight = AuthRequest::new(
        if parts.method == axum::http::Method::POST {
            better_auth_core::HttpMethod::Post
        } else {
            better_auth_core::HttpMethod::Get
        },
        parts.uri.path().trim_start_matches("/api/auth"),
    );
    preflight.headers = parts
        .headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_owned(), value.to_owned()))
        })
        .collect();
    preflight.body = Some(body.to_vec());
    preflight.query = parts
        .uri
        .query()
        .map(|query| {
            serde_urlencoded::from_str::<std::collections::HashMap<String, String>>(query)
                .map(serde_json::to_value)
        })
        .transpose()
        .map_err(|_| ())
        .ok()
        .flatten()
        .and_then(Result::ok);
    let dispatched = matches!(
        preflight.path(),
        "/security/session/revoke"
            | "/onboarding/signup"
            | "/security/password/confirm"
            | "/security/passkey/start"
            | "/security/passkey/confirm"
            | "/security/totp/start"
            | "/security/totp/finish"
            | "/security/totp/login"
            | "/security/totp/disable/confirm"
    );
    if !dispatched {
        match super::limits::check(&service, &preflight).await {
            Ok(Some(response)) => {
                return better_auth_core::error::materialize_axum_response(response)
                    .unwrap_or_else(|_| {
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
                    });
            }
            Err(error) => return error.into_response(),
            Ok(None) => {}
        }
    }
    let mut request = Request::from_parts(parts, axum::body::Body::from(body));
    request.extensions_mut().insert(preflight.clone());
    let signup = request.uri().path().ends_with("/onboarding/signup");
    let request_id = request
        .extensions()
        .get::<LocoRequestId>()
        .map(|id| id.get().to_owned())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    match REQUEST
        .scope(
            RefCell::new(RequestContext {
                request_id,
                principal: None,
            }),
            async { Ok::<_, AuthError>(next.run(request).await) },
        )
        .await
    {
        Ok(response) if signup && response.status() == axum::http::StatusCode::CONFLICT => {
            axum::Json(serde_json::json!({"status":true})).into_response()
        }
        Ok(mut response) if response.status().is_server_error() => {
            response
                .headers_mut()
                .remove(axum::http::header::SET_COOKIE);
            response
        }
        Ok(response) => response,
        Err(error) => error.into_response(),
    }
}

pub fn dispatch(
    service: &IdentityService,
    request: AuthRequest,
    request_id: String,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = AuthResult<AuthResponse>> + Send + '_>> {
    Box::pin(async move {
        if let Some(response) = super::limits::check(service, &request).await? {
            return Ok(response);
        }
        let signup = request.path == "/onboarding/signup";
        let result = REQUEST
            .scope(
                RefCell::new(RequestContext {
                    request_id,
                    principal: None,
                }),
                async {
                    if matches!(
                        request.path(),
                        "/security/session/revoke"
                            | "/onboarding/signup"
                            | "/security/password/confirm"
                            | "/security/passkey/start"
                            | "/security/passkey/confirm"
                            | "/security/totp/start"
                            | "/security/totp/finish"
                            | "/security/totp/login"
                            | "/security/totp/disable/confirm"
                    ) {
                        use better_auth_core::middleware::Middleware;
                        let csrf = better_auth_core::middleware::CsrfMiddleware::new(
                            better_auth_core::middleware::CsrfConfig::new(),
                            service.context().config.clone(),
                        );
                        if let Some(response) = csrf.before_request(&request).await? {
                            return Ok(response);
                        }
                        ClinicalRequest(service.policy.clone())
                            .before_request(&request, service.context())
                            .await?;
                        return match request.path() {
                            "/security/session/revoke" => {
                                super::sessions::revoke(service, &request).await
                            }
                            "/onboarding/signup" => {
                                super::onboarding::signup(service, &request).await
                            }
                            "/security/password/confirm" => {
                                super::operations::handle(service, &request).await
                            }
                            "/security/passkey/start" => {
                                super::passkeys::start(service, &request).await
                            }
                            "/security/passkey/confirm" => {
                                super::passkeys::confirm(service, &request).await
                            }
                            _ => super::totp::handle(service, &request).await,
                        };
                    }
                    service.handle_request(request).await
                },
            )
            .await;
        match result {
            Err(AuthError::Conflict(_)) if signup => Ok(AuthResponse::json(
                200,
                &serde_json::json!({"status":true}),
            )?),
            Ok(response) if signup && response.status == 409 => Ok(AuthResponse::json(
                200,
                &serde_json::json!({"status":true}),
            )?),
            Ok(mut response) if response.status >= 500 => {
                response.headers.remove("set-cookie");
                Ok(response)
            }
            result => result,
        }
    })
}

pub(crate) async fn trusted<T>(
    service: &IdentityService,
    request: &AuthRequest,
    request_id: String,
    work: impl std::future::Future<Output = AuthResult<T>>,
) -> AuthResult<T> {
    REQUEST
        .scope(
            RefCell::new(RequestContext {
                request_id,
                principal: None,
            }),
            async {
                ClinicalRequest(service.policy.clone())
                    .before_request(request, service.context())
                    .await?;
                work.await
            },
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::has_clinical_assurance;
    use better_auth_core::{FieldMap, FieldValue};
    use serde_json::Value;

    #[test]
    fn only_explicit_authenticated_sessions_admit_clinical_requests() {
        for value in [
            Value::Null,
            Value::Bool(true),
            Value::String("enrolment".into()),
            Value::String("pending".into()),
        ] {
            assert!(!has_clinical_assurance(&FieldMap::from_iter([(
                "clinical_session_purpose".into(),
                FieldValue::from_json(value).unwrap()
            )])));
        }
        assert!(!has_clinical_assurance(&FieldMap::new()));
        assert!(has_clinical_assurance(&FieldMap::from_iter([(
            "clinical_session_purpose".into(),
            FieldValue::from_json(Value::String("authenticated".into())).unwrap()
        )])));
    }
}
