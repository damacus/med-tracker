use super::Application;
use chrono::{Duration, Utc};
use med_tracker::models::entities::identity_session;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, Statement, sea_query::Expr,
};
use serde_json::{Value, json};

fn cookies(response: &reqwest::Response) -> String {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|value| value.to_str().unwrap().split(';').next().unwrap())
        .collect::<Vec<_>>()
        .join("; ")
}

fn code(uri: &str) -> String {
    let uri = url::Url::parse(uri).unwrap();
    let secret = uri
        .query_pairs()
        .find(|(key, _)| key == "secret")
        .unwrap()
        .1
        .into_owned();
    let secret = String::from_utf8(
        base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &secret).unwrap(),
    )
    .unwrap();
    better_auth::plugins::TwoFactorPlugin::new()
        .generate_totp(&secret)
        .unwrap()
}

async fn login(app: &Application) -> reqwest::Response {
    app.client
        .post(format!("{}/api/auth/sign-in/email", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn public_authenticator_setup_replaces_pending_factor_and_finishes_once() {
    let app = Application::new().await;
    let cookie = super::platform::sign_in(&app, 71001, "persistence@example.test").await;
    let selected = identity_session::Entity::update_many()
        .col_expr(
            identity_session::Column::ActiveHouseholdId,
            Expr::value(72001_i64),
        )
        .filter(identity_session::Column::UserId.eq(71001_i64))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(selected.rows_affected, 1);
    let mut pending = Vec::new();
    for _ in 0..2 {
        let response = app
            .client
            .post(format!("{}/api/auth/security/totp/start", app.origin))
            .header("origin", app.context.config.server.full_url())
            .header("cookie", &cookie)
            .json(&json!({"password":"password"}))
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body: Value = response.json().await.unwrap();
        assert_eq!(status.as_u16(), 200, "{body}");
        assert_eq!(body["backupCodes"].as_array().unwrap().len(), 10);
        assert!(body["totpURI"].as_str().unwrap().contains("MedTracker"));
        pending.push(body);
    }
    let current_code = code(pending[1]["totpURI"].as_str().unwrap());
    let rejected = app
        .client
        .post(format!("{}/api/auth/security/totp/finish", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", &cookie)
        .json(&json!({"code":"invalid"}))
        .send()
        .await
        .unwrap();
    assert!(rejected.status().is_client_error());
    let state = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT two_factor_enabled FROM accounts WHERE id=71001",
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(!state.try_get::<bool>("", "two_factor_enabled").unwrap());
    let enrolled = app
        .client
        .post(format!("{}/api/auth/security/totp/finish", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", &cookie)
        .json(&json!({"code":current_code}))
        .send()
        .await
        .unwrap();
    let status = enrolled.status();
    let enrolled_cookie = cookies(&enrolled);
    let body: Value = enrolled.json().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    let state=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT a.two_factor_enabled,f.verified FROM accounts a JOIN identity_two_factors f ON f.account_id=a.id WHERE a.id=71001")).await.unwrap().unwrap();
    assert!(state.try_get::<bool>("", "two_factor_enabled").unwrap());
    assert!(state.try_get::<bool>("", "verified").unwrap());
    let rotated = identity_session::Entity::find()
        .filter(identity_session::Column::UserId.eq(71001_i64))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(rotated.len(), 1);
    assert_eq!(rotated[0].active_household_id, Some(72001));
    let current = app
        .client
        .get(format!("{}/api/auth/get-session", app.origin))
        .header("cookie", &enrolled_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(current.status().as_u16(), 200);
    let current: Value = current.json().await.unwrap();
    assert_eq!(current["session"]["active_household_id"], "72001");
    assert!(
        current["session"]
            .get("activeOrganizationId")
            .is_none_or(Value::is_null)
    );
    let home = app
        .client
        .get(format!("{}/", app.origin))
        .header("cookie", &enrolled_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(home.status().as_u16(), 200);
    assert_eq!(
        home.url().path(),
        "/households/persistence-fixture/dashboard"
    );
    let old = app
        .client
        .get(format!("{}/api/auth/get-session", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert!(old.json::<Value>().await.unwrap().is_null());
    let first_totp = login(&app).await;
    let totp_cookie = cookies(&first_totp);
    let challenge: Value = first_totp.json().await.unwrap();
    assert_eq!(challenge["twoFactorRedirect"], true);
    let totp_login = app
        .client
        .post(format!("{}/api/auth/security/totp/login", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", totp_cookie)
        .json(&json!({"code":current_code}))
        .send()
        .await
        .unwrap();
    let status = totp_login.status();
    let totp_session = cookies(&totp_login);
    let body: Value = totp_login.json().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    let live = app
        .client
        .get(format!("{}/api/auth/get-session", app.origin))
        .header("cookie", totp_session)
        .send()
        .await
        .unwrap();
    assert_eq!(live.status().as_u16(), 200);
    assert!(live.json::<Value>().await.unwrap()["session"].is_object());
    let first = login(&app).await;
    assert_eq!(first.status().as_u16(), 200);
    let pending_cookie = cookies(&first);
    let body: Value = first.json().await.unwrap();
    assert_eq!(body["twoFactorRedirect"], true, "{body}");
    let backup = pending[1]["backupCodes"][0].as_str().unwrap();
    let proved = app
        .client
        .post(format!("{}/api/auth/security/totp/login", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", &pending_cookie)
        .json(&json!({"backup":true,"code":backup}))
        .send()
        .await
        .unwrap();
    let status = proved.status();
    let proof_cookie = cookies(&proved);
    let body: Value = proved.json().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    let authenticated = app
        .client
        .get(format!("{}/api/auth/get-session", app.origin))
        .header("cookie", proof_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(authenticated.status().as_u16(), 200);
    assert_eq!(
        authenticated.json::<Value>().await.unwrap()["user"]["twoFactorEnabled"],
        true
    );
    let replay_challenge = login(&app).await;
    let replay_cookie = cookies(&replay_challenge);
    let replay = app
        .client
        .post(format!("{}/api/auth/security/totp/login", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", replay_cookie)
        .json(&json!({"backup":true,"code":backup}))
        .send()
        .await
        .unwrap();
    assert!(replay.status().is_client_error());
    let before = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT backup_codes FROM identity_two_factors WHERE account_id=71001",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<String>("", "backup_codes")
        .unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO account_lockouts(account_id,key,deadline) VALUES(71001,'native-mfa-lock',now()+interval '1 hour')").await.unwrap();
    for path in ["/two-factor/generate-backup-codes", "/two-factor/disable"] {
        let denied = app
            .client
            .post(format!("{}/api/auth{path}", app.origin))
            .header("origin", app.context.config.server.full_url())
            .header("cookie", &enrolled_cookie)
            .json(&json!({"password":"password"}))
            .send()
            .await
            .unwrap();
        assert!(denied.status().is_client_error());
    }
    let after = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT backup_codes FROM identity_two_factors WHERE account_id=71001",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<String>("", "backup_codes")
        .unwrap();
    assert_eq!(before, after);
    app.fixture
        .admin
        .execute_unprepared("DELETE FROM account_lockouts WHERE account_id=71001")
        .await
        .unwrap();
    let disabled = app
        .client
        .post(format!(
            "{}/api/auth/security/totp/disable/confirm",
            app.origin
        ))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", &enrolled_cookie)
        .json(&json!({"password":"password"}))
        .send()
        .await
        .unwrap();
    let status = disabled.status();
    let disabled_cookie = cookies(&disabled);
    let body: Value = disabled.json().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    let disabled_state=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT a.two_factor_enabled,(SELECT count(*) FROM identity_two_factors WHERE account_id=a.id) AS factors FROM accounts a WHERE a.id=71001")).await.unwrap().unwrap();
    assert!(
        !disabled_state
            .try_get::<bool>("", "two_factor_enabled")
            .unwrap()
    );
    assert_eq!(disabled_state.try_get::<i64>("", "factors").unwrap(), 0);
    let current = app
        .client
        .get(format!("{}/api/auth/get-session", app.origin))
        .header("cookie", disabled_cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(
        current.json::<Value>().await.unwrap()["user"]["twoFactorEnabled"],
        false
    );
    let old = app
        .client
        .get(format!("{}/api/auth/get-session", app.origin))
        .header("cookie", &enrolled_cookie)
        .send()
        .await
        .unwrap();
    assert!(old.json::<Value>().await.unwrap().is_null());
    app.close().await;
}

#[tokio::test]
async fn zero_quota_personal_key_public_replay_is_denied_with_redacted_errors() {
    let app = Application::new().await;
    let service = app
        .context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let issued = service
        .api_keys()
        .unwrap()
        .create(
            "71001",
            better_auth::server_api::CreateKeyOptions {
                config_id: Some("medtracker-personal".into()),
                name: Some("Exhausted synthetic key".into()),
                expires_in: Some(Duration::days(1).num_seconds() as f64),
                remaining: Some(0.0),
                metadata: Some(json!({"households":[72001]})),
                permissions: Some(std::collections::HashMap::from([(
                    "medtracker".into(),
                    vec!["care:read".into()],
                )])),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let secret = issued.key;
    let mut first = None;
    for _ in 0..2 {
        let response = app
            .client
            .get(format!("{}/api/v1/households/72001/profile", app.origin))
            .bearer_auth(&secret)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), 401);
        let mut body: Value = response.json().await.unwrap();
        assert!(!body.to_string().contains(&secret));
        let request_id = body["error"]["request_id"].as_str().unwrap().to_owned();
        assert!(!request_id.is_empty());
        if let Some(first) = &first {
            assert_ne!(&request_id, first);
        }
        first = Some(request_id);
        body["error"].as_object_mut().unwrap().remove("request_id");
        assert_eq!(
            body,
            json!({"error":{"code":"unauthorized","message":"Authentication required"}})
        );
    }
    assert!(
        issued
            .api_key
            .expires_at
            .typed()
            .unwrap()
            .as_ref()
            .unwrap()
            .to_datetime()
            .unwrap()
            .unwrap()
            > Utc::now()
    );
    app.close().await;
}

#[tokio::test]
async fn native_personal_key_scoped_reads_writes_and_live_membership_denial() {
    let app = Application::new().await;
    app.fixture.seed_foreign_household(72002).await;
    let service = app
        .context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let issued = service
        .api_keys()
        .unwrap()
        .create(
            "71001",
            better_auth::server_api::CreateKeyOptions {
                config_id: Some("medtracker-personal".into()),
                name: Some("Scoped synthetic key".into()),
                expires_in: Some(Duration::days(1).num_seconds() as f64),
                metadata: Some(json!({"households":[72001]})),
                permissions: Some(std::collections::HashMap::from([(
                    "medtracker".into(),
                    vec!["care:read".into(), "care:write".into()],
                )])),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let secret = issued.key;
    let read = app
        .client
        .get(format!("{}/api/v1/households/72001/profile", app.origin))
        .bearer_auth(&secret)
        .send()
        .await
        .unwrap();
    let status = read.status();
    let body: Value = read.json().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    assert!(!body.to_string().contains(&secret));
    let write = app
        .client
        .patch(format!("{}/api/v1/households/72001/profile", app.origin))
        .bearer_auth(&secret)
        .header("idempotency-key", "native-key-profile-write")
        .json(&json!({"profile":{"time_zone":"Europe/London"}}))
        .send()
        .await
        .unwrap();
    let status = write.status();
    let body: Value = write.json().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    let stored = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT preferences->>'time_zone' AS zone FROM accounts WHERE id=71001",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<String>("", "zone")
        .unwrap();
    assert_eq!(stored, "Europe/London");
    let foreign = app
        .client
        .get(format!("{}/api/v1/households/72002/profile", app.origin))
        .bearer_auth(&secret)
        .send()
        .await
        .unwrap();
    assert_eq!(foreign.status().as_u16(), 403);
    let revoked = app
        .fixture
        .admin
        .execute_unprepared(
            "UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE id=74001",
        )
        .await
        .unwrap();
    assert_eq!(revoked.rows_affected(), 1);
    let denied = app
        .client
        .get(format!("{}/api/v1/households/72001/profile", app.origin))
        .bearer_auth(&secret)
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status().as_u16(), 403);
    app.close().await;
}

async fn key_lifecycle_state(app: &Application) -> Value {
    app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT jsonb_build_object('keys',(SELECT jsonb_agg(to_jsonb(k) ORDER BY id) FROM identity_api_keys k WHERE reference_id=71001),'audits',(SELECT jsonb_agg(to_jsonb(v) ORDER BY id) FROM versions v WHERE event LIKE 'native_auth/api_key/%'),'notices',(SELECT jsonb_agg(to_jsonb(q) ORDER BY id) FROM pg_loco_queue q WHERE task_data->>'subject' IN('API key created','API key revoked'))) AS state"
    )).await.unwrap().unwrap().try_get("","state").unwrap()
}

async fn create_lifecycle_key(app: &Application, cookie: &str) -> reqwest::Response {
    app.client.post(format!("{}/api/auth/security/api-key/create",app.origin))
        .header("origin",app.context.config.server.full_url()).header("cookie",cookie)
        .json(&json!({"name":"Atomic lifecycle key","households":[72001],"permissions":["care:read"],"expires_days":30})).send().await.unwrap()
}

#[tokio::test]
async fn native_personal_key_lifecycle_rolls_back_notice_and_audit() {
    let app = Application::new().await;
    let cookie = super::platform::sign_in(&app, 71001, "persistence@example.test").await;
    let before = key_lifecycle_state(&app).await;
    app.fixture.admin.execute_unprepared("CREATE SEQUENCE key_failure_reached; GRANT USAGE,SELECT ON SEQUENCE key_failure_reached TO med_tracker_app; CREATE FUNCTION reject_key_notice() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('public.key_failure_reached'); RAISE EXCEPTION 'synthetic key notice failure' USING ERRCODE='23514'; END $$; CREATE TRIGGER reject_key_notice BEFORE INSERT ON pg_loco_queue FOR EACH ROW WHEN(NEW.task_data->>'subject' IN('API key created','API key revoked')) EXECUTE FUNCTION reject_key_notice()").await.unwrap();
    let rejected = create_lifecycle_key(&app, &cookie).await;
    assert_eq!(rejected.status().as_u16(), 500);
    assert!(
        app.fixture
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT is_called FROM key_failure_reached"
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get::<bool>("", "is_called")
            .unwrap()
    );
    assert_eq!(
        key_lifecycle_state(&app).await,
        before,
        "key, notice and audit must all roll back"
    );
    app.fixture
        .admin
        .execute_unprepared("DROP TRIGGER reject_key_notice ON pg_loco_queue")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("REVOKE INSERT ON public.versions FROM med_tracker_app")
        .await
        .unwrap();
    let audit_rejected = create_lifecycle_key(&app, &cookie).await;
    assert_eq!(audit_rejected.status().as_u16(), 500);
    assert_eq!(
        key_lifecycle_state(&app).await,
        before,
        "failed key audit must roll back native insertion"
    );
    app.fixture
        .admin
        .execute_unprepared("GRANT INSERT ON public.versions TO med_tracker_app")
        .await
        .unwrap();
    let issued = create_lifecycle_key(&app, &cookie).await;
    assert_eq!(issued.status().as_u16(), 200);
    let body: Value = issued.json().await.unwrap();
    let secret = body["apiKey"].as_str().unwrap();
    assert!(secret.starts_with("medtracker_"));
    let created = key_lifecycle_state(&app).await;
    assert_eq!(created["keys"].as_array().unwrap().len(), 1);
    assert_eq!(created["audits"].as_array().unwrap().len(), 1);
    assert_eq!(created["audits"][0]["event"], "native_auth/api_key/created");
    assert_eq!(created["notices"].as_array().unwrap().len(), 1);
    assert!(!serde_json::to_string(&created).unwrap().contains(secret));
    assert_eq!(
        created["keys"][0]["metadata"],
        serde_json::to_string(&json!({"households":[72001]})).unwrap()
    );
    let key_id = created["keys"][0]["id"].as_i64().unwrap().to_string();
    app.fixture.admin.execute_unprepared("CREATE TRIGGER reject_key_notice BEFORE INSERT ON pg_loco_queue FOR EACH ROW WHEN(NEW.task_data->>'subject'='API key revoked') EXECUTE FUNCTION reject_key_notice()").await.unwrap();
    let revoke = || {
        app.client
            .post(format!("{}/api/auth/security/api-key/revoke", app.origin))
            .header("origin", app.context.config.server.full_url())
            .header("cookie", &cookie)
            .json(&json!({"key_id":key_id}))
            .send()
    };
    assert_eq!(revoke().await.unwrap().status().as_u16(), 500);
    assert_eq!(
        key_lifecycle_state(&app).await,
        created,
        "failed revocation notice must preserve enabled key and audit"
    );
    app.fixture
        .admin
        .execute_unprepared("DROP TRIGGER reject_key_notice ON pg_loco_queue")
        .await
        .unwrap();
    assert_eq!(revoke().await.unwrap().status().as_u16(), 200);
    let revoked = key_lifecycle_state(&app).await;
    assert_eq!(revoked["keys"][0]["enabled"], false);
    assert_eq!(revoked["audits"].as_array().unwrap().len(), 2);
    assert_eq!(revoked["notices"].as_array().unwrap().len(), 2);
    assert_eq!(revoked["audits"][1]["event"], "native_auth/api_key/updated");
    assert_eq!(revoke().await.unwrap().status().as_u16(), 200);
    assert_eq!(
        key_lifecycle_state(&app).await,
        revoked,
        "idempotent revoke must not repeat lifecycle side effects"
    );
    app.close().await;
}

#[tokio::test]
async fn native_personal_key_create_retains_authoritative_admission_and_typed_scopes() {
    let app = Application::new().await;
    let cookie = super::platform::sign_in(&app, 71001, "persistence@example.test").await;
    let input = json!({"name":"Admission key","households":[72001],"permissions":["care:read"],"expires_days":30});
    let before = key_lifecycle_state(&app).await;
    let anonymous = app
        .client
        .post(format!("{}/api/auth/security/api-key/create", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&input)
        .send()
        .await
        .unwrap();
    assert_eq!(anonymous.status().as_u16(), 401);
    let csrf = app
        .client
        .post(format!("{}/api/auth/security/api-key/create", app.origin))
        .header("origin", "https://foreign.example.test")
        .header("cookie", &cookie)
        .json(&input)
        .send()
        .await
        .unwrap();
    assert_eq!(csrf.status().as_u16(), 403);
    for injected in [
        json!({"userId":"71002"}),
        json!({"configId":"other"}),
        json!({"metadata":{"households":[72002]}}),
        json!({"remaining":100}),
    ] {
        let mut body = input.clone();
        body.as_object_mut()
            .unwrap()
            .extend(injected.as_object().unwrap().clone());
        let denied = app
            .client
            .post(format!("{}/api/auth/security/api-key/create", app.origin))
            .header("origin", app.context.config.server.full_url())
            .header("cookie", &cookie)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(denied.status().as_u16(), 400);
    }
    for (field, value) in [
        ("households", json!([72002])),
        ("permissions", json!(["unsupported:write"])),
        ("expires_days", json!(366)),
    ] {
        let mut body = input.clone();
        body[field] = value;
        let denied = app
            .client
            .post(format!("{}/api/auth/security/api-key/create", app.origin))
            .header("origin", app.context.config.server.full_url())
            .header("cookie", &cookie)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert!(denied.status().is_client_error());
    }
    let bypass = app
        .client
        .post(format!("{}/api/auth/api-key/create", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", &cookie)
        .json(&json!({"name":"Bypass key"}))
        .send()
        .await
        .unwrap();
    assert_eq!(bypass.status().as_u16(), 403);
    for (mutation, restore) in [
        (
            "UPDATE identity_sessions SET purpose='enrolment' WHERE account_id=71001",
            "UPDATE identity_sessions SET purpose='authenticated' WHERE account_id=71001",
        ),
        (
            "UPDATE accounts SET status=3 WHERE id=71001",
            "UPDATE accounts SET status=2 WHERE id=71001",
        ),
        (
            "INSERT INTO account_lockouts(account_id,key,deadline) VALUES(71001,'key-admission',now()+interval '1 hour')",
            "DELETE FROM account_lockouts WHERE account_id=71001",
        ),
        (
            "UPDATE identity_sessions SET expires_at=now()-interval '1 minute' WHERE account_id=71001",
            "UPDATE identity_sessions SET expires_at=now()+interval '1 day' WHERE account_id=71001",
        ),
    ] {
        app.fixture
            .admin
            .execute_unprepared(mutation)
            .await
            .unwrap();
        let denied = app
            .client
            .post(format!("{}/api/auth/security/api-key/create", app.origin))
            .header("origin", app.context.config.server.full_url())
            .header("cookie", &cookie)
            .json(&input)
            .send()
            .await
            .unwrap();
        assert!(denied.status().is_client_error(), "{}", denied.status());
        app.fixture.admin.execute_unprepared(restore).await.unwrap();
    }
    assert_eq!(key_lifecycle_state(&app).await, before);
    let cookie = super::platform::sign_in(&app, 71001, "persistence@example.test").await;
    let allowed = create_lifecycle_key(&app, &cookie).await;
    assert_eq!(allowed.status().as_u16(), 200);
    let body: Value = allowed.json().await.unwrap();
    let secret = body["apiKey"].as_str().unwrap();
    let before_use = key_lifecycle_state(&app).await;
    let clinical = app
        .client
        .get(format!("{}/api/v1/households/72001/profile", app.origin))
        .bearer_auth(secret)
        .send()
        .await
        .unwrap();
    assert_eq!(clinical.status().as_u16(), 200);
    let after_use = key_lifecycle_state(&app).await;
    assert_eq!(after_use["audits"], before_use["audits"]);
    assert_eq!(after_use["notices"], before_use["notices"]);
    assert!(
        !serde_json::to_string(&after_use["audits"])
            .unwrap()
            .contains(secret)
    );
    app.close().await;
}

#[tokio::test]
async fn native_personal_key_create_waits_for_account_closure_and_rechecks_status() {
    use sea_orm::TransactionTrait;
    let app = Application::new().await;
    let cookie = super::platform::sign_in(&app, 71001, "persistence@example.test").await;
    let transaction = app.fixture.admin.begin().await.unwrap();
    transaction
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id FROM accounts WHERE id=71001 FOR UPDATE",
        ))
        .await
        .unwrap();
    let client = app.client.clone();
    let origin = app.origin.clone();
    let public = app.context.config.server.full_url();
    let create = tokio::spawn(async move {
        client.post(format!("{origin}/api/auth/security/api-key/create")).header("origin",public).header("cookie",cookie).json(&json!({"name":"Concurrent close key","households":[72001],"permissions":["care:read"],"expires_days":30})).send().await.unwrap()
    });
    let waited=tokio::time::timeout(std::time::Duration::from_secs(5),async {
        loop {
            let row=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE '%FOR NO KEY UPDATE%') AS waited")).await.unwrap().unwrap();
            if row.try_get::<bool>("","waited").unwrap() {break;}
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await;
    transaction
        .execute_unprepared("UPDATE accounts SET status=3 WHERE id=71001")
        .await
        .unwrap();
    transaction.commit().await.unwrap();
    let response = tokio::time::timeout(std::time::Duration::from_secs(5), create)
        .await
        .unwrap()
        .unwrap();
    assert!(
        waited.is_ok(),
        "key creation must serialise with account closure"
    );
    assert_eq!(response.status().as_u16(), 401);
    let state = key_lifecycle_state(&app).await;
    assert!(state["keys"].is_null());
    assert!(state["notices"].is_null());
    assert!(state["audits"].is_null());
    app.close().await;
}
