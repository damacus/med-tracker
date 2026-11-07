use std::collections::{HashMap, HashSet, VecDeque};

use super::{
    contracts::{
        DestinationId, DestinationRecord, ExpectedRecord, ExpectedRelationship, FieldOutcome,
        PersonId, ReasonCode, RecordKind, RelationshipKind, SourceField, SourceIdentity,
        SourceInventory,
    },
    projection::{PlannedOutcomes, PlannedRecord, PlannedRelationship},
};
use crate::models::care::portable_imports::{
    ProfileDifference, ResolvedAlias,
    planning::{
        AssessmentIssue, AssessmentReason, ImportAssessment, PlannedAction, PlannedRow,
        SourceCollection,
    },
};

/// A finding attributed by the caller to a real source person.
pub(crate) struct PersonFinding {
    pub(crate) person: SourceIdentity,
    pub(crate) field: Option<String>,
    pub(crate) reason: ReasonCode,
    pub(crate) detail: String,
}

/// Attachment ownership cannot be derived from the current inventory alone.
pub(crate) struct AttachmentParent {
    pub(crate) attachment: SourceIdentity,
    pub(crate) parent: SourceIdentity,
}

pub(crate) struct BridgeIssue {
    pub(crate) source: Option<SourceIdentity>,
    pub(crate) field: Option<String>,
    pub(crate) reason: ReasonCode,
    pub(crate) detail: String,
}

struct AssessmentView<'a> {
    rows: &'a [PlannedRow],
    issues: &'a [AssessmentIssue],
    aliases: &'a [ResolvedAlias],
}

pub(crate) struct BridgeOutput {
    pub(crate) outcomes: PlannedOutcomes,
    pub(crate) issues: Vec<BridgeIssue>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum BridgeError {
    DuplicateRecord,
    DuplicateRelationship,
    UnknownPerson,
    UnknownAttachmentParent,
    DuplicatePlannerRow,
    UnknownPlannerRow,
    InvalidGlobalIssue,
    MissingAlias,
}

fn kind(collection: SourceCollection) -> RecordKind {
    match collection {
        SourceCollection::People => RecordKind::Person,
        SourceCollection::Locations => RecordKind::Location,
        SourceCollection::Medications => RecordKind::Medication,
        SourceCollection::DosageOptions => RecordKind::DosageOption,
        SourceCollection::Schedules => RecordKind::Schedule,
        SourceCollection::PersonMedications => RecordKind::Assignment,
        SourceCollection::MedicationTakes => RecordKind::Dose,
        SourceCollection::NotificationPreferences => RecordKind::NotificationPreference,
        SourceCollection::MedicationPausePeriods => RecordKind::Pause,
        SourceCollection::DoseOccurrences => RecordKind::Occurrence,
        SourceCollection::HealthEvents => RecordKind::HealthEvent,
    }
}

fn reason(reason: AssessmentReason) -> ReasonCode {
    match reason {
        AssessmentReason::OccupiedSourceId
        | AssessmentReason::NameConflict
        | AssessmentReason::IncompatiblePerson => ReasonCode::IdentityConflict,
        AssessmentReason::InvalidSource => ReasonCode::SourceInvalid,
        AssessmentReason::MissingDependency => ReasonCode::MissingDependency,
        AssessmentReason::InsufficientAuthority => ReasonCode::InsufficientAuthority,
    }
}

fn actor(field: &str) -> bool {
    matches!(
        field,
        "recorded_by_person_portable_id" | "resumed_by_person_portable_id"
    )
}

/// Produces plans only; neither a match nor a link asserts a completed write.
pub(crate) fn bridge(
    inventory: &SourceInventory,
    assessment: &ImportAssessment,
    findings: &[PersonFinding],
    attachment_parents: &[AttachmentParent],
) -> Result<BridgeOutput, BridgeError> {
    project_inventory(
        inventory,
        Some(AssessmentView {
            rows: &assessment.rows,
            issues: &assessment.issues,
            aliases: &assessment.aliases,
        }),
        findings,
        attachment_parents,
        None,
    )
}

/// Project an inventory when identity normalisation stopped before assessment.
pub(crate) fn blocked_inventory(
    inventory: &SourceInventory,
    findings: &[PersonFinding],
    attachment_parents: &[AttachmentParent],
    global_issue: BridgeIssue,
) -> Result<BridgeOutput, BridgeError> {
    if global_issue.source.is_some() {
        return Err(BridgeError::InvalidGlobalIssue);
    }
    project_inventory(
        inventory,
        None,
        findings,
        attachment_parents,
        Some(global_issue),
    )
}

fn project_inventory(
    inventory: &SourceInventory,
    assessment: Option<AssessmentView<'_>>,
    findings: &[PersonFinding],
    attachment_parents: &[AttachmentParent],
    global_issue: Option<BridgeIssue>,
) -> Result<BridgeOutput, BridgeError> {
    let mut records = HashMap::new();
    let mut by_key = HashMap::new();
    for record in &inventory.records {
        if records.insert(record.identity.clone(), record).is_some()
            || by_key
                .insert(
                    (record.identity.kind, record.identity.id.as_str()),
                    &record.identity,
                )
                .is_some()
        {
            return Err(BridgeError::DuplicateRecord);
        }
    }
    let mut rows = HashMap::new();
    for row in assessment
        .as_ref()
        .into_iter()
        .flat_map(|assessment| assessment.rows)
    {
        let Some(identity) = by_key.get(&(kind(row.collection), row.portable_id.as_str())) else {
            return Err(BridgeError::UnknownPlannerRow);
        };
        if rows.insert((*identity).clone(), row.action).is_some() {
            return Err(BridgeError::DuplicatePlannerRow);
        }
    }
    let mut issues: Vec<_> = global_issue.into_iter().collect();
    let mut row_reasons = HashMap::new();
    for issue in assessment
        .as_ref()
        .into_iter()
        .flat_map(|assessment| assessment.issues)
    {
        let source = issue
            .collection
            .zip(issue.portable_id.as_deref())
            .and_then(|(collection, id)| by_key.get(&(kind(collection), id)).copied())
            .cloned();
        if let Some(source) = &source {
            row_reasons
                .entry(source.clone())
                .or_insert(reason(issue.reason));
        }
        issues.push(BridgeIssue {
            source,
            field: issue.field.clone(),
            reason: reason(issue.reason),
            detail: issue.detail.clone(),
        });
    }
    let mut blocked_people: HashMap<_, _> = row_reasons
        .iter()
        .filter(|(source, _)| source.kind == RecordKind::Person)
        .map(|(source, reason)| (source.clone(), *reason))
        .collect();
    for finding in findings {
        if finding.person.kind != RecordKind::Person || !records.contains_key(&finding.person) {
            return Err(BridgeError::UnknownPerson);
        }
        blocked_people
            .entry(finding.person.clone())
            .or_insert(finding.reason);
        issues.push(BridgeIssue {
            source: Some(finding.person.clone()),
            field: finding.field.clone(),
            reason: finding.reason,
            detail: finding.detail.clone(),
        });
    }
    let owners = ownership(inventory, attachment_parents, &records)?;
    let aliases: HashMap<_, _> = assessment
        .as_ref()
        .into_iter()
        .flat_map(|assessment| assessment.aliases)
        .map(|alias| (alias.source_person_portable_id.as_str(), alias))
        .collect();
    for (source, action) in &rows {
        if *action == PlannedAction::Match
            && (source.kind != RecordKind::Person || !aliases.contains_key(source.id.as_str()))
        {
            return Err(BridgeError::MissingAlias);
        }
    }
    for (source, action) in &rows {
        if *action == PlannedAction::Reject {
            row_reasons
                .entry(source.clone())
                .or_insert(ReasonCode::SourceInvalid);
        }
    }
    for (person, why) in &blocked_people {
        row_reasons.entry(person.clone()).or_insert(*why);
    }
    if assessment.is_some() {
        for item in &inventory.records {
            if item.identity.kind != RecordKind::Attachment && !rows.contains_key(&item.identity) {
                row_reasons
                    .entry(item.identity.clone())
                    .or_insert(ReasonCode::MissingDependency);
                issues.push(BridgeIssue {
                    source: Some(item.identity.clone()),
                    field: None,
                    reason: ReasonCode::MissingDependency,
                    detail: "No planner row for source record".into(),
                });
            }
        }
        for edge in &inventory.relationships {
            if dependency_source(edge.source.kind)
                && !actor(&edge.field)
                && edge.kind != RelationshipKind::PersonLocation
                && edge.target.kind != RecordKind::Attachment
                && edge.reason.is_none()
                && !records.contains_key(&edge.target)
            {
                row_reasons
                    .entry(edge.source.clone())
                    .or_insert(ReasonCode::MissingDependency);
                issues.push(BridgeIssue {
                    source: records
                        .contains_key(&edge.source)
                        .then(|| edge.source.clone()),
                    field: Some(edge.field.clone()),
                    reason: ReasonCode::MissingDependency,
                    detail: "Source dependency is unavailable".into(),
                });
            }
        }
    }
    let mut dependants: HashMap<SourceIdentity, Vec<SourceIdentity>> = HashMap::new();
    for edge in &inventory.relationships {
        if !actor(&edge.field)
            && edge.kind != RelationshipKind::PersonLocation
            && edge.reason.is_none()
            && edge.target.kind != RecordKind::Attachment
            && dependency_source(edge.source.kind)
        {
            dependants
                .entry(edge.target.clone())
                .or_default()
                .push(edge.source.clone());
        }
    }
    let mut pending: VecDeque<_> = row_reasons
        .iter()
        .map(|(source, why)| (source.clone(), *why))
        .collect();
    while let Some((source, why)) = pending.pop_front() {
        if let Some(dependent) = dependants.get(&source) {
            for target in dependent {
                if !row_reasons.contains_key(target) {
                    row_reasons.insert(target.clone(), why);
                    pending.push_back((target.clone(), why));
                }
            }
        }
    }
    let mut planned = Vec::with_capacity(inventory.records.len());
    for record in &inventory.records {
        let identity = &record.identity;
        let people = owners.get(identity).cloned().unwrap_or_default();
        let mut details = record.details.clone();
        let alias = (identity.kind == RecordKind::Person)
            .then(|| aliases.get(identity.id.as_str()))
            .flatten();
        if let Some(alias) = alias {
            record_alias_field(
                &mut details.source_fields,
                "portable_id",
                Some(&identity.id),
                Some(&alias.destination_person_portable_id),
            );
            for difference in &alias.profile_differences {
                let (field, source, destination) = match difference {
                    ProfileDifference::Name {
                        source,
                        destination,
                    } => ("name", Some(source.as_str()), Some(destination.as_str())),
                    ProfileDifference::Email {
                        source,
                        destination,
                    } => ("email", source.as_deref(), destination.as_deref()),
                };
                record_alias_field(&mut details.source_fields, field, source, destination);
            }
        }
        let blocker = row_reasons.get(identity).copied().or_else(|| {
            matches!(
                identity.kind,
                RecordKind::Person
                    | RecordKind::Schedule
                    | RecordKind::Assignment
                    | RecordKind::Dose
                    | RecordKind::Pause
                    | RecordKind::Occurrence
                    | RecordKind::HealthEvent
                    | RecordKind::NotificationPreference
            )
            .then(|| {
                people
                    .iter()
                    .find_map(|person| blocked_people.get(person).copied())
            })
            .flatten()
        });
        let (expected, why) = if identity.kind == RecordKind::Attachment {
            (
                ExpectedRecord::Skip,
                Some(ReasonCode::AttachmentUnsupported),
            )
        } else if let Some(reason) = blocker {
            (ExpectedRecord::Reject, Some(reason))
        } else if assessment.is_none() {
            (ExpectedRecord::DecisionRequired, None)
        } else {
            match rows.get(identity) {
                Some(PlannedAction::Create) => (ExpectedRecord::Create, None),
                Some(PlannedAction::Match) => (ExpectedRecord::Match, None),
                Some(PlannedAction::Reject) => {
                    (ExpectedRecord::Reject, Some(ReasonCode::SourceInvalid))
                }
                None => (ExpectedRecord::Reject, Some(ReasonCode::MissingDependency)),
            }
        };
        let destination = alias.map(|a| DestinationRecord {
            id: DestinationId::Person(PersonId(a.destination_person_id.get())),
            href: None,
        });
        planned.push(PlannedRecord {
            source: identity.clone(),
            details,
            expected,
            destination,
            reason: why,
            people,
        });
    }
    let planned_index: HashMap<_, _> = planned
        .iter()
        .map(|item| (item.source.clone(), item))
        .collect();
    let mut relationships = Vec::with_capacity(inventory.relationships.len());
    for edge in &inventory.relationships {
        let people = owners.get(&edge.source).cloned().unwrap_or_default();
        let (expected, why) = if edge.kind == RelationshipKind::PersonLocation {
            (
                ExpectedRelationship::Omit,
                Some(ReasonCode::ObsoletePersonLocation),
            )
        } else if edge.source.kind == RecordKind::Attachment
            || edge.target.kind == RecordKind::Attachment
        {
            (
                ExpectedRelationship::Omit,
                Some(ReasonCode::AttachmentUnsupported),
            )
        } else if actor(&edge.field) {
            (
                ExpectedRelationship::Omit,
                Some(edge.reason.unwrap_or(ReasonCode::UnsupportedRelationship)),
            )
        } else if let Some(reason) = edge.reason {
            (ExpectedRelationship::Omit, Some(reason))
        } else if let Some(reason) = planned_index
            .get(&edge.source)
            .and_then(|item| {
                (item.expected == ExpectedRecord::Reject)
                    .then_some(item.reason)
                    .flatten()
            })
            .or_else(|| {
                planned_index.get(&edge.target).and_then(|item| {
                    (item.expected == ExpectedRecord::Reject)
                        .then_some(item.reason)
                        .flatten()
                })
            })
        {
            (ExpectedRelationship::Reject, Some(reason))
        } else if !records.contains_key(&edge.source) || !records.contains_key(&edge.target) {
            (
                ExpectedRelationship::Reject,
                Some(ReasonCode::MissingDependency),
            )
        } else if assessment.is_none()
            || planned_index
                .get(&edge.source)
                .is_some_and(|item| item.expected == ExpectedRecord::DecisionRequired)
            || planned_index
                .get(&edge.target)
                .is_some_and(|item| item.expected == ExpectedRecord::DecisionRequired)
        {
            (ExpectedRelationship::DecisionRequired, None)
        } else {
            (ExpectedRelationship::Link, None)
        };
        relationships.push(PlannedRelationship {
            source: edge.source.clone(),
            target: edge.target.clone(),
            field: edge.field.clone(),
            expected,
            reason: why,
            people,
        });
    }
    Ok(BridgeOutput {
        outcomes: PlannedOutcomes {
            records: planned,
            relationships,
        },
        issues,
    })
}

fn clinical(kind: RecordKind) -> bool {
    matches!(
        kind,
        RecordKind::Schedule
            | RecordKind::Assignment
            | RecordKind::Dose
            | RecordKind::Pause
            | RecordKind::Occurrence
            | RecordKind::HealthEvent
            | RecordKind::NotificationPreference
    )
}

fn dependency_source(kind: RecordKind) -> bool {
    clinical(kind) || matches!(kind, RecordKind::Medication | RecordKind::DosageOption)
}

fn record_alias_field(
    fields: &mut Vec<SourceField>,
    name: &str,
    source: Option<&str>,
    destination: Option<&str>,
) {
    let mut seen = false;
    fields.retain_mut(|field| {
        if field.field != name {
            return true;
        }
        if seen {
            return false;
        }
        seen = true;
        field.outcome = if source == destination {
            FieldOutcome::Preserved
        } else {
            FieldOutcome::Changed
        };
        field.destination_value = destination.map(str::to_owned);
        field.reason = (source != destination).then_some(ReasonCode::UnsupportedField);
        true
    });
    if !seen {
        fields.push(SourceField {
            field: name.into(),
            source_value: source.map_or_else(
                || "null".into(),
                |value| serde_json::Value::String(value.into()).to_string(),
            ),
            outcome: if source == destination {
                FieldOutcome::Preserved
            } else {
                FieldOutcome::Changed
            },
            destination_value: destination.map(str::to_owned),
            reason: (source != destination).then_some(ReasonCode::UnsupportedField),
        });
    }
}

fn ownership(
    inventory: &SourceInventory,
    attachment_parents: &[AttachmentParent],
    records: &HashMap<SourceIdentity, &super::contracts::SourceRecord>,
) -> Result<HashMap<SourceIdentity, Vec<SourceIdentity>>, BridgeError> {
    // Follow only subject ownership and clinical dependencies. Actor references never carry ownership.
    let mut edges: HashMap<SourceIdentity, Vec<SourceIdentity>> = HashMap::new();
    let mut unique = HashSet::new();
    for edge in &inventory.relationships {
        if !unique.insert((edge.source.clone(), edge.target.clone(), edge.field.clone())) {
            return Err(BridgeError::DuplicateRelationship);
        }
        if actor(&edge.field)
            || edge.kind == RelationshipKind::PersonLocation
            || edge.source.kind == RecordKind::Attachment
            || edge.target.kind == RecordKind::Attachment
        {
            continue;
        }
        let subject = edge.field == "person_portable_id" && edge.target.kind == RecordKind::Person;
        let history = edge.field == "source_portable_id"
            && matches!(
                edge.source.kind,
                RecordKind::Dose | RecordKind::Pause | RecordKind::Occurrence
            );
        if subject || history {
            edges
                .entry(edge.target.clone())
                .or_default()
                .push(edge.source.clone());
        } else if matches!(
            edge.source.kind,
            RecordKind::Schedule
                | RecordKind::Assignment
                | RecordKind::Dose
                | RecordKind::Occurrence
                | RecordKind::HealthEvent
                | RecordKind::Medication
                | RecordKind::DosageOption
        ) && matches!(
            edge.target.kind,
            RecordKind::Medication | RecordKind::Location | RecordKind::DosageOption
        ) {
            edges
                .entry(edge.source.clone())
                .or_default()
                .push(edge.target.clone());
        }
    }
    for link in attachment_parents {
        if link.attachment.kind != RecordKind::Attachment
            || !records.contains_key(&link.attachment)
            || !records.contains_key(&link.parent)
        {
            return Err(BridgeError::UnknownAttachmentParent);
        }
    }
    let mut owners: HashMap<SourceIdentity, Vec<SourceIdentity>> = HashMap::new();
    let mut queue = VecDeque::new();
    for record in &inventory.records {
        if record.identity.kind == RecordKind::Person {
            owners.insert(record.identity.clone(), vec![record.identity.clone()]);
            queue.push_back((record.identity.clone(), record.identity.clone()));
        }
    }
    while let Some((node, person)) = queue.pop_front() {
        if let Some(next) = edges.get(&node) {
            for target in next {
                let list = owners.entry(target.clone()).or_default();
                if !list.contains(&person) {
                    list.push(person.clone());
                    queue.push_back((target.clone(), person.clone()));
                }
            }
        }
    }
    Ok(owners)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::data_imports::contracts::{
        Omission, RecordDetails, SourceRecord, SourceRelationship, SourceSystem, SourceTimeZone,
    };

    fn id(kind: RecordKind, value: &str) -> SourceIdentity {
        SourceIdentity {
            source: SourceSystem::MedTracker05,
            instance: "instance".into(),
            kind,
            id: value.into(),
        }
    }

    #[test]
    fn ownership_follows_subjects_and_used_dependencies_not_actors() {
        let p = id(RecordKind::Person, "patient");
        let q = id(RecordKind::Person, "other");
        let carer = id(RecordKind::Person, "carer");
        let schedule = id(RecordKind::Schedule, "schedule");
        let assignment = id(RecordKind::Assignment, "prn");
        let dose = id(RecordKind::Dose, "dose");
        let pause = id(RecordKind::Pause, "pause");
        let medication = id(RecordKind::Medication, "medicine");
        let used = id(RecordKind::DosageOption, "used");
        let unused = id(RecordKind::DosageOption, "unused");
        let attachment = id(RecordKind::Attachment, "attachment");
        let all = [
            p.clone(),
            q.clone(),
            carer.clone(),
            schedule.clone(),
            assignment.clone(),
            dose.clone(),
            pause.clone(),
            medication.clone(),
            used.clone(),
            unused.clone(),
            attachment.clone(),
        ];
        let mut inventory = SourceInventory {
            records: all
                .into_iter()
                .map(|identity| SourceRecord {
                    identity,
                    details: RecordDetails::default(),
                })
                .collect(),
            relationships: vec![],
            omissions: vec![],
        };
        for (source, target, field) in [
            (&schedule, &p, "person_portable_id"),
            (&assignment, &q, "person_portable_id"),
            (&schedule, &medication, "medication_portable_id"),
            (&assignment, &medication, "medication_portable_id"),
            (&schedule, &used, "source_dosage_option_portable_id"),
            (&unused, &medication, "medication_portable_id"),
            (&dose, &assignment, "source_portable_id"),
            (&pause, &schedule, "source_portable_id"),
            (&pause, &carer, "recorded_by_person_portable_id"),
        ] {
            inventory.relationships.push(SourceRelationship {
                source: source.clone(),
                target: target.clone(),
                field: field.into(),
                kind: RelationshipKind::Other,
                reason: None,
            });
        }
        let records = inventory
            .records
            .iter()
            .map(|record| (record.identity.clone(), record))
            .collect();
        let owners = ownership(
            &inventory,
            &[AttachmentParent {
                attachment: attachment.clone(),
                parent: pause.clone(),
            }],
            &records,
        )
        .unwrap();
        assert_eq!(owners[&pause], vec![p.clone()]);
        assert_eq!(owners[&dose], vec![q.clone()]);
        assert!(!owners.contains_key(&attachment));
        assert_eq!(owners[&used], vec![p.clone()]);
        assert!(!owners.contains_key(&unused));
        assert_eq!(owners[&medication].len(), 2);
        assert!(owners[&medication].contains(&p));
        assert!(owners[&medication].contains(&q));
        assert!(!owners[&pause].contains(&carer));
    }

    #[test]
    fn diagnostics_are_typed_and_actor_fields_are_distinct() {
        assert_eq!(
            reason(AssessmentReason::MissingDependency),
            ReasonCode::MissingDependency
        );
        assert_eq!(
            reason(AssessmentReason::IncompatiblePerson),
            ReasonCode::IdentityConflict
        );
        assert!(actor("recorded_by_person_portable_id"));
        assert!(actor("resumed_by_person_portable_id"));
        assert!(!actor("person_portable_id"));
    }

    fn fixture() -> SourceInventory {
        let mut inventory = SourceInventory {
            records: Vec::new(),
            relationships: Vec::new(),
            omissions: Vec::new(),
        };
        for (kind, name) in [
            (RecordKind::Person, "p"),
            (RecordKind::Person, "q"),
            (RecordKind::Person, "carer"),
            (RecordKind::Schedule, "s"),
            (RecordKind::Assignment, "a"),
            (RecordKind::Dose, "d"),
            (RecordKind::Pause, "pause"),
            (RecordKind::Pause, "pause_missing"),
            (RecordKind::Occurrence, "occ"),
            (RecordKind::Medication, "m"),
            (RecordKind::DosageOption, "used"),
            (RecordKind::DosageOption, "unused"),
            (RecordKind::Location, "location"),
            (RecordKind::Attachment, "attachment"),
        ] {
            inventory.records.push(SourceRecord {
                identity: id(kind, name),
                details: RecordDetails::default(),
            });
        }
        for (sk, s, tk, t, field, relation, why) in [
            (
                RecordKind::Schedule,
                "s",
                RecordKind::Person,
                "p",
                "person_portable_id",
                RelationshipKind::SchedulePerson,
                None,
            ),
            (
                RecordKind::Assignment,
                "a",
                RecordKind::Person,
                "q",
                "person_portable_id",
                RelationshipKind::AssignmentPerson,
                None,
            ),
            (
                RecordKind::Schedule,
                "s",
                RecordKind::Medication,
                "m",
                "medication_portable_id",
                RelationshipKind::ScheduleMedication,
                None,
            ),
            (
                RecordKind::Assignment,
                "a",
                RecordKind::Medication,
                "m",
                "medication_portable_id",
                RelationshipKind::AssignmentMedication,
                None,
            ),
            (
                RecordKind::Schedule,
                "s",
                RecordKind::DosageOption,
                "used",
                "source_dosage_option_portable_id",
                RelationshipKind::Other,
                None,
            ),
            (
                RecordKind::DosageOption,
                "unused",
                RecordKind::Medication,
                "m",
                "medication_portable_id",
                RelationshipKind::Other,
                None,
            ),
            (
                RecordKind::Dose,
                "d",
                RecordKind::Assignment,
                "a",
                "source_portable_id",
                RelationshipKind::DoseSource,
                None,
            ),
            (
                RecordKind::Pause,
                "pause",
                RecordKind::Schedule,
                "s",
                "source_portable_id",
                RelationshipKind::DoseSource,
                None,
            ),
            (
                RecordKind::Pause,
                "pause_missing",
                RecordKind::Assignment,
                "a",
                "source_portable_id",
                RelationshipKind::DoseSource,
                None,
            ),
            (
                RecordKind::Occurrence,
                "occ",
                RecordKind::Schedule,
                "s",
                "source_portable_id",
                RelationshipKind::DoseSource,
                None,
            ),
            (
                RecordKind::Pause,
                "pause",
                RecordKind::Person,
                "carer",
                "recorded_by_person_portable_id",
                RelationshipKind::Other,
                None,
            ),
            (
                RecordKind::Pause,
                "pause",
                RecordKind::Person,
                "carer",
                "resumed_by_person_portable_id",
                RelationshipKind::Other,
                None,
            ),
            (
                RecordKind::Pause,
                "pause_missing",
                RecordKind::Person,
                "missing",
                "recorded_by_person_portable_id",
                RelationshipKind::Other,
                Some(ReasonCode::HistoricalActorUnavailable),
            ),
            (
                RecordKind::Person,
                "p",
                RecordKind::Location,
                "location",
                "location_portable_ids",
                RelationshipKind::PersonLocation,
                None,
            ),
        ] {
            inventory.relationships.push(SourceRelationship {
                source: id(sk, s),
                target: id(tk, t),
                field: field.into(),
                kind: relation,
                reason: why,
            });
        }
        inventory
    }

    fn assessment_rows() -> Vec<PlannedRow> {
        [
            (SourceCollection::People, "p"),
            (SourceCollection::People, "q"),
            (SourceCollection::People, "carer"),
            (SourceCollection::Schedules, "s"),
            (SourceCollection::PersonMedications, "a"),
            (SourceCollection::MedicationTakes, "d"),
            (SourceCollection::MedicationPausePeriods, "pause"),
            (SourceCollection::MedicationPausePeriods, "pause_missing"),
            (SourceCollection::DoseOccurrences, "occ"),
            (SourceCollection::Medications, "m"),
            (SourceCollection::DosageOptions, "used"),
            (SourceCollection::DosageOptions, "unused"),
            (SourceCollection::Locations, "location"),
        ]
        .into_iter()
        .map(|(collection, portable_id)| PlannedRow {
            collection,
            portable_id: portable_id.into(),
            action: PlannedAction::Create,
        })
        .collect()
    }

    fn assessment(
        rows: Vec<PlannedRow>,
        issues: Vec<AssessmentIssue>,
        aliases: Vec<ResolvedAlias>,
    ) -> ImportAssessment {
        use crate::models::{
            care::portable_imports::planning::DestinationExpectation,
            entities::membership,
            ids::{AccountId, HouseholdId},
        };
        ImportAssessment {
            rows,
            issues,
            aliases,
            expectation: DestinationExpectation {
                household_id: HouseholdId::from_database(1),
                actor_id: AccountId::from_database(1),
                actor_membership: membership::Model {
                    id: 1,
                    created_at: Default::default(),
                    household_id: 1,
                    joined_at: Default::default(),
                    permissions_version: 0,
                    person_id: 1,
                    revoked_at: None,
                    role: "owner".into(),
                    status: "active".into(),
                    updated_at: Default::default(),
                },
                aliases: vec![],
                slots: vec![],
            },
            source_compatible: true,
            has_blockers: false,
        }
    }

    fn result<'a>(output: &'a BridgeOutput, kind: RecordKind, name: &str) -> &'a PlannedRecord {
        output
            .outcomes
            .records
            .iter()
            .find(|row| row.source == id(kind, name))
            .unwrap()
    }

    fn edge_result<'a>(
        output: &'a BridgeOutput,
        source: &str,
        field: &str,
    ) -> &'a PlannedRelationship {
        output
            .outcomes
            .relationships
            .iter()
            .find(|row| row.source.id == source && row.field == field)
            .unwrap()
    }

    #[test]
    fn assessed_bridge_covers_inventory_edges_omissions_and_subjects() {
        let mut inventory = fixture();
        inventory.omissions.push(Omission {
            item: id(RecordKind::Person, "p"),
            field: Some("location_portable_ids:location".into()),
            reason: ReasonCode::ObsoletePersonLocation,
            acknowledgement_id: "exact-acknowledgement".into(),
            acknowledged: false,
        });
        let rows = assessment_rows();
        let output = bridge(
            &inventory,
            &assessment(rows, vec![], vec![]),
            &[],
            &[AttachmentParent {
                attachment: id(RecordKind::Attachment, "attachment"),
                parent: id(RecordKind::Schedule, "s"),
            }],
        )
        .unwrap();
        assert_eq!(output.outcomes.records.len(), inventory.records.len());
        assert_eq!(
            output.outcomes.relationships.len(),
            inventory.relationships.len()
        );
        for item in &inventory.records {
            assert_eq!(
                output
                    .outcomes
                    .records
                    .iter()
                    .filter(|p| p.source == item.identity)
                    .count(),
                1
            );
        }
        for edge in &inventory.relationships {
            assert_eq!(
                output
                    .outcomes
                    .relationships
                    .iter()
                    .filter(|p| p.source == edge.source
                        && p.target == edge.target
                        && p.field == edge.field)
                    .count(),
                1
            );
        }
        assert_eq!(result(&output, RecordKind::Medication, "m").people.len(), 2);
        assert!(
            result(&output, RecordKind::DosageOption, "unused")
                .people
                .is_empty()
        );
        assert_eq!(
            result(&output, RecordKind::Dose, "d").people,
            vec![id(RecordKind::Person, "q")]
        );
        assert_eq!(
            result(&output, RecordKind::Pause, "pause").people,
            vec![id(RecordKind::Person, "p")]
        );
        assert_eq!(
            result(&output, RecordKind::Attachment, "attachment").expected,
            ExpectedRecord::Skip
        );
        assert!(
            result(&output, RecordKind::Attachment, "attachment")
                .people
                .is_empty()
        );
        assert_eq!(
            edge_result(&output, "pause", "recorded_by_person_portable_id").expected,
            ExpectedRelationship::Omit
        );
        assert_eq!(
            edge_result(&output, "pause", "resumed_by_person_portable_id").reason,
            Some(ReasonCode::UnsupportedRelationship)
        );
        assert_eq!(
            edge_result(&output, "pause_missing", "recorded_by_person_portable_id").reason,
            Some(ReasonCode::HistoricalActorUnavailable)
        );
        assert_eq!(
            edge_result(&output, "p", "location_portable_ids").reason,
            Some(ReasonCode::ObsoletePersonLocation)
        );
        assert_eq!(
            edge_result(&output, "s", "medication_portable_id").people,
            vec![id(RecordKind::Person, "p")]
        );
        let expected_creates = u64::try_from(
            output
                .outcomes
                .records
                .iter()
                .filter(|item| item.expected == ExpectedRecord::Create)
                .count(),
        )
        .unwrap();
        let report = super::super::projection::project(
            super::super::projection::ProjectionMeta {
                id: "preview".into(),
                source: SourceSystem::MedTracker05,
                source_sha256: "digest".into(),
                timezone: SourceTimeZone::parse("UTC").unwrap(),
                created_at: chrono::Utc::now(),
            },
            &inventory,
            output.outcomes,
            &[],
        )
        .unwrap();
        assert_eq!(report.totals.expected_create, expected_creates);
        assert_eq!(report.totals.omissions, 1);
        assert_eq!(
            report
                .people
                .iter()
                .flat_map(|person| &person.omissions)
                .next()
                .unwrap()
                .acknowledgement_id,
            "exact-acknowledgement"
        );
    }

    #[test]
    fn identified_blocker_does_not_reject_other_patient_or_shared_medicine() {
        let inventory = fixture();
        let rows = assessment_rows();
        let findings = [PersonFinding {
            person: id(RecordKind::Person, "p"),
            field: Some("date_of_birth".into()),
            reason: ReasonCode::IdentityConflict,
            detail: "Date of birth differs".into(),
        }];
        let output = bridge(
            &inventory,
            &assessment(rows, vec![], vec![]),
            &findings,
            &[],
        )
        .unwrap();
        for (kind, name) in [
            (RecordKind::Person, "p"),
            (RecordKind::Schedule, "s"),
            (RecordKind::Pause, "pause"),
            (RecordKind::Occurrence, "occ"),
        ] {
            assert_eq!(result(&output, kind, name).expected, ExpectedRecord::Reject);
        }
        for (kind, name) in [
            (RecordKind::Person, "q"),
            (RecordKind::Assignment, "a"),
            (RecordKind::Dose, "d"),
            (RecordKind::Medication, "m"),
        ] {
            assert_eq!(result(&output, kind, name).expected, ExpectedRecord::Create);
        }
        assert_eq!(output.issues.len(), 1);
        assert_eq!(output.issues[0].source, Some(id(RecordKind::Person, "p")));
        assert_eq!(
            edge_result(&output, "a", "medication_portable_id").expected,
            ExpectedRelationship::Link
        );
    }

    #[test]
    fn failed_identity_before_assessment_preserves_every_real_row() {
        let inventory = fixture();
        let finding = PersonFinding {
            person: id(RecordKind::Person, "p"),
            field: Some("has_capacity".into()),
            reason: ReasonCode::IdentityConflict,
            detail: "Capacity differs".into(),
        };
        let global = BridgeIssue {
            source: None,
            field: None,
            reason: ReasonCode::IdentityConflict,
            detail: "Assessment could not be performed".into(),
        };
        let output = blocked_inventory(&inventory, &[finding], &[], global).unwrap();
        assert_eq!(output.outcomes.records.len(), inventory.records.len());
        assert_eq!(
            output.outcomes.relationships.len(),
            inventory.relationships.len()
        );
        assert_eq!(
            result(&output, RecordKind::Person, "p").expected,
            ExpectedRecord::Reject
        );
        assert_eq!(
            result(&output, RecordKind::Pause, "pause").expected,
            ExpectedRecord::Reject
        );
        assert_eq!(
            result(&output, RecordKind::Person, "q").expected,
            ExpectedRecord::DecisionRequired
        );
        assert_eq!(
            result(&output, RecordKind::Dose, "d").expected,
            ExpectedRecord::DecisionRequired
        );
        assert_eq!(
            result(&output, RecordKind::Medication, "m").expected,
            ExpectedRecord::DecisionRequired
        );
        assert_eq!(
            edge_result(&output, "a", "medication_portable_id").expected,
            ExpectedRelationship::DecisionRequired
        );
        assert_eq!(
            result(&output, RecordKind::Attachment, "attachment").expected,
            ExpectedRecord::Skip
        );
        assert!(
            result(&output, RecordKind::Attachment, "attachment")
                .people
                .is_empty()
        );
        assert_eq!(output.issues.len(), 2);
        assert!(output.issues.iter().any(|issue| issue.source.is_none()));
    }

    #[test]
    fn alias_and_typed_dependency_issue_preserve_native_destinations_and_propagate() {
        let inventory = fixture();
        let mut rows = assessment_rows();
        rows.iter_mut()
            .find(|row| row.collection == SourceCollection::People && row.portable_id == "q")
            .unwrap()
            .action = PlannedAction::Match;
        let aliases = [ResolvedAlias {
            source_person_portable_id: "q".into(),
            destination_person_id: crate::models::ids::PersonId::from_database(42),
            destination_person_portable_id: "native".into(),
            profile_differences: vec![ProfileDifference::Name {
                source: "Old".into(),
                destination: "Current".into(),
            }],
        }];
        let issues = [
            AssessmentIssue {
                collection: Some(SourceCollection::Schedules),
                portable_id: Some("s".into()),
                field: Some("medication_portable_id".into()),
                reason: AssessmentReason::MissingDependency,
                detail: "Missing medicine".into(),
            },
            AssessmentIssue {
                collection: None,
                portable_id: None,
                field: None,
                reason: AssessmentReason::InsufficientAuthority,
                detail: "Global restriction".into(),
            },
        ];
        let output = bridge(
            &inventory,
            &assessment(rows, Vec::from(issues), Vec::from(aliases)),
            &[],
            &[],
        )
        .unwrap();
        let person = result(&output, RecordKind::Person, "q");
        assert_eq!(person.expected, ExpectedRecord::Match);
        assert_eq!(
            person.destination.as_ref().map(|record| &record.id),
            Some(&DestinationId::Person(PersonId(42)))
        );
        let name = person
            .details
            .source_fields
            .iter()
            .find(|field| field.field == "name")
            .unwrap();
        assert_eq!(name.outcome, FieldOutcome::Changed);
        assert_eq!(name.destination_value.as_deref(), Some("Current"));
        assert_eq!(
            result(&output, RecordKind::Schedule, "s").reason,
            Some(ReasonCode::MissingDependency)
        );
        assert_eq!(
            result(&output, RecordKind::Pause, "pause").reason,
            Some(ReasonCode::MissingDependency)
        );
        assert_eq!(
            result(&output, RecordKind::Dose, "d").expected,
            ExpectedRecord::Create
        );
        assert_eq!(output.issues[0].source, Some(id(RecordKind::Schedule, "s")));
        assert!(output.issues[1].source.is_none());
    }

    #[test]
    fn missing_clinical_dependency_blocks_its_history() {
        let mut inventory = fixture();
        inventory.relationships.push(SourceRelationship {
            kind: RelationshipKind::ScheduleMedication,
            field: "additional_medication".into(),
            source: id(RecordKind::Schedule, "s"),
            target: id(RecordKind::Medication, "missing"),
            reason: None,
        });
        let output = bridge(
            &inventory,
            &assessment(assessment_rows(), vec![], vec![]),
            &[],
            &[],
        )
        .unwrap();
        assert_eq!(
            result(&output, RecordKind::Schedule, "s").reason,
            Some(ReasonCode::MissingDependency)
        );
        assert_eq!(
            result(&output, RecordKind::Pause, "pause").reason,
            Some(ReasonCode::MissingDependency)
        );
        assert_eq!(
            result(&output, RecordKind::Assignment, "a").expected,
            ExpectedRecord::Create
        );
        assert_eq!(
            edge_result(&output, "s", "additional_medication").expected,
            ExpectedRelationship::Reject
        );
        assert!(
            output
                .issues
                .iter()
                .any(|issue| issue.source == Some(id(RecordKind::Schedule, "s"))
                    && issue.field.as_deref() == Some("additional_medication"))
        );
    }

    #[test]
    fn unproved_and_conflicting_attachment_claims_leave_omissions_global() {
        let mut inventory = fixture();
        inventory.omissions.push(Omission {
            item: id(RecordKind::Attachment, "attachment"),
            field: None,
            reason: ReasonCode::AttachmentUnsupported,
            acknowledgement_id: "attachment-ack".into(),
            acknowledged: false,
        });
        for claims in [
            vec![AttachmentParent {
                attachment: id(RecordKind::Attachment, "attachment"),
                parent: id(RecordKind::Assignment, "a"),
            }],
            vec![
                AttachmentParent {
                    attachment: id(RecordKind::Attachment, "attachment"),
                    parent: id(RecordKind::Assignment, "a"),
                },
                AttachmentParent {
                    attachment: id(RecordKind::Attachment, "attachment"),
                    parent: id(RecordKind::Schedule, "s"),
                },
            ],
        ] {
            let output = bridge(
                &inventory,
                &assessment(assessment_rows(), vec![], vec![]),
                &[],
                &claims,
            )
            .unwrap();
            assert!(
                result(&output, RecordKind::Attachment, "attachment")
                    .people
                    .is_empty()
            );
            let report = super::super::projection::project(
                super::super::projection::ProjectionMeta {
                    id: "preview".into(),
                    source: SourceSystem::MedTracker05,
                    source_sha256: "digest".into(),
                    timezone: SourceTimeZone::parse("UTC").unwrap(),
                    created_at: chrono::Utc::now(),
                },
                &inventory,
                output.outcomes,
                &[],
            )
            .unwrap();
            assert_eq!(report.omissions.len(), 1);
            assert_eq!(report.omissions[0].acknowledgement_id, "attachment-ack");
            assert!(
                report
                    .people
                    .iter()
                    .all(|person| person.omissions.is_empty())
            );
        }
    }

    #[test]
    fn alias_replaces_archive_field_outcomes_and_preserves_null_email() {
        let mut inventory = fixture();
        let source = inventory
            .records
            .iter_mut()
            .find(|record| record.identity == id(RecordKind::Person, "q"))
            .unwrap();
        source.details.source_fields = [
            ("portable_id", "\"q\""),
            ("name", "\"Old\""),
            ("email", "null"),
        ]
        .into_iter()
        .map(|(field, value)| SourceField {
            field: field.into(),
            source_value: value.into(),
            outcome: FieldOutcome::Preserved,
            destination_value: None,
            reason: None,
        })
        .collect();
        let mut rows = assessment_rows();
        rows.iter_mut()
            .find(|row| row.collection == SourceCollection::People && row.portable_id == "q")
            .unwrap()
            .action = PlannedAction::Match;
        let aliases = vec![ResolvedAlias {
            source_person_portable_id: "q".into(),
            destination_person_id: crate::models::ids::PersonId::from_database(42),
            destination_person_portable_id: "existing-id".into(),
            profile_differences: vec![
                ProfileDifference::Name {
                    source: "Old".into(),
                    destination: "Current".into(),
                },
                ProfileDifference::Email {
                    source: None,
                    destination: Some("current@example.test".into()),
                },
            ],
        }];
        let output = bridge(&inventory, &assessment(rows, vec![], aliases), &[], &[]).unwrap();
        let fields = &result(&output, RecordKind::Person, "q")
            .details
            .source_fields;
        for (key, raw, destination) in [
            ("portable_id", "\"q\"", "existing-id"),
            ("name", "\"Old\"", "Current"),
            ("email", "null", "current@example.test"),
        ] {
            let matches: Vec<_> = fields.iter().filter(|field| field.field == key).collect();
            assert_eq!(matches.len(), 1);
            assert_eq!(matches[0].source_value, raw);
            assert_eq!(matches[0].outcome, FieldOutcome::Changed);
            assert_eq!(matches[0].destination_value.as_deref(), Some(destination));
        }
    }

    #[test]
    fn rejected_inventory_dependencies_block_dependants_not_people() {
        let mut inventory = fixture();
        for (source, target, field, kind) in [
            (
                id(RecordKind::Medication, "m"),
                id(RecordKind::Location, "location"),
                "location_portable_id",
                RelationshipKind::MedicationLocation,
            ),
            (
                id(RecordKind::DosageOption, "used"),
                id(RecordKind::Medication, "m"),
                "medication_portable_id",
                RelationshipKind::Other,
            ),
        ] {
            inventory.relationships.push(SourceRelationship {
                source,
                target,
                field: field.into(),
                kind,
                reason: None,
            });
        }
        for (collection, rejected, affected) in [
            (
                SourceCollection::Locations,
                "location",
                vec![
                    (RecordKind::Medication, "m"),
                    (RecordKind::DosageOption, "used"),
                    (RecordKind::Schedule, "s"),
                    (RecordKind::Pause, "pause"),
                    (RecordKind::Assignment, "a"),
                    (RecordKind::Dose, "d"),
                ],
            ),
            (
                SourceCollection::Medications,
                "m",
                vec![
                    (RecordKind::DosageOption, "used"),
                    (RecordKind::DosageOption, "unused"),
                    (RecordKind::Schedule, "s"),
                    (RecordKind::Pause, "pause"),
                    (RecordKind::Assignment, "a"),
                    (RecordKind::Dose, "d"),
                ],
            ),
        ] {
            let issues = vec![AssessmentIssue {
                collection: Some(collection),
                portable_id: Some(rejected.into()),
                field: None,
                reason: AssessmentReason::MissingDependency,
                detail: "Dependency rejected".into(),
            }];
            let output = bridge(
                &inventory,
                &assessment(assessment_rows(), issues, vec![]),
                &[],
                &[],
            )
            .unwrap();
            for (kind, name) in affected {
                assert_eq!(
                    result(&output, kind, name).reason,
                    Some(ReasonCode::MissingDependency)
                );
            }
            assert_eq!(
                result(&output, RecordKind::Person, "p").expected,
                ExpectedRecord::Create
            );
            assert_eq!(
                result(&output, RecordKind::Person, "q").expected,
                ExpectedRecord::Create
            );
        }
        let finding = PersonFinding {
            person: id(RecordKind::Person, "p"),
            field: None,
            reason: ReasonCode::ConsentRequired,
            detail: "Consent required".into(),
        };
        let output = bridge(
            &inventory,
            &assessment(assessment_rows(), vec![], vec![]),
            &[finding],
            &[],
        )
        .unwrap();
        for (kind, name) in [
            (RecordKind::Medication, "m"),
            (RecordKind::Location, "location"),
            (RecordKind::DosageOption, "used"),
            (RecordKind::Person, "q"),
            (RecordKind::Assignment, "a"),
            (RecordKind::Dose, "d"),
        ] {
            assert_eq!(result(&output, kind, name).expected, ExpectedRecord::Create);
        }
    }

    #[test]
    fn health_preferences_and_taken_from_storage_follow_actual_subjects() {
        let mut inventory = fixture();
        for (kind, name) in [
            (RecordKind::HealthEvent, "health"),
            (RecordKind::NotificationPreference, "preference"),
            (RecordKind::Location, "taken-from"),
        ] {
            inventory.records.push(SourceRecord {
                identity: id(kind, name),
                details: RecordDetails::default(),
            });
        }
        for (source, target, field, kind) in [
            (
                id(RecordKind::HealthEvent, "health"),
                id(RecordKind::Person, "p"),
                "person_portable_id",
                RelationshipKind::Other,
            ),
            (
                id(RecordKind::HealthEvent, "health"),
                id(RecordKind::Medication, "m"),
                "medication_portable_ids",
                RelationshipKind::HealthEventMedication,
            ),
            (
                id(RecordKind::NotificationPreference, "preference"),
                id(RecordKind::Person, "q"),
                "person_portable_id",
                RelationshipKind::Other,
            ),
            (
                id(RecordKind::Medication, "m"),
                id(RecordKind::Location, "location"),
                "location_portable_id",
                RelationshipKind::MedicationLocation,
            ),
            (
                id(RecordKind::Dose, "d"),
                id(RecordKind::Location, "taken-from"),
                "taken_from_location_portable_id",
                RelationshipKind::DoseTakenFromLocation,
            ),
        ] {
            inventory.relationships.push(SourceRelationship {
                source,
                target,
                field: field.into(),
                kind,
                reason: None,
            });
        }
        let mut rows = assessment_rows();
        for (collection, portable_id) in [
            (SourceCollection::HealthEvents, "health"),
            (SourceCollection::NotificationPreferences, "preference"),
            (SourceCollection::Locations, "taken-from"),
        ] {
            rows.push(PlannedRow {
                collection,
                portable_id: portable_id.into(),
                action: PlannedAction::Create,
            });
        }
        let output = bridge(&inventory, &assessment(rows, vec![], vec![]), &[], &[]).unwrap();
        assert_eq!(
            result(&output, RecordKind::HealthEvent, "health").people,
            vec![id(RecordKind::Person, "p")]
        );
        assert_eq!(
            result(&output, RecordKind::NotificationPreference, "preference").people,
            vec![id(RecordKind::Person, "q")]
        );
        assert_eq!(
            result(&output, RecordKind::Location, "taken-from").people,
            vec![id(RecordKind::Person, "q")]
        );
        assert_eq!(
            result(&output, RecordKind::Location, "location")
                .people
                .len(),
            2
        );
        assert_eq!(
            edge_result(&output, "health", "medication_portable_ids").people,
            vec![id(RecordKind::Person, "p")]
        );
        assert_eq!(
            edge_result(&output, "d", "taken_from_location_portable_id").people,
            vec![id(RecordKind::Person, "q")]
        );
    }

    #[test]
    fn alias_with_absent_destination_email_retains_source_json_null_distinction() {
        let mut inventory = fixture();
        let person = inventory
            .records
            .iter_mut()
            .find(|record| record.identity == id(RecordKind::Person, "q"))
            .unwrap();
        person.details.source_fields.push(SourceField {
            field: "email".into(),
            source_value: "\"source@example.test\"".into(),
            outcome: FieldOutcome::Preserved,
            destination_value: None,
            reason: None,
        });
        let mut rows = assessment_rows();
        rows.iter_mut()
            .find(|row| row.collection == SourceCollection::People && row.portable_id == "q")
            .unwrap()
            .action = PlannedAction::Match;
        let alias = ResolvedAlias {
            source_person_portable_id: "q".into(),
            destination_person_id: crate::models::ids::PersonId::from_database(42),
            destination_person_portable_id: "q".into(),
            profile_differences: vec![ProfileDifference::Email {
                source: Some("source@example.test".into()),
                destination: None,
            }],
        };
        let output = bridge(&inventory, &assessment(rows, vec![], vec![alias]), &[], &[]).unwrap();
        let fields = &result(&output, RecordKind::Person, "q")
            .details
            .source_fields;
        let email = fields.iter().find(|field| field.field == "email").unwrap();
        assert_eq!(email.source_value, "\"source@example.test\"");
        assert_eq!(email.outcome, FieldOutcome::Changed);
        assert!(email.destination_value.is_none());
        let portable = fields
            .iter()
            .find(|field| field.field == "portable_id")
            .unwrap();
        assert_eq!(portable.outcome, FieldOutcome::Preserved);
        assert_eq!(portable.destination_value.as_deref(), Some("q"));
    }
}
