use crate::{
    controllers::medications::{authentication_error, operation_error, rendering, unavailable},
    models::{entities::platform_admin, errors::OperationError, identity::browser, nhs_dmd},
};
use axum::{
    extract::{DefaultBodyLimit, Multipart},
    http::{StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::json;

pub fn routes() -> Routes {
    Routes::new().prefix("/admin").add(
        "/nhs-dmd",
        get(index).post(upload).layer(DefaultBodyLimit::max(
            nhs_dmd::storage::MAX_ARCHIVE_BYTES + 1024 * 1024,
        )),
    )
}

async fn authorize(
    ctx: &AppContext,
    session: &Session<SessionPgPool>,
) -> std::result::Result<(), Box<Response>> {
    let principal = browser::authenticate(&ctx.db, session)
        .await
        .map_err(|error| Box::new(authentication_error(error)))?;
    let admin = platform_admin::Entity::find()
        .filter(platform_admin::Column::AccountId.eq(principal.account_id()))
        .filter(platform_admin::Column::Status.eq("active"))
        .one(&ctx.db)
        .await
        .map_err(|_| Box::new(unavailable()))?;
    if admin.is_none() {
        return Err(Box::new(operation_error(OperationError::Forbidden)));
    }
    Ok(())
}

async fn index(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    if let Err(response) = authorize(&ctx, &session).await {
        return *response;
    }
    render(&ctx, &token, &view, None).await
}

async fn render(
    ctx: &AppContext,
    token: &CsrfToken,
    view: &TeraView,
    error: Option<&OperationError>,
) -> Response {
    let rows = match nhs_dmd::latest(ctx).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let Ok(authenticity) = token.authenticity_token() else {
        return unavailable();
    };
    let mut data = rendering::appearance_context();
    data["title"] = json!("NHS dm+d catalogue");
    data["authenticity_token"] = json!(authenticity);
    data["error"] = json!(error.map(crate::controllers::medications::forms::message));
    data["imports"]=json!(rows.into_iter().map(|row|json!({"id":row.id,"filename":row.uploaded_filename,"status":match row.status{0=>"Queued",1=>"Extracting",2=>"Counting",3=>"Importing",4=>"Completed",_=>"Failed"},"active":row.status<4,"total":row.total_records,"processed":row.processed_records,"created":row.created_count,"updated":row.updated_count,"unchanged":row.unchanged_count,"skipped":row.skipped_count,"error":row.error_message})).collect::<Vec<_>>());
    match format::render().view(view, "admin/nhs-dmd.html", data) {
        Ok(response) => (
            error
                .map(crate::controllers::medications::forms::status)
                .unwrap_or(StatusCode::OK),
            token.clone(),
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}

async fn upload(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    mut multipart: Multipart,
) -> Response {
    if let Err(response) = authorize(&ctx, &session).await {
        return *response;
    }
    let mut authenticity = String::new();
    let mut archive = None;
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(_) => {
                return operation_error(OperationError::Validation {
                    details: json!({"error":"The upload could not be read."}),
                });
            }
        };
        match field.name() {
            Some("authenticity_token") => {
                authenticity = match field.text().await {
                    Ok(value) => value,
                    Err(_) => return operation_error(OperationError::Forbidden),
                }
            }
            Some("archive") => {
                let filename = field.file_name().unwrap_or("").to_owned();
                let bytes = match field.bytes().await {
                    Ok(value) => value.to_vec(),
                    Err(_) => {
                        return operation_error(OperationError::Validation {
                            details: json!({"error":"The archive could not be read."}),
                        });
                    }
                };
                archive = Some((filename, bytes));
            }
            _ => {}
        }
    }
    if token.verify(&authenticity).is_err() {
        return operation_error(OperationError::Forbidden);
    }
    if let Err(response) = authorize(&ctx, &session).await {
        return *response;
    }
    let result = match archive {
        Some((filename, bytes)) => nhs_dmd::upload(&ctx, &filename, bytes).await,
        None => Err(OperationError::Validation {
            details: json!({"error":"Choose a release archive."}),
        }),
    };
    match result {
        Ok(_) => (
            StatusCode::SEE_OTHER,
            [
                (header::LOCATION, "/admin/nhs-dmd"),
                (header::CACHE_CONTROL, "no-store"),
            ],
        )
            .into_response(),
        Err(error) => render(&ctx, &token, &view, Some(&error)).await,
    }
}
