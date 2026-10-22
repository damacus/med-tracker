use crate::models::identity::{browser, resource::AuthenticationError};
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
    token: std::result::Result<CsrfToken, <CsrfToken as FromRequestParts<AppContext>>::Rejection>,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let mut auth_notice = None;
    if let Ok(session) = session {
        match browser::authenticate(&ctx.db, &session, ctx.config.settings.as_ref()).await {
            Ok(principal) => match principal.active_household(&ctx.db).await {
                Ok(Some(household)) => {
                    return redirect(&format!("/households/{}/dashboard", household.slug));
                }
                Ok(None) => return redirect("/households/choose"),
                Err(error) => return authentication_error(error),
            },
            Err(AuthenticationError::Unauthenticated) => {
                auth_notice = session.get::<String>("auth_notice");
                session.remove("auth_notice");
            }
            Err(error) => return authentication_error(error),
        }
    }
    render_home(view, false, Vec::new(), auth_notice, token.ok())
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
    render_home(view, true, households, auth_notice, Some(token))
}

fn render_home(
    view: TeraView,
    signed_in: bool,
    households: Vec<browser::BrowserHousehold>,
    auth_notice: Option<String>,
    token: Option<CsrfToken>,
) -> Response {
    let authenticity_token = if signed_in {
        match token
            .as_ref()
            .and_then(|token| token.authenticity_token().ok())
        {
            Some(value) => value,
            None => return unavailable(),
        }
    } else {
        String::new()
    };
    match format::render().view(
        &view,
        "home/index.html",
        data!({
            "title": if signed_in { "Choose a household" } else { "Welcome" },
            "allow_palette": signed_in,
            "signed_in": signed_in,
            "auth_notice": auth_notice,
            "households": households,
            "authenticity_token": authenticity_token,
            "appearances": [
                {"id": "light", "label": "Light"},
                {"id": "dark", "label": "Dark"},
                {"id": "system", "label": "System"}
            ],
            "palettes": [
                {"id": "default", "label": "Command Centre"},
                {"id": "serene-sage", "label": "Serene Sage"},
                {"id": "modern-clinical", "label": "Modern Clinical"},
                {"id": "warm-earth", "label": "Warm Earth"},
                {"id": "deep-lavender", "label": "Deep Lavender"},
                {"id": "forest-care", "label": "Forest Care"},
                {"id": "sunset-support", "label": "Sunset Support"},
                {"id": "tech-indigo", "label": "Tech Indigo"},
                {"id": "soft-rose", "label": "Soft Rose"},
                {"id": "minty-fresh", "label": "Minty Fresh"}
            ]
        }),
    ) {
        Ok(response) => match token {
            Some(token) => (token, [(header::CACHE_CONTROL, "no-store")], response).into_response(),
            None => ([(header::CACHE_CONTROL, "no-store")], response).into_response(),
        },
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
        let mut response = render_home(
            view,
            true,
            households,
            Some("Enter a household name of up to 100 characters.".to_owned()),
            Some(token),
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
        Ok(household) => redirect(&format!("/households/{}/dashboard", household.slug)),
        Err(error) => authentication_error(error),
    }
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
