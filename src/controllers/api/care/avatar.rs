use super::*;
use crate::models::{
    access::PersonAccess,
    profile::{self, avatar},
};
use axum::{
    body::Body,
    extract::{Multipart, multipart::MultipartRejection},
    http::{HeaderValue, header},
};

fn no_store(mut reply: Response) -> Response {
    reply
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    reply.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    reply
}

fn storage_error(request_id: &str) -> Response {
    no_store(response::error(
        response::Failure::avatar_unavailable(),
        request_id,
    ))
}

async fn uploaded(mut form: Multipart) -> std::result::Result<avatar::Image, response::Failure> {
    let field = form
        .next_field()
        .await
        .map_err(|_| response::Failure::field("avatar", "is invalid"))?
        .ok_or_else(|| response::Failure::field("avatar", "is required"))?;
    if field.name() != Some("avatar") {
        return Err(response::Failure::field("avatar", "is invalid"));
    }
    let content_type = field.content_type().unwrap_or_default().to_owned();
    let bytes = field
        .bytes()
        .await
        .map_err(|_| response::Failure::field("avatar", "is invalid"))?;
    if form
        .next_field()
        .await
        .map_err(|_| response::Failure::field("avatar", "is invalid"))?
        .is_some()
    {
        return Err(response::Failure::field("avatar", "is invalid"));
    }
    avatar::decode(&bytes, &content_type).map_err(response::operation)
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
        Err(error) => return no_store(response::error(error, &request_id)),
    };
    let attached =
        match avatar::attachment(&tenant, principal.account_id(), PersonAccess::View).await {
            Ok(Some(value)) => value,
            Ok(None) => {
                return no_store(
                    finish(
                        tenant,
                        principal.provenance(),
                        audit::RequestAudit::avatar("GET", "show"),
                        Err(response::operation(OperationError::NotFound)),
                        &request_id,
                    )
                    .await,
                );
            }
            Err(error) => {
                return no_store(
                    finish(
                        tenant,
                        principal.provenance(),
                        audit::RequestAudit::avatar("GET", "show"),
                        Err(response::operation(error)),
                        &request_id,
                    )
                    .await,
                );
            }
        };
    if tenant.commit().await.is_err() {
        return storage_error(&request_id);
    }
    let storage = match avatar::Storage::configured(&ctx) {
        Ok(value) => value,
        Err(_) => return storage_error(&request_id),
    };
    let bytes = match storage.get(&attached.key).await {
        Ok(value) => value,
        Err(_) => return storage_error(&request_id),
    };
    let image = match avatar::decode(&bytes, &attached.content_type) {
        Ok(value) => value,
        Err(_) => return storage_error(&request_id),
    };
    let tenant = match principal
        .begin_household(&ctx.db, household_id, request_id.clone())
        .await
    {
        Ok(value) => value,
        Err(error) => {
            return no_store(response::error(
                response::authentication(error),
                &request_id,
            ));
        }
    };
    let current = avatar::attachment(&tenant, principal.account_id(), PersonAccess::View).await;
    match current {
        Ok(Some(value)) if value.key == attached.key => {}
        Ok(Some(_)) | Ok(None) => {
            return no_store(
                finish(
                    tenant,
                    principal.provenance(),
                    audit::RequestAudit::avatar("GET", "show"),
                    Err(response::operation(OperationError::NotFound)),
                    &request_id,
                )
                .await,
            );
        }
        Err(error) => {
            return no_store(
                finish(
                    tenant,
                    principal.provenance(),
                    audit::RequestAudit::avatar("GET", "show"),
                    Err(response::operation(error)),
                    &request_id,
                )
                .await,
            );
        }
    }
    if audit::record(
        &tenant,
        principal.provenance(),
        audit::RequestAudit::avatar("GET", "show"),
        StatusCode::OK,
    )
    .await
    .is_err()
        || tenant.commit().await.is_err()
    {
        return storage_error(&request_id);
    }
    let mut reply = (
        StatusCode::OK,
        [(header::CONTENT_TYPE, image.content_type)],
        Body::from(image.bytes),
    )
        .into_response();
    reply.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).expect("Generated request UUID"),
    );
    no_store(reply)
}

pub(super) async fn upload(
    State(ctx): State<AppContext>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    request: Option<Extension<LocoRequestId>>,
    form: std::result::Result<Multipart, MultipartRejection>,
) -> Response {
    let request_id = request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    );
    let (principal, tenant) = match begin(&ctx, &headers, household_id, &request_id).await {
        Ok(value) => value,
        Err(error) => return no_store(response::error(error, &request_id)),
    };
    if let Err(error) =
        profile::linked_person(&tenant, principal.account_id(), PersonAccess::Manage).await
    {
        return no_store(
            finish(
                tenant,
                principal.provenance(),
                audit::RequestAudit::avatar("PUT", "update"),
                Err(response::operation(error)),
                &request_id,
            )
            .await,
        );
    }
    if tenant.commit().await.is_err() {
        return storage_error(&request_id);
    }
    let image = match form {
        Ok(form) => uploaded(form).await,
        Err(_) => Err(response::Failure::field("avatar", "is invalid")),
    };
    let image = match image {
        Ok(value) => value,
        Err(error) => return no_store(response::error(error, &request_id)),
    };
    let storage = match avatar::Storage::configured(&ctx) {
        Ok(value) => value,
        Err(_) => return storage_error(&request_id),
    };
    let key = format!("avatars/{household_id}/{}", uuid::Uuid::new_v4().simple());
    if storage.put(&key, &image).await.is_err() {
        avatar::discard_upload(&ctx, &storage, &key).await;
        return storage_error(&request_id);
    }
    let tenant = match principal
        .begin_household(&ctx.db, household_id, request_id.clone())
        .await
    {
        Ok(value) => value,
        Err(error) => {
            avatar::discard_upload(&ctx, &storage, &key).await;
            return no_store(response::error(
                response::authentication(error),
                &request_id,
            ));
        }
    };
    let saved = async {
        let retired = avatar::replace(
            &tenant,
            principal.account_id(),
            key.clone(),
            &image,
            storage.bucket(),
        )
        .await
        .map_err(response::operation)?;
        let snapshot = profile::read(&tenant, principal.account_id())
            .await
            .map_err(response::operation)?;
        Ok::<_, response::Failure>((retired, super::profile::representation(&snapshot)))
    }
    .await;
    match saved {
        Ok((retired, body)) => {
            let reply = no_store(
                finish(
                    tenant,
                    principal.provenance(),
                    audit::RequestAudit::avatar("PUT", "update"),
                    Ok((StatusCode::OK, body, None)),
                    &request_id,
                )
                .await,
            );
            if !reply.status().is_success() {
                avatar::discard_upload(&ctx, &storage, &key).await;
            }
            let _ = retired;
            reply
        }
        Err(error) => {
            let _ = tenant.rollback().await;
            avatar::discard_upload(&ctx, &storage, &key).await;
            no_store(response::error(error, &request_id))
        }
    }
}

pub(super) async fn remove(
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
        Err(error) => return no_store(response::error(error, &request_id)),
    };
    let storage = match avatar::Storage::configured(&ctx) {
        Ok(value) => value,
        Err(_) => return storage_error(&request_id),
    };
    let retired = match avatar::remove(&tenant, principal.account_id(), storage.bucket()).await {
        Ok(value) => value,
        Err(error) => {
            return no_store(
                finish(
                    tenant,
                    principal.provenance(),
                    audit::RequestAudit::avatar("DELETE", "destroy"),
                    Err(response::operation(error)),
                    &request_id,
                )
                .await,
            );
        }
    };
    let mut reply = no_store(
        finish(
            tenant,
            principal.provenance(),
            audit::RequestAudit::avatar("DELETE", "destroy"),
            Ok((StatusCode::NO_CONTENT, json!({}), None)),
            &request_id,
        )
        .await,
    );
    if reply.status().is_success() {
        let _ = retired;
        *reply.body_mut() = Body::empty();
        reply.headers_mut().remove(header::CONTENT_TYPE);
    }
    reply
}
