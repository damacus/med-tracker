use loco_rs::{app::AppContext, environment::Environment};
use reqwest::{Client, StatusCode, Url, redirect::Policy};
use serde::Deserialize;
use std::time::Duration;

use super::{Product, barcode_candidates, product};
use crate::models::errors::OperationError;

pub(crate) const SOURCE: &str = "open_food_facts";
const BASE_URL: &str = "https://world.openfoodfacts.org";
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub(crate) struct ClientService {
    client: Client,
    base_url: Url,
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct Settings {
    enabled: Option<bool>,
    contact_email: Option<String>,
    base_url: Option<String>,
}

#[derive(Deserialize)]
struct ProviderResponse {
    status: u8,
    code: Option<String>,
    product: Option<ProviderProduct>,
}

#[derive(Deserialize)]
struct ProviderProduct {
    product_name: Option<String>,
    brands: Option<String>,
    categories_tags: Option<Vec<String>>,
}

impl ClientService {
    pub(crate) fn from_context(ctx: &AppContext) -> Result<Option<Self>, &'static str> {
        let settings: Settings = ctx
            .config
            .settings
            .as_ref()
            .and_then(|settings| settings.get(SOURCE))
            .filter(|value| !value.is_null())
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()
            .map_err(|_| "Invalid Open Food Facts settings")?
            .unwrap_or_default();
        if !settings
            .enabled
            .unwrap_or(ctx.environment != Environment::Test)
        {
            return Ok(None);
        }
        let mut base_url = Url::parse(BASE_URL).map_err(|_| "Invalid Open Food Facts URL")?;
        if ctx.environment == Environment::Test
            && let Some(test_url) = settings.base_url
        {
            base_url = Url::parse(&test_url).map_err(|_| "Invalid Open Food Facts test URL")?;
            if base_url.scheme() != "http" || base_url.host_str() != Some("127.0.0.1") {
                return Err("Open Food Facts test URL must use HTTP loopback");
            }
        }
        let contact = settings
            .contact_email
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("support@medtracker.app");
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .redirect(Policy::none())
            .user_agent(format!(
                "MedTracker/{} ({contact})",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|_| "Could not configure the Open Food Facts HTTP client")?;
        Ok(Some(Self { client, base_url }))
    }

    pub(super) async fn lookup(&self, barcode: &str) -> Result<Option<Product>, OperationError> {
        let code = if barcode.len() == 14 {
            barcode.strip_prefix('0').unwrap_or(barcode)
        } else {
            barcode
        };
        let mut url = self.base_url.clone();
        url.set_path(&format!("/api/v2/product/{code}.json"));
        url.set_query(None);
        url.query_pairs_mut()
            .append_pair("fields", "product_name,brands,categories_tags");
        let mut response = crate::telemetry::units::send(
            self.client.get(url),
            self.base_url.host_str().unwrap_or_default(),
            "GET",
        )
        .await
        .map_err(|error| {
            unavailable(if error.is_timeout() {
                "timeout"
            } else {
                "transport"
            })
        })?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(unavailable("http_status"));
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| unavailable("body"))? {
            if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(unavailable("response_size"));
            }
            body.extend_from_slice(&chunk);
        }
        let payload: ProviderResponse =
            serde_json::from_slice(&body).map_err(|_| unavailable("invalid_response"))?;
        if payload.status == 0 {
            return Ok(None);
        }
        let barcodes = barcode_candidates(barcode);
        if payload.status != 1
            || !payload.code.is_some_and(|code| {
                code.bytes().all(|byte| byte.is_ascii_digit())
                    && barcode_candidates(&code)
                        .iter()
                        .any(|value| barcodes.contains(value))
            })
        {
            return Err(unavailable("unexpected_product"));
        }
        Ok(payload.product.and_then(|data| supplement(data, barcode)))
    }
}

fn supplement(data: ProviderProduct, barcode: &str) -> Option<Product> {
    let name = data.product_name.as_deref()?.trim();
    if name.is_empty() {
        return None;
    }
    let categories = data.categories_tags?;
    let has = |tag| categories.iter().any(|category| category == tag);
    let category = if has("en:vitamin-supplements") {
        "Vitamin"
    } else if has("en:mineral-supplements") {
        "Mineral"
    } else if has("en:dietary-supplements") || has("en:vitamin-and-mineral-supplements") {
        "Supplement"
    } else {
        return None;
    };
    let display = data
        .brands
        .as_deref()
        .map(str::trim)
        .filter(|brand| !brand.is_empty() && !name.to_lowercase().contains(&brand.to_lowercase()))
        .map_or_else(|| name.to_owned(), |brand| format!("{brand} {name}"));
    let mut found = product(&display, Some(barcode), None, None, None, "Open Food Facts");
    found.name = Some(name.to_owned());
    found.category = Some(category.to_owned());
    Some(found)
}

fn unavailable(kind: &str) -> OperationError {
    tracing::warn!(
        event.name = "medicine_lookup.open_food_facts_unavailable",
        error.type = kind,
        "Open Food Facts barcode lookup unavailable"
    );
    OperationError::Unavailable
}
