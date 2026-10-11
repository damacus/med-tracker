use crate::models::entities::medication;
use regex::Regex;
use sea_orm::prelude::Decimal;
use serde_json::Value;
use std::{str::FromStr, sync::LazyLock};

static STRENGTH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(\d+(?:\.\d+)?)\s*(mg|mcg|micrograms?|g|iu)\b(?:\s*/\s*(\d+(?:\.\d+)?)\s*(ml|l)\b)?").expect("valid strength expression")
});
static FILTER_COMPONENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^([0-9]+(?:\.[0-9]+)?)\s*(micrograms?|ug|μg|mcg|milligrams?|mg|grams?|g|millilitres?|milliliters?|ml|litres?|liters?|l)$").expect("valid strength component")
});
static FILTER_STRENGTH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b\d[\d,.]*\s*(?:micrograms?|ug|μg|mcg|milligrams?|mg|grams?|g|millilitres?|milliliters?|ml|litres?|liters?|l)(?:\s*/\s*\d[\d,.]*\s*(?:micrograms?|ug|μg|mcg|milligrams?|mg|grams?|g|millilitres?|milliliters?|ml|litres?|liters?|l))?").expect("valid strength filter")
});
static PACK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b\d+(?:\.\d+)?\s*(?:tablets?|capsules?|caplets?|sachets?|sprays?|drops?|pads?|ml|l)\b").expect("valid pack expression")
});
static MAKER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\([^)]*\)").expect("valid manufacturer expression"));
const FORMS: &[(&[&str], &str)] = &[
    (&["tablet", "tablets", "caplet", "caplets"], "tablet"),
    (&["capsule", "capsules"], "capsule"),
    (&["sachet", "sachets"], "sachet"),
    (&["spray", "sprays"], "spray"),
    (&["drop", "drops"], "drop"),
    (&["pad", "pads"], "pad"),
    (&["solution", "suspension", "liquid"], "liquid"),
];

fn words(value: &str) -> Vec<String> {
    value
        .to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

fn name_key(value: &str) -> String {
    let without_maker = MAKER.replace_all(value, " ");
    let without_strength = STRENGTH.replace_all(&without_maker, " ");
    let without_pack = PACK.replace_all(&without_strength, " ");
    words(&without_pack)
        .into_iter()
        .filter(|word| {
            !["oral", "powder"].contains(&word.as_str())
                && !FORMS
                    .iter()
                    .any(|(terms, _)| terms.contains(&word.as_str()))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn unit_key(value: &str) -> String {
    let value = value.to_ascii_lowercase();
    if value.starts_with("microgram") {
        "mcg".into()
    } else {
        value
    }
}

fn strength(name: &str, amount: Option<f64>, unit: Option<&str>) -> Option<String> {
    if let Some(found) = STRENGTH.captures(name) {
        let amount = Decimal::from_str(found.get(1)?.as_str()).ok()?.normalize();
        let unit = unit_key(found.get(2)?.as_str());
        return Some(match found.get(3) {
            Some(denominator) => format!(
                "{amount} {unit}/{} {}",
                Decimal::from_str(denominator.as_str()).ok()?.normalize(),
                found.get(4)?.as_str().to_ascii_lowercase()
            ),
            None => format!("{amount} {unit}"),
        });
    }
    let unit = unit_key(unit?);
    if !["mg", "mcg", "g", "iu"].contains(&unit.as_str()) {
        return None;
    }
    let amount = Decimal::from_str(&amount?.to_string()).ok()?.normalize();
    Some(format!("{amount} {unit}"))
}

pub(super) fn normalize_strength(value: &str) -> Option<String> {
    let normalized = value.to_lowercase().replace(',', "");
    let components = normalized.split('/').map(str::trim).collect::<Vec<_>>();
    if components.is_empty() || components.len() > 2 {
        return None;
    }
    components
        .into_iter()
        .map(|component| {
            let captures = FILTER_COMPONENT.captures(component)?;
            let mut amount = Decimal::from_str(captures.get(1)?.as_str()).ok()?;
            let unit = match captures.get(2)?.as_str() {
                "microgram" | "micrograms" | "ug" | "μg" | "mcg" => {
                    amount /= Decimal::from(1000);
                    "mg"
                }
                "milligram" | "milligrams" | "mg" => "mg",
                "gram" | "grams" | "g" => {
                    amount *= Decimal::from(1000);
                    "mg"
                }
                "millilitre" | "millilitres" | "milliliter" | "milliliters" | "ml" => "ml",
                "litre" | "litres" | "liter" | "liters" | "l" => {
                    amount *= Decimal::from(1000);
                    "ml"
                }
                _ => return None,
            };
            Some(format!("{}{unit}", amount.normalize()))
        })
        .collect::<Option<Vec<_>>>()
        .map(|parts| parts.join("/"))
}

pub(super) fn strength_matches(entry: &Value, requested: &str) -> bool {
    ["display", "name", "description", "package_size"]
        .iter()
        .filter_map(|field| entry[*field].as_str())
        .any(|field| {
            FILTER_STRENGTH
                .find_iter(field)
                .filter_map(|found| normalize_strength(found.as_str()))
                .any(|strength| strength == requested)
        })
}

fn form(name: &str, unit: Option<&str>) -> Option<&'static str> {
    if let Some(unit) = unit {
        let unit = unit.to_ascii_lowercase();
        if let Some((_, form)) = FORMS
            .iter()
            .find(|(terms, _)| terms.contains(&unit.as_str()))
        {
            return Some(form);
        }
    }
    let words = words(name);
    FORMS
        .iter()
        .find(|(terms, _)| words.iter().any(|word| terms.contains(&word.as_str())))
        .map(|(_, form)| *form)
}

pub(super) fn compatible(name: &str, unit: Option<&str>, medication: &medication::Model) -> bool {
    let key = name_key(name);
    let existing = medication.name.as_deref().unwrap_or("");
    if key.is_empty()
        || key != name_key(existing)
        || strength(name, None, unit)
            != strength(
                existing,
                medication.dose_amount,
                medication.dose_unit.as_deref(),
            )
    {
        return false;
    }
    match (
        form(name, unit),
        form(existing, medication.dose_unit.as_deref()),
    ) {
        (Some(candidate), Some(existing)) => candidate == existing,
        _ => true,
    }
}
