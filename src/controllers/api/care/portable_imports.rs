use super::*;
use crate::models::care::{self, portable_crypto::PortableCryptoError};

const PASSPHRASE_HEADER: &str = "x-medtracker-portable-passphrase";

fn passphrase(headers: &HeaderMap) -> Option<String> {
    headers
        .get(PASSPHRASE_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
}

fn crypto_message(error: PortableCryptoError) -> &'static str {
    match error {
        PortableCryptoError::TooLarge => "Portable bundle is too large",
        PortableCryptoError::InvalidEnvelope => "Portable bundle could not be decrypted",
        PortableCryptoError::Unavailable => "Portable decryption is temporarily unavailable",
    }
}

pub(super) async fn dry_run(
    ctx: State<AppContext>,
    household: Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    handle(ctx, household, headers, request, body, true).await
}

pub(super) async fn apply(
    ctx: State<AppContext>,
    household: Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
) -> Response {
    handle(ctx, household, headers, request, body, false).await
}

async fn handle(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    body: std::result::Result<AxumJson<Value>, JsonRejection>,
    is_dry_run: bool,
) -> Response {
    let action = if is_dry_run { "dry_run" } else { "create" };
    let audit = || audit::RequestAudit::portable_import(action);
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let object = match body {
        Ok(AxumJson(body)) => match body {
            Value::Object(object) => object,
            _ => {
                return finish(
                    tenant,
                    principal.provenance(),
                    audit(),
                    Err(response::Failure::bad_request(
                        "Portable bundle is required",
                    )),
                    &request_id,
                )
                .await;
            }
        },
        Err(rejection) => {
            let failure = if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
                response::Failure::validation("Portable bundle is too large")
            } else {
                response::Failure::bad_request("Invalid JSON request body")
            };
            return finish(
                tenant,
                principal.provenance(),
                audit(),
                Err(failure),
                &request_id,
            )
            .await;
        }
    };
    let Some(bundle) = object.get("bundle") else {
        return finish(
            tenant,
            principal.provenance(),
            audit(),
            Err(response::Failure::bad_request(
                "Portable bundle is required",
            )),
            &request_id,
        )
        .await;
    };
    if object
        .keys()
        .any(|key| !matches!(key.as_str(), "bundle" | "initial_guardian_declarations"))
    {
        return finish(
            tenant,
            principal.provenance(),
            audit(),
            Err(response::Failure::validation(
                "Unsupported import request fields",
            )),
            &request_id,
        )
        .await;
    }
    let initial_declarations = match care::portable_imports::InitialDeclarations::parse(
        object.get("initial_guardian_declarations"),
    ) {
        Ok(value) => value,
        Err(error) => {
            return finish(
                tenant,
                principal.provenance(),
                audit(),
                Err(response::operation(error)),
                &request_id,
            )
            .await;
        }
    };
    let Some(passphrase) = passphrase(&headers) else {
        return finish(
            tenant,
            principal.provenance(),
            audit(),
            Err(response::Failure::validation(
                "Portable passphrase header is required",
            )),
            &request_id,
        )
        .await;
    };
    if tenant.commit().await.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    let bundle = bundle.clone();
    let decrypted =
        tokio::task::spawn_blocking(move || care::portable_crypto::decrypt(&bundle, &passphrase))
            .await;
    let Ok(decrypted) = decrypted else {
        return response::error(response::unavailable(), &request_id);
    };
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let payload = match decrypted {
        Ok(payload) => payload,
        Err(error) => {
            return finish(
                tenant,
                principal.provenance(),
                audit(),
                Err(response::Failure::validation(crypto_message(error))),
                &request_id,
            )
            .await;
        }
    };
    let plan = match care::portable_imports::ImportPlan::parse(payload) {
        Ok(plan) => plan,
        Err(message) => {
            return finish(
                tenant,
                principal.provenance(),
                audit(),
                Err(response::Failure::validation(&message)),
                &request_id,
            )
            .await;
        }
    };
    if tenant.rollback().await.is_err() {
        return response::error(response::unavailable(), &request_id);
    }
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return response::error(error, &request_id),
    };
    let locked = async {
        care::guardian_declarations::lock_creator_account(&tenant).await?;
        care::sync::lock(&tenant).await
    }
    .await;
    if let Err(error) = locked {
        return finish(
            tenant,
            principal.provenance(),
            audit(),
            Err(response::operation(error)),
            &request_id,
        )
        .await;
    }
    let mut index =
        match care::portable_imports::ExistingIndex::load(tenant.transaction(), household_id).await
        {
            Ok(index) => index,
            Err(_) => {
                return finish(
                    tenant,
                    principal.provenance(),
                    audit(),
                    Err(response::unavailable()),
                    &request_id,
                )
                .await;
            }
        };
    let today = chrono::Utc::now()
        .with_timezone(&principal.time_zone())
        .date_naive();
    let mut declarations = match initial_declarations
        .prepare(&tenant, &plan, &index, today)
        .await
    {
        Ok(value) => value,
        Err(error) => {
            return finish(
                tenant,
                principal.provenance(),
                audit(),
                Err(response::operation(error)),
                &request_id,
            )
            .await;
        }
    };
    let allowed = match care::portable_imports::authorized(
        &tenant,
        &plan,
        &index,
        &declarations,
        today,
    )
    .await
    {
        Ok(allowed) => allowed,
        Err(error) => {
            return finish(
                tenant,
                principal.provenance(),
                audit(),
                Err(response::operation(error)),
                &request_id,
            )
            .await;
        }
    };
    if !allowed {
        return finish(
            tenant,
            principal.provenance(),
            audit(),
            Err(response::operation(OperationError::Forbidden)),
            &request_id,
        )
        .await;
    }
    let result =
        match care::portable_imports::preflight(tenant.transaction(), &tenant, &plan, &index).await
        {
            Ok(result) => result,
            Err(_) => {
                return finish(
                    tenant,
                    principal.provenance(),
                    audit(),
                    Err(response::unavailable()),
                    &request_id,
                )
                .await;
            }
        };
    if is_dry_run || !result.conflicts.is_empty() || !result.errors.is_empty() {
        let status = if is_dry_run {
            StatusCode::OK
        } else {
            StatusCode::UNPROCESSABLE_ENTITY
        };
        return finish(
            tenant,
            principal.provenance(),
            audit(),
            Ok((status, json!({"data": result.body(false)}), None)),
            &request_id,
        )
        .await;
    }
    if let Err(message) = care::portable_imports::write_records(
        tenant.transaction(),
        &tenant,
        principal.provenance(),
        &plan,
        &mut index,
        &request_id,
        &mut declarations,
    )
    .await
    {
        if tenant.rollback().await.is_err() {
            return response::error(response::unavailable(), &request_id);
        }
        let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
            Ok(value) => value,
            Err(error) => return response::error(error, &request_id),
        };
        return finish(
            tenant,
            principal.provenance(),
            audit(),
            Err(response::Failure::validation(&message)),
            &request_id,
        )
        .await;
    }
    let now = chrono::Utc::now().naive_utc();
    let event = security_audit_event::ActiveModel {
        household_id: Set(household_id),
        actor_account_id: Set(Some(tenant.scope().actor.account_id)),
        actor_membership_id: Set(Some(tenant.membership().id)),
        event_type: Set("portable_data.imported".to_owned()),
        request_id: Set(Some(request_id.clone())),
        ip: Set(None),
        metadata: Set(json!({"record_counts": plan.counts, "dry_run": false})),
        audit_context: Set(
            json!({"request_id": request_id, "actor_membership_id": tenant.membership().id}),
        ),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await;
    if event.is_err() {
        return finish(
            tenant,
            principal.provenance(),
            audit(),
            Err(response::unavailable()),
            &request_id,
        )
        .await;
    }
    finish(
        tenant,
        principal.provenance(),
        audit(),
        Ok((
            StatusCode::CREATED,
            json!({"data": result.body(true)}),
            None,
        )),
        &request_id,
    )
    .await
}
