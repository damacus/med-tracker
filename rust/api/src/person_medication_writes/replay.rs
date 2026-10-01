use super::*;

pub(super) async fn replay_or_conflict(
    db: &DatabaseTransaction,
    context: &AuthContext,
    headers: &HeaderMap,
    method: &str,
    action: &str,
    request_path: &str,
    request_digest: &str,
) -> Result<Option<Response>, ApiError> {
    let Some(key) = mutation_idempotency::key(headers) else {
        return Ok(None);
    };
    match mutation_idempotency::lookup(db, context, key, method, request_path, request_digest)
        .await?
    {
        Lookup::New => Ok(None),
        Lookup::Conflict => {
            let request_id = Uuid::new_v4().to_string();
            audit::record_resource_request_with_id(
                db,
                context,
                &request_id,
                method,
                CONTROLLER,
                POLICY,
                action,
                StatusCode::CONFLICT,
                false,
            )
            .await
            .map_err(database_error)?;
            let mut response = (
                StatusCode::CONFLICT,
                Json(json!({"error": {"code": "idempotency_key_reused", "message": "Idempotency key has already been used for a different request", "request_id": request_id}})),
            )
                .into_response();
            response.headers_mut().insert(
                "x-request-id",
                HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
            );
            Ok(Some(response))
        }
        Lookup::Replay(saved) => {
            let request_id = Uuid::new_v4().to_string();
            let status = StatusCode::from_u16(saved.response_status as u16)
                .map_err(|_| ApiError::internal())?;
            audit::record_resource_request_with_id(
                db,
                context,
                &request_id,
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
                HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
            );
            Ok(Some(response))
        }
    }
}
