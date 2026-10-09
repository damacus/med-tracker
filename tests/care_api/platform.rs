use super::*;
use sea_orm::TransactionTrait;

#[tokio::test]
async fn platform_owner_recovery_cannot_be_bypassed_through_membership_updates() {
    for target in [74001, 74002] {
        let app = Application::new().await;
        synthetic_account(
            &app,
            71002,
            73004,
            77002,
            "platform-recovery-member@example.test",
            2,
        )
        .await;
        app.fixture.admin.execute_unprepared("INSERT INTO household_memberships(id,account_id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74002,71002,72001,73004,'member','active',now(),now(),now())").await.unwrap();
        admin_session(&app).await;
        let bearer = app.token().await;
        let response = app
            .client
            .patch(format!(
                "{}/api/v1/households/72001/admin/memberships/{target}",
                app.origin
            ))
            .bearer_auth(bearer)
            .json(&json!({"household_membership":{"role":"owner"}}))
            .send()
            .await
            .unwrap();
        let status = response.status();
        let row = app
            .fixture
            .admin
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT role FROM household_memberships WHERE id=$1",
                [target.into()],
            ))
            .await
            .unwrap()
            .unwrap();
        let role = row.try_get::<String>("", "role").unwrap();
        app.close().await;
        assert!(
            status.is_client_error(),
            "owner promotion requires the dedicated proof-bound recovery boundary for target {target}"
        );
        assert_ne!(role, "owner");
    }
}

async fn session(app: &Application) -> String {
    sign_in(app, 71001, "persistence@example.test").await
}

async fn admin_session(app: &Application) -> String {
    app.fixture.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71001,'active',now(),now()) ON CONFLICT(account_id) DO UPDATE SET status='active',updated_at=now()").await.unwrap();
    session(app).await
}

async fn sign_in(app: &Application, account_id: i64, email: &str) -> String {
    app.fixture.admin.execute_unprepared(&format!("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES({account_id},now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=now()")).await.unwrap();
    let service = app
        .context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let mut request =
        better_auth_core::AuthRequest::new(better_auth_core::HttpMethod::Post, "/sign-in/email");
    request
        .headers
        .insert("origin".into(), app.context.config.server.full_url());
    request
        .headers
        .insert("x-forwarded-for".into(), "127.0.0.1".into());
    request.body = Some(serde_json::to_vec(&json!({"email":email,"password":"password"})).unwrap());
    let response = med_tracker::models::identity::better_auth::dispatch(
        &service,
        request,
        "platform-fixture-sign-in".into(),
    )
    .await
    .unwrap();
    assert_eq!(response.status, 200);
    response
        .headers
        .get("set-cookie")
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

async fn synthetic_account(
    app: &Application,
    account_id: i64,
    person_id: i64,
    user_id: i64,
    email: &str,
    status: i32,
) {
    app.fixture.admin.execute_unprepared(&format!("INSERT INTO accounts(id,email,password_hash,status,created_at,updated_at) VALUES({account_id},'{email}',crypt('password',gen_salt('bf',4)),{status},now(),now())")).await.unwrap();
    app.fixture.admin.execute_unprepared(&format!("INSERT INTO people(id,account_id,household_id,name,person_type,has_capacity,created_at,updated_at) VALUES({person_id},{account_id},72001,'Synthetic platform member',0,true,now(),now())")).await.unwrap();
    app.fixture.admin.execute_unprepared(&format!("INSERT INTO users(id,person_id,email_address,password_digest,active,created_at,updated_at) VALUES({user_id},{person_id},'{email}',crypt('password',gen_salt('bf',4)),true,now(),now())")).await.unwrap();
}

async fn operation_start(
    app: &Application,
    cookie: &str,
    action: Value,
) -> (reqwest::StatusCode, Value) {
    let response = app
        .client
        .post(format!("{}/api/auth/security/operation/start", app.origin))
        .header("cookie", cookie)
        .header("origin", app.context.config.server.full_url())
        .json(&action)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.json::<Value>().await.unwrap_or_default();
    (status, body)
}

async fn operation_confirm(
    app: &Application,
    cookie: &str,
    operation_id: &str,
) -> reqwest::StatusCode {
    app.client
        .post(format!("{}/api/auth/security/password/confirm", app.origin))
        .header("cookie", cookie)
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"operation_id":operation_id,"password":"password"}))
        .send()
        .await
        .unwrap()
        .status()
}

async fn admin_status(app: &Application, account_id: i64) -> Option<String> {
    app.fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT status FROM platform_admins WHERE account_id=$1",
            [account_id.into()],
        ))
        .await
        .unwrap()
        .and_then(|row| row.try_get::<String>("", "status").ok())
}

async fn account_status(app: &Application, account_id: i64) -> Option<i32> {
    app.fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT status FROM accounts WHERE id=$1",
            [account_id.into()],
        ))
        .await
        .unwrap()
        .and_then(|row| row.try_get::<i32>("", "status").ok())
}

async fn user_active(app: &Application, user_id: i64) -> Option<bool> {
    app.fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT active FROM users WHERE id=$1",
            [user_id.into()],
        ))
        .await
        .unwrap()
        .and_then(|row| row.try_get::<bool>("", "active").ok())
}

fn hidden_field(body: &str, name: &str) -> String {
    body.split(&format!("name=\"{name}\" value=\""))
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .unwrap_or_default()
        .to_owned()
}

fn response_cookies(base: &str, response: &reqwest::Response) -> String {
    let mut cookie = base.to_owned();
    for value in response.headers().get_all("set-cookie") {
        if let Some(pair) = value
            .to_str()
            .ok()
            .and_then(|value| value.split(';').next())
            && !pair.ends_with('=')
        {
            cookie.push_str("; ");
            cookie.push_str(pair);
        }
    }
    cookie
}

async fn form_page(app: &Application, cookie: &str, path: &str) -> (String, String) {
    let response = app
        .client
        .get(format!("{}{}", app.origin, path))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let cookies = response_cookies(cookie, &response);
    (cookies, response.text().await.unwrap())
}

async fn post_form(
    app: &Application,
    cookie: &str,
    path: &str,
    form: &[(&str, &str)],
) -> reqwest::Response {
    let public = app.context.config.server.full_url();
    let url = url::Url::parse(&public).unwrap();
    let authority = match url.port() {
        Some(port) => format!("{}:{port}", url.host_str().unwrap()),
        None => url.host_str().unwrap().to_owned(),
    };
    app.client
        .post(format!("{}{}", app.origin, path))
        .header("cookie", cookie)
        .header("host", authority)
        .header("origin", public.trim_end_matches('/'))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(serde_urlencoded::to_string(form).unwrap())
        .send()
        .await
        .unwrap()
}

async fn browser_confirm(
    app: &Application,
    cookie: &str,
    operation_id: &str,
    authenticity_token: &str,
) -> (reqwest::StatusCode, String) {
    let response = post_form(
        app,
        cookie,
        "/account/security/password/confirm",
        &[
            ("operation_id", operation_id),
            ("password", "password"),
            ("authenticity_token", authenticity_token),
        ],
    )
    .await;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    (status, body)
}

#[tokio::test]
async fn platform_users_requires_sign_in() {
    let app = Application::new().await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let response = client
        .get(format!("{}/platform/users", app.origin))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let target = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    app.close().await;
    assert!(status.is_redirection());
    assert_eq!(target.as_deref(), Some("/login"));
}

#[tokio::test]
async fn platform_users_rejects_a_household_administrator_without_platform_rights() {
    let app = Application::new().await;
    let cookie = session(&app).await;
    let response = app
        .client
        .get(format!("{}/platform/users", app.origin))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    let status = response.status();
    app.close().await;
    assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn platform_users_lists_users_for_an_active_platform_administrator() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71001,'active',now(),now())").await.unwrap();
    let cookie = session(&app).await;
    let response = app
        .client
        .get(format!("{}/platform/users", app.origin))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    app.close().await;
    assert_eq!(status, reqwest::StatusCode::OK);
    assert!(body.contains("persistence@example.test"));
    assert!(!body.contains("password_hash"));
}

#[tokio::test]
async fn platform_users_search_filters_the_listing() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71001,'active',now(),now())").await.unwrap();
    let cookie = session(&app).await;
    let matching = app
        .client
        .get(format!("{}/platform/users?q=persist", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let other = app
        .client
        .get(format!("{}/platform/users?q=someone-else", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    app.close().await;
    assert!(matching.contains("persistence@example.test"));
    assert!(!other.contains("persistence@example.test"));
}

#[tokio::test]
async fn platform_users_clamps_pages_beyond_the_listing() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71001,'active',now(),now())").await.unwrap();
    let cookie = session(&app).await;
    let response = app
        .client
        .get(format!("{}/platform/users?page=2", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    app.close().await;
    assert_eq!(status, reqwest::StatusCode::OK);
    assert!(body.contains("persistence@example.test"));
}

#[tokio::test]
async fn platform_users_rechecks_withdrawn_platform_rights() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71001,'active',now(),now())").await.unwrap();
    let cookie = session(&app).await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE platform_admins SET status='disabled' WHERE account_id=71001")
        .await
        .unwrap();
    let response = app
        .client
        .get(format!("{}/platform/users", app.origin))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    let status = response.status();
    app.close().await;
    assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn platform_users_rechecks_a_deactivated_account() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71001,'active',now(),now())").await.unwrap();
    let cookie = session(&app).await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE accounts SET status=3 WHERE id=71001")
        .await
        .unwrap();
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let response = client
        .get(format!("{}/platform/users", app.origin))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let target = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    app.close().await;
    assert!(status.is_redirection());
    assert_eq!(target.as_deref(), Some("/login"));
}

#[tokio::test]
async fn platform_users_requires_a_live_identity_session() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71001,'active',now(),now())").await.unwrap();
    let cookie = session(&app).await;
    app.fixture
        .admin
        .execute_unprepared("DELETE FROM identity_sessions")
        .await
        .unwrap();
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let response = client
        .get(format!("{}/platform/users", app.origin))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let target = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    app.close().await;
    assert!(status.is_redirection());
    assert_eq!(target.as_deref(), Some("/login"));
}

#[tokio::test]
async fn platform_administrator_grant_after_password_proof() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":71002,"grant":true}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    assert_eq!(
        admin_status(&app, 71002).await,
        None,
        "no mutation at proof start"
    );
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let granted = admin_status(&app, 71002).await;
    app.close().await;
    assert!(confirm.is_success());
    assert_eq!(granted.as_deref(), Some("active"));
}

#[tokio::test]
async fn platform_administrator_revoke_after_password_proof() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    app.fixture.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71002,'active',now(),now())").await.unwrap();
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":71002,"grant":false}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let revoked = admin_status(&app, 71002).await;
    app.close().await;
    assert!(confirm.is_success());
    assert_eq!(revoked.as_deref(), Some("disabled"));
}

#[tokio::test]
async fn platform_user_deactivate_and_reactivate_after_password_proof() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_user","account_id":71002,"active":false}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    assert_eq!(
        user_active(&app, 77002).await,
        Some(true),
        "no mutation at proof start"
    );
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    assert!(confirm.is_success());
    assert_eq!(user_active(&app, 77002).await, Some(false));
    assert_eq!(
        account_status(&app, 71002).await,
        Some(2),
        "verified account must not be closed by platform disable"
    );
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_user","account_id":71002,"active":true}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let active = user_active(&app, 77002).await;
    app.close().await;
    assert!(confirm.is_success());
    assert_eq!(active, Some(true));
}

#[tokio::test]
async fn platform_operation_denied_without_platform_rights() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    let cookie = session(&app).await;
    let (status, _) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":71002,"grant":true}),
    )
    .await;
    let granted = admin_status(&app, 71002).await;
    app.close().await;
    assert!(status.is_client_error());
    assert_eq!(granted, None);
}

#[tokio::test]
async fn platform_operation_refused_when_admin_rights_withdrawn_before_confirm() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":71002,"grant":true}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    app.fixture
        .admin
        .execute_unprepared("UPDATE platform_admins SET status='disabled' WHERE account_id=71001")
        .await
        .unwrap();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let granted = admin_status(&app, 71002).await;
    app.close().await;
    assert!(!confirm.is_success());
    assert_eq!(granted, None);
}

#[tokio::test]
async fn platform_operation_refused_when_session_revoked_before_confirm() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":71002,"grant":true}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    app.fixture
        .admin
        .execute_unprepared("DELETE FROM identity_sessions")
        .await
        .unwrap();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let granted = admin_status(&app, 71002).await;
    app.close().await;
    assert!(!confirm.is_success());
    assert_eq!(granted, None);
}

#[tokio::test]
async fn platform_proof_bound_to_requesting_session() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    let cookie = admin_session(&app).await;
    let other = session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":71002,"grant":true}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    let confirm = operation_confirm(&app, &other, &operation_id).await;
    let granted = admin_status(&app, 71002).await;
    app.close().await;
    assert!(!confirm.is_success());
    assert_eq!(granted, None);
}

#[tokio::test]
async fn platform_proof_bound_to_target_account() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    synthetic_account(&app, 71003, 73005, 77003, "platform-third@example.test", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":71002,"grant":true}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let granted = admin_status(&app, 71002).await;
    let untouched = admin_status(&app, 71003).await;
    app.close().await;
    assert!(confirm.is_success());
    assert_eq!(granted.as_deref(), Some("active"));
    assert_eq!(untouched, None);
}

#[tokio::test]
async fn platform_proof_bound_to_action() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_user","account_id":71002,"active":false}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let active = user_active(&app, 77002).await;
    let granted = admin_status(&app, 71002).await;
    app.close().await;
    assert!(confirm.is_success());
    assert_eq!(active, Some(false));
    assert_eq!(
        granted, None,
        "a platform_user proof must not create administrator rights"
    );
}

#[tokio::test]
async fn platform_proof_expires() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":71002,"grant":true}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    app.fixture.admin.execute_unprepared(&format!("UPDATE identity_verifications SET expires_at=now()-interval '1 minute' WHERE identifier='password-change:{operation_id}'")).await.unwrap();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let granted = admin_status(&app, 71002).await;
    app.close().await;
    assert!(!confirm.is_success());
    assert_eq!(granted, None);
}

#[tokio::test]
async fn platform_proof_single_use() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":71002,"grant":true}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    assert!(
        operation_confirm(&app, &cookie, &operation_id)
            .await
            .is_success()
    );
    let replay = operation_confirm(&app, &cookie, &operation_id).await;
    let granted = admin_status(&app, 71002).await;
    app.close().await;
    assert!(!replay.is_success());
    assert_eq!(granted.as_deref(), Some("active"));
}

#[tokio::test]
async fn platform_administrator_revoke_refuses_last_active_administrator() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":71001,"grant":false}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let remaining = admin_status(&app, 71001).await;
    app.close().await;
    assert!(!confirm.is_success());
    assert_eq!(remaining.as_deref(), Some("active"));
}

#[tokio::test]
async fn platform_user_disable_refuses_last_active_administrator() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_user","account_id":71001,"active":false}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let active = user_active(&app, 77001).await;
    app.close().await;
    assert!(!confirm.is_success());
    assert_eq!(active, Some(true));
}

#[tokio::test]
async fn platform_last_administrator_guard_survives_concurrent_removals() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    app.fixture.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71002,'active',now(),now())").await.unwrap();
    let first = admin_session(&app).await;
    let second = sign_in(&app, 71002, "platform-second@example.test").await;
    let (status, body) = operation_start(
        &app,
        &first,
        json!({"action":"platform_administrator","account_id":71002,"grant":false}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let first_operation = body["operation_id"].as_str().unwrap().to_owned();
    let (status, body) = operation_start(
        &app,
        &second,
        json!({"action":"platform_administrator","account_id":71001,"grant":false}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let second_operation = body["operation_id"].as_str().unwrap().to_owned();
    let barrier = app.fixture.admin.begin().await.unwrap();
    barrier
        .execute_unprepared("SELECT pg_advisory_xact_lock(1920296809,3)")
        .await
        .unwrap();
    let release = async {
        let waiting = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let row = app.fixture.admin.query_one_raw(sea_orm::Statement::from_string(
                    sea_orm::DbBackend::Postgres,
                    "SELECT count(*) AS count FROM pg_locks WHERE locktype='advisory' AND database=(SELECT oid FROM pg_database WHERE datname=current_database()) AND classid=1920296809 AND objid=3 AND NOT granted",
                )).await.unwrap().unwrap();
                if row.try_get::<i64>("", "count").unwrap() == 2 {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        }).await;
        barrier.commit().await.unwrap();
        waiting.unwrap();
    };
    let (one, two, ()) = tokio::join!(
        operation_confirm(&app, &first, &first_operation),
        operation_confirm(&app, &second, &second_operation),
        release
    );
    let remaining = admin_status(&app, 71001).await;
    let second_remaining = admin_status(&app, 71002).await;
    app.close().await;
    assert!(
        one.is_success() ^ two.is_success(),
        "exactly one removal should win: {one} {two}"
    );
    assert!(
        !one.is_server_error() && !two.is_server_error(),
        "competing removals must return a deliberate authorization or conflict result: {one} {two}"
    );
    assert!(
        remaining.as_deref() == Some("active") || second_remaining.as_deref() == Some("active"),
        "at least one eligible platform administrator must remain"
    );
}

#[tokio::test]
async fn platform_audit_failure_rolls_back_mutation_and_proof() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":71002,"grant":true}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    app.fixture
        .admin
        .execute_unprepared("REVOKE INSERT ON public.versions FROM med_tracker_app")
        .await
        .unwrap();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let blocked = admin_status(&app, 71002).await;
    app.fixture
        .admin
        .execute_unprepared("GRANT INSERT ON public.versions TO med_tracker_app")
        .await
        .unwrap();
    let retry = operation_confirm(&app, &cookie, &operation_id).await;
    let granted = admin_status(&app, 71002).await;
    app.close().await;
    assert!(!confirm.is_success());
    assert_eq!(blocked, None, "failed audit must roll back the mutation");
    assert!(
        retry.is_success(),
        "failed mutation must not consume the proof"
    );
    assert_eq!(granted.as_deref(), Some("active"));
}

#[tokio::test]
async fn platform_inactive_user_administrator_is_not_viable_remaining_admin() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    app.fixture.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71002,'active',now(),now())").await.unwrap();
    app.fixture
        .admin
        .execute_unprepared("UPDATE users SET active=false WHERE id=77002")
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72002,71002,'Second profile household','second-profile-household','Europe/London',now(),now()); INSERT INTO people(id,account_id,household_id,name,person_type,has_capacity,created_at,updated_at) VALUES(73005,71002,72002,'Later active profile',0,true,now(),now()); INSERT INTO users(id,person_id,email_address,password_digest,active,created_at,updated_at) VALUES(77003,73005,'later-profile@example.test',crypt('password',gen_salt('bf',4)),true,now(),now())").await.unwrap();
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":71001,"grant":false}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let remaining = admin_status(&app, 71001).await;
    app.close().await;
    assert!(!confirm.is_success());
    assert_eq!(remaining.as_deref(), Some("active"));
}

#[tokio::test]
async fn platform_administrator_grant_refuses_ineligible_accounts() {
    let app = Application::new().await;
    synthetic_account(&app, 71003, 73005, 77003, "platform-closed@example.test", 3).await;
    synthetic_account(
        &app,
        71004,
        73006,
        77004,
        "platform-unverified@example.test",
        1,
    )
    .await;
    synthetic_account(&app, 71005, 73007, 77005, "platform-mixed@example.test", 2).await;
    app.fixture.admin.execute_unprepared("INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72002,71005,'Second grant household','second-grant-household','Europe/London',now(),now()); UPDATE users SET active=false WHERE id=77005; INSERT INTO people(id,account_id,household_id,name,person_type,has_capacity,created_at,updated_at) VALUES(73008,71005,72002,'Later active profile',0,true,now(),now()); INSERT INTO users(id,person_id,email_address,password_digest,active,created_at,updated_at) VALUES(77006,73008,'later-grant@example.test',crypt('password',gen_salt('bf',4)),true,now(),now())").await.unwrap();
    let cookie = admin_session(&app).await;
    for target in [71003, 71004, 71005, 79999] {
        let (status, body) = operation_start(
            &app,
            &cookie,
            json!({"action":"platform_administrator","account_id":target,"grant":true}),
        )
        .await;
        if status == reqwest::StatusCode::OK {
            let operation_id = body["operation_id"].as_str().unwrap().to_owned();
            let confirm = operation_confirm(&app, &cookie, &operation_id).await;
            assert!(
                !confirm.is_success(),
                "grant to ineligible account {target} must not complete"
            );
        }
        assert_eq!(
            admin_status(&app, target).await,
            None,
            "ineligible account {target} must not gain rights"
        );
    }
    app.close().await;
}

#[tokio::test]
async fn platform_user_enable_refuses_closed_and_unverified_accounts() {
    let app = Application::new().await;
    synthetic_account(&app, 71003, 73005, 77003, "platform-closed@example.test", 3).await;
    synthetic_account(
        &app,
        71004,
        73006,
        77004,
        "platform-unverified@example.test",
        1,
    )
    .await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE users SET active=false WHERE id IN (77003,77004)")
        .await
        .unwrap();
    let cookie = admin_session(&app).await;
    for (account_id, user_id) in [(71003, 77003), (71004, 77004)] {
        let (status, body) = operation_start(
            &app,
            &cookie,
            json!({"action":"platform_user","account_id":account_id,"active":true}),
        )
        .await;
        if status == reqwest::StatusCode::OK {
            let operation_id = body["operation_id"].as_str().unwrap().to_owned();
            let confirm = operation_confirm(&app, &cookie, &operation_id).await;
            assert!(
                !confirm.is_success(),
                "enable of account {account_id} must not complete"
            );
        }
        assert_eq!(
            user_active(&app, user_id).await,
            Some(false),
            "account {account_id} user must stay disabled"
        );
    }
    app.close().await;
}

#[tokio::test]
async fn platform_last_administrator_cannot_close_own_account() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(&app, &cookie, json!({"action":"close_account"})).await;
    let confirmed = if status.is_success() {
        operation_confirm(&app, &cookie, body["operation_id"].as_str().unwrap()).await
    } else {
        status
    };
    let account = account_status(&app, 71001).await;
    let active = user_active(&app, 77001).await;
    app.close().await;
    assert!(
        !confirmed.is_success(),
        "self-closure must preserve the last usable platform administrator"
    );
    assert_eq!(account, Some(2));
    assert_eq!(active, Some(true));
}

#[tokio::test]
async fn platform_last_administrator_guard_survives_concurrent_self_closures() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    app.fixture.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71002,'active',now(),now())").await.unwrap();
    let first = admin_session(&app).await;
    let second = sign_in(&app, 71002, "platform-second@example.test").await;
    let (one, first_body) = operation_start(&app, &first, json!({"action":"close_account"})).await;
    let (two, second_body) =
        operation_start(&app, &second, json!({"action":"close_account"})).await;
    assert_eq!(one, reqwest::StatusCode::OK);
    assert_eq!(two, reqwest::StatusCode::OK);
    let (one, two) = tokio::join!(
        operation_confirm(&app, &first, first_body["operation_id"].as_str().unwrap()),
        operation_confirm(&app, &second, second_body["operation_id"].as_str().unwrap())
    );
    let first_status = account_status(&app, 71001).await;
    let second_status = account_status(&app, 71002).await;
    app.close().await;
    assert!(
        one.is_success() ^ two.is_success(),
        "exactly one self-closure can leave a usable administrator: {one} {two}"
    );
    assert!(
        !one.is_server_error() && !two.is_server_error(),
        "competing closures must return a deliberate conflict result: {one} {two}"
    );
    assert!(first_status == Some(2) || second_status == Some(2));
}

#[tokio::test]
async fn platform_can_revoke_an_inactive_administrator_without_removing_the_last_usable_admin() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    app.fixture.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71002,'active',now(),now()); UPDATE users SET active=false WHERE id=77002").await.unwrap();
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":71002,"grant":false}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let confirmed = operation_confirm(&app, &cookie, body["operation_id"].as_str().unwrap()).await;
    let disabled = admin_status(&app, 71002).await;
    let remaining = admin_status(&app, 71001).await;
    app.close().await;
    assert!(confirmed.is_success());
    assert_eq!(disabled.as_deref(), Some("disabled"));
    assert_eq!(remaining.as_deref(), Some("active"));
}

#[tokio::test]
async fn platform_confirmation_cannot_replace_the_bound_target_or_action() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    synthetic_account(&app, 71003, 73005, 77003, "platform-third@example.test", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":71002,"grant":true}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap();
    let rejected = app.client.post(format!("{}/api/auth/security/password/confirm", app.origin))
        .header("cookie", &cookie).header("origin", app.context.config.server.full_url())
        .json(&json!({"operation_id":operation_id,"password":"password","account_id":71003,"action":"platform_user","active":false}))
        .send().await.unwrap().status();
    assert!(!rejected.is_success());
    assert_eq!(admin_status(&app, 71002).await, None);
    assert_eq!(admin_status(&app, 71003).await, None);
    assert_eq!(user_active(&app, 77003).await, Some(true));
    let accepted = operation_confirm(&app, &cookie, operation_id).await;
    let bound = admin_status(&app, 71002).await;
    let other = admin_status(&app, 71003).await;
    app.close().await;
    assert!(accepted.is_success());
    assert_eq!(bound.as_deref(), Some("active"));
    assert_eq!(other, None);
}

#[tokio::test]
async fn platform_user_reactivation_does_not_revive_old_sessions() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    let actor = admin_session(&app).await;
    let old = sign_in(&app, 71002, "platform-second@example.test").await;
    for active in [false, true] {
        let (status, body) = operation_start(
            &app,
            &actor,
            json!({"action":"platform_user","account_id":71002,"active":active}),
        )
        .await;
        assert_eq!(status, reqwest::StatusCode::OK);
        assert!(
            operation_confirm(&app, &actor, body["operation_id"].as_str().unwrap())
                .await
                .is_success()
        );
    }
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let stale = client
        .get(format!("{}/account/security", app.origin))
        .header("cookie", old)
        .send()
        .await
        .unwrap();
    let stale_status = stale.status();
    let stale_target = stale
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let fresh = sign_in(&app, 71002, "platform-second@example.test").await;
    let current = client
        .get(format!("{}/account/security", app.origin))
        .header("cookie", fresh)
        .send()
        .await
        .unwrap()
        .status();
    app.close().await;
    assert!(stale_status.is_redirection());
    assert_eq!(stale_target.as_deref(), Some("/login"));
    assert_eq!(current, reqwest::StatusCode::OK);
}

#[tokio::test]
async fn platform_users_page_shows_user_state_and_action_forms() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE users SET active=false WHERE id=77002")
        .await
        .unwrap();
    let cookie = admin_session(&app).await;
    let (_cookies, body) = form_page(&app, &cookie, "/platform/users").await;
    app.close().await;
    assert!(!hidden_field(&body, "authenticity_token").is_empty());
    assert!(body.contains("action=\"/platform/users/administrator\""));
    assert!(body.contains("action=\"/platform/users/active\""));
    assert!(body.contains("name=\"account_id\" value=\"71002\""));
    assert!(
        body.contains("name=\"active\" value=\"true\""),
        "disabled user must offer an enable action"
    );
    assert!(
        body.contains("Sign-in disabled"),
        "page must show the linked user is disabled"
    );
}

#[tokio::test]
async fn platform_users_page_links_are_bounded() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,password_hash,status,created_at,updated_at) SELECT 80000+i,format('bulk-%s@example.test',i),crypt('password',gen_salt('bf',4)),2,now(),now() FROM generate_series(1,300) i").await.unwrap();
    let cookie = admin_session(&app).await;
    let (_cookies, body) =
        form_page(&app, &cookie, "/platform/users?page=9223372036854775807").await;
    let links = body.matches("join-item").count();
    app.close().await;
    assert!(links <= 9, "page links must be bounded, got {links}");
    assert!(
        body.contains("bulk-300@example.test"),
        "oversized pages must show the final page"
    );
}

#[tokio::test]
async fn platform_browser_administrator_grant_roundtrip() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    let cookie = admin_session(&app).await;
    let (cookies, body) = form_page(&app, &cookie, "/platform/users").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/users/administrator",
        &[
            ("account_id", "71002"),
            ("grant", "true"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let status = response.status();
    let cookies = response_cookies(&cookies, &response);
    let page = response.text().await.unwrap();
    assert_eq!(status, reqwest::StatusCode::OK, "{page}");
    let operation_id = hidden_field(&page, "operation_id");
    let confirm_token = hidden_field(&page, "authenticity_token");
    assert!(!operation_id.is_empty());
    assert_eq!(admin_status(&app, 71002).await, None);
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    assert_eq!(admin_status(&app, 71002).await.as_deref(), Some("active"));
    app.close().await;
}

#[tokio::test]
async fn platform_browser_preserves_the_authentication_origin_boundary() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    let cookie = admin_session(&app).await;
    let (cookies, body) = form_page(&app, &cookie, "/platform/users").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = app
        .client
        .post(format!("{}/platform/users/administrator", app.origin))
        .header("cookie", cookies)
        .header("host", "untrusted.example.test")
        .header("origin", "https://untrusted.example.test")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(
            serde_urlencoded::to_string([
                ("account_id", "71002"),
                ("grant", "true"),
                ("authenticity_token", token.as_str()),
            ])
            .unwrap(),
        )
        .send()
        .await
        .unwrap();
    let status = response.status();
    let target = admin_status(&app, 71002).await;
    app.close().await;
    assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
    assert_eq!(target, None);
}

#[tokio::test]
async fn platform_settings_initialization_cannot_race_signup_into_two_rows() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("DELETE FROM app_settings; CREATE FUNCTION public.platform_settings_insert_barrier() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_advisory_xact_lock(1920296809,77); RETURN NEW; END $$; CREATE TRIGGER platform_settings_insert_barrier BEFORE INSERT ON app_settings FOR EACH ROW EXECUTE FUNCTION public.platform_settings_insert_barrier()").await.unwrap();
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({
            "action":"platform_settings",
            "invite_only":true,
            "medicine_lookup_base_url":"https://lookup.example.test/api",
            "medicine_lookup_token_url":"https://lookup.example.test/token",
            "medicine_lookup_source_priority":["nhs_dmd"]
        }),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    let barrier = app.fixture.admin.begin().await.unwrap();
    barrier
        .execute_unprepared("SELECT pg_advisory_xact_lock(1920296809,77)")
        .await
        .unwrap();
    let release = async {
        let waiting = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let row = app.fixture.admin.query_one_raw(sea_orm::Statement::from_string(
                    sea_orm::DbBackend::Postgres,
                    "SELECT count(*) AS count FROM pg_locks WHERE locktype='advisory' AND database=(SELECT oid FROM pg_database WHERE datname=current_database()) AND classid=1920296809 AND objid IN (1,77) AND NOT granted",
                )).await.unwrap().unwrap();
                if row.try_get::<i64>("", "count").unwrap() == 2 {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        }).await;
        barrier.commit().await.unwrap();
        waiting.unwrap();
    };
    let (signup, changed, ()) = tokio::join!(
        app.client
            .get(format!("{}/create-account", app.origin))
            .send(),
        operation_confirm(&app, &cookie, &operation_id),
        release
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            "SELECT count(*) AS count, bool_and(invite_only) AS closed FROM app_settings",
        ))
        .await
        .unwrap()
        .unwrap();
    let count = row.try_get::<i64>("", "count").unwrap();
    let closed = row.try_get::<bool>("", "closed").unwrap();
    app.close().await;
    assert!(signup.unwrap().status().is_success());
    assert_eq!(changed, reqwest::StatusCode::OK);
    assert_eq!(
        count, 1,
        "signup and platform initialization must share one settings row"
    );
    assert!(
        closed,
        "the confirmed invite-only setting must be authoritative"
    );
}

#[tokio::test]
async fn platform_browser_administrator_revoke_roundtrip() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    app.fixture.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71002,'active',now(),now())").await.unwrap();
    let cookie = admin_session(&app).await;
    let (cookies, body) = form_page(&app, &cookie, "/platform/users").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/users/administrator",
        &[
            ("account_id", "71002"),
            ("grant", "false"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let status = response.status();
    let cookies = response_cookies(&cookies, &response);
    let page = response.text().await.unwrap();
    assert_eq!(status, reqwest::StatusCode::OK, "{page}");
    let operation_id = hidden_field(&page, "operation_id");
    let confirm_token = hidden_field(&page, "authenticity_token");
    assert!(!operation_id.is_empty());
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    assert_eq!(admin_status(&app, 71002).await.as_deref(), Some("disabled"));
    app.close().await;
}

#[tokio::test]
async fn platform_browser_user_disable_and_enable_roundtrip() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    let cookie = admin_session(&app).await;
    for (active, expected) in [("false", false), ("true", true)] {
        let (cookies, body) = form_page(&app, &cookie, "/platform/users").await;
        let token = hidden_field(&body, "authenticity_token");
        let response = post_form(
            &app,
            &cookies,
            "/platform/users/active",
            &[
                ("account_id", "71002"),
                ("active", active),
                ("authenticity_token", &token),
            ],
        )
        .await;
        let status = response.status();
        let cookies = response_cookies(&cookies, &response);
        let page = response.text().await.unwrap();
        assert_eq!(status, reqwest::StatusCode::OK, "{page}");
        let operation_id = hidden_field(&page, "operation_id");
        let confirm_token = hidden_field(&page, "authenticity_token");
        assert!(!operation_id.is_empty());
        let (confirmed, outcome) =
            browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
        assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
        assert_eq!(user_active(&app, 77002).await, Some(expected));
        assert_eq!(account_status(&app, 71002).await, Some(2));
    }
    app.close().await;
}

#[tokio::test]
async fn platform_browser_forms_require_csrf_and_platform_rights() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    let cookie = admin_session(&app).await;
    let denied = post_form(
        &app,
        &cookie,
        "/platform/users/administrator",
        &[("account_id", "71002"), ("grant", "true")],
    )
    .await;
    assert!(!denied.status().is_success());
    let bogus = post_form(
        &app,
        &cookie,
        "/platform/users/administrator",
        &[
            ("account_id", "71002"),
            ("grant", "true"),
            ("authenticity_token", "bogus"),
        ],
    )
    .await;
    assert!(!bogus.status().is_success());
    let member = session(&app).await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE platform_admins SET status='disabled' WHERE account_id=71001")
        .await
        .unwrap();
    let (cookies, _body) = form_page(&app, &member, "/account/security").await;
    let token = hidden_field(&_body, "authenticity_token");
    let forbidden = post_form(
        &app,
        &cookies,
        "/platform/users/administrator",
        &[
            ("account_id", "71002"),
            ("grant", "true"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    assert!(!forbidden.status().is_success());
    assert_eq!(admin_status(&app, 71002).await, None);
    app.close().await;
}

#[tokio::test]
async fn platform_browser_last_administrator_revoke_conflicts() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let (cookies, body) = form_page(&app, &cookie, "/platform/users").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/users/administrator",
        &[
            ("account_id", "71001"),
            ("grant", "false"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    if response.status().is_success() {
        let cookies = response_cookies(&cookies, &response);
        let page = response.text().await.unwrap();
        let operation_id = hidden_field(&page, "operation_id");
        let confirm_token = hidden_field(&page, "authenticity_token");
        let (confirmed, outcome) =
            browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
        assert!(
            !confirmed.is_success(),
            "last usable administrator revoke must conflict: {outcome}"
        );
    }
    assert_eq!(admin_status(&app, 71001).await.as_deref(), Some("active"));
    app.close().await;
}

async fn app_settings(app: &Application) -> Option<(bool, String, String)> {
    app.fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT invite_only,medicine_lookup_base_url,medicine_lookup_token_url FROM app_settings ORDER BY id LIMIT 1",
            [],
        ))
        .await
        .unwrap()
        .and_then(|row| {
            Some((
                row.try_get::<bool>("", "invite_only").ok()?,
                row.try_get::<String>("", "medicine_lookup_base_url").ok()?,
                row.try_get::<String>("", "medicine_lookup_token_url").ok()?,
            ))
        })
}

async fn platform_version_events(app: &Application, item_type: &str) -> i64 {
    app.fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM versions WHERE item_type=$1",
            [item_type.into()],
        ))
        .await
        .unwrap()
        .and_then(|row| row.try_get::<i64>("", "count").ok())
        .unwrap_or_default()
}

#[tokio::test]
async fn platform_settings_requires_platform_administrator() {
    let app = Application::new().await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let anonymous = client
        .get(format!("{}/platform/settings", app.origin))
        .send()
        .await
        .unwrap();
    assert!(anonymous.status().is_redirection());
    let member = client
        .get(format!("{}/platform/settings", app.origin))
        .header("cookie", session(&app).await)
        .send()
        .await
        .unwrap();
    assert_eq!(member.status(), reqwest::StatusCode::FORBIDDEN);
    let cookie = admin_session(&app).await;
    let (_cookies, body) = form_page(&app, &cookie, "/platform/settings").await;
    app.close().await;
    assert!(body.contains("medicine_lookup_base_url"));
    assert!(body.contains("invite_only"));
    assert!(body.contains("ontology.nhs.uk"));
    assert!(!body.contains("password_hash"));
}

#[tokio::test]
async fn platform_settings_update_via_browser_proof() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let (cookies, body) = form_page(&app, &cookie, "/platform/settings").await;
    let token = hidden_field(&body, "authenticity_token");
    let before = platform_version_events(&app, "AppSettings").await;
    let response = post_form(
        &app,
        &cookies,
        "/platform/settings",
        &[
            ("invite_only", "true"),
            (
                "medicine_lookup_base_url",
                "https://lookup.example.test/fhir",
            ),
            (
                "medicine_lookup_token_url",
                "https://lookup.example.test/token",
            ),
            ("medicine_lookup_source_priority", "nhs_dmd,supplements"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let status = response.status();
    let cookies = response_cookies(&cookies, &response);
    let page = response.text().await.unwrap();
    assert_eq!(status, reqwest::StatusCode::OK, "{page}");
    let operation_id = hidden_field(&page, "operation_id");
    let confirm_token = hidden_field(&page, "authenticity_token");
    assert!(!operation_id.is_empty());
    assert_eq!(
        app_settings(&app).await.map(|row| row.1),
        None,
        "proof start must not change settings"
    );
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let settings = app_settings(&app).await.unwrap();
    assert!(settings.0);
    assert_eq!(settings.1, "https://lookup.example.test/fhir");
    assert_eq!(settings.2, "https://lookup.example.test/token");
    let audits = platform_version_events(&app, "AppSettings").await;
    app.close().await;
    assert!(audits > before, "settings change must be audited");
}

#[tokio::test]
async fn platform_settings_rejects_insecure_and_unknown_values() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let (cookies, body) = form_page(&app, &cookie, "/platform/settings").await;
    let token = hidden_field(&body, "authenticity_token");
    for form in [
        vec![
            ("invite_only", "false"),
            ("medicine_lookup_base_url", "http://insecure.example.test"),
            (
                "medicine_lookup_token_url",
                "https://lookup.example.test/token",
            ),
            ("medicine_lookup_source_priority", "nhs_dmd"),
            ("authenticity_token", token.as_str()),
        ],
        vec![
            ("invite_only", "false"),
            (
                "medicine_lookup_base_url",
                "https://lookup.example.test/fhir",
            ),
            (
                "medicine_lookup_token_url",
                "https://lookup.example.test/token",
            ),
            ("medicine_lookup_source_priority", "unknown_source"),
            ("authenticity_token", token.as_str()),
        ],
        vec![
            ("invite_only", "false"),
            (
                "medicine_lookup_base_url",
                "https://client:secret@lookup.example.test/fhir",
            ),
            (
                "medicine_lookup_token_url",
                "https://lookup.example.test/token",
            ),
            ("medicine_lookup_source_priority", "nhs_dmd"),
            ("authenticity_token", token.as_str()),
        ],
    ] {
        let response = post_form(&app, &cookies, "/platform/settings", &form).await;
        let status = response.status();
        let cookies = response_cookies(&cookies, &response);
        let body = response.text().await.unwrap_or_default();
        let operation_id = hidden_field(&body, "operation_id");
        let confirm_token = hidden_field(&body, "authenticity_token");
        if status.is_success() && !operation_id.is_empty() {
            let (confirmed, outcome) =
                browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
            assert!(
                !confirmed.is_success(),
                "invalid settings must not complete: {outcome}"
            );
        } else {
            assert!(!status.is_success(), "unexpected refusal body: {body}");
        }
    }
    assert!(app_settings(&app).await.is_none());
    app.close().await;
}

#[tokio::test]
async fn platform_settings_audit_failure_rolls_back() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let (cookies, body) = form_page(&app, &cookie, "/platform/settings").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/settings",
        &[
            ("invite_only", "true"),
            (
                "medicine_lookup_base_url",
                "https://lookup.example.test/fhir",
            ),
            (
                "medicine_lookup_token_url",
                "https://lookup.example.test/token",
            ),
            ("medicine_lookup_source_priority", "nhs_dmd"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let status = response.status();
    let cookies = response_cookies(&cookies, &response);
    let page = response.text().await.unwrap();
    assert_eq!(status, reqwest::StatusCode::OK, "{page}");
    let operation_id = hidden_field(&page, "operation_id");
    let confirm_token = hidden_field(&page, "authenticity_token");
    app.fixture
        .admin
        .execute_unprepared("REVOKE INSERT ON public.versions FROM med_tracker_app")
        .await
        .unwrap();
    let (blocked, _) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    let unchanged = app_settings(&app).await.map(|row| row.1);
    app.fixture
        .admin
        .execute_unprepared("GRANT INSERT ON public.versions TO med_tracker_app")
        .await
        .unwrap();
    let (retried, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    let applied = app_settings(&app).await.map(|row| row.1);
    app.close().await;
    assert!(!blocked.is_success());
    assert_eq!(unchanged, None);
    assert!(
        retried.is_success(),
        "proof must survive an audit failure: {outcome}"
    );
    assert_eq!(applied, Some("https://lookup.example.test/fhir".to_owned()));
}

async fn household(app: &Application, id: i64, slug: &str, name: &str) {
    app.fixture.admin.execute_unprepared(&format!("INSERT INTO households(id,created_by_account_id,name,slug,timezone,status,lifecycle_state,created_at,updated_at) VALUES({id},71001,'{name}','{slug}','Europe/London','active','active',now(),now())")).await.unwrap();
}

async fn household_member(
    app: &Application,
    ids: (i64, i64, i64, i64, i64),
    email: &str,
    name: &str,
    role: &str,
) {
    let (account_id, person_id, user_id, membership_id, household_id) = ids;
    app.fixture.admin.execute_unprepared(&format!("INSERT INTO accounts(id,email,password_hash,status,created_at,updated_at) VALUES({account_id},'{email}',crypt('password',gen_salt('bf',4)),2,now(),now())")).await.unwrap();
    app.fixture.admin.execute_unprepared(&format!("INSERT INTO people(id,account_id,household_id,name,person_type,has_capacity,created_at,updated_at) VALUES({person_id},{account_id},{household_id},'{name}',0,true,now(),now())")).await.unwrap();
    app.fixture.admin.execute_unprepared(&format!("INSERT INTO users(id,person_id,email_address,password_digest,active,created_at,updated_at) VALUES({user_id},{person_id},'{email}',crypt('password',gen_salt('bf',4)),true,now(),now())")).await.unwrap();
    app.fixture.admin.execute_unprepared(&format!("INSERT INTO household_memberships(id,account_id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES({membership_id},{account_id},{household_id},{person_id},'{role}','active',now(),now(),now())")).await.unwrap();
}

async fn recovery_household(app: &Application) {
    household(app, 72601, "recovery-fixture", "Recovery household").await;
    household_member(
        app,
        (71601, 73601, 77601, 74601, 72601),
        "recovery-member@example.test",
        "Recovery member",
        "member",
    )
    .await;
}

async fn supported_household(app: &Application) {
    household(app, 72602, "supported-fixture", "Supported household").await;
    household_member(
        app,
        (71605, 73605, 77605, 74605, 72602),
        "support-owner@example.test",
        "Supported owner",
        "owner",
    )
    .await;
    household_member(
        app,
        (71611, 73611, 77611, 74611, 72602),
        "supported-member@example.test",
        "Supported member",
        "member",
    )
    .await;
    app.fixture.admin.execute_unprepared("INSERT INTO locations(id,household_id,name,created_at,updated_at) VALUES(79601,72602,'Supported cabinet',now(),now())").await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at) VALUES(82601,72602,79601,'Supported tablets',10,2,'tablet',now(),now())").await.unwrap();
}

async fn membership_state(app: &Application, id: i64) -> Option<(String, i32)> {
    app.fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT role,permissions_version FROM household_memberships WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .map(|row| {
            (
                row.try_get::<String>("", "role").unwrap(),
                row.try_get::<i32>("", "permissions_version").unwrap(),
            )
        })
}

async fn membership_count(app: &Application, household_id: i64) -> i64 {
    app.fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM household_memberships WHERE household_id=$1",
            [household_id.into()],
        ))
        .await
        .unwrap()
        .and_then(|row| row.try_get::<i64>("", "count").ok())
        .unwrap_or_default()
}

async fn support_session(
    app: &Application,
    household_id: i64,
) -> Option<(i64, String, Option<String>, Option<String>)> {
    app.fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT id,expires_at::text AS expires_at,ended_at::text AS ended_at,expired_at::text AS expired_at FROM support_access_sessions WHERE household_id=$1 ORDER BY id DESC LIMIT 1",
            [household_id.into()],
        ))
        .await
        .unwrap()
        .map(|row| {
            (
                row.try_get::<i64>("", "id").unwrap(),
                row.try_get::<String>("", "expires_at").unwrap(),
                row.try_get::<Option<String>>("", "ended_at").unwrap(),
                row.try_get::<Option<String>>("", "expired_at").unwrap(),
            )
        })
}

async fn support_window(app: &Application, id: i64) -> Option<(bool, bool, bool, bool)> {
    app.fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT (expires_at BETWEEN now() + interval '23 hours' AND now() + interval '25 hours') AS request_deadline,(expires_at BETWEEN now() + interval '29 minutes' AND now() + interval '31 minutes') AS active_window,(starts_at <= now()) AS started,(ended_at IS NULL AND expired_at IS NULL) AS open FROM support_access_sessions WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .map(|row| {
            (
                row.try_get::<bool>("", "request_deadline").unwrap(),
                row.try_get::<bool>("", "active_window").unwrap(),
                row.try_get::<bool>("", "started").unwrap(),
                row.try_get::<bool>("", "open").unwrap(),
            )
        })
}

async fn support_platform_admin(app: &Application, id: i64) -> Option<i64> {
    app.fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT platform_admin_id FROM support_access_sessions WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .and_then(|row| row.try_get::<i64>("", "platform_admin_id").ok())
}

async fn start_browser_proof(
    response: reqwest::Response,
    cookies: &str,
) -> (String, String, String) {
    let status = response.status();
    let cookies = response_cookies(cookies, &response);
    let page = response.text().await.unwrap();
    assert_eq!(status, reqwest::StatusCode::OK, "{page}");
    let operation_id = hidden_field(&page, "operation_id");
    let confirm_token = hidden_field(&page, "authenticity_token");
    assert!(
        !operation_id.is_empty(),
        "expected a proof confirmation page: {page}"
    );
    (cookies, operation_id, confirm_token)
}

async fn request_support(
    app: &Application,
    admin_cookie: &str,
    household_id: i64,
    reason: &str,
) -> (String, String, String) {
    let (cookies, body) = form_page(app, admin_cookie, "/platform/support").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        app,
        &cookies,
        "/platform/support",
        &[
            ("household_id", &household_id.to_string()),
            ("reason", reason),
            ("authenticity_token", &token),
        ],
    )
    .await;
    start_browser_proof(response, &cookies).await
}

async fn submit_support_form(
    app: &Application,
    cookie: &str,
    list_path: &str,
    action_path: &str,
    support_id: i64,
) -> reqwest::StatusCode {
    let (cookies, body) = form_page(app, cookie, list_path).await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        app,
        &cookies,
        action_path,
        &[
            ("support_id", &support_id.to_string()),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let status = response.status();
    let cookies = response_cookies(&cookies, &response);
    let page = response.text().await.unwrap();
    let operation_id = hidden_field(&page, "operation_id");
    if status == reqwest::StatusCode::OK && !operation_id.is_empty() {
        let confirm_token = hidden_field(&page, "authenticity_token");
        browser_confirm(app, &cookies, &operation_id, &confirm_token)
            .await
            .0
    } else {
        status
    }
}

async fn approve_support(
    app: &Application,
    owner_cookie: &str,
    support_id: i64,
) -> reqwest::StatusCode {
    submit_support_form(
        app,
        owner_cookie,
        "/account/support",
        "/account/support/approve",
        support_id,
    )
    .await
}

async fn activate_support(
    app: &Application,
    admin_cookie: &str,
    support_id: i64,
) -> reqwest::StatusCode {
    submit_support_form(
        app,
        admin_cookie,
        "/platform/support",
        "/platform/support/activate",
        support_id,
    )
    .await
}

async fn active_support(app: &Application, admin_cookie: &str) -> i64 {
    supported_household(app).await;
    let owner = sign_in(app, 71605, "support-owner@example.test").await;
    let (cookies, operation_id, confirm_token) = request_support(
        app,
        admin_cookie,
        72602,
        "Confidential support reason alpha",
    )
    .await;
    let (confirmed, outcome) = browser_confirm(app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) = support_session(app, 72602).await.unwrap();
    let approved = approve_support(app, &owner, support_id).await;
    assert_eq!(approved, reqwest::StatusCode::OK);
    let activated = activate_support(app, admin_cookie, support_id).await;
    assert_eq!(activated, reqwest::StatusCode::OK);
    support_id
}

#[tokio::test]
async fn platform_owner_recovery_page_requires_platform_administrator() {
    let app = Application::new().await;
    recovery_household(&app).await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let anonymous = client
        .get(format!("{}/platform/owner-recovery", app.origin))
        .send()
        .await
        .unwrap();
    assert!(anonymous.status().is_redirection());
    let member = client
        .get(format!("{}/platform/owner-recovery", app.origin))
        .header("cookie", session(&app).await)
        .send()
        .await
        .unwrap();
    assert_eq!(member.status(), reqwest::StatusCode::FORBIDDEN);
    let cookie = admin_session(&app).await;
    let (status, body) = {
        let response = app
            .client
            .get(format!("{}/platform/owner-recovery", app.origin))
            .header("cookie", &cookie)
            .send()
            .await
            .unwrap();
        (response.status(), response.text().await.unwrap())
    };
    let (detail_status, detail) = {
        let response = app
            .client
            .get(format!(
                "{}/platform/owner-recovery?household=72601",
                app.origin
            ))
            .header("cookie", &cookie)
            .send()
            .await
            .unwrap();
        (response.status(), response.text().await.unwrap())
    };
    app.close().await;
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    assert!(body.contains("Recovery household"), "{body}");
    assert!(
        !body.contains("Recovery member"),
        "the selection list must not expand household members: {body}"
    );
    assert_eq!(detail_status, reqwest::StatusCode::OK, "{detail}");
    assert!(detail.contains("Recovery member"), "{detail}");
}

#[tokio::test]
async fn platform_owner_recovery_promotes_eligible_member_via_browser_proof() {
    let app = Application::new().await;
    recovery_household(&app).await;
    let cookie = admin_session(&app).await;
    let before = membership_count(&app, 72601).await;
    let (cookies, body) =
        form_page(&app, &cookie, "/platform/owner-recovery?household=72601").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/owner-recovery",
        &[
            ("household_id", "72601"),
            ("membership_id", "74601"),
            ("reason", "Household owner left the platform"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let (cookies, operation_id, confirm_token) = start_browser_proof(response, &cookies).await;
    assert_eq!(
        membership_state(&app, 74601).await.map(|row| row.0),
        Some("member".to_owned()),
        "no mutation at proof start"
    );
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let promoted = membership_state(&app, 74601).await.unwrap();
    let count = membership_count(&app, 72601).await;
    let audits = platform_version_events(&app, "HouseholdMembership").await;
    app.close().await;
    assert_eq!(promoted.0, "owner");
    assert!(promoted.1 > 1, "permissions_version must be bumped");
    assert_eq!(count, before, "recovery must not create memberships");
    assert!(
        audits > 0,
        "owner recovery must record a versions audit event"
    );
}

#[tokio::test]
async fn platform_owner_recovery_resulting_owner_authority_is_immediate() {
    let app = Application::new().await;
    recovery_household(&app).await;
    let cookie = admin_session(&app).await;
    let (cookies, body) =
        form_page(&app, &cookie, "/platform/owner-recovery?household=72601").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/owner-recovery",
        &[
            ("household_id", "72601"),
            ("membership_id", "74601"),
            ("reason", "Household owner left the platform"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let (cookies, operation_id, confirm_token) = start_browser_proof(response, &cookies).await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let owner = sign_in(&app, 71601, "recovery-member@example.test").await;
    let admin_page = app
        .client
        .get(format!("{}/households/recovery-fixture/admin", app.origin))
        .header("cookie", owner)
        .send()
        .await
        .unwrap()
        .status();
    app.close().await;
    assert_eq!(admin_page, reqwest::StatusCode::OK);
}

#[tokio::test]
async fn platform_owner_recovery_requires_csrf() {
    let app = Application::new().await;
    recovery_household(&app).await;
    let cookie = admin_session(&app).await;
    let response = post_form(
        &app,
        &cookie,
        "/platform/owner-recovery",
        &[
            ("household_id", "72601"),
            ("membership_id", "74601"),
            ("reason", "no token"),
        ],
    )
    .await;
    let status = response.status();
    let role = membership_state(&app, 74601).await.map(|row| row.0);
    app.close().await;
    assert!(status.is_client_error(), "{status}");
    assert_eq!(role, Some("member".to_owned()));
}

#[tokio::test]
async fn platform_owner_recovery_refuses_self_promotion() {
    let app = Application::new().await;
    recovery_household(&app).await;
    let cookie = admin_session(&app).await;
    let (cookies, body) =
        form_page(&app, &cookie, "/platform/owner-recovery?household=72601").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/owner-recovery",
        &[
            ("household_id", "72001"),
            ("membership_id", "74001"),
            ("reason", "self promotion"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let status = response.status();
    let cookies = response_cookies(&cookies, &response);
    let page = response.text().await.unwrap();
    let operation_id = hidden_field(&page, "operation_id");
    if status == reqwest::StatusCode::OK && !operation_id.is_empty() {
        let confirm_token = hidden_field(&page, "authenticity_token");
        let (confirmed, outcome) =
            browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
        assert!(
            !confirmed.is_success(),
            "self-promotion must fail: {outcome}"
        );
    } else {
        assert!(
            status.is_client_error(),
            "self-promotion POST must be rejected: {status} {page}"
        );
    }
    let role = membership_state(&app, 74001).await.map(|row| row.0);
    app.close().await;
    assert_eq!(role, Some("administrator".to_owned()));
}

#[tokio::test]
async fn platform_owner_recovery_refuses_ineligible_members() {
    let app = Application::new().await;
    recovery_household(&app).await;
    household_member(
        &app,
        (71603, 73603, 77603, 74603, 72601),
        "recovery-minor@example.test",
        "Recovery minor",
        "member",
    )
    .await;
    household_member(
        &app,
        (71604, 73604, 77604, 74604, 72601),
        "recovery-dependent@example.test",
        "Recovery dependent",
        "member",
    )
    .await;
    household_member(
        &app,
        (71606, 73606, 77606, 74606, 72601),
        "recovery-nocapacity@example.test",
        "Recovery no capacity",
        "member",
    )
    .await;
    household_member(
        &app,
        (71607, 73607, 77607, 74607, 72601),
        "recovery-inactive@example.test",
        "Recovery inactive",
        "member",
    )
    .await;
    household_member(
        &app,
        (71608, 73608, 77608, 74608, 72601),
        "recovery-revoked@example.test",
        "Recovery revoked",
        "member",
    )
    .await;
    household_member(
        &app,
        (71609, 73609, 77609, 74609, 72601),
        "recovery-closed@example.test",
        "Recovery closed",
        "member",
    )
    .await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE people SET person_type=1,has_capacity=false WHERE id=73603")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("UPDATE people SET person_type=2,has_capacity=false WHERE id=73604")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("UPDATE people SET has_capacity=false WHERE id=73606")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("UPDATE users SET active=false WHERE id=77607")
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE id=74608",
        )
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("UPDATE accounts SET status=3 WHERE id=71609")
        .await
        .unwrap();
    let cookie = admin_session(&app).await;
    for (membership, label) in [
        (74603_i64, "minor"),
        (74604, "dependent adult"),
        (74606, "no capacity"),
        (74607, "inactive user"),
        (74608, "revoked membership"),
        (74609, "closed account"),
    ] {
        let (cookies, body) =
            form_page(&app, &cookie, "/platform/owner-recovery?household=72601").await;
        let token = hidden_field(&body, "authenticity_token");
        let response = post_form(
            &app,
            &cookies,
            "/platform/owner-recovery",
            &[
                ("household_id", "72601"),
                ("membership_id", &membership.to_string()),
                ("reason", "recovery attempt"),
                ("authenticity_token", &token),
            ],
        )
        .await;
        let status = response.status();
        let cookies = response_cookies(&cookies, &response);
        let page = response.text().await.unwrap();
        let operation_id = hidden_field(&page, "operation_id");
        if status == reqwest::StatusCode::OK && !operation_id.is_empty() {
            let confirm_token = hidden_field(&page, "authenticity_token");
            let (confirmed, outcome) =
                browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
            assert!(
                !confirmed.is_success(),
                "{label} promotion must fail: {outcome}"
            );
        } else {
            assert!(
                status.is_client_error(),
                "{label} POST must be rejected: {status} {page}"
            );
        }
        let role = membership_state(&app, membership).await.map(|row| row.0);
        assert_eq!(
            role,
            Some("member".to_owned()),
            "{label} must not become owner"
        );
    }
    app.close().await;
}

#[tokio::test]
async fn platform_owner_recovery_refuses_healthy_owner_household() {
    let app = Application::new().await;
    supported_household(&app).await;
    let cookie = admin_session(&app).await;
    let (cookies, body) = form_page(&app, &cookie, "/platform/users").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/owner-recovery",
        &[
            ("household_id", "72602"),
            ("membership_id", "74611"),
            ("reason", "healthy household"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let status = response.status();
    let next = response_cookies(&cookies, &response);
    let page = response.text().await.unwrap();
    let operation_id = hidden_field(&page, "operation_id");
    if status == reqwest::StatusCode::OK && !operation_id.is_empty() {
        let confirm_token = hidden_field(&page, "authenticity_token");
        let (confirmed, outcome) =
            browser_confirm(&app, &next, &operation_id, &confirm_token).await;
        assert!(
            !confirmed.is_success(),
            "healthy-owner recovery must fail: {outcome}"
        );
    } else {
        assert!(
            status.is_client_error(),
            "healthy-owner POST must be rejected: {status} {page}"
        );
    }
    let role = membership_state(&app, 74611).await.map(|row| row.0);
    app.close().await;
    assert_eq!(role, Some("member".to_owned()));
}

#[tokio::test]
async fn platform_owner_recovery_refuses_cross_household_target() {
    let app = Application::new().await;
    recovery_household(&app).await;
    supported_household(&app).await;
    let cookie = admin_session(&app).await;
    let (cookies, body) =
        form_page(&app, &cookie, "/platform/owner-recovery?household=72601").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/owner-recovery",
        &[
            ("household_id", "72602"),
            ("membership_id", "74601"),
            ("reason", "cross household"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let status = response.status();
    let next = response_cookies(&cookies, &response);
    let page = response.text().await.unwrap();
    let operation_id = hidden_field(&page, "operation_id");
    if status == reqwest::StatusCode::OK && !operation_id.is_empty() {
        let confirm_token = hidden_field(&page, "authenticity_token");
        let (confirmed, outcome) =
            browser_confirm(&app, &next, &operation_id, &confirm_token).await;
        assert!(
            !confirmed.is_success(),
            "cross-household recovery must fail: {outcome}"
        );
    } else {
        assert!(
            status.is_client_error(),
            "cross-household POST must be rejected: {status} {page}"
        );
    }
    let role = membership_state(&app, 74601).await.map(|row| row.0);
    app.close().await;
    assert_eq!(role, Some("member".to_owned()));
}

#[tokio::test]
async fn platform_owner_recovery_refused_when_admin_rights_withdrawn_before_confirm() {
    let app = Application::new().await;
    recovery_household(&app).await;
    let cookie = admin_session(&app).await;
    let (cookies, body) =
        form_page(&app, &cookie, "/platform/owner-recovery?household=72601").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/owner-recovery",
        &[
            ("household_id", "72601"),
            ("membership_id", "74601"),
            ("reason", "recovery"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let (cookies, operation_id, confirm_token) = start_browser_proof(response, &cookies).await;
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE platform_admins SET status='revoked',updated_at=now() WHERE account_id=71001",
        )
        .await
        .unwrap();
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    let role = membership_state(&app, 74601).await.map(|row| row.0);
    app.close().await;
    assert!(
        !confirmed.is_success(),
        "withdrawn authority must fail: {outcome}"
    );
    assert_eq!(role, Some("member".to_owned()));
}

#[tokio::test]
async fn platform_owner_recovery_proof_binds_household_target_and_reason() {
    let app = Application::new().await;
    recovery_household(&app).await;
    household_member(
        &app,
        (71602, 73602, 77602, 74602, 72601),
        "recovery-other@example.test",
        "Recovery other",
        "member",
    )
    .await;
    let cookie = admin_session(&app).await;
    let (cookies, body) =
        form_page(&app, &cookie, "/platform/owner-recovery?household=72601").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/owner-recovery",
        &[
            ("household_id", "72601"),
            ("membership_id", "74601"),
            ("reason", "recovery reason alpha"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let (cookies, operation_id, confirm_token) = start_browser_proof(response, &cookies).await;
    let substituted = post_form(
        &app,
        &cookies,
        "/account/security/password/confirm",
        &[
            ("operation_id", &operation_id),
            ("password", "password"),
            ("authenticity_token", &confirm_token),
            ("household_id", "72602"),
            ("membership_id", "74602"),
            ("reason", "substituted reason"),
        ],
    )
    .await
    .status();
    let first = membership_state(&app, 74601).await.map(|row| row.0);
    let second = membership_state(&app, 74602).await.map(|row| row.0);
    let applied = if substituted.is_success() {
        first == Some("owner".to_owned()) && second == Some("member".to_owned())
    } else {
        let (confirmed, _) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
        confirmed.is_success()
            && membership_state(&app, 74601).await.map(|row| row.0) == Some("owner".to_owned())
            && membership_state(&app, 74602).await.map(|row| row.0) == Some("member".to_owned())
    };
    app.close().await;
    assert!(
        applied,
        "proof must apply only its bound household/membership/reason"
    );
}

#[tokio::test]
async fn platform_owner_recovery_serializes_concurrent_confirmations() {
    let app = Application::new().await;
    recovery_household(&app).await;
    household_member(
        &app,
        (71602, 73602, 77602, 74602, 72601),
        "recovery-other@example.test",
        "Recovery other",
        "member",
    )
    .await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    app.fixture.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71002,'active',now(),now())").await.unwrap();
    let first = admin_session(&app).await;
    let second = sign_in(&app, 71002, "platform-second@example.test").await;
    let mut proofs = Vec::new();
    for (cookie, membership) in [(&first, 74601), (&second, 74602)] {
        let (cookies, body) =
            form_page(&app, cookie, "/platform/owner-recovery?household=72601").await;
        let token = hidden_field(&body, "authenticity_token");
        let response = post_form(
            &app,
            &cookies,
            "/platform/owner-recovery",
            &[
                ("household_id", "72601"),
                ("membership_id", &membership.to_string()),
                ("reason", "concurrent recovery"),
                ("authenticity_token", &token),
            ],
        )
        .await;
        proofs.push(start_browser_proof(response, &cookies).await);
    }
    let (cookies_a, op_a, token_a) = proofs[0].clone();
    let (cookies_b, op_b, token_b) = proofs[1].clone();
    let (won, outcome) = browser_confirm(&app, &cookies_a, &op_a, &token_a).await;
    assert_eq!(won, reqwest::StatusCode::OK, "{outcome}");
    let (lost, outcome) = browser_confirm(&app, &cookies_b, &op_b, &token_b).await;
    let first_role = membership_state(&app, 74601).await.map(|row| row.0);
    let second_role = membership_state(&app, 74602).await.map(|row| row.0);
    let owners = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM household_memberships WHERE household_id=72601 AND role='owner' AND status='active' AND revoked_at IS NULL",
            [],
        ))
        .await
        .unwrap()
        .and_then(|row| row.try_get::<i64>("", "count").ok())
        .unwrap_or_default();
    app.close().await;
    assert!(
        !lost.is_success(),
        "second recovery must lose the race: {outcome}"
    );
    assert_eq!(first_role, Some("owner".to_owned()));
    assert_eq!(second_role, Some("member".to_owned()));
    assert_eq!(owners, 1);
}

#[tokio::test]
async fn platform_owner_recovery_audit_failure_rolls_back() {
    let app = Application::new().await;
    recovery_household(&app).await;
    let cookie = admin_session(&app).await;
    let (cookies, body) =
        form_page(&app, &cookie, "/platform/owner-recovery?household=72601").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/owner-recovery",
        &[
            ("household_id", "72601"),
            ("membership_id", "74601"),
            ("reason", "recovery"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let (cookies, operation_id, confirm_token) = start_browser_proof(response, &cookies).await;
    app.fixture
        .admin
        .execute_unprepared("REVOKE INSERT ON public.versions FROM med_tracker_app")
        .await
        .unwrap();
    let (blocked, _) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    let unchanged = membership_state(&app, 74601).await.map(|row| row.0);
    app.fixture
        .admin
        .execute_unprepared("GRANT INSERT ON public.versions TO med_tracker_app")
        .await
        .unwrap();
    let (retried, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    let applied = membership_state(&app, 74601).await.map(|row| row.0);
    app.close().await;
    assert!(!blocked.is_success());
    assert_eq!(unchanged, Some("member".to_owned()));
    assert!(
        retried.is_success(),
        "proof must survive an audit failure: {outcome}"
    );
    assert_eq!(applied, Some("owner".to_owned()));
}

#[tokio::test]
async fn platform_support_requires_platform_administrator() {
    let app = Application::new().await;
    supported_household(&app).await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let anonymous = client
        .get(format!("{}/platform/support", app.origin))
        .send()
        .await
        .unwrap();
    assert!(anonymous.status().is_redirection());
    let member = client
        .get(format!("{}/platform/support", app.origin))
        .header("cookie", session(&app).await)
        .send()
        .await
        .unwrap();
    assert_eq!(member.status(), reqwest::StatusCode::FORBIDDEN);
    let cookie = admin_session(&app).await;
    let response = app
        .client
        .get(format!("{}/platform/support", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    app.close().await;
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
}

#[tokio::test]
async fn platform_support_pagination_keeps_older_approved_requests_reachable() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    app.fixture.admin.execute_unprepared(&format!("UPDATE support_access_sessions SET activated_at=NULL WHERE id={support_id}; INSERT INTO support_access_sessions(household_id,platform_admin_id,reason,request_id,ip,starts_at,expires_at,ended_at,created_at,updated_at) SELECT household_id,platform_admin_id,'Newer support request '||g,request_id,ip,starts_at,expires_at,now(),now(),now() FROM support_access_sessions CROSS JOIN generate_series(1,50) g WHERE id={support_id}")).await.unwrap();
    let (_, first) = form_page(&app, &cookie, "/platform/support").await;
    let (_, second) = form_page(&app, &cookie, "/platform/support?page=2").await;
    let (_, oversized) =
        form_page(&app, &cookie, "/platform/support?page=9223372036854775807").await;
    app.close().await;
    assert!(first.contains("href=\"/platform/support?page=2\""));
    assert!(!first.contains("Confidential support reason alpha"));
    assert!(second.contains("Confidential support reason alpha"));
    assert!(second.contains("action=\"/platform/support/activate\""));
    assert!(second.contains(&format!("name=\"support_id\" value=\"{support_id}\"")));
    assert!(oversized.contains("Confidential support reason alpha"));
}

#[tokio::test]
async fn platform_support_request_requires_reason_and_sets_request_deadline() {
    let app = Application::new().await;
    supported_household(&app).await;
    let cookie = admin_session(&app).await;
    let (cookies, body) = form_page(&app, &cookie, "/platform/support").await;
    let token = hidden_field(&body, "authenticity_token");
    let missing = post_form(
        &app,
        &cookies,
        "/platform/support",
        &[("household_id", "72602"), ("authenticity_token", &token)],
    )
    .await;
    let missing_status = missing.status();
    let missing_cookies = response_cookies(&cookies, &missing);
    let missing_page = missing.text().await.unwrap();
    let missing_operation = hidden_field(&missing_page, "operation_id");
    if missing_status == reqwest::StatusCode::OK && !missing_operation.is_empty() {
        let confirm_token = hidden_field(&missing_page, "authenticity_token");
        let (confirmed, _) =
            browser_confirm(&app, &missing_cookies, &missing_operation, &confirm_token).await;
        assert!(
            !confirmed.is_success(),
            "reasonless request must not complete"
        );
    } else {
        assert!(
            missing_status.is_client_error(),
            "reasonless request must be rejected: {missing_status} {missing_page}"
        );
    }
    assert!(
        support_session(&app, 72602).await.is_none(),
        "no row without reason"
    );
    let (cookies, body) = form_page(&app, &cookie, "/platform/support").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/support",
        &[
            ("household_id", "72602"),
            ("reason", "Confidential support reason alpha"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let (cookies, operation_id, confirm_token) = start_browser_proof(response, &cookies).await;
    assert!(
        support_session(&app, 72602).await.is_none(),
        "no row at proof start"
    );
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, _, ended, expired) = support_session(&app, 72602).await.unwrap();
    let (deadline, _, _, open) = support_window(&app, support_id).await.unwrap();
    let admin_id = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT id FROM platform_admins WHERE account_id=71001",
            [],
        ))
        .await
        .unwrap()
        .and_then(|row| row.try_get::<i64>("", "id").ok())
        .unwrap();
    let stored_admin = support_platform_admin(&app, support_id).await;
    app.close().await;
    assert_eq!(stored_admin, Some(admin_id));
    assert!(deadline, "request deadline must be 24 hours");
    assert!(open, "fresh request must not be ended or expired");
    assert_eq!(ended, None);
    assert_eq!(expired, None);
}

#[tokio::test]
async fn platform_support_request_proof_binds_household_and_reason() {
    let app = Application::new().await;
    supported_household(&app).await;
    recovery_household(&app).await;
    let cookie = admin_session(&app).await;
    let (cookies, operation_id, confirm_token) =
        request_support(&app, &cookie, 72602, "Confidential support reason alpha").await;
    let substituted = post_form(
        &app,
        &cookies,
        "/account/security/password/confirm",
        &[
            ("operation_id", &operation_id),
            ("password", "password"),
            ("authenticity_token", &confirm_token),
            ("household_id", "72601"),
            ("reason", "substituted reason"),
        ],
    )
    .await
    .status();
    if !substituted.is_success() {
        let (confirmed, outcome) =
            browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
        assert!(confirmed.is_success(), "{outcome}");
    }
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT household_id,reason FROM support_access_sessions ORDER BY id DESC LIMIT 1",
            [],
        ))
        .await
        .unwrap()
        .map(|row| {
            (
                row.try_get::<i64>("", "household_id").unwrap(),
                row.try_get::<String>("", "reason").unwrap(),
            )
        })
        .unwrap();
    app.close().await;
    assert_eq!(row.0, 72602, "proof must keep the bound household");
    assert_eq!(row.1, "Confidential support reason alpha");
}

#[tokio::test]
async fn platform_support_owner_approves_and_admin_activates() {
    let app = Application::new().await;
    supported_household(&app).await;
    let cookie = admin_session(&app).await;
    let owner = sign_in(&app, 71605, "support-owner@example.test").await;
    let member = sign_in(&app, 71611, "supported-member@example.test").await;
    let nonowner = app
        .client
        .get(format!("{}/account/support", app.origin))
        .header("cookie", &member)
        .send()
        .await
        .unwrap();
    assert_eq!(nonowner.status(), reqwest::StatusCode::FORBIDDEN);
    let (cookies, operation_id, confirm_token) =
        request_support(&app, &cookie, 72602, "Confidential support reason alpha").await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) = support_session(&app, 72602).await.unwrap();
    let page = app
        .client
        .get(format!("{}/account/support", app.origin))
        .header("cookie", &owner)
        .send()
        .await
        .unwrap();
    let status = page.status();
    let body = page.text().await.unwrap();
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    assert!(
        body.contains(&support_id.to_string()) || body.contains("Supported household"),
        "{body}"
    );
    let approved = approve_support(&app, &owner, support_id).await;
    assert_eq!(approved, reqwest::StatusCode::OK);
    let activated = activate_support(&app, &cookie, support_id).await;
    assert_eq!(activated, reqwest::StatusCode::OK);
    let (_, window, started, open) = support_window(&app, support_id).await.unwrap();
    app.close().await;
    assert!(started, "activation must stamp starts_at");
    assert!(
        window,
        "activation window must be 30 minutes from database time"
    );
    assert!(open);
}

#[tokio::test]
async fn platform_support_cannot_activate_without_owner_approval() {
    let app = Application::new().await;
    supported_household(&app).await;
    let cookie = admin_session(&app).await;
    let (cookies, operation_id, confirm_token) =
        request_support(&app, &cookie, 72602, "Confidential support reason alpha").await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) = support_session(&app, 72602).await.unwrap();
    let activated = activate_support(&app, &cookie, support_id).await;
    let (_, window, _, _) = support_window(&app, support_id).await.unwrap_or_default();
    app.close().await;
    assert!(
        !activated.is_success(),
        "unapproved request must not activate"
    );
    assert!(
        !window,
        "unapproved request must not gain a 30 minute window"
    );
}

#[tokio::test]
async fn platform_support_activate_replay_and_wrong_operator_rejected() {
    let app = Application::new().await;
    synthetic_account(&app, 71002, 73004, 77002, "platform-second@example.test", 2).await;
    app.fixture.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71002,'active',now(),now())").await.unwrap();
    let cookie = admin_session(&app).await;
    let other = sign_in(&app, 71002, "platform-second@example.test").await;
    let support_id = active_support(&app, &cookie).await;
    let wrong = activate_support(&app, &other, support_id).await;
    let replay = activate_support(&app, &cookie, support_id).await;
    let (_, window, _, open) = support_window(&app, support_id).await.unwrap();
    app.close().await;
    assert!(
        !wrong.is_success(),
        "a different admin must not activate this request"
    );
    assert!(!replay.is_success(), "activation must not be replayable");
    assert!(window && open);
}

#[tokio::test]
async fn platform_support_read_is_a_read_only_projection() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    let read = app
        .client
        .get(format!("{}/platform/support/{support_id}", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    let status = read.status();
    let body = read.text().await.unwrap();
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    assert!(body.contains("Supported household"), "{body}");
    assert!(body.contains("Supported member"), "{body}");
    assert!(body.contains("Supported tablets"), "{body}");
    let memberships = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM household_memberships WHERE household_id=72602 AND account_id=71001",
            [],
        ))
        .await
        .unwrap()
        .and_then(|row| row.try_get::<i64>("", "count").ok())
        .unwrap_or_default();
    assert_eq!(
        memberships, 0,
        "support must not insert a household membership"
    );
    let write = post_form(
        &app,
        &cookie,
        "/households/supported-fixture/admin/users/74605/membership_role",
        &[("household_membership[role]", "member")],
    )
    .await;
    assert!(
        !write.status().is_success(),
        "membership writes stay denied under support"
    );
    let download = app
        .client
        .get(format!(
            "{}/households/supported-fixture/reports/health-history",
            app.origin
        ))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert!(
        !download.status().is_success(),
        "exports/downloads stay denied under support"
    );
    let role = membership_state(&app, 74605).await.map(|row| row.0);
    app.close().await;
    assert_eq!(role, Some("owner".to_owned()));
}

#[tokio::test]
async fn platform_support_end_is_idempotent_and_audited() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    let owner = sign_in(&app, 71605, "support-owner@example.test").await;
    let (cookies, body) = form_page(&app, &owner, "/account/support").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/account/support/end",
        &[
            ("support_id", &support_id.to_string()),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let status = response.status();
    let cookies = response_cookies(&cookies, &response);
    let page = response.text().await.unwrap();
    let operation_id = hidden_field(&page, "operation_id");
    if !operation_id.is_empty() {
        let confirm_token = hidden_field(&page, "authenticity_token");
        let (ended, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
        assert!(
            ended.is_success(),
            "owner termination must succeed: {outcome}"
        );
    } else {
        assert!(
            status.is_success(),
            "owner termination must succeed: {status} {page}"
        );
    }
    let first = support_session(&app, 72602).await.unwrap();
    assert!(first.2.is_some(), "ending must stamp ended_at");
    let read = app
        .client
        .get(format!("{}/platform/support/{support_id}", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert!(
        !read.status().is_success(),
        "terminated support must not read"
    );
    let (cookies, body) = form_page(&app, &cookie, "/platform/support").await;
    let token = hidden_field(&body, "authenticity_token");
    let again = post_form(
        &app,
        &cookies,
        "/platform/support/end",
        &[
            ("support_id", &support_id.to_string()),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let status = again.status();
    let cookies = response_cookies(&cookies, &again);
    let page = again.text().await.unwrap();
    let operation_id = hidden_field(&page, "operation_id");
    if !operation_id.is_empty() {
        let confirm_token = hidden_field(&page, "authenticity_token");
        let (confirmed, outcome) =
            browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
        assert!(
            confirmed.is_success(),
            "re-ending an ended session must be idempotent: {outcome}"
        );
    } else {
        assert!(
            !status.is_server_error(),
            "re-ending must not crash: {status} {page}"
        );
    }
    let second = support_session(&app, 72602).await.unwrap();
    let audits = platform_version_events(&app, "SupportAccessSession").await;
    app.close().await;
    assert_eq!(
        first.2, second.2,
        "ended_at must not move on repeat termination"
    );
    assert!(audits >= 4, "request/approve/activate/end must each audit");
}

#[tokio::test]
async fn platform_support_expiry_revokes_read_access() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "UPDATE support_access_sessions SET expires_at=now() - interval '1 second',updated_at=now() WHERE id={support_id}"
        ))
        .await
        .unwrap();
    let read = app
        .client
        .get(format!("{}/platform/support/{support_id}", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    let denied = read.status();
    let row = support_session(&app, 72602).await.unwrap();
    let again = app
        .client
        .get(format!("{}/platform/support/{support_id}", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap()
        .status();
    app.close().await;
    assert!(!denied.is_success(), "expired support must not read");
    assert!(row.3.is_some(), "expiry must be persisted via expired_at");
    assert!(!again.is_success(), "expiry denial must be stable");
}

#[tokio::test]
async fn platform_support_read_denied_after_authority_changes() {
    for change in [
        "UPDATE platform_admins SET status='revoked',updated_at=now() WHERE account_id=71001",
        "UPDATE household_memberships SET status='revoked',revoked_at=now(),updated_at=now() WHERE id=74605",
        "UPDATE users SET active=false,updated_at=now() WHERE id=77001",
        "UPDATE households SET status='closed',lifecycle_state='closed',updated_at=now() WHERE id=72602",
    ] {
        let app = Application::new().await;
        let cookie = admin_session(&app).await;
        let support_id = active_support(&app, &cookie).await;
        app.fixture.admin.execute_unprepared(change).await.unwrap();
        let read = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap()
            .get(format!("{}/platform/support/{support_id}", app.origin))
            .header("cookie", &cookie)
            .send()
            .await
            .unwrap();
        let status = read.status();
        app.close().await;
        assert!(
            !status.is_success(),
            "support read must stop after: {change}"
        );
    }
}

#[tokio::test]
async fn platform_support_reason_is_not_leaked_in_denied_responses() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE platform_admins SET status='revoked',updated_at=now() WHERE account_id=71001",
        )
        .await
        .unwrap();
    let denied = app
        .client
        .get(format!("{}/platform/support/{support_id}", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    let denied_body = denied.text().await.unwrap();
    let anonymous = app
        .client
        .get(format!("{}/platform/support/{support_id}", app.origin))
        .send()
        .await
        .unwrap();
    let anonymous_body = anonymous.text().await.unwrap();
    app.close().await;
    assert!(!denied_body.contains("Confidential support reason alpha"));
    assert!(!anonymous_body.contains("Confidential support reason alpha"));
}

#[tokio::test]
async fn platform_support_audit_failure_rolls_back_request() {
    let app = Application::new().await;
    supported_household(&app).await;
    let cookie = admin_session(&app).await;
    let (cookies, operation_id, confirm_token) =
        request_support(&app, &cookie, 72602, "Confidential support reason alpha").await;
    app.fixture
        .admin
        .execute_unprepared("REVOKE INSERT ON public.versions FROM med_tracker_app")
        .await
        .unwrap();
    let (blocked, _) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    let absent = support_session(&app, 72602).await.is_none();
    app.fixture
        .admin
        .execute_unprepared("GRANT INSERT ON public.versions TO med_tracker_app")
        .await
        .unwrap();
    let (retried, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    let present = support_session(&app, 72602).await.is_some();
    app.close().await;
    assert!(!blocked.is_success());
    assert!(absent, "audit failure must roll back the request row");
    assert!(
        retried.is_success(),
        "proof must survive an audit failure: {outcome}"
    );
    assert!(present);
}

#[tokio::test]
async fn platform_support_request_deadline_is_enforced() {
    let app = Application::new().await;
    supported_household(&app).await;
    let cookie = admin_session(&app).await;
    let owner = sign_in(&app, 71605, "support-owner@example.test").await;
    let (cookies, operation_id, confirm_token) =
        request_support(&app, &cookie, 72602, "Confidential support reason alpha").await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) = support_session(&app, 72602).await.unwrap();
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "UPDATE support_access_sessions SET expires_at=now() - interval '1 second',updated_at=now() WHERE id={support_id}"
        ))
        .await
        .unwrap();
    let approved = approve_support(&app, &owner, support_id).await;
    let (_, window, _, _) = support_window(&app, support_id).await.unwrap_or_default();
    app.close().await;
    assert!(
        !approved.is_success(),
        "expired requests must not be approvable"
    );
    assert!(!window);
}

async fn start_support_form(
    app: &Application,
    cookie: &str,
    list_path: &str,
    action_path: &str,
    support_id: i64,
) -> (String, String, String) {
    let (cookies, body) = form_page(app, cookie, list_path).await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        app,
        &cookies,
        action_path,
        &[
            ("support_id", &support_id.to_string()),
            ("authenticity_token", &token),
        ],
    )
    .await;
    start_browser_proof(response, &cookies).await
}

async fn membership_lock(
    app: &Application,
    household_id: i64,
    membership_id: i64,
) -> (sea_orm::DatabaseTransaction, i32) {
    let lock = sea_orm::Database::connect(&app.fixture.runtime_uri)
        .await
        .unwrap()
        .begin()
        .await
        .unwrap();
    let holder: i32 = lock
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_backend_pid() AS pid".to_owned(),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    lock.execute_unprepared(&format!(
        "SET LOCAL med_tracker.current_household_id='{household_id}'"
    ))
    .await
    .unwrap();
    lock.execute_unprepared(&format!(
        "SELECT id FROM public.household_memberships WHERE id={membership_id} FOR UPDATE"
    ))
    .await
    .unwrap();
    (lock, holder)
}

async fn wait_blocked_on(app: &Application, holder: i32, support_id: i64) -> bool {
    for _ in 0..60 {
        let row = app
            .fixture
            .admin
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity a WHERE a.datname=current_database() AND $1::int=ANY(pg_blocking_pids(a.pid))) AS waiting,clock_timestamp() < (SELECT expires_at FROM public.support_access_sessions WHERE id=$2) AS before_deadline",
                [holder.into(), support_id.into()],
            ))
            .await
            .unwrap()
            .unwrap();
        if row.try_get::<bool>("", "waiting").unwrap() {
            assert!(
                row.try_get::<bool>("", "before_deadline").unwrap(),
                "the blocked operation must begin before the deadline"
            );
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    false
}

async fn wait_past_deadline(app: &Application, support_id: i64) {
    for _ in 0..200 {
        let past: bool = app
            .fixture
            .admin
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT clock_timestamp() > expires_at AS past FROM public.support_access_sessions WHERE id=$1",
                [support_id.into()],
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "past")
            .unwrap();
        if past {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("the support deadline never passed");
}

async fn support_decision(app: &Application, support_id: i64) -> (Option<String>, Option<String>) {
    app.fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT approved_at::text AS approved_at,activated_at::text AS activated_at FROM support_access_sessions WHERE id=$1",
            [support_id.into()],
        ))
        .await
        .unwrap()
        .map(|row| {
            (
                row.try_get::<Option<String>>("", "approved_at").unwrap(),
                row.try_get::<Option<String>>("", "activated_at").unwrap(),
            )
        })
        .unwrap_or_default()
}

async fn support_audit_count(app: &Application, event: &str, support_id: i64) -> i64 {
    app.fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM versions WHERE event=$1 AND item_type='SupportAccessSession' AND item_id=$2",
            [event.into(), support_id.into()],
        ))
        .await
        .unwrap()
        .and_then(|row| row.try_get::<i64>("", "count").ok())
        .unwrap_or_default()
}

#[tokio::test]
async fn platform_owner_recovery_page_omits_the_administrator_membership() {
    let app = Application::new().await;
    recovery_household(&app).await;
    app.fixture.admin.execute_unprepared("INSERT INTO people(id,account_id,household_id,name,person_type,has_capacity,created_at,updated_at) VALUES(73005,71001,72601,'Administrator recovery person',0,true,now(),now())").await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO users(id,person_id,email_address,password_digest,active,created_at,updated_at) VALUES(77005,73005,'administrator-member@example.test',crypt('password',gen_salt('bf',4)),true,now(),now())").await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO household_memberships(id,account_id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74003,71001,72601,73005,'member','active',now(),now(),now())").await.unwrap();
    let cookie = admin_session(&app).await;
    let (status, detail) = {
        let response = app
            .client
            .get(format!(
                "{}/platform/owner-recovery?household=72601",
                app.origin
            ))
            .header("cookie", &cookie)
            .send()
            .await
            .unwrap();
        (response.status(), response.text().await.unwrap())
    };
    app.close().await;
    assert_eq!(status, reqwest::StatusCode::OK, "{detail}");
    assert!(detail.contains("value=\"74601\""), "{detail}");
    assert!(!detail.contains("value=\"74003\""), "{detail}");
}

#[tokio::test]
async fn platform_owner_recovery_excludes_mismatched_person_ownership() {
    let app = Application::new().await;
    recovery_household(&app).await;
    app.fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,password_hash,status,created_at,updated_at) VALUES(71613,'mismatched-holder@example.test',crypt('password',gen_salt('bf',4)),2,now(),now())").await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO people(id,account_id,household_id,name,person_type,has_capacity,created_at,updated_at) VALUES(73615,71613,72601,'Mismatched person',0,true,now(),now())").await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,password_hash,status,created_at,updated_at) VALUES(71614,'mismatched-member@example.test',crypt('password',gen_salt('bf',4)),2,now(),now())").await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO users(id,person_id,email_address,password_digest,active,created_at,updated_at) VALUES(77614,73615,'mismatched-member@example.test',crypt('password',gen_salt('bf',4)),true,now(),now())").await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO household_memberships(id,account_id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74615,71614,72601,73615,'member','active',now(),now(),now())").await.unwrap();
    let cookie = admin_session(&app).await;
    let (cookies, detail) =
        form_page(&app, &cookie, "/platform/owner-recovery?household=72601").await;
    assert!(detail.contains("value=\"74601\""), "{detail}");
    assert!(!detail.contains("value=\"74615\""), "{detail}");
    let token = hidden_field(&detail, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/owner-recovery",
        &[
            ("household_id", "72601"),
            ("membership_id", "74615"),
            ("reason", "mismatched account person ownership"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let status = response.status();
    let role = membership_state(&app, 74615).await.map(|row| row.0);
    app.close().await;
    assert!(status.is_client_error(), "{status}");
    assert_eq!(role, Some("member".to_owned()));
}

#[tokio::test]
async fn platform_owner_recovery_treats_mismatched_owner_as_ownerless() {
    let app = Application::new().await;
    household(&app, 72603, "mismatched-fixture", "Mismatched household").await;
    app.fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,password_hash,status,created_at,updated_at) VALUES(71602,'mismatched-owner-account@example.test',crypt('password',gen_salt('bf',4)),2,now(),now())").await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,password_hash,status,created_at,updated_at) VALUES(71603,'mismatched-person-account@example.test',crypt('password',gen_salt('bf',4)),2,now(),now())").await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO people(id,account_id,household_id,name,person_type,has_capacity,created_at,updated_at) VALUES(73603,71603,72603,'Mismatched owner person',0,true,now(),now())").await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO users(id,person_id,email_address,password_digest,active,created_at,updated_at) VALUES(77616,73603,'mismatched-owner-user@example.test',crypt('password',gen_salt('bf',4)),true,now(),now())").await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO household_memberships(id,account_id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74604,71602,72603,73603,'owner','active',now(),now(),now())").await.unwrap();
    household_member(
        &app,
        (71614, 73614, 77614, 74614, 72603),
        "mismatched-member@example.test",
        "Mismatched member",
        "member",
    )
    .await;
    let cookie = admin_session(&app).await;
    let (cookies, detail) =
        form_page(&app, &cookie, "/platform/owner-recovery?household=72603").await;
    assert!(detail.contains("Mismatched member"), "{detail}");
    let token = hidden_field(&detail, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/owner-recovery",
        &[
            ("household_id", "72603"),
            ("membership_id", "74614"),
            ("reason", "owner record is unusable"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let (cookies, operation_id, confirm_token) = start_browser_proof(response, &cookie).await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    let promoted = membership_state(&app, 74614).await.map(|row| row.0);
    app.close().await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    assert_eq!(promoted, Some("owner".to_owned()));
}

#[tokio::test]
async fn platform_support_read_waiting_on_row_lock_denies_after_deadline() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "UPDATE public.support_access_sessions SET expires_at=clock_timestamp()+interval '3 seconds' WHERE id={support_id}"
        ))
        .await
        .unwrap();
    let lock = sea_orm::Database::connect(&app.fixture.runtime_uri)
        .await
        .unwrap()
        .begin()
        .await
        .unwrap();
    let holder: i32 = lock
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_backend_pid() AS pid".to_owned(),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    lock.execute_unprepared(&format!(
        "SELECT id FROM public.support_access_sessions WHERE id={support_id} FOR UPDATE"
    ))
    .await
    .unwrap();
    let read_client = app.client.clone();
    let read_origin = app.origin.clone();
    let read_cookie = cookie.clone();
    let pending = tokio::spawn(async move {
        read_client
            .get(format!("{read_origin}/platform/support/{support_id}"))
            .header("cookie", read_cookie)
            .send()
            .await
            .unwrap()
    });
    let mut waiting = false;
    for _ in 0..60 {
        let row = app
            .fixture
            .admin
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity a WHERE a.datname=current_database() AND $1::int=ANY(pg_blocking_pids(a.pid))) AS waiting,clock_timestamp() < (SELECT expires_at FROM public.support_access_sessions WHERE id=$2) AS before_deadline",
                [holder.into(), support_id.into()],
            ))
            .await
            .unwrap()
            .unwrap();
        if row.try_get::<bool>("", "waiting").unwrap() {
            assert!(!pending.is_finished());
            assert!(
                row.try_get::<bool>("", "before_deadline").unwrap(),
                "the blocked read transaction must begin before the deadline"
            );
            waiting = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(
        waiting,
        "the support read must wait on the session row lock"
    );
    loop {
        let past: bool = app
            .fixture
            .admin
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT clock_timestamp() > expires_at AS past FROM public.support_access_sessions WHERE id=$1",
                [support_id.into()],
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "past")
            .unwrap();
        if past {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    lock.commit().await.unwrap();
    let response = pending.await.unwrap();
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let (_, _, _, expired) = support_session(&app, 72602).await.unwrap();
    let audits = support_audit_count(&app, "platform/support/expired", support_id).await;
    let second = app
        .client
        .get(format!("{}/platform/support/{support_id}", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap()
        .status();
    let audits_after = support_audit_count(&app, "platform/support/expired", support_id).await;
    app.close().await;
    assert!(status.is_client_error(), "{body}");
    assert!(!body.contains("Supported tablets"), "{body}");
    assert!(expired.is_some(), "the expiry must be persisted");
    assert_eq!(audits, 1, "exactly one expiry audit must be recorded");
    assert!(second.is_client_error());
    assert_eq!(audits_after, 1, "a repeat read must not audit expiry again");
}

#[tokio::test]
async fn platform_support_expiry_audit_failure_rolls_back() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "UPDATE public.support_access_sessions SET expires_at=clock_timestamp()-interval '1 second' WHERE id={support_id}"
        ))
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("REVOKE INSERT ON public.versions FROM med_tracker_app")
        .await
        .unwrap();
    let failed = app
        .client
        .get(format!("{}/platform/support/{support_id}", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap()
        .status();
    app.fixture
        .admin
        .execute_unprepared("GRANT INSERT ON public.versions TO med_tracker_app")
        .await
        .unwrap();
    let (_, _, _, expired) = support_session(&app, 72602).await.unwrap();
    let audits = support_audit_count(&app, "platform/support/expired", support_id).await;
    let retry = app
        .client
        .get(format!("{}/platform/support/{support_id}", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap()
        .status();
    let (_, _, _, expired_after) = support_session(&app, 72602).await.unwrap();
    let audits_after = support_audit_count(&app, "platform/support/expired", support_id).await;
    app.close().await;
    assert!(!failed.is_success());
    assert!(
        expired.is_none(),
        "a failed expiry audit must roll back expired_at"
    );
    assert_eq!(audits, 0);
    assert!(!retry.is_success());
    assert!(expired_after.is_some());
    assert_eq!(audits_after, 1);
}

#[tokio::test]
async fn platform_support_read_includes_allocation_and_schedule_details() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    app.fixture.admin.execute_unprepared("INSERT INTO public.person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,max_daily_doses,position,created_at,updated_at) VALUES(83601,72602,73611,82601,1.5,'caplet',2,1,now(),now())").await.unwrap();
    app.fixture.admin.execute_unprepared("INSERT INTO public.schedules(id,household_id,person_id,medication_id,active,schedule_type,schedule_config,frequency,dose_amount,dose_unit,created_at,updated_at) VALUES(84601,72602,73611,82601,true,0,'{\"times\":[\"08:00\",\"20:00\"]}','every_6_hours',1.5,'caplet',now(),now())").await.unwrap();
    let response = app
        .client
        .get(format!("{}/platform/support/{support_id}", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    app.close().await;
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    assert!(body.contains("Supported member"), "{body}");
    assert!(body.contains("Supported tablets"), "{body}");
    assert!(body.contains("1.5 caplet"), "{body}");
    assert!(!body.contains("1.50"), "{body}");
    assert!(body.contains("Every 6 hours"), "{body}");
    assert!(!body.contains("every_6_hours"), "{body}");
    assert!(
        !body.contains("Confidential support reason"),
        "protected reasons must never render: {body}"
    );
}

fn raw_timestamp_present(body: &str) -> bool {
    body.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != ':' && ch != '.')
        .any(|token| token.contains(':') && token.contains('.'))
}

#[tokio::test]
async fn platform_support_owner_page_shows_requester_reason_and_scope() {
    let app = Application::new().await;
    supported_household(&app).await;
    let cookie = admin_session(&app).await;
    let owner = sign_in(&app, 71605, "support-owner@example.test").await;
    let (cookies, operation_id, confirm_token) =
        request_support(&app, &cookie, 72602, "Confidential support reason alpha").await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let page = app
        .client
        .get(format!("{}/account/support", app.origin))
        .header("cookie", &owner)
        .send()
        .await
        .unwrap();
    let status = page.status();
    let body = page.text().await.unwrap_or_default();
    app.close().await;
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    assert!(
        body.contains("persistence@example.test"),
        "owners must see the requesting administrator identity: {body}"
    );
    assert!(
        body.contains("Confidential support reason alpha"),
        "owners must see the request reason before approving: {body}"
    );
    assert!(
        body.contains("read-only"),
        "owners must see the read-only scope: {body}"
    );
    assert!(
        body.contains("24 hours") && body.contains("30 minutes"),
        "owners must see the request and access durations: {body}"
    );
    assert!(
        body.contains("cannot write") && body.contains("export") && body.contains("download"),
        "owners must see the no-write, no-export and no-download boundaries: {body}"
    );
    assert!(
        body.contains("end access") || body.contains("End access"),
        "owners must see that they can end access: {body}"
    );
    assert!(
        !raw_timestamp_present(&body),
        "deadlines must render as readable UTC labels: {body}"
    );
    assert!(body.contains(" UTC"), "{body}");
}

#[tokio::test]
async fn platform_support_pages_render_readable_utc_deadlines() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    let list = app
        .client
        .get(format!("{}/platform/support", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(list.status(), reqwest::StatusCode::OK);
    let list_body = list.text().await.unwrap_or_default();
    let read = app
        .client
        .get(format!("{}/platform/support/{support_id}", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(read.status(), reqwest::StatusCode::OK);
    let read_body = read.text().await.unwrap_or_default();
    app.close().await;
    assert!(
        list_body.contains(" UTC"),
        "session list deadlines must carry an explicit UTC label: {list_body}"
    );
    assert!(
        !raw_timestamp_present(&list_body),
        "session list must not render raw timestamps: {list_body}"
    );
    assert!(
        read_body.contains(" UTC"),
        "the read page expiry must carry an explicit UTC label: {read_body}"
    );
    assert!(
        !raw_timestamp_present(&read_body),
        "the read page must not render raw timestamps: {read_body}"
    );
}

#[tokio::test]
async fn platform_owner_recovery_reason_has_a_visible_label() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    recovery_household(&app).await;
    let (_, body) = form_page(&app, &cookie, "/platform/owner-recovery?household=72601").await;
    app.close().await;
    assert!(body.contains("for=\"reason-"), "{body}");
    assert!(
        !body.contains("class=\"sr-only\" for=\"reason-"),
        "the recovery reason label must be visible for touch and pointer users: {body}"
    );
}

async fn table_lock(app: &Application) -> (sea_orm::DatabaseTransaction, i32) {
    let lock = sea_orm::Database::connect(&app.fixture.runtime_uri)
        .await
        .unwrap()
        .begin()
        .await
        .unwrap();
    let holder: i32 = lock
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_backend_pid() AS pid".to_owned(),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    lock.execute_unprepared("LOCK TABLE public.household_memberships IN ACCESS EXCLUSIVE MODE")
        .await
        .unwrap();
    (lock, holder)
}

fn confirm_task(
    app: &Application,
    cookies: String,
    operation_id: String,
    confirm_token: String,
) -> tokio::task::JoinHandle<(reqwest::StatusCode, String)> {
    let client = app.client.clone();
    let origin = app.origin.clone();
    let public = app.context.config.server.full_url();
    tokio::spawn(async move {
        let url = url::Url::parse(&public).unwrap();
        let authority = match url.port() {
            Some(port) => format!("{}:{port}", url.host_str().unwrap()),
            None => url.host_str().unwrap().to_owned(),
        };
        let response = client
            .post(format!("{origin}/account/security/password/confirm"))
            .header("cookie", cookies)
            .header("host", authority)
            .header("origin", public.trim_end_matches('/'))
            .header("content-type", "application/x-www-form-urlencoded")
            .timeout(std::time::Duration::from_secs(30))
            .body(
                serde_urlencoded::to_string([
                    ("operation_id", operation_id.as_str()),
                    ("password", "password"),
                    ("authenticity_token", confirm_token.as_str()),
                ])
                .unwrap(),
            )
            .send()
            .await
            .unwrap();
        (response.status(), response.text().await.unwrap_or_default())
    })
}

#[tokio::test]
async fn platform_support_read_waiting_on_authority_query_denies_after_deadline() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    let (lock, holder) = table_lock(&app).await;
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "UPDATE public.support_access_sessions SET expires_at=clock_timestamp()+interval '3 seconds' WHERE id={support_id}"
        ))
        .await
        .unwrap();
    let read_client = app.client.clone();
    let read_origin = app.origin.clone();
    let read_cookie = cookie.clone();
    let pending = tokio::spawn(async move {
        read_client
            .get(format!("{read_origin}/platform/support/{support_id}"))
            .header("cookie", read_cookie)
            .send()
            .await
            .unwrap()
    });
    let waited = wait_blocked_on(&app, holder, support_id).await;
    wait_past_deadline(&app, support_id).await;
    lock.commit().await.unwrap();
    let response = pending.await.unwrap();
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let (_, _, _, expired) = support_session(&app, 72602).await.unwrap();
    let audits = support_audit_count(&app, "platform/support/expired", support_id).await;
    assert!(
        status.is_client_error(),
        "an expired read must be denied, got {status}: {body}"
    );
    assert!(!body.contains("Supported tablets"), "{body}");
    assert!(
        waited,
        "the owner authority check must wait on the authority query"
    );
    assert!(expired.is_some(), "the expiry must be persisted");
    assert_eq!(audits, 1);
    let repeat = app
        .client
        .get(format!("{}/platform/support/{support_id}", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert!(repeat.status().is_client_error());
    assert_eq!(
        support_audit_count(&app, "platform/support/expired", support_id).await,
        1,
        "repeated reads must not duplicate the expiry audit"
    );
    app.close().await;
}

#[tokio::test]
async fn platform_support_approval_waiting_on_owner_lock_denies_after_deadline() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    supported_household(&app).await;
    let owner = sign_in(&app, 71605, "support-owner@example.test").await;
    let (cookies, operation_id, confirm_token) =
        request_support(&app, &cookie, 72602, "Confidential support reason alpha").await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) = support_session(&app, 72602)
        .await
        .expect("the request must persist");
    let (cookies, operation_id, confirm_token) = start_support_form(
        &app,
        &owner,
        "/account/support",
        "/account/support/approve",
        support_id,
    )
    .await;
    let (lock, holder) = membership_lock(&app, 72602, 74605).await;
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "UPDATE public.support_access_sessions SET expires_at=clock_timestamp()+interval '3 seconds' WHERE id={support_id}"
        ))
        .await
        .unwrap();
    let pending = confirm_task(&app, cookies, operation_id, confirm_token);
    let waited = wait_blocked_on(&app, holder, support_id).await;
    wait_past_deadline(&app, support_id).await;
    lock.commit().await.unwrap();
    let (status, outcome) = pending.await.unwrap();
    let (approved_at, _) = support_decision(&app, support_id).await;
    assert!(
        status.is_client_error(),
        "an expired request must not gain approval, got {status}: {outcome}"
    );
    assert!(
        waited,
        "the approval must wait on the owner membership lock"
    );
    assert!(approved_at.is_none(), "{approved_at:?}");
    assert_eq!(
        support_audit_count(&app, "platform/support/approved", support_id).await,
        0
    );
    app.close().await;
}

#[tokio::test]
async fn platform_support_activation_waiting_on_authority_query_denies_after_deadline() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    supported_household(&app).await;
    let owner = sign_in(&app, 71605, "support-owner@example.test").await;
    let (cookies, operation_id, confirm_token) =
        request_support(&app, &cookie, 72602, "Confidential support reason alpha").await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) = support_session(&app, 72602)
        .await
        .expect("the request must persist");
    assert_eq!(
        submit_support_form(
            &app,
            &owner,
            "/account/support",
            "/account/support/approve",
            support_id,
        )
        .await,
        reqwest::StatusCode::OK,
        "the owner approval must succeed before activation"
    );
    let (cookies, operation_id, confirm_token) = start_support_form(
        &app,
        &cookie,
        "/platform/support",
        "/platform/support/activate",
        support_id,
    )
    .await;
    let (lock, holder) = table_lock(&app).await;
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "UPDATE public.support_access_sessions SET expires_at=clock_timestamp()+interval '3 seconds' WHERE id={support_id}"
        ))
        .await
        .unwrap();
    let pending = confirm_task(&app, cookies, operation_id, confirm_token);
    let waited = wait_blocked_on(&app, holder, support_id).await;
    wait_past_deadline(&app, support_id).await;
    lock.commit().await.unwrap();
    let (status, outcome) = pending.await.unwrap();
    let (_, activated_at) = support_decision(&app, support_id).await;
    assert!(
        status.is_client_error(),
        "an expired request must not activate, got {status}: {outcome}"
    );
    assert!(
        waited,
        "the activation authority check must wait on the authority query"
    );
    assert!(activated_at.is_none(), "{activated_at:?}");
    assert_eq!(
        support_audit_count(&app, "platform/support/activated", support_id).await,
        0
    );
    app.close().await;
}

async fn security_page(app: &Application, cookie: &str) -> (reqwest::StatusCode, String) {
    let response = app
        .client
        .get(format!("{}/account/security", app.origin))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    let status = response.status();
    (status, response.text().await.unwrap_or_default())
}

#[tokio::test]
async fn platform_account_security_shows_scoped_navigation_links() {
    let app = Application::new().await;
    supported_household(&app).await;
    let admin = admin_session(&app).await;
    let owner = sign_in(&app, 71605, "support-owner@example.test").await;
    let member = sign_in(&app, 71611, "supported-member@example.test").await;
    let (status, body) = security_page(&app, &admin).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    assert!(
        body.contains("/platform/users"),
        "platform administrators need the platform administration link: {body}"
    );
    assert!(
        !body.contains("/account/support"),
        "non-owner administrators must not see the owner support link: {body}"
    );
    let (status, body) = security_page(&app, &owner).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    assert!(
        body.contains("/account/support"),
        "household owners need the support access link: {body}"
    );
    assert!(
        !body.contains("/platform/users"),
        "ordinary owners must not see platform administration: {body}"
    );
    let (status, body) = security_page(&app, &member).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    assert!(
        !body.contains("/platform/users") && !body.contains("/account/support"),
        "ordinary members must see neither link: {body}"
    );
    app.close().await;
}

async fn session_lock(app: &Application, support_id: i64) -> (sea_orm::DatabaseTransaction, i32) {
    let lock = sea_orm::Database::connect(&app.fixture.runtime_uri)
        .await
        .unwrap()
        .begin()
        .await
        .unwrap();
    let holder: i32 = lock
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_backend_pid() AS pid".to_owned(),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    lock.execute_unprepared(&format!(
        "SELECT id FROM public.support_access_sessions WHERE id={support_id} FOR UPDATE"
    ))
    .await
    .unwrap();
    (lock, holder)
}

#[tokio::test]
async fn platform_support_read_rechecks_admin_authority_after_authority_wait() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    let (lock, holder) = table_lock(&app).await;
    let read_client = app.client.clone();
    let read_origin = app.origin.clone();
    let read_cookie = cookie.clone();
    let pending = tokio::spawn(async move {
        read_client
            .get(format!("{read_origin}/platform/support/{support_id}"))
            .header("cookie", read_cookie)
            .send()
            .await
            .unwrap()
    });
    let waited = wait_blocked_on(&app, holder, support_id).await;
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE platform_admins SET status='revoked',updated_at=clock_timestamp() WHERE account_id=71001",
        )
        .await
        .unwrap();
    lock.commit().await.unwrap();
    let response = pending.await.unwrap();
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    app.close().await;
    assert!(waited, "the read must wait on the owner authority query");
    assert!(
        status.is_client_error(),
        "a read after administrator withdrawal must be denied, got {status}: {body}"
    );
    assert!(!body.contains("Supported tablets"), "{body}");
}

#[tokio::test]
async fn platform_support_approval_rechecks_owner_lockout_after_row_wait() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    supported_household(&app).await;
    let owner = sign_in(&app, 71605, "support-owner@example.test").await;
    let (cookies, operation_id, confirm_token) =
        request_support(&app, &cookie, 72602, "Confidential support reason alpha").await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) = support_session(&app, 72602)
        .await
        .expect("the request must persist");
    let (cookies, operation_id, confirm_token) = start_support_form(
        &app,
        &owner,
        "/account/support",
        "/account/support/approve",
        support_id,
    )
    .await;
    let (lock, holder) = session_lock(&app, support_id).await;
    let pending = confirm_task(&app, cookies, operation_id, confirm_token);
    let waited = wait_blocked_on(&app, holder, support_id).await;
    app.fixture
        .admin
        .execute_unprepared(
            "INSERT INTO public.account_lockouts(account_id,deadline,key,created_at,updated_at) VALUES(71605,clock_timestamp()+interval '1 hour','support-approval-lockout',now(),now()) ON CONFLICT (account_id) DO UPDATE SET deadline=excluded.deadline",
        )
        .await
        .unwrap();
    lock.commit().await.unwrap();
    let (status, outcome) = pending.await.unwrap();
    let (approved_at, _) = support_decision(&app, support_id).await;
    let audits = support_audit_count(&app, "platform/support/approved", support_id).await;
    app.close().await;
    assert!(waited, "the approval must wait on the session row lock");
    assert!(
        status.is_client_error(),
        "a locked-out owner must not approve, got {status}: {outcome}"
    );
    assert!(approved_at.is_none(), "{approved_at:?}");
    assert_eq!(audits, 0);
}

#[tokio::test]
async fn platform_support_activation_rechecks_admin_after_row_wait() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    supported_household(&app).await;
    let owner = sign_in(&app, 71605, "support-owner@example.test").await;
    let (cookies, operation_id, confirm_token) =
        request_support(&app, &cookie, 72602, "Confidential support reason alpha").await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) = support_session(&app, 72602)
        .await
        .expect("the request must persist");
    let approved = approve_support(&app, &owner, support_id).await;
    assert_eq!(approved, reqwest::StatusCode::OK);
    let (cookies, operation_id, confirm_token) = start_support_form(
        &app,
        &cookie,
        "/platform/support",
        "/platform/support/activate",
        support_id,
    )
    .await;
    let (lock, holder) = session_lock(&app, support_id).await;
    let pending = confirm_task(&app, cookies, operation_id, confirm_token);
    let waited = wait_blocked_on(&app, holder, support_id).await;
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE platform_admins SET status='revoked',updated_at=clock_timestamp() WHERE account_id=71001",
        )
        .await
        .unwrap();
    lock.commit().await.unwrap();
    let (status, outcome) = pending.await.unwrap();
    let (_, activated_at) = support_decision(&app, support_id).await;
    let audits = support_audit_count(&app, "platform/support/activated", support_id).await;
    app.close().await;
    assert!(waited, "the activation must wait on the session row lock");
    assert!(
        status.is_client_error(),
        "a withdrawn administrator must not activate, got {status}: {outcome}"
    );
    assert!(activated_at.is_none(), "{activated_at:?}");
    assert_eq!(audits, 0);
}

#[tokio::test]
async fn platform_support_activation_rechecks_household_after_row_wait() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    supported_household(&app).await;
    let owner = sign_in(&app, 71605, "support-owner@example.test").await;
    let (cookies, operation_id, confirm_token) =
        request_support(&app, &cookie, 72602, "Confidential support reason alpha").await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) = support_session(&app, 72602)
        .await
        .expect("the request must persist");
    let approved = approve_support(&app, &owner, support_id).await;
    assert_eq!(approved, reqwest::StatusCode::OK);
    let (cookies, operation_id, confirm_token) = start_support_form(
        &app,
        &cookie,
        "/platform/support",
        "/platform/support/activate",
        support_id,
    )
    .await;
    let (lock, holder) = session_lock(&app, support_id).await;
    let pending = confirm_task(&app, cookies, operation_id, confirm_token);
    let waited = wait_blocked_on(&app, holder, support_id).await;
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE households SET status='suspended',updated_at=clock_timestamp() WHERE id=72602",
        )
        .await
        .unwrap();
    lock.commit().await.unwrap();
    let (status, outcome) = pending.await.unwrap();
    let (_, activated_at) = support_decision(&app, support_id).await;
    let audits = support_audit_count(&app, "platform/support/activated", support_id).await;
    app.close().await;
    assert!(waited, "the activation must wait on the session row lock");
    assert!(
        status.is_client_error(),
        "a suspended household must not activate, got {status}: {outcome}"
    );
    assert!(activated_at.is_none(), "{activated_at:?}");
    assert_eq!(audits, 0);
}

#[tokio::test]
async fn platform_support_end_rechecks_actor_after_row_wait() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    let (cookies, body) = form_page(&app, &cookie, "/platform/support").await;
    let token = hidden_field(&body, "authenticity_token");
    let public = app.context.config.server.full_url();
    let url = url::Url::parse(&public).unwrap();
    let authority = match url.port() {
        Some(port) => format!("{}:{port}", url.host_str().unwrap()),
        None => url.host_str().unwrap().to_owned(),
    };
    let (lock, holder) = session_lock(&app, support_id).await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let origin = app.origin.clone();
    let request_cookies = cookies.clone();
    let end = tokio::spawn(async move {
        client
            .post(format!("{origin}/platform/support/end"))
            .header("cookie", request_cookies)
            .header("host", authority)
            .header("origin", public.trim_end_matches('/'))
            .header("content-type", "application/x-www-form-urlencoded")
            .timeout(std::time::Duration::from_secs(30))
            .body(
                serde_urlencoded::to_string([
                    ("support_id", support_id.to_string()),
                    ("authenticity_token", token),
                ])
                .unwrap(),
            )
            .send()
            .await
            .unwrap()
    });
    let waited = wait_blocked_on(&app, holder, support_id).await;
    app.fixture
        .admin
        .execute_unprepared(
            "INSERT INTO public.account_lockouts(account_id,deadline,key,created_at,updated_at) VALUES(71001,clock_timestamp()+interval '1 hour','support-end-lockout',now(),now()) ON CONFLICT (account_id) DO UPDATE SET deadline=excluded.deadline",
        )
        .await
        .unwrap();
    lock.commit().await.unwrap();
    let response = end.await.unwrap();
    let status = response.status();
    let location = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let ended = support_session(&app, 72602).await.unwrap().2;
    let audits = support_audit_count(&app, "platform/support/ended", support_id).await;
    app.close().await;
    assert!(waited, "the end request must wait on the session row lock");
    assert_eq!(status, reqwest::StatusCode::SEE_OTHER);
    assert_eq!(location.as_deref(), Some("/login"));
    assert!(ended.is_none(), "ended_at must stay unset: {ended:?}");
    assert_eq!(audits, 0);
}

#[tokio::test]
async fn platform_support_expired_request_denies_approval_and_records_expiry() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    supported_household(&app).await;
    let owner = sign_in(&app, 71605, "support-owner@example.test").await;
    let (cookies, operation_id, confirm_token) =
        request_support(&app, &cookie, 72602, "Confidential support reason alpha").await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) = support_session(&app, 72602)
        .await
        .expect("the request must persist");
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "UPDATE public.support_access_sessions SET expires_at=clock_timestamp()-interval '1 second' WHERE id={support_id}"
        ))
        .await
        .unwrap();
    let denied = approve_support(&app, &owner, support_id).await;
    let (approved_at, _) = support_decision(&app, support_id).await;
    let (_, _, _, expired) = support_session(&app, 72602).await.unwrap();
    let audits = support_audit_count(&app, "platform/support/expired", support_id).await;
    let again = approve_support(&app, &owner, support_id).await;
    let audits_after = support_audit_count(&app, "platform/support/expired", support_id).await;
    app.close().await;
    assert!(
        denied.is_client_error(),
        "an expired request must not gain approval"
    );
    assert!(approved_at.is_none(), "{approved_at:?}");
    assert!(expired.is_some(), "the denial must persist expiry");
    assert_eq!(audits, 1, "exactly one expiry audit must be recorded");
    assert!(again.is_client_error());
    assert_eq!(
        audits_after, 1,
        "a repeated denial must not duplicate the expiry audit"
    );
}

#[tokio::test]
async fn platform_support_pending_expiry_audit_failure_rolls_back() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    supported_household(&app).await;
    let owner = sign_in(&app, 71605, "support-owner@example.test").await;
    let (cookies, operation_id, confirm_token) =
        request_support(&app, &cookie, 72602, "Confidential support reason alpha").await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) = support_session(&app, 72602)
        .await
        .expect("the request must persist");
    let (owner_cookies, approval_id, approval_token) = start_support_form(
        &app,
        &owner,
        "/account/support",
        "/account/support/approve",
        support_id,
    )
    .await;
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "UPDATE public.support_access_sessions SET expires_at=clock_timestamp()-interval '1 second' WHERE id={support_id}"
        ))
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("REVOKE INSERT ON public.versions FROM med_tracker_app")
        .await
        .unwrap();
    let (denied, denied_body) =
        browser_confirm(&app, &owner_cookies, &approval_id, &approval_token).await;
    app.fixture
        .admin
        .execute_unprepared("GRANT INSERT ON public.versions TO med_tracker_app")
        .await
        .unwrap();
    let (_, _, _, expired) = support_session(&app, 72602).await.unwrap();
    let audits = support_audit_count(&app, "platform/support/expired", support_id).await;
    let (retried, retry_body) =
        browser_confirm(&app, &owner_cookies, &approval_id, &approval_token).await;
    let (_, _, _, expired_after) = support_session(&app, 72602).await.unwrap();
    let audits_after = support_audit_count(&app, "platform/support/expired", support_id).await;
    app.close().await;
    assert!(!denied.is_success());
    assert!(
        expired.is_none(),
        "a failed expiry audit must roll back expired_at"
    );
    assert_eq!(audits, 0);
    assert!(retried.is_client_error());
    assert!(
        expired_after.is_some(),
        "a recovered audit must persist the expiry; first {denied}: {denied_body}; retry {retried}: {retry_body}"
    );
    assert_eq!(audits_after, 1);
}

#[tokio::test]
async fn platform_support_end_on_expired_session_records_expiry_once() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "UPDATE public.support_access_sessions SET expires_at=clock_timestamp()-interval '1 second' WHERE id={support_id}"
        ))
        .await
        .unwrap();
    let (cookies, body) = form_page(&app, &cookie, "/platform/support").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/support/end",
        &[
            ("support_id", &support_id.to_string()),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let status = response.status();
    let cookies = response_cookies(&cookies, &response);
    let page = response.text().await.unwrap_or_default();
    let operation_id = hidden_field(&page, "operation_id");
    let final_status = if status == reqwest::StatusCode::OK && !operation_id.is_empty() {
        let confirm_token = hidden_field(&page, "authenticity_token");
        browser_confirm(&app, &cookies, &operation_id, &confirm_token)
            .await
            .0
    } else {
        status
    };
    let (_, _, ended, expired) = support_session(&app, 72602).await.unwrap();
    let audits = support_audit_count(&app, "platform/support/expired", support_id).await;
    app.close().await;
    assert!(
        final_status.is_client_error(),
        "ending an expired session must deny deliberately: {page}"
    );
    assert!(expired.is_some(), "the end path must persist expiry");
    assert!(ended.is_none(), "an expired session must not gain ended_at");
    assert_eq!(audits, 1, "exactly one expiry audit must be recorded");
}

#[tokio::test]
async fn platform_owner_recovery_search_does_not_place_csrf_tokens_in_urls() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    recovery_household(&app).await;
    let (_, body) = form_page(&app, &cookie, "/platform/owner-recovery").await;
    let (_, detail) = form_page(&app, &cookie, "/platform/owner-recovery?household=72601").await;
    app.close().await;
    let search = body
        .split("<form method=\"get\"")
        .nth(1)
        .and_then(|part| part.split("</form>").next())
        .unwrap_or_default();
    assert!(
        !search.is_empty(),
        "the household search must be a GET form: {body}"
    );
    assert!(
        !search.contains("authenticity_token"),
        "the GET search must not leak CSRF tokens into the URL: {search}"
    );
    assert!(
        detail.contains("name=\"authenticity_token\""),
        "POST promotion forms keep CSRF protection: {detail}"
    );
}

#[tokio::test]
async fn platform_settings_get_renders_defaults_without_creating_the_singleton() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    app.fixture
        .admin
        .execute_unprepared("DELETE FROM public.app_settings")
        .await
        .unwrap();
    let (_, body) = form_page(&app, &cookie, "/platform/settings").await;
    let persisted = app_settings(&app).await;
    app.close().await;
    assert!(
        body.contains("invite_only"),
        "GET settings must render defaults for a missing row: {body}"
    );
    assert!(
        persisted.is_none(),
        "a settings read must not write the singleton row"
    );
}

#[tokio::test]
async fn platform_audit_does_not_trust_spoofed_forwarded_for() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    let (cookies, body) = form_page(&app, &cookie, "/platform/support").await;
    let token = hidden_field(&body, "authenticity_token");
    let public = app.context.config.server.full_url();
    let url = url::Url::parse(&public).unwrap();
    let authority = match url.port() {
        Some(port) => format!("{}:{port}", url.host_str().unwrap()),
        None => url.host_str().unwrap().to_owned(),
    };
    app.client
        .post(format!("{}/platform/support/end", app.origin))
        .header("cookie", &cookies)
        .header("host", authority)
        .header("origin", public.trim_end_matches('/'))
        .header("content-type", "application/x-www-form-urlencoded")
        .header("x-forwarded-for", "198.51.100.77")
        .header("x-real-ip", "198.51.100.77")
        .body(
            serde_urlencoded::to_string([
                ("support_id", support_id.to_string()),
                ("authenticity_token", token),
            ])
            .unwrap(),
        )
        .send()
        .await
        .unwrap();
    let ip = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT audit_context->>'ip' AS ip FROM versions WHERE event='platform/support/ended' AND item_id=$1 ORDER BY id DESC LIMIT 1",
            [support_id.into()],
        ))
        .await
        .unwrap()
        .and_then(|row| row.try_get::<Option<String>>("", "ip").unwrap());
    app.close().await;
    assert!(
        ip.is_some(),
        "an audited origin must be recorded for the end event"
    );
    assert_ne!(
        ip.as_deref(),
        Some("198.51.100.77"),
        "spoofed client forwarding headers must never reach audit metadata"
    );
}

#[tokio::test]
async fn platform_support_listing_expiry_audit_is_race_safe() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    app.fixture.admin.execute_unprepared(&format!(
        "UPDATE public.support_access_sessions SET expires_at=clock_timestamp()-interval '1 second' WHERE id={support_id}"
    )).await.unwrap();
    let (lock, _) = session_lock(&app, support_id).await;
    let mut requests = Vec::new();
    for _ in 0..2 {
        let client = app.client.clone();
        let origin = app.origin.clone();
        let cookie = cookie.clone();
        requests.push(tokio::spawn(async move {
            client
                .get(format!("{origin}/platform/support"))
                .header("cookie", cookie)
                .timeout(std::time::Duration::from_secs(30))
                .send()
                .await
                .unwrap()
                .status()
        }));
    }
    let mut waiting = 0_i64;
    for _ in 0..60 {
        waiting = app.fixture.admin.query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS waiting FROM pg_stat_activity WHERE datname=current_database() AND state='active' AND cardinality(pg_blocking_pids(pid)) > 0".to_owned(),
        )).await.unwrap().unwrap().try_get("","waiting").unwrap();
        if waiting == 2 {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    lock.commit().await.unwrap();
    for request in requests {
        assert_eq!(request.await.unwrap(), reqwest::StatusCode::OK);
    }
    let audits = support_audit_count(&app, "platform/support/expired", support_id).await;
    let expired = support_session(&app, 72602).await.unwrap().3;
    app.close().await;
    assert_eq!(waiting, 2, "both listings must reach the database wait");
    assert!(expired.is_some());
    assert_eq!(
        audits, 1,
        "concurrent listings must persist one expiry audit"
    );
}

#[tokio::test]
async fn platform_support_listing_does_not_expire_an_ended_session() {
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    let (cookies, body) = form_page(&app, &cookie, "/platform/support").await;
    let token = hidden_field(&body, "authenticity_token");
    let ended = post_form(
        &app,
        &cookies,
        "/platform/support/end",
        &[
            ("support_id", &support_id.to_string()),
            ("authenticity_token", &token),
        ],
    )
    .await;
    assert!(ended.status().is_success());
    assert!(support_session(&app, 72602).await.unwrap().2.is_some());
    app.fixture.admin.execute_unprepared(&format!(
        "UPDATE public.support_access_sessions SET expires_at=clock_timestamp()-interval '1 second' WHERE id={support_id}"
    )).await.unwrap();
    let (_, body) = form_page(&app, &cookie, "/platform/support").await;
    let expired = support_session(&app, 72602).await.unwrap().3;
    let audits = support_audit_count(&app, "platform/support/expired", support_id).await;
    app.close().await;
    assert!(body.contains("ended"));
    assert!(
        expired.is_none(),
        "ending before expiry must remain the final state"
    );
    assert_eq!(audits, 0);
}
