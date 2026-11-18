use super::super::medications::{
    authentication_error, begin, operation_error, request_id, unavailable,
};
use crate::models::{errors::OperationError, profile::avatar};
use axum::{
    Extension,
    extract::Multipart,
    http::{StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::controller::middleware::request_id::LocoRequestId;
use loco_rs::prelude::*;
use serde_json::json;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/households")
        .add("/{slug}/people/{id}/avatar", get(show_person))
}

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

async fn show_person(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    show_image(ctx, slug, session, request, id).await
}

async fn show_image(
    ctx: AppContext,
    slug: String,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    person: String,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id).await {
        Ok(value) => value,
        Err(error) => return private(authentication_error(error)),
    };
    let person_id =
        match crate::models::care::people::read(&tenant, &person, principal.time_zone()).await {
            Ok((person, _)) => match person["data"]["id"].as_i64() {
                Some(id) => id,
                None => return private(unavailable()),
            },
            Err(error) => return private(operation_error(error)),
        };
    let attachment = match avatar::person_attachment(&tenant, person_id).await {
        Ok(Some(value)) => value,
        Ok(None) => return private(operation_error(OperationError::NotFound)),
        Err(error) => return private(operation_error(error)),
    };
    if tenant.commit().await.is_err() {
        return private(unavailable());
    }
    let storage = match avatar::Storage::configured(&ctx) {
        Ok(value) => value,
        Err(_) => return private(unavailable()),
    };
    let bytes = match storage.get(&attachment.key).await {
        Ok(value) => value,
        Err(_) => return private(unavailable()),
    };
    let image = match avatar::decode(&bytes, &attachment.content_type) {
        Ok(value) => value,
        Err(_) => return private(unavailable()),
    };
    let tenant = match principal
        .begin_household_slug(&ctx.db, &slug, request_id, ctx.config.settings.as_ref())
        .await
    {
        Ok(value) => value,
        Err(error) => return private(authentication_error(error)),
    };
    match avatar::person_attachment(&tenant, person_id).await {
        Ok(Some(current)) if current.key == attachment.key => {}
        Ok(_) => return private(operation_error(OperationError::NotFound)),
        Err(error) => return private(operation_error(error)),
    }
    if tenant.commit().await.is_err() {
        return private(unavailable());
    }
    private(
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, image.content_type)],
            image.bytes,
        )
            .into_response(),
    )
}

pub(super) async fn uploaded(
    mut form: Multipart,
    token: &CsrfToken,
) -> std::result::Result<avatar::Image, OperationError> {
    let invalid = || OperationError::Validation {
        details: json!({"avatar":["is invalid"]}),
    };
    let mut authenticity_token = None;
    let mut image = None;
    while let Some(field) = form.next_field().await.map_err(|_| invalid())? {
        match field.name() {
            Some("authenticity_token") if authenticity_token.is_none() => {
                authenticity_token = Some(field.text().await.map_err(|_| invalid())?);
            }
            Some("avatar") if image.is_none() => {
                let content_type = field.content_type().unwrap_or_default().to_owned();
                let bytes = field.bytes().await.map_err(|_| invalid())?;
                image = Some((bytes, content_type));
            }
            _ => return Err(invalid()),
        }
    }
    token
        .verify(authenticity_token.as_deref().unwrap_or_default())
        .map_err(|_| OperationError::Forbidden)?;
    let (bytes, content_type) = image.ok_or_else(invalid)?;
    avatar::decode(&bytes, &content_type)
}
