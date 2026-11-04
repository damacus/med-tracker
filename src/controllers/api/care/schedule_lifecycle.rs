use super::administration::{complete, key, request_id, store};
use super::*;
use crate::models::care::{locations::SavedResponse, treatments::lifecycle};
use headers::HeaderMapExt;

pub(super) async fn index(
    State(ctx): State<AppContext>,
    ApiPath(household_id): ApiPath<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    page: std::result::Result<
        axum::extract::Query<lifecycle::Pagination>,
        axum::extract::rejection::QueryRejection,
    >,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = match page {
        Ok(axum::extract::Query(page)) => lifecycle::list(&tenant, page)
            .await
            .map(|body| (StatusCode::OK, body, None))
            .map_err(response::operation),
        Err(_) => Err(response::Failure::validation("Invalid pagination")),
    };
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::treatment("GET", "index"),
        result,
        &request_id,
    )
    .await
}
pub(super) async fn show(
    State(ctx): State<AppContext>,
    ApiPath((household_id, id)): ApiPath<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let result = lifecycle::read(&tenant, &id)
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
        audit::RequestAudit::treatment("GET", "show"),
        result,
        &request_id,
    )
    .await
}
pub(super) async fn update(
    State(ctx): State<AppContext>,
    ApiPath((household_id, id)): ApiPath<(i64, String)>,
    method: axum::http::Method,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    #[cfg(debug_assertions)]
    let probe_start = std::time::Instant::now();
    let request_id = request_id(request);
    let method = if method == axum::http::Method::PUT {
        "PUT"
    } else {
        "PATCH"
    };
    #[cfg(debug_assertions)]
    eprintln!(
        "timeout-probe edit-controller begin-start {:.3}",
        probe_start.elapsed().as_secs_f64() * 1000.0
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    #[cfg(debug_assertions)]
    eprintln!(
        "timeout-probe edit-controller begin-end {:.3}",
        probe_start.elapsed().as_secs_f64() * 1000.0
    );
    let savepoint = match tenant.transaction().begin().await {
        Ok(value) => value,
        Err(_) => return response::error(response::unavailable(), &request_id),
    };
    let result = async {
        #[cfg(debug_assertions)]
        eprintln!(
            "timeout-probe edit-controller first-authorize-start {:.3}",
            probe_start.elapsed().as_secs_f64() * 1000.0
        );
        lifecycle::authorize_update(&tenant, &id)
            .await
            .map_err(response::operation)?;
        #[cfg(debug_assertions)]
        eprintln!(
            "timeout-probe edit-controller first-authorize-end {:.3}",
            probe_start.elapsed().as_secs_f64() * 1000.0
        );
        let AxumJson(body) =
            body.map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?;
        let path = format!("/api/v1/households/{household_id}/schedules/{id}");
        let key = key(&headers);
        if let Some(key) = key
            && let Some(reply) = locations::keyed_replay(&tenant, key, method, &path, &body).await?
        {
            return Ok(reply);
        }
        let etag = headers
            .get(axum::http::header::IF_MATCH)
            .map(|value| value.to_str().unwrap_or(""));
        #[cfg(debug_assertions)]
        eprintln!(
            "timeout-probe edit-controller update-start {:.3}",
            probe_start.elapsed().as_secs_f64() * 1000.0
        );
        let (reply, etag) =
            lifecycle::update(&tenant, &id, &body, etag, Some(principal.provenance()))
                .await
                .map_err(response::operation)?;
        #[cfg(debug_assertions)]
        eprintln!(
            "timeout-probe edit-controller update-end {:.3}",
            probe_start.elapsed().as_secs_f64() * 1000.0
        );
        if let Some(key) = key {
            return store(
                &tenant,
                &principal,
                SavedResponse {
                    key,
                    method,
                    path: &path,
                    request: &body,
                    status: 200,
                    body: reply,
                    etag: Some(&etag),
                },
            )
            .await;
        }
        Ok((StatusCode::OK, reply, Some(etag), false))
    }
    .await;
    let restored = if result.is_ok() {
        savepoint.commit().await
    } else {
        savepoint.rollback().await
    };
    if restored.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    complete(
        tenant,
        &principal,
        audit::RequestAudit::treatment(method, "update"),
        result,
        &request_id,
    )
    .await
}
