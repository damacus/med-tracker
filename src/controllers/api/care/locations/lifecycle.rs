use super::*;
use headers::HeaderMapExt;

pub(crate) async fn index(
    State(ctx): State<AppContext>,
    ApiPath(household_id): ApiPath<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    page: std::result::Result<
        axum::extract::Query<locations::Pagination>,
        axum::extract::rejection::QueryRejection,
    >,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let reply = match page {
        Ok(axum::extract::Query(page)) => locations::collection(&tenant, page)
            .await
            .map(|body| (StatusCode::OK, body, None))
            .map_err(response::operation),
        Err(_) => Err(response::Failure::validation(
            "page must be positive and per_page must be between 1 and 100",
        )),
    };
    finish_typed(
        tenant,
        principal.provenance(),
        audit::RequestAudit::location("GET", "index"),
        reply,
        &request_id,
    )
    .await
}

pub(crate) async fn show(
    State(ctx): State<AppContext>,
    ApiPath((household_id, id)): ApiPath<(i64, String)>,
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
    let reply = locations::read(&tenant, &id)
        .await
        .map(|record| {
            let (body, etag) = locations::representation(&record);
            let not_modified = headers
                .typed_get::<headers::IfNoneMatch>()
                .zip(etag.parse::<headers::ETag>().ok())
                .is_some_and(|(condition, etag)| !condition.precondition_passes(&etag));
            if not_modified {
                (StatusCode::NOT_MODIFIED, None, Some(etag))
            } else {
                (StatusCode::OK, Some(body), Some(etag))
            }
        })
        .map_err(response::operation);
    finish_typed(
        tenant,
        principal.provenance(),
        audit::RequestAudit::location("GET", "show"),
        reply,
        &request_id,
    )
    .await
}

pub(crate) async fn update(
    State(ctx): State<AppContext>,
    ApiPath((household_id, id)): ApiPath<(i64, String)>,
    method: axum::http::Method,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    let method = if method == axum::http::Method::PUT {
        "PUT"
    } else {
        "PATCH"
    };
    mutate(ctx, household_id, id, headers, request, body, method).await
}

pub(crate) async fn destroy(
    State(ctx): State<AppContext>,
    ApiPath((household_id, id)): ApiPath<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    mutate(
        ctx,
        household_id,
        id,
        headers,
        request,
        Ok(AxumJson(json!({}))),
        "DELETE",
    )
    .await
}

async fn mutate(
    ctx: AppContext,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
    method: &'static str,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = mutation(
        &tenant,
        principal.provenance(),
        household_id,
        &id,
        &headers,
        body,
        method,
    )
    .await;
    let replay = result.as_ref().is_ok_and(|(_, _, _, replay)| *replay);
    let mut response = finish_typed(
        tenant,
        principal.provenance(),
        audit::RequestAudit::location(
            method,
            if method == "DELETE" {
                "destroy"
            } else {
                "update"
            },
        ),
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

async fn mutation(
    tenant: &TenantTransaction,
    provenance: &CredentialProvenance,
    household_id: i64,
    id: &str,
    headers: &HeaderMap,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
    method: &str,
) -> LocationMutation<LocationResponse> {
    locations::authorize(tenant)
        .await
        .map_err(response::operation)?;
    let AxumJson(body) =
        body.map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?;
    let path = format!("/api/v1/households/{household_id}/locations/{id}");
    let key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty());
    if let Some(key) = key
        && let Some((status, body, etag, replay)) =
            keyed_replay(tenant, key, method, &path, &body).await?
    {
        return Ok((status, MutationBody::Replay(body), etag, replay));
    }
    let reply = if !doses::valid_identifier(id) {
        (
            StatusCode::BAD_REQUEST,
            MutationBody::Error(
                response::Failure::bad_request("Invalid resource ID")
                    .into_body(&tenant.scope().request_id),
            ),
            None,
        )
    } else {
        let record = locations::read(tenant, id)
            .await
            .map_err(response::operation)?;
        let etag = headers
            .get("if-match")
            .and_then(|value| value.to_str().ok())
            .filter(|value| !value.is_empty());
        if let Some(etag) = etag {
            if locations::representation(&record).1 != etag {
                return Err(response::operation(OperationError::Conflict {
                    code: "conflict".into(),
                    details: json!({"message":"Record has changed since it was last read"}),
                }));
            }
            let savepoint = tenant
                .transaction()
                .begin()
                .await
                .map_err(|_| response::unavailable())?;
            let result = if method == "DELETE" {
                locations::retire(tenant, id, etag, Some(provenance))
                    .await
                    .map(|()| (StatusCode::NO_CONTENT, MutationBody::Empty, None))
            } else {
                let attributes = body
                    .as_object()
                    .filter(|outer| outer.len() == 1)
                    .and_then(|outer| outer.get("location"))
                    .cloned()
                    .unwrap_or(Value::Null);
                locations::update(
                    tenant,
                    id,
                    locations::LocationInput::from_json(attributes),
                    etag,
                    Some(provenance),
                )
                .await
                .map(|record| {
                    let (body, etag) = locations::representation(&record);
                    (StatusCode::OK, MutationBody::Resource(body), Some(etag))
                })
            };
            match result {
                Ok(reply) => {
                    savepoint
                        .commit()
                        .await
                        .map_err(|_| response::unavailable())?;
                    reply
                }
                Err(OperationError::Validation { details }) => {
                    savepoint
                        .rollback()
                        .await
                        .map_err(|_| response::unavailable())?;
                    (
                        StatusCode::UNPROCESSABLE_ENTITY,
                        MutationBody::Error(
                            response::Failure::validation_fields(
                                details
                                    .get("error")
                                    .and_then(Value::as_str)
                                    .unwrap_or("Validation failed"),
                                details["errors"].clone(),
                            )
                            .into_body(&tenant.scope().request_id),
                        ),
                        None,
                    )
                }
                Err(error) => {
                    savepoint
                        .rollback()
                        .await
                        .map_err(|_| response::unavailable())?;
                    return Err(response::operation(error));
                }
            }
        } else {
            (
                StatusCode::PRECONDITION_REQUIRED,
                MutationBody::Error(
                    response::Failure::precondition_required("If-Match is required")
                        .into_body(&tenant.scope().request_id),
                ),
                None,
            )
        }
    };
    if let Some(key) = key {
        locations::store(
            tenant,
            provenance,
            SavedResponse {
                key,
                method,
                path: &path,
                request: &body,
                status: reply.0.as_u16(),
                body: serde_json::to_value(&reply.1).map_err(|_| response::unavailable())?,
                etag: reply.2.as_deref(),
            },
        )
        .await
        .map_err(response::operation)?;
    }
    Ok((reply.0, reply.1, reply.2, false))
}
