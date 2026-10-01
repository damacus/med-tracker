use super::*;
use axum::extract::Query;
use serde::Deserialize;

#[derive(Default, Deserialize)]
pub(super) struct ScheduleQuery {
    #[serde(rename = "type")]
    kind: Option<String>,
}

pub(super) fn defaults(kind: Kind, schedule_type: Option<&str>) -> TreatmentDraft {
    let mut fields = BTreeMap::from([
        ("submission_id".into(), Uuid::new_v4().to_string()),
        ("dose_cycle".into(), "daily".into()),
    ]);
    match kind {
        Kind::Assignment => {
            fields.insert("administration_kind".into(), "as_needed".into());
        }
        Kind::Schedule => {
            fields.insert(
                "schedule_type".into(),
                schedule_type.unwrap_or("daily").into(),
            );
        }
    }
    TreatmentDraft { fields }
}

fn value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        value => value.to_string(),
    }
}

pub(super) fn from_record(record: &Value, etag: String, kind: Kind) -> TreatmentDraft {
    let mut draft = defaults(kind, record["schedule_type"].as_str());
    for name in [
        "medication_id",
        "dose_amount",
        "dose_unit",
        "administration_kind",
        "dose_cycle",
        "max_daily_doses",
        "min_hours_between_doses",
        "notes",
        "frequency",
        "start_date",
        "end_date",
    ] {
        draft.fields.insert(name.to_owned(), value(&record[name]));
    }
    draft.fields.insert("etag".into(), etag);
    if matches!(kind, Kind::Schedule) {
        let config = &record["schedule_config"];
        draft
            .fields
            .insert("config_original".into(), config.to_string());
        draft.fields.insert(
            "config_type_original".into(),
            field(record, "schedule_type").into(),
        );
        for (prefix, key) in [("time", "times"), ("date", "dates")] {
            if let Some(values) = config[key].as_array() {
                for (index, entry) in values.iter().enumerate() {
                    draft
                        .fields
                        .insert(format!("{prefix}_{index}"), value(entry));
                }
            }
        }
        if let Some(days) = config["weekdays"].as_array() {
            for day in days {
                let name = match day.as_str() {
                    Some("monday" | "mon" | "1") => "monday",
                    Some("tuesday" | "tue" | "2") => "tuesday",
                    Some("wednesday" | "wed" | "3") => "wednesday",
                    Some("thursday" | "thu" | "4") => "thursday",
                    Some("friday" | "fri" | "5") => "friday",
                    Some("saturday" | "sat" | "6") => "saturday",
                    Some("sunday" | "sun" | "0" | "7") => "sunday",
                    _ => continue,
                };
                draft
                    .fields
                    .insert(format!("weekday_{name}"), "true".into());
            }
        }
        if let Some(steps) = config["taper_steps"].as_array() {
            for (index, step) in steps.iter().enumerate() {
                for name in [
                    "start_date",
                    "end_date",
                    "dose_amount",
                    "dose_unit",
                    "max_daily_doses",
                    "min_hours_between_doses",
                ] {
                    let entry = step
                        .get(name)
                        .or_else(|| match name {
                            "dose_amount" => step.get("amount"),
                            "dose_unit" => step.get("unit"),
                            _ => None,
                        })
                        .unwrap_or(&Value::Null);
                    draft
                        .fields
                        .insert(format!("step_{index}_{name}"), value(entry));
                }
                if let Some(times) = step["times"].as_array() {
                    for (time_index, time) in times.iter().enumerate() {
                        draft
                            .fields
                            .insert(format!("step_{index}_time_{time_index}"), value(time));
                    }
                }
            }
        }
    }
    draft
}

pub(super) fn render(
    context: Context,
    location: FormLocation,
    draft: TreatmentDraft,
    errors: Errors,
    status: StatusCode,
) -> Response {
    let context::FormChoices {
        medications,
        dosages,
        units,
    } = match context::choices(&context) {
        Ok(choices) => choices,
        Err(error) => return error.response(),
    };
    let action = location.id.as_ref().map_or_else(
        || base(&location.slug, &location.person, location.kind),
        |id| {
            format!(
                "{}/{}",
                base(&location.slug, &location.person, location.kind),
                path_segment(id)
            )
        },
    );
    let editing = location.id.is_some();
    let page = TreatmentFormPage {
        household_name: context.household_name,
        slug: location.slug,
        person_id: location.person,
        person_name: field(&context.person, "name").to_owned(),
        csrf: context.api.csrf,
        locale: context.api.locale,
        action,
        editing,
        schedule: matches!(location.kind, Kind::Schedule),
        medications,
        dosages,
        units,
        draft,
        errors,
    };
    let result = match location.kind {
        Kind::Assignment => medtracker_web::treatments::render_assignment_form(page),
        Kind::Schedule => medtracker_web::treatments::render_schedule_form(page),
    };
    match result {
        Ok(body) => page_status(body, context.api.cookie, status),
        Err(_) => failure(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn new(
    state: AppState,
    headers: HeaderMap,
    slug: String,
    person: String,
    kind: Kind,
    schedule_type: Option<&str>,
) -> Response {
    let context = match load(state, headers, &slug, &person).await {
        Ok(context) => context,
        Err(error) => return error.response(),
    };
    render(
        context,
        FormLocation {
            slug,
            person,
            id: None,
            kind,
        },
        defaults(kind, schedule_type),
        Errors::new(),
        StatusCode::OK,
    )
}

async fn edit(
    state: AppState,
    headers: HeaderMap,
    slug: String,
    person: String,
    id: String,
    kind: Kind,
) -> Response {
    let mut context = match load(state, headers, &slug, &person).await {
        Ok(context) => context,
        Err(error) => return error.response(),
    };
    let reply = match source(&mut context, kind, &id).await {
        Ok(reply) => reply,
        Err(error) => return error.response(),
    };
    let draft = from_record(&reply.value["data"], reply.etag.unwrap_or_default(), kind);
    render(
        context,
        FormLocation {
            slug,
            person,
            id: Some(id),
            kind,
        },
        draft,
        Errors::new(),
        StatusCode::OK,
    )
}

pub(super) async fn new_assignment(
    State(state): State<AppState>,
    Path((slug, person)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    new(state, headers, slug, person, Kind::Assignment, None).await
}

pub(super) async fn new_schedule(
    State(state): State<AppState>,
    Path((slug, person)): Path<(String, String)>,
    Query(query): Query<ScheduleQuery>,
    headers: HeaderMap,
) -> Response {
    new(
        state,
        headers,
        slug,
        person,
        Kind::Schedule,
        query.kind.as_deref(),
    )
    .await
}

pub(super) async fn edit_assignment(
    State(state): State<AppState>,
    Path((slug, person, id)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Response {
    edit(state, headers, slug, person, id, Kind::Assignment).await
}

pub(super) async fn edit_schedule(
    State(state): State<AppState>,
    Path((slug, person, id)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Response {
    edit(state, headers, slug, person, id, Kind::Schedule).await
}
