use med_tracker::models::{
    access::{self, Actor, HouseholdScope},
    care::doses::{self, Command, Outcome, Take},
    errors::OperationError,
};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};

struct Fixture {
    admin: DatabaseConnection,
    runtime: DatabaseConnection,
}

impl Fixture {
    async fn new() -> Self {
        assert_eq!(std::env::var("MEDTRACKER_OWNED_DATABASE").unwrap(), "1");
        let uri = std::env::var("DATABASE_URL").unwrap();
        let endpoint = uri
            .strip_prefix("postgres://medtracker:medtracker_password@127.0.0.1:")
            .unwrap();
        let (port, _) = endpoint.split_once('/').unwrap();
        assert!(port.parse::<u16>().unwrap() > 0);
        let suffix = uuid::Uuid::new_v4().simple();
        let name = format!("care_doses_{suffix}");
        let role = format!("care_runtime_{suffix}");
        let control = Database::connect(&uri).await.unwrap();
        control
            .execute_unprepared(&format!(
                "CREATE DATABASE {name} TEMPLATE medtracker_reference"
            ))
            .await
            .unwrap();
        control.execute_unprepared(&format!("CREATE ROLE {role} LOGIN PASSWORD 'password' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS IN ROLE med_tracker_app")).await.unwrap();
        control.close().await.unwrap();
        let admin = Database::connect(format!(
            "postgres://medtracker:medtracker_password@127.0.0.1:{port}/{name}"
        ))
        .await
        .unwrap();
        <migration::Migrator as migration::MigratorTrait>::up(&admin, None)
            .await
            .unwrap();
        admin
            .execute_unprepared(include_str!("fixtures/persistence-records.sql"))
            .await
            .unwrap();
        admin
            .execute_unprepared(include_str!("fixtures/care-doses.sql"))
            .await
            .unwrap();
        let runtime = Database::connect(format!(
            "postgres://{role}:password@127.0.0.1:{port}/{name}"
        ))
        .await
        .unwrap();
        Self { admin, runtime }
    }

    async fn close(self) {
        self.runtime.close().await.unwrap();
        self.admin.close().await.unwrap();
    }

    async fn effect(&self) -> (i64, String, i64, i64) {
        let row = self.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM medication_takes) AS takes,
                    (SELECT current_supply::text FROM medications WHERE id = 80001) AS supply,
                    (SELECT count(*) FROM versions WHERE item_type = 'MedicationTake' AND event = 'create') AS audits,
                    (SELECT count(*) FROM api_change_events WHERE record_type = 'MedicationTake' AND action = 'create') AS changes"
        )).await.unwrap().unwrap();
        (
            row.try_get("", "takes").unwrap(),
            row.try_get("", "supply").unwrap(),
            row.try_get("", "audits").unwrap(),
            row.try_get("", "changes").unwrap(),
        )
    }
}

fn scope() -> HouseholdScope {
    HouseholdScope {
        actor: Actor { account_id: 71001 },
        household_id: 72001,
        request_id: "synthetic-dose-request".into(),
    }
}

fn command() -> Command {
    Command::Take(Take {
        client_uuid: Some("006b49d9-1da2-42f1-800b-8c80867aee1c".into()),
        source_type: "person_medication".into(),
        source_id: "81001".into(),
        taken_at: "2026-10-05T10:00:00Z".into(),
        dose_amount: Some("2".into()),
        dose_unit: Some("tablet".into()),
        taken_from_medication_id: Some(80001),
        expected_effective_amount: None,
        expected_effective_unit: None,
    })
}

async fn take(fixture: &Fixture, command: Command) -> Result<Outcome, OperationError> {
    let tenant = access::begin(&fixture.runtime, &scope()).await?;
    match doses::execute(&tenant, command).await {
        Ok(outcome) => {
            tenant.commit().await?;
            Ok(outcome)
        }
        Err(error) => {
            tenant.rollback().await?;
            Err(error)
        }
    }
}

#[tokio::test]
async fn dose_take_atomically_records_stock_and_clinical_audit() {
    let fixture = Fixture::new().await;
    assert!(matches!(
        take(&fixture, command()).await,
        Ok(Outcome::Created(_))
    ));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_duplicate_replay_does_not_repeat_clinical_effects() {
    let fixture = Fixture::new().await;
    assert!(matches!(
        take(&fixture, command()).await,
        Ok(Outcome::Created(_))
    ));
    assert!(matches!(
        take(&fixture, command()).await,
        Ok(Outcome::Replayed(_))
    ));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_rejects_payload_mismatch_without_writes() {
    let fixture = Fixture::new().await;
    assert!(take(&fixture, command()).await.is_ok());
    let Command::Take(mut changed) = command();
    changed.dose_amount = Some("3".into());
    assert!(matches!(
        take(&fixture, Command::Take(changed)).await,
        Err(OperationError::Conflict { .. })
    ));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_insufficient_stock_has_no_partial_writes() {
    let fixture = Fixture::new().await;
    fixture
        .admin
        .execute_unprepared("UPDATE medications SET current_supply = 1 WHERE id = 80001")
        .await
        .unwrap();
    assert!(matches!(
        take(&fixture, command()).await,
        Err(OperationError::Validation { .. })
    ));
    assert_eq!(fixture.effect().await, (0, "1.00".into(), 0, 0));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_rechecks_current_grant_in_open_transaction() {
    let fixture = Fixture::new().await;
    let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
    fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at = now() WHERE id = 78001")
        .await
        .unwrap();
    let result = doses::execute(&tenant, command()).await;
    assert_eq!(result.unwrap_err(), OperationError::NotFound);
    tenant.rollback().await.unwrap();
    assert_eq!(fixture.effect().await, (0, "10.00".into(), 0, 0));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_audit_failure_rolls_back_stock_and_take() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("CREATE SEQUENCE public.synthetic_audit_attempts; GRANT USAGE ON SEQUENCE public.synthetic_audit_attempts TO med_tracker_app; CREATE FUNCTION public.reject_synthetic_take_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type = 'MedicationTake' THEN IF EXISTS (SELECT 1 FROM medication_takes WHERE id = NEW.item_id) AND (SELECT current_supply FROM medications WHERE id = 80001) = 8 THEN PERFORM nextval('public.synthetic_audit_attempts'); END IF; RAISE EXCEPTION 'synthetic audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_synthetic_take_audit BEFORE INSERT ON public.versions FOR EACH ROW EXECUTE FUNCTION public.reject_synthetic_take_audit()").await.unwrap();
    assert!(matches!(
        take(&fixture, command()).await,
        Err(OperationError::Unavailable)
    ));
    let attempt = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT is_called FROM public.synthetic_audit_attempts",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(
        attempt.try_get::<bool>("", "is_called").unwrap(),
        "The failure never reached audit after inserting the take and decrementing stock"
    );
    assert_eq!(fixture.effect().await, (0, "10.00".into(), 0, 0));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_concurrent_uuid_has_one_clinical_effect() {
    let fixture = Fixture::new().await;
    let (first, second) = tokio::join!(take(&fixture, command()), take(&fixture, command()));
    assert!(first.is_ok() && second.is_ok());
    assert!(matches!(
        (&first, &second),
        (Ok(Outcome::Created(_)), Ok(Outcome::Replayed(_)))
            | (Ok(Outcome::Replayed(_)), Ok(Outcome::Created(_)))
    ));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

async fn tracked_fixture(fixture: &Fixture) {
    fixture.admin.execute_unprepared("INSERT INTO dosages(id, household_id, medication_id, amount, unit, frequency, current_supply, default_dose_cycle, default_max_daily_doses, default_min_hours_between_doses, created_at, updated_at) VALUES (82001,72001,80001,2,'tablet','daily',6,0,4,0,'2020-01-01','2020-01-01'),(82002,72001,80001,1,'tablet','daily',4,0,4,0,'2020-01-01','2020-01-01'); UPDATE person_medications SET source_dosage_option_id=82001 WHERE id=81001").await.unwrap();
}

#[tokio::test]
async fn dose_take_tracked_option_updates_stock_aggregate_timestamp_and_audits() {
    let fixture = Fixture::new().await;
    tracked_fixture(&fixture).await;
    assert!(matches!(
        take(&fixture, command()).await,
        Ok(Outcome::Created(_))
    ));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    let row=fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT current_supply::text AS supply, updated_at > timestamp '2020-01-01' AS touched, (SELECT current_supply::text FROM dosages WHERE id=82002) AS untouched_supply, (SELECT count(*) FROM versions WHERE item_type='MedicationDosageOption') AS audits FROM dosages WHERE id=82001")).await.unwrap().unwrap();
    assert_eq!(row.try_get::<String>("", "supply").unwrap(), "4.00");
    assert_eq!(
        row.try_get::<String>("", "untouched_supply").unwrap(),
        "4.00"
    );
    assert_eq!(row.try_get::<i64>("", "audits").unwrap(), 1);
    assert!(
        row.try_get::<bool>("", "touched").unwrap(),
        "The tracked dosage timestamp did not change with its stock"
    );
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_unrelated_unique_collision_is_not_a_replay_conflict() {
    let fixture = Fixture::new().await;
    assert!(matches!(
        take(&fixture, command()).await,
        Ok(Outcome::Created(_))
    ));
    fixture.admin.execute_unprepared("CREATE FUNCTION public.synthetic_portable_collision() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.portable_id := (SELECT portable_id FROM medication_takes LIMIT 1); RETURN NEW; END $$; CREATE TRIGGER synthetic_portable_collision BEFORE INSERT ON medication_takes FOR EACH ROW EXECUTE FUNCTION public.synthetic_portable_collision()").await.unwrap();
    let Command::Take(mut input) = command();
    input.client_uuid = Some("22222222-2222-4222-8222-222222222222".into());
    let result = take(&fixture, Command::Take(input)).await;
    assert_eq!(
        result.unwrap_err(),
        OperationError::Unavailable,
        "An unrelated portable identity collision was swallowed as a client replay conflict"
    );
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_omitted_replay_options_preserve_original_after_default_change() {
    let fixture = Fixture::new().await;
    assert!(matches!(
        take(&fixture, command()).await,
        Ok(Outcome::Created(_))
    ));
    fixture
        .admin
        .execute_unprepared(
            "UPDATE person_medications SET dose_amount=3,dose_unit='capsule' WHERE id=81001",
        )
        .await
        .unwrap();
    let Command::Take(mut input) = command();
    input.dose_amount = None;
    input.dose_unit = None;
    input.taken_from_medication_id = None;
    assert!(matches!(
        take(&fixture, Command::Take(input)).await,
        Ok(Outcome::Replayed(_))
    ));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_explicit_replay_options_must_match_stored_effect() {
    let fixture = Fixture::new().await;
    assert!(matches!(
        take(&fixture, command()).await,
        Ok(Outcome::Created(_))
    ));
    for field in ["amount", "unit", "location"] {
        let Command::Take(mut input) = command();
        match field {
            "amount" => input.dose_amount = Some("3".into()),
            "unit" => input.dose_unit = Some("capsule".into()),
            _ => input.taken_from_medication_id = Some(80002),
        }
        assert!(
            matches!(
                take(&fixture, Command::Take(input)).await,
                Err(OperationError::Conflict { .. })
            ),
            "{field}"
        );
    }
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_paused_and_retired_sources_have_no_clinical_effect() {
    for (sql, retired) in [
        (
            "UPDATE person_medications SET active=false WHERE id=81001",
            false,
        ),
        (
            "UPDATE person_medications SET retired_at=now() WHERE id=81001",
            true,
        ),
    ] {
        let fixture = Fixture::new().await;
        fixture.admin.execute_unprepared(sql).await.unwrap();
        let error = take(&fixture, command()).await.unwrap_err();
        if retired {
            assert_eq!(error, OperationError::NotFound);
        } else {
            assert!(matches!(error, OperationError::Validation { .. }));
        }
        assert_eq!(fixture.effect().await, (0, "10.00".into(), 0, 0));
        fixture.close().await;
    }
}

#[tokio::test]
async fn dose_take_multiple_locations_require_selection_and_preserve_other_stock() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("INSERT INTO locations(id,household_id,name,created_at,updated_at) VALUES(79002,72001,'Other synthetic cabinet',now(),now()); INSERT INTO medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at) VALUES(80002,72001,79002,'Synthetic tablets',10,2,'tablet',now(),now()); INSERT INTO person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at) VALUES(81002,72001,73001,80002,2,'tablet',1,now(),now())").await.unwrap();
    let Command::Take(mut input) = command();
    input.taken_from_medication_id = None;
    assert!(matches!(
        take(&fixture, Command::Take(input)).await,
        Err(OperationError::Validation { .. })
    ));
    assert_eq!(fixture.effect().await, (0, "10.00".into(), 0, 0));
    let Command::Take(mut input) = command();
    input.taken_from_medication_id = Some(80002);
    assert!(matches!(
        take(&fixture, Command::Take(input)).await,
        Ok(Outcome::Created(_))
    ));
    assert_eq!(fixture.effect().await, (1, "10.00".into(), 1, 1));
    let row = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT current_supply::text AS supply FROM medications WHERE id=80002",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "supply").unwrap(), "8.00");
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_ambiguous_and_insufficient_tracked_options_roll_back() {
    for ambiguous in [false, true] {
        let fixture = Fixture::new().await;
        tracked_fixture(&fixture).await;
        if ambiguous {
            fixture.admin.execute_unprepared("UPDATE person_medications SET source_dosage_option_id=NULL WHERE id=81001; UPDATE dosages SET amount=2 WHERE id=82002").await.unwrap();
        } else {
            fixture
                .admin
                .execute_unprepared("UPDATE dosages SET current_supply=1 WHERE id=82001")
                .await
                .unwrap();
        }
        assert!(matches!(
            take(&fixture, command()).await,
            Err(OperationError::Validation { .. })
        ));
        assert_eq!(fixture.effect().await, (0, "10.00".into(), 0, 0));
        let row = fixture
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT count(*) AS total FROM versions",
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<i64>("", "total").unwrap(), 0);
        fixture.close().await;
    }
}

#[tokio::test]
async fn dose_take_schedule_date_kinds_and_taper_amounts() {
    for (kind, config, allowed, amount) in [
        (2, r#"{"weekdays":["monday"]}"#, true, "2.00"),
        (2, r#"{"weekdays":["tuesday"]}"#, false, "2.00"),
        (3, r#"{"dates":["2026-10-05"]}"#, true, "2.00"),
        (3, r#"{"dates":["2026-10-06"]}"#, false, "2.00"),
        (
            5,
            r#"{"taper_steps":[{"start_date":"2026-10-05","end_date":"2026-10-05","amount":"3","unit":"tablet"}]}"#,
            true,
            "3.00",
        ),
        (
            5,
            r#"{"taper_steps":[{"start_date":"2026-10-06","end_date":"2026-10-06","amount":"3"}]}"#,
            false,
            "2.00",
        ),
        (6, "{}", true, "2.00"),
    ] {
        let fixture = Fixture::new().await;
        fixture.admin.execute_unprepared(&format!("INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83001,72001,73001,80001,true,'2026-10-05','2026-10-10',2,'tablet',{kind},'{config}',now(),now())")).await.unwrap();
        let Command::Take(mut input) = command();
        input.source_type = "schedule".into();
        input.source_id = "83001".into();
        input.dose_amount = None;
        let result = take(&fixture, Command::Take(input)).await;
        if allowed {
            let Ok(Outcome::Created(row)) = result else {
                panic!("Expected supported schedule {kind}: {result:?}")
            };
            assert_eq!(row.dose_amount.unwrap().to_string(), amount);
        } else {
            assert!(matches!(result, Err(OperationError::Validation { .. })));
            assert_eq!(fixture.effect().await, (0, "10.00".into(), 0, 0));
        }
        fixture.close().await;
    }
}

#[tokio::test]
async fn public_take_keeps_an_explicit_amount_override_without_browser_confirmation() {
    let fixture = Fixture::new().await;
    let Command::Take(mut input) = command();
    input.dose_amount = Some("3".into());
    let outcome = take(&fixture, Command::Take(input)).await;
    let Ok(Outcome::Created(record)) = outcome else {
        panic!("Expected public dose override: {outcome:?}");
    };
    assert_eq!(record.dose_amount.unwrap().to_string(), "3.00");
    assert_eq!(fixture.effect().await, (1, "7.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_timing_cycle_and_minimum_interval_deny_extra_effects() {
    for cycle in [0, 1, 2] {
        let fixture = Fixture::new().await;
        fixture
            .admin
            .execute_unprepared(&format!(
                "UPDATE person_medications SET max_daily_doses=1,dose_cycle={cycle} WHERE id=81001"
            ))
            .await
            .unwrap();
        assert!(matches!(
            take(&fixture, command()).await,
            Ok(Outcome::Created(_))
        ));
        let Command::Take(mut input) = command();
        input.client_uuid = None;
        input.taken_at = "2026-10-05T11:00:00Z".into();
        assert!(matches!(
            take(&fixture, Command::Take(input)).await,
            Err(OperationError::Validation { .. })
        ));
        assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
        fixture.close().await;
    }
    let fixture = Fixture::new().await;
    fixture
        .admin
        .execute_unprepared(
            "UPDATE person_medications SET min_hours_between_doses=4 WHERE id=81001",
        )
        .await
        .unwrap();
    assert!(matches!(
        take(&fixture, command()).await,
        Ok(Outcome::Created(_))
    ));
    let Command::Take(mut input) = command();
    input.client_uuid = None;
    input.taken_at = "2026-10-05T11:00:00Z".into();
    assert!(matches!(
        take(&fixture, Command::Take(input)).await,
        Err(OperationError::Validation { .. })
    ));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_request_timezone_selects_near_midnight_schedule_date() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared(r#"INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,dose_amount,dose_unit,schedule_type,schedule_config,created_at,updated_at) VALUES(83001,72001,73001,80001,true,'2026-09-28','2026-09-30',2,'tablet',2,'{"weekdays":["tuesday"]}',now(),now())"#).await.unwrap();
    let Command::Take(mut input) = command();
    input.source_type = "schedule".into();
    input.source_id = "83001".into();
    input.taken_at = "2026-09-28T23:30:00Z".into();
    let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
    let utc =
        doses::execute_in_timezone(&tenant, Command::Take(input.clone()), chrono_tz::UTC, None)
            .await;
    assert!(matches!(utc, Err(OperationError::Validation { .. })));
    tenant.rollback().await.unwrap();
    let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
    let london = doses::execute_in_timezone(
        &tenant,
        Command::Take(input),
        chrono_tz::Europe::London,
        None,
    )
    .await;
    assert!(
        matches!(london, Ok(Outcome::Created(_))),
        "The validated request timezone was ignored: {london:?}"
    );
    tenant.commit().await.unwrap();
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_grant_expired_after_begin_has_no_effect() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001; UPDATE person_access_grants SET expires_at=timezone('UTC',clock_timestamp())+interval '100 milliseconds' WHERE id=78001").await.unwrap();
    let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
    fixture
        .admin
        .execute_unprepared("SELECT pg_sleep(0.2)")
        .await
        .unwrap();
    let result = doses::execute(&tenant, command()).await;
    assert_eq!(result.unwrap_err(), OperationError::NotFound);
    tenant.rollback().await.unwrap();
    assert_eq!(fixture.effect().await, (0, "10.00".into(), 0, 0));
    fixture.close().await;
}

async fn take_in_timezone(
    fixture: &Fixture,
    command: Command,
    zone: chrono_tz::Tz,
) -> Result<Outcome, OperationError> {
    let tenant = access::begin(&fixture.runtime, &scope()).await?;
    match doses::execute_in_timezone(&tenant, command, zone, None).await {
        Ok(outcome) => {
            tenant.commit().await?;
            Ok(outcome)
        }
        Err(error) => {
            tenant.rollback().await?;
            Err(error)
        }
    }
}

#[tokio::test]
async fn dose_take_request_timezone_controls_daily_weekly_monthly_and_dst_cycles() {
    for (cycle, first, second, allowed) in [
        (0, "2026-09-28T23:30:00Z", "2026-09-29T00:30:00Z", false),
        (1, "2026-09-27T23:30:00Z", "2026-09-28T00:30:00Z", false),
        (2, "2026-08-31T23:30:00Z", "2026-09-01T00:30:00Z", false),
        (0, "2026-03-29T00:30:00Z", "2026-03-29T23:30:00Z", true),
    ] {
        let fixture = Fixture::new().await;
        fixture
            .admin
            .execute_unprepared(&format!(
                "UPDATE person_medications SET max_daily_doses=1,dose_cycle={cycle} WHERE id=81001"
            ))
            .await
            .unwrap();
        let Command::Take(mut input) = command();
        input.taken_at = first.into();
        assert!(matches!(
            take_in_timezone(
                &fixture,
                Command::Take(input.clone()),
                chrono_tz::Europe::London
            )
            .await,
            Ok(Outcome::Created(_))
        ));
        input.client_uuid = None;
        input.taken_at = second.into();
        let result =
            take_in_timezone(&fixture, Command::Take(input), chrono_tz::Europe::London).await;
        if allowed {
            assert!(matches!(result, Ok(Outcome::Created(_))));
            assert_eq!(fixture.effect().await, (2, "6.00".into(), 2, 2));
        } else {
            assert!(
                matches!(result, Err(OperationError::Validation { .. })),
                "Unexpected zone cycle {cycle}: {result:?}"
            );
            assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
        }
        fixture.close().await;
    }
}

#[tokio::test]
async fn dose_take_member_uses_assignment_stock_and_excludes_unlinked_match() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001; INSERT INTO locations(id,household_id,name,created_at,updated_at) VALUES(79002,72001,'Unlinked synthetic cabinet',now(),now()); INSERT INTO medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_by_membership_id,created_at,updated_at) VALUES(80002,72001,79002,'Synthetic tablets',20,2,'tablet',74001,now(),now())").await.unwrap();
    let Command::Take(mut input) = command();
    input.taken_from_medication_id = None;
    let created = take(&fixture, Command::Take(input)).await;
    let Ok(Outcome::Created(record)) = created else {
        panic!("The canonical member could not take linked stock: {created:?}");
    };
    assert_eq!(record.taken_from_medication_id, Some(80001));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    let Command::Take(mut input) = command();
    input.client_uuid = None;
    input.taken_from_medication_id = Some(80002);
    assert!(matches!(
        take(&fixture, Command::Take(input)).await,
        Err(OperationError::Validation { .. })
    ));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    let unlinked = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT current_supply::text AS supply, (SELECT count(*) FROM person_medications WHERE medication_id=80002) AS assignments FROM medications WHERE id=80002"
    )).await.unwrap().unwrap();
    assert_eq!(unlinked.try_get::<String>("", "supply").unwrap(), "20.00");
    assert_eq!(unlinked.try_get::<i64>("", "assignments").unwrap(), 0);
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_rejects_unrepresentable_tracked_aggregate_without_effects() {
    let fixture = Fixture::new().await;
    tracked_fixture(&fixture).await;
    fixture.admin.execute_unprepared("UPDATE dosages SET current_supply=CASE id WHEN 82001 THEN 60000000 ELSE 50000000 END WHERE id IN (82001,82002)").await.unwrap();
    assert!(matches!(
        take(&fixture, command()).await,
        Err(OperationError::Validation { .. })
    ));
    assert_eq!(fixture.effect().await, (0, "10.00".into(), 0, 0));
    let rows = fixture.admin.query_all_raw(Statement::from_string(DbBackend::Postgres, "SELECT current_supply::text AS supply FROM dosages WHERE id IN (82001,82002) ORDER BY id")).await.unwrap();
    assert_eq!(
        rows[0].try_get::<String>("", "supply").unwrap(),
        "60000000.00"
    );
    assert_eq!(
        rows[1].try_get::<String>("", "supply").unwrap(),
        "50000000.00"
    );
    fixture.close().await;
}

fn uuid_command(client_uuid: String) -> Command {
    let Command::Take(mut input) = command();
    input.client_uuid = Some(client_uuid);
    Command::Take(input)
}

#[tokio::test]
async fn dose_take_uuid_spellings_replay_one_effect() {
    let fixture = Fixture::new().await;
    let id = uuid::Uuid::parse_str("006b49d9-1da2-42f1-800b-8c80867aee1c").unwrap();
    let first = take(
        &fixture,
        uuid_command(id.hyphenated().to_string().to_uppercase()),
    )
    .await
    .unwrap();
    let Outcome::Created(first) = first else {
        panic!("First dose was not created")
    };
    for spelling in [
        id.hyphenated().to_string(),
        id.simple().to_string(),
        id.braced().to_string(),
        id.urn().to_string(),
        id.urn().to_string().to_uppercase(),
    ] {
        let result = take(&fixture, uuid_command(spelling)).await.unwrap();
        assert!(matches!(result, Outcome::Replayed(record) if record.id == first.id));
    }
    assert_eq!(
        first.client_uuid.as_deref(),
        Some("006b49d9-1da2-42f1-800b-8c80867aee1c")
    );
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_uuid_canonical_retry_preserves_historical_spelling() {
    for historical in [
        "006B49D9-1DA2-42F1-800B-8C80867AEE1C",
        "006b49d91da242f1800b8c80867aee1c",
        "{006b49d9-1da2-42f1-800b-8c80867aee1c}",
        "urn:uuid:006b49d9-1da2-42f1-800b-8c80867aee1c",
        "URN:UUID:006B49D9-1DA2-42F1-800B-8C80867AEE1C",
    ] {
        let fixture = Fixture::new().await;
        let Outcome::Created(first) = take(&fixture, command()).await.unwrap() else {
            panic!("First dose was not created")
        };
        fixture
            .admin
            .execute_unprepared(&format!(
                "UPDATE medication_takes SET client_uuid='{historical}' WHERE id={}",
                first.id
            ))
            .await
            .unwrap();
        let replay = take(&fixture, command()).await.unwrap();
        assert!(
            matches!(replay, Outcome::Replayed(record) if record.id == first.id && record.client_uuid.as_deref() == Some(historical))
        );
        assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
        fixture.close().await;
    }
}

#[tokio::test]
async fn dose_take_uuid_concurrent_variants_share_one_effect() {
    let fixture = Fixture::new().await;
    let id = uuid::Uuid::parse_str("006b49d9-1da2-42f1-800b-8c80867aee1c").unwrap();
    let (first, second) = tokio::join!(
        take(&fixture, uuid_command(id.urn().to_string())),
        take(
            &fixture,
            uuid_command(id.hyphenated().to_string().to_uppercase())
        )
    );
    assert!(matches!(
        (&first, &second),
        (Ok(Outcome::Created(_)), Ok(Outcome::Replayed(_)))
            | (Ok(Outcome::Replayed(_)), Ok(Outcome::Created(_)))
    ));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_uuid_historical_duplicate_is_a_stable_conflict() {
    let fixture = Fixture::new().await;
    take(&fixture, command()).await.unwrap();
    fixture
        .admin
        .execute_unprepared("DROP INDEX IF EXISTS index_medication_takes_on_client_uuid_canonical")
        .await
        .unwrap();
    fixture.admin.execute_unprepared("INSERT INTO medication_takes(household_id, client_uuid, person_medication_id, taken_from_medication_id, taken_from_location_id, dose_amount, dose_unit, taken_at, created_at, updated_at) SELECT household_id, upper(client_uuid), person_medication_id, taken_from_medication_id, taken_from_location_id, dose_amount, dose_unit, taken_at, created_at, updated_at FROM medication_takes LIMIT 1").await.unwrap();
    let before = fixture.effect().await;
    assert!(matches!(
        take(&fixture, command()).await,
        Err(OperationError::Conflict { .. })
    ));
    assert_eq!(fixture.effect().await, before);
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_uuid_foreign_historical_collision_is_hidden_without_new_effect() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(92001,71001,'Other synthetic household','uuid-foreign-household','UTC',now(),now()); INSERT INTO people(id,household_id,name,person_type,has_capacity,created_at,updated_at) VALUES(92002,92001,'Other synthetic adult',0,true,now(),now()); INSERT INTO locations(id,household_id,name,created_at,updated_at) VALUES(92003,92001,'Other synthetic cabinet',now(),now()); INSERT INTO medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at) VALUES(92004,92001,92003,'Other synthetic tablets',10,2,'tablet',now(),now()); INSERT INTO person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at) VALUES(92005,92001,92002,92004,2,'tablet',0,now(),now()); INSERT INTO medication_takes(id,household_id,client_uuid,person_medication_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(92006,92001,'006B49D9-1DA2-42F1-800B-8C80867AEE1C',92005,2,'tablet','2026-10-05T10:00:00',now(),now())").await.unwrap();
    let before = fixture.effect().await;
    let error = take(&fixture, command()).await.unwrap_err();
    assert!(
        matches!(&error, OperationError::Conflict { code, .. } if code == "idempotency_key_unavailable")
    );
    assert!(!format!("{error:?}").contains("006B49D9-1DA2-42F1-800B-8C80867AEE1C"));
    assert_eq!(fixture.effect().await, before);
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_conformance_backdated_interval_preserves_preceding_neighbour_policy() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("UPDATE person_medications SET max_daily_doses=4,min_hours_between_doses=4 WHERE id=81001").await.unwrap();
    for time in ["2026-10-05T10:00:00Z", "2026-10-05T18:00:00Z"] {
        let Command::Take(mut input) = command();
        input.client_uuid = None;
        input.taken_at = time.into();
        assert!(matches!(
            take(&fixture, Command::Take(input)).await,
            Ok(Outcome::Created(_))
        ));
    }
    let Command::Take(mut input) = command();
    input.client_uuid = None;
    input.taken_at = "2026-10-05T16:00:00Z".into();
    assert!(matches!(
        take(&fixture, Command::Take(input)).await,
        Ok(Outcome::Created(_))
    ));
    assert_eq!(fixture.effect().await, (3, "4.00".into(), 3, 3));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_review_skipped_midnight_keeps_local_daily_cycles_separate() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("UPDATE person_medications SET max_daily_doses=1,min_hours_between_doses=0,dose_cycle=0 WHERE id=81001").await.unwrap();
    for time in ["2025-09-07T02:30:00Z", "2025-09-07T04:30:00Z"] {
        let Command::Take(mut input) = command();
        input.client_uuid = None;
        input.taken_at = time.into();
        let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
        let result = doses::execute_in_timezone(
            &tenant,
            Command::Take(input),
            chrono_tz::America::Santiago,
            None,
        )
        .await;
        if result.is_ok() {
            tenant.commit().await.unwrap();
        } else {
            tenant.rollback().await.unwrap();
        }
        assert!(
            matches!(result, Ok(Outcome::Created(_))),
            "The preceding local day was counted across the skipped midnight: {result:?}"
        );
    }
    assert_eq!(fixture.effect().await, (2, "6.00".into(), 2, 2));
    fixture.close().await;
}
