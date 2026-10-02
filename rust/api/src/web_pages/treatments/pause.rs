use super::*;
use medtracker_web::treatments::{PauseHistoryPage, PauseHistoryRow};

fn kind(resource: &str) -> Result<Kind, PageError> {
    match resource {
        "assignments" => Ok(Kind::Assignment),
        "schedules" => Ok(Kind::Schedule),
        _ => Err(error(StatusCode::NOT_FOUND)),
    }
}

async fn periods(
    context: &mut Context,
    kind: Kind,
    portable_id: &str,
) -> Result<Vec<Value>, PageError> {
    let mut records = Vec::new();
    let mut page = 1_u64;
    loop {
        let reply = context.api.get(&format!("/api/v1/households/{}/medication_pause_periods?source_type={}&source_id={}&page={page}&per_page=100", context.household_id, kind.body_key(), path_segment(portable_id))).await?;
        let total = reply
            .pointer("/meta/total_count")
            .and_then(Value::as_u64)
            .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
        let batch = reply["data"]
            .as_array()
            .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
        if batch.is_empty() && (records.len() as u64) < total {
            return Err(error(StatusCode::BAD_GATEWAY));
        }
        records.extend(batch.iter().cloned());
        if (records.len() as u64) >= total {
            return Ok(records);
        }
        page = page
            .checked_add(1)
            .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
    }
}

fn medication_name(context: &Context, record: &Value) -> String {
    context
        .medications
        .iter()
        .find(|medication| numeric(medication, "id") == numeric(record, "medication_id"))
        .map(|medication| field(medication, "name").to_owned())
        .unwrap_or_else(|| {
            numeric(record, "medication_id")
                .unwrap_or_default()
                .to_string()
        })
}

fn render(
    context: Context,
    location: FormLocation,
    action: &str,
    draft: TreatmentDraft,
    errors: Errors,
    status: StatusCode,
    medication_name: String,
) -> Response {
    let page = TreatmentFormPage {
        household_name: context.household_name,
        slug: location.slug.clone(),
        person_id: location.person.clone(),
        person_name: field(&context.person, "name").to_owned(),
        csrf: context.api.csrf,
        locale: context.api.locale,
        action: format!(
            "{}/{}/{}",
            base(&location.slug, &location.person, location.kind),
            path_segment(location.id.as_deref().unwrap_or_default()),
            action
        ),
        editing: true,
        schedule: matches!(location.kind, Kind::Schedule),
        medications: Vec::new(),
        dosages: Vec::new(),
        units: Vec::new(),
        draft,
        errors,
    };
    match medtracker_web::treatments::render_pause_form(page, action == "resume", medication_name) {
        Ok(body) => page_status(body, context.api.cookie, status),
        Err(_) => failure(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn date(value: &Value) -> Option<String> {
    value
        .as_str()
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
        .map(|date| {
            date.with_timezone(&crate::web_pages::time::configured_timezone())
                .format("%Y-%m-%d %H:%M %Z")
                .to_string()
        })
}

pub(super) async fn show(
    State(state): State<AppState>,
    Path((slug, person, resource, id, action)): Path<(String, String, String, String, String)>,
    headers: HeaderMap,
) -> Response {
    let kind = match kind(&resource) {
        Ok(kind) => kind,
        Err(error) => return error.response(),
    };
    if !matches!(action.as_str(), "pause" | "resume" | "history") {
        return failure(StatusCode::NOT_FOUND);
    }
    let mut context = match context::load_read(state, headers, &slug, &person).await {
        Ok(context) => context,
        Err(error) => return error.response(),
    };
    if action != "history" && !context.can_manage {
        return failure(StatusCode::FORBIDDEN);
    }
    let reply = match source(&mut context, kind, &id).await {
        Ok(reply) => reply,
        Err(error) => return error.response(),
    };
    let record = reply.value["data"].clone();
    let medication_name = medication_name(&context, &record);
    let rows = match periods(&mut context, kind, field(&record, "portable_id")).await {
        Ok(rows) => rows,
        Err(error) => return error.response(),
    };
    if action == "history" {
        let history = PauseHistoryPage {
            household_name: context.household_name,
            slug,
            person_id: person,
            medication_name,
            locale: context.api.locale,
            rows: rows
                .into_iter()
                .map(|row| PauseHistoryRow {
                    reason: field(&row, "reason").into(),
                    note: field(&row, "note").into(),
                    started_at: date(&row["started_at"]),
                    ended_at: date(&row["ended_at"]),
                    recorded_by: row["recorded_by_name"].as_str().map(str::to_owned),
                    resumed_by: row["resumed_by_name"].as_str().map(str::to_owned),
                })
                .collect(),
        };
        return match medtracker_web::treatments::render_pause_history(history) {
            Ok(body) => page(body, context.api.cookie),
            Err(_) => failure(StatusCode::INTERNAL_SERVER_ERROR),
        };
    }
    let mut draft = TreatmentDraft {
        fields: BTreeMap::from([
            ("etag".into(), reply.etag.unwrap_or_default()),
            ("source_type".into(), kind.body_key().into()),
            ("source_id".into(), field(&record, "portable_id").into()),
            ("submission_id".into(), Uuid::new_v4().to_string()),
        ]),
    };
    if action == "resume" {
        let open = rows
            .iter()
            .filter(|row| row["ended_at"].is_null())
            .collect::<Vec<_>>();
        if open.len() != 1 {
            return failure(StatusCode::NOT_FOUND);
        }
        draft
            .fields
            .insert("pause_period_id".into(), field(open[0], "id").into());
        draft.fields.insert(
            "period_etag".into(),
            crate::representation_etag(&json!({"data": open[0]})),
        );
    }
    render(
        context,
        FormLocation {
            slug,
            person,
            id: Some(id),
            kind,
        },
        &action,
        draft,
        Errors::new(),
        StatusCode::OK,
        medication_name,
    )
}

pub(super) async fn save(
    State(state): State<AppState>,
    Path((slug, person, resource, id, action)): Path<(String, String, String, String, String)>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    if !matches!(action.as_str(), "pause" | "resume") {
        return failure(StatusCode::NOT_FOUND);
    }
    if !oauth::trusted_cookie_origin(&state, &headers) {
        return failure(StatusCode::FORBIDDEN);
    }
    let kind = match kind(&resource) {
        Ok(kind) => kind,
        Err(error) => return error.response(),
    };
    let mut context = match load(state, headers, &slug, &person).await {
        Ok(context) => context,
        Err(error) => return error.response(),
    };
    if fields.get("authenticity_token") != Some(&context.api.csrf) {
        return failure(StatusCode::FORBIDDEN);
    }
    let reply = match source(&mut context, kind, &id).await {
        Ok(reply) => reply,
        Err(error) => return error.response(),
    };
    let record = reply.value["data"].clone();
    let medication_name = medication_name(&context, &record);
    let mut draft = TreatmentDraft {
        fields: fields.into_iter().collect(),
    };
    if Uuid::parse_str(draft.value("submission_id")).is_err() {
        return render(
            context,
            FormLocation {
                slug,
                person,
                id: Some(id),
                kind,
            },
            &action,
            draft,
            BTreeMap::from([("base".into(), vec!["missing_browser_precondition".into()])]),
            StatusCode::PRECONDITION_REQUIRED,
            medication_name,
        );
    }
    let mut extra = HeaderMap::new();
    if let Ok(key) = HeaderValue::from_str(draft.value("submission_id")) {
        extra.insert("idempotency-key", key);
    }
    let (path, body) = if action == "pause" {
        (
            format!(
                "/api/v1/households/{}/medication_pause_periods",
                context.household_id
            ),
            json!({"medication_pause_period": {
                "source_type": kind.body_key(), "source_id": field(&record, "portable_id"), "reason": draft.value("reason"), "note": draft.value("note")
            }}),
        )
    } else {
        let rows = match periods(&mut context, kind, field(&record, "portable_id")).await {
            Ok(rows) => rows,
            Err(error) => return error.response(),
        };
        if !rows.iter().any(|row| {
            field(row, "id") == draft.value("pause_period_id")
                && field(row, "source_id") == field(&record, "portable_id")
                && field(row, "source_type") == kind.body_key()
        }) {
            return failure(StatusCode::NOT_FOUND);
        }
        if let Ok(token) = HeaderValue::from_str(draft.value("period_etag")) {
            extra.insert(header::IF_MATCH, token);
        }
        (
            format!(
                "/api/v1/households/{}/medication_pause_periods/{}/resume",
                context.household_id,
                path_segment(draft.value("pause_period_id"))
            ),
            json!({}),
        )
    };
    let guard = crate::pause_lifecycle::BrowserSourceGuard {
        source_type: draft.value("source_type").into(),
        source_id: draft.value("source_id").into(),
        original_etag: draft.value("etag").into(),
    };
    let reply = match context
        .api
        .pause_with_original_source(&path, body, &extra, guard)
        .await
    {
        Ok(reply) => reply,
        Err(error) => return error.response(),
    };
    if reply.status.is_success() {
        return redirect(
            format!(
                "/households/{}/people/{}",
                path_segment(&slug),
                path_segment(&person)
            ),
            context.api.cookie,
        );
    }
    if matches!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY | StatusCode::CONFLICT | StatusCode::PRECONDITION_REQUIRED
    ) {
        let errors = writing::response_errors(&reply.value);
        if reply.status == StatusCode::UNPROCESSABLE_ENTITY {
            draft
                .fields
                .insert("submission_id".into(), Uuid::new_v4().to_string());
        }
        return render(
            context,
            FormLocation {
                slug,
                person,
                id: Some(id),
                kind,
            },
            &action,
            draft,
            errors,
            reply.status,
            medication_name,
        );
    }
    if reply.status == StatusCode::UNAUTHORIZED {
        return login_redirect();
    }
    page_status(String::new(), context.api.cookie, reply.status)
}
