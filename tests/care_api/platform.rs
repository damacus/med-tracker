use super::*;
use med_tracker::models::_entities::{
    accounts, app_settings as app_setting_rows, households, locations, medications, people,
    platform_admins, support_access_sessions, users,
};
use sea_orm::TransactionTrait;
use sea_orm::sea_query::{Alias, Expr, ExprTrait, Func, Query};
use sea_orm::{ActiveValue::NotSet, IntoActiveModel};

#[tokio::test]
async fn platform_owner_recovery_cannot_be_bypassed_through_membership_updates() {
    crate::authentication_fixture::reset_source_quota().await;
    for other in [false, true] {
        let app = Application::new().await;
        synthetic_account(&app, "platform-recovery-member", 2).await;
        app.fixture
            .admin
            .execute_raw({
                let query = Query::insert()
                    .into_table("household_memberships")
                    .columns([
                        "household_id",
                        "person_id",
                        "role",
                        "status",
                        "joined_at",
                        "created_at",
                        "updated_at",
                    ])
                    .values_panic([
                        Expr::val(app.fixture.household.id),
                        Expr::val(actor(&app, "platform-recovery-member").await.person.id),
                        Expr::val("member"),
                        Expr::val("active"),
                        Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                        Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                        Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    ])
                    .to_owned();
                app.fixture.admin.get_database_backend().build(&query)
            })
            .await
            .unwrap();
        let target = if other {
            membership_link(
                &app,
                app.fixture.household.id,
                actor(&app, "platform-recovery-member").await.person.id,
            )
            .await
            .id
        } else {
            app.fixture.membership.id
        };
        admin_session(&app).await;
        let bearer = app.token().await;
        let response = app
            .client
            .patch(format!(
                "{}/api/v1/households/{identity_72001}/admin/memberships/{target}",
                app.origin,
                identity_72001 = app.fixture.household.id
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
            .query_one_raw({
                let query = Query::select()
                    .expr(Expr::col("role"))
                    .from("household_memberships")
                    .and_where(Expr::col("id").eq(Expr::val(target)))
                    .to_owned();
                app.fixture.admin.get_database_backend().build(&query)
            })
            .await
            .unwrap()
            .unwrap();
        let role = row.try_get::<String>("", "role").unwrap();

        assert!(
            status.is_client_error(),
            "owner promotion requires the dedicated proof-bound recovery boundary for target {target}"
        );
        assert_ne!(role, "owner");
        finish(&app, None, None).await;
        app.close().await;
    }
}

async fn session(app: &Application) -> String {
    sign_in(app, app.fixture.account.id, &app.fixture.account.email).await
}

async fn seed_platform_admin(app: &Application) {
    use med_tracker::models::_entities::platform_admins;
    let now = chrono::Utc::now().naive_utc();
    platform_admins::Entity::insert(platform_admins::ActiveModel {
        account_id: Set(app.fixture.account.id),
        status: Set("active".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    })
    .on_conflict(
        sea_orm::sea_query::OnConflict::column(platform_admins::Column::AccountId)
            .update_columns([
                platform_admins::Column::Status,
                platform_admins::Column::UpdatedAt,
            ])
            .to_owned(),
    )
    .exec(&app.fixture.admin)
    .await
    .unwrap();
}

pub(super) async fn admin_session(app: &Application) -> String {
    seed_platform_admin(app).await;
    session(app).await
}

async fn admin_fixture_session(app: &Application) -> String {
    seed_platform_admin(app).await;
    fixture_session(app, app.fixture.account.id).await
}

async fn seed_onboarding(app: &Application, account_id: i64) {
    use med_tracker::models::_entities::identity_onboarding;
    identity_onboarding::Entity::insert(identity_onboarding::ActiveModel {
        account_id: Set(account_id),
        completed_at: Set(Some(chrono::Utc::now().fixed_offset())),
    })
    .on_conflict(
        sea_orm::sea_query::OnConflict::column(identity_onboarding::Column::AccountId)
            .update_column(identity_onboarding::Column::CompletedAt)
            .to_owned(),
    )
    .exec(&app.fixture.admin)
    .await
    .unwrap();
}

pub(super) async fn fixture_session(app: &Application, account_id: i64) -> String {
    seed_onboarding(app, account_id).await;
    crate::session_fixture::browser_cookie(&app.context, account_id).await
}

pub(super) async fn sign_in(app: &Application, account_id: i64, email: &str) -> String {
    seed_onboarding(app, account_id).await;
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
    request
        .headers
        .insert("content-type".into(), "application/json".into());
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

async fn membership_link(
    app: &Application,
    household_id: i64,
    person_id: i64,
) -> household_memberships::Model {
    household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(household_id))
        .filter(household_memberships::Column::PersonId.eq(person_id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
}

struct PlatformActor {
    account: accounts::Model,
    person: people::Model,
    user: users::Model,
}
async fn synthetic_account(app: &Application, label: &str, status: i32) -> PlatformActor {
    let now = chrono::Utc::now();
    let person = people::ActiveModel {
        name: Set("Synthetic platform member".into()),
        date_of_birth: Set(Some("1990-01-01".parse().unwrap())),
        person_type: Set(0),
        has_capacity: Set(true),
        portable_id: Set(format!("{}-{label}", app.fixture.client_uuid)),
        created_at: Set(now.naive_utc()),
        updated_at: Set(now.naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let account = accounts::ActiveModel {
        person_id: Set(person.id),
        name: Set(Some("Synthetic platform member".into())),
        email: Set(format!("{label}-{}@example.test", app.fixture.client_uuid)),
        status: Set(status),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let user = users::ActiveModel {
        person_id: Set(person.id),
        email_address: Set(account.email.clone()),
        active: Set(true),
        created_at: Set(now.naive_utc()),
        updated_at: Set(now.naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    crate::fixtures::seed_native_password(&app.fixture.admin, account.id).await;
    PlatformActor {
        account,
        person,
        user,
    }
}
async fn actor(app: &Application, label: &str) -> PlatformActor {
    let account = accounts::Entity::find()
        .filter(
            accounts::Column::Email.eq(format!("{label}-{}@example.test", app.fixture.client_uuid)),
        )
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let person = people::Entity::find_by_id(account.person_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let user = users::Entity::find()
        .filter(users::Column::PersonId.eq(person.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    PlatformActor {
        account,
        person,
        user,
    }
}
async fn household_named(app: &Application, label: &str) -> households::Model {
    households::Entity::find()
        .filter(households::Column::Slug.eq(format!("{label}-{}", app.fixture.client_uuid)))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
}
async fn member_named(
    app: &Application,
    household: &str,
    label: &str,
) -> household_memberships::Model {
    household_memberships::Entity::find()
        .filter(
            household_memberships::Column::HouseholdId.eq(household_named(app, household).await.id),
        )
        .filter(household_memberships::Column::PersonId.eq(actor(app, label).await.person.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
}
async fn participant_named(app: &Application, name: &str) -> people::Model {
    people::Entity::find()
        .filter(people::Column::PortableId.eq(format!("{}-{name}", app.fixture.client_uuid)))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
}
async fn accountless_member(
    app: &Application,
    household: &str,
    name: &str,
) -> household_memberships::Model {
    household_memberships::Entity::find()
        .filter(
            household_memberships::Column::HouseholdId.eq(household_named(app, household).await.id),
        )
        .filter(household_memberships::Column::PersonId.eq(participant_named(app, name).await.id))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
}
async fn supported_medication(app: &Application) -> medications::Model {
    medications::Entity::find()
        .filter(
            medications::Column::HouseholdId.eq(household_named(app, "supported-fixture").await.id),
        )
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap()
}

pub(super) async fn operation_start(
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

pub(super) async fn operation_confirm(
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
        .query_one_raw({
            let query = Query::select()
                .expr(Expr::col("status"))
                .from("platform_admins")
                .and_where(Expr::col("account_id").eq(Expr::val(account_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .and_then(|row| row.try_get::<String>("", "status").ok())
}

async fn account_status(app: &Application, account_id: i64) -> Option<i32> {
    app.fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(Expr::col("status"))
                .from("accounts")
                .and_where(Expr::col("id").eq(Expr::val(account_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .and_then(|row| row.try_get::<i32>("", "status").ok())
}

async fn user_active(app: &Application, user_id: i64) -> Option<bool> {
    app.fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(Expr::col("active"))
                .from("users")
                .and_where(Expr::col("id").eq(Expr::val(user_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .and_then(|row| row.try_get::<bool>("", "active").ok())
}

pub(super) fn hidden_field(body: &str, name: &str) -> String {
    body.split(&format!("name=\"{name}\" value=\""))
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .unwrap_or_default()
        .to_owned()
}

pub(super) fn response_cookies(base: &str, response: &reqwest::Response) -> String {
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

pub(super) async fn form_page(app: &Application, cookie: &str, path: &str) -> (String, String) {
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

pub(super) async fn post_form(
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
        .get(format!(
            "{}/platform/users?q={}",
            app.origin, app.fixture.client_uuid
        ))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let target = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    assert!(status.is_redirection());
    assert_eq!(target.as_deref(), Some("/login"));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_users_rejects_a_household_administrator_without_platform_rights() {
    let app = Application::new().await;
    let cookie = fixture_session(&app, app.fixture.account.id).await;
    let response = app
        .client
        .get(format!("{}/platform/users", app.origin))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    let status = response.status();

    assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_users_lists_users_for_an_active_platform_administrator() {
    let app = Application::new().await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("platform_admins")
                .columns(["account_id", "status", "created_at", "updated_at"])
                .values_panic([
                    Expr::val(app.fixture.account.id),
                    Expr::val("active"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let cookie = fixture_session(&app, app.fixture.account.id).await;
    let mut endpoint = url::Url::parse(&format!("{}/platform/users", app.origin)).unwrap();
    endpoint
        .query_pairs_mut()
        .append_pair("q", &app.fixture.account.email);
    let response = app
        .client
        .get(endpoint)
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();

    assert_eq!(status, reqwest::StatusCode::OK);
    assert!(str::contains(&body, app.fixture.account.email.as_str()));
    assert!(!str::contains(&body, "password_hash"));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_users_search_filters_the_listing() {
    let app = Application::new().await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("platform_admins")
                .columns(["account_id", "status", "created_at", "updated_at"])
                .values_panic([
                    Expr::val(app.fixture.account.id),
                    Expr::val("active"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let cookie = fixture_session(&app, app.fixture.account.id).await;
    let matching = app
        .client
        .get(format!(
            "{}/platform/users?q={}",
            app.origin, app.fixture.account.email
        ))
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

    assert!(str::contains(&matching, app.fixture.account.email.as_str()));
    assert!(!str::contains(&other, app.fixture.account.email.as_str()));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_users_clamps_pages_beyond_the_listing() {
    let app = Application::new().await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("platform_admins")
                .columns(["account_id", "status", "created_at", "updated_at"])
                .values_panic([
                    Expr::val(app.fixture.account.id),
                    Expr::val("active"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let cookie = fixture_session(&app, app.fixture.account.id).await;
    let response = app
        .client
        .get(format!(
            "{}/platform/users?q={}&page=2",
            app.origin, app.fixture.account.email
        ))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();

    assert_eq!(status, reqwest::StatusCode::OK);
    assert!(str::contains(&body, app.fixture.account.email.as_str()));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_users_rechecks_withdrawn_platform_rights() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("platform_admins")
                .columns(["account_id", "status", "created_at", "updated_at"])
                .values_panic([
                    Expr::val(app.fixture.account.id),
                    Expr::val("active"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let cookie = session(&app).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("platform_admins")
                .value("status", Expr::val("disabled"))
                .and_where(Expr::col("account_id").eq(Expr::val(app.fixture.account.id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
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

    assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_users_rechecks_a_deactivated_account() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("platform_admins")
                .columns(["account_id", "status", "created_at", "updated_at"])
                .values_panic([
                    Expr::val(app.fixture.account.id),
                    Expr::val("active"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let cookie = session(&app).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("accounts")
                .value("status", Expr::val(3))
                .and_where(Expr::col("id").eq(Expr::val(app.fixture.account.id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
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

    assert!(status.is_redirection());
    assert_eq!(target.as_deref(), Some("/login"));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_users_requires_a_live_identity_session() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("platform_admins")
                .columns(["account_id", "status", "created_at", "updated_at"])
                .values_panic([
                    Expr::val(app.fixture.account.id),
                    Expr::val("active"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let cookie = session(&app).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::delete()
                .from_table("identity_sessions")
                .and_where(Expr::col("account_id").eq(app.fixture.account.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
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

    assert!(status.is_redirection());
    assert_eq!(target.as_deref(), Some("/login"));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_administrator_grant_after_password_proof() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":actor(&app,"platform-second").await.account.id,"grant":true}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    assert_eq!(
        admin_status(&app, actor(&app, "platform-second").await.account.id).await,
        None,
        "no mutation at proof start"
    );
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let granted = admin_status(&app, actor(&app, "platform-second").await.account.id).await;

    assert!(confirm.is_success());
    assert_eq!(granted.as_deref(), Some("active"));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_administrator_revoke_after_password_proof() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("platform_admins")
                .columns(["account_id", "status", "created_at", "updated_at"])
                .values_panic([
                    Expr::val(actor(&app, "platform-second").await.account.id),
                    Expr::val("active"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":actor(&app,"platform-second").await.account.id,"grant":false}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let revoked = admin_status(&app, actor(&app, "platform-second").await.account.id).await;

    assert!(confirm.is_success());
    assert_eq!(revoked.as_deref(), Some("disabled"));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_user_deactivate_and_reactivate_after_password_proof() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_user","account_id":actor(&app,"platform-second").await.account.id,"active":false}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    assert_eq!(
        user_active(&app, actor(&app, "platform-second").await.user.id).await,
        Some(true),
        "no mutation at proof start"
    );
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    assert!(confirm.is_success());
    assert_eq!(
        user_active(&app, actor(&app, "platform-second").await.user.id).await,
        Some(false)
    );
    assert_eq!(
        account_status(&app, actor(&app, "platform-second").await.account.id).await,
        Some(2),
        "verified account must not be closed by platform disable"
    );
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_user","account_id":actor(&app,"platform-second").await.account.id,"active":true}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let active = user_active(&app, actor(&app, "platform-second").await.user.id).await;

    assert!(confirm.is_success());
    assert_eq!(active, Some(true));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_operation_denied_without_platform_rights() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    let cookie = session(&app).await;
    let (status, _) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":actor(&app,"platform-second").await.account.id,"grant":true}),
    )
    .await;
    let granted = admin_status(&app, actor(&app, "platform-second").await.account.id).await;

    assert!(status.is_client_error());
    assert_eq!(granted, None);
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_operation_refused_when_admin_rights_withdrawn_before_confirm() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":actor(&app,"platform-second").await.account.id,"grant":true}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("platform_admins")
                .value("status", Expr::val("disabled"))
                .and_where(Expr::col("account_id").eq(Expr::val(app.fixture.account.id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let granted = admin_status(&app, actor(&app, "platform-second").await.account.id).await;

    assert!(!confirm.is_success());
    assert_eq!(granted, None);
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_operation_refused_when_session_revoked_before_confirm() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":actor(&app,"platform-second").await.account.id,"grant":true}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::delete()
                .from_table("identity_sessions")
                .and_where(Expr::col("account_id").eq(app.fixture.account.id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let granted = admin_status(&app, actor(&app, "platform-second").await.account.id).await;

    assert!(!confirm.is_success());
    assert_eq!(granted, None);
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_proof_bound_to_requesting_session() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    let cookie = admin_session(&app).await;
    let other = session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":actor(&app,"platform-second").await.account.id,"grant":true}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    let confirm = operation_confirm(&app, &other, &operation_id).await;
    let granted = admin_status(&app, actor(&app, "platform-second").await.account.id).await;

    assert!(!confirm.is_success());
    assert_eq!(granted, None);
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_proof_bound_to_target_account() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    synthetic_account(&app, "platform-third", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":actor(&app,"platform-second").await.account.id,"grant":true}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let granted = admin_status(&app, actor(&app, "platform-second").await.account.id).await;
    let untouched = admin_status(&app, actor(&app, "platform-third").await.account.id).await;

    assert!(confirm.is_success());
    assert_eq!(granted.as_deref(), Some("active"));
    assert_eq!(untouched, None);
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_proof_bound_to_action() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_user","account_id":actor(&app,"platform-second").await.account.id,"active":false}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let active = user_active(&app, actor(&app, "platform-second").await.user.id).await;
    let granted = admin_status(&app, actor(&app, "platform-second").await.account.id).await;

    assert!(confirm.is_success());
    assert_eq!(active, Some(false));
    assert_eq!(
        granted, None,
        "a platform_user proof must not create administrator rights"
    );
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_proof_expires() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":actor(&app,"platform-second").await.account.id,"grant":true}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("identity_verifications")
                .value(
                    "expires_at",
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new()))
                        .sub(Expr::val("1 minute").cast_as(Alias::new("interval"))),
                )
                .and_where(
                    Expr::col("identifier")
                        .eq(Expr::val(format!("password-change:{operation_id}"))),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let granted = admin_status(&app, actor(&app, "platform-second").await.account.id).await;

    assert!(!confirm.is_success());
    assert_eq!(granted, None);
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_proof_single_use() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":actor(&app,"platform-second").await.account.id,"grant":true}),
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
    let granted = admin_status(&app, actor(&app, "platform-second").await.account.id).await;

    assert!(!replay.is_success());
    assert_eq!(granted.as_deref(), Some("active"));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_administrator_revoke_refuses_last_active_administrator() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let admins_before = isolate_administrators(&app).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":app.fixture.account.id,"grant":false}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let remaining = admin_status(&app, app.fixture.account.id).await;

    assert!(!confirm.is_success());
    assert_eq!(remaining.as_deref(), Some("active"));
    finish(&app, Some(&admins_before), None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_user_disable_refuses_last_active_administrator() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let admins_before = isolate_administrators(&app).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_user","account_id":app.fixture.account.id,"active":false}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let active = user_active(&app, app.fixture.user.id).await;

    assert!(!confirm.is_success());
    assert_eq!(active, Some(true));
    finish(&app, Some(&admins_before), None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_last_administrator_guard_survives_concurrent_removals() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let admins_before = isolate_administrators(&app).await;
    synthetic_account(&app, "platform-second", 2).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("platform_admins")
                .columns(["account_id", "status", "created_at", "updated_at"])
                .values_panic([
                    Expr::val(actor(&app, "platform-second").await.account.id),
                    Expr::val("active"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let first = admin_session(&app).await;
    let second = sign_in(
        &app,
        actor(&app, "platform-second").await.account.id,
        &actor(&app, "platform-second").await.account.email,
    )
    .await;
    let (status, body) = operation_start(
        &app,
        &first,
        json!({"action":"platform_administrator","account_id":actor(&app,"platform-second").await.account.id,"grant":false}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let first_operation = body["operation_id"].as_str().unwrap().to_owned();
    let (status, body) = operation_start(
        &app,
        &second,
        json!({"action":"platform_administrator","account_id":app.fixture.account.id,"grant":false}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let second_operation = body["operation_id"].as_str().unwrap().to_owned();
    let barrier = app.fixture.admin.begin().await.unwrap();
    barrier
        .execute_raw({
            let query = Query::select()
                .expr(Expr::from(
                    Func::cust("pg_advisory_xact_lock").args([Expr::val(1920296809), Expr::val(3)]),
                ))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let release = async {
        let waiting = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let row =
                    app.fixture
                        .admin
                        .query_one_raw({
                            let query = Query::select()
                                .expr_as(Expr::col(sea_orm::sea_query::Asterisk).count(), "count")
                                .from("pg_locks")
                                .and_where(
                                    Expr::col("locktype").eq(Expr::val("advisory")).and(
                                        Expr::col("database")
                                            .eq(Expr::from(
                                                Query::select()
                                                    .expr(Expr::col("oid"))
                                                    .from("pg_database")
                                                    .and_where(Expr::col("datname").eq(Expr::from(
                                                        Func::cust("current_database").args(Vec::<
                                                            sea_orm::sea_query::Expr,
                                                        >::new(
                                                        )),
                                                    )))
                                                    .to_owned(),
                                            ))
                                            .and(
                                                Expr::col("classid").eq(Expr::val(1920296809)).and(
                                                    Expr::col("objid")
                                                        .eq(Expr::val(3))
                                                        .and(Expr::col("granted").not()),
                                                ),
                                            ),
                                    ),
                                )
                                .to_owned();
                            app.fixture.admin.get_database_backend().build(&query)
                        })
                        .await
                        .unwrap()
                        .unwrap();
                if row.try_get::<i64>("", "count").unwrap() == 2 {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await;
        barrier.commit().await.unwrap();
        waiting.unwrap();
    };
    let (one, two, ()) = tokio::join!(
        operation_confirm(&app, &first, &first_operation),
        operation_confirm(&app, &second, &second_operation),
        release
    );
    let remaining = admin_status(&app, app.fixture.account.id).await;
    let second_remaining =
        admin_status(&app, actor(&app, "platform-second").await.account.id).await;

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
    finish(&app, Some(&admins_before), None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_audit_failure_rolls_back_mutation_and_proof() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":actor(&app,"platform-second").await.account.id,"grant":true}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    // SeaQuery has no PostgreSQL GRANT/REVOKE builder; the real restricted-role denial is required.
    app.fixture
        .admin
        .execute_unprepared("REVOKE INSERT ON public.versions FROM med_tracker_app")
        .await
        .unwrap();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let blocked = admin_status(&app, actor(&app, "platform-second").await.account.id).await;
    // SeaQuery has no PostgreSQL GRANT/REVOKE builder; the real restricted-role denial is required.
    app.fixture
        .admin
        .execute_unprepared("GRANT INSERT ON public.versions TO med_tracker_app")
        .await
        .unwrap();
    let retry = operation_confirm(&app, &cookie, &operation_id).await;
    let granted = admin_status(&app, actor(&app, "platform-second").await.account.id).await;

    assert!(!confirm.is_success());
    assert_eq!(blocked, None, "failed audit must roll back the mutation");
    assert!(
        retry.is_success(),
        "failed mutation must not consume the proof"
    );
    assert_eq!(granted.as_deref(), Some("active"));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_inactive_user_administrator_is_not_viable_remaining_admin() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let admins_before = isolate_administrators(&app).await;
    synthetic_account(&app, "platform-second", 2).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("platform_admins")
                .columns(["account_id", "status", "created_at", "updated_at"])
                .values_panic([
                    Expr::val(actor(&app, "platform-second").await.account.id),
                    Expr::val("active"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("users")
                .value("active", Expr::val(false))
                .and_where(
                    Expr::col("id").eq(Expr::val(actor(&app, "platform-second").await.user.id)),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("households")
                .columns([
                    "created_by_account_id",
                    "name",
                    "slug",
                    "timezone",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(actor(&app, "platform-second").await.account.id),
                    Expr::val("Second profile household"),
                    Expr::val(format!(
                        "{}-{}",
                        "second-profile-household", app.fixture.client_uuid
                    )),
                    Expr::val("Europe/London"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture
                .admin
                .execute_raw(app.fixture.admin.get_database_backend().build(&query))
                .await
                .unwrap();
            let query = Query::insert()
                .into_table("household_memberships")
                .columns([
                    "household_id",
                    "person_id",
                    "role",
                    "status",
                    "joined_at",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(household_named(&app, "second-profile-household").await.id),
                    Expr::val(actor(&app, "platform-second").await.person.id),
                    Expr::val("member"),
                    Expr::val("active"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":app.fixture.account.id,"grant":false}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap().to_owned();
    let confirm = operation_confirm(&app, &cookie, &operation_id).await;
    let remaining = admin_status(&app, app.fixture.account.id).await;

    assert!(!confirm.is_success());
    assert_eq!(remaining.as_deref(), Some("active"));
    finish(&app, Some(&admins_before), None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_administrator_grant_refuses_ineligible_accounts() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-closed", 3).await;
    synthetic_account(&app, "platform-unverified", 1).await;
    synthetic_account(&app, "platform-mixed", 2).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("households")
                .columns([
                    "created_by_account_id",
                    "name",
                    "slug",
                    "timezone",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(actor(&app, "platform-mixed").await.account.id),
                    Expr::val("Second grant household"),
                    Expr::val(format!(
                        "{}-{}",
                        "second-grant-household", app.fixture.client_uuid
                    )),
                    Expr::val("Europe/London"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture
                .admin
                .execute_raw(app.fixture.admin.get_database_backend().build(&query))
                .await
                .unwrap();
            let query = Query::update()
                .table("users")
                .value("active", Expr::val(false))
                .and_where(
                    Expr::col("id").eq(Expr::val(actor(&app, "platform-mixed").await.user.id)),
                )
                .to_owned();
            app.fixture
                .admin
                .execute_raw(app.fixture.admin.get_database_backend().build(&query))
                .await
                .unwrap();
            let query = Query::insert()
                .into_table("household_memberships")
                .columns([
                    "household_id",
                    "person_id",
                    "role",
                    "status",
                    "joined_at",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(household_named(&app, "second-grant-household").await.id),
                    Expr::val(actor(&app, "platform-mixed").await.person.id),
                    Expr::val("member"),
                    Expr::val("active"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let cookie = admin_session(&app).await;
    for target in [
        actor(&app, "platform-closed").await.account.id,
        actor(&app, "platform-unverified").await.account.id,
        actor(&app, "platform-mixed").await.account.id,
        -1,
    ] {
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

    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_user_enable_refuses_closed_and_unverified_accounts() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-closed", 3).await;
    synthetic_account(&app, "platform-unverified", 1).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("users")
                .value("active", Expr::val(false))
                .and_where(Expr::col("id").is_in([
                    Expr::val(actor(&app, "platform-closed").await.user.id),
                    Expr::val(actor(&app, "platform-unverified").await.user.id),
                ]))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let cookie = admin_session(&app).await;
    for (account_id, user_id) in [
        (
            actor(&app, "platform-closed").await.account.id,
            actor(&app, "platform-closed").await.user.id,
        ),
        (
            actor(&app, "platform-unverified").await.account.id,
            actor(&app, "platform-unverified").await.user.id,
        ),
    ] {
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

    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_last_administrator_cannot_close_own_account() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let admins_before = isolate_administrators(&app).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(&app, &cookie, json!({"action":"close_account"})).await;
    let confirmed = if status.is_success() {
        operation_confirm(&app, &cookie, body["operation_id"].as_str().unwrap()).await
    } else {
        status
    };
    let account = account_status(&app, app.fixture.account.id).await;
    let active = user_active(&app, app.fixture.user.id).await;

    assert!(
        !confirmed.is_success(),
        "self-closure must preserve the last usable platform administrator"
    );
    assert_eq!(account, Some(2));
    assert_eq!(active, Some(true));
    finish(&app, Some(&admins_before), None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_last_administrator_guard_survives_concurrent_self_closures() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let admins_before = isolate_administrators(&app).await;
    synthetic_account(&app, "platform-second", 2).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("platform_admins")
                .columns(["account_id", "status", "created_at", "updated_at"])
                .values_panic([
                    Expr::val(actor(&app, "platform-second").await.account.id),
                    Expr::val("active"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let first = admin_session(&app).await;
    let second = sign_in(
        &app,
        actor(&app, "platform-second").await.account.id,
        &actor(&app, "platform-second").await.account.email,
    )
    .await;
    let (one, first_body) = operation_start(&app, &first, json!({"action":"close_account"})).await;
    let (two, second_body) =
        operation_start(&app, &second, json!({"action":"close_account"})).await;
    assert_eq!(one, reqwest::StatusCode::OK);
    assert_eq!(two, reqwest::StatusCode::OK);
    let (one, two) = tokio::join!(
        operation_confirm(&app, &first, first_body["operation_id"].as_str().unwrap()),
        operation_confirm(&app, &second, second_body["operation_id"].as_str().unwrap())
    );
    let first_status = account_status(&app, app.fixture.account.id).await;
    let second_status = account_status(&app, actor(&app, "platform-second").await.account.id).await;

    assert!(
        one.is_success() ^ two.is_success(),
        "exactly one self-closure can leave a usable administrator: {one} {two}"
    );
    assert!(
        !one.is_server_error() && !two.is_server_error(),
        "competing closures must return a deliberate conflict result: {one} {two}"
    );
    assert!(first_status == Some(2) || second_status == Some(2));
    finish(&app, Some(&admins_before), None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_can_revoke_an_inactive_administrator_without_removing_the_last_usable_admin() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let admins_before = isolate_administrators(&app).await;
    synthetic_account(&app, "platform-second", 2).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("platform_admins")
                .columns(["account_id", "status", "created_at", "updated_at"])
                .values_panic([
                    Expr::val(actor(&app, "platform-second").await.account.id),
                    Expr::val("active"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture
                .admin
                .execute_raw(app.fixture.admin.get_database_backend().build(&query))
                .await
                .unwrap();
            let query = Query::update()
                .table("users")
                .value("active", Expr::val(false))
                .and_where(
                    Expr::col("id").eq(Expr::val(actor(&app, "platform-second").await.user.id)),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":actor(&app,"platform-second").await.account.id,"grant":false}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let confirmed = operation_confirm(&app, &cookie, body["operation_id"].as_str().unwrap()).await;
    let disabled = admin_status(&app, actor(&app, "platform-second").await.account.id).await;
    let remaining = admin_status(&app, app.fixture.account.id).await;

    assert!(confirmed.is_success());
    assert_eq!(disabled.as_deref(), Some("disabled"));
    assert_eq!(remaining.as_deref(), Some("active"));
    finish(&app, Some(&admins_before), None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_confirmation_cannot_replace_the_bound_target_or_action() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    synthetic_account(&app, "platform-third", 2).await;
    let cookie = admin_session(&app).await;
    let (status, body) = operation_start(
        &app,
        &cookie,
        json!({"action":"platform_administrator","account_id":actor(&app,"platform-second").await.account.id,"grant":true}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    let operation_id = body["operation_id"].as_str().unwrap();
    let rejected = app.client.post(format!("{}/api/auth/security/password/confirm", app.origin))
        .header("cookie", &cookie).header("origin", app.context.config.server.full_url())
        .json(&json!({"operation_id":operation_id,"password":"password","account_id":actor(&app,"platform-third").await.account.id,"action":"platform_user","active":false}))
        .send().await.unwrap().status();
    assert!(!rejected.is_success());
    assert_eq!(
        admin_status(&app, actor(&app, "platform-second").await.account.id).await,
        None
    );
    assert_eq!(
        admin_status(&app, actor(&app, "platform-third").await.account.id).await,
        None
    );
    assert_eq!(
        user_active(&app, actor(&app, "platform-third").await.user.id).await,
        Some(true)
    );
    let accepted = operation_confirm(&app, &cookie, operation_id).await;
    let bound = admin_status(&app, actor(&app, "platform-second").await.account.id).await;
    let other = admin_status(&app, actor(&app, "platform-third").await.account.id).await;

    assert!(accepted.is_success());
    assert_eq!(bound.as_deref(), Some("active"));
    assert_eq!(other, None);
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_user_reactivation_does_not_revive_old_sessions() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    let administrator_cookie = admin_session(&app).await;
    let old = sign_in(
        &app,
        actor(&app, "platform-second").await.account.id,
        &actor(&app, "platform-second").await.account.email,
    )
    .await;
    for active in [false, true] {
        let (status, body) = operation_start(
            &app,
            &administrator_cookie,
            json!({"action":"platform_user","account_id":actor(&app,"platform-second").await.account.id,"active":active}),
        )
        .await;
        assert_eq!(status, reqwest::StatusCode::OK);
        assert!(
            operation_confirm(
                &app,
                &administrator_cookie,
                body["operation_id"].as_str().unwrap()
            )
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
    let fresh = sign_in(
        &app,
        actor(&app, "platform-second").await.account.id,
        &actor(&app, "platform-second").await.account.email,
    )
    .await;
    let current = client
        .get(format!("{}/account/security", app.origin))
        .header("cookie", fresh)
        .send()
        .await
        .unwrap()
        .status();

    assert!(stale_status.is_redirection());
    assert_eq!(stale_target.as_deref(), Some("/login"));
    assert_eq!(current, reqwest::StatusCode::OK);
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_users_page_shows_user_state_and_action_forms() {
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("users")
                .value("active", Expr::val(false))
                .and_where(
                    Expr::col("id").eq(Expr::val(actor(&app, "platform-second").await.user.id)),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let cookie = admin_fixture_session(&app).await;
    let (_cookies, body) = form_page(
        &app,
        &cookie,
        &format!("/platform/users?q={}", app.fixture.client_uuid),
    )
    .await;

    assert!(!hidden_field(&body, "authenticity_token").is_empty());
    assert!(str::contains(
        &body,
        "action=\"/platform/users/administrator\""
    ));
    assert!(str::contains(&body, "action=\"/platform/users/active\""));
    assert!(str::contains(
        &body,
        format!(
            "name=\"account_id\" value=\"{identity_71002}\"",
            identity_71002 = actor(&app, "platform-second").await.account.id
        )
        .as_str()
    ));
    assert!(
        str::contains(&body, "name=\"active\" value=\"true\""),
        "disabled user must offer an enable action"
    );
    assert!(
        str::contains(&body, "Sign-in disabled"),
        "page must show the linked user is disabled"
    );
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_users_page_links_are_bounded() {
    let app = Application::new().await;
    for i in 1..=300 {
        synthetic_account(&app, &format!("bulk-{}-{i:03}", app.fixture.client_uuid), 2).await;
    }
    let cookie = admin_fixture_session(&app).await;
    let (_cookies, body) = form_page(
        &app,
        &cookie,
        &format!(
            "/platform/users?q=bulk-{}&page=9223372036854775807",
            app.fixture.client_uuid
        ),
    )
    .await;
    let links = body.matches("join-item").count();

    assert!(links <= 9, "page links must be bounded, got {links}");
    assert!(
        str::contains(
            &body,
            format!(
                "bulk-{}-300-{}@example.test",
                app.fixture.client_uuid, app.fixture.client_uuid
            )
            .as_str()
        ),
        "oversized pages must show the final page"
    );
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_browser_administrator_grant_roundtrip() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    let cookie = admin_session(&app).await;
    let (cookies, body) = form_page(&app, &cookie, "/platform/users").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/users/administrator",
        &[
            (
                "account_id",
                &(actor(&app, "platform-second").await.account.id).to_string(),
            ),
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
    assert_eq!(
        admin_status(&app, actor(&app, "platform-second").await.account.id).await,
        None
    );
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    assert_eq!(
        admin_status(&app, actor(&app, "platform-second").await.account.id)
            .await
            .as_deref(),
        Some("active")
    );

    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_browser_preserves_the_authentication_origin_boundary() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
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
                (
                    "account_id",
                    (actor(&app, "platform-second").await.account.id)
                        .to_string()
                        .as_str(),
                ),
                ("grant", "true"),
                ("authenticity_token", token.as_str()),
            ])
            .unwrap(),
        )
        .send()
        .await
        .unwrap();
    let status = response.status();
    let target = admin_status(&app, actor(&app, "platform-second").await.account.id).await;

    assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
    assert_eq!(target, None);
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_settings_initialization_cannot_race_signup_into_two_rows() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let settings_before = isolate_settings(&app).await;
    // SeaQuery has no PostgreSQL trigger/function DDL builder; this barrier exercises singleton initialization.
    let barrier_name = format!("settings_barrier_{}", app.fixture.client_uuid.simple());
    app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION public.{barrier_name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_advisory_xact_lock(1920296809,77); RETURN NEW; END $$; CREATE TRIGGER {barrier_name} BEFORE INSERT ON app_settings FOR EACH ROW EXECUTE FUNCTION public.{barrier_name}()")).await.unwrap();
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
        .execute_raw({
            let query = Query::select()
                .expr(Expr::from(
                    Func::cust("pg_advisory_xact_lock")
                        .args([Expr::val(1920296809), Expr::val(77)]),
                ))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let release = async {
        let waiting = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let row =
                    app.fixture
                        .admin
                        .query_one_raw({
                            let query = Query::select()
                                .expr_as(Expr::col(sea_orm::sea_query::Asterisk).count(), "count")
                                .from("pg_locks")
                                .and_where(
                                    Expr::col("locktype").eq(Expr::val("advisory")).and(
                                        Expr::col("database")
                                            .eq(Expr::from(
                                                Query::select()
                                                    .expr(Expr::col("oid"))
                                                    .from("pg_database")
                                                    .and_where(Expr::col("datname").eq(Expr::from(
                                                        Func::cust("current_database").args(Vec::<
                                                            sea_orm::sea_query::Expr,
                                                        >::new(
                                                        )),
                                                    )))
                                                    .to_owned(),
                                            ))
                                            .and(
                                                Expr::col("classid").eq(Expr::val(1920296809)).and(
                                                    Expr::col("objid")
                                                        .is_in([Expr::val(1), Expr::val(77)])
                                                        .and(Expr::col("granted").not()),
                                                ),
                                            ),
                                    ),
                                )
                                .to_owned();
                            app.fixture.admin.get_database_backend().build(&query)
                        })
                        .await
                        .unwrap()
                        .unwrap();
                if row.try_get::<i64>("", "count").unwrap() == 2 {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await;
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
        .query_one_raw({
            let query = Query::select()
                .expr_as(Expr::col(sea_orm::sea_query::Asterisk).count(), "count")
                .expr_as(
                    Expr::from(Func::cust("bool_and").args([Expr::col("invite_only")])),
                    "closed",
                )
                .from("app_settings")
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap();
    let count = row.try_get::<i64>("", "count").unwrap();
    let closed = row.try_get::<bool>("", "closed").unwrap();

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
    finish(&app, None, Some(&settings_before)).await;
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "DROP TRIGGER {barrier_name} ON app_settings; DROP FUNCTION public.{barrier_name}()"
        ))
        .await
        .unwrap();
    app.close().await;
}

#[tokio::test]
async fn platform_browser_administrator_revoke_roundtrip() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("platform_admins")
                .columns(["account_id", "status", "created_at", "updated_at"])
                .values_panic([
                    Expr::val(actor(&app, "platform-second").await.account.id),
                    Expr::val("active"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let cookie = admin_session(&app).await;
    let (cookies, body) = form_page(&app, &cookie, "/platform/users").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/users/administrator",
        &[
            (
                "account_id",
                &(actor(&app, "platform-second").await.account.id).to_string(),
            ),
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
    assert_eq!(
        admin_status(&app, actor(&app, "platform-second").await.account.id)
            .await
            .as_deref(),
        Some("disabled")
    );

    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_browser_user_disable_and_enable_roundtrip() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    let cookie = admin_session(&app).await;
    for (active, expected) in [("false", false), ("true", true)] {
        let (cookies, body) = form_page(&app, &cookie, "/platform/users").await;
        let token = hidden_field(&body, "authenticity_token");
        let response = post_form(
            &app,
            &cookies,
            "/platform/users/active",
            &[
                (
                    "account_id",
                    &(actor(&app, "platform-second").await.account.id).to_string(),
                ),
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
        assert_eq!(
            user_active(&app, actor(&app, "platform-second").await.user.id).await,
            Some(expected)
        );
        assert_eq!(
            account_status(&app, actor(&app, "platform-second").await.account.id).await,
            Some(2)
        );
    }

    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_browser_forms_require_csrf_and_platform_rights() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    let cookie = admin_session(&app).await;
    let denied = post_form(
        &app,
        &cookie,
        "/platform/users/administrator",
        &[
            (
                "account_id",
                &(actor(&app, "platform-second").await.account.id).to_string(),
            ),
            ("grant", "true"),
        ],
    )
    .await;
    assert!(!denied.status().is_success());
    let bogus = post_form(
        &app,
        &cookie,
        "/platform/users/administrator",
        &[
            (
                "account_id",
                &(actor(&app, "platform-second").await.account.id).to_string(),
            ),
            ("grant", "true"),
            ("authenticity_token", "bogus"),
        ],
    )
    .await;
    assert!(!bogus.status().is_success());
    let member = session(&app).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("platform_admins")
                .value("status", Expr::val("disabled"))
                .and_where(Expr::col("account_id").eq(Expr::val(app.fixture.account.id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let (cookies, _body) = form_page(&app, &member, "/account/security").await;
    let token = hidden_field(&_body, "authenticity_token");
    let forbidden = post_form(
        &app,
        &cookies,
        "/platform/users/administrator",
        &[
            (
                "account_id",
                &(actor(&app, "platform-second").await.account.id).to_string(),
            ),
            ("grant", "true"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    assert!(!forbidden.status().is_success());
    assert_eq!(
        admin_status(&app, actor(&app, "platform-second").await.account.id).await,
        None
    );

    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_browser_last_administrator_revoke_conflicts() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let admins_before = isolate_administrators(&app).await;
    let cookie = admin_session(&app).await;
    let (cookies, body) = form_page(&app, &cookie, "/platform/users").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/users/administrator",
        &[
            ("account_id", &(app.fixture.account.id).to_string()),
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
    assert_eq!(
        admin_status(&app, app.fixture.account.id).await.as_deref(),
        Some("active")
    );

    finish(&app, Some(&admins_before), None).await;
    app.close().await;
}

async fn app_settings(app: &Application) -> Option<(bool, String, String)> {
    app.fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(Expr::col("invite_only"))
                .expr(Expr::col("medicine_lookup_base_url"))
                .expr(Expr::col("medicine_lookup_token_url"))
                .from("app_settings")
                .order_by("id", sea_orm::Order::Asc)
                .limit(1)
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .and_then(|row| {
            Some((
                row.try_get::<bool>("", "invite_only").ok()?,
                row.try_get::<String>("", "medicine_lookup_base_url").ok()?,
                row.try_get::<String>("", "medicine_lookup_token_url")
                    .ok()?,
            ))
        })
}

async fn platform_version_events(app: &Application, item_type: &str) -> i64 {
    app.fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(Expr::col(sea_orm::sea_query::Asterisk).count(), "count")
                .from("versions")
                .and_where(Expr::col("item_type").eq(Expr::val(item_type)))
                .and_where(
                    Expr::from(
                        Func::cust("jsonb_extract_path_text")
                            .args([Expr::col("audit_context"), Expr::val("actor_account_id")]),
                    )
                    .eq(app.fixture.account.id.to_string()),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .and_then(|row| row.try_get::<i64>("", "count").ok())
        .unwrap_or_default()
}

#[tokio::test]
async fn platform_settings_requires_platform_administrator() {
    let app = Application::new().await;
    let settings_before = isolate_settings(&app).await;
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
        .header(
            "cookie",
            fixture_session(&app, app.fixture.account.id).await,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(member.status(), reqwest::StatusCode::FORBIDDEN);
    let cookie = admin_fixture_session(&app).await;
    let (_cookies, body) = form_page(&app, &cookie, "/platform/settings").await;

    assert!(str::contains(&body, "medicine_lookup_base_url"));
    assert!(str::contains(&body, "invite_only"));
    assert!(str::contains(&body, "ontology.nhs.uk"));
    assert!(!str::contains(&body, "password_hash"));
    finish(&app, None, Some(&settings_before)).await;
    app.close().await;
}

#[tokio::test]
async fn platform_settings_update_via_browser_proof() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let settings_before = isolate_settings(&app).await;
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

    assert!(audits > before, "settings change must be audited");
    finish(&app, None, Some(&settings_before)).await;
    app.close().await;
}

#[tokio::test]
async fn platform_settings_rejects_insecure_and_unknown_values() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let settings_before = isolate_settings(&app).await;
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

    finish(&app, None, Some(&settings_before)).await;
    app.close().await;
}

#[tokio::test]
async fn platform_settings_audit_failure_rolls_back() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let settings_before = isolate_settings(&app).await;
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
    // SeaQuery has no PostgreSQL GRANT/REVOKE builder; the real restricted-role denial is required.
    app.fixture
        .admin
        .execute_unprepared("REVOKE INSERT ON public.versions FROM med_tracker_app")
        .await
        .unwrap();
    let (blocked, _) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    let unchanged = app_settings(&app).await.map(|row| row.1);
    // SeaQuery has no PostgreSQL GRANT/REVOKE builder; the real restricted-role denial is required.
    app.fixture
        .admin
        .execute_unprepared("GRANT INSERT ON public.versions TO med_tracker_app")
        .await
        .unwrap();
    let (retried, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    let applied = app_settings(&app).await.map(|row| row.1);

    assert!(!blocked.is_success());
    assert_eq!(unchanged, None);
    assert!(
        retried.is_success(),
        "proof must survive an audit failure: {outcome}"
    );
    assert_eq!(applied, Some("https://lookup.example.test/fhir".to_owned()));
    finish(&app, None, Some(&settings_before)).await;
    app.close().await;
}

async fn household(app: &Application, label: &str, name: &str) -> households::Model {
    let now = chrono::Utc::now().naive_utc();
    households::ActiveModel {
        created_by_account_id: Set(Some(app.fixture.account.id)),
        name: Set(name.into()),
        slug: Set(format!("{label}-{}", app.fixture.client_uuid)),
        timezone: Set("Europe/London".into()),
        status: Set("active".into()),
        lifecycle_state: Set("active".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap()
}
async fn household_member(
    app: &Application,
    household: i64,
    label: &str,
    name: &str,
    role: &str,
) -> household_memberships::Model {
    let member = synthetic_account(app, label, 2).await;
    let mut person = member.person.clone().into_active_model();
    person.name = Set(name.into());
    person.update(&app.fixture.admin).await.unwrap();
    let mut account = member.account.into_active_model();
    account.name = Set(Some(name.into()));
    account.update(&app.fixture.admin).await.unwrap();
    let now = chrono::Utc::now().naive_utc();
    household_memberships::ActiveModel {
        household_id: Set(household),
        person_id: Set(member.person.id),
        role: Set(role.into()),
        status: Set("active".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap()
}
async fn recovery_household(app: &Application) {
    let target = household(app, "recovery-fixture", "Recovery household").await;
    household_member(
        app,
        target.id,
        "recovery-member",
        "Recovery member",
        "member",
    )
    .await;
}
async fn supported_household(app: &Application) {
    let target = household(app, "supported-fixture", "Supported household").await;
    household_member(app, target.id, "support-owner", "Supported owner", "owner").await;
    household_member(
        app,
        target.id,
        "supported-member",
        "Supported member",
        "member",
    )
    .await;
    let now = chrono::Utc::now().naive_utc();
    let location = locations::ActiveModel {
        household_id: Set(target.id),
        name: Set("Supported cabinet".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    medications::ActiveModel {
        household_id: Set(target.id),
        location_id: Set(location.id),
        name: Set(Some("Supported tablets".into())),
        current_supply: Set(Some(sea_orm::prelude::Decimal::new(10, 0))),
        dose_amount: Set(Some(2.0)),
        dose_unit: Set(Some("tablet".into())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
}

async fn membership_state(app: &Application, id: i64) -> Option<(String, i32)> {
    app.fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(Expr::col("role"))
                .expr(Expr::col("permissions_version"))
                .from("household_memberships")
                .and_where(Expr::col("id").eq(Expr::val(id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
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
        .query_one_raw({
            let query = Query::select()
                .expr_as(Expr::col(sea_orm::sea_query::Asterisk).count(), "count")
                .from("household_memberships")
                .and_where(Expr::col("household_id").eq(Expr::val(household_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
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
        .query_one_raw({
            let query = Query::select()
                .expr(Expr::col("id"))
                .expr_as(
                    Expr::col("expires_at").cast_as(Alias::new("text")),
                    "expires_at",
                )
                .expr_as(
                    Expr::col("ended_at").cast_as(Alias::new("text")),
                    "ended_at",
                )
                .expr_as(
                    Expr::col("expired_at").cast_as(Alias::new("text")),
                    "expired_at",
                )
                .from("support_access_sessions")
                .and_where(Expr::col("household_id").eq(Expr::val(household_id)))
                .order_by("id", sea_orm::Order::Desc)
                .limit(1)
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
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
        .query_one_raw({
            let now = Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new()));
            let interval = |text: &str| Expr::val(text.to_owned()).cast_as(Alias::new("interval"));
            let query = Query::select()
                .expr_as(
                    Expr::col("expires_at").between(
                        now.clone().add(interval("23 hours")),
                        now.clone().add(interval("25 hours")),
                    ),
                    "request_deadline",
                )
                .expr_as(
                    Expr::col("expires_at").between(
                        now.clone().add(interval("29 minutes")),
                        now.clone().add(interval("31 minutes")),
                    ),
                    "active_window",
                )
                .expr_as(Expr::col("starts_at").lte(now), "started")
                .expr_as(
                    Expr::col("ended_at")
                        .is_null()
                        .and(Expr::col("expired_at").is_null()),
                    "open",
                )
                .from("support_access_sessions")
                .and_where(Expr::col("id").eq(id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
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
        .query_one_raw({
            let query = Query::select()
                .expr(Expr::col("platform_admin_id"))
                .from("support_access_sessions")
                .and_where(Expr::col("id").eq(Expr::val(id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
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

pub(super) async fn active_support(app: &Application, admin_cookie: &str) -> i64 {
    supported_household(app).await;
    let owner = sign_in(
        app,
        actor(app, "support-owner").await.account.id,
        &actor(app, "support-owner").await.account.email,
    )
    .await;
    activate_seeded_support(app, admin_cookie, &owner).await
}

async fn active_fixture_support(app: &Application, admin_cookie: &str) -> i64 {
    supported_household(app).await;
    let owner = fixture_session(app, actor(app, "support-owner").await.account.id).await;
    activate_seeded_support(app, admin_cookie, &owner).await
}

async fn activate_seeded_support(app: &Application, admin_cookie: &str, owner: &str) -> i64 {
    let (cookies, operation_id, confirm_token) = request_support(
        app,
        admin_cookie,
        household_named(app, "supported-fixture").await.id,
        "Confidential support reason alpha",
    )
    .await;
    let (confirmed, outcome) = browser_confirm(app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) = support_session(app, household_named(app, "supported-fixture").await.id)
        .await
        .unwrap();
    let approved = approve_support(app, owner, support_id).await;
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
        .get(format!(
            "{}/platform/owner-recovery?search={}",
            app.origin, app.fixture.client_uuid
        ))
        .send()
        .await
        .unwrap();
    assert!(anonymous.status().is_redirection());
    let member = client
        .get(format!("{}/platform/owner-recovery", app.origin))
        .header(
            "cookie",
            fixture_session(&app, app.fixture.account.id).await,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(member.status(), reqwest::StatusCode::FORBIDDEN);
    let cookie = admin_fixture_session(&app).await;
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
                "{}/platform/owner-recovery?household={identity_72601}",
                app.origin,
                identity_72601 = household_named(&app, "recovery-fixture").await.id
            ))
            .header("cookie", &cookie)
            .send()
            .await
            .unwrap();
        (response.status(), response.text().await.unwrap())
    };

    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    assert!(str::contains(&body, "Recovery household"), "{body}");
    assert!(
        !str::contains(&body, "Recovery member"),
        "the selection list must not expand household members: {body}"
    );
    assert_eq!(detail_status, reqwest::StatusCode::OK, "{detail}");
    assert!(str::contains(&detail, "Recovery member"), "{detail}");
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_owner_recovery_promotes_eligible_member_via_browser_proof() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    recovery_household(&app).await;
    let cookie = admin_session(&app).await;
    let before = membership_count(&app, household_named(&app, "recovery-fixture").await.id).await;
    let (cookies, body) = form_page(
        &app,
        &cookie,
        &format!(
            "/platform/owner-recovery?household={identity_72601}",
            identity_72601 = household_named(&app, "recovery-fixture").await.id
        ),
    )
    .await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/owner-recovery",
        &[
            (
                "household_id",
                &(household_named(&app, "recovery-fixture").await.id).to_string(),
            ),
            (
                "membership_id",
                &(member_named(&app, "recovery-fixture", "recovery-member")
                    .await
                    .id)
                    .to_string(),
            ),
            ("reason", "Household owner left the platform"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let (cookies, operation_id, confirm_token) = start_browser_proof(response, &cookies).await;
    assert_eq!(
        membership_state(
            &app,
            member_named(&app, "recovery-fixture", "recovery-member")
                .await
                .id
        )
        .await
        .map(|row| row.0),
        Some("member".to_owned()),
        "no mutation at proof start"
    );
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let promoted = membership_state(
        &app,
        member_named(&app, "recovery-fixture", "recovery-member")
            .await
            .id,
    )
    .await
    .unwrap();
    let count = membership_count(&app, household_named(&app, "recovery-fixture").await.id).await;
    let audits = platform_version_events(&app, "HouseholdMembership").await;

    assert_eq!(promoted.0, "owner");
    assert!(promoted.1 > 1, "permissions_version must be bumped");
    assert_eq!(count, before, "recovery must not create memberships");
    assert!(
        audits > 0,
        "owner recovery must record a versions audit event"
    );
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_owner_recovery_resulting_owner_authority_is_immediate() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    recovery_household(&app).await;
    let cookie = admin_session(&app).await;
    let (cookies, body) = form_page(
        &app,
        &cookie,
        &format!(
            "/platform/owner-recovery?household={identity_72601}",
            identity_72601 = household_named(&app, "recovery-fixture").await.id
        ),
    )
    .await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/owner-recovery",
        &[
            (
                "household_id",
                &(household_named(&app, "recovery-fixture").await.id).to_string(),
            ),
            (
                "membership_id",
                &(member_named(&app, "recovery-fixture", "recovery-member")
                    .await
                    .id)
                    .to_string(),
            ),
            ("reason", "Household owner left the platform"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let (cookies, operation_id, confirm_token) = start_browser_proof(response, &cookies).await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let owner = sign_in(
        &app,
        actor(&app, "recovery-member").await.account.id,
        &actor(&app, "recovery-member").await.account.email,
    )
    .await;
    let admin_page = app
        .client
        .get(format!(
            "{}/households/{recovery_fixture_slug}/admin",
            app.origin,
            recovery_fixture_slug = household_named(&app, "recovery-fixture").await.slug
        ))
        .header("cookie", owner)
        .send()
        .await
        .unwrap()
        .status();

    assert_eq!(admin_page, reqwest::StatusCode::OK);
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_owner_recovery_requires_csrf() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    recovery_household(&app).await;
    let cookie = admin_session(&app).await;
    let response = post_form(
        &app,
        &cookie,
        "/platform/owner-recovery",
        &[
            (
                "household_id",
                &(household_named(&app, "recovery-fixture").await.id).to_string(),
            ),
            (
                "membership_id",
                &(member_named(&app, "recovery-fixture", "recovery-member")
                    .await
                    .id)
                    .to_string(),
            ),
            ("reason", "no token"),
        ],
    )
    .await;
    let status = response.status();
    let role = membership_state(
        &app,
        member_named(&app, "recovery-fixture", "recovery-member")
            .await
            .id,
    )
    .await
    .map(|row| row.0);

    assert!(status.is_client_error(), "{status}");
    assert_eq!(role, Some("member".to_owned()));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_owner_recovery_refuses_self_promotion() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    recovery_household(&app).await;
    let cookie = admin_session(&app).await;
    let (cookies, body) = form_page(
        &app,
        &cookie,
        &format!(
            "/platform/owner-recovery?household={identity_72601}",
            identity_72601 = household_named(&app, "recovery-fixture").await.id
        ),
    )
    .await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/owner-recovery",
        &[
            ("household_id", &(app.fixture.household.id).to_string()),
            ("membership_id", &(app.fixture.membership.id).to_string()),
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
    let role = membership_state(&app, app.fixture.membership.id)
        .await
        .map(|row| row.0);

    assert_eq!(role, Some("administrator".to_owned()));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_owner_recovery_refuses_ineligible_members() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    recovery_household(&app).await;
    household_member(
        &app,
        household_named(&app, "recovery-fixture").await.id,
        "recovery-minor",
        "Recovery minor",
        "member",
    )
    .await;
    household_member(
        &app,
        household_named(&app, "recovery-fixture").await.id,
        "recovery-dependent",
        "Recovery dependent",
        "member",
    )
    .await;
    household_member(
        &app,
        household_named(&app, "recovery-fixture").await.id,
        "recovery-nocapacity",
        "Recovery no capacity",
        "member",
    )
    .await;
    household_member(
        &app,
        household_named(&app, "recovery-fixture").await.id,
        "recovery-inactive",
        "Recovery inactive",
        "member",
    )
    .await;
    household_member(
        &app,
        household_named(&app, "recovery-fixture").await.id,
        "recovery-revoked",
        "Recovery revoked",
        "member",
    )
    .await;
    household_member(
        &app,
        household_named(&app, "recovery-fixture").await.id,
        "recovery-closed",
        "Recovery closed",
        "member",
    )
    .await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("people")
                .value("person_type", Expr::val(1))
                .value("has_capacity", Expr::val(false))
                .and_where(
                    Expr::col("id").eq(Expr::val(actor(&app, "recovery-minor").await.person.id)),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("people")
                .value("person_type", Expr::val(2))
                .value("has_capacity", Expr::val(false))
                .and_where(
                    Expr::col("id")
                        .eq(Expr::val(actor(&app, "recovery-dependent").await.person.id)),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("people")
                .value("has_capacity", Expr::val(false))
                .and_where(Expr::col("id").eq(Expr::val(
                    actor(&app, "recovery-nocapacity").await.person.id,
                )))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("users")
                .value("active", Expr::val(false))
                .and_where(
                    Expr::col("id").eq(Expr::val(actor(&app, "recovery-inactive").await.user.id)),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("household_memberships")
                .value("status", Expr::val("revoked"))
                .value(
                    "revoked_at",
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                )
                .and_where(
                    Expr::col("id").eq(Expr::val(
                        member_named(&app, "recovery-fixture", "recovery-revoked")
                            .await
                            .id,
                    )),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("accounts")
                .value("status", Expr::val(3))
                .and_where(
                    Expr::col("id").eq(Expr::val(actor(&app, "recovery-closed").await.account.id)),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let cookie = admin_session(&app).await;
    for (membership, label) in [
        (
            member_named(&app, "recovery-fixture", "recovery-minor")
                .await
                .id,
            "minor",
        ),
        (
            member_named(&app, "recovery-fixture", "recovery-dependent")
                .await
                .id,
            "dependent adult",
        ),
        (
            member_named(&app, "recovery-fixture", "recovery-nocapacity")
                .await
                .id,
            "no capacity",
        ),
        (
            member_named(&app, "recovery-fixture", "recovery-inactive")
                .await
                .id,
            "inactive user",
        ),
        (
            member_named(&app, "recovery-fixture", "recovery-revoked")
                .await
                .id,
            "revoked membership",
        ),
        (
            member_named(&app, "recovery-fixture", "recovery-closed")
                .await
                .id,
            "closed account",
        ),
    ] {
        let (cookies, body) = form_page(
            &app,
            &cookie,
            &format!(
                "/platform/owner-recovery?household={identity_72601}",
                identity_72601 = household_named(&app, "recovery-fixture").await.id
            ),
        )
        .await;
        let token = hidden_field(&body, "authenticity_token");
        let response = post_form(
            &app,
            &cookies,
            "/platform/owner-recovery",
            &[
                (
                    "household_id",
                    &(household_named(&app, "recovery-fixture").await.id).to_string(),
                ),
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

    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_owner_recovery_refuses_healthy_owner_household() {
    crate::authentication_fixture::reset_source_quota().await;
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
            (
                "household_id",
                &(household_named(&app, "supported-fixture").await.id).to_string(),
            ),
            (
                "membership_id",
                &(member_named(&app, "supported-fixture", "supported-member")
                    .await
                    .id)
                    .to_string(),
            ),
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
    let role = membership_state(
        &app,
        member_named(&app, "supported-fixture", "supported-member")
            .await
            .id,
    )
    .await
    .map(|row| row.0);

    assert_eq!(role, Some("member".to_owned()));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_owner_recovery_refuses_cross_household_target() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    recovery_household(&app).await;
    supported_household(&app).await;
    let cookie = admin_session(&app).await;
    let (cookies, body) = form_page(
        &app,
        &cookie,
        &format!(
            "/platform/owner-recovery?household={identity_72601}",
            identity_72601 = household_named(&app, "recovery-fixture").await.id
        ),
    )
    .await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/owner-recovery",
        &[
            (
                "household_id",
                &(household_named(&app, "supported-fixture").await.id).to_string(),
            ),
            (
                "membership_id",
                &(member_named(&app, "recovery-fixture", "recovery-member")
                    .await
                    .id)
                    .to_string(),
            ),
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
    let role = membership_state(
        &app,
        member_named(&app, "recovery-fixture", "recovery-member")
            .await
            .id,
    )
    .await
    .map(|row| row.0);

    assert_eq!(role, Some("member".to_owned()));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_owner_recovery_refused_when_admin_rights_withdrawn_before_confirm() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    recovery_household(&app).await;
    let cookie = admin_session(&app).await;
    let (cookies, body) = form_page(
        &app,
        &cookie,
        &format!(
            "/platform/owner-recovery?household={identity_72601}",
            identity_72601 = household_named(&app, "recovery-fixture").await.id
        ),
    )
    .await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/owner-recovery",
        &[
            (
                "household_id",
                &(household_named(&app, "recovery-fixture").await.id).to_string(),
            ),
            (
                "membership_id",
                &(member_named(&app, "recovery-fixture", "recovery-member")
                    .await
                    .id)
                    .to_string(),
            ),
            ("reason", "recovery"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let (cookies, operation_id, confirm_token) = start_browser_proof(response, &cookies).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("platform_admins")
                .value("status", Expr::val("revoked"))
                .value(
                    "updated_at",
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                )
                .and_where(Expr::col("account_id").eq(Expr::val(app.fixture.account.id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    let role = membership_state(
        &app,
        member_named(&app, "recovery-fixture", "recovery-member")
            .await
            .id,
    )
    .await
    .map(|row| row.0);

    assert!(
        !confirmed.is_success(),
        "withdrawn authority must fail: {outcome}"
    );
    assert_eq!(role, Some("member".to_owned()));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_owner_recovery_proof_binds_household_target_and_reason() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    recovery_household(&app).await;
    supported_household(&app).await;
    household_member(
        &app,
        household_named(&app, "recovery-fixture").await.id,
        "recovery-other",
        "Recovery other",
        "member",
    )
    .await;
    let cookie = admin_session(&app).await;
    let (cookies, body) = form_page(
        &app,
        &cookie,
        &format!(
            "/platform/owner-recovery?household={identity_72601}",
            identity_72601 = household_named(&app, "recovery-fixture").await.id
        ),
    )
    .await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/owner-recovery",
        &[
            (
                "household_id",
                &(household_named(&app, "recovery-fixture").await.id).to_string(),
            ),
            (
                "membership_id",
                &(member_named(&app, "recovery-fixture", "recovery-member")
                    .await
                    .id)
                    .to_string(),
            ),
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
            (
                "household_id",
                &(household_named(&app, "supported-fixture").await.id).to_string(),
            ),
            (
                "membership_id",
                &(member_named(&app, "recovery-fixture", "recovery-other")
                    .await
                    .id)
                    .to_string(),
            ),
            ("reason", "substituted reason"),
        ],
    )
    .await
    .status();
    let first = membership_state(
        &app,
        member_named(&app, "recovery-fixture", "recovery-member")
            .await
            .id,
    )
    .await
    .map(|row| row.0);
    let second = membership_state(
        &app,
        member_named(&app, "recovery-fixture", "recovery-other")
            .await
            .id,
    )
    .await
    .map(|row| row.0);
    let applied = if substituted.is_success() {
        first == Some("owner".to_owned()) && second == Some("member".to_owned())
    } else {
        let (confirmed, _) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
        confirmed.is_success()
            && membership_state(
                &app,
                member_named(&app, "recovery-fixture", "recovery-member")
                    .await
                    .id,
            )
            .await
            .map(|row| row.0)
                == Some("owner".to_owned())
            && membership_state(
                &app,
                member_named(&app, "recovery-fixture", "recovery-other")
                    .await
                    .id,
            )
            .await
            .map(|row| row.0)
                == Some("member".to_owned())
    };

    assert!(
        applied,
        "proof must apply only its bound household/membership/reason"
    );
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_owner_recovery_serializes_concurrent_confirmations() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    recovery_household(&app).await;
    household_member(
        &app,
        household_named(&app, "recovery-fixture").await.id,
        "recovery-other",
        "Recovery other",
        "member",
    )
    .await;
    synthetic_account(&app, "platform-second", 2).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("platform_admins")
                .columns(["account_id", "status", "created_at", "updated_at"])
                .values_panic([
                    Expr::val(actor(&app, "platform-second").await.account.id),
                    Expr::val("active"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let first = admin_session(&app).await;
    let second = sign_in(
        &app,
        actor(&app, "platform-second").await.account.id,
        &actor(&app, "platform-second").await.account.email,
    )
    .await;
    let mut proofs = Vec::new();
    for (cookie, membership) in [
        (
            &first,
            member_named(&app, "recovery-fixture", "recovery-member")
                .await
                .id,
        ),
        (
            &second,
            member_named(&app, "recovery-fixture", "recovery-other")
                .await
                .id,
        ),
    ] {
        let (cookies, body) = form_page(
            &app,
            cookie,
            &format!(
                "/platform/owner-recovery?household={identity_72601}",
                identity_72601 = household_named(&app, "recovery-fixture").await.id
            ),
        )
        .await;
        let token = hidden_field(&body, "authenticity_token");
        let response = post_form(
            &app,
            &cookies,
            "/platform/owner-recovery",
            &[
                (
                    "household_id",
                    &(household_named(&app, "recovery-fixture").await.id).to_string(),
                ),
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
    let first_role = membership_state(
        &app,
        member_named(&app, "recovery-fixture", "recovery-member")
            .await
            .id,
    )
    .await
    .map(|row| row.0);
    let second_role = membership_state(
        &app,
        member_named(&app, "recovery-fixture", "recovery-other")
            .await
            .id,
    )
    .await
    .map(|row| row.0);
    let owners = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(Expr::col(sea_orm::sea_query::Asterisk).count(), "count")
                .from("household_memberships")
                .and_where(
                    Expr::col("household_id")
                        .eq(Expr::val(
                            household_named(&app, "recovery-fixture").await.id,
                        ))
                        .and(
                            Expr::col("role").eq(Expr::val("owner")).and(
                                Expr::col("status")
                                    .eq(Expr::val("active"))
                                    .and(Expr::col("revoked_at").is_null()),
                            ),
                        ),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .and_then(|row| row.try_get::<i64>("", "count").ok())
        .unwrap_or_default();

    assert!(
        !lost.is_success(),
        "second recovery must lose the race: {outcome}"
    );
    assert_eq!(first_role, Some("owner".to_owned()));
    assert_eq!(second_role, Some("member".to_owned()));
    assert_eq!(owners, 1);
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_owner_recovery_audit_failure_rolls_back() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    recovery_household(&app).await;
    let cookie = admin_session(&app).await;
    let (cookies, body) = form_page(
        &app,
        &cookie,
        &format!(
            "/platform/owner-recovery?household={identity_72601}",
            identity_72601 = household_named(&app, "recovery-fixture").await.id
        ),
    )
    .await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/owner-recovery",
        &[
            (
                "household_id",
                &(household_named(&app, "recovery-fixture").await.id).to_string(),
            ),
            (
                "membership_id",
                &(member_named(&app, "recovery-fixture", "recovery-member")
                    .await
                    .id)
                    .to_string(),
            ),
            ("reason", "recovery"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let (cookies, operation_id, confirm_token) = start_browser_proof(response, &cookies).await;
    // SeaQuery has no PostgreSQL GRANT/REVOKE builder; the real restricted-role denial is required.
    app.fixture
        .admin
        .execute_unprepared("REVOKE INSERT ON public.versions FROM med_tracker_app")
        .await
        .unwrap();
    let (blocked, _) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    let unchanged = membership_state(
        &app,
        member_named(&app, "recovery-fixture", "recovery-member")
            .await
            .id,
    )
    .await
    .map(|row| row.0);
    // SeaQuery has no PostgreSQL GRANT/REVOKE builder; the real restricted-role denial is required.
    app.fixture
        .admin
        .execute_unprepared("GRANT INSERT ON public.versions TO med_tracker_app")
        .await
        .unwrap();
    let (retried, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    let applied = membership_state(
        &app,
        member_named(&app, "recovery-fixture", "recovery-member")
            .await
            .id,
    )
    .await
    .map(|row| row.0);

    assert!(!blocked.is_success());
    assert_eq!(unchanged, Some("member".to_owned()));
    assert!(
        retried.is_success(),
        "proof must survive an audit failure: {outcome}"
    );
    assert_eq!(applied, Some("owner".to_owned()));
    finish(&app, None, None).await;
    app.close().await;
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
        .header(
            "cookie",
            fixture_session(&app, app.fixture.account.id).await,
        )
        .send()
        .await
        .unwrap();
    assert_eq!(member.status(), reqwest::StatusCode::FORBIDDEN);
    let cookie = admin_fixture_session(&app).await;
    let response = app
        .client
        .get(format!("{}/platform/support", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();

    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_support_pagination_keeps_older_approved_requests_reachable() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let cookie = admin_fixture_session(&app).await;
    let support_id = active_fixture_support(&app, &cookie).await;
    let seed = support_access_sessions::Entity::find_by_id(support_id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut pending = seed.clone().into_active_model();
    pending.activated_at = Set(None);
    pending.update(&app.fixture.admin).await.unwrap();
    for i in 1..=50 {
        let mut newer = seed.clone().into_active_model();
        newer.id = NotSet;
        newer.reason = Set(format!("Newer support request {i}"));
        newer.ended_at = Set(Some(chrono::Utc::now().naive_utc()));
        newer.insert(&app.fixture.admin).await.unwrap();
    }
    let (_, first) = form_page(&app, &cookie, "/platform/support").await;
    let (_, second) = form_page(&app, &cookie, "/platform/support?page=2").await;
    let (_, oversized) =
        form_page(&app, &cookie, "/platform/support?page=9223372036854775807").await;

    assert!(str::contains(&first, "href=\"/platform/support?page=2\""));
    assert!(!str::contains(&first, "Confidential support reason alpha"));
    assert!(str::contains(&second, "Confidential support reason alpha"));
    assert!(str::contains(
        &second,
        "action=\"/platform/support/activate\""
    ));
    assert!(str::contains(
        &second,
        format!("name=\"support_id\" value=\"{support_id}\"").as_str()
    ));
    assert!(str::contains(
        &oversized,
        "Confidential support reason alpha"
    ));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_support_request_requires_reason_and_sets_request_deadline() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    supported_household(&app).await;
    let cookie = admin_session(&app).await;
    let (cookies, body) = form_page(&app, &cookie, "/platform/support").await;
    let token = hidden_field(&body, "authenticity_token");
    let missing = post_form(
        &app,
        &cookies,
        "/platform/support",
        &[
            (
                "household_id",
                &(household_named(&app, "supported-fixture").await.id).to_string(),
            ),
            ("authenticity_token", &token),
        ],
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
        support_session(&app, household_named(&app, "supported-fixture").await.id)
            .await
            .is_none(),
        "no row without reason"
    );
    let (cookies, body) = form_page(&app, &cookie, "/platform/support").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/support",
        &[
            (
                "household_id",
                &(household_named(&app, "supported-fixture").await.id).to_string(),
            ),
            ("reason", "Confidential support reason alpha"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let (cookies, operation_id, confirm_token) = start_browser_proof(response, &cookies).await;
    assert!(
        support_session(&app, household_named(&app, "supported-fixture").await.id)
            .await
            .is_none(),
        "no row at proof start"
    );
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, _, ended, expired) =
        support_session(&app, household_named(&app, "supported-fixture").await.id)
            .await
            .unwrap();
    let (deadline, _, _, open) = support_window(&app, support_id).await.unwrap();
    let admin_id = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(Expr::col("id"))
                .from("platform_admins")
                .and_where(Expr::col("account_id").eq(Expr::val(app.fixture.account.id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .and_then(|row| row.try_get::<i64>("", "id").ok())
        .unwrap();
    let stored_admin = support_platform_admin(&app, support_id).await;

    assert_eq!(stored_admin, Some(admin_id));
    assert!(deadline, "request deadline must be 24 hours");
    assert!(open, "fresh request must not be ended or expired");
    assert_eq!(ended, None);
    assert_eq!(expired, None);
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_support_request_proof_binds_household_and_reason() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    supported_household(&app).await;
    recovery_household(&app).await;
    let cookie = admin_session(&app).await;
    let (cookies, operation_id, confirm_token) = request_support(
        &app,
        &cookie,
        household_named(&app, "supported-fixture").await.id,
        "Confidential support reason alpha",
    )
    .await;
    let substituted = post_form(
        &app,
        &cookies,
        "/account/security/password/confirm",
        &[
            ("operation_id", &operation_id),
            ("password", "password"),
            ("authenticity_token", &confirm_token),
            (
                "household_id",
                &(household_named(&app, "recovery-fixture").await.id).to_string(),
            ),
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
        .query_one_raw({
            let query = Query::select()
                .expr(Expr::col("household_id"))
                .expr(Expr::col("reason"))
                .from("support_access_sessions")
                .and_where(
                    Expr::col("platform_admin_id").in_subquery(
                        Query::select()
                            .column("id")
                            .from("platform_admins")
                            .and_where(Expr::col("account_id").eq(app.fixture.account.id))
                            .to_owned(),
                    ),
                )
                .order_by("id", sea_orm::Order::Desc)
                .limit(1)
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .map(|row| {
            (
                row.try_get::<i64>("", "household_id").unwrap(),
                row.try_get::<String>("", "reason").unwrap(),
            )
        })
        .unwrap();

    assert_eq!(
        row.0,
        household_named(&app, "supported-fixture").await.id,
        "proof must keep the bound household"
    );
    assert_eq!(row.1, "Confidential support reason alpha");
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_support_approval_read_and_termination_journey() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    supported_household(&app).await;
    let cookie = admin_session(&app).await;
    let owner = sign_in(
        &app,
        actor(&app, "support-owner").await.account.id,
        &actor(&app, "support-owner").await.account.email,
    )
    .await;
    let member = sign_in(
        &app,
        actor(&app, "supported-member").await.account.id,
        &actor(&app, "supported-member").await.account.email,
    )
    .await;
    let nonowner = app
        .client
        .get(format!("{}/account/support", app.origin))
        .header("cookie", &member)
        .send()
        .await
        .unwrap();
    assert_eq!(nonowner.status(), reqwest::StatusCode::FORBIDDEN);
    let (cookies, operation_id, confirm_token) = request_support(
        &app,
        &cookie,
        household_named(&app, "supported-fixture").await.id,
        "Confidential support reason alpha",
    )
    .await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) =
        support_session(&app, household_named(&app, "supported-fixture").await.id)
            .await
            .unwrap();
    let unapproved = activate_support(&app, &cookie, support_id).await;
    let (_, window, _, _) = support_window(&app, support_id).await.unwrap_or_default();
    assert!(
        !unapproved.is_success(),
        "unapproved request must not activate"
    );
    assert!(
        !window,
        "unapproved request must not gain a 30 minute window"
    );
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
        str::contains(&body, support_id.to_string().as_str())
            || str::contains(&body, "Supported household"),
        "{body}"
    );
    let approved = approve_support(&app, &owner, support_id).await;
    assert_eq!(approved, reqwest::StatusCode::OK);
    let activated = activate_support(&app, &cookie, support_id).await;
    assert_eq!(activated, reqwest::StatusCode::OK);
    let (_, window, started, open) = support_window(&app, support_id).await.unwrap();
    assert!(started, "activation must stamp starts_at");
    assert!(
        window,
        "activation window must be 30 minutes from database time"
    );
    assert!(open);
    assert_support_is_read_only(&app, &cookie, support_id).await;
    assert_support_end_is_idempotent_and_audited(&app, &cookie, support_id).await;

    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_support_activate_replay_and_wrong_operator_rejected() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    synthetic_account(&app, "platform-second", 2).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("platform_admins")
                .columns(["account_id", "status", "created_at", "updated_at"])
                .values_panic([
                    Expr::val(actor(&app, "platform-second").await.account.id),
                    Expr::val("active"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let cookie = admin_session(&app).await;
    let other = sign_in(
        &app,
        actor(&app, "platform-second").await.account.id,
        &actor(&app, "platform-second").await.account.email,
    )
    .await;
    let support_id = active_support(&app, &cookie).await;
    let wrong = activate_support(&app, &other, support_id).await;
    let replay = activate_support(&app, &cookie, support_id).await;
    let (_, window, _, open) = support_window(&app, support_id).await.unwrap();

    assert!(
        !wrong.is_success(),
        "a different admin must not activate this request"
    );
    assert!(!replay.is_success(), "activation must not be replayable");
    assert!(window && open);
    finish(&app, None, None).await;
    app.close().await;
}

async fn assert_support_is_read_only(app: &Application, cookie: &str, support_id: i64) {
    let read = app
        .client
        .get(format!("{}/platform/support/{support_id}", app.origin))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    let status = read.status();
    let body = read.text().await.unwrap();
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    assert!(str::contains(&body, "Supported household"), "{body}");
    assert!(str::contains(&body, "Supported member"), "{body}");
    assert!(str::contains(&body, "Supported tablets"), "{body}");
    let memberships = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(Expr::col(sea_orm::sea_query::Asterisk).count(), "count")
                .from("household_memberships")
                .and_where(
                    Expr::col("household_id")
                        .eq(Expr::val(
                            household_named(app, "supported-fixture").await.id,
                        ))
                        .and(
                            Expr::col("person_id").eq(Expr::from(
                                Query::select()
                                    .expr(Expr::col("person_id"))
                                    .from(("public", "accounts"))
                                    .and_where(
                                        Expr::col("id").eq(Expr::val(app.fixture.account.id)),
                                    )
                                    .to_owned(),
                            )),
                        ),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .and_then(|row| row.try_get::<i64>("", "count").ok())
        .unwrap_or_default();
    assert_eq!(
        memberships, 0,
        "support must not insert a household membership"
    );
    let write = post_form(
        app,
        cookie,
        &format!(
            "/households/{supported_fixture_slug}/admin/users/{identity_74605}/membership_role",
            identity_74605 = member_named(app, "supported-fixture", "support-owner")
                .await
                .id,
            supported_fixture_slug = household_named(app, "supported-fixture").await.slug
        ),
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
            "{}/households/{supported_fixture_slug}/reports/health-history",
            app.origin,
            supported_fixture_slug = household_named(app, "supported-fixture").await.slug
        ))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    assert!(
        !download.status().is_success(),
        "exports/downloads stay denied under support"
    );
    let role = membership_state(
        app,
        member_named(app, "supported-fixture", "support-owner")
            .await
            .id,
    )
    .await
    .map(|row| row.0);
    assert_eq!(role, Some("owner".to_owned()));
}

async fn assert_support_end_is_idempotent_and_audited(
    app: &Application,
    cookie: &str,
    support_id: i64,
) {
    let owner = sign_in(
        app,
        actor(app, "support-owner").await.account.id,
        &actor(app, "support-owner").await.account.email,
    )
    .await;
    let (cookies, body) = form_page(app, &owner, "/account/support").await;
    let token = hidden_field(&body, "authenticity_token");
    let response = post_form(
        app,
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
        let (ended, outcome) = browser_confirm(app, &cookies, &operation_id, &confirm_token).await;
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
    let first = support_session(app, household_named(app, "supported-fixture").await.id)
        .await
        .unwrap();
    assert!(first.2.is_some(), "ending must stamp ended_at");
    let read = app
        .client
        .get(format!("{}/platform/support/{support_id}", app.origin))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    assert!(
        !read.status().is_success(),
        "terminated support must not read"
    );
    let (cookies, body) = form_page(app, cookie, "/platform/support").await;
    let token = hidden_field(&body, "authenticity_token");
    let again = post_form(
        app,
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
            browser_confirm(app, &cookies, &operation_id, &confirm_token).await;
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
    let second = support_session(app, household_named(app, "supported-fixture").await.id)
        .await
        .unwrap();
    let mut audits = 0;
    for event in [
        "platform/support/requested",
        "platform/support/approved",
        "platform/support/activated",
        "platform/support/ended",
    ] {
        let count = support_audit_count(app, event, support_id).await;
        assert_eq!(
            count, 1,
            "{event} must be recorded once for this support session"
        );
        audits += count;
    }
    assert_eq!(
        first.2, second.2,
        "ended_at must not move on repeat termination"
    );
    assert!(audits >= 4, "request/approve/activate/end must each audit");
}

#[tokio::test]
async fn platform_support_expiry_revokes_read_access() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("support_access_sessions")
                .value(
                    "expires_at",
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new()))
                        .sub(Expr::val("1 second").cast_as(Alias::new("interval"))),
                )
                .value(
                    "updated_at",
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                )
                .and_where(Expr::col("id").eq(support_id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
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
    let row = support_session(&app, household_named(&app, "supported-fixture").await.id)
        .await
        .unwrap();
    let again = app
        .client
        .get(format!("{}/platform/support/{support_id}", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap()
        .status();

    assert!(!denied.is_success(), "expired support must not read");
    assert!(row.3.is_some(), "expiry must be persisted via expired_at");
    assert!(!again.is_success(), "expiry denial must be stable");
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_support_read_denied_after_authority_changes() {
    crate::authentication_fixture::reset_source_quota().await;
    for change in 0..4 {
        let app = Application::new().await;
        let cookie = admin_session(&app).await;
        let support_id = active_support(&app, &cookie).await;
        let query = match change {
            0 => Query::update()
                .table("platform_admins")
                .value("status", "revoked")
                .and_where(Expr::col("account_id").eq(app.fixture.account.id))
                .to_owned(),
            1 => Query::update()
                .table("household_memberships")
                .value("status", "revoked")
                .value("revoked_at", chrono::Utc::now().fixed_offset())
                .and_where(
                    Expr::col("id").eq(member_named(&app, "supported-fixture", "support-owner")
                        .await
                        .id),
                )
                .to_owned(),
            2 => Query::update()
                .table("users")
                .value("active", false)
                .and_where(Expr::col("id").eq(app.fixture.user.id))
                .to_owned(),
            _ => Query::update()
                .table("households")
                .value("status", "closed")
                .value("lifecycle_state", "closed")
                .and_where(Expr::col("id").eq(household_named(&app, "supported-fixture").await.id))
                .to_owned(),
        };
        app.fixture
            .admin
            .execute_raw(app.fixture.admin.get_database_backend().build(&query))
            .await
            .unwrap();
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

        assert!(
            !status.is_success(),
            "support read must stop after: {change}"
        );
        finish(&app, None, None).await;
        app.close().await;
    }
}

#[tokio::test]
async fn platform_support_reason_is_not_leaked_in_denied_responses() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("platform_admins")
                .value("status", Expr::val("revoked"))
                .value(
                    "updated_at",
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                )
                .and_where(Expr::col("account_id").eq(Expr::val(app.fixture.account.id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
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

    assert!(!str::contains(
        &denied_body,
        "Confidential support reason alpha"
    ));
    assert!(!str::contains(
        &anonymous_body,
        "Confidential support reason alpha"
    ));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_support_audit_failure_rolls_back_request() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    supported_household(&app).await;
    let cookie = admin_session(&app).await;
    let (cookies, operation_id, confirm_token) = request_support(
        &app,
        &cookie,
        household_named(&app, "supported-fixture").await.id,
        "Confidential support reason alpha",
    )
    .await;
    // SeaQuery has no PostgreSQL GRANT/REVOKE builder; the real restricted-role denial is required.
    app.fixture
        .admin
        .execute_unprepared("REVOKE INSERT ON public.versions FROM med_tracker_app")
        .await
        .unwrap();
    let (blocked, _) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    let absent = support_session(&app, household_named(&app, "supported-fixture").await.id)
        .await
        .is_none();
    // SeaQuery has no PostgreSQL GRANT/REVOKE builder; the real restricted-role denial is required.
    app.fixture
        .admin
        .execute_unprepared("GRANT INSERT ON public.versions TO med_tracker_app")
        .await
        .unwrap();
    let (retried, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    let present = support_session(&app, household_named(&app, "supported-fixture").await.id)
        .await
        .is_some();

    assert!(!blocked.is_success());
    assert!(absent, "audit failure must roll back the request row");
    assert!(
        retried.is_success(),
        "proof must survive an audit failure: {outcome}"
    );
    assert!(present);
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_support_request_deadline_is_enforced() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    supported_household(&app).await;
    let cookie = admin_session(&app).await;
    let owner = sign_in(
        &app,
        actor(&app, "support-owner").await.account.id,
        &actor(&app, "support-owner").await.account.email,
    )
    .await;
    let (cookies, operation_id, confirm_token) = request_support(
        &app,
        &cookie,
        household_named(&app, "supported-fixture").await.id,
        "Confidential support reason alpha",
    )
    .await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) =
        support_session(&app, household_named(&app, "supported-fixture").await.id)
            .await
            .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table("support_access_sessions")
                .value(
                    "expires_at",
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new()))
                        .sub(Expr::val("1 second").cast_as(Alias::new("interval"))),
                )
                .value(
                    "updated_at",
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                )
                .and_where(Expr::col("id").eq(support_id))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let approved = approve_support(&app, &owner, support_id).await;
    let (_, window, _, _) = support_window(&app, support_id).await.unwrap_or_default();

    assert!(
        !approved.is_success(),
        "expired requests must not be approvable"
    );
    assert!(!window);
    finish(&app, None, None).await;
    app.close().await;
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
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    Expr::from(
                        Func::cust("pg_backend_pid").args(Vec::<sea_orm::sea_query::Expr>::new()),
                    ),
                    "pid",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    lock.execute_raw({
        let query = Query::select()
            .expr(Func::cust("set_config").args([
                Expr::val("med_tracker.current_household_id"),
                Expr::val(household_id.to_string()),
                Expr::val(true),
            ]))
            .to_owned();
        app.fixture.admin.get_database_backend().build(&query)
    })
    .await
    .unwrap();
    lock.execute_raw({
        let query = Query::select()
            .expr(Expr::col("id"))
            .from(("public", "household_memberships"))
            .and_where(Expr::col("id").eq(Expr::val(membership_id)))
            .lock(sea_orm::sea_query::LockType::Update)
            .to_owned();
        app.fixture.admin.get_database_backend().build(&query)
    })
    .await
    .unwrap();
    (lock, holder)
}

async fn wait_blocked_on(app: &Application, holder: i32, support_id: i64) -> bool {
    for _ in 0..60 {
        let row = app
            .fixture
            .admin
            .query_one_raw({let query=Query::select().expr_as(Expr::exists(Query::select().expr(Expr::val(1)).from_as("pg_stat_activity","a").and_where(Expr::col(("a","datname")).eq(Expr::from(Func::cust("current_database").args(Vec::<sea_orm::sea_query::Expr>::new()))).and(Expr::val(holder).cast_as(Alias::new("int")).eq(Expr::from(sea_orm::sea_query::extension::postgres::PgFunc::any(Expr::from(Func::cust("pg_blocking_pids").args([Expr::col(("a","pid"))]))))))).to_owned()),"waiting").expr_as(Expr::from(Func::cust("clock_timestamp").args(Vec::<sea_orm::sea_query::Expr>::new())).lt(Expr::from(Query::select().expr(Expr::col("expires_at")).from(("public","support_access_sessions")).and_where(Expr::col("id").eq(Expr::val(support_id))).to_owned())),"before_deadline").to_owned();app.fixture.admin.get_database_backend().build(&query)})
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
            .query_one_raw({
                let query = Query::select()
                    .expr_as(
                        Expr::from(
                            Func::cust("clock_timestamp")
                                .args(Vec::<sea_orm::sea_query::Expr>::new()),
                        )
                        .gt(Expr::col("expires_at")),
                        "past",
                    )
                    .from(("public", "support_access_sessions"))
                    .and_where(Expr::col("id").eq(Expr::val(support_id)))
                    .to_owned();
                app.fixture.admin.get_database_backend().build(&query)
            })
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
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    Expr::col("approved_at").cast_as(Alias::new("text")),
                    "approved_at",
                )
                .expr_as(
                    Expr::col("activated_at").cast_as(Alias::new("text")),
                    "activated_at",
                )
                .from("support_access_sessions")
                .and_where(Expr::col("id").eq(Expr::val(support_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
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
        .query_one_raw({
            let query = Query::select()
                .expr_as(Expr::col(sea_orm::sea_query::Asterisk).count(), "count")
                .from("versions")
                .and_where(
                    Expr::col("event").eq(Expr::val(event)).and(
                        Expr::col("item_type")
                            .eq(Expr::val("SupportAccessSession"))
                            .and(Expr::col("item_id").eq(Expr::val(support_id))),
                    ),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .and_then(|row| row.try_get::<i64>("", "count").ok())
        .unwrap_or_default()
}

#[tokio::test]
async fn platform_owner_recovery_page_omits_the_administrator_membership() {
    let app = Application::new().await;
    recovery_household(&app).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("household_memberships")
                .columns([
                    "household_id",
                    "person_id",
                    "role",
                    "status",
                    "joined_at",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(household_named(&app, "recovery-fixture").await.id),
                    Expr::val(app.fixture.adult.id),
                    Expr::val("member"),
                    Expr::val("active"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let cookie = admin_fixture_session(&app).await;
    let (status, detail) = {
        let response = app
            .client
            .get(format!(
                "{}/platform/owner-recovery?household={identity_72601}",
                app.origin,
                identity_72601 = household_named(&app, "recovery-fixture").await.id
            ))
            .header("cookie", &cookie)
            .send()
            .await
            .unwrap();
        (response.status(), response.text().await.unwrap())
    };

    assert_eq!(status, reqwest::StatusCode::OK, "{detail}");
    assert!(
        str::contains(
            &detail,
            format!(
                "value=\"{identity_74601}\"",
                identity_74601 = member_named(&app, "recovery-fixture", "recovery-member")
                    .await
                    .id
            )
            .as_str()
        ),
        "{detail}"
    );
    assert!(
        !str::contains(
            &detail,
            format!(
                "value=\"{identity_74003}\"",
                identity_74003 = membership_link(
                    &app,
                    household_named(&app, "recovery-fixture").await.id,
                    app.fixture.adult.id
                )
                .await
                .id
            )
            .as_str()
        ),
        "{detail}"
    );
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_owner_recovery_excludes_accountless_participants() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    recovery_household(&app).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("people")
                .columns([
                    "name",
                    "person_type",
                    "has_capacity",
                    "created_at",
                    "updated_at",
                    "portable_id",
                ])
                .values_panic([
                    Expr::val("Accountless participant"),
                    Expr::val(0),
                    Expr::val(true),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::val(format!(
                        "{}-{}",
                        app.fixture.client_uuid, "Accountless participant"
                    )),
                ])
                .to_owned();
            app.fixture
                .admin
                .execute_raw(app.fixture.admin.get_database_backend().build(&query))
                .await
                .unwrap();
            let query = Query::insert()
                .into_table("household_memberships")
                .columns([
                    "household_id",
                    "person_id",
                    "role",
                    "status",
                    "joined_at",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(household_named(&app, "recovery-fixture").await.id),
                    Expr::val(participant_named(&app, "Accountless participant").await.id),
                    Expr::val("member"),
                    Expr::val("active"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let cookie = admin_session(&app).await;
    let (cookies, detail) = form_page(
        &app,
        &cookie,
        &format!(
            "/platform/owner-recovery?household={identity_72601}",
            identity_72601 = household_named(&app, "recovery-fixture").await.id
        ),
    )
    .await;
    assert!(
        str::contains(
            &detail,
            format!(
                "value=\"{identity_74601}\"",
                identity_74601 = member_named(&app, "recovery-fixture", "recovery-member")
                    .await
                    .id
            )
            .as_str()
        ),
        "{detail}"
    );
    assert!(
        !str::contains(
            &detail,
            format!(
                "value=\"{identity_74615}\"",
                identity_74615 =
                    accountless_member(&app, "recovery-fixture", "Accountless participant")
                        .await
                        .id
            )
            .as_str()
        ),
        "{detail}"
    );
    let token = hidden_field(&detail, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/owner-recovery",
        &[
            (
                "household_id",
                &(household_named(&app, "recovery-fixture").await.id).to_string(),
            ),
            (
                "membership_id",
                &(accountless_member(&app, "recovery-fixture", "Accountless participant")
                    .await
                    .id)
                    .to_string(),
            ),
            (
                "reason",
                "accountless participant cannot become an account owner",
            ),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let status = response.status();
    let role = membership_state(
        &app,
        accountless_member(&app, "recovery-fixture", "Accountless participant")
            .await
            .id,
    )
    .await
    .map(|row| row.0);

    assert!(status.is_client_error(), "{status}");
    assert_eq!(role, Some("member".to_owned()));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_owner_recovery_treats_accountless_owner_as_unusable() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    household(
        &app,
        "accountless-owner-fixture",
        "Accountless owner household",
    )
    .await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table("people")
                .columns([
                    "name",
                    "person_type",
                    "has_capacity",
                    "created_at",
                    "updated_at",
                    "portable_id",
                ])
                .values_panic([
                    Expr::val("Accountless owner"),
                    Expr::val(0),
                    Expr::val(true),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::val(format!(
                        "{}-{}",
                        app.fixture.client_uuid, "Accountless owner"
                    )),
                ])
                .to_owned();
            app.fixture
                .admin
                .execute_raw(app.fixture.admin.get_database_backend().build(&query))
                .await
                .unwrap();
            let query = Query::insert()
                .into_table("household_memberships")
                .columns([
                    "household_id",
                    "person_id",
                    "role",
                    "status",
                    "joined_at",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(household_named(&app, "accountless-owner-fixture").await.id),
                    Expr::val(participant_named(&app, "Accountless owner").await.id),
                    Expr::val("owner"),
                    Expr::val("active"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    household_member(
        &app,
        household_named(&app, "accountless-owner-fixture").await.id,
        "eligible-member",
        "Eligible member",
        "member",
    )
    .await;
    let cookie = admin_session(&app).await;
    let (cookies, detail) = form_page(
        &app,
        &cookie,
        &format!(
            "/platform/owner-recovery?household={identity_72603}",
            identity_72603 = household_named(&app, "accountless-owner-fixture").await.id
        ),
    )
    .await;
    assert!(str::contains(&detail, "Eligible member"), "{detail}");
    let token = hidden_field(&detail, "authenticity_token");
    let response = post_form(
        &app,
        &cookies,
        "/platform/owner-recovery",
        &[
            (
                "household_id",
                &(household_named(&app, "accountless-owner-fixture").await.id).to_string(),
            ),
            (
                "membership_id",
                &(member_named(&app, "accountless-owner-fixture", "eligible-member")
                    .await
                    .id)
                    .to_string(),
            ),
            ("reason", "owner record is unusable"),
            ("authenticity_token", &token),
        ],
    )
    .await;
    let (cookies, operation_id, confirm_token) = start_browser_proof(response, &cookie).await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    let promoted = membership_state(
        &app,
        member_named(&app, "accountless-owner-fixture", "eligible-member")
            .await
            .id,
    )
    .await
    .map(|row| row.0);

    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    assert_eq!(promoted, Some("owner".to_owned()));
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_support_read_waiting_on_row_lock_denies_after_deadline() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table(("public", "support_access_sessions"))
                .value(
                    "expires_at",
                    Expr::from(
                        Func::cust("clock_timestamp").args(Vec::<sea_orm::sea_query::Expr>::new()),
                    )
                    .add(Expr::val("3 seconds").cast_as(Alias::new("interval"))),
                )
                .and_where(Expr::col("id").eq(Expr::val(support_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let lock = sea_orm::Database::connect(&app.fixture.runtime_uri)
        .await
        .unwrap()
        .begin()
        .await
        .unwrap();
    let holder: i32 = lock
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    Expr::from(
                        Func::cust("pg_backend_pid").args(Vec::<sea_orm::sea_query::Expr>::new()),
                    ),
                    "pid",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    lock.execute_raw({
        let query = Query::select()
            .expr(Expr::col("id"))
            .from(("public", "support_access_sessions"))
            .and_where(Expr::col("id").eq(Expr::val(support_id)))
            .lock(sea_orm::sea_query::LockType::Update)
            .to_owned();
        app.fixture.admin.get_database_backend().build(&query)
    })
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
            .query_one_raw({let query=Query::select().expr_as(Expr::exists(Query::select().expr(Expr::val(1)).from_as("pg_stat_activity","a").and_where(Expr::col(("a","datname")).eq(Expr::from(Func::cust("current_database").args(Vec::<sea_orm::sea_query::Expr>::new()))).and(Expr::val(holder).cast_as(Alias::new("int")).eq(Expr::from(sea_orm::sea_query::extension::postgres::PgFunc::any(Expr::from(Func::cust("pg_blocking_pids").args([Expr::col(("a","pid"))]))))))).to_owned()),"waiting").expr_as(Expr::from(Func::cust("clock_timestamp").args(Vec::<sea_orm::sea_query::Expr>::new())).lt(Expr::from(Query::select().expr(Expr::col("expires_at")).from(("public","support_access_sessions")).and_where(Expr::col("id").eq(Expr::val(support_id))).to_owned())),"before_deadline").to_owned();app.fixture.admin.get_database_backend().build(&query)})
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
            .query_one_raw({
                let query = Query::select()
                    .expr_as(
                        Expr::from(
                            Func::cust("clock_timestamp")
                                .args(Vec::<sea_orm::sea_query::Expr>::new()),
                        )
                        .gt(Expr::col("expires_at")),
                        "past",
                    )
                    .from(("public", "support_access_sessions"))
                    .and_where(Expr::col("id").eq(Expr::val(support_id)))
                    .to_owned();
                app.fixture.admin.get_database_backend().build(&query)
            })
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
    let (_, _, _, expired) =
        support_session(&app, household_named(&app, "supported-fixture").await.id)
            .await
            .unwrap();
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

    assert!(status.is_client_error(), "{body}");
    assert!(!str::contains(&body, "Supported tablets"), "{body}");
    assert!(expired.is_some(), "the expiry must be persisted");
    assert_eq!(audits, 1, "exactly one expiry audit must be recorded");
    assert!(second.is_client_error());
    assert_eq!(audits_after, 1, "a repeat read must not audit expiry again");
    finish(&app, None, None).await;
    app.close().await;
}

async fn expiry_source_counts(
    app: &Application,
    id: i64,
) -> Result<(i64, i64, i64), sea_orm::DbErr> {
    let row = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    Expr::from(
                        Query::select()
                            .expr(Expr::col(sea_orm::sea_query::Asterisk).count())
                            .from(("public", "versions"))
                            .and_where(
                                Expr::col("item_type")
                                    .eq(Expr::val("SupportAccessSession"))
                                    .and(
                                        Expr::col("event")
                                            .eq(Expr::val("platform/support/expired"))
                                            .and(Expr::col("item_id").eq(Expr::val(id))),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "versions",
                )
                .expr_as(
                    Expr::from(
                        Query::select()
                            .expr(Expr::col(sea_orm::sea_query::Asterisk).count())
                            .from(("public", "security_audit_events"))
                            .and_where(
                                Expr::col("event_type")
                                    .eq(Expr::val("support_access_session.expired"))
                                    .and(
                                        Expr::from(Func::cust("jsonb_extract_path_text").args([
                                            Expr::col("metadata"),
                                            Expr::val("support_access_session_id"),
                                        ]))
                                        .eq(Expr::val(id).cast_as(Alias::new("text"))),
                                    ),
                            )
                            .to_owned(),
                    ),
                    "security",
                )
                .expr_as(
                    Expr::from(
                        Query::select()
                            .expr(Expr::col(sea_orm::sea_query::Asterisk).count())
                            .from(("public", "audit_ledger_entries"))
                            .and_where(
                                Expr::col("source_table")
                                    .eq(Expr::val("versions"))
                                    .and(
                                        Expr::from(Func::cust("jsonb_extract_path_text").args([
                                            Expr::col("source_payload"),
                                            Expr::val("item_type"),
                                        ]))
                                        .eq(Expr::val("SupportAccessSession"))
                                        .and(
                                            Expr::from(Func::cust("jsonb_extract_path_text").args(
                                                [Expr::col("source_payload"), Expr::val("event")],
                                            ))
                                            .eq(Expr::val("platform/support/expired"))
                                            .and(
                                                Expr::from(
                                                    Func::cust("jsonb_extract_path_text").args([
                                                        Expr::col("source_payload"),
                                                        Expr::val("item_id"),
                                                    ]),
                                                )
                                                .eq(Expr::val(id).cast_as(Alias::new("text"))),
                                            ),
                                        ),
                                    )
                                    .or(Expr::col("source_table")
                                        .eq(Expr::val("security_audit_events"))
                                        .and(
                                            Expr::from(Func::cust("jsonb_extract_path_text").args(
                                                [
                                                    Expr::col("source_payload"),
                                                    Expr::val("event_type"),
                                                ],
                                            ))
                                            .eq(Expr::val("support_access_session.expired"))
                                            .and(
                                                Expr::from(
                                                    Func::cust("jsonb_extract_path_text").args([
                                                        Expr::col("source_payload"),
                                                        Expr::val("metadata"),
                                                        Expr::val("support_access_session_id"),
                                                    ]),
                                                )
                                                .eq(Expr::val(id).cast_as(Alias::new("text"))),
                                            ),
                                        )),
                            )
                            .to_owned(),
                    ),
                    "ledger",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await?
        .ok_or_else(|| sea_orm::DbErr::Custom("Synthetic expiry counts missing".into()))?;
    Ok((
        row.try_get("", "versions")?,
        row.try_get("", "security")?,
        row.try_get("", "ledger")?,
    ))
}

async fn reject_expiry_audit(app: &Application) -> Result<(), sea_orm::DbErr> {
    let name = format!("expiry_audit_{}", app.fixture.client_uuid.simple());
    let owner = app.fixture.account.id;
    // SeaQuery has no PostgreSQL trigger/function/sequence DDL builder; only this owned actor's expiry is rejected.
    app.fixture.admin.execute_unprepared(&format!("CREATE SEQUENCE public.{name}_reached; ALTER SEQUENCE public.{name}_reached OWNER TO med_tracker_owner; CREATE FUNCTION public.{name}() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN IF NEW.item_type='SupportAccessSession' AND NEW.event='platform/support/expired' AND NEW.item_id IN (SELECT id FROM public.support_access_sessions WHERE platform_admin_id IN (SELECT id FROM public.platform_admins WHERE account_id={owner})) THEN PERFORM pg_catalog.nextval('public.{name}_reached'::regclass); RAISE EXCEPTION USING ERRCODE='23514',MESSAGE='Synthetic expiry audit rejection'; END IF; RETURN NEW; END $$; ALTER FUNCTION public.{name}() OWNER TO med_tracker_owner; REVOKE ALL ON FUNCTION public.{name}() FROM PUBLIC; CREATE TRIGGER {name} BEFORE INSERT ON public.versions FOR EACH ROW EXECUTE FUNCTION public.{name}()")).await?;
    Ok(())
}
async fn drop_expiry_audit(app: &Application) -> Result<(), sea_orm::DbErr> {
    let name = format!("expiry_audit_{}", app.fixture.client_uuid.simple());
    // SeaQuery has no PostgreSQL trigger/function/sequence DDL builder.
    app.fixture.admin.execute_unprepared(&format!("DROP TRIGGER {name} ON public.versions; DROP FUNCTION public.{name}(); DROP SEQUENCE public.{name}_reached")).await?;
    Ok(())
}

async fn expiry_rejection_reached(app: &Application) -> Result<bool, sea_orm::DbErr> {
    app.fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr(Expr::col("is_called"))
                .from((
                    Alias::new("public"),
                    Alias::new(format!(
                        "expiry_audit_{}_reached",
                        app.fixture.client_uuid.simple()
                    )),
                ))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await?
        .ok_or_else(|| sea_orm::DbErr::Custom("Synthetic expiry marker missing".into()))?
        .try_get("", "is_called")
}

#[tokio::test]
async fn platform_support_expiry_audit_failure_rolls_back() -> Result<(), Box<dyn std::error::Error>>
{
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table(("public", "support_access_sessions"))
                .value(
                    "expires_at",
                    Expr::from(
                        Func::cust("clock_timestamp").args(Vec::<sea_orm::sea_query::Expr>::new()),
                    )
                    .sub(Expr::val("1 second").cast_as(Alias::new("interval"))),
                )
                .and_where(Expr::col("id").eq(Expr::val(support_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    reject_expiry_audit(&app).await?;
    let failed = app
        .client
        .get(format!("{}/platform/support/{support_id}", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap()
        .status();
    let reached = expiry_rejection_reached(&app).await?;
    let failed_counts = expiry_source_counts(&app, support_id).await?;
    drop_expiry_audit(&app).await?;
    let (_, _, _, expired) =
        support_session(&app, household_named(&app, "supported-fixture").await.id)
            .await
            .unwrap();
    let audits = support_audit_count(&app, "platform/support/expired", support_id).await;
    let retry = app
        .client
        .get(format!("{}/platform/support/{support_id}", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap()
        .status();
    let (_, _, _, expired_after) =
        support_session(&app, household_named(&app, "supported-fixture").await.id)
            .await
            .unwrap();
    let audits_after = support_audit_count(&app, "platform/support/expired", support_id).await;
    let retried_counts = expiry_source_counts(&app, support_id).await?;

    assert!(!failed.is_success());
    assert!(
        expired.is_none(),
        "a failed expiry audit must roll back expired_at"
    );
    assert_eq!(audits, 0);
    assert!(!retry.is_success());
    assert!(expired_after.is_some());
    assert_eq!(audits_after, 1);
    assert!(reached);
    assert_eq!(failed_counts, (0, 0, 0));
    assert_eq!(retried_counts, (1, 1, 2));
    finish(&app, None, None).await;
    app.close().await;
    Ok(())
}

#[tokio::test]
async fn platform_support_read_includes_allocation_and_schedule_details() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let cookie = admin_fixture_session(&app).await;
    let support_id = active_fixture_support(&app, &cookie).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table(("public", "person_medications"))
                .columns([
                    "household_id",
                    "person_id",
                    "medication_id",
                    "dose_amount",
                    "dose_unit",
                    "max_daily_doses",
                    "position",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(household_named(&app, "supported-fixture").await.id),
                    Expr::val(actor(&app, "supported-member").await.person.id),
                    Expr::val(supported_medication(&app).await.id),
                    Expr::val(1.5),
                    Expr::val("caplet"),
                    Expr::val(2),
                    Expr::val(1),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_raw({
            let query = Query::insert()
                .into_table(("public", "schedules"))
                .columns([
                    "household_id",
                    "person_id",
                    "medication_id",
                    "active",
                    "schedule_type",
                    "schedule_config",
                    "frequency",
                    "dose_amount",
                    "dose_unit",
                    "created_at",
                    "updated_at",
                ])
                .values_panic([
                    Expr::val(household_named(&app, "supported-fixture").await.id),
                    Expr::val(actor(&app, "supported-member").await.person.id),
                    Expr::val(supported_medication(&app).await.id),
                    Expr::val(true),
                    Expr::val(0),
                    Expr::val(json!({"times":["08:00","20:00"]})),
                    Expr::val("every_6_hours"),
                    Expr::val(1.5),
                    Expr::val("caplet"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let response = app
        .client
        .get(format!("{}/platform/support/{support_id}", app.origin))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    assert!(str::contains(&body, "Supported member"), "{body}");
    assert!(str::contains(&body, "Supported tablets"), "{body}");
    assert!(str::contains(&body, "1.5 caplet"), "{body}");
    assert!(!str::contains(&body, "1.50"), "{body}");
    assert!(str::contains(&body, "Every 6 hours"), "{body}");
    assert!(!str::contains(&body, "every_6_hours"), "{body}");
    assert!(
        !str::contains(&body, "Confidential support reason"),
        "protected reasons must never render: {body}"
    );
    finish(&app, None, None).await;
    app.close().await;
}

fn raw_timestamp_present(body: &str) -> bool {
    body.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != ':' && ch != '.')
        .any(|token| str::contains(token, ':') && str::contains(token, '.'))
}

#[tokio::test]
async fn platform_support_owner_page_shows_requester_reason_and_scope() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    supported_household(&app).await;
    let cookie = admin_fixture_session(&app).await;
    let owner = fixture_session(&app, actor(&app, "support-owner").await.account.id).await;
    let (cookies, operation_id, confirm_token) = request_support(
        &app,
        &cookie,
        household_named(&app, "supported-fixture").await.id,
        "Confidential support reason alpha",
    )
    .await;
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

    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    assert!(
        str::contains(&body, app.fixture.account.email.as_str()),
        "owners must see the requesting administrator identity: {body}"
    );
    assert!(
        str::contains(&body, "Confidential support reason alpha"),
        "owners must see the request reason before approving: {body}"
    );
    assert!(
        str::contains(&body, "read-only"),
        "owners must see the read-only scope: {body}"
    );
    assert!(
        str::contains(&body, "24 hours") && str::contains(&body, "30 minutes"),
        "owners must see the request and access durations: {body}"
    );
    assert!(
        str::contains(&body, "cannot write")
            && str::contains(&body, "export")
            && str::contains(&body, "download"),
        "owners must see the no-write, no-export and no-download boundaries: {body}"
    );
    assert!(
        str::contains(&body, "end access") || str::contains(&body, "End access"),
        "owners must see that they can end access: {body}"
    );
    assert!(
        !raw_timestamp_present(&body),
        "deadlines must render as readable UTC labels: {body}"
    );
    assert!(str::contains(&body, " UTC"), "{body}");
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_support_pages_render_readable_utc_deadlines() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let cookie = admin_fixture_session(&app).await;
    let support_id = active_fixture_support(&app, &cookie).await;
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

    assert!(
        str::contains(&list_body, " UTC"),
        "session list deadlines must carry an explicit UTC label: {list_body}"
    );
    assert!(
        !raw_timestamp_present(&list_body),
        "session list must not render raw timestamps: {list_body}"
    );
    assert!(
        str::contains(&read_body, " UTC"),
        "the read page expiry must carry an explicit UTC label: {read_body}"
    );
    assert!(
        !raw_timestamp_present(&read_body),
        "the read page must not render raw timestamps: {read_body}"
    );
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_owner_recovery_reason_has_a_visible_label() {
    let app = Application::new().await;
    let cookie = admin_fixture_session(&app).await;
    recovery_household(&app).await;
    let (_, body) = form_page(
        &app,
        &cookie,
        &format!(
            "/platform/owner-recovery?household={identity_72601}",
            identity_72601 = household_named(&app, "recovery-fixture").await.id
        ),
    )
    .await;

    assert!(str::contains(&body, "for=\"reason-"), "{body}");
    assert!(
        !str::contains(&body, "class=\"sr-only\" for=\"reason-"),
        "the recovery reason label must be visible for touch and pointer users: {body}"
    );
    finish(&app, None, None).await;
    app.close().await;
}

async fn table_lock(app: &Application) -> (sea_orm::DatabaseTransaction, i32) {
    let lock = sea_orm::Database::connect(&app.fixture.runtime_uri)
        .await
        .unwrap()
        .begin()
        .await
        .unwrap();
    let holder: i32 = lock
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    Expr::from(
                        Func::cust("pg_backend_pid").args(Vec::<sea_orm::sea_query::Expr>::new()),
                    ),
                    "pid",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    // SeaQuery row-lock builders cannot express a table-level ACCESS EXCLUSIVE lock.
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
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    let (lock, holder) = table_lock(&app).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table(("public", "support_access_sessions"))
                .value(
                    "expires_at",
                    Expr::from(
                        Func::cust("clock_timestamp").args(Vec::<sea_orm::sea_query::Expr>::new()),
                    )
                    .add(Expr::val("3 seconds").cast_as(Alias::new("interval"))),
                )
                .and_where(Expr::col("id").eq(Expr::val(support_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
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
    let (_, _, _, expired) =
        support_session(&app, household_named(&app, "supported-fixture").await.id)
            .await
            .unwrap();
    let audits = support_audit_count(&app, "platform/support/expired", support_id).await;
    assert!(
        status.is_client_error(),
        "an expired read must be denied, got {status}: {body}"
    );
    assert!(!str::contains(&body, "Supported tablets"), "{body}");
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

    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_support_approval_waiting_on_owner_lock_denies_after_deadline() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    supported_household(&app).await;
    let owner = sign_in(
        &app,
        actor(&app, "support-owner").await.account.id,
        &actor(&app, "support-owner").await.account.email,
    )
    .await;
    let (cookies, operation_id, confirm_token) = request_support(
        &app,
        &cookie,
        household_named(&app, "supported-fixture").await.id,
        "Confidential support reason alpha",
    )
    .await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) =
        support_session(&app, household_named(&app, "supported-fixture").await.id)
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
    let (lock, holder) = membership_lock(
        &app,
        household_named(&app, "supported-fixture").await.id,
        member_named(&app, "supported-fixture", "support-owner")
            .await
            .id,
    )
    .await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table(("public", "support_access_sessions"))
                .value(
                    "expires_at",
                    Expr::from(
                        Func::cust("clock_timestamp").args(Vec::<sea_orm::sea_query::Expr>::new()),
                    )
                    .add(Expr::val("3 seconds").cast_as(Alias::new("interval"))),
                )
                .and_where(Expr::col("id").eq(Expr::val(support_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
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

    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_support_activation_waiting_on_authority_query_denies_after_deadline() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    supported_household(&app).await;
    let owner = sign_in(
        &app,
        actor(&app, "support-owner").await.account.id,
        &actor(&app, "support-owner").await.account.email,
    )
    .await;
    let (cookies, operation_id, confirm_token) = request_support(
        &app,
        &cookie,
        household_named(&app, "supported-fixture").await.id,
        "Confidential support reason alpha",
    )
    .await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) =
        support_session(&app, household_named(&app, "supported-fixture").await.id)
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
        .execute_raw({
            let query = Query::update()
                .table(("public", "support_access_sessions"))
                .value(
                    "expires_at",
                    Expr::from(
                        Func::cust("clock_timestamp").args(Vec::<sea_orm::sea_query::Expr>::new()),
                    )
                    .add(Expr::val("3 seconds").cast_as(Alias::new("interval"))),
                )
                .and_where(Expr::col("id").eq(Expr::val(support_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
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

    finish(&app, None, None).await;
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
    let admin = admin_fixture_session(&app).await;
    let owner = fixture_session(&app, actor(&app, "support-owner").await.account.id).await;
    let member = fixture_session(&app, actor(&app, "supported-member").await.account.id).await;
    let (status, body) = security_page(&app, &admin).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    assert!(
        str::contains(&body, "/platform/users"),
        "platform administrators need the platform administration link: {body}"
    );
    assert!(
        !str::contains(&body, "/account/support"),
        "non-owner administrators must not see the owner support link: {body}"
    );
    let (status, body) = security_page(&app, &owner).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    assert!(
        str::contains(&body, "/account/support"),
        "household owners need the support access link: {body}"
    );
    assert!(
        !str::contains(&body, "/platform/users"),
        "ordinary owners must not see platform administration: {body}"
    );
    let (status, body) = security_page(&app, &member).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    assert!(
        !str::contains(&body, "/platform/users") && !str::contains(&body, "/account/support"),
        "ordinary members must see neither link: {body}"
    );

    finish(&app, None, None).await;
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
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    Expr::from(
                        Func::cust("pg_backend_pid").args(Vec::<sea_orm::sea_query::Expr>::new()),
                    ),
                    "pid",
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    lock.execute_raw({
        let query = Query::select()
            .expr(Expr::col("id"))
            .from(("public", "support_access_sessions"))
            .and_where(Expr::col("id").eq(Expr::val(support_id)))
            .lock(sea_orm::sea_query::LockType::Update)
            .to_owned();
        app.fixture.admin.get_database_backend().build(&query)
    })
    .await
    .unwrap();
    (lock, holder)
}

#[tokio::test]
async fn platform_support_read_rechecks_admin_authority_after_authority_wait() {
    crate::authentication_fixture::reset_source_quota().await;
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
        .execute_raw({
            let query = Query::update()
                .table("platform_admins")
                .value("status", Expr::val("revoked"))
                .value(
                    "updated_at",
                    Expr::from(
                        Func::cust("clock_timestamp").args(Vec::<sea_orm::sea_query::Expr>::new()),
                    ),
                )
                .and_where(Expr::col("account_id").eq(Expr::val(app.fixture.account.id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    lock.commit().await.unwrap();
    let response = pending.await.unwrap();
    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    assert!(waited, "the read must wait on the owner authority query");
    assert!(
        status.is_client_error(),
        "a read after administrator withdrawal must be denied, got {status}: {body}"
    );
    assert!(!str::contains(&body, "Supported tablets"), "{body}");
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_support_approval_rechecks_owner_lockout_after_row_wait() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    supported_household(&app).await;
    let owner = sign_in(
        &app,
        actor(&app, "support-owner").await.account.id,
        &actor(&app, "support-owner").await.account.email,
    )
    .await;
    let (cookies, operation_id, confirm_token) = request_support(
        &app,
        &cookie,
        household_named(&app, "supported-fixture").await.id,
        "Confidential support reason alpha",
    )
    .await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) =
        support_session(&app, household_named(&app, "supported-fixture").await.id)
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
        .execute_raw({
            let query = Query::insert()
                .into_table(("public", "account_lockouts"))
                .columns(["account_id", "deadline", "key", "created_at", "updated_at"])
                .values_panic([
                    Expr::val(actor(&app, "support-owner").await.account.id),
                    Expr::from(
                        Func::cust("clock_timestamp").args(Vec::<sea_orm::sea_query::Expr>::new()),
                    )
                    .add(Expr::val("1 hour").cast_as(Alias::new("interval"))),
                    Expr::val("support-approval-lockout"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .on_conflict(
                    sea_orm::sea_query::OnConflict::column("account_id")
                        .update_column("deadline")
                        .to_owned(),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    lock.commit().await.unwrap();
    let (status, outcome) = pending.await.unwrap();
    let (approved_at, _) = support_decision(&app, support_id).await;
    let audits = support_audit_count(&app, "platform/support/approved", support_id).await;

    assert!(waited, "the approval must wait on the session row lock");
    assert!(
        status.is_client_error(),
        "a locked-out owner must not approve, got {status}: {outcome}"
    );
    assert!(approved_at.is_none(), "{approved_at:?}");
    assert_eq!(audits, 0);
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_support_activation_rechecks_admin_after_row_wait() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    supported_household(&app).await;
    let owner = sign_in(
        &app,
        actor(&app, "support-owner").await.account.id,
        &actor(&app, "support-owner").await.account.email,
    )
    .await;
    let (cookies, operation_id, confirm_token) = request_support(
        &app,
        &cookie,
        household_named(&app, "supported-fixture").await.id,
        "Confidential support reason alpha",
    )
    .await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) =
        support_session(&app, household_named(&app, "supported-fixture").await.id)
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
        .execute_raw({
            let query = Query::update()
                .table("platform_admins")
                .value("status", Expr::val("revoked"))
                .value(
                    "updated_at",
                    Expr::from(
                        Func::cust("clock_timestamp").args(Vec::<sea_orm::sea_query::Expr>::new()),
                    ),
                )
                .and_where(Expr::col("account_id").eq(Expr::val(app.fixture.account.id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    lock.commit().await.unwrap();
    let (status, outcome) = pending.await.unwrap();
    let (_, activated_at) = support_decision(&app, support_id).await;
    let audits = support_audit_count(&app, "platform/support/activated", support_id).await;

    assert!(waited, "the activation must wait on the session row lock");
    assert!(
        status.is_client_error(),
        "a withdrawn administrator must not activate, got {status}: {outcome}"
    );
    assert!(activated_at.is_none(), "{activated_at:?}");
    assert_eq!(audits, 0);
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_support_activation_rechecks_household_after_row_wait() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    supported_household(&app).await;
    let owner = sign_in(
        &app,
        actor(&app, "support-owner").await.account.id,
        &actor(&app, "support-owner").await.account.email,
    )
    .await;
    let (cookies, operation_id, confirm_token) = request_support(
        &app,
        &cookie,
        household_named(&app, "supported-fixture").await.id,
        "Confidential support reason alpha",
    )
    .await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) =
        support_session(&app, household_named(&app, "supported-fixture").await.id)
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
        .execute_raw({
            let query = Query::update()
                .table("households")
                .value("status", Expr::val("suspended"))
                .value(
                    "updated_at",
                    Expr::from(
                        Func::cust("clock_timestamp").args(Vec::<sea_orm::sea_query::Expr>::new()),
                    ),
                )
                .and_where(Expr::col("id").eq(Expr::val(
                    household_named(&app, "supported-fixture").await.id,
                )))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    lock.commit().await.unwrap();
    let (status, outcome) = pending.await.unwrap();
    let (_, activated_at) = support_decision(&app, support_id).await;
    let audits = support_audit_count(&app, "platform/support/activated", support_id).await;

    assert!(waited, "the activation must wait on the session row lock");
    assert!(
        status.is_client_error(),
        "a suspended household must not activate, got {status}: {outcome}"
    );
    assert!(activated_at.is_none(), "{activated_at:?}");
    assert_eq!(audits, 0);
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_support_end_rechecks_actor_after_row_wait() {
    crate::authentication_fixture::reset_source_quota().await;
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
        .execute_raw({
            let query = Query::insert()
                .into_table(("public", "account_lockouts"))
                .columns(["account_id", "deadline", "key", "created_at", "updated_at"])
                .values_panic([
                    Expr::val(app.fixture.account.id),
                    Expr::from(
                        Func::cust("clock_timestamp").args(Vec::<sea_orm::sea_query::Expr>::new()),
                    )
                    .add(Expr::val("1 hour").cast_as(Alias::new("interval"))),
                    Expr::val("support-end-lockout"),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                    Expr::from(Func::cust("now").args(Vec::<sea_orm::sea_query::Expr>::new())),
                ])
                .on_conflict(
                    sea_orm::sea_query::OnConflict::column("account_id")
                        .update_column("deadline")
                        .to_owned(),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
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
    let ended = support_session(&app, household_named(&app, "supported-fixture").await.id)
        .await
        .unwrap()
        .2;
    let audits = support_audit_count(&app, "platform/support/ended", support_id).await;

    assert!(waited, "the end request must wait on the session row lock");
    assert_eq!(status, reqwest::StatusCode::SEE_OTHER);
    assert_eq!(location.as_deref(), Some("/login"));
    assert!(ended.is_none(), "ended_at must stay unset: {ended:?}");
    assert_eq!(audits, 0);
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_support_expired_request_denies_approval_and_records_expiry() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    supported_household(&app).await;
    let owner = sign_in(
        &app,
        actor(&app, "support-owner").await.account.id,
        &actor(&app, "support-owner").await.account.email,
    )
    .await;
    let (cookies, operation_id, confirm_token) = request_support(
        &app,
        &cookie,
        household_named(&app, "supported-fixture").await.id,
        "Confidential support reason alpha",
    )
    .await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) =
        support_session(&app, household_named(&app, "supported-fixture").await.id)
            .await
            .expect("the request must persist");
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table(("public", "support_access_sessions"))
                .value(
                    "expires_at",
                    Expr::from(
                        Func::cust("clock_timestamp").args(Vec::<sea_orm::sea_query::Expr>::new()),
                    )
                    .sub(Expr::val("1 second").cast_as(Alias::new("interval"))),
                )
                .and_where(Expr::col("id").eq(Expr::val(support_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let denied = approve_support(&app, &owner, support_id).await;
    let (approved_at, _) = support_decision(&app, support_id).await;
    let (_, _, _, expired) =
        support_session(&app, household_named(&app, "supported-fixture").await.id)
            .await
            .unwrap();
    let audits = support_audit_count(&app, "platform/support/expired", support_id).await;
    let again = approve_support(&app, &owner, support_id).await;
    let audits_after = support_audit_count(&app, "platform/support/expired", support_id).await;

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
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_support_pending_expiry_audit_failure_rolls_back()
-> Result<(), Box<dyn std::error::Error>> {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    supported_household(&app).await;
    let owner = sign_in(
        &app,
        actor(&app, "support-owner").await.account.id,
        &actor(&app, "support-owner").await.account.email,
    )
    .await;
    let (cookies, operation_id, confirm_token) = request_support(
        &app,
        &cookie,
        household_named(&app, "supported-fixture").await.id,
        "Confidential support reason alpha",
    )
    .await;
    let (confirmed, outcome) = browser_confirm(&app, &cookies, &operation_id, &confirm_token).await;
    assert_eq!(confirmed, reqwest::StatusCode::OK, "{outcome}");
    let (support_id, ..) =
        support_session(&app, household_named(&app, "supported-fixture").await.id)
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
        .execute_raw({
            let query = Query::update()
                .table(("public", "support_access_sessions"))
                .value(
                    "expires_at",
                    Expr::from(
                        Func::cust("clock_timestamp").args(Vec::<sea_orm::sea_query::Expr>::new()),
                    )
                    .sub(Expr::val("1 second").cast_as(Alias::new("interval"))),
                )
                .and_where(Expr::col("id").eq(Expr::val(support_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    reject_expiry_audit(&app).await?;
    let (denied, denied_body) =
        browser_confirm(&app, &owner_cookies, &approval_id, &approval_token).await;
    let reached = expiry_rejection_reached(&app).await?;
    let failed_counts = expiry_source_counts(&app, support_id).await?;
    drop_expiry_audit(&app).await?;
    let (_, _, _, expired) =
        support_session(&app, household_named(&app, "supported-fixture").await.id)
            .await
            .unwrap();
    let audits = support_audit_count(&app, "platform/support/expired", support_id).await;
    let (retried, retry_body) =
        browser_confirm(&app, &owner_cookies, &approval_id, &approval_token).await;
    let (_, _, _, expired_after) =
        support_session(&app, household_named(&app, "supported-fixture").await.id)
            .await
            .unwrap();
    let audits_after = support_audit_count(&app, "platform/support/expired", support_id).await;
    let retried_counts = expiry_source_counts(&app, support_id).await?;

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
    assert!(reached);
    assert_eq!(failed_counts, (0, 0, 0));
    assert_eq!(retried_counts, (1, 1, 2));
    finish(&app, None, None).await;
    app.close().await;
    Ok(())
}

#[tokio::test]
async fn platform_support_end_on_expired_session_records_expiry_once() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table(("public", "support_access_sessions"))
                .value(
                    "expires_at",
                    Expr::from(
                        Func::cust("clock_timestamp").args(Vec::<sea_orm::sea_query::Expr>::new()),
                    )
                    .sub(Expr::val("1 second").cast_as(Alias::new("interval"))),
                )
                .and_where(Expr::col("id").eq(Expr::val(support_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
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
    let (_, _, ended, expired) =
        support_session(&app, household_named(&app, "supported-fixture").await.id)
            .await
            .unwrap();
    let audits = support_audit_count(&app, "platform/support/expired", support_id).await;

    assert!(
        final_status.is_client_error(),
        "ending an expired session must deny deliberately: {page}"
    );
    assert!(expired.is_some(), "the end path must persist expiry");
    assert!(ended.is_none(), "an expired session must not gain ended_at");
    assert_eq!(audits, 1, "exactly one expiry audit must be recorded");
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_owner_recovery_search_does_not_place_csrf_tokens_in_urls() {
    let app = Application::new().await;
    let cookie = admin_fixture_session(&app).await;
    recovery_household(&app).await;
    let (_, body) = form_page(&app, &cookie, "/platform/owner-recovery").await;
    let (_, detail) = form_page(
        &app,
        &cookie,
        &format!(
            "/platform/owner-recovery?household={identity_72601}",
            identity_72601 = household_named(&app, "recovery-fixture").await.id
        ),
    )
    .await;

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
        !str::contains(search, "authenticity_token"),
        "the GET search must not leak CSRF tokens into the URL: {search}"
    );
    assert!(
        str::contains(&detail, "name=\"authenticity_token\""),
        "POST promotion forms keep CSRF protection: {detail}"
    );
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_settings_get_renders_defaults_without_creating_the_singleton() {
    let app = Application::new().await;
    let settings_before = isolate_settings(&app).await;
    let cookie = admin_fixture_session(&app).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::delete()
                .from_table(("public", "app_settings"))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let (_, body) = form_page(&app, &cookie, "/platform/settings").await;
    let persisted = app_settings(&app).await;

    assert!(
        str::contains(&body, "invite_only"),
        "GET settings must render defaults for a missing row: {body}"
    );
    assert!(
        persisted.is_none(),
        "a settings read must not write the singleton row"
    );
    finish(&app, None, Some(&settings_before)).await;
    app.close().await;
}

#[tokio::test]
async fn platform_audit_does_not_trust_spoofed_forwarded_for() {
    crate::authentication_fixture::reset_source_quota().await;
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
        .query_one_raw({
            let query = Query::select()
                .expr_as(
                    Expr::from(
                        Func::cust("jsonb_extract_path_text")
                            .args([Expr::col("audit_context"), Expr::val("ip")]),
                    ),
                    "ip",
                )
                .from("versions")
                .and_where(
                    Expr::col("event")
                        .eq(Expr::val("platform/support/ended"))
                        .and(Expr::col("item_id").eq(Expr::val(support_id))),
                )
                .order_by("id", sea_orm::Order::Desc)
                .limit(1)
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap()
        .and_then(|row| row.try_get::<Option<String>>("", "ip").unwrap());

    assert!(
        ip.is_some(),
        "an audited origin must be recorded for the end event"
    );
    assert_ne!(
        ip.as_deref(),
        Some("198.51.100.77"),
        "spoofed client forwarding headers must never reach audit metadata"
    );
    finish(&app, None, None).await;
    app.close().await;
}

#[tokio::test]
async fn platform_support_listing_expiry_audit_is_race_safe()
-> Result<(), Box<dyn std::error::Error>> {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let cookie = admin_session(&app).await;
    let support_id = active_support(&app, &cookie).await;
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table(("public", "support_access_sessions"))
                .value(
                    "expires_at",
                    Expr::from(
                        Func::cust("clock_timestamp").args(Vec::<sea_orm::sea_query::Expr>::new()),
                    )
                    .sub(Expr::val("1 second").cast_as(Alias::new("interval"))),
                )
                .and_where(Expr::col("id").eq(Expr::val(support_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let (lock, _) = session_lock(&app, support_id).await;
    let listing = || {
        app.client
            .get(format!("{}/platform/support", app.origin))
            .header("cookie", &cookie)
            .timeout(std::time::Duration::from_secs(5))
            .send()
    };
    let (first, second) = tokio::join!(listing(), listing());
    let first = first?.status();
    let second = second?.status();
    let busy_expired = support_session(&app, household_named(&app, "supported-fixture").await.id)
        .await
        .ok_or_else(|| std::io::Error::other("Synthetic support row missing"))?
        .3;
    let busy_audits = support_audit_count(&app, "platform/support/expired", support_id).await;
    let busy_security = expiry_source_counts(&app, support_id).await?;
    lock.commit().await?;
    let (retry_first, retry_second) = tokio::join!(listing(), listing());
    let retry_first = retry_first?.status();
    let retry_second = retry_second?.status();
    let audits = support_audit_count(&app, "platform/support/expired", support_id).await;
    let expired = support_session(&app, household_named(&app, "supported-fixture").await.id)
        .await
        .unwrap()
        .3;
    let mut tasks = loco_rs::task::Tasks::default();
    <med_tracker::app::App as loco_rs::app::Hooks>::register_tasks(&mut tasks);
    tasks
        .run(
            &app.context,
            "support-access-expire",
            &loco_rs::task::Vars::default(),
        )
        .await?;
    let after_worker = support_audit_count(&app, "platform/support/expired", support_id).await;
    let security = app
        .fixture
        .admin
        .query_one_raw({
            let query = Query::select()
                .expr_as(Expr::col(sea_orm::sea_query::Asterisk).count(), "count")
                .expr_as(
                    Expr::from(
                        Func::cust("bool_and").args([Expr::col("actor_account_id")
                            .eq(Expr::val(app.fixture.account.id))
                            .and(
                                Expr::col("household_id")
                                    .eq(Expr::val(
                                        household_named(&app, "supported-fixture").await.id,
                                    ))
                                    .and(
                                        Expr::from(Func::cust("jsonb_extract_path_text").args([
                                            Expr::col("audit_context"),
                                            Expr::val("active_role"),
                                        ]))
                                        .eq(Expr::val("support_expiry_task"))
                                        .and(
                                            Expr::from(Func::cust("jsonb_extract_path_text").args(
                                                [
                                                    Expr::col("audit_context"),
                                                    Expr::val("authentication_method"),
                                                ],
                                            ))
                                            .eq(Expr::val("system")),
                                        ),
                                    ),
                            )]),
                    ),
                    "attributed",
                )
                .from(("public", "security_audit_events"))
                .and_where(
                    Expr::col("event_type")
                        .eq(Expr::val("support_access_session.expired"))
                        .and(
                            Expr::from(Func::cust("jsonb_extract_path_text").args([
                                Expr::col("metadata"),
                                Expr::val("support_access_session_id"),
                            ]))
                            .eq(Expr::val(support_id).cast_as(Alias::new("text"))),
                        ),
                )
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await?
        .ok_or_else(|| std::io::Error::other("Synthetic expiry audit row missing"))?;
    let security_count = security.try_get::<i64>("", "count")?;
    let attributed = security.try_get::<Option<bool>>("", "attributed")?;

    assert_eq!(
        (first, second),
        (reqwest::StatusCode::OK, reqwest::StatusCode::OK)
    );
    assert!(
        busy_expired.is_none(),
        "A locked source must remain retryable"
    );
    assert_eq!(busy_audits, 0);
    assert_eq!(busy_security, (0, 0, 0));
    assert_eq!(
        (retry_first, retry_second),
        (reqwest::StatusCode::OK, reqwest::StatusCode::OK)
    );
    assert!(expired.is_some());
    assert_eq!(
        audits, 1,
        "concurrent listings must persist one expiry audit"
    );
    assert_eq!(
        after_worker, 1,
        "Worker retry must not duplicate listing expiry"
    );
    assert_eq!(
        security_count, 1,
        "Listing-first expiry must retain the security audit after worker retry"
    );
    assert_eq!(attributed, Some(true));
    finish(&app, None, None).await;
    app.close().await;
    Ok(())
}

#[tokio::test]
async fn platform_support_listing_does_not_expire_an_ended_session() {
    crate::authentication_fixture::reset_source_quota().await;
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
    assert!(
        support_session(&app, household_named(&app, "supported-fixture").await.id)
            .await
            .unwrap()
            .2
            .is_some()
    );
    app.fixture
        .admin
        .execute_raw({
            let query = Query::update()
                .table(("public", "support_access_sessions"))
                .value(
                    "expires_at",
                    Expr::from(
                        Func::cust("clock_timestamp").args(Vec::<sea_orm::sea_query::Expr>::new()),
                    )
                    .sub(Expr::val("1 second").cast_as(Alias::new("interval"))),
                )
                .and_where(Expr::col("id").eq(Expr::val(support_id)))
                .to_owned();
            app.fixture.admin.get_database_backend().build(&query)
        })
        .await
        .unwrap();
    let (_, body) = form_page(&app, &cookie, "/platform/support").await;
    let expired = support_session(&app, household_named(&app, "supported-fixture").await.id)
        .await
        .unwrap()
        .3;
    let audits = support_audit_count(&app, "platform/support/expired", support_id).await;

    assert!(str::contains(&body, "ended"));
    assert!(
        expired.is_none(),
        "ending before expiry must remain the final state"
    );
    assert_eq!(audits, 0);
    finish(&app, None, None).await;
    app.close().await;
}

async fn isolate_administrators(app: &Application) -> Vec<platform_admins::Model> {
    let before = platform_admins::Entity::find()
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let query = Query::update()
        .table("platform_admins")
        .value("status", "disabled")
        .to_owned();
    app.fixture
        .admin
        .execute_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap();
    before
}
async fn isolate_settings(app: &Application) -> Vec<app_setting_rows::Model> {
    let before = app_setting_rows::Entity::find()
        .all(&app.fixture.admin)
        .await
        .unwrap();
    app_setting_rows::Entity::delete_many()
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    before
}
async fn finish(
    app: &Application,
    admins: Option<&[platform_admins::Model]>,
    settings: Option<&[app_setting_rows::Model]>,
) {
    let ids = Query::select()
        .column("id")
        .from("accounts")
        .and_where(
            Expr::col("id")
                .eq(app.fixture.account.id)
                .or(Expr::col("email").like(format!("%-{}@example.test", app.fixture.client_uuid))),
        )
        .to_owned();
    let query = Query::update()
        .table("platform_admins")
        .value("status", "disabled")
        .and_where(Expr::col("account_id").in_subquery(ids))
        .to_owned();
    app.fixture
        .admin
        .execute_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap();
    if let Some(rows) = admins {
        for row in rows {
            {
                let mut restored = row.clone().into_active_model();
                restored.status = Set(row.status.clone());
                restored.updated_at = Set(row.updated_at);
                restored
            }
            .update(&app.fixture.admin)
            .await
            .unwrap();
        }
    }
    if let Some(rows) = settings {
        app_setting_rows::Entity::delete_many()
            .exec(&app.fixture.admin)
            .await
            .unwrap();
        for row in rows {
            row.clone()
                .into_active_model()
                .insert(&app.fixture.admin)
                .await
                .unwrap();
        }
    }
}
