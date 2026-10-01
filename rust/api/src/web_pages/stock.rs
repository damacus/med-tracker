use super::*;
use medtracker_web::household::path_segment;
use medtracker_web::stock::{render_stock, OptionStock, StockDraft, StockPage};
use std::collections::BTreeMap;
use uuid::Uuid;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/households/{slug}/medications/{id}/stock", get(index))
        .route(
            "/households/{slug}/medications/{id}/stock/{action}",
            get(edit).post(save),
        )
}

struct Context {
    api: WebApi,
    household_id: i64,
    household_name: String,
    medication: Value,
    etag: String,
    options: Vec<Value>,
    can_manage: bool,
}

async fn context(
    state: AppState,
    headers: HeaderMap,
    slug: &str,
    id: &str,
) -> Result<Context, PageError> {
    let mut api = WebApi::authenticated(state, headers).await?;
    let (household_id, household_name) = api.household(slug).await?;
    let reply = api
        .get_reply(&format!(
            "/api/v1/households/{household_id}/medications/{}",
            path_segment(id)
        ))
        .await?;
    let etag = reply
        .etag
        .filter(|etag| !etag.is_empty())
        .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
    let medication = reply.value["data"].clone();
    let parent = numeric(&medication, "id").ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
    let options = api
        .collection(&format!("/api/v1/households/{household_id}/dosage_options"))
        .await?
        .into_iter()
        .filter(|row| numeric(row, "medication_id") == Some(parent))
        .collect();
    let capabilities = api.capabilities(household_id).await?;
    let can_manage = capabilities
        .pointer("/data/medications/update")
        .and_then(Value::as_bool)
        .ok_or_else(|| error(StatusCode::BAD_GATEWAY))?;
    Ok(Context {
        api,
        household_id,
        household_name,
        medication,
        etag,
        options,
        can_manage,
    })
}

fn permitted(context: &Context, action: &str) -> Result<(), PageError> {
    if !matches!(action, "adjust" | "order" | "remove" | "receive") {
        return Err(error(StatusCode::NOT_FOUND));
    }
    if matches!(action, "adjust" | "remove") && !context.can_manage {
        return Err(error(StatusCode::FORBIDDEN));
    }
    Ok(())
}

fn draft(fields: &HashMap<String, String>) -> StockDraft {
    let value = |name: &str| fields.get(name).cloned().unwrap_or_default();
    StockDraft {
        etag: value("etag"),
        new_quantity: value("new_quantity"),
        reason: value("reason"),
        supplier: value("supplier"),
        quantity: value("quantity"),
        expected_arrival_on: value("expected_arrival_on"),
        note: value("note"),
        dosage_id: value("dosage_id"),
        submission_id: value("submission_id"),
    }
}

fn render(
    context: Context,
    slug: String,
    action: Option<String>,
    draft: StockDraft,
    errors: BTreeMap<String, Vec<String>>,
    status: StatusCode,
) -> Response {
    let body = render_stock(StockPage {
        household_name: context.household_name,
        slug,
        medication_id: numeric(&context.medication, "id")
            .expect("validated parent")
            .to_string(),
        medication_name: field(&context.medication, "name").to_owned(),
        supply: context.medication["current_supply"]
            .as_str()
            .map(str::to_owned),
        unit: field(&context.medication, "dose_unit").to_owned(),
        status: field(&context.medication, "reorder_status").to_owned(),
        options: context
            .options
            .iter()
            .map(|row| OptionStock {
                id: numeric(row, "id").unwrap_or_default(),
                quantity: row["current_supply"].as_str().map(str::to_owned),
                unit: field(row, "unit").to_owned(),
            })
            .collect(),
        can_manage: context.can_manage,
        csrf: context.api.csrf,
        locale: context.api.locale,
        action,
        draft,
        errors,
    });
    match body {
        Ok(body) => page_status(body, context.api.cookie, status),
        Err(_) => failure(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn index(
    State(state): State<AppState>,
    Path((slug, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let context = match context(state, headers, &slug, &id).await {
        Ok(context) => context,
        Err(response) => return response.response(),
    };
    render(
        context,
        slug,
        None,
        StockDraft::default(),
        BTreeMap::new(),
        StatusCode::OK,
    )
}

async fn edit(
    State(state): State<AppState>,
    Path((slug, id, action)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Response {
    let context = match context(state, headers, &slug, &id).await {
        Ok(context) => context,
        Err(response) => return response.response(),
    };
    if action == "receive" {
        return failure(StatusCode::NOT_FOUND);
    }
    if let Err(response) = permitted(&context, &action) {
        return response.response();
    }
    if action == "adjust" && !context.options.is_empty() {
        return failure(StatusCode::CONFLICT);
    }
    let draft = StockDraft {
        etag: context.etag.clone(),
        new_quantity: field(&context.medication, "current_supply").to_owned(),
        submission_id: if action == "remove" {
            Uuid::new_v4().to_string()
        } else {
            String::new()
        },
        ..Default::default()
    };
    render(
        context,
        slug,
        Some(action),
        draft,
        BTreeMap::new(),
        StatusCode::OK,
    )
}

fn errors(value: &Value) -> BTreeMap<String, Vec<String>> {
    let mut errors: BTreeMap<String, Vec<String>> = value
        .pointer("/error/errors")
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .unwrap_or_default();
    if errors.is_empty() {
        errors.insert(
            "stock".into(),
            vec![value
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("is invalid")
                .to_owned()],
        );
    }
    errors
}

async fn save(
    State(state): State<AppState>,
    Path((slug, id, action)): Path<(String, String, String)>,
    headers: HeaderMap,
    Form(fields): Form<HashMap<String, String>>,
) -> Response {
    if !oauth::trusted_cookie_origin(&state, &headers) {
        return failure(StatusCode::FORBIDDEN);
    }
    let mut context = match context(state, headers, &slug, &id).await {
        Ok(context) => context,
        Err(response) => return response.response(),
    };
    if fields.get("authenticity_token") != Some(&context.api.csrf) {
        return failure(StatusCode::FORBIDDEN);
    }
    if let Err(response) = permitted(&context, &action) {
        return response.response();
    }
    let draft = draft(&fields);
    if action == "adjust" && draft.etag.trim().is_empty() {
        return render(
            context,
            slug,
            Some(action),
            draft,
            BTreeMap::from([("stock".into(), vec!["missing_browser_precondition".into()])]),
            StatusCode::PRECONDITION_REQUIRED,
        );
    }
    if action == "remove" {
        let invalid_source = if context.options.is_empty() {
            !draft.dosage_id.is_empty()
        } else {
            !context.options.iter().any(|row| {
                numeric(row, "id").map(|id| id.to_string()).as_deref()
                    == Some(draft.dosage_id.as_str())
                    && row["current_supply"].is_string()
            })
        };
        if invalid_source || draft.submission_id.trim().is_empty() {
            let (field, key, status) = if invalid_source {
                (
                    "dosage_id",
                    "stock_removals.errors.invalid_source",
                    StatusCode::UNPROCESSABLE_ENTITY,
                )
            } else {
                (
                    "stock",
                    "stock_removals.errors.invalid_submission",
                    StatusCode::PRECONDITION_REQUIRED,
                )
            };
            let message = medtracker_web::household_i18n::Text::new(context.api.locale)
                .get(key, &[])
                .expect("catalogue key");
            return render(
                context,
                slug,
                Some(action),
                draft,
                BTreeMap::from([(field.into(), vec![message])]),
                status,
            );
        }
    }
    let parent = numeric(&context.medication, "id").expect("validated parent");
    let base = format!(
        "/api/v1/households/{}/medications/{parent}",
        context.household_id
    );
    let (method, endpoint, body) = match action.as_str() {
        "adjust" => (
            Method::PATCH,
            "adjust_inventory",
            json!({"adjustment": {"new_quantity": draft.new_quantity, "reason": draft.reason}}),
        ),
        "order" => {
            let mut details = serde_json::Map::new();
            for (key, value) in [
                ("supplier", &draft.supplier),
                ("quantity", &draft.quantity),
                ("expected_arrival_on", &draft.expected_arrival_on),
            ] {
                if !value.is_empty() {
                    details.insert(key.into(), json!(value));
                }
            }
            (
                Method::PATCH,
                "mark_as_ordered",
                json!({"order_details": details}),
            )
        }
        "receive" => (Method::PATCH, "mark_as_received", json!({})),
        "remove" => {
            let mut body = json!({"stock_removal": {"quantity": draft.quantity, "reason": draft.reason, "note": draft.note, "submission_id": draft.submission_id}});
            if !draft.dosage_id.is_empty() {
                body["stock_removal"]["dosage_id"] = json!(draft.dosage_id);
            }
            (Method::POST, "stock_removals", body)
        }
        _ => return failure(StatusCode::NOT_FOUND),
    };
    let response = if action == "adjust" {
        context
            .api
            .adjust_scalar_stock(
                &format!("{base}/{endpoint}"),
                body,
                fields.get("authenticity_token").expect("validated CSRF"),
                draft.etag.clone(),
            )
            .await
    } else {
        context
            .api
            .call(
                method,
                &format!("{base}/{endpoint}"),
                Some(body),
                fields.get("authenticity_token").map(String::as_str),
            )
            .await
    };
    let reply = match response {
        Ok(reply) => reply,
        Err(response) => return response.response(),
    };
    if matches!(
        reply.status,
        StatusCode::CONFLICT | StatusCode::UNPROCESSABLE_ENTITY | StatusCode::BAD_REQUEST
    ) {
        return render(
            context,
            slug,
            Some(action),
            draft,
            errors(&reply.value),
            reply.status,
        );
    }
    if !reply.status.is_success() {
        return page_status(String::new(), context.api.cookie, reply.status);
    }
    if let Err(response) = context.api.get(&base).await {
        return response.response();
    }
    redirect(
        format!(
            "/households/{}/medications/{parent}/stock",
            path_segment(&slug)
        ),
        context.api.cookie,
    )
}
