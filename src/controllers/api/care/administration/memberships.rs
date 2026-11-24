use super::*;
use crate::views::administration::MembershipSummary;

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
    let result = administration::memberships::list(&tenant)
        .await
        .map(|data| (StatusCode::OK, DataResponse { data }, None))
        .map_err(response::operation);
    finish_typed(
        tenant,
        principal.provenance(),
        audit::RequestAudit::administration("GET", "memberships", "index"),
        result,
        &request_id,
    )
    .await
}
pub(in crate::controllers::api::care) async fn update(
    State(ctx): State<AppContext>,
    ApiPath((household_id, id)): ApiPath<(i64, String)>,
    method: axum::http::Method,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    mutate(
        ctx,
        household_id,
        id,
        headers,
        request,
        Some(body),
        if method == axum::http::Method::PUT {
            "PUT"
        } else {
            "PATCH"
        },
    )
    .await
}
pub(in crate::controllers::api::care) async fn destroy(
    State(ctx): State<AppContext>,
    ApiPath((household_id, id)): ApiPath<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    mutate(ctx, household_id, id, headers, request, None, "DELETE").await
}
async fn mutate(
    ctx: AppContext,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: Option<std::result::Result<AxumJson<Value>, JsonRejection>>,
    method: &'static str,
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
        &id,
        &headers,
        body,
        method,
    )
    .await;
    complete_typed(
        tenant,
        &principal,
        audit::RequestAudit::administration(
            method,
            "memberships",
            if method == "DELETE" {
                "destroy"
            } else {
                "update"
            },
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
    id: &str,
    headers: &HeaderMap,
    body: Option<std::result::Result<AxumJson<Value>, JsonRejection>>,
    method: &str,
) -> AdminMutation<DataResponse<MembershipSummary>> {
    administration::authorize(tenant)
        .await
        .map_err(response::operation)?;
    let numeric_id = numeric_id(id)?;
    let body = match body {
        Some(body) => {
            body.map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?
                .0
        }
        None => Value::Null,
    };
    let path = format!("/api/v1/households/{household_id}/admin/memberships/{id}");
    let key = key(headers);
    if let Some(key) = key
        && let Some((status, body, etag, replay)) =
            super::super::locations::keyed_replay(tenant, key, method, &path, &body).await?
    {
        return Ok((status, MutationBody::Replay(body), etag, replay));
    }
    let attributes = if method == "DELETE" {
        administration::memberships::MembershipInput::Revoke
    } else {
        administration::memberships::MembershipInput::from_json(attributes(
            &body,
            "household_membership",
        )?)
    };
    let savepoint = tenant
        .transaction()
        .begin()
        .await
        .map_err(|_| response::unavailable())?;
    let result = administration::memberships::change(
        tenant,
        numeric_id,
        attributes,
        Some(principal.provenance()),
    )
    .await
    .map(|data| DataResponse { data });
    let (status, mut reply) = reply(
        savepoint,
        tenant,
        result,
        if method == "DELETE" {
            StatusCode::NO_CONTENT
        } else {
            StatusCode::OK
        },
    )
    .await?;
    if status == StatusCode::NO_CONTENT {
        reply = MutationBody::Empty;
    }
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
