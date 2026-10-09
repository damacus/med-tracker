use super::*;
use crate::models::notification_preferences;
use crate::models::{access::PersonAccess, care::locations::SavedResponse, profile};
use axum::http::HeaderValue;

pub(super) async fn show(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let reply = notification_preferences::read(&tenant, principal.account_id())
        .await
        .map(|(row, owner)| {
            let (body, etag) = notification_preferences::representation(&row, &owner);
            (StatusCode::OK, body, Some(etag))
        })
        .map_err(response::operation);
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::notification_preference("GET", "show"),
        reply,
        &request_id,
    )
    .await
}

pub(super) async fn update(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    method: axum::http::Method,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) =
        match begin_profile_write(&ctx, &headers, household_id, &request_id).await {
            Ok(value) => value,
            Err(error) => return response::error(error, &request_id),
        };
    let method = if method == axum::http::Method::PUT {
        "PUT"
    } else {
        "PATCH"
    };
    let path = format!("/api/v1/households/{household_id}/notification_preference");
    let key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty());
    let raw = body
        .map(|AxumJson(value)| value)
        .map_err(|_| response::Failure::bad_request("Invalid JSON request body"));
    if let (Ok(body), Some(key)) = (&raw, key) {
        let authorized =
            profile::linked_person(&tenant, principal.account_id(), PersonAccess::Manage)
                .await
                .map_err(response::operation);
        let replay = match authorized {
            Ok(_) => super::locations::keyed_replay(&tenant, key, method, &path, body).await,
            Err(error) => Err(error),
        };
        match replay {
            Ok(Some((status, saved, etag, _))) => {
                let mut reply = finish(
                    tenant,
                    principal.provenance(),
                    audit::RequestAudit::notification_preference(method, "update"),
                    Ok((status, saved, etag)),
                    &request_id,
                )
                .await;
                if reply.status().is_success() {
                    reply
                        .headers_mut()
                        .insert("idempotency-replayed", HeaderValue::from_static("true"));
                }
                return reply;
            }
            Err(error) => {
                return finish(
                    tenant,
                    principal.provenance(),
                    audit::RequestAudit::notification_preference(method, "update"),
                    Err(error),
                    &request_id,
                )
                .await;
            }
            Ok(None) => {}
        }
    }
    let input = match &raw {
        Ok(body) => {
            if body
                .as_object()
                .is_some_and(|outer| !outer.contains_key("notification_preference"))
            {
                Err(response::Failure::bad_request(
                    "Invalid notification preference body",
                ))
            } else {
                notification_preferences::parse(body).map_err(response::operation)
            }
        }
        Err(_) => Err(response::Failure::bad_request("Invalid JSON request body")),
    };
    let savepoint = match tenant.transaction().begin().await {
        Ok(value) => value,
        Err(_) => return response::error(response::unavailable(), &request_id),
    };
    let reply = async {
        let changes = input?;
        let (row, owner) =
            notification_preferences::update(&tenant, principal.account_id(), changes)
                .await
                .map_err(response::operation)?;
        let (body, etag) = notification_preferences::representation(&row, &owner);
        if let (Some(key), Ok(raw)) = (key, &raw) {
            crate::models::care::locations::store(
                &tenant,
                principal.provenance(),
                SavedResponse {
                    key,
                    method,
                    path: &path,
                    request: raw,
                    status: 200,
                    body: body.clone(),
                    etag: Some(&etag),
                },
            )
            .await
            .map_err(response::operation)?;
        }
        Ok::<_, response::Failure>((StatusCode::OK, body, Some(etag)))
    }
    .await;
    let savepoint_result = if reply.is_ok() {
        savepoint.commit().await
    } else {
        savepoint.rollback().await
    };
    if savepoint_result.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::notification_preference(method, "update"),
        reply,
        &request_id,
    )
    .await
}
