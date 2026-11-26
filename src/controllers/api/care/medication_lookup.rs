use super::*;
use crate::models::care::medication_lookup::{self, Search};
use crate::views::medication_lookup::{Lookup, LookupUnavailable};
use axum::extract::Query;

#[derive(serde::Serialize)]
#[serde(untagged)]
enum Reply {
    Found(Lookup),
    Unavailable(LookupUnavailable),
}

pub(super) async fn show(
    State(ctx): State<AppContext>,
    ApiPath(household_id): ApiPath<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    Query(search): Query<Search>,
) -> Response {
    let request_id = administration::request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let provider = ctx
        .shared_store
        .get::<medication_lookup::open_food_facts::ClientService>();
    let result = match medication_lookup::search(&tenant, &search, provider.as_ref()).await {
        Ok(found) => Ok((StatusCode::OK, Reply::Found(found.body), None)),
        Err(OperationError::Unavailable) => Ok((
            StatusCode::SERVICE_UNAVAILABLE,
            Reply::Unavailable(LookupUnavailable {
                results: [],
                error: "Medication search is temporarily unavailable.",
            }),
            None,
        )),
        Err(error) => Err(response::operation(error)),
    };
    finish_typed(
        tenant,
        principal.provenance(),
        audit::RequestAudit::medication_lookup(),
        result,
        &request_id,
    )
    .await
}
