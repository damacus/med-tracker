use super::*;
use medtracker_web::household::path_segment;
use medtracker_web::household_i18n::Text;
use medtracker_web::settings::{render_settings, SettingsPage};
use serde::Deserialize;

#[derive(Default, Deserialize)]
pub(super) struct SettingsQuery {
    saved: Option<String>,
}

pub(super) fn routes() -> Router<AppState> {
    Router::new().route("/households/{slug}/settings", get(show).post(update))
}

pub(super) fn profile_errors(reply: &Value) -> HashMap<String, Vec<String>> {
    let mut errors = HashMap::new();
    if let Some(fields) = reply.pointer("/error/errors").and_then(Value::as_object) {
        for (field, messages) in fields {
            let messages: Vec<_> = messages
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect();
            if !messages.is_empty() {
                errors.insert(field.clone(), messages);
            }
        }
    }
    if errors.is_empty() {
        errors.insert("time_zone".to_owned(), vec!["is invalid".to_owned()]);
    }
    errors
}

pub(super) fn can_edit_profile(profile: &Value, capabilities: &Value) -> bool {
    let Some(person_id) = profile
        .pointer("/data/person_id")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<i64>().ok())
    else {
        return false;
    };
    capabilities
        .pointer("/data/people/manage_ids")
        .and_then(Value::as_array)
        .is_some_and(|ids| ids.iter().any(|id| id.as_i64() == Some(person_id)))
}

pub(super) fn time_zone_error(value: &str) -> Option<&'static str> {
    value.is_empty().then_some("can't be blank")
}

fn render(
    api: &WebApi,
    slug: &str,
    household_name: &str,
    time_zone: String,
    can_edit: bool,
    errors: HashMap<String, Vec<String>>,
    notice: String,
) -> Result<String, PageError> {
    render_settings(SettingsPage {
        household_name: household_name.to_owned(),
        slug: slug.to_owned(),
        locale: api.locale,
        csrf: api.csrf.clone(),
        time_zone,
        can_edit,
        errors,
        notice,
    })
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))
}

pub(super) async fn show(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<SettingsQuery>,
    headers: HeaderMap,
) -> Response {
    let mut api = match WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let result = async {
        let (household_id, household_name) = api.household(&slug).await?;
        let profile = api
            .get(&format!("/api/v1/households/{household_id}/profile"))
            .await?;
        let capabilities = api.capabilities(household_id).await?;
        let can_edit = can_edit_profile(&profile, &capabilities);
        let time_zone = profile
            .pointer("/data/time_zone")
            .and_then(Value::as_str)
            .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?
            .to_owned();
        let notice = if query.saved.as_deref() == Some("1") {
            Text::new(api.locale)
                .get("profiles.updated", &[])
                .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
        } else {
            String::new()
        };
        render(
            &api,
            &slug,
            &household_name,
            time_zone,
            can_edit,
            HashMap::new(),
            notice,
        )
    }
    .await;
    match result {
        Ok(body) => page(body, api.cookie),
        Err(error) => error.response(),
    }
}

pub(super) async fn update(
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
    let time_zone = fields.get("time_zone").cloned().unwrap_or_default();
    let result = async {
        let (household_id, household_name) = api.household(&slug).await?;
        let path = format!("/api/v1/households/{household_id}/profile");
        let profile = api.get(&path).await?;
        let capabilities = api.capabilities(household_id).await?;
        let can_edit = can_edit_profile(&profile, &capabilities);
        if !can_edit {
            return Err(error(StatusCode::FORBIDDEN));
        }
        if let Some(message) = time_zone_error(&time_zone) {
            let body = render(
                &api,
                &slug,
                &household_name,
                time_zone,
                can_edit,
                HashMap::from([("time_zone".to_owned(), vec![message.to_owned()])]),
                String::new(),
            )?;
            return Ok(page_status(
                body,
                api.cookie.take(),
                StatusCode::UNPROCESSABLE_ENTITY,
            ));
        }
        let csrf = api.csrf.clone();
        let reply = api
            .call(
                Method::PATCH,
                &path,
                Some(json!({"profile": {"time_zone": time_zone}})),
                Some(&csrf),
            )
            .await?;
        if reply.status.is_success() {
            let saved = api.get(&path).await?;
            if saved.pointer("/data/time_zone").and_then(Value::as_str) != Some(time_zone.as_str())
            {
                return Err(error(StatusCode::BAD_GATEWAY));
            }
            let location = format!("/households/{}/settings?saved=1", path_segment(&slug));
            return Ok(redirect(location, api.cookie.take()));
        }
        if reply.status != StatusCode::UNPROCESSABLE_ENTITY {
            return Err(error(reply.status));
        }
        let body = render(
            &api,
            &slug,
            &household_name,
            time_zone,
            can_edit,
            profile_errors(&reply.value),
            String::new(),
        )?;
        Ok(page_status(body, api.cookie.take(), reply.status))
    }
    .await;
    match result {
        Ok(response) => response,
        Err(error) => error.response(),
    }
}
