use super::*;
use crate::models::care;
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
    Path((household_id, mode)): Path<(i64, String)>,
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
    let passphrase = portable_passphrase(&headers);
    let reply = care::data_exports::export(&tenant, &mode, passphrase.as_deref())
        .await
        .map(|data| (StatusCode::OK, json!({"data": data}), None))
        .map_err(response::operation);
    let mut response = finish(
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
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    query: std::result::Result<Query<care::data_exports::ExportQuery>, QueryRejection>,
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
    let reply = match query {
        Ok(Query(query)) => {
            care::data_exports::portable_export(&tenant, &query, passphrase.as_deref())
                .await
                .map(|data| (StatusCode::OK, json!({"data": data}), None))
                .map_err(response::operation)
        }
        Err(_) => Err(response::Failure::validation("Export query is invalid")),
    };
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::portable_export(),
        reply,
        &request_id,
    )
    .await
}

pub(super) async fn mobile_snapshot(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
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
        .map(|data| (StatusCode::OK, json!({"data": data}), None))
        .map_err(response::operation);
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::mobile_snapshot(),
        reply,
        &request_id,
    )
    .await
}
