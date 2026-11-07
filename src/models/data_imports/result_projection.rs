use std::collections::{HashMap, HashSet};

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use sha2::{Digest, Sha256};

use super::contracts::{
    ActualRecord, ActualRelationship, AssignmentId, DestinationId, DestinationRecord,
    DosageOptionId, DoseId, ExpectedRecord, ExpectedRelationship, HealthEventId, ImportReport,
    ImportState, LocationId, MedicationId, NotificationPreferenceId, OccurrenceId, PauseId,
    PersonId, PreviewToken, RecordKind, RecordResult, RelationshipResult, ScheduleId,
    SourceIdentity, SourceInventory,
};
use crate::models::{
    access::TenantTransaction,
    care::portable_imports::{
        CanonicalRestoredSchedule, CreatedRecord, ExistingIndex, ResolvedAlias,
    },
    entities::health_event_medication,
    errors::OperationError,
};

fn invalid() -> OperationError {
    OperationError::unavailable("import_result_verification")
}

pub(crate) fn token_verifier(token: &PreviewToken) -> String {
    let mut digest = Sha256::new();
    digest.update(b"medtracker.import.confirmation.v1\0");
    digest.update(token.0.as_bytes());
    hex::encode(digest.finalize())
}

pub(crate) fn token_matches(expected: &str, token: &PreviewToken) -> bool {
    let actual = token_verifier(token);
    expected.len() == actual.len()
        && expected
            .bytes()
            .zip(actual.bytes())
            .fold(0_u8, |diff, (a, b)| diff | (a ^ b))
            == 0
}

fn checked_link(verified: bool) -> Result<ActualRelationship, OperationError> {
    if verified {
        Ok(ActualRelationship::Verified)
    } else {
        Err(invalid())
    }
}

struct Native<'a> {
    id: i64,
    kind: RecordKind,
    destination: DestinationRecord,
    created: Option<&'a CreatedRecord>,
}

fn created_identity(
    record: &CreatedRecord,
) -> (RecordKind, &str, i64, DestinationId, &'static str) {
    match record {
        CreatedRecord::Person(row) => (
            RecordKind::Person,
            &row.portable_id,
            row.id,
            DestinationId::Person(PersonId(row.id)),
            "people",
        ),
        CreatedRecord::Location(row) => (
            RecordKind::Location,
            &row.portable_id,
            row.id,
            DestinationId::Location(LocationId(row.id)),
            "locations",
        ),
        CreatedRecord::Medication(row) => (
            RecordKind::Medication,
            &row.portable_id,
            row.id,
            DestinationId::Medication(MedicationId(row.id)),
            "medications",
        ),
        CreatedRecord::DosageOption(row) => (
            RecordKind::DosageOption,
            &row.portable_id,
            row.id,
            DestinationId::DosageOption(DosageOptionId(row.id)),
            "dosage_options",
        ),
        CreatedRecord::Schedule(row) => (
            RecordKind::Schedule,
            &row.portable_id,
            row.id,
            DestinationId::Schedule(ScheduleId(row.id)),
            "schedules",
        ),
        CreatedRecord::Assignment(row) => (
            RecordKind::Assignment,
            &row.portable_id,
            row.id,
            DestinationId::Assignment(AssignmentId(row.id)),
            "person_medications",
        ),
        CreatedRecord::Pause(row) => (
            RecordKind::Pause,
            &row.portable_id,
            row.id,
            DestinationId::Pause(PauseId(row.id)),
            "medication_pause_periods",
        ),
        CreatedRecord::Dose(row) => (
            RecordKind::Dose,
            &row.portable_id,
            row.id,
            DestinationId::Dose(DoseId(row.id)),
            "medication_takes",
        ),
        CreatedRecord::NotificationPreference(row) => (
            RecordKind::NotificationPreference,
            &row.portable_id,
            row.id,
            DestinationId::NotificationPreference(NotificationPreferenceId(row.id)),
            "notification_preferences",
        ),
        CreatedRecord::Occurrence(row) => (
            RecordKind::Occurrence,
            &row.portable_id,
            row.id,
            DestinationId::Occurrence(OccurrenceId(row.id)),
            "dose_occurrences",
        ),
        CreatedRecord::HealthEvent(row) => (
            RecordKind::HealthEvent,
            &row.portable_id,
            row.id,
            DestinationId::HealthEvent(HealthEventId(row.id)),
            "health_events",
        ),
    }
}

fn native_link(source: &Native<'_>, target: &Native<'_>, field: &str) -> bool {
    let kinds_match = match (source.kind, field) {
        (RecordKind::Person, "notification_preference_portable_id") => {
            target.kind == RecordKind::NotificationPreference
        }
        (RecordKind::Medication, "location_portable_id")
        | (RecordKind::Dose, "taken_from_location_portable_id") => {
            target.kind == RecordKind::Location
        }
        (
            RecordKind::DosageOption | RecordKind::Schedule | RecordKind::Assignment,
            "medication_portable_id",
        )
        | (RecordKind::Dose, "taken_from_medication_portable_id") => {
            target.kind == RecordKind::Medication
        }
        (
            RecordKind::Schedule
            | RecordKind::Assignment
            | RecordKind::NotificationPreference
            | RecordKind::HealthEvent,
            "person_portable_id",
        ) => target.kind == RecordKind::Person,
        (RecordKind::Schedule | RecordKind::Assignment, "source_dosage_option_portable_id") => {
            target.kind == RecordKind::DosageOption
        }
        (RecordKind::Dose | RecordKind::Pause | RecordKind::Occurrence, "source_portable_id") => {
            matches!(target.kind, RecordKind::Schedule | RecordKind::Assignment)
        }
        (RecordKind::Occurrence, "medication_take_portable_id") => target.kind == RecordKind::Dose,
        _ => false,
    };
    if !kinds_match {
        return false;
    }
    match (source.created, field) {
        (Some(CreatedRecord::Medication(row)), "location_portable_id") => {
            row.location_id == target.id
        }
        (Some(CreatedRecord::DosageOption(row)), "medication_portable_id") => {
            row.medication_id == target.id
        }
        (Some(CreatedRecord::Schedule(row)), "person_portable_id") => row.person_id == target.id,
        (Some(CreatedRecord::Schedule(row)), "medication_portable_id") => {
            row.medication_id == target.id
        }
        (Some(CreatedRecord::Schedule(row)), "source_dosage_option_portable_id") => {
            row.source_dosage_option_id == Some(target.id)
        }
        (Some(CreatedRecord::Assignment(row)), "person_portable_id") => row.person_id == target.id,
        (Some(CreatedRecord::Assignment(row)), "medication_portable_id") => {
            row.medication_id == target.id
        }
        (Some(CreatedRecord::Assignment(row)), "source_dosage_option_portable_id") => {
            row.source_dosage_option_id == Some(target.id)
        }
        (Some(CreatedRecord::Dose(row)), "source_portable_id") => {
            row.schedule_id == Some(target.id) && target.kind == RecordKind::Schedule
                || row.person_medication_id == Some(target.id)
                    && target.kind == RecordKind::Assignment
        }
        (Some(CreatedRecord::Dose(row)), "taken_from_medication_portable_id") => {
            row.taken_from_medication_id == Some(target.id)
        }
        (Some(CreatedRecord::Dose(row)), "taken_from_location_portable_id") => {
            row.taken_from_location_id == Some(target.id)
        }
        (Some(CreatedRecord::Pause(row)), "source_portable_id") => {
            row.schedule_id == Some(target.id) && target.kind == RecordKind::Schedule
                || row.person_medication_id == Some(target.id)
                    && target.kind == RecordKind::Assignment
        }
        (Some(CreatedRecord::Occurrence(row)), "source_portable_id") => {
            row.schedule_id == Some(target.id) && target.kind == RecordKind::Schedule
                || row.person_medication_id == Some(target.id)
                    && target.kind == RecordKind::Assignment
        }
        (Some(CreatedRecord::Occurrence(row)), "medication_take_portable_id") => {
            row.medication_take_id == Some(target.id)
        }
        (Some(CreatedRecord::NotificationPreference(row)), "person_portable_id") => {
            row.person_id == target.id
        }
        (Some(CreatedRecord::HealthEvent(row)), "person_portable_id") => row.person_id == target.id,
        (_, "notification_preference_portable_id") => {
            matches!(target.created, Some(CreatedRecord::NotificationPreference(row)) if row.person_id == source.id)
        }
        _ => false,
    }
}

type EdgeKey = (SourceIdentity, SourceIdentity, String);

fn project_record(
    record: &mut RecordResult,
    native: &HashMap<SourceIdentity, Native<'_>>,
    originals: &mut HashMap<SourceIdentity, RecordResult>,
    totals: &mut super::contracts::ReportTotals,
) -> Result<(), OperationError> {
    if let Some(previous) = originals.get(&record.source) {
        if previous != record {
            return Err(invalid());
        }
    } else {
        originals.insert(record.source.clone(), record.clone());
        match record.expected {
            ExpectedRecord::Create => totals.created += 1,
            ExpectedRecord::Match => totals.matched += 1,
            ExpectedRecord::Skip => totals.skipped += 1,
            ExpectedRecord::Reject | ExpectedRecord::DecisionRequired => return Err(invalid()),
        }
    }
    if record.actual.is_some() {
        return Err(invalid());
    }
    match record.expected {
        ExpectedRecord::Create | ExpectedRecord::Match => {
            let row = native.get(&record.source).ok_or_else(invalid)?;
            if (record.expected == ExpectedRecord::Create) != row.created.is_some() {
                return Err(invalid());
            }
            if record.expected == ExpectedRecord::Match
                && record.destination.as_ref() != Some(&row.destination)
            {
                return Err(invalid());
            }
            record.destination = Some(row.destination.clone());
            record.actual = Some(if row.created.is_some() {
                ActualRecord::Created
            } else {
                ActualRecord::Matched
            });
        }
        ExpectedRecord::Skip => {
            if native.contains_key(&record.source) || record.destination.is_some() {
                return Err(invalid());
            }
            record.actual = Some(ActualRecord::Skipped);
        }
        ExpectedRecord::Reject | ExpectedRecord::DecisionRequired => return Err(invalid()),
    }
    Ok(())
}

fn project_edge(
    edge: &mut RelationshipResult,
    native: &HashMap<SourceIdentity, Native<'_>>,
    junctions: &HashSet<(i64, i64)>,
    expected_junctions: &mut HashSet<(i64, i64)>,
    originals: &mut HashMap<EdgeKey, RelationshipResult>,
    totals: &mut super::contracts::ReportTotals,
) -> Result<(), OperationError> {
    let field = edge.field.as_ref().ok_or_else(invalid)?;
    let key = (edge.source.clone(), edge.target.clone(), field.clone());
    let first = if let Some(previous) = originals.get(&key) {
        if previous != edge {
            return Err(invalid());
        }
        false
    } else {
        originals.insert(key, edge.clone());
        true
    };
    if edge.actual.is_some()
        || edge.destination_source.is_some()
        || edge.destination_target.is_some()
    {
        return Err(invalid());
    }
    match edge.expected {
        ExpectedRelationship::Omit => edge.actual = Some(ActualRelationship::Omitted),
        ExpectedRelationship::Link => {
            let source = native.get(&edge.source).ok_or_else(invalid)?;
            let target = native.get(&edge.target).ok_or_else(invalid)?;
            let verified = if source.kind == RecordKind::HealthEvent
                && target.kind == RecordKind::Medication
                && field == "medication_portable_ids"
            {
                let pair = (source.id, target.id);
                expected_junctions.insert(pair);
                junctions.contains(&pair)
            } else {
                native_link(source, target, field)
            };
            edge.actual = Some(checked_link(verified)?);
            edge.destination_source = Some(source.destination.clone());
            edge.destination_target = Some(target.destination.clone());
            if first {
                totals.verified_links += 1;
            }
        }
        ExpectedRelationship::Reject | ExpectedRelationship::DecisionRequired => {
            return Err(invalid());
        }
    }
    Ok(())
}

/// Projects only evidence returned by the create-only and canonical restore operations.
/// The caller must retain their transaction and authority locks until this returns.
pub(crate) async fn completed(
    tenant: &TenantTransaction,
    preview: &ImportReport,
    inventory: &SourceInventory,
    aliases: &[ResolvedAlias],
    index: &ExistingIndex,
    created: &[CreatedRecord],
    canonical: &[CanonicalRestoredSchedule],
) -> Result<ImportReport, OperationError> {
    if !matches!(preview.state, ImportState::Ready | ImportState::Applying)
        || !preview.issues.is_empty()
        || preview.totals.decisions_required != 0
    {
        return Err(invalid());
    }
    let mut identities = HashMap::new();
    for record in &inventory.records {
        if identities
            .insert(
                (record.identity.kind, record.identity.id.as_str()),
                record.identity.clone(),
            )
            .is_some()
        {
            return Err(invalid());
        }
    }
    let mut native = HashMap::new();
    let mut schedules = HashMap::new();
    let mut scheduled_takes = HashMap::new();
    let mut event_ids = Vec::new();
    for row in created {
        let (kind, portable, id, destination, collection) = created_identity(row);
        if id <= 0 || index.get(collection, portable) != Some(id) {
            return Err(invalid());
        }
        let source = identities
            .get(&(kind, portable))
            .ok_or_else(invalid)?
            .clone();
        if native
            .insert(
                source,
                Native {
                    id,
                    kind,
                    destination: DestinationRecord {
                        id: destination,
                        href: None,
                    },
                    created: Some(row),
                },
            )
            .is_some()
        {
            return Err(invalid());
        }
        match row {
            CreatedRecord::Schedule(schedule) => {
                if schedules.insert(schedule.id, schedule).is_some() {
                    return Err(invalid());
                }
            }
            CreatedRecord::Dose(take) if take.schedule_id.is_some() => {
                if scheduled_takes.insert(take.id, take).is_some() {
                    return Err(invalid());
                }
            }
            CreatedRecord::HealthEvent(event) => event_ids.push(event.id),
            _ => (),
        }
    }
    let mut canonical_ids = HashSet::new();
    let mut canonical_takes = HashSet::new();
    for item in canonical {
        let source = schedules.get(&item.execution.id).ok_or_else(invalid)?;
        if !canonical_ids.insert(item.execution.id)
            || source.portable_id != item.execution.portable_id
            || source.person_id != item.execution.person_id
            || source.medication_id != item.execution.medication_id
            || source.household_id != item.execution.household_id
        {
            return Err(invalid());
        }
        for take in &item.takes {
            let original = scheduled_takes.get(&take.take.id).ok_or_else(invalid)?;
            if original.as_ref() != &take.take
                || take.take.schedule_id != Some(item.execution.id)
                || !canonical_takes.insert(take.take.id)
            {
                return Err(invalid());
            }
        }
    }
    if schedules.keys().copied().collect::<HashSet<_>>() != canonical_ids
        || scheduled_takes.keys().copied().collect::<HashSet<_>>() != canonical_takes
    {
        return Err(invalid());
    }
    for alias in aliases {
        let source = identities
            .get(&(RecordKind::Person, alias.source_person_portable_id.as_str()))
            .ok_or_else(invalid)?
            .clone();
        let id = alias.destination_person_id.get();
        if index.get("people", &alias.destination_person_portable_id) != Some(id)
            || native
                .insert(
                    source,
                    Native {
                        id,
                        kind: RecordKind::Person,
                        destination: DestinationRecord {
                            id: DestinationId::Person(PersonId(id)),
                            href: None,
                        },
                        created: None,
                    },
                )
                .is_some()
        {
            return Err(invalid());
        }
    }
    let mut junctions = HashSet::new();
    if !event_ids.is_empty() {
        let rows = health_event_medication::Entity::find()
            .filter(health_event_medication::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(health_event_medication::Column::HealthEventId.is_in(event_ids))
            .all(tenant.transaction())
            .await?;
        for row in rows {
            if !junctions.insert((row.health_event_id, row.medication_id.ok_or_else(invalid)?)) {
                return Err(invalid());
            }
        }
    }
    let mut result = preview.clone();
    result.totals.created = 0;
    result.totals.matched = 0;
    result.totals.verified_links = 0;
    result.totals.skipped = 0;
    result.totals.rejected = 0;
    result.totals.unlinked = 0;
    let mut records = HashMap::new();
    let mut edges = HashMap::new();
    let mut expected_junctions = HashSet::new();
    let mut omissions = HashMap::new();
    for person in &mut result.people {
        project_record(
            &mut person.person,
            &native,
            &mut records,
            &mut result.totals,
        )?;
        for record in &mut person.records {
            project_record(record, &native, &mut records, &mut result.totals)?;
        }
        for edge in &mut person.relationships {
            project_edge(
                edge,
                &native,
                &junctions,
                &mut expected_junctions,
                &mut edges,
                &mut result.totals,
            )?;
        }
        for item in &person.omissions {
            check_omission(item, &mut omissions)?;
        }
    }
    for record in &mut result.records {
        project_record(record, &native, &mut records, &mut result.totals)?;
    }
    for edge in &mut result.relationships {
        project_edge(
            edge,
            &native,
            &junctions,
            &mut expected_junctions,
            &mut edges,
            &mut result.totals,
        )?;
    }
    for item in &result.omissions {
        check_omission(item, &mut omissions)?;
    }
    if junctions != expected_junctions {
        return Err(invalid());
    }
    let expected_records: HashSet<_> = inventory
        .records
        .iter()
        .map(|item| item.identity.clone())
        .collect();
    let expected_edges: HashSet<_> = inventory
        .relationships
        .iter()
        .map(|edge| (edge.source.clone(), edge.target.clone(), edge.field.clone()))
        .collect();
    if records.keys().cloned().collect::<HashSet<_>>() != expected_records
        || edges.keys().cloned().collect::<HashSet<_>>() != expected_edges
        || expected_edges.len() != inventory.relationships.len()
        || inventory.relationships.iter().any(|edge| {
            edges
                .get(&(edge.source.clone(), edge.target.clone(), edge.field.clone()))
                .is_none_or(|reported| reported.kind != edge.kind)
        })
        || native.len()
            != usize::try_from(result.totals.created + result.totals.matched)
                .map_err(|_| invalid())?
        || result.totals.created != result.totals.expected_create
        || result.totals.matched != result.totals.expected_match
        || result.totals.verified_links != result.totals.expected_link
        || omissions.len() != inventory.omissions.len()
        || u64::try_from(omissions.len()).map_err(|_| invalid())? != result.totals.omissions
        || inventory.omissions.iter().any(|item| {
            omissions.get(&item.acknowledgement_id).is_none_or(|found| {
                let mut expected = item.clone();
                expected.acknowledged = true;
                *found != expected
            })
        })
    {
        return Err(invalid());
    }
    result.state = ImportState::Completed;
    Ok(result)
}

fn check_omission(
    item: &super::contracts::Omission,
    seen: &mut HashMap<String, super::contracts::Omission>,
) -> Result<(), OperationError> {
    if !item.acknowledged {
        return Err(invalid());
    }
    if let Some(previous) = seen.insert(item.acknowledgement_id.clone(), item.clone())
        && previous != *item
    {
        return Err(invalid());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_verifier_accepts_only_the_original_token() {
        let token = PreviewToken("preview-token".into());
        let verifier = token_verifier(&token);
        assert!(token_matches(&verifier, &token));
        assert!(!token_matches(
            &verifier,
            &PreviewToken("other-token".into())
        ));
        assert!(!token_matches("invalid", &token));
    }

    #[test]
    fn unverified_relationship_cannot_be_reported_as_completed() {
        assert!(checked_link(false).is_err());
        assert_eq!(checked_link(true).unwrap(), ActualRelationship::Verified);
    }

    #[test]
    fn notification_link_checks_the_actual_person_and_target_kind() {
        let preference = CreatedRecord::NotificationPreference(Box::new(
            crate::models::entities::notification_preference::Model {
                id: 8,
                household_id: 1,
                person_id: 4,
                portable_id: "preference".into(),
                created_at: Default::default(),
                updated_at: Default::default(),
                enabled: true,
                dose_due_enabled: true,
                missed_dose_enabled: true,
                low_stock_enabled: true,
                private_text_enabled: true,
                morning_time: None,
                afternoon_time: None,
                evening_time: None,
                night_time: None,
            },
        ));
        let preference = Native {
            id: 8,
            kind: RecordKind::NotificationPreference,
            destination: DestinationRecord {
                id: DestinationId::NotificationPreference(NotificationPreferenceId(8)),
                href: None,
            },
            created: Some(&preference),
        };
        let mut person = Native {
            id: 4,
            kind: RecordKind::Person,
            destination: DestinationRecord {
                id: DestinationId::Person(PersonId(4)),
                href: None,
            },
            created: None,
        };
        assert!(native_link(
            &person,
            &preference,
            "notification_preference_portable_id"
        ));
        person.id = 5;
        assert!(!native_link(
            &person,
            &preference,
            "notification_preference_portable_id"
        ));
        person.id = 4;
        person.kind = RecordKind::Medication;
        assert!(!native_link(
            &person,
            &preference,
            "notification_preference_portable_id"
        ));
    }

    #[test]
    fn repeated_matched_record_is_counted_once_and_changed_copy_is_rejected() {
        let preview: super::super::contracts::PreviewResult =
            serde_json::from_str(include_str!("examples/preview.json")).unwrap();
        let mut person = preview.report.people[0].person.clone();
        person.expected = ExpectedRecord::Match;
        let destination = DestinationRecord {
            id: DestinationId::Person(PersonId(4)),
            href: None,
        };
        person.destination = Some(destination.clone());
        let native = HashMap::from([(
            person.source.clone(),
            Native {
                id: 4,
                kind: RecordKind::Person,
                destination,
                created: None,
            },
        )]);
        let mut originals = HashMap::new();
        let mut totals = super::super::contracts::ReportTotals::default();
        let mut first = person.clone();
        let mut second = person.clone();
        project_record(&mut first, &native, &mut originals, &mut totals).unwrap();
        project_record(&mut second, &native, &mut originals, &mut totals).unwrap();
        assert_eq!(totals.matched, 1);
        assert_eq!(first.actual, Some(ActualRecord::Matched));
        person.details.name = Some("Changed".into());
        assert!(project_record(&mut person, &native, &mut originals, &mut totals).is_err());
    }

    #[test]
    fn health_event_link_requires_the_actual_junction() {
        let preview: super::super::contracts::PreviewResult =
            serde_json::from_str(include_str!("examples/preview.json")).unwrap();
        let mut edge = preview.report.people[0].relationships[0].clone();
        edge.source.kind = RecordKind::HealthEvent;
        edge.target.kind = RecordKind::Medication;
        edge.field = Some("medication_portable_ids".into());
        let native = HashMap::from([
            (
                edge.source.clone(),
                Native {
                    id: 10,
                    kind: RecordKind::HealthEvent,
                    destination: DestinationRecord {
                        id: DestinationId::HealthEvent(HealthEventId(10)),
                        href: None,
                    },
                    created: None,
                },
            ),
            (
                edge.target.clone(),
                Native {
                    id: 20,
                    kind: RecordKind::Medication,
                    destination: DestinationRecord {
                        id: DestinationId::Medication(MedicationId(20)),
                        href: None,
                    },
                    created: None,
                },
            ),
        ]);
        let mut originals = HashMap::new();
        let mut expected = HashSet::new();
        let mut totals = super::super::contracts::ReportTotals::default();
        assert!(
            project_edge(
                &mut edge.clone(),
                &native,
                &HashSet::new(),
                &mut expected,
                &mut originals,
                &mut totals
            )
            .is_err()
        );
        originals.clear();
        expected.clear();
        project_edge(
            &mut edge,
            &native,
            &HashSet::from([(10, 20)]),
            &mut expected,
            &mut originals,
            &mut totals,
        )
        .unwrap();
        assert_eq!(edge.actual, Some(ActualRelationship::Verified));
        assert_eq!(expected, HashSet::from([(10, 20)]));
        assert_eq!(totals.verified_links, 1);
    }
}
