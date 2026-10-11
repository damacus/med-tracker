pub(super) mod forms;
mod management;
pub(super) mod rendering;
mod scanner;
mod wizard;
mod workflow;

use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::{browser_query, doses, medications},
    errors::OperationError,
    identity::{
        browser::{self, BrowserPrincipal},
        resource::AuthenticationError,
    },
};
use axum::{
    Extension,
    extract::{Form as AxumForm, Query},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::controller::middleware::request_id::LocoRequestId;
use loco_rs::prelude::*;
use std::collections::HashMap;

#[derive(serde::Deserialize)]
struct PreviewRequest {
    source_type: String,
    source_id: String,
    taken_at: String,
}

#[derive(Default, serde::Deserialize)]
struct ShowRequest {
    record_source_type: Option<String>,
    record_source_id: Option<i64>,
    record_person_id: Option<i64>,
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/households")
        .add("/{slug}/medications", get(index).post(management::create))
        .add("/{slug}/medications/new", get(management::new))
        .add("/{slug}/medications/finder", get(scanner::finder))
        .add("/{slug}/medications/workflow", get(workflow::open))
        .add("/{slug}/medications/lookup", get(scanner::lookup))
        .add("/{slug}/medications/wizard", post(wizard::create))
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
        .add("/{slug}/medications/{id}/doses/preview", get(dose_preview))
        .add("/{slug}/medications/{id}/refill", post(refill))
        .add(
            "/{slug}/medications/{id}/stock/adjust",
            get(edit_stock).post(adjust_stock),
        )
}

async fn dose_preview(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    Query(input): Query<PreviewRequest>,
) -> Response {
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let snapshot = match medications::read_stock_snapshot(&tenant, &id).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let preview = match doses::browser_preview(
        &tenant,
        snapshot.medication.id,
        &input.source_type,
        &input.source_id,
        &input.taken_at,
        principal.time_zone(),
    )
    .await
    {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    let label = preview
        .amount
        .as_deref()
        .zip(preview.unit.as_deref())
        .map(|(amount, unit)| format!("{} {}", amount, rendering::unit_label(unit, amount)));
    (
        StatusCode::OK,
        [(header::CACHE_CONTROL, "no-store")],
        axum::Json(serde_json::json!({
            "available": preview.available,
            "amount": preview.amount,
            "unit": preview.unit,
            "label": label,
        })),
    )
        .into_response()
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
    let can_assign = match access::has_person_access(&tenant, PersonAccess::Manage).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let response = rendering::index(
        &view,
        &token,
        &slug,
        &medications,
        rendering::IndexPermissions {
            create: can_create,
            adjust: forms::can_adjust(&tenant),
            assign: can_assign,
            manage: can_manage,
        },
    );
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
    Query(input): Query<ShowRequest>,
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
    let selected = match (
        input.record_source_type.as_deref(),
        input.record_source_id,
        input.record_person_id,
    ) {
        (None, None, None) => None,
        (Some(kind), Some(source_id), Some(person_id)) => {
            let Some(source) = detail.assignments.iter().find(|source| {
                source.can_record
                    && source.source_type == kind
                    && source.id == source_id
                    && source.person_id == person_id
            }) else {
                return operation_error(OperationError::NotFound);
            };
            Some(forms::dose_for(
                source,
                detail.stock.medication.id,
                principal.time_zone(),
            ))
        }
        _ => return operation_error(OperationError::NotFound),
    };
    let draft = selected
        .as_ref()
        .cloned()
        .unwrap_or_else(|| forms::dose(&detail, principal.time_zone()));
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
            open: selected.is_some(),
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
            let mut detail = match browser_query::detail(&tenant, &id, principal.time_zone()).await
            {
                Ok(value) => value,
                Err(error) => return operation_error(error),
            };
            if let Ok(preview) = doses::browser_preview(
                &tenant,
                detail.stock.medication.id,
                forms::field(&draft, "source_type"),
                forms::field(&draft, "source_id"),
                forms::field(&draft, "taken_at"),
                principal.time_zone(),
            )
            .await
                && let (Some(amount), Some(unit)) = (preview.amount, preview.unit)
                && let Some(source) = detail.assignments.iter_mut().find(|source| {
                    source.source_type == forms::field(&draft, "source_type")
                        && source.id.to_string() == forms::field(&draft, "source_id")
                })
            {
                source.amount = amount;
                source.unit = unit;
                source.available = preview.available;
            }
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
                    open: true,
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
    (request, headers): (Option<Extension<LocoRequestId>>, HeaderMap),
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
    if forms::field(&draft, "etag").trim().is_empty() {
        if let Err(error) = browser_query::detail(&tenant, &id, principal.time_zone()).await {
            return operation_error(error);
        }
        if tenant.commit().await.is_err() {
            return unavailable();
        }
        return (
            StatusCode::PRECONDITION_REQUIRED,
            [(header::CACHE_CONTROL, "no-store")],
            "Reopen the refill form before saving.",
        )
            .into_response();
    }
    let input = medications::Restock {
        medication_id: id.clone(),
        quantity: forms::field(&draft, "quantity").into(),
        restock_date: forms::field(&draft, "restock_date").into(),
        original_etag: forms::field(&draft, "etag").into(),
    };
    let result = match forms::optional(&draft, "dosage_option_id") {
        Some(id) => {
            medications::restock_option(&tenant, input, &id, Some(principal.provenance())).await
        }
        None => medications::restock(&tenant, input, Some(principal.provenance())).await,
    };
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
                rendering::RefillPresentation {
                    zone: principal.time_zone(),
                    language: headers
                        .get(header::ACCEPT_LANGUAGE)
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or("en"),
                    draft: &draft,
                    error: &error,
                },
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
    (request, headers): (Option<Extension<LocoRequestId>>, HeaderMap),
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
    if forms::field(&draft, "etag").trim().is_empty() {
        if let Err(error) = medications::read_stock_snapshot(&tenant, &id).await {
            return operation_error(error);
        }
        if !forms::can_adjust(&tenant) {
            return operation_error(OperationError::Forbidden);
        }
        if tenant.commit().await.is_err() {
            return unavailable();
        }
        return (
            StatusCode::PRECONDITION_REQUIRED,
            [(header::CACHE_CONTROL, "no-store")],
            "Reopen the stock adjustment form before saving.",
        )
            .into_response();
    }
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
            if forms::field(&draft, "presentation") == "stock" && forms::can_adjust(&tenant) {
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
                return response;
            }
            let detail = match browser_query::detail(&tenant, &id, principal.time_zone()).await {
                Ok(value) => value,
                Err(error) => return operation_error(error),
            };
            let response = rendering::adjustment_failure(
                &view,
                &token,
                &slug,
                detail,
                rendering::AdjustmentPresentation {
                    zone: principal.time_zone(),
                    language: headers
                        .get(header::ACCEPT_LANGUAGE)
                        .and_then(|value| value.to_str().ok())
                        .unwrap_or("en"),
                    draft: &draft,
                    error: &error,
                },
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
