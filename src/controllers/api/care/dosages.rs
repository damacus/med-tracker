use super::administration::request_id;
use super::*;
use crate::models::care::dosages;
use headers::HeaderMapExt;

pub(super) async fn index(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    pagination: std::result::Result<
        axum::extract::Query<dosages::Pagination>,
        axum::extract::rejection::QueryRejection,
    >,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = match pagination {
        Ok(axum::extract::Query(page)) => dosages::list(&tenant, page)
            .await
            .map(|body| (StatusCode::OK, body, None))
            .map_err(response::operation),
        Err(_) => Err(response::Failure::validation("Invalid pagination")),
    };
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::dosage("GET", "index"),
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
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = dosages::read(&tenant, &id)
        .await
        .map(|(body, etag)| {
            let unchanged = headers
                .typed_get::<headers::IfNoneMatch>()
                .zip(etag.parse::<headers::ETag>().ok())
                .is_some_and(|(condition, etag)| !condition.precondition_passes(&etag));
            (
                if unchanged {
                    StatusCode::NOT_MODIFIED
                } else {
                    StatusCode::OK
                },
                if unchanged { Value::Null } else { body },
                Some(etag),
            )
        })
        .map_err(response::operation);
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::dosage("GET", "show"),
        result,
        &request_id,
    )
    .await
}

pub(super) async fn create(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<axum::Json<Value>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = async {
        if !dosages::can_manage(&tenant)
            .await
            .map_err(response::operation)?
        {
            return Err(response::operation(OperationError::Forbidden));
        }
        let body = body
            .map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?
            .0;
        dosages::create(&tenant, body, Some(principal.provenance()))
            .await
            .map(|(body, etag)| (StatusCode::CREATED, body, Some(etag)))
            .map_err(response::operation)
    }
    .await;
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::dosage("POST", "create"),
        result,
        &request_id,
    )
    .await
}

pub(super) async fn patch(
    State(ctx): State<AppContext>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<axum::Json<Value>, axum::extract::rejection::JsonRejection>,
) -> Response {
    update(ctx, household_id, id, headers, request, body, "PATCH").await
}
pub(super) async fn put(
    State(ctx): State<AppContext>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<axum::Json<Value>, axum::extract::rejection::JsonRejection>,
) -> Response {
    update(ctx, household_id, id, headers, request, body, "PUT").await
}

async fn update(
    ctx: AppContext,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<axum::Json<Value>, axum::extract::rejection::JsonRejection>,
    method: &'static str,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = async {
        if !dosages::can_manage(&tenant)
            .await
            .map_err(response::operation)?
        {
            return Err(response::operation(OperationError::Forbidden));
        }
        if !valid_id(&id) {
            return Err(response::Failure::bad_request("Invalid resource ID"));
        }
        let body = body
            .map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?
            .0;
        let etag = headers
            .get(axum::http::header::IF_MATCH)
            .map(|value| value.to_str().unwrap_or(""));
        dosages::update(&tenant, &id, body, etag, Some(principal.provenance()))
            .await
            .map(|(body, etag)| (StatusCode::OK, body, Some(etag)))
            .map_err(response::operation)
    }
    .await;
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::dosage(method, "update"),
        result,
        &request_id,
    )
    .await
}

fn valid_id(id: &str) -> bool {
    id.parse::<i64>().is_ok_and(|value| value > 0) || uuid::Uuid::parse_str(id).is_ok()
}
