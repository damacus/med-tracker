#[path = "support/request_trace_capture.rs"]
mod capture;
#[path = "support/clinical_fixture.rs"]
mod clinical_fixture;
#[path = "support/scenario.rs"]
pub mod scenario;
use med_tracker::models::{
    _entities::{
        api_change_events, dosages, household_memberships, medication_takes, medications,
        person_access_grants, person_medications as person_medication, schedules, versions,
    },
    access::{self, HouseholdScope},
    care::doses::{self, Command, Outcome, TakeInput as Take},
    errors::OperationError,
};
use scenario::Scenario;
use sea_orm::prelude::Decimal;
use sea_orm::sea_query::ExprTrait;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter,
};

struct Fixture {
    scenario: Scenario,
    medication: medications::Model,
    assignment: person_medication::Model,
    client_uuid: uuid::Uuid,
}

impl std::ops::Deref for Fixture {
    type Target = Scenario;
    fn deref(&self) -> &Self::Target {
        &self.scenario
    }
}

fn now() -> chrono::NaiveDateTime {
    chrono::Utc::now().naive_utc()
}
fn date(value: &str) -> chrono::NaiveDate {
    value.parse().unwrap()
}

impl Fixture {
    async fn new() -> Self {
        let scenario = Scenario::new().await;
        let medication = medication(&scenario, 10).await;
        let assignment = assignment(&scenario, medication.id, 0).await;
        Self {
            scenario,
            medication,
            assignment,
            client_uuid: uuid::Uuid::new_v4(),
        }
    }

    fn scope(&self) -> HouseholdScope {
        self.scenario.scope()
    }
    async fn close(self) {
        self.scenario.close().await;
    }

    async fn effect(&self) -> (u64, String, u64, u64) {
        let takes = medication_takes::Entity::find()
            .filter(medication_takes::Column::HouseholdId.eq(self.household.id))
            .count(&self.admin)
            .await
            .unwrap();
        let medication = medications::Entity::find_by_id(self.medication.id)
            .one(&self.admin)
            .await
            .unwrap()
            .unwrap();
        let audits = versions::Entity::find()
            .filter(versions::Column::HouseholdId.eq(self.household.id))
            .filter(versions::Column::ItemType.eq("MedicationTake"))
            .filter(versions::Column::Event.eq("create"))
            .count(&self.admin)
            .await
            .unwrap();
        let changes = api_change_events::Entity::find()
            .filter(api_change_events::Column::HouseholdId.eq(self.household.id))
            .filter(api_change_events::Column::RecordType.eq("MedicationTake"))
            .filter(api_change_events::Column::Action.eq("create"))
            .count(&self.admin)
            .await
            .unwrap();
        (
            takes,
            format!("{:.2}", medication.current_supply.unwrap()),
            audits,
            changes,
        )
    }

    async fn update_assignment(&self, edit: impl FnOnce(&mut person_medication::ActiveModel)) {
        let mut row = person_medication::Entity::find_by_id(self.assignment.id)
            .one(&self.admin)
            .await
            .unwrap()
            .unwrap()
            .into_active_model();
        edit(&mut row);
        row.update(&self.admin).await.unwrap();
    }

    async fn member(&self) {
        let mut row = self.membership.clone().into_active_model();
        row.role = Set("member".into());
        row.update(&self.admin).await.unwrap();
    }

    async fn grant(&self) -> person_access_grants::Model {
        person_access_grants::ActiveModel {
            household_id: Set(self.household.id),
            household_membership_id: Set(self.membership.id),
            person_id: Set(self.account.person_id),
            access_level: Set("manage".into()),
            relationship_type: Set("self".into()),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        }
        .insert(&self.admin)
        .await
        .unwrap()
    }

    async fn version_count(&self) -> u64 {
        versions::Entity::find()
            .filter(
                sea_orm::Condition::any()
                    .add(versions::Column::HouseholdId.eq(self.household.id))
                    .add(
                        sea_orm::Condition::all()
                            .add(versions::Column::ItemType.eq("NativeAuthentication"))
                            .add(versions::Column::ItemId.eq(self.account.id)),
                    ),
            )
            .count(&self.admin)
            .await
            .unwrap()
    }

    async fn schedule(
        &self,
        kind: i32,
        config: serde_json::Value,
        start: &str,
        end: &str,
        amount: i64,
    ) -> schedules::Model {
        schedules::ActiveModel {
            household_id: Set(self.household.id),
            person_id: Set(self.account.person_id),
            medication_id: Set(self.medication.id),
            active: Set(true),
            start_date: Set(Some(date(start))),
            end_date: Set(Some(date(end))),
            dose_amount: Set(Some(Decimal::new(amount, 0))),
            dose_unit: Set(Some("tablet".into())),
            schedule_type: Set(kind),
            schedule_config: Set(config),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        }
        .insert(&self.admin)
        .await
        .unwrap()
    }
}

async fn medication(scenario: &Scenario, supply: i64) -> medications::Model {
    let mut medication = scenario
        .medication("Synthetic tablets", Decimal::new(supply, 0))
        .await
        .into_active_model();
    medication.dmd_system = Set(Some("https://dmd.nhs.uk".into()));
    medication.dmd_concept_class = Set(Some("VMP".into()));
    medication.dmd_code = Set(Some("123456".into()));
    medication.update(&scenario.admin).await.unwrap()
}

async fn assignment(
    scenario: &Scenario,
    medication_id: i64,
    position: i32,
) -> person_medication::Model {
    scenario.personal_assignment(medication_id, position).await
}

fn take_input(fixture: &Fixture) -> Take {
    Take {
        client_uuid: Some(fixture.client_uuid.to_string()),
        source_type: "person_medication".into(),
        source_id: fixture.assignment.id.to_string(),
        taken_at: "2026-10-05T10:00:00Z".into(),
        dose_amount: Some("2".into()),
        dose_unit: Some("tablet".into()),
        taken_from_medication_id: Some(fixture.medication.id),
        expected_effective_amount: None,
        expected_effective_unit: None,
    }
}

fn command(fixture: &Fixture) -> Command {
    Command::Take(take_input(fixture).validate().unwrap())
}

async fn take(fixture: &Fixture, command: Command) -> Result<Outcome, OperationError> {
    let tenant = access::begin(&fixture.runtime, &fixture.scope(), None).await?;
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
        take(&fixture, command(&fixture)).await,
        Ok(Outcome::Created(_))
    ));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_portable_source_and_microsecond_decimal_retry_has_one_effect() {
    let fixture = Fixture::new().await;
    let assignment = person_medication::Entity::find_by_id(fixture.assignment.id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut first = take_input(&fixture);
    first.source_id = assignment.portable_id;
    first.taken_at = "2026-10-05T11:00:00.123456+01:00".into();
    first.dose_amount = Some("2.00".into());
    let Outcome::Created(record) = take(&fixture, Command::Take(first.clone().validate().unwrap()))
        .await
        .unwrap()
    else {
        panic!("portable source dose was not created")
    };
    assert_eq!(
        record.taken_at.unwrap().to_string(),
        "2026-10-05 10:00:00.123456"
    );
    first.taken_at = "2026-10-05T10:00:00.123456Z".into();
    first.dose_amount = Some("2.0".into());
    assert!(
        matches!(take(&fixture, Command::Take(first.validate().unwrap())).await,
        Ok(Outcome::Replayed(replay)) if replay.id == record.id)
    );
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_duplicate_replay_does_not_repeat_clinical_effects() {
    let fixture = Fixture::new().await;
    assert!(matches!(
        take(&fixture, command(&fixture)).await,
        Ok(Outcome::Created(_))
    ));
    assert!(matches!(
        take(&fixture, command(&fixture)).await,
        Ok(Outcome::Replayed(_))
    ));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_rejects_payload_mismatch_without_writes() {
    let fixture = Fixture::new().await;
    assert!(take(&fixture, command(&fixture)).await.is_ok());
    let mut changed = take_input(&fixture);
    changed.dose_amount = Some("3".into());
    assert!(matches!(
        take(&fixture, Command::Take(changed.validate().unwrap())).await,
        Err(OperationError::Conflict { .. })
    ));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_insufficient_stock_has_no_partial_writes() {
    let fixture = Fixture::new().await;
    let mut stock = fixture.medication.clone().into_active_model();
    stock.current_supply = Set(Some(Decimal::ONE));
    stock.update(&fixture.admin).await.unwrap();
    assert!(matches!(
        take(&fixture, command(&fixture)).await,
        Err(OperationError::Validation { .. })
    ));
    assert_eq!(fixture.effect().await, (0, "1.00".into(), 0, 0));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_participant_records_after_legacy_self_grant_revocation()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new().await;
    fixture.member().await;
    let grant = fixture.grant().await;
    let tenant = access::begin(&fixture.runtime, &fixture.scope(), None).await?;
    let mut grant = grant.into_active_model();
    grant.revoked_at = Set(Some(now()));
    let changed = grant.update(&fixture.admin).await?;
    let result = doses::execute(&tenant, command(&fixture)).await;
    assert!(matches!(result, Ok(Outcome::Created(_))));
    tenant.commit().await?;
    assert_eq!(changed.household_id, fixture.household.id);
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
    Ok(())
}

#[tokio::test]
async fn dose_take_rechecks_revoked_membership_in_open_transaction()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new().await;
    let tenant = access::begin(&fixture.runtime, &fixture.scope(), None).await?;
    let changed = household_memberships::Entity::update_many()
        .col_expr(
            household_memberships::Column::Status,
            sea_orm::sea_query::Expr::val("revoked"),
        )
        .col_expr(
            household_memberships::Column::RevokedAt,
            sea_orm::sea_query::Expr::val(now()),
        )
        .col_expr(
            household_memberships::Column::UpdatedAt,
            sea_orm::sea_query::Expr::val(now()),
        )
        .col_expr(
            household_memberships::Column::PermissionsVersion,
            sea_orm::sea_query::Expr::col(household_memberships::Column::PermissionsVersion).add(1),
        )
        .filter(household_memberships::Column::Id.eq(fixture.membership.id))
        .filter(household_memberships::Column::HouseholdId.eq(fixture.household.id))
        .filter(household_memberships::Column::PersonId.eq(fixture.account.person_id))
        .exec(&fixture.admin)
        .await?;
    let result = doses::execute(&tenant, command(&fixture)).await;
    assert!(matches!(result, Err(OperationError::Forbidden)));
    tenant.rollback().await?;
    assert_eq!(changed.rows_affected, 1);
    assert_eq!(fixture.effect().await, (0, "10.00".into(), 0, 0));
    fixture.close().await;
    Ok(())
}

#[tokio::test]
async fn dose_take_audit_failure_rolls_back_stock_and_take() {
    let fixture = Fixture::new().await;
    let suffix = fixture.client_uuid.simple();
    let sequence = format!("dose_audit_{suffix}");
    let function = format!("reject_dose_{suffix}");
    // SeaORM Schema/Query builders cannot define PostgreSQL trigger functions.
    // The sequence retains evidence across rollback. Keep this fault
    // injection exclusive; generated identifiers and numeric IDs enter DDL only.
    fixture.admin.execute_unprepared(&format!(
        "CREATE SEQUENCE public.{sequence}; GRANT USAGE ON SEQUENCE public.{sequence} TO med_tracker_app;
         CREATE FUNCTION public.{function}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
         IF NEW.item_type = 'MedicationTake' AND NEW.household_id = {} THEN
           IF EXISTS (SELECT 1 FROM medication_takes WHERE id = NEW.item_id AND household_id = NEW.household_id)
             AND (SELECT current_supply FROM medications WHERE id = {}) = 8 THEN
             PERFORM nextval('public.{sequence}'); END IF;
           RAISE EXCEPTION 'synthetic audit failure'; END IF; RETURN NEW; END $$;
         CREATE TRIGGER {function} BEFORE INSERT ON public.versions FOR EACH ROW EXECUTE FUNCTION public.{function}()",
        fixture.household.id, fixture.medication.id
    )).await.unwrap();
    let result = take(&fixture, command(&fixture)).await;
    let sequence_state = sea_orm::sea_query::Query::select()
        .column(sea_orm::sea_query::Alias::new("is_called"))
        .from((
            sea_orm::sea_query::Alias::new("public"),
            sea_orm::sea_query::Alias::new(&sequence),
        ))
        .to_owned();
    let attempt = fixture
        .admin
        .query_one_raw(fixture.admin.get_database_backend().build(&sequence_state))
        .await
        .unwrap()
        .unwrap();
    let reached_audit = attempt.try_get::<bool>("", "is_called").unwrap();
    let effect = fixture.effect().await;
    fixture.admin.execute_unprepared(&format!("DROP TRIGGER {function} ON public.versions; DROP FUNCTION public.{function}(); DROP SEQUENCE public.{sequence}")).await.unwrap();
    assert!(matches!(result, Err(OperationError::Unavailable)));
    assert!(
        reached_audit,
        "The failure never reached audit after inserting the take and decrementing stock"
    );
    assert_eq!(effect, (0, "10.00".into(), 0, 0));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_database_failure_emits_the_safe_diagnostic() {
    use tracing::instrument::WithSubscriber;

    let fixture = Fixture::new().await;
    let function = format!("reject_dose_{}", fixture.client_uuid.simple());
    // SeaORM Schema/Query builders cannot define PostgreSQL trigger functions.
    // The exclusive fault injection targets only the generated household.
    fixture.admin.execute_unprepared(&format!(
        "CREATE FUNCTION public.{function}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
         IF NEW.household_id = {} THEN RAISE EXCEPTION 'synthetic take failure'; END IF; RETURN NEW; END $$;
         CREATE TRIGGER {function} BEFORE INSERT ON medication_takes FOR EACH ROW EXECUTE FUNCTION public.{function}()",
        fixture.household.id
    )).await.unwrap();
    let captured = capture::Captured::default();
    let result = take(&fixture, command(&fixture))
        .with_subscriber(captured.clone())
        .await;
    fixture
        .admin
        .execute_unprepared(&format!(
            "DROP TRIGGER {function} ON medication_takes; DROP FUNCTION public.{function}()"
        ))
        .await
        .unwrap();
    assert!(matches!(result, Err(OperationError::Unavailable)));
    let events = captured.events();
    let failures = events
        .iter()
        .filter(|event| event.contains("Database operation failed"))
        .collect::<Vec<_>>();
    assert_eq!(failures.len(), 1, "{events:?}");
    let event = failures[0];
    assert!(event.contains("category"), "{event}");
    assert!(event.contains("error_trace_id"), "{event}");
    assert!(!event.contains("synthetic take failure"), "{event}");
    assert_eq!(fixture.effect().await, (0, "10.00".into(), 0, 0));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_concurrent_uuid_has_one_clinical_effect() {
    let fixture = Fixture::new().await;
    let (first, second) = tokio::join!(
        take(&fixture, command(&fixture)),
        take(&fixture, command(&fixture))
    );
    assert!(first.is_ok() && second.is_ok());
    assert!(matches!(
        (&first, &second),
        (Ok(Outcome::Created(_)), Ok(Outcome::Replayed(_)))
            | (Ok(Outcome::Replayed(_)), Ok(Outcome::Created(_)))
    ));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

async fn tracked_fixture(fixture: &Fixture) -> [dosages::Model; 2] {
    let mut rows = Vec::new();
    for (amount, supply) in [(2, 6), (1, 4)] {
        rows.push(
            dosages::ActiveModel {
                household_id: Set(fixture.household.id),
                medication_id: Set(fixture.medication.id),
                amount: Set(Decimal::new(amount, 0)),
                unit: Set("tablet".into()),
                frequency: Set("daily".into()),
                current_supply: Set(Some(Decimal::new(supply, 0))),
                default_dose_cycle: Set(0),
                default_max_daily_doses: Set(4),
                default_min_hours_between_doses: Set(Decimal::ZERO),
                created_at: Set(date("2020-01-01").and_hms_opt(0, 0, 0).unwrap()),
                updated_at: Set(date("2020-01-01").and_hms_opt(0, 0, 0).unwrap()),
                ..Default::default()
            }
            .insert(&fixture.admin)
            .await
            .unwrap(),
        );
    }
    fixture
        .update_assignment(|row| row.source_dosage_option_id = Set(Some(rows[0].id)))
        .await;
    rows.try_into().unwrap()
}

#[tokio::test]
async fn dose_take_tracked_option_updates_stock_aggregate_timestamp_and_audits() {
    let fixture = Fixture::new().await;
    let options = tracked_fixture(&fixture).await;
    assert!(matches!(
        take(&fixture, command(&fixture)).await,
        Ok(Outcome::Created(_))
    ));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    let tracked = dosages::Entity::find_by_id(options[0].id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let untouched = dosages::Entity::find_by_id(options[1].id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(tracked.current_supply.unwrap(), Decimal::new(4, 0));
    assert_eq!(untouched.current_supply.unwrap(), Decimal::new(4, 0));
    let audits = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(fixture.household.id))
        .filter(versions::Column::ItemType.eq("MedicationDosageOption"))
        .count(&fixture.admin)
        .await
        .unwrap();
    assert_eq!(audits, 1);
    assert!(
        tracked.updated_at > options[0].updated_at,
        "The tracked dosage timestamp did not change with its stock"
    );
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_unrelated_unique_collision_is_not_a_replay_conflict() {
    let fixture = Fixture::new().await;
    assert!(matches!(
        take(&fixture, command(&fixture)).await,
        Ok(Outcome::Created(_))
    ));
    let function = format!("collide_dose_{}", fixture.client_uuid.simple());
    // SeaORM Schema/Query builders do not define PostgreSQL trigger functions.
    // This exclusive fault injection targets only the generated household.
    fixture.admin.execute_unprepared(&format!("CREATE FUNCTION public.{function}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
        IF NEW.household_id = {} THEN NEW.portable_id := (SELECT portable_id FROM medication_takes WHERE household_id = {} ORDER BY id LIMIT 1); END IF;
        RETURN NEW; END $$; CREATE TRIGGER {function} BEFORE INSERT ON medication_takes FOR EACH ROW EXECUTE FUNCTION public.{function}()",
        fixture.household.id, fixture.household.id)).await.unwrap();
    let mut input = take_input(&fixture);
    input.client_uuid = Some(uuid::Uuid::new_v4().to_string());
    let result = take(&fixture, Command::Take(input.validate().unwrap())).await;
    let effect = fixture.effect().await;
    fixture
        .admin
        .execute_unprepared(&format!(
            "DROP TRIGGER {function} ON medication_takes; DROP FUNCTION public.{function}()"
        ))
        .await
        .unwrap();
    assert_eq!(
        result.unwrap_err(),
        OperationError::Unavailable,
        "An unrelated portable identity collision was swallowed as a client replay conflict"
    );
    assert_eq!(effect, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_omitted_replay_options_preserve_original_after_default_change() {
    let fixture = Fixture::new().await;
    assert!(matches!(
        take(&fixture, command(&fixture)).await,
        Ok(Outcome::Created(_))
    ));
    fixture
        .update_assignment(|row| {
            row.dose_amount = Set(Some(Decimal::new(3, 0)));
            row.dose_unit = Set(Some("capsule".into()));
        })
        .await;
    let mut input = take_input(&fixture);
    input.dose_amount = None;
    input.dose_unit = None;
    input.taken_from_medication_id = None;
    assert!(matches!(
        take(&fixture, Command::Take(input.validate().unwrap())).await,
        Ok(Outcome::Replayed(_))
    ));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_explicit_replay_options_must_match_stored_effect() {
    let fixture = Fixture::new().await;
    assert!(matches!(
        take(&fixture, command(&fixture)).await,
        Ok(Outcome::Created(_))
    ));
    let other = medication(&fixture, 10).await;
    for field in ["amount", "unit", "location"] {
        let mut input = take_input(&fixture);
        match field {
            "amount" => input.dose_amount = Some("3".into()),
            "unit" => input.dose_unit = Some("capsule".into()),
            _ => input.taken_from_medication_id = Some(other.id),
        }
        assert!(
            matches!(
                take(&fixture, Command::Take(input.validate().unwrap())).await,
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
    for retired in [false, true] {
        let fixture = Fixture::new().await;
        fixture
            .update_assignment(|row| {
                if retired {
                    row.retired_at = Set(Some(now()));
                } else {
                    row.active = Set(false);
                }
            })
            .await;
        let error = take(&fixture, command(&fixture)).await.unwrap_err();
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
    let other = medication(&fixture, 10).await;
    assignment(&fixture, other.id, 1).await;
    let mut input = take_input(&fixture);
    input.taken_from_medication_id = None;
    assert!(matches!(
        take(&fixture, Command::Take(input.validate().unwrap())).await,
        Err(OperationError::Validation { .. })
    ));
    assert_eq!(fixture.effect().await, (0, "10.00".into(), 0, 0));
    let mut input = take_input(&fixture);
    input.taken_from_medication_id = Some(other.id);
    assert!(matches!(
        take(&fixture, Command::Take(input.validate().unwrap())).await,
        Ok(Outcome::Created(_))
    ));
    assert_eq!(fixture.effect().await, (1, "10.00".into(), 1, 1));
    let row = medications::Entity::find_by_id(other.id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.current_supply.unwrap(), Decimal::new(8, 0));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_ambiguous_and_insufficient_tracked_options_roll_back() {
    for ambiguous in [false, true] {
        let fixture = Fixture::new().await;
        let options = tracked_fixture(&fixture).await;
        if ambiguous {
            fixture
                .update_assignment(|row| row.source_dosage_option_id = Set(None))
                .await;
            let mut option = options[1].clone().into_active_model();
            option.amount = Set(Decimal::new(2, 0));
            option.update(&fixture.admin).await.unwrap();
        } else {
            let mut option = options[0].clone().into_active_model();
            option.current_supply = Set(Some(Decimal::ONE));
            option.update(&fixture.admin).await.unwrap();
        }
        let baseline_versions = fixture.version_count().await;
        assert!(baseline_versions > 0);
        let non_authentication = versions::Entity::find()
            .filter(versions::Column::HouseholdId.eq(fixture.household.id))
            .filter(versions::Column::ItemType.ne("NativeAuthentication"))
            .count(&fixture.admin)
            .await
            .unwrap();
        assert_eq!(non_authentication, 0);
        assert!(matches!(
            take(&fixture, command(&fixture)).await,
            Err(OperationError::Validation { .. })
        ));
        assert_eq!(fixture.effect().await, (0, "10.00".into(), 0, 0));
        assert_eq!(fixture.version_count().await, baseline_versions);
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
        let schedule = fixture
            .schedule(
                kind,
                serde_json::from_str(config).unwrap(),
                "2026-10-05",
                "2026-10-10",
                2,
            )
            .await;
        let mut input = take_input(&fixture);
        input.source_type = "schedule".into();
        input.source_id = schedule.id.to_string();
        input.dose_amount = None;
        let result = take(&fixture, Command::Take(input.validate().unwrap())).await;
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
    let mut input = take_input(&fixture);
    input.dose_amount = Some("3".into());
    let outcome = take(&fixture, Command::Take(input.validate().unwrap())).await;
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
            .update_assignment(|row| {
                row.max_daily_doses = Set(Some(1));
                row.dose_cycle = Set(Some(cycle));
            })
            .await;
        assert!(matches!(
            take(&fixture, command(&fixture)).await,
            Ok(Outcome::Created(_))
        ));
        let mut input = take_input(&fixture);
        input.client_uuid = None;
        input.taken_at = "2026-10-05T11:00:00Z".into();
        assert!(matches!(
            take(&fixture, Command::Take(input.validate().unwrap())).await,
            Err(OperationError::Validation { .. })
        ));
        assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
        fixture.close().await;
    }
    let fixture = Fixture::new().await;
    fixture
        .update_assignment(|row| row.min_hours_between_doses = Set(Some(4)))
        .await;
    assert!(matches!(
        take(&fixture, command(&fixture)).await,
        Ok(Outcome::Created(_))
    ));
    let mut input = take_input(&fixture);
    input.client_uuid = None;
    input.taken_at = "2026-10-05T11:00:00Z".into();
    assert!(matches!(
        take(&fixture, Command::Take(input.validate().unwrap())).await,
        Err(OperationError::Validation { .. })
    ));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_request_timezone_selects_near_midnight_schedule_date() {
    let fixture = Fixture::new().await;
    let schedule = fixture
        .schedule(
            2,
            serde_json::json!({"weekdays":["tuesday"]}),
            "2026-09-28",
            "2026-09-30",
            2,
        )
        .await;
    let mut input = take_input(&fixture);
    input.source_type = "schedule".into();
    input.source_id = schedule.id.to_string();
    input.taken_at = "2026-09-28T23:30:00Z".into();
    let tenant = access::begin(&fixture.runtime, &fixture.scope(), None)
        .await
        .unwrap();
    let utc = doses::execute_in_timezone(
        &tenant,
        Command::Take(input.clone().validate().unwrap()),
        chrono_tz::UTC,
        None,
    )
    .await;
    assert!(matches!(utc, Err(OperationError::Validation { .. })));
    tenant.rollback().await.unwrap();
    let tenant = access::begin(&fixture.runtime, &fixture.scope(), None)
        .await
        .unwrap();
    let london = doses::execute_in_timezone(
        &tenant,
        Command::Take(input.validate().unwrap()),
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
async fn dose_take_participant_records_after_legacy_self_grant_expiry()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new().await;
    fixture.member().await;
    let grant = fixture.grant().await;
    let mut grant = grant.into_active_model();
    grant.expires_at = Set(Some(now() + chrono::Duration::hours(1)));
    let grant = grant.update(&fixture.admin).await?;
    let tenant = access::begin(&fixture.runtime, &fixture.scope(), None).await?;
    let mut grant = grant.into_active_model();
    grant.expires_at = Set(Some(now() - chrono::Duration::seconds(1)));
    let grant = grant.update(&fixture.admin).await?;
    assert!(grant.expires_at.unwrap() < now());
    let result = doses::execute(&tenant, command(&fixture)).await;
    assert!(matches!(result, Ok(Outcome::Created(_))));
    tenant.commit().await?;
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
    Ok(())
}

async fn take_in_timezone(
    fixture: &Fixture,
    command: Command,
    zone: chrono_tz::Tz,
) -> Result<Outcome, OperationError> {
    let tenant = access::begin(&fixture.runtime, &fixture.scope(), None).await?;
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
            .update_assignment(|row| {
                row.max_daily_doses = Set(Some(1));
                row.dose_cycle = Set(Some(cycle));
            })
            .await;
        let mut input = take_input(&fixture);
        input.taken_at = first.into();
        assert!(matches!(
            take_in_timezone(
                &fixture,
                Command::Take(input.clone().validate().unwrap()),
                chrono_tz::Europe::London
            )
            .await,
            Ok(Outcome::Created(_))
        ));
        input.client_uuid = None;
        input.taken_at = second.into();
        let result = take_in_timezone(
            &fixture,
            Command::Take(input.validate().unwrap()),
            chrono_tz::Europe::London,
        )
        .await;
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
    fixture.member().await;
    let other = medication(&fixture, 20).await;
    let mut input = take_input(&fixture);
    input.taken_from_medication_id = None;
    let created = take(&fixture, Command::Take(input.validate().unwrap())).await;
    let Ok(Outcome::Created(record)) = created else {
        panic!("The canonical member could not take linked stock: {created:?}");
    };
    assert_eq!(record.taken_from_medication_id, Some(fixture.medication.id));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    let mut input = take_input(&fixture);
    input.client_uuid = None;
    input.taken_from_medication_id = Some(other.id);
    assert!(matches!(
        take(&fixture, Command::Take(input.validate().unwrap())).await,
        Err(OperationError::Validation { .. })
    ));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    let unlinked = medications::Entity::find_by_id(other.id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(unlinked.current_supply.unwrap(), Decimal::new(20, 0));
    assert_eq!(
        person_medication::Entity::find()
            .filter(person_medication::Column::MedicationId.eq(other.id))
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_rejects_unrepresentable_tracked_aggregate_without_effects() {
    let fixture = Fixture::new().await;
    let options = tracked_fixture(&fixture).await;
    for (option, supply) in options.iter().zip([60_000_000, 50_000_000]) {
        let mut option = option.clone().into_active_model();
        option.current_supply = Set(Some(Decimal::new(supply, 0)));
        option.update(&fixture.admin).await.unwrap();
    }
    assert!(matches!(
        take(&fixture, command(&fixture)).await,
        Err(OperationError::Validation { .. })
    ));
    assert_eq!(fixture.effect().await, (0, "10.00".into(), 0, 0));
    for (option, supply) in options.iter().zip([60_000_000, 50_000_000]) {
        let row = dosages::Entity::find_by_id(option.id)
            .one(&fixture.admin)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.current_supply.unwrap(), Decimal::new(supply, 0));
    }
    fixture.close().await;
}

fn uuid_command(fixture: &Fixture, client_uuid: String) -> Command {
    let mut input = take_input(fixture);
    input.client_uuid = Some(client_uuid);
    Command::Take(input.validate().unwrap())
}

#[tokio::test]
async fn dose_take_uuid_spellings_replay_one_effect() {
    let fixture = Fixture::new().await;
    let id = fixture.client_uuid;
    let first = take(
        &fixture,
        uuid_command(&fixture, id.hyphenated().to_string().to_uppercase()),
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
        let result = take(&fixture, uuid_command(&fixture, spelling))
            .await
            .unwrap();
        assert!(matches!(result, Outcome::Replayed(record) if record.id == first.id));
    }
    assert_eq!(first.client_uuid.as_deref(), Some(id.to_string().as_str()));
    assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_uuid_canonical_retry_preserves_historical_spelling() {
    for spelling in 0..5 {
        let fixture = Fixture::new().await;
        let id = fixture.client_uuid;
        let historical = match spelling {
            0 => id.to_string().to_uppercase(),
            1 => id.simple().to_string(),
            2 => id.braced().to_string(),
            3 => id.urn().to_string(),
            _ => id.urn().to_string().to_uppercase(),
        };
        let Outcome::Created(first) = take(&fixture, command(&fixture)).await.unwrap() else {
            panic!("First dose was not created")
        };
        let mut stored = first.clone().into_active_model();
        stored.client_uuid = Set(Some(historical.clone()));
        stored.update(&fixture.admin).await.unwrap();
        let replay = take(&fixture, command(&fixture)).await.unwrap();
        assert!(
            matches!(replay, Outcome::Replayed(record) if record.id == first.id && record.client_uuid.as_deref() == Some(historical.as_str()))
        );
        assert_eq!(fixture.effect().await, (1, "8.00".into(), 1, 1));
        fixture.close().await;
    }
}

#[tokio::test]
async fn dose_take_uuid_concurrent_variants_share_one_effect() {
    let fixture = Fixture::new().await;
    let id = fixture.client_uuid;
    let (first, second) = tokio::join!(
        take(&fixture, uuid_command(&fixture, id.urn().to_string())),
        take(
            &fixture,
            uuid_command(&fixture, id.hyphenated().to_string().to_uppercase())
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
    let Outcome::Created(first) = take(&fixture, command(&fixture)).await.unwrap() else {
        panic!("First dose was not created")
    };
    let index = sea_orm::sea_query::Index::drop()
        .name("index_medication_takes_on_client_uuid_canonical")
        .to_owned();
    fixture
        .admin
        .execute_raw(fixture.admin.get_database_backend().build(&index))
        .await
        .unwrap();
    let duplicate = medication_takes::ActiveModel {
        household_id: Set(first.household_id),
        client_uuid: Set(first.client_uuid.as_ref().map(|value| value.to_uppercase())),
        person_medication_id: Set(first.person_medication_id),
        taken_from_medication_id: Set(first.taken_from_medication_id),
        taken_from_location_id: Set(first.taken_from_location_id),
        dose_amount: Set(first.dose_amount),
        dose_unit: Set(first.dose_unit),
        taken_at: Set(first.taken_at),
        created_at: Set(first.created_at),
        updated_at: Set(first.updated_at),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    let before = fixture.effect().await;
    let result = take(&fixture, command(&fixture)).await;
    let after = fixture.effect().await;
    medication_takes::Entity::delete_by_id(duplicate.id)
        .exec(&fixture.admin)
        .await
        .unwrap();
    // SeaORM IndexCreateStatement cannot express the migration's regex/CASE
    // expression without custom SQL. Restore its exact accepted definition;
    // this historical-schema case must run exclusively.
    fixture.admin.execute_unprepared(&format!("CREATE UNIQUE INDEX index_medication_takes_on_client_uuid_canonical ON public.medication_takes (({}))", migration::CANONICAL_CLIENT_UUID_SQL)).await.unwrap();
    assert!(matches!(result, Err(OperationError::Conflict { .. })));
    assert_eq!(after, before);
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_uuid_foreign_historical_collision_is_hidden_without_new_effect() {
    let fixture = Fixture::new().await;
    let foreign = Fixture::new().await;
    let historical = fixture.client_uuid.to_string().to_uppercase();
    medication_takes::ActiveModel {
        household_id: Set(foreign.household.id),
        client_uuid: Set(Some(historical.clone())),
        person_medication_id: Set(Some(foreign.assignment.id)),
        dose_amount: Set(Some(Decimal::new(2, 0))),
        dose_unit: Set(Some("tablet".into())),
        taken_at: Set(Some(date("2026-10-05").and_hms_opt(10, 0, 0).unwrap())),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    let foreign_before = foreign.effect().await;
    let before = fixture.effect().await;
    let error = take(&fixture, command(&fixture)).await.unwrap_err();
    assert!(
        matches!(&error, OperationError::Conflict { code, .. } if code == "idempotency_key_unavailable")
    );
    assert!(!format!("{error:?}").contains(&historical));
    assert_eq!(foreign.effect().await, foreign_before);
    foreign.close().await;
    assert_eq!(fixture.effect().await, before);
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_conformance_backdated_interval_preserves_preceding_neighbour_policy() {
    let fixture = Fixture::new().await;
    fixture
        .update_assignment(|row| {
            row.max_daily_doses = Set(Some(4));
            row.min_hours_between_doses = Set(Some(4));
        })
        .await;
    for time in ["2026-10-05T10:00:00Z", "2026-10-05T18:00:00Z"] {
        let mut input = take_input(&fixture);
        input.client_uuid = None;
        input.taken_at = time.into();
        assert!(matches!(
            take(&fixture, Command::Take(input.validate().unwrap())).await,
            Ok(Outcome::Created(_))
        ));
    }
    let mut input = take_input(&fixture);
    input.client_uuid = None;
    input.taken_at = "2026-10-05T16:00:00Z".into();
    assert!(matches!(
        take(&fixture, Command::Take(input.validate().unwrap())).await,
        Ok(Outcome::Created(_))
    ));
    assert_eq!(fixture.effect().await, (3, "4.00".into(), 3, 3));
    fixture.close().await;
}

#[tokio::test]
async fn dose_take_review_skipped_midnight_keeps_local_daily_cycles_separate() {
    let fixture = Fixture::new().await;
    fixture
        .update_assignment(|row| {
            row.max_daily_doses = Set(Some(1));
            row.min_hours_between_doses = Set(Some(0));
            row.dose_cycle = Set(Some(0));
        })
        .await;
    for time in ["2025-09-07T02:30:00Z", "2025-09-07T04:30:00Z"] {
        let mut input = take_input(&fixture);
        input.client_uuid = None;
        input.taken_at = time.into();
        let tenant = access::begin(&fixture.runtime, &fixture.scope(), None)
            .await
            .unwrap();
        let result = doses::execute_in_timezone(
            &tenant,
            Command::Take(input.validate().unwrap()),
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

async fn take_in_zone(
    fixture: &Fixture,
    command: Command,
    zone: chrono_tz::Tz,
) -> Result<Outcome, OperationError> {
    let tenant = access::begin(&fixture.runtime, &fixture.scope(), None).await?;
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
async fn dose_take_keeps_elapsed_cooldown_distinct_from_local_calendar_limits_and_taper_steps() {
    let fixture = Fixture::new().await;
    fixture
        .update_assignment(|row| {
            row.max_daily_doses = Set(Some(2));
            row.min_hours_between_doses = Set(Some(6));
            row.dose_cycle = Set(Some(0));
        })
        .await;

    let mut input = take_input(&fixture);
    input.client_uuid = None;
    input.taken_at = "2026-10-05T21:00:00Z".into();
    let outcome = take_in_zone(
        &fixture,
        Command::Take(input.validate().unwrap()),
        chrono_tz::Europe::London,
    )
    .await;
    assert!(matches!(outcome, Ok(Outcome::Created(_))));

    let mut input = take_input(&fixture);
    input.client_uuid = None;
    input.taken_at = "2026-10-06T01:00:00Z".into();
    let outcome = take_in_zone(
        &fixture,
        Command::Take(input.validate().unwrap()),
        chrono_tz::Europe::London,
    )
    .await;
    assert!(matches!(outcome, Err(OperationError::Validation { .. })));

    let mut input = take_input(&fixture);
    input.client_uuid = None;
    input.taken_at = "2026-10-06T03:00:00Z".into();
    let outcome = take_in_zone(
        &fixture,
        Command::Take(input.validate().unwrap()),
        chrono_tz::Europe::London,
    )
    .await;
    assert!(matches!(outcome, Ok(Outcome::Created(_))));

    fixture
        .update_assignment(|row| row.active = Set(false))
        .await;
    let schedule = fixture.schedule(5, serde_json::json!({"max_daily_doses":1,"min_hours_between_doses":4,"unit":"tablet","taper_steps":[{"start_date":"2026-10-07","end_date":"2026-10-09","amount":"1"}]}), "2026-10-01", "2026-10-31", 1).await;

    let taper_take1 = Take {
        client_uuid: None,
        source_type: "schedule".into(),
        source_id: schedule.id.to_string(),
        taken_at: "2026-10-07T09:00:00Z".into(),
        dose_amount: Some("1".into()),
        dose_unit: Some("tablet".into()),
        taken_from_medication_id: Some(fixture.medication.id),
        expected_effective_amount: None,
        expected_effective_unit: None,
    };
    let outcome = take_in_zone(
        &fixture,
        Command::Take(taper_take1.validate().unwrap()),
        chrono_tz::Europe::London,
    )
    .await;
    assert!(matches!(outcome, Ok(Outcome::Created(_))));

    let taper_attempt = Take {
        client_uuid: None,
        source_type: "schedule".into(),
        source_id: schedule.id.to_string(),
        taken_at: "2026-10-07T15:00:00Z".into(),
        dose_amount: Some("1".into()),
        dose_unit: Some("tablet".into()),
        taken_from_medication_id: Some(fixture.medication.id),
        expected_effective_amount: None,
        expected_effective_unit: None,
    };
    let outcome = take_in_zone(
        &fixture,
        Command::Take(taper_attempt.validate().unwrap()),
        chrono_tz::Europe::London,
    )
    .await;
    assert!(matches!(outcome, Err(OperationError::Validation { .. })));

    let taper_take2 = Take {
        client_uuid: None,
        source_type: "schedule".into(),
        source_id: schedule.id.to_string(),
        taken_at: "2026-10-07T23:30:00Z".into(),
        dose_amount: Some("1".into()),
        dose_unit: Some("tablet".into()),
        taken_from_medication_id: Some(fixture.medication.id),
        expected_effective_amount: None,
        expected_effective_unit: None,
    };
    let outcome = take_in_zone(
        &fixture,
        Command::Take(taper_take2.validate().unwrap()),
        chrono_tz::Europe::London,
    )
    .await;
    assert!(matches!(outcome, Ok(Outcome::Created(_))));

    assert_eq!(fixture.effect().await, (4, "4.00".into(), 4, 4));
    fixture.close().await;
}
