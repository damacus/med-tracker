use super::medications::{self as browser, forms, rendering};
use crate::models::{
    care::{medications, orders},
    entities::medication,
    errors::OperationError,
};
use axum::{
    Extension,
    extract::Form as AxumForm,
    http::{StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};
use serde_json::json;
use std::collections::HashMap;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/households")
        .add("/{slug}/medications/{id}/order", get(show).post(order))
        .add("/{slug}/medications/{id}/received", post(receive))
}

async fn show(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let (_, tenant) =
        match browser::begin(&ctx, &session, &slug, &browser::request_id(request)).await {
            Ok(value) => value,
            Err(error) => return browser::authentication_error(error),
        };
    let snapshot = match medications::read_stock_snapshot(&tenant, &id).await {
        Ok(value) => value,
        Err(error) => return browser::operation_error(error),
    };
    let draft = defaults(&snapshot.medication);
    let response = render(&view, &token, &slug, &snapshot.medication, &draft, None);
    if tenant.commit().await.is_err() {
        return browser::unavailable();
    }
    response
}

async fn order(
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
        return browser::operation_error(OperationError::Forbidden);
    }
    let (principal, tenant) =
        match browser::begin(&ctx, &session, &slug, &browser::request_id(request)).await {
            Ok(value) => value,
            Err(error) => return browser::authentication_error(error),
        };
    let mut fields = serde_json::Map::new();
    for name in ["supplier", "quantity", "expected_arrival_on"] {
        if let Some(value) = forms::optional(&draft, name) {
            fields.insert(name.into(), json!(value));
        }
    }
    match orders::mark_as_ordered(
        &tenant,
        &id,
        &json!({"order_details":fields}),
        Some(principal.provenance()),
    )
    .await
    {
        Ok(_) => {
            if tenant.commit().await.is_err() {
                return browser::unavailable();
            }
            redirect(&slug, &id)
        }
        Err(error @ OperationError::Validation { .. }) => {
            let snapshot = match medications::read_stock_snapshot(&tenant, &id).await {
                Ok(value) => value,
                Err(error) => return browser::operation_error(error),
            };
            render(
                &view,
                &token,
                &slug,
                &snapshot.medication,
                &draft,
                Some(&error),
            )
        }
        Err(error) => browser::operation_error(error),
    }
}

async fn receive(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    if token
        .verify(forms::field(&draft, "authenticity_token"))
        .is_err()
    {
        return browser::operation_error(OperationError::Forbidden);
    }
    let (principal, tenant) =
        match browser::begin(&ctx, &session, &slug, &browser::request_id(request)).await {
            Ok(value) => value,
            Err(error) => return browser::authentication_error(error),
        };
    match orders::mark_as_received(&tenant, &id, &json!({}), Some(principal.provenance())).await {
        Ok(_) => {
            if tenant.commit().await.is_err() {
                return browser::unavailable();
            }
            redirect(&slug, &id)
        }
        Err(error) => browser::operation_error(error),
    }
}

fn defaults(record: &medication::Model) -> HashMap<String, String> {
    HashMap::from([
        (
            "supplier".into(),
            record.order_supplier.clone().unwrap_or_default(),
        ),
        (
            "quantity".into(),
            record
                .order_quantity
                .map(|value| value.normalize().to_string())
                .unwrap_or_default(),
        ),
        (
            "expected_arrival_on".into(),
            record
                .expected_arrival_on
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ),
    ])
}

fn render(
    view: &TeraView,
    token: &CsrfToken,
    slug: &str,
    record: &medication::Model,
    draft: &HashMap<String, String>,
    error: Option<&OperationError>,
) -> Response {
    let Ok(authenticity) = token.authenticity_token() else {
        return browser::unavailable();
    };
    let mut data = rendering::appearance_context();
    data["title"] = json!("Order medication");
    data["slug"] = json!(slug);
    data["medication_id"] = json!(record.id);
    data["name"] = json!(
        record
            .friendly_name
            .as_deref()
            .or(record.name.as_deref())
            .unwrap_or("Medication")
    );
    data["draft"] = json!(draft);
    data["authenticity_token"] = json!(authenticity);
    data["order_status"] = json!(match record.reorder_status {
        Some(1) => "Ordered",
        Some(2) => "Received",
        _ => "Not ordered",
    });
    data["errors"] = match error {
        Some(OperationError::Validation { details }) => {
            details.get("errors").cloned().unwrap_or(json!({}))
        }
        _ => json!({}),
    };
    let status = error.map(forms::status).unwrap_or(StatusCode::OK);
    match format::render().view(view, "medications/order.html", data) {
        Ok(response) => (
            status,
            token.clone(),
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(_) => browser::unavailable(),
    }
}

fn redirect(slug: &str, id: &str) -> Response {
    (
        StatusCode::SEE_OTHER,
        [
            (
                header::LOCATION,
                format!("/households/{slug}/medications/{id}/order"),
            ),
            (header::CACHE_CONTROL, "no-store".into()),
        ],
    )
        .into_response()
}
