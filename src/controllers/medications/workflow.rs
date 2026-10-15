use super::*;
use crate::models::{care::report_pdf, entities::account};
use axum::{extract::Query, http::HeaderMap};
use sea_orm::EntityTrait;
use serde::Deserialize;
use serde_json::json;

#[derive(Default, Deserialize)]
pub(super) struct WorkflowQuery {
    person_id: Option<i64>,
    intent: Option<String>,
    medication_id: Option<i64>,
}

pub(super) async fn open(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    ViewEngine(view): ViewEngine<TeraView>,
    headers: HeaderMap,
    Query(query): Query<WorkflowQuery>,
) -> Response {
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let people = match crate::models::person_identity::current_summaries(&tenant).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let account = match account::Entity::find_by_id(principal.account_id())
        .one(tenant.transaction())
        .await
    {
        Ok(Some(value)) => value,
        _ => return unavailable(),
    };
    let context_aware =
        account.preferences["medication_launcher_variant"].as_str() == Some("context_aware");
    let schedule_intent = query.intent.as_deref() == Some("schedule");
    if context_aware
        && query
            .intent
            .as_deref()
            .is_none_or(|value| matches!(value, "assign_medication" | "schedule"))
        && let Some(person) = people
            .iter()
            .find(|person| Some(person.id.get()) == query.person_id)
    {
        if tenant.commit().await.is_err() {
            return unavailable();
        }
        return (
            StatusCode::SEE_OTHER,
            [
                (
                    header::LOCATION,
                    format!(
                        "/households/{slug}/people/{}/treatments/{}/new?source=workflow{}",
                        person.id,
                        if schedule_intent {
                            "schedules"
                        } else {
                            "assignments"
                        },
                        query
                            .medication_id
                            .map(|id| format!("&medication_id={id}"))
                            .unwrap_or_default(),
                    ),
                ),
                (header::CACHE_CONTROL, "no-store".into()),
            ],
        )
            .into_response();
    }
    let can_create = match medications::crud::can_create(&tenant).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let locale = report_pdf::locale(
        headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("en"),
    );
    let labels = match report_pdf::translations(locale) {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    let mut data = rendering::appearance_context();
    data["title"] = labels["medication_workflow"]["chooser"]["title"].clone();
    data["slug"] = json!(slug);
    data["can_create"] = json!(can_create);
    data["people"] = json!(
        people
            .iter()
            .map(|person| json!({"id":person.id,"name":person.name}))
            .collect::<Vec<_>>()
    );
    data["schedule_intent"] = json!(schedule_intent);
    data["medication_id"] = json!(query.medication_id);
    data["lang"] = json!(locale);
    data["labels"] = labels;
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    match format::render().view(
        &view,
        "medications/workflow.html",
        crate::views::template_data(&data),
    ) {
        Ok(response) => ([(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(_) => unavailable(),
    }
}
