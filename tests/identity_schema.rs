use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};

struct Fixture {
    admin: DatabaseConnection,
    database: DatabaseConnection,
    name: String,
    url: url::Url,
}

impl Fixture {
    async fn new() -> Self {
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
        Migrator::up(&database, None)
            .await
            .expect("fresh migrations");
        Self {
            admin,
            database,
            name,
            url,
        }
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
async fn restricted_role_cannot_read_or_write_foreign_people_by_selecting_their_household() {
    let fixture = Fixture::new().await;
    fixture.database.execute_unprepared(
        "INSERT INTO public.people(id, name, person_type, has_capacity, created_at, updated_at)
         VALUES (90301, 'Current adult', 0, true, now(), now()),
                (90302, 'Foreign adult', 0, true, now(), now());
         INSERT INTO public.accounts(id, person_id, email, status, created_at, updated_at)
         VALUES (90303, 90301, 'current@example.test', 2, now(), now()),
                (90304, 90302, 'foreign@example.test', 2, now(), now());
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
            "UPDATE public.people SET name = 'Unauthorised change' WHERE id = 90302",
        )
        .await;
    let unchanged = fixture
        .database
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT name FROM public.people WHERE id = 90302",
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
        "INSERT INTO public.people(id, name, person_type, has_capacity, created_at, updated_at)
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
            "INSERT INTO public.people(id, name, person_type, has_capacity, created_at, updated_at)
         VALUES (90101, 'Adult identity', 0, true, now(), now());
         INSERT INTO public.accounts(id, person_id, email, status, created_at, updated_at)
         VALUES (90102, 90101, 'first@example.test', 2, now(), now());",
        )
        .await
        .expect("one adult account with global identity");
    let duplicate = fixture
        .database
        .execute_unprepared(
            "INSERT INTO public.accounts(id, person_id, email, status, created_at, updated_at)
         VALUES (90103, 90101, 'second@example.test', 2, now(), now())",
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
            "INSERT INTO public.accounts(id, person_id, email, status, created_at, updated_at)
         VALUES (90104, 90999, 'missing@example.test', 2, now(), now())",
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
            "INSERT INTO public.people(id, name, person_type, has_capacity, created_at, updated_at)
         VALUES (90201, 'Child identity', 1, false, now(), now()),
                (90202, 'Dependent adult', 2, false, now(), now());",
        )
        .await
        .expect("accountless dependent identities");
    let minor_account = fixture
        .database
        .execute_unprepared(
            "INSERT INTO public.accounts(id, person_id, email, status, created_at, updated_at)
         VALUES (90203, 90201, 'child@example.test', 2, now(), now())",
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
