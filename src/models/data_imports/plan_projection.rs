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
    ProfileDifference,
    planning::{AssessmentReason, ImportAssessment, PlannedAction, SourceCollection},
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
    for row in &assessment.rows {
        let Some(identity) = by_key.get(&(kind(row.collection), row.portable_id.as_str())) else {
            return Err(BridgeError::UnknownPlannerRow);
        };
        if rows.insert((*identity).clone(), row.action).is_some() {
            return Err(BridgeError::DuplicatePlannerRow);
        }
    }
    let mut issues = Vec::new();
    let mut row_reasons = HashMap::new();
    for issue in &assessment.issues {
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
        .aliases
        .iter()
        .map(|alias| (alias.source_person_portable_id.as_str(), alias))
        .collect();
    let mut planned = Vec::with_capacity(inventory.records.len());
    for record in &inventory.records {
        let identity = &record.identity;
        let people = owners.get(identity).cloned().unwrap_or_default();
        let mut details = record.details.clone();
        let alias = (identity.kind == RecordKind::Person)
            .then(|| aliases.get(identity.id.as_str()))
            .flatten();
        if let Some(alias) = alias {
            for difference in &alias.profile_differences {
                let (field, source, destination) = match difference {
                    ProfileDifference::Name {
                        source,
                        destination,
                    } => ("name", source.clone(), Some(destination.clone())),
                    ProfileDifference::Email {
                        source,
                        destination,
                    } => (
                        "email",
                        source.clone().unwrap_or_default(),
                        destination.clone(),
                    ),
                };
                details.source_fields.push(SourceField {
                    field: field.into(),
                    source_value: source,
                    outcome: FieldOutcome::Changed,
                    destination_value: destination,
                    reason: Some(ReasonCode::UnsupportedField),
                });
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
        let (expected, why) = if let Some(reason) = blocker {
            (ExpectedRecord::Reject, Some(reason))
        } else if identity.kind == RecordKind::Attachment {
            (
                ExpectedRecord::Skip,
                Some(ReasonCode::AttachmentUnsupported),
            )
        } else {
            match rows.get(identity) {
                Some(PlannedAction::Create) => (ExpectedRecord::Create, None),
                Some(PlannedAction::Match) => (ExpectedRecord::Match, None),
                Some(PlannedAction::Reject) => {
                    (ExpectedRecord::Reject, Some(ReasonCode::SourceInvalid))
                }
                None => {
                    issues.push(BridgeIssue {
                        source: Some(identity.clone()),
                        field: None,
                        reason: ReasonCode::MissingDependency,
                        detail: "No planner row for source record".into(),
                    });
                    (ExpectedRecord::Reject, Some(ReasonCode::MissingDependency))
                }
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
        if actor(&edge.field) || edge.kind == RelationshipKind::PersonLocation {
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
        edges
            .entry(link.parent.clone())
            .or_default()
            .push(link.attachment.clone());
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
        RecordDetails, SourceRecord, SourceRelationship, SourceSystem,
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
        assert_eq!(owners[&attachment], vec![p.clone()]);
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
}
