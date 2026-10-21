use super::platform::{operation_start, sign_in};
use super::*;

async fn prepare(app: &Application) -> (String, String) {
    super::account_sessions::stored_credentials(app).await;
    app.fixture.admin.execute_unprepared(
        "INSERT INTO native_device_tokens(account_id,device_token,platform,created_at,updated_at) VALUES(71001,'closure-device','ios',now(),now());
         INSERT INTO push_subscriptions(account_id,endpoint,p256dh,auth,created_at,updated_at) VALUES(71001,'https://example.test/closure','synthetic-key','synthetic-auth',now(),now());
         INSERT INTO identity_api_keys(id,account_id,key_hash,payload) VALUES('closure-key',71001,'closure-hash','{\"enabled\":true}');
         INSERT INTO account_active_session_keys(account_id,session_id,created_at,last_use) VALUES(71001,'closure-session',now(),now())"
    ).await.unwrap();
    let cookie = sign_in(app, 71001, "persistence@example.test").await;
    assert_bearers(app, reqwest::StatusCode::OK).await;
    let (status, body) = operation_start(app, &cookie, json!({"action":"close_account"})).await;
    assert_eq!(status, reqwest::StatusCode::OK);
    (cookie, body["operation_id"].as_str().unwrap().to_owned())
}

async fn confirm(app: &Application, cookie: &str, operation: &str) -> reqwest::Response {
    app.client
        .post(format!("{}/api/auth/security/password/confirm", app.origin))
        .header("cookie", cookie)
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"operation_id":operation,"password":"password"}))
        .send()
        .await
        .unwrap()
}

async fn state(app: &Application) -> Value {
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT jsonb_build_object(
        'account',(SELECT status FROM accounts WHERE id=71001),
        'user',(SELECT active FROM users WHERE person_id=73001),
        'operations',(SELECT jsonb_agg(to_jsonb(v) ORDER BY id) FROM identity_verifications v),
        'memberships',(SELECT jsonb_agg(to_jsonb(m) ORDER BY id) FROM household_memberships m WHERE person_id=73001),
        'grants',(SELECT jsonb_agg(to_jsonb(g) ORDER BY id) FROM person_access_grants g),
        'api_sessions',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM api_sessions s WHERE account_id=71001),
        'app_tokens',(SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM api_app_tokens t WHERE account_id=71001),
        'oauth',(SELECT jsonb_agg(jsonb_build_array(id,revoked_at) ORDER BY id) FROM oauth_grants WHERE account_id=71001),
        'keys',(SELECT jsonb_agg(to_jsonb(k) ORDER BY id) FROM identity_api_keys k WHERE account_id=71001),
        'sessions',(SELECT jsonb_agg(jsonb_build_array(id,active) ORDER BY id) FROM identity_sessions WHERE account_id=71001),
        'registry',(SELECT jsonb_agg(session_id ORDER BY session_id) FROM account_active_session_keys WHERE account_id=71001),
        'push',(SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM push_subscriptions p WHERE account_id=71001),
        'devices',(SELECT jsonb_agg(to_jsonb(d) ORDER BY id) FROM native_device_tokens d WHERE account_id=71001)
        ) AS state")).await.unwrap().unwrap();
    row.try_get("", "state").unwrap()
}

async fn assert_bearers(app: &Application, expected: reqwest::StatusCode) {
    for bearer in ["synthetic-current-session", "synthetic-app-token"] {
        let response = app
            .client
            .get(format!("{}/api/v1/auth/sessions", app.origin))
            .bearer_auth(bearer)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
}

async fn notices_and_audits(app: &Application) -> (i64, i64) {
    let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM pg_loco_queue WHERE task_data->>'subject'='Account closed' AND task_data->>'to'='persistence@example.test') AS notices,
        (SELECT count(*) FROM versions WHERE item_type='AuthenticationToken' AND event='auth_token/account/closed') AS audits"
    )).await.unwrap().unwrap();
    (
        row.try_get("", "notices").unwrap(),
        row.try_get("", "audits").unwrap(),
    )
}

async fn rollback_case(deferred: bool, enqueue: bool) {
    let app = Application::new().await;
    let (cookie, operation) = prepare(&app).await;
    let before = state(&app).await;
    assert_eq!(notices_and_audits(&app).await, (0, 0));
    let trigger = if enqueue {
        "CREATE TRIGGER closure_failure BEFORE INSERT ON pg_loco_queue FOR EACH ROW WHEN (NEW.task_data->>'subject'='Account closed') EXECUTE FUNCTION closure_failure()"
    } else if deferred {
        "CREATE CONSTRAINT TRIGGER closure_failure AFTER UPDATE ON accounts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN (NEW.id=71001 AND NEW.status=3) EXECUTE FUNCTION closure_failure()"
    } else {
        "CREATE TRIGGER closure_failure AFTER UPDATE ON accounts FOR EACH ROW WHEN (NEW.id=71001 AND NEW.status=3) EXECUTE FUNCTION closure_failure()"
    };
    let queued = if enqueue {
        ""
    } else {
        "IF NOT EXISTS(SELECT 1 FROM pg_loco_queue WHERE task_data->>'subject'='Account closed') THEN RAISE EXCEPTION 'closure notice was not reached'; END IF;"
    };
    app.fixture.admin.execute_unprepared(&format!(
        "CREATE SEQUENCE closure_failure_reached;
        GRANT USAGE,SELECT ON SEQUENCE closure_failure_reached TO med_tracker_app;
        CREATE FUNCTION closure_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN {queued} PERFORM nextval('closure_failure_reached'); RAISE EXCEPTION 'synthetic closure failure' USING ERRCODE='23514'; END $$;
        {trigger}"
    )).await.unwrap();
    let response = confirm(&app, &cookie, &operation).await;
    assert_eq!(
        response.status(),
        reqwest::StatusCode::INTERNAL_SERVER_ERROR
    );
    assert!(!response.headers().contains_key("set-cookie"));
    let marker = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT is_called FROM closure_failure_reached",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(
        marker.try_get::<bool>("", "is_called").unwrap(),
        "injected failure must be reached"
    );
    assert_eq!(
        state(&app).await,
        before,
        "all closure effects must roll back"
    );
    assert_eq!(notices_and_audits(&app).await, (0, 0));
    assert_bearers(&app, reqwest::StatusCode::OK).await;
    app.close().await;
}

#[tokio::test]
async fn account_closure_mail_late_failure_rolls_back() {
    rollback_case(false, false).await;
}

#[tokio::test]
async fn account_closure_mail_commit_failure_rolls_back() {
    rollback_case(true, false).await;
}

#[tokio::test]
async fn account_closure_mail_enqueue_failure_rolls_back() {
    rollback_case(false, true).await;
}

#[tokio::test]
async fn account_closure_mail_success_and_replay() {
    let app = Application::new().await;
    let (cookie, operation) = prepare(&app).await;
    let records = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT (SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM people p) AS people,
        (SELECT jsonb_agg(to_jsonb(m) ORDER BY id) FROM medications m) AS medications",
        ))
        .await
        .unwrap()
        .unwrap();
    let response = confirm(&app, &cookie, &operation).await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert!(response.headers().contains_key("set-cookie"));
    assert_eq!(notices_and_audits(&app).await, (1, 1));
    let after = state(&app).await;
    assert_bearers(&app, reqwest::StatusCode::UNAUTHORIZED).await;
    assert_eq!(after["account"], 3);
    assert_eq!(after["user"], false);
    for membership in after["memberships"].as_array().unwrap() {
        assert_eq!(membership["status"], "revoked");
        assert!(!membership["revoked_at"].is_null());
    }
    for kind in ["api_sessions", "app_tokens"] {
        for credential in after[kind].as_array().unwrap() {
            assert!(!credential["revoked_at"].is_null());
        }
    }
    for credential in after["oauth"].as_array().unwrap() {
        assert!(!credential[1].is_null());
    }
    assert_eq!(after["keys"][0]["payload"]["enabled"], false);
    for session in after["sessions"].as_array().unwrap() {
        assert_eq!(session[1], false);
    }
    for kind in ["registry", "push", "devices"] {
        assert!(after[kind].is_null(), "{kind} must be removed");
    }
    let preserved = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT (SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM people p) AS people,
        (SELECT jsonb_agg(to_jsonb(m) ORDER BY id) FROM medications m) AS medications",
        ))
        .await
        .unwrap()
        .unwrap();
    for kind in ["people", "medications"] {
        assert_eq!(
            records.try_get::<Value>("", kind).unwrap(),
            preserved.try_get::<Value>("", kind).unwrap()
        );
    }
    assert!(
        !confirm(&app, &cookie, &operation)
            .await
            .status()
            .is_success()
    );
    assert_eq!(notices_and_audits(&app).await, (1, 1));
    app.close().await;
}
