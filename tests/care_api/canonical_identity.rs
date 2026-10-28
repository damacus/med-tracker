use super::*;
use med_tracker::models::_entities::{accounts, people, platform_admins};
use sea_orm::sea_query::{Expr, ExprTrait, Query};

#[tokio::test]
async fn self_identity_reports_the_canonical_account() {
    let app = Application::new().await;
    let token = app.token().await;
    let response = app
        .client
        .get(format!(
            "{}/api/v1/households/{}/me",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let body: Value = response.json().await.unwrap();
    let account_id = app.fixture.account.id;
    let person_id = app.fixture.adult.id;
    app.close().await;
    assert_eq!(body["data"]["id"], account_id);
    assert_eq!(body["data"]["person"]["id"], person_id);
}

#[tokio::test]
async fn canonical_account_suspension_blocks_existing_and_new_authentication() {
    let app = Application::new().await;
    let cookie = super::platform::admin_session(&app).await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{}/me",
        app.origin, app.fixture.household.id
    );
    assert_eq!(
        app.client
            .get(&endpoint)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::OK
    );
    accounts::Entity::update_many()
        .col_expr(accounts::Column::Active, Expr::value(false))
        .filter(accounts::Column::Id.eq(app.fixture.account.id))
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let bearer_status = app
        .client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status();
    let login_status = app
        .client
        .post(format!("{}/api/auth/sign-in/email", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"email":app.fixture.account.email,"password":"password"}))
        .send()
        .await
        .unwrap()
        .status();
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let browser_status = client
        .get(format!("{}/platform/users", app.origin))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap()
        .status();
    app.close().await;
    assert_eq!(bearer_status, reqwest::StatusCode::UNAUTHORIZED);
    assert_eq!(login_status, reqwest::StatusCode::UNAUTHORIZED);
    assert!(!browser_status.is_success() && !browser_status.is_server_error());
}

#[tokio::test]
async fn platform_account_works_without_a_second_profile_record() {
    let app = Application::new().await;
    let now = chrono::Utc::now();
    let person = people::ActiveModel {
        name: Set("Canonical support actor".into()),
        date_of_birth: Set(chrono::NaiveDate::from_ymd_opt(1990, 1, 1)),
        person_type: Set(0),
        has_capacity: Set(true),
        created_at: Set(now.naive_utc()),
        updated_at: Set(now.naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let account = accounts::ActiveModel {
        person_id: Set(person.id),
        name: Set(Some(person.name)),
        email: Set(format!(
            "canonical-{}@example.test",
            app.fixture.client_uuid
        )),
        status: Set(2),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    platform_admins::ActiveModel {
        account_id: Set(account.id),
        status: Set("active".into()),
        created_at: Set(now.naive_utc()),
        updated_at: Set(now.naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    crate::fixtures::seed_native_password(&app.fixture.admin, account.id).await;
    let cookie = super::platform::sign_in(&app, account.id, &account.email).await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let status = client
        .get(format!("{}/platform/users", app.origin))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap()
        .status();
    app.close().await;
    assert_eq!(status, reqwest::StatusCode::OK);
}

#[tokio::test]
async fn fresh_schema_has_only_account_and_person_identity() {
    let app = Application::new().await;
    let query = Query::select()
        .expr_as(Expr::col("table_name").count(), "count")
        .from(("information_schema", "tables"))
        .and_where(Expr::col("table_schema").eq("public"))
        .and_where(Expr::col("table_name").eq("users"))
        .to_owned();
    let row = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap();
    let count = row.try_get::<i64>("", "count").unwrap();
    app.close().await;
    assert_eq!(
        count, 0,
        "Fresh installations must not create a separate users/profile table"
    );
}
