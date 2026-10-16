use super::*;

#[tokio::test]
async fn account_authentication_without_clinical_records_can_login_and_refresh() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,name,password_hash,status,created_at,updated_at) VALUES(71002,'account-auth-only@example.test','Canonical account name',crypt('password',gen_salt('bf',4)),2,now(),now()); INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71002,now())").await.unwrap();
    let request = server(&boot);
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"account-auth-only@example.test","password":"password"}))
        .await;
    assert_eq!(signed_in.status_code(), 200, "{}", signed_in.text());
    let body: Value = signed_in.json();
    assert_eq!(body["user"]["name"], "Canonical account name");
    let cookie = signed_in
        .header("set-cookie")
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    fixture.admin.execute_unprepared("UPDATE identity_sessions SET created_at=now()-interval '2 days',updated_at=now()-interval '2 days',expires_at=now()+interval '1 hour' WHERE account_id=71002").await.unwrap();
    let refreshed = request
        .get("/api/auth/get-session")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(refreshed.status_code(), 200, "{}", refreshed.text());
    assert_eq!(
        refreshed.json::<Value>()["user"]["name"],
        "Canonical account name"
    );
    let renewed = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT expires_at>now()+interval '2 hours' AS renewed FROM identity_sessions WHERE account_id=71002"
    )).await.unwrap().unwrap();
    assert!(renewed.try_get::<bool>("", "renewed").unwrap());
    let security = request
        .get("/account/security")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(security.status_code(), 200);
    let operation = request
        .post("/api/auth/security/operation/start")
        .add_header("origin", boot.app_context.config.server.full_url())
        .add_header("cookie", cookie.clone())
        .json(&json!({"action":"add_passkey"}))
        .await;
    assert_eq!(operation.status_code(), 200, "{}", operation.text());
    let operation_id = operation.json::<Value>()["operation_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let proof = request
        .post("/api/auth/security/password/confirm")
        .add_header("origin", boot.app_context.config.server.full_url())
        .add_header("cookie", cookie.clone())
        .json(&json!({"operation_id":operation_id,"password":"password"}))
        .await;
    assert_eq!(proof.status_code(), 200, "{}", proof.text());
    let clinical = request
        .get("/households/persistence-fixture/profile")
        .add_header("cookie", cookie)
        .await;
    assert_eq!(clinical.status_code(), 403, "{}", clinical.text());
    let audit = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS events,bool_and(whodunnit IS NULL AND audit_context->>'actor_account_id'='71002' AND audit_context->>'actor_user_id' IS NULL) AS account_only FROM versions WHERE item_type='AuthenticationToken' AND item_id=71002")).await.unwrap().unwrap();
    assert!(audit.try_get::<i64>("", "events").unwrap() > 0);
    assert!(audit.try_get::<bool>("", "account_only").unwrap());
    let token = super::account::mobile_token(&fixture).await;
    fixture
        .admin
        .execute_unprepared("UPDATE oauth_grants SET account_id=71002 WHERE client_kind='mobile'")
        .await
        .unwrap();
    let profile = request
        .get("/api/v1/auth/profile")
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    assert_eq!(profile.status_code(), 200);
    assert_eq!(
        profile.json::<Value>()["data"]["name"],
        "Canonical account name"
    );
    let clinical = request
        .get("/api/v1/households/72001/people")
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    assert_eq!(clinical.status_code(), 403);
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_get_session_rejects_lockout_before_refresh() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_unprepared("UPDATE users SET active=false; INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=now()").await.unwrap();
    let request = server(&boot);
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(signed_in.status_code(), 200, "{}", signed_in.text());
    assert!(signed_in.json::<Value>()["user"]["name"].is_null());
    let cookie = signed_in
        .header("set-cookie")
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    fixture.admin.execute_unprepared("UPDATE identity_sessions SET created_at=now()-interval '2 days',updated_at=now()-interval '2 days',expires_at=now()+interval '1 hour' WHERE account_id=71001; INSERT INTO account_lockouts(account_id,key,deadline) VALUES(71001,'synthetic-account-auth-lock',now()+interval '1 hour')").await.unwrap();
    let rejected = request
        .get("/api/auth/get-session")
        .add_header("cookie", cookie)
        .await;
    assert_eq!(rejected.status_code(), 401, "{}", rejected.text());
    let state = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT expires_at<now()+interval '2 hours' AS unchanged FROM identity_sessions WHERE account_id=71001")).await.unwrap().unwrap();
    assert!(state.try_get::<bool>("", "unchanged").unwrap());
    let login = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(login.status_code(), 401, "{}", login.text());
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_audit_failure_rolls_back_session() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,password_hash,status,created_at,updated_at) VALUES(71002,'account-auth-only@example.test',crypt('password',gen_salt('bf',4)),2,now(),now()); CREATE FUNCTION reject_account_auth_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='AuthenticationToken' AND NEW.item_id=71002 THEN RAISE EXCEPTION 'synthetic authentication audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_account_auth_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_account_auth_audit()").await.unwrap();
    let request = server(&boot);
    let rejected = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"account-auth-only@example.test","password":"password"}))
        .await;
    assert_eq!(rejected.status_code(), 500);
    let state = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM identity_sessions WHERE account_id=71002) AS sessions,(SELECT count(*) FROM versions WHERE item_type='AuthenticationToken' AND item_id=71002) AS audits")).await.unwrap().unwrap();
    assert_eq!(state.try_get::<i64>("", "sessions").unwrap(), 0);
    assert_eq!(state.try_get::<i64>("", "audits").unwrap(), 0);
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_account_status_and_session_lifetime_remain_required() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,password_hash,status,created_at,updated_at) VALUES(71002,'account-auth-only@example.test',crypt('password',gen_salt('bf',4)),1,now(),now()); INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71002,now())").await.unwrap();
    let request = server(&boot);
    for status in [1, 3] {
        fixture
            .admin
            .execute_unprepared(&format!(
                "UPDATE accounts SET status={status} WHERE id=71002"
            ))
            .await
            .unwrap();
        let rejected = request
            .post("/api/auth/sign-in/email")
            .add_header("origin", boot.app_context.config.server.full_url())
            .json(&json!({"email":"account-auth-only@example.test","password":"password"}))
            .await;
        assert_eq!(rejected.status_code(), 401, "{}", rejected.text());
    }
    fixture
        .admin
        .execute_unprepared("UPDATE accounts SET status=2 WHERE id=71002")
        .await
        .unwrap();
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"account-auth-only@example.test","password":"password"}))
        .await;
    assert_eq!(signed_in.status_code(), 200, "{}", signed_in.text());
    assert!(signed_in.json::<Value>()["user"]["name"].is_null());
    let cookie = signed_in
        .header("set-cookie")
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    for mutation in [
        "active=false",
        "active=true,expires_at=now()-interval '1 second'",
        "expires_at=now()+interval '1 hour',created_at=now()-interval '31 days'",
    ] {
        fixture
            .admin
            .execute_unprepared(&format!(
                "UPDATE identity_sessions SET {mutation} WHERE account_id=71002"
            ))
            .await
            .unwrap();
        let rejected = request
            .get("/api/auth/get-session")
            .add_header("cookie", cookie.clone())
            .await;
        assert_eq!(rejected.status_code(), 200, "{}", rejected.text());
        assert!(rejected.json::<Value>().is_null());
    }
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_retained_factor_promotes_without_clinical_actor() {
    let (fixture, boot) = boot_fixture_with(|config| {
        config.settings.get_or_insert_with(|| json!({}))["rails_compat"] =
            json!({"secret_key_base":"synthetic-auth-factor-secret"});
    })
    .await;
    fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,password_hash,status,created_at,updated_at) VALUES(71002,'account-auth-only@example.test',crypt('password',gen_salt('bf',4)),2,now(),now()); INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71002,now()); INSERT INTO account_otp_keys(id,key,last_use,num_failures) VALUES(71002,'abcdefghijklmnop',now()-interval '1 day',0)").await.unwrap();
    let request = server(&boot);
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"account-auth-only@example.test","password":"password"}))
        .await;
    assert_eq!(signed_in.status_code(), 200, "{}", signed_in.text());
    let cookie = signed_in
        .header("set-cookie")
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let pending = request
        .get("/api/auth/get-session")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(
        pending.json::<Value>()["session"]["legacy_factor_pending"],
        true
    );
    let secret = med_tracker::models::identity::totp::derive_rodauth_otp_secret(
        "abcdefghijklmnop",
        Some(b"synthetic-auth-factor-secret"),
    )
    .unwrap();
    let code = totp_rs::Builder::new()
        .with_algorithm(totp_rs::Algorithm::SHA1)
        .with_digits(6)
        .with_step_duration(30)
        .with_skew(0)
        .with_secret(totp_rs::Secret::try_from_base32(secret.to_ascii_uppercase()).unwrap())
        .build_noncompliant()
        .generate_current()
        .to_string();
    let verified = request
        .post("/api/auth/security/totp/login")
        .add_header("origin", boot.app_context.config.server.full_url())
        .add_header("cookie", cookie.clone())
        .json(&json!({"code":code}))
        .await;
    assert_eq!(verified.status_code(), 200, "{}", verified.text());
    let authenticated_cookie = verified
        .header("set-cookie")
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let promoted = request
        .get("/api/auth/get-session")
        .add_header("cookie", authenticated_cookie)
        .await;
    assert_eq!(
        promoted.json::<Value>()["session"]["clinical_session_purpose"],
        "authenticated"
    );
    let consumed = request
        .get("/api/auth/get-session")
        .add_header("cookie", cookie)
        .await;
    assert_eq!(consumed.status_code(), 200);
    assert!(consumed.json::<Value>().is_null());
    drop(request);
    close(fixture, boot).await;
}
