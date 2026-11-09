use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, header, request::Parts},
    response::Response,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::{controller::views::ViewRenderer, prelude::*};
use sea_orm::EntityTrait;
use serde::Serialize;
use serde_json::{Value, json};

use super::medications::{authentication_error, begin, operation_error, unavailable};
use crate::models::{
    access::{PersonAccess, TenantTransaction},
    care::{administration, report_pdf},
    entities::account,
    errors::OperationError,
    identity::{
        browser::{BrowserHousehold, BrowserPrincipal},
        resource::AuthenticationError,
    },
    platform, profile,
};

#[derive(Clone, Serialize)]
pub(super) struct NavigationLink {
    pub id: &'static str,
    pub label: String,
    pub href: String,
}

#[derive(Serialize)]
pub(super) struct ShellContext {
    pub name: String,
    pub initials: String,
    pub avatar_attached: bool,
    pub gravatar_url: Option<String>,
    pub authenticity_token: String,
    pub sidebar: Vec<NavigationLink>,
    pub choices: Vec<NavigationLink>,
    pub households: Vec<BrowserHousehold>,
    pub active_household_id: Option<String>,
    pub platform_url: Option<&'static str>,
    pub labels: Value,
    pub navigation_label: String,
    pub slug: String,
    pub selected_section: Option<&'static str>,
    pub sidebar_collapsed: bool,
    pub links: Vec<NavigationLink>,
    pub shortcut_label: String,
    #[serde(skip)]
    translations: Value,
    #[serde(skip)]
    locale: String,
}

pub(super) fn choices(slug: &str, labels: &Value, can_manage: bool) -> Vec<NavigationLink> {
    [
        ("dashboard", "dashboard"),
        ("inventory", "medications"),
        ("locations", "locations"),
        ("people", "people"),
        ("finder", "medications/finder"),
        ("medicine_reviews", "medicine-reviews"),
        ("reports", "reports"),
        ("profile", "profile"),
        ("administration", "admin"),
    ]
    .into_iter()
    .filter(|(id, _)| *id != "administration" || can_manage)
    .map(|(id, path)| {
        let label = match id {
            "profile" => &labels["layouts"]["profile_menu"]["profile"],
            "dashboard" => &labels["layouts"]["mobile_rail"]["home"],
            "finder" => &labels["layouts"]["mobile_rail"]["finder"],
            _ => &labels["layouts"]["sidebar"][id],
        };
        NavigationLink {
            id,
            label: label.as_str().unwrap_or_default().to_owned(),
            href: format!("/households/{slug}/{path}"),
        }
    })
    .collect()
}

pub(super) async fn load(
    ctx: &AppContext,
    session: &Session<SessionPgPool>,
    slug: &str,
    headers: &HeaderMap,
    token: &CsrfToken,
    request_id: &str,
    path: &str,
) -> std::result::Result<ShellContext, Box<Response>> {
    let (principal, tenant) = begin(ctx, session, slug, request_id)
        .await
        .map_err(authentication_error)?;
    load_authorized(ctx, (principal, tenant), slug, headers, token, path).await
}

async fn load_authorized(
    ctx: &AppContext,
    authority: (BrowserPrincipal, TenantTransaction),
    slug: &str,
    headers: &HeaderMap,
    token: &CsrfToken,
    path: &str,
) -> std::result::Result<ShellContext, Box<Response>> {
    let (principal, tenant) = authority;
    let account = account::Entity::find_by_id(principal.account_id())
        .one(tenant.transaction())
        .await
        .map_err(|error| operation_error(error.into()))?
        .ok_or_else(|| operation_error(OperationError::Unauthenticated))?;
    let can_manage = administration::can_manage(&tenant)
        .await
        .map_err(operation_error)?;
    let locale = report_pdf::locale(
        headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("en"),
    );
    let labels = report_pdf::translations(locale).map_err(|_| unavailable())?;
    let choices = choices(slug, &labels, can_manage);
    let links = super::profile::stored_shortcuts(&account)
        .into_iter()
        .filter_map(|id| choices.iter().find(|choice| choice.id == id).cloned())
        .collect();
    let person =
        match profile::linked_person(&tenant, principal.account_id(), PersonAccess::View).await {
            Ok(person) => Some(person),
            Err(OperationError::Forbidden | OperationError::NotFound) => None,
            Err(error) => return Err(Box::new(operation_error(error))),
        };
    let name = person
        .as_ref()
        .map_or(account.email.as_str(), |person| person.name.as_str())
        .to_owned();
    let initials = name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .flat_map(char::to_uppercase)
        .collect();
    let avatar_attached = if person.is_some() {
        profile::avatar::attachment(&tenant, principal.account_id(), PersonAccess::View)
            .await
            .map_err(operation_error)?
            .is_some()
    } else {
        false
    };
    let gravatar_url = if let Some(person) = &person {
        profile::gravatar::for_person(&tenant, person.id, avatar_attached)
            .await
            .map_err(operation_error)?
    } else {
        None
    };
    let authenticity_token = token.authenticity_token().map_err(|_| unavailable())?;
    let sidebar = choices
        .iter()
        .filter(|link| link.id != "profile")
        .cloned()
        .map(|mut link| {
            if link.id != "profile" {
                link.label = labels["layouts"]["sidebar"][link.id]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned();
            }
            link
        })
        .collect();
    tenant.commit().await.map_err(operation_error)?;
    let households = principal
        .households(&ctx.db)
        .await
        .map_err(authentication_error)?;
    if !households.iter().any(|household| household.slug == slug) {
        return Err(Box::new(operation_error(OperationError::Forbidden)));
    }
    let active_household_id = principal
        .active_household(&ctx.db)
        .await
        .map_err(authentication_error)?
        .map(|household| household.id.to_string());
    let transaction = platform::begin(&ctx.db).await.map_err(operation_error)?;
    let platform_url = match platform::administrator(&transaction, principal.account_id()).await {
        Ok(_) => {
            transaction
                .commit()
                .await
                .map_err(|error| operation_error(error.into()))?;
            Some("/platform/users")
        }
        Err(OperationError::Forbidden | OperationError::Unauthenticated) => {
            transaction
                .rollback()
                .await
                .map_err(|error| operation_error(error.into()))?;
            None
        }
        Err(error) => return Err(Box::new(operation_error(error))),
    };
    let selected_section = choices
        .iter()
        .filter(|link| path == link.href || path.starts_with(&format!("{}/", link.href)))
        .max_by_key(|link| link.href.len())
        .map(|link| link.id)
        .or_else(|| {
            ["add-medication", "dose-occurrences"]
                .iter()
                .any(|section| {
                    let prefix = format!("/households/{slug}/{section}");
                    path == prefix || path.starts_with(&format!("{prefix}/"))
                })
                .then_some("inventory")
        });
    let sidebar_collapsed = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|cookies| {
            cookies
                .split(';')
                .any(|cookie| cookie.trim() == "medtracker.sidebar=collapsed")
        });
    Ok(ShellContext {
        name,
        initials,
        avatar_attached,
        gravatar_url,
        authenticity_token,
        sidebar,
        choices,
        households,
        active_household_id,
        platform_url,
        navigation_label: labels["ruby_ui"]["common"]["navigation_menu"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        shortcut_label: labels["profiles"]["mobile_shortcuts"]["title"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        labels: labels["layouts"].clone(),
        translations: labels,
        locale: locale.to_owned(),
        slug: slug.to_owned(),
        selected_section,
        sidebar_collapsed,
        links,
    })
}

fn optional_shell_error(
    error: AuthenticationError,
) -> std::result::Result<Option<ShellContext>, Box<Response>> {
    match error {
        AuthenticationError::Unauthenticated
        | AuthenticationError::Forbidden
        | AuthenticationError::NotFound
        | AuthenticationError::InsufficientScope { .. } => Ok(None),
        AuthenticationError::Unavailable => Err(Box::new(authentication_error(error))),
    }
}

pub(crate) struct ShellView {
    engine: TeraView,
    shell: Option<ShellContext>,
    presentation: Option<super::profile::dialogs::Presentation>,
    flow: Option<super::profile::dialogs::Frame>,
}

pub(crate) struct ShellViewEngine<E>(pub E);

impl ShellView {
    pub(crate) fn document_engine(&self) -> TeraView {
        self.engine.clone()
    }
    pub(crate) fn profile_presentation(&self) -> Option<&super::profile::dialogs::Presentation> {
        self.presentation.as_ref()
    }
    pub(crate) fn profile_destination(&self, path: &str) -> String {
        self.profile_presentation().map_or_else(
            || path.to_owned(),
            |presentation| presentation.destination(path),
        )
    }
    pub(crate) fn profile_redirect(&self, path: &str) -> Response {
        use axum::response::IntoResponse;
        (
            axum::http::StatusCode::SEE_OTHER,
            [
                (header::LOCATION, self.profile_destination(path)),
                (header::CACHE_CONTROL, "no-store".into()),
            ],
        )
            .into_response()
    }
    pub(crate) fn profile_error(
        &self,
        status: axum::http::StatusCode,
        message: &str,
    ) -> Option<Response> {
        use axum::response::IntoResponse;
        self.flow.as_ref().map(|flow| {
            match flow.render(
                &self.engine,
                "profile/flow-error.html",
                json!({"title":"Profile","error":message}),
            ) {
                Ok(html) => (
                    status,
                    [
                        (header::CACHE_CONTROL, "no-store"),
                        (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                    ],
                    html,
                )
                    .into_response(),
                Err(_) => unavailable(),
            }
        })
    }
    pub(crate) fn profile_rejection(&self, status: axum::http::StatusCode) -> Response {
        use axum::response::IntoResponse;
        self.flow
            .as_ref()
            .and_then(|flow| self.profile_error(status, flow.rejected()))
            .unwrap_or_else(|| status.into_response())
    }
}

impl std::ops::Deref for ShellView {
    type Target = TeraView;

    fn deref(&self) -> &Self::Target {
        &self.engine
    }
}

impl FromRequestParts<AppContext> for ShellViewEngine<ShellView> {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppContext,
    ) -> std::result::Result<Self, Self::Rejection> {
        let ViewEngine(engine) = ViewEngine::<TeraView>::from_request_parts(parts, state)
            .await
            .map_err(|_| unavailable())?;
        let presentation = super::profile::dialogs::Presentation::from_request(parts)
            .map_err(|response| *response)?;
        let mut segments = parts.uri.path().split('/');
        let slug = match (
            segments.next(),
            segments.next(),
            segments.next(),
            segments.next(),
        ) {
            (Some(""), Some("households"), Some(slug), Some(_)) => Some(slug.to_owned()),
            _ => None,
        }
        .or_else(|| {
            presentation
                .as_ref()
                .and_then(|value| value.household.clone())
        });
        let shell = if let Some(slug) = slug {
            let session = Session::<SessionPgPool>::from_request_parts(parts, state)
                .await
                .map_err(|_| unavailable())?;
            let token = CsrfToken::from_request_parts(parts, state)
                .await
                .map_err(|_| unavailable())?;
            let request_id = parts
                .extensions
                .get::<loco_rs::controller::middleware::request_id::LocoRequestId>()
                .map(|id| id.get().to_owned())
                .unwrap_or_default();
            match begin(state, &session, &slug, &request_id).await {
                Ok(authority) => Some(
                    load_authorized(
                        state,
                        authority,
                        &slug,
                        &parts.headers,
                        &token,
                        parts.uri.path(),
                    )
                    .await
                    .map_err(|response| *response)?,
                ),
                Err(error) => optional_shell_error(error).map_err(|response| *response)?,
            }
        } else {
            None
        };
        let flow = if let Some(presentation) = presentation.clone() {
            let session = Session::<SessionPgPool>::from_request_parts(parts, state)
                .await
                .map_err(|_| unavailable())?;
            super::profile::dialogs::Frame::load(state, &session, &parts.headers, presentation)
                .await
                .map_err(|response| *response)?
        } else {
            None
        };
        Ok(Self(ShellView {
            engine,
            shell,
            presentation,
            flow,
        }))
    }
}

impl ViewRenderer for ShellView {
    fn render<S: Serialize>(&self, key: &str, data: S) -> Result<String> {
        let mut data = serde_json::to_value(data)?;
        if let Some(slug) = self
            .presentation
            .as_ref()
            .and_then(|presentation| presentation.household.as_ref())
        {
            data["slug"] = json!(slug);
        }
        if let Some(shell) = &self.shell
            && data.get("slug").and_then(Value::as_str) == Some(shell.slug.as_str())
        {
            if data.get("labels").is_none() {
                data["labels"] = shell.translations.clone();
            }
            if data.get("lang").is_none() {
                data["lang"] = json!(shell.locale);
            }
            let labels = &data["labels"];
            let mut choices = choices(
                &shell.slug,
                labels,
                shell.choices.iter().any(|link| link.id == "administration"),
            );
            let language_suffix = data
                .get("language_suffix")
                .and_then(Value::as_str)
                .unwrap_or_default();
            for link in &mut choices {
                if link.id == "administration" {
                    link.href.push_str(language_suffix);
                }
            }
            let sidebar = choices
                .iter()
                .filter(|link| link.id != "profile")
                .cloned()
                .map(|mut link| {
                    link.label = labels["layouts"]["sidebar"][link.id]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned();
                    link
                })
                .collect::<Vec<_>>();
            let links = shell
                .links
                .iter()
                .filter_map(|link| choices.iter().find(|choice| choice.id == link.id).cloned())
                .collect::<Vec<_>>();
            let mut presentation = json!(shell);
            presentation["labels"] = labels["layouts"].clone();
            presentation["sidebar"] = json!(sidebar);
            presentation["choices"] = json!(choices);
            presentation["links"] = json!(links);
            presentation["shortcut_label"] =
                labels["profiles"]["mobile_shortcuts"]["title"].clone();
            data["shell"] = presentation;
        }
        if let Some(flow) = &self.flow {
            flow.render(&self.engine, key, data).map_err(|error| *error)
        } else {
            self.engine.render(key, data)
        }
    }
}
