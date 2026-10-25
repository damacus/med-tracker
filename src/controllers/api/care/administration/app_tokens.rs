use super::*;
use crate::models::care::locations::KeyedResponse;
use administration::app_tokens::{CreateInput, TextInput};

enum Mutation {
    Create(CreateInput),
    Revoke(i64),
}

pub(in crate::controllers::api::care) async fn index(
    State(ctx): State<AppContext>,
    ApiPath(household_id): ApiPath<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(context) => context,
        Err(error) => return response::error(error, &request_id),
    };
    let result = administration::app_tokens::index(&tenant)
        .await
        .map(|rows| (StatusCode::OK, json!({"data": rows}), None))
        .map_err(response::operation);
    finish(
        tenant,
        principal.provenance(),
        audit::RequestAudit::administration("GET", "app_tokens", "index"),
        result,
        &request_id,
    )
    .await
}

pub(in crate::controllers::api::care) async fn create(
    State(ctx): State<AppContext>,
    ApiPath(household_id): ApiPath<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    mutate(ctx, household_id, None, headers, request, Some(body)).await
}

pub(in crate::controllers::api::care) async fn destroy(
    State(ctx): State<AppContext>,
    ApiPath((household_id, id)): ApiPath<(i64, String)>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    mutate(ctx, household_id, Some(id), headers, request, None).await
}

fn path(household_id: i64, id: Option<&str>) -> String {
    match id {
        Some(id) => format!("/api/v1/households/{household_id}/admin/app_tokens/{id}"),
        None => format!("/api/v1/households/{household_id}/admin/app_tokens"),
    }
}

fn text_input(value: Option<&Value>) -> TextInput {
    match value {
        None => TextInput::Missing,
        Some(Value::String(value)) => TextInput::Text(value.clone()),
        Some(_) => TextInput::Invalid,
    }
}

fn attributes(body: &Value) -> std::result::Result<CreateInput, response::Failure> {
    let outer = body
        .as_object()
        .ok_or_else(|| response::Failure::bad_request("Invalid request body"))?;
    let inner = outer
        .get("api_app_token")
        .and_then(Value::as_object)
        .ok_or_else(|| response::Failure::bad_request("Invalid request body"))?;
    if outer.len() != 1
        || inner
            .keys()
            .any(|key| !matches!(key.as_str(), "name" | "expires_at"))
    {
        return Err(response::Failure::field(
            "api_app_token",
            "contains an unsupported field",
        ));
    }
    Ok(CreateInput {
        name: text_input(inner.get("name")),
        expires_at: text_input(inner.get("expires_at")),
    })
}

async fn mutate(
    ctx: AppContext,
    household_id: i64,
    id: Option<String>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: Option<std::result::Result<AxumJson<Value>, JsonRejection>>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(context) => context,
        Err(error) => return response::error(error, &request_id),
    };
    let result = process(
        &tenant,
        &principal,
        household_id,
        id.as_deref(),
        &headers,
        body,
    )
    .await;
    let mut response = complete(
        tenant,
        &principal,
        audit::RequestAudit::administration(
            if id.is_some() { "DELETE" } else { "POST" },
            "app_tokens",
            if id.is_some() { "destroy" } else { "create" },
        ),
        result,
        &request_id,
    )
    .await;
    if id.is_none() && response.status() == StatusCode::CREATED {
        response.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("no-store"),
        );
    }
    response
}

async fn process(
    tenant: &TenantTransaction,
    principal: &ValidatedPrincipal,
    household_id: i64,
    id: Option<&str>,
    headers: &HeaderMap,
    body: Option<std::result::Result<AxumJson<Value>, JsonRejection>>,
) -> std::result::Result<(StatusCode, Value, Option<String>, bool), response::Failure> {
    administration::authorize(tenant)
        .await
        .map_err(response::operation)?;
    let numeric_id = id.map(numeric_id).transpose()?;
    let body = match body {
        Some(body) => {
            body.map_err(|_| response::Failure::bad_request("Invalid request body"))?
                .0
        }
        None => Value::Null,
    };
    let method = if id.is_some() { "DELETE" } else { "POST" };
    let path = path(household_id, id);
    let key = key(headers);
    if let Some(key) = key {
        match locations::lookup(tenant, key, method, &path, &body)
            .await
            .map_err(response::operation)?
        {
            KeyedResponse::New => {}
            KeyedResponse::Conflict => {
                return Err(response::operation(OperationError::Conflict {
                    code: "idempotency_key_reused".into(),
                    details: json!({"message":"Idempotency key has already been used for a different request"}),
                }));
            }
            KeyedResponse::Replay(saved) => {
                if id.is_none() && saved.response_status == 201 {
                    return Ok((
                        StatusCode::CONFLICT,
                        json!({"error":{"code":"token_already_issued","message":"Token was already issued; create with a new idempotency key if its one-time secret was lost"}}),
                        None,
                        false,
                    ));
                }
                return super::super::locations::replay_response(tenant, saved);
            }
        }
    }
    let mutation = match numeric_id {
        Some(id) => Mutation::Revoke(id),
        None => Mutation::Create(attributes(&body)?),
    };
    let savepoint = tenant
        .transaction()
        .begin()
        .await
        .map_err(|_| response::unavailable())?;
    let (result, receipt) = match mutation {
        Mutation::Revoke(id) => (
            administration::app_tokens::revoke(tenant, id, principal.provenance())
                .await
                .map(|()| Value::Null),
            None,
        ),
        Mutation::Create(input) => {
            match administration::app_tokens::create(tenant, input, principal.provenance()).await {
                Ok(issued) => {
                    let receipt = json!({"receipt":{"token_id":issued.summary.id}});
                    (Ok(json!({"data": issued})), Some(receipt))
                }
                Err(error) => (Err(error), None),
            }
        }
    };
    let (status, reply) = reply(
        savepoint,
        tenant,
        result,
        if id.is_some() {
            StatusCode::NO_CONTENT
        } else {
            StatusCode::CREATED
        },
    )
    .await?;
    if let Some(key) = key {
        let stored = receipt.unwrap_or_else(|| reply.clone());
        locations::store(
            tenant,
            principal.provenance(),
            SavedResponse {
                key,
                method,
                path: &path,
                request: &body,
                status: status.as_u16(),
                body: stored,
                etag: None,
            },
        )
        .await
        .map_err(response::operation)?;
        Ok((status, reply, None, false))
    } else {
        Ok((status, reply, None, false))
    }
}
