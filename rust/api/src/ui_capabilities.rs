use crate::medication_management::{finish, household_manager, request_context};
use crate::{locations, medication_management, people, ApiError, AppState};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use serde_json::json;

pub(super) async fn show(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let manager = household_manager(&context);
    let manage_locations = locations::manager(&context);
    let create_person = people::may_create(&db, &context.membership).await?;
    let manageable_people = people::manageable_ids(&db, &context).await?;
    let create_medication = medication_management::may_create(&db, &context).await?;
    let body = json!({"data": {
        "people": {"create": create_person, "manage_ids": manageable_people},
        "locations": {"create": manage_locations, "update": manage_locations},
        "medications": {"create": create_medication, "update": manager, "manage_stock": manager}
    }});
    finish(
        db,
        &context,
        "GET",
        "api/v1/ui_capabilities",
        "HouseholdPolicy",
        "show",
        StatusCode::OK,
        true,
        body,
        None,
    )
    .await
}
