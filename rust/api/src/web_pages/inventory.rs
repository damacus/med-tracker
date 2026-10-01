use super::api_client::WebApi;
use super::response::{error, failure, page, page_status, PageError};
use super::time::now_local;
use super::{field, numeric};
use crate::AppState;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use medtracker_web::{DoseFormState, DoseSource, MedicationCard, MedicationDetail};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

fn card(value: &Value) -> Option<MedicationCard> {
    Some(MedicationCard {
        id: numeric(value, "id")?,
        name: value
            .get("display_name")
            .and_then(Value::as_str)
            .or_else(|| value.get("name").and_then(Value::as_str))
            .unwrap_or("Medication")
            .to_owned(),
        supply: field(value, "current_supply")
            .strip_suffix(".0")
            .unwrap_or(field(value, "current_supply"))
            .to_owned(),
        unit: field(value, "dose_unit").to_owned(),
    })
}

pub(super) async fn medications(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    let mut api = match WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(response) => return response.response(),
    };
    let (household_id, household_name) = match api.household(&slug).await {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    let records = match api
        .collection(&format!("/api/v1/households/{household_id}/medications"))
        .await
    {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    let capabilities = match api.capabilities(household_id).await {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    let can_create = match capabilities
        .pointer("/data/medications/create")
        .and_then(Value::as_bool)
    {
        Some(value) => value,
        None => return failure(StatusCode::BAD_GATEWAY),
    };
    page(
        medtracker_web::render_medication_list_with_management(
            &household_name,
            &slug,
            &api.csrf,
            records.iter().filter_map(card).collect(),
            can_create,
            api.locale,
        ),
        api.cookie,
    )
}

async fn detail(
    api: &mut WebApi,
    household_id: i64,
    id: &str,
) -> Result<(MedicationDetail, Vec<MedicationCard>), PageError> {
    let base = format!("/api/v1/households/{household_id}");
    let medication = api.get(&format!("{base}/medications/{id}")).await?;
    let row = medication
        .get("data")
        .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
    let selected = card(row).ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
    let inventory = api.collection(&format!("{base}/medications")).await?;
    let inventory: HashMap<i64, MedicationCard> = inventory
        .iter()
        .filter_map(card)
        .map(|candidate| (candidate.id, candidate))
        .collect();
    let people = api.collection(&format!("{base}/people")).await?;
    let people: HashMap<i64, String> = people
        .iter()
        .filter_map(|person| Some((numeric(person, "id")?, field(person, "name").to_owned())))
        .collect();
    let mut sources = Vec::new();
    for (kind, resource) in [
        ("person_medication", "person_medications"),
        ("schedule", "schedules"),
    ] {
        for source in api.collection(&format!("{base}/{resource}")).await? {
            if numeric(&source, "medication_id") != Some(selected.id)
                || source.get("active").and_then(Value::as_bool) != Some(true)
                || source.get("paused").and_then(Value::as_bool) == Some(true)
            {
                continue;
            }
            let Some(source_id) = numeric(&source, "id") else {
                continue;
            };
            let Some(person_id) = numeric(&source, "person_id") else {
                continue;
            };
            let Some(person_name) = people.get(&person_id) else {
                continue;
            };
            let eligible_stock_ids: Vec<i64> = source
                .get("eligible_stock_medication_ids")
                .and_then(Value::as_array)
                .map(|ids| ids.iter().filter_map(Value::as_i64).collect())
                .unwrap_or_default();
            if eligible_stock_ids
                .iter()
                .any(|id| !inventory.contains_key(id))
            {
                return Err(error(StatusCode::BAD_GATEWAY));
            }
            sources.push(DoseSource {
                id: source_id,
                kind: kind.to_owned(),
                person_name: person_name.clone(),
                amount: if kind == "schedule" {
                    String::new()
                } else {
                    field(&source, "dose_amount").to_owned()
                },
                unit: if kind == "schedule" {
                    String::new()
                } else {
                    field(&source, "dose_unit").to_owned()
                },
                portable_id: field(&source, "portable_id").to_owned(),
                can_record: source.get("can_record").and_then(Value::as_bool) == Some(true),
                eligible_stock_ids,
            });
        }
    }
    let mut stock_ids = Vec::new();
    for source in &sources {
        for id in &source.eligible_stock_ids {
            if !stock_ids.contains(id) {
                stock_ids.push(*id);
            }
        }
    }
    let options: Vec<MedicationCard> = stock_ids
        .into_iter()
        .filter_map(|id| inventory.get(&id).cloned())
        .collect();
    Ok((
        MedicationDetail {
            id: selected.id,
            name: selected.name,
            description: field(row, "description").to_owned(),
            supply: selected.supply,
            unit: selected.unit,
            location: "Household inventory".to_owned(),
            sources,
        },
        options,
    ))
}

pub(super) async fn medication(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
    Query(query): Query<MedicationQuery>,
    headers: HeaderMap,
) -> Response {
    render_detail(
        state,
        slug,
        id,
        headers,
        DetailOutcome::normal(query.logged),
    )
    .await
}

#[derive(Deserialize)]
pub(super) struct MedicationQuery {
    logged: Option<String>,
}

pub(super) struct DetailOutcome {
    logged: Option<String>,
    client_uuid: Option<String>,
    error_state: Option<(String, DoseFormState)>,
    status: StatusCode,
}

impl DetailOutcome {
    fn normal(logged: Option<String>) -> Self {
        Self {
            logged,
            client_uuid: None,
            error_state: None,
            status: StatusCode::OK,
        }
    }

    pub(super) fn rejected(
        status: StatusCode,
        client_uuid: Option<String>,
        message: &str,
        form: DoseFormState,
    ) -> Self {
        Self {
            logged: None,
            client_uuid,
            error_state: Some((message.to_owned(), form)),
            status,
        }
    }
}

pub(super) async fn render_detail(
    state: AppState,
    slug: String,
    id: String,
    headers: HeaderMap,
    outcome: DetailOutcome,
) -> Response {
    let mut api = match WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(response) => return response.response(),
    };
    let (household_id, household_name) = match api.household(&slug).await {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    let (medication, options) = match detail(&mut api, household_id, &id).await {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    let notice = if let Some(take_id) = outcome.logged {
        let takes = match api
            .collection(&format!(
                "/api/v1/households/{household_id}/medication_takes"
            ))
            .await
        {
            Ok(value) => value,
            Err(response) => return response.response(),
        };
        takes
            .iter()
            .any(|take| {
                field(take, "portable_id") == take_id
                    && numeric(take, "medication_id") == Some(medication.id)
            })
            .then_some("Medication taken successfully.".to_owned())
    } else {
        outcome
            .error_state
            .as_ref()
            .map(|(message, _)| message.clone())
    };
    let form_state = outcome.error_state.map(|(_, state)| state);
    let uuid = outcome
        .client_uuid
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let capabilities = match api.capabilities(household_id).await {
        Ok(value) => value,
        Err(response) => return response.response(),
    };
    let can_edit = match capabilities
        .pointer("/data/medications/update")
        .and_then(Value::as_bool)
    {
        Some(value) => value,
        None => return failure(StatusCode::BAD_GATEWAY),
    };
    page_status(
        medtracker_web::render_medication_detail_with_management(
            medtracker_web::MedicationDetailRender {
                household_name: &household_name,
                slug: &slug,
                csrf: &api.csrf,
                medication,
                stock_options: options,
                taken_at: &now_local(),
                client_uuid: &uuid,
                notice: notice.as_deref(),
                form_state,
            },
            can_edit,
            api.locale,
        ),
        api.cookie,
        outcome.status,
    )
}
