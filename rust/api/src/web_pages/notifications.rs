use super::api_client::cookie_value;
use super::*;
use axum::extract::RawForm;
use medtracker_web::household::path_segment;
use medtracker_web::notifications::{
    render_notification_profile, render_notification_settings, ManagedPerson, NotificationDraft,
    NotificationPage, ProfileNotificationPage,
};
use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionTrait};

const SAVED_COOKIE: &str = "medtracker_notification_saved";

fn saved_cookie(slug: &str) -> Option<HeaderValue> {
    HeaderValue::from_str(&format!(
        "{SAVED_COOKIE}=1; Path=/households/{}/settings/notifications; HttpOnly; SameSite=Lax",
        path_segment(slug)
    ))
    .ok()
}

fn clear_saved_cookie(slug: &str) -> Option<HeaderValue> {
    HeaderValue::from_str(&format!(
        "{SAVED_COOKIE}=; Path=/households/{}/settings/notifications; Max-Age=0; HttpOnly; SameSite=Lax",
        path_segment(slug)
    ))
    .ok()
}

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/households/{slug}/profile/notifications",
            post(update_profile),
        )
        .route(
            "/households/{slug}/settings/notifications",
            get(edit).post(update),
        )
}

async fn managed_people(
    state: &AppState,
    account_id: i64,
    household_id: i64,
) -> Result<Vec<ManagedPerson>, PageError> {
    let membership_id =
        crate::profile_advanced_state::membership_id(state, account_id, household_id)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
            .ok_or_else(|| error(StatusCode::FORBIDDEN))?;
    let db = state
        .db
        .begin()
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    crate::restricted_role(&db)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    crate::tenant_setting(&db, "med_tracker.current_account_id", account_id)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    crate::tenant_setting(&db, "med_tracker.current_household_id", household_id)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    let rows = db.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT p.id, p.name, p.person_type, g.missed_dose_notifications_enabled FROM person_access_grants g JOIN people p ON p.id = g.person_id JOIN household_memberships m ON m.id = g.household_membership_id WHERE g.household_id = $1 AND g.household_membership_id = $2 AND g.access_level = 'manage' AND g.revoked_at IS NULL AND (g.expires_at IS NULL OR g.expires_at > now()) AND (m.person_id IS NULL OR g.person_id <> m.person_id) ORDER BY lower(p.name), p.id",
        [household_id.into(), membership_id.into()])).await.map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    let people = rows
        .into_iter()
        .map(|row| {
            let person_type: i32 = row
                .try_get("", "person_type")
                .map_err(|_| error(StatusCode::BAD_GATEWAY))?;
            Ok(ManagedPerson {
                id: row
                    .try_get("", "id")
                    .map_err(|_| error(StatusCode::BAD_GATEWAY))?,
                name: row
                    .try_get("", "name")
                    .map_err(|_| error(StatusCode::BAD_GATEWAY))?,
                automatic: matches!(person_type, 1 | 2),
                selected: row
                    .try_get("", "missed_dose_notifications_enabled")
                    .map_err(|_| error(StatusCode::BAD_GATEWAY))?,
            })
        })
        .collect::<Result<Vec<_>, PageError>>()?;
    db.commit()
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    Ok(people)
}

pub(super) async fn section(
    state: &AppState,
    api: &mut WebApi,
    household_id: i64,
    slug: &str,
    status: Option<&str>,
) -> Result<String, PageError> {
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
    let draft = match reply.status {
        StatusCode::OK => draft_from_record(&reply.value)?,
        StatusCode::NOT_FOUND
            if reply.value.pointer("/error/code").and_then(Value::as_str)
                == Some("not_configured") =>
        {
            NotificationDraft::default()
        }
        other => return Err(error(other)),
    };
    let data = reply.value.get("data");
    let times = [
        "morning_time",
        "afternoon_time",
        "evening_time",
        "night_time",
    ]
    .map(|key| {
        data.and_then(|value| value.get(key))
            .and_then(Value::as_str)
            .map(|value| value.chars().take(5).collect())
    });
    let managed = managed_people(state, api.account_id, household_id).await?;
    render_notification_profile(ProfileNotificationPage {
        slug: slug.to_owned(),
        csrf: api.csrf.clone(),
        household_id,
        locale: api.locale,
        preferences: draft,
        editable,
        times,
        managed,
        saved: status == Some("saved"),
        error: status == Some("failed"),
        push_configured: std::env::var("VAPID_PUBLIC_KEY").is_ok_and(|key| !key.is_empty()),
    })
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))
}

async fn update_profile(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    RawForm(raw): RawForm,
) -> Response {
    if !oauth::trusted_cookie_origin(&state, &headers) {
        return failure(StatusCode::FORBIDDEN);
    }
    let mut api = match WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let mut fields = HashMap::new();
    let mut managed = Vec::new();
    let mut managed_present = false;
    for (key, value) in url::form_urlencoded::parse(&raw) {
        if key == "managed_person_ids[]" {
            managed_present = true;
            if !value.is_empty() {
                let Ok(id) = value.parse::<i64>() else {
                    return failure(StatusCode::BAD_REQUEST);
                };
                managed.push(id);
            }
        } else {
            fields.insert(key.into_owned(), value.into_owned());
        }
    }
    if fields.get("authenticity_token") != Some(&api.csrf) {
        return failure(StatusCode::FORBIDDEN);
    }
    let (household_id, _) = match api.household(&slug).await {
        Ok(value) => value,
        Err(error) => return error.response(),
    };
    let flags = match draft_from_fields(&fields) {
        Ok(value) => value,
        Err(error) => return error.response(),
    };
    let mut attributes = json!({
        "enabled": flags.enabled, "dose_due_enabled": flags.dose_due_enabled,
        "missed_dose_enabled": flags.missed_dose_enabled, "low_stock_enabled": flags.low_stock_enabled,
        "private_text_enabled": flags.private_text_enabled,
    });
    for key in [
        "morning_time",
        "afternoon_time",
        "evening_time",
        "night_time",
    ] {
        let value = fields.get(key).map(String::as_str).unwrap_or("");
        attributes[key] = if value.is_empty() {
            Value::Null
        } else {
            json!(value)
        };
    }
    if managed_present {
        attributes["managed_person_ids"] = json!(managed);
    }
    let csrf = api.csrf.clone();
    match api
        .call(
            Method::PATCH,
            &format!("/api/v1/households/{household_id}/notification_preference"),
            Some(json!({"notification_preference": attributes})),
            Some(&csrf),
        )
        .await
    {
        Ok(reply) if reply.status.is_success() => redirect(
            format!(
                "/households/{}/profile?section=notifications&notification_status=saved",
                path_segment(&slug)
            ),
            api.cookie.take(),
        ),
        Ok(reply) if reply.status == StatusCode::UNPROCESSABLE_ENTITY => redirect(
            format!(
                "/households/{}/profile?section=notifications&notification_status=failed",
                path_segment(&slug)
            ),
            api.cookie.take(),
        ),
        Ok(reply) => failure(reply.status),
        Err(error) => error.response(),
    }
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
    consume_saved: bool,
    notifications_visible: bool,
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
        notifications_visible: state.notifications_visible,
    }) {
        Ok(body) => {
            let mut response = page_status(body, cookie, state.status);
            if state.consume_saved {
                if let Some(cookie) = clear_saved_cookie(slug) {
                    response.headers_mut().append(header::SET_COOKIE, cookie);
                }
            }
            response
        }
        Err(_) => failure(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn edit(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    let marked = cookie_value(&headers, SAVED_COOKIE) == Some("1");
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
                        consume_saved: marked,
                        notifications_visible: first_use,
                        status: StatusCode::OK,
                    },
                ))
            }
            status if status.is_success() => {
                let draft = draft_from_record(&reply.value)?;
                Ok(render(
                    api,
                    name,
                    &slug,
                    RenderState {
                        draft: Some(draft),
                        editable,
                        saved: marked,
                        failed: false,
                        consume_saved: marked,
                        notifications_visible: true,
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
    let marked = cookie_value(&headers, SAVED_COOKIE) == Some("1");
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
            let mut response = redirect(
                format!("/households/{}/settings/notifications", path_segment(&slug)),
                api.cookie.take(),
            );
            if let Some(cookie) = saved_cookie(&slug) {
                response.headers_mut().append(header::SET_COOKIE, cookie);
            }
            return Ok(response);
        }
        if reply.status == StatusCode::UNAUTHORIZED {
            return Err(PageError::Login);
        }
        let notifications_visible = api.notifications_visible(household_id).await;
        Ok(render(
            api,
            name,
            &slug,
            RenderState {
                draft: Some(draft),
                editable: reply.status != StatusCode::FORBIDDEN,
                saved: false,
                failed: true,
                consume_saved: marked,
                notifications_visible,
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
