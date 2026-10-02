use super::*;
use medtracker_web::household::path_segment;
use medtracker_web::household_i18n::{Text, TranslationError};
use medtracker_web::locations::{
    render_location_detail, render_location_form, render_location_list, LocationDetailPage,
    LocationDraft, LocationFormPage, LocationMedication, LocationRow, LocationsPage,
};

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/households/{slug}/locations", get(index).post(create))
        .route("/households/{slug}/locations/new", get(new))
        .route("/households/{slug}/locations/{id}", get(show))
        .route(
            "/households/{slug}/locations/{id}/edit",
            get(edit).post(update),
        )
}

fn location_row(value: &Value) -> Result<LocationRow, PageError> {
    Ok(LocationRow {
        id: numeric(value, "id")
            .filter(|id| *id > 0)
            .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?,
        name: value
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?
            .to_owned(),
        description: field(value, "description").to_owned(),
    })
}

fn location_page(
    body: Result<String, TranslationError>,
    cookie: Option<HeaderValue>,
    status: StatusCode,
) -> Response {
    match body {
        Ok(body) => page_status(body, cookie, status),
        Err(_) => failure(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn valid_path_id(id: &str) -> bool {
    id.parse::<i64>().is_ok_and(|id| id > 0) || uuid::Uuid::parse_str(id).is_ok()
}

async fn index(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    let mut api = match WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let (household_id, household_name) = match api.household(&slug).await {
        Ok(value) => value,
        Err(error) => return error.response(),
    };
    let capabilities = match api.capabilities(household_id).await {
        Ok(value) => value,
        Err(error) => return error.response(),
    };
    let Some(can_create) = capabilities
        .pointer("/data/locations/create")
        .and_then(Value::as_bool)
    else {
        return failure(StatusCode::BAD_GATEWAY);
    };
    let locations = match api
        .collection(&format!("/api/v1/households/{household_id}/locations"))
        .await
    {
        Ok(rows) => match rows.iter().map(location_row).collect::<Result<Vec<_>, _>>() {
            Ok(rows) => rows,
            Err(error) => return error.response(),
        },
        Err(error) => return error.response(),
    };
    location_page(
        render_location_list(LocationsPage {
            household_name,
            slug,
            locale: api.locale,
            can_create,
            locations,
        }),
        api.cookie,
        StatusCode::OK,
    )
}

async fn show(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if !valid_path_id(&id) {
        return failure(StatusCode::NOT_FOUND);
    }
    let mut api = match WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let (household_id, household_name) = match api.household(&slug).await {
        Ok(value) => value,
        Err(error) => return error.response(),
    };
    let value = match api
        .get(&format!("/api/v1/households/{household_id}/locations/{id}"))
        .await
    {
        Ok(value) => value,
        Err(error) => return error.response(),
    };
    let location = match location_row(&value["data"]) {
        Ok(value) => value,
        Err(error) => return error.response(),
    };
    let capabilities = match api.capabilities(household_id).await {
        Ok(value) => value,
        Err(error) => return error.response(),
    };
    let Some(can_update) = capabilities
        .pointer("/data/locations/update")
        .and_then(Value::as_bool)
    else {
        return failure(StatusCode::BAD_GATEWAY);
    };
    let rows = match api
        .collection(&format!("/api/v1/households/{household_id}/medications"))
        .await
    {
        Ok(value) => value,
        Err(error) => return error.response(),
    };
    let medications = rows
        .iter()
        .filter(|row| numeric(row, "location_id") == Some(location.id))
        .map(|row| {
            Ok(LocationMedication {
                id: numeric(row, "id").ok_or_else(|| error(StatusCode::BAD_GATEWAY))?,
                name: row
                    .get("display_name")
                    .and_then(Value::as_str)
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| field(row, "name"))
                    .to_owned(),
                supply: field(row, "current_supply").to_owned(),
                unit: field(row, "dose_unit").to_owned(),
            })
        })
        .collect::<Result<Vec<_>, PageError>>();
    let medications = match medications {
        Ok(value) => value,
        Err(error) => return error.response(),
    };
    location_page(
        render_location_detail(LocationDetailPage {
            household_name,
            slug,
            locale: api.locale,
            location,
            can_update,
            medications,
            notice: String::new(),
        }),
        api.cookie,
        StatusCode::OK,
    )
}

async fn new(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    form(state, slug, None, headers).await
}

async fn edit(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    form(state, slug, Some(id), headers).await
}

async fn form(state: AppState, slug: String, id: Option<String>, headers: HeaderMap) -> Response {
    if id.as_deref().is_some_and(|id| !valid_path_id(id)) {
        return failure(StatusCode::NOT_FOUND);
    }
    let mut api = match WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let (household_id, household_name) = match api.household(&slug).await {
        Ok(value) => value,
        Err(error) => return error.response(),
    };
    let capabilities = match api.capabilities(household_id).await {
        Ok(value) => value,
        Err(error) => return error.response(),
    };
    let action = if id.is_some() { "update" } else { "create" };
    match capabilities
        .pointer(&format!("/data/locations/{action}"))
        .and_then(Value::as_bool)
    {
        Some(true) => {}
        Some(false) => return failure(StatusCode::FORBIDDEN),
        None => return failure(StatusCode::BAD_GATEWAY),
    }
    let mut draft = LocationDraft {
        idempotency_key: uuid::Uuid::new_v4().to_string(),
        ..Default::default()
    };
    if let Some(id) = &id {
        let reply = match api
            .get_reply(&format!("/api/v1/households/{household_id}/locations/{id}"))
            .await
        {
            Ok(reply) => reply,
            Err(error) => return error.response(),
        };
        let location = match location_row(&reply.value["data"]) {
            Ok(value) => value,
            Err(error) => return error.response(),
        };
        let Some(etag) = reply.etag else {
            return failure(StatusCode::BAD_GATEWAY);
        };
        draft.name = location.name;
        draft.description = location.description;
        draft.etag = etag;
    }
    draft_response(
        &mut api,
        household_name,
        slug,
        id,
        draft,
        HashMap::new(),
        StatusCode::OK,
    )
}

fn draft_response(
    api: &mut WebApi,
    household_name: String,
    slug: String,
    id: Option<String>,
    draft: LocationDraft,
    errors: HashMap<String, Vec<String>>,
    status: StatusCode,
) -> Response {
    let prefix = format!("/households/{}/locations", path_segment(&slug));
    let action = id
        .as_ref()
        .map(|id| format!("{prefix}/{id}/edit"))
        .unwrap_or(prefix);
    let key = if id.is_some() {
        "locations.show.edit_location"
    } else {
        "forms.locations.new_title"
    };
    let title = match Text::new(api.locale).get(key, &[]) {
        Ok(value) => value,
        Err(_) => return failure(StatusCode::INTERNAL_SERVER_ERROR),
    };
    location_page(
        render_location_form(LocationFormPage {
            household_name,
            slug,
            locale: api.locale,
            csrf: api.csrf.clone(),
            action,
            title,
            draft,
            errors,
        }),
        api.cookie.take(),
        status,
    )
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
    if id.as_deref().is_some_and(|id| !valid_path_id(id)) {
        return failure(StatusCode::NOT_FOUND);
    }
    let mut api = match WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    if fields.get("authenticity_token") != Some(&api.csrf) {
        return failure(StatusCode::FORBIDDEN);
    }
    let (household_id, household_name) = match api.household(&slug).await {
        Ok(value) => value,
        Err(error) => return error.response(),
    };
    let draft = LocationDraft {
        name: fields.get("name").cloned().unwrap_or_default(),
        description: fields.get("description").cloned().unwrap_or_default(),
        etag: fields.get("etag").cloned().unwrap_or_default(),
        idempotency_key: fields
            .get("idempotency_key")
            .cloned()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
    };
    let mut extra = HeaderMap::new();
    let key = match HeaderValue::from_str(&draft.idempotency_key) {
        Ok(value) => value,
        Err(_) => return failure(StatusCode::BAD_REQUEST),
    };
    extra.insert("idempotency-key", key);
    let path = if let Some(id) = &id {
        let tag = match HeaderValue::from_str(&draft.etag) {
            Ok(value) => value,
            Err(_) => return failure(StatusCode::BAD_REQUEST),
        };
        extra.insert(header::IF_MATCH, tag);
        format!("/api/v1/households/{household_id}/locations/{id}")
    } else {
        format!("/api/v1/households/{household_id}/locations")
    };
    let method = if id.is_some() {
        Method::PATCH
    } else {
        Method::POST
    };
    let csrf = api.csrf.clone();
    let body = json!({"location": {"name": draft.name, "description": draft.description}});
    let reply = match api
        .call_with_headers(method, &path, Some(body), Some(&csrf), &extra)
        .await
    {
        Ok(reply) => reply,
        Err(error) => return error.response(),
    };
    if reply.status.is_success() {
        let location = match location_row(&reply.value["data"]) {
            Ok(value) => value,
            Err(error) => return error.response(),
        };
        return redirect(
            format!(
                "/households/{}/locations/{}",
                path_segment(&slug),
                location.id
            ),
            api.cookie,
        );
    }
    if reply.status == StatusCode::UNAUTHORIZED {
        return login_redirect();
    }
    if !matches!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY | StatusCode::CONFLICT | StatusCode::PRECONDITION_REQUIRED
    ) {
        return failure(reply.status);
    }
    let mut errors: HashMap<String, Vec<String>> = reply
        .value
        .pointer("/error/errors")
        .and_then(Value::as_object)
        .map(|fields| {
            fields
                .iter()
                .map(|(field, messages)| {
                    (
                        field.clone(),
                        messages
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    if errors.is_empty() {
        let message = reply
            .value
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("The location could not be saved.");
        errors.insert("base".to_owned(), vec![message.to_owned()]);
    }
    draft_response(
        &mut api,
        household_name,
        slug,
        id,
        draft,
        errors,
        reply.status,
    )
}
