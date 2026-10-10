use super::*;
use crate::models::care::{
    locations::{self, SavedResponse},
    people,
};
use headers::HeaderMapExt;

pub(super) async fn index(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    page: std::result::Result<
        axum::extract::Query<people::Pagination>,
        axum::extract::rejection::QueryRejection,
    >,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(context) => context,
        Err(error) => return response::error(error, &request_id),
    };
    let reply = match page {
        Ok(axum::extract::Query(page)) => people::list(&tenant, page, principal.time_zone())
            .await
            .map(|body| (StatusCode::OK, body, None))
            .map_err(response::operation),
        Err(_) => Err(response::Failure::validation("Invalid pagination")),
    };
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::person("GET", "index"),
        reply,
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
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(context) => context,
        Err(error) => return response::error(error, &request_id),
    };
    let reply = people::read(&tenant, &id, principal.time_zone())
        .await
        .map(|(body, etag)| {
            let unchanged = headers
                .typed_get::<headers::IfNoneMatch>()
                .zip(etag.parse::<headers::ETag>().ok())
                .is_some_and(|(condition, etag)| !condition.precondition_passes(&etag));
            if unchanged {
                (StatusCode::NOT_MODIFIED, Value::Null, Some(etag))
            } else {
                (StatusCode::OK, body, Some(etag))
            }
        })
        .map_err(response::operation);
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::person("GET", "show"),
        reply,
        &request_id,
    )
    .await
}

pub(super) async fn create(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    mutate(ctx, household_id, None, headers, request, body, "POST").await
}
pub(super) async fn update(
    State(ctx): State<AppContext>,
    Path((household_id, id)): Path<(i64, String)>,
    method: axum::http::Method,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    mutate(
        ctx,
        household_id,
        Some(id),
        headers,
        request,
        body,
        if method == axum::http::Method::PUT {
            "PUT"
        } else {
            "PATCH"
        },
    )
    .await
}

async fn mutate(
    ctx: AppContext,
    household_id: i64,
    id: Option<String>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
    method: &'static str,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(context) => context,
        Err(error) => return response::error(error, &request_id),
    };
    let result = process(
        age_reference_date(&ctx),
        &tenant,
        &principal,
        id.as_deref(),
        &headers,
        body,
        method,
    )
    .await;
    let replay = result.as_ref().is_ok_and(|(_, _, _, replay)| *replay);
    let mut response = finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::person(method, if id.is_some() { "update" } else { "create" }),
        result.map(|(status, body, etag, _)| (status, body, etag)),
        &request_id,
    )
    .await;
    if replay && response.status().as_u16() < 500 && response.status() != StatusCode::CONFLICT {
        response.headers_mut().insert(
            "idempotency-replayed",
            axum::http::HeaderValue::from_static("true"),
        );
    }
    response
}

async fn process(
    today: chrono::NaiveDate,
    tenant: &TenantTransaction,
    principal: &ValidatedPrincipal,
    id: Option<&str>,
    headers: &HeaderMap,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
    method: &str,
) -> std::result::Result<(StatusCode, Value, Option<String>, bool), response::Failure> {
    if let Some(id) = id {
        if !doses::valid_identifier(id) {
            return Err(response::Failure::bad_request("Invalid resource ID"));
        }
        people::authorize_update(tenant, id)
            .await
            .map_err(response::operation)?;
    } else {
        people::authorize_create(tenant)
            .await
            .map_err(response::operation)?;
    }
    let AxumJson(body) =
        body.map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?;
    let fields = body
        .get("person")
        .and_then(Value::as_object)
        .ok_or_else(|| response::Failure::bad_request("Invalid request body"))?;
    let attributes: serde_json::Map<String, Value> = fields
        .iter()
        .filter(|(name, _)| {
            matches!(
                name.as_str(),
                "name" | "date_of_birth" | "email" | "person_type" | "has_capacity"
            )
        })
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    if attributes.is_empty() {
        return Err(response::Failure::bad_request("Invalid request body"));
    }
    let attributes = Value::Object(attributes);
    let household_id = tenant.scope().household_id;
    let path = match id {
        Some(id) => format!("/api/v1/households/{household_id}/people/{id}"),
        None => format!("/api/v1/households/{household_id}/people"),
    };
    let key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty());
    if let Some(key) = key
        && let Some(reply) =
            super::locations::keyed_replay(tenant, key, method, &path, &body).await?
    {
        return Ok(reply);
    }
    let savepoint = tenant
        .transaction()
        .begin()
        .await
        .map_err(|_| response::unavailable())?;
    let result = match id {
        Some(id) => {
            people::update(tenant, id, attributes, today, Some(principal.provenance())).await
        }
        None => people::create(tenant, attributes, today, Some(principal.provenance())).await,
    };
    let reply = match result {
        Ok(record) => {
            let (body, etag) = people::representation(tenant, &record, principal.time_zone())
                .await
                .map_err(response::operation)?;
            savepoint
                .commit()
                .await
                .map_err(|_| response::unavailable())?;
            (
                if id.is_some() {
                    StatusCode::OK
                } else {
                    StatusCode::CREATED
                },
                body,
                Some(etag),
            )
        }
        Err(OperationError::Validation { details }) => {
            savepoint
                .rollback()
                .await
                .map_err(|_| response::unavailable())?;
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({"error":{"code":"validation_failed","message":"Validation failed","request_id":tenant.scope().request_id,"errors":details["errors"]}}),
                None,
            )
        }
        Err(error) => {
            savepoint
                .rollback()
                .await
                .map_err(|_| response::unavailable())?;
            return Err(response::operation(error));
        }
    };
    if let Some(key) = key {
        locations::store(
            tenant,
            principal.provenance(),
            SavedResponse {
                key,
                method,
                path: &path,
                request: &body,
                status: reply.0.as_u16(),
                body: reply.1.clone(),
                etag: reply.2.as_deref(),
            },
        )
        .await
        .map_err(response::operation)?;
    }
    Ok((reply.0, reply.1, reply.2, false))
}
