use super::*;
mod lifecycle;
use crate::models::care::locations::{self, KeyedResponse, SavedResponse};
pub(super) use lifecycle::{destroy, index, show, update};

pub(super) async fn keyed_replay(
    tenant: &TenantTransaction,
    key: &str,
    method: &str,
    path: &str,
    request: &Value,
) -> std::result::Result<Option<(StatusCode, Value, Option<String>, bool)>, response::Failure> {
    match locations::lookup(tenant, key, method, path, request)
        .await
        .map_err(response::operation)?
    {
        KeyedResponse::New => Ok(None),
        KeyedResponse::Conflict => Err(response::operation(OperationError::Conflict {
            code: "idempotency_key_reused".into(),
            details: json!({"message":"Idempotency key has already been used for a different request"}),
        })),
        KeyedResponse::Replay(saved) => {
            let status = StatusCode::from_u16(saved.response_status as u16)
                .map_err(|_| response::unavailable())?;
            let etag = saved
                .response_headers
                .get("ETag")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let mut body = saved.response_body;
            if let Some(error) = body.get_mut("error").and_then(Value::as_object_mut) {
                error.insert("request_id".into(), json!(tenant.scope().request_id));
            }
            Ok(Some((status, body, etag, true)))
        }
    }
}

pub(super) async fn create(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
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
    let result = process(
        &tenant,
        principal.provenance(),
        household_id,
        &headers,
        body,
    )
    .await;
    let replay = result.as_ref().is_ok_and(|(_, _, _, replay)| *replay);
    let reply = result.map(|(status, body, etag, _)| (status, body, etag));
    let mut response = finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::location_create(),
        reply,
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

async fn process(
    tenant: &TenantTransaction,
    provenance: &CredentialProvenance,
    household_id: i64,
    headers: &HeaderMap,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> std::result::Result<(StatusCode, Value, Option<String>, bool), response::Failure> {
    locations::authorize(tenant)
        .await
        .map_err(response::operation)?;
    let AxumJson(body) =
        body.map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?;
    let path = format!("/api/v1/households/{household_id}/locations");
    let key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty());
    if let Some(key) = key
        && let Some(reply) = keyed_replay(tenant, key, "POST", &path, &body).await?
    {
        return Ok(reply);
    }
    let attributes = body
        .as_object()
        .filter(|outer| outer.len() == 1)
        .and_then(|outer| outer.get("location"))
        .cloned()
        .unwrap_or(Value::Null);
    let savepoint = tenant
        .transaction()
        .begin()
        .await
        .map_err(|_| response::unavailable())?;
    let created = locations::create(tenant, attributes, Some(provenance)).await;
    let (status, representation, etag) = match created {
        Ok(record) => {
            savepoint
                .commit()
                .await
                .map_err(|_| response::unavailable())?;
            let (body, etag) = locations::representation(&record);
            (StatusCode::CREATED, body, Some(etag))
        }
        Err(OperationError::Validation { details }) => {
            savepoint
                .rollback()
                .await
                .map_err(|_| response::unavailable())?;
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({"error":{"code":"validation_failed","message":"Validation failed","request_id":tenant.scope().request_id,"errors":details["errors"]}}),
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
                body: representation.clone(),
                etag: etag.as_deref(),
            },
        )
        .await
        .map_err(response::operation)?;
    }
    Ok((status, representation, etag, false))
}
