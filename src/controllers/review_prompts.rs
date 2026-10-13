use crate::{
    controllers::medications::{
        authentication_error, begin, operation_error, rendering::appearance_context, request_id,
        unavailable,
    },
    models::{
        access::{self, PersonAccess},
        care::{
            report_pdf,
            review_prompts::{self, edit},
            sync,
        },
        errors::OperationError,
        identity::browser,
    },
};
use axum::{
    Extension,
    extract::{Form, Query},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use axum_csrf::CsrfToken;
use axum_session::Session;
use axum_session_sqlx::SessionPgPool;
use loco_rs::{controller::middleware::request_id::LocoRequestId, prelude::*};
use sea_orm::ActiveModelTrait;
use serde_json::{Value, json};
use std::collections::HashMap;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/households")
        .add("/{slug}/medicine-reviews", get(index))
        .add("/{slug}/medicine-reviews/{id}/history", get(history))
        .add("/{slug}/medicine-reviews/{id}/edit", post(edit_page))
        .add("/{slug}/medicine-reviews/{id}/save", post(save))
        .add("/{slug}/medicine-reviews/{id}/renew", post(renew))
        .add("/{slug}/medicine-reviews/{id}/cancel", post(cancel))
}

fn language(headers: &HeaderMap) -> &str {
    headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("en")
}

fn render(
    view: &TeraView,
    token: &CsrfToken,
    template: &str,
    slug: &str,
    language: &str,
    mut payload: Value,
    status: StatusCode,
) -> Response {
    let selected = report_pdf::locale(language);
    let Ok(labels) = report_pdf::translations(selected) else {
        return unavailable();
    };
    let Ok(authenticity) = token.authenticity_token() else {
        return unavailable();
    };
    let mut context = appearance_context();
    context["slug"] = json!(slug);
    context["title"] = labels["medication_reviews"]["title"].clone();
    context["lang"] = json!(selected);
    context["labels"] = labels;
    context["authenticity_token"] = json!(authenticity);
    if payload.is_object() {
        context
            .as_object_mut()
            .unwrap()
            .extend(payload.as_object_mut().unwrap().clone());
    }
    match format::render().view(view, template, context) {
        Ok(response) => (
            status,
            token.clone(),
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}

fn verified(token: &CsrfToken, draft: &HashMap<String, String>) -> bool {
    draft
        .get("authenticity_token")
        .is_some_and(|value| token.verify(value).is_ok())
}

fn redirect(slug: &str) -> Response {
    (
        StatusCode::SEE_OTHER,
        [
            (
                header::LOCATION,
                format!("/households/{slug}/medicine-reviews"),
            ),
            (header::CACHE_CONTROL, "no-store".to_owned()),
        ],
    )
        .into_response()
}

async fn index(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    headers: HeaderMap,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    query: std::result::Result<
        Query<review_prompts::ListQuery>,
        axum::extract::rejection::QueryRejection,
    >,
) -> Response {
    let (_, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let query = query.ok().map(|Query(value)| value).unwrap_or_default();
    let updated = query.updated.as_deref() == Some("1");
    let result = review_prompts::index(&tenant, Some(query)).await;
    let mut data = match result {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let people = match access::people_with_access(&tenant, PersonAccess::View).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let manageable = match access::people_with_access(&tenant, PersonAccess::Manage).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let names: HashMap<i64, String> = people
        .into_iter()
        .map(|person| (person.id, person.name))
        .collect();
    let managed: std::collections::HashSet<i64> =
        manageable.into_iter().map(|person| person.id).collect();
    if let Some(rows) = data["data"].as_array_mut() {
        for row in rows {
            if let Some(id) = row["person_id"].as_str().and_then(|id| id.parse().ok()) {
                row["person_name"] = json!(names.get(&id));
                row["can_edit"] = json!(managed.contains(&id));
            }
        }
    }
    let page = data["meta"]["page"].as_u64().unwrap_or(1);
    let total = data["meta"]["total_count"].as_u64().unwrap_or(0);
    let size = data["meta"]["per_page"].as_u64().unwrap_or(20);
    let payload = json!({
        "reviews":data,
        "page":page,
        "has_next":page.saturating_mul(size)<total,
        "previous_page":page.saturating_sub(1),
        "next_page":page.saturating_add(1),
        "updated":updated,
    });
    let response = render(
        &view,
        &token,
        "reviews/index.html",
        &slug,
        language(&headers),
        payload,
        StatusCode::OK,
    );
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    response
}

async fn history(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    headers: HeaderMap,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let (_, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let (data, _) = match review_prompts::show(&tenant, &id).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let response = render(
        &view,
        &token,
        "reviews/history.html",
        &slug,
        language(&headers),
        json!({"review":data}),
        StatusCode::OK,
    );
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    response
}

async fn edit_page(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    headers: HeaderMap,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(draft): Form<HashMap<String, String>>,
) -> Response {
    if !verified(&token, &draft) {
        return operation_error(OperationError::Forbidden);
    }
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let lease = match edit::acquire(&tenant, &id).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let (review, etag) = match review_prompts::show(&tenant, &id).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let submitted_version = draft.get("etag").filter(|value| !value.is_empty());
    let changed = submitted_version.is_some_and(|value| value != &etag);
    let response = render(
        &view,
        &token,
        "reviews/edit.html",
        &slug,
        language(&headers),
        json!({"review":review["data"],"lease":lease["data"],"draft":draft,"changed":changed,"error":Value::Null}),
        StatusCode::OK,
    );
    if !browser::authenticate(&ctx.db, &session)
        .await
        .is_ok_and(|current| current.account_id() == principal.account_id())
    {
        let _ = tenant.rollback().await;
        return authentication_error(
            crate::models::identity::resource::AuthenticationError::Unauthenticated,
        );
    }
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    response
}

async fn save(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    headers: HeaderMap,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Form(draft): Form<HashMap<String, String>>,
) -> Response {
    if !verified(&token, &draft) {
        return operation_error(OperationError::Forbidden);
    }
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let result = async {
        sync::lock(&tenant).await?;
        let context = sync::review_prompts::AuthContext { tenant: &tenant };
        if !sync::review_prompts::actor_adult(tenant.transaction(), &context).await? {
            return Err(OperationError::Forbidden);
        }
        let record = sync::review_prompts::visible_prompt(tenant.transaction(), &context, &id).await?.ok_or(OperationError::NotFound)?;
        if !sync::review_prompts::person_access(tenant.transaction(), &context, record.person_id, true).await? {
            return Err(OperationError::Forbidden);
        }
        review_prompts::reconcile_person(&tenant, record.person_id).await?;
        let record = sync::review_prompts::visible_prompt(tenant.transaction(), &context, &id).await?.ok_or(OperationError::NotFound)?;
        let lease = draft.get("lease_token").map(String::as_str).unwrap_or("");
        edit::require(&tenant, &record, lease).await?;
        if draft.get("etag").map(String::as_str) != Some(sync::review_prompts::tag(&record).as_str()) {
            return Err(OperationError::Conflict { code:"review_changed".into(), details:json!({"error":"The review changed while you were editing. Check the current evidence before saving."}) });
        }
        let mut attributes = json!({"medication_review_prompt":{
            "status":draft.get("status"),
            "practitioner_name":draft.get("practitioner_name"),
            "practitioner_role":draft.get("practitioner_role"),
            "review_note":draft.get("review_note"),
        }});
        let empty_date = draft.get("reviewed_on").is_some_and(String::is_empty);
        if !empty_date {
            attributes["medication_review_prompt"]["reviewed_on"] = json!(draft.get("reviewed_on"));
        }
        let changes = sync::review_prompts::Changes::parse(&attributes).map_err(|_| OperationError::Validation { details:json!({"error":"Review fields are invalid"}) })?;
        let mut active = changes.apply(&record, tenant.membership().id).ok_or(OperationError::Validation { details:json!({"error":"Practitioner name, role and date are required for this outcome"}) })?;
        if empty_date {
            if matches!(draft.get("status").map(String::as_str), Some("reviewed_with_practitioner" | "expected_prescribed_combination")) {
                return Err(OperationError::Validation { details:json!({"error":"Practitioner name, role and date are required for this outcome"}) });
            }
            active.reviewed_on = sea_orm::Set(None);
        }
        edit::archive(&tenant, &record, "review").await?;
        let updated = active.update(tenant.transaction()).await?;
        sync::review_prompts::record_rest_update(&tenant, &record, &updated).await?;
        edit::consume(&tenant, &record, lease).await?;
        Ok::<_,OperationError>(())
    }.await;
    match result {
        Ok(()) => {
            if !browser::authenticate(&ctx.db, &session)
                .await
                .is_ok_and(|current| current.account_id() == principal.account_id())
            {
                let _ = tenant.rollback().await;
                return authentication_error(
                    crate::models::identity::resource::AuthenticationError::Unauthenticated,
                );
            }
            if tenant.commit().await.is_err() {
                return unavailable();
            }
            (
                StatusCode::SEE_OTHER,
                [
                    (
                        header::LOCATION,
                        format!("/households/{slug}/medicine-reviews?updated=1"),
                    ),
                    (header::CACHE_CONTROL, "no-store".to_owned()),
                ],
            )
                .into_response()
        }
        Err(error) => {
            if tenant.rollback().await.is_err() {
                return unavailable();
            }
            let (_, read) =
                match begin(&ctx, &session, &slug, &uuid::Uuid::new_v4().to_string()).await {
                    Ok(value) => value,
                    Err(error) => return authentication_error(error),
                };
            if let Err(error) = sync::lock(&read).await {
                return operation_error(error);
            }
            let context = sync::review_prompts::AuthContext { tenant: &read };
            let record =
                match sync::review_prompts::visible_prompt(read.transaction(), &context, &id).await
                {
                    Ok(Some(record)) => record,
                    Ok(None) => return operation_error(OperationError::NotFound),
                    Err(error) => return operation_error(error),
                };
            if let Err(error) = review_prompts::reconcile_person(&read, record.person_id).await {
                return operation_error(error);
            }
            let (review, _) = match review_prompts::show(&read, &id).await {
                Ok(value) => value,
                Err(error) => return operation_error(error),
            };
            let response = render(
                &view,
                &token,
                "reviews/edit.html",
                &slug,
                language(&headers),
                json!({"review":review["data"],"lease":{"token":draft.get("lease_token"),"etag":draft.get("etag"),"expires_at":""},"draft":draft,"error":crate::controllers::medications::forms::message(&error),"changed":matches!(error, OperationError::Conflict { ref code, .. } if code=="review_changed"),"reacquire":matches!(error, OperationError::Conflict { .. })}),
                crate::controllers::medications::forms::status(&error),
            );
            if read.commit().await.is_err() {
                return unavailable();
            }
            response
        }
    }
}

async fn renew(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    Form(draft): Form<HashMap<String, String>>,
) -> Response {
    if !verified(&token, &draft) {
        return operation_error(OperationError::Forbidden);
    }
    let (_, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let result = edit::renew(
        &tenant,
        &id,
        draft.get("lease_token").map(String::as_str).unwrap_or(""),
    )
    .await;
    match result {
        Ok(value) => {
            if tenant.commit().await.is_err() {
                return unavailable();
            }
            (
                StatusCode::OK,
                [(header::CACHE_CONTROL, "no-store")],
                axum::Json(value),
            )
                .into_response()
        }
        Err(error) => {
            let _ = tenant.rollback().await;
            operation_error(error)
        }
    }
}

async fn cancel(
    State(ctx): State<AppContext>,
    Path((slug, id)): Path<(String, String)>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    Form(draft): Form<HashMap<String, String>>,
) -> Response {
    if !verified(&token, &draft) {
        return operation_error(OperationError::Forbidden);
    }
    let (_, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let result = edit::release(
        &tenant,
        &id,
        draft.get("lease_token").map(String::as_str).unwrap_or(""),
    )
    .await;
    match result {
        Ok(()) => {
            if tenant.commit().await.is_err() {
                return unavailable();
            }
            redirect(&slug)
        }
        Err(error) => {
            let _ = tenant.rollback().await;
            operation_error(error)
        }
    }
}
