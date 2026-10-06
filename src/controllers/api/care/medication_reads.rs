use super::*;
use axum::extract::{Query, rejection::QueryRejection};
use chrono::{DateTime, NaiveDateTime};
use headers::HeaderMapExt;
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
    let result = match parameters(pagination) {
        Ok((page, per_page, updated_since)) => {
            medications::reading::list(&tenant, page, per_page, updated_since)
                .await
                .map(|body| (StatusCode::OK, body, None))
                .map_err(response::operation)
        }
        Err(error) => Err(error),
    };
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::medication("GET", "index"),
        result,
        &request_id,
    )
    .await
}

pub(super) async fn show(
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
    let result = medications::read_stock_snapshot(&tenant, &id)
        .await
        .map(|snapshot| {
            let unchanged = headers
                .typed_get::<headers::IfNoneMatch>()
                .zip(snapshot.etag.parse::<headers::ETag>().ok())
                .is_some_and(|(condition, etag)| !condition.precondition_passes(&etag));
            (
                if unchanged {
                    StatusCode::NOT_MODIFIED
                } else {
                    StatusCode::OK
                },
                if unchanged {
                    Value::Null
                } else {
                    snapshot.representation
                },
                Some(snapshot.etag),
            )
        })
        .map_err(response::operation);
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::medication("GET", "show"),
        result,
        &request_id,
    )
    .await
}

fn parameters(
    pagination: std::result::Result<Query<Pagination>, QueryRejection>,
) -> std::result::Result<(i64, i64, Option<NaiveDateTime>), response::Failure> {
    let Query(pagination) = pagination.map_err(|_| {
        response::Failure::validation(
            "page must be positive and per_page must be between 1 and 100",
        )
    })?;
    let page = pagination.page.unwrap_or(1);
    let per_page = pagination.per_page.unwrap_or(20);
    if page < 1 || !(1..=100).contains(&per_page) {
        return Err(response::Failure::validation(
            "page must be positive and per_page must be between 1 and 100",
        ));
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
