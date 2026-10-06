use super::*;
use axum::extract::{Query, rejection::QueryRejection};
use medications::stock_removals::{self, RemoveStock};
use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct Pagination {
    page: Option<i64>,
    per_page: Option<i64>,
}

pub(super) async fn create(
    State(ctx): State<AppContext>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(context) => context,
        Err(error) => return response::error(error, &request_id),
    };
    let savepoint = match tenant.transaction().begin().await {
        Ok(savepoint) => savepoint,
        Err(_) => return response::error(response::unavailable(), &request_id),
    };
    let input = match stock_removals::authorize(&tenant, &id).await {
        Ok(_) => match body {
            Ok(AxumJson(body)) => parse(body, id),
            Err(_) => Err(response::Failure::bad_request("Invalid request body")),
        },
        Err(error) => Err(response::operation(error)),
    };
    let result = match input {
        Ok(input) => stock_removals::create(&tenant, input, Some(principal.provenance()))
            .await
            .map(|record| (StatusCode::CREATED, json!({"data":record}), None))
            .map_err(response::operation),
        Err(error) => Err(error),
    };
    let completed = if result.is_ok() {
        savepoint.commit().await
    } else {
        savepoint.rollback().await
    };
    if completed.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::removal(false),
        result,
        &request_id,
    )
    .await
}

pub(super) async fn history(
    State(ctx): State<AppContext>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    pagination: std::result::Result<Query<Pagination>, QueryRejection>,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(context) => context,
        Err(error) => return response::error(error, &request_id),
    };
    let result = match stock_removals::authorize(&tenant, &id).await {
        Ok(_) => match pagination {
            Ok(Query(page)) => stock_removals::history(
                &tenant,
                &id,
                page.page.unwrap_or(1),
                page.per_page.unwrap_or(20),
            )
            .await
            .map(|body| (StatusCode::OK, body, None))
            .map_err(response::operation),
            Err(_) => Err(response::Failure::validation(
                "Invalid pagination parameters",
            )),
        },
        Err(error) => Err(response::operation(error)),
    };
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::removal(true),
        result,
        &request_id,
    )
    .await
}

fn parse(
    body: Value,
    medication_id: String,
) -> std::result::Result<RemoveStock, response::Failure> {
    let outer = body
        .as_object()
        .ok_or_else(|| response::Failure::bad_request("Invalid request body"))?;
    let attributes = outer
        .get("stock_removal")
        .and_then(Value::as_object)
        .ok_or_else(|| response::Failure::bad_request("Invalid request body"))?;
    let invalid = || response::Failure::validation("Stock removal could not be recorded");
    if outer.len() != 1
        || attributes.keys().any(|key| {
            !matches!(
                key.as_str(),
                "quantity" | "reason" | "note" | "dosage_id" | "submission_id"
            )
        })
    {
        return Err(invalid());
    }
    let required = |key: &str| {
        attributes
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(invalid)
    };
    let optional = |key: &str| {
        attributes
            .get(key)
            .map(|value| value.as_str().map(str::to_owned).ok_or_else(invalid))
            .transpose()
    };
    Ok(RemoveStock {
        medication_id,
        quantity: required("quantity")?,
        reason: required("reason")?,
        note: optional("note")?,
        dosage_id: optional("dosage_id")?,
        submission_id: required("submission_id")?,
    })
}
