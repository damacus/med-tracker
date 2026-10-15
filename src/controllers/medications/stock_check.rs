use super::*;
use crate::models::care::{report_pdf, stock_check};
use serde::Deserialize;
use serde_json::json;

#[derive(Default, Deserialize)]
pub(super) struct StockCheckQuery {
    #[serde(default, deserialize_with = "empty_location")]
    location_id: Option<i64>,
}

pub(super) fn empty_location<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if value.is_empty() {
        Ok(None)
    } else {
        value.parse().map(Some).map_err(serde::de::Error::custom)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn show(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    headers: HeaderMap,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(query): Query<StockCheckQuery>,
) -> Response {
    let (_, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    render_page(
        tenant,
        RenderPage {
            slug: &slug,
            headers: &headers,
            token,
            view: &view,
        },
        query.location_id,
        &HashMap::new(),
        None,
    )
    .await
}

struct RenderPage<'a> {
    slug: &'a str,
    headers: &'a HeaderMap,
    token: CsrfToken,
    view: &'a TeraView,
}

async fn render_page(
    tenant: TenantTransaction,
    page: RenderPage<'_>,
    location_id: Option<i64>,
    draft: &HashMap<String, String>,
    error: Option<&OperationError>,
) -> Response {
    let RenderPage {
        slug,
        headers,
        token,
        view,
    } = page;
    let snapshot = match stock_check::read(&tenant, location_id).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let locale = report_pdf::locale(
        headers
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("en"),
    );
    let labels = match report_pdf::translations(locale) {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    let Some(authenticity_token) = token.authenticity_token().ok() else {
        return unavailable();
    };
    let mut rows = match serde_json::to_value(snapshot.rows) {
        Ok(value) => value,
        Err(_) => return unavailable(),
    };
    if let Some(rows) = rows.as_array_mut() {
        for row in rows {
            let Some(key) = row["key"].as_str().map(str::to_owned) else {
                continue;
            };
            let unit = row["unit"].as_str().unwrap_or_default().to_owned();
            let names = &labels["medications"]["stock_check"]["unit_names"][&unit];
            let one = names["one"].as_str().unwrap_or(&unit).to_owned();
            let other = names["other"].as_str().unwrap_or(&unit).to_owned();
            row["unit_one"] = json!(one);
            row["unit_other"] = json!(other);
            row["display_unit"] = if row["quantity"].as_str() == Some("1") {
                row["unit_one"].clone()
            } else {
                row["unit_other"].clone()
            };
            row["selected"] = json!(false);
            if let Some(quantity) = draft.get(&format!("quantity_{key}")) {
                row["selected"] = json!(true);
                row["new_quantity"] = json!(quantity);
            }
        }
    }
    let selected_count = rows.as_array().map_or(0, |rows| {
        rows.iter().filter(|row| row["selected"] == true).count()
    });
    let mut data = rendering::appearance_context();
    data["title"] = labels["medications"]["stock_check"]["title"].clone();
    data["lang"] = json!(locale);
    data["labels"] = labels;
    data["slug"] = json!(slug);
    data["rows"] = rows;
    data["selected_count"] = json!(selected_count);
    data["selected_count_label"] = json!(selected_count.to_string());
    data["locations"] = json!(snapshot.locations);
    data["current_location_id"] = json!(location_id);
    data["authenticity_token"] = json!(authenticity_token);
    data["reason"] = json!(
        draft
            .get("reason")
            .map(String::as_str)
            .unwrap_or("house_stock_check")
    );
    data["error"] = match error {
        Some(OperationError::Conflict { .. }) => {
            data["labels"]["medications"]["stock_check"]["errors"]["stale"].clone()
        }
        Some(OperationError::Validation { .. }) => {
            data["labels"]["medications"]["stock_check"]["errors"]["invalid"].clone()
        }
        Some(_) => data["labels"]["medications"]["stock_check"]["errors"]["unavailable"].clone(),
        None => serde_json::Value::Null,
    };
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    match format::render().view(
        view,
        "medications/stock_check.html",
        crate::views::template_data(&data),
    ) {
        Ok(response) => (
            error.map_or(StatusCode::OK, forms::status),
            token,
            [(header::CACHE_CONTROL, "no-store")],
            response,
        )
            .into_response(),
        Err(_) => unavailable(),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn apply(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    headers: HeaderMap,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
    Query(query): Query<StockCheckQuery>,
    AxumForm(draft): AxumForm<HashMap<String, String>>,
) -> Response {
    if token
        .verify(forms::field(&draft, "authenticity_token"))
        .is_err()
    {
        return operation_error(OperationError::Forbidden);
    }
    let request_id = request_id(request);
    let (principal, tenant) = match begin(&ctx, &session, &slug, &request_id).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let rows = draft
        .iter()
        .filter_map(|(key, quantity)| {
            let row_key = key.strip_prefix("quantity_")?;
            Some(stock_check::SubmittedRow {
                key: row_key.to_owned(),
                snapshot: forms::field(&draft, &format!("snapshot_{row_key}")).to_owned(),
                quantity: quantity.to_owned(),
            })
        })
        .collect::<Vec<_>>();
    match stock_check::apply(
        &tenant,
        rows,
        forms::field(&draft, "reason"),
        Some(principal.provenance()),
    )
    .await
    {
        Ok(()) => {
            if tenant.commit().await.is_err() {
                return unavailable();
            }
            (
                StatusCode::SEE_OTHER,
                [
                    (header::LOCATION, format!("/households/{slug}/medications")),
                    (header::CACHE_CONTROL, "no-store".to_owned()),
                ],
            )
                .into_response()
        }
        Err(error) => {
            if tenant.rollback().await.is_err() {
                return unavailable();
            }
            if matches!(
                error,
                OperationError::Forbidden
                    | OperationError::NotFound
                    | OperationError::Unauthenticated
            ) {
                return operation_error(error);
            }
            let (_, tenant) = match begin(&ctx, &session, &slug, &request_id).await {
                Ok(value) => value,
                Err(error) => return authentication_error(error),
            };
            render_page(
                tenant,
                RenderPage {
                    slug: &slug,
                    headers: &headers,
                    token,
                    view: &view,
                },
                query.location_id,
                &draft,
                Some(&error),
            )
            .await
        }
    }
}
