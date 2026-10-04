use super::*;
use axum::extract::Query;
use medtracker_web::dosage_options::{
    render_dosage_form, render_dosage_list, DosageDraft, DosageFormPage, DosageListPage,
    DosageOption,
};
use medtracker_web::household::path_segment;
use medtracker_web::household_i18n::Text;
use std::collections::BTreeMap;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/households/{slug}/medications/{medication_id}/dosage_options",
            get(index).post(create),
        )
        .route(
            "/households/{slug}/medications/{medication_id}/dosage_options/new",
            get(new),
        )
        .route(
            "/households/{slug}/medications/{medication_id}/dosage_options/{id}/edit",
            get(edit),
        )
        .route(
            "/households/{slug}/medications/{medication_id}/dosage_options/{id}",
            post(update),
        )
}

struct Context {
    api: WebApi,
    household_id: i64,
    household_name: String,
    medication: Value,
    can_manage: bool,
    notifications_visible: bool,
}

async fn context(
    state: AppState,
    headers: HeaderMap,
    slug: &str,
    parent: &str,
) -> Result<Context, PageError> {
    let mut api = WebApi::authenticated(state, headers).await?;
    let (household_id, household_name) = api.household(slug).await?;
    let medication = api
        .get(&format!(
            "/api/v1/households/{household_id}/medications/{}",
            path_segment(parent)
        ))
        .await?["data"]
        .clone();
    if numeric(&medication, "id").is_none() {
        return Err(error(StatusCode::BAD_GATEWAY));
    }
    let capabilities = api.capabilities(household_id).await?;
    let can_manage = capabilities
        .pointer("/data/medications/update")
        .and_then(Value::as_bool)
        .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
    let notifications_visible = api.notifications_visible(household_id).await;
    Ok(Context {
        api,
        household_id,
        household_name,
        medication,
        can_manage,
        notifications_visible,
    })
}

async fn options(context: &mut Context) -> Result<Vec<Value>, PageError> {
    let rows = context
        .api
        .collection(&format!(
            "/api/v1/households/{}/dosage_options",
            context.household_id
        ))
        .await?;
    Ok(rows
        .into_iter()
        .filter(|row| numeric(row, "medication_id") == numeric(&context.medication, "id"))
        .collect())
}

async fn option(context: &mut Context, id: &str) -> Result<api_client::ApiReply, PageError> {
    let reply = context
        .api
        .get_reply(&format!(
            "/api/v1/households/{}/dosage_options/{}",
            context.household_id,
            path_segment(id)
        ))
        .await?;
    if numeric(&reply.value["data"], "medication_id") != numeric(&context.medication, "id") {
        return Err(error(StatusCode::NOT_FOUND));
    }
    if reply.etag.as_deref().is_none_or(str::is_empty) {
        return Err(error(StatusCode::BAD_GATEWAY));
    }
    Ok(reply)
}

fn draft_from_record(row: &Value, etag: String) -> DosageDraft {
    let value = |key: &str| field(row, key).to_owned();
    DosageDraft {
        amount: value("amount"),
        unit: value("unit"),
        frequency: value("frequency"),
        description: value("description"),
        default_for_adults: row["default_for_adults"].as_bool().unwrap_or(false),
        default_for_children: row["default_for_children"].as_bool().unwrap_or(false),
        default_max_daily_doses: numeric(row, "default_max_daily_doses")
            .map(|number| number.to_string())
            .unwrap_or_default(),
        default_min_hours_between_doses: value("default_min_hours_between_doses"),
        default_dose_cycle: value("default_dose_cycle"),
        current_supply: value("current_supply"),
        reorder_threshold: value("reorder_threshold"),
        etag,
        confirm_option_mode: false,
    }
}

fn draft_from_fields(fields: &HashMap<String, String>) -> DosageDraft {
    let value = |key: &str| fields.get(key).cloned().unwrap_or_default();
    DosageDraft {
        amount: value("amount"),
        unit: value("unit"),
        frequency: value("frequency"),
        description: value("description"),
        default_for_adults: value("default_for_adults") == "true",
        default_for_children: value("default_for_children") == "true",
        default_max_daily_doses: value("default_max_daily_doses"),
        default_min_hours_between_doses: value("default_min_hours_between_doses"),
        default_dose_cycle: value("default_dose_cycle"),
        current_supply: value("current_supply"),
        reorder_threshold: value("reorder_threshold"),
        etag: value("etag"),
        confirm_option_mode: value("confirm_option_mode") == "true",
    }
}

fn payload(
    draft: &DosageDraft,
    parent: i64,
    creating: bool,
) -> Result<Value, BTreeMap<String, Vec<String>>> {
    let maximum = draft.default_max_daily_doses.parse::<i32>().map_err(|_| {
        BTreeMap::from([("default_max_daily_doses".into(), vec!["is invalid".into()])])
    })?;
    let nullable = |value: &str| {
        if value.is_empty() {
            Value::Null
        } else {
            json!(value)
        }
    };
    let mut body = json!({"dosage_option": {
        "amount": draft.amount, "unit": draft.unit, "frequency": draft.frequency, "description": draft.description,
        "default_for_adults": draft.default_for_adults, "default_for_children": draft.default_for_children,
        "default_max_daily_doses": maximum, "default_min_hours_between_doses": draft.default_min_hours_between_doses,
        "default_dose_cycle": draft.default_dose_cycle, "current_supply": nullable(&draft.current_supply), "reorder_threshold": nullable(&draft.reorder_threshold),
    }});
    if creating {
        body["dosage_option"]["medication_id"] = json!(parent.to_string());
    }
    Ok(body)
}

fn response_errors(value: &Value) -> BTreeMap<String, Vec<String>> {
    let fields = value.pointer("/error/errors").and_then(Value::as_object);
    let errors: BTreeMap<_, _> = fields
        .into_iter()
        .flat_map(|fields| fields.iter())
        .map(|(name, messages)| {
            let messages = messages
                .as_array()
                .map(|messages| {
                    messages
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_else(|| vec![messages.as_str().unwrap_or("is invalid").to_owned()]);
            (name.clone(), messages)
        })
        .collect();
    if errors.is_empty() {
        BTreeMap::from([(
            "dosage_option".into(),
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

fn render_form(
    context: Context,
    slug: String,
    id: Option<String>,
    draft: DosageDraft,
    first_option: bool,
    errors: BTreeMap<String, Vec<String>>,
    status: StatusCode,
) -> Response {
    let body = render_dosage_form(DosageFormPage {
        household_name: context.household_name,
        slug,
        medication_id: numeric(&context.medication, "id")
            .expect("validated parent")
            .to_string(),
        medication_name: field(&context.medication, "name").to_owned(),
        csrf: context.api.csrf,
        locale: context.api.locale,
        option_id: id,
        draft,
        first_option,
        errors,
        notifications_visible: context.notifications_visible,
    });
    match body {
        Ok(body) => page_status(body, context.api.cookie, status),
        Err(_) => failure(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn index(
    State(state): State<AppState>,
    Path((slug, parent)): Path<(String, String)>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let mut context = match context(state, headers, &slug, &parent).await {
        Ok(context) => context,
        Err(response) => return response.response(),
    };
    let rows = match options(&mut context).await {
        Ok(rows) => rows,
        Err(response) => return response.response(),
    };
    let notice = ["created", "updated"].into_iter().find_map(|action| {
        let id = query.get(action)?;
        rows.iter()
            .any(|row| numeric(row, "id").map(|number| number.to_string()).as_ref() == Some(id))
            .then(|| {
                Text::new(context.api.locale)
                    .get(&format!("dosages.{action}"), &[])
                    .ok()
            })
            .flatten()
    });
    let body = render_dosage_list(DosageListPage {
        household_name: context.household_name,
        slug,
        medication_id: numeric(&context.medication, "id")
            .expect("validated parent")
            .to_string(),
        medication_name: field(&context.medication, "name").to_owned(),
        locale: context.api.locale,
        options: rows
            .into_iter()
            .map(|row| DosageOption {
                id: numeric(&row, "id").unwrap_or_default(),
                amount: field(&row, "amount").to_owned(),
                unit: field(&row, "unit").to_owned(),
                frequency: field(&row, "frequency").to_owned(),
                description: field(&row, "description").to_owned(),
                current_supply: row["current_supply"].as_str().map(str::to_owned),
                reorder_threshold: row["reorder_threshold"].as_str().map(str::to_owned),
                default_for_adults: row["default_for_adults"].as_bool().unwrap_or(false),
                default_for_children: row["default_for_children"].as_bool().unwrap_or(false),
            })
            .collect(),
        can_manage: context.can_manage,
        notice,
        notifications_visible: context.notifications_visible,
    });
    match body {
        Ok(body) => page_status(body, context.api.cookie, StatusCode::OK),
        Err(_) => failure(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn new(
    State(state): State<AppState>,
    Path((slug, parent)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let mut context = match context(state, headers, &slug, &parent).await {
        Ok(context) => context,
        Err(response) => return response.response(),
    };
    if !context.can_manage {
        return failure(StatusCode::FORBIDDEN);
    }
    let first = match options(&mut context).await {
        Ok(rows) => rows.is_empty(),
        Err(response) => return response.response(),
    };
    render_form(
        context,
        slug,
        None,
        DosageDraft {
            default_max_daily_doses: "1".into(),
            default_min_hours_between_doses: "0".into(),
            default_dose_cycle: "daily".into(),
            ..Default::default()
        },
        first,
        BTreeMap::new(),
        StatusCode::OK,
    )
}

async fn edit(
    State(state): State<AppState>,
    Path((slug, parent, id)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Response {
    let mut context = match context(state, headers, &slug, &parent).await {
        Ok(context) => context,
        Err(response) => return response.response(),
    };
    if !context.can_manage {
        return failure(StatusCode::FORBIDDEN);
    }
    let reply = match option(&mut context, &id).await {
        Ok(reply) => reply,
        Err(response) => return response.response(),
    };
    let draft = draft_from_record(&reply.value["data"], reply.etag.unwrap_or_default());
    render_form(
        context,
        slug,
        Some(id),
        draft,
        false,
        BTreeMap::new(),
        StatusCode::OK,
    )
}

async fn create(
    State(state): State<AppState>,
    Path((slug, parent)): Path<(String, String)>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    save(state, slug, parent, None, headers, fields).await
}

async fn update(
    State(state): State<AppState>,
    Path((slug, parent, id)): Path<(String, String, String)>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    save(state, slug, parent, Some(id), headers, fields).await
}

async fn save(
    state: AppState,
    slug: String,
    parent: String,
    id: Option<String>,
    headers: HeaderMap,
    fields: HashMap<String, String>,
) -> Response {
    if !oauth::trusted_cookie_origin(&state, &headers) {
        return failure(StatusCode::FORBIDDEN);
    }
    let mut context = match context(state, headers, &slug, &parent).await {
        Ok(context) => context,
        Err(response) => return response.response(),
    };
    if fields.get("authenticity_token") != Some(&context.api.csrf) || !context.can_manage {
        return failure(StatusCode::FORBIDDEN);
    }
    let draft = draft_from_fields(&fields);
    let creating = id.is_none();
    let first = if let Some(id) = id.as_ref() {
        if let Err(response) = option(&mut context, id).await {
            return response.response();
        }
        if draft.etag.trim().is_empty() {
            return render_form(
                context,
                slug,
                Some(id.clone()),
                draft,
                false,
                BTreeMap::from([(
                    "dosage_option".into(),
                    vec!["missing_browser_precondition".into()],
                )]),
                StatusCode::PRECONDITION_REQUIRED,
            );
        }
        false
    } else {
        match options(&mut context).await {
            Ok(rows) => rows.is_empty(),
            Err(response) => return response.response(),
        }
    };
    if first && !draft.confirm_option_mode {
        return render_form(
            context,
            slug,
            id,
            draft,
            true,
            BTreeMap::from([(
                "confirm_option_mode".into(),
                vec!["confirm_option_mode".into()],
            )]),
            StatusCode::UNPROCESSABLE_ENTITY,
        );
    }
    let body = match payload(
        &draft,
        numeric(&context.medication, "id").expect("validated parent"),
        creating,
    ) {
        Ok(body) => body,
        Err(errors) => {
            return render_form(
                context,
                slug,
                id,
                draft,
                first,
                errors,
                StatusCode::UNPROCESSABLE_ENTITY,
            )
        }
    };
    let base = format!("/api/v1/households/{}/dosage_options", context.household_id);
    let path = id
        .as_ref()
        .map_or_else(|| base.clone(), |id| format!("{base}/{}", path_segment(id)));
    let mut extra = HeaderMap::new();
    if !creating {
        let etag = match HeaderValue::from_str(&draft.etag) {
            Ok(etag) => etag,
            Err(_) => {
                return render_form(
                    context,
                    slug,
                    id,
                    draft,
                    first,
                    BTreeMap::from([(
                        "dosage_option".into(),
                        vec!["missing_browser_precondition".into()],
                    )]),
                    StatusCode::BAD_REQUEST,
                )
            }
        };
        extra.insert(header::IF_MATCH, etag);
    }
    let csrf = context.api.csrf.clone();
    let reply = match context
        .api
        .call_with_headers(
            if creating {
                Method::POST
            } else {
                Method::PATCH
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
        let Some(saved_id) = numeric(&reply.value["data"], "id").filter(|id| *id > 0) else {
            return failure(StatusCode::BAD_GATEWAY);
        };
        if let Err(response) = option(&mut context, &saved_id.to_string()).await {
            return response.response();
        }
        return redirect(
            format!(
                "/households/{}/medications/{}/dosage_options?{}={saved_id}",
                path_segment(&slug),
                numeric(&context.medication, "id").expect("validated parent"),
                if creating { "created" } else { "updated" }
            ),
            context.api.cookie,
        );
    }
    if matches!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY | StatusCode::CONFLICT
    ) {
        return render_form(
            context,
            slug,
            id,
            draft,
            first,
            response_errors(&reply.value),
            reply.status,
        );
    }
    if reply.status == StatusCode::UNAUTHORIZED {
        return login_redirect();
    }
    page_status(String::new(), context.api.cookie, reply.status)
}
