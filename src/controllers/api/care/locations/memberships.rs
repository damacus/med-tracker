use super::*;

fn numeric(value: &str) -> bool {
    value
        .as_bytes()
        .first()
        .is_some_and(|byte| (b'1'..=b'9').contains(byte))
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && value.parse::<i64>().is_ok()
}

pub(crate) async fn create_membership(
    State(ctx): State<AppContext>,
    ApiPath((household_id, location_id)): ApiPath<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(context) => context,
        Err(error) => return response::error(error, &request_id),
    };
    let result = creation(
        &tenant,
        principal.provenance(),
        household_id,
        &location_id,
        &headers,
        body,
    )
    .await;
    let replay = result.as_ref().is_ok_and(|(_, _, _, replay)| *replay);
    let mut response = finish_typed(
        tenant,
        principal.provenance(),
        audit::RequestAudit::location_membership("POST", "create"),
        result.map(|(status, body, etag, _)| (status, body, etag)),
        &request_id,
    )
    .await;
    if replay && response.status().as_u16() < 500 && response.status() != StatusCode::CONFLICT {
        response.headers_mut().insert(
            "idempotency-replayed",
            axum::http::HeaderValue::from_static("true"),
        );
    }
    response
}

async fn creation(
    tenant: &TenantTransaction,
    provenance: &CredentialProvenance,
    household_id: i64,
    location_id: &str,
    headers: &HeaderMap,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> LocationMutation<LocationMembershipResponse> {
    locations::authorize(tenant)
        .await
        .map_err(response::operation)?;
    if !doses::valid_identifier(location_id) {
        return Ok((
            StatusCode::BAD_REQUEST,
            MutationBody::Error(
                response::Failure::bad_request("Invalid location ID")
                    .into_body(&tenant.scope().request_id),
            ),
            None,
            false,
        ));
    }
    let location = locations::read(tenant, location_id)
        .await
        .map_err(response::operation)?;
    let AxumJson(body) =
        body.map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?;
    let path =
        format!("/api/v1/households/{household_id}/locations/{location_id}/location_memberships");
    let key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty());
    if let Some(key) = key
        && let Some((status, body, etag, replay)) =
            keyed_replay(tenant, key, "POST", &path, &body).await?
    {
        return Ok((status, MutationBody::Replay(body), etag, replay));
    }
    let savepoint = tenant
        .transaction()
        .begin()
        .await
        .map_err(|_| response::unavailable())?;
    let result = locations::create_membership(
        tenant,
        &location,
        locations::LocationMembershipInput::from_json(&body),
        Some(provenance),
    )
    .await;
    let (status, representation) = match result {
        Ok((record, person)) => {
            savepoint
                .commit()
                .await
                .map_err(|_| response::unavailable())?;
            (
                StatusCode::CREATED,
                MutationBody::Resource(locations::membership_representation(
                    &record, &location, &person,
                )),
            )
        }
        Err(OperationError::Validation { details }) => {
            savepoint
                .rollback()
                .await
                .map_err(|_| response::unavailable())?;
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                MutationBody::Error(
                    response::Failure::fields(details["errors"].clone())
                        .into_body(&tenant.scope().request_id),
                ),
            )
        }
        Err(error) => {
            savepoint
                .rollback()
                .await
                .map_err(|_| response::unavailable())?;
            return Err(response::operation(error));
        }
    };
    if let Some(key) = key {
        locations::store(
            tenant,
            provenance,
            SavedResponse {
                key,
                method: "POST",
                path: &path,
                request: &body,
                status: status.as_u16(),
                body: serde_json::to_value(&representation).map_err(|_| response::unavailable())?,
                etag: None,
            },
        )
        .await
        .map_err(response::operation)?;
    }
    Ok((status, representation, None, false))
}

pub(crate) async fn destroy_membership(
    State(ctx): State<AppContext>,
    ApiPath((household_id, location_id, id)): ApiPath<(i64, String, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(context) => context,
        Err(error) => return response::error(error, &request_id),
    };
    let result = removal(
        &tenant,
        principal.provenance(),
        household_id,
        &location_id,
        &id,
        &headers,
    )
    .await;
    let replay = result.as_ref().is_ok_and(|(_, _, _, replay)| *replay);
    let mut response = finish_typed(
        tenant,
        principal.provenance(),
        audit::RequestAudit::location_membership("DELETE", "destroy"),
        result.map(|(status, body, etag, _)| (status, body, etag)),
        &request_id,
    )
    .await;
    if replay && response.status().as_u16() < 500 && response.status() != StatusCode::CONFLICT {
        response.headers_mut().insert(
            "idempotency-replayed",
            axum::http::HeaderValue::from_static("true"),
        );
    }
    response
}

async fn removal(
    tenant: &TenantTransaction,
    provenance: &CredentialProvenance,
    household_id: i64,
    location_id: &str,
    id: &str,
    headers: &HeaderMap,
) -> LocationMutation<()> {
    locations::authorize(tenant)
        .await
        .map_err(response::operation)?;
    if !doses::valid_identifier(location_id) || !numeric(id) {
        return Ok((
            StatusCode::BAD_REQUEST,
            MutationBody::Error(
                response::Failure::bad_request("Invalid resource ID")
                    .into_body(&tenant.scope().request_id),
            ),
            None,
            false,
        ));
    }
    let location = locations::read(tenant, location_id)
        .await
        .map_err(response::operation)?;
    let path = format!(
        "/api/v1/households/{household_id}/locations/{location_id}/location_memberships/{id}"
    );
    let request = json!({});
    let key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty());
    if let Some(key) = key
        && let Some((status, body, etag, replay)) =
            keyed_replay(tenant, key, "DELETE", &path, &request).await?
    {
        return Ok((status, MutationBody::Replay(body), etag, replay));
    }
    let savepoint = tenant
        .transaction()
        .begin()
        .await
        .map_err(|_| response::unavailable())?;
    let member_id = id.parse::<i64>().map_err(|_| response::unavailable())?;
    match locations::destroy_membership(tenant, &location, member_id, Some(provenance)).await {
        Ok(()) => {
            savepoint
                .commit()
                .await
                .map_err(|_| response::unavailable())?;
        }
        Err(error) => {
            savepoint
                .rollback()
                .await
                .map_err(|_| response::unavailable())?;
            return Err(response::operation(error));
        }
    }
    if let Some(key) = key {
        locations::store(
            tenant,
            provenance,
            SavedResponse {
                key,
                method: "DELETE",
                path: &path,
                request: &request,
                status: StatusCode::NO_CONTENT.as_u16(),
                body: Value::Null,
                etag: None,
            },
        )
        .await
        .map_err(response::operation)?;
    }
    Ok((StatusCode::NO_CONTENT, MutationBody::Empty, None, false))
}
