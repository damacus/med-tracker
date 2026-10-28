use super::*;
use base64::Engine as _;
use chrono::{Duration, Utc};
use med_tracker::models::_entities::{
    account_lockouts, household_memberships, households, oauth_grants, versions,
};
use med_tracker::models::_entities::{accounts, app_settings, identity_onboarding, people};
use med_tracker::models::entities::{
    identity_api_key as identity_api_keys, identity_passkey, identity_session,
};
use sea_orm::sea_query::{Expr, Query};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter, Set,
};

fn is_auth_session_cookie(header: &str) -> bool {
    matches!(
        header.split('=').next(),
        Some("better-auth.session_token" | "__Secure-better-auth.session_token")
    )
}

async fn enable_completed_onboarding(fixture: &RequestFixture, account_id: i64) {
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
    account_id: i64,
    name: &str,
    config_id: &str,
    created_at: chrono::DateTime<Utc>,
    expires_at: chrono::DateTime<Utc>,
    enabled: bool,
    remaining: f64,
) -> identity_api_keys::ActiveModel {
    identity_api_keys::ActiveModel {
        reference_id: Set(account_id),
        name: Set(Some(name.into())),
        config_id: Set(config_id.into()),
        key_hash: Set(format!("synthetic-inventory-{}", uuid::Uuid::new_v4())),
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

async fn native_account_fields(fixture: &RequestFixture, id: i64) -> sea_orm::QueryResult {
    let query = {
        use sea_orm::sea_query::ExprTrait;
        Query::select()
            .columns(["status", "email_verified", "metadata"])
            .from("accounts")
            .and_where(Expr::col("id").eq(id))
            .to_owned()
    };
    fixture
        .admin
        .query_one_raw(fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
}

async fn verification_mails(fixture: &RequestFixture, email: &str) -> Vec<Value> {
    let query = {
        use sea_orm::sea_query::{ExprTrait, extension::postgres::PgExpr};
        Query::select()
            .column("task_data")
            .from("pg_loco_queue")
            .and_where(PgExpr::cast_json_field(Expr::col("task_data"), "to").eq(email))
            .and_where(
                PgExpr::cast_json_field(Expr::col("task_data"), "subject")
                    .eq("Verify your account"),
            )
            .order_by("created_at", sea_orm::sea_query::Order::Desc)
            .to_owned()
    };
    fixture
        .admin
        .query_all_raw(fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.try_get("", "task_data").unwrap())
        .collect()
}

async fn native_credential(fixture: &RequestFixture, account_id: i64) -> (String, String) {
    let query = {
        use sea_orm::sea_query::ExprTrait;
        Query::select()
            .columns(["id", "password"])
            .from("auth_credentials")
            .and_where(Expr::col("user_id").eq(account_id))
            .and_where(Expr::col("provider_id").eq("credential"))
            .to_owned()
    };
    let row = fixture
        .admin
        .query_one_raw(fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap();
    (
        row.try_get("", "id").unwrap(),
        row.try_get("", "password").unwrap(),
    )
}

async fn account_status(fixture: &RequestFixture, id: i64, status: i32, verified: bool) {
    for (column, value) in [
        ("email_verified", Expr::val(verified)),
        ("status", Expr::val(status)),
    ] {
        let query = {
            use sea_orm::sea_query::ExprTrait;
            Query::update()
                .table("accounts")
                .value(column, value)
                .and_where(Expr::col("id").eq(id))
                .to_owned()
        };
        fixture
            .admin
            .execute_raw(fixture.admin.get_database_backend().build(&query))
            .await
            .unwrap();
    }
}

async fn authentication_only_account(fixture: &RequestFixture, name: Option<&str>, status: i32) {
    household_memberships::Entity::delete_by_id(fixture.foreign.membership.id)
        .exec(&fixture.admin)
        .await
        .unwrap();
    let mut account = fixture.foreign.account.clone().into_active_model();
    account.name = Set(name.map(str::to_owned));
    account.status = Set(status);
    account.update(&fixture.admin).await.unwrap();
}

pub(super) struct RegistrationSetting {
    previous: Option<app_settings::Model>,
    id: i64,
}

pub(super) async fn registration_setting(fixture: &RequestFixture) -> RegistrationSetting {
    let previous = app_settings::Entity::find()
        .one(&fixture.admin)
        .await
        .unwrap();
    let now = Utc::now().naive_utc();
    let setting = if let Some(previous) = previous.clone() {
        let mut setting = previous.into_active_model();
        setting.invite_only = Set(false);
        setting.update(&fixture.admin).await.unwrap()
    } else {
        app_settings::ActiveModel {
            invite_only: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&fixture.admin)
        .await
        .unwrap()
    };
    RegistrationSetting {
        previous,
        id: setting.id,
    }
}

pub(super) async fn restore_registration(fixture: &RequestFixture, saved: RegistrationSetting) {
    if let Some(previous) = saved.previous {
        previous
            .into_active_model()
            .reset_all()
            .update(&fixture.admin)
            .await
            .unwrap();
    } else {
        app_settings::Entity::delete_by_id(saved.id)
            .exec(&fixture.admin)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn account_authentication_password_signup_verifies_email_and_retains_first_login() {
    let (fixture, boot) = boot_request_fixture().await;
    let previous_settings = registration_setting(&fixture).await;
    let email = format!(
        "{}@example.test",
        fixture.credential("password-confirmation")
    );
    let password = "Verification-passphrase-42";
    let mut request = server(&boot);
    request.save_cookies();
    let signup = request
        .post("/api/auth/onboarding/signup")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"name":"Password confirmation owner","email":email,"date_of_birth":"1990-04-12","credential":"password","password":password}))
        .await;
    assert_eq!(signup.status_code(), 200, "{}", signup.text());
    let pending = accounts::Entity::find()
        .filter(accounts::Column::Email.eq(&email))
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let account_id = pending.id;
    let (credential_id, credential_password) = native_credential(&fixture, account_id).await;
    assert_eq!(pending.status, 1);
    let mails = verification_mails(&fixture, &email).await;
    assert_eq!(mails.len(), 1);
    assert!(!credential_password.is_empty());
    let mail = &mails[0];
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
    let pending_sessions = identity_session::Entity::find()
        .filter(identity_session::Column::UserId.eq(account_id))
        .count(&fixture.admin)
        .await
        .unwrap();
    assert_eq!(pending_sessions, 0);
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
    let verified = accounts::Entity::find_by_id(account_id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(verified.status, 2);
    let (verified_credential_id, verified_password) = native_credential(&fixture, account_id).await;
    assert_eq!(verified_credential_id, credential_id);
    assert_eq!(verified_password, credential_password);
    let audits = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("NativeAuthentication"))
        .filter(versions::Column::ItemId.eq(account_id))
        .filter(versions::Column::Event.eq("native_auth/account/updated"))
        .count(&fixture.admin)
        .await
        .unwrap();
    assert_eq!(audits, 1);
    assert_eq!(verification_mails(&fixture, &email).await.len(), 1);
    assert_eq!(
        identity_session::Entity::find()
            .filter(identity_session::Column::UserId.eq(account_id))
            .filter(identity_session::Column::Active.eq(true))
            .filter(identity_session::Column::ClinicalSessionPurpose.eq("enrolment"))
            .count(&fixture.admin)
            .await
            .unwrap(),
        1
    );
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
    restore_registration(&fixture, previous_settings).await;
    drop(request);
    close_request(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_open_signup_without_invitation_persists_identity_and_mail() {
    let (fixture, boot) = boot_request_fixture().await;

    let previous_settings = registration_setting(&fixture).await;
    let settings = app_settings::Entity::find()
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let open_email = format!(
        "{}@example.test",
        fixture.credential("open-null-invitation")
    );
    let closed_email = format!(
        "{}@example.test",
        fixture.credential("closed-null-invitation")
    );
    let request = server(&boot);
    let signed_up = request
        .post("/api/auth/onboarding/signup")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"name":"Open registration owner","email":open_email,"date_of_birth":"1990-04-12","credential":"passkey"}))
        .await;
    assert_eq!(signed_up.status_code(), 200, "{}", signed_up.text());
    assert_eq!(signed_up.json::<Value>()["status"], true);
    assert_eq!(
        accounts::Entity::find()
            .filter(accounts::Column::Email.eq(&open_email))
            .filter(accounts::Column::Status.eq(1))
            .count(&fixture.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(verification_mails(&fixture, &open_email).await.len(), 1);
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
        .json(&json!({"name":"Closed registration","email":closed_email,"date_of_birth":"1990-04-12","credential":"passkey"}))
        .await;
    assert_eq!(closed.status_code(), 403, "{}", closed.text());
    assert_eq!(
        accounts::Entity::find()
            .filter(accounts::Column::Email.eq(&closed_email))
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    restore_registration(&fixture, previous_settings).await;
    drop(request);
    close_request(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_security_inventory_includes_later_credentials() {
    let (fixture, boot) = boot_request_fixture().await;

    enable_completed_onboarding(&fixture, fixture.account.id).await;
    enable_completed_onboarding(&fixture, fixture.foreign.account.id).await;
    let now = Utc::now();
    let mut keys = (0..101)
        .map(|index| {
            let created = now - Duration::days(2) + Duration::seconds(index);
            let expired = index % 3 == 0;
            inventory_key(
                fixture.account.id,
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
        fixture.account.id,
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
    let later_key = identity_api_keys::Entity::find()
        .filter(identity_api_keys::Column::ReferenceId.eq(fixture.account.id))
        .filter(identity_api_keys::Column::Name.eq("later-active-personal"))
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let request = server(&boot);
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":fixture.account.email,"password":"password"}))
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
        .get(&format!("/households/{}/profile", fixture.household.slug))
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
        .json(&json!({"email":fixture.foreign.account.email,"password":"password"}))
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
        .json(&json!({"key_id":later_key.id.to_string()}))
        .await;
    assert_eq!(denied.status_code(), 401, "{}", denied.text());
    let revoked = request
        .post("/api/auth/security/api-key/revoke")
        .add_header("origin", boot.app_context.config.server.full_url())
        .add_header("cookie", owner_cookie.clone())
        .json(&json!({"key_id":later_key.id.to_string()}))
        .await;
    assert_eq!(revoked.status_code(), 200, "{}", revoked.text());
    let listing = request
        .get("/account/security/keys")
        .add_header("cookie", owner_cookie)
        .await;
    assert_eq!(listing.status_code(), 200, "{}", listing.text());
    assert!(!listing.text().contains("later-active-personal"));
    drop(request);
    close_request(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_accepts_split_cookie_headers() {
    let (fixture, boot) = boot_request_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;

    let account_id = fixture.account.id;
    enable_completed_onboarding(&fixture, fixture.account.id).await;
    let request = server(&boot);
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":fixture.account.email,"password":"password"}))
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
    assert_eq!(
        ordinary.json::<Value>()["user"]["id"],
        account_id.to_string()
    );
    let anonymous = request.get("/api/auth/get-session").await;
    assert!(anonymous.json::<Value>().is_null());
    let refreshed = request
        .get("/api/auth/get-session")
        .add_header("cookie", cookie.clone())
        .add_header("cookie", "unrelated=1")
        .await;
    assert_eq!(refreshed.status_code(), 200, "{}", refreshed.text());
    assert_eq!(
        refreshed.json::<Value>()["user"]["id"],
        account_id.to_string()
    );
    let reversed = request
        .get("/api/auth/get-session")
        .add_header("cookie", "unrelated=1")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(
        reversed.json::<Value>()["user"]["id"],
        account_id.to_string()
    );
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
    close_request(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_without_clinical_records_can_login_and_refresh() {
    let (fixture, boot) = boot_request_fixture().await;
    let foreign_account_id = fixture.foreign.account.id;

    authentication_only_account(&fixture, Some("Canonical account name"), 2).await;
    let request = server(&boot);
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":fixture.foreign.account.email,"password":"password","active_household_id":fixture.household.id}))
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
            .filter(|session| session.user_id == foreign_account_id)
            .all(|session| session.active_household_id.is_none())
    );
    identity_session::Entity::update_many()
        .col_expr(
            identity_session::Column::CreatedAt,
            Expr::value(Utc::now() - Duration::days(2)),
        )
        .col_expr(
            identity_session::Column::UpdatedAt,
            Expr::value(Utc::now() - Duration::days(2)),
        )
        .col_expr(
            identity_session::Column::ExpiresAt,
            Expr::value(Utc::now() + Duration::hours(1)),
        )
        .filter(identity_session::Column::UserId.eq(fixture.foreign.account.id))
        .exec(&fixture.admin)
        .await
        .unwrap();
    let refreshed = request
        .get("/api/auth/get-session")
        .add_header("cookie", cookie.clone())
        .await;
    assert_eq!(refreshed.status_code(), 200, "{}", refreshed.text());
    assert_eq!(
        refreshed.json::<Value>()["user"]["name"],
        "Canonical account name"
    );
    let renewed = identity_session::Entity::find()
        .filter(identity_session::Column::UserId.eq(fixture.foreign.account.id))
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert!(renewed.expires_at > Utc::now() + Duration::hours(2));
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
        .json(&json!({"email":fixture.foreign.account.email,"password":"password"}))
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
        .get(&format!("/households/{}/profile", fixture.household.slug))
        .add_header("cookie", cookie)
        .await;
    assert_eq!(clinical.status_code(), 403, "{}", clinical.text());
    let audit = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("NativeAuthentication"))
        .filter(versions::Column::ItemId.eq(fixture.foreign.account.id))
        .all(&fixture.admin)
        .await
        .unwrap();
    assert!(!audit.is_empty());
    assert!(audit.iter().all(|row| row.whodunnit.is_none()
        && row.audit_context["actor_account_id"].is_null()
        && row.audit_context["actor_person_id"].is_null()));
    let token = fixture.token().await;
    oauth_grants::Entity::update_many()
        .col_expr(
            oauth_grants::Column::AccountId,
            Expr::value(fixture.foreign.account.id),
        )
        .filter(oauth_grants::Column::Id.eq(fixture.authorization.id))
        .exec(&fixture.admin)
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
        .get(&format!(
            "/api/v1/households/{}/people",
            fixture.household.id
        ))
        .add_header("authorization", format!("Bearer {token}"))
        .await;
    assert_eq!(clinical.status_code(), 403);
    drop(request);
    close_request(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_locked_session_cannot_authenticate_or_change_credentials() {
    let (fixture, boot) = boot_request_fixture().await;
    enable_completed_onboarding(&fixture, fixture.account.id).await;
    let request = server(&boot);
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":fixture.account.email,"password":"password"}))
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
    identity_session::Entity::update_many()
        .col_expr(
            identity_session::Column::CreatedAt,
            Expr::value(Utc::now() - Duration::days(2)),
        )
        .col_expr(
            identity_session::Column::UpdatedAt,
            Expr::value(Utc::now() - Duration::days(2)),
        )
        .col_expr(
            identity_session::Column::ExpiresAt,
            Expr::value(Utc::now() + Duration::hours(1)),
        )
        .filter(identity_session::Column::UserId.eq(fixture.account.id))
        .exec(&fixture.admin)
        .await
        .unwrap();
    account_lockouts::ActiveModel {
        account_id: Set(fixture.account.id),
        key: Set(fixture.credential("synthetic-account-auth-lock")),
        deadline: Set((Utc::now() + Duration::hours(1)).naive_utc()),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
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
        .get(&format!("/households/{}/profile", fixture.household.slug))
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
    assert_eq!(
        identity_session::Entity::find()
            .filter(identity_session::Column::UserId.eq(fixture.account.id))
            .count(&fixture.admin)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        identity_passkey::Entity::find()
            .filter(identity_passkey::Column::UserId.eq(fixture.account.id))
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    let login = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":fixture.account.email,"password":"password"}))
        .await;
    assert_eq!(login.status_code(), 401, "{}", login.text());
    drop(request);
    close_request(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_audit_failure_rolls_back_session() {
    let (fixture, boot) = boot_request_fixture().await;
    let foreign_account_id = fixture.foreign.account.id;

    authentication_only_account(&fixture, None, 2).await;
    fixture.admin.execute_unprepared(&format!("CREATE FUNCTION reject_account_auth_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='NativeAuthentication' AND NEW.event='native_auth/session/created' AND NEW.item_id={foreign_account_id} THEN RAISE EXCEPTION 'synthetic authentication audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_account_auth_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_account_auth_audit()")).await.unwrap();
    let request = server(&boot);
    let rejected = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":fixture.foreign.account.email,"password":"password"}))
        .await;
    assert_eq!(rejected.status_code(), 500);
    assert_eq!(
        identity_session::Entity::find()
            .filter(identity_session::Column::UserId.eq(fixture.foreign.account.id))
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq("NativeAuthentication"))
            .filter(versions::Column::Event.eq("native_auth/session/created"))
            .filter(versions::Column::ItemId.eq(fixture.foreign.account.id))
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    fixture.admin.execute_unprepared("DROP TRIGGER reject_account_auth_audit ON versions; DROP FUNCTION reject_account_auth_audit()").await.unwrap();
    drop(request);
    close_request(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_account_status_and_session_lifetime_remain_required() {
    let (fixture, boot) = boot_request_fixture().await;

    authentication_only_account(&fixture, None, 1).await;
    let request = server(&boot);
    for (status, verified) in [(1, false), (3, true)] {
        account_status(&fixture, fixture.foreign.account.id, status, verified).await;
        let stored = native_account_fields(&fixture, fixture.foreign.account.id).await;
        assert_eq!(stored.try_get::<i32>("", "status").unwrap(), status);
        assert_eq!(
            stored.try_get::<bool>("", "email_verified").unwrap(),
            verified
        );
        let rejected = request
            .post("/api/auth/sign-in/email")
            .add_header("origin", boot.app_context.config.server.full_url())
            .json(&json!({"email":fixture.foreign.account.email,"password":"password"}))
            .await;
        assert!(
            rejected.status_code().is_client_error(),
            "status {status}: {}",
            rejected.status_code()
        );
        assert_eq!(
            identity_session::Entity::find()
                .filter(identity_session::Column::UserId.eq(fixture.foreign.account.id))
                .count(&fixture.admin)
                .await
                .unwrap(),
            0
        );
    }
    account_status(&fixture, fixture.foreign.account.id, 2, true).await;
    for mutation in [
        "active=false",
        "expires_at=now()-interval '1 second'",
        "created_at=now()-interval '31 days'",
    ] {
        let signed_in = request
            .post("/api/auth/sign-in/email")
            .add_header("origin", boot.app_context.config.server.full_url())
            .json(&json!({"email":fixture.foreign.account.email,"password":"password"}))
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
        let query = identity_session::Entity::update_many();
        let query = match mutation {
            "active=false" => query.col_expr(identity_session::Column::Active, Expr::value(false)),
            "expires_at=now()-interval '1 second'" => query.col_expr(
                identity_session::Column::ExpiresAt,
                Expr::value(Utc::now() - Duration::seconds(1)),
            ),
            _ => query.col_expr(
                identity_session::Column::CreatedAt,
                Expr::value(Utc::now() - Duration::days(31)),
            ),
        };
        let updated = query
            .filter(identity_session::Column::Id.eq(id.parse::<i64>().unwrap()))
            .exec(&fixture.admin)
            .await
            .unwrap();
        assert_eq!(updated.rows_affected, 1);
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
    close_request(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_session_revocation_clears_only_current_cookie_family() {
    let (fixture, boot) = boot_request_fixture().await;

    enable_completed_onboarding(&fixture, fixture.account.id).await;
    let request = server(&boot);
    let mut sessions = Vec::new();
    for _ in 0..2 {
        let login = request
            .post("/api/auth/sign-in/email")
            .add_header("origin", boot.app_context.config.server.full_url())
            .json(&json!({"email":fixture.account.email,"password":"password"}))
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
    close_request(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_throttle_retains_body_and_retry_after() {
    let (fixture, boot) = boot_request_fixture().await;

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
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_fixture().await;

    let native_email = format!("{}@example.test", fixture.credential("native-account"));
    let previous_settings = registration_setting(&fixture).await;
    let mut request = server(&boot);
    request.save_cookies();
    let response = request
        .post("/api/auth/sign-up/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({
            "name":"Native account owner", "email":native_email,
            "password":"Native-account-passphrase-42",
            "metadata":{"date_of_birth":"1990-04-12"}
        }))
        .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    let account = accounts::Entity::find()
        .filter(accounts::Column::Email.eq(&native_email))
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        people::Entity::find_by_id(account.person_id)
            .count(&fixture.admin)
            .await
            .unwrap(),
        1
    );
    let credential_query = {
        use sea_orm::sea_query::ExprTrait;
        Query::select()
            .expr_as(Expr::col("id").count(), "total")
            .from("auth_credentials")
            .and_where(Expr::col("user_id").eq(account.id))
            .and_where(Expr::col("provider_id").eq("credential"))
            .and_where(Expr::col("password").is_not_null())
            .to_owned()
    };
    let credentials: i64 = fixture
        .admin
        .query_one_raw(
            fixture
                .admin
                .get_database_backend()
                .build(&credential_query),
        )
        .await
        .unwrap()
        .unwrap()
        .try_get("", "total")
        .unwrap();
    assert_eq!(credentials, 1);
    let metadata = native_account_fields(&fixture, account.id)
        .await
        .try_get::<Value>("", "metadata")
        .unwrap();
    assert_eq!(metadata, json!({}));
    let mails = verification_mails(&fixture, &native_email).await;
    assert_eq!(mails.len(), 1);
    let mail = &mails[0];
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
    let membership = household_memberships::Entity::find()
        .filter(household_memberships::Column::PersonId.eq(account.person_id))
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let home = households::Entity::find_by_id(membership.household_id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let home_id = home.id;
    let slug = home.slug;
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
    let login = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(
            &json!({"email":native_email.to_uppercase(),"password":"Native-account-passphrase-42"}),
        )
        .await;
    assert_eq!(login.status_code(), 200, "{}", login.text());
    restore_registration(&fixture, previous_settings).await;
    drop(request);
    close_request(fixture, boot).await;
}

#[tokio::test]
async fn account_authentication_security_inventory_is_complete_live_and_account_scoped() {
    let (fixture, boot) = boot_request_fixture().await;

    authentication_only_account(&fixture, Some("Other inventory owner"), 2).await;
    enable_completed_onboarding(&fixture, fixture.account.id).await;
    let now = Utc::now();
    let sessions = (0..203)
        .map(|index| identity_session::ActiveModel {
            user_id: Set(if index == 202 {
                fixture.foreign.account.id
            } else {
                fixture.account.id
            }),
            token: Set(fixture.credential(&format!("inventory-secret-{index}"))),
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
            user_id: Set(if index == 102 {
                fixture.foreign.account.id
            } else {
                fixture.account.id
            }),
            name: Set(Some(format!("inventory-passkey-{index}"))),
            public_key: Set("inventory-only".into()),
            credential_id: Set(fixture.credential(&format!("inventory-credential-{index}"))),
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
    let mut invisible_sessions = Vec::new();
    for index in [0, 99, 202] {
        invisible_sessions.push(
            identity_session::Entity::find()
                .filter(
                    identity_session::Column::Token
                        .eq(fixture.credential(&format!("inventory-secret-{index}"))),
                )
                .one(&fixture.admin)
                .await
                .unwrap()
                .unwrap()
                .id,
        );
    }
    let own_session = identity_session::Entity::find()
        .filter(identity_session::Column::Token.eq(fixture.credential("inventory-secret-201")))
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .id;
    let own_passkey = identity_passkey::Entity::find()
        .filter(
            identity_passkey::Column::CredentialId
                .eq(fixture.credential("inventory-credential-101")),
        )
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .id;
    let foreign_session = *invisible_sessions.last().unwrap();
    let foreign_passkey = identity_passkey::Entity::find()
        .filter(
            identity_passkey::Column::CredentialId
                .eq(fixture.credential("inventory-credential-102")),
        )
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .id;
    let request = server(&boot);
    let login = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":fixture.account.email,"password":"password"}))
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
    for id in invisible_sessions {
        assert!(!html.contains(&format!("name=\"session_id\" value=\"{id}\"")));
    }
    assert!(!html.contains("inventory-passkey-102"));
    assert!(!html.contains("inventory-secret-"));
    assert!(html.contains("inventory-passkey-101"));
    let revoked = request
        .post("/api/auth/security/session/revoke")
        .add_header("origin", boot.app_context.config.server.full_url())
        .add_header("cookie", cookie.clone())
        .json(&json!({"session_id":own_session.to_string()}))
        .await;
    assert_eq!(revoked.status_code(), 200, "{}", revoked.text());
    let removed = request
        .post("/api/auth/passkey/delete-passkey")
        .add_header("origin", boot.app_context.config.server.full_url())
        .add_header("cookie", cookie.clone())
        .json(&json!({"id":own_passkey.to_string()}))
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
        identity_session::Entity::find_by_id(foreign_session)
            .one(&fixture.admin)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        identity_passkey::Entity::find_by_id(foreign_passkey)
            .one(&fixture.admin)
            .await
            .unwrap()
            .is_some()
    );
    drop(request);
    close_request(fixture, boot).await;
}

#[tokio::test]
async fn malformed_auth_entry_links_offer_safe_recovery() {
    let (fixture, boot) = boot_request_fixture().await;

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
    close_request(fixture, boot).await;
}

#[tokio::test]
async fn public_origin_auth_discovery_and_secure_session_use_proxy_origin() {
    let public = "https://mt-proof.localhost:1356";
    let (fixture, boot) = boot_request_fixture_with(|config| {
        let settings = config.settings.get_or_insert_with(|| json!({}));
        settings["public_origin"] = json!(public);
        settings["browser_session"] = json!({
            "key": base64::engine::general_purpose::STANDARD.encode([7_u8; 64]),
            "secure": true,
        });
    })
    .await;

    let account_id = fixture.account.id;
    assert_ne!(boot.app_context.config.server.port, 1356);
    let request = server(&boot);
    let discovery = request.get("/.well-known/oauth-authorization-server").await;
    assert_eq!(discovery.status_code(), 200);
    let discovery = discovery.json::<Value>();
    assert_eq!(discovery["issuer"], public);
    assert_eq!(discovery["token_endpoint"], format!("{public}/token"));
    let capabilities = request.get("/api/v1/capabilities").await;
    assert_eq!(capabilities.status_code(), 200);
    assert_eq!(
        capabilities.json::<Value>()["data"]["authentication"]["mobile_oauth"]["discovery_url"],
        format!("{public}/.well-known/oauth-authorization-server")
    );
    let rejected = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", "https://untrusted.example.test")
        .json(&json!({"email":fixture.account.email,"password":"password"}))
        .await;
    assert_eq!(rejected.status_code(), 403);
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", public)
        .json(&json!({"email":fixture.account.email,"password":"password"}))
        .await;
    assert_eq!(signed_in.status_code(), 200, "{}", signed_in.text());
    let header = signed_in.header("set-cookie");
    let header = header.to_str().unwrap();
    assert!(
        header
            .split(';')
            .any(|part| part.trim().eq_ignore_ascii_case("secure"))
    );
    let cookie = header.split(';').next().unwrap();
    let session = request
        .get("/api/auth/get-session")
        .add_header("cookie", cookie)
        .await;
    assert_eq!(session.status_code(), 200);
    assert_eq!(
        session.json::<Value>()["user"]["id"],
        account_id.to_string()
    );
    drop(request);
    close_request(fixture, boot).await;
}
