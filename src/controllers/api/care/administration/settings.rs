use super::*;
use crate::views::administration::HouseholdSettings;

pub(in crate::controllers::api::care) async fn show(
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
    let reply = administration::settings::read(&tenant)
        .await
        .map(|data| (StatusCode::OK, DataResponse { data }, None))
        .map_err(response::operation);
    finish_typed(
        tenant,
        principal.provenance(),
        audit::RequestAudit::administration("GET", "settings", "show"),
        reply,
        &request_id,
    )
    .await
}
pub(in crate::controllers::api::care) async fn update(
    State(ctx): State<AppContext>,
    ApiPath(household_id): ApiPath<i64>,
    method: axum::http::Method,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    let request_id = request_id(request);
    let method = if method == axum::http::Method::PUT {
        "PUT"
    } else {
        "PATCH"
    };
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(context) => context,
        Err(error) => return response::error(error, &request_id),
    };
    let result = process(&tenant, &principal, household_id, &headers, body, method).await;
    complete_typed(
        tenant,
        &principal,
        audit::RequestAudit::administration(method, "settings", "update"),
        result,
        &request_id,
    )
    .await
}
async fn process(
    tenant: &TenantTransaction,
    principal: &ValidatedPrincipal,
    household_id: i64,
    headers: &HeaderMap,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
    method: &str,
) -> AdminMutation<DataResponse<HouseholdSettings>> {
    administration::authorize(tenant)
        .await
        .map_err(response::operation)?;
    let AxumJson(body) =
        body.map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?;
    let path = format!("/api/v1/households/{household_id}/admin/settings");
    let key = key(headers);
    if let Some(key) = key
        && let Some((status, body, etag, replay)) =
            super::super::locations::keyed_replay(tenant, key, method, &path, &body).await?
    {
        return Ok((status, MutationBody::Replay(body), etag, replay));
    }
    let attributes = attributes(&body, "household")?;
    let savepoint = tenant
        .transaction()
        .begin()
        .await
        .map_err(|_| response::unavailable())?;
    let result = administration::settings::update(
        tenant,
        administration::settings::HouseholdSettingsInput::from_json(attributes),
        Some(principal.provenance()),
    )
    .await
    .map(|data| DataResponse { data });
    let (status, reply) = reply(savepoint, tenant, result, StatusCode::OK).await?;
    if let Some(key) = key {
        locations::store(
            tenant,
            principal.provenance(),
            SavedResponse {
                key,
                method,
                path: &path,
                request: &body,
                status: status.as_u16(),
                body: serde_json::to_value(&reply).map_err(|_| response::unavailable())?,
                etag: None,
            },
        )
        .await
        .map_err(response::operation)?;
    }
    Ok((status, reply, None, false))
}
