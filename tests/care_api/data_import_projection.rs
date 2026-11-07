use med_tracker::models::data_imports::{
    contracts::{
        ExpectedRecord, ExpectedRelationship, ImportState, RecordDetails, RecordKind,
        RelationshipKind, SourceIdentity, SourceInventory, SourceRecord, SourceRelationship,
        SourceSystem, SourceTimeZone,
    },
    projection::{self, PlannedOutcomes, PlannedRecord, PlannedRelationship},
};

fn source(kind: RecordKind, id: &str) -> SourceIdentity {
    SourceIdentity {
        source: SourceSystem::MedTracker05,
        instance: "synthetic".into(),
        kind,
        id: id.into(),
    }
}

fn meta() -> projection::ProjectionMeta {
    projection::ProjectionMeta {
        id: "id".into(),
        source: SourceSystem::MedTracker05,
        source_sha256: "a".repeat(64),
        timezone: SourceTimeZone::parse("UTC").unwrap(),
        created_at: chrono::Utc::now(),
    }
}

#[test]
fn shared_inventory_is_visible_for_both_people_but_counted_once() {
    let first = source(RecordKind::Person, "first");
    let second = source(RecordKind::Person, "second");
    let medicine = source(RecordKind::Medication, "shared");
    let inventory = SourceInventory {
        records: [first.clone(), second.clone(), medicine.clone()]
            .into_iter()
            .map(|identity| SourceRecord {
                identity,
                details: RecordDetails::default(),
            })
            .collect(),
        relationships: [first.clone(), second.clone()]
            .into_iter()
            .map(|person| SourceRelationship {
                kind: RelationshipKind::PersonMedication,
                field: "medication".into(),
                reason: None,
                source: person,
                target: medicine.clone(),
            })
            .collect(),
        omissions: Vec::new(),
    };
    let outcomes = PlannedOutcomes {
        records: vec![
            PlannedRecord {
                source: first.clone(),
                details: RecordDetails::default(),
                expected: ExpectedRecord::Create,
                destination: None,
                reason: None,
                people: vec![first.clone()],
            },
            PlannedRecord {
                source: second.clone(),
                details: RecordDetails::default(),
                expected: ExpectedRecord::Create,
                destination: None,
                reason: None,
                people: vec![second.clone()],
            },
            PlannedRecord {
                source: medicine.clone(),
                details: RecordDetails::default(),
                expected: ExpectedRecord::Create,
                destination: None,
                reason: None,
                people: vec![first.clone(), second.clone()],
            },
        ],
        relationships: [first, second]
            .into_iter()
            .map(|person| PlannedRelationship {
                source: person.clone(),
                target: medicine.clone(),
                field: "medication".into(),
                expected: ExpectedRelationship::Link,
                reason: None,
                people: vec![person],
            })
            .collect(),
    };
    let mut forged = outcomes.clone();
    forged.records.pop();
    assert!(projection::project(meta(), &inventory, forged, &[],).is_err());
    assert!(
        projection::project(meta(), &inventory, outcomes.clone(), &["forged".into()],).is_err()
    );
    let report = projection::project(meta(), &inventory, outcomes, &[]).unwrap();
    assert_eq!(report.state, ImportState::Blocked);
    assert_eq!(report.totals.expected_create, 3);
    assert_eq!(report.totals.expected_link, 2);
    assert_eq!(report.totals.created, 0);
    assert_eq!(report.totals.verified_links, 0);
    assert_eq!(report.people.len(), 2);
    assert!(
        report
            .people
            .iter()
            .all(|person| person.records.len() == 1 && person.records[0].source == medicine)
    );
    assert!(
        report
            .people
            .iter()
            .all(|person| person.relationships[0].actual.is_none())
    );
}
