use crate::models::identity::{
    authorization::{AuthorizationInput, DisplayedConsent},
    browser,
    resource::AuthenticationError,
};
use crate::models::ids::HouseholdId;
use axum::{
    Extension,
    extract::{Form as AxumForm, FromRequestParts, Query},
    http::{StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};

pub async fn index(
    State(ctx): State<AppContext>,
    session: std::result::Result<
        Session<SessionPgPool>,
        <Session<SessionPgPool> as FromRequestParts<AppContext>>::Rejection,
    >,
) -> Response {
    if let Ok(session) = session {
        match browser::authenticate(&ctx.db, &session, ctx.config.settings.as_ref()).await {
            Ok(principal) => match principal.active_household(&ctx.db).await {
                Ok(Some(household)) => {
                    return redirect(&format!("/households/{}/dashboard", household.slug));
                }
                Ok(None) => return redirect("/households/choose"),
                Err(error) => return authentication_error(error),
            },
            Err(AuthenticationError::Unauthenticated) => {}
            Err(error) => return authentication_error(error),
        }
    }
    redirect("/login")
}

#[derive(serde::Deserialize)]
pub struct Choose {
    #[serde(default)]
    switch: Option<String>,
}

pub async fn choose(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(query): Query<Choose>,
) -> Response {
    let principal =
        match browser::authenticate(&ctx.db, &session, ctx.config.settings.as_ref()).await {
            Ok(principal) => principal,
            Err(AuthenticationError::Unauthenticated) => return redirect("/login"),
            Err(error) => return authentication_error(error),
        };
    if query.switch.is_none() {
        match principal.active_household(&ctx.db).await {
            Ok(Some(household)) => {
                return redirect(&format!("/households/{}/dashboard", household.slug));
            }
            Ok(None) => {}
            Err(error) => return authentication_error(error),
        }
    }
    let households = match principal.households(&ctx.db).await {
        Ok(households) => households,
        Err(error) => return authentication_error(error),
    };
    let auth_notice = session.get::<String>("auth_notice");
    session.remove("auth_notice");
    render_chooser(
        view,
        households,
        auth_notice,
        super::auth::pending_invitation_url(&session),
        token,
    )
}

fn render_chooser(
    view: TeraView,
    households: Vec<browser::BrowserHousehold>,
    auth_notice: Option<String>,
    pending_invitation: Option<String>,
    token: CsrfToken,
) -> Response {
    let Ok(authenticity_token) = token.authenticity_token() else {
        return unavailable();
    };
    match format::render().view(
        &view,
        "home/index.html",
        data!({
            "title": "Choose a household",
            "allow_palette": true,
            "auth_notice": auth_notice,
            "pending_invitation": pending_invitation,
            "households": households,
            "authenticity_token": authenticity_token
        }),
    ) {
        Ok(response) => (token, [(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(error) => error.into_response(),
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    household_id: String,
    authenticity_token: String,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Creation {
    name: String,
    authenticity_token: String,
}

pub async fn create(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    request: Option<Extension<LocoRequestId>>,
    form: std::result::Result<AxumForm<Creation>, axum::extract::rejection::FormRejection>,
) -> Response {
    let AxumForm(form) = match form {
        Ok(form) => form,
        Err(error) => return ([(header::CACHE_CONTROL, "no-store")], error).into_response(),
    };
    if token.verify(&form.authenticity_token).is_err() {
        return forbidden();
    }
    let principal =
        match browser::authenticate(&ctx.db, &session, ctx.config.settings.as_ref()).await {
            Ok(principal) => principal,
            Err(error) => return authentication_error(error),
        };
    let Some(name) = browser::household_name(&form.name) else {
        let households = match principal.households(&ctx.db).await {
            Ok(households) => households,
            Err(error) => return authentication_error(error),
        };
        if !households.is_empty() {
            return forbidden();
        }
        let mut response = render_chooser(
            view,
            households,
            Some("Enter a household name of up to 100 characters.".to_owned()),
            super::auth::pending_invitation_url(&session),
            token,
        );
        *response.status_mut() = StatusCode::UNPROCESSABLE_ENTITY;
        return response;
    };
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let household = match principal
        .create_first_household(&ctx.db, name, &request_id, ctx.config.settings.as_ref())
        .await
    {
        Ok(household) => household,
        Err(error) => return authentication_error(error),
    };
    let Ok(created_id) = household.id.to_string().parse::<HouseholdId>() else {
        return unavailable();
    };
    if principal
        .select_household(&ctx.db, created_id)
        .await
        .is_err()
    {
        session.set(
            "auth_notice",
            "Your household was created. Select it below to continue.",
        );
        return redirect("/households/choose");
    }
    if let Some(destination) = pending_household_authorization(&session) {
        return redirect(&destination);
    }
    (
        StatusCode::SEE_OTHER,
        [
            (
                header::LOCATION,
                format!("/households/{}/dashboard", household.slug),
            ),
            (header::CACHE_CONTROL, "no-store".to_owned()),
        ],
    )
        .into_response()
}

pub async fn select(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    form: std::result::Result<AxumForm<Selection>, axum::extract::rejection::FormRejection>,
) -> Response {
    let AxumForm(form) = match form {
        Ok(form) => form,
        Err(error) => return ([(header::CACHE_CONTROL, "no-store")], error).into_response(),
    };
    if token.verify(&form.authenticity_token).is_err() {
        return forbidden();
    }
    let principal =
        match browser::authenticate(&ctx.db, &session, ctx.config.settings.as_ref()).await {
            Ok(principal) => principal,
            Err(error) => return authentication_error(error),
        };
    let Ok(household_id) = form.household_id.parse::<HouseholdId>() else {
        return (StatusCode::NOT_FOUND, [(header::CACHE_CONTROL, "no-store")]).into_response();
    };
    match principal.select_household(&ctx.db, household_id).await {
        Ok(household) => match pending_household_authorization(&session) {
            Some(destination) => redirect(&destination),
            None => redirect(&format!("/households/{}/dashboard", household.slug)),
        },
        Err(error) => authentication_error(error),
    }
}

fn pending_household_authorization(session: &Session<SessionPgPool>) -> Option<String> {
    if session.get::<DisplayedConsent>("oauth_displayed").is_some() {
        return None;
    }
    session
        .get::<AuthorizationInput>("oauth_pending")
        .and_then(|input| serde_urlencoded::to_string(input.0).ok())
        .map(|query| format!("/authorize?{query}"))
}

fn forbidden() -> Response {
    (StatusCode::FORBIDDEN, [(header::CACHE_CONTROL, "no-store")]).into_response()
}

fn redirect(destination: &str) -> Response {
    (
        StatusCode::SEE_OTHER,
        [
            (header::LOCATION, destination),
            (header::CACHE_CONTROL, "no-store"),
        ],
    )
        .into_response()
}

fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        [(header::CACHE_CONTROL, "no-store")],
    )
        .into_response()
}

fn authentication_error(error: AuthenticationError) -> Response {
    let status = match error {
        AuthenticationError::Unauthenticated => StatusCode::UNAUTHORIZED,
        AuthenticationError::Forbidden | AuthenticationError::InsufficientScope { .. } => {
            StatusCode::FORBIDDEN
        }
        AuthenticationError::NotFound => StatusCode::NOT_FOUND,
        AuthenticationError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    };
    (status, [(header::CACHE_CONTROL, "no-store")]).into_response()
}
