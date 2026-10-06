use super::administration::{complete, key, numeric_id, request_id, store};
use super::*;
use crate::models::care::{invitations, locations::SavedResponse};
mod acceptance;
mod issuing;
mod lifecycle;
pub(super) use acceptance::accept;
pub(super) use issuing::{create, index};
pub(super) use lifecycle::{destroy, resend};

fn no_store(mut response: Response) -> Response {
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}

fn attributes(body: &Value) -> std::result::Result<Value, response::Failure> {
    let source = body
        .get("household_invitation")
        .and_then(Value::as_object)
        .ok_or_else(|| response::Failure::bad_request("Invalid request body"))?;
    let values: serde_json::Map<_, _> = source
        .iter()
        .filter(|(name, value)| {
            matches!(name.as_str(), "email" | "membership_role")
                && !value.is_array()
                && !value.is_object()
        })
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    if values.is_empty() {
        return Err(response::Failure::bad_request("Invalid request body"));
    }
    Ok(Value::Object(values))
}

async fn reply(
    savepoint: DatabaseTransaction,
    tenant: &TenantTransaction,
    result: Result<Value, OperationError>,
    success: StatusCode,
) -> std::result::Result<(StatusCode, Value), response::Failure> {
    match result {
        Ok(body) => {
            savepoint
                .commit()
                .await
                .map_err(|_| response::unavailable())?;
            Ok((success, body))
        }
        Err(OperationError::Validation { details }) => {
            savepoint
                .rollback()
                .await
                .map_err(|_| response::unavailable())?;
            Ok((
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({"error":{"code":"validation_failed","message":"Validation failed","request_id":tenant.scope().request_id,"errors":details["errors"]}}),
            ))
        }
        Err(error) => {
            savepoint
                .rollback()
                .await
                .map_err(|_| response::unavailable())?;
            Err(response::operation(error))
        }
    }
}
