use axum::{Extension, http::{HeaderMap, StatusCode, header}, response::IntoResponse};
use axum_csrf::CsrfToken;
use loco_rs::prelude::*;
use serde_json::json;

use crate::models::identity::better_auth::{BrowserIdentity, IdentityService, browser_identity};

pub(super) async fn settings(Extension(service): Extension<IdentityService>, headers: HeaderMap, token: CsrfToken, ViewEngine(view): ViewEngine<TeraView>) -> Response {
    let identity = browser_identity(&service, &super::passkeys::request(&headers)).await;
    let Ok(BrowserIdentity::Authenticated { user, session }) = identity else { return super::rendering::redirect("/login"); };
    let Ok(sessions) = service.context().database.get_user_sessions(&user.id).await else { return super::rendering::unavailable(); };
    let Ok(accounts) = service.context().database.get_user_accounts(&user.id).await else { return super::rendering::unavailable(); };
    let Ok(passkeys) = service.context().database.list_passkeys_by_user(&user.id).await else { return super::rendering::unavailable(); };
    let Ok(authenticity_token) = token.authenticity_token() else { return super::rendering::unavailable(); };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Account security");
    data["email"] = json!(user.email);
    let Ok(retained_factor) = service.store.retained_factor_enabled(&user.id).await else { return super::rendering::unavailable(); };
    data["totp"] = json!(user.two_factor_enabled || retained_factor);
    data["password"] = json!(accounts.iter().any(|account| account.provider_id == "credential" && account.password.is_some()));
    data["passkeys"] = json!(passkeys.into_iter().map(|key| json!({"id":key.id,"name":key.name.unwrap_or_else(|| "Passkey".into())})).collect::<Vec<_>>());
    data["sessions"] = json!(sessions.into_iter().map(|entry| json!({"current":entry.id == session.id,"created":entry.created_at.format("%d %b %Y %H:%M UTC").to_string(),"expires":entry.expires_at.format("%d %b %Y %H:%M UTC").to_string()})).collect::<Vec<_>>());
    data["authenticity_token"] = json!(authenticity_token);
    match format::render().view(&view, "identity_onboarding/security.html", data) {
        Ok(response) => (StatusCode::OK, token, [(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(_) => super::rendering::unavailable(),
    }
}
