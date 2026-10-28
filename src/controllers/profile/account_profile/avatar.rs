use super::*;
use crate::models::profile::avatar;
use axum::extract::{Multipart, multipart::MultipartRejection};

fn private(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        header::HeaderValue::from_static("nosniff"),
    );
    response
}

pub(super) async fn show(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
) -> Response {
    let (principal, transaction) = match begin(&ctx, &session, false).await {
        Ok(value) => value,
        Err(error) => return private(authentication_error(error)),
    };
    let account_id = AccountId::from_database(principal.account_id());
    let attachment = match avatar::account_attachment(&transaction, account_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return private(operation_error(OperationError::NotFound)),
        Err(error) => return private(operation_error(error)),
    };
    if let Err(error) = transaction.commit().await {
        return private(operation_error(error.into()));
    }
    let storage = match avatar::Storage::configured(&ctx) {
        Ok(value) => value,
        Err(error) => return private(operation_error(error)),
    };
    let bytes = match storage.get(&attachment.key).await {
        Ok(value) => value,
        Err(error) => return private(operation_error(error)),
    };
    let image = match avatar::decode(&bytes, &attachment.content_type) {
        Ok(value) => value,
        Err(error) => return private(operation_error(error)),
    };
    let transaction = match principal.account_transaction(&ctx.db, false).await {
        Ok(value) => value,
        Err(error) => return private(authentication_error(error)),
    };
    match avatar::account_attachment(&transaction, account_id).await {
        Ok(Some(current)) if current.key == attachment.key => {}
        Ok(_) => return private(operation_error(OperationError::NotFound)),
        Err(error) => return private(operation_error(error)),
    }
    if let Err(error) = transaction.commit().await {
        return private(operation_error(error.into()));
    }
    private(([(header::CONTENT_TYPE, image.content_type)], image.bytes).into_response())
}

async fn failure(page: &AccountPage<'_>, key: &str, status: StatusCode) -> Response {
    let mut response = page
        .render(Some(HashMap::from([
            ("setting".into(), "avatar".into()),
            ("avatar_error".into(), key.into()),
        ])))
        .await;
    if response.status() == StatusCode::UNPROCESSABLE_ENTITY {
        *response.status_mut() = status;
    }
    private(response)
}

pub(super) async fn upload(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    headers: HeaderMap,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    request: Option<Extension<LocoRequestId>>,
    form: std::result::Result<Multipart, MultipartRejection>,
) -> Response {
    let page = AccountPage {
        ctx: &ctx,
        session: &session,
        headers: &headers,
        token: &token,
        view: &view,
        request_id: request_id(request),
    };
    let (principal, transaction) = match begin(&ctx, &session, false).await {
        Ok(value) => value,
        Err(error) => return private(authentication_error(error)),
    };
    if let Err(error) = transaction.commit().await {
        return private(operation_error(error.into()));
    }
    let image = match form {
        Ok(form) => super::super::avatar::uploaded(form, &token).await,
        Err(_) => Err(profile::validation("avatar", "is invalid")),
    };
    let image = match image {
        Ok(value) => value,
        Err(OperationError::Validation { .. }) => {
            return failure(&page, "invalid_image", StatusCode::UNPROCESSABLE_ENTITY).await;
        }
        Err(error) => return private(operation_error(error)),
    };
    let storage = match avatar::Storage::configured(&ctx) {
        Ok(value) => value,
        Err(_) => {
            return failure(
                &page,
                "storage_unavailable",
                StatusCode::SERVICE_UNAVAILABLE,
            )
            .await;
        }
    };
    let account_id = AccountId::from_database(principal.account_id());
    let key = format!(
        "avatars/accounts/{account_id}/{}",
        uuid::Uuid::new_v4().simple()
    );
    if storage.put(&key, &image).await.is_err() {
        avatar::discard_upload(&ctx, &storage, &key).await;
        return failure(
            &page,
            "storage_unavailable",
            StatusCode::SERVICE_UNAVAILABLE,
        )
        .await;
    }
    let transaction = match principal.account_transaction(&ctx.db, true).await {
        Ok(value) => value,
        Err(error) => {
            avatar::discard_upload(&ctx, &storage, &key).await;
            return private(authentication_error(error));
        }
    };
    match avatar::replace_account(
        &transaction,
        account_id,
        &page.request_id,
        key.clone(),
        &image,
        storage.bucket(),
    )
    .await
    {
        Ok(_) => match transaction.commit().await {
            Ok(()) => private(redirect()),
            Err(_) => {
                avatar::discard_upload(&ctx, &storage, &key).await;
                failure(
                    &page,
                    "storage_unavailable",
                    StatusCode::SERVICE_UNAVAILABLE,
                )
                .await
            }
        },
        Err(error) => {
            if let Err(rollback) = transaction.rollback().await {
                avatar::discard_upload(&ctx, &storage, &key).await;
                return private(operation_error(rollback.into()));
            }
            avatar::discard_upload(&ctx, &storage, &key).await;
            private(operation_error(error))
        }
    }
}

pub(super) async fn remove(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    request: Option<Extension<LocoRequestId>>,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    if token
        .verify(fields.get("authenticity_token").map_or("", String::as_str))
        .is_err()
    {
        return private(operation_error(OperationError::Forbidden));
    }
    let (principal, transaction) = match begin(&ctx, &session, true).await {
        Ok(value) => value,
        Err(error) => return private(authentication_error(error)),
    };
    let storage = match avatar::Storage::configured(&ctx) {
        Ok(value) => value,
        Err(error) => return private(operation_error(error)),
    };
    match avatar::remove_account(
        &transaction,
        AccountId::from_database(principal.account_id()),
        &request_id(request),
        storage.bucket(),
    )
    .await
    {
        Ok(_) => match transaction.commit().await {
            Ok(()) => private(redirect()),
            Err(error) => private(operation_error(error.into())),
        },
        Err(error) => private(operation_error(error)),
    }
}
