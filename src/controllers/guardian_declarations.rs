use crate::{
    controllers::medications::{authentication_error, operation_error, request_id, unavailable},
    models::{
        care::{guardian_declarations, report_pdf},
        errors::OperationError,
        identity::browser,
    },
};
use axum::{
    Extension,
    extract::{Form, Query},
    http::{HeaderMap, HeaderValue, header},
    response::{IntoResponse, Redirect},
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/profile/guardian-declarations")
        .add("/", get(index))
        .add("/{id}/withdraw", post(withdraw))
}

#[derive(Default, Deserialize)]
struct Pagination {
    before_id: Option<i64>,
}

async fn index(
    State(ctx): State<AppContext>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    headers: HeaderMap,
    Query(page): Query<Pagination>,
) -> Response {
    let principal =
        match browser::authenticate(&ctx.db, &session, ctx.config.settings.as_ref()).await {
            Ok(value) => value,
            Err(error) => return authentication_error(error),
        };
    let transaction = match principal
        .guardian_declaration_transaction(&ctx.db, false)
        .await
    {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let records =
        guardian_declarations::own_page(&transaction, principal.account_id(), 25, page.before_id)
            .await;
    let records = match records {
        Ok(value) => value,
        Err(error) => {
            if let Err(error) = transaction.rollback().await {
                return operation_error(error.into());
            }
            return operation_error(error);
        }
    };
    if let Err(error) = transaction.commit().await {
        return operation_error(error.into());
    }
    let locale = report_pdf::locale(
        headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("en"),
    );
    let labels = match report_pdf::translations(locale) {
        Ok(labels) => labels,
        Err(_) => return unavailable(),
    };
    let Ok(authenticity) = token.authenticity_token() else {
        return unavailable();
    };
    let mut data = super::medications::rendering::appearance_context();
    data["title"] = labels["guardian_declarations"]["title"].clone();
    data["guardian_labels"] = labels["guardian_declarations"].clone();
    data["lang"] = json!(locale);
    data["declarations"] = json!(records.records);
    data["next_before_id"] = json!(records.next_before_id);
    data["authenticity_token"] = json!(authenticity);
    match format::render().view(&view, "profile/guardian_declarations.html", data) {
        Ok(response) => no_store((token, response).into_response()),
        Err(_) => unavailable(),
    }
}

async fn withdraw(
    State(ctx): State<AppContext>,
    Path(id): Path<String>,
    session: Session<SessionPgPool>,
    token: CsrfToken,
    request: Option<Extension<LocoRequestId>>,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    if token
        .verify(
            fields
                .get("authenticity_token")
                .map(String::as_str)
                .unwrap_or(""),
        )
        .is_err()
    {
        return no_store(operation_error(OperationError::Forbidden));
    }
    let Some(id) = id.parse::<i64>().ok().filter(|id| *id > 0) else {
        return no_store(operation_error(OperationError::NotFound));
    };
    let Some(revision) = fields
        .get("revision")
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value >= 0)
    else {
        return no_store(operation_error(guardian_declarations::validation(
            "revision",
            "is invalid",
        )));
    };
    if fields.get("withdrawn").map(String::as_str) != Some("true") {
        return no_store(operation_error(guardian_declarations::validation(
            "withdrawn",
            "must be true",
        )));
    }
    let principal =
        match browser::authenticate(&ctx.db, &session, ctx.config.settings.as_ref()).await {
            Ok(value) => value,
            Err(error) => return no_store(authentication_error(error)),
        };
    let transaction = match principal
        .guardian_declaration_transaction(&ctx.db, true)
        .await
    {
        Ok(value) => value,
        Err(error) => return no_store(authentication_error(error)),
    };
    let result = guardian_declarations::withdraw_own(
        &transaction,
        principal.account_id(),
        id,
        revision,
        &request_id(request),
        principal.provenance(),
    )
    .await;
    match result {
        Ok(_) => match transaction.commit().await {
            Ok(()) => no_store(Redirect::to("/profile/guardian-declarations").into_response()),
            Err(error) => no_store(operation_error(error.into())),
        },
        Err(error) => {
            if let Err(error) = transaction.rollback().await {
                return no_store(operation_error(error.into()));
            }
            no_store(operation_error(error))
        }
    }
}

fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
