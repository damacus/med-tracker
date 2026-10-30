#[path = "support/clinical_fixture.rs"]
mod clinical_fixture;
#[path = "support/scenario.rs"]
pub mod scenario;

use med_tracker::models::{
    access::{self, HouseholdScope},
    care::medications::{self, AdjustStock, Command, Restock, ScalarPrecondition},
    entities::{
        api_change_event, api_tombstone, dosage, grant, medication, medication_take, membership,
        person_medication, version,
    },
    errors::OperationError,
};
use scenario::Scenario;
use sea_orm::prelude::Decimal;
use sea_orm::sea_query::ExprTrait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};

struct Fixture {
    scenario: Scenario,
    scope: HouseholdScope,
    location_id: i64,
    medication_id: i64,
    submission_id: String,
}

impl std::ops::Deref for Fixture {
    type Target = Scenario;

    fn deref(&self) -> &Self::Target {
        &self.scenario
    }
}

async fn seed_medication(scenario: &Scenario, supply: Decimal) -> (i64, i64) {
    let medication = scenario.medication("Synthetic stock tablets", supply).await;
    (medication.location_id, medication.id)
}

impl Fixture {
    async fn new() -> Self {
        let scenario = Scenario::new().await;
        let scope = scenario.scope();
        let (location_id, medication_id) = seed_medication(&scenario, Decimal::new(10, 0)).await;
        let now = chrono::Utc::now().naive_utc();
        grant::ActiveModel {
            household_id: Set(scenario.household.id),
            household_membership_id: Set(scenario.membership.id),
            person_id: Set(scenario.account.person_id),
            access_level: Set("manage".into()),
            relationship_type: Set("self".into()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&scenario.admin)
        .await
        .unwrap();
        Self {
            scenario,
            scope,
            location_id,
            medication_id,
            submission_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    async fn close(self) {
        self.scenario.close().await;
    }

    fn audits(&self) -> sea_orm::Select<version::Entity> {
        version::Entity::find()
            .filter(version::Column::HouseholdId.eq(self.household.id))
            .filter(version::Column::ItemType.eq("Medication"))
            .filter(version::Column::ItemId.eq(self.medication_id))
    }

    fn changes(&self) -> sea_orm::Select<api_change_event::Entity> {
        api_change_event::Entity::find()
            .filter(api_change_event::Column::HouseholdId.eq(self.household.id))
    }

    async fn effect(&self) -> (String, u64, u64) {
        let medication = medication::Entity::find_by_id(self.medication_id)
            .one(&self.admin)
            .await
            .unwrap()
            .unwrap();
        (
            medication.current_supply.unwrap().to_string(),
            self.audits().count(&self.admin).await.unwrap(),
            self.changes()
                .filter(api_change_event::Column::RecordType.eq("Medication"))
                .filter(api_change_event::Column::RecordId.eq(self.medication_id))
                .count(&self.admin)
                .await
                .unwrap(),
        )
    }

    async fn demote(&self) {
        membership::Entity::update_many()
            .col_expr(
                membership::Column::Role,
                sea_orm::sea_query::Expr::val("member"),
            )
            .col_expr(
                membership::Column::PermissionsVersion,
                sea_orm::sea_query::Expr::col(membership::Column::PermissionsVersion).add(1),
            )
            .filter(membership::Column::Id.eq(self.membership.id))
            .exec(&self.admin)
            .await
            .unwrap();
    }

    async fn option(&self, amount: i64, supply: i64) -> dosage::Model {
        let now = chrono::NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        dosage::ActiveModel {
            household_id: Set(self.household.id),
            medication_id: Set(self.medication_id),
            amount: Set(Decimal::new(amount, 0)),
            unit: Set("tablet".into()),
            frequency: Set("daily".into()),
            current_supply: Set(Some(Decimal::new(supply, 0))),
            default_dose_cycle: Set(0),
            default_max_daily_doses: Set(4),
            default_min_hours_between_doses: Set(Decimal::ZERO),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&self.admin)
        .await
        .unwrap()
    }
}

fn command(fixture: &Fixture, quantity: &str) -> Command {
    Command::AdjustStock(AdjustStock {
        medication_id: fixture.medication_id.to_string(),
        new_quantity: quantity.into(),
        reason: Some("cycle count".into()),
    })
}

fn previous_supply(row: &version::Model) -> String {
    let object: serde_json::Value = serde_json::from_str(row.object.as_deref().unwrap()).unwrap();
    object["current_supply"].as_str().unwrap().to_owned()
}

struct AuditFailure {
    name: String,
}

impl AuditFailure {
    async fn install(fixture: &Fixture, item_type: &str, expected_supply: Decimal) -> Self {
        let name = format!("medication_audit_{}", uuid::Uuid::new_v4().simple());
        let household_id = fixture.household.id;
        let medication_id = fixture.medication_id;
        // SeaORM Schema/Table builders cannot express PostgreSQL trigger functions or
        // non-transactional sequence probes. This isolated DDL tests rollback after
        // stock writes, using generated identifiers and typed fixture values only.
        assert!(matches!(item_type, "Medication" | "MedicationStockRemoval"));
        fixture.admin.execute_unprepared(&format!(
            "CREATE SEQUENCE {name};
             GRANT USAGE ON SEQUENCE {name} TO med_tracker_app;
             CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$
             BEGIN
               IF NEW.household_id={household_id} AND NEW.item_type='{item_type}' THEN
                 IF (SELECT current_supply FROM medications WHERE id={medication_id})={expected_supply}
                 THEN PERFORM nextval('{name}'); END IF;
                 RAISE EXCEPTION 'synthetic medication audit rejection';
               END IF;
               RETURN NEW;
             END $$;
             CREATE TRIGGER {name} BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION {name}()"
        )).await.unwrap();
        Self { name }
    }

    async fn reached_and_remove(self, fixture: &Fixture) -> bool {
        use sea_orm::sea_query::{Alias, Query};
        let query = Query::select()
            .column(Alias::new("is_called"))
            .from(Alias::new(self.name.clone()))
            .to_owned();
        let reached = fixture
            .admin
            .query_one_raw(fixture.admin.get_database_backend().build(&query))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "is_called")
            .unwrap();
        fixture
            .admin
            .execute_unprepared(&format!(
                "DROP TRIGGER {0} ON versions; DROP FUNCTION {0}(); DROP SEQUENCE {0}",
                self.name
            ))
            .await
            .unwrap();
        reached
    }
}

#[path = "care_medications/crud.rs"]
mod crud;
#[path = "care_medications/stock_check.rs"]
mod stock_check;
#[path = "care_medications/stock_removals.rs"]
mod stock_removals;

#[tokio::test]
async fn restock_validation_exposes_the_invalid_field() {
    let fixture = Fixture::new().await;
    for (quantity, date, field) in [
        ("0", "2026-10-08", "quantity"),
        ("5", "not-a-date", "restock_date"),
    ] {
        let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
            .await
            .unwrap();
        let snapshot =
            medications::read_stock_snapshot(&tenant, &fixture.medication_id.to_string())
                .await
                .unwrap();
        let error = medications::restock(
            &tenant,
            Restock {
                medication_id: fixture.medication_id.to_string(),
                quantity: quantity.into(),
                restock_date: date.into(),
                original_etag: snapshot.etag,
            },
            None,
        )
        .await
        .unwrap_err();
        let OperationError::Validation { details } = error else {
            panic!("expected validation for {field}");
        };
        assert!(details["errors"][field][0].is_string(), "{details}");
        tenant.rollback().await.unwrap();
    }
    assert_eq!(fixture.effect().await, ("10.00".into(), 0, 0));
    fixture.close().await;
}

async fn adjust(fixture: &Fixture, command: Command) -> Result<medication::Model, OperationError> {
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None).await?;
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
    let result = adjust(&fixture, command(&fixture, "12.25")).await.unwrap();
    assert_eq!(result.current_supply.unwrap().to_string(), "12.25");
    assert_eq!(fixture.effect().await, ("12.25".into(), 1, 1));
    let row = fixture.audits().one(&fixture.admin).await.unwrap().unwrap();
    assert_eq!(
        row.event,
        "adjust inventory (qty: 12.25, reason: cycle count)"
    );
    assert_eq!(previous_supply(&row), "10.00");
    assert_eq!(
        row.whodunnit.as_deref().unwrap(),
        fixture.account.id.to_string()
    );
    assert_eq!(row.request_id.as_deref().unwrap(), fixture.scope.request_id);
    let context = row.audit_context;
    assert_eq!(context["actor_account_id"], fixture.account.id);
    assert_eq!(context["actor_membership_id"], fixture.membership.id);
    assert_eq!(context["active_role"], "administrator");
    assert_eq!(context["policy_class"], "MedicationPolicy");
    assert_eq!(context["policy_query"], "adjust_inventory?");
    fixture.close().await;
}

#[tokio::test]
async fn stock_adjustment_preserves_exact_decimal_bounds_and_zero() {
    let fixture = Fixture::new().await;
    for quantity in ["0", "0.01", "99999999.99"] {
        let result = adjust(&fixture, command(&fixture, quantity)).await.unwrap();
        assert_eq!(
            result.current_supply.unwrap().normalize().to_string(),
            quantity
        );
    }
    assert_eq!(fixture.effect().await, ("99999999.99".into(), 3, 3));
    for quantity in ["-1", "1.001", "100000000", "invalid"] {
        assert!(
            matches!(
                adjust(&fixture, command(&fixture, quantity)).await,
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
    fixture.demote().await;
    assert_eq!(
        adjust(&fixture, command(&fixture, "12.25"))
            .await
            .unwrap_err(),
        OperationError::Forbidden
    );
    assert_eq!(fixture.effect().await, ("10.00".into(), 0, 0));
    fixture.close().await;
}

#[tokio::test]
async fn stock_adjustment_cross_household_id_is_hidden() {
    let fixture = Fixture::new().await;
    let other = Scenario::new().await;
    let (_, other_medication_id) = seed_medication(&other, Decimal::new(20, 0)).await;
    let Command::AdjustStock(mut input) = command(&fixture, "12.25");
    input.medication_id = other_medication_id.to_string();
    assert_eq!(
        adjust(&fixture, Command::AdjustStock(input))
            .await
            .unwrap_err(),
        OperationError::NotFound
    );
    assert_eq!(fixture.effect().await, ("10.00".into(), 0, 0));
    let row = medication::Entity::find_by_id(other_medication_id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.current_supply.unwrap().to_string(), "20.00");
    other.close().await;
    fixture.close().await;
}

#[tokio::test]
async fn stock_adjustment_rechecks_role_after_transaction_begin() {
    let fixture = Fixture::new().await;
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    fixture.demote().await;
    assert_eq!(
        medications::execute(&tenant, command(&fixture, "12.25"))
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
    let failure = AuditFailure::install(&fixture, "Medication", Decimal::new(1225, 2)).await;
    let result = adjust(&fixture, command(&fixture, "12.25")).await;
    let reached = failure.reached_and_remove(&fixture).await;
    assert_eq!(result.unwrap_err(), OperationError::Unavailable);
    assert!(
        reached,
        "The operation never reached audit after changing stock"
    );
    assert_eq!(fixture.effect().await, ("10.00".into(), 0, 0));
    fixture.close().await;
}

#[tokio::test]
async fn stock_adjustment_concurrent_counts_preserve_audit_predecessors() {
    let fixture = Fixture::new().await;
    let (first, second) = tokio::join!(
        adjust(&fixture, command(&fixture, "11")),
        adjust(&fixture, command(&fixture, "12"))
    );
    assert!(
        first.is_ok() && second.is_ok(),
        "Concurrent stock writes failed: {first:?} / {second:?}"
    );
    let effect = fixture.effect().await;
    assert!(matches!(effect.0.as_str(), "11.00" | "12.00"));
    assert_eq!((effect.1, effect.2), (2, 2));
    let rows = fixture
        .audits()
        .order_by_asc(version::Column::Id)
        .all(&fixture.admin)
        .await
        .unwrap();
    assert_eq!(previous_supply(&rows[0]), "10.00");
    assert_eq!(
        previous_supply(&rows[1]),
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
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None).await?;
    let result = medications::execute_with_options(
        &tenant,
        command(fixture, quantity),
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
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    let snapshot = medications::read_stock_snapshot(&tenant, &fixture.medication_id.to_string())
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
    let option = fixture.option(2, 6).await;
    assert!(matches!(
        guarded_adjust(&fixture, "12", &etag).await,
        Err(OperationError::Conflict { .. })
    ));
    assert_eq!(fixture.effect().await, ("10.00".into(), 0, 0));
    adjust(&fixture, command(&fixture, "12")).await.unwrap();
    let option = dosage::Entity::find_by_id(option.id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(option.current_supply.unwrap().to_string(), "6.00");
    assert_eq!(fixture.effect().await, ("12.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn stock_adjustment_portable_id_and_trusted_provenance_preserve_audit() {
    let fixture = Fixture::new().await;
    let tenant = access::begin(&fixture.runtime, &fixture.scope, None)
        .await
        .unwrap();
    let snapshot = medications::read_stock_snapshot(&tenant, &fixture.medication_id.to_string())
        .await
        .unwrap();
    let provenance = med_tracker::models::identity::credentials::CredentialProvenance {
        method: med_tracker::models::identity::credentials::CredentialMethod::OauthGrant,
        reference: "synthetic-validated-grant".into(),
    };
    let Command::AdjustStock(mut input) = command(&fixture, "12");
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
    let row = fixture.audits().one(&fixture.admin).await.unwrap().unwrap();
    let audit = row.audit_context;
    assert_eq!(audit["actor_account_id"], fixture.account.id);
    assert_eq!(audit["actor_person_id"], fixture.account.person_id);
    assert_eq!(audit["actor_membership_id"], fixture.membership.id);
    assert_eq!(audit["household_id"], fixture.household.id);
    assert_eq!(audit["request_id"], fixture.scope.request_id);
    assert_eq!(audit["active_role"], "administrator");
    assert_eq!(audit["policy_class"], "MedicationPolicy");
    assert_eq!(audit["policy_query"], "adjust_inventory?");
    assert_eq!(audit["authentication_method"], "oauth");
    assert_eq!(
        audit["session_reference"],
        "oauth_grant:synthetic-validated-grant"
    );
    assert_eq!(fixture.effect().await, ("12.00".into(), 1, 1));
    fixture.close().await;
}
