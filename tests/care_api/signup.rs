use super::*;

async fn open_registration(app: &Application) {
    app.fixture.admin.execute_unprepared("DELETE FROM app_settings; INSERT INTO app_settings(invite_only,created_at,updated_at) VALUES(false,now(),now())").await.unwrap();
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
    open_registration(&app).await;
    for (email, credential) in [
        ("signup-owner@example.test", "password"),
        ("signup-passkey@example.test", "passkey"),
    ] {
        let response = signup(&app, email, credential).await;
        let status = response.status();
        let body = response.text().await.unwrap();
        assert_eq!(status.as_u16(), 200, "{body}");
        let row=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT a.status,NOT EXISTS(SELECT 1 FROM information_schema.columns WHERE table_schema='public' AND table_name='accounts' AND column_name='password_hash') AS no_password,NOT EXISTS(SELECT 1 FROM information_schema.columns WHERE table_schema='public' AND table_name='users' AND column_name='password_digest') AS no_digest,p.person_type,p.has_capacity,m.role,(SELECT count(*) FROM auth_credentials c WHERE c.user_id=a.id) AS credentials,(SELECT count(*) FROM person_access_grants g WHERE g.household_membership_id=m.id AND g.person_id=p.id) AS grants,(SELECT count(*) FROM pg_loco_queue q WHERE q.task_data->>'to'=a.email AND q.task_data->>'subject'='Verify your account') AS messages,(SELECT count(*) FROM identity_sessions s WHERE s.account_id=a.id) AS sessions FROM accounts a JOIN people p ON p.id=a.person_id JOIN users u ON u.person_id=p.id JOIN household_memberships m ON m.person_id=a.person_id WHERE a.email=$1",[email.into()])).await.unwrap().unwrap();
        assert_eq!(row.try_get::<i32>("", "status").unwrap(), 1);
        assert!(row.try_get::<bool>("", "no_password").unwrap());
        assert!(row.try_get::<bool>("", "no_digest").unwrap());
        assert_eq!(row.try_get::<i32>("", "person_type").unwrap(), 0);
        assert!(row.try_get::<bool>("", "has_capacity").unwrap());
        assert_eq!(row.try_get::<String>("", "role").unwrap(), "owner");
        assert_eq!(
            row.try_get::<i64>("", "credentials").unwrap(),
            i64::from(credential == "password")
        );
        assert_eq!(row.try_get::<i64>("", "grants").unwrap(), 0);
        assert_eq!(row.try_get::<i64>("", "messages").unwrap(), 1);
        assert_eq!(row.try_get::<i64>("", "sessions").unwrap(), 0);
        if credential == "password" {
            let duplicate = signup(&app, email, credential).await;
            let status = duplicate.status();
            let body = duplicate.text().await.unwrap();
            assert_eq!(status.as_u16(), 200, "{body}");
            let unchanged=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT (SELECT count(*) FROM accounts WHERE email=$1) AS accounts,(SELECT count(*) FROM pg_loco_queue WHERE task_data->>'to'=$1) AS messages",[email.into()])).await.unwrap().unwrap();
            assert_eq!(unchanged.try_get::<i64>("", "accounts").unwrap(), 1);
            assert_eq!(unchanged.try_get::<i64>("", "messages").unwrap(), 1);
        }
    }
    app.close().await;
}

#[tokio::test]
async fn signup_create_queue_failure_rolls_back_account_household_and_verification() {
    let app = Application::new().await;
    open_registration(&app).await;
    let state_sql = "SELECT jsonb_build_object('accounts',(SELECT count(*) FROM accounts),'people',(SELECT count(*) FROM people),'users',(SELECT count(*) FROM users),'households',(SELECT count(*) FROM households),'memberships',(SELECT count(*) FROM household_memberships),'credentials',(SELECT count(*) FROM auth_credentials),'verifications',(SELECT count(*) FROM identity_verifications),'versions',(SELECT count(*) FROM versions),'audits',(SELECT count(*) FROM security_audit_events),'changes',(SELECT count(*) FROM api_change_events),'queue',(SELECT count(*) FROM pg_loco_queue)) AS state";
    let before: Value = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(DbBackend::Postgres, state_sql))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "state")
        .unwrap();
    app.fixture.admin.execute_unprepared("CREATE SEQUENCE signup_queue_attempt; GRANT USAGE ON SEQUENCE signup_queue_attempt TO med_tracker_app; CREATE FUNCTION reject_signup_queue() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('signup_queue_attempt'); RAISE EXCEPTION 'synthetic mail queue failure'; END $$; CREATE TRIGGER reject_signup_queue BEFORE INSERT ON pg_loco_queue FOR EACH ROW EXECUTE FUNCTION reject_signup_queue()").await.unwrap();
    let mut outcomes = Vec::new();
    for (email, credential) in [
        ("rolled-back-password@example.test", "password"),
        ("rolled-back-passkey@example.test", "passkey"),
    ] {
        let response = signup(&app, email, credential).await;
        let status = response.status();
        let body = response.text().await.unwrap();
        outcomes.push((credential, status, body));
        let after: Value = app
            .fixture
            .admin
            .query_one_raw(Statement::from_string(DbBackend::Postgres, state_sql))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "state")
            .unwrap();
        assert_eq!(before, after);
    }
    let reached = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT last_value FROM signup_queue_attempt",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "last_value")
        .unwrap();
    assert_eq!(reached, 2);
    app.fixture
        .admin
        .execute_unprepared("DROP TRIGGER reject_signup_queue ON pg_loco_queue")
        .await
        .unwrap();
    for (email, credential) in [
        ("rolled-back-password@example.test", "password"),
        ("rolled-back-passkey@example.test", "passkey"),
    ] {
        let retry = signup(&app, email, credential).await;
        let status = retry.status();
        let body = retry.text().await.unwrap();
        assert_eq!(status.as_u16(), 200, "{body}");
        let row=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT (SELECT count(*) FROM accounts WHERE email=$1) AS accounts,(SELECT count(*) FROM pg_loco_queue WHERE task_data->>'to'=$1) AS messages",[email.into()])).await.unwrap().unwrap();
        assert_eq!(row.try_get::<i64>("", "accounts").unwrap(), 1);
        assert_eq!(row.try_get::<i64>("", "messages").unwrap(), 1);
    }
    app.close().await;
    assert!(
        outcomes
            .iter()
            .all(|(_, status, _)| status.is_server_error()),
        "queue failure outcomes: {outcomes:?}"
    );
}
