use axum::{
    Extension,
    extract::Form,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::models::{
    errors::OperationError,
    identity::better_auth::{BrowserIdentity, IdentityService, browser_identity, clinical_id},
    platform,
};

use super::rendering;

pub(super) async fn page(
    State(ctx): State<AppContext>,
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let identity = match browser_identity(&service, &super::passkeys::request(&headers)).await {
        Ok(identity) => identity,
        Err(_) => return rendering::unavailable(),
    };
    let BrowserIdentity::Authenticated { user, .. } = identity else {
        return rendering::redirect("/login");
    };
    let Ok(account_id) = user.id.typed().and_then(|id| clinical_id(id)) else {
        return rendering::unavailable();
    };
    let Ok(transaction) = platform::begin(&ctx.db).await else {
        return rendering::unavailable();
    };
    let sessions = match platform::support::owner_page(&transaction, account_id).await {
        Ok(sessions) => sessions,
        Err(error) => {
            let unauthenticated = matches!(error, OperationError::Unauthenticated);
            let persisted = matches!(error, OperationError::Forbidden);
            let ended = if persisted {
                transaction.commit().await
            } else {
                transaction.rollback().await
            };
            if ended.is_err() {
                return rendering::unavailable();
            }
            return if unauthenticated {
                rendering::redirect("/login")
            } else {
                crate::controllers::medications::operation_error(error)
            };
        }
    };
    let Ok(authenticity) = token.authenticity_token() else {
        return rendering::unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Support access");
    data["sessions"] = json!(
        sessions
            .iter()
            .map(|session| json!({
                "id": session.id,
                "household_name": session.household_name,
                "requester": session.requester,
                "reason": session.reason,
                "state": session.state,
                "expires_at": session.expires_at,
            }))
            .collect::<Vec<Value>>()
    );
    data["authenticity_token"] = json!(authenticity);
    let response = match format::render().view(&view, "identity_onboarding/support.html", data) {
        Ok(response) => (
            StatusCode::OK,
            token.clone(),
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(_) => rendering::unavailable(),
    };
    if transaction.commit().await.is_err() {
        return rendering::unavailable();
    }
    response
}

#[derive(Deserialize)]
pub(super) struct ApproveForm {
    support_id: i64,
    authenticity_token: String,
}

pub(super) async fn approve(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(form): Form<ApproveForm>,
) -> Response {
    crate::controllers::platform::proof(
        &service,
        &headers,
        request_id,
        token,
        &view,
        &form.authenticity_token,
        (
            json!({"action": "support_approve", "support_id": form.support_id}),
            "support access approval",
        ),
    )
    .await
}

#[derive(Deserialize)]
pub(super) struct EndForm {
    support_id: i64,
    authenticity_token: String,
}

pub(super) async fn end(
    State(ctx): State<AppContext>,
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    Form(form): Form<EndForm>,
) -> Response {
    crate::controllers::platform::end_support(
        &ctx,
        &service,
        &headers,
        request_id,
        token,
        form.support_id,
        &form.authenticity_token,
        "/account/support",
    )
    .await
}
