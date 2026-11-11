#[path = "support/clinical_fixture.rs"]
mod clinical_fixture;
#[path = "support/fixtures.rs"]
pub mod fixtures;
#[path = "support/scenario.rs"]
pub mod scenario;
use futures_util::FutureExt;
use loco_rs::prelude::ConnectionTrait;
use migration::MigratorTrait;

#[tokio::test]
async fn medication_barcode_upgrade_preserves_rows_and_household_uniqueness() {
    schema_case(async {
        use med_tracker::models::_entities::medications;
        use migration::SchemaManager;
        use sea_orm::sea_query::{ConditionalStatement, Expr, ExprTrait, Index};
        use sea_orm::{ActiveModelTrait, EntityTrait, Set, prelude::Decimal};

        let db = restored_database().await;
        let upgrade_position = migration::Migrator::migrations()
            .iter()
            .position(|migration| {
                migration.name() == "m20261111_000036_household_medication_barcodes"
            })
            .unwrap();
        migration::Migrator::up(&db, Some(u32::try_from(upgrade_position).unwrap()))
            .await
            .unwrap();
        let manager = SchemaManager::new(&db);
        manager
            .drop_index(
                Index::drop()
                    .name("index_medications_on_barcode")
                    .table("medications")
                    .to_owned(),
            )
            .await
            .unwrap();
        manager
            .create_index(
                Index::create()
                    .name("index_medications_on_barcode")
                    .table("medications")
                    .unique()
                    .col("barcode")
                    .and_where(Expr::col("barcode").is_not_null())
                    .and_where(Expr::col("barcode").ne(""))
                    .to_owned(),
            )
            .await
            .unwrap();

        let first = scenario::Scenario::new().await;
        let second = scenario::Scenario::new().await;
        let first_medication = first
            .medication("Synthetic first tablets", Decimal::ZERO)
            .await;
        let second_medication = second
            .medication("Synthetic second tablets", Decimal::ZERO)
            .await;
        let spare = first
            .medication("Synthetic spare tablets", Decimal::ZERO)
            .await;
        let barcode = format!("2{:012}", first.household.id);
        let original = medications::ActiveModel {
            id: Set(first_medication.id),
            barcode: Set(Some(barcode.clone())),
            ..Default::default()
        }
        .update(&db)
        .await
        .unwrap();

        migration::Migrator::up(&db, None).await.unwrap();
        let preserved = medications::Entity::find_by_id(original.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(preserved, original);
        let shared = medications::ActiveModel {
            id: Set(second_medication.id),
            barcode: Set(Some(barcode.clone())),
            ..Default::default()
        }
        .update(&db)
        .await
        .unwrap();
        assert_eq!(shared.barcode, original.barcode);
        assert_ne!(shared.household_id, original.household_id);

        let error = medications::ActiveModel {
            id: Set(spare.id),
            barcode: Set(Some(barcode)),
            ..Default::default()
        }
        .update(&db)
        .await
        .unwrap_err();
        assert!(
            matches!(error.sql_err(), Some(sea_orm::SqlErr::UniqueConstraintViolation(name))
            if name.contains("index_medications_on_barcode"))
        );
        first.close().await;
        second.close().await;
        db.close().await.unwrap();
    })
    .await;
}

#[tokio::test]
async fn fixture_slices_preserve_baseline_and_care_memberships() {
    schema_case(async {
        use med_tracker::models::_entities::{accounts, household_memberships, medications};
        use sea_orm::{EntityTrait, PaginatorTrait};

        let db = restored_database().await;
        migration::Migrator::up(&db, None).await.unwrap();
        fixtures::seed_persistence(&db).await;
        let account = accounts::Entity::find_by_id(71001)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(account.status, 1);
        assert_eq!(
            household_memberships::Entity::find()
                .count(&db)
                .await
                .unwrap(),
            1
        );
        assert!(account.active);
        fixtures::seed_care_doses(&db).await;
        assert_eq!(
            accounts::Entity::find_by_id(71001)
                .one(&db)
                .await
                .unwrap()
                .unwrap()
                .status,
            2
        );
        assert_eq!(
            household_memberships::Entity::find()
                .count(&db)
                .await
                .unwrap(),
            3
        );
        assert_eq!(accounts::Entity::find().count(&db).await.unwrap(), 1);
        assert_eq!(medications::Entity::find().count(&db).await.unwrap(), 1);
        db.close().await.unwrap();
    })
    .await
}

async fn restored_database() -> loco_rs::prelude::DatabaseConnection {
    let verified = scenario::setup_connection().await;
    verified.close().await.unwrap();
    let mut options = migration::sea_orm::ConnectOptions::new(
        std::env::var("MEDTRACKER_TEST_SETUP_DATABASE_URL").unwrap(),
    );
    options.min_connections(1).max_connections(1);
    migration::sea_orm::Database::connect(options)
        .await
        .unwrap()
}

async fn reset_schema(migrate: bool) {
    let db = scenario::setup_connection().await;
    // SeaQuery's SchemaStatement and SeaORM's SchemaManager cannot express
    // PostgreSQL DROP SCHEMA CASCADE. Only whole-run exclusive tests use this
    // exception, after setup_connection verifies the UUID database owner marker.
    db.execute_unprepared(
        "DROP SCHEMA IF EXISTS public CASCADE; DROP SCHEMA IF EXISTS med_tracker CASCADE; CREATE SCHEMA public AUTHORIZATION pg_database_owner; GRANT USAGE ON SCHEMA public TO PUBLIC",
    )
    .await
    .unwrap();
    if migrate {
        migration::Migrator::up(&db, None).await.unwrap();
    }
    db.close().await.unwrap();
}

async fn schema_case<T>(body: impl std::future::Future<Output = T>) -> T {
    reset_schema(false).await;
    let result = std::panic::AssertUnwindSafe(body).catch_unwind().await;
    reset_schema(true).await;
    match result {
        Ok(value) => value,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

async fn ledger_rows(db: &loco_rs::prelude::DatabaseConnection) -> Vec<(String, i64)> {
    db.query_all_raw(migration::sea_orm::Statement::from_string(
        migration::sea_orm::DbBackend::Postgres,
        "SELECT version, applied_at FROM public.seaql_migrations ORDER BY version",
    ))
    .await
    .unwrap()
    .into_iter()
    .map(|row| {
        (
            row.try_get("", "version").unwrap(),
            row.try_get("", "applied_at").unwrap(),
        )
    })
    .collect()
}

#[tokio::test]
async fn fresh_database_is_created_by_loco_migrations() {
    schema_case(async {
        let db = restored_database().await;
        migration::Migrator::up(&db, None)
            .await
            .expect("Loco must build an empty database without a schema import");
        let ledger = ledger_rows(&db).await;
        assert!(!ledger.is_empty());
        assert!(
            ledger
                .iter()
                .any(|row| row.0 == "m20261102_000029_nhs_dmd_import_trace_id")
        );
        migration::Migrator::up(&db, None).await.unwrap();
        assert_eq!(ledger_rows(&db).await, ledger);
        db.close().await.unwrap();
    })
    .await
}

#[tokio::test]
async fn adoption_fact_backfill_indexes_pre_existing_linked_take_evidence_idempotently() {
    schema_case(async {
        use med_tracker::models::_entities::{
            medication_takes, medications, person_schedules, schedules,
            shared_schedule_adoption_facts, shared_schedule_administrations,
            shared_schedule_executions, shared_schedule_states, versions,
        };
        use sea_orm::{
            ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait,
            QueryFilter, prelude::Decimal,
        };
        let db = restored_database().await;
        // Apply through m20261029_000024_live_adoption_history, leaving the
        // frozen-evidence and backfill migrations pending.
        migration::Migrator::up(&db, Some(23)).await.unwrap();
        fixtures::seed_persistence(&db).await;
        fixtures::seed_care_doses(&db).await;
        // SeaORM/SeaQuery cannot toggle validation triggers. These seed writes
        // emulate pre-feature rows that runtime paths originally wrote under
        // tenant authority, so the runtime guards are paused exactly around the
        // fixture statements and restored before the pending migrations run.
        db.execute_unprepared(
            "ALTER TABLE public.schedules DISABLE TRIGGER shared_execution_guard;\
             ALTER TABLE public.person_schedules DISABLE TRIGGER legacy_canonical_insert_guard;\
             ALTER TABLE public.shared_schedule_states DISABLE TRIGGER shared_schedule_guard",
        )
        .await
        .unwrap();
        let stock = medications::Entity::find_by_id(80001)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        let now = chrono::Utc::now().naive_utc();
        let plan_id = uuid::Uuid::new_v4().to_string();
        let source = schedules::ActiveModel {
            household_id: Set(72001),
            person_id: Set(73001),
            medication_id: Set(80001),
            active: Set(true),
            start_date: Set(Some("2020-01-01".parse().unwrap())),
            end_date: Set(Some("2099-12-31".parse().unwrap())),
            dose_amount: Set(Some(Decimal::new(2, 0))),
            dose_unit: Set(Some("tablet".into())),
            schedule_type: Set(6),
            schedule_config: Set(serde_json::json!({})),
            portable_id: Set(uuid::Uuid::new_v4().to_string()),
            person_schedule_id: Set(None),
            canonical_origin: Set(true),
            clinical_revision: Set(Some(1)),
            origin_time_zone: Set(Some("Europe/London".into())),
            created_at: Set(now - chrono::Duration::days(3)),
            updated_at: Set(now - chrono::Duration::days(2)),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        let taken_at = now - chrono::Duration::days(1);
        let take = medication_takes::ActiveModel {
            household_id: Set(72001),
            schedule_id: Set(Some(source.id)),
            portable_id: Set(uuid::Uuid::new_v4().to_string()),
            client_uuid: Set(Some(uuid::Uuid::new_v4().to_string())),
            dose_amount: Set(Some(Decimal::new(2, 0))),
            dose_unit: Set(Some("tablet".into())),
            taken_at: Set(Some(taken_at)),
            taken_from_medication_id: Set(Some(80001)),
            created_at: Set(taken_at),
            updated_at: Set(taken_at),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        let snapshot = serde_json::json!({
            "id":take.id,"portable_id":take.portable_id,"client_uuid":take.client_uuid,
            "household_id":72001,"schedule_id":source.id,"person_medication_id":null,
            "person_id":73001,"medication_id":80001,"taken_from_medication_id":80001,
            "taken_from_location_id":null,"dose_amount":"2","dose_unit":"tablet",
            "taken_at":taken_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "created_at":take.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "updated_at":take.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string()
        });
        let changes: serde_json::Map<String, serde_json::Value> = snapshot
            .as_object()
            .unwrap()
            .iter()
            .map(|(key, value)| (key.clone(), serde_json::json!([null, value])))
            .collect();
        versions::ActiveModel {
            item_type: Set("MedicationTake".into()),
            item_id: Set(take.id),
            event: Set("create".into()),
            object: Set(Some(snapshot.to_string())),
            object_changes: Set(Some(
                serde_json::Value::Object(changes).to_string(),
            )),
            whodunnit: Set(Some("71001".into())),
            request_id: Set(Some("seeded-take-request".into())),
            household_id: Set(Some(72001)),
            actor_membership_id: Set(Some(74001)),
            audit_context: Set(serde_json::json!({"actor_account_id":71001,"actor_person_id":73001,"actor_membership_id":74001,"household_id":72001,"request_id":"seeded-take-request","active_role":"administrator","permissions_version":1,"authentication_method":"api_session","session_reference":"api_session:synthetic"})),
            created_at: Set(Some(taken_at)),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        person_schedules::ActiveModel {
            id: Set(plan_id.clone()),
            person_id: Set(73001),
            origin_household_id: Set(72001),
            origin_schedule_id: Set(source.id),
            origin_stock_id: Set(80001),
            origin_stock_portable_id: Set(stock.portable_id.clone()),
            medicine_name: Set(stock.name.clone()),
            stock_unit: Set(stock.dose_unit.clone()),
            stock_default_known: Set(true),
            origin_time_zone: Set("Europe/London".into()),
            timezone_provenance: Set("preserved_origin_zone".into()),
            clinical_revision: Set(1),
            active: Set(true),
            dose_amount: Set(Some(Decimal::new(2, 0))),
            dose_unit: Set(Some("tablet".into())),
            start_date: Set(Some("2020-01-01".parse().unwrap())),
            end_date: Set(Some("2099-12-31".parse().unwrap())),
            schedule_type: Set(6),
            schedule_config: Set(serde_json::json!({})),
            created_at: Set(now - chrono::Duration::days(2)),
            updated_at: Set(now - chrono::Duration::days(2)),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        shared_schedule_states::ActiveModel {
            id: Set(plan_id.clone()),
            person_id: Set(73001),
            medicine_name: Set(stock.name.clone()),
            stock_unit: Set(stock.dose_unit.clone()),
            stock_default_known: Set(true),
            origin_time_zone: Set("Europe/London".into()),
            clinical_revision: Set(1),
            active: Set(true),
            dose_amount: Set(Some(Decimal::new(2, 0))),
            dose_unit: Set(Some("tablet".into())),
            start_date: Set(Some("2020-01-01".parse().unwrap())),
            end_date: Set(Some("2099-12-31".parse().unwrap())),
            schedule_type: Set(6),
            schedule_config: Set(serde_json::json!({})),
            created_at: Set(now - chrono::Duration::days(2)),
            updated_at: Set(now - chrono::Duration::days(2)),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        shared_schedule_executions::ActiveModel {
            schedule_id: Set(source.id),
            portable_id: Set(source.portable_id.clone()),
            household_id: Set(72001),
            person_id: Set(73001),
            person_schedule_id: Set(plan_id.clone()),
            approval_id: Set(None),
        }
        .insert(&db)
        .await
        .unwrap();
        // The pre-feature link between the source and its canonical plan is
        // restored while the runtime guards are still paused.
        schedules::Entity::update_many()
            .col_expr(
                schedules::Column::PersonScheduleId,
                sea_orm::sea_query::Expr::val(plan_id.clone()),
            )
            .filter(schedules::Column::Id.eq(source.id))
            .exec(&db)
            .await
            .unwrap();
        db.execute_unprepared(
            "ALTER TABLE public.schedules ENABLE TRIGGER shared_execution_guard;\
             ALTER TABLE public.person_schedules ENABLE TRIGGER legacy_canonical_insert_guard;\
             ALTER TABLE public.shared_schedule_states ENABLE TRIGGER shared_schedule_guard",
        )
        .await
        .unwrap();
        let request = uuid::Uuid::new_v4().to_string();
        versions::ActiveModel {
            item_type: Set("PersonSchedule".into()),
            item_id: Set(source.id),
            event: Set("create".into()),
            object: Set(None),
            object_changes: Set(Some(
                serde_json::json!({"id":[null,plan_id.clone()]}).to_string(),
            )),
            whodunnit: Set(Some("71001".into())),
            request_id: Set(Some(request.clone())),
            household_id: Set(Some(72001)),
            actor_membership_id: Set(Some(74001)),
            audit_context: Set(serde_json::json!({"actor_account_id":71001,"actor_person_id":73001,"actor_membership_id":74001,"household_id":72001,"request_id":request,"active_role":"administrator","permissions_version":1,"authentication_method":"api_session","session_reference":"api_session:synthetic-grant"})),
            created_at: Set(Some(now - chrono::Duration::days(2))),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        migration::Migrator::up(&db, None).await.unwrap();
        let facts = shared_schedule_adoption_facts::Entity::find()
            .all(&db)
            .await
            .unwrap();
        assert_eq!(facts.len(), 1, "The linked original take must be backfilled");
        assert_eq!(facts[0].medication_take_id, take.id);
        assert_eq!(facts[0].person_schedule_id, Some(plan_id.clone()));
        assert_eq!(facts[0].source_schedule_id, source.id);
        assert_eq!(facts[0].person_id, 73001);
        assert_eq!(facts[0].household_id, 72001);
        assert_eq!(facts[0].taken_at, take.taken_at.unwrap());
        assert!(!facts[0].fact_hash.is_empty());
        assert_eq!(facts[0].captured_by_account_id, 71001);
        assert_eq!(facts[0].captured_by_membership_id, 74001);
        assert_eq!(facts[0].request_id, request);
        assert_eq!(facts[0].guardian_declaration_id, None);
        let entries = shared_schedule_administrations::Entity::find()
            .all(&db)
            .await
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].person_schedule_id, plan_id);
        assert_eq!(entries[0].medication_take_id, take.id);
        assert_eq!(entries[0].taken_at, take.taken_at.unwrap());
        let state = shared_schedule_states::Entity::find_by_id(&plan_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(state.latest_taken_at, take.taken_at);
        migration::Migrator::up(&db, None).await.unwrap();
        assert_eq!(
            shared_schedule_adoption_facts::Entity::find()
                .count(&db)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            shared_schedule_administrations::Entity::find()
                .count(&db)
                .await
                .unwrap(),
            1
        );
        db.close().await.unwrap();
    })
    .await
}

#[tokio::test]
async fn migrations_provision_the_standard_durable_browser_session_store() {
    let db = restored_database().await;
    let exists = db
        .query_one_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            "SELECT to_regclass('public.browser_sessions') IS NOT NULL AS present",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<bool>("", "present")
        .unwrap();
    db.close().await.unwrap();
    assert!(exists, "Standard durable browser session store is absent");
}

#[tokio::test]
async fn medication_review_history_is_append_only_for_the_application_role() {
    let db = restored_database().await;
    let row = db
        .query_one_raw(migration::sea_orm::Statement::from_string(
            migration::sea_orm::DbBackend::Postgres,
            "SELECT has_table_privilege('med_tracker_app','public.medication_review_histories','SELECT') AS readable,has_table_privilege('med_tracker_app','public.medication_review_histories','INSERT') AS appendable,has_table_privilege('med_tracker_app','public.medication_review_histories','UPDATE') AS mutable,has_table_privilege('med_tracker_app','public.medication_review_histories','DELETE') AS deletable",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(row.try_get::<bool>("", "readable").unwrap());
    assert!(row.try_get::<bool>("", "appendable").unwrap());
    assert!(!row.try_get::<bool>("", "mutable").unwrap());
    assert!(!row.try_get::<bool>("", "deletable").unwrap());
    db.close().await.unwrap();
}

#[tokio::test]
async fn registration_policy_exposes_only_active_owner_existence() {
    schema_case(async {
        let db = restored_database().await;
        migration::Migrator::up(&db, None).await.unwrap();
        fixtures::seed_persistence(&db).await;
        for (change, expected) in [
            ("", false),
            (
                "UPDATE public.household_memberships SET role='owner' WHERE id=74001",
                true,
            ),
            (
                "UPDATE public.household_memberships SET status='revoked' WHERE id=74001",
                false,
            ),
        ] {
            db.execute_unprepared("RESET ROLE").await.unwrap();
            if !change.is_empty() {
                db.execute_unprepared(change).await.unwrap();
            }
            db.execute_unprepared("SET ROLE med_tracker_app")
                .await
                .unwrap();
            let result = db.query_one_raw(migration::sea_orm::Statement::from_string(
                migration::sea_orm::DbBackend::Postgres,
                "SELECT public.registration_has_active_owner() AS has_owner, (SELECT count(*) FROM public.household_memberships) AS visible_memberships, pg_has_role(current_user, 'med_tracker_owner', 'MEMBER') AS can_assume_owner_role",
            )).await.unwrap().unwrap();
            assert_eq!(result.try_get::<bool>("", "has_owner").unwrap(), expected);
            assert_eq!(result.try_get::<i64>("", "visible_memberships").unwrap(), 0);
            assert!(!result.try_get::<bool>("", "can_assume_owner_role").unwrap());
        }
        db.close().await.unwrap();
    }).await
}

#[tokio::test]
async fn development_seed_creates_a_usable_account_and_household() {
    schema_case(async {
        use loco_rs::{
            app::{AppContext, Hooks},
            config::Config,
            environment::Environment,
        };
        use med_tracker::models::{
            access::{self, Actor, HouseholdScope},
            entities::{account, medication},
        };
        use sea_orm::EntityTrait;
        let db = restored_database().await;
        let context = AppContext::builder(
            Environment::Development,
            db.clone(),
            Config::new(&Environment::Development).unwrap(),
        )
        .build();
        let owned_url = std::env::var("MEDTRACKER_TEST_SETUP_DATABASE_URL").unwrap();
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(120),
            tokio::process::Command::new(env!("CARGO_BIN_EXE_med-tracker"))
                .args(["db", "seed", "--environment", "test"])
                .env("DATABASE_URL", &owned_url)
                .kill_on_drop(true)
                .output(),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(
            result.status.success(),
            "Standard development seeding must be usable: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let account = account::Entity::find_by_id(71001)
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(account.email, "persistence@example.test");
        let tenant = access::begin(
            &db,
            &HouseholdScope {
                actor: Actor {
                    account_id: account.id,
                },
                household_id: 72001,
                request_id: "seed-proof".into(),
            },
            None,
        )
        .await
        .unwrap();
        let medicines = access::medication_scope(&tenant)
            .all(tenant.transaction())
            .await
            .unwrap();
        assert_eq!(medicines.len(), 1);
        assert_eq!(medicines[0].id, 80001);
        assert_eq!(medicines[0].current_supply.unwrap().to_string(), "10.00");
        tenant.rollback().await.unwrap();
        let production =
            AppContext::builder(Environment::Production, db.clone(), context.config.clone())
                .build();
        assert!(
            med_tracker::app::App::seed(&production, std::path::Path::new("src/fixtures"))
                .await
                .is_err()
        );
        assert_eq!(medication::Entity::find().all(&db).await.unwrap().len(), 1);
        assert!(
            med_tracker::app::App::seed(&context, std::path::Path::new("src/fixtures"))
                .await
                .is_err()
        );
        db.close().await.unwrap();
        let runtime_uri = std::env::var("DATABASE_URL").unwrap();
        let mut config = Config::new(&Environment::Test).unwrap();
        config.database.uri = runtime_uri.clone();
        let Some(loco_rs::config::QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
            panic!("PostgreSQL queue required")
        };
        queue.uri = runtime_uri.clone();
        let boot = med_tracker::app::App::boot(
            loco_rs::boot::StartMode::ServerOnly,
            &Environment::Test,
            config,
        )
        .await
        .unwrap();
        let mut server = loco_rs::TestServer::new(
            boot.router
                .as_ref()
                .unwrap()
                .clone()
                .into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .unwrap();
        server.save_cookies();
        let login = server
            .post("/api/auth/sign-in/email")
            .add_header("origin", boot.app_context.config.server.full_url())
            .json(&serde_json::json!({"email":"persistence@example.test","password":"password"}))
            .await;
        assert_eq!(login.status_code(), 200, "{}", login.text());
        assert_eq!(
            login.json::<serde_json::Value>()["user"]["id"]
                .to_string()
                .trim_matches('"'),
            "71001"
        );
        let session = server.get("/api/auth/get-session").await;
        assert_eq!(session.status_code(), 200, "{}", session.text());
        assert_eq!(
            session.json::<serde_json::Value>()["user"]["id"]
                .to_string()
                .trim_matches('"'),
            "71001"
        );
        let medicines = server
            .get("/households/persistence-fixture/medications")
            .await;
        assert_eq!(medicines.status_code(), 200, "{}", medicines.text());
        assert!(medicines.text().contains("Synthetic"));
        assert_eq!(server.get("/admin/nhs-dmd").await.status_code(), 403);
        drop(server);
        let mut platform_server = loco_rs::TestServer::new(
            boot.router
                .unwrap()
                .into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .unwrap();
        platform_server.save_cookies();
        let login = platform_server
            .post("/api/auth/sign-in/email")
            .add_header("origin", boot.app_context.config.server.full_url())
            .json(&serde_json::json!({"email":"platform-admin@example.test","password":"password"}))
            .await;
        assert_eq!(login.status_code(), 200, "{}", login.text());
        let platform_users = platform_server.get("/platform/users").await;
        assert_eq!(
            platform_users.status_code(),
            200,
            "{}",
            platform_users.text()
        );
        let importer = platform_server.get("/admin/nhs-dmd").await;
        assert_eq!(importer.status_code(), 200, "{}", importer.text());
        assert_eq!(
            platform_server
                .get("/households/persistence-fixture/admin")
                .await
                .status_code(),
            403
        );
        drop(platform_server);
        if let Some(queue) = boot.app_context.queue_provider.as_ref() {
            queue.shutdown().unwrap();
        }
        boot.app_context.db.close().await.unwrap();
    })
    .await
}

#[tokio::test]
async fn fresh_migration_refuses_an_existing_accounts_table() {
    schema_case(async {
        let db = restored_database().await;
        db.execute_unprepared(
            "CREATE TABLE accounts (sentinel integer); INSERT INTO accounts VALUES (42)",
        )
        .await
        .unwrap();
        assert!(migration::Migrator::up(&db, None).await.is_err());
        db.close().await.unwrap();
    })
    .await
}

#[tokio::test]
async fn fresh_audit_triggers_preserve_concurrent_chain_and_runtime_permissions() {
    schema_case(async {
        use migration::sea_orm::{DbBackend, Statement};
        let db = restored_database().await;
        migration::Migrator::up(&db, None).await.unwrap();
        fixtures::seed_persistence(&db).await;
        let uri = std::env::var("DATABASE_URL").unwrap();
        let baseline: i64 = db
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT count(*) AS entries FROM audit_ledger_entries",
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "entries")
            .unwrap();
        let mut writers = tokio::task::JoinSet::new();
        for _ in 0..6 {
            let uri = uri.clone();
            writers.spawn(async move {
                let mut options = migration::sea_orm::ConnectOptions::new(uri);
                options.max_connections(1);
                let writer = migration::sea_orm::Database::connect(options).await.unwrap();
                let result = std::panic::AssertUnwindSafe(async {
                writer.execute_unprepared("SET med_tracker.current_household_id = '72001'; INSERT INTO security_audit_events(household_id,event_type,created_at,updated_at) VALUES (72001,'migration-proof',now(),now()); INSERT INTO versions(household_id,item_type,item_id,event,created_at) VALUES (72001,'Medication',80001,'update',now())").await.unwrap();
                for statement in [
                    "INSERT INTO audit_ledger_entries DEFAULT VALUES",
                    "UPDATE audit_ledger_entries SET sequence=0",
                    "DELETE FROM audit_ledger_entries",
                    "UPDATE security_audit_events SET event_type='tampered'",
                    "DELETE FROM versions",
                    "UPDATE seaql_migrations SET applied_at=0",
                ] {
                    assert!(writer.execute_unprepared(statement).await.is_err(), "Runtime accepted {statement}");
                }
                }).catch_unwind().await;
                writer.close().await.unwrap();
                if let Err(payload) = result {
                    std::panic::resume_unwind(payload);
                }
            });
        }
        let mut failures = Vec::new();
        while let Some(result) = writers.join_next().await {
            if let Err(error) = result {
                failures.push(error);
            }
        }
        assert!(failures.is_empty(), "Audit writers failed: {failures:?}");
        let clinical = db.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM security_audit_events WHERE event_type='migration-proof') AS security,(SELECT count(*) FROM versions WHERE item_type='Medication' AND item_id=80001 AND event='update') AS medications")).await.unwrap().unwrap();
        assert_eq!(clinical.try_get::<i64>("", "security").unwrap(), 6);
        assert_eq!(clinical.try_get::<i64>("", "medications").unwrap(), 6);
        let row = db.query_one_raw(Statement::from_string(DbBackend::Postgres, r#"
            SELECT count(*)::bigint AS entries,
                bool_and(sequence = ordinal AND previous_hash IS NOT DISTINCT FROM previous_entry_hash) AS linked,
                bool_and(entry_hash = digest(convert_to('medtracker.audit.ledger.v1' || chr(31) || chain_key || chr(31) || chain_epoch::text || chr(31) || sequence::text || chr(31), 'UTF8') || COALESCE(previous_hash, '\x'::bytea) || canonical_payload, 'sha256')) AS hashes,
                bool_and(retain_until >= occurred_at + interval '9 years') AS retained
            FROM (SELECT *, row_number() OVER (PARTITION BY chain_key,chain_epoch ORDER BY sequence) AS ordinal, lag(entry_hash) OVER (PARTITION BY chain_key,chain_epoch ORDER BY sequence) AS previous_entry_hash FROM audit_ledger_entries) entries
        "#)).await.unwrap().unwrap();
        assert_eq!(row.try_get::<i64>("", "entries").unwrap(), baseline + 12);
        for column in ["linked", "hashes", "retained"] {
            assert!(
                row.try_get::<bool>("", column).unwrap(),
                "Audit check failed: {column}"
            );
        }
        let pending: i64 = db.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT count(*)::bigint AS pending FROM audit_export_deliveries WHERE status='pending' AND audit_ledger_entry_id IS NOT NULL")).await.unwrap().unwrap().try_get("", "pending").unwrap();
        assert_eq!(pending, baseline + 12);
        db.execute_unprepared("DELETE FROM security_audit_events; DELETE FROM versions")
            .await
            .unwrap();
        let retained: i64 = db
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT count(*)::bigint AS retained FROM audit_ledger_entries",
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "retained")
            .unwrap();
        assert_eq!(retained, baseline + 12);
        db.close().await.unwrap();
    }).await
}

#[test]
fn development_has_a_stable_session_key() {
    use base64::Engine;
    use loco_rs::{config::Config, environment::Environment};
    let first = Config::new(&Environment::Development).unwrap();
    let second = Config::new(&Environment::Development).unwrap();
    let key = first
        .settings
        .as_ref()
        .and_then(|settings| settings["browser_session"]["key"].as_str())
        .expect("Development must provide a stable session key");
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(key)
            .unwrap()
            .len(),
        64
    );
    assert_eq!(
        second.settings.as_ref().unwrap()["browser_session"]["key"],
        key
    );
}

#[tokio::test]
async fn owned_postgres_fixture() {
    let setup = scenario::setup_connection().await;
    setup.close().await.unwrap();
    let scenario = scenario::Scenario::new().await;
    let run = uuid::Uuid::parse_str(&std::env::var("MEDTRACKER_TEST_RUN_ID").unwrap()).unwrap();
    assert_eq!(
        scenario.database_name().await,
        format!("api_run_{}", run.simple())
    );
    let row = scenario.runtime.query_one_raw(migration::sea_orm::Statement::from_string(
        scenario.runtime.get_database_backend(),
        "SELECT current_setting('server_version_num')::int AS version, r.rolsuper, r.rolcreatedb, r.rolcreaterole, r.rolbypassrls, pg_has_role(current_user, 'med_tracker_app', 'MEMBER') AS app_member, pg_has_role(current_user, 'med_tracker_owner', 'MEMBER') AS owner_member FROM pg_catalog.pg_roles r WHERE r.rolname=current_user",
    )).await.unwrap().unwrap();
    let version: i32 = row.try_get("", "version").unwrap();
    let app_member: bool = row.try_get("", "app_member").unwrap();
    let forbidden = [
        "rolsuper",
        "rolcreatedb",
        "rolcreaterole",
        "rolbypassrls",
        "owner_member",
    ]
    .map(|column| row.try_get::<bool>("", column).unwrap());
    scenario.close().await;
    assert!((180000..190000).contains(&version));
    assert!(app_member);
    assert!(forbidden.into_iter().all(|value| !value));
    assert_ne!(
        std::env::var("MEDTRACKER_PERSISTENCE_FORCE_FAILURE").as_deref(),
        Ok("1"),
        "Forced persistence fixture failure"
    );
}

#[tokio::test]
async fn fresh_setup_refuses_an_unrecognised_schema_without_changing_data() {
    schema_case(async {
        use med_tracker::models::_entities::accounts;
        use sea_orm::{
            EntityTrait,
            sea_query::{Alias, Expr, ExprTrait, Query},
        };

        let db = restored_database().await;
        migration::Migrator::up(&db, None).await.unwrap();
        fixtures::seed_persistence(&db).await;
        db.execute(
            Query::update()
                .table(Alias::new("seaql_migrations"))
                .value(Alias::new("version"), "retired_authentication_baseline")
                .and_where(
                    Expr::col(Alias::new("version"))
                        .eq("m20261013_000001_create_medtracker_native"),
                ),
        )
        .await
        .unwrap();
        let ledger = ledger_rows(&db).await;
        let account = accounts::Entity::find_by_id(71001).one(&db).await.unwrap();
        assert!(account.is_some());
        assert!(migration::Migrator::up(&db, None).await.is_err());
        assert_eq!(ledger_rows(&db).await, ledger);
        assert_eq!(
            accounts::Entity::find_by_id(71001).one(&db).await.unwrap(),
            account
        );
        db.close().await.unwrap();
    })
    .await
}

async fn setup_context(
    directory: &std::path::Path,
    runtime: Option<&str>,
    update: Option<&str>,
) -> loco_rs::app::AppContext {
    use loco_rs::{app::Hooks, config::QueueConfig, environment::Environment};
    let mut config = med_tracker::app::App::load_config(&Environment::Production)
        .await
        .unwrap();
    config.database.uri = std::env::var("MEDTRACKER_TEST_SETUP_DATABASE_URL").unwrap();
    if let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() {
        queue.uri.clone_from(&config.database.uri);
    }
    config.settings.as_mut().unwrap()["database_setup"] = serde_json::json!({
        "runtime_url":runtime,"update_url":update,"output_directory":directory,"persist_config":true
    });
    loco_rs::boot::create_context::<med_tracker::app::App>(&Environment::Production, config)
        .await
        .unwrap()
}

async fn setup_access_case(supplied: bool) {
    schema_case(async {
        use loco_rs::{
            app::Hooks,
            boot::StartMode,
            config::{Config, QueueConfig},
            environment::Environment,
            task::{Tasks, Vars},
        };
        use sea_orm::{Database, EntityTrait};
        let directory = tempfile::tempdir().unwrap();
        let control =
            url::Url::parse(&std::env::var("MEDTRACKER_TEST_SETUP_DATABASE_URL").unwrap()).unwrap();
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        let chosen = |kind: &str| {
            let mut uri = control.clone();
            uri.set_username(&format!("setup_{kind}_{suffix}")).unwrap();
            uri.set_password(Some("password'with:+@symbols")).unwrap();
            uri.to_string()
        };
        let chosen_runtime = chosen("runtime");
        let chosen_update = chosen("update");
        let ctx = setup_context(
            directory.path(),
            supplied.then_some(chosen_runtime.as_str()),
            supplied.then_some(chosen_update.as_str()),
        )
        .await;
        let mut tasks = Tasks::default();
        med_tracker::app::App::register_tasks(&mut tasks);
        tasks
            .run(&ctx, "database-setup", &Vars::default())
            .await
            .unwrap();
        let read_uri = |kind: &str| {
            let path = directory.path().join(kind).join("production.local.yaml");
            let contents = std::fs::read(path).unwrap();
            let value: serde_yaml_ng::Value = serde_yaml_ng::from_slice(&contents).unwrap();
            value["settings"]["database_setup"]["stored_database_url"]
                .as_str()
                .unwrap()
                .to_owned()
        };
        let runtime_uri = read_uri("runtime");
        let update_uri = read_uri("update");
        if supplied {
            assert_eq!(runtime_uri, chosen_runtime);
            assert_eq!(update_uri, chosen_update);
        }
        let runtime = Database::connect(runtime_uri.clone()).await.unwrap();
        let update = Database::connect(update_uri.clone()).await.unwrap();
        migration::provision::validate(
            &runtime,
            url::Url::parse(&runtime_uri).unwrap().username(),
            migration::provision::Access::Runtime,
        )
        .await
        .unwrap();
        migration::provision::validate(
            &update,
            url::Url::parse(&update_uri).unwrap().username(),
            migration::provision::Access::Update,
        )
        .await
        .unwrap();
        assert!(migration::Migrator::up(&runtime, None).await.is_err());
        migration::Migrator::up(&update, None).await.unwrap();
        fixtures::seed_persistence(&ctx.db).await;
        let before = med_tracker::models::_entities::accounts::Entity::find_by_id(71001)
            .one(&ctx.db)
            .await
            .unwrap()
            .unwrap();
        tasks
            .run(&ctx, "database-setup", &Vars::default())
            .await
            .unwrap();
        assert_eq!(read_uri("runtime"), runtime_uri);
        assert_eq!(read_uri("update"), update_uri);
        assert_eq!(
            med_tracker::models::_entities::accounts::Entity::find_by_id(71001)
                .one(&ctx.db)
                .await
                .unwrap()
                .unwrap(),
            before
        );
        let mut config =
            Config::from_folder(&Environment::Production, &directory.path().join("runtime"))
                .unwrap();
        let mut administrator_config = config.clone();
        administrator_config
            .database
            .uri
            .clone_from(&ctx.config.database.uri);
        if let Some(QueueConfig::Postgres(queue)) = administrator_config.queue.as_mut() {
            queue.uri.clone_from(&ctx.config.database.uri);
        }
        let administrator_boot = med_tracker::app::App::boot(
            StartMode::ServerOnly,
            &Environment::Production,
            administrator_config,
        )
        .await;
        let rejected = administrator_boot.is_err();
        if let Ok(boot) = administrator_boot {
            if let Some(queue) = &boot.app_context.queue_provider {
                queue.shutdown().unwrap();
            }
            boot.app_context.db.close().await.unwrap();
        }
        assert!(
            rejected,
            "Normal production startup must reject bootstrap credentials"
        );
        config.database.uri.clone_from(&runtime_uri);
        if let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() {
            queue.uri.clone_from(&runtime_uri);
        }
        let boot =
            med_tracker::app::App::boot(StartMode::ServerOnly, &Environment::Production, config)
                .await
                .unwrap();
        if let Some(queue) = &boot.app_context.queue_provider {
            queue.shutdown().unwrap();
        }
        boot.app_context.db.close().await.unwrap();
        runtime.close().await.unwrap();
        update.close().await.unwrap();
        if let Some(queue) = &ctx.queue_provider {
            queue.shutdown().unwrap();
        }
        // SeaQuery has no DROP ROLE statement; only these test-created logins
        // are removed, after UUID database ownership was checked by schema_case.
        for uri in [runtime_uri, update_uri] {
            let username = url::Url::parse(&uri).unwrap().username().to_owned();
            assert!(
                username
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            );
            ctx.db
                .execute_unprepared(&format!("DROP ROLE {username}"))
                .await
                .unwrap();
        }
        ctx.db.close().await.unwrap();
    })
    .await;
}

#[tokio::test]
async fn database_setup_generates_credentials_and_supports_restricted_boot_and_updates() {
    setup_access_case(false).await;
}

#[tokio::test]
async fn database_setup_uses_supplied_credentials_and_preserves_data_on_repeat() {
    setup_access_case(true).await;
}

#[tokio::test]
async fn database_setup_rejects_an_incompatible_existing_login_before_installing_schema() {
    schema_case(async {
        use loco_rs::{
            app::Hooks,
            task::{Tasks, Vars},
        };
        let directory = tempfile::tempdir().unwrap();
        let mut runtime =
            url::Url::parse(&std::env::var("MEDTRACKER_TEST_SETUP_DATABASE_URL").unwrap()).unwrap();
        runtime.set_username("pg_read_all_data").unwrap();
        runtime.set_password(Some("password")).unwrap();
        let ctx = setup_context(directory.path(), Some(runtime.as_str()), None).await;
        let mut tasks = Tasks::default();
        med_tracker::app::App::register_tasks(&mut tasks);
        assert!(
            tasks
                .run(&ctx, "database-setup", &Vars::default())
                .await
                .is_err()
        );
        assert!(
            !migration::SchemaManager::new(&ctx.db)
                .has_table("accounts")
                .await
                .unwrap()
        );
        if let Some(queue) = &ctx.queue_provider {
            queue.shutdown().unwrap();
        }
        ctx.db.close().await.unwrap();
    })
    .await;
}
