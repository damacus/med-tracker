pub(super) mod forms;
mod management;
pub(super) mod rendering;

use crate::models::{
    access::TenantTransaction,
    care::{browser_query, doses, medications},
    errors::OperationError,
    identity::{
        browser::{self, BrowserPrincipal},
        resource::AuthenticationError,
    },
};
use axum::{
    Extension,
    extract::Form as AxumForm,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::controller::middleware::request_id::LocoRequestId;
use loco_rs::prelude::*;
use std::collections::HashMap;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/households")
        .add("/{slug}/medications", get(index).post(management::create))
        .add("/{slug}/medications/new", get(management::new))
        .add(
            "/{slug}/medications/{id}",
            get(show).post(management::update),
        )
        .add("/{slug}/medications/{id}/edit", get(management::edit))
        .add(
            "/{slug}/medications/{id}/destroy",
            post(management::destroy),
        )
        .add("/{slug}/medications/{id}/doses", post(take))
        .add("/{slug}/medications/{id}/refill", post(refill))
        .add(
            "/{slug}/medications/{id}/stock/adjust",
            get(edit_stock).post(adjust_stock),
        )
}

async fn index(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let (_, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let medications = match browser_query::index(&tenant).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let can_create = match medications::crud::can_create(&tenant).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let can_manage = match crate::models::care::administration::can_manage(&tenant).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let response = rendering::index(&view, &token, &slug, &medications, can_create, can_manage);
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    response
}

pub(super) async fn begin(
    ctx: &AppContext,
    session: &Session<SessionPgPool>,
    slug: &str,
    request_id: &str,
) -> std::result::Result<(BrowserPrincipal, TenantTransaction), AuthenticationError> {
    let principal = browser::authenticate(&ctx.db, session).await?;
    let tenant = principal
        .begin_household_slug(&ctx.db, slug, request_id.to_owned())
        .await?;
    Ok((principal, tenant))
}

async fn show(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    (request, headers): (Option<Extension<LocoRequestId>>, HeaderMap),
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let detail = match browser_query::detail(&tenant, &id, principal.time_zone()).await {
        Ok(detail) => detail,
        Err(error) => return operation_error(error),
    };
    let draft = forms::dose(&detail, principal.time_zone());
    let response = rendering::detail(
        &view,
        &token,
        &slug,
        detail,
        rendering::DosePresentation {
            zone: principal.time_zone(),
            language: headers
                .get(header::ACCEPT_LANGUAGE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("en"),
            draft: &draft,
            error: None,
        },
    );
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    response
}

async fn edit_stock(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let request_id = request_id(request);
    let (_, tenant) = match begin(&ctx, &session, &slug, &request_id).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let snapshot = match medications::read_stock_snapshot(&tenant, &id).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    if !forms::can_adjust(&tenant) {
        return operation_error(OperationError::Forbidden);
    }
    let draft = HashMap::from([
        ("etag".into(), snapshot.etag.clone()),
        (
            "new_quantity".into(),
            snapshot
                .medication
                .current_supply
                .map(|value| value.normalize().to_string())
                .unwrap_or_default(),
        ),
        ("reason".into(), String::new()),
    ]);
    let response = rendering::stock(&view, &token, &slug, snapshot, &draft, None, StatusCode::OK);
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    response
}

async fn take(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    (request, headers): (Option<Extension<LocoRequestId>>, HeaderMap),
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    if token
        .verify(forms::field(&draft, "authenticity_token"))
        .is_err()
    {
        return operation_error(OperationError::Forbidden);
    }
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let result = async {
        let detail = browser_query::detail(&tenant, &id, principal.time_zone()).await?;
        let input = forms::take(&detail, &draft, principal.time_zone())?;
        doses::execute_in_timezone(
            &tenant,
            doses::Command::Take(input),
            principal.time_zone(),
            Some(principal.provenance()),
        )
        .await
    }
    .await;
    match result {
        Ok(_) => {
            if tenant.commit().await.is_err() {
                return unavailable();
            }
            redirect(&slug, &id)
        }
        Err(error) => {
            if tenant.rollback().await.is_err() {
                return unavailable();
            }
            let tenant = match principal
                .begin_household_slug(&ctx.db, &slug, request_id)
                .await
            {
                Ok(value) => value,
                Err(error) => return authentication_error(error),
            };
            let detail = match browser_query::detail(&tenant, &id, principal.time_zone()).await {
                Ok(value) => value,
                Err(error) => return operation_error(error),
            };
            let response = rendering::detail(
                &view,
                &token,
                &slug,
                detail,
                rendering::DosePresentation {
                    zone: principal.time_zone(),
                    language: headers
                        .get(header::ACCEPT_LANGUAGE)
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or("en"),
                    draft: &draft,
                    error: Some(&error),
                },
            );
            if tenant.commit().await.is_err() {
                return unavailable();
            }
            response
        }
    }
}

async fn refill(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    if token
        .verify(forms::field(&draft, "authenticity_token"))
        .is_err()
    {
        return operation_error(OperationError::Forbidden);
    }
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let input = medications::Restock {
        medication_id: id.clone(),
        quantity: forms::field(&draft, "quantity").into(),
        restock_date: forms::field(&draft, "restock_date").into(),
        original_etag: forms::field(&draft, "etag").into(),
    };
    let result = medications::restock(&tenant, input, Some(principal.provenance())).await;
    match result {
        Ok(_) => {
            if tenant.commit().await.is_err() {
                return unavailable();
            }
            redirect(&slug, &id)
        }
        Err(error) => {
            if tenant.rollback().await.is_err() {
                return unavailable();
            }
            let tenant = match principal
                .begin_household_slug(&ctx.db, &slug, request_id)
                .await
            {
                Ok(value) => value,
                Err(error) => return authentication_error(error),
            };
            let detail = match browser_query::detail(&tenant, &id, principal.time_zone()).await {
                Ok(value) => value,
                Err(error) => return operation_error(error),
            };
            let response = rendering::refill_failure(
                &view,
                &token,
                &slug,
                detail,
                principal.time_zone(),
                &draft,
                &error,
            );
            if tenant.commit().await.is_err() {
                return unavailable();
            }
            response
        }
    }
}

async fn adjust_stock(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    if token
        .verify(forms::field(&draft, "authenticity_token"))
        .is_err()
    {
        return operation_error(OperationError::Forbidden);
    }
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let input = medications::AdjustStock {
        medication_id: id.clone(),
        new_quantity: forms::field(&draft, "new_quantity").into(),
        reason: forms::optional(&draft, "reason"),
    };
    let original = medications::ScalarPrecondition {
        original_etag: forms::field(&draft, "etag").into(),
    };
    let result = medications::execute_with_options(
        &tenant,
        medications::Command::AdjustStock(input),
        Some(&original),
        Some(principal.provenance()),
    )
    .await;
    match result {
        Ok(_) => {
            if tenant.commit().await.is_err() {
                return unavailable();
            }
            redirect(&slug, &id)
        }
        Err(error) => {
            if tenant.rollback().await.is_err() {
                return unavailable();
            }
            let tenant = match principal
                .begin_household_slug(&ctx.db, &slug, request_id)
                .await
            {
                Ok(value) => value,
                Err(error) => return authentication_error(error),
            };
            let snapshot = match medications::read_stock_snapshot(&tenant, &id).await {
                Ok(value) => value,
                Err(error) => return operation_error(error),
            };
            let response = rendering::stock(
                &view,
                &token,
                &slug,
                snapshot,
                &draft,
                Some(forms::message(&error)),
                forms::status(&error),
            );
            if tenant.commit().await.is_err() {
                return unavailable();
            }
            response
        }
    }
}

pub(super) fn request_id(request: Option<Extension<LocoRequestId>>) -> String {
    request.map_or_else(
        || uuid::Uuid::new_v4().to_string(),
        |Extension(id)| id.get().to_owned(),
    )
}

fn redirect(slug: &str, id: &str) -> Response {
    (
        StatusCode::SEE_OTHER,
        [
            (
                header::LOCATION,
                format!("/households/{slug}/medications/{id}"),
            ),
            (header::CACHE_CONTROL, "no-store".into()),
        ],
    )
        .into_response()
}

pub(super) fn operation_error(error: OperationError) -> Response {
    (
        forms::status(&error),
        [(header::CACHE_CONTROL, "no-store")],
        forms::message(&error),
    )
        .into_response()
}

pub(super) fn authentication_error(error: AuthenticationError) -> Response {
    match error {
        AuthenticationError::Unauthenticated => (
            StatusCode::SEE_OTHER,
            [
                (header::LOCATION, "/login"),
                (header::CACHE_CONTROL, "no-store"),
            ],
        )
            .into_response(),
        AuthenticationError::Forbidden | AuthenticationError::InsufficientScope { .. } => {
            operation_error(OperationError::Forbidden)
        }
        AuthenticationError::Unavailable => unavailable(),
    }
}

pub(super) fn unavailable() -> Response {
    operation_error(OperationError::Unavailable)
}
