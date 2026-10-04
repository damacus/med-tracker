use super::*;
use axum::response::IntoResponse;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use medtracker_web::household::path_segment;
use medtracker_web::profile_advanced::{render_advanced, AdvancedPage, TokenSummary, Variants};

const EXPORT_RESPONSE_LIMIT: usize = 24 * 1024 * 1024;
const EXPORT_FILE_LIMIT: usize = 16 * 1024 * 1024;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/households/{slug}/profile/api_tokens", post(create_token))
        .route(
            "/households/{slug}/profile/api_tokens/{id}",
            post(revoke_token),
        )
        .route(
            "/households/{slug}/profile/data_exports/{mode}",
            get(download_export),
        )
        .route(
            "/households/{slug}/profile/experiments",
            post(save_experiment),
        )
        .route(
            "/households/{slug}/profile/close_account",
            post(close_account),
        )
}

fn location(slug: &str, status: &str) -> String {
    format!(
        "/households/{}/profile?section=advanced&advanced_status={status}",
        path_segment(slug)
    )
}

async fn form_api(
    state: &AppState,
    slug: &str,
    headers: HeaderMap,
    fields: &HashMap<String, String>,
) -> Result<(WebApi, i64), Box<Response>> {
    if !oauth::trusted_cookie_origin(state, &headers) {
        return Err(Box::new(failure(StatusCode::FORBIDDEN)));
    }
    let mut api = WebApi::authenticated(state.clone(), headers)
        .await
        .map_err(|error| Box::new(error.response()))?;
    if fields.get("authenticity_token") != Some(&api.csrf) {
        return Err(Box::new(failure(StatusCode::FORBIDDEN)));
    }
    let (household_id, _) = api
        .household(slug)
        .await
        .map_err(|error| Box::new(error.response()))?;
    Ok((api, household_id))
}

fn profile_editable(profile: &Value, capabilities: &Value) -> bool {
    super::settings::can_edit_profile(profile, capabilities)
}

async fn editable(api: &mut WebApi, household_id: i64) -> Result<bool, PageError> {
    let profile = api
        .get(&format!("/api/v1/households/{household_id}/profile"))
        .await?;
    let capabilities = api.capabilities(household_id).await?;
    Ok(profile_editable(&profile, &capabilities))
}

pub(super) async fn section(
    state: &AppState,
    api: &mut WebApi,
    household_id: i64,
    slug: &str,
    status: Option<&str>,
) -> Result<String, PageError> {
    section_with_token(state, api, household_id, slug, status, None).await
}

async fn section_with_token(
    state: &AppState,
    api: &mut WebApi,
    household_id: i64,
    slug: &str,
    status: Option<&str>,
    new_token: Option<String>,
) -> Result<String, PageError> {
    let can_edit = editable(api, household_id).await?;
    let membership_id =
        crate::profile_advanced_state::membership_id(state, api.account_id, household_id)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
            .ok_or_else(|| error(StatusCode::FORBIDDEN))?;
    let token_reply = api
        .call(
            Method::GET,
            &format!("/api/v1/households/{household_id}/admin/app_tokens"),
            None,
            None,
        )
        .await?;
    let can_manage_tokens = can_edit && token_reply.status.is_success();
    let tokens = if can_manage_tokens {
        token_reply
            .value
            .pointer("/data")
            .and_then(Value::as_array)
            .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?
            .iter()
            .filter(|row| row.get("revoked_at").is_none_or(Value::is_null))
            .map(|row| {
                Ok(TokenSummary {
                    id: row
                        .get("id")
                        .and_then(Value::as_i64)
                        .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?,
                    name: field(row, "name").to_owned(),
                    last_used_at: field(row, "last_used_at").to_owned(),
                    expires_at: field(row, "expires_at").to_owned(),
                })
            })
            .collect::<Result<Vec<_>, PageError>>()?
    } else {
        Vec::new()
    };
    let preferences = crate::profile_advanced_state::preferences(state, api.account_id)
        .await
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
    let variants = Variants {
        wizard: preferences
            .get("wizard_variant")
            .and_then(Value::as_str)
            .filter(|v| crate::profile_advanced_state::WIZARDS.contains(v))
            .unwrap_or("fullpage")
            .to_owned(),
        dashboard: preferences
            .get("dashboard_variant")
            .and_then(Value::as_str)
            .filter(|v| crate::profile_advanced_state::DASHBOARDS.contains(v))
            .unwrap_or("current")
            .to_owned(),
        medication_launcher: preferences
            .get("medication_launcher_variant")
            .and_then(Value::as_str)
            .filter(|v| crate::profile_advanced_state::LAUNCHERS.contains(v))
            .unwrap_or("current")
            .to_owned(),
    };
    let notice = match status {
        Some("token_revoked") => Some("API token revoked.".to_owned()),
        Some("experiments_saved") => Some("Experiment preference saved.".to_owned()),
        _ => None,
    };
    render_advanced(AdvancedPage {
        locale: api.locale,
        slug: slug.to_owned(),
        csrf: api.csrf.clone(),
        membership_id,
        can_edit,
        can_manage_tokens,
        tokens,
        variants,
        version: std::env::var("APP_VERSION")
            .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_owned()),
        notice,
        new_token,
    })
    .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))
}

async fn save_experiment(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    let (mut api, household_id) = match form_api(&state, &slug, headers, &fields).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let result = async {
        if !editable(&mut api, household_id).await? {
            return Err(error(StatusCode::FORBIDDEN));
        }
        let choices: Vec<_> = fields
            .iter()
            .filter(|(key, _)| {
                matches!(
                    key.as_str(),
                    "wizard_variant" | "dashboard_variant" | "medication_launcher_variant"
                )
            })
            .collect();
        if choices.len() != 1 {
            return Err(error(StatusCode::UNPROCESSABLE_ENTITY));
        }
        let (key, value) = choices[0];
        if !crate::profile_advanced_state::save_choice(&state, api.account_id, key, value)
            .await
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?
        {
            return Err(error(StatusCode::UNPROCESSABLE_ENTITY));
        }
        Ok(redirect(
            location(&slug, "experiments_saved"),
            api.cookie.take(),
        ))
    }
    .await;
    result.unwrap_or_else(PageError::response)
}

async fn create_token(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    let (mut api, household_id) = match form_api(&state, &slug, headers, &fields).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let result = async {
        if !editable(&mut api, household_id).await? {
            return Err(error(StatusCode::FORBIDDEN));
        }
        let name = fields
            .get("api_app_token[name]")
            .map(String::as_str)
            .unwrap_or("");
        let membership_id = fields
            .get("api_app_token[household_membership_id]")
            .and_then(|v| v.parse::<i64>().ok());
        let own =
            crate::profile_advanced_state::membership_id(&state, api.account_id, household_id)
                .await
                .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        if membership_id != own {
            return Err(error(StatusCode::NOT_FOUND));
        }
        if name.trim().is_empty() || name.len() > 120 {
            return Err(error(StatusCode::UNPROCESSABLE_ENTITY));
        }
        let csrf = api.csrf.clone();
        let reply = api
            .call(
                Method::POST,
                &format!("/api/v1/households/{household_id}/admin/app_tokens"),
                Some(json!({"api_app_token": {"name": name}})),
                Some(&csrf),
            )
            .await?;
        if reply.status != StatusCode::CREATED {
            return Err(error(reply.status));
        }
        let token = reply
            .value
            .pointer("/data/token")
            .and_then(Value::as_str)
            .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?
            .to_owned();
        let (_, household_name) = api.household(&slug).await?;
        let profile = api
            .get(&format!("/api/v1/households/{household_id}/profile"))
            .await?;
        let me = api
            .get(&format!("/api/v1/households/{household_id}/me"))
            .await?;
        let capabilities = api.capabilities(household_id).await?;
        let can_edit = profile_editable(&profile, &capabilities);
        let time_zone = profile
            .pointer("/data/time_zone")
            .and_then(Value::as_str)
            .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?
            .to_owned();
        let advanced =
            section_with_token(&state, &mut api, household_id, &slug, None, Some(token)).await?;
        let body = super::settings::render(super::settings::ProfileRender {
            api: &api,
            slug: &slug,
            household_name: &household_name,
            household_id,
            me: &me,
            profile: &profile,
            time_zone,
            can_edit,
            errors: HashMap::new(),
            notice: String::new(),
            active_section: "advanced",
            security_html: String::new(),
            notifications_html: String::new(),
            advanced_html: advanced,
        })?;
        Ok(super::response::profile_page_status(
            body,
            api.cookie.take(),
            StatusCode::CREATED,
        ))
    }
    .await;
    result.unwrap_or_else(PageError::response)
}

async fn revoke_token(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    let (mut api, household_id) = match form_api(&state, &slug, headers, &fields).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if id.parse::<i64>().ok().filter(|id| *id > 0).is_none() {
        return failure(StatusCode::NOT_FOUND);
    }
    match editable(&mut api, household_id).await {
        Ok(true) => {}
        Ok(false) => return failure(StatusCode::FORBIDDEN),
        Err(error) => return error.response(),
    };
    let csrf = api.csrf.clone();
    match api
        .call(
            Method::DELETE,
            &format!("/api/v1/households/{household_id}/admin/app_tokens/{id}"),
            None,
            Some(&csrf),
        )
        .await
    {
        Ok(reply) if reply.status.is_success() => {
            redirect(location(&slug, "token_revoked"), api.cookie.take())
        }
        Ok(reply) => failure(reply.status),
        Err(error) => error.response(),
    }
}

async fn download_export(
    State(state): State<AppState>,
    Path((slug, mode)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if !matches!(mode.as_str(), "health_data_json" | "backup_zip") {
        return failure(StatusCode::NOT_FOUND);
    }
    let mut api = match WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let result = async {
        let (household_id, _) = api.household(&slug).await?;
        let reply = api
            .get_with_limit(
                &format!("/api/v1/households/{household_id}/data_exports/{mode}"),
                EXPORT_RESPONSE_LIMIT,
            )
            .await?;
        if !reply.status.is_success() {
            return Err(error(reply.status));
        }
        let (content_type, filename, bytes) = if mode == "backup_zip" {
            let data = reply
                .value
                .pointer("/data")
                .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
            if field(data, "content_type") != "application/zip" {
                return Err(error(StatusCode::BAD_GATEWAY));
            }
            let filename = field(data, "filename");
            if !filename.starts_with("medtracker-backup-")
                || !filename.ends_with(".zip")
                || !filename
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
            {
                return Err(error(StatusCode::BAD_GATEWAY));
            }
            let bytes = STANDARD
                .decode(field(data, "base64"))
                .map_err(|_| error(StatusCode::BAD_GATEWAY))?;
            ("application/zip", filename.to_owned(), bytes)
        } else {
            let data = reply
                .value
                .pointer("/data")
                .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
            (
                "application/json",
                "medtracker-health-data.json".to_owned(),
                serde_json::to_vec(data).map_err(|_| error(StatusCode::BAD_GATEWAY))?,
            )
        };
        if bytes.len() > EXPORT_FILE_LIMIT {
            return Err(error(StatusCode::BAD_GATEWAY));
        }
        let disposition = HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
            .map_err(|_| error(StatusCode::BAD_GATEWAY))?;
        let mut response = (StatusCode::OK, bytes).into_response();
        response
            .headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
        response
            .headers_mut()
            .insert(header::CONTENT_DISPOSITION, disposition);
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        Ok(response)
    }
    .await;
    result.unwrap_or_else(PageError::response)
}

async fn close_account(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    let (api, _) = match form_api(&state, &slug, headers, &fields).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let password = fields.get("password").map(String::as_str).unwrap_or("");
    if password.is_empty() || password.len() > 1024 {
        return failure(StatusCode::UNPROCESSABLE_ENTITY);
    }
    match crate::profile_advanced_state::close_account(&state, api.account_id, password.to_owned())
        .await
    {
        Ok(Some(true)) => redirect("/login", Some(oauth::clear_browser_session_cookie(&state))),
        Ok(Some(false)) => failure(StatusCode::UNPROCESSABLE_ENTITY),
        Ok(None) => failure(StatusCode::UNAUTHORIZED),
        Err(_) => failure(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
