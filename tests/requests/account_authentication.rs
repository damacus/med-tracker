use super::*;
use base64::Engine as _;
use chrono::{Duration, Utc};
use med_tracker::models::_entities::{
    accounts, app_settings, identity_api_keys, identity_onboarding, people,
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

async fn enable_completed_onboarding(fixture: &Fixture, account_id: i64) {
    let active = identity_onboarding::ActiveModel {
        account_id: Set(account_id),
        recovery_saved_at: Set(Some(Utc::now().fixed_offset())),
        ..Default::default()
    };
    if identity_onboarding::Entity::find_by_id(account_id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .is_some()
    {
        active.update(&fixture.admin).await.unwrap();
    } else {
        active.insert(&fixture.admin).await.unwrap();
    }
}

fn inventory_key(
    id: &str,
    config_id: &str,
    created_at: chrono::DateTime<Utc>,
    expires_at: chrono::DateTime<Utc>,
    enabled: bool,
    remaining: f64,
) -> identity_api_keys::ActiveModel {
    let hash = format!("{id}-hash");
    identity_api_keys::ActiveModel {
        id: Set(id.to_owned()),
        account_id: Set(71001),
        key_hash: Set(hash.clone()),
        expires_at: Set(Some(expires_at.fixed_offset())),
        payload: Set(json!({
            "id":id,"key":hash,"referenceId":"71001","configId":config_id,
            "name":id,"enabled":enabled,"remaining":remaining,
            "rateLimitEnabled":true,"rateLimitTimeWindow":60_000.0,"rateLimitMax":120.0,
            "requestCount":0.0,"refillAmount":null,"permissions":null,"metadata":null,
            "createdAt":created_at.to_rfc3339(),"updatedAt":created_at.to_rfc3339(),
            "expiresAt":expires_at.to_rfc3339()
        })),
    }
}

#[tokio::test]
async fn account_authentication_open_signup_without_invitation_persists_identity_and_mail() {
    let (fixture, boot) = boot_fixture().await;
    let settings = app_settings::ActiveModel {
        invite_only: Set(false),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    let request = server(&boot);
    let signed_up = request
        .post("/api/auth/onboarding/signup")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"name":"Open registration owner","email":"open-null-invitation@example.test","date_of_birth":"1990-04-12","credential":"passkey"}))
        .await;
    assert_eq!(signed_up.status_code(), 200, "{}", signed_up.text());
    assert_eq!(signed_up.json::<Value>()["status"], true);
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM accounts WHERE email='open-null-invitation@example.test' AND status=1) AS accounts,(SELECT count(*) FROM pg_loco_queue WHERE task_data->>'to'='open-null-invitation@example.test') AS messages"
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "accounts").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "messages").unwrap(), 1);
    app_settings::ActiveModel {
        id: Set(settings.id),
        invite_only: Set(true),
        ..Default::default()
    }
    .update(&fixture.admin)
    .await
    .unwrap();
    let closed = request
        .post("/api/auth/onboarding/signup")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"name":"Closed registration","email":"closed-null-invitation@example.test","date_of_birth":"1990-04-12","credential":"passkey"}))
        .await;
    assert_eq!(closed.status_code(), 403, "{}", closed.text());
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) AS accounts FROM accounts WHERE email='closed-null-invitation@example.test'"
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "accounts").unwrap(), 0);
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_security_inventory_includes_later_credentials() {
    let (fixture, boot) = boot_fixture().await;
    people::ActiveModel {
        id: Set(73992),
        name: Set("Other account owner".into()),
        person_type: Set(0),
        has_capacity: Set(true),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    let password_hash = accounts::Entity::find_by_id(71001)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .password_hash;
    accounts::ActiveModel {
        id: Set(71002),
        person_id: Set(73992),
        email: Set("foreign@example.test".into()),
        password_hash: Set(password_hash),
        status: Set(2),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    enable_completed_onboarding(&fixture, 71001).await;
    enable_completed_onboarding(&fixture, 71002).await;
    let now = Utc::now();
    let mut keys = (0..101)
        .map(|index| {
            let created = now - Duration::days(2) + Duration::seconds(index);
            let expired = index % 3 == 0;
            inventory_key(
                &format!("earlier-{index:03}"),
                if index % 2 == 0 {
                    "medtracker-personal"
                } else {
                    "medtracker-recovery"
                },
                created,
                if expired {
                    now - Duration::hours(1)
                } else {
                    now + Duration::days(1)
                },
                index % 3 != 1,
                if index % 3 == 2 { 0.0 } else { 1.0 },
            )
        })
        .collect::<Vec<_>>();
    keys.push(inventory_key(
        "later-active-personal",
        "medtracker-personal",
        now,
        now + Duration::days(1),
        true,
        1.0,
    ));
    for index in 0..10 {
        keys.push(inventory_key(
            &format!("later-valid-recovery-{index}"),
            "medtracker-recovery",
            now + Duration::seconds(index + 1),
            now + Duration::days(1),
            true,
            1.0,
        ));
    }
    identity_api_keys::Entity::insert_many(keys)
        .exec(&fixture.admin)
        .await
        .unwrap();
    let request = server(&boot);
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(signed_in.status_code(), 200, "{}", signed_in.text());
    let owner_cookie = signed_in
        .header("set-cookie")
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let listing = request
        .get("/account/security/keys")
        .add_header("cookie", owner_cookie.clone())
        .await;
    assert_eq!(listing.status_code(), 200, "{}", listing.text());
    assert!(listing.text().contains("later-active-personal"));
    let profile = request
        .get("/households/persistence-fixture/profile")
        .add_header("cookie", owner_cookie.clone())
        .await;
    assert_eq!(profile.status_code(), 200, "{}", profile.text());
    assert!(profile.text().contains("10 recovery codes available"));
    let foreign_sign_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"foreign@example.test","password":"password"}))
        .await;
    assert_eq!(
        foreign_sign_in.status_code(),
        200,
        "{}",
        foreign_sign_in.text()
    );
    let foreign_cookie = foreign_sign_in
        .header("set-cookie")
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let foreign_listing = request
        .get("/account/security/keys")
        .add_header("cookie", foreign_cookie.clone())
        .await;
    assert_eq!(
        foreign_listing.status_code(),
        200,
        "{}",
        foreign_listing.text()
    );
    assert!(!foreign_listing.text().contains("later-active-personal"));
    let denied = request
        .post("/api/auth/security/api-key/revoke")
        .add_header("origin", boot.app_context.config.server.full_url())
        .add_header("cookie", foreign_cookie)
        .json(&json!({"key_id":"later-active-personal"}))
        .await;
    assert_eq!(denied.status_code(), 401, "{}", denied.text());
    let revoked = request
        .post("/api/auth/security/api-key/revoke")
        .add_header("origin", boot.app_context.config.server.full_url())
        .add_header("cookie", owner_cookie.clone())
        .json(&json!({"key_id":"later-active-personal"}))
        .await;
    assert_eq!(revoked.status_code(), 200, "{}", revoked.text());
    let listing = request
        .get("/account/security/keys")
        .add_header("cookie", owner_cookie)
        .await;
    assert_eq!(listing.status_code(), 200, "{}", listing.text());
    assert!(!listing.text().contains("later-active-personal"));
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_accepts_split_cookie_headers() {
    let (fixture, boot) = boot_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=now()").await.unwrap();
    let request = server(&boot);
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
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
    let ordinary = request
        .get("/api/auth/get-session")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(ordinary.json::<Value>()["user"]["id"], "71001");
    let anonymous = request.get("/api/auth/get-session").await;
    assert!(anonymous.json::<Value>().is_null());
    let refreshed = request
        .get("/api/auth/get-session")
        .add_header("cookie", cookie.clone())
        .add_header("cookie", "unrelated=1")
        .await;
    assert_eq!(refreshed.status_code(), 200, "{}", refreshed.text());
    assert_eq!(refreshed.json::<Value>()["user"]["id"], "71001");
    let reversed = request
        .get("/api/auth/get-session")
        .add_header("cookie", "unrelated=1")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(reversed.json::<Value>()["user"]["id"], "71001");
    let browser = request
        .get("/account/security")
        .add_header("cookie", cookie)
        .add_header("cookie", "unrelated=1")
        .await;
    assert_eq!(browser.status_code(), 200, "{}", browser.text());
    let malformed = axum::http::HeaderValue::from_bytes(b"strange=\xff").unwrap();
    let rejected = request
        .get("/api/auth/get-session")
        .add_header("cookie", malformed.clone())
        .await;
    assert_eq!(rejected.status_code(), 400);
    assert_eq!(rejected.header("cache-control"), "no-store");
    let rejected = request
        .get("/account/security")
        .add_header("cookie", malformed)
        .await;
    assert_eq!(rejected.status_code(), 400);
    assert_eq!(rejected.header("cache-control"), "no-store");
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_without_clinical_records_can_login_and_refresh() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73992,'Synthetic account identity','1990-01-01',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,name,password_hash,status,created_at,updated_at) VALUES(71002,73992,'account-auth-only@example.test','Canonical account name',crypt('password',gen_salt('bf',4)),2,now(),now()); INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71002,now())").await.unwrap();
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
    fixture.admin.execute_unprepared("INSERT INTO people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73992,'Synthetic account identity','1990-01-01',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,password_hash,status,created_at,updated_at) VALUES(71002,73992,'account-auth-only@example.test',crypt('password',gen_salt('bf',4)),2,now(),now()); CREATE FUNCTION reject_account_auth_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='AuthenticationToken' AND NEW.item_id=71002 THEN RAISE EXCEPTION 'synthetic authentication audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_account_auth_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_account_auth_audit()").await.unwrap();
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
    fixture.admin.execute_unprepared("INSERT INTO people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73992,'Synthetic account identity','1990-01-01',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,password_hash,status,created_at,updated_at) VALUES(71002,73992,'account-auth-only@example.test',crypt('password',gen_salt('bf',4)),1,now(),now()); INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71002,now())").await.unwrap();
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
    fixture.admin.execute_unprepared("INSERT INTO people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73992,'Synthetic account identity','1990-01-01',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,password_hash,status,created_at,updated_at) VALUES(71002,73992,'account-auth-only@example.test',crypt('password',gen_salt('bf',4)),2,now(),now()); INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71002,now()); INSERT INTO account_otp_keys(id,key,last_use,num_failures) VALUES(71002,'abcdefghijklmnop',now()-interval '1 day',0)").await.unwrap();
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
    let cookie_name = cookie.split('=').next().unwrap();
    let authenticated_cookie = verified
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|header| header.to_str().ok())
        .rfind(|header| header.split('=').next() == Some(cookie_name))
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    assert_ne!(authenticated_cookie, cookie);
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
