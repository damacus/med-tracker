use super::*;
use crate::models::access::PersonAccess;
use crate::models::care::locations::SavedResponse;
use crate::models::profile::{self, Changes, Snapshot};
use axum::http::{HeaderValue, header};

fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub(super) fn representation(snapshot: &Snapshot) -> Value {
    json!({"data": {
        "person_id": snapshot.person_id.to_string(),
        "account_id": snapshot.account_id.to_string(),
        "date_of_birth": snapshot.date_of_birth,
        "time_zone": snapshot.time_zone,
        "gravatar_enabled": snapshot.gravatar_enabled,
        "mobile_shortcuts": snapshot.mobile_shortcuts,
        "avatar_attached": snapshot.avatar_attached,
    }})
}

fn parse(body: Value) -> std::result::Result<Changes, response::Failure> {
    let outer = body
        .as_object()
        .ok_or_else(|| response::Failure::bad_request("Invalid profile body"))?;
    let inner = outer
        .get("profile")
        .and_then(Value::as_object)
        .ok_or_else(|| response::Failure::bad_request("Invalid profile body"))?;
    if outer.len() != 1
        || inner.keys().any(|field| {
            !matches!(
                field.as_str(),
                "date_of_birth" | "time_zone" | "gravatar_enabled" | "mobile_shortcuts"
            )
        })
    {
        return Err(response::Failure::validation(
            "Profile contains an unsupported field",
        ));
    }
    let date_of_birth = match inner.get("date_of_birth") {
        None => None,
        Some(Value::String(value)) => {
            Some(profile::parse_date(value).map_err(response::operation)?)
        }
        _ => return Err(response::Failure::field("date_of_birth", "is invalid")),
    };
    let time_zone = match inner.get("time_zone") {
        None => None,
        Some(Value::String(value)) => Some(value.clone()),
        _ => return Err(response::Failure::field("time_zone", "is invalid")),
    };
    let gravatar_enabled = match inner.get("gravatar_enabled") {
        None => None,
        Some(Value::Bool(value)) => Some(*value),
        _ => return Err(response::Failure::field("gravatar_enabled", "is invalid")),
    };
    let mobile_shortcuts = match inner.get("mobile_shortcuts") {
        None => None,
        Some(Value::Array(values)) => Some(
            values
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| response::Failure::field("mobile_shortcuts", "is invalid"))
                })
                .collect::<std::result::Result<Vec<_>, _>>()?,
        ),
        _ => return Err(response::Failure::field("mobile_shortcuts", "is invalid")),
    };
    let changes = Changes {
        date_of_birth,
        time_zone,
        gravatar_enabled,
        mobile_shortcuts,
    };
    profile::validate(&changes).map_err(response::operation)?;
    Ok(changes)
}

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
    let reply = profile::read(&tenant, principal.account_id())
        .await
        .map(|snapshot| (StatusCode::OK, representation(&snapshot), None))
        .map_err(response::operation);
    no_store(
        finish(
            tenant,
            principal.provenance(),
            audit::RequestAudit::profile("GET", "show"),
            reply,
            &request_id,
        )
        .await,
    )
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
    let body = body
        .map(|AxumJson(value)| value)
        .map_err(|_| response::Failure::bad_request("Invalid JSON request body"));
    let path = format!("/api/v1/households/{household_id}/profile");
    let key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty());
    if let (Ok(body), Some(key)) = (&body, key) {
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
                let mut response = no_store(
                    finish(
                        tenant,
                        principal.provenance(),
                        audit::RequestAudit::profile(method, "update"),
                        Ok((status, saved, etag)),
                        &request_id,
                    )
                    .await,
                );
                if response.status().is_success() {
                    response
                        .headers_mut()
                        .insert("idempotency-replayed", HeaderValue::from_static("true"));
                }
                return response;
            }
            Err(error) => {
                return no_store(
                    finish(
                        tenant,
                        principal.provenance(),
                        audit::RequestAudit::profile(method, "update"),
                        Err(error),
                        &request_id,
                    )
                    .await,
                );
            }
            Ok(None) => {}
        }
    }
    let savepoint = match tenant.transaction().begin().await {
        Ok(value) => value,
        Err(_) => return response::error(response::unavailable(), &request_id),
    };
    let reply = async {
        let raw = body?;
        let changes = parse(raw.clone())?;
        let snapshot = profile::update(
            &tenant,
            principal.account_id(),
            changes,
            principal.time_zone(),
            Some(principal.provenance()),
        )
        .await
        .map_err(response::operation)?;
        let value = representation(&snapshot);
        if let Some(key) = key {
            crate::models::care::locations::store(
                &tenant,
                principal.provenance(),
                SavedResponse {
                    key,
                    method,
                    path: &path,
                    request: &raw,
                    status: 200,
                    body: value.clone(),
                    etag: None,
                },
            )
            .await
            .map_err(response::operation)?;
        }
        Ok::<_, response::Failure>((StatusCode::OK, value, None))
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
    no_store(
        finish(
            tenant,
            principal.provenance(),
            audit::RequestAudit::profile(method, "update"),
            reply,
            &request_id,
        )
        .await,
    )
}
