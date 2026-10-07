use axum::{Extension, http::HeaderMap};
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use better_auth_core::{AuthRequest, HttpMethod};
use loco_rs::prelude::*;

use crate::models::identity::better_auth::{BrowserIdentity, IdentityService, browser_identity};

pub(super) fn request(headers: &HeaderMap) -> AuthRequest {
    let mut request = AuthRequest::new(HttpMethod::Get, "/get-session");
    request.headers = headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_owned(), value.to_owned()))
        })
        .collect();
    request
}

pub(super) async fn setup(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    match browser_identity(&service, &request(&headers)).await {
        Ok(BrowserIdentity::Enrolment { user, .. }) => {
            match service.context().database.get_user_accounts(&user.id).await {
                Ok(accounts) => {
                    let Ok(passkeys) = service
                        .context()
                        .database
                        .list_passkeys_by_user(&user.id)
                        .await
                    else {
                        return super::rendering::unavailable();
                    };
                    super::rendering::setup(
                        &view,
                        !passkeys.is_empty()
                            || accounts.iter().any(|account| {
                                account.provider_id == "credential" && account.password.is_some()
                            }),
                    )
                }
                Err(_) => super::rendering::unavailable(),
            }
        }
        Ok(BrowserIdentity::Authenticated { .. }) => super::rendering::redirect("/"),
        Ok(BrowserIdentity::PendingFactor) => super::rendering::redirect("/auth/totp"),
        Ok(BrowserIdentity::SignedOut) => super::rendering::redirect("/login"),
        Err(_) => super::rendering::unavailable(),
    }
}

pub(super) async fn complete(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    session: Session<SessionPgPool>,
) -> Response {
    match browser_identity(&service, &request(&headers)).await {
        Ok(BrowserIdentity::Authenticated { .. }) => {
            crate::controllers::auth::authenticated_redirect(&session)
        }
        Ok(BrowserIdentity::Enrolment { .. }) => super::rendering::redirect("/auth/passkey/setup"),
        Ok(BrowserIdentity::PendingFactor) => super::rendering::redirect("/auth/totp"),
        Ok(BrowserIdentity::SignedOut) => super::rendering::redirect("/login"),
        Err(_) => super::rendering::unavailable(),
    }
}
