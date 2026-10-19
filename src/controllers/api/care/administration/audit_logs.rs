use super::*;

pub(in crate::controllers::api::care) async fn index(
    State(ctx): State<AppContext>,
    ApiPath(household_id): ApiPath<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(context) => context,
        Err(error) => return response::error(error, &request_id),
    };
    let result = administration::audit_logs::index(&tenant)
        .await
        .map(|rows| (StatusCode::OK, json!({"data": rows}), None))
        .map_err(response::operation);
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::administration("GET", "audit_logs", "index"),
        result,
        &request_id,
    )
    .await
}
