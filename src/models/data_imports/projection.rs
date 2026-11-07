use std::collections::{HashMap, HashSet};

use super::contracts::{
    ActualRecord, DestinationRecord, ExpectedRecord, ExpectedRelationship, ImportReport,
    ImportState, Omission, PersonReport, ReasonCode, RecordDetails, RecordKind, RecordResult,
    RelationshipResult, ReportTotals, SourceIdentity, SourceInventory, SourceSystem,
    SourceTimeZone,
};
use crate::models::errors::OperationError;
use chrono::{DateTime, Utc};

#[derive(Clone)]
pub struct PlannedRecord {
    pub source: SourceIdentity,
    pub details: RecordDetails,
    pub expected: ExpectedRecord,
    pub destination: Option<DestinationRecord>,
    pub reason: Option<ReasonCode>,
    pub people: Vec<SourceIdentity>,
}

#[derive(Clone)]
pub struct PlannedRelationship {
    pub source: SourceIdentity,
    pub target: SourceIdentity,
    pub field: String,
    pub expected: ExpectedRelationship,
    pub reason: Option<ReasonCode>,
    pub people: Vec<SourceIdentity>,
}

#[derive(Clone)]
pub struct PlannedOutcomes {
    pub records: Vec<PlannedRecord>,
    pub relationships: Vec<PlannedRelationship>,
}

pub struct ProjectionMeta {
    pub id: String,
    pub source: SourceSystem,
    pub source_sha256: String,
    pub timezone: SourceTimeZone,
    pub created_at: DateTime<Utc>,
}

fn invalid() -> OperationError {
    OperationError::unavailable("import_projection_mismatch")
}

fn check_people(
    people: &[SourceIdentity],
    known: &HashSet<SourceIdentity>,
) -> Result<(), OperationError> {
    let mut seen = HashSet::new();
    for person in people {
        if !known.contains(person) || !seen.insert(person) {
            return Err(invalid());
        }
    }
    Ok(())
}

pub fn project(
    meta: ProjectionMeta,
    inventory: &SourceInventory,
    outcomes: PlannedOutcomes,
    acknowledgements: &[String],
) -> Result<ImportReport, OperationError> {
    let mut records = HashMap::new();
    let mut people = HashSet::new();
    for record in &inventory.records {
        if records.insert(record.identity.clone(), record).is_some() {
            return Err(invalid());
        }
        if record.identity.kind == RecordKind::Person {
            people.insert(record.identity.clone());
        }
    }
    let mut planned_records = HashMap::new();
    for record in outcomes.records {
        check_people(&record.people, &people)?;
        if !records.contains_key(&record.source)
            || planned_records
                .insert(record.source.clone(), record)
                .is_some()
        {
            return Err(invalid());
        }
    }
    if planned_records.len() != records.len() {
        return Err(invalid());
    }
    let mut planned_relationships = HashMap::new();
    for relationship in outcomes.relationships {
        check_people(&relationship.people, &people)?;
        let key = (
            relationship.source.clone(),
            relationship.target.clone(),
            relationship.field.clone(),
        );
        if planned_relationships.insert(key, relationship).is_some() {
            return Err(invalid());
        }
    }
    if planned_relationships.len() != inventory.relationships.len() {
        return Err(invalid());
    }
    let required: HashSet<_> = inventory
        .omissions
        .iter()
        .map(|item| item.acknowledgement_id.as_str())
        .collect();
    if required.len() != inventory.omissions.len() {
        return Err(invalid());
    }
    let mut accepted = HashSet::new();
    for id in acknowledgements {
        if !required.contains(id.as_str()) || !accepted.insert(id.as_str()) {
            return Err(invalid());
        }
    }
    let mut report = ImportReport {
        id: meta.id,
        state: ImportState::Blocked,
        source: meta.source,
        source_sha256: meta.source_sha256,
        source_timezone: meta.timezone,
        created_at: meta.created_at,
        totals: ReportTotals::default(),
        people: Vec::new(),
        records: Vec::new(),
        relationships: Vec::new(),
        omissions: Vec::new(),
        issues: Vec::new(),
    };
    let mut person_indices = HashMap::new();
    for item in &inventory.records {
        if item.identity.kind != RecordKind::Person {
            continue;
        }
        let plan = planned_records.get(&item.identity).ok_or_else(invalid)?;
        let result = RecordResult {
            source: item.identity.clone(),
            expected: plan.expected,
            actual: None::<ActualRecord>,
            destination: plan.destination.clone(),
            reason: plan.reason,
            details: plan.details.clone(),
        };
        person_indices.insert(item.identity.clone(), report.people.len());
        report.people.push(PersonReport {
            person: result,
            records: Vec::new(),
            relationships: Vec::new(),
            omissions: Vec::new(),
        });
    }
    for item in &inventory.records {
        let plan = planned_records.get(&item.identity).ok_or_else(invalid)?;
        match plan.expected {
            ExpectedRecord::Create => report.totals.expected_create += 1,
            ExpectedRecord::Match => report.totals.expected_match += 1,
            ExpectedRecord::Reject => report.totals.decisions_required += 1,
            ExpectedRecord::DecisionRequired => report.totals.decisions_required += 1,
            ExpectedRecord::Skip => (),
        }
        if item.identity.kind == RecordKind::Person {
            continue;
        }
        let result = RecordResult {
            source: item.identity.clone(),
            expected: plan.expected,
            actual: None,
            destination: plan.destination.clone(),
            reason: plan.reason,
            details: plan.details.clone(),
        };
        if plan.people.is_empty() {
            report.records.push(result);
        } else {
            for person in &plan.people {
                report.people[*person_indices.get(person).ok_or_else(invalid)?]
                    .records
                    .push(result.clone());
            }
        }
    }
    for item in &inventory.relationships {
        let key = (item.source.clone(), item.target.clone(), item.field.clone());
        let plan = planned_relationships.remove(&key).ok_or_else(invalid)?;
        match plan.expected {
            ExpectedRelationship::Link => report.totals.expected_link += 1,
            ExpectedRelationship::Reject => report.totals.decisions_required += 1,
            ExpectedRelationship::DecisionRequired => report.totals.decisions_required += 1,
            ExpectedRelationship::Omit => (),
        }
        let result = RelationshipResult {
            kind: item.kind,
            field: Some(item.field.clone()),
            source: item.source.clone(),
            target: item.target.clone(),
            destination_source: None,
            destination_target: None,
            expected: plan.expected,
            actual: None,
            reason: plan.reason.or(item.reason),
        };
        if plan.people.is_empty() {
            report.relationships.push(result);
        } else {
            for person in &plan.people {
                report.people[*person_indices.get(person).ok_or_else(invalid)?]
                    .relationships
                    .push(result.clone());
            }
        }
    }
    for item in &inventory.omissions {
        let omission = Omission {
            acknowledged: accepted.contains(item.acknowledgement_id.as_str()),
            ..item.clone()
        };
        let plan = planned_records.get(&item.item).ok_or_else(invalid)?;
        report.totals.omissions += 1;
        if !omission.acknowledged {
            report.totals.decisions_required += 1;
        }
        if let Some(index) = person_indices.get(&item.item) {
            report.people[*index].omissions.push(omission);
        } else if plan.people.is_empty() {
            report.omissions.push(omission);
        } else {
            for person in &plan.people {
                report.people[*person_indices.get(person).ok_or_else(invalid)?]
                    .omissions
                    .push(omission.clone());
            }
        }
    }
    Ok(report)
}
