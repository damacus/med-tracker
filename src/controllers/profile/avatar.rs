use super::super::medications::{
    authentication_error, begin, operation_error, request_id, unavailable,
};
use crate::models::{
    access::PersonAccess,
    errors::OperationError,
    profile::{self, avatar},
};
use axum::{
    Extension,
    extract::{Form as AxumForm, Multipart, multipart::MultipartRejection},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::controller::middleware::request_id::LocoRequestId;
use loco_rs::prelude::*;
use serde_json::json;
use std::collections::HashMap;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/households")
        .add("/{slug}/profile/avatar/settings", get(settings))
        .add(
            "/{slug}/profile/avatar",
            get(show)
                .post(upload)
                .layer(axum::extract::DefaultBodyLimit::max(6 * 1024 * 1024)),
        )
        .add("/{slug}/profile/avatar/remove", post(remove))
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

struct Page<'a> {
    ctx: &'a AppContext,
    session: &'a Session<SessionPgPool>,
    token: &'a CsrfToken,
    view: &'a TeraView,
    headers: &'a HeaderMap,
    slug: &'a str,
    request_id: String,
}

impl Page<'_> {
    async fn render(&self, error: Option<&str>, status: StatusCode) -> Response {
        let draft = error.map(|key| {
            HashMap::from([
                ("setting".into(), "avatar".into()),
                ("avatar_error".into(), key.into()),
            ])
        });
        let mut response = super::Page {
            ctx: self.ctx,
            slug: self.slug,
            session: self.session,
            request_id: self.request_id.clone(),
            headers: self.headers,
            token: self.token,
            view: self.view,
        }
        .render(draft)
        .await;
        if response.status().is_success() || response.status() == StatusCode::UNPROCESSABLE_ENTITY {
            *response.status_mut() = status;
        }
        private(response)
    }

    fn redirect(&self) -> Response {
        private(
            (
                StatusCode::SEE_OTHER,
                [(
                    header::LOCATION,
                    format!("/households/{}/profile#profile", self.slug),
                )],
            )
                .into_response(),
        )
    }
}

async fn settings(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    (headers, request): (HeaderMap, Option<Extension<LocoRequestId>>),
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    Page {
        ctx: &ctx,
        session: &session,
        token: &token,
        headers: &headers,
        view: &view,
        slug: &slug,
        request_id: request_id(request),
    }
    .render(None, StatusCode::OK)
    .await
}

async fn show(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    show_image(ctx, slug, session, request, None).await
}

async fn show_person(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
) -> Response {
    show_image(ctx, slug, session, request, Some(id)).await
}

async fn attached(
    tenant: &crate::models::access::TenantTransaction,
    account_id: i64,
    person_id: Option<i64>,
) -> std::result::Result<Option<avatar::Attachment>, OperationError> {
    match person_id {
        Some(id) => avatar::person_attachment(tenant, id).await,
        None => avatar::attachment(tenant, account_id, PersonAccess::View).await,
    }
}

async fn show_image(
    ctx: AppContext,
    slug: String,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    person: Option<String>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id).await {
        Ok(value) => value,
        Err(error) => return private(authentication_error(error)),
    };
    let person_id = if let Some(id) = person {
        match crate::models::care::people::read(&tenant, &id, principal.time_zone()).await {
            Ok((person, _)) => match person["data"]["id"].as_i64() {
                Some(id) => Some(id),
                None => return private(unavailable()),
            },
            Err(error) => return private(operation_error(error)),
        }
    } else {
        None
    };
    let attachment = match attached(&tenant, principal.account_id(), person_id).await {
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
        .begin_household_slug(&ctx.db, &slug, request_id)
        .await
    {
        Ok(value) => value,
        Err(error) => return private(authentication_error(error)),
    };
    match attached(&tenant, principal.account_id(), person_id).await {
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

async fn uploaded(
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

async fn upload(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    (headers, request): (HeaderMap, Option<Extension<LocoRequestId>>),
    ViewEngine(view): ViewEngine<TeraView>,
    form: std::result::Result<Multipart, MultipartRejection>,
) -> Response {
    let page = Page {
        ctx: &ctx,
        session: &session,
        token: &token,
        headers: &headers,
        view: &view,
        slug: &slug,
        request_id: request_id(request),
    };
    let (principal, tenant) = match begin(&ctx, &session, &slug, &page.request_id).await {
        Ok(value) => value,
        Err(error) => return private(authentication_error(error)),
    };
    if let Err(error) =
        profile::linked_person(&tenant, principal.account_id(), PersonAccess::Manage).await
    {
        return private(operation_error(error));
    }
    let tenant_household_id = tenant.scope().household_id;
    if tenant.commit().await.is_err() {
        return private(unavailable());
    }
    let image = match form {
        Ok(form) => uploaded(form, &token).await,
        Err(_) => Err(OperationError::Validation {
            details: json!({"avatar":["is invalid"]}),
        }),
    };
    let image = match image {
        Ok(value) => value,
        Err(OperationError::Validation { .. }) => {
            return page
                .render(Some("invalid_image"), StatusCode::UNPROCESSABLE_ENTITY)
                .await;
        }
        Err(error) => return private(operation_error(error)),
    };
    let storage = match avatar::Storage::configured(&ctx) {
        Ok(value) => value,
        Err(_) => {
            return page
                .render(Some("storage_unavailable"), StatusCode::SERVICE_UNAVAILABLE)
                .await;
        }
    };
    let key = format!(
        "avatars/{}/{}",
        tenant_household_id,
        uuid::Uuid::new_v4().simple()
    );
    if storage.put(&key, &image).await.is_err() {
        avatar::discard_upload(&ctx, &storage, &key).await;
        return page
            .render(Some("storage_unavailable"), StatusCode::SERVICE_UNAVAILABLE)
            .await;
    }
    let tenant = match principal
        .begin_household_slug(&ctx.db, &slug, page.request_id.clone())
        .await
    {
        Ok(value) => value,
        Err(error) => {
            avatar::discard_upload(&ctx, &storage, &key).await;
            return private(authentication_error(error));
        }
    };
    match avatar::replace(
        &tenant,
        principal.account_id(),
        key.clone(),
        &image,
        storage.bucket(),
    )
    .await
    {
        Ok(_) => {
            if tenant.commit().await.is_err() {
                avatar::discard_upload(&ctx, &storage, &key).await;
                return page
                    .render(Some("storage_unavailable"), StatusCode::SERVICE_UNAVAILABLE)
                    .await;
            }
            page.redirect()
        }
        Err(error) => {
            let _ = tenant.rollback().await;
            avatar::discard_upload(&ctx, &storage, &key).await;
            if matches!(error, OperationError::Unavailable) {
                page.render(Some("storage_unavailable"), StatusCode::SERVICE_UNAVAILABLE)
                    .await
            } else {
                private(operation_error(error))
            }
        }
    }
}

async fn remove(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    (headers, request): (HeaderMap, Option<Extension<LocoRequestId>>),
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(form): AxumForm<HashMap<String, String>>,
) -> Response {
    if token
        .verify(
            form.get("authenticity_token")
                .map(String::as_str)
                .unwrap_or_default(),
        )
        .is_err()
    {
        return private(operation_error(OperationError::Forbidden));
    }
    let page = Page {
        ctx: &ctx,
        session: &session,
        token: &token,
        headers: &headers,
        view: &view,
        slug: &slug,
        request_id: request_id(request),
    };
    let (principal, tenant) = match begin(&ctx, &session, &slug, &page.request_id).await {
        Ok(value) => value,
        Err(error) => return private(authentication_error(error)),
    };
    let storage = match avatar::Storage::configured(&ctx) {
        Ok(value) => value,
        Err(_) => {
            return page
                .render(Some("storage_unavailable"), StatusCode::SERVICE_UNAVAILABLE)
                .await;
        }
    };
    match avatar::remove(&tenant, principal.account_id(), storage.bucket()).await {
        Ok(_) => {
            if tenant.commit().await.is_err() {
                return page
                    .render(Some("storage_unavailable"), StatusCode::SERVICE_UNAVAILABLE)
                    .await;
            }
            page.redirect()
        }
        Err(error) => {
            let _ = tenant.rollback().await;
            private(operation_error(error))
        }
    }
}
