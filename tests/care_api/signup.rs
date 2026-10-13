use super::fixture::Fixture;
use med_tracker::models::identity::signup::{self, SignupInput};
use sea_orm::{ConnectionTrait, DbBackend, Statement};

fn input(email: &str) -> SignupInput {
    SignupInput {
        invitation_token: String::new(),
        email: email.into(),
        name: "New household owner".into(),
        date_of_birth: "1990-01-01".into(),
        password: "synthetic-signup-1!".into(),
        password_confirm: "synthetic-signup-1!".into(),
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
async fn signup_create_saves_canonical_account_and_queued_verification_together() {
    let fixture = Fixture::new().await;
    open_registration(&fixture).await;
    signup::create(
        &fixture.runtime,
        &input("signup-owner@example.test"),
        "https://app.example.test",
        Some("signup-canonical"),
        crate::template::test_settings().as_ref(),
    )
    .await
    .unwrap();
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT a.status, a.password_hash IS NOT NULL AS has_password, u.password_digest IS NOT NULL AS has_digest, p.person_type, p.has_capacity, m.role, g.access_level, (SELECT count(*) FROM account_verification_keys k WHERE k.account_id=a.id) AS keys, (SELECT count(*) FROM pg_loco_queue q WHERE q.task_data->>'to'=a.email AND q.task_data->>'text' LIKE '%/verify-account?key=%') AS messages, (SELECT count(*) FROM identity_sessions s WHERE s.account_id=a.id) AS sessions FROM accounts a JOIN people p ON p.account_id=a.id JOIN users u ON u.person_id=p.id JOIN household_memberships m ON m.account_id=a.id AND m.person_id=p.id JOIN person_access_grants g ON g.household_membership_id=m.id AND g.person_id=p.id WHERE a.email='signup-owner@example.test'"
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i32>("", "status").unwrap(), 1);
    assert!(row.try_get::<bool>("", "has_password").unwrap());
    assert!(row.try_get::<bool>("", "has_digest").unwrap());
    assert_eq!(row.try_get::<i32>("", "person_type").unwrap(), 0);
    assert!(row.try_get::<bool>("", "has_capacity").unwrap());
    assert_eq!(row.try_get::<String>("", "role").unwrap(), "owner");
    assert_eq!(row.try_get::<String>("", "access_level").unwrap(), "manage");
    assert_eq!(row.try_get::<i64>("", "keys").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "messages").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "sessions").unwrap(), 0);
    fixture.close().await;
}

#[tokio::test]
async fn signup_create_queue_failure_rolls_back_account_household_and_verification() {
    let fixture = Fixture::new().await;
    open_registration(&fixture).await;
    fixture.admin.execute_unprepared("CREATE SEQUENCE signup_queue_attempt; GRANT USAGE ON SEQUENCE signup_queue_attempt TO med_tracker_app; CREATE FUNCTION reject_signup_queue() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('signup_queue_attempt'); RAISE EXCEPTION 'synthetic mail queue failure'; END $$; CREATE TRIGGER reject_signup_queue BEFORE INSERT ON pg_loco_queue FOR EACH ROW EXECUTE FUNCTION reject_signup_queue()")
        .await.unwrap();
    assert!(
        signup::create(
            &fixture.runtime,
            &input("rolled-back-owner@example.test"),
            "https://app.example.test",
            Some("signup-rollback"),
            crate::template::test_settings().as_ref(),
        )
        .await
        .is_err()
    );
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT is_called FROM signup_queue_attempt) AS reached_queue, (SELECT count(*) FROM accounts WHERE email='rolled-back-owner@example.test') AS accounts, (SELECT count(*) FROM people WHERE email='rolled-back-owner@example.test') AS people, (SELECT count(*) FROM households WHERE name='New household owner Household') AS households, (SELECT count(*) FROM account_verification_keys k JOIN accounts a ON a.id=k.account_id WHERE a.email='rolled-back-owner@example.test') AS verifications, (SELECT count(*) FROM versions WHERE request_id='signup-rollback') AS versions, (SELECT count(*) FROM security_audit_events WHERE request_id='signup-rollback') AS audits, (SELECT count(*) FROM api_change_events WHERE request_id='signup-rollback') AS changes"
    )).await.unwrap().unwrap();
    assert!(row.try_get::<bool>("", "reached_queue").unwrap());
    for field in [
        "accounts",
        "people",
        "households",
        "verifications",
        "versions",
        "audits",
        "changes",
    ] {
        assert_eq!(row.try_get::<i64>("", field).unwrap(), 0, "{field}");
    }
    fixture.close().await;
}
