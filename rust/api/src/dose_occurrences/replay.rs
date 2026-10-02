use super::*;

pub(super) enum Replay {
    New,
    Saved(Response),
    Conflict,
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn keyed_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    kind: Kind,
    method: &str,
    action: &str,
    path: &str,
    body: &Value,
) -> Result<Replay, ApiError> {
    let Some(key) = mutation_idempotency::key(headers) else {
        return Ok(Replay::New);
    };
    let digest = mutation_idempotency::digest(method, path, body);
    match mutation_idempotency::lookup(db, context, key, method, path, &digest).await? {
        Lookup::New => Ok(Replay::New),
        Lookup::Conflict => Ok(Replay::Conflict),
        Lookup::Replay(saved) => {
            let request_id = Uuid::new_v4().to_string();
            let status = StatusCode::from_u16(saved.response_status as u16)
                .map_err(|_| ApiError::internal())?;
            audit::record_resource_request_with_id(
                db,
                context,
                &request_id,
                method,
                kind.controller(),
                kind.policy(),
                action,
                status,
                status.is_success(),
            )
            .await
            .map_err(database_error)?;
            let mut response = mutation_idempotency::replay(*saved)?;
            response.headers_mut().insert(
                "x-request-id",
                request_id.parse().map_err(|_| ApiError::internal())?,
            );
            Ok(Replay::Saved(response))
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn store_key(
    db: &DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    path: &str,
    request: &Value,
    body: &Value,
    request_id: &str,
    etag: &str,
) -> Result<(), ApiError> {
    let Some(key) = mutation_idempotency::key(headers) else {
        return Ok(());
    };
    let digest = mutation_idempotency::digest(method, path, request);
    mutation_idempotency::store(
        db,
        context,
        StoredResponse {
            key,
            method,
            path,
            digest: &digest,
            status: StatusCode::OK,
            body: body.clone(),
            request_id,
            etag: Some(etag),
        },
    )
    .await
}
