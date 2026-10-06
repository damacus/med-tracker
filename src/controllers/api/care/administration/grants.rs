use super::*;

pub(in crate::controllers::api::care) async fn index(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(context) => context,
        Err(error) => return response::error(error, &request_id),
    };
    let result = administration::grants::list(&tenant)
        .await
        .map(|body| (StatusCode::OK, body, None))
        .map_err(response::operation);
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::administration("GET", "person_access_grants", "index"),
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
    mutate(ctx, household_id, None, headers, request, Some(body)).await
}
pub(in crate::controllers::api::care) async fn destroy(
    State(ctx): State<AppContext>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    mutate(ctx, household_id, Some(id), headers, request, None).await
}
async fn mutate(
    ctx: AppContext,
    household_id: i64,
    id: Option<String>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: Option<std::result::Result<AxumJson<Value>, JsonRejection>>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(context) => context,
        Err(error) => return response::error(error, &request_id),
    };
    let result = process(
        &tenant,
        &principal,
        household_id,
        id.as_deref(),
        &headers,
        body,
    )
    .await;
    complete(
        tenant,
        &principal,
        audit::RequestAudit::administration(
            if id.is_some() { "DELETE" } else { "POST" },
            "person_access_grants",
            if id.is_some() { "destroy" } else { "create" },
        ),
        result,
        &request_id,
    )
    .await
}
async fn process(
    tenant: &TenantTransaction,
    principal: &ValidatedPrincipal,
    household_id: i64,
    id: Option<&str>,
    headers: &HeaderMap,
    body: Option<std::result::Result<AxumJson<Value>, JsonRejection>>,
) -> std::result::Result<(StatusCode, Value, Option<String>, bool), response::Failure> {
    administration::authorize(tenant)
        .await
        .map_err(response::operation)?;
    let numeric_id = id.map(numeric_id).transpose()?;
    let body = match body {
        Some(body) => {
            body.map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?
                .0
        }
        None => Value::Null,
    };
    let method = if id.is_some() { "DELETE" } else { "POST" };
    let path = if let Some(id) = id {
        format!("/api/v1/households/{household_id}/admin/person_access_grants/{id}")
    } else {
        format!("/api/v1/households/{household_id}/admin/person_access_grants")
    };
    let key = key(headers);
    if let Some(key) = key
        && let Some(reply) =
            super::super::locations::keyed_replay(tenant, key, method, &path, &body).await?
    {
        return Ok(reply);
    }
    let attributes = if id.is_some() {
        Value::Null
    } else {
        attributes(&body, "person_access_grant")?
    };
    let savepoint = tenant
        .transaction()
        .begin()
        .await
        .map_err(|_| response::unavailable())?;
    let result = if let Some(id) = numeric_id {
        administration::grants::revoke(tenant, id, Some(principal.provenance()))
            .await
            .map(|()| Value::Null)
    } else {
        administration::grants::create(tenant, attributes, Some(principal.provenance())).await
    };
    let (status, reply) = reply(
        savepoint,
        tenant,
        result,
        if id.is_some() {
            StatusCode::NO_CONTENT
        } else {
            StatusCode::CREATED
        },
    )
    .await?;
    if let Some(key) = key {
        store(
            tenant,
            principal,
            SavedResponse {
                key,
                method,
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
