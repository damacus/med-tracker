use super::*;
use crate::models::profile::avatar;
use axum::{
    body::Body,
    extract::{Multipart, multipart::MultipartRejection},
};

fn private(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn unavailable(request_id: &str) -> Response {
    tracing::warn!(request_id, "Account avatar operation unavailable");
    private(failure(
        StatusCode::SERVICE_UNAVAILABLE,
        "avatar_unavailable",
        "Avatar storage is temporarily unavailable",
        request_id,
    ))
}

pub(super) async fn show(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    let principal =
        match resource::authenticate(&ctx.db, &headers, ctx.config.settings.as_ref()).await {
            Ok(value) => value,
            Err(error) => return private(authentication(error, &request_id)),
        };
    let account_id = AccountId::from_database(principal.account_id());
    let transaction = match principal
        .account_transaction(&ctx.db, ctx.config.settings.as_ref(), false)
        .await
    {
        Ok(value) => value,
        Err(error) => return private(authentication(error, &request_id)),
    };
    let attached = match avatar::account_attachment(&transaction, account_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return private(operation(OperationError::NotFound, &request_id)),
        Err(_) => return unavailable(&request_id),
    };
    if transaction.commit().await.is_err() {
        return unavailable(&request_id);
    }
    let storage = match avatar::Storage::configured(&ctx) {
        Ok(value) => value,
        Err(_) => return unavailable(&request_id),
    };
    let bytes = match storage.get(&attached.key).await {
        Ok(value) => value,
        Err(_) => return unavailable(&request_id),
    };
    let image = match avatar::decode(&bytes, &attached.content_type) {
        Ok(value) => value,
        Err(_) => return unavailable(&request_id),
    };
    let transaction = match principal
        .account_transaction(&ctx.db, ctx.config.settings.as_ref(), false)
        .await
    {
        Ok(value) => value,
        Err(error) => return private(authentication(error, &request_id)),
    };
    match avatar::account_attachment(&transaction, account_id).await {
        Ok(Some(current)) if current.key == attached.key => {}
        Ok(_) => return private(operation(OperationError::NotFound, &request_id)),
        Err(_) => return unavailable(&request_id),
    }
    if transaction.commit().await.is_err() {
        return unavailable(&request_id);
    }
    let mut response = (
        StatusCode::OK,
        [(header::CONTENT_TYPE, image.content_type)],
        Body::from(image.bytes),
    )
        .into_response();
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", value);
    }
    private(response)
}

pub(super) async fn upload(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
    form: std::result::Result<Multipart, MultipartRejection>,
) -> Response {
    let request_id = request_id(request);
    let principal =
        match resource::authenticate(&ctx.db, &headers, ctx.config.settings.as_ref()).await {
            Ok(value) => value,
            Err(error) => return private(authentication(error, &request_id)),
        };
    let account_id = AccountId::from_database(principal.account_id());
    let transaction = match principal
        .account_transaction(&ctx.db, ctx.config.settings.as_ref(), false)
        .await
    {
        Ok(value) => value,
        Err(error) => return private(authentication(error, &request_id)),
    };
    if transaction.commit().await.is_err() {
        return unavailable(&request_id);
    }
    let image = match form {
        Ok(form) => super::super::care::avatar::uploaded(form).await,
        Err(_) => Err(crate::models::profile::validation("avatar", "is invalid")),
    };
    let image = match image {
        Ok(value) => value,
        Err(_) => {
            return private(failure(
                StatusCode::UNPROCESSABLE_ENTITY,
                "unprocessable_content",
                "Avatar contains an invalid image",
                &request_id,
            ));
        }
    };
    let storage = match avatar::Storage::configured(&ctx) {
        Ok(value) => value,
        Err(_) => return unavailable(&request_id),
    };
    let key = format!(
        "avatars/accounts/{account_id}/{}",
        uuid::Uuid::new_v4().simple()
    );
    if storage.put(&key, &image).await.is_err() {
        avatar::discard_upload(&ctx, &storage, &key).await;
        return unavailable(&request_id);
    }
    let transaction = match principal
        .account_transaction(&ctx.db, ctx.config.settings.as_ref(), true)
        .await
    {
        Ok(value) => value,
        Err(error) => {
            avatar::discard_upload(&ctx, &storage, &key).await;
            return private(authentication(error, &request_id));
        }
    };
    let saved = async {
        avatar::replace_account(
            &transaction,
            account_id,
            &request_id,
            key.clone(),
            &image,
            storage.bucket(),
        )
        .await?;
        self_account::read(&transaction, account_id, ctx.config.settings.as_ref()).await
    }
    .await;
    match saved {
        Ok(snapshot) => match transaction.commit().await {
            Ok(()) => private(reply(
                StatusCode::OK,
                Some(representation(snapshot)),
                &request_id,
            )),
            Err(_) => {
                avatar::discard_upload(&ctx, &storage, &key).await;
                unavailable(&request_id)
            }
        },
        Err(_) => {
            let _ = transaction.rollback().await;
            avatar::discard_upload(&ctx, &storage, &key).await;
            unavailable(&request_id)
        }
    }
}

pub(super) async fn remove(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    request: Option<axum::Extension<LocoRequestId>>,
) -> Response {
    let request_id = request_id(request);
    let principal =
        match resource::authenticate(&ctx.db, &headers, ctx.config.settings.as_ref()).await {
            Ok(value) => value,
            Err(error) => return private(authentication(error, &request_id)),
        };
    let transaction = match principal
        .account_transaction(&ctx.db, ctx.config.settings.as_ref(), true)
        .await
    {
        Ok(value) => value,
        Err(error) => return private(authentication(error, &request_id)),
    };
    let storage = match avatar::Storage::configured(&ctx) {
        Ok(value) => value,
        Err(_) => return unavailable(&request_id),
    };
    match avatar::remove_account(
        &transaction,
        AccountId::from_database(principal.account_id()),
        &request_id,
        storage.bucket(),
    )
    .await
    {
        Ok(_) => match transaction.commit().await {
            Ok(()) => private(reply(StatusCode::NO_CONTENT, None, &request_id)),
            Err(_) => unavailable(&request_id),
        },
        Err(_) => {
            let _ = transaction.rollback().await;
            unavailable(&request_id)
        }
    }
}
