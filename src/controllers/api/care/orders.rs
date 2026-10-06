use super::administration::{complete, key, request_id, store};
use super::*;
use crate::models::care::{locations::SavedResponse, medications, orders};

pub(super) async fn ordered(
    State(ctx): State<AppContext>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: axum::body::Bytes,
) -> Response {
    change(ctx, household_id, id, headers, request, body, true).await
}
pub(super) async fn received(
    State(ctx): State<AppContext>,
    Path((household_id, id)): Path<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: axum::body::Bytes,
) -> Response {
    change(ctx, household_id, id, headers, request, body, false).await
}
async fn change(
    ctx: AppContext,
    household_id: i64,
    id: String,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    bytes: axum::body::Bytes,
    ordered: bool,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let action = if ordered {
        "mark_as_ordered"
    } else {
        "mark_as_received"
    };
    let savepoint = match tenant.transaction().begin().await {
        Ok(savepoint) => savepoint,
        Err(_) => return response::error(response::unavailable(), &request_id),
    };
    let result = async {
        orders::lock_visible(&tenant, &id)
            .await
            .map_err(response::operation)?;
        let body = if bytes.is_empty() {
            json!({})
        } else {
            serde_json::from_slice::<Value>(&bytes)
                .map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?
        };
        let path = format!("/api/v1/households/{household_id}/medications/{id}/{action}");
        let key = key(&headers);
        if let Some(key) = key
            && let Some(reply) =
                locations::keyed_replay(&tenant, key, "PATCH", &path, &body).await?
        {
            return Ok(reply);
        }
        let result = if ordered {
            orders::mark_as_ordered(&tenant, &id, &body, Some(principal.provenance())).await
        } else {
            orders::mark_as_received(&tenant, &id, &body, Some(principal.provenance())).await
        };
        result.map_err(response::operation)?;
        let snapshot = medications::read_stock_snapshot(&tenant, &id)
            .await
            .map_err(response::operation)?;
        if let Some(key) = key {
            return store(
                &tenant,
                &principal,
                SavedResponse {
                    key,
                    method: "PATCH",
                    path: &path,
                    request: &body,
                    status: 200,
                    body: snapshot.representation,
                    etag: Some(&snapshot.etag),
                },
            )
            .await;
        }
        Ok((
            StatusCode::OK,
            snapshot.representation,
            Some(snapshot.etag),
            false,
        ))
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
        audit::RequestAudit::medication("PATCH", action),
        result,
        &request_id,
    )
    .await
}
