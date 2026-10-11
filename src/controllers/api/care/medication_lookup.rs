use super::*;
use crate::models::care::medication_lookup::{self, Search};
use axum::extract::Query;

pub(super) async fn show(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    Query(search): Query<Search>,
) -> Response {
    let request_id = administration::request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = match medication_lookup::search(&tenant, &search).await {
        Ok(found) => Ok((StatusCode::OK, found.body, None)),
        Err(OperationError::Unavailable) => Ok((
            StatusCode::SERVICE_UNAVAILABLE,
            json!({"results":[],"error":"Medication search is temporarily unavailable."}),
            None,
        )),
        Err(error) => Err(response::operation(error)),
    };
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::medication_lookup(),
        result,
        &request_id,
    )
    .await
}
