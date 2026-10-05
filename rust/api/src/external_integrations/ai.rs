use super::{bounded_json, configured_url, http_client};
use crate::entities::{household, version};
use crate::external_integrations::lookup::finder_allowed;
use crate::medication_management::{error_response, finish_with_request_id, request_context};
use crate::{database_error, ApiError, AppState};
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use chrono::Utc;
use regex::Regex;
use scraper::{Html, Selector};
use sea_orm::{prelude::Decimal, ActiveModelTrait, DatabaseTransaction, EntityTrait, Set};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use url::Url;
use uuid::Uuid;

const CONTROLLER: &str = "api/v1/ai_medication_suggestions";
const POLICY: &str = "MedicationPolicy";
const INSTRUCTIONS: &str = "Draft medication onboarding fields only from the fetched source text provided. Return JSON with medication, doses, sources, and errors. Do not guess dose guidance or cite unfetched URLs.";
const IDENTITY_FIELDS: &[&str] = &[
    "name",
    "barcode",
    "dmd_code",
    "dmd_system",
    "dmd_concept_class",
    "category",
    "description",
];

#[derive(Deserialize)]
struct SourceConfig {
    sources: Vec<TrustedSource>,
}

#[derive(Deserialize)]
struct TrustedSource {
    domains: Vec<String>,
    path_patterns: Vec<String>,
    seed_urls: Vec<SeedUrl>,
}

#[derive(Deserialize)]
struct SeedUrl {
    url: String,
    keywords: Vec<String>,
}

#[derive(Clone)]
struct FetchedSource {
    url: String,
    title: String,
    text: String,
}

#[derive(Clone, Copy)]
enum Provider {
    OpenAi,
    Anthropic,
    Gemini,
    OpenRouter,
}

impl Provider {
    fn name(&self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::OpenRouter => "openrouter",
        }
    }

    fn key_name(&self) -> &'static str {
        match self {
            Self::OpenAi => "OPENAI_API_KEY",
            Self::Anthropic => "ANTHROPIC_API_KEY",
            Self::Gemini => "GEMINI_API_KEY",
            Self::OpenRouter => "OPENROUTER_API_KEY",
        }
    }

    fn default_base(&self) -> &'static str {
        match self {
            Self::OpenAi => "https://api.openai.com/v1/",
            Self::Anthropic => "https://api.anthropic.com/v1/",
            Self::Gemini => "https://generativelanguage.googleapis.com/v1beta/",
            Self::OpenRouter => "https://openrouter.ai/api/v1/",
        }
    }

    fn model_matches(&self, model: &str) -> bool {
        match self {
            Self::OpenAi => {
                model.starts_with("gpt-")
                    || model.starts_with("o1")
                    || model.starts_with("o3")
                    || model.starts_with("o4")
            }
            Self::Anthropic => model.starts_with("claude-"),
            Self::Gemini => model.starts_with("gemini-"),
            Self::OpenRouter => model.contains('/') && !model.starts_with('/'),
        }
    }
}

struct ProviderConfig {
    provider: Provider,
    model: String,
    key: String,
    base: Url,
}

fn configured_provider() -> Result<ProviderConfig, &'static str> {
    let provided: Vec<Provider> = [
        Provider::OpenAi,
        Provider::Anthropic,
        Provider::Gemini,
        Provider::OpenRouter,
    ]
    .into_iter()
    .filter(|provider| {
        std::env::var(provider.key_name()).is_ok_and(|value| !value.trim().is_empty())
    })
    .collect();
    if provided.is_empty() {
        return Err("ruby_llm_unconfigured");
    }
    let model = std::env::var("MEDTRACKER_AI_MEDICATION_HELP_MODEL")
        .map_err(|_| "ruby_llm_unconfigured")?;
    if model.trim().is_empty() {
        return Err("ruby_llm_unconfigured");
    }
    let provider = match std::env::var("MEDTRACKER_AI_MEDICATION_HELP_PROVIDER") {
        Ok(selector) => provided
            .into_iter()
            .find(|provider| provider.name() == selector)
            .ok_or("provider_configuration_error")?,
        Err(_) => {
            if provided.len() != 1 {
                return Err("provider_configuration_error");
            }
            provided
                .into_iter()
                .next()
                .ok_or("provider_configuration_error")?
        }
    };
    if !provider.model_matches(&model) {
        return Err("provider_configuration_error");
    }
    let key = std::env::var(provider.key_name()).map_err(|_| "provider_configuration_error")?;
    let base = std::env::var("MEDTRACKER_AI_MEDICATION_HELP_API_BASE_URL")
        .unwrap_or_else(|_| provider.default_base().to_owned());
    let mut base = configured_url(&base, true).ok_or("provider_configuration_error")?;
    if base.scheme() == "http" && base.host_str() != Some("127.0.0.1") {
        return Err("provider_configuration_error");
    }
    if !base.path().ends_with('/') {
        let path = format!("{}/", base.path());
        base.set_path(&path);
    }
    Ok(ProviderConfig {
        provider,
        model,
        key,
        base,
    })
}

fn empty_draft(error: &str) -> Value {
    json!({"medication": {}, "doses": [], "sources": [], "errors": [error]})
}

fn parsed_identity(body: &Bytes) -> Option<Value> {
    let value: Value = if body.is_empty() {
        json!({})
    } else {
        serde_json::from_slice(body).ok()?
    };
    let object = value.as_object()?;
    if object.keys().any(|key| key != "medication") {
        return None;
    }
    let Some(medication) = object.get("medication") else {
        return Some(json!({}));
    };
    let medication = medication.as_object()?;
    if medication
        .keys()
        .any(|key| !IDENTITY_FIELDS.contains(&key.as_str()))
        || medication.values().any(|value| !value.is_string())
    {
        return None;
    }
    Some(Value::Object(medication.clone()))
}

fn source_config() -> Option<SourceConfig> {
    serde_yaml::from_str(include_str!(
        "../../../../rails/config/ai_medication_sources.yml"
    ))
    .ok()
}

fn allowed_source(url: &str, config: &SourceConfig) -> bool {
    let Ok(url) = Url::parse(url) else {
        return false;
    };
    if url.scheme() != "https"
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    config.sources.iter().any(|source| {
        source.domains.iter().any(|domain| {
            host.eq_ignore_ascii_case(domain)
                || host
                    .to_ascii_lowercase()
                    .ends_with(&format!(".{}", domain.to_ascii_lowercase()))
        }) && source
            .path_patterns
            .iter()
            .any(|pattern| Regex::new(pattern).is_ok_and(|pattern| pattern.is_match(url.path())))
    })
}

fn query_tokens(value: &str) -> HashSet<String> {
    value
        .to_ascii_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| token.len() >= 3)
        .map(str::to_owned)
        .collect()
}

fn matching_seeds(identity: &Value, config: &SourceConfig) -> Vec<String> {
    let query = identity
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let tokens = query_tokens(query);
    config
        .sources
        .iter()
        .flat_map(|source| &source.seed_urls)
        .filter(|seed| {
            seed.keywords
                .iter()
                .any(|keyword| !query_tokens(keyword).is_disjoint(&tokens))
        })
        .filter(|seed| allowed_source(&seed.url, config))
        .map(|seed| seed.url.clone())
        .take(3)
        .collect()
}

fn source_proxy_url(original: &str) -> Result<Url, ()> {
    let proxy = std::env::var("MEDTRACKER_AI_MEDICATION_SOURCE_PROXY_ORIGIN").map_err(|_| ())?;
    let mut proxy = configured_url(&proxy, true).ok_or(())?;
    if proxy.host_str() != Some("127.0.0.1") || proxy.path() != "/" || proxy.query().is_some() {
        return Err(());
    }
    proxy.set_path("/fetch");
    proxy.query_pairs_mut().append_pair("url", original);
    Ok(proxy)
}

async fn fetch_source(client: &reqwest::Client, source_url: &str) -> Option<FetchedSource> {
    let request_url = if std::env::var("MEDTRACKER_AI_MEDICATION_SOURCE_PROXY_ORIGIN").is_ok() {
        source_proxy_url(source_url).ok()?
    } else {
        Url::parse(source_url).ok()?
    };
    let mut response = client.get(request_url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        if bytes.len() + chunk.len() > 100_000 {
            return None;
        }
        bytes.extend_from_slice(&chunk);
    }
    let html = String::from_utf8(bytes).ok()?;
    let parsed = Html::parse_document(&html);
    let title = Selector::parse("title")
        .ok()
        .and_then(|selector| parsed.select(&selector).next())
        .map(|element| element.text().collect::<Vec<_>>().join(" "))
        .unwrap_or_default();
    let text = Selector::parse("body")
        .ok()
        .and_then(|selector| parsed.select(&selector).next())
        .map(|element| element.text().collect::<Vec<_>>().join(" "))
        .unwrap_or_default();
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let text = text.chars().take(30_000).collect::<String>();
    if title.is_empty() || text.is_empty() {
        return None;
    }
    Some(FetchedSource {
        url: source_url.to_owned(),
        title,
        text,
    })
}

async fn call_provider(config: &ProviderConfig, prompt: &str) -> Result<String, ()> {
    let client = http_client(10).map_err(|_| ())?;
    let (path, request) = match config.provider {
        Provider::OpenAi | Provider::OpenRouter => {
            let path = "chat/completions".to_owned();
            let body = json!({"model": config.model, "messages": [{"role":"system","content": INSTRUCTIONS},{"role":"user","content":prompt}]});
            (path, body)
        }
        Provider::Anthropic => {
            let path = "messages".to_owned();
            let body = json!({"model": config.model, "max_tokens": 1024, "system": INSTRUCTIONS, "messages":[{"role":"user","content":prompt}]});
            (path, body)
        }
        Provider::Gemini => {
            let path = format!("models/{}:generateContent", config.model);
            let body = json!({"systemInstruction":{"parts":[{"text":INSTRUCTIONS}]},"contents":[{"parts":[{"text":prompt}]}],"generationConfig":{"responseMimeType":"application/json"}});
            (path, body)
        }
    };
    let url = config.base.join(&path).map_err(|_| ())?;
    let mut request = client.post(url).json(&request);
    request = match config.provider {
        Provider::OpenAi | Provider::OpenRouter => request.bearer_auth(&config.key),
        Provider::Anthropic => request
            .header("x-api-key", &config.key)
            .header("anthropic-version", "2023-06-01"),
        Provider::Gemini => request.header("x-goog-api-key", &config.key),
    };
    let response = request.send().await.map_err(|_| ())?;
    if !response.status().is_success() {
        return Err(());
    }
    let body = bounded_json(response).await.ok_or(())?;
    let content = match config.provider {
        Provider::OpenAi | Provider::OpenRouter => body
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str),
        Provider::Anthropic => body.pointer("/content/0/text").and_then(Value::as_str),
        Provider::Gemini => body
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(Value::as_str),
    };
    content.map(str::to_owned).ok_or(())
}

fn candidate_urls(content: &str, config: &SourceConfig) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<Value>(content) else {
        return vec![];
    };
    value
        .get("candidate_urls")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|url| allowed_source(url, config))
        .map(str::to_owned)
        .take(3)
        .collect()
}

fn decimal_string(value: &Value, positive: bool) -> Option<String> {
    let raw = value
        .as_str()
        .map(str::trim)
        .map(str::to_owned)
        .or_else(|| value.as_number().map(ToString::to_string))?;
    let decimal = raw.parse::<Decimal>().ok()?;
    if positive && decimal <= Decimal::ZERO || !positive && decimal < Decimal::ZERO {
        return None;
    }
    Some(decimal.normalize().to_string())
}

fn positive_integer(value: &Value) -> Option<u64> {
    let raw = value
        .as_str()
        .map(str::to_owned)
        .or_else(|| value.as_number().map(ToString::to_string))?;
    if !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    raw.parse::<u64>().ok().filter(|value| *value > 0)
}

fn validated_draft(content: &str, fetched: &[FetchedSource]) -> Value {
    let Ok(raw) = serde_json::from_str::<Value>(content) else {
        return empty_draft("invalid_model_response");
    };
    let Some(raw) = raw.as_object() else {
        return empty_draft("invalid_model_response");
    };
    let fetched_by_url = fetched
        .iter()
        .map(|source| (source.url.as_str(), source))
        .collect::<std::collections::HashMap<_, _>>();
    let sources = raw
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|source| {
            let url = source.get("url")?.as_str()?;
            let fetched = fetched_by_url.get(url)?;
            Some(json!({"url": url, "title": fetched.title}))
        })
        .collect::<Vec<_>>();
    if sources.is_empty() {
        return empty_draft("trusted_source_unavailable");
    }
    let medication = raw
        .get("medication")
        .and_then(Value::as_object)
        .map(|medication| {
            medication
                .iter()
                .filter(|(key, value)| {
                    let Some(value) = value.as_str() else {
                        return false;
                    };
                    if key.as_str() == "name" {
                        return true;
                    }
                    matches!(key.as_str(), "category" | "description" | "warnings")
                        && fetched.iter().any(|source| {
                            source
                                .text
                                .to_ascii_lowercase()
                                .contains(&value.to_ascii_lowercase())
                        })
                })
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect::<Map<String, Value>>()
        })
        .unwrap_or_default();
    let doses = raw
        .get("doses")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|dose| {
            let dose = dose.as_object()?;
            let amount = decimal_string(dose.get("amount")?, true)?;
            let unit = dose.get("unit")?.as_str()?;
            if ![
                "tablet", "capsule", "gummy", "mg", "ml", "g", "mcg", "IU", "spray", "drop",
                "sachet", "pad",
            ]
            .contains(&unit)
            {
                return None;
            }
            let max_doses = positive_integer(dose.get("default_max_daily_doses")?)?;
            let min_hours = decimal_string(dose.get("default_min_hours_between_doses")?, false)?;
            let cycle = dose.get("default_dose_cycle")?.as_str()?;
            if !["daily", "weekly", "monthly"].contains(&cycle) {
                return None;
            }
            let evidence = dose.get("evidence")?.as_object()?;
            let url = evidence.get("url")?.as_str()?;
            let source = fetched_by_url.get(url)?;
            let text = evidence.get("text")?.as_str()?.trim();
            if text.is_empty()
                || !source
                    .text
                    .to_ascii_lowercase()
                    .contains(&text.to_ascii_lowercase())
            {
                return None;
            }
            let amount_unit = Regex::new(&format!(
                r"(?i)\b{}\s*{}\b",
                regex::escape(&amount),
                regex::escape(unit)
            ))
            .ok()?;
            let maximum = Regex::new(&format!(r"(?i)\b{}\s+doses?\b", max_doses)).ok()?;
            let spacing = Regex::new(&format!(
                r"(?i)\b{}\s*(?:hours?|hrs?)\b",
                regex::escape(&min_hours)
            ))
            .ok()?;
            let period = match cycle {
                "daily" => Regex::new(r"(?i)\b(daily|days?|24\s*hours?)\b").ok()?,
                "weekly" => Regex::new(r"(?i)\b(weekly|weeks?|7\s*days?)\b").ok()?,
                "monthly" => Regex::new(r"(?i)\b(monthly|months?|30\s*days?)\b").ok()?,
                _ => return None,
            };
            if !amount_unit.is_match(text)
                || !maximum.is_match(text)
                || !spacing.is_match(text)
                || !period.is_match(text)
            {
                return None;
            }
            let mut result = json!({
                "amount": amount, "unit": unit, "default_max_daily_doses": max_doses,
                "default_min_hours_between_doses": min_hours, "default_dose_cycle": cycle,
                "evidence": {"url": url, "title": source.title, "text": text}
            });
            if let Some(description) = dose.get("description").and_then(Value::as_str) {
                result["description"] = json!(description);
            }
            Some(result)
        })
        .collect::<Vec<_>>();
    let errors = raw
        .get("errors")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|error| !error.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    json!({"medication": medication, "doses": doses, "sources": sources, "errors": errors})
}

async fn generate(identity: &Value) -> Value {
    let config = match configured_provider() {
        Ok(config) => config,
        Err(error) => return empty_draft(error),
    };
    let Some(source_config) = source_config() else {
        return empty_draft("trusted_source_unavailable");
    };
    let mut urls = matching_seeds(identity, &source_config);
    if urls.is_empty() {
        let prompt = json!({"task":"Suggest up to three likely public medication guidance URLs from the configured trusted domains. Return JSON with candidate_urls only. Do not provide clinical guidance.","medication_identity":identity,"allowed_domains":source_config.sources.iter().flat_map(|source| &source.domains).collect::<Vec<_>>()}).to_string();
        let candidates = match call_provider(&config, &prompt).await {
            Ok(content) => content,
            Err(()) => return empty_draft("suggestion_unavailable"),
        };
        urls = candidate_urls(&candidates, &source_config);
    }
    let client = match http_client(5) {
        Ok(client) => client,
        Err(_) => return empty_draft("trusted_source_unavailable"),
    };
    let mut fetched = Vec::new();
    for url in urls {
        if !allowed_source(&url, &source_config) {
            continue;
        }
        if let Some(source) = fetch_source(&client, &url).await {
            fetched.push(source);
        }
    }
    if fetched.is_empty() {
        return empty_draft("trusted_source_unavailable");
    }
    let prompt = json!({
        "task": "Find trusted source evidence and draft medication onboarding fields. Return JSON with medication, doses, sources, errors. Cite only the fetched URLs and quote exact evidence text.",
        "medication_identity": identity,
        "fetched_sources": fetched.iter().map(|source| json!({"url":source.url,"title":source.title,"text":source.text})).collect::<Vec<_>>()
    }).to_string();
    match call_provider(&config, &prompt).await {
        Ok(content) => validated_draft(&content, &fetched),
        Err(()) => empty_draft("suggestion_unavailable"),
    }
}

async fn record_suggestion(
    db: &DatabaseTransaction,
    context: &crate::AuthContext,
    request_id: &str,
    identity: &Value,
    draft: &Value,
) -> Result<(), ApiError> {
    let identity_hash = hex::encode(Sha256::digest(
        identity.to_string().trim().to_ascii_lowercase().as_bytes(),
    ));
    let source_count = draft
        .get("sources")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let dose_count = draft
        .get("doses")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let error_count = draft
        .get("errors")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let object = json!({
        "identity_hash": identity_hash,
        "source_count": source_count,
        "dose_count": dose_count,
        "error_count": error_count,
        "result_status": if error_count > 0 { "error" } else { "found" }
    });
    version::ActiveModel {
        item_type: Set("AiMedicationSuggestion".to_owned()),
        item_id: Set(0),
        event: Set("ai_medication/suggestion".to_owned()),
        object: Set(Some(object.to_string())),
        object_changes: Set(None),
        whodunnit: Set(Some(context.user_id.to_string())),
        request_id: Set(Some(request_id.to_owned())),
        household_id: Set(Some(context.membership.household_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        audit_context: Set(json!({"request_id": request_id, "actor_user_id": context.user_id, "household_id": context.membership.household_id})),
        created_at: Set(Some(Utc::now().naive_utc())),
        ..Default::default()
    }.insert(db).await.map_err(database_error)?;
    Ok(())
}

pub(crate) async fn ai_medication_suggestions(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let (allowed, _) = finder_allowed(&db, &context).await?;
    if !allowed {
        return error_response(
            db,
            &context,
            "POST",
            CONTROLLER,
            POLICY,
            "create",
            StatusCode::FORBIDDEN,
            "forbidden",
            "Forbidden",
            None,
        )
        .await;
    }
    let enabled = ["1", "true", "yes", "on"].contains(
        &std::env::var("MEDTRACKER_AI_MEDICATION_HELP_ENABLED")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
    );
    let household = household::Entity::find_by_id(household_id)
        .one(&db)
        .await
        .map_err(database_error)?;
    if !enabled || household.is_none_or(|household| household.subscription_plan != "family_plus") {
        return error_response(
            db,
            &context,
            "POST",
            CONTROLLER,
            POLICY,
            "create",
            StatusCode::NOT_FOUND,
            "not_found",
            "Not found",
            None,
        )
        .await;
    }
    let Some(identity) = parsed_identity(&body) else {
        return error_response(
            db,
            &context,
            "POST",
            CONTROLLER,
            POLICY,
            "create",
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Validation failed",
            Some(json!({"medication": ["is invalid"]})),
        )
        .await;
    };
    let draft = generate(&identity).await;
    let request_id = Uuid::new_v4().to_string();
    record_suggestion(&db, &context, &request_id, &identity, &draft).await?;
    finish_with_request_id(
        db,
        &context,
        &request_id,
        "POST",
        CONTROLLER,
        POLICY,
        "create",
        StatusCode::OK,
        true,
        json!({"data": draft}),
        None,
    )
    .await
}
