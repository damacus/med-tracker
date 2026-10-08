use super::fixture::Fixture;
use chrono::NaiveDate;
use med_tracker::models::identity::{
    better_auth::ClinicalStore,
    signup::{AccountProfile, create_passwordless},
};
use sea_orm::{ConnectionTrait, DbBackend, Statement};

fn profile(email: &str) -> AccountProfile {
    AccountProfile {
        email: email.into(),
        name: "New household owner".into(),
        date_of_birth: NaiveDate::from_ymd_opt(1990, 1, 1).unwrap(),
        invitation_token: None,
    }
}

fn config() -> better_auth::AuthConfig {
    better_auth::AuthConfig {
        base_url: "https://medtracker.example.test/api/auth".into(),
        base_path: "/api/auth".into(),
        secret: "synthetic-signup-verification-secret-32-bytes".into(),
        ..Default::default()
    }
}

async fn open_registration(fixture: &Fixture) {
    fixture
        .admin
        .execute_unprepared("DELETE FROM app_settings; INSERT INTO app_settings(invite_only,created_at,updated_at) VALUES(false,now(),now())")
        .await
        .unwrap();
}

#[tokio::test]
async fn passwordless_signup_saves_canonical_account_and_queued_verification_together() {
    let fixture = Fixture::new().await;
    open_registration(&fixture).await;
    let store = ClinicalStore::new(fixture.runtime.clone());
    create_passwordless(
        &fixture.runtime,
        &profile("passwordless-owner@example.test"),
        &store,
        &config(),
        "passwordless-signup-success",
    )
    .await
    .unwrap();
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT a.status, a.password_hash IS NULL AS no_password, u.password_digest IS NULL AS no_digest, p.person_type, p.has_capacity, m.role, g.access_level, (SELECT count(*) FROM identity_verifications v WHERE v.identifier='passkey-enrolment:'||a.id::text AND v.expires_at>CURRENT_TIMESTAMP) AS challenges, (SELECT count(*) FROM pg_loco_queue q WHERE q.task_data->>'to'=a.email AND q.task_data->>'text' LIKE '%/api/auth/verify-email?%') AS messages, (SELECT count(*) FROM identity_sessions s WHERE s.account_id=a.id) AS sessions FROM accounts a JOIN people p ON p.account_id=a.id JOIN users u ON u.person_id=p.id JOIN household_memberships m ON m.account_id=a.id AND m.person_id=p.id JOIN person_access_grants g ON g.household_membership_id=m.id AND g.person_id=p.id WHERE a.email='passwordless-owner@example.test'"
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i32>("", "status").unwrap(), 1);
    assert!(row.try_get::<bool>("", "no_password").unwrap());
    assert!(row.try_get::<bool>("", "no_digest").unwrap());
    assert_eq!(row.try_get::<i32>("", "person_type").unwrap(), 0);
    assert!(row.try_get::<bool>("", "has_capacity").unwrap());
    assert_eq!(row.try_get::<String>("", "role").unwrap(), "owner");
    assert_eq!(row.try_get::<String>("", "access_level").unwrap(), "manage");
    assert_eq!(row.try_get::<i64>("", "challenges").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "messages").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "sessions").unwrap(), 0);
    fixture.close().await;
}

#[tokio::test]
async fn passwordless_signup_queue_failure_rolls_back_account_household_and_verification() {
    let fixture = Fixture::new().await;
    open_registration(&fixture).await;
    fixture.admin.execute_unprepared("CREATE SEQUENCE passwordless_queue_attempt; CREATE FUNCTION reject_passwordless_queue() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('passwordless_queue_attempt'); RAISE EXCEPTION 'synthetic mail queue failure'; END $$; CREATE TRIGGER reject_passwordless_queue BEFORE INSERT ON pg_loco_queue FOR EACH ROW EXECUTE FUNCTION reject_passwordless_queue()")
        .await.unwrap();
    let store = ClinicalStore::new(fixture.runtime.clone());
    assert!(create_passwordless(
        &fixture.runtime,
        &profile("rolled-back-owner@example.test"),
        &store,
        &config(),
        "passwordless-signup-rollback",
    ).await.is_err());
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT is_called FROM passwordless_queue_attempt) AS reached_queue, (SELECT count(*) FROM accounts WHERE email='rolled-back-owner@example.test') AS accounts, (SELECT count(*) FROM people WHERE email='rolled-back-owner@example.test') AS people, (SELECT count(*) FROM households WHERE name='New household owner Household') AS households, (SELECT count(*) FROM identity_verifications) AS verifications, (SELECT count(*) FROM versions WHERE request_id='passwordless-signup-rollback') AS versions, (SELECT count(*) FROM security_audit_events WHERE request_id='passwordless-signup-rollback') AS audits, (SELECT count(*) FROM api_change_events WHERE request_id='passwordless-signup-rollback') AS changes"
    )).await.unwrap().unwrap();
    assert!(row.try_get::<bool>("", "reached_queue").unwrap());
    for field in ["accounts", "people", "households", "verifications", "versions", "audits", "changes"] {
        assert_eq!(row.try_get::<i64>("", field).unwrap(), 0, "{field}");
    }
    fixture.close().await;
}
