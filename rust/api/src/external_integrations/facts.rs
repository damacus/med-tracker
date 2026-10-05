use super::{bounded_json, configured_url, http_client};
use regex::Regex;
use serde_json::{json, Value};
use url::Url;

const OPF_BASE: &str = "https://world.openproductsfacts.org/";
const OFF_BASE: &str = "https://world.openfoodfacts.org/";
const PRODUCT_FIELDS: &str =
    "product_name,generic_name,brands,quantity,categories_tags_en,image_url";

fn base_url(name: &str, default: &str) -> Option<Url> {
    let override_url = std::env::var(name).ok();
    let mut base = configured_url(
        override_url.as_deref().unwrap_or(default),
        override_url.is_some(),
    )?;
    if base.query().is_some() {
        return None;
    }
    if !base.path().ends_with('/') {
        let path = format!("{}/", base.path());
        base.set_path(&path);
    }
    Some(base)
}

async fn get_json(base: Url, path: &str, params: &[(&str, &str)]) -> Option<Value> {
    let mut url = base.join(path).ok()?;
    url.query_pairs_mut().extend_pairs(params.iter().copied());
    let client = http_client(5).ok()?;
    let response = client
        .get(url)
        .header("accept", "application/json")
        .header("user-agent", "MedTracker/1.0 (support@medtracker.app)")
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    bounded_json(response).await
}

async fn product(name: &str, default: &str, barcode: &str) -> Option<Value> {
    if !(8..=14).contains(&barcode.len()) || !barcode.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let base = base_url(name, default)?;
    let body = get_json(
        base,
        &format!("api/v2/product/{barcode}.json"),
        &[("fields", PRODUCT_FIELDS)],
    )
    .await?;
    if body.get("status").and_then(Value::as_u64) != Some(1) {
        return None;
    }
    Some(body)
}

fn product_data(body: &Value) -> &Value {
    body.get("product").unwrap_or(body)
}

fn value_text<'a>(body: &'a Value, key: &str) -> Option<&'a str> {
    body.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn display(body: &Value) -> Option<String> {
    let name = value_text(body, "product_name")?;
    let mut display = name.to_owned();
    if let Some(brand) = value_text(body, "brands") {
        display.push_str(&format!(" ({brand})"));
    }
    if let Some(quantity) = value_text(body, "quantity") {
        display.push_str(&format!(" {quantity}"));
    }
    Some(display)
}

fn categories(body: &Value) -> Vec<&str> {
    ["categories_tags_en", "categories_tags"]
        .iter()
        .flat_map(|key| {
            body.get(*key)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(Value::as_str)
        .collect()
}

pub(super) async fn opf_barcode(barcode: &str) -> Option<Value> {
    let body = product("MEDTRACKER_OPEN_PRODUCTS_FACTS_BASE_URL", OPF_BASE, barcode).await?;
    let body = product_data(&body);
    let display = display(body)?;
    let medicine = Regex::new(r"(?i)\b(medicines?|medications?|pharmaceuticals?|pharmacy|drugs?|analgesics?|painkillers?|paracetamol|acetaminophen|ibuprofen|aspirin)\b").ok()?;
    if !categories(body)
        .iter()
        .any(|category| medicine.is_match(category))
    {
        return None;
    }
    Some(
        json!({"gtin":barcode,"code":null,"display":display,"system":OPF_BASE.trim_end_matches('/'),"concept_class":"OTC Medicine","source":"open_products_facts"}),
    )
}

fn supplement(body: &Value) -> bool {
    const TERMS: &[&str] = &["supplement", "vitamin", "multivitamin"];
    body.get("categories_tags_en")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|category| {
            let category = category.to_ascii_lowercase();
            TERMS.iter().any(|term| category.contains(term))
        })
}

fn package(body: &Value) -> (Option<String>, Option<Value>, Option<String>) {
    let size = value_text(body, "quantity").map(str::to_owned);
    let Some(size_text) = size.as_deref() else {
        return (size, None, None);
    };
    let Ok(pattern) = Regex::new(r"(?i)^\s*(\d+(?:[.,]\d+)?)\s*([a-z]+)?") else {
        return (size, None, None);
    };
    let Some(captures) = pattern.captures(size_text) else {
        return (size, None, None);
    };
    let quantity = captures.get(1).map(|capture| {
        let value = capture.as_str().replace(',', ".");
        json!(if value.contains('.') {
            value
        } else {
            format!("{value}.0")
        })
    });
    let unit = captures
        .get(2)
        .and_then(
            |capture| match capture.as_str().to_ascii_lowercase().as_str() {
                "tablet" | "tablets" => Some("tablet"),
                "capsule" | "capsules" => Some("capsule"),
                "sachet" | "sachets" => Some("sachet"),
                "spray" | "sprays" => Some("spray"),
                "drop" | "drops" => Some("drop"),
                "pad" | "pads" => Some("pad"),
                "ml" | "millilitre" | "millilitres" | "milliliter" | "milliliters" => Some("ml"),
                "g" | "gram" | "grams" => Some("g"),
                _ => None,
            },
        )
        .map(str::to_owned);
    (size, quantity, unit)
}

fn off_result(product: &Value) -> Option<Value> {
    let body = product_data(product);
    if !supplement(body) {
        return None;
    }
    let name = value_text(body, "product_name")?;
    let display = display(body)?;
    let barcode = product
        .get("code")
        .or_else(|| body.get("code"))
        .and_then(Value::as_str)
        .map(|code| {
            code.chars()
                .filter(char::is_ascii_digit)
                .collect::<String>()
        })
        .filter(|code| !code.is_empty());
    let (package_size, package_quantity, package_unit) = package(body);
    Some(json!({
        "code":null,"barcode":barcode,"name":name,"description":value_text(body,"generic_name"),
        "display":display,"system":OFF_BASE.trim_end_matches('/'),"category":"Supplement",
        "package_size":package_size,"package_quantity":package_quantity,"package_unit":package_unit,
        "concept_class":"Supplement","source":"open_food_facts"
    }))
}

pub(super) async fn off_barcode(barcode: &str) -> Option<Value> {
    let body = product("MEDTRACKER_OPEN_FOOD_FACTS_BASE_URL", OFF_BASE, barcode).await?;
    off_result(&body)
}

pub(super) async fn off_search(query: &str) -> Vec<Value> {
    let Some(base) = base_url("MEDTRACKER_OPEN_FOOD_FACTS_BASE_URL", OFF_BASE) else {
        return vec![];
    };
    let Some(body) = get_json(
        base,
        "cgi/search.pl",
        &[
            ("search_terms", query),
            ("search_simple", "1"),
            ("action", "process"),
            ("json", "1"),
            ("page_size", "10"),
        ],
    )
    .await
    else {
        return vec![];
    };
    body.get("products")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(off_result)
        .collect()
}

pub(super) fn curated_barcode(barcode: &str) -> Option<Value> {
    let config: Value = serde_yaml::from_str(include_str!(
        "../../../../rails/config/nhs_dmd_curated_products.yml"
    ))
    .ok()?;
    let product = config
        .get("products")?
        .as_array()?
        .iter()
        .find(|product| product.get("gtin").and_then(Value::as_str) == Some(barcode))?;
    Some(json!({
        "gtin": barcode, "code":product.get("code"),"display":product.get("display"),
        "system":product.get("system"),"concept_class":product.get("concept_class"),"source":"curated"
    }))
}
