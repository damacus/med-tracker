use med_tracker::models::{
    access::{self, Actor, HouseholdScope},
    care::medications::{self, AdjustStock, Command, ScalarPrecondition},
    entities::medication,
    errors::OperationError,
};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};
#[path = "care_medications/stock_removals.rs"]
mod stock_removals;

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
        let name = format!("care_medications_{suffix}");
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
            .execute_unprepared(include_str!("fixtures/care-medications.sql"))
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

    async fn effect(&self) -> (String, i64, i64) {
        let row = self.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT current_supply::text AS supply,
                    (SELECT count(*) FROM versions WHERE item_type='Medication' AND item_id=90001) AS audits,
                    (SELECT count(*) FROM api_change_events WHERE record_type='Medication' AND record_id=90001) AS changes
             FROM medications WHERE id=90001"
        )).await.unwrap().unwrap();
        (
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
        request_id: "synthetic-stock-adjustment".into(),
    }
}

fn command(quantity: &str) -> Command {
    Command::AdjustStock(AdjustStock {
        medication_id: "90001".into(),
        new_quantity: quantity.into(),
        reason: Some("cycle count".into()),
    })
}

async fn adjust(fixture: &Fixture, command: Command) -> Result<medication::Model, OperationError> {
    let tenant = access::begin(&fixture.runtime, &scope()).await?;
    match medications::execute(&tenant, command).await {
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
async fn stock_adjustment_atomically_records_quantity_audit_and_change() {
    let fixture = Fixture::new().await;
    let result = adjust(&fixture, command("12.25")).await.unwrap();
    assert_eq!(result.current_supply.unwrap().to_string(), "12.25");
    assert_eq!(fixture.effect().await, ("12.25".into(), 1, 1));
    let row=fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT event, whodunnit, request_id, audit_context::text AS context, object::jsonb->>'current_supply' AS previous FROM versions WHERE item_type='Medication' AND item_id=90001"
    )).await.unwrap().unwrap();
    assert_eq!(
        row.try_get::<String>("", "event").unwrap(),
        "adjust inventory (qty: 12.25, reason: cycle count)"
    );
    assert_eq!(row.try_get::<String>("", "previous").unwrap(), "10.00");
    assert_eq!(row.try_get::<String>("", "whodunnit").unwrap(), "77001");
    assert_eq!(
        row.try_get::<String>("", "request_id").unwrap(),
        scope().request_id
    );
    let context: serde_json::Value =
        serde_json::from_str(&row.try_get::<String>("", "context").unwrap()).unwrap();
    assert_eq!(context["actor_account_id"], 71001);
    assert_eq!(context["actor_membership_id"], 74001);
    assert_eq!(context["active_role"], "administrator");
    assert_eq!(context["policy_class"], "MedicationPolicy");
    assert_eq!(context["policy_query"], "adjust_inventory?");
    fixture.close().await;
}

#[tokio::test]
async fn stock_adjustment_preserves_exact_decimal_bounds_and_zero() {
    let fixture = Fixture::new().await;
    for quantity in ["0", "0.01", "99999999.99"] {
        let result = adjust(&fixture, command(quantity)).await.unwrap();
        assert_eq!(
            result.current_supply.unwrap().normalize().to_string(),
            quantity
        );
    }
    assert_eq!(fixture.effect().await, ("99999999.99".into(), 3, 3));
    for quantity in ["-1", "1.001", "100000000", "invalid"] {
        assert!(
            matches!(
                adjust(&fixture, command(quantity)).await,
                Err(OperationError::Validation { .. })
            ),
            "{quantity}"
        );
        assert_eq!(fixture.effect().await, ("99999999.99".into(), 3, 3));
    }
    fixture.close().await;
}

#[tokio::test]
async fn stock_adjustment_member_manage_grant_does_not_elevate_household_role() {
    let fixture = Fixture::new().await;
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    assert_eq!(
        adjust(&fixture, command("12.25")).await.unwrap_err(),
        OperationError::Forbidden
    );
    assert_eq!(fixture.effect().await, ("10.00".into(), 0, 0));
    fixture.close().await;
}

#[tokio::test]
async fn stock_adjustment_cross_household_id_is_hidden() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(92001,71001,'Other synthetic household','other-stock-household','UTC',now(),now()); INSERT INTO locations(id,household_id,name,created_at,updated_at) VALUES(92002,92001,'Other stock cabinet',now(),now()); INSERT INTO medications(id,household_id,location_id,name,current_supply,created_at,updated_at) VALUES(92003,92001,92002,'Other stock tablets',20,now(),now())").await.unwrap();
    let Command::AdjustStock(mut input) = command("12.25");
    input.medication_id = "92003".into();
    assert_eq!(
        adjust(&fixture, Command::AdjustStock(input))
            .await
            .unwrap_err(),
        OperationError::NotFound
    );
    assert_eq!(fixture.effect().await, ("10.00".into(), 0, 0));
    let row = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT current_supply::text AS supply FROM medications WHERE id=92003",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "supply").unwrap(), "20.00");
    fixture.close().await;
}

#[tokio::test]
async fn stock_adjustment_rechecks_role_after_transaction_begin() {
    let fixture = Fixture::new().await;
    let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    assert_eq!(
        medications::execute(&tenant, command("12.25"))
            .await
            .unwrap_err(),
        OperationError::Forbidden
    );
    tenant.rollback().await.unwrap();
    assert_eq!(fixture.effect().await, ("10.00".into(), 0, 0));
    fixture.close().await;
}

#[tokio::test]
async fn stock_adjustment_audit_rejection_rolls_back_an_attempted_stock_write() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("CREATE SEQUENCE synthetic_stock_audit_attempts; GRANT USAGE ON SEQUENCE synthetic_stock_audit_attempts TO med_tracker_app; CREATE FUNCTION synthetic_stock_audit_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='Medication' AND NEW.item_id=90001 THEN IF (SELECT current_supply FROM medications WHERE id=90001)=12.25 THEN PERFORM nextval('synthetic_stock_audit_attempts'); END IF; RAISE EXCEPTION 'synthetic stock audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER synthetic_stock_audit_failure BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION synthetic_stock_audit_failure()").await.unwrap();
    assert_eq!(
        adjust(&fixture, command("12.25")).await.unwrap_err(),
        OperationError::Unavailable
    );
    let row = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT is_called FROM synthetic_stock_audit_attempts",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(
        row.try_get::<bool>("", "is_called").unwrap(),
        "The operation never reached audit after changing stock"
    );
    assert_eq!(fixture.effect().await, ("10.00".into(), 0, 0));
    fixture.close().await;
}

#[tokio::test]
async fn stock_adjustment_concurrent_counts_preserve_audit_predecessors() {
    let fixture = Fixture::new().await;
    let (first, second) = tokio::join!(
        adjust(&fixture, command("11")),
        adjust(&fixture, command("12"))
    );
    assert!(
        first.is_ok() && second.is_ok(),
        "Concurrent stock writes failed: {first:?} / {second:?}"
    );
    let effect = fixture.effect().await;
    assert!(matches!(effect.0.as_str(), "11.00" | "12.00"));
    assert_eq!((effect.1, effect.2), (2, 2));
    let rows=fixture.admin.query_all_raw(Statement::from_string(DbBackend::Postgres,"SELECT object::jsonb->>'current_supply' AS previous FROM versions WHERE item_type='Medication' AND item_id=90001 ORDER BY id")).await.unwrap();
    assert_eq!(rows[0].try_get::<String>("", "previous").unwrap(), "10.00");
    assert_eq!(
        rows[1].try_get::<String>("", "previous").unwrap(),
        if effect.0 == "12.00" {
            "11.00"
        } else {
            "12.00"
        }
    );
    fixture.close().await;
}

async fn guarded_adjust(
    fixture: &Fixture,
    quantity: &str,
    etag: &str,
) -> Result<medication::Model, OperationError> {
    let tenant = access::begin(&fixture.runtime, &scope()).await?;
    let result = medications::execute_with_options(
        &tenant,
        command(quantity),
        Some(&ScalarPrecondition {
            original_etag: etag.into(),
        }),
        None,
    )
    .await;
    match result {
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

async fn current_etag(fixture: &Fixture) -> String {
    let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
    let snapshot = medications::read_stock_snapshot(&tenant, "90001")
        .await
        .unwrap();
    tenant.rollback().await.unwrap();
    snapshot.etag
}

#[tokio::test]
async fn stock_adjustment_browser_precondition_rejects_stale_invalid_drafts() {
    let fixture = Fixture::new().await;
    let etag = current_etag(&fixture).await;
    guarded_adjust(&fixture, "12", &etag).await.unwrap();
    assert!(matches!(
        guarded_adjust(&fixture, "invalid", &etag).await,
        Err(OperationError::Conflict { .. })
    ));
    assert_eq!(fixture.effect().await, ("12.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn stock_adjustment_concurrent_browser_snapshots_allow_only_one_write() {
    let fixture = Fixture::new().await;
    let etag = current_etag(&fixture).await;
    let (first, second) = tokio::join!(
        guarded_adjust(&fixture, "11", &etag),
        guarded_adjust(&fixture, "12", &etag)
    );
    assert!(matches!(
        (&first, &second),
        (Ok(_), Err(OperationError::Conflict { .. }))
            | (Err(OperationError::Conflict { .. }), Ok(_))
    ));
    let effect = fixture.effect().await;
    assert!(matches!(effect.0.as_str(), "11.00" | "12.00"));
    assert_eq!((effect.1, effect.2), (1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn stock_adjustment_scalar_guard_excludes_options_but_api_preserves_parent_contract() {
    let fixture = Fixture::new().await;
    let etag = current_etag(&fixture).await;
    fixture.admin.execute_unprepared("INSERT INTO dosages(id, household_id, medication_id, amount, unit, frequency, current_supply, default_dose_cycle, default_max_daily_doses, default_min_hours_between_doses, created_at, updated_at) VALUES (95001,72001,90001,2,'tablet','daily',6,0,4,0,'2020-01-01','2020-01-01')").await.unwrap();
    assert!(matches!(
        guarded_adjust(&fixture, "12", &etag).await,
        Err(OperationError::Conflict { .. })
    ));
    assert_eq!(fixture.effect().await, ("10.00".into(), 0, 0));
    adjust(&fixture, command("12")).await.unwrap();
    let option = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT current_supply::text AS supply FROM dosages WHERE id=95001",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(option.try_get::<String>("", "supply").unwrap(), "6.00");
    assert_eq!(fixture.effect().await, ("12.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn stock_adjustment_portable_id_and_trusted_provenance_preserve_audit() {
    let fixture = Fixture::new().await;
    let tenant = access::begin(&fixture.runtime, &scope()).await.unwrap();
    let snapshot = medications::read_stock_snapshot(&tenant, "90001")
        .await
        .unwrap();
    let provenance = med_tracker::models::care::doses::CredentialProvenance {
        method: med_tracker::models::care::doses::CredentialMethod::OauthGrant,
        reference: "synthetic-validated-grant".into(),
    };
    let Command::AdjustStock(mut input) = command("12");
    input.medication_id = snapshot.medication.portable_id;
    medications::execute_with_options(
        &tenant,
        Command::AdjustStock(input),
        None,
        Some(&provenance),
    )
    .await
    .unwrap();
    tenant.commit().await.unwrap();
    let row = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT audit_context FROM versions WHERE item_type='Medication' AND item_id=90001",
        ))
        .await
        .unwrap()
        .unwrap();
    let audit: serde_json::Value = row.try_get("", "audit_context").unwrap();
    assert_eq!(audit["authentication_method"], "oauth");
    assert_eq!(
        audit["session_reference"],
        "oauth_grant:synthetic-validated-grant"
    );
    assert_eq!(fixture.effect().await, ("12.00".into(), 1, 1));
    fixture.close().await;
}
