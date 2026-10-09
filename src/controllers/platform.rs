use super::medications::{operation_error, unavailable};
use crate::models::{
    errors::OperationError,
    identity::better_auth::{
        BrowserIdentity, IdentityService, browser_identity, clinical_id, dispatch,
    },
    platform,
};
use axum::{
    Extension,
    extract::{Form, Query},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect},
};
use axum_csrf::CsrfToken;
use better_auth_core::{AuthRequest, HttpMethod};
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};
use serde::Deserialize;
use serde_json::{Value, json};

mod recovery;
mod support;

pub fn routes() -> Routes {
    Routes::new()
        .add("/platform", get(index))
        .add("/platform/users", get(users))
        .add("/platform/users/administrator", post(administrator_form))
        .add("/platform/users/active", post(active_form))
        .add("/platform/settings", get(settings).post(update_settings))
        .add(
            "/platform/owner-recovery",
            get(recovery::show).post(recovery::submit),
        )
        .add(
            "/platform/support",
            get(support::index).post(support::request),
        )
        .add("/platform/support/activate", post(support::activate))
        .add("/platform/support/end", post(support::end))
        .add("/platform/support/{id}", get(support::read))
}

async fn index() -> Response {
    Redirect::to("/platform/users").into_response()
}

#[derive(Deserialize)]
pub struct UsersQuery {
    q: Option<String>,
    page: Option<i64>,
}

pub(super) fn session_request(headers: &HeaderMap) -> AuthRequest {
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

fn render(view: &TeraView, token: &CsrfToken, page: &platform::UsersPage) -> Response {
    let Ok(authenticity) = token.authenticity_token() else {
        return unavailable();
    };
    let mut data = super::medications::rendering::appearance_context();
    data["title"] = json!("Platform users");
    data["users"] = json!(
        page.users
            .iter()
            .map(|entry| json!({
                "id": entry.id,
                "email": entry.email,
                "active": entry.status == 2,
                "user_active": entry.user_active,
                "administrator": entry.administrator.as_deref() == Some("active"),
            }))
            .collect::<Vec<Value>>()
    );
    data["query"] = json!(page.query);
    data["query_encoded"] =
        json!(url::form_urlencoded::byte_serialize(page.query.as_bytes()).collect::<String>());
    data["page"] = json!(page.page);
    data["pages"] = json!(page.pages);
    data["page_links"] = json!(page.page_links);
    data["total"] = json!(page.total);
    data["authenticity_token"] = json!(authenticity);
    match format::render().view(view, "platform/users.html", data) {
        Ok(response) => (
            StatusCode::OK,
            token.clone(),
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}

async fn users(
    State(ctx): State<AppContext>,
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    Query(params): Query<UsersQuery>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let identity = match browser_identity(&service, &session_request(&headers)).await {
        Ok(identity) => identity,
        Err(_) => return unavailable(),
    };
    let BrowserIdentity::Authenticated { user, .. } = identity else {
        return Redirect::to("/login").into_response();
    };
    let Ok(account_id) = clinical_id(&user.id) else {
        return unavailable();
    };
    let Ok(transaction) = platform::begin(&ctx.db).await else {
        return unavailable();
    };
    let result = async {
        platform::administrator(&transaction, account_id).await?;
        platform::users(
            &transaction,
            params.q.as_deref().unwrap_or_default(),
            params.page.unwrap_or(1),
        )
        .await
    }
    .await;
    let page = match result {
        Ok(page) => page,
        Err(error) => {
            let unauthenticated = matches!(error, OperationError::Unauthenticated);
            if transaction.rollback().await.is_err() {
                return unavailable();
            }
            return if unauthenticated {
                Redirect::to("/login").into_response()
            } else {
                operation_error(error)
            };
        }
    };
    let response = render(&view, &token, &page);
    if transaction.commit().await.is_err() {
        return unavailable();
    }
    response
}

pub(super) fn browser_meta(
    session: &better_auth_core::wire::SessionView,
    request_id: Option<Extension<LocoRequestId>>,
    headers: &HeaderMap,
) -> platform::RequestMeta {
    platform::RequestMeta {
        session_reference: Some(crate::models::identity::better_auth::session_digest(
            &session.token,
        )),
        request_id: Some(super::medications::request_id(request_id)),
        ip: headers
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
    }
}

pub(super) async fn proof(
    service: &IdentityService,
    headers: &HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    view: &TeraView,
    authenticity_token: &str,
    operation: (Value, &str),
) -> Response {
    let (action, label) = operation;
    if token.verify(authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let session = session_request(headers);
    if !matches!(
        browser_identity(service, &session).await,
        Ok(BrowserIdentity::Authenticated { .. })
    ) {
        return Redirect::to("/login").into_response();
    }
    let mut request = session;
    request.method = HttpMethod::Post;
    request.path = "/security/operation/start".into();
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    request.body = serde_json::to_vec(&action).ok();
    match dispatch(service, request, super::medications::request_id(request_id)).await {
        Ok(response) if response.status == 200 => {
            let Ok(body) = serde_json::from_slice::<Value>(&response.body) else {
                return unavailable();
            };
            let Some(id) = body["operation_id"].as_str() else {
                return unavailable();
            };
            super::identity_onboarding::render_named(
                view,
                token,
                Some(id),
                None,
                StatusCode::OK,
                (
                    body["password_proof"].as_bool() == Some(true),
                    body["passkey_proof"].as_bool() == Some(true),
                    body["oidc_proof"].as_bool() == Some(true),
                ),
                label,
            )
        }
        Ok(response) => (
            if response.status == 403 {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::BAD_REQUEST
            },
            "This platform change is unavailable.",
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}

#[derive(Deserialize)]
pub struct AdministratorForm {
    account_id: i64,
    grant: bool,
    authenticity_token: String,
}

async fn administrator_form(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(form): Form<AdministratorForm>,
) -> Response {
    proof(
        &service,
        &headers,
        request_id,
        token,
        &view,
        &form.authenticity_token,
        (json!({"action":"platform_administrator","account_id":form.account_id,"grant":form.grant}), "platform administrator change"),
    )
    .await
}

#[derive(Deserialize)]
pub struct ActiveForm {
    account_id: i64,
    active: bool,
    authenticity_token: String,
}

async fn active_form(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(form): Form<ActiveForm>,
) -> Response {
    proof(
        &service,
        &headers,
        request_id,
        token,
        &view,
        &form.authenticity_token,
        (
            json!({"action":"platform_user","account_id":form.account_id,"active":form.active}),
            "platform user change",
        ),
    )
    .await
}

fn render_settings(view: &TeraView, token: &CsrfToken, settings: &platform::Settings) -> Response {
    let Ok(authenticity) = token.authenticity_token() else {
        return unavailable();
    };
    let mut data = super::medications::rendering::appearance_context();
    data["title"] = json!("Platform settings");
    data["invite_only"] = json!(settings.invite_only);
    data["invite_only_locked"] = json!(settings.invite_only_locked);
    data["medicine_lookup_base_url"] = json!(settings.medicine_lookup_base_url);
    data["medicine_lookup_token_url"] = json!(settings.medicine_lookup_token_url);
    data["medicine_lookup_source_priority"] =
        json!(settings.medicine_lookup_source_priority.join(","));
    data["lookup_sources"] = json!(platform::LOOKUP_SOURCES.join(", "));
    data["authenticity_token"] = json!(authenticity);
    match format::render().view(view, "platform/settings.html", data) {
        Ok(response) => (
            StatusCode::OK,
            token.clone(),
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}

async fn settings(
    State(ctx): State<AppContext>,
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let identity = match browser_identity(&service, &session_request(&headers)).await {
        Ok(identity) => identity,
        Err(_) => return unavailable(),
    };
    let BrowserIdentity::Authenticated { user, .. } = identity else {
        return Redirect::to("/login").into_response();
    };
    let Ok(account_id) = clinical_id(&user.id) else {
        return unavailable();
    };
    let Ok(transaction) = platform::begin(&ctx.db).await else {
        return unavailable();
    };
    let result = platform::settings(&transaction, account_id).await;
    let settings = match result {
        Ok(settings) => settings,
        Err(error) => {
            let unauthenticated = matches!(error, OperationError::Unauthenticated);
            if transaction.rollback().await.is_err() {
                return unavailable();
            }
            return if unauthenticated {
                Redirect::to("/login").into_response()
            } else {
                operation_error(error)
            };
        }
    };
    let response = render_settings(&view, &token, &settings);
    if transaction.commit().await.is_err() {
        return unavailable();
    }
    response
}

#[derive(Deserialize)]
pub struct SettingsForm {
    invite_only: bool,
    medicine_lookup_base_url: String,
    medicine_lookup_token_url: String,
    medicine_lookup_source_priority: String,
    authenticity_token: String,
}

async fn update_settings(
    Extension(service): Extension<IdentityService>,
    headers: HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(form): Form<SettingsForm>,
) -> Response {
    let sources = form
        .medicine_lookup_source_priority
        .split(',')
        .map(str::trim)
        .filter(|source| !source.is_empty())
        .map(str::to_owned)
        .collect::<Vec<String>>();
    proof(
        &service,
        &headers,
        request_id,
        token,
        &view,
        &form.authenticity_token,
        (
            json!({
                "action": "platform_settings",
                "invite_only": form.invite_only,
                "medicine_lookup_base_url": form.medicine_lookup_base_url,
                "medicine_lookup_token_url": form.medicine_lookup_token_url,
                "medicine_lookup_source_priority": sources,
            }),
            "platform settings change",
        ),
    )
    .await
}

#[derive(Deserialize)]
pub(super) struct SupportEndForm {
    support_id: i64,
    authenticity_token: String,
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn end_support(
    ctx: &AppContext,
    service: &IdentityService,
    headers: &HeaderMap,
    request_id: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    support_id: i64,
    authenticity_token: &str,
    redirect: &str,
) -> Response {
    if token.verify(authenticity_token).is_err() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let identity = match browser_identity(service, &session_request(headers)).await {
        Ok(identity) => identity,
        Err(_) => return unavailable(),
    };
    let BrowserIdentity::Authenticated { user, session } = identity else {
        return Redirect::to("/login").into_response();
    };
    let Ok(account_id) = clinical_id(&user.id) else {
        return unavailable();
    };
    let Ok(transaction) = platform::begin(&ctx.db).await else {
        return unavailable();
    };
    let meta = browser_meta(&session, request_id, headers);
    match platform::support::end(&transaction, account_id, support_id, &meta).await {
        Ok(_) => {
            if transaction.commit().await.is_err() {
                return unavailable();
            }
            Redirect::to(redirect).into_response()
        }
        Err(error) => {
            let unauthenticated = matches!(error, OperationError::Unauthenticated);
            let persisted = matches!(error, OperationError::Forbidden);
            let ended = if persisted {
                transaction.commit().await
            } else {
                transaction.rollback().await
            };
            if ended.is_err() {
                return unavailable();
            }
            if unauthenticated {
                Redirect::to("/login").into_response()
            } else {
                operation_error(error)
            }
        }
    }
}
