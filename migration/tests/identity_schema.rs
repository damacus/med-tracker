use migration::sea_orm;
use migration::{Migrator, MigratorTrait};
#[path = "identity_schema/audit_isolation.rs"]
mod audit_isolation;
#[path = "identity_schema/account_clinical_access.rs"]
mod account_clinical_access;
#[path = "identity_schema/care_provenance.rs"]
mod care_provenance;
#[path = "../../src/models/_entities/mod.rs"]
pub mod entities;
#[path = "../examples/generate_entities/relations.rs"]
mod generated_relations;
#[path = "identity_schema/report_denials.rs"]
mod report_denials;
#[path = "identity_schema/review_regressions.rs"]
mod review_regressions;
#[path = "identity_schema/runtime_provisioning.rs"]
mod runtime_provisioning;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};

#[tokio::test]
async fn generated_composite_relations_match_ordered_postgres_metadata_idempotently() {
    let fixture = Fixture::new().await;
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/models/_entities");
    let output = std::env::temp_dir().join(format!("identity-entities-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&output).expect("owned generated scratch directory");
    let relations = generated_relations::ordered_relations(&fixture.database)
        .await
        .expect("ordered foreign keys");
    assert!(!relations.is_empty());
    for (table, _, _, _) in &relations {
        let name = format!("{table}.rs");
        std::fs::copy(source.join(&name), output.join(name)).expect("copy generated mapping");
    }
    generated_relations::reconcile(&fixture.database, &output)
        .await
        .expect("reconcile metadata");
    generated_relations::reconcile(&fixture.database, &output)
        .await
        .expect("repeat reconciliation");
    let mismatches = relations
        .iter()
        .filter_map(|(table, _, _, _)| {
            let name = format!("{table}.rs");
            let original = std::fs::read(source.join(&name)).expect("original mapping");
            let reconciled = std::fs::read(output.join(&name)).expect("reconciled mapping");
            (original != reconciled).then_some(table.clone())
        })
        .collect::<Vec<_>>();
    std::fs::remove_dir_all(output).expect("remove owned scratch directory");
    fixture.close().await;
    assert!(
        mismatches.is_empty(),
        "Mappings differ from ordered PostgreSQL metadata: {mismatches:?}"
    );
}

#[tokio::test]
async fn concurrent_minor_edit_cannot_race_account_creation() {
    use sea_orm::TransactionTrait;
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared(
        "INSERT INTO public.people(id,clinical_name,created_at,updated_at) VALUES (90701,'Adult',now(),now())"
    ).await.expect("adult identity");
    let transaction = fixture.database.begin().await.expect("account transaction");
    transaction.execute_unprepared(
        "INSERT INTO public.accounts(person_id,name,email,status,created_at,updated_at) VALUES (90701,'Adult profile','race@example.test',2,now(),now())"
    ).await.expect("account insertion locks person");
    let mut options = sea_orm::ConnectOptions::new(fixture.url.as_str());
    options.min_connections(1).max_connections(1);
    let competing = Database::connect(options).await.expect("competing writer");
    competing
        .execute_unprepared("SET lock_timeout='100ms'")
        .await
        .expect("bounded lock wait");
    let blocked = competing
        .execute_unprepared(
            "UPDATE public.people SET person_type=1,has_capacity=false WHERE id=90701",
        )
        .await;
    assert_database_code(
        blocked.expect_err("Concurrent clinical identity update must wait for account link"),
        "55P03",
    );
    transaction.commit().await.expect("commit account");
    let rejected = competing
        .execute_unprepared(
            "UPDATE public.people SET person_type=1,has_capacity=false WHERE id=90701",
        )
        .await;
    assert_database_code(
        rejected.expect_err("Committed account must reject minor mutation"),
        "23514",
    );
    let row = competing
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT person_type,has_capacity FROM public.people WHERE id=90701",
        ))
        .await
        .expect("unchanged clinical identity")
        .expect("person");
    assert_eq!(row.try_get::<i32>("", "person_type").expect("type"), 0);
    assert!(row.try_get::<bool>("", "has_capacity").expect("capacity"));
    competing.close().await.expect("close competing writer");
    fixture.close().await;
}

fn assert_database_code(error: sea_orm::DbErr, expected: &str) {
    let (sea_orm::DbErr::Exec(sea_orm::RuntimeErr::SqlxError(error))
    | sea_orm::DbErr::Query(sea_orm::RuntimeErr::SqlxError(error))) = error
    else {
        panic!("Expected PostgreSQL execution error, received {error:?}");
    };
    assert_eq!(
        error
            .as_database_error()
            .and_then(|error| error.code())
            .as_deref(),
        Some(expected)
    );
}

#[tokio::test]
async fn entity_relations_resolve_the_actual_household_person_pair() {
    use sea_orm::{EntityTrait, ModelTrait};
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared(
        "INSERT INTO public.people(id,clinical_name,created_at,updated_at) VALUES (90801,'Shared person',now(),now());
         INSERT INTO public.households(id,name,slug,timezone,created_at,updated_at)
         VALUES (90802,'First','first','UTC',now(),now()), (90803,'Second','second','UTC',now(),now());
         INSERT INTO public.household_memberships(id,person_id,household_id,joined_at,created_at,updated_at)
         VALUES (90804,90801,90802,now(),now(),now()), (90805,90801,90803,now(),now(),now());
         INSERT INTO public.health_events(id,person_id,household_id,title,event_kind,started_on,created_at,updated_at)
         VALUES (90806,90801,90803,'Second household event',0,CURRENT_DATE,now(),now())"
    ).await.expect("two household clinical event");
    let event = entities::health_events::Entity::find_by_id(90806)
        .one(&fixture.database)
        .await
        .expect("event query")
        .expect("event");
    let member = event
        .find_related(entities::household_memberships::Entity)
        .one(&fixture.database)
        .await
        .expect("mapped participation query")
        .expect("participation");
    let person = member
        .find_related(entities::people::Entity)
        .one(&fixture.database)
        .await
        .expect("mapped global person query")
        .expect("person");
    fixture.close().await;
    assert_eq!(member.id, 90805);
    assert_eq!(person.id, 90801);
}

struct Fixture {
    admin: DatabaseConnection,
    database: DatabaseConnection,
    name: String,
    url: url::Url,
}

impl Fixture {
    async fn runtime(&self) -> (DatabaseConnection, String) {
        let role = format!("identity_runtime_{}", uuid::Uuid::new_v4().simple());
        self.admin.execute_unprepared(&format!(
            "CREATE ROLE {role} LOGIN PASSWORD 'test-password' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS IN ROLE med_tracker_app"
        )).await.expect("restricted runtime role");
        let mut url = self.url.clone();
        url.set_username(&role).expect("runtime username");
        url.set_password(Some("test-password"))
            .expect("runtime password");
        let mut options = sea_orm::ConnectOptions::new(url.as_str());
        options.min_connections(1).max_connections(1);
        (
            Database::connect(options)
                .await
                .expect("runtime connection"),
            role,
        )
    }

    async fn close_runtime(&self, runtime: DatabaseConnection, role: &str) {
        runtime.close().await.expect("close runtime");
        self.admin
            .execute_unprepared(&format!("DROP ROLE {role}"))
            .await
            .expect("drop runtime role");
    }
    async fn empty() -> Self {
        assert_eq!(
            std::env::var("MEDTRACKER_OWNED_DATABASE").as_deref(),
            Ok("1")
        );
        let uri = std::env::var("DATABASE_URL").expect("owned PostgreSQL database");
        let admin = Database::connect(&uri)
            .await
            .expect("database administrator");
        let name = format!("identity_schema_{}", uuid::Uuid::new_v4().simple());
        admin
            .execute_unprepared(&format!("CREATE DATABASE {name} TEMPLATE template0"))
            .await
            .expect("fresh owned database");
        let mut url = url::Url::parse(&uri).expect("PostgreSQL URL");
        url.set_path(&name);
        let database = Database::connect(url.as_str())
            .await
            .expect("fresh connection");
        Self {
            admin,
            database,
            name,
            url,
        }
    }

    async fn new() -> Self {
        let fixture = Self::empty().await;
        Migrator::up(&fixture.database, None)
            .await
            .expect("fresh migrations");
        fixture
    }

    async fn close(self) {
        self.database.close().await.expect("close test connection");
        self.admin
            .execute_unprepared(&format!("DROP DATABASE {}", self.name))
            .await
            .expect("remove owned database");
        self.admin.close().await.expect("close administrator");
    }
}

#[tokio::test]
async fn fresh_schema_repeats_without_data_loss_and_rejects_recognised_old_ledger() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared(
        "INSERT INTO public.people(id, clinical_name, created_at, updated_at) VALUES (90401, 'Retained', now(), now())"
    ).await.expect("fresh data");
    Migrator::up(&fixture.database, None)
        .await
        .expect("repeat migration");
    let row = fixture
        .database
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT clinical_name FROM public.people WHERE id=90401",
        ))
        .await
        .expect("retained data")
        .expect("retained person");
    assert_eq!(
        row.try_get::<String>("", "clinical_name").expect("name"),
        "Retained"
    );
    fixture.close().await;

    let old = Fixture::empty().await;
    old.database.execute_unprepared(
        "CREATE TABLE public.seaql_migrations(version varchar PRIMARY KEY, applied_at bigint NOT NULL);
         INSERT INTO public.seaql_migrations VALUES ('m20261013_000001_create_medtracker', 1);
         CREATE TABLE public.accounts(id bigint PRIMARY KEY, email text NOT NULL);
         INSERT INTO public.accounts VALUES (1, 'retain@example.test')"
    ).await.expect("populated recognised old schema");
    let rejected = Migrator::up(&old.database, None).await;
    let row = old.database.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT email, (SELECT count(*) FROM public.seaql_migrations) AS migrations FROM public.accounts WHERE id=1"
    )).await.expect("old data intact").expect("old account");
    old.close().await;
    assert!(
        rejected.is_err(),
        "Old populated ledgers must not be adopted"
    );
    assert_eq!(
        row.try_get::<String>("", "email").expect("old email"),
        "retain@example.test"
    );
    assert_eq!(
        row.try_get::<i64>("", "migrations").expect("ledger count"),
        1
    );
}

#[tokio::test]
async fn bootstrap_only_creates_new_pending_adult_identity_and_rolls_back_failure() {
    let fixture = Fixture::new().await;
    let (runtime, role) = fixture.runtime().await;
    for sql in [
        "INSERT INTO public.people(clinical_name,created_at,updated_at) VALUES ('Bypass',now(),now())",
        "INSERT INTO public.accounts(person_id,name,email,status,created_at,updated_at) VALUES (1,'Bypass','bypass@example.test',2,now(),now())",
        "SELECT * FROM med_tracker.bootstrap_account('minor@example.test','Child',(CURRENT_DATE - interval '10 years')::date,NULL)",
        "SELECT * FROM med_tracker.bootstrap_account('blank@example.test','  ',DATE '1990-01-01',NULL)",
    ] {
        assert!(
            runtime.execute_unprepared(sql).await.is_err(),
            "Rejected bootstrap boundary: {sql}"
        );
    }
    let created = runtime.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT * FROM med_tracker.bootstrap_account('new@example.test','New adult',DATE '1990-01-01',NULL)"
    )).await.expect("restricted bootstrap").expect("created identity");
    let account_id: i64 = created.try_get("", "account_id").expect("account id");
    let person_id: i64 = created.try_get("", "person_id").expect("person id");
    let duplicate = runtime.execute_unprepared(
        "SELECT * FROM med_tracker.bootstrap_account('new@example.test','Duplicate',DATE '1990-01-01',NULL)"
    ).await;
    let row = fixture
        .database
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM public.people) AS people,
                (SELECT count(*) FROM public.accounts WHERE status=1) AS pending,
                (SELECT count(*) FROM public.household_memberships) AS memberships",
        ))
        .await
        .expect("atomic identity counts")
        .expect("counts");
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    assert!(account_id > 0 && person_id > 0);
    assert!(duplicate.is_err());
    assert_eq!(row.try_get::<i64>("", "people").expect("people"), 1);
    assert_eq!(row.try_get::<i64>("", "pending").expect("pending"), 1);
    assert_eq!(
        row.try_get::<i64>("", "memberships").expect("memberships"),
        0
    );
}

#[tokio::test]
async fn dependent_adult_can_have_an_account_without_capacity() {
    let fixture = Fixture::new().await;
    let result = fixture.database.execute_unprepared(
        "INSERT INTO public.people(id,clinical_name,person_type,has_capacity,created_at,updated_at)
         VALUES (90501,'Dependent adult',2,false,now(),now());
         INSERT INTO public.accounts(person_id,name,email,status,created_at,updated_at)
         VALUES (90501,'Adult profile','dependent@example.test',2,now(),now())"
    ).await;
    fixture.close().await;
    assert!(
        result.is_ok(),
        "Only minors are prohibited accounts: {result:?}"
    );
}

#[tokio::test]
async fn clinical_names_survive_account_closure_without_disclosing_private_profile() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared(
        "INSERT INTO public.people(id,clinical_name,created_at,updated_at)
         VALUES (90601,NULL,now(),now()), (90602,NULL,now(),now());
         INSERT INTO public.accounts(id,person_id,name,email,status,preferences,created_at,updated_at)
         VALUES (90603,90601,'Current adult','current@example.test',2,'{}',now(),now()),
                (90604,90602,'Clinical display','private@example.test',2,'{\"private\":true}',now(),now());
         INSERT INTO public.households(id,name,slug,timezone,created_at,updated_at)
         VALUES (90605,'Shared home','shared-home','Europe/London',now(),now());
         INSERT INTO public.household_memberships(id,person_id,household_id,role,joined_at,created_at,updated_at)
         VALUES (90606,90601,90605,'member',now(),now(),now()),
                (90607,90602,90605,'member',now(),now(),now())"
    ).await.expect("shared household");
    let (runtime, role) = fixture.runtime().await;
    runtime.execute_unprepared(
        "SET med_tracker.current_account_id='90603'; SET med_tracker.current_household_id='90605'; SET med_tracker.current_membership_id='90606'"
    ).await.expect("current household");
    let private = runtime
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT email,preferences FROM public.accounts WHERE id=90604",
        ))
        .await
        .expect("private account query");
    assert!(
        private.is_empty(),
        "Clinical visibility must not disclose private account fields"
    );
    for change in [
        "SELECT 1",
        "UPDATE public.accounts SET status=3,name=NULL,email='erased@example.test',preferences='{}' WHERE id=90604",
        "DELETE FROM public.accounts WHERE id=90604",
    ] {
        fixture
            .database
            .execute_unprepared(change)
            .await
            .expect("account lifecycle");
        let row = runtime
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT med_tracker.person_display_name(90602) AS name",
            ))
            .await
            .expect("scoped clinical display")
            .expect("display name");
        assert_eq!(
            row.try_get::<String>("", "name").expect("name"),
            "Clinical display"
        );
    }
    let unnamed = fixture
        .database
        .execute_unprepared("UPDATE public.people SET clinical_name=NULL WHERE id=90602")
        .await;
    fixture.close_runtime(runtime, &role).await;
    fixture.close().await;
    assert!(
        unnamed.is_err(),
        "Accountless clinical identity must remain nameable"
    );
}

#[tokio::test]
async fn restricted_role_cannot_read_or_write_foreign_people_by_selecting_their_household() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared(
        "INSERT INTO public.people(id, clinical_name, person_type, has_capacity, created_at, updated_at)
         VALUES (90301, 'Current adult', 0, true, now(), now()),
                (90302, 'Foreign adult', 0, true, now(), now());
         INSERT INTO public.accounts(id, person_id, name, email, status, created_at, updated_at)
         VALUES (90303, 90301, 'Account profile', 'current@example.test', 2, now(), now()),
                (90304, 90302, 'Account profile', 'foreign@example.test', 2, now(), now());
         INSERT INTO public.households(id, name, slug, timezone, created_at, updated_at)
         VALUES (90305, 'Current home', 'current-home', 'Europe/London', now(), now()),
                (90306, 'Foreign home', 'foreign-home', 'Europe/London', now(), now());
         INSERT INTO public.household_memberships(id, household_id, person_id, role, joined_at, created_at, updated_at)
         VALUES (90307, 90305, 90301, 'member', now(), now(), now()),
                (90308, 90306, 90302, 'member', now(), now(), now());",
    ).await.expect("separate household identities");
    let role = format!("identity_runtime_{}", uuid::Uuid::new_v4().simple());
    fixture.admin.execute_unprepared(&format!(
        "CREATE ROLE {role} LOGIN PASSWORD 'test-password' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS IN ROLE med_tracker_app"
    )).await.expect("restricted runtime role");
    let mut runtime_url = fixture.url.clone();
    runtime_url.set_username(&role).expect("runtime username");
    runtime_url
        .set_password(Some("test-password"))
        .expect("runtime password");
    let mut options = sea_orm::ConnectOptions::new(runtime_url.as_str());
    options.min_connections(1).max_connections(1);
    let runtime = Database::connect(options)
        .await
        .expect("restricted runtime connection");
    runtime
        .execute_unprepared(
            "SET med_tracker.current_account_id = '90303';
         SET med_tracker.current_household_id = '90305';
         SET med_tracker.current_membership_id = '90307'",
        )
        .await
        .expect("own household context");
    let own = runtime
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id FROM public.people ORDER BY id",
        ))
        .await
        .expect("own clinical visibility");
    let own_ids: Vec<i64> = own
        .iter()
        .map(|row| row.try_get("", "id").expect("person id"))
        .collect();
    runtime
        .execute_unprepared(
            "SET med_tracker.current_household_id = '90306';
         SET med_tracker.current_membership_id = '90308'",
        )
        .await
        .expect("forged foreign selection");
    let foreign = runtime
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id FROM public.people WHERE id = 90302",
        ))
        .await
        .expect("foreign visibility query");
    let update = runtime
        .execute_unprepared(
            "UPDATE public.people SET clinical_name = 'Unauthorised change' WHERE id = 90302",
        )
        .await;
    let unchanged = fixture
        .database
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT clinical_name AS name FROM public.people WHERE id = 90302",
        ))
        .await
        .expect("persisted foreign identity")
        .expect("foreign person");
    let name: String = unchanged.try_get("", "name").expect("foreign name");
    runtime.close().await.expect("close runtime connection");
    fixture
        .admin
        .execute_unprepared(&format!("DROP ROLE {role}"))
        .await
        .expect("remove runtime role");
    fixture.close().await;
    assert_eq!(own_ids, vec![90301]);
    assert!(
        foreign.is_empty(),
        "Selection does not grant foreign membership"
    );
    assert!(update.is_err() || update.is_ok_and(|result| result.rows_affected() == 0));
    assert_eq!(name, "Foreign adult");
}

#[tokio::test]
async fn account_owns_one_required_person_without_users_table() {
    let fixture = Fixture::new().await;
    let row = fixture
        .database
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT to_regclass('public.users') IS NULL AS no_users,
          EXISTS (SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'public' AND table_name = 'accounts'
              AND column_name = 'person_id' AND is_nullable = 'NO') AS required_person,
          NOT EXISTS (SELECT 1 FROM information_schema.columns
            WHERE table_schema = 'public' AND table_name = 'people'
              AND column_name IN ('account_id', 'household_id')) AS global_person",
        ))
        .await
        .expect("identity schema inspection")
        .expect("schema row");
    let no_users: bool = row.try_get("", "no_users").expect("users absence");
    let required_person: bool = row.try_get("", "required_person").expect("required person");
    let global_person: bool = row.try_get("", "global_person").expect("global person");
    fixture.close().await;
    assert!(
        no_users,
        "Accounts must be the only authentication/profile entity"
    );
    assert!(required_person, "Every account must reference its person");
    assert!(
        global_person,
        "People must not carry one household or account FK"
    );
}

#[tokio::test]
async fn accountless_minor_participates_in_two_households_with_one_identity() {
    let fixture = Fixture::new().await;
    let result = fixture.database.execute_unprepared(
        "INSERT INTO public.people(id, clinical_name, person_type, has_capacity, created_at, updated_at)
         VALUES (90001, 'Accountless child', 1, false, now(), now());
         INSERT INTO public.households(id, name, slug, timezone, created_at, updated_at)
         VALUES (90002, 'First home', 'first-home', 'Europe/London', now(), now()),
                (90003, 'Second home', 'second-home', 'Europe/London', now(), now());
         INSERT INTO public.household_memberships(id, household_id, person_id, role, joined_at, created_at, updated_at)
         VALUES (90004, 90002, 90001, 'member', now(), now(), now()),
                (90005, 90003, 90001, 'member', now(), now(), now());",
    ).await;
    let counts = fixture.database.query_one_raw(Statement::from_string(
        DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM public.people WHERE id = 90001) AS people,
                (SELECT count(*) FROM public.household_memberships WHERE person_id = 90001) AS memberships,
                (SELECT count(*) FROM public.accounts) AS accounts",
    )).await.expect("participation counts").expect("counts row");
    let people: i64 = counts.try_get("", "people").expect("person count");
    let memberships: i64 = counts.try_get("", "memberships").expect("membership count");
    let accounts: i64 = counts.try_get("", "accounts").expect("account count");
    fixture.close().await;
    assert!(
        result.is_ok(),
        "Accountless multi-household participation: {result:?}"
    );
    assert_eq!((people, memberships, accounts), (1, 2, 0));
}

#[tokio::test]
async fn person_cannot_have_two_accounts_or_become_minor_while_linked() {
    let fixture = Fixture::new().await;
    fixture
        .database
        .execute_unprepared(
            "INSERT INTO public.people(id, clinical_name, person_type, has_capacity, created_at, updated_at)
         VALUES (90101, 'Adult identity', 0, true, now(), now());
         INSERT INTO public.accounts(id, person_id, name, email, status, created_at, updated_at)
         VALUES (90102, 90101, 'Account profile', 'first@example.test', 2, now(), now());",
        )
        .await
        .expect("one adult account with global identity");
    let duplicate = fixture
        .database
        .execute_unprepared(
            "INSERT INTO public.accounts(id, person_id, name, email, status, created_at, updated_at)
         VALUES (90103, 90101, 'Account profile', 'second@example.test', 2, now(), now())",
        )
        .await;
    let minor_edit = fixture
        .database
        .execute_unprepared(
            "UPDATE public.people SET person_type = 1, has_capacity = false WHERE id = 90101",
        )
        .await;
    let missing_person = fixture
        .database
        .execute_unprepared(
            "INSERT INTO public.accounts(id, person_id, name, email, status, created_at, updated_at)
         VALUES (90104, 90999, 'Account profile', 'missing@example.test', 2, now(), now())",
        )
        .await;
    fixture.close().await;
    assert!(
        duplicate.is_err(),
        "A second account cannot claim the same person"
    );
    assert!(
        minor_edit.is_err(),
        "Editing clinical identity cannot bypass the minor login ban"
    );
    assert!(
        missing_person.is_err(),
        "An account must reference a real person"
    );
}

#[tokio::test]
async fn minor_account_link_and_dependent_capacity_are_database_rejected() {
    let fixture = Fixture::new().await;
    fixture
        .database
        .execute_unprepared(
            "INSERT INTO public.people(id, clinical_name, person_type, has_capacity, created_at, updated_at)
         VALUES (90201, 'Child identity', 1, false, now(), now()),
                (90202, 'Dependent adult', 2, false, now(), now());",
        )
        .await
        .expect("accountless dependent identities");
    let minor_account = fixture
        .database
        .execute_unprepared(
            "INSERT INTO public.accounts(id, person_id, name, email, status, created_at, updated_at)
         VALUES (90203, 90201, 'Account profile', 'child@example.test', 2, now(), now())",
        )
        .await;
    let minor_capacity = fixture
        .database
        .execute_unprepared("UPDATE public.people SET has_capacity = true WHERE id = 90201")
        .await;
    let dependent_capacity = fixture
        .database
        .execute_unprepared("UPDATE public.people SET has_capacity = true WHERE id = 90202")
        .await;
    fixture.close().await;
    assert!(minor_account.is_err(), "Minors cannot have login accounts");
    assert!(minor_capacity.is_err(), "Minors cannot have capacity");
    assert!(
        dependent_capacity.is_err(),
        "Dependent adults cannot have capacity"
    );
}
