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

use super::{ClinicalAuthSchema, IdentityService, store::clinical_id};

#[derive(Clone)]
pub(super) struct AuthenticatedRequest {
    pub account_id: i64,
    pub session_token: String,
    pub request_id: String,
    pub active_household_id: Option<i64>,
}

struct RequestContext {
    request_id: String,
    principal: Option<AuthenticatedRequest>,
    recovery_issuance_account: Option<i64>,
    totp_failure_status: Option<u16>,
    support_expiry_denial: bool,
    password_login: bool,
    email_change: Option<(String, String)>,
}

pub(super) fn allow_email_change(account: &str, email: &str) -> AuthResult<()> {
    REQUEST
        .try_with(|state| state.borrow_mut().email_change = Some((account.into(), email.into())))
        .map_err(|_| AuthError::internal("Identity request context missing"))
}

pub(super) fn email_change_allowed(account: &str, email: &str) -> bool {
    REQUEST
        .try_with(|state| {
            state
                .borrow()
                .email_change
                .as_ref()
                .is_some_and(|(allowed_account, allowed_email)| {
                    allowed_account == account && allowed_email == email
                })
        })
        .unwrap_or(false)
}

pub(super) fn password_login() -> bool {
    REQUEST
        .try_with(|state| state.borrow().password_login)
        .unwrap_or(false)
}

pub(super) fn persist_totp_failure(status: u16) -> AuthResult<()> {
    REQUEST
        .try_with(|state| state.borrow_mut().totp_failure_status = Some(status))
        .map_err(|_| AuthError::internal("Identity request context missing"))
}

pub(super) fn persist_support_expiry() -> AuthResult<()> {
    REQUEST
        .try_with(|state| state.borrow_mut().support_expiry_denial = true)
        .map_err(|_| AuthError::internal("Identity request context missing"))
}

fn commits_status(status: u16) -> bool {
    status < 400
        || REQUEST
            .try_with(|state| {
                let state = state.borrow();
                state.totp_failure_status == Some(status)
                    || (status == 403 && state.support_expiry_denial)
            })
            .unwrap_or(false)
}

pub(super) fn allow_recovery_issuance(account_id: i64) -> AuthResult<()> {
    REQUEST
        .try_with(|state| state.borrow_mut().recovery_issuance_account = Some(account_id))
        .map_err(|_| AuthError::internal("Identity request context missing"))
}

pub(super) fn recovery_issuance_allowed(account_id: i64) -> bool {
    REQUEST
        .try_with(|state| state.borrow().recovery_issuance_account == Some(account_id))
        .unwrap_or(false)
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

pub(super) struct ClinicalRequest;

pub enum BrowserIdentity {
    SignedOut,
    PendingFactor,
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
    match service
        .context()
        .require_authoritative_session(request)
        .await
    {
        Ok((_, session))
            if session
                .additional_fields
                .get("legacy_factor_pending")
                .and_then(serde_json::Value::as_bool)
                == Some(true) =>
        {
            Ok(BrowserIdentity::PendingFactor)
        }
        Ok((user, session)) => match session
            .additional_fields
            .get("clinical_session_purpose")
            .and_then(serde_json::Value::as_str)
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

fn has_clinical_assurance(fields: &serde_json::Map<String, serde_json::Value>) -> bool {
    fields
        .get("clinical_session_purpose")
        .and_then(serde_json::Value::as_str)
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
            "/security/api-key/revoke"
                | "/security/email/confirm"
                | "/security/session/revoke"
                | "/security/recovery/acknowledge"
                | "/security/operation/start"
                | "/security/password/email/confirm"
                | "/security/totp/start"
                | "/security/totp/finish"
                | "/security/totp/login"
                | "/security/totp/disable/start"
                | "/security/totp/disable/confirm"
                | "/security/passkey/start"
                | "/security/passkey/confirm"
        );
        if !totp
            && !matches!(
                request.path(),
                "/onboarding/signup"
                    | "/send-verification-email"
                    | "/onboarding/confirm-email"
                    | "/sign-in/email"
                    | "/recovery/login"
                    | "/passkey/generate-authenticate-options"
                    | "/passkey/verify-authentication"
                    | "/passkey/generate-register-options"
                    | "/passkey/verify-registration"
                    | "/onboarding/recovery-codes"
                    | "/onboarding/complete"
                    | "/get-session"
                    | "/sign-out"
                    | "/security/password/start"
                    | "/security/password/confirm"
            )
        {
            return Err(AuthError::forbidden(
                "This account operation is unavailable",
            ));
        }
        let principal = match context.require_authoritative_session(request).await {
            Ok((user, session)) if has_clinical_assurance(&session.additional_fields) => Some((
                clinical_id(&user.id)?,
                session.token,
                session
                    .active_organization_id
                    .as_deref()
                    .map(clinical_id)
                    .transpose()?,
            )),
            Ok(_) => None,
            Err(AuthError::Unauthenticated | AuthError::SessionNotFound) => None,
            Err(error) => return Err(error),
        };
        REQUEST
            .try_with(|state| {
                let mut state = state.borrow_mut();
                state.principal =
                    principal.map(|(account_id, session_token, active_household_id)| {
                        AuthenticatedRequest {
                            account_id,
                            session_token,
                            active_household_id,
                            request_id: state.request_id.clone(),
                        }
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
        .axum_router()
        .with_state(service.auth.clone())
        .layer(middleware::from_fn_with_state(service, request_context))
}

async fn request_context(
    State(service): State<IdentityService>,
    request: Request,
    next: Next,
) -> Response {
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
    match super::limits::check(&service, &preflight).await {
        Ok(Some(response)) => {
            let mut builder = axum::http::Response::builder().status(response.status);
            for (name, value) in response.headers.iter() {
                builder = builder.header(name, value);
            }
            return builder
                .body(axum::body::Body::from(response.body))
                .unwrap_or_else(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
        Err(error) => return error.into_response(),
        Ok(None) => {}
    }
    let request = Request::from_parts(parts, axum::body::Body::from(body));
    let signup = request.uri().path().ends_with("/onboarding/signup");
    let password_login = request.uri().path().ends_with("/sign-in/email");
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
                recovery_issuance_account: None,
                totp_failure_status: None,
                support_expiry_denial: false,
                password_login,
                email_change: None,
            }),
            service
                .store
                .atomic(async { Ok(next.run(request).await) }, |response| {
                    commits_status(response.status().as_u16())
                }),
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

pub async fn dispatch(
    service: &IdentityService,
    request: AuthRequest,
    request_id: String,
) -> AuthResult<AuthResponse> {
    if let Some(response) = super::limits::check(service, &request).await? {
        return Ok(response);
    }
    let signup = request.path == "/onboarding/signup";
    let password_login = request.path == "/sign-in/email";
    let result = REQUEST
        .scope(
            RefCell::new(RequestContext {
                request_id,
                principal: None,
                recovery_issuance_account: None,
                totp_failure_status: None,
                support_expiry_denial: false,
                password_login,
                email_change: None,
            }),
            service
                .store
                .atomic(service.handle_request(request), |response| {
                    commits_status(response.status)
                }),
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
}

pub(super) async fn trusted<T>(
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
                recovery_issuance_account: None,
                totp_failure_status: None,
                support_expiry_denial: false,
                password_login: false,
                email_change: None,
            }),
            service.store.atomic(
                async {
                    ClinicalRequest
                        .before_request(request, service.context())
                        .await?;
                    work.await
                },
                |_| true,
            ),
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::has_clinical_assurance;
    use serde_json::{Map, Value};

    #[test]
    fn only_explicit_authenticated_sessions_admit_clinical_requests() {
        for value in [
            Value::Null,
            Value::Bool(true),
            Value::String("enrolment".into()),
            Value::String("pending".into()),
        ] {
            assert!(!has_clinical_assurance(&Map::from_iter([(
                "clinical_session_purpose".into(),
                value
            )])));
        }
        assert!(!has_clinical_assurance(&Map::new()));
        assert!(has_clinical_assurance(&Map::from_iter([(
            "clinical_session_purpose".into(),
            Value::String("authenticated".into())
        )])));
    }
}
