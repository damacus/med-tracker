use super::*;
use crate::models::entities::account;
use axum::extract::Query;
use sea_orm::EntityTrait;
use serde::Deserialize;
use serde_json::json;

#[derive(Default, Deserialize)]
pub(super) struct WorkflowQuery {
    person_id: Option<i64>,
    intent: Option<String>,
}

pub(super) async fn open(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(query): Query<WorkflowQuery>,
) -> Response {
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let people = match access::people_with_access(&tenant, PersonAccess::Manage).await {
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
    if context_aware
        && query
            .intent
            .as_deref()
            .is_none_or(|value| value == "assign_medication")
        && let Some(person) = people
            .iter()
            .find(|person| Some(person.id) == query.person_id)
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
                        "/households/{slug}/people/{}/treatments/assignments/new",
                        person.id
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
    let mut data = rendering::appearance_context();
    data["title"] = json!("Choose a person");
    data["slug"] = json!(slug);
    data["can_create"] = json!(can_create);
    data["people"] = json!(
        people
            .iter()
            .map(|person| json!({"id":person.id,"name":person.name}))
            .collect::<Vec<_>>()
    );
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    match format::render().view(&view, "medications/workflow.html", data) {
        Ok(response) => ([(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(_) => unavailable(),
    }
}
