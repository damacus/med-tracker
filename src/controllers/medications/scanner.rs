use super::*;
use crate::models::care::medication_lookup;
use serde_json::json;

pub(super) async fn finder(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    token: CsrfToken,
    ViewEngine(view): ViewEngine<TeraView>,
) -> Response {
    let (_, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let found =
        match medication_lookup::search(&tenant, &medication_lookup::Search::default()).await {
            Ok(value) => value,
            Err(error) => return operation_error(error),
        };
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    let mut data = rendering::appearance_context();
    data["title"] = json!("Medication Finder");
    data["slug"] = json!(slug);
    data["can_create"] = found.body["permissions"]["can_create"].clone();
    match format::render().view(&view, "medications/finder.html", data) {
        Ok(response) => (token, [(header::CACHE_CONTROL, "no-store")], response).into_response(),
        Err(_) => unavailable(),
    }
}

pub(super) async fn lookup(
    State(ctx): State<AppContext>,
    Path(slug): Path<String>,
    session: Session<SessionPgPool>,
    request: Option<Extension<LocoRequestId>>,
    Query(input): Query<medication_lookup::Search>,
) -> Response {
    let (_, tenant) = match begin(&ctx, &session, &slug, &request_id(request)).await {
        Ok(value) => value,
        Err(error) => return authentication_error(error),
    };
    let mut found = match medication_lookup::search(&tenant, &input).await {
        Ok(value) => value,
        Err(error) => return operation_error(error),
    };
    let curated = match medication_lookup::curated_products() {
        Ok(products) => products,
        Err(error) => return operation_error(error),
    };
    if let Some(results) = found.body["results"].as_array_mut() {
        for product in results {
            if let Some(entry) = curated.iter().find(|entry| {
                entry["gtin"] == product["barcode"] && !entry["gtin"].is_null()
                    || entry["code"] == product["code"] && !entry["code"].is_null()
            }) {
                product["suggested_doses"] = entry["suggested_doses"].clone();
            }
        }
    }
    found.body["matches"] = json!(found.matches);
    if tenant.commit().await.is_err() {
        return unavailable();
    }
    (
        [(header::CACHE_CONTROL, "no-store")],
        axum::Json(found.body),
    )
        .into_response()
}
