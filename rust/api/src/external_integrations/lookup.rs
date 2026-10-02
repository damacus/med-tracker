use super::{bounded_json, configured_url, http_client};
use crate::entities::{grant, location, medication, review_evidence, version};
use crate::medication_management::{
    error_response, finish, finish_with_request_id, household_manager, request_context,
};
use crate::{database_error, scope, ApiError, AppState, AuthContext};
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait, QueryFilter, Set,
    Statement,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use url::form_urlencoded;
use uuid::Uuid;

const CONTROLLER: &str = "api/v1/medication_lookup";
const POLICY: &str = "MedicationPolicy";
const BASE_URL: &str = "https://ontology.nhs.uk/production1/fhir";
const TOKEN_URL: &str = "https://ontology.nhs.uk/authorisation/auth/realms/nhs-digital-terminology/protocol/openid-connect/token";

#[derive(Default)]
struct SearchQuery {
    query: String,
    form: Option<String>,
    strength: Option<String>,
}

impl SearchQuery {
    fn parse(raw: Option<&str>) -> Self {
        let mut result = Self::default();
        for (key, value) in form_urlencoded::parse(raw.unwrap_or_default().as_bytes()) {
            match key.as_ref() {
                "q" => result.query = value.trim().to_owned(),
                "form" => result.form = Some(value.into_owned()),
                "strength" => result.strength = Some(value.into_owned()),
                _ => {}
            }
        }
        result
    }
}

struct LookupEvent {
    event: &'static str,
    status: &'static str,
    count: usize,
}

async fn record_lookup_events(
    db: &DatabaseTransaction,
    context: &AuthContext,
    request_id: &str,
    query: &str,
    events: Vec<LookupEvent>,
) -> Result<(), ApiError> {
    if events.is_empty() {
        return Ok(());
    }
    let query_hash = hex::encode(Sha256::digest(query.trim().to_ascii_lowercase().as_bytes()));
    let now = chrono::Utc::now().naive_utc();
    let records = events.into_iter().map(|event| version::ActiveModel {
        item_type: Set("ExternalMedicineLookup".to_owned()),
        item_id: Set(0),
        event: Set(event.event.to_owned()),
        object: Set(Some(json!({
            "query_hash": query_hash,
            "result_status": event.status,
            "result_count": event.count
        }).to_string())),
        object_changes: Set(None),
        whodunnit: Set(Some(context.user_id.to_string())),
        request_id: Set(Some(request_id.to_owned())),
        household_id: Set(Some(context.membership.household_id)),
        actor_membership_id: Set(Some(context.membership.id)),
        audit_context: Set(json!({"request_id": request_id, "actor_user_id": context.user_id, "household_id": context.membership.household_id})),
        created_at: Set(Some(now)),
        ..Default::default()
    }).collect::<Vec<_>>();
    version::Entity::insert_many(records)
        .exec(db)
        .await
        .map_err(database_error)?;
    Ok(())
}

pub(super) async fn finder_allowed(
    db: &DatabaseTransaction,
    context: &AuthContext,
) -> Result<(bool, bool), ApiError> {
    if household_manager(context) {
        return Ok((true, true));
    }
    let grants = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(context.membership.household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(context.membership.id))
        .filter(grant::Column::RevokedAt.is_null())
        .all(db)
        .await
        .map_err(database_error)?;
    let now = chrono::Utc::now().naive_utc();
    let active = grants
        .iter()
        .filter(|grant| grant.expires_at.is_none_or(|date| date > now));
    let levels: HashSet<&str> = active.map(|grant| grant.access_level.as_str()).collect();
    Ok((
        levels.contains("view") || levels.contains("record") || levels.contains("manage"),
        levels.contains("manage"),
    ))
}

fn barcode_candidates(query: &str) -> Vec<String> {
    let normalized: String = query.chars().filter(char::is_ascii_digit).collect();
    if !matches!(normalized.len(), 13 | 14) {
        return vec![];
    }
    let alternate = if normalized.len() == 13 {
        format!("0{normalized}")
    } else {
        normalized
            .strip_prefix('0')
            .unwrap_or(&normalized)
            .to_owned()
    };
    if alternate == normalized {
        vec![normalized]
    } else {
        vec![normalized, alternate]
    }
}

async fn source_priority(db: &DatabaseTransaction) -> Result<Vec<String>, ApiError> {
    let row = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT medicine_lookup_source_priority FROM app_settings ORDER BY id LIMIT 1"
                .to_owned(),
        ))
        .await
        .map_err(database_error)?;
    let configured = row
        .and_then(|row| {
            row.try_get::<Value>("", "medicine_lookup_source_priority")
                .ok()
        })
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();
    let known = [
        "imported_catalog",
        "local_nhs_dmd",
        "cached_open_products_facts",
        "open_products_facts",
        "curated_catalog",
    ];
    let mut priority: Vec<String> = configured
        .into_iter()
        .filter_map(|value| value.as_str().map(str::to_owned))
        .filter(|value| known.contains(&value.as_str()))
        .collect();
    for source in known {
        if !priority.iter().any(|item| item == source) {
            priority.push(source.to_owned());
        }
    }
    Ok(priority)
}

async fn local_barcode_rows(
    db: &DatabaseTransaction,
    candidates: &[String],
) -> Result<(Vec<Value>, Vec<Value>), ApiError> {
    let second = candidates.get(1).unwrap_or(&candidates[0]);
    let catalog = db.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT gtin, code, display, system, concept_class, source FROM barcode_catalog_entries WHERE gtin IN ($1, $2) ORDER BY id",
        [candidates[0].clone().into(), second.clone().into()],
    )).await.map_err(database_error)?;
    let catalog = catalog.into_iter().map(|row| -> Result<Value, ApiError> {
        Ok(json!({
            "gtin":row.try_get::<String>("","gtin").map_err(database_error)?,
            "code":row.try_get::<Option<String>>("","code").map_err(database_error)?,
            "display":row.try_get::<String>("","display").map_err(database_error)?,
            "system":row.try_get::<Option<String>>("","system").map_err(database_error)?,
            "concept_class":row.try_get::<Option<String>>("","concept_class").map_err(database_error)?,
            "source":row.try_get::<String>("","source").map_err(database_error)?
        }))
    }).collect::<Result<Vec<_>,_>>()?;
    let nhs = db.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT gtin, code, COALESCE(NULLIF(vmp_name, ''), display) AS display, system, concept_class FROM nhs_dmd_barcodes WHERE gtin IN ($1, $2)",
        [candidates[0].clone().into(), second.clone().into()],
    )).await.map_err(database_error)?;
    let nhs = nhs.into_iter().map(|row| -> Result<Value, ApiError> {
        Ok(json!({
            "gtin":row.try_get::<String>("","gtin").map_err(database_error)?,
            "code":row.try_get::<String>("","code").map_err(database_error)?,
            "display":row.try_get::<String>("","display").map_err(database_error)?,
            "system":row.try_get::<String>("","system").map_err(database_error)?,
            "concept_class":row.try_get::<Option<String>>("","concept_class").map_err(database_error)?,
            "source":"nhs_dmd"
        }))
    }).collect::<Result<Vec<_>,_>>()?;
    Ok((catalog, nhs))
}

async fn catalogue_match(
    db: &DatabaseTransaction,
    query: &str,
    events: &mut Vec<LookupEvent>,
) -> Result<Option<Value>, ApiError> {
    let candidates = barcode_candidates(query);
    if candidates.is_empty() {
        return Ok(None);
    }
    let priority = source_priority(db).await?;
    let (catalog, nhs) = local_barcode_rows(db, &candidates).await?;
    let mut selected = None;
    'candidates: for candidate in candidates {
        for source in &priority {
            let found = match source.as_str() {
                "imported_catalog" => catalog
                    .iter()
                    .find(|row| row["gtin"] == candidate && row["source"] != "open_products_facts")
                    .cloned(),
                "local_nhs_dmd" => nhs.iter().find(|row| row["gtin"] == candidate).cloned(),
                "cached_open_products_facts" => catalog
                    .iter()
                    .find(|row| row["gtin"] == candidate && row["source"] == "open_products_facts")
                    .cloned(),
                "open_products_facts" => {
                    let result = super::facts::opf_barcode(&candidate).await;
                    events.push(LookupEvent {
                        event: "open_products_facts/barcode_lookup",
                        status: if result.is_some() {
                            "success"
                        } else {
                            "not_found"
                        },
                        count: usize::from(result.is_some()),
                    });
                    result
                }
                "curated_catalog" => super::facts::curated_barcode(&candidate),
                _ => None,
            };
            if found.is_some() {
                selected = found;
                break 'candidates;
            }
        }
    }
    if let Some(found) = selected
        .as_ref()
        .filter(|found| found["source"] == "open_products_facts")
    {
        let gtin = found
            .get("gtin")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let display = found
            .get("display")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let system = found
            .get("system")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let concept_class = found
            .get("concept_class")
            .and_then(Value::as_str)
            .unwrap_or_default();
        db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO barcode_catalog_entries (gtin, display, source, system, concept_class, created_at, updated_at) VALUES ($1, $2, 'open_products_facts', $3, $4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT (source, gtin) DO NOTHING",
            [gtin.into(), display.into(), system.into(), concept_class.into()],
        )).await.map_err(database_error)?;
    }
    Ok(selected)
}

async fn provider_urls(db: &DatabaseTransaction) -> Result<(String, String), ApiError> {
    let row = db.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT medicine_lookup_base_url, medicine_lookup_token_url FROM app_settings ORDER BY id LIMIT 1".to_owned(),
    )).await.map_err(database_error)?;
    let base = row
        .as_ref()
        .and_then(|row| row.try_get::<String>("", "medicine_lookup_base_url").ok())
        .unwrap_or_else(|| BASE_URL.to_owned());
    let token = row
        .as_ref()
        .and_then(|row| row.try_get::<String>("", "medicine_lookup_token_url").ok())
        .unwrap_or_else(|| TOKEN_URL.to_owned());
    Ok((
        std::env::var("MEDTRACKER_NHS_DMD_BASE_URL").unwrap_or(base),
        std::env::var("MEDTRACKER_NHS_DMD_TOKEN_URL").unwrap_or(token),
    ))
}

async fn remote_results(db: &DatabaseTransaction, query: &str) -> Result<Vec<Value>, ()> {
    let client_id = std::env::var("NHS_DMD_CLIENT_ID").map_err(|_| ())?;
    let client_secret = std::env::var("NHS_DMD_CLIENT_SECRET").map_err(|_| ())?;
    if client_id.is_empty() || client_secret.is_empty() {
        return Err(());
    }
    let (base, token) = provider_urls(db).await.map_err(|_| ())?;
    let base_http = std::env::var("MEDTRACKER_NHS_DMD_BASE_URL").is_ok();
    let token_http = std::env::var("MEDTRACKER_NHS_DMD_TOKEN_URL").is_ok();
    let mut base = configured_url(&base, base_http).ok_or(())?;
    if !base.path().ends_with('/') {
        let path = format!("{}/", base.path());
        base.set_path(&path);
    }
    let token = configured_url(&token, token_http).ok_or(())?;
    let client = http_client(10).map_err(|_| ())?;
    let token_response = client
        .post(token)
        .form(&[
            ("grant_type", "client_credentials"),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
        ])
        .send()
        .await
        .map_err(|_| ())?;
    if !token_response.status().is_success() {
        return Err(());
    }
    let token_body = bounded_json(token_response).await.ok_or(())?;
    let access_token = token_body
        .get("access_token")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(())?;
    let mut results = Vec::new();
    let mut seen = HashSet::new();
    for value_set in [
        "https://dmd.nhs.uk/ValueSet/VMP",
        "https://dmd.nhs.uk/ValueSet/AMP",
    ] {
        let mut url = base.join("ValueSet/$expand").map_err(|_| ())?;
        url.query_pairs_mut()
            .append_pair("url", value_set)
            .append_pair("count", "20")
            .append_pair("filter", query);
        let response = client
            .get(url)
            .header("accept", "application/json")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|_| ())?;
        if !response.status().is_success() {
            return Err(());
        }
        let body = bounded_json(response).await.ok_or(())?;
        let entries = body
            .pointer("/expansion/contains")
            .and_then(Value::as_array)
            .ok_or(())?;
        for entry in entries {
            if let Some(code) = entry.get("code").and_then(Value::as_str) {
                if seen.insert(code.to_owned()) {
                    results.push(entry.clone());
                }
            }
        }
    }
    Ok(results)
}

fn supplement_query(query: &str) -> bool {
    let words = query.to_ascii_lowercase();
    let tokens: HashSet<&str> = words
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| word.len() >= 3)
        .collect();
    [
        "supplement",
        "supplements",
        "vitamin",
        "vitamins",
        "multivitamin",
        "multivitamins",
    ]
    .iter()
    .any(|word| tokens.contains(word))
}

fn low_relevance(query: &str, results: &[Value]) -> bool {
    let tokens: HashSet<String> = query
        .to_ascii_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| word.len() >= 3)
        .map(str::to_owned)
        .collect();
    !tokens.is_empty()
        && results.iter().all(|result| {
            let display = result
                .get("display")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_ascii_lowercase();
            tokens.iter().all(|token| {
                !display
                    .split(|character: char| !character.is_alphanumeric())
                    .any(|word| word == token)
            })
        })
}

async fn search_results(
    db: &DatabaseTransaction,
    query: &str,
    events: &mut Vec<LookupEvent>,
) -> Result<(Vec<Value>, Option<String>, Option<String>), ()> {
    let normalized_barcode = barcode_candidates(query).first().cloned();
    if let Some(barcode) = normalized_barcode.as_deref() {
        match remote_results(db, query).await {
            Ok(results) if !results.is_empty() => {
                return Ok((
                    results,
                    Some(barcode.to_owned()),
                    Some("nhs_dmd_api".to_owned()),
                ))
            }
            Ok(_) | Err(()) => {
                let supplement = super::facts::off_barcode(barcode).await;
                events.push(LookupEvent {
                    event: "open_food_facts/barcode_lookup",
                    status: if supplement.is_some() {
                        "success"
                    } else {
                        "not_found"
                    },
                    count: usize::from(supplement.is_some()),
                });
                if let Some(result) = supplement {
                    return Ok((
                        vec![result],
                        Some(barcode.to_owned()),
                        Some("open_food_facts".to_owned()),
                    ));
                }
            }
        }
    }
    match remote_results(db, query).await {
        Ok(mut results) => {
            if !query.bytes().all(|byte| byte.is_ascii_digit())
                && ((results.is_empty() && supplement_query(query))
                    || (!results.is_empty() && low_relevance(query, &results)))
            {
                let supplements = super::facts::off_search(query).await;
                events.push(LookupEvent {
                    event: "open_food_facts/search",
                    status: if supplements.is_empty() {
                        "not_found"
                    } else {
                        "success"
                    },
                    count: supplements.len(),
                });
                let mut seen: HashSet<(String, String, String, String)> =
                    results.iter().map(result_key).collect();
                for supplement in supplements {
                    if seen.insert(result_key(&supplement)) {
                        results.push(supplement);
                    }
                }
            }
            Ok((results, None, None))
        }
        Err(()) => {
            if supplement_query(query) {
                let supplements = super::facts::off_search(query).await;
                events.push(LookupEvent {
                    event: "open_food_facts/search",
                    status: if supplements.is_empty() {
                        "not_found"
                    } else {
                        "success"
                    },
                    count: supplements.len(),
                });
                if !supplements.is_empty() {
                    return Ok((supplements, None, None));
                }
            }
            Err(())
        }
    }
}

fn result_key(result: &Value) -> (String, String, String, String) {
    let field = |key| {
        result
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    (
        field("system"),
        field("code"),
        field("barcode"),
        field("display"),
    )
}

fn normalized_form(value: Option<&str>) -> Option<String> {
    let value = value?.to_ascii_lowercase();
    let normalized = value.trim().replace([' ', '-'], "_");
    let form = match normalized.as_str() {
        "tablet" | "tablets" | "caplet" | "caplets" => "tablet",
        "capsule" | "capsules" => "capsule",
        "liquid" | "solution" | "suspension" | "syrup" | "oral_solution" | "oral_suspension" => {
            "liquid"
        }
        "cream" | "ointment" | "gel" => "cream",
        "inhaler" | "inhalation" => "inhaler",
        "injection" | "injectable" => "injection",
        "patch" | "patches" => "patch",
        "drops" | "eye_drops" | "ear_drops" => "drops",
        "spray" | "sprays" => "spray",
        "powder" | "powders" | "sachet" | "sachets" => "powder",
        _ => return None,
    };
    Some(form.to_owned())
}

fn matches_form(display: &str, form: &str) -> bool {
    let display = display.to_ascii_lowercase();
    let terms: &[&str] = match form {
        "tablet" => &["tablet", "caplet"],
        "capsule" => &["capsule"],
        "liquid" => &["liquid", "solution", "suspension", "syrup", " ml"],
        "cream" => &["cream", "ointment", "gel"],
        "inhaler" => &["inhaler", "inhalation"],
        "injection" => &["injection", "syringe", "ampoule", "vial"],
        "patch" => &["patch"],
        "drops" => &["drops"],
        "spray" => &["spray"],
        "powder" => &["powder", "sachet", "granules"],
        _ => &[],
    };
    terms.iter().any(|term| display.contains(term))
}

fn normalized_strength(value: Option<&str>) -> Option<String> {
    let value = value?.to_ascii_lowercase().replace([' ', ','], "");
    if value.is_empty() || !value.bytes().any(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(value)
}

fn concept_label(value: Option<&str>) -> Option<&str> {
    match value {
        Some("VMP") => Some("Virtual Medicinal Product"),
        Some("AMP") => Some("Actual Medicinal Product"),
        Some("VMPP") => Some("Virtual Medicinal Product Pack"),
        Some("AMPP") => Some("Actual Medicinal Product Pack"),
        other => other,
    }
}

fn lookup_result(entry: &Value, barcode: Option<&str>) -> Option<Value> {
    let display = entry.get("display")?.as_str()?;
    let code = entry.get("code").cloned().unwrap_or(Value::Null);
    let system = entry.get("system").cloned().unwrap_or(Value::Null);
    let concept_class = entry
        .get("concept_class")
        .cloned()
        .or_else(|| {
            entry
                .get("extension")?
                .as_array()?
                .iter()
                .find(|extension| {
                    extension.get("url").and_then(Value::as_str)
                        == Some("http://hl7.org/fhir/StructureDefinition/valueset-concept-comments")
                })?
                .get("valueString")
                .cloned()
        })
        .unwrap_or(Value::Null);
    let concept = concept_class.as_str();
    let source_label = match system.as_str() {
        Some("https://dmd.nhs.uk") => Some("NHS dm+d"),
        Some("https://world.openfoodfacts.org") => Some("Open Food Facts"),
        Some("https://world.openproductsfacts.org") => Some("Open Products Facts"),
        other => other,
    };
    let match_reason = barcode.map(|_| "barcode_match");
    Some(json!({
        "barcode": entry.get("barcode").and_then(Value::as_str).or(barcode), "code": code,
        "name": entry.get("name"), "description": entry.get("description"),
        "display": display, "system": system, "concept_class": concept_class,
        "category": entry.get("category"), "trade_family": null, "trade_family_group": null,
        "package_size": entry.get("package_size"), "package_quantity": entry.get("package_quantity"), "package_unit": entry.get("package_unit"),
        "directions": entry.get("directions"), "warnings": entry.get("warnings"), "pil_url": entry.get("pil_url"), "spc_url": entry.get("spc_url"),
        "concept_class_label": concept_label(concept), "source_label": source_label,
        "match_reason": match_reason,
        "match_reason_label": match_reason.map(|_| "Barcode match"),
        "related_medications": [], "review_prompts": [], "review_prompt_filter": {"hidden_count": 0}
    }))
}

async fn enrich_families(db: &DatabaseTransaction, results: &mut [Value]) -> Result<(), ApiError> {
    let candidates: Vec<(String, String)> = results
        .iter()
        .filter_map(|result| {
            let kind = result.get("concept_class")?.as_str()?;
            if !matches!(kind, "AMP" | "AMPP") {
                return None;
            }
            Some((result.get("code")?.as_str()?.to_owned(), kind.to_owned()))
        })
        .collect();
    if candidates.is_empty() {
        return Ok(());
    }
    let placeholders = (0..candidates.len())
        .map(|index| format!("(${}::text, ${}::text)", index * 2 + 1, index * 2 + 2))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT input.code AS result_code, family.code AS family_code, family.name AS family_name, family_group.code AS group_code, family_group.name AS group_name \
         FROM (VALUES {placeholders}) AS input(code, concept_class) \
         LEFT JOIN nhs_dmd_ampp_relationships relationship ON input.concept_class = 'AMPP' AND relationship.ampp_code = input.code \
         LEFT JOIN nhs_dmd_barcodes barcode ON barcode.code = input.code \
         JOIN nhs_dmd_amp_trade_families membership ON membership.amp_code = CASE WHEN input.concept_class = 'AMPP' THEN COALESCE(barcode.amp_code, relationship.amp_code) ELSE COALESCE(barcode.amp_code, input.code) END \
         JOIN nhs_dmd_trade_families family ON family.id = membership.trade_family_id \
         LEFT JOIN nhs_dmd_trade_family_groups family_group ON family_group.id = family.trade_family_group_id"
    );
    let values: Vec<sea_orm::Value> = candidates
        .into_iter()
        .flat_map(|(code, kind)| [code.into(), kind.into()])
        .collect();
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            values,
        ))
        .await
        .map_err(database_error)?;
    let mut families = HashMap::new();
    for row in rows {
        let code: String = row.try_get("", "result_code").map_err(database_error)?;
        let family_code: String = row.try_get("", "family_code").map_err(database_error)?;
        let family_name: String = row.try_get("", "family_name").map_err(database_error)?;
        let group_code: Option<String> = row.try_get("", "group_code").map_err(database_error)?;
        let group_name: Option<String> = row.try_get("", "group_name").map_err(database_error)?;
        let group = group_code
            .zip(group_name)
            .map(|(code, name)| json!({"code":code,"name":name}));
        families.insert(
            code,
            (json!({"code":family_code,"name":family_name}), group),
        );
    }
    for result in results {
        if let Some((family, group)) = result
            .get("code")
            .and_then(Value::as_str)
            .and_then(|code| families.get(code))
        {
            result["trade_family"] = family.clone();
            result["trade_family_group"] = group.clone().unwrap_or(Value::Null);
        }
    }
    Ok(())
}

async fn enrich_existing(
    db: &DatabaseTransaction,
    context: &AuthContext,
    results: &mut [Value],
    barcode: Option<&str>,
) -> Result<(), ApiError> {
    let visible = scope(context.membership.household_id, &context.membership)
        .all(db)
        .await
        .map_err(database_error)?;
    if visible.is_empty() {
        return Ok(());
    }
    let ids: Vec<i64> = visible
        .iter()
        .map(|medication| medication.location_id)
        .collect();
    let locations: HashMap<i64, String> = location::Entity::find()
        .filter(location::Column::Id.is_in(ids))
        .all(db)
        .await
        .map_err(database_error)?
        .into_iter()
        .map(|location| (location.id, location.name))
        .collect();
    let family_members = related_family_members(db, &visible).await?;
    let evidence = review_evidence::Entity::find()
        .filter(review_evidence::Column::MatchStatus.is_in(["unreviewed", "reviewed_pair"]))
        .all(db)
        .await
        .map_err(database_error)?;
    for result in results {
        let code = result.get("code").and_then(Value::as_str);
        let match_record = visible.iter().find(|record| {
            barcode.is_some_and(|barcode| record.barcode.as_deref() == Some(barcode))
                || code.is_some_and(|code| record.dmd_code.as_deref() == Some(code))
        });
        if let Some(record) = match_record {
            result["existing_medication"] = medication_payload(record, &locations);
        }
        if let Some(family_code) = result.pointer("/trade_family/code").and_then(Value::as_str) {
            if let Some(members) = family_members.get(family_code) {
                result["related_medications"] = json!(members
                    .iter()
                    .filter(|member_id| match_record.is_none_or(|record| record.id != **member_id))
                    .filter_map(|member_id| visible.iter().find(|record| record.id == *member_id))
                    .map(|record| medication_payload(record, &locations))
                    .collect::<Vec<_>>());
            }
        }
        let candidate = result
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .or_else(|| result.get("display").and_then(Value::as_str))
            .unwrap_or_default();
        let mut prompts = Vec::new();
        let mut hidden_count = 0;
        for record in &visible {
            let existing_name = medication_name(record);
            for matched in crate::review_evidence::matches(candidate, existing_name, &evidence) {
                if matched.risk == "low" || matched.confidence == "low" {
                    hidden_count += 1;
                } else {
                    prompts.push(review_payload(&matched, existing_name));
                }
            }
        }
        result["review_prompts"] = json!(prompts);
        result["review_prompt_filter"] = json!({"hidden_count": hidden_count});
    }
    Ok(())
}

fn medication_name(record: &medication::Model) -> &str {
    record
        .friendly_name
        .as_deref()
        .filter(|name| !name.is_empty())
        .or(record.name.as_deref())
        .unwrap_or("Medication")
}

fn medication_payload(record: &medication::Model, locations: &HashMap<i64, String>) -> Value {
    json!({
        "id": record.id, "name": medication_name(record),
        "location": locations.get(&record.location_id).map(String::as_str).unwrap_or("Location"),
        "current_supply": record.current_supply.map(|value| value.to_string()).unwrap_or_else(|| "0".to_owned())
    })
}

fn review_payload(
    matched: &crate::review_evidence::EvidenceMatch<'_>,
    existing_name: &str,
) -> Value {
    let label = |value: &str| match value {
        "high" => "High",
        "moderate" => "Moderate",
        "low" => "Low",
        _ => "Unknown - unclassified",
    };
    json!({
        "evidence_record_id": matched.evidence.id,
        "risk_level": matched.risk,
        "risk_level_label": label(&matched.risk),
        "match_confidence": matched.confidence,
        "match_confidence_label": label(&matched.confidence),
        "matched_term": matched.matched_term,
        "match_type": matched.match_type,
        "source_instruction": matched.instruction,
        "match_reason": matched.reason,
        "interacting_medication_name": existing_name,
        "description": "Public medicine-label evidence suggests this combination may be worth reviewing with a pharmacist, nurse, GP, or prescriber.",
        "source_name": matched.evidence.source_name,
        "source_checked_on": matched.evidence.retrieved_on,
        "source_version": matched.evidence.source_version,
        "source_effective_on": matched.evidence.source_effective_on,
        "source_url": matched.evidence.source_url,
        "evidence_text": matched.excerpt.chars().take(500).collect::<String>()
    })
}

async fn related_family_members(
    db: &DatabaseTransaction,
    visible: &[medication::Model],
) -> Result<HashMap<String, Vec<i64>>, ApiError> {
    let ids: Vec<i64> = visible.iter().map(|record| record.id).collect();
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let placeholders = (1..=ids.len())
        .map(|index| format!("${index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT DISTINCT medications.id AS medication_id, family.code AS family_code \
         FROM medications JOIN nhs_dmd_barcodes barcode ON barcode.gtin IN \
         (medications.barcode, CASE WHEN char_length(medications.barcode) = 13 THEN '0' || medications.barcode END, \
         CASE WHEN char_length(medications.barcode) = 14 AND left(medications.barcode, 1) = '0' THEN substring(medications.barcode FROM 2) END) \
         JOIN nhs_dmd_amp_trade_families membership ON membership.amp_code = barcode.amp_code \
         JOIN nhs_dmd_trade_families family ON family.id = membership.trade_family_id \
         WHERE medications.id IN ({placeholders})"
    );
    let values: Vec<sea_orm::Value> = ids.into_iter().map(Into::into).collect();
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            values,
        ))
        .await
        .map_err(database_error)?;
    let mut groups: HashMap<String, Vec<i64>> = HashMap::new();
    for row in rows {
        let family: String = row.try_get("", "family_code").map_err(database_error)?;
        let medication_id: i64 = row.try_get("", "medication_id").map_err(database_error)?;
        groups.entry(family).or_default().push(medication_id);
    }
    Ok(groups)
}

pub(crate) async fn medication_lookup(
    State(state): State<AppState>,
    Path(household_id): Path<i64>,
    raw_query: RawQuery,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (db, context) = request_context(&state, &headers, household_id).await?;
    let (allowed, can_create) = finder_allowed(&db, &context).await?;
    if !allowed {
        return error_response(
            db,
            &context,
            "GET",
            CONTROLLER,
            POLICY,
            "show",
            StatusCode::FORBIDDEN,
            "forbidden",
            "Forbidden",
            None,
        )
        .await;
    }
    let query = SearchQuery::parse(raw_query.0.as_deref());
    if query.query.is_empty() {
        return finish(
            db,
            &context,
            "GET",
            CONTROLLER,
            POLICY,
            "show",
            StatusCode::OK,
            true,
            json!({"results": [], "permissions": {"can_create": can_create, "can_update": false}}),
            None,
        )
        .await;
    }
    let request_id = Uuid::new_v4().to_string();
    let mut events = Vec::new();
    let catalogue = catalogue_match(&db, &query.query, &mut events).await?;
    let (raw_results, resolved_query, barcode, barcode_source) = if let Some(local) = catalogue {
        let resolved_query = local
            .get("display")
            .and_then(Value::as_str)
            .unwrap_or(&query.query)
            .to_owned();
        let barcode = barcode_candidates(&query.query)
            .into_iter()
            .next()
            .unwrap_or_else(|| query.query.clone());
        let source = local
            .get("source")
            .and_then(Value::as_str)
            .map(str::to_owned);
        (vec![local], resolved_query, Some(barcode), source)
    } else {
        match search_results(&db, &query.query, &mut events).await {
            Ok((results, barcode, source)) => (results, query.query.clone(), barcode, source),
            Err(()) => {
                events.push(LookupEvent {
                    event: "nhs_dmd/search",
                    status: "error",
                    count: 0,
                });
                record_lookup_events(&db, &context, &request_id, &query.query, events).await?;
                return finish_with_request_id(db, &context, &request_id, "GET", CONTROLLER, POLICY, "show", StatusCode::SERVICE_UNAVAILABLE, true,
                    json!({"results": [], "error": "Medication search is temporarily unavailable."}), None).await;
            }
        }
    };
    events.push(LookupEvent {
        event: "nhs_dmd/search",
        status: if raw_results.is_empty() {
            "not_found"
        } else {
            "success"
        },
        count: raw_results.len(),
    });
    let form = normalized_form(query.form.as_deref());
    let strength = normalized_strength(query.strength.as_deref());
    let filtered = raw_results.into_iter().filter(|entry| {
        let display = entry
            .get("display")
            .and_then(Value::as_str)
            .unwrap_or_default();
        form.as_deref()
            .is_none_or(|form| matches_form(display, form))
            && strength.as_deref().is_none_or(|strength| {
                display
                    .to_ascii_lowercase()
                    .replace(' ', "")
                    .contains(strength)
            })
    });
    let mut results: Vec<Value> = filtered
        .filter_map(|entry| lookup_result(&entry, barcode.as_deref()))
        .collect();
    enrich_families(&db, &mut results).await?;
    enrich_existing(&db, &context, &mut results, barcode.as_deref()).await?;
    record_lookup_events(&db, &context, &request_id, &query.query, events).await?;
    finish_with_request_id(db, &context, &request_id, "GET", CONTROLLER, POLICY, "show", StatusCode::OK, true,
        json!({
            "results": results,
            "review_guidance": {"status": "available"},
            "query": resolved_query,
            "barcode": barcode,
            "barcode_resolution": barcode_source.map(|source| json!({"status": "resolved", "source": source})),
            "form": form,
            "strength": strength,
            "permissions": {"can_create": can_create, "can_update": false}
        }), None).await
}
