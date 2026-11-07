use super::Application;
use futures_util::FutureExt;
use med_tracker::models::{
    _entities::{
        accounts, dosages, health_event_medications, health_events, household_memberships,
        locations, medication_dose_occurrences, medication_pause_periods, medication_takes,
        medications, notification_preferences, people, person_medications, schedules,
    },
    data_imports::{
        confirmation,
        contracts::{
            ActualRecord, ActualRelationship, ConsentInput, DestinationId, ExpectedRecord,
            ExpectedRelationship, ImportReport, ImportState, PersonDestination, PersonMapping,
            PreviewResult, RecordKind, SourceSystem,
        },
        inspection::{self, Upload},
        preview,
    },
    identity::credentials::{CredentialMethod, CredentialProvenance},
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
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
    let ready = ready(&app).await;
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
    assert_eq!(med.current_supply.unwrap().to_string(), "37.50");
    let dose = dosages::Entity::find_by_id(dosage.0)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(dose.medication_id, med.id);
    assert_eq!(dose.current_supply.unwrap().to_string(), "8.50");
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
    let pause = medication_pause_periods::Entity::find_by_id(pause.0)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pause.schedule_id, Some(schedule.id));
    assert_eq!(pause.reason, "out_of_supply");
    let occurrence = medication_dose_occurrences::Entity::find_by_id(occurrence.0)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(occurrence.schedule_id, Some(schedule.id));
    assert_eq!(occurrence.outcome, "not_taken");
    assert_eq!(occurrence.reason.as_deref(), Some("refused"));
    let preference = notification_preferences::Entity::find_by_id(preference.0)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(preference.person_id, person.0);
    assert!(preference.enabled && preference.missed_dose_enabled && !preference.dose_due_enabled);
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
    app.close().await;
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
