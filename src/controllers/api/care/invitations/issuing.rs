use super::*;

pub(in crate::controllers::api::care) async fn index(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = invitations::list_api(&tenant)
        .await
        .map(|body| (StatusCode::OK, body, None))
        .map_err(response::operation);
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::invitations("GET", "index"),
        result,
        &request_id,
    )
    .await
}

pub(in crate::controllers::api::care) async fn create(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = process(&tenant, &principal, household_id, &headers, body).await;
    complete(
        tenant,
        &principal,
        audit::RequestAudit::invitations("POST", "create"),
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
) -> std::result::Result<(StatusCode, Value, Option<String>, bool), response::Failure> {
    crate::models::care::administration::authorize(tenant)
        .await
        .map_err(response::operation)?;
    let AxumJson(body) =
        body.map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?;
    let path = format!("/api/v1/households/{household_id}/admin/invitations");
    if let Some(key) = key(headers)
        && let Some(saved) =
            super::super::locations::keyed_replay(tenant, key, "POST", &path, &body).await?
    {
        return Ok(saved);
    }
    let attributes = attributes(&body)?;
    let savepoint = tenant
        .transaction()
        .begin()
        .await
        .map_err(|_| response::unavailable())?;
    let result = invitations::create(tenant, attributes, None, Some(principal.provenance())).await;
    let (status, reply) = reply(savepoint, tenant, result, StatusCode::CREATED).await?;
    if let Some(key) = key(headers) {
        store(
            tenant,
            principal,
            SavedResponse {
                key,
                method: "POST",
                path: &path,
                request: &body,
                status: status.as_u16(),
                body: reply,
                etag: None,
            },
        )
        .await
    } else {
        Ok((status, reply, None, false))
    }
}
