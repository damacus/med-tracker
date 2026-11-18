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
    models::{care::report_pdf, entities::account, errors::OperationError, profile},
};
use axum::{
    Extension,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};
use serde_json::{Value, json};
use std::collections::HashMap;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/profile/households")
        .add("/{slug}/navigation", get(navigation))
        .add("/{slug}/notifications", post(notifications::save))
        .add("/{slug}/export", post(export::download))
        .add(
            "/{slug}/push",
            get(push::status).post(push::register).delete(push::remove),
        )
        .add("/{slug}/push/status", post(push::subscription_status))
        .add("/{slug}/push/test", post(push::send_test))
}

fn profile_url(household: Option<&str>, tab: &str) -> String {
    match household {
        Some(slug) => {
            let query = url::form_urlencoded::Serializer::new(String::new())
                .append_pair("household", slug)
                .finish();
            format!("/profile?{query}#{tab}")
        }
        None => format!("/profile#{tab}"),
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
