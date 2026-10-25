use super::*;
use base64::Engine as _;
use chrono::{Duration, Utc};
use med_tracker::models::_entities::{accounts, app_settings, identity_onboarding, people};
use med_tracker::models::entities::{
    identity_api_key as identity_api_keys, identity_passkey, identity_session,
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

fn is_auth_session_cookie(header: &str) -> bool {
    matches!(
        header.split('=').next(),
        Some("better-auth.session_token" | "__Secure-better-auth.session_token")
    )
}

async fn enable_completed_onboarding(fixture: &Fixture, account_id: i64) {
    let active = identity_onboarding::ActiveModel {
        account_id: Set(account_id),
        completed_at: Set(Some(Utc::now().fixed_offset())),
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
    id: i64,
    name: &str,
    config_id: &str,
    created_at: chrono::DateTime<Utc>,
    expires_at: chrono::DateTime<Utc>,
    enabled: bool,
    remaining: f64,
) -> identity_api_keys::ActiveModel {
    identity_api_keys::ActiveModel {
        id: Set(id),
        reference_id: Set(71001),
        name: Set(Some(name.into())),
        config_id: Set(config_id.into()),
        key_hash: Set(format!("synthetic-inventory-{id}")),
        enabled: Set(enabled),
        remaining: Set(Some(remaining)),
        rate_limit_enabled: Set(true),
        rate_limit_time_window: Set(Some(60_000.0)),
        rate_limit_max: Set(Some(120.0)),
        request_count: Set(Some(0.0)),
        created_at: Set(created_at),
        updated_at: Set(created_at),
        expires_at: Set(Some(expires_at)),
        ..Default::default()
    }
}

async fn authentication_only_account(fixture: &Fixture, name: Option<&str>, status: i32) {
    fixture.admin.execute_unprepared("INSERT INTO people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73992,'Synthetic account identity','1990-01-01',0,true,now(),now())").await.unwrap();
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO accounts(id,person_id,email,name,status,email_verified,created_at,updated_at) VALUES(71002,73992,'account-auth-only@example.test',$1,$2,true,now(),now())",[name.map(str::to_owned).into(),status.into()])).await.unwrap();
    fixture.admin.execute_unprepared("INSERT INTO auth_credentials(account_id,provider_id,user_id,password,created_at,updated_at) SELECT '71002','credential',71002,password,now(),now() FROM auth_credentials WHERE user_id=71001 AND provider_id='credential'; INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71002,now())").await.unwrap();
}

#[tokio::test]
async fn account_authentication_password_signup_verifies_email_and_retains_first_login() {
    let (fixture, boot) = boot_fixture().await;
    app_settings::ActiveModel {
        invite_only: Set(false),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    let email = "password-confirmation@example.test";
    let password = "Verification-passphrase-42";
    let mut request = server(&boot);
    request.save_cookies();
    let signup = request
        .post("/api/auth/onboarding/signup")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"name":"Password confirmation owner","email":email,"date_of_birth":"1990-04-12","credential":"password","password":password}))
        .await;
    assert_eq!(signup.status_code(), 200, "{}", signup.text());
    let pending = fixture.admin.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT a.id,a.status,p.id AS credential_id,p.password AS credential_password,(SELECT count(*) FROM pg_loco_queue q WHERE q.task_data->>'to'=$1 AND q.task_data->>'subject'='Verify your account') AS mail_count FROM accounts a JOIN auth_credentials p ON p.user_id=a.id AND p.provider_id='credential' WHERE a.email=$1",
        [email.into()],
    )).await.unwrap().unwrap();
    let account_id: i64 = pending.try_get("", "id").unwrap();
    let credential_id: String = pending.try_get("", "credential_id").unwrap();
    let credential_password: String = pending.try_get("", "credential_password").unwrap();
    assert_eq!(pending.try_get::<i32>("", "status").unwrap(), 1);
    assert_eq!(pending.try_get::<i64>("", "mail_count").unwrap(), 1);
    assert!(!credential_password.is_empty());
    let mail = fixture.admin.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT task_data FROM pg_loco_queue WHERE task_data->>'to'=$1 AND task_data->>'subject'='Verify your account' ORDER BY created_at DESC LIMIT 1",
        [email.into()],
    )).await.unwrap().unwrap();
    let mail: Value = mail.try_get("", "task_data").unwrap();
    let link = url::Url::parse(
        mail["text"]
            .as_str()
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(link.path(), "/verify-account-confirm");
    let token = link
        .query_pairs()
        .find(|(name, _)| name == "token")
        .map(|(_, value)| value.into_owned())
        .unwrap();
    let confirmation_page = request
        .get(&format!("{}?{}", link.path(), link.query().unwrap()))
        .await;
    assert_eq!(
        confirmation_page.status_code(),
        200,
        "{}",
        confirmation_page.text()
    );
    let csrf = confirmation_page
        .text()
        .split("name=\"authenticity_token\" value=\"")
        .nth(1)
        .and_then(|value| value.split('"').next())
        .unwrap()
        .to_owned();
    let tampered = request
        .post("/verify-account-confirm")
        .add_header("origin", boot.app_context.config.server.full_url())
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("token", format!("invalid{token}").as_str()),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert_eq!(tampered.status_code(), 303, "{}", tampered.text());
    assert_eq!(tampered.header("location"), "/login");
    assert!(
        !tampered
            .headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|header| header.to_str().ok())
            .any(is_auth_session_cookie)
    );
    let pending = accounts::Entity::find_by_id(account_id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pending.status, 1);
    let pending_sessions = fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS sessions FROM identity_sessions WHERE account_id=$1",
            [account_id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pending_sessions.try_get::<i64>("", "sessions").unwrap(), 0);
    let confirmed = request
        .post("/verify-account-confirm")
        .add_header("origin", boot.app_context.config.server.full_url())
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("token", token.as_str()),
            ("callbackURL", "/auth/passkey/setup"),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert_eq!(confirmed.status_code(), 302, "{}", confirmed.text());
    assert_eq!(confirmed.header("location"), "/auth/passkey/setup");
    let session_cookie = confirmed
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|header| header.to_str().ok())
        .rfind(|header| is_auth_session_cookie(header))
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let native_session = request
        .get("/api/auth/get-session")
        .add_header("cookie", session_cookie.clone())
        .await;
    assert_eq!(
        native_session.status_code(),
        200,
        "{}",
        native_session.text()
    );
    assert_eq!(
        native_session.json::<Value>()["session"]["clinical_session_purpose"],
        "enrolment"
    );
    let setup = request
        .get("/auth/passkey/setup")
        .add_header("cookie", session_cookie)
        .await;
    assert_eq!(setup.status_code(), 200, "{}", setup.text());
    assert!(setup.text().contains("Your sign-in method is ready"));
    let replay = request
        .post("/verify-account-confirm")
        .add_header("origin", boot.app_context.config.server.full_url())
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("token", token.as_str()),
            ("callbackURL", "/auth/passkey/setup"),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert_eq!(replay.status_code(), 303, "{}", replay.text());
    assert_eq!(replay.header("location"), "/login");
    assert!(
        !replay
            .headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|header| header.to_str().ok())
            .any(is_auth_session_cookie)
    );
    let verified = fixture.admin.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT a.status,p.password,(SELECT count(*) FROM versions v WHERE v.item_type='NativeAuthentication' AND v.item_id=a.id AND v.event='native_auth/account/updated') AS audits,(SELECT count(*) FROM pg_loco_queue q WHERE q.task_data->>'to'=a.email::text AND q.task_data->>'subject'='Verify your account') AS mail_count,(SELECT count(*) FROM identity_sessions s WHERE s.account_id=a.id AND s.active AND s.purpose='enrolment') AS sessions FROM accounts a JOIN auth_credentials p ON p.user_id=a.id AND p.id=$2 WHERE a.id=$1",
        [account_id.into(), credential_id.into()],
    )).await.unwrap().unwrap();
    assert_eq!(verified.try_get::<i32>("", "status").unwrap(), 2);
    assert_eq!(
        verified.try_get::<String>("", "password").unwrap(),
        credential_password
    );
    assert_eq!(verified.try_get::<i64>("", "audits").unwrap(), 1);
    assert_eq!(verified.try_get::<i64>("", "mail_count").unwrap(), 1);
    assert_eq!(verified.try_get::<i64>("", "sessions").unwrap(), 1);
    let fresh = server(&boot);
    let signed_in = fresh
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":email,"password":password}))
        .await;
    assert_eq!(signed_in.status_code(), 200, "{}", signed_in.text());
    assert_eq!(
        signed_in.json::<Value>()["user"]["id"],
        account_id.to_string()
    );
    let denied = fresh
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":email,"password":"wrong-password"}))
        .await;
    assert_eq!(denied.status_code(), 401, "{}", denied.text());
    drop(fresh);
    drop(request);
    close(fixture, boot).await;
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
        date_of_birth: Set(Some(chrono::NaiveDate::from_ymd_opt(1990, 1, 1).unwrap())),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    accounts::ActiveModel {
        id: Set(71002),
        person_id: Set(73992),
        email: Set("foreign@example.test".into()),
        status: Set(2),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    fixture.admin.execute_unprepared("UPDATE accounts SET email_verified=true WHERE id=71002; INSERT INTO auth_credentials(account_id,provider_id,user_id,password,created_at,updated_at) SELECT '71002','credential',71002,password,now(),now() FROM auth_credentials WHERE user_id=71001 AND provider_id='credential'").await.unwrap();
    enable_completed_onboarding(&fixture, 71001).await;
    enable_completed_onboarding(&fixture, 71002).await;
    let now = Utc::now();
    let mut keys = (0..101)
        .map(|index| {
            let created = now - Duration::days(2) + Duration::seconds(index);
            let expired = index % 3 == 0;
            inventory_key(
                90000 + index,
                &format!("earlier-{index:03}"),
                if index % 2 == 0 {
                    "medtracker-personal"
                } else {
                    "unrelated-plugin"
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
        90101,
        "later-active-personal",
        "medtracker-personal",
        now,
        now + Duration::days(1),
        true,
        1.0,
    ));
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
    assert!(profile.text().contains("href=\"/account/security\""));
    assert!(
        profile
            .text()
            .contains("href=\"/account/security/passkey\"")
    );
    assert!(!profile.text().contains("regenerate_recovery"));
    let center = request
        .get("/account/security")
        .add_header("cookie", owner_cookie.clone())
        .await;
    assert_eq!(center.status_code(), 200);
    assert!(center.text().contains("Add passkey"));
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
        .json(&json!({"key_id":"90101"}))
        .await;
    assert_eq!(denied.status_code(), 401, "{}", denied.text());
    let revoked = request
        .post("/api/auth/security/api-key/revoke")
        .add_header("origin", boot.app_context.config.server.full_url())
        .add_header("cookie", owner_cookie.clone())
        .json(&json!({"key_id":"90101"}))
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
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET completed_at=now()").await.unwrap();
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
    authentication_only_account(&fixture, Some("Canonical account name"), 2).await;
    let request = server(&boot);
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"account-auth-only@example.test","password":"password","active_household_id":72001}))
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
    let untouched = identity_session::Entity::find()
        .all(&fixture.admin)
        .await
        .unwrap();
    assert!(
        untouched
            .iter()
            .filter(|session| session.user_id == 71002)
            .all(|session| session.active_household_id.is_none())
    );
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
    let registration = request
        .get("/api/auth/passkey/generate-register-options")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(registration.status_code(), 403, "{}", registration.text());
    assert!(registration.text().contains("SESSION_NOT_FRESH"));
    let fresh = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"account-auth-only@example.test","password":"password"}))
        .await;
    assert_eq!(fresh.status_code(), 200);
    let fresh_cookie = fresh
        .header("set-cookie")
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let registration = request
        .get("/api/auth/passkey/generate-register-options")
        .add_header("cookie", fresh_cookie)
        .await;
    assert_eq!(registration.status_code(), 200, "{}", registration.text());
    let clinical = request
        .get("/households/persistence-fixture/profile")
        .add_header("cookie", cookie)
        .await;
    assert_eq!(clinical.status_code(), 403, "{}", clinical.text());
    let audit = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS events,bool_and(whodunnit IS NULL AND audit_context->>'actor_account_id' IS NULL AND audit_context->>'actor_user_id' IS NULL) AS account_only FROM versions WHERE item_type='NativeAuthentication' AND item_id=71002")).await.unwrap().unwrap();
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
async fn account_authentication_locked_session_cannot_authenticate_or_change_credentials() {
    let (fixture, boot) = boot_fixture().await;
    fixture.admin.execute_unprepared("UPDATE users SET active=false; INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET completed_at=now()").await.unwrap();
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
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(rejected.status_code(), 401, "{}", rejected.text());
    let registration = request
        .get("/api/auth/passkey/generate-register-options")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(registration.status_code(), 401);
    let clinical = request
        .get("/households/persistence-fixture/profile")
        .add_header("cookie", cookie)
        .await;
    assert_eq!(clinical.status_code(), 303);
    assert!(
        clinical
            .header("location")
            .to_str()
            .unwrap()
            .starts_with("/login")
    );
    let state = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM identity_sessions WHERE account_id=71001) AS sessions,(SELECT count(*) FROM identity_passkeys WHERE account_id=71001) AS passkeys")).await.unwrap().unwrap();
    assert_eq!(state.try_get::<i64>("", "sessions").unwrap(), 1);
    assert_eq!(state.try_get::<i64>("", "passkeys").unwrap(), 0);
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
    authentication_only_account(&fixture, None, 2).await;
    fixture.admin.execute_unprepared("CREATE FUNCTION reject_account_auth_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='NativeAuthentication' AND NEW.event='native_auth/session/created' AND NEW.item_id=71002 THEN RAISE EXCEPTION 'synthetic authentication audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_account_auth_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_account_auth_audit()").await.unwrap();
    let request = server(&boot);
    let rejected = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"account-auth-only@example.test","password":"password"}))
        .await;
    assert_eq!(rejected.status_code(), 500);
    let state = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM identity_sessions WHERE account_id=71002) AS sessions,(SELECT count(*) FROM versions WHERE item_type='NativeAuthentication' AND event='native_auth/session/created' AND item_id=71002) AS audits")).await.unwrap().unwrap();
    assert_eq!(state.try_get::<i64>("", "sessions").unwrap(), 0);
    assert_eq!(state.try_get::<i64>("", "audits").unwrap(), 0);
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_account_status_and_session_lifetime_remain_required() {
    let (fixture, boot) = boot_fixture().await;
    authentication_only_account(&fixture, None, 1).await;
    let request = server(&boot);
    for (status, verified) in [(1, false), (3, true)] {
        fixture
            .admin
            .execute_unprepared(&format!(
                "UPDATE accounts SET email_verified={verified} WHERE id=71002; UPDATE accounts SET status={status} WHERE id=71002"
            ))
            .await
            .unwrap();
        let stored = fixture
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT status,email_verified FROM accounts WHERE id=71002",
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.try_get::<i32>("", "status").unwrap(), status);
        assert_eq!(
            stored.try_get::<bool>("", "email_verified").unwrap(),
            verified
        );
        let rejected = request
            .post("/api/auth/sign-in/email")
            .add_header("origin", boot.app_context.config.server.full_url())
            .json(&json!({"email":"account-auth-only@example.test","password":"password"}))
            .await;
        assert!(
            rejected.status_code().is_client_error(),
            "status {status}: {}",
            rejected.status_code()
        );
        let state = fixture
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT count(*) AS sessions FROM identity_sessions WHERE account_id=71002",
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(state.try_get::<i64>("", "sessions").unwrap(), 0);
    }
    fixture
        .admin
        .execute_unprepared("UPDATE accounts SET status=2,email_verified=true WHERE id=71002")
        .await
        .unwrap();
    for mutation in [
        "active=false",
        "expires_at=now()-interval '1 second'",
        "created_at=now()-interval '31 days'",
    ] {
        let signed_in = request
            .post("/api/auth/sign-in/email")
            .add_header("origin", boot.app_context.config.server.full_url())
            .json(&json!({"email":"account-auth-only@example.test","password":"password"}))
            .await;
        assert_eq!(signed_in.status_code(), 200);
        let body: Value = signed_in.json();
        assert!(body["user"]["name"].is_null());
        let session_id = body["session"]["id"]
            .as_i64()
            .map(|id| id.to_string())
            .or_else(|| body["session"]["id"].as_str().map(str::to_owned));
        let cookie = signed_in
            .header("set-cookie")
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        let valid = request
            .get("/api/auth/get-session")
            .add_header("cookie", cookie.clone())
            .await;
        assert_eq!(valid.status_code(), 200);
        let valid_body: Value = valid.json();
        assert!(valid_body["session"].is_object());
        let id = session_id.unwrap_or_else(|| {
            valid_body["session"]["id"]
                .to_string()
                .trim_matches('"')
                .to_owned()
        });
        let updated = fixture
            .admin
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                format!("UPDATE identity_sessions SET {mutation} WHERE id=$1"),
                [id.parse::<i64>().unwrap().into()],
            ))
            .await
            .unwrap();
        assert_eq!(updated.rows_affected(), 1);
        let rejected = request
            .get("/api/auth/get-session")
            .add_header("cookie", cookie.clone())
            .await;
        assert!(
            rejected.status_code().is_client_error()
                || (rejected.status_code() == 200 && rejected.json::<Value>().is_null()),
            "{mutation}: {}",
            rejected.status_code()
        );
        let security = request
            .get("/account/security")
            .add_header("cookie", cookie)
            .await;
        assert_ne!(security.status_code(), 200, "{mutation}");
    }
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_session_revocation_clears_only_current_cookie_family() {
    let (fixture, boot) = boot_fixture().await;
    enable_completed_onboarding(&fixture, 71001).await;
    let request = server(&boot);
    let mut sessions = Vec::new();
    for _ in 0..2 {
        let login = request
            .post("/api/auth/sign-in/email")
            .add_header("origin", boot.app_context.config.server.full_url())
            .json(&json!({"email":"persistence@example.test","password":"password"}))
            .await;
        assert_eq!(login.status_code(), 200, "{}", login.text());
        let cookie = login
            .headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .find(|value| is_auth_session_cookie(value))
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        let session = request
            .get("/api/auth/get-session")
            .add_header("cookie", cookie.clone())
            .await;
        assert_eq!(session.status_code(), 200, "{}", session.text());
        sessions.push((
            cookie,
            session.json::<Value>()["session"]["id"]
                .as_str()
                .unwrap()
                .to_owned(),
        ));
    }
    let prefix = sessions[0]
        .0
        .split('=')
        .next()
        .unwrap()
        .strip_suffix("session_token")
        .unwrap();
    let cookie = format!(
        "{}; {prefix}session_data.0=stale; {prefix}session_data.1=stale; {prefix}dont_remember=true",
        sessions[0].0
    );
    fixture.admin.execute_unprepared("CREATE FUNCTION reject_cookie_revoke_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='NativeAuthentication' AND NEW.event='native_auth/session/deleted' THEN RAISE EXCEPTION 'synthetic revoke audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_cookie_revoke_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_cookie_revoke_audit()").await.unwrap();
    let rejected = request
        .post("/api/auth/security/session/revoke")
        .add_header("origin", boot.app_context.config.server.full_url())
        .add_header("cookie", cookie.clone())
        .json(&json!({"session_id":sessions[0].1}))
        .await;
    assert_eq!(rejected.status_code(), 500, "{}", rejected.text());
    assert!(
        !rejected
            .headers()
            .get_all("set-cookie")
            .iter()
            .any(|value| value
                .to_str()
                .is_ok_and(|value| is_auth_session_cookie(value) && value.contains("Max-Age=0")))
    );
    let active = request
        .get("/api/auth/get-session")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(active.status_code(), 200, "{}", active.text());
    fixture.admin.execute_unprepared("DROP TRIGGER reject_cookie_revoke_audit ON versions; DROP FUNCTION reject_cookie_revoke_audit()").await.unwrap();
    for (target, current) in [(&sessions[1].1, false), (&sessions[0].1, true)] {
        let operation_cookie = if current { &cookie } else { &sessions[0].0 };
        let response = request
            .post("/api/auth/security/session/revoke")
            .add_header("origin", boot.app_context.config.server.full_url())
            .add_header("cookie", operation_cookie.clone())
            .json(&json!({"session_id":target}))
            .await;
        assert_eq!(response.status_code(), 200, "{}", response.text());
        assert_eq!(response.json::<Value>()["current"], current);
        let cleared: Vec<_> = response
            .headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .filter(|value| value.contains("Max-Age=0"))
            .collect();
        if current {
            for name in [
                "session_token",
                "session_data",
                "session_data.0",
                "session_data.1",
                "dont_remember",
            ] {
                assert!(
                    cleared
                        .iter()
                        .any(|value| value.starts_with(&format!("{prefix}{name}="))),
                    "missing {name}: {cleared:?}"
                );
            }
        } else {
            assert!(
                !cleared.iter().any(|value| is_auth_session_cookie(value)),
                "other session revoke cleared current credential: {cleared:?}"
            );
        }
        let active = request
            .get("/api/auth/get-session")
            .add_header("cookie", cookie.clone())
            .await;
        assert_eq!(active.status_code(), 200, "{}", active.text());
        if current {
            assert!(active.json::<Value>().is_null(), "{}", active.text());
            let rejected = request
                .post("/api/auth/security/session/revoke")
                .add_header("origin", boot.app_context.config.server.full_url())
                .add_header("cookie", sessions[0].0.clone())
                .json(&json!({"session_id":target}))
                .await;
            assert_eq!(rejected.status_code(), 401, "{}", rejected.text());
        } else {
            assert_eq!(active.json::<Value>()["session"]["id"], sessions[0].1);
        }
    }
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_throttle_retains_body_and_retry_after() {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    for attempt in 0..11 {
        let response = request
            .post("/api/auth/sign-in/email")
            .add_header("origin", boot.app_context.config.server.full_url())
            .json(&json!({"email":"missing-throttled@example.test","password":"password"}))
            .await;
        if attempt < 10 {
            assert_eq!(response.status_code(), 401, "{}", response.text());
        } else {
            assert_eq!(response.status_code(), 429, "{}", response.text());
            assert_eq!(
                response.json::<Value>(),
                json!({"error":"Too many authentication attempts. Try again later."})
            );
            let retry_after: u32 = response
                .header("retry-after")
                .to_str()
                .unwrap()
                .parse()
                .unwrap();
            assert!((1..=900).contains(&retry_after));
        }
    }
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_native_signup_uses_canonical_account_and_native_credentials() {
    let mut logger = loco_rs::config::Config::new(&loco_rs::environment::Environment::Test)
        .unwrap()
        .logger;
    logger.enable = true;
    logger.level = loco_rs::logger::LogLevel::Error;
    logger.override_filter = Some("error".into());
    loco_rs::logger::init::<med_tracker::app::App>(&logger).unwrap();
    let (fixture, boot) = boot_fixture().await;
    app_settings::ActiveModel {
        invite_only: Set(false),
        created_at: Set(Utc::now().naive_utc()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    let mut request = server(&boot);
    request.save_cookies();
    let response = request
        .post("/api/auth/sign-up/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({
            "name":"Native account owner", "email":"native-account@example.test",
            "password":"Native-account-passphrase-42",
            "metadata":{"date_of_birth":"1990-04-12"}
        }))
        .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT a.id,a.person_id,(SELECT count(*) FROM people p WHERE p.id=a.person_id) AS people,(SELECT count(*) FROM auth_credentials c WHERE c.user_id=a.id AND c.provider_id='credential' AND c.password IS NOT NULL) AS credentials,(SELECT count(*) FROM users u WHERE u.person_id=a.person_id) AS clinical_profiles,(SELECT count(*) FROM pg_loco_queue q WHERE q.task_data->>'to'=a.email::text AND q.task_data->>'subject'='Verify your account') AS mails FROM accounts a WHERE a.email='native-account@example.test'"
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "people").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "credentials").unwrap(), 1);
    assert_eq!(row.try_get::<i64>("", "clinical_profiles").unwrap(), 1);
    let metadata = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT metadata FROM accounts WHERE email='native-account@example.test'",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<Value>("", "metadata")
        .unwrap();
    assert_eq!(metadata, json!({}));
    assert_eq!(row.try_get::<i64>("", "mails").unwrap(), 1);
    let mail = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT task_data FROM pg_loco_queue WHERE task_data->>'to'='native-account@example.test' AND task_data->>'subject'='Verify your account'")).await.unwrap().unwrap();
    let mail: Value = mail.try_get("", "task_data").unwrap();
    let link = url::Url::parse(
        mail["text"]
            .as_str()
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(link.path(), "/verify-account-confirm");
    let token = link
        .query_pairs()
        .find(|(name, _)| name == "token")
        .unwrap()
        .1
        .into_owned();
    let page = request
        .get(&format!("{}?{}", link.path(), link.query().unwrap()))
        .await;
    assert_eq!(page.status_code(), 200, "{}", page.text());
    let csrf = page
        .text()
        .split("name=\"authenticity_token\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_owned();
    let confirmed = request
        .post("/verify-account-confirm")
        .add_header("origin", boot.app_context.config.server.full_url())
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("token", token.as_str()),
            ("authenticity_token", csrf.as_str()),
        ])
        .await;
    assert_eq!(confirmed.status_code(), 200, "{}", confirmed.text());
    assert_eq!(confirmed.json::<Value>()["status"], true);
    let cookie = confirmed
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|header| header.to_str().ok())
        .find(|header| is_auth_session_cookie(header))
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let pending = request
        .get("/api/auth/get-session")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(pending.status_code(), 200, "{}", pending.text());
    assert_eq!(
        pending.json::<Value>()["session"]["clinical_session_purpose"],
        "enrolment"
    );
    let denied = request
        .post("/api/auth/two-factor/generate-backup-codes")
        .add_header("origin", boot.app_context.config.server.full_url())
        .add_header("cookie", cookie.clone())
        .json(&json!({"password":"Native-account-passphrase-42"}))
        .await;
    assert_eq!(denied.status_code(), 403, "{}", denied.text());
    let completed = request
        .post("/api/auth/onboarding/complete")
        .add_header("origin", boot.app_context.config.server.full_url())
        .add_header("cookie", cookie.clone())
        .json(&json!({}))
        .await;
    assert_eq!(completed.status_code(), 200, "{}", completed.text());
    let home = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT h.id,h.slug FROM households h JOIN household_memberships m ON m.household_id=h.id JOIN accounts a ON a.person_id=m.person_id WHERE a.email='native-account@example.test'")).await.unwrap().unwrap();
    let home_id: i64 = home.try_get("", "id").unwrap();
    let slug: String = home.try_get("", "slug").unwrap();
    let dashboard = request
        .get(&format!("/households/{slug}/dashboard"))
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(dashboard.status_code(), 200, "{}", dashboard.text());
    let care = request
        .get(&format!("/households/{slug}/medications"))
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(care.status_code(), 200, "{}", care.text());
    let api = request
        .get(&format!("/api/v1/households/{home_id}/profile"))
        .add_header("cookie", cookie)
        .await;
    assert_eq!(api.status_code(), 401, "{}", api.text());
    let login = request.post("/api/auth/sign-in/email").add_header("origin",boot.app_context.config.server.full_url()).json(&json!({"email":"NATIVE-ACCOUNT@EXAMPLE.TEST","password":"Native-account-passphrase-42"})).await;
    assert_eq!(login.status_code(), 200, "{}", login.text());
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_security_inventory_is_complete_live_and_account_scoped() {
    let (fixture, boot) = boot_fixture().await;
    authentication_only_account(&fixture, Some("Other inventory owner"), 2).await;
    enable_completed_onboarding(&fixture, 71001).await;
    let now = Utc::now();
    let sessions = (0..203)
        .map(|index| identity_session::ActiveModel {
            id: Set(92000 + index),
            user_id: Set(if index == 202 { 71002 } else { 71001 }),
            token: Set(format!("inventory-secret-{index}")),
            created_at: Set(now - Duration::days(1) + Duration::seconds(index)),
            updated_at: Set(now),
            expires_at: Set(if index < 100 {
                now - Duration::hours(1)
            } else {
                now + Duration::days(1)
            }),
            active: Set(true),
            clinical_session_purpose: Set("authenticated".into()),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    identity_session::Entity::insert_many(sessions)
        .exec(&fixture.admin)
        .await
        .unwrap();
    let passkeys = (0..103)
        .map(|index| identity_passkey::ActiveModel {
            id: Set(93000 + index),
            user_id: Set(if index == 102 { 71002 } else { 71001 }),
            name: Set(Some(format!("inventory-passkey-{index}"))),
            public_key: Set("inventory-only".into()),
            credential_id: Set(format!("inventory-credential-{index}")),
            counter: Set(0),
            device_type: Set("singleDevice".into()),
            backed_up: Set(false),
            credential: Set("{}".into()),
            created_at: Set(now - Duration::days(1) + Duration::seconds(index)),
            updated_at: Set(now),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    identity_passkey::Entity::insert_many(passkeys)
        .exec(&fixture.admin)
        .await
        .unwrap();
    let request = server(&boot);
    let login = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(login.status_code(), 200, "{}", login.text());
    let cookie = login
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|header| header.to_str().ok())
        .find(|header| is_auth_session_cookie(header))
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let listing = request
        .get("/account/security")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(listing.status_code(), 200, "{}", listing.text());
    let html = listing.text();
    assert!(
        html.contains("Current session"),
        "current session hidden by older rows"
    );
    assert_eq!(
        html.matches("name=\"session_id\"").count(),
        102,
        "live session inventory must be complete"
    );
    assert_eq!(
        html.matches("data-passkey-id=").count(),
        102,
        "passkey inventory must be complete"
    );
    for id in [92000, 92099, 92202] {
        assert!(!html.contains(&format!("name=\"session_id\" value=\"{id}\"")));
    }
    assert!(!html.contains("inventory-passkey-102"));
    assert!(!html.contains("inventory-secret-"));
    assert!(html.contains("inventory-passkey-101"));
    let revoked = request
        .post("/api/auth/security/session/revoke")
        .add_header("origin", boot.app_context.config.server.full_url())
        .add_header("cookie", cookie.clone())
        .json(&json!({"session_id":"92201"}))
        .await;
    assert_eq!(revoked.status_code(), 200, "{}", revoked.text());
    let removed = request
        .post("/api/auth/passkey/delete-passkey")
        .add_header("origin", boot.app_context.config.server.full_url())
        .add_header("cookie", cookie.clone())
        .json(&json!({"id":"93101"}))
        .await;
    assert_eq!(removed.status_code(), 200, "{}", removed.text());
    let listing = request
        .get("/account/security")
        .add_header("cookie", cookie)
        .await;
    assert_eq!(listing.status_code(), 200, "{}", listing.text());
    assert_eq!(listing.text().matches("name=\"session_id\"").count(), 101);
    assert_eq!(listing.text().matches("data-passkey-id=").count(), 101);
    assert!(!listing.text().contains("inventory-passkey-101"));
    assert!(
        identity_session::Entity::find_by_id(92202)
            .one(&fixture.admin)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        identity_passkey::Entity::find_by_id(93102)
            .one(&fixture.admin)
            .await
            .unwrap()
            .is_some()
    );
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn malformed_auth_entry_links_offer_safe_recovery() {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    for (path, recovery) in [
        ("/reset-password/confirm", "/reset-password"),
        ("/verify-account-confirm", "/verify-account-resend"),
    ] {
        for query in ["", "?callbackURL=private-callback-marker"] {
            let response = request.get(&format!("{path}{query}")).await;
            assert_eq!(response.status_code(), 400);
            let html = response.text();
            assert!(html.contains("Incomplete link"), "{html}");
            assert!(html.contains(&format!("href=\"{recovery}\"")), "{html}");
            assert!(html.contains("href=\"/login\""));
            assert!(
                response
                    .header("content-type")
                    .to_str()
                    .unwrap()
                    .starts_with("text/html")
            );
            assert_eq!(response.header("cache-control"), "no-store");
            assert_eq!(response.header("referrer-policy"), "strict-origin");
            for forbidden in [
                "missing field",
                "Failed to deserialize",
                "private-callback-marker",
                "name=\"token\"",
                "<form",
            ] {
                assert!(!html.contains(forbidden), "{forbidden}: {html}");
            }
        }
        let empty = request.get(&format!("{path}?token=")).await;
        assert_eq!(empty.status_code(), 200);
        let oversized = request
            .get(&format!("{path}?token={}", "a".repeat(4097)))
            .await;
        assert_eq!(
            oversized.status_code(),
            if path.starts_with("/reset") { 400 } else { 303 }
        );
        let welsh = request.get(path).add_header("accept-language", "cy").await;
        assert_eq!(welsh.status_code(), 400);
        assert!(welsh.text().contains("Dolen anghyflawn"));
    }
    drop(request);
    close(fixture, boot).await;
}
