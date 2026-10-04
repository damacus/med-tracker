use super::*;
use medtracker_web::household_i18n::Text;
use medtracker_web::people::{
    render_people, render_person_form, render_person_with_treatment_access, PersonDraft, PersonRow,
};

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/households/{slug}/people", get(index).post(create))
        .route("/households/{slug}/people/new", get(new))
        .route("/households/{slug}/people/{id}", get(show).post(update))
        .route("/households/{slug}/people/{id}/edit", get(edit))
}

fn person_row(value: &Value, manageable: &[i64]) -> Result<PersonRow, PageError> {
    let id = numeric(value, "id").ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
    Ok(PersonRow {
        id,
        name: field(value, "name").to_owned(),
        email: field(value, "email").to_owned(),
        date_of_birth: field(value, "date_of_birth").to_owned(),
        person_type: field(value, "person_type").to_owned(),
        has_capacity: value
            .get("has_capacity")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        can_edit: manageable.contains(&id),
    })
}

fn draft_from_record(value: &Value) -> PersonDraft {
    PersonDraft {
        name: field(value, "name").to_owned(),
        email: field(value, "email").to_owned(),
        date_of_birth: field(value, "date_of_birth").to_owned(),
        person_type: field(value, "person_type").to_owned(),
        has_capacity: value
            .get("has_capacity")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            .to_string(),
    }
}

fn draft_from_fields(fields: &HashMap<String, String>) -> PersonDraft {
    PersonDraft {
        name: fields.get("name").cloned().unwrap_or_default(),
        email: fields.get("email").cloned().unwrap_or_default(),
        date_of_birth: fields.get("date_of_birth").cloned().unwrap_or_default(),
        person_type: fields.get("person_type").cloned().unwrap_or_default(),
        has_capacity: fields
            .get("has_capacity")
            .cloned()
            .unwrap_or_else(|| "false".to_owned()),
    }
}

struct Context {
    household_id: i64,
    name: String,
    can_create: bool,
    manageable: Vec<i64>,
    notifications_visible: bool,
}

async fn context(api: &mut WebApi, slug: &str) -> Result<Context, PageError> {
    let (household_id, name) = api.household(slug).await?;
    let value = api.capabilities(household_id).await?;
    let can_create = value
        .pointer("/data/people/create")
        .and_then(Value::as_bool)
        .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
    let manageable = value
        .pointer("/data/people/manage_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?
        .iter()
        .map(|id| id.as_i64().ok_or_else(|| error(StatusCode::BAD_GATEWAY)))
        .collect::<Result<Vec<_>, _>>()?;
    let notifications_visible = api.notifications_visible(household_id).await;
    Ok(Context {
        household_id,
        name,
        can_create,
        manageable,
        notifications_visible,
    })
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
    let result = async {
        let context = context(&mut api, &slug).await?;
        let records = api
            .collection(&format!(
                "/api/v1/households/{}/people",
                context.household_id
            ))
            .await?;
        let rows = records
            .iter()
            .map(|value| person_row(value, &context.manageable))
            .collect::<Result<Vec<_>, _>>()?;
        render_people(
            &context.name,
            &slug,
            &api.csrf,
            api.locale,
            rows,
            context.can_create,
            context.notifications_visible,
        )
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))
    }
    .await;
    match result {
        Ok(body) => page(body, api.cookie),
        Err(error) => error.response(),
    }
}

async fn show(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, i64)>,
    headers: HeaderMap,
) -> Response {
    let mut api = match WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let result = async {
        let context = context(&mut api, &slug).await?;
        let household_id = context.household_id;
        let value = api
            .get(&format!("/api/v1/households/{household_id}/people/{id}"))
            .await?;
        let row = person_row(
            value
                .get("data")
                .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?,
            &context.manageable,
        )?;
        let treatments = super::treatments::rows(&mut api, household_id, id).await?;
        render_person_with_treatment_access(
            &context.name,
            &slug,
            &api.csrf,
            api.locale,
            row,
            treatments,
            context.notifications_visible,
        )
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))
    }
    .await;
    match result {
        Ok(body) => page(body, api.cookie),
        Err(error) => error.response(),
    }
}

async fn new(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    form_page(state, slug, None, headers).await
}

async fn edit(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, i64)>,
    headers: HeaderMap,
) -> Response {
    form_page(state, slug, Some(id), headers).await
}

async fn form_page(state: AppState, slug: String, id: Option<i64>, headers: HeaderMap) -> Response {
    let mut api = match WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    let result = async {
        let context = context(&mut api, &slug).await?;
        let household_id = context.household_id;
        let draft = if let Some(id) = id {
            let value = api
                .get(&format!("/api/v1/households/{household_id}/people/{id}"))
                .await?;
            if !context.manageable.contains(&id) {
                return Err(error(StatusCode::FORBIDDEN));
            }
            draft_from_record(
                value
                    .get("data")
                    .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?,
            )
        } else {
            if !context.can_create {
                return Err(error(StatusCode::FORBIDDEN));
            }
            PersonDraft {
                person_type: "adult".to_owned(),
                has_capacity: "true".to_owned(),
                ..Default::default()
            }
        };
        render_person_form(
            &context.name,
            &slug,
            &api.csrf,
            api.locale,
            id,
            draft,
            vec![],
            context.notifications_visible,
        )
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))
    }
    .await;
    match result {
        Ok(body) => page(body, api.cookie),
        Err(error) => error.response(),
    }
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
    Path((slug, id)): Path<(String, i64)>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    save(state, slug, Some(id), headers, fields).await
}

async fn save(
    state: AppState,
    slug: String,
    id: Option<i64>,
    headers: HeaderMap,
    fields: HashMap<String, String>,
) -> Response {
    if !oauth::trusted_cookie_origin(&state, &headers) {
        return failure(StatusCode::FORBIDDEN);
    }
    let mut api = match WebApi::authenticated(state, headers).await {
        Ok(api) => api,
        Err(error) => return error.response(),
    };
    if fields.get("authenticity_token") != Some(&api.csrf) {
        return failure(StatusCode::FORBIDDEN);
    }
    let draft = draft_from_fields(&fields);
    let result = async {
        let context = context(&mut api, &slug).await?;
        let base = format!("/api/v1/households/{}/people", context.household_id);
        let path = id.map_or_else(|| base.clone(), |id| format!("{base}/{id}"));
        let capacity = match draft.has_capacity.as_str() {
            "true" => Value::Bool(true), "false" => Value::Bool(false), value => Value::String(value.to_owned()),
        };
        let body = json!({"person": {"name": draft.name, "email": draft.email, "date_of_birth": draft.date_of_birth, "person_type": draft.person_type, "has_capacity": capacity}});
        let csrf = api.csrf.clone();
        let reply = api.call(if id.is_some() { Method::PATCH } else { Method::POST }, &path, Some(body), Some(&csrf)).await?;
        if reply.status.is_success() {
            let saved_id = reply.value.pointer("/data/id").and_then(Value::as_i64)
                .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
            api.get(&format!("{base}/{saved_id}")).await?;
            let location = format!("/households/{}/people/{saved_id}", medtracker_web::household::path_segment(&slug));
            return Ok(redirect(location, api.cookie.take()));
        }
        if reply.status != StatusCode::UNPROCESSABLE_ENTITY { return Err(error(reply.status)); }
        let text = Text::new(api.locale);
        let mut errors = Vec::new();
        if let Some(fields) = reply.value.pointer("/error/errors").and_then(Value::as_object) {
            for (field, messages) in fields {
                if let Some(messages) = messages.as_array() {
                    for message in messages.iter().filter_map(Value::as_str) {
                        errors.push((field.clone(), text.api_error(message).unwrap_or_else(|| message.to_owned())));
                    }
                }
            }
        }
        if errors.is_empty() { errors.push(("person".to_owned(), field(&reply.value["error"], "message").to_owned())); }
        let body = render_person_form(
            &context.name,
            &slug,
            &api.csrf,
            api.locale,
            id,
            draft,
            errors,
            context.notifications_visible,
        )
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR))?;
        Ok(page_status(body, api.cookie.take(), reply.status))
    }.await;
    match result {
        Ok(response) => response,
        Err(error) => error.response(),
    }
}
