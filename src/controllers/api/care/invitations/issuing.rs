use super::*;

pub(in crate::controllers::api::care) async fn index(
    State(ctx): State<AppContext>,
    ApiPath(household_id): ApiPath<i64>,
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
        .map(|data| (StatusCode::OK, DataResponse { data }, None))
        .map_err(response::operation);
    finish_typed(
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
    ApiPath(household_id): ApiPath<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    dependent: Option<Extension<super::DependentSharing>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = process(
        &tenant,
        &principal,
        household_id,
        &headers,
        body,
        dependent.is_some(),
    )
    .await;
    complete_typed(
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
    dependent: bool,
) -> MutationReply<DataResponse<InvitationSummary>> {
    crate::models::care::administration::authorize(tenant)
        .await
        .map_err(response::operation)?;
    let AxumJson(body) =
        body.map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?;
    let path = format!(
        "/api/v1/households/{household_id}/admin/{}",
        if dependent {
            "dependent_sharing_invitations"
        } else {
            "invitations"
        }
    );
    if let Some(key) = key(headers)
        && let Some((status, body, etag, replay)) =
            super::super::locations::keyed_replay(tenant, key, "POST", &path, &body).await?
    {
        return Ok((status, MutationBody::Replay(body), etag, replay));
    }
    let attributes = if dependent {
        body.as_object()
            .filter(|values| values.len() == 1 && values.contains_key("household_invitation"))
            .ok_or_else(|| response::Failure::bad_request("Invalid request body"))?;
        let values = body
            .get("household_invitation")
            .and_then(Value::as_object)
            .filter(|values| values.len() == 1)
            .ok_or_else(|| {
                response::Failure::bad_request("Only the guardian recipient email may be selected")
            })?;
        invitations::InvitationInput::dependent(
            values
                .get("email")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        )
    } else {
        attributes(&body)?
    };
    let savepoint = tenant
        .transaction()
        .begin()
        .await
        .map_err(|_| response::unavailable())?;
    let result = invitations::create(tenant, attributes, None, Some(principal.provenance()))
        .await
        .map(|data| DataResponse { data });
    let (status, reply) = reply(savepoint, tenant, result, StatusCode::CREATED).await?;
    if let Some(key) = key(headers) {
        crate::models::care::locations::store(
            tenant,
            principal.provenance(),
            SavedResponse {
                key,
                method: "POST",
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
