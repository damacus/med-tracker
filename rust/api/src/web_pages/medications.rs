use super::*;
use medtracker_web::household::path_segment;
use medtracker_web::medication_management::{
    render_medication_form_with_options, MedicationDraft, MedicationFormPage,
};
use std::collections::BTreeMap;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/households/{slug}/medications/new", get(new))
        .route("/households/{slug}/medications", post(create))
        .route("/households/{slug}/medications/{id}/edit", get(edit))
        .route("/households/{slug}/medications/{id}", post(update))
}

async fn context(
    state: AppState,
    headers: HeaderMap,
    slug: &str,
) -> Result<(WebApi, i64, String), PageError> {
    let mut api = WebApi::authenticated(state, headers).await?;
    let (household_id, household_name) = api.household(slug).await?;
    Ok((api, household_id, household_name))
}

async fn permitted(api: &mut WebApi, household_id: i64, action: &str) -> Result<(), PageError> {
    let capabilities = api.capabilities(household_id).await?;
    if capabilities
        .pointer(&format!("/data/medications/{action}"))
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err(error(StatusCode::FORBIDDEN));
    }
    Ok(())
}

fn uses_dosage_options(medication: &Value, options: &[Value]) -> bool {
    numeric(medication, "id").is_some_and(|id| {
        options
            .iter()
            .any(|option| numeric(option, "medication_id") == Some(id))
    })
}

fn edit_precondition(
    fields: &HashMap<String, String>,
    current_etag: &str,
    options_mode: bool,
) -> Option<StatusCode> {
    if fields.get("etag").is_none_or(String::is_empty) {
        Some(StatusCode::PRECONDITION_REQUIRED)
    } else if fields.get("etag").is_some_and(|etag| etag != current_etag) {
        Some(StatusCode::CONFLICT)
    } else if options_mode
        && ["dose_amount", "dose_unit", "current_supply"]
            .iter()
            .any(|key| fields.contains_key(*key))
    {
        Some(StatusCode::BAD_REQUEST)
    } else {
        None
    }
}

async fn options_mode(
    api: &mut WebApi,
    household_id: i64,
    medication: &Value,
) -> Result<bool, PageError> {
    let options = api
        .collection(&format!("/api/v1/households/{household_id}/dosage_options"))
        .await?;
    Ok(uses_dosage_options(medication, &options))
}

fn draft_from_record(row: &Value, etag: Option<String>) -> MedicationDraft {
    MedicationDraft {
        name: field(row, "name").to_owned(),
        friendly_name: field(row, "friendly_name").to_owned(),
        description: field(row, "description").to_owned(),
        barcode: field(row, "barcode").to_owned(),
        dose_amount: field(row, "dose_amount").to_owned(),
        dose_unit: field(row, "dose_unit").to_owned(),
        current_supply: field(row, "current_supply").to_owned(),
        reorder_threshold: field(row, "reorder_threshold").to_owned(),
        location_id: numeric(row, "location_id")
            .map(|id| id.to_string())
            .unwrap_or_default(),
        warnings: field(row, "warnings").to_owned(),
        etag: etag.unwrap_or_default(),
    }
}

fn draft_from_fields(fields: &HashMap<String, String>) -> MedicationDraft {
    let value = |key: &str| fields.get(key).cloned().unwrap_or_default();
    MedicationDraft {
        name: value("name"),
        friendly_name: value("friendly_name"),
        description: value("description"),
        barcode: value("barcode"),
        dose_amount: value("dose_amount"),
        dose_unit: value("dose_unit"),
        current_supply: value("current_supply"),
        reorder_threshold: value("reorder_threshold"),
        location_id: value("location_id"),
        warnings: value("warnings"),
        etag: value("etag"),
    }
}

fn payload(draft: &MedicationDraft, options_mode: bool) -> Option<Value> {
    let location_id = draft.location_id.parse::<i64>().ok().filter(|id| *id > 0)?;
    let mut attributes = serde_json::Map::new();
    for (name, value) in [
        ("name", &draft.name),
        ("friendly_name", &draft.friendly_name),
        ("description", &draft.description),
        ("barcode", &draft.barcode),
        ("warnings", &draft.warnings),
    ] {
        attributes.insert(name.to_owned(), Value::String(value.clone()));
    }
    attributes.insert("location_id".to_owned(), json!(location_id));
    attributes.insert(
        "reorder_threshold".to_owned(),
        json!(draft.reorder_threshold),
    );
    if !options_mode {
        attributes.insert("dose_unit".to_owned(), json!(draft.dose_unit));
        for (name, value) in [
            ("dose_amount", &draft.dose_amount),
            ("current_supply", &draft.current_supply),
        ] {
            attributes.insert(
                name.to_owned(),
                if value.is_empty() {
                    Value::Null
                } else {
                    json!(value)
                },
            );
        }
    }
    Some(json!({"medication": attributes}))
}

fn response_errors(value: &Value) -> BTreeMap<String, Vec<String>> {
    let errors: BTreeMap<String, Vec<String>> = value
        .pointer("/error/errors")
        .and_then(Value::as_object)
        .map(|fields| {
            fields
                .iter()
                .map(|(name, values)| {
                    let messages = values
                        .as_array()
                        .map(|rows| {
                            rows.iter()
                                .filter_map(Value::as_str)
                                .map(str::to_owned)
                                .collect()
                        })
                        .unwrap_or_else(|| {
                            vec![values.as_str().unwrap_or("is invalid").to_owned()]
                        });
                    (name.clone(), messages)
                })
                .collect()
        })
        .unwrap_or_default();
    if errors.is_empty() {
        BTreeMap::from([(
            "medication".to_owned(),
            vec![value
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("is invalid")
                .to_owned()],
        )])
    } else {
        errors
    }
}

struct FormState {
    slug: String,
    household_id: i64,
    household_name: String,
    medication_id: Option<String>,
    draft: MedicationDraft,
    errors: BTreeMap<String, Vec<String>>,
    options_mode: bool,
    status: StatusCode,
}

async fn render_form(mut api: WebApi, form: FormState) -> Response {
    let locations = match api
        .collection(&format!(
            "/api/v1/households/{}/locations",
            form.household_id
        ))
        .await
    {
        Ok(rows) => rows
            .into_iter()
            .filter_map(|row| {
                Some((
                    numeric(&row, "id")?.to_string(),
                    field(&row, "name").to_owned(),
                ))
            })
            .collect(),
        Err(response) => return response.response(),
    };
    let body = match render_medication_form_with_options(
        MedicationFormPage {
            household_name: form.household_name,
            slug: form.slug,
            csrf: api.csrf,
            locale: api.locale,
            medication_id: form.medication_id,
            draft: form.draft,
            locations,
            errors: form.errors,
        },
        form.options_mode,
    ) {
        Ok(body) => body,
        Err(_) => return failure(StatusCode::INTERNAL_SERVER_ERROR),
    };
    page_status(body, api.cookie, form.status)
}

async fn new(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    let (mut api, household_id, household_name) = match context(state, headers, &slug).await {
        Ok(context) => context,
        Err(response) => return response.response(),
    };
    if let Err(response) = permitted(&mut api, household_id, "create").await {
        return response.response();
    }
    render_form(
        api,
        FormState {
            slug,
            household_id,
            household_name,
            medication_id: None,
            draft: MedicationDraft {
                current_supply: "0".into(),
                reorder_threshold: "0".into(),
                dose_unit: "tablet".into(),
                ..Default::default()
            },
            errors: BTreeMap::new(),
            options_mode: false,
            status: StatusCode::OK,
        },
    )
    .await
}

async fn edit(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let (mut api, household_id, household_name) = match context(state, headers, &slug).await {
        Ok(context) => context,
        Err(response) => return response.response(),
    };
    if let Err(response) = permitted(&mut api, household_id, "update").await {
        return response.response();
    }
    let reply = match api
        .get_reply(&format!(
            "/api/v1/households/{household_id}/medications/{}",
            path_segment(&id)
        ))
        .await
    {
        Ok(reply) => reply,
        Err(response) => return response.response(),
    };
    if !reply.status.is_success() {
        return page_status(String::new(), api.cookie, reply.status);
    }
    let Some(row) = reply.value.get("data").filter(|row| row.is_object()) else {
        return failure(StatusCode::BAD_GATEWAY);
    };
    if reply.etag.as_deref().is_none_or(str::is_empty) {
        return failure(StatusCode::BAD_GATEWAY);
    }
    let options_mode = match options_mode(&mut api, household_id, row).await {
        Ok(mode) => mode,
        Err(response) => return response.response(),
    };
    let draft = draft_from_record(row, reply.etag);
    render_form(
        api,
        FormState {
            slug,
            household_id,
            household_name,
            medication_id: Some(id),
            draft,
            errors: BTreeMap::new(),
            options_mode,
            status: StatusCode::OK,
        },
    )
    .await
}

async fn create(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    save(state, slug, None, headers, fields).await
}

async fn update(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    save(state, slug, Some(id), headers, fields).await
}

async fn save(
    state: AppState,
    slug: String,
    id: Option<String>,
    headers: HeaderMap,
    fields: HashMap<String, String>,
) -> Response {
    if !oauth::trusted_cookie_origin(&state, &headers) {
        return failure(StatusCode::FORBIDDEN);
    }
    let (mut api, household_id, household_name) = match context(state, headers, &slug).await {
        Ok(context) => context,
        Err(response) => return response.response(),
    };
    if fields.get("authenticity_token") != Some(&api.csrf) {
        return failure(StatusCode::FORBIDDEN);
    }
    let action = if id.is_some() { "update" } else { "create" };
    if let Err(response) = permitted(&mut api, household_id, action).await {
        return response.response();
    }
    let base = format!("/api/v1/households/{household_id}/medications");
    let path = id
        .as_ref()
        .map_or_else(|| base.clone(), |id| format!("{base}/{}", path_segment(id)));
    let mut current_etag = String::new();
    let options_mode = if id.is_some() {
        let reply = match api.get_reply(&path).await {
            Ok(reply) => reply,
            Err(response) => return response.response(),
        };
        if !reply.status.is_success() {
            return page_status(String::new(), api.cookie, reply.status);
        }
        let Some(etag) = reply.etag.as_deref().filter(|etag| !etag.is_empty()) else {
            return failure(StatusCode::BAD_GATEWAY);
        };
        current_etag = etag.to_owned();
        let Some(row) = reply.value.get("data").filter(|row| row.is_object()) else {
            return failure(StatusCode::BAD_GATEWAY);
        };
        match options_mode(&mut api, household_id, row).await {
            Ok(mode) => mode,
            Err(response) => return response.response(),
        }
    } else {
        false
    };
    if id.is_some() {
        if let Some(status) = edit_precondition(&fields, &current_etag, options_mode) {
            if status == StatusCode::CONFLICT {
                let original_options_mode = !fields.contains_key("dose_amount");
                return render_form(
                    api,
                    FormState {
                        slug,
                        household_id,
                        household_name,
                        medication_id: id,
                        draft: draft_from_fields(&fields),
                        errors: BTreeMap::from([(
                            "medication".to_owned(),
                            vec!["Record has changed since it was last read".to_owned()],
                        )]),
                        options_mode: original_options_mode,
                        status,
                    },
                )
                .await;
            }
            return failure(status);
        }
    }
    let draft = draft_from_fields(&fields);
    let Some(body) = payload(&draft, options_mode) else {
        return render_form(
            api,
            FormState {
                slug,
                household_id,
                household_name,
                medication_id: id,
                draft,
                errors: BTreeMap::from([("location_id".into(), vec!["is invalid".into()])]),
                options_mode,
                status: StatusCode::UNPROCESSABLE_ENTITY,
            },
        )
        .await;
    };
    let mut extra = HeaderMap::new();
    if id.is_some() {
        let Ok(etag) = HeaderValue::from_str(&draft.etag) else {
            return failure(StatusCode::BAD_REQUEST);
        };
        extra.insert(header::IF_MATCH, etag);
    }
    let csrf = api.csrf.clone();
    let reply = match api
        .call_with_headers(
            if id.is_some() {
                Method::PATCH
            } else {
                Method::POST
            },
            &path,
            Some(body),
            Some(&csrf),
            &extra,
        )
        .await
    {
        Ok(reply) => reply,
        Err(response) => return response.response(),
    };
    if reply.status.is_success() {
        let Some(saved_id) = reply
            .value
            .pointer("/data/id")
            .and_then(Value::as_i64)
            .filter(|id| *id > 0)
        else {
            return failure(StatusCode::BAD_GATEWAY);
        };
        let persisted = match api.get(&format!("{base}/{saved_id}")).await {
            Ok(persisted) => persisted,
            Err(response) => return response.response(),
        };
        if persisted.pointer("/data/id").and_then(Value::as_i64) != Some(saved_id) {
            return failure(StatusCode::BAD_GATEWAY);
        }
        return redirect(
            format!("/households/{}/medications", path_segment(&slug)),
            api.cookie,
        );
    }
    if matches!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY | StatusCode::CONFLICT
    ) {
        return render_form(
            api,
            FormState {
                slug,
                household_id,
                household_name,
                medication_id: id,
                draft,
                errors: response_errors(&reply.value),
                options_mode,
                status: reply.status,
            },
        )
        .await;
    }
    if reply.status == StatusCode::UNAUTHORIZED {
        return login_redirect();
    }
    page_status(String::new(), api.cookie, reply.status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_dose_without_options_stays_editable() {
        let medication = json!({"id": 7, "dose_amount": null});
        assert!(!uses_dosage_options(&medication, &[]));
        assert!(!uses_dosage_options(
            &medication,
            &[json!({"medication_id": 8})]
        ));
        assert!(uses_dosage_options(
            &medication,
            &[json!({"medication_id": 7})]
        ));
    }

    #[test]
    fn changed_edit_version_takes_precedence_over_new_options_mode() {
        let fields = HashMap::from([
            ("etag".to_owned(), "old".to_owned()),
            ("dose_amount".to_owned(), "2.50".to_owned()),
        ]);
        assert_eq!(
            super::edit_precondition(&fields, "new", true),
            Some(StatusCode::CONFLICT)
        );
    }
}
