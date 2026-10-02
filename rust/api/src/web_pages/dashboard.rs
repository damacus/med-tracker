use super::api_client::WebApi;
use super::dashboard_projection::{
    dashboard_history, dashboard_sources, dashboard_stock, project_source_tasks, SourceTaskInput,
};
use super::response::{dashboard_page, failure, page_status};
use super::time::{configured_timezone, dashboard_now};
use super::{field, numeric};
use crate::AppState;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use chrono::Timelike;
use medtracker_web::dashboard::{calculate_metrics, DashboardPage, DashboardPerson};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Deserialize)]
pub(super) struct DashboardQuery {
    dashboard_person_id: Option<String>,
    contract_fail_dashboard_read: Option<String>,
}

fn dashboard_read_error(cookie: Option<HeaderValue>) -> Response {
    page_status(format!("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><link rel=\"stylesheet\" href=\"/dashboard.css?v={:016x}\"><title>Dashboard unavailable | MedTracker</title></head><body><main class=\"dashboard-error\"><h1>Dashboard unavailable</h1><p>We could not load your dashboard. Please reconnect and try again.</p><a href=\"/reconnect\">Try again</a></main></body></html>", medtracker_web::dashboard::DASHBOARD_CSS_VERSION), cookie, StatusCode::SERVICE_UNAVAILABLE)
}

pub(super) async fn dashboard(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<DashboardQuery>,
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
    if cfg!(debug_assertions)
        && std::env::var_os("CONTRACT_PROJECT").is_some()
        && query.contract_fail_dashboard_read.as_deref() == Some("people")
    {
        return dashboard_read_error(api.cookie);
    }
    let base = format!("/api/v1/households/{household_id}");
    let visible = match api.collection(&format!("{base}/people")).await {
        Ok(value) => value,
        Err(_) => return dashboard_read_error(api.cookie),
    };
    let selectable_people: Vec<(i64, String)> = visible
        .iter()
        .filter_map(|row| Some((numeric(row, "id")?, field(row, "name").to_owned())))
        .collect();
    let profile = match api.get(&format!("{base}/profile")).await {
        Ok(value) => value,
        Err(_) => return dashboard_read_error(api.cookie),
    };
    let me = match api.get(&format!("{base}/me")).await {
        Ok(value) => value,
        Err(_) => return dashboard_read_error(api.cookie),
    };
    let household_manager = matches!(
        me.pointer("/data/membership_role").and_then(Value::as_str),
        Some("owner" | "administrator")
    );
    let mobile_shortcuts = profile
        .pointer("/data/mobile_shortcuts")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_else(|| vec!["dashboard".into(), "inventory".into(), "finder".into()]);
    let account_person_id = profile
        .pointer("/data/person_id")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<i64>().ok());
    let selection = query.dashboard_person_id.as_deref().unwrap_or("");
    let selected = if selection == "all" {
        selectable_people
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>()
    } else if selection.is_empty() {
        account_person_id
            .filter(|id| {
                selectable_people
                    .iter()
                    .any(|(visible_id, _)| visible_id == id)
            })
            .or_else(|| selectable_people.first().map(|(id, _)| *id))
            .map(|id| vec![id])
            .unwrap_or_default()
    } else if let Ok(id) = selection.parse::<i64>() {
        if selectable_people
            .iter()
            .any(|(candidate, _)| *candidate == id)
        {
            vec![id]
        } else {
            return failure(StatusCode::NOT_FOUND);
        }
    } else {
        return failure(StatusCode::BAD_REQUEST);
    };
    let selected_id = if selection.is_empty() {
        selected
            .first()
            .map(i64::to_string)
            .unwrap_or_else(|| "all".to_owned())
    } else {
        selection.to_owned()
    };
    let selected_name = if selected_id == "all" {
        "All Family".to_owned()
    } else {
        selectable_people
            .iter()
            .find(|(id, _)| id.to_string() == selected_id)
            .map(|(_, name)| name.clone())
            .unwrap_or_else(|| "All Family".to_owned())
    };
    let account_name = selectable_people
        .iter()
        .find(|(id, _)| Some(*id) == account_person_id)
        .map(|(_, name)| name.as_str())
        .unwrap_or("there");
    let timezone = profile
        .pointer("/data/time_zone")
        .and_then(Value::as_str)
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(configured_timezone);
    let schedules = match api.collection(&format!("{base}/schedules")).await {
        Ok(value) => value,
        Err(_) => return dashboard_read_error(api.cookie),
    };
    let assignments = match api.collection(&format!("{base}/person_medications")).await {
        Ok(value) => value,
        Err(_) => return dashboard_read_error(api.cookie),
    };
    let now = dashboard_now();
    let takes = match api
        .dashboard_takes(
            &format!("{base}/medication_takes"),
            &selected,
            now,
            timezone,
        )
        .await
    {
        Ok(value) => value,
        Err(_) => return dashboard_read_error(api.cookie),
    };
    let inventory = match api.collection(&format!("{base}/medications")).await {
        Ok(value) => value,
        Err(_) => return dashboard_read_error(api.cookie),
    };
    let medications: HashMap<i64, &Value> = inventory
        .iter()
        .filter_map(|row| Some((numeric(row, "id")?, row)))
        .collect();
    let today = now.with_timezone(&timezone).date_naive();
    let mut permitted_medications: HashMap<i64, std::collections::HashSet<i64>> = HashMap::new();
    for source in schedules.iter().chain(&assignments) {
        if let (Some(person_id), Some(medication_id)) = (
            numeric(source, "person_id"),
            numeric(source, "medication_id"),
        ) {
            permitted_medications
                .entry(person_id)
                .or_default()
                .insert(medication_id);
        }
    }
    let sources = dashboard_sources(schedules, assignments, &selected, today);
    let mut people: Vec<DashboardPerson> = selectable_people
        .iter()
        .filter(|(id, _)| selected.contains(id))
        .map(|(id, name)| DashboardPerson {
            id: *id,
            name: name.clone(),
            tasks: Vec::new(),
            outcomes: Vec::new(),
        })
        .collect();
    let mut metric_tasks = Vec::new();
    let mut stock_ids = Vec::new();
    for (kind, source) in &sources {
        let (Some(id), Some(person_id), Some(medication_id)) = (
            numeric(source, "id"),
            numeric(source, "person_id"),
            numeric(source, "medication_id"),
        ) else {
            return failure(StatusCode::BAD_GATEWAY);
        };
        stock_ids.push(medication_id);
        let path =
            format!("{base}/{kind}/{id}/dose_occurrences?start_date={today}&end_date={today}");
        let occurrences = match crate::dose_occurrences::with_dashboard_timezone(
            timezone,
            api.get(&path),
        )
        .await
        {
            Ok(value) => value,
            Err(_) => return dashboard_read_error(api.cookie),
        };
        let Some(rows) = occurrences.get("data").and_then(Value::as_array) else {
            return failure(StatusCode::BAD_GATEWAY);
        };
        let Some(person) = people.iter_mut().find(|person| person.id == person_id) else {
            continue;
        };
        project_source_tasks(
            SourceTaskInput {
                kind,
                id,
                medication_id,
                source,
                rows,
                now,
                timezone,
                takes: &takes,
                medications: &medications,
                permitted_medications: &permitted_medications,
                household_manager,
            },
            person,
            &mut metric_tasks,
        );
    }
    for person in &mut people {
        person
            .tasks
            .sort_by_key(|row| (row.scheduled_at.is_none(), row.scheduled_at));
        person
            .outcomes
            .sort_by_key(|row| (row.scheduled_at.is_none(), row.scheduled_at));
    }
    let history = dashboard_history(
        &takes,
        &selected,
        &selectable_people,
        &medications,
        now,
        timezone,
    );
    let stock = dashboard_stock(stock_ids, &medications);
    let greeting = format!(
        "Good {}, {}",
        if now.with_timezone(&timezone).hour() < 12 {
            "morning"
        } else if now.with_timezone(&timezone).hour() < 18 {
            "afternoon"
        } else {
            "evening"
        },
        account_name.split_whitespace().next().unwrap_or("there")
    );
    dashboard_page(
        medtracker_web::dashboard::render_dashboard(DashboardPage {
            household_name,
            slug,
            csrf: api.csrf.clone(),
            timezone,
            greeting,
            date: now
                .with_timezone(&timezone)
                .format("%A, %b %d")
                .to_string()
                .to_uppercase(),
            selected_id,
            selected_name,
            mobile_shortcuts,
            household_manager,
            people,
            selectable_people,
            metrics: calculate_metrics(&metric_tasks, now),
            stock,
            history,
        }),
        api.cookie,
    )
}
