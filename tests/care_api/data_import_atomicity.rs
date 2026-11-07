use super::Application;
use futures_util::FutureExt;
use med_tracker::models::{
    _entities::{
        accounts, carer_relationships, dosages, health_event_medications, health_events,
        household_memberships, locations, medication_dose_occurrences, medication_pause_periods,
        medication_takes, medications, notification_preferences, people, person_medications,
        person_schedules, schedules, shared_schedule_administrations,
        shared_schedule_adoption_facts, shared_schedule_executions, shared_schedule_states,
        versions,
    },
    data_imports::{
        confirmation,
        contracts::{
            ActualRecord, ActualRelationship, ConsentInput, DestinationId, ExpectedRecord,
            ExpectedRelationship, FieldOutcome, ImportReport, ImportState, PersonDestination,
            PersonMapping, PreviewResult, ReasonCode, RecordKind, SourceSystem,
        },
        inspection::{self, Upload},
        preview,
    },
    identity::credentials::{CredentialMethod, CredentialProvenance},
};
use sea_orm::prelude::Decimal;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, TransactionTrait,
    sea_query::{Alias, ColumnDef, Expr, ExprTrait, Func, Query, Table},
};
use std::{
    io::{Cursor, Write},
    panic::AssertUnwindSafe,
};
use zip::{ZipWriter, write::SimpleFileOptions};

fn upload(comment: &str) -> Upload {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    zip.set_comment(comment);
    for (name, data) in [
        (
            "portable.json",
            include_bytes!("../fixtures/portable_rails/prepared-history-v2.json").as_slice(),
        ),
        (
            "manifest.json",
            include_bytes!("../fixtures/portable_rails/prepared-history-v2.manifest.json")
                .as_slice(),
        ),
    ] {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(data).unwrap();
    }
    Upload {
        source: SourceSystem::MedTracker05,
        archive: zip.finish().unwrap().into_inner(),
        passphrase: "password".into(),
    }
}

async fn ready(app: &Application) -> PreviewResult {
    let scope = app.fixture.scenario.scope();
    let comment = uuid::Uuid::new_v4().to_string();
    let timezone = inspection::confirmed_timezone("Europe/London", true).unwrap();
    let inspected = inspection::inspect(&app.context, &scope, upload(&comment), timezone)
        .await
        .unwrap();
    let person = inspected
        .inventory
        .records
        .iter()
        .find(|row| row.identity.kind == RecordKind::Person)
        .unwrap()
        .identity
        .clone();
    let preview = preview::create(
        &app.context,
        &scope,
        preview::PreviewInput {
            inspection_id: inspected.id,
            upload: upload(&comment),
            timezone,
            mappings: vec![PersonMapping {
                source: person.clone(),
                destination: PersonDestination::NewDependent,
            }],
            declarations: vec![ConsentInput::InitialGuardianDeclaration {
                source_person: person,
                accepted: true,
            }],
            recipient_consents: vec![],
            acknowledged_omissions: inspected
                .inventory
                .omissions
                .iter()
                .map(|item| item.acknowledgement_id.clone())
                .collect(),
        },
    )
    .await
    .unwrap();
    assert_eq!(preview.report.state, ImportState::Ready);
    preview
}

fn unique_upload(tag: &str) -> Upload {
    let payload = include_str!("../fixtures/portable_rails/prepared-history-v2.json")
        .replace("prepared-history-v2", tag)
        .replace("550e8400-e29b-41d4-a716-000000000006", tag);
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in [
        ("portable.json", payload.as_bytes()),
        (
            "manifest.json",
            include_bytes!("../fixtures/portable_rails/prepared-history-v2.manifest.json")
                .as_slice(),
        ),
    ] {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(data).unwrap();
    }
    Upload {
        source: SourceSystem::MedTracker05,
        archive: zip.finish().unwrap().into_inner(),
        passphrase: "password".into(),
    }
}

async fn ready_unique(app: &Application) -> (PreviewResult, String) {
    let tag = uuid::Uuid::new_v4().to_string();
    let source_email = format!("portable-fixture-{tag}@example.test");
    let scope = app.fixture.scenario.scope();
    let timezone = inspection::confirmed_timezone("Europe/London", true).unwrap();
    let inspected = inspection::inspect(&app.context, &scope, unique_upload(&tag), timezone)
        .await
        .unwrap();
    let person = inspected
        .inventory
        .records
        .iter()
        .find(|row| row.identity.kind == RecordKind::Person)
        .unwrap()
        .identity
        .clone();
    let result = preview::create(
        &app.context,
        &scope,
        preview::PreviewInput {
            inspection_id: inspected.id,
            upload: unique_upload(&tag),
            timezone,
            mappings: vec![PersonMapping {
                source: person.clone(),
                destination: PersonDestination::NewDependent,
            }],
            declarations: vec![ConsentInput::InitialGuardianDeclaration {
                source_person: person,
                accepted: true,
            }],
            recipient_consents: vec![],
            acknowledged_omissions: inspected
                .inventory
                .omissions
                .iter()
                .map(|row| row.acknowledgement_id.clone())
                .collect(),
        },
    )
    .await
    .unwrap();
    assert_eq!(result.report.state, ImportState::Ready);
    (result, source_email)
}

fn destination(report: &ImportReport, kind: RecordKind) -> DestinationId {
    report
        .people
        .iter()
        .flat_map(|person| std::iter::once(&person.person).chain(person.records.iter()))
        .chain(report.records.iter())
        .find(|record| record.source.kind == kind)
        .and_then(|record| record.destination.as_ref())
        .unwrap()
        .id
        .clone()
}

#[tokio::test]
async fn restored_history_preserves_native_values_and_relationships() {
    let app = Application::new().await;
    let scope = app.fixture.scenario.scope();
    let (ready, source_email) = ready_unique(&app).await;
    let live_before = live_effects(&app, &source_email).await;
    let stock_before = medications::Entity::find()
        .filter(medications::Column::HouseholdId.eq(scope.household_id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let dosage_before = dosages::Entity::find()
        .filter(dosages::Column::HouseholdId.eq(scope.household_id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let provenance = CredentialProvenance {
        method: CredentialMethod::BrowserSession,
        reference: "synthetic-session".into(),
    };
    let report = confirmation::confirm(
        &app.context,
        &scope,
        &ready.report.id,
        ready.confirmation.unwrap(),
        "password".into(),
        &provenance,
        &scope.request_id,
    )
    .await
    .unwrap();
    assert_eq!(report.state, ImportState::Completed);
    assert_eq!(
        serde_json::to_value(report.source_timezone).unwrap(),
        serde_json::json!({"name":"Europe/London", "provenance":"operator_confirmed"})
    );
    let records: Vec<_> = report
        .people
        .iter()
        .flat_map(|person| std::iter::once(&person.person).chain(person.records.iter()))
        .chain(report.records.iter())
        .collect();
    let relationships: Vec<_> = report
        .people
        .iter()
        .flat_map(|person| person.relationships.iter())
        .chain(report.relationships.iter())
        .collect();
    let omissions: Vec<_> = report
        .people
        .iter()
        .flat_map(|person| person.omissions.iter())
        .chain(report.omissions.iter())
        .collect();
    for record in &records {
        for field in ["updated_at", "etag"] {
            let detail = record
                .details
                .source_fields
                .iter()
                .find(|detail| detail.field == field)
                .unwrap();
            assert_eq!(detail.outcome, FieldOutcome::Unsupported);
            assert_eq!(detail.reason, Some(ReasonCode::UnsupportedField));
            assert!(
                omissions
                    .iter()
                    .any(|omission| omission.item == record.source
                        && omission.field.as_deref() == Some(field)
                        && omission.reason == ReasonCode::UnsupportedField
                        && omission.acknowledged)
            );
        }
    }
    let pause_report = records
        .iter()
        .find(|record| record.source.kind == RecordKind::Pause)
        .unwrap();
    let imported_context = pause_report
        .details
        .source_fields
        .iter()
        .find(|field| field.field == "imported_context")
        .unwrap();
    assert_eq!(imported_context.source_value, "false");
    assert_eq!(imported_context.outcome, FieldOutcome::Changed);
    assert_eq!(imported_context.reason, Some(ReasonCode::UnsupportedField));
    assert!(
        omissions
            .iter()
            .any(|omission| omission.item == pause_report.source
                && omission.field.as_deref() == Some("imported_context")
                && omission.reason == ReasonCode::UnsupportedField
                && omission.acknowledged)
    );
    for field in [
        "recorded_by_person_portable_id",
        "resumed_by_person_portable_id",
    ] {
        let edge = relationships
            .iter()
            .find(|edge| edge.source == pause_report.source && edge.field.as_deref() == Some(field))
            .unwrap();
        assert_eq!(edge.reason, Some(ReasonCode::HistoricalActorUnavailable));
        assert_eq!(edge.expected, ExpectedRelationship::Omit);
        assert_eq!(edge.actual, Some(ActualRelationship::Omitted));
    }
    assert_eq!(
        report.totals.created,
        records
            .iter()
            .filter(|r| r.actual == Some(ActualRecord::Created))
            .count() as u64
    );
    assert_eq!(
        report.totals.verified_links,
        relationships
            .iter()
            .filter(|r| r.actual == Some(ActualRelationship::Verified))
            .count() as u64
    );
    assert!(
        records
            .iter()
            .filter(|r| r.expected == ExpectedRecord::Create)
            .all(|r| r.actual == Some(ActualRecord::Created) && r.destination.is_some())
    );
    assert!(
        relationships
            .iter()
            .filter(|r| r.expected == ExpectedRelationship::Link)
            .all(|r| r.actual == Some(ActualRelationship::Verified)
                && r.destination_source.is_some()
                && r.destination_target.is_some())
    );
    let DestinationId::Person(person) = destination(&report, RecordKind::Person) else {
        panic!("person destination")
    };
    let DestinationId::Medication(medication) = destination(&report, RecordKind::Medication) else {
        panic!("medication destination")
    };
    let DestinationId::DosageOption(dosage) = destination(&report, RecordKind::DosageOption) else {
        panic!("dosage destination")
    };
    let DestinationId::Schedule(schedule) = destination(&report, RecordKind::Schedule) else {
        panic!("schedule destination")
    };
    let DestinationId::Assignment(assignment) = destination(&report, RecordKind::Assignment) else {
        panic!("assignment destination")
    };
    let DestinationId::Dose(take) = destination(&report, RecordKind::Dose) else {
        panic!("take destination")
    };
    let DestinationId::HealthEvent(event) = destination(&report, RecordKind::HealthEvent) else {
        panic!("event destination")
    };
    let DestinationId::Pause(pause) = destination(&report, RecordKind::Pause) else {
        panic!("pause destination")
    };
    let DestinationId::Occurrence(occurrence) = destination(&report, RecordKind::Occurrence) else {
        panic!("occurrence destination")
    };
    let DestinationId::NotificationPreference(preference) =
        destination(&report, RecordKind::NotificationPreference)
    else {
        panic!("preference destination")
    };
    let db = &app.fixture.admin;
    let subject = people::Entity::find_by_id(person.0)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(subject.name, "Synthetic Dependent");
    assert!(!subject.has_capacity);
    assert_eq!(subject.date_of_birth.unwrap().to_string(), "1990-01-15");
    assert_eq!(subject.email.as_deref(), Some(source_email.as_str()));
    let participation = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(scope.household_id))
        .filter(household_memberships::Column::PersonId.eq(person.0))
        .all(db)
        .await
        .unwrap();
    assert_eq!(participation.len(), 1);
    assert_eq!(participation[0].status, "active");
    let med = medications::Entity::find_by_id(medication.0)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    let location = locations::Entity::find_by_id(med.location_id)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(location.name, "Synthetic current cupboard");
    assert_eq!(
        location.description.as_deref(),
        Some("Fixture storage label")
    );
    assert_eq!(med.current_supply.unwrap().to_string(), "37.50");
    assert_eq!(med.reorder_threshold.to_string(), "5.50");
    assert_eq!(med.name.as_deref(), Some("Synthetic medicine"));
    assert_eq!(med.friendly_name.as_deref(), Some("Fixture tablet"));
    assert_eq!(med.category.as_deref(), Some("Vitamin"));
    assert_eq!(
        med.description.as_deref(),
        Some("No real medicine or patient data")
    );
    assert_eq!(med.dose_amount, Some(1.0));
    assert_eq!(med.dose_unit.as_deref(), Some("tablet"));
    assert_eq!(med.default_schedule_type, 0);
    assert_eq!(med.barcode.as_deref(), Some("2000000000060"));
    assert_eq!(med.dmd_code.as_deref(), Some("synthetic-dmd"));
    assert_eq!(med.dmd_system.as_deref(), Some("synthetic"));
    assert_eq!(med.dmd_concept_class.as_deref(), Some("VMP"));
    assert_eq!(med.current_supply, Some(Decimal::new(375, 1)));
    assert_eq!(med.reorder_threshold, Decimal::new(55, 1));
    let dose = dosages::Entity::find_by_id(dosage.0)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(dose.medication_id, med.id);
    assert_eq!(dose.current_supply.unwrap().to_string(), "8.50");
    assert_eq!(dose.amount.to_string(), "0.50");
    assert_eq!(dose.reorder_threshold.unwrap().to_string(), "2.50");
    assert_eq!(dose.unit, "tablet");
    assert_eq!(dose.amount, Decimal::new(5, 1));
    assert_eq!(dose.current_supply, Some(Decimal::new(85, 1)));
    assert_eq!(dose.reorder_threshold, Some(Decimal::new(25, 1)));
    assert_eq!(dose.frequency, "daily");
    assert_eq!(dose.description.as_deref(), Some("Synthetic half tablet"));
    assert!(dose.default_for_adults);
    assert!(!dose.default_for_children);
    assert_eq!(dose.default_max_daily_doses, 2);
    assert_eq!(dose.default_min_hours_between_doses, Decimal::new(4, 0));
    assert_eq!(dose.default_dose_cycle, 0);
    let schedule = schedules::Entity::find_by_id(schedule.0)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            schedule.person_id,
            schedule.medication_id,
            schedule.source_dosage_option_id
        ),
        (person.0, med.id, Some(dose.id))
    );
    assert_eq!(schedule.dose_amount.unwrap().to_string(), "0.50");
    assert_eq!(schedule.dose_unit.as_deref(), Some("tablet"));
    assert_eq!(schedule.start_date.unwrap().to_string(), "2026-10-01");
    assert_eq!(schedule.dose_amount, Some(Decimal::new(5, 1)));
    assert_eq!(schedule.dose_cycle, Some(0));
    assert_eq!(schedule.frequency.as_deref(), Some("daily"));
    assert_eq!(schedule.max_daily_doses, Some(2));
    assert_eq!(schedule.min_hours_between_doses, Some(4));
    assert_eq!(schedule.schedule_type, 0);
    assert_eq!(schedule.schedule_config, serde_json::json!({}));
    assert_eq!(
        schedule.start_date,
        Some(chrono::NaiveDate::from_ymd_opt(2026, 10, 1).unwrap())
    );
    assert_eq!(schedule.end_date, None);
    assert_eq!(schedule.retired_at, None);
    assert!(schedule.active);
    assert_eq!(
        schedule.notes.as_deref(),
        Some("Synthetic care instructions")
    );
    assert_eq!(schedule.origin_time_zone.as_deref(), Some("Europe/London"));
    assert_eq!(schedule.clinical_revision, Some(1));
    let assignment = person_medications::Entity::find_by_id(assignment.0)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            assignment.person_id,
            assignment.medication_id,
            assignment.source_dosage_option_id
        ),
        (person.0, med.id, Some(dose.id))
    );
    assert_eq!(assignment.dose_amount.unwrap().to_string(), "0.50");
    assert_eq!(assignment.dose_unit.as_deref(), Some("tablet"));
    assert_eq!(assignment.dose_amount, Some(Decimal::new(5, 1)));
    assert_eq!(assignment.dose_cycle, Some(0));
    assert_eq!(assignment.max_daily_doses, Some(2));
    assert_eq!(assignment.min_hours_between_doses, Some(4));
    assert_eq!(assignment.administration_kind, 0);
    assert_eq!(assignment.position, 1);
    assert!(assignment.active);
    assert_eq!(assignment.retired_at, None);
    assert_eq!(
        assignment.notes.as_deref(),
        Some("Synthetic care instructions")
    );
    let take = medication_takes::Entity::find_by_id(take.0)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (take.schedule_id, take.taken_from_medication_id),
        (Some(schedule.id), Some(med.id))
    );
    assert_eq!(take.dose_amount.unwrap().to_string(), "0.50");
    assert_eq!(take.dose_unit.as_deref(), Some("tablet"));
    assert_eq!(take.person_medication_id, None);
    assert_eq!(take.dose_amount, Some(Decimal::new(5, 1)));
    assert_eq!(
        take.taken_at,
        Some(
            chrono::NaiveDateTime::parse_from_str(
                "2026-10-10 08:15:00.123456",
                "%Y-%m-%d %H:%M:%S%.f"
            )
            .unwrap()
        )
    );
    assert_eq!(
        take.client_uuid.as_deref(),
        source_email
            .strip_prefix("portable-fixture-")
            .and_then(|value| value.strip_suffix("@example.test"))
    );
    assert_eq!(
        take.taken_at.unwrap().to_string(),
        "2026-10-10 08:15:00.123456"
    );
    let former = locations::Entity::find_by_id(take.taken_from_location_id.unwrap())
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(former.name, "Synthetic former cupboard");
    assert_eq!(
        former.description.as_deref(),
        Some("Historical dose location")
    );
    let pause = medication_pause_periods::Entity::find_by_id(pause.0)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pause.schedule_id, Some(schedule.id));
    assert_eq!(pause.person_medication_id, None);
    assert_eq!(pause.recorded_by_membership_id, None);
    assert_eq!(pause.resumed_by_membership_id, None);
    assert_eq!(pause.reason, "out_of_supply");
    assert_eq!(pause.note.as_deref(), Some("Synthetic closed pause"));
    assert!(!pause.legacy_context);
    assert!(pause.imported_context);
    assert_eq!(
        pause.imported_actor_references,
        serde_json::json!({"recorded_by_person_portable_id":"synthetic-absent-actor", "resumed_by_person_portable_id":"synthetic-absent-actor"})
    );
    assert_eq!(
        pause.created_at,
        chrono::NaiveDateTime::parse_from_str("2026-10-05 10:00:00.123456", "%Y-%m-%d %H:%M:%S%.f")
            .unwrap()
    );
    assert_eq!(
        pause.started_at,
        Some(
            chrono::NaiveDateTime::parse_from_str(
                "2026-10-05 10:00:00.123456",
                "%Y-%m-%d %H:%M:%S%.f"
            )
            .unwrap()
        )
    );
    assert_eq!(
        pause.ended_at,
        Some(
            chrono::NaiveDateTime::parse_from_str(
                "2026-10-06 11:00:00.234567",
                "%Y-%m-%d %H:%M:%S%.f"
            )
            .unwrap()
        )
    );
    assert_eq!(
        pause.started_at.unwrap().to_string(),
        "2026-10-05 10:00:00.123456"
    );
    assert_eq!(
        pause.ended_at.unwrap().to_string(),
        "2026-10-06 11:00:00.234567"
    );
    let occurrence = medication_dose_occurrences::Entity::find_by_id(occurrence.0)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(occurrence.schedule_id, Some(schedule.id));
    assert_eq!(occurrence.person_medication_id, None);
    assert_eq!(occurrence.medication_take_id, None);
    assert_eq!(occurrence.outcome, "not_taken");
    assert_eq!(occurrence.reason.as_deref(), Some("refused"));
    assert_eq!(occurrence.note.as_deref(), Some("Synthetic decision"));
    assert_eq!(
        occurrence.window_starts_on,
        chrono::NaiveDate::from_ymd_opt(2026, 10, 11).unwrap()
    );
    assert_eq!(
        occurrence.window_ends_on,
        Some(chrono::NaiveDate::from_ymd_opt(2026, 10, 11).unwrap())
    );
    assert_eq!(occurrence.position, 1);
    assert_eq!(
        occurrence.resolved_by_membership_id,
        Some(app.fixture.membership.id)
    );
    assert_eq!(
        occurrence.scheduled_at,
        Some(
            chrono::NaiveDateTime::parse_from_str("2026-10-11 08:15:00", "%Y-%m-%d %H:%M:%S")
                .unwrap()
        )
    );
    assert_eq!(
        occurrence.resolved_at,
        Some(
            chrono::NaiveDateTime::parse_from_str(
                "2026-10-11 08:20:00.345678",
                "%Y-%m-%d %H:%M:%S%.f"
            )
            .unwrap()
        )
    );
    assert_eq!(
        occurrence.scheduled_at.unwrap().to_string(),
        "2026-10-11 08:15:00"
    );
    assert_eq!(
        occurrence.resolved_at.unwrap().to_string(),
        "2026-10-11 08:20:00.345678"
    );
    let preference = notification_preferences::Entity::find_by_id(preference.0)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(preference.person_id, person.0);
    assert!(preference.enabled && preference.missed_dose_enabled && !preference.dose_due_enabled);
    assert!(!preference.low_stock_enabled);
    assert!(preference.private_text_enabled);
    assert_eq!(preference.afternoon_time, None);
    assert_eq!(preference.night_time, None);
    assert_eq!(
        preference.morning_time,
        Some(chrono::NaiveTime::from_hms_opt(8, 15, 0).unwrap())
    );
    assert_eq!(
        preference.evening_time,
        Some(chrono::NaiveTime::from_hms_opt(19, 30, 0).unwrap())
    );
    assert_eq!(preference.morning_time.unwrap().to_string(), "08:15:00");
    assert_eq!(preference.evening_time.unwrap().to_string(), "19:30:00");
    let event = health_events::Entity::find_by_id(event.0)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (event.person_id, event.title.as_str()),
        (person.0, "Synthetic health event")
    );
    let links = health_event_medications::Entity::find()
        .filter(health_event_medications::Column::HouseholdId.eq(scope.household_id))
        .filter(health_event_medications::Column::HealthEventId.eq(event.id))
        .all(db)
        .await
        .unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].medication_id, Some(med.id));
    assert_eq!(event.event_kind, 0);
    assert_eq!(event.severity, Some(0));
    assert_eq!(event.notes.as_deref(), Some("Fixture only"));
    assert_eq!(
        event.started_on,
        chrono::NaiveDate::from_ymd_opt(2026, 10, 9).unwrap()
    );
    assert_eq!(
        event.ended_on,
        Some(chrono::NaiveDate::from_ymd_opt(2026, 10, 10).unwrap())
    );
    assert_eq!(event.started_on.to_string(), "2026-10-09");
    assert_eq!(event.ended_on.unwrap().to_string(), "2026-10-10");
    assert_guardian_and_canonical(
        &app,
        person.0,
        &schedule,
        &med,
        &take,
        former.id,
        &scope.request_id,
    )
    .await;
    for previous in stock_before {
        assert_eq!(
            medications::Entity::find_by_id(previous.id)
                .one(db)
                .await
                .unwrap()
                .unwrap()
                .current_supply,
            previous.current_supply
        );
    }
    for previous in dosage_before {
        assert_eq!(
            dosages::Entity::find_by_id(previous.id)
                .one(db)
                .await
                .unwrap()
                .unwrap()
                .current_supply,
            previous.current_supply
        );
    }
    assert_eq!(live_effects(&app, &source_email).await, live_before);
    app.close().await;
}

async fn assert_guardian_and_canonical(
    app: &Application,
    person: i64,
    schedule: &schedules::Model,
    medication: &medications::Model,
    take: &medication_takes::Model,
    former_location: i64,
    request_id: &str,
) {
    let db = &app.fixture.admin;
    let household = app.fixture.household.id;
    let query = Query::select()
        .columns(["id", "guardian_person_id", "declared_by_account_id"])
        .from("guardian_declarations")
        .and_where(Expr::col("subject_person_id").eq(person))
        .and_where(Expr::col("origin_household_id").eq(household))
        .to_owned();
    let declarations = db
        .query_all_raw(db.get_database_backend().build(&query))
        .await
        .unwrap();
    assert_eq!(declarations.len(), 1);
    let declaration_id: i64 = declarations[0].try_get("", "id").unwrap();
    assert_eq!(
        declarations[0]
            .try_get::<i64>("", "guardian_person_id")
            .unwrap(),
        app.fixture.adult.id
    );
    assert_eq!(
        declarations[0]
            .try_get::<i64>("", "declared_by_account_id")
            .unwrap(),
        app.fixture.account.id
    );
    let carers = carer_relationships::Entity::find()
        .filter(carer_relationships::Column::HouseholdId.eq(household))
        .filter(carer_relationships::Column::PatientId.eq(person))
        .all(db)
        .await
        .unwrap();
    assert_eq!(carers.len(), 1);
    assert_eq!(carers[0].carer_id, app.fixture.adult.id);
    assert_eq!(carers[0].guardian_declaration_id, Some(declaration_id));
    let plan_id = schedule.person_schedule_id.as_ref().unwrap();
    let plan = person_schedules::Entity::find_by_id(plan_id)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            plan.person_id,
            plan.origin_household_id,
            plan.origin_schedule_id,
            plan.origin_stock_id
        ),
        (person, household, schedule.id, medication.id)
    );
    assert_eq!(plan.origin_time_zone, "Europe/London");
    assert_eq!(plan.timezone_provenance, "captured_sharer_zone");
    assert_eq!(plan.clinical_revision, 1);
    assert!(plan.active);
    assert_eq!(plan.retired_at, None);
    assert_eq!(plan.dose_amount, Some(Decimal::new(5, 1)));
    assert_eq!(plan.dose_unit.as_deref(), Some("tablet"));
    assert_eq!(plan.dose_cycle, Some(0));
    assert_eq!(plan.frequency.as_deref(), Some("daily"));
    assert_eq!(
        plan.start_date,
        Some(chrono::NaiveDate::from_ymd_opt(2026, 10, 1).unwrap())
    );
    assert_eq!(plan.end_date, None);
    assert_eq!(plan.notes.as_deref(), Some("Synthetic care instructions"));
    assert_eq!(plan.max_daily_doses, Some(2));
    assert_eq!(plan.min_hours_between_doses, Some(4));
    assert_eq!(plan.schedule_type, 0);
    assert_eq!(plan.schedule_config, serde_json::json!({}));
    assert_eq!(plan.medicine_name.as_deref(), Some("Synthetic medicine"));
    assert_eq!(plan.stock_unit.as_deref(), Some("tablet"));
    assert_eq!(plan.stock_dose_default, Some(Decimal::new(1, 0)));
    assert!(plan.stock_default_known);
    assert_eq!(plan.dmd_system.as_deref(), Some("synthetic"));
    assert_eq!(plan.dmd_code.as_deref(), Some("synthetic-dmd"));
    assert_eq!(plan.dmd_concept_class.as_deref(), Some("VMP"));
    assert_eq!(plan.pause_started_at, None);
    assert_eq!(plan.pause_instance_id, None);
    assert_eq!(plan.pause_reason, None);
    assert_eq!(plan.pause_note, None);
    let execution = shared_schedule_executions::Entity::find_by_id(schedule.id)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            execution.person_schedule_id.as_str(),
            execution.person_id,
            execution.household_id
        ),
        (plan_id.as_str(), person, household)
    );
    let fact = shared_schedule_adoption_facts::Entity::find_by_id(take.id)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            fact.person_schedule_id.as_deref(),
            fact.source_schedule_id,
            fact.person_id,
            fact.household_id
        ),
        (Some(plan_id.as_str()), schedule.id, person, household)
    );
    assert_eq!(
        (
            fact.captured_by_account_id,
            fact.captured_by_person_id,
            fact.captured_by_membership_id,
            fact.guardian_declaration_id
        ),
        (
            app.fixture.account.id,
            app.fixture.adult.id,
            app.fixture.membership.id,
            Some(declaration_id)
        )
    );
    assert_eq!(fact.request_id, request_id);
    assert_eq!(fact.source_portable_id, schedule.portable_id);
    assert_eq!(fact.take_portable_id, take.portable_id);
    assert_eq!(
        fact.authentication_method.as_deref(),
        Some("browser_session")
    );
    assert_eq!(
        fact.credential_reference.as_deref(),
        Some("synthetic-session")
    );
    let version = versions::Entity::find_by_id(fact.original_version_id)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (
            version.item_type.as_str(),
            version.item_id,
            version.household_id,
            version.actor_membership_id
        ),
        (
            "MedicationTake",
            take.id,
            Some(household),
            Some(app.fixture.membership.id)
        )
    );
    assert_eq!(fact.taken_at, take.taken_at.unwrap());
    assert_eq!(
        fact.taken_at,
        chrono::NaiveDateTime::parse_from_str("2026-10-10 08:15:00.123456", "%Y-%m-%d %H:%M:%S%.f")
            .unwrap()
    );
    assert_eq!(fact.dose_amount, Decimal::new(5, 1));
    assert_eq!(fact.dose_unit.as_deref(), Some("tablet"));
    assert_eq!(fact.taken_from_medication_id, Some(medication.id));
    assert_eq!(fact.taken_from_location_id, Some(former_location));
    let safety = shared_schedule_administrations::Entity::find()
        .filter(shared_schedule_administrations::Column::PersonScheduleId.eq(plan_id))
        .filter(shared_schedule_administrations::Column::MedicationTakeId.eq(take.id))
        .all(db)
        .await
        .unwrap();
    assert_eq!(safety.len(), 1);
    assert_eq!(safety[0].household_id, household);
    assert_eq!(
        safety[0].taken_at,
        chrono::NaiveDateTime::parse_from_str("2026-10-10 08:15:00.123456", "%Y-%m-%d %H:%M:%S%.f")
            .unwrap()
    );
    assert_eq!(safety[0].occurrence_starts_on, None);
    assert_eq!(safety[0].occurrence_position, None);
    let state = shared_schedule_states::Entity::find_by_id(plan_id)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(state.latest_taken_at, take.taken_at);
    assert_eq!(state.origin_time_zone, "Europe/London");
    assert_eq!(state.clinical_revision, 1);
    assert!(state.active);
    assert_eq!(state.retired_at, None);
    assert_eq!(state.dose_amount, Some(Decimal::new(5, 1)));
    assert_eq!(state.dose_unit.as_deref(), Some("tablet"));
    assert_eq!(state.dose_cycle, Some(0));
    assert_eq!(state.frequency.as_deref(), Some("daily"));
    assert_eq!(
        state.start_date,
        Some(chrono::NaiveDate::from_ymd_opt(2026, 10, 1).unwrap())
    );
    assert_eq!(state.end_date, None);
    assert_eq!(state.max_daily_doses, Some(2));
    assert_eq!(state.min_hours_between_doses, Some(4));
    assert_eq!(state.schedule_type, 0);
    assert_eq!(state.schedule_config, serde_json::json!({}));
    assert_eq!(
        state.latest_taken_at,
        Some(
            chrono::NaiveDateTime::parse_from_str(
                "2026-10-10 08:15:00.123456",
                "%Y-%m-%d %H:%M:%S%.f"
            )
            .unwrap()
        )
    );
}

#[derive(Debug, PartialEq, Eq)]
struct LiveEffects {
    notifications: i64,
    deliveries: i64,
    household_jobs: i64,
    mail: Vec<(String, i64)>,
}

async fn live_effects(app: &Application, source_email: &str) -> LiveEffects {
    let household = app.fixture.household.id;
    let people = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(household))
        .all(&app.fixture.admin)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.person_id)
        .collect::<Vec<_>>();
    let mut addresses = accounts::Entity::find()
        .filter(accounts::Column::PersonId.is_in(people))
        .all(&app.fixture.admin)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.email)
        .collect::<Vec<_>>();
    addresses.push(source_email.into());
    addresses.sort();
    addresses.dedup();
    let mut mail = Vec::new();
    for address in addresses {
        let count = queued_mail_count(app, &address).await;
        mail.push((address, count));
    }
    let jobs = Query::select()
        .expr_as(Expr::col("id").count(), "count")
        .from("pg_loco_queue")
        .and_where(
            Func::cust("jsonb_extract_path_text")
                .args([Expr::col("task_data"), Expr::val("household_id")])
                .eq(household.to_string()),
        )
        .to_owned();
    let household_jobs = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&jobs))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap();
    LiveEffects {
        notifications: scoped_count(app, "notification_events", "household_id").await,
        deliveries: scoped_count(app, "notification_deliveries", "household_id").await,
        household_jobs,
        mail,
    }
}

async fn scoped_count(app: &Application, table: &str, column: &str) -> i64 {
    let query = Query::select()
        .expr_as(Expr::col(Alias::new(column)).count(), "count")
        .from(Alias::new(table))
        .and_where(Expr::col(Alias::new(column)).eq(app.fixture.household.id))
        .to_owned();
    app.fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap()
}

async fn queued_mail_count(app: &Application, address: &str) -> i64 {
    let query = Query::select()
        .expr_as(Expr::col("id").count(), "count")
        .from("pg_loco_queue")
        .and_where(
            Func::cust("jsonb_extract_path_text")
                .args([Expr::col("task_data"), Expr::val("to")])
                .eq(address),
        )
        .to_owned();
    app.fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap()
}

async fn health_sequence(app: &Application) -> (i64, bool) {
    let query = Query::select()
        .columns(["last_value", "is_called"])
        .from("health_events_id_seq")
        .to_owned();
    let row = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap();
    (
        row.try_get("", "last_value").unwrap(),
        row.try_get("", "is_called").unwrap(),
    )
}

async fn wait_for_account_waiters(
    app: &Application,
    blocker: i32,
    predecessor: i32,
    minimum: usize,
) -> Vec<i32> {
    let blocked_by = |pid| {
        Expr::expr(Func::cust("array_position").args([
            Expr::expr(Func::cust("pg_blocking_pids").arg(Expr::col("pid"))),
            Expr::val(pid),
        ]))
        .is_not_null()
    };
    let query = Query::select()
        .column("pid")
        .from("pg_stat_activity")
        .and_where(Expr::col("wait_event_type").eq("Lock"))
        .and_where(blocked_by(blocker).or(blocked_by(predecessor)))
        .to_owned();
    for _ in 0..300 {
        let waiters = app
            .fixture
            .admin
            .query_all_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
            .unwrap();
        if waiters.len() >= minimum {
            return waiters
                .into_iter()
                .map(|row| row.try_get("", "pid").unwrap())
                .collect();
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    Vec::new()
}

#[tokio::test]
async fn concurrent_confirmations_serialize_and_replay_one_restore() {
    let app = Application::new().await;
    let scope = app.fixture.scenario.scope();
    let (ready, source_email) = ready_unique(&app).await;
    let token = ready.confirmation.clone().unwrap();
    let run_id = ready.report.id.clone();
    let live_before = live_effects(&app, &source_email).await;
    let stock_before = medications::Entity::find()
        .filter(medications::Column::HouseholdId.eq(scope.household_id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let dosage_before = dosages::Entity::find()
        .filter(dosages::Column::HouseholdId.eq(scope.household_id))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let tables = [
        ("locations", 2),
        ("medications", 1),
        ("dosages", 1),
        ("schedules", 1),
        ("person_medications", 1),
        ("medication_pause_periods", 1),
        ("medication_takes", 1),
        ("medication_dose_occurrences", 1),
        ("notification_preferences", 1),
        ("health_events", 1),
        ("health_event_medications", 1),
        ("guardian_declarations", 1),
        ("household_memberships", 1),
        ("carer_relationships", 1),
        ("person_schedules", 1),
        ("shared_schedule_executions", 1),
        ("shared_schedule_adoption_facts", 1),
        ("shared_schedule_administrations", 1),
    ];
    let mut before = Vec::new();
    for (table, _) in tables {
        before.push(
            scoped_count(
                &app,
                table,
                if table == "guardian_declarations" || table == "person_schedules" {
                    "origin_household_id"
                } else {
                    "household_id"
                },
            )
            .await,
        );
    }
    let holder = app.fixture.admin.begin().await.unwrap();
    accounts::Entity::find_by_id(app.fixture.account.id)
        .lock_exclusive()
        .one(&holder)
        .await
        .unwrap()
        .unwrap();
    let blocker = super::data_import_consents::backend_pid(&holder).await;
    let spawn_confirm = || {
        let ctx = app.context.clone();
        let scope = scope.clone();
        let token = token.clone();
        let run_id = run_id.clone();
        tokio::spawn(async move {
            let provenance = CredentialProvenance {
                method: CredentialMethod::BrowserSession,
                reference: "synthetic-session".into(),
            };
            confirmation::confirm(
                &ctx,
                &scope,
                &run_id,
                token,
                "password".into(),
                &provenance,
                &scope.request_id,
            )
            .await
        })
    };
    let first = spawn_confirm();
    let first_waiting = wait_for_account_waiters(&app, blocker, blocker, 1).await;
    let second = spawn_confirm();
    let both_waiting = if let Some(first_pid) = first_waiting.first() {
        wait_for_account_waiters(&app, blocker, *first_pid, 2).await
    } else {
        Vec::new()
    };
    holder.rollback().await.unwrap();
    let first_result = tokio::time::timeout(std::time::Duration::from_secs(30), first)
        .await
        .unwrap()
        .unwrap();
    let second_result = tokio::time::timeout(std::time::Duration::from_secs(30), second)
        .await
        .unwrap()
        .unwrap();
    assert!(
        first_waiting.len() == 1
            && both_waiting.len() >= 2
            && both_waiting.contains(&first_waiting[0]),
        "both confirmations must pass crypto and wait on the actor account lock"
    );
    let first_report = first_result.unwrap();
    let second_report = second_result.unwrap();
    assert_eq!(first_report.state, ImportState::Completed);
    assert_eq!(first_report, second_report);
    assert_eq!(
        first_report.totals.created,
        ready.report.totals.expected_create
    );
    assert_eq!(
        first_report.totals.verified_links,
        ready.report.totals.expected_link
    );
    assert!(first_report.totals.created > 0 && first_report.totals.verified_links > 0);
    let records = first_report
        .people
        .iter()
        .flat_map(|p| std::iter::once(&p.person).chain(p.records.iter()))
        .chain(first_report.records.iter());
    assert!(
        records
            .filter(|row| row.expected == ExpectedRecord::Create)
            .all(|row| row.actual == Some(ActualRecord::Created) && row.destination.is_some())
    );
    let edges = first_report
        .people
        .iter()
        .flat_map(|p| p.relationships.iter())
        .chain(first_report.relationships.iter());
    assert!(
        edges
            .filter(|row| row.expected == ExpectedRelationship::Link)
            .all(|row| row.actual == Some(ActualRelationship::Verified))
    );
    let mut after = Vec::new();
    for ((table, expected), original) in tables.into_iter().zip(before) {
        let column = if table == "guardian_declarations" || table == "person_schedules" {
            "origin_household_id"
        } else {
            "household_id"
        };
        let actual = scoped_count(&app, table, column).await;
        assert_eq!(actual, original + expected, "one created set in {table}");
        after.push(actual);
    }
    let DestinationId::Medication(id) = destination(&first_report, RecordKind::Medication) else {
        panic!("medication destination")
    };
    let medication = medications::Entity::find_by_id(id.0)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(medication.current_supply.unwrap().to_string(), "37.50");
    let DestinationId::DosageOption(id) = destination(&first_report, RecordKind::DosageOption)
    else {
        panic!("dosage destination")
    };
    let dosage = dosages::Entity::find_by_id(id.0)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(dosage.current_supply.unwrap().to_string(), "8.50");
    for previous in stock_before {
        assert_eq!(
            medications::Entity::find_by_id(previous.id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap()
                .current_supply,
            previous.current_supply
        );
    }
    for previous in dosage_before {
        assert_eq!(
            dosages::Entity::find_by_id(previous.id)
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .unwrap()
                .current_supply,
            previous.current_supply
        );
    }
    assert_eq!(live_effects(&app, &source_email).await, live_before);
    let provenance = CredentialProvenance {
        method: CredentialMethod::BrowserSession,
        reference: "synthetic-session".into(),
    };
    let replay = confirmation::confirm(
        &app.context,
        &scope,
        &run_id,
        token,
        "password".into(),
        &provenance,
        &scope.request_id,
    )
    .await
    .unwrap();
    assert_eq!(replay, first_report);
    assert_eq!(
        medications::Entity::find_by_id(medication.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .current_supply,
        medication.current_supply
    );
    assert_eq!(
        dosages::Entity::find_by_id(dosage.id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .current_supply,
        dosage.current_supply
    );
    for ((table, _), expected) in tables.into_iter().zip(after) {
        let column = if table == "guardian_declarations" || table == "person_schedules" {
            "origin_household_id"
        } else {
            "household_id"
        };
        assert_eq!(
            scoped_count(&app, table, column).await,
            expected,
            "replay must not create another {table} row"
        );
    }
    assert_eq!(live_effects(&app, &source_email).await, live_before);
    app.close().await;
}

#[tokio::test]
async fn late_health_event_failure_rolls_back_import_and_persists_failed_report() {
    let app = Application::new().await;
    let scope = app.fixture.scenario.scope();
    let ready = ready(&app).await;
    let tables = [
        ("household_memberships", "household_id"),
        ("guardian_declarations", "origin_household_id"),
        ("carer_relationships", "household_id"),
        ("person_access_grants", "household_id"),
        ("locations", "household_id"),
        ("medications", "household_id"),
        ("dosages", "household_id"),
        ("schedules", "household_id"),
        ("person_medications", "household_id"),
        ("medication_takes", "household_id"),
        ("medication_pause_periods", "household_id"),
        ("medication_dose_occurrences", "household_id"),
        ("notification_preferences", "household_id"),
        ("health_events", "household_id"),
        ("health_event_medications", "household_id"),
        ("versions", "household_id"),
        ("api_change_events", "household_id"),
        ("person_schedules", "origin_household_id"),
        ("shared_schedule_adoption_facts", "household_id"),
        ("shared_schedule_administrations", "household_id"),
        ("notification_events", "household_id"),
        ("notification_deliveries", "household_id"),
    ];
    let mut before = Vec::new();
    for (table, column) in tables {
        before.push(scoped_count(&app, table, column).await);
    }
    let stock = medications::Entity::find()
        .filter(medications::Column::HouseholdId.eq(scope.household_id))
        .order_by_asc(medications::Column::Id)
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let dosage_stock = dosages::Entity::find()
        .filter(dosages::Column::HouseholdId.eq(scope.household_id))
        .order_by_asc(dosages::Column::Id)
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let people_before = people::Entity::find()
        .filter(people::Column::PortableId.eq("synthetic-rails-person-prepared-history-v2"))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let member_people = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(scope.household_id))
        .all(&app.fixture.admin)
        .await
        .unwrap()
        .into_iter()
        .map(|member| member.person_id)
        .collect::<Vec<_>>();
    let mut addresses = accounts::Entity::find()
        .filter(accounts::Column::PersonId.is_in(member_people))
        .all(&app.fixture.admin)
        .await
        .unwrap()
        .into_iter()
        .map(|account| account.email)
        .collect::<Vec<_>>();
    addresses.push("portable-fixture-prepared-history-v2@example.test".into());
    addresses.sort();
    addresses.dedup();
    let mut mail_before = Vec::new();
    for address in &addresses {
        mail_before.push(queued_mail_count(&app, address).await);
    }
    // This sequence has CACHE 1 and advances even when the child transaction rolls back.
    let sequence_before = health_sequence(&app).await;
    // SeaQuery can add a column CHECK but has no table-alter add-CHECK operation.
    let add = Table::alter()
        .table("health_events")
        .add_column(
            ColumnDef::new("task_43_reject_fixture").boolean().check((
                "task_43_health_rejection",
                Expr::col("household_id")
                    .ne(scope.household_id)
                    .or(Expr::col("title").ne("Synthetic health event")),
            )),
        )
        .to_owned();
    app.fixture
        .admin
        .execute_raw(app.fixture.admin.get_database_backend().build(&add))
        .await
        .unwrap();
    let outcome = AssertUnwindSafe(async {
        let provenance = CredentialProvenance {
            method: CredentialMethod::BrowserSession,
            reference: "synthetic-session".into(),
        };
        let report = confirmation::confirm(
            &app.context,
            &scope,
            &ready.report.id,
            ready.confirmation.unwrap(),
            "password".into(),
            &provenance,
            &scope.request_id,
        )
        .await
        .unwrap();
        assert_eq!(report.state, ImportState::Failed);
        let sequence_after = health_sequence(&app).await;
        assert!(
            sequence_after.0 > sequence_before.0 || (!sequence_before.1 && sequence_after.1),
            "the late health event INSERT must reach its sequence"
        );
        assert_eq!(report.totals.created, 0);
        assert_eq!(report.totals.matched, 0);
        assert_eq!(report.totals.verified_links, 0);
        let records = report
            .people
            .iter()
            .flat_map(|p| std::iter::once(&p.person).chain(p.records.iter()))
            .chain(report.records.iter());
        assert!(
            records
                .into_iter()
                .all(|row| row.actual != Some(ActualRecord::Created)
                    && row.actual != Some(ActualRecord::Matched)
                    && row.destination.is_none())
        );
        let edges = report
            .people
            .iter()
            .flat_map(|p| p.relationships.iter())
            .chain(report.relationships.iter());
        assert!(
            edges
                .into_iter()
                .all(|row| row.actual != Some(ActualRelationship::Verified))
        );
        let tenant = med_tracker::models::access::begin(
            &app.fixture.runtime,
            &scope,
            app.context.config.settings.as_ref(),
        )
        .await
        .unwrap();
        let saved = med_tracker::models::data_imports::persistence::find(&tenant, &report.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(saved.snapshot.report, report);
        assert!(saved.snapshot.confirmation.is_none());
        tenant.rollback().await.unwrap();
        for ((table, column), expected) in tables.into_iter().zip(before) {
            assert_eq!(
                scoped_count(&app, table, column).await,
                expected,
                "partial rows in {table}"
            );
        }
        let people_after = people::Entity::find()
            .filter(people::Column::PortableId.eq("synthetic-rails-person-prepared-history-v2"))
            .count(&app.fixture.admin)
            .await
            .unwrap();
        assert_eq!(
            people_after, people_before,
            "dependent creation must roll back"
        );
        assert_eq!(
            medications::Entity::find()
                .filter(medications::Column::HouseholdId.eq(scope.household_id))
                .order_by_asc(medications::Column::Id)
                .all(&app.fixture.admin)
                .await
                .unwrap(),
            stock
        );
        assert_eq!(
            dosages::Entity::find()
                .filter(dosages::Column::HouseholdId.eq(scope.household_id))
                .order_by_asc(dosages::Column::Id)
                .all(&app.fixture.admin)
                .await
                .unwrap(),
            dosage_stock
        );
        for (address, expected) in addresses.iter().zip(mail_before) {
            assert_eq!(queued_mail_count(&app, address).await, expected);
        }
    })
    .catch_unwind()
    .await;
    // Run both cleanup statements before propagating an assertion panic.
    let drop_check = Table::alter()
        .table("health_events")
        .drop_constraint("task_43_health_rejection")
        .to_owned();
    let cleanup_check = app
        .fixture
        .admin
        .execute_raw(app.fixture.admin.get_database_backend().build(&drop_check))
        .await;
    let drop_column = Table::alter()
        .table("health_events")
        .drop_column("task_43_reject_fixture")
        .to_owned();
    let cleanup_column = app
        .fixture
        .admin
        .execute_raw(app.fixture.admin.get_database_backend().build(&drop_column))
        .await;
    app.close().await;
    cleanup_check.expect("remove temporary health event check");
    cleanup_column.expect("remove temporary health event column");
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
