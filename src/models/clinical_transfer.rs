use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    fs::File,
    io::{Read, Write},
    path::Path,
};

use sea_orm::prelude::Decimal;
use serde::{Deserialize, Serialize, de};
use serde_json::{Map, Value, value::RawValue};
use sha2::{Digest, Sha256};

use super::care::portable_crypto;

const PAYLOAD_LIMIT: usize = 16 * 1024 * 1024;
const MANIFEST_LIMIT: usize = 1024 * 1024;
const SECRET_LIMIT: usize = 4096;
const V2: &str = "medtracker.portable.v2";
const V3: &str = "medtracker.portable.v3";
const COLLECTIONS: [&str; 11] = [
    "people",
    "locations",
    "medications",
    "dosage_options",
    "schedules",
    "person_medications",
    "medication_takes",
    "notification_preferences",
    "health_events",
    "dose_occurrences",
    "medication_pause_periods",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparationError {
    InputUnavailable,
    TooLarge,
    InvalidDocument,
    UnsupportedNumber,
    UnsupportedFormat,
    UnsupportedFields,
    UnsupportedAttachments,
    ObsoleteRelationships,
    InvalidGraph,
    ManifestMismatch,
    InvalidPassphrase,
    EncryptionUnavailable,
    OutputUnavailable,
}

impl fmt::Display for PreparationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InputUnavailable => "Cannot read preparation input.",
            Self::TooLarge => "Preparation input exceeds its documented size limit.",
            Self::InvalidDocument => "Invalid JSON: use unique keys and decimal strings outside schedule configuration.",
            Self::UnsupportedNumber => "Schedule configuration numbers must be fixed-point decimals with a 96-bit coefficient, at most 28 decimal places and no JSON round-trip precision loss.",
            Self::UnsupportedFormat => "Expected a household v2 payload and household-export v1 manifest.",
            Self::UnsupportedFields => "Unsupported or missing collection/field; review the source export.",
            Self::UnsupportedAttachments => "Attachments are unsupported; no output was prepared.",
            Self::ObsoleteRelationships => "Non-empty person-location relationships are unsupported; review the source export.",
            Self::InvalidGraph => "Invalid or unresolved portable identity/reference; review the complete source export.",
            Self::ManifestMismatch => "Manifest record counts do not match the payload.",
            Self::InvalidPassphrase => "Provide one non-empty passphrase line on stdin (at most 4096 bytes).",
            Self::EncryptionUnavailable => "Cannot encrypt the preparation; no output was prepared.",
            Self::OutputUnavailable => "Cannot publish output; choose a new path in an existing private directory.",
        })
    }
}

#[derive(Serialize)]
pub struct PreparedTransfer {
    format: &'static str,
    pub bundle: Value,
    conversion_receipt: ConversionReceipt,
}

#[derive(Serialize)]
struct ConversionReceipt {
    format: &'static str,
    source_format: &'static str,
    target_format: &'static str,
    payload_sha256: String,
    manifest_sha256: String,
    target_sha256: String,
    record_counts: BTreeMap<String, usize>,
    empty_person_location_fields_removed: usize,
}

pub fn prepare(
    payload_bytes: &[u8],
    manifest_bytes: &[u8],
    passphrase: &str,
) -> Result<PreparedTransfer, PreparationError> {
    if passphrase.is_empty()
        || passphrase.len() > SECRET_LIMIT
        || passphrase.contains('\r')
        || passphrase.contains('\n')
    {
        return Err(PreparationError::InvalidPassphrase);
    }
    let mut payload = parse(payload_bytes, PAYLOAD_LIMIT, JsonScope::Payload)?;
    let manifest = parse(manifest_bytes, MANIFEST_LIMIT, JsonScope::Other)?;
    exact_keys(
        &payload,
        "format scope exported_at source_instance_id records",
    )?;
    exact_keys(
        &manifest,
        "format export_id portable_format record_counts attachments",
    )?;
    if payload["format"] != V2
        || payload["scope"] != "household"
        || manifest["format"] != "medtracker.household-export.v1"
        || manifest["portable_format"] != V2
        || manifest["export_id"].as_u64().is_none_or(|id| id == 0)
    {
        return Err(PreparationError::UnsupportedFormat);
    }
    let exported_at = text(&payload["exported_at"])?;
    chrono::DateTime::parse_from_rfc3339(exported_at)
        .map_err(|_| PreparationError::InvalidGraph)?;
    text(&payload["source_instance_id"])?;
    if manifest["attachments"]
        .as_array()
        .is_none_or(|items| !items.is_empty())
    {
        return Err(PreparationError::UnsupportedAttachments);
    }
    let records = payload["records"]
        .as_object()
        .ok_or(PreparationError::InvalidGraph)?;
    let counts = manifest["record_counts"]
        .as_object()
        .ok_or(PreparationError::ManifestMismatch)?;
    if records.len() != COLLECTIONS.len() || counts.len() != COLLECTIONS.len() {
        return Err(PreparationError::UnsupportedFields);
    }
    let mut identities = BTreeMap::new();
    let mut record_counts = BTreeMap::new();
    for kind in COLLECTIONS {
        let rows = records
            .get(kind)
            .and_then(Value::as_array)
            .ok_or(PreparationError::UnsupportedFields)?;
        if counts.get(kind).and_then(Value::as_u64) != u64::try_from(rows.len()).ok() {
            return Err(PreparationError::ManifestMismatch);
        }
        let mut ids = BTreeSet::new();
        for row in rows {
            exact_keys(row, fields(kind))?;
            if !ids.insert(text(&row["portable_id"])?) {
                return Err(PreparationError::InvalidGraph);
            }
            text(&row["etag"])?;
            chrono::DateTime::parse_from_rfc3339(text(&row["updated_at"])?)
                .map_err(|_| PreparationError::InvalidGraph)?;
            if kind == "people"
                && row["location_portable_ids"]
                    .as_array()
                    .is_none_or(|ids| !ids.is_empty())
            {
                return Err(PreparationError::ObsoleteRelationships);
            }
        }
        identities.insert(kind, ids);
        record_counts.insert(kind.to_owned(), rows.len());
    }
    for kind in COLLECTIONS {
        for row in records[kind]
            .as_array()
            .ok_or(PreparationError::InvalidGraph)?
        {
            for (field, target, nullable) in references(kind) {
                check_reference(&identities, &row[*field], target, *nullable)?;
            }
            if matches!(
                kind,
                "medication_takes" | "dose_occurrences" | "medication_pause_periods"
            ) {
                let target = match row["source_type"].as_str() {
                    Some("schedule") => "schedules",
                    Some("person_medication") => "person_medications",
                    _ => return Err(PreparationError::InvalidGraph),
                };
                check_reference(&identities, &row["source_portable_id"], target, false)?;
            }
            if kind == "health_events" {
                for id in row["medication_portable_ids"]
                    .as_array()
                    .ok_or(PreparationError::InvalidGraph)?
                {
                    check_reference(&identities, id, "medications", false)?;
                }
            }
            if kind == "medication_pause_periods" {
                for field in [
                    "recorded_by_person_portable_id",
                    "resumed_by_person_portable_id",
                ] {
                    if !row[field].is_null() {
                        text(&row[field])?;
                    }
                }
            }
        }
    }
    let people = payload["records"]["people"]
        .as_array_mut()
        .ok_or(PreparationError::InvalidGraph)?;
    let removed = people.len();
    for person in people {
        person
            .as_object_mut()
            .ok_or(PreparationError::InvalidGraph)?
            .remove("location_portable_ids");
    }
    payload["format"] = Value::String(V3.to_owned());
    let target_bytes =
        serde_json::to_vec(&payload).map_err(|_| PreparationError::InvalidDocument)?;
    let bundle = portable_crypto::encrypt(&payload, passphrase).map_err(|error| match error {
        portable_crypto::PortableCryptoError::TooLarge => PreparationError::TooLarge,
        portable_crypto::PortableCryptoError::InvalidEnvelope if passphrase.is_empty() => {
            PreparationError::InvalidPassphrase
        }
        _ => PreparationError::EncryptionUnavailable,
    })?;
    Ok(PreparedTransfer {
        format: "medtracker.clinical-preparation.v1",
        bundle,
        conversion_receipt: ConversionReceipt {
            format: "medtracker.clinical-conversion-receipt.v1",
            source_format: V2,
            target_format: V3,
            payload_sha256: digest(payload_bytes),
            manifest_sha256: digest(manifest_bytes),
            target_sha256: digest(&target_bytes),
            record_counts,
            empty_person_location_fields_removed: removed,
        },
    })
}

pub fn prepare_files(
    payload: &Path,
    manifest: &Path,
    output: &Path,
    passphrase_input: impl Read,
) -> Result<(), PreparationError> {
    let mut secret = bounded(passphrase_input, SECRET_LIMIT + 2)
        .map_err(|_| PreparationError::InvalidPassphrase)?;
    if secret.last() == Some(&b'\n') {
        secret.pop();
        if secret.last() == Some(&b'\r') {
            secret.pop();
        }
    }
    let passphrase =
        std::str::from_utf8(&secret).map_err(|_| PreparationError::InvalidPassphrase)?;
    let payload_bytes = bounded(
        File::open(payload).map_err(|_| PreparationError::InputUnavailable)?,
        PAYLOAD_LIMIT,
    )?;
    let manifest_bytes = bounded(
        File::open(manifest).map_err(|_| PreparationError::InputUnavailable)?,
        MANIFEST_LIMIT,
    )?;
    let prepared = prepare(&payload_bytes, &manifest_bytes, passphrase)?;
    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut file =
        tempfile::NamedTempFile::new_in(parent).map_err(|_| PreparationError::OutputUnavailable)?;
    serde_json::to_writer_pretty(&mut file, &prepared)
        .map_err(|_| PreparationError::OutputUnavailable)?;
    file.write_all(b"\n")
        .and_then(|()| file.as_file().sync_all())
        .map_err(|_| PreparationError::OutputUnavailable)?;
    file.persist_noclobber(output)
        .map_err(|_| PreparationError::OutputUnavailable)?;
    Ok(())
}

fn bounded(reader: impl Read, limit: usize) -> Result<Vec<u8>, PreparationError> {
    let mut bytes = Vec::new();
    reader
        .take(u64::try_from(limit).map_err(|_| PreparationError::TooLarge)? + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| PreparationError::InputUnavailable)?;
    if bytes.len() > limit {
        return Err(PreparationError::TooLarge);
    }
    Ok(bytes)
}

fn digest(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn text(value: &Value) -> Result<&str, PreparationError> {
    value
        .as_str()
        .filter(|value| !value.is_empty() && value.trim() == *value)
        .ok_or(PreparationError::InvalidGraph)
}

fn exact_keys(value: &Value, fields: &str) -> Result<(), PreparationError> {
    let object = value
        .as_object()
        .ok_or(PreparationError::UnsupportedFields)?;
    if object.len() != fields.split_ascii_whitespace().count()
        || fields
            .split_ascii_whitespace()
            .any(|field| !object.contains_key(field))
    {
        return Err(PreparationError::UnsupportedFields);
    }
    Ok(())
}

fn check_reference(
    ids: &BTreeMap<&str, BTreeSet<&str>>,
    value: &Value,
    target: &str,
    nullable: bool,
) -> Result<(), PreparationError> {
    if nullable && value.is_null() {
        return Ok(());
    }
    if !ids
        .get(target)
        .is_some_and(|ids| text(value).is_ok_and(|id| ids.contains(id)))
    {
        return Err(PreparationError::InvalidGraph);
    }
    Ok(())
}

fn references(kind: &str) -> &'static [(&'static str, &'static str, bool)] {
    match kind {
        "people" => &[(
            "notification_preference_portable_id",
            "notification_preferences",
            true,
        )],
        "medications" => &[("location_portable_id", "locations", false)],
        "dosage_options" => &[("medication_portable_id", "medications", false)],
        "schedules" | "person_medications" => &[
            ("person_portable_id", "people", false),
            ("medication_portable_id", "medications", false),
            ("source_dosage_option_portable_id", "dosage_options", true),
        ],
        "medication_takes" => &[
            ("taken_from_medication_portable_id", "medications", true),
            ("taken_from_location_portable_id", "locations", true),
        ],
        "notification_preferences" | "health_events" => &[("person_portable_id", "people", false)],
        "dose_occurrences" => &[("medication_take_portable_id", "medication_takes", true)],
        _ => &[],
    }
}

fn fields(kind: &str) -> &'static str {
    match kind {
        "people" => {
            "portable_id updated_at etag name email date_of_birth person_type has_capacity location_portable_ids notification_preference_portable_id"
        }
        "locations" => "portable_id updated_at etag name description",
        "medications" => {
            "portable_id updated_at etag location_portable_id name friendly_name category description dose_amount dose_unit default_schedule_type current_supply reorder_threshold barcode dmd_code dmd_system dmd_concept_class"
        }
        "dosage_options" => {
            "portable_id updated_at etag medication_portable_id amount unit frequency description default_for_adults default_for_children default_max_daily_doses default_min_hours_between_doses default_dose_cycle current_supply reorder_threshold"
        }
        "schedules" => {
            "portable_id updated_at etag source_dosage_option_portable_id retired_at person_portable_id medication_portable_id dose_amount dose_unit frequency dose_cycle max_daily_doses min_hours_between_doses schedule_type schedule_config start_date end_date active notes"
        }
        "person_medications" => {
            "portable_id updated_at etag source_dosage_option_portable_id retired_at person_portable_id medication_portable_id dose_amount dose_unit dose_cycle max_daily_doses min_hours_between_doses administration_kind active notes position"
        }
        "medication_takes" => {
            "portable_id updated_at etag client_uuid source_type source_portable_id taken_at dose_amount dose_unit taken_from_medication_portable_id taken_from_location_portable_id"
        }
        "notification_preferences" => {
            "portable_id updated_at etag person_portable_id enabled dose_due_enabled missed_dose_enabled low_stock_enabled private_text_enabled morning_time afternoon_time evening_time night_time"
        }
        "health_events" => {
            "portable_id updated_at etag person_portable_id event_kind severity title notes started_on ended_on medication_portable_ids"
        }
        "dose_occurrences" => {
            "portable_id updated_at etag source_type source_portable_id window_starts_on window_ends_on position scheduled_at outcome reason note resolved_at medication_take_portable_id"
        }
        "medication_pause_periods" => {
            "portable_id updated_at etag source_type source_portable_id reason note started_at ended_at created_at legacy_context imported_context recorded_by_person_portable_id resumed_by_person_portable_id"
        }
        _ => "",
    }
}

#[derive(Clone, Copy)]
enum JsonScope {
    Payload,
    Records,
    Schedules,
    Schedule,
    Configuration,
    Other,
}

impl JsonScope {
    fn member(self, key: &str) -> Self {
        match (self, key) {
            (Self::Payload, "records") => Self::Records,
            (Self::Records, "schedules") => Self::Schedules,
            (Self::Schedule, "schedule_config") | (Self::Configuration, _) => Self::Configuration,
            _ => Self::Other,
        }
    }

    fn element(self) -> Self {
        match self {
            Self::Schedules => Self::Schedule,
            Self::Configuration => Self::Configuration,
            _ => Self::Other,
        }
    }
}

struct UniqueObject<'a>(Vec<(String, &'a RawValue)>);

impl<'de> Deserialize<'de> for UniqueObject<'de> {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> de::Visitor<'de> for Visitor {
            type Value = UniqueObject<'de>;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON object with unique, non-reserved keys")
            }
            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut keys = BTreeSet::new();
                let mut values = Vec::new();
                while let Some(key) = map.next_key::<String>()? {
                    if matches!(
                        key.as_str(),
                        "$serde_json::private::Number" | "$serde_json::private::RawValue"
                    ) || !keys.insert(key.clone())
                    {
                        return Err(de::Error::custom("duplicate or reserved JSON key"));
                    }
                    values.push((key, map.next_value()?));
                }
                Ok(UniqueObject(values))
            }
        }
        deserializer.deserialize_map(Visitor)
    }
}

fn parse(bytes: &[u8], limit: usize, scope: JsonScope) -> Result<Value, PreparationError> {
    if bytes.len() > limit {
        return Err(PreparationError::TooLarge);
    }
    let raw: &RawValue =
        serde_json::from_slice(bytes).map_err(|_| PreparationError::InvalidDocument)?;
    parse_unique(raw, scope, 0)
}

fn parse_unique(raw: &RawValue, scope: JsonScope, depth: usize) -> Result<Value, PreparationError> {
    if depth >= 128 {
        return Err(PreparationError::InvalidDocument);
    }
    let token = raw.get();
    match token.as_bytes().first() {
        Some(b'{') => {
            let UniqueObject(members) =
                serde_json::from_str(token).map_err(|_| PreparationError::InvalidDocument)?;
            let mut values = Map::new();
            for (key, value) in members {
                let value = parse_unique(value, scope.member(&key), depth + 1)?;
                values.insert(key, value);
            }
            Ok(Value::Object(values))
        }
        Some(b'[') => {
            let elements: Vec<&RawValue> =
                serde_json::from_str(token).map_err(|_| PreparationError::InvalidDocument)?;
            elements
                .iter()
                .map(|value| parse_unique(value, scope.element(), depth + 1))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array)
        }
        Some(b'-' | b'0'..=b'9') if matches!(scope, JsonScope::Configuration) => {
            if token.contains(['e', 'E'])
                || token
                    .split_once('.')
                    .is_some_and(|(_, digits)| digits.len() > 28)
            {
                return Err(PreparationError::UnsupportedNumber);
            }
            let original =
                Decimal::from_str_exact(token).map_err(|_| PreparationError::UnsupportedNumber)?;
            let value: Value =
                serde_json::from_str(token).map_err(|_| PreparationError::UnsupportedNumber)?;
            let Value::Number(number) = &value else {
                return Err(PreparationError::UnsupportedNumber);
            };
            let serialized = Decimal::from_str_exact(&number.to_string())
                .map_err(|_| PreparationError::UnsupportedNumber)?;
            if original != serialized {
                return Err(PreparationError::UnsupportedNumber);
            }
            Ok(value)
        }
        _ => {
            let value: Value =
                serde_json::from_str(token).map_err(|_| PreparationError::InvalidDocument)?;
            if value.is_number() && value.as_i64().is_none() && value.as_u64().is_none() {
                return Err(PreparationError::InvalidDocument);
            }
            Ok(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const PAYLOAD: &[u8] =
        include_bytes!("../../tests/fixtures/portable_rails/prepared-history-v2.json");
    const MANIFEST: &[u8] =
        include_bytes!("../../tests/fixtures/portable_rails/prepared-history-v2.manifest.json");

    #[test]
    fn preparation_preserves_actual_rails_taper_json_numbers() {
        let bytes = include_bytes!("../../tests/fixtures/portable_rails/prepared-taper-v2.json");
        let manifest =
            include_bytes!("../../tests/fixtures/portable_rails/prepared-taper-v2.manifest.json");
        let original: Value = serde_json::from_slice(bytes).unwrap();
        let steps = &original["records"]["schedules"][0]["schedule_config"]["taper_steps"];
        assert_eq!(steps[0]["amount"].as_f64(), Some(0.5));
        assert_eq!(steps[1]["amount"].as_f64(), Some(1.0));
        let prepared = prepare(bytes, manifest, "fixture secret").unwrap();
        let restored = portable_crypto::decrypt(&prepared.bundle, "fixture secret").unwrap();
        assert_eq!(
            restored["records"]["schedules"][0]["schedule_config"],
            original["records"]["schedules"][0]["schedule_config"]
        );
        assert!(
            restored["records"]["schedules"][0]["schedule_config"]["taper_steps"][0]["amount"]
                .is_number()
        );
    }

    #[test]
    fn preparation_config_numbers_remain_exact_and_scoped() {
        let source = include_str!("../../tests/fixtures/portable_rails/prepared-taper-v2.json");
        let manifest =
            include_bytes!("../../tests/fixtures/portable_rails/prepared-taper-v2.manifest.json");
        for token in ["0.1", "1.00", "-0.125", "9007199254740993"] {
            let input = source.replacen("\"amount\": 0.5", &format!("\"amount\": {token}"), 1);
            assert_ne!(input, source);
            let prepared = prepare(input.as_bytes(), manifest, "fixture secret").unwrap();
            let restored = portable_crypto::decrypt(&prepared.bundle, "fixture secret").unwrap();
            let amount =
                &restored["records"]["schedules"][0]["schedule_config"]["taper_steps"][0]["amount"];
            assert!(amount.is_number());
            assert_eq!(
                Decimal::from_str_exact(&amount.to_string()).unwrap(),
                Decimal::from_str_exact(token).unwrap()
            );
        }
        for token in [
            "0.12345678901234567890123456789".to_owned(),
            "0.123456789123456789".to_owned(),
            "0.00000000000000000000000000000".to_owned(),
            "79228162514264337593543950336".to_owned(),
            "0.5e0".to_owned(),
            "1E0".to_owned(),
            "1e-400".to_owned(),
            "1e400".to_owned(),
            "9".repeat(1024),
        ] {
            let input = source.replacen("\"amount\": 0.5", &format!("\"amount\": {token}"), 1);
            assert_ne!(input, source);
            assert_eq!(
                prepare(input.as_bytes(), manifest, "fixture secret").err(),
                Some(PreparationError::UnsupportedNumber),
                "must reject unsupported number {token}"
            );
        }
        let mut clinical: Value = serde_json::from_str(source).unwrap();
        clinical["records"]["medications"][0]["dose_amount"] = json!(0.5);
        assert_eq!(
            prepare(
                &serde_json::to_vec(&clinical).unwrap(),
                manifest,
                "fixture secret"
            )
            .err(),
            Some(PreparationError::InvalidDocument)
        );
        for replacement in [
            "\"amount\": 0.5, \"amount\": 1.0".to_owned(),
            r#""amount": {"nested": {"key": 1, "\u006bey": 2}}"#.to_owned(),
            "\"amount\": {\"$serde_json::private::Number\": \"0.5\"}".to_owned(),
            "\"amount\": {\"$serde_json::private::RawValue\": \"0.5\"}".to_owned(),
            format!("\"amount\": {}0{}", "[".repeat(150), "]".repeat(150)),
        ] {
            let malformed = source.replacen("\"amount\": 0.5", &replacement, 1);
            assert_ne!(malformed, source);
            assert_eq!(
                prepare(malformed.as_bytes(), manifest, "fixture secret").err(),
                Some(PreparationError::InvalidDocument)
            );
        }
        for bytes in [
            br#"{"records":{"schedules":{"schedule_config":{"amount":0.5}}}}"#.as_slice(),
            br#"{"records":{"medications":[{"schedule_config":{"amount":0.5}}]}}"#,
            br#"{"schedule_config":{"amount":0.5}}"#,
        ] {
            assert_eq!(
                parse(bytes, PAYLOAD_LIMIT, JsonScope::Payload),
                Err(PreparationError::InvalidDocument)
            );
        }
        assert_eq!(
            parse(source.as_bytes(), MANIFEST_LIMIT, JsonScope::Other),
            Err(PreparationError::InvalidDocument)
        );
    }

    #[test]
    fn preparation_accepts_near_limit_nested_configuration() {
        let mut payload: Value = serde_json::from_slice(PAYLOAD).unwrap();
        let mut configuration = json!("");
        for _ in 0..100 {
            configuration = json!({"nested": configuration});
        }
        payload["records"]["schedules"][0]["schedule_config"] = configuration;
        let padding_len = PAYLOAD_LIMIT - 16 * 1024 - serde_json::to_vec(&payload).unwrap().len();
        let mut leaf = &mut payload["records"]["schedules"][0]["schedule_config"];
        for _ in 0..100 {
            leaf = &mut leaf["nested"];
        }
        *leaf = json!("x".repeat(padding_len));
        let bytes = serde_json::to_vec(&payload).unwrap();
        drop(payload);
        assert_eq!(bytes.len(), PAYLOAD_LIMIT - 16 * 1024);
        let decoded = parse(&bytes, PAYLOAD_LIMIT, JsonScope::Payload).unwrap();
        let mut leaf = &decoded["records"]["schedules"][0]["schedule_config"];
        for _ in 0..100 {
            leaf = &leaf["nested"];
        }
        assert_eq!(leaf.as_str().unwrap().len(), padding_len);
    }

    #[test]
    fn preparation_preserves_every_clinical_value_and_records_only_explicit_mapping() {
        let prepared = prepare(PAYLOAD, MANIFEST, "fixture secret").unwrap();
        let restored = portable_crypto::decrypt(&prepared.bundle, "fixture secret").unwrap();
        let mut expected: Value = serde_json::from_slice(PAYLOAD).unwrap();
        expected["format"] = json!(V3);
        for person in expected["records"]["people"].as_array_mut().unwrap() {
            person
                .as_object_mut()
                .unwrap()
                .remove("location_portable_ids");
        }
        assert_eq!(restored, expected);
        assert_eq!(restored["records"].as_object().unwrap().len(), 11);
        assert_ne!(
            restored["records"]["medications"][0]["location_portable_id"],
            restored["records"]["medication_takes"][0]["taken_from_location_portable_id"]
        );
        let receipt = &prepared.conversion_receipt;
        assert_eq!(receipt.payload_sha256, digest(PAYLOAD));
        assert_eq!(receipt.manifest_sha256, digest(MANIFEST));
        assert_eq!(receipt.target_sha256, prepared.bundle["checksum"]);
        assert_eq!(receipt.empty_person_location_fields_removed, 1);
        assert_eq!(receipt.record_counts.len(), 11);
        let serialized = serde_json::to_string(&prepared).unwrap();
        assert!(!serialized.contains("Synthetic Dependent"));
        assert!(!serialized.contains("fixture secret"));
        assert!(!serialized.contains("guardian_declaration"));
        assert!(!serialized.contains("RestoreHistory"));
    }

    #[test]
    fn preparation_rejects_unsupported_data_without_silently_dropping_it() {
        type Mutation = fn(&mut Value, &mut Value);
        let cases: &[(Mutation, PreparationError)] = &[
            (
                |p, _| p["format"] = json!(V3),
                PreparationError::UnsupportedFormat,
            ),
            (
                |p, _| p["scope"] = json!("single_person"),
                PreparationError::UnsupportedFormat,
            ),
            (
                |_, m| m["attachments"] = json!([{"path":"avatar.png"}]),
                PreparationError::UnsupportedAttachments,
            ),
            (
                |_, m| m["record_counts"]["people"] = json!(99),
                PreparationError::ManifestMismatch,
            ),
            (
                |p, _| {
                    p["records"]
                        .as_object_mut()
                        .unwrap()
                        .remove("health_events");
                },
                PreparationError::UnsupportedFields,
            ),
            (
                |p, _| p["records"]["accounts"] = json!([]),
                PreparationError::UnsupportedFields,
            ),
            (
                |p, _| p["records"]["people"][0]["account_id"] = json!(1),
                PreparationError::UnsupportedFields,
            ),
            (
                |p, _| {
                    p["records"]["people"][0]["location_portable_ids"] = json!(["historical-link"])
                },
                PreparationError::ObsoleteRelationships,
            ),
            (
                |p, _| {
                    p["records"]["medications"][0]["location_portable_id"] = json!("absent-label")
                },
                PreparationError::InvalidGraph,
            ),
            (
                |p, _| {
                    p["records"]["health_events"][0]["medication_portable_ids"] =
                        json!(["absent-medicine"])
                },
                PreparationError::InvalidGraph,
            ),
            (
                |p, _| p["records"]["medication_takes"][0]["source_type"] = json!("unknown"),
                PreparationError::InvalidGraph,
            ),
            (
                |p, _| {
                    p["records"]["medication_takes"][0]["source_portable_id"] =
                        json!("absent-source")
                },
                PreparationError::InvalidGraph,
            ),
            (
                |p, _| {
                    p["records"]["dose_occurrences"][0]["medication_take_portable_id"] =
                        json!("absent-take")
                },
                PreparationError::InvalidGraph,
            ),
            (
                |p, _| p["records"]["people"][0]["portable_id"] = json!(1),
                PreparationError::InvalidGraph,
            ),
            (
                |p, _| p["records"]["people"][0]["updated_at"] = json!("not-a-time"),
                PreparationError::InvalidGraph,
            ),
            (
                |p, m| {
                    let duplicate = p["records"]["people"][0].clone();
                    p["records"]["people"]
                        .as_array_mut()
                        .unwrap()
                        .push(duplicate);
                    m["record_counts"]["people"] = json!(2);
                },
                PreparationError::InvalidGraph,
            ),
        ];
        for (mutate, expected) in cases {
            let mut payload: Value = serde_json::from_slice(PAYLOAD).unwrap();
            let mut manifest: Value = serde_json::from_slice(MANIFEST).unwrap();
            mutate(&mut payload, &mut manifest);
            assert_eq!(
                prepare(
                    &serde_json::to_vec(&payload).unwrap(),
                    &serde_json::to_vec(&manifest).unwrap(),
                    "secret"
                )
                .err(),
                Some(*expected)
            );
        }
    }

    #[test]
    fn preparation_rejects_ambiguous_or_lossy_json_and_bounds_inputs() {
        for bytes in [
            br#"{"format":"one","format":"two"}"#.as_slice(),
            br#"{"nested":{"key":1,"key":2}}"#,
            br#"{"fraction":0.123456789123456789}"#,
            br#"{} {}"#,
        ] {
            assert_eq!(
                prepare(bytes, MANIFEST, "secret").err(),
                Some(PreparationError::InvalidDocument)
            );
        }
        assert_eq!(
            prepare(&vec![b' '; PAYLOAD_LIMIT + 1], MANIFEST, "secret").err(),
            Some(PreparationError::TooLarge)
        );
        assert_eq!(
            prepare(PAYLOAD, &vec![b' '; MANIFEST_LIMIT + 1], "secret").err(),
            Some(PreparationError::TooLarge)
        );
    }

    #[test]
    fn preparation_file_is_private_complete_and_never_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let payload = directory.path().join("portable input.json");
        let manifest = directory.path().join("manifest input.json");
        let output = directory.path().join("prepared output.json");
        std::fs::write(&payload, PAYLOAD).unwrap();
        std::fs::write(&manifest, MANIFEST).unwrap();
        prepare_files(
            &payload,
            &manifest,
            &output,
            b"  exact secret  \r\n".as_slice(),
        )
        .unwrap();
        let bytes = std::fs::read(&output).unwrap();
        let package: Value = serde_json::from_slice(&bytes).unwrap();
        let decrypted = portable_crypto::decrypt(&package["bundle"], "  exact secret  ").unwrap();
        assert_eq!(decrypted["format"], V3);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&output).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert_eq!(
            prepare_files(&payload, &manifest, &output, b"other secret".as_slice()),
            Err(PreparationError::OutputUnavailable)
        );
        assert_eq!(std::fs::read(&output).unwrap(), bytes);
        let invalid_output = directory.path().join("invalid.json");
        for secret in [
            Vec::new(),
            vec![b'x'; SECRET_LIMIT + 3],
            b"one\ntwo\n".to_vec(),
        ] {
            assert_eq!(
                prepare_files(&payload, &manifest, &invalid_output, secret.as_slice()),
                Err(PreparationError::InvalidPassphrase)
            );
            assert!(!invalid_output.exists());
        }
        std::fs::write(&manifest, b"{}").unwrap();
        assert!(prepare_files(&payload, &manifest, &invalid_output, b"secret".as_slice()).is_err());
        assert!(!invalid_output.exists());
    }
}
