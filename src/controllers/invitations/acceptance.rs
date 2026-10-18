use super::*;
use crate::models::identity::{browser, resource::AuthenticationError};
use better_auth_core::store::SessionStore;
use serde::Deserialize;

struct Invitation {
    token: String,
    language: &'static str,
    explicit_language: bool,
}

#[derive(Deserialize)]
pub(super) struct Accept {
    token: String,
    authenticity_token: String,
    lang: Option<String>,
}

pub(super) async fn preview(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    incoming: Request,
) -> Response {
    let invitation = match parse_query(incoming.uri().query(), incoming.headers()) {
        Ok(value) => value,
        Err(_) => {
            return invalid_link(
                &view,
                fallback_language(incoming.headers()),
                StatusCode::UNPROCESSABLE_ENTITY,
            );
        }
    };
    let details = match invitations::preview(&ctx.db, &invitation.token).await {
        Ok(value) => value,
        Err(OperationError::Validation { .. } | OperationError::NotFound) => {
            return invalid_link(&view, invitation.language, StatusCode::NOT_FOUND);
        }
        Err(error) => {
            return super::super::administration::presentation_error(
                &view,
                invitation.language,
                error,
            );
        }
    };
    let signed_in =
        match browser::authenticate(&ctx.db, &session, ctx.config.settings.as_ref()).await {
            Ok(_) => true,
            Err(AuthenticationError::Unauthenticated) => {
                crate::controllers::auth::store_invitation_continuation(
                    &session,
                    &invitation.token,
                    invitation.explicit_language.then_some(invitation.language),
                );
                false
            }
            Err(error) => {
                return super::super::administration::presentation_authentication_error(
                    &view,
                    invitation.language,
                    error,
                );
            }
        };
    let Ok(authenticity_token) = token.authenticity_token() else {
        return super::super::administration::presentation_error(
            &view,
            invitation.language,
            OperationError::Unavailable,
        );
    };
    let Some(data) = acceptance_data(
        invitation.language,
        invitation.explicit_language,
        &invitation.token,
        details,
        signed_in,
        &authenticity_token,
    ) else {
        return unavailable();
    };
    match format::render().view(&view, "invitations/accept.html", data) {
        Ok(response) => (token, [(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(error) => error.into_response(),
    }
}

pub(super) async fn accept(
    State(ctx): State<AppContext>,
    Extension(service): Extension<crate::models::identity::better_auth::IdentityService>,
    session: Session<SessionPgPool>,
    (request, headers): (Option<Extension<LocoRequestId>>, axum::http::HeaderMap),
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(form): AxumForm<Accept>,
) -> Response {
    if token.verify(&form.authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let language = match form.lang.as_deref() {
        Some(value) => match supported_language(value) {
            Some(value) => value,
            None => {
                return invalid_link(
                    &view,
                    fallback_language(&headers),
                    StatusCode::UNPROCESSABLE_ENTITY,
                );
            }
        },
        None => fallback_language(&headers),
    };
    let explicit_language = form.lang.is_some();
    let principal =
        match browser::authenticate(&ctx.db, &session, ctx.config.settings.as_ref()).await {
            Ok(value) => value,
            Err(error) => {
                return super::super::administration::presentation_authentication_error(
                    &view, language, error,
                );
            }
        };
    let accepted = match invitations::accept_browser(
        &ctx.db,
        &principal,
        &form.token,
        &request_id(request),
        ctx.config.settings.as_ref(),
    )
    .await
    {
        Ok(value) => value,
        Err(OperationError::Validation { .. } | OperationError::NotFound) => {
            return invalid_link(&view, language, StatusCode::NOT_FOUND);
        }
        Err(error) => {
            return super::super::administration::presentation_error(&view, language, error);
        }
    };
    let homes = match principal.households(&ctx.db).await {
        Ok(value) => value,
        Err(_) => {
            return selection_recovery(
                &view,
                &token,
                &form.token,
                language,
                explicit_language,
                StatusCode::SERVICE_UNAVAILABLE,
            );
        }
    };
    let Some(home) = homes.iter().find(|home| {
        Some(home.id.to_string().as_str()) == accepted["data"]["household_id"].as_str()
    }) else {
        return selection_recovery(
            &view,
            &token,
            &form.token,
            language,
            explicit_language,
            StatusCode::SERVICE_UNAVAILABLE,
        );
    };
    if let Err(error) = service
        .store
        .update_session_active_organization(principal.session_token(), Some(&home.id.to_string()))
        .await
    {
        return selection_recovery(
            &view,
            &token,
            &form.token,
            language,
            explicit_language,
            StatusCode::from_u16(error.status_code()).unwrap_or(StatusCode::SERVICE_UNAVAILABLE),
        );
    }
    (
        StatusCode::SEE_OTHER,
        [
            (
                header::LOCATION,
                format!("/households/{}/dashboard", home.slug),
            ),
            (header::CACHE_CONTROL, "no-store".into()),
        ],
    )
        .into_response()
}

fn selection_recovery(
    view: &TeraView,
    token: &CsrfToken,
    invitation: &str,
    language: &str,
    explicit_language: bool,
    status: StatusCode,
) -> Response {
    let Ok(authenticity_token) = token.authenticity_token() else {
        return unavailable();
    };
    let mut data = crate::controllers::medications::rendering::appearance_context();
    let Ok(labels) = crate::models::care::report_pdf::translations(language) else {
        return unavailable();
    };
    data["lang"] = json!(language);
    data["labels"] = labels.clone();
    data["invitation_labels"] = labels["admin"]["invitations"]["acceptance"].clone();
    data["title"] = data["invitation_labels"]["recovery_title"].clone();
    data["invitation_token"] = json!(invitation);
    data["explicit_language"] = json!(explicit_language);
    data["authenticity_token"] = json!(authenticity_token);
    match format::render().view(view, "invitations/recovery.html", data) {
        Ok(response) => (
            status,
            token.clone(),
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}

fn parse_query(
    raw: Option<&str>,
    headers: &axum::http::HeaderMap,
) -> Result<Invitation, OperationError> {
    let mut token = None;
    let mut language = None;
    for (key, value) in url::form_urlencoded::parse(raw.unwrap_or_default().as_bytes()) {
        match key.as_ref() {
            "token" if token.is_none() && !value.is_empty() && value.len() <= 256 => {
                token = Some(value.into_owned());
            }
            "lang" if language.is_none() => {
                language = supported_language(&value);
                if language.is_none() {
                    return Err(OperationError::NotFound);
                }
            }
            _ => return Err(OperationError::NotFound),
        }
    }
    let token = token.ok_or(OperationError::NotFound)?;
    let explicit_language = language.is_some();
    let language = language.unwrap_or_else(|| fallback_language(headers));
    Ok(Invitation {
        token,
        language,
        explicit_language,
    })
}
