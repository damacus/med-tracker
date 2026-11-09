use crate::controllers::shell::{ShellView as TeraView, ShellViewEngine as ViewEngine};
pub(crate) mod account_profile;
pub(crate) mod avatar;
pub(crate) mod dialogs;
mod export;
mod notifications;
mod push;

use crate::{
    controllers::medications::{
        authentication_error, begin, operation_error, request_id, unavailable,
    },
    models::{
        access::PersonAccess,
        care::{administration, report_pdf},
        entities::account,
        errors::OperationError,
        profile,
    },
};
use axum::{
    Extension,
    extract::Form,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use chrono::Datelike;
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};
use sea_orm::EntityTrait;
use serde_json::{Value, json};
use std::collections::HashMap;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/households")
        .add("/{slug}/profile", get(show).post(save))
        .add("/{slug}/profile/navigation", get(navigation))
        .add("/{slug}/profile/notifications", post(notifications::save))
        .add("/{slug}/profile/export", post(export::download))
        .add(
            "/{slug}/profile/push",
            get(push::status).post(push::register).delete(push::remove),
        )
        .add(
            "/{slug}/profile/push/status",
            post(push::subscription_status),
        )
        .add("/{slug}/profile/push/test", post(push::send_test))
}

struct Page<'a> {
    ctx: &'a AppContext,
    slug: &'a str,
    session: &'a Session<SessionPgPool>,
    request_id: String,
    headers: &'a HeaderMap,
    token: &'a CsrfToken,
    view: &'a TeraView,
}

async fn show(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    (headers, request): (HeaderMap, Option<Extension<LocoRequestId>>),
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    if view
        .profile_presentation()
        .is_some_and(|value| !value.complete)
    {
        return dialogs::open(dialogs::Open {
            ctx,
            session,
            headers,
            token,
            view,
            request,
        })
        .await;
    }
    Page {
        ctx: &ctx,
        slug: &slug,
        session: &session,
        request_id: request_id(request),
        headers: &headers,
        token: &token,
        view: &view,
    }
    .render(None)
    .await
}

async fn save(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    (headers, request): (HeaderMap, Option<Extension<LocoRequestId>>),
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(draft): Form<HashMap<String, String>>,
) -> Response {
    if token
        .verify(
            draft
                .get("authenticity_token")
                .map(String::as_str)
                .unwrap_or(""),
        )
        .is_err()
    {
        return operation_error(OperationError::Forbidden);
    }
    let page = Page {
        ctx: &ctx,
        slug: &slug,
        session: &session,
        request_id: request_id(request),
        headers: &headers,
        token: &token,
        view: &view,
    };
    let (principal, tenant) = match begin(page.ctx, page.session, page.slug, &page.request_id).await
    {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let result = async {
        let changes = parse_preferences(&draft)?;
        if changes
            .mobile_shortcuts
            .as_ref()
            .is_some_and(|values| values.iter().any(|value| value == "administration"))
            && !administration::can_manage(&tenant).await?
        {
            return Err(profile::validation("mobile_shortcuts", "is unavailable"));
        }
        profile::update_preferences(&tenant, principal.account_id(), changes).await
    }
    .await;
    match result {
        Ok(()) => {
            if tenant.commit().await.is_err() {
                return unavailable();
            }
            (
                StatusCode::SEE_OTHER,
                [
                    (
                        header::LOCATION,
                        format!("/households/{}/profile#profile", page.slug),
                    ),
                    (header::CACHE_CONTROL, "no-store".into()),
                ],
            )
                .into_response()
        }
        Err(OperationError::Validation { .. }) => {
            if tenant.rollback().await.is_err() {
                return unavailable();
            }
            page.render(Some(draft)).await
        }
        Err(error) => operation_error(error),
    }
}

fn parse_preferences(
    draft: &HashMap<String, String>,
) -> std::result::Result<profile::Changes, OperationError> {
    let setting = draft.get("setting").map(String::as_str).unwrap_or("");
    let allowed: &[&str] = match setting {
        "time_zone" => &["authenticity_token", "setting", "time_zone"],
        "gravatar" => &["authenticity_token", "setting", "gravatar_enabled"],
        "shortcuts" => &[
            "authenticity_token",
            "setting",
            "shortcut_1",
            "shortcut_2",
            "shortcut_3",
        ],
        _ => {
            return Err(profile::validation(
                "profile",
                "contains an unsupported field",
            ));
        }
    };
    if draft.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(profile::validation(
            "profile",
            "contains an unsupported field",
        ));
    }
    let mut changes = profile::Changes::default();
    if setting == "time_zone" {
        changes.time_zone = Some(
            draft
                .get("time_zone")
                .ok_or_else(|| profile::validation("time_zone", "is required"))?
                .clone(),
        );
    } else if setting == "gravatar" {
        changes.gravatar_enabled = Some(match draft.get("gravatar_enabled").map(String::as_str) {
            Some("true") => true,
            None | Some("false") => false,
            _ => return Err(profile::validation("gravatar_enabled", "is invalid")),
        });
    } else {
        changes.mobile_shortcuts = Some(
            (1..=3)
                .filter_map(|slot| {
                    draft
                        .get(&format!("shortcut_{slot}"))
                        .filter(|value| !value.is_empty())
                        .cloned()
                })
                .collect(),
        );
    }
    profile::validate(&changes)?;
    Ok(changes)
}

pub(super) fn shortcut_choices(slug: &str, labels: &Value, can_manage: bool) -> Vec<Value> {
    super::shell::choices(slug, labels, can_manage)
        .into_iter()
        .map(|link| json!(link))
        .collect()
}

pub(super) fn stored_shortcuts(account: &account::Model) -> Vec<String> {
    account.preferences["mobile_shortcuts"]
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_else(|| vec!["dashboard".into(), "inventory".into(), "finder".into()])
}

async fn navigation(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    (headers, request): (HeaderMap, Option<Extension<LocoRequestId>>),
    token: CsrfToken,
) -> Response {
    let shell = match super::shell::load(
        &ctx,
        &session,
        &slug,
        &headers,
        &token,
        &request_id(request),
        "",
    )
    .await
    {
        Ok(shell) => shell,
        Err(response) => return *response,
    };
    (
        StatusCode::OK,
        token,
        [(header::CACHE_CONTROL, "no-store")],
        axum::Json(json!({"label":shell.shortcut_label,"links":shell.links,"shell":shell})),
    )
        .into_response()
}

impl Page<'_> {
    async fn render(self, draft: Option<HashMap<String, String>>) -> Response {
        let Self {
            ctx,
            slug,
            session,
            request_id,
            headers,
            token,
            view,
        } = self;
        let (principal, tenant) = match begin(ctx, session, slug, &request_id).await {
            Ok(value) => value,
            Err(error) => return authentication_error(error),
        };
        let person =
            match profile::linked_person(&tenant, principal.account_id(), PersonAccess::View).await
            {
                Ok(value) => value,
                Err(error) => return operation_error(error),
            };
        let account = match account::Entity::find_by_id(principal.account_id())
            .one(tenant.transaction())
            .await
        {
            Ok(Some(value)) => value,
            Ok(None) => return operation_error(OperationError::Unauthenticated),
            Err(_) => return unavailable(),
        };
        let locale = report_pdf::locale(
            headers
                .get(header::ACCEPT_LANGUAGE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("en"),
        );
        let labels = match report_pdf::translations(locale) {
            Ok(value) => value,
            Err(_) => return unavailable(),
        };
        let Ok(authenticity_token) = token.authenticity_token() else {
            return unavailable();
        };
        let mut data = crate::controllers::medications::rendering::appearance_context();
        if let Some(palettes) = data["palettes"].as_array_mut() {
            for palette in palettes {
                let key = palette["id"].as_str().unwrap_or_default().replace('-', "_");
                palette["label"] = labels["profiles"]["theme_picker"]["themes"][&key].clone();
            }
        }
        data["title"] = labels["profiles"]["show"]["title"].clone();
        data["system_info"] = json!({"development":ctx.environment == loco_rs::environment::Environment::Development,"version":env!("CARGO_PKG_VERSION"),"worktree":env!("MEDTRACKER_BUILD_WORKTREE"),"commit":env!("MEDTRACKER_BUILD_COMMIT")});
        data["lang"] = json!(locale);
        data["labels"] = labels.clone();
        data["slug"] = json!(slug);
        data["name"] = json!(person.name);
        data["email"] = json!(account.email);
        data["initials"] = json!(
            person
                .name
                .split_whitespace()
                .filter_map(|word| word.chars().next())
                .take(2)
                .flat_map(char::to_uppercase)
                .collect::<String>()
        );
        data["date_of_birth"] = json!(person.date_of_birth.map(|date| {
            let month = labels["reports"]["medication_review"]["months"][date.month0() as usize]
                .as_str()
                .unwrap_or("");
            format!("{month} {}", date.format("%d, %Y"))
        }));
        data["age_present"] = json!(person.date_of_birth.is_some());
        data["age"] = json!(person.date_of_birth.map(|birthday| {
            crate::models::care::people::age(
                birthday,
                chrono::Utc::now()
                    .with_timezone(&crate::models::care::doses::app_zone(
                        ctx.config.settings.as_ref(),
                    ))
                    .date_naive(),
            )
        }));
        data["age_label"] = json!(
            labels["people"]["show"]["age"]
                .as_str()
                .unwrap_or("")
                .trim_end_matches(':')
        );
        data["person_type"] = json!(person.person_type);
        data["has_capacity"] = json!(person.has_capacity);
        data["avatar_attached"] =
            match profile::avatar::attachment(&tenant, principal.account_id(), PersonAccess::View)
                .await
            {
                Ok(value) => json!(value.is_some()),
                Err(error) => return operation_error(error),
            };
        data["can_edit_profile"] = match crate::models::access::require_person_access(
            &tenant,
            person.id,
            PersonAccess::Manage,
        )
        .await
        {
            Ok(()) => json!(true),
            Err(OperationError::Forbidden) => json!(false),
            Err(error) => return operation_error(error),
        };
        data["gravatar_enabled"] = json!(profile::gravatar::enabled(&account.preferences));
        data["selected_gravatar"] = json!(
            draft
                .as_ref()
                .filter(|draft| draft
                    .get("setting")
                    .is_some_and(|setting| setting == "gravatar"))
                .map_or(profile::gravatar::enabled(&account.preferences), |draft| {
                    draft
                        .get("gravatar_enabled")
                        .is_some_and(|value| value == "true")
                })
        );
        data["gravatar_url"] = match profile::gravatar::for_person(
            &tenant,
            person.id,
            data["avatar_attached"].as_bool().unwrap_or(false),
        )
        .await
        {
            Ok(value) => json!(value),
            Err(error) => return operation_error(error),
        };
        data["time_zone"] = json!(
            account.preferences["time_zone"]
                .as_str()
                .filter(|zone| !zone.is_empty())
                .unwrap_or("UTC")
        );
        let selected_zone = draft
            .as_ref()
            .and_then(|draft| draft.get("time_zone"))
            .map(String::as_str)
            .unwrap_or_else(|| data["time_zone"].as_str().unwrap_or("UTC"))
            .to_owned();
        let mut zones = profile::supported_zones();
        if !zones.iter().any(|(_, value)| value == &selected_zone) {
            zones.push((selected_zone.clone(), selected_zone.clone()));
        }
        data["zones"] = json!(
            zones
                .into_iter()
                .map(|(name, value)| json!({"name":name,"value":value}))
                .collect::<Vec<_>>()
        );
        data["selected_zone"] = json!(selected_zone);
        let setting = draft
            .as_ref()
            .and_then(|draft| draft.get("setting"))
            .map(String::as_str);
        data["timezone_error"] = json!(setting == Some("time_zone"));
        data["shortcuts_error"] = json!(setting == Some("shortcuts"));
        data["gravatar_error"] = json!(setting == Some("gravatar"));
        data["avatar_error"] = draft
            .as_ref()
            .and_then(|draft| draft.get("avatar_error"))
            .map_or(Value::Null, |key| labels["profiles"]["avatar"][key].clone());
        let can_manage = match administration::can_manage(&tenant).await {
            Ok(value) => value,
            Err(error) => return operation_error(error),
        };
        data["shortcut_choices"] = json!(shortcut_choices(slug, &labels, can_manage));
        data["sidebar_links"] = json!(
            shortcut_choices(slug, &labels, can_manage)
                .into_iter()
                .filter(|link| link["id"] != "profile")
                .map(|mut link| {
                    let id = link["id"].as_str().unwrap_or_default().to_owned();
                    link["label"] = labels["layouts"]["sidebar"][&id].clone();
                    link
                })
                .collect::<Vec<_>>()
        );
        data["role_label"] =
            labels["admin"]["labels"]["membership_roles"][&tenant.membership().role].clone();
        let stored = stored_shortcuts(&account);
        data["shortcut_slots"] = json!((1..=3).map(|slot| {
        let selected = if setting == Some("shortcuts") { draft.as_ref().and_then(|draft| draft.get(&format!("shortcut_{slot}"))).cloned().unwrap_or_default() } else { stored.get(slot - 1).cloned().unwrap_or_default() };
        json!({"number":slot,"label":labels["profiles"]["mobile_shortcuts"]["slot"].as_str().unwrap_or("").replace("%{number}", &slot.to_string()),"selected":selected})
    }).collect::<Vec<_>>());
        data["authenticity_token"] = json!(authenticity_token);
        data["notifications"] =
            match notifications::context(&tenant, principal.account_id(), &labels, draft.as_ref())
                .await
            {
                Ok(value) => value,
                Err(error) => return operation_error(error),
            };
        data["notifications_error"] = json!(setting == Some("notifications"));
        if tenant.commit().await.is_err() {
            return unavailable();
        }
        let Some(service) = ctx
            .shared_store
            .get::<crate::models::identity::better_auth::IdentityService>()
        else {
            return unavailable();
        };
        let security = match crate::models::identity::better_auth::security::overview(
            &service,
            &crate::controllers::identity_onboarding::identity_request(headers),
            request_id.clone(),
        )
        .await
        {
            Ok(value) if value.account_id.get() == principal.account_id() => value,
            Ok(_) => return operation_error(OperationError::Forbidden),
            Err(_) => return unavailable(),
        };
        data["account_access"] =
            match profile::account_access::load(&ctx.db, security.account_id).await {
                Ok(value) => json!(value),
                Err(error) => return operation_error(error),
            };
        data["security"] = json!(crate::views::profile_security::context(
            &security,
            principal.time_zone(),
            &labels,
            chrono::Utc::now()
        ));
        let (_, tenant) = match begin(ctx, session, slug, &request_id).await {
            Ok(value) => value,
            Err(error) => return authentication_error(error),
        };
        match profile::linked_person(&tenant, principal.account_id(), PersonAccess::View).await {
            Ok(current) if current.id == person.id => {}
            Ok(_) => return unavailable(),
            Err(error) => return operation_error(error),
        }
        if tenant.commit().await.is_err() {
            return unavailable();
        }
        match format::render().view(view, "profile/show.html", data) {
            Ok(response) => (
                if draft.is_some() {
                    StatusCode::UNPROCESSABLE_ENTITY
                } else {
                    StatusCode::OK
                },
                token.clone(),
                [(header::CACHE_CONTROL, "no-store")],
                response,
            )
                .into_response(),
            Err(_) => unavailable(),
        }
    }
}
