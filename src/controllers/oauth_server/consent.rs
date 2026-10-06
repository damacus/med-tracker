use super::authorization_response;
use crate::models::identity::{
    authorization::{self, AuthorizationInput},
    browser,
    resource::AuthenticationError,
};
use axum::{
    Extension,
    extract::{Form, Query},
    http::{StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::controller::middleware::request_id::LocoRequestId;
use loco_rs::prelude::*;

pub fn routes() -> Routes {
    Routes::new().add("/authorize", get(consent).post(approve))
}

fn redirect(url: &str) -> Response {
    (
        StatusCode::SEE_OTHER,
        [(header::LOCATION, url), (header::CACHE_CONTROL, "no-store")],
    )
        .into_response()
}

async fn consent(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(fields): Query<Vec<(String, String)>>,
) -> Response {
    let mut input = AuthorizationInput(fields);
    let prepared = match authorization::prepare(&ctx.db, &input).await {
        Ok(prepared) => prepared,
        Err(error) => return authorization_response::failure(&view, &input, error),
    };
    if input.value("scope").is_none() {
        input.0.push((
            "scope".into(),
            prepared.pending.pre_grant().scope.to_string(),
        ));
    }
    session.set("oauth_pending", input.clone());
    session.set_store(true);
    let principal = match browser::authenticate(&ctx.db, &session).await {
        Ok(principal) => principal,
        Err(AuthenticationError::Unauthenticated) => return redirect("/login"),
        Err(_) => return StatusCode::FORBIDDEN.into_response(),
    };
    let mut data = match authorization::context(
        &ctx.db,
        &principal,
        prepared.application.client_kind == "mobile",
    )
    .await
    {
        Ok(data) => data,
        Err(error) => return super::failure(error),
    };
    let Ok(authenticity_token) = token.authenticity_token() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let scopes = prepared.pending.pre_grant().scope.to_string().split_whitespace().map(|value| {
        let (label, description) = match value {
            "medtracker" => ("MedTracker data", "View and manage your medication records."),
            "offline_access" => ("Stay signed in", "Allow this application to renew access until you revoke it."),
            "launch/patient" => ("Patient context", "Share the selected person's identity with this application."),
            _ => ("Read patient records", "Read the selected person's health records."),
        };
        serde_json::json!({"value":value,"label":label,"description":description,"offline":value=="offline_access"})
    }).collect::<Vec<_>>();
    let fields = input
        .0
        .iter()
        .filter(|(name, _)| name != "scope")
        .map(|(name, value)| serde_json::json!({"name":name,"value":value}))
        .collect::<Vec<_>>();
    let object = data.as_object_mut().expect("authorization context object");
    object.extend(serde_json::json!({"title":"Authorise access","allow_palette":false,"appearances":[{"id":"light","label":"Light"},{"id":"dark","label":"Dark"},{"id":"system","label":"System"}],"palettes":[],"client_name":prepared.application.name,"fields":fields,"scopes":scopes,"authenticity_token":authenticity_token}).as_object().expect("consent context object").clone());
    match format::render().view(&view, "oauth/consent.html", data) {
        Ok(response) => (token, [(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(error) => error.into_response(),
    }
}

async fn approve(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    request_id: Option<Extension<LocoRequestId>>,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(fields): Form<Vec<(String, String)>>,
) -> Response {
    let supplied = fields
        .iter()
        .filter(|(name, _)| name == "authenticity_token")
        .collect::<Vec<_>>();
    if supplied.len() != 1 || token.verify(&supplied[0].1).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(original) = session.get::<AuthorizationInput>("oauth_pending") else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let protocol_fields = fields
        .iter()
        .filter(|(name, _)| {
            name != "scope[]" && name != "authenticity_token" && name != "consent_action"
        })
        .cloned()
        .collect::<Vec<_>>();
    let expected = original
        .0
        .iter()
        .filter(|(name, _)| name != "scope")
        .cloned()
        .collect::<Vec<_>>();
    if protocol_fields != expected {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let principal = match browser::authenticate(&ctx.db, &session).await {
        Ok(principal) => principal,
        Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let actions = fields
        .iter()
        .filter(|(name, _)| name == "consent_action")
        .map(|(_, value)| value.as_str())
        .collect::<Vec<_>>();
    if actions.len() > 1
        || actions
            .first()
            .is_some_and(|action| !matches!(*action, "approve" | "deny"))
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    if actions.first() == Some(&"deny") {
        return match authorization::deny(&ctx.db, &original).await {
            Ok(output) => {
                session.remove("oauth_pending");
                authorization_response::render(&view, &original, output)
            }
            Err(error) => authorization_response::failure(&view, &original, error),
        };
    }
    let selected = fields
        .iter()
        .filter(|(name, _)| name == "scope[]")
        .map(|(_, value)| value.as_str())
        .collect::<Vec<_>>();
    let requested = original
        .value("scope")
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>();
    if selected.is_empty() || selected.iter().any(|scope| !requested.contains(scope)) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let mut input = AuthorizationInput(protocol_fields);
    input.0.push(("scope".into(), selected.join(" ")));
    let request_id = request_id.map(|Extension(id)| id.get().to_owned());
    match authorization::approve(&ctx.db, &principal, &input, request_id.as_deref()).await {
        Ok(output) => {
            session.remove("oauth_pending");
            authorization_response::render(&view, &input, output)
        }
        Err(error) => authorization_response::failure(&view, &input, error),
    }
}
