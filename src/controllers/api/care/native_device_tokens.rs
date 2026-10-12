use super::administration::complete;
use super::*;
use crate::models::care::native_device_tokens;

pub(super) async fn create(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    let request_id = administration::request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let savepoint = match tenant.transaction().begin().await {
        Ok(savepoint) => savepoint,
        Err(_) => return response::error(response::unavailable(), &request_id),
    };
    let result = async {
        let AxumJson(body) =
            body.map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?;
        if body.get("native_device_token").is_none() {
            return Err(response::Failure::bad_request(
                "native_device_token is required",
            ));
        }
        let user_agent = headers
            .get(axum::http::header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        native_device_tokens::register(&tenant, &body, user_agent)
            .await
            .map_err(|error| match error {
                OperationError::Validation { details } => {
                    response::Failure::fields(details["errors"].clone())
                }
                other => response::operation(other),
            })?;
        Ok((StatusCode::CREATED, json!({}), None, false))
    }
    .await;
    let restored = if result.is_ok() {
        savepoint.commit().await
    } else {
        savepoint.rollback().await
    };
    if restored.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    empty_success(
        complete(
            tenant,
            &principal,
            audit::RequestAudit::native_device_token("POST", "create"),
            result,
            &request_id,
        )
        .await,
    )
}

pub(super) async fn destroy(
    State(ctx): State<AppContext>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    let request_id = administration::request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let savepoint = match tenant.transaction().begin().await {
        Ok(savepoint) => savepoint,
        Err(_) => return response::error(response::unavailable(), &request_id),
    };
    let result = async {
        native_device_tokens::revoke(&tenant, &id)
            .await
            .map_err(response::operation)?;
        Ok((StatusCode::NO_CONTENT, json!({}), None, false))
    }
    .await;
    let restored = if result.is_ok() {
        savepoint.commit().await
    } else {
        savepoint.rollback().await
    };
    if restored.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    empty_success(
        complete(
            tenant,
            &principal,
            audit::RequestAudit::native_device_token("DELETE", "destroy"),
            result,
            &request_id,
        )
        .await,
    )
}

fn empty_success(mut response: Response) -> Response {
    if response.status().is_success() {
        *response.body_mut() = axum::body::Body::empty();
        response
            .headers_mut()
            .remove(axum::http::header::CONTENT_TYPE);
    }
    response
}
