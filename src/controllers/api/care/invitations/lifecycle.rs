use super::*;

pub(in crate::controllers::api::care) async fn destroy(
    State(ctx): State<AppContext>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return no_store(response::error(error, &request_id)),
    };
    let result = destroy_record(&tenant, &principal, household_id, &id, &headers).await;
    complete(
        tenant,
        &principal,
        audit::RequestAudit::invitations("DELETE", "destroy"),
        result,
        &request_id,
    )
    .await
}
async fn destroy_record(
    tenant: &TenantTransaction,
    principal: &ValidatedPrincipal,
    household_id: i64,
    id: &str,
    headers: &HeaderMap,
) -> std::result::Result<(StatusCode, Value, Option<String>, bool), response::Failure> {
    crate::models::care::administration::authorize(tenant)
        .await
        .map_err(response::operation)?;
    let numeric = numeric_id(id)?;
    let path = format!("/api/v1/households/{household_id}/admin/invitations/{id}");
    let body = json!({});
    if let Some(key) = key(headers)
        && let Some(saved) =
            super::super::locations::keyed_replay(tenant, key, "DELETE", &path, &body).await?
    {
        return Ok(saved);
    }
    let savepoint = tenant
        .transaction()
        .begin()
        .await
        .map_err(|_| response::unavailable())?;
    let result = invitations::revoke(tenant, numeric, Some(principal.provenance()))
        .await
        .map(|()| json!({}));
    let (status, reply) = reply(savepoint, tenant, result, StatusCode::NO_CONTENT).await?;
    if let Some(key) = key(headers) {
        store(
            tenant,
            principal,
            SavedResponse {
                key,
                method: "DELETE",
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

pub(in crate::controllers::api::care) async fn resend(
    State(ctx): State<AppContext>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: axum::body::Bytes,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return no_store(response::error(error, &request_id)),
    };
    let result = resend_record(&ctx, &tenant, &principal, &id, &headers, &body).await;
    no_store(
        complete(
            tenant,
            &principal,
            audit::RequestAudit::invitations("POST", "resend"),
            result,
            &request_id,
        )
        .await,
    )
}
async fn resend_record(
    ctx: &AppContext,
    tenant: &TenantTransaction,
    principal: &ValidatedPrincipal,
    id: &str,
    headers: &HeaderMap,
    body: &[u8],
) -> std::result::Result<(StatusCode, Value, Option<String>, bool), response::Failure> {
    crate::models::care::administration::authorize(tenant)
        .await
        .map_err(response::operation)?;
    if !matches!(
        principal.provenance().method,
        CredentialMethod::ApiSession
            | CredentialMethod::OauthGrant
            | CredentialMethod::PersonalApiKey
    ) {
        return Err(response::operation(OperationError::Forbidden));
    }
    let numeric = numeric_id(id)?;
    let body = if body.is_empty() {
        json!({})
    } else {
        serde_json::from_slice(body)
            .map_err(|_| response::Failure::bad_request("Invalid request body"))?
    };
    let path = format!(
        "/api/v1/households/{}/admin/invitations/{id}/resend",
        tenant.scope().household_id
    );
    if let Some(key) = key(headers)
        && let Some(saved) =
            super::super::locations::keyed_replay(tenant, key, "POST", &path, &body).await?
    {
        return Ok(saved);
    }
    let target = url::Url::parse(&ctx.config.server.full_url())
        .and_then(|url| url.join("/invitations/accept"))
        .map_err(|_| response::unavailable())?;
    let savepoint = tenant
        .transaction()
        .begin()
        .await
        .map_err(|_| response::unavailable())?;
    let result =
        match invitations::resend(tenant, numeric, &target, Some(principal.provenance())).await {
            Ok(body) => Ok(body),
            Err(invitations::ResendError::Operation(error)) => Err(error),
            Err(invitations::ResendError::DeliveryUnavailable) => {
                savepoint
                    .rollback()
                    .await
                    .map_err(|_| response::unavailable())?;
                return Err(response::Failure::invitation_delivery_unavailable());
            }
        };
    let (status, reply) = reply(savepoint, tenant, result, StatusCode::OK).await?;
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
