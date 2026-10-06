use super::*;
use crate::models::care::dose_history;
use axum::extract::{Query, rejection::QueryRejection};
use chrono::{DateTime, NaiveDateTime};
use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct Pagination {
    page: Option<i64>,
    per_page: Option<i64>,
    updated_since: Option<String>,
}

pub(super) async fn index(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    pagination: std::result::Result<Query<Pagination>, QueryRejection>,
) -> Response {
    let request_id = administration::request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = async {
        let (page, per_page, updated_since) = parameters(pagination)?;
        let history = dose_history::list(&tenant, page, per_page, updated_since)
            .await
            .map_err(response::operation)?;
        let data = projection::serialize(tenant.transaction(), &history.records, household_id)
            .await
            .map_err(response::operation)?;
        Ok((
            StatusCode::OK,
            json!({"data":data,"meta":{"page":page,"per_page":per_page,"total_count":history.total_count}}),
            None,
        ))
    }
    .await;
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::take_history(),
        result,
        &request_id,
    )
    .await
}

fn parameters(
    pagination: std::result::Result<Query<Pagination>, QueryRejection>,
) -> std::result::Result<(i64, i64, Option<NaiveDateTime>), response::Failure> {
    let Query(pagination) =
        pagination.map_err(|_| response::Failure::validation("Invalid pagination"))?;
    let page = pagination.page.unwrap_or(1);
    let per_page = pagination.per_page.unwrap_or(20);
    if page < 1 || !(1..=100).contains(&per_page) {
        return Err(response::Failure::validation("Invalid pagination"));
    }
    let updated_since = pagination
        .updated_since
        .map(|timestamp| {
            DateTime::parse_from_rfc3339(&timestamp)
                .map(|value| value.naive_utc())
                .map_err(|_| response::Failure::validation("updated_since must be ISO8601"))
        })
        .transpose()?;
    Ok((page, per_page, updated_since))
}
