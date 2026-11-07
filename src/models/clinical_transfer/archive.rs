use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read},
};

use chrono::{DateTime, Utc};
use sea_orm::prelude::Decimal;
use serde_json::Value;
use zip::ZipArchive;

use super::{
    COLLECTIONS, JsonScope, MANIFEST_LIMIT, PAYLOAD_LIMIT, PreparationError, digest, fields, parse,
    prepare_inner, references, text,
};
use crate::models::data_imports::contracts::{
    ArchivePreparation, ArchivePreparationError, FieldOutcome, Omission, PreparationReceipt,
    PreparedArchive, ReasonCode, RecordDetails, RecordKind, RelationshipKind, SourceField,
    SourceIdentity, SourceInventory, SourcePreparationDiagnostic, SourceRecord, SourceRelationship,
    SourceSystem,
};

// Includes the existing 16 MiB payload and 1 MiB manifest limits, leaving at
// least 47 MiB for attachment content and ZIP metadata. Both compressed input
// and total expanded bytes are bounded; the entry cap includes the two documents.
const ARCHIVE_LIMIT: usize = 64 * 1024 * 1024;
const ENTRY_LIMIT: usize = 1024;

pub fn prepare_archive(
    input: ArchivePreparation<'_>,
) -> Result<PreparedArchive, ArchivePreparationError> {
    use ArchivePreparationError as E;
    if input.archive.len() > ARCHIVE_LIMIT {
        return Err(E::TooLarge);
    }
    let mut archive = ZipArchive::new(Cursor::new(input.archive)).map_err(|_| E::InvalidArchive)?;
    if archive.has_duplicate_names() {
        return Err(E::DuplicateEntry);
    }
    if archive.len() > ENTRY_LIMIT {
        return Err(E::TooLarge);
    }
    let mut entries = BTreeMap::new();
    let mut total = 0usize;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|_| E::InvalidArchive)?;
        let name = entry.name().map_err(|_| E::InvalidArchive)?.into_owned();
        if entry.is_dir()
            || entry.is_symlink()
            || entry.encrypted()
            || entry.enclosed_name().is_none()
            || name.contains('\\')
            || name.starts_with('/')
            || name
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(E::UnsupportedEntry);
        }
        if entries.contains_key(&name) {
            return Err(E::DuplicateEntry);
        }
        let size = usize::try_from(entry.size()).map_err(|_| E::TooLarge)?;
        if (name == "portable.json" && size > PAYLOAD_LIMIT)
            || (name == "manifest.json" && size > MANIFEST_LIMIT)
        {
            return Err(E::TooLarge);
        }
        total = total.checked_add(size).ok_or(E::TooLarge)?;
        if total > ARCHIVE_LIMIT {
            return Err(E::TooLarge);
        }
        let mut bytes = Vec::new();
        entry
            .by_ref()
            .take(u64::try_from(size).map_err(|_| E::TooLarge)? + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| E::InvalidArchive)?;
        if bytes.len() != size {
            return Err(E::InvalidArchive);
        }
        entries.insert(name, bytes);
    }
    let payload_bytes = entries.remove("portable.json").ok_or(E::InvalidArchive)?;
    let manifest_bytes = entries.remove("manifest.json").ok_or(E::InvalidArchive)?;
    let payload =
        parse(&payload_bytes, PAYLOAD_LIMIT, JsonScope::Payload).map_err(E::Preparation)?;
    let manifest =
        parse(&manifest_bytes, MANIFEST_LIMIT, JsonScope::Other).map_err(E::Preparation)?;
    check_fields(&payload, &manifest)?;
    let instance = text(&payload["source_instance_id"])
        .map_err(E::Preparation)?
        .to_owned();
    let mut inventory = SourceInventory {
        records: Vec::new(),
        relationships: Vec::new(),
        omissions: Vec::new(),
    };
    let attachments = manifest["attachments"]
        .as_array()
        .ok_or(E::Preparation(PreparationError::UnsupportedAttachments))?;
    for item in attachments {
        super::exact_keys(
            item,
            "attachment_id record_type record_id byte_size checksum_sha256 archive_path",
        )
        .map_err(E::Preparation)?;
        let id = item["attachment_id"]
            .as_u64()
            .filter(|id| *id > 0)
            .ok_or(E::InvalidArchive)?;
        let path = text(&item["archive_path"]).map_err(E::Preparation)?;
        if path != format!("attachments/{id}.bin") {
            return Err(E::UnsupportedEntry);
        }
        let bytes = entries.remove(path).ok_or(E::InvalidArchive)?;
        if item["byte_size"].as_u64() != u64::try_from(bytes.len()).ok()
            || item["checksum_sha256"].as_str() != Some(digest(&bytes).as_str())
            || text(&item["record_type"]).is_err()
            || item["record_id"].as_u64().is_none_or(|id| id == 0)
        {
            return Err(E::InvalidArchive);
        }
        let identity = identity(
            input.source,
            &instance,
            RecordKind::Attachment,
            &id.to_string(),
        );
        inventory.records.push(SourceRecord {
            identity: identity.clone(),
            details: details(item, true, false)?,
        });
        inventory.omissions.push(Omission {
            item: identity,
            field: None,
            reason: ReasonCode::AttachmentUnsupported,
            acknowledgement_id: omission_id(&instance, "attachment", &id.to_string(), ""),
            acknowledged: false,
        });
    }
    if !entries.is_empty() {
        return Err(E::UnsupportedEntry);
    }
    let records = payload["records"]
        .as_object()
        .ok_or(E::Preparation(PreparationError::InvalidGraph))?;
    let index = index_records(records)?;
    validate_source(records)?;
    validate_associations(records, &index)?;
    for collection in COLLECTIONS {
        for row in records[collection]
            .as_array()
            .ok_or(E::Preparation(PreparationError::InvalidGraph))?
        {
            let id = text(&row["portable_id"]).map_err(E::Preparation)?;
            let subject = collection_identity(input.source, &instance, collection, id);
            inventory.records.push(SourceRecord {
                identity: subject.clone(),
                details: details(row, false, collection == "people")?,
            });
            for field in ["updated_at", "etag"] {
                inventory.omissions.push(Omission {
                    item: subject.clone(),
                    field: Some(field.to_owned()),
                    reason: ReasonCode::UnsupportedField,
                    acknowledgement_id: omission_id(
                        &instance,
                        "unsupported-field",
                        &format!("{collection}:{id}"),
                        field,
                    ),
                    acknowledged: false,
                });
            }
            if collection == "medication_pause_periods" && row["imported_context"] != true {
                inventory.omissions.push(Omission {
                    item: subject.clone(),
                    field: Some("imported_context".to_owned()),
                    reason: ReasonCode::UnsupportedField,
                    acknowledgement_id: omission_id(
                        &instance,
                        "unsupported-field",
                        &format!("{collection}:{id}"),
                        "imported_context",
                    ),
                    acknowledged: false,
                });
            }
            for (field, target, _) in references(collection) {
                if let Some(id) = row[*field].as_str() {
                    inventory.relationships.push(SourceRelationship {
                        kind: relationship(collection, field),
                        field: (*field).to_owned(),
                        reason: None,
                        source: subject.clone(),
                        target: collection_identity(input.source, &instance, target, id),
                    });
                }
            }
            if collection == "medication_pause_periods" {
                for field in [
                    "recorded_by_person_portable_id",
                    "resumed_by_person_portable_id",
                ] {
                    if let Some(id) = row[field].as_str() {
                        inventory.relationships.push(SourceRelationship {
                            kind: RelationshipKind::Other,
                            field: field.to_owned(),
                            reason: if index["people"].contains_key(id) {
                                None
                            } else {
                                Some(ReasonCode::HistoricalActorUnavailable)
                            },
                            source: subject.clone(),
                            target: identity(input.source, &instance, RecordKind::Person, id),
                        });
                    }
                }
            }
            if matches!(
                collection,
                "medication_takes" | "dose_occurrences" | "medication_pause_periods"
            ) {
                let target = if row["source_type"] == "schedule" {
                    RecordKind::Schedule
                } else {
                    RecordKind::Assignment
                };
                inventory.relationships.push(SourceRelationship {
                    kind: RelationshipKind::DoseSource,
                    field: "source_portable_id".to_owned(),
                    reason: None,
                    source: subject.clone(),
                    target: identity(
                        input.source,
                        &instance,
                        target,
                        text(&row["source_portable_id"]).map_err(E::Preparation)?,
                    ),
                });
            }
            if collection == "health_events" {
                for id in row["medication_portable_ids"]
                    .as_array()
                    .ok_or(E::Preparation(PreparationError::InvalidGraph))?
                {
                    inventory.relationships.push(SourceRelationship {
                        kind: RelationshipKind::HealthEventMedication,
                        field: "medication_portable_ids".to_owned(),
                        reason: None,
                        source: subject.clone(),
                        target: identity(
                            input.source,
                            &instance,
                            RecordKind::Medication,
                            text(id).map_err(E::Preparation)?,
                        ),
                    });
                }
            }
            if collection == "people" {
                let mut seen = BTreeSet::new();
                for value in row["location_portable_ids"]
                    .as_array()
                    .ok_or(E::Preparation(PreparationError::InvalidGraph))?
                {
                    let id = text(value).map_err(E::Preparation)?;
                    if !seen.insert(id) || !index["locations"].contains_key(id) {
                        return Err(E::Preparation(PreparationError::InvalidGraph));
                    }
                    inventory.relationships.push(SourceRelationship {
                        kind: RelationshipKind::PersonLocation,
                        field: "location_portable_ids".to_owned(),
                        reason: None,
                        source: subject.clone(),
                        target: identity(input.source, &instance, RecordKind::Location, id),
                    });
                    inventory.omissions.push(Omission {
                        item: subject.clone(),
                        field: Some(format!("location_portable_ids:{id}")),
                        reason: ReasonCode::ObsoletePersonLocation,
                        acknowledgement_id: omission_id(
                            &instance,
                            "person-location",
                            &subject.id,
                            id,
                        ),
                        acknowledged: false,
                    });
                }
            }
        }
    }
    let transfer = prepare_inner(&payload_bytes, &manifest_bytes, input.passphrase, true)
        .map_err(E::Preparation)?;
    let receipt = PreparationReceipt {
        payload_sha256: transfer.conversion_receipt.payload_sha256.clone(),
        manifest_sha256: transfer.conversion_receipt.manifest_sha256.clone(),
        target_sha256: transfer.conversion_receipt.target_sha256.clone(),
    };
    Ok(PreparedArchive {
        transfer,
        archive_sha256: digest(input.archive),
        receipt,
        inventory,
    })
}

fn identity(source: SourceSystem, instance: &str, kind: RecordKind, id: &str) -> SourceIdentity {
    SourceIdentity {
        source,
        instance: instance.to_owned(),
        kind,
        id: id.to_owned(),
    }
}

fn collection_identity(
    source: SourceSystem,
    instance: &str,
    collection: &str,
    id: &str,
) -> SourceIdentity {
    identity(source, instance, kind(collection), id)
}

fn omission_id(instance: &str, category: &str, item: &str, target: &str) -> String {
    let mut bytes = Vec::new();
    for part in ["medtracker05", instance, category, item, target] {
        bytes.extend_from_slice(&(part.len() as u64).to_be_bytes());
        bytes.extend_from_slice(part.as_bytes());
    }
    digest(&bytes)
}

type SourceIndex<'a> = BTreeMap<&'static str, BTreeMap<&'a str, &'a Value>>;

fn decimal_exact(value: &Value) -> Option<Decimal> {
    match value {
        Value::String(value) => Decimal::from_str_exact(value).ok(),
        Value::Number(value) => Decimal::from_str_exact(&value.to_string()).ok(),
        _ => None,
    }
}

fn exactly_representable_as_f64(value: Decimal) -> bool {
    let value = value.normalize();
    let mut numerator = value.mantissa().unsigned_abs();
    for _ in 0..value.scale() {
        if !numerator.is_multiple_of(5) {
            return false;
        }
        numerator /= 5;
    }
    if numerator == 0 {
        return true;
    }
    numerator >>= numerator.trailing_zeros();
    numerator < (1_u128 << 53)
}

fn validate_source(
    records: &serde_json::Map<String, Value>,
) -> Result<(), ArchivePreparationError> {
    let mut source_errors = Vec::new();
    let mut decimal_errors = Vec::new();
    for collection in COLLECTIONS {
        let rows = records.get(collection).and_then(Value::as_array).ok_or(
            ArchivePreparationError::Preparation(PreparationError::InvalidGraph),
        )?;
        for (index, row) in rows.iter().enumerate() {
            if !row.is_object() {
                return Err(ArchivePreparationError::Preparation(
                    PreparationError::InvalidGraph,
                ));
            }
            source_errors.extend(crate::models::care::portable_imports::validate_source_row(
                collection, index, row,
            ));
            for field in [
                "amount",
                "dose_amount",
                "current_supply",
                "reorder_threshold",
                "default_min_hours_between_doses",
            ] {
                let Some(value) = row.get(field).filter(|value| !value.is_null()) else {
                    continue;
                };
                let valid = decimal_exact(value).is_some_and(|number| {
                    let normal = number.normalize();
                    match (collection, field) {
                        ("dosage_options", "amount") => true,
                        ("medications", "dose_amount") => exactly_representable_as_f64(number),
                        (_, "default_min_hours_between_doses") => {
                            normal.scale() <= 1 && normal.abs() < Decimal::from(1000)
                        }
                        _ => normal.scale() <= 2 && normal.abs() < Decimal::from(100_000_000),
                    }
                });
                if !valid {
                    decimal_errors.push(SourcePreparationDiagnostic {
                        path: format!("/records/{collection}/{index}/{field}"),
                    });
                }
            }
        }
    }
    if !decimal_errors.is_empty() {
        return Err(ArchivePreparationError::InvalidFields(decimal_errors));
    }
    if !source_errors.is_empty() {
        return Err(ArchivePreparationError::SourceValidation(source_errors));
    }
    Ok(())
}

fn index_records(
    records: &serde_json::Map<String, Value>,
) -> Result<SourceIndex<'_>, ArchivePreparationError> {
    let mut index = BTreeMap::new();
    for collection in COLLECTIONS {
        let rows = records.get(collection).and_then(Value::as_array).ok_or(
            ArchivePreparationError::Preparation(PreparationError::InvalidGraph),
        )?;
        let mut items = BTreeMap::new();
        for row in rows {
            let id = text(&row["portable_id"]).map_err(ArchivePreparationError::Preparation)?;
            if items.insert(id, row).is_some() {
                return Err(ArchivePreparationError::Preparation(
                    PreparationError::InvalidGraph,
                ));
            }
        }
        index.insert(collection, items);
    }
    Ok(index)
}

fn validate_associations(
    records: &serde_json::Map<String, Value>,
    index: &SourceIndex<'_>,
) -> Result<(), ArchivePreparationError> {
    let invalid = || ArchivePreparationError::Preparation(PreparationError::InvalidGraph);
    let find = |collection: &str, id: &Value| -> Option<&Value> {
        index.get(collection)?.get(id.as_str()?).copied()
    };
    for collection in ["schedules", "person_medications"] {
        for row in records[collection].as_array().ok_or_else(invalid)? {
            if !row["source_dosage_option_portable_id"].is_null() {
                let dosage = find("dosage_options", &row["source_dosage_option_portable_id"])
                    .ok_or_else(invalid)?;
                if dosage["medication_portable_id"] != row["medication_portable_id"] {
                    return Err(invalid());
                }
                let amount = decimal_exact(&dosage["amount"]);
                if amount.is_none()
                    || amount != decimal_exact(&row["dose_amount"])
                    || dosage["unit"] != row["dose_unit"]
                {
                    return Err(invalid());
                }
            }
        }
    }
    for row in records["people"].as_array().ok_or_else(invalid)? {
        if !row["notification_preference_portable_id"].is_null() {
            let preference = find(
                "notification_preferences",
                &row["notification_preference_portable_id"],
            )
            .ok_or_else(invalid)?;
            if preference["person_portable_id"] != row["portable_id"] {
                return Err(invalid());
            }
        }
    }
    for row in records["medication_takes"].as_array().ok_or_else(invalid)? {
        let collection = if row["source_type"] == "schedule" {
            "schedules"
        } else {
            "person_medications"
        };
        let source = find(collection, &row["source_portable_id"]).ok_or_else(invalid)?;
        if !row["taken_from_medication_portable_id"].is_null()
            && row["taken_from_medication_portable_id"] != source["medication_portable_id"]
        {
            return Err(invalid());
        }
    }
    for row in records["dose_occurrences"].as_array().ok_or_else(invalid)? {
        if !row["medication_take_portable_id"].is_null() {
            let take = find("medication_takes", &row["medication_take_portable_id"])
                .ok_or_else(invalid)?;
            if take["source_type"] != row["source_type"]
                || take["source_portable_id"] != row["source_portable_id"]
            {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

fn check_fields(payload: &Value, manifest: &Value) -> Result<(), ArchivePreparationError> {
    let mut diagnostics = Vec::new();
    fn check(
        diagnostics: &mut Vec<SourcePreparationDiagnostic>,
        value: &Value,
        allowed: &str,
        path: &str,
    ) {
        if let Some(object) = value.as_object() {
            for key in object.keys() {
                if !allowed.split_ascii_whitespace().any(|field| field == key) {
                    let escaped = key.replace('~', "~0").replace('/', "~1");
                    diagnostics.push(SourcePreparationDiagnostic {
                        path: format!("{path}/{escaped}"),
                    });
                }
            }
        }
    }
    check(
        &mut diagnostics,
        payload,
        "format scope exported_at source_instance_id records",
        "",
    );
    check(
        &mut diagnostics,
        manifest,
        "format export_id portable_format record_counts attachments",
        "/manifest",
    );
    check(
        &mut diagnostics,
        &manifest["record_counts"],
        &COLLECTIONS.join(" "),
        "/manifest/record_counts",
    );
    if let Some(records) = payload["records"].as_object() {
        for (collection, rows) in records {
            if !COLLECTIONS.contains(&collection.as_str()) {
                diagnostics.push(SourcePreparationDiagnostic {
                    path: format!(
                        "/records/{}",
                        collection.replace('~', "~0").replace('/', "~1")
                    ),
                });
            } else if let Some(rows) = rows.as_array() {
                for (index, row) in rows.iter().enumerate() {
                    check(
                        &mut diagnostics,
                        row,
                        fields(collection),
                        &format!("/records/{collection}/{index}"),
                    );
                }
            }
        }
    }
    if let Some(attachments) = manifest["attachments"].as_array() {
        for (index, attachment) in attachments.iter().enumerate() {
            check(
                &mut diagnostics,
                attachment,
                "attachment_id record_type record_id byte_size checksum_sha256 archive_path",
                &format!("/manifest/attachments/{index}"),
            );
        }
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(ArchivePreparationError::UnsupportedFields(diagnostics))
    }
}

fn kind(collection: &str) -> RecordKind {
    match collection {
        "people" => RecordKind::Person,
        "locations" => RecordKind::Location,
        "medications" => RecordKind::Medication,
        "dosage_options" => RecordKind::DosageOption,
        "schedules" => RecordKind::Schedule,
        "person_medications" => RecordKind::Assignment,
        "medication_takes" => RecordKind::Dose,
        "health_events" => RecordKind::HealthEvent,
        "dose_occurrences" => RecordKind::Occurrence,
        "medication_pause_periods" => RecordKind::Pause,
        "notification_preferences" => RecordKind::NotificationPreference,
        _ => unreachable!("known collection"),
    }
}

fn relationship(collection: &str, field: &str) -> RelationshipKind {
    match (collection, field) {
        ("medications", "location_portable_id") => RelationshipKind::MedicationLocation,
        ("schedules", "person_portable_id") => RelationshipKind::SchedulePerson,
        ("schedules", "medication_portable_id") => RelationshipKind::ScheduleMedication,
        ("person_medications", "person_portable_id") => RelationshipKind::AssignmentPerson,
        ("person_medications", "medication_portable_id") => RelationshipKind::AssignmentMedication,
        ("medication_takes", "taken_from_medication_portable_id") => {
            RelationshipKind::DoseTakenFromMedication
        }
        ("medication_takes", "taken_from_location_portable_id") => {
            RelationshipKind::DoseTakenFromLocation
        }
        _ => RelationshipKind::Other,
    }
}

fn details(
    row: &Value,
    attachment: bool,
    person: bool,
) -> Result<RecordDetails, ArchivePreparationError> {
    let decimal = |field: &str| -> Result<Option<Decimal>, ArchivePreparationError> {
        match &row[field] {
            Value::Null => Ok(None),
            Value::String(value) => Decimal::from_str_exact(value).map(Some).map_err(|_| {
                ArchivePreparationError::Preparation(PreparationError::UnsupportedNumber)
            }),
            _ => Err(ArchivePreparationError::Preparation(
                PreparationError::UnsupportedNumber,
            )),
        }
    };
    let source_fields = row
        .as_object()
        .ok_or(ArchivePreparationError::InvalidArchive)?
        .iter()
        .map(|(field, value)| SourceField {
            field: field.clone(),
            source_value: value.to_string(),
            outcome: if attachment || field == "updated_at" || field == "etag" {
                FieldOutcome::Unsupported
            } else if person && field == "location_portable_ids" {
                FieldOutcome::Omitted
            } else if field == "imported_context" && value != true {
                FieldOutcome::Changed
            } else {
                FieldOutcome::Preserved
            },
            destination_value: None,
            reason: if attachment {
                Some(ReasonCode::AttachmentUnsupported)
            } else if field == "updated_at"
                || field == "etag"
                || (field == "imported_context" && value != true)
            {
                Some(ReasonCode::UnsupportedField)
            } else if person && field == "location_portable_ids" {
                Some(ReasonCode::ObsoletePersonLocation)
            } else {
                None
            },
        })
        .collect();
    let occurred_at = row["taken_at"]
        .as_str()
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .map_err(|_| ArchivePreparationError::Preparation(PreparationError::InvalidGraph))?
        .map(|date| date.with_timezone(&Utc));
    Ok(RecordDetails {
        name: row["name"].as_str().map(str::to_owned),
        dose_amount: decimal("dose_amount")?,
        dose_unit: row["dose_unit"].as_str().map(str::to_owned),
        timing: row["frequency"].as_str().map(str::to_owned),
        stock_quantity: decimal("current_supply")?,
        stock_unit: None,
        occurred_at,
        source_fields,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    use zip::{ZipWriter, write::SimpleFileOptions};

    const PAYLOAD: &[u8] =
        include_bytes!("../../../tests/fixtures/portable_rails/prepared-history-v2.json");
    const MANIFEST: &[u8] =
        include_bytes!("../../../tests/fixtures/portable_rails/prepared-history-v2.manifest.json");

    fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in entries {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn prepare(bytes: &[u8]) -> Result<PreparedArchive, ArchivePreparationError> {
        prepare_archive(ArchivePreparation {
            source: SourceSystem::MedTracker05,
            archive: bytes,
            passphrase: "test secret",
        })
    }

    fn documents(payload: &Value, manifest: &Value) -> Vec<u8> {
        zip(&[
            ("portable.json", &serde_json::to_vec(payload).unwrap()),
            ("manifest.json", &serde_json::to_vec(manifest).unwrap()),
        ])
    }

    #[test]
    fn rejects_unsafe_duplicate_missing_extra_and_malformed_entries() {
        assert!(matches!(
            prepare(b"not a zip"),
            Err(ArchivePreparationError::InvalidArchive)
        ));
        assert!(matches!(
            prepare(&zip(&[
                ("../portable.json", PAYLOAD),
                ("manifest.json", MANIFEST)
            ])),
            Err(ArchivePreparationError::UnsupportedEntry)
        ));
        assert!(matches!(
            prepare(&zip(&[("portable.json", PAYLOAD)])),
            Err(ArchivePreparationError::InvalidArchive)
        ));
        assert!(matches!(
            prepare(&zip(&[
                ("portable.json", PAYLOAD),
                ("manifest.json", MANIFEST),
                ("unexpected", b"x")
            ])),
            Err(ArchivePreparationError::UnsupportedEntry)
        ));
        let mut duplicate = zip(&[
            ("portable.json", PAYLOAD),
            ("manifest.json", MANIFEST),
            ("portablE.json", b"x"),
        ]);
        let from = b"portablE.json";
        let to = b"portable.json";
        for index in (0..=duplicate.len() - from.len()).rev() {
            if &duplicate[index..index + from.len()] == from {
                duplicate[index..index + from.len()].copy_from_slice(to);
            }
        }
        assert!(matches!(
            prepare(&duplicate),
            Err(ArchivePreparationError::DuplicateEntry)
        ));
        assert!(matches!(
            prepare(&vec![0; ARCHIVE_LIMIT + 1]),
            Err(ArchivePreparationError::TooLarge)
        ));
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .add_symlink("portable.json", "elsewhere", SimpleFileOptions::default())
            .unwrap();
        writer
            .start_file("manifest.json", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(MANIFEST).unwrap();
        assert!(matches!(
            prepare(&writer.finish().unwrap().into_inner()),
            Err(ArchivePreparationError::UnsupportedEntry)
        ));
    }

    #[test]
    fn rejects_bad_counts_references_precision_and_duplicate_json() {
        let mut payload: Value = serde_json::from_slice(PAYLOAD).unwrap();
        let mut manifest: Value = serde_json::from_slice(MANIFEST).unwrap();
        manifest["record_counts"]["people"] = json!(999);
        assert!(matches!(
            prepare(&documents(&payload, &manifest)),
            Err(ArchivePreparationError::Preparation(
                PreparationError::ManifestMismatch
            ))
        ));
        manifest = serde_json::from_slice(MANIFEST).unwrap();
        payload["records"]["medications"][0]["location_portable_id"] = json!("missing");
        assert!(matches!(
            prepare(&documents(&payload, &manifest)),
            Err(ArchivePreparationError::Preparation(
                PreparationError::InvalidGraph
            ))
        ));
        let malformed = zip(&[
            ("portable.json", br#"{"format":"a","format":"b"}"#),
            ("manifest.json", MANIFEST),
        ]);
        assert!(matches!(
            prepare(&malformed),
            Err(ArchivePreparationError::Preparation(
                PreparationError::InvalidDocument
            ))
        ));
        let bad_number = String::from_utf8(PAYLOAD.to_vec()).unwrap().replace(
            "\"schedule_config\": {}",
            "\"schedule_config\": {\"amount\": 0.12345678912345678912345678912}",
        );
        assert_ne!(bad_number.as_bytes(), PAYLOAD);
        assert!(matches!(
            prepare(&zip(&[
                ("portable.json", bad_number.as_bytes()),
                ("manifest.json", MANIFEST)
            ])),
            Err(ArchivePreparationError::Preparation(
                PreparationError::UnsupportedNumber
            ))
        ));
    }

    #[test]
    fn reports_unknown_fields_and_verifies_attachment_contents() {
        let mut payload: Value = serde_json::from_slice(PAYLOAD).unwrap();
        let mut manifest: Value = serde_json::from_slice(MANIFEST).unwrap();
        payload["records"]["people"][0]["unexpected/field"] = json!("value");
        match prepare(&documents(&payload, &manifest)) {
            Err(ArchivePreparationError::UnsupportedFields(items)) => assert!(
                items
                    .iter()
                    .any(|item| item.path == "/records/people/0/unexpected~1field")
            ),
            _ => panic!("expected item-level diagnostic"),
        }
        payload = serde_json::from_slice(PAYLOAD).unwrap();
        manifest["attachments"] = json!([{ "attachment_id": 1, "record_type": "Person", "record_id": 7, "byte_size": 3, "checksum_sha256": digest(b"abc"), "archive_path": "attachments/1.bin" }]);
        let payload_bytes = serde_json::to_vec(&payload).unwrap();
        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
        let result = prepare(&zip(&[
            ("portable.json", &payload_bytes),
            ("manifest.json", &manifest_bytes),
            ("attachments/1.bin", b"abc"),
        ]))
        .unwrap();
        assert!(
            result
                .inventory
                .omissions
                .iter()
                .any(|item| item.reason == ReasonCode::AttachmentUnsupported
                    && item.acknowledgement_id.len() == 64)
        );
        assert!(matches!(
            prepare(&zip(&[
                ("portable.json", &payload_bytes),
                ("manifest.json", &manifest_bytes),
                ("attachments/1.bin", b"bad")
            ])),
            Err(ArchivePreparationError::InvalidArchive)
        ));
        assert!(matches!(
            prepare(&zip(&[
                ("portable.json", &payload_bytes),
                ("manifest.json", &manifest_bytes)
            ])),
            Err(ArchivePreparationError::InvalidArchive)
        ));
    }

    #[test]
    fn inventories_links_and_unavailable_pause_actors_without_authority() {
        let mut payload: Value = serde_json::from_slice(PAYLOAD).unwrap();
        let manifest: Value = serde_json::from_slice(MANIFEST).unwrap();
        let locations = payload["records"]["locations"].as_array().unwrap();
        let ids: Vec<Value> = locations
            .iter()
            .map(|row| row["portable_id"].clone())
            .collect();
        assert!(ids.len() >= 2);
        payload["records"]["people"][0]["location_portable_ids"] = json!([ids[0], ids[1]]);
        let result = prepare(&documents(&payload, &manifest)).unwrap();
        let links: Vec<_> = result
            .inventory
            .omissions
            .iter()
            .filter(|item| item.reason == ReasonCode::ObsoletePersonLocation)
            .collect();
        assert_eq!(links.len(), 2);
        assert_ne!(links[0].acknowledgement_id, links[1].acknowledgement_id);
        assert_eq!(
            result
                .transfer
                .conversion_receipt
                .empty_person_location_fields_removed,
            0
        );
        assert!(
            result
                .inventory
                .records
                .iter()
                .all(|record| record.details.stock_unit.is_none())
        );
        assert!(
            result
                .inventory
                .relationships
                .iter()
                .any(|link| link.field == "recorded_by_person_portable_id"
                    && link.target.id == "synthetic-absent-actor"
                    && link.reason == Some(ReasonCode::HistoricalActorUnavailable))
        );
        let preference = payload["records"]["notification_preferences"][0]["portable_id"]
            .as_str()
            .unwrap();
        assert!(
            result
                .inventory
                .records
                .iter()
                .any(
                    |record| record.identity.kind == RecordKind::NotificationPreference
                        && record.identity.id == preference
                )
        );
        payload["records"]["medication_pause_periods"][0]["source_portable_id"] =
            json!("missing-source");
        assert!(matches!(
            prepare(&documents(&payload, &manifest)),
            Err(ArchivePreparationError::Preparation(
                PreparationError::InvalidGraph
            ))
        ));
    }

    #[test]
    fn rejects_source_precision_and_mismatched_dosage_and_dates() {
        let original: Value = serde_json::from_slice(PAYLOAD).unwrap();
        let manifest: Value = serde_json::from_slice(MANIFEST).unwrap();
        for (collection, field, value) in [
            ("dosage_options", "amount", "not-decimal"),
            ("dosage_options", "current_supply", "0.001"),
            ("dosage_options", "reorder_threshold", "100000000"),
            ("dosage_options", "default_min_hours_between_doses", "1000"),
            ("schedules", "dose_amount", "0.001"),
            ("medications", "dose_amount", "0.1"),
        ] {
            let mut payload = original.clone();
            payload["records"][collection][0][field] = json!(value);
            assert!(
                matches!(
                    prepare(&documents(&payload, &manifest)),
                    Err(ArchivePreparationError::InvalidFields(_))
                ),
                "{collection}.{field}"
            );
        }
        let mut payload = original.clone();
        payload["records"]["schedules"][0]["dose_unit"] = json!("capsule");
        assert!(matches!(
            prepare(&documents(&payload, &manifest)),
            Err(ArchivePreparationError::Preparation(
                PreparationError::InvalidGraph
            ))
        ));
        for (collection, field) in [
            ("medication_pause_periods", "started_at"),
            ("dose_occurrences", "scheduled_at"),
        ] {
            let mut payload = original.clone();
            payload["records"][collection][0][field] = json!("not-a-date");
            assert!(matches!(
                prepare(&documents(&payload, &manifest)),
                Err(ArchivePreparationError::SourceValidation(_))
            ));
        }
        let mut payload = original;
        payload["records"]["schedules"][0]["dose_amount"] = json!("0.5000");
        assert!(prepare(&documents(&payload, &manifest)).is_ok());
    }

    #[test]
    fn rejects_expanded_size_entry_count_and_encrypted_flag() {
        let oversized = vec![0_u8; ARCHIVE_LIMIT];
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        writer.start_file("portable.json", options).unwrap();
        writer.write_all(PAYLOAD).unwrap();
        writer.start_file("manifest.json", options).unwrap();
        writer.write_all(MANIFEST).unwrap();
        writer.start_file("attachments/1.bin", options).unwrap();
        writer.write_all(&oversized).unwrap();
        assert!(matches!(
            prepare(&writer.finish().unwrap().into_inner()),
            Err(ArchivePreparationError::TooLarge)
        ));
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for i in 0..=ENTRY_LIMIT {
            writer
                .start_file(format!("file{i}"), SimpleFileOptions::default())
                .unwrap();
        }
        assert!(matches!(
            prepare(&writer.finish().unwrap().into_inner()),
            Err(ArchivePreparationError::TooLarge)
        ));
        let mut bytes = zip(&[("portable.json", PAYLOAD), ("manifest.json", MANIFEST)]);
        for i in 0..bytes.len().saturating_sub(10) {
            if &bytes[i..i + 4] == b"PK\x03\x04" {
                bytes[i + 6] |= 1;
            }
            if &bytes[i..i + 4] == b"PK\x01\x02" {
                bytes[i + 8] |= 1;
            }
        }
        assert!(prepare(&bytes).is_err());
    }
}
