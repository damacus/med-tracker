use super::*;

pub(super) async fn keyed_replay(
    db: &DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    action: &str,
    path: &str,
    body: &Value,
) -> Result<ReplayDecision, ApiError> {
    let Some(key) = mutation_idempotency::key(headers) else {
        return Ok(ReplayDecision::New);
    };
    let digest = mutation_idempotency::digest(method, path, body);
    match mutation_idempotency::lookup(db, context, key, method, path, &digest).await? {
        Lookup::New => Ok(ReplayDecision::New),
        Lookup::Replay(saved) => {
            let status = StatusCode::from_u16(saved.response_status as u16)
                .map_err(|_| ApiError::internal())?;
            let replay_id = Uuid::new_v4().to_string();
            audit::record_resource_request_with_id(
                db,
                context,
                &replay_id,
                method,
                CONTROLLER,
                POLICY,
                action,
                status,
                status.is_success(),
            )
            .await
            .map_err(database_error)?;
            let mut response = mutation_idempotency::replay(*saved)?;
            response.headers_mut().insert(
                "x-request-id",
                replay_id.parse().map_err(|_| ApiError::internal())?,
            );
            Ok(ReplayDecision::Replay(response))
        }
        Lookup::Conflict => Ok(ReplayDecision::Conflict),
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
    status: StatusCode,
    body: &Value,
    request_id: &str,
    etag: Option<&str>,
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
            status,
            body: body.clone(),
            request_id,
            etag,
        },
    )
    .await
}
