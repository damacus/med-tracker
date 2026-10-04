use super::*;
use crate::nhs_dmd;
use crate::ApiError;
use axum::extract::Multipart;
use medtracker_web::admin::{render_dmd_import, DmdImportPage, DmdImportRun};
use medtracker_web::household::path_segment;
use medtracker_web::household_i18n::Text;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/households/{slug}/admin/nhs-dmd-import",
            get(import_form).post(import_create),
        )
        .route_layer(axum::extract::DefaultBodyLimit::max(512 * 1024 * 1024))
}

async fn require_platform_admin(api: &mut api_client::WebApi) -> Result<(), PageError> {
    if !api.is_platform_admin().await? {
        return Err(error(StatusCode::FORBIDDEN));
    }
    Ok(())
}

fn run_value(value: &Value) -> Option<DmdImportRun> {
    Some(DmdImportRun {
        id: numeric(value, "id")?,
        filename: field(value, "uploaded_filename").to_owned(),
        status: field(value, "status").to_owned(),
        active: value.get("active").and_then(Value::as_bool)?,
        total_records: numeric(value, "total_records").unwrap_or(0),
        processed_records: numeric(value, "processed_records").unwrap_or(0),
        progress_percentage: numeric(value, "progress_percentage").unwrap_or(0),
        imported_count: numeric(value, "imported_count").unwrap_or(0),
        skipped_count: numeric(value, "skipped_count").unwrap_or(0),
        created_count: numeric(value, "created_count").unwrap_or(0),
        updated_count: numeric(value, "updated_count").unwrap_or(0),
        unchanged_count: numeric(value, "unchanged_count").unwrap_or(0),
        skipped_expired_count: numeric(value, "skipped_expired_count").unwrap_or(0),
        skipped_missing_name_count: numeric(value, "skipped_missing_name_count").unwrap_or(0),
        skipped_invalid_count: numeric(value, "skipped_invalid_count").unwrap_or(0),
        error_message: field(value, "error_message").to_owned(),
        log: field(value, "log").to_owned(),
        created_at: field(value, "created_at").to_owned(),
    })
}

async fn render_page(
    api: &mut api_client::WebApi,
    household_name: String,
    slug: String,
    household_id: i64,
    alert: String,
    notice: String,
    status: StatusCode,
) -> Response {
    let value = match api
        .get(&format!(
            "/api/v1/households/{household_id}/admin/nhs_dmd_imports"
        ))
        .await
    {
        Ok(value) => value,
        Err(error) => return error.response(),
    };
    let runs: Vec<DmdImportRun> = value
        .get("data")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().filter_map(run_value).collect())
        .unwrap_or_default();
    let action = format!("/households/{}/admin/nhs-dmd-import", path_segment(&slug));
    let notifications_visible = api.notifications_visible(household_id).await;
    let body = render_dmd_import(DmdImportPage {
        household_name,
        slug,
        locale: api.locale,
        csrf: api.csrf.clone(),
        action,
        runs,
        alert,
        notice,
        notifications_visible,
    });
    match body {
        Ok(body) => page_status(body, api.cookie.take(), status),
        Err(_) => failure(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn import_form(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    let mut api = match api_client::WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    if let Err(error) = require_platform_admin(&mut api).await {
        return error.response();
    }
    let (household_id, household_name) = match api.household(&slug).await {
        Ok(value) => value,
        Err(error) => return error.response(),
    };
    render_page(
        &mut api,
        household_name,
        slug,
        household_id,
        String::new(),
        String::new(),
        StatusCode::OK,
    )
    .await
}

async fn import_create(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    multipart: Multipart,
) -> Response {
    if !oauth::trusted_cookie_origin(&state, &headers) {
        return failure(StatusCode::FORBIDDEN);
    }
    let mut api = match api_client::WebApi::authenticated(state.clone(), headers.clone()).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    if let Err(error) = require_platform_admin(&mut api).await {
        return error.response();
    }
    let (household_id, household_name) = match api.household(&slug).await {
        Ok(value) => value,
        Err(error) => return error.response(),
    };
    let (path, filename, token) = match nhs_dmd::stage_upload(multipart).await {
        Ok(upload) => upload,
        Err(status) => {
            let alert = translate(&api, "admin.nhs_dmd_imports.missing_file");
            return render_page(
                &mut api,
                household_name,
                slug,
                household_id,
                alert,
                String::new(),
                status,
            )
            .await;
        }
    };
    if token != api.csrf {
        let _ = std::fs::remove_file(&path);
        return failure(StatusCode::FORBIDDEN);
    }
    match nhs_dmd::enqueue(&state, &headers, household_id, path.clone(), filename).await {
        Ok(_) => redirect(
            format!("/households/{}/admin/nhs-dmd-import", path_segment(&slug)),
            api.cookie,
        ),
        Err(error) => {
            let _ = std::fs::remove_file(&path);
            let alert = error_message(&api, &error);
            render_page(
                &mut api,
                household_name,
                slug,
                household_id,
                alert,
                String::new(),
                StatusCode::UNPROCESSABLE_ENTITY,
            )
            .await
        }
    }
}

fn translate(api: &api_client::WebApi, key: &str) -> String {
    Text::new(api.locale)
        .get(key, &[])
        .unwrap_or_else(|_| key.to_owned())
}

fn error_message(api: &api_client::WebApi, error: &ApiError) -> String {
    if error.status == StatusCode::CONFLICT {
        return translate(api, "admin.nhs_dmd_imports.already_running");
    }
    if error.status == StatusCode::FORBIDDEN {
        return translate(api, "errors.messages.forbidden");
    }
    error.message.to_owned()
}
