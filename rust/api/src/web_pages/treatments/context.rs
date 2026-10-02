use super::*;
use crate::web_pages::api_client::ApiReply;

pub(super) struct Context {
    pub(super) api: WebApi,
    pub(super) household_id: i64,
    pub(super) household_name: String,
    pub(super) person: Value,
    pub(super) person_id: i64,
    pub(super) can_manage: bool,
    pub(super) medications: Vec<Value>,
    pub(super) options: Vec<Value>,
}

pub(super) async fn load(
    state: AppState,
    headers: HeaderMap,
    slug: &str,
    person: &str,
) -> Result<Context, PageError> {
    let context = load_read(state, headers, slug, person).await?;
    if !context.can_manage {
        return Err(error(StatusCode::FORBIDDEN));
    }
    Ok(context)
}

pub(super) async fn load_read(
    state: AppState,
    headers: HeaderMap,
    slug: &str,
    person: &str,
) -> Result<Context, PageError> {
    let mut api = WebApi::authenticated(state, headers).await?;
    let (household_id, household_name) = api.household(slug).await?;
    let person = api
        .get(&format!(
            "/api/v1/households/{household_id}/people/{}",
            path_segment(person)
        ))
        .await?["data"]
        .clone();
    let person_id = numeric(&person, "id").ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
    let capabilities = api.capabilities(household_id).await?;
    let manageable = capabilities
        .pointer("/data/people/manage_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
    let can_manage = manageable.iter().any(|id| id.as_i64() == Some(person_id));
    let medications = api
        .collection(&format!("/api/v1/households/{household_id}/medications"))
        .await?;
    let options = api
        .collection(&format!("/api/v1/households/{household_id}/dosage_options"))
        .await?;
    Ok(Context {
        api,
        household_id,
        household_name,
        person,
        person_id,
        can_manage,
        medications,
        options,
    })
}

pub(super) async fn source(
    context: &mut Context,
    kind: Kind,
    id: &str,
) -> Result<ApiReply, PageError> {
    let reply = context
        .api
        .get_reply(&format!(
            "/api/v1/households/{}/{}/{}",
            context.household_id,
            kind.resource(),
            path_segment(id)
        ))
        .await?;
    if numeric(&reply.value["data"], "person_id") != Some(context.person_id) {
        return Err(error(StatusCode::NOT_FOUND));
    }
    if reply.etag.as_deref().is_none_or(str::is_empty) {
        return Err(error(StatusCode::BAD_GATEWAY));
    }
    Ok(reply)
}

pub(super) struct FormChoices {
    pub(super) medications: Vec<(String, String)>,
    pub(super) dosages: Vec<(String, String)>,
    pub(super) units: Vec<(String, String)>,
}

pub(super) fn choices(context: &Context) -> Result<FormChoices, PageError> {
    let medications = context
        .medications
        .iter()
        .map(|row| {
            let id = numeric(row, "id").ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
            Ok((id.to_string(), field(row, "name").to_owned()))
        })
        .collect::<Result<Vec<_>, PageError>>()?;
    let dosages = context
        .options
        .iter()
        .map(|row| {
            let id = numeric(row, "id").ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
            let parent =
                numeric(row, "medication_id").ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
            let medication = context
                .medications
                .iter()
                .find(|medication| numeric(medication, "id") == Some(parent))
                .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
            Ok((
                id.to_string(),
                format!(
                    "{} — {} {}",
                    field(medication, "name"),
                    field(row, "amount"),
                    field(row, "unit")
                ),
            ))
        })
        .collect::<Result<Vec<_>, PageError>>()?;
    let mut units = [
        "tablet", "capsule", "gummy", "mg", "ml", "g", "mcg", "IU", "spray", "drop", "sachet",
        "pad",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    for row in &context.options {
        let unit = field(row, "unit");
        if !unit.is_empty() && !units.iter().any(|known| known == unit) {
            units.push(unit.to_owned());
        }
    }
    Ok(FormChoices {
        medications,
        dosages,
        units: units.into_iter().map(|unit| (unit.clone(), unit)).collect(),
    })
}
