use crate::models::{
    access::{self, PersonAccess, TenantTransaction},
    care::medications,
    entities::{barcode_catalog_entry, household, location, medication, nhs_dmd_barcode},
    errors::OperationError,
};
use crate::views::medication_lookup::{
    BarcodeResolution, Details, ExistingMedication, Lookup, Permissions, Product, ReviewGuidance,
    StockMatch,
};
use loco_rs::model::TenantQueryExt;
use sea_orm::sea_query::{Expr, ExprTrait, Func};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    Statement, TransactionTrait,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
mod enrichment;
mod matching;
pub(crate) mod open_food_facts;

pub const LOOKUP_SOURCES: [&str; 5] = [
    "imported_catalog",
    "local_nhs_dmd",
    "cached_open_products_facts",
    "curated_catalog",
    open_food_facts::SOURCE,
];

#[derive(Default, Deserialize)]
pub struct Search {
    #[serde(default)]
    pub q: String,
    pub form: Option<String>,
    pub strength: Option<String>,
}

pub(crate) struct Found {
    pub body: Lookup,
    pub matches: Vec<StockMatch>,
}

pub fn normalize_gtin(value: &str) -> String {
    value.chars().filter(char::is_ascii_digit).collect()
}

pub fn barcode_candidates(value: &str) -> Vec<String> {
    let value = normalize_gtin(value);
    let value = if value.len() == 12 {
        format!("0{value}")
    } else {
        value
    };
    if !matches!(value.len(), 13 | 14) {
        return Vec::new();
    }
    let mut candidates = vec![value.clone()];
    if value.len() == 13 {
        candidates.push(format!("0{value}"));
    } else if let Some(stripped) = value.strip_prefix('0') {
        candidates.push(stripped.into());
    }
    candidates
}

pub fn curated_products() -> Result<Vec<Value>, OperationError> {
    let data: Value = serde_yaml_ng::from_str(include_str!(
        "../../../assets/catalogue/curated-products.yml"
    ))
    .map_err(|_| OperationError::Unavailable)?;
    Ok(data["products"].as_array().cloned().unwrap_or_default())
}

fn product(
    display: &str,
    barcode: Option<&str>,
    code: Option<&str>,
    system: Option<&str>,
    concept: Option<&str>,
    source: &str,
) -> Product {
    Product {
        barcode: barcode.map(str::to_owned),
        code: code.map(str::to_owned),
        display: display.to_owned(),
        system: system.map(str::to_owned),
        concept_class: concept.map(str::to_owned),
        concept_class_label: match concept {
            Some("VMP") => Some("Virtual Medicinal Product"),
            Some("AMP") => Some("Actual Medicinal Product"),
            Some("VMPP") => Some("Virtual Medicinal Product Pack"),
            Some("AMPP") => Some("Actual Medicinal Product Pack"),
            value => value,
        }
        .map(str::to_owned),
        source_label: Some(
            if system == Some("https://dmd.nhs.uk") {
                "NHS dm+d"
            } else {
                system.unwrap_or(source)
            }
            .to_owned(),
        ),
        match_reason: barcode.map(|_| "barcode_match".to_owned()),
        match_reason_label: barcode.map(|_| "Barcode match".to_owned()),
        ..Product::default()
    }
}

async fn priorities(tenant: &TenantTransaction) -> Result<Vec<String>, OperationError> {
    let row = tenant
        .transaction()
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT medicine_lookup_source_priority FROM app_settings ORDER BY id LIMIT 1",
        ))
        .await?;
    let configured: Value = row
        .map(|row| row.try_get("", "medicine_lookup_source_priority"))
        .transpose()?
        .unwrap_or(Value::Null);
    let mut ordered: Vec<String> = configured
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|source| LOOKUP_SOURCES.contains(source))
        .map(str::to_owned)
        .collect();
    for source in LOOKUP_SOURCES {
        if !ordered.iter().any(|item| item == source) {
            ordered.push(source.into());
        }
    }
    ordered.dedup();
    Ok(ordered)
}

async fn local_catalogue(
    tenant: &TenantTransaction,
    query: &str,
    sources: &[String],
    accepts: &impl Fn(&Product) -> bool,
) -> Result<Vec<(Product, String)>, OperationError> {
    let candidates = barcode_candidates(query);
    let numeric = !candidates.is_empty();
    let mut results = Vec::new();
    let alternatives = if numeric {
        candidates
    } else {
        vec![query.to_owned()]
    };
    for candidate in alternatives {
        for source in sources {
            match source.as_str() {
                "imported_catalog" | "cached_open_products_facts" => {
                    let mut rows = barcode_catalog_entry::Entity::find();
                    rows = if source == "cached_open_products_facts" {
                        rows.filter(barcode_catalog_entry::Column::Source.eq("open_products_facts"))
                    } else {
                        rows.filter(barcode_catalog_entry::Column::Source.ne("open_products_facts"))
                    };
                    rows = if numeric {
                        rows.filter(barcode_catalog_entry::Column::Gtin.eq(&candidate))
                    } else {
                        rows.filter(
                            sea_orm::Condition::any()
                                .add(
                                    Func::lower(Expr::col(barcode_catalog_entry::Column::Display))
                                        .like(format!("%{}%", query.to_lowercase())),
                                )
                                .add(barcode_catalog_entry::Column::Code.eq(query)),
                        )
                    };
                    for row in rows
                        .order_by_asc(barcode_catalog_entry::Column::Id)
                        .limit(50)
                        .all(tenant.transaction())
                        .await?
                    {
                        results.push((
                            product(
                                &row.display,
                                Some(&row.gtin),
                                row.code.as_deref(),
                                row.system.as_deref(),
                                row.concept_class.as_deref(),
                                &row.source,
                            ),
                            row.source,
                        ));
                    }
                }
                "local_nhs_dmd" => {
                    let mut rows = nhs_dmd_barcode::Entity::find();
                    rows = if numeric {
                        rows.filter(nhs_dmd_barcode::Column::Gtin.eq(&candidate))
                    } else {
                        rows.filter(
                            sea_orm::Condition::any()
                                .add(
                                    Func::lower(Expr::col(nhs_dmd_barcode::Column::Display))
                                        .like(format!("%{}%", query.to_lowercase())),
                                )
                                .add(
                                    Func::lower(Expr::col(nhs_dmd_barcode::Column::VmpName))
                                        .like(format!("%{}%", query.to_lowercase())),
                                )
                                .add(nhs_dmd_barcode::Column::Code.eq(query)),
                        )
                    };
                    for row in rows
                        .order_by_asc(nhs_dmd_barcode::Column::Id)
                        .limit(50)
                        .all(tenant.transaction())
                        .await?
                    {
                        let display = row
                            .vmp_name
                            .as_deref()
                            .filter(|value| !value.is_empty())
                            .unwrap_or(&row.display);
                        results.push((
                            product(
                                display,
                                Some(&row.gtin),
                                Some(&row.code),
                                Some(&row.system),
                                row.concept_class.as_deref(),
                                "nhs_dmd",
                            ),
                            "nhs_dmd".into(),
                        ));
                    }
                }
                "curated_catalog" => {
                    for entry in curated_products()? {
                        let display = entry["display"].as_str().unwrap_or("");
                        let hit = if numeric {
                            entry["gtin"].as_str() == Some(&candidate)
                        } else {
                            display.to_lowercase().contains(&query.to_lowercase())
                                || entry["code"].as_str() == Some(query)
                        };
                        if hit {
                            let mut value = product(
                                display,
                                entry["gtin"].as_str(),
                                entry["code"].as_str(),
                                entry["system"].as_str(),
                                entry["concept_class"].as_str(),
                                "curated",
                            );
                            value.category = entry["category"].as_str().map(str::to_owned);
                            value.description = entry["description"].as_str().map(str::to_owned);
                            value.warnings = entry["warnings"].as_str().map(str::to_owned);
                            results.push((value, "curated".into()));
                        }
                    }
                }
                _ => {}
            }
            if numeric {
                results.retain(|(product, _)| accepts(product));
                if !results.is_empty() {
                    results.truncate(1);
                    return Ok(results);
                }
            }
        }
    }
    let mut seen = HashSet::new();
    results.retain(|(entry, _)| {
        seen.insert((
            entry.system.clone(),
            entry.code.clone(),
            entry.barcode.clone(),
            entry.display.clone(),
        ))
    });
    results.truncate(50);
    Ok(results)
}

async fn catalogue(
    tenant: &TenantTransaction,
    query: &str,
    barcode: Option<&str>,
    provider: Option<&open_food_facts::ClientService>,
    accepts: &impl Fn(&Product) -> bool,
) -> Result<Vec<(Product, String)>, OperationError> {
    let sources = priorities(tenant).await?;
    let (Some(barcode), Some(provider)) = (barcode, provider) else {
        return local_catalogue(tenant, query, &sources, accepts).await;
    };
    let Some(position) = sources
        .iter()
        .position(|source| source == open_food_facts::SOURCE)
    else {
        return local_catalogue(tenant, query, &sources, accepts).await;
    };
    let (local_first, local_after) = sources.split_at(position);
    let matches = local_catalogue(tenant, query, local_first, accepts).await?;
    if !matches.is_empty() {
        return Ok(matches);
    }
    let failure = match provider.lookup(barcode).await {
        Ok(Some(found)) if accepts(&found) => {
            return Ok(vec![(found, open_food_facts::SOURCE.into())]);
        }
        Ok(_) => None,
        Err(error) => Some(error),
    };
    let matches = local_catalogue(tenant, query, local_after, accepts).await?;
    if matches.is_empty()
        && let Some(error) = failure
    {
        return Err(error);
    }
    Ok(matches)
}

pub struct StockMatchCandidate {
    pub barcode: String,
    pub code: String,
    pub name: String,
    pub display: String,
    pub package_unit: Option<String>,
}

impl StockMatchCandidate {
    fn from_product(value: &Product) -> Self {
        Self {
            barcode: value.barcode.clone().unwrap_or_default(),
            code: value.code.clone().unwrap_or_default(),
            name: value.name.clone().unwrap_or_default(),
            display: value.display.clone(),
            package_unit: value.package_unit.clone(),
        }
    }
}

pub async fn stock_matches(
    tenant: &TenantTransaction,
    candidate: &StockMatchCandidate,
) -> Result<Vec<medication::Model>, OperationError> {
    access::recheck(tenant).await?;
    let visible = access::medication_scope(tenant)
        .order_by_asc(medication::Column::Id)
        .all(tenant.transaction())
        .await?;
    Ok(matches_from(&visible, candidate))
}

fn matches_from(
    visible: &[medication::Model],
    candidate: &StockMatchCandidate,
) -> Vec<medication::Model> {
    let gtins = barcode_candidates(&candidate.barcode);
    let code = &candidate.code;
    let name = if candidate.name.is_empty() {
        &candidate.display
    } else {
        &candidate.name
    };
    let direct: Vec<_> = visible
        .iter()
        .filter(|row| {
            row.barcode
                .as_ref()
                .is_some_and(|value| gtins.contains(value))
        })
        .cloned()
        .collect();
    if !direct.is_empty() {
        return direct;
    }
    let coded: Vec<_> = visible
        .iter()
        .filter(|row| !code.is_empty() && row.dmd_code.as_deref() == Some(code.as_str()))
        .cloned()
        .collect();
    if !coded.is_empty() {
        return coded;
    }
    visible
        .iter()
        .filter(|row| {
            (gtins.is_empty() || row.barcode.as_deref().is_none_or(str::is_empty))
                && matching::compatible(name, candidate.package_unit.as_deref(), row)
        })
        .cloned()
        .collect()
}

pub(crate) async fn search(
    tenant: &TenantTransaction,
    search: &Search,
    provider: Option<&open_food_facts::ClientService>,
) -> Result<Found, OperationError> {
    access::recheck(tenant).await?;
    let can_create = medications::crud::can_create(tenant).await?;
    let can_refill = access::can_manage_household(tenant)
        || access::has_person_access(tenant, PersonAccess::View).await?;
    if !can_create && !can_refill {
        return Err(OperationError::Forbidden);
    }
    let query = search.q.trim();
    let permissions = Permissions {
        can_create,
        can_update: false,
    };
    if query.is_empty() {
        return Ok(Found {
            body: Lookup {
                results: Vec::new(),
                permissions,
                details: None,
            },
            matches: Vec::new(),
        });
    }
    if query.len() > 256 {
        return Err(OperationError::Validation {
            details: json!({"errors":{"q":["is too long"]}}),
        });
    }
    let slug = household::Entity::find_by_id(tenant.scope().household_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?
        .slug;
    let form = normalized_form(search.form.as_deref());
    let strength = search
        .strength
        .as_deref()
        .and_then(matching::normalize_strength);
    let locations: HashMap<i64, String> = location::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|row| (row.id, row.name))
        .collect();
    let visible = access::medication_scope(tenant)
        .order_by_asc(medication::Column::Name)
        .order_by_asc(medication::Column::Id)
        .all(tenant.transaction())
        .await?;
    let candidates = barcode_candidates(query);
    let barcode = candidates.first().cloned();
    let known_stock = visible.iter().any(|row| {
        row.barcode
            .as_ref()
            .is_some_and(|gtin| candidates.contains(gtin))
    });
    let provider =
        provider.filter(|_| !known_stock && query.bytes().all(|byte| byte.is_ascii_digit()));
    let accepts = |entry: &Product| {
        form.as_deref()
            .is_none_or(|value| form_matches(entry, value))
            && strength
                .as_deref()
                .is_none_or(|value| matching::strength_matches(entry, value))
    };
    let mut entries = catalogue(tenant, query, barcode.as_deref(), provider, &accepts).await?;
    let enrich = enrichment::Context::load(tenant, &entries, &visible).await?;
    let source = entries.first().map(|(_, source)| source.clone());
    let mut matches = Vec::new();
    let mut seen = HashSet::new();
    let mut results = Vec::new();
    for (mut entry, _) in entries.drain(..) {
        if !accepts(&entry) {
            continue;
        }
        if barcode.is_none() {
            entry.match_reason = None;
            entry.match_reason_label = None;
        }
        let records = matches_from(&visible, &StockMatchCandidate::from_product(&entry));
        let choices: Vec<StockMatch> = records
            .iter()
            .map(|row| stock_value(row, &locations, &slug))
            .collect();
        if let [choice] = choices.as_slice() {
            entry.existing_medication = Some(ExistingMedication {
                medication: choice.clone(),
                refill_path: format!("{}?refill=true", choice.path),
            });
        }
        enrich.apply(&mut entry, &visible, &locations, &slug);
        for choice in choices {
            if seen.insert(choice.id) {
                matches.push(choice);
            }
        }
        results.push(entry);
    }
    if results.is_empty() && barcode.is_some() {
        for row in matches_from(
            &visible,
            &StockMatchCandidate {
                barcode: barcode.as_deref().unwrap_or_default().to_owned(),
                code: String::new(),
                name: String::new(),
                display: String::new(),
                package_unit: None,
            },
        ) {
            matches.push(stock_value(&row, &locations, &slug));
        }
    }
    let resolved_query = if barcode.is_some() {
        results
            .first()
            .map(|entry| entry.display.as_str())
            .unwrap_or(query)
    } else {
        query
    };
    let details = Details {
        query: resolved_query.to_owned(),
        barcode_resolution: source
            .filter(|_| barcode.is_some())
            .map(|source| BarcodeResolution {
                status: "resolved",
                source,
            }),
        barcode,
        form,
        strength,
        review_guidance: ReviewGuidance {
            status: if enrich.available() {
                "available"
            } else {
                "unavailable"
            },
        },
    };
    let body = Lookup {
        results,
        permissions,
        details: Some(details),
    };
    Ok(Found { body, matches })
}

fn form_matches(entry: &Product, requested: &str) -> bool {
    let terms: &[&[&str]] = &[
        &["tablet", "tablets", "caplet", "caplets"],
        &["capsule", "capsules"],
        &[
            "liquid",
            "solution",
            "suspension",
            "syrup",
            "oral_solution",
            "oral_suspension",
            "ml",
        ],
        &["cream", "ointment", "gel"],
        &[
            "inhaler",
            "inhalation",
            "inhalation_powder",
            "nebuliser",
            "nebulizer",
        ],
        &["injection", "injectable", "syringe", "ampoule", "vial"],
        &["patch", "patches"],
        &["drops", "eye_drops", "ear_drops"],
        &["spray", "sprays"],
        &["powder", "powders", "sachet", "sachets", "granules"],
    ];
    let normalize = |value: &str| {
        value
            .to_ascii_lowercase()
            .split(|character: char| !character.is_ascii_alphanumeric())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("_")
    };
    let requested = normalize(requested);
    let Some(terms) = terms
        .iter()
        .find(|terms| terms.contains(&requested.as_str()))
    else {
        return true;
    };
    let searchable = normalize(
        &[
            entry.package_unit.as_deref().unwrap_or(""),
            &entry.display,
            entry.name.as_deref().unwrap_or(""),
            entry.description.as_deref().unwrap_or(""),
        ]
        .join(" "),
    );
    terms.iter().any(|term| searchable.contains(term))
}

fn normalized_form(value: Option<&str>) -> Option<String> {
    let requested = value?
        .to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    let groups: &[(&str, &[&str])] = &[
        ("tablet", &["tablet", "tablets", "caplet", "caplets"]),
        ("capsule", &["capsule", "capsules"]),
        (
            "liquid",
            &[
                "liquid",
                "solution",
                "suspension",
                "syrup",
                "oral_solution",
                "oral_suspension",
                "ml",
            ],
        ),
        ("cream", &["cream", "ointment", "gel"]),
        (
            "inhaler",
            &[
                "inhaler",
                "inhalation",
                "inhalation_powder",
                "nebuliser",
                "nebulizer",
            ],
        ),
        (
            "injection",
            &["injection", "injectable", "syringe", "ampoule", "vial"],
        ),
        ("patch", &["patch", "patches"]),
        ("drops", &["drops", "eye_drops", "ear_drops"]),
        ("spray", &["spray", "sprays"]),
        (
            "powder",
            &["powder", "powders", "sachet", "sachets", "granules"],
        ),
    ];
    groups
        .iter()
        .find(|(_, aliases)| aliases.contains(&requested.as_str()))
        .map(|(name, _)| (*name).to_owned())
}

fn stock_value(
    row: &medication::Model,
    locations: &HashMap<i64, String>,
    slug: &str,
) -> StockMatch {
    StockMatch {
        id: row.id,
        name: row
            .friendly_name
            .as_deref()
            .filter(|value| !value.is_empty())
            .or(row.name.as_deref())
            .unwrap_or("Medication")
            .to_owned(),
        location: locations
            .get(&row.location_id)
            .map(String::as_str)
            .unwrap_or("Location")
            .to_owned(),
        path: format!("/households/{slug}/medications/{}", row.id),
        current_supply: row
            .current_supply
            .map(|quantity| quantity.normalize().to_string())
            .unwrap_or_default(),
    }
}
