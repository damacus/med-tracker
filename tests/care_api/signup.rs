use super::*;
use med_tracker::models::_entities::{accounts, app_settings, people, person_access_grants};
use sea_orm::sea_query::{Expr, ExprTrait, Query};

pub(super) async fn open_registration(app: &Application) -> Vec<app_settings::Model> {
    let saved = app_settings::Entity::find()
        .all(&app.fixture.admin)
        .await
        .unwrap();
    if saved.is_empty() {
        let now = chrono::Utc::now().naive_utc();
        app_settings::ActiveModel {
            invite_only: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&app.fixture.admin)
        .await
        .unwrap();
    } else {
        for row in &saved {
            let mut row = row.clone().into_active_model();
            row.invite_only = Set(false);
            row.update(&app.fixture.admin).await.unwrap();
        }
    }
    saved
}

pub(super) async fn restore_registration(app: &Application, saved: Vec<app_settings::Model>) {
    app_settings::Entity::delete_many()
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    for row in saved {
        row.into_active_model()
            .insert(&app.fixture.admin)
            .await
            .unwrap();
    }
}

async fn credential_count(app: &Application, account: i64) -> i64 {
    let query = Query::select()
        .expr_as(Expr::col("id").count(), "count")
        .from("auth_credentials")
        .and_where(Expr::col("user_id").eq(account))
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

async fn session_count(app: &Application, account: i64) -> i64 {
    let query = Query::select()
        .expr_as(Expr::col("id").count(), "count")
        .from("identity_sessions")
        .and_where(Expr::col("account_id").eq(account))
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

pub(super) async fn legacy_column_count(app: &Application, table: &str, column: &str) -> i64 {
    let query = Query::select()
        .expr_as(Expr::col("column_name").count(), "count")
        .from(("information_schema", "columns"))
        .and_where(Expr::col("table_schema").eq("public"))
        .and_where(Expr::col("table_name").eq(table))
        .and_where(Expr::col("column_name").eq(column))
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

async fn signup_counts(app: &Application, email: &str) -> (u64, usize) {
    let accounts = accounts::Entity::find()
        .filter(accounts::Column::Email.eq(email))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let messages = super::native_password_reset::queued_mail(app, email, "Verify your account")
        .await
        .len();
    (accounts, messages)
}

async fn rollback_state(app: &Application) -> Value {
    let mut state = serde_json::Map::new();
    for table in [
        "accounts",
        "people",
        "households",
        "household_memberships",
        "auth_credentials",
        "identity_verifications",
        "versions",
        "security_audit_events",
        "api_change_events",
        "pg_loco_queue",
    ] {
        let query = Query::select()
            .expr_as(Expr::col("id").count(), "count")
            .from(table)
            .to_owned();
        let count: i64 = app
            .fixture
            .admin
            .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "count")
            .unwrap();
        state.insert(table.into(), json!(count));
    }
    Value::Object(state)
}

async fn signup(app: &Application, email: &str, credential: &str) -> reqwest::Response {
    let mut input = json!({"name":"New household owner","email":email,"date_of_birth":"1990-01-01","credential":credential});
    if credential == "password" {
        input["password"] = json!("synthetic-signup-1!");
    }
    app.client
        .post(format!("{}/api/auth/onboarding/signup", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&input)
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn signup_create_saves_canonical_account_and_queued_verification_together() {
    let app = Application::new().await;
    let registration = open_registration(&app).await;
    for (email, credential) in [
        ("signup-owner@example.test", "password"),
        ("signup-passkey@example.test", "passkey"),
    ] {
        let email = format!("{}-{email}", uuid::Uuid::new_v4());
        let email = email.as_str();
        let response = signup(&app, email, credential).await;
        let status = response.status();
        let body = response.text().await.unwrap();
        assert_eq!(status.as_u16(), 200, "{body}");
        let account = accounts::Entity::find()
            .filter(accounts::Column::Email.eq(email))
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        let person = people::Entity::find_by_id(account.person_id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        let membership = household_memberships::Entity::find()
            .filter(household_memberships::Column::PersonId.eq(person.id))
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(account.status, 1);
        assert_eq!(
            legacy_column_count(&app, "accounts", "password_hash").await,
            0
        );
        assert_eq!(
            legacy_column_count(&app, "users", "password_digest").await,
            0
        );
        assert_eq!(person.person_type, 0);
        assert!(person.has_capacity);
        assert_eq!(account.person_id, person.id);
        assert_eq!(membership.role, "owner");
        assert_eq!(
            credential_count(&app, account.id).await,
            i64::from(credential == "password")
        );
        let grants = person_access_grants::Entity::find()
            .filter(person_access_grants::Column::HouseholdMembershipId.eq(membership.id))
            .filter(person_access_grants::Column::PersonId.eq(person.id))
            .count(&app.fixture.admin)
            .await
            .unwrap();
        assert_eq!(grants, 0);
        assert_eq!(
            super::native_password_reset::queued_mail(&app, email, "Verify your account")
                .await
                .len(),
            1
        );
        assert_eq!(session_count(&app, account.id).await, 0);
        if credential == "password" {
            let duplicate = signup(&app, email, credential).await;
            let status = duplicate.status();
            let body = duplicate.text().await.unwrap();
            assert_eq!(status.as_u16(), 200, "{body}");
            assert_eq!(signup_counts(&app, email).await, (1, 1));
        }
    }
    restore_registration(&app, registration).await;
    app.close().await;
}

#[tokio::test]
async fn signup_create_queue_failure_rolls_back_account_household_and_verification() {
    let app = Application::new().await;
    let registration = open_registration(&app).await;
    let before = rollback_state(&app).await;
    let fault = format!("signup_queue_{}", uuid::Uuid::new_v4().simple());
    let marker = format!("{fault}_attempt");
    // SeaORM has no trigger, PL/pgSQL function or GRANT builder. This schema
    // fault and the global registration policy require whole-run exclusivity.
    app.fixture.admin.execute_unprepared(&format!("CREATE SEQUENCE {marker}; GRANT USAGE ON SEQUENCE {marker} TO med_tracker_app; CREATE FUNCTION {fault}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('{marker}'); RAISE EXCEPTION 'synthetic mail queue failure'; END $$; CREATE TRIGGER {fault} BEFORE INSERT ON pg_loco_queue FOR EACH ROW EXECUTE FUNCTION {fault}()")).await.unwrap();
    let mut outcomes = Vec::new();
    let attempts = [
        (
            format!("rolled-back-password-{}@example.test", uuid::Uuid::new_v4()),
            "password",
        ),
        (
            format!("rolled-back-passkey-{}@example.test", uuid::Uuid::new_v4()),
            "passkey",
        ),
    ];
    for (email, credential) in &attempts {
        let response = signup(&app, email, credential).await;
        let status = response.status();
        let body = response.text().await.unwrap();
        outcomes.push((credential, status, body));
        let after = rollback_state(&app).await;
        assert_eq!(before, after);
    }
    let query = Query::select()
        .column("last_value")
        .from(sea_orm::sea_query::Alias::new(marker.clone()))
        .to_owned();
    let reached = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "last_value")
        .unwrap();
    assert_eq!(reached, 2);
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "DROP TRIGGER {fault} ON pg_loco_queue; DROP FUNCTION {fault}(); DROP SEQUENCE {marker}"
        ))
        .await
        .unwrap();
    for (email, credential) in &attempts {
        let retry = signup(&app, email, credential).await;
        let status = retry.status();
        let body = retry.text().await.unwrap();
        assert_eq!(status.as_u16(), 200, "{body}");
        assert_eq!(signup_counts(&app, email).await, (1, 1));
    }
    restore_registration(&app, registration).await;
    app.close().await;
    assert!(
        outcomes
            .iter()
            .all(|(_, status, _)| status.is_server_error()),
        "queue failure outcomes: {outcomes:?}"
    );
}
