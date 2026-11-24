use super::*;
use crate::models::{
    care::medications::allocations::{self, AllocateStock, AllocationInput},
    ids::{HouseholdId, MedicationId},
};
use crate::views::DataResponse;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AllocationRequest {
    stock_allocation: AllocationInput,
}

pub(super) async fn create(
    State(ctx): State<AppContext>,
    ApiPath((household_id, medication_id)): ApiPath<(HouseholdId, MedicationId)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<AllocationRequest>, JsonRejection>,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id.get(), &request_id).await {
        Ok(context) => context,
        Err(error) => return response::error(error, &request_id),
    };
    let savepoint = match tenant.transaction().begin().await {
        Ok(savepoint) => savepoint,
        Err(_) => return response::error(response::unavailable(), &request_id),
    };
    let result = async {
        let AxumJson(body) =
            body.map_err(|_| response::Failure::bad_request("Invalid stock allocation request"))?;
        if medication_id.get() <= 0 {
            return Err(response::operation(OperationError::NotFound));
        }
        let command =
            AllocateStock::try_from(body.stock_allocation).map_err(response::operation)?;
        allocations::reserve(&tenant, &principal, medication_id, command)
            .await
            .map_err(response::operation)
    }
    .await;
    let replayed = result.as_ref().is_ok_and(|(_, replayed)| *replayed);
    if settle_savepoint(savepoint, result.is_ok()).await.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    let mut result = finish_typed(
        tenant,
        principal.provenance(),
        audit::RequestAudit::stock_allocation(),
        result.map(|(receipt, _)| (StatusCode::CREATED, DataResponse { data: receipt }, None)),
        &request_id,
    )
    .await;
    if replayed && result.status() == StatusCode::CREATED {
        result.headers_mut().insert(
            "idempotency-replayed",
            axum::http::HeaderValue::from_static("true"),
        );
    }
    result
}
