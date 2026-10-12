use super::*;
use crate::models::care;
use axum::http::{HeaderValue, header};

const PASSPHRASE_HEADER: &str = "x-medtracker-portable-passphrase";

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
    let passphrase = headers
        .get(PASSPHRASE_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned);
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
