use super::api_client::WebApi;
use super::inventory::{render_detail, DetailOutcome};
use super::response::failure;
use super::time::taken_at;
use super::{field, numeric};
use crate::{oauth, AppState};
use axum::extract::{Form, Path, State};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use medtracker_web::DoseFormState;
use serde_json::json;
use std::collections::HashMap;

pub(super) async fn record_dose(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    if !oauth::trusted_cookie_origin(&state, &headers) {
        return failure(StatusCode::FORBIDDEN);
    }
    let mut api = match WebApi::authenticated(state.clone(), headers.clone()).await {
        Ok(api) => api,
        Err(response) => return response.response(),
    };
    if fields.get("authenticity_token") != Some(&api.csrf) {
        return failure(StatusCode::FORBIDDEN);
    }
    let (household_id, _) = match api.household(&slug).await {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    let bad = || failure(StatusCode::UNPROCESSABLE_ENTITY);
    let amount = fields.get("dose_amount").map(String::as_str).unwrap_or("");
    let form_state = DoseFormState {
        source_type: fields.get("source_type").cloned().unwrap_or_default(),
        source_id: fields.get("source_id").cloned().unwrap_or_default(),
        dose_amount: amount.to_owned(),
        dose_unit: fields.get("dose_unit").cloned().unwrap_or_default(),
        taken_at: fields.get("taken_at").cloned().unwrap_or_default(),
        stock_id: fields
            .get("taken_from_medication_id")
            .cloned()
            .unwrap_or_default(),
    };
    let Some(time) = fields.get("taken_at").and_then(|value| taken_at(value)) else {
        return render_detail(
            state,
            slug,
            id,
            headers,
            DetailOutcome::rejected(
                StatusCode::UNPROCESSABLE_ENTITY,
                fields.get("client_uuid").cloned(),
                "Taken at is invalid.",
                form_state,
            ),
        )
        .await;
    };
    let Some(csrf) = fields.get("authenticity_token") else {
        return failure(StatusCode::FORBIDDEN);
    };
    let Some(source_type) = fields.get("source_type") else {
        return bad();
    };
    let Some(source_id) = fields.get("source_id") else {
        return bad();
    };
    let Some(client_uuid) = fields.get("client_uuid") else {
        return bad();
    };
    let source_resource = match source_type.as_str() {
        "person_medication" => "person_medications",
        "schedule" => "schedules",
        _ => return bad(),
    };
    let medication = match api
        .get(&format!(
            "/api/v1/households/{household_id}/medications/{id}"
        ))
        .await
    {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    let source = match api
        .get(&format!(
            "/api/v1/households/{household_id}/{source_resource}/{source_id}"
        ))
        .await
    {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    if numeric(&source["data"], "medication_id") != numeric(&medication["data"], "id") {
        return render_detail(
            state,
            slug,
            id,
            headers,
            DetailOutcome::rejected(
                StatusCode::FORBIDDEN,
                Some(client_uuid.clone()),
                "This source does not belong to this medication.",
                form_state,
            ),
        )
        .await;
    }
    let stock = fields
        .get("taken_from_medication_id")
        .and_then(|value| value.parse::<i64>().ok());
    let mut attributes = json!({
        "client_uuid": client_uuid, "source_type": source_type, "source_id": source_id,
        "taken_at": time,
        "taken_from_medication_id": stock
    });
    if source_type == "person_medication" {
        attributes["dose_amount"] = json!(amount);
        attributes["dose_unit"] = json!(fields.get("dose_unit"));
    }
    let body = json!({"medication_take": attributes});
    let reply = match api
        .call(
            Method::POST,
            &format!("/api/v1/households/{household_id}/medication_takes"),
            Some(body),
            Some(csrf),
        )
        .await
    {
        Ok(reply) => reply,
        Err(response) => return response.response(),
    };
    if reply.status.is_success() {
        let take_id = field(&reply.value["data"], "portable_id");
        if take_id.is_empty() {
            return failure(StatusCode::BAD_GATEWAY);
        }
        let location = format!("/households/{slug}/medications/{id}?logged={take_id}");
        let mut response = (
            StatusCode::SEE_OTHER,
            [
                (header::LOCATION, location),
                (header::CACHE_CONTROL, "no-store".to_owned()),
            ],
        )
            .into_response();
        if let Some(cookie) = api.cookie {
            response.headers_mut().append(header::SET_COOKIE, cookie);
        }
        return response;
    }
    let notice = if reply.status == StatusCode::UNPROCESSABLE_ENTITY {
        "Invalid dose configured"
    } else if reply.status == StatusCode::FORBIDDEN {
        "You cannot record this dose."
    } else if reply.status == StatusCode::CONFLICT {
        "This dose request conflicts with a previous record."
    } else {
        return failure(reply.status);
    };
    render_detail(
        state,
        slug,
        id,
        headers,
        DetailOutcome::rejected(reply.status, Some(client_uuid.clone()), notice, form_state),
    )
    .await
}
