use super::administration::{complete, key, request_id, store};
use super::*;
use crate::models::care::{locations::SavedResponse, treatments};

pub(super) async fn create(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let savepoint = match tenant.transaction().begin().await {
        Ok(savepoint) => savepoint,
        Err(_) => return response::error(response::unavailable(), &request_id),
    };
    let result = async {
        let AxumJson(body) =
            body.map_err(|_| response::Failure::bad_request("Invalid JSON request body"))?;
        treatments::authorize_person(&tenant, &body)
            .await
            .map_err(response::operation)?;
        let path = format!("/api/v1/households/{household_id}/schedules");
        let key = key(&headers);
        if let Some(key) = key
            && let Some(reply) = locations::keyed_replay(&tenant, key, "POST", &path, &body).await?
        {
            return Ok(reply);
        }
        let (reply, etag) = treatments::create(&tenant, &body, Some(principal.provenance()))
            .await
            .map_err(response::operation)?;
        if let Some(key) = key {
            return store(
                &tenant,
                &principal,
                SavedResponse {
                    key,
                    method: "POST",
                    path: &path,
                    request: &body,
                    status: 201,
                    body: reply,
                    etag: Some(&etag),
                },
            )
            .await;
        }
        Ok((StatusCode::CREATED, reply, Some(etag), false))
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
        audit::RequestAudit::treatment("POST", "create"),
        result,
        &request_id,
    )
    .await
}
