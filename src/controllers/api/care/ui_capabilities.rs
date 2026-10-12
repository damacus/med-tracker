use super::*;
use crate::models::care::ui_capabilities;

pub(super) async fn show(
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
    let reply = ui_capabilities::show(&tenant)
        .await
        .map(|body| (StatusCode::OK, body, None))
        .map_err(response::operation);
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::ui_capabilities(),
        reply,
        &request_id,
    )
    .await
}
