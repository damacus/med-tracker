mod extract;
mod import;

use crate::entities::{nhs_dmd_import, platform_admin};
use crate::medication_management::request_context;
use crate::{database_error, ApiError, AppState};
use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter,
    QueryOrder, Set, Statement,
};
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;

const MAX_UPLOAD_BYTES: usize = 512 * 1024 * 1024;
const STALE_AFTER_MINUTES: i64 = 30;
pub(crate) const INTERRUPTION_MESSAGE: &str =
    "Import interrupted because the worker stopped reporting progress.";

const STATUS_NAMES: [&str; 6] = [
    "queued",
    "extracting",
    "counting",
    "importing",
    "completed",
    "failed",
];

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/households/{household_id}/admin/nhs_dmd_imports",
            get(index).post(create),
        )
        .route(
            "/api/v1/households/{household_id}/admin/nhs_dmd_imports/{id}",
            get(show),
        )
        .route_layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES))
}

fn api_error(status: StatusCode, code: &'static str, message: &'static str) -> ApiError {
    ApiError {
        status,
        code,
        message,
        preserve_activity: false,
    }
}

fn run_value(row: &nhs_dmd_import::Model) -> Value {
    let status = STATUS_NAMES
        .get(row.status as usize)
        .copied()
        .unwrap_or("failed");
    let percentage = if row.total_records > 0 {
        (row.processed_records as f64 / row.total_records as f64 * 100.0).floor() as i64
    } else {
        0
    };
    json!({
        "id": row.id,
        "uploaded_filename": row.uploaded_filename,
        "status": status,
        "active": row.status <= 3,
        "total_records": row.total_records,
        "processed_records": row.processed_records,
        "progress_percentage": percentage,
        "imported_count": row.imported_count,
        "skipped_count": row.skipped_count,
        "created_count": row.created_count,
        "updated_count": row.updated_count,
        "unchanged_count": row.unchanged_count,
        "skipped_expired_count": row.skipped_expired_count,
        "skipped_missing_name_count": row.skipped_missing_name_count,
        "skipped_invalid_count": row.skipped_invalid_count,
        "error_message": row.error_message,
        "log": row.log,
        "started_at": row.started_at.map(|at| at.and_utc().to_rfc3339()),
        "completed_at": row.completed_at.map(|at| at.and_utc().to_rfc3339()),
        "created_at": row.created_at.and_utc().to_rfc3339(),
    })
}

pub(crate) async fn platform_admin(
    db: &impl ConnectionTrait,
    account_id: i64,
) -> Result<bool, ApiError> {
    let row = platform_admin::Entity::find()
        .filter(platform_admin::Column::AccountId.eq(account_id))
        .one(db)
        .await
        .map_err(database_error)?;
    Ok(row.is_some_and(|row| row.status == "active"))
}

async fn recover_stale_runs(db: &impl ConnectionTrait) -> Result<(), ApiError> {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "UPDATE nhs_dmd_imports SET status = 5, completed_at = now(), error_message = $1, log = COALESCE(log, '') || $1 || E'\\n', updated_at = now() WHERE status IN (0, 1, 2, 3) AND updated_at < now() - $2::int * interval '1 minute'",
        [INTERRUPTION_MESSAGE.into(), STALE_AFTER_MINUTES.into()],
    ))
    .await
    .map_err(database_error)?;
    Ok(())
}

pub(crate) async fn enqueue(
    state: &AppState,
    headers: &HeaderMap,
    household_id: i64,
    path: PathBuf,
    filename: String,
) -> Result<i64, ApiError> {
    let (db, context) = request_context(state, headers, household_id).await?;
    if !platform_admin(&db, context.account_id).await? {
        return Err(ApiError::forbidden());
    }
    recover_stale_runs(&db).await?;
    let now = Utc::now().naive_utc();
    let run = nhs_dmd_import::ActiveModel {
        uploaded_filename: Set(filename),
        status: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .map_err(|error| {
        if error
            .to_string()
            .contains("index_nhs_dmd_imports_one_active")
        {
            api_error(
                StatusCode::CONFLICT,
                "conflict",
                "An NHS dm+d import is already running. Please wait for it to finish.",
            )
        } else {
            database_error(error)
        }
    })?;
    db.commit().await.map_err(database_error)?;
    let task_db = state.db.clone();
    let run_id = run.id;
    tokio::spawn(async move {
        if let Err(message) = import::run_import(task_db.clone(), run_id, path).await {
            import::fail_run(&task_db, run_id, &message).await;
        }
    });
    Ok(run.id)
}

async fn index(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !platform_admin(&db, context.account_id).await? {
        return Err(ApiError::forbidden());
    }
    recover_stale_runs(&db).await?;
    let rows = nhs_dmd_import::Entity::find()
        .order_by_desc(nhs_dmd_import::Column::Id)
        .all(&db)
        .await
        .map_err(database_error)?;
    let data: Vec<Value> = rows.iter().map(run_value).collect();
    db.commit().await.map_err(database_error)?;
    Ok(Json(json!({ "data": data })).into_response())
}

async fn show(
    State(state): State<AppState>,
    Path((household_id, id)): Path<(i64, i64)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !platform_admin(&db, context.account_id).await? {
        return Err(ApiError::forbidden());
    }
    let row = nhs_dmd_import::Entity::find_by_id(id)
        .one(&db)
        .await
        .map_err(database_error)?
        .ok_or_else(ApiError::not_found)?;
    Ok(Json(json!({ "data": run_value(&row) })).into_response())
}

async fn create(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    multipart: Multipart,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    if !platform_admin(&db, context.account_id).await? {
        return Err(ApiError::forbidden());
    }
    db.commit().await.map_err(database_error)?;
    let (path, filename, _) = match stage_upload(multipart).await {
        Ok(upload) => upload,
        Err(status) => {
            return Err(api_error(
                status,
                "invalid_upload",
                "Select an NHS dm+d release ZIP to import.",
            ))
        }
    };
    let run_id = match enqueue(&state, &headers, household_id, path.clone(), filename).await {
        Ok(id) => id,
        Err(error) => {
            let _ = std::fs::remove_file(&path);
            return Err(error);
        }
    };
    Ok((
        StatusCode::CREATED,
        Json(json!({ "data": { "id": run_id } })),
    )
        .into_response())
}

pub(crate) async fn stage_upload(
    mut multipart: Multipart,
) -> Result<(PathBuf, String, String), StatusCode> {
    let mut saved: Option<(PathBuf, String)> = None;
    let mut token = String::new();
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        if field.name() == Some("authenticity_token") {
            token = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            continue;
        }
        if field.name() != Some("release_zip") {
            continue;
        }
        let filename = field.file_name().unwrap_or("release.zip").to_owned();
        let path =
            std::env::temp_dir().join(format!("nhs-dmd-upload-{}.zip", uuid::Uuid::new_v4()));
        let mut file =
            std::fs::File::create(&path).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        let mut written = 0_usize;
        loop {
            match field.chunk().await {
                Ok(Some(chunk)) => {
                    written += chunk.len();
                    if written > MAX_UPLOAD_BYTES {
                        let _ = std::fs::remove_file(&path);
                        return Err(StatusCode::PAYLOAD_TOO_LARGE);
                    }
                    file.write_all(&chunk)
                        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
                }
                Ok(None) => break,
                Err(_) => {
                    let _ = std::fs::remove_file(&path);
                    return Err(StatusCode::BAD_REQUEST);
                }
            }
        }
        file.flush().map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        if written == 0 {
            let _ = std::fs::remove_file(&path);
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        saved = Some((path, filename));
    }
    saved
        .map(|(path, filename)| (path, filename, token))
        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)
}
