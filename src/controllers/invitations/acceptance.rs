use super::*;
use crate::models::identity::{browser, resource::AuthenticationError};
use axum::extract::Query;
use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct Invitation {
    token: String,
}

#[derive(Deserialize)]
pub(super) struct Accept {
    token: String,
    authenticity_token: String,
}

pub(super) async fn preview(
    State(ctx): State<AppContext>,
    Query(invitation): Query<Invitation>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let details = match invitations::preview(&ctx.db, &invitation.token).await {
        Ok(value) => value,
        Err(OperationError::Validation { .. } | OperationError::NotFound) => return invalid_link(),
        Err(error) => return operation_error(error),
    };
    let signed_in = match browser::authenticate(&ctx.db, &session).await {
        Ok(_) => true,
        Err(AuthenticationError::Unauthenticated) => {
            crate::controllers::auth::store_invitation_continuation(&session, &invitation.token);
            false
        }
        Err(error) => return authentication_error(error),
    };
    let Ok(authenticity_token) = token.authenticity_token() else {
        return unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    data["title"] = json!("Accept invitation");
    data["invitation"] = details;
    data["invitation_token"] = json!(invitation.token);
    data["signed_in"] = json!(signed_in);
    data["authenticity_token"] = json!(authenticity_token);
    match format::render().view(&view, "invitations/accept.html", data) {
        Ok(response) => (token, [(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(error) => error.into_response(),
    }
}

pub(super) async fn accept(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    AxumForm(form): AxumForm<Accept>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let principal = match browser::authenticate(&ctx.db, &session).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let accepted =
        match invitations::accept_browser(&ctx.db, &principal, &form.token, &request_id(request))
            .await
        {
            Ok(value) => value,
            Err(OperationError::Validation { .. } | OperationError::NotFound) => {
                return invalid_link();
            }
            Err(error) => return operation_error(error),
        };
    let homes = match principal.households(&ctx.db).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let Some(home) = homes.iter().find(|home| {
        Some(home.id.to_string().as_str()) == accepted["data"]["household_id"].as_str()
    }) else {
        return unavailable();
    };
    (
        StatusCode::SEE_OTHER,
        [
            (
                header::LOCATION,
                format!("/households/{}/medications", home.slug),
            ),
            (header::CACHE_CONTROL, "no-store".into()),
        ],
    )
        .into_response()
}

fn invalid_link() -> Response {
    (
        StatusCode::NOT_FOUND,
        [(header::CACHE_CONTROL, "no-store")],
        "This invitation link is invalid or has expired.",
    )
        .into_response()
}
