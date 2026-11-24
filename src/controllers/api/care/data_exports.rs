use super::*;
use crate::models::care;
use crate::views::{DataResponse, portable::ExportStatus};
use axum::extract::{Query, rejection::QueryRejection};
use axum::http::{HeaderValue, header};

const PASSPHRASE_HEADER: &str = "x-medtracker-portable-passphrase";

fn portable_passphrase(headers: &HeaderMap) -> Option<String> {
    headers
        .get(PASSPHRASE_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
}

pub(super) async fn show(
    State(ctx): State<AppContext>,
    ApiPath((household_id, mode)): ApiPath<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let reply = if mode == "encrypted_migration_bundle" {
        Err(response::Failure::validation(
            "Submit encrypted exports with POST",
        ))
    } else {
        care::data_exports::export(&tenant, &mode, None)
            .await
            .map(|data| (StatusCode::OK, DataResponse { data }, None))
            .map_err(response::operation)
    };
    let mut response = finish_typed(
        tenant,
        principal.provenance(),
        audit::RequestAudit::data_export(),
        reply,
        &request_id,
    )
    .await;
    if matches!(
        response.status(),
        StatusCode::OK | StatusCode::UNPROCESSABLE_ENTITY
    ) {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    response
}

pub(super) async fn portable_export(
    State(ctx): State<AppContext>,
    ApiPath(household_id): ApiPath<i64>,
    headers: HeaderMap,
    query: std::result::Result<Query<care::data_exports::ExportQuery>, QueryRejection>,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    submit(
        ctx,
        household_id,
        headers,
        query.map(|Query(query)| query).ok(),
        false,
        request,
    )
    .await
}

pub(super) async fn submit_mode(
    State(ctx): State<AppContext>,
    ApiPath((household_id, mode)): ApiPath<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    submit(
        ctx,
        household_id,
        headers,
        (mode == "encrypted_migration_bundle").then(care::data_exports::ExportQuery::default),
        true,
        request,
    )
    .await
}

async fn submit(
    ctx: AppContext,
    household_id: i64,
    headers: HeaderMap,
    query: Option<care::data_exports::ExportQuery>,
    migration_bundle: bool,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let passphrase = portable_passphrase(&headers);
    let submitted = match (query, passphrase) {
        (Some(query), Some(passphrase)) => {
            care::data_exports::jobs::submit(&ctx, &tenant, query, &passphrase, migration_bundle)
                .await
                .map_err(response::operation)
        }
        (None, _) => Err(response::Failure::validation(if migration_bundle {
            "Export mode is unsupported"
        } else {
            "Export query is invalid"
        })),
        (_, None) => Err(response::Failure::validation(
            "Portable passphrase header is required",
        )),
    };
    let path = submitted
        .as_ref()
        .ok()
        .map(|id| format!("/api/v1/households/{household_id}/portable_export/{id}"));
    let reply = submitted.map(|id| {
        (
            StatusCode::ACCEPTED,
            DataResponse {
                data: ExportStatus {
                    id: id.to_string(),
                    status: "requested".into(),
                },
            },
            None,
        )
    });
    let mut response = finish_typed(
        tenant,
        principal.provenance(),
        audit::RequestAudit::submit_export(migration_bundle),
        reply,
        &request_id,
    )
    .await;
    if response.status() == StatusCode::ACCEPTED
        && let Some(path) = path.and_then(|path| HeaderValue::from_str(&path).ok())
    {
        response.headers_mut().insert(header::LOCATION, path);
    }
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub(super) async fn status(
    State(ctx): State<AppContext>,
    ApiPath((household_id, job_id)): ApiPath<(i64, i64)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    read_job(ctx, household_id, job_id, headers, request, false).await
}

pub(super) async fn download(
    State(ctx): State<AppContext>,
    ApiPath((household_id, job_id)): ApiPath<(i64, i64)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    read_job(ctx, household_id, job_id, headers, request, true).await
}

async fn read_job(
    ctx: AppContext,
    household_id: i64,
    job_id: i64,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    download: bool,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let reply = care::data_exports::jobs::read(&tenant, job_id, download)
        .await
        .map(|data| (StatusCode::OK, DataResponse { data }, None))
        .map_err(response::operation);
    let mut response = finish_typed(
        tenant,
        principal.provenance(),
        audit::RequestAudit::portable_export(),
        reply,
        &request_id,
    )
    .await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub(super) async fn mobile_snapshot(
    State(ctx): State<AppContext>,
    ApiPath(household_id): ApiPath<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let reply = care::data_exports::mobile_snapshot(&tenant)
        .await
        .map(|data| (StatusCode::OK, DataResponse { data }, None))
        .map_err(response::operation);
    finish_typed(
        tenant,
        principal.provenance(),
        audit::RequestAudit::mobile_snapshot(),
        reply,
        &request_id,
    )
    .await
}
