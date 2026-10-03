use super::*;
use axum::extract::Query;
use medtracker_web::household::path_segment;
use medtracker_web::notifications::{
    render_notification_settings, NotificationDraft, NotificationPage,
};

pub(super) fn routes() -> Router<AppState> {
    Router::new().route(
        "/households/{slug}/settings/notifications",
        get(edit).post(update),
    )
}

fn flag(record: &Value, key: &str) -> Result<bool, PageError> {
    record
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| error(StatusCode::BAD_GATEWAY))
}

fn draft_from_record(value: &Value) -> Result<NotificationDraft, PageError> {
    let data = value
        .get("data")
        .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
    Ok(NotificationDraft {
        enabled: flag(data, "enabled")?,
        dose_due_enabled: flag(data, "dose_due_enabled")?,
        missed_dose_enabled: flag(data, "missed_dose_enabled")?,
        low_stock_enabled: flag(data, "low_stock_enabled")?,
        private_text_enabled: flag(data, "private_text_enabled")?,
    })
}

fn draft_from_fields(fields: &HashMap<String, String>) -> Result<NotificationDraft, PageError> {
    let flag = |key: &str| match fields.get(key).map(String::as_str) {
        None => Ok(false),
        Some("true") => Ok(true),
        Some(_) => Err(error(StatusCode::BAD_REQUEST)),
    };
    Ok(NotificationDraft {
        enabled: flag("enabled")?,
        dose_due_enabled: flag("dose_due_enabled")?,
        missed_dose_enabled: flag("missed_dose_enabled")?,
        low_stock_enabled: flag("low_stock_enabled")?,
        private_text_enabled: flag("private_text_enabled")?,
    })
}

struct RenderState {
    draft: Option<NotificationDraft>,
    editable: bool,
    saved: bool,
    failed: bool,
    status: StatusCode,
}

fn render(mut api: WebApi, household_name: String, slug: &str, state: RenderState) -> Response {
    let cookie = api.cookie.take();
    match render_notification_settings(NotificationPage {
        household_name,
        slug: slug.to_owned(),
        csrf: api.csrf,
        locale: api.locale,
        preferences: state.draft,
        editable: state.editable,
        saved: state.saved,
        error: state.failed,
    }) {
        Ok(body) => page_status(body, cookie, state.status),
        Err(_) => failure(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn edit(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let mut api = match WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let result = async {
        let (household_id, name) = api.household(&slug).await?;
        let capabilities = api.capabilities(household_id).await?;
        let editable = capabilities
            .pointer("/data/notifications/manage")
            .and_then(Value::as_bool)
            .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
        let reply = api
            .call(
                Method::GET,
                &format!("/api/v1/households/{household_id}/notification_preference"),
                None,
                None,
            )
            .await?;
        match reply.status {
            StatusCode::UNAUTHORIZED => Err(PageError::Login),
            StatusCode::NOT_FOUND => {
                let first_use = reply.value.pointer("/error/code").and_then(Value::as_str)
                    == Some("not_configured");
                Ok(render(
                    api,
                    name,
                    &slug,
                    RenderState {
                        draft: (first_use && editable).then(NotificationDraft::default),
                        editable,
                        saved: false,
                        failed: false,
                        status: StatusCode::OK,
                    },
                ))
            }
            status if status.is_success() => {
                let saved = query.get("saved").is_some_and(|value| value == "1");
                let draft = draft_from_record(&reply.value)?;
                Ok(render(
                    api,
                    name,
                    &slug,
                    RenderState {
                        draft: Some(draft),
                        editable,
                        saved,
                        failed: false,
                        status: StatusCode::OK,
                    },
                ))
            }
            status => Err(error(status)),
        }
    }
    .await;
    match result {
        Ok(response) => response,
        Err(error) => error.response(),
    }
}

async fn update(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    if !oauth::trusted_cookie_origin(&state, &headers) {
        return failure(StatusCode::FORBIDDEN);
    }
    let mut api = match WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    if fields.get("authenticity_token") != Some(&api.csrf) {
        return failure(StatusCode::FORBIDDEN);
    }
    let draft = match draft_from_fields(&fields) {
        Ok(draft) => draft,
        Err(error) => return error.response(),
    };
    let result = async {
        let (household_id, name) = api.household(&slug).await?;
        let body = json!({"notification_preference": {
            "enabled": draft.enabled,
            "dose_due_enabled": draft.dose_due_enabled,
            "missed_dose_enabled": draft.missed_dose_enabled,
            "low_stock_enabled": draft.low_stock_enabled,
            "private_text_enabled": draft.private_text_enabled,
        }});
        let csrf = api.csrf.clone();
        let reply = api
            .call(
                Method::PATCH,
                &format!("/api/v1/households/{household_id}/notification_preference"),
                Some(body),
                Some(&csrf),
            )
            .await?;
        if reply.status.is_success() {
            return Ok(redirect(
                format!(
                    "/households/{}/settings/notifications?saved=1",
                    path_segment(&slug)
                ),
                api.cookie.take(),
            ));
        }
        if reply.status == StatusCode::UNAUTHORIZED {
            return Err(PageError::Login);
        }
        Ok(render(
            api,
            name,
            &slug,
            RenderState {
                draft: Some(draft),
                editable: reply.status != StatusCode::FORBIDDEN,
                saved: false,
                failed: true,
                status: reply.status,
            },
        ))
    }
    .await;
    match result {
        Ok(response) => response,
        Err(error) => error.response(),
    }
}
