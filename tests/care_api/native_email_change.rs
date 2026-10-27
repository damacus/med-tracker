use super::*;
use med_tracker::models::_entities::{accounts, users};
use sea_orm::sea_query::{Expr, ExprTrait, Query};

fn verify_url(app: &Application, token: &str) -> url::Url {
    let mut url = url::Url::parse(&format!("{}/api/auth/verify-email", app.origin)).unwrap();
    url.query_pairs_mut().append_pair("token", token);
    url
}
async fn state(app: &Application) -> Value {
    let account = accounts::Entity::find_by_id(app.fixture.account.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let user = users::Entity::find_by_id(app.fixture.user.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let query = Query::select()
        .expr_as(Expr::col("id").count(), "sessions")
        .from("identity_sessions")
        .and_where(Expr::col("account_id").eq(account.id))
        .to_owned();
    let sessions: i64 = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "sessions")
        .unwrap();
    let audit = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("NativeAuthentication"))
        .filter(versions::Column::ItemId.eq(account.id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    let notices = super::native_password_reset::queued_mail(
        app,
        &app.fixture.account.email,
        "Email address changed",
    )
    .await
    .len();
    json!({"email":account.email,"profile":user.email_address,"sessions":sessions,"audit":audit,"notices":notices})
}
async fn issued_change(app: &Application, cookie: &str, email: &str) -> String {
    let response = app
        .client
        .post(format!("{}/api/auth/change-email", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("cookie", cookie)
        .json(&json!({"newEmail":email}))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    let messages =
        super::native_password_reset::queued_mail(app, email, "Verify your new email address")
            .await;
    assert_eq!(messages.len(), 1);
    let mail = &messages[0];
    let link = url::Url::parse(
        mail["text"]
            .as_str()
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap(),
    )
    .unwrap();
    link.query_pairs()
        .find(|(name, _)| name == "token")
        .unwrap()
        .1
        .into_owned()
}
#[tokio::test]
async fn native_email_change_uses_stock_confirmation_and_rejects_a_different_signed_in_account() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let cookie =
        super::platform::sign_in(&app, app.fixture.account.id, &app.fixture.account.email).await;
    let other = app.fixture.foreign_household().await;
    let foreign = super::platform::sign_in(&app, other.account.id, &other.account.email).await;
    let email = format!("changed-native-{}@example.test", uuid::Uuid::new_v4());
    let token = issued_change(&app, &cookie, &email).await;
    let before = state(&app).await;
    let wrong = app
        .client
        .get(verify_url(&app, &token))
        .header("cookie", foreign)
        .send()
        .await
        .unwrap();
    assert!(wrong.status().is_client_error());
    assert_eq!(state(&app).await, before);
    let confirmed = app
        .client
        .get(verify_url(&app, &token))
        .send()
        .await
        .unwrap();
    assert_eq!(confirmed.status().as_u16(), 200);
    assert!(
        confirmed
            .headers()
            .get_all("set-cookie")
            .iter()
            .any(|value| value.to_str().unwrap().contains("session_token=")
                && !value.to_str().unwrap().contains("Max-Age=0"))
    );
    let after = state(&app).await;
    assert_eq!(after["email"], email);
    assert_eq!(after["profile"], email);
    assert_eq!(
        after["sessions"].as_i64().unwrap(),
        before["sessions"].as_i64().unwrap() + 1
    );
    assert_eq!(after["notices"], 1);
    let replay = app
        .client
        .get(verify_url(&app, &token))
        .send()
        .await
        .unwrap();
    assert!(replay.status().is_client_error());
    assert_eq!(state(&app).await, after);
    other.close().await;
    app.close().await;
}
#[tokio::test]
async fn native_email_change_notice_failure_rolls_back_account_profile_and_subject_audit() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let cookie =
        super::platform::sign_in(&app, app.fixture.account.id, &app.fixture.account.email).await;
    let email = format!("rollback-native-{}@example.test", uuid::Uuid::new_v4());
    let token = issued_change(&app, &cookie, &email).await;
    let before = state(&app).await;
    let fault = format!(
        "reject_native_email_notice_{}",
        uuid::Uuid::new_v4().simple()
    );
    // SeaORM has no trigger or PL/pgSQL function builder. This fault injection
    // requires whole-run exclusivity; both schema objects are removed below.
    app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION {fault}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic native email notice failure'; END $$; CREATE TRIGGER {fault} BEFORE INSERT ON pg_loco_queue FOR EACH ROW WHEN(NEW.task_data->>'subject'='Email address changed') EXECUTE FUNCTION {fault}()")).await.unwrap();
    let rejected = app
        .client
        .get(verify_url(&app, &token))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    let rejected_status = rejected.status();
    let rejected_state = state(&app).await;
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "DROP TRIGGER {fault} ON pg_loco_queue; DROP FUNCTION {fault}()"
        ))
        .await
        .unwrap();
    assert!(rejected_status.is_server_error());
    assert_eq!(rejected_state, before);
    let retry = app
        .client
        .get(verify_url(&app, &token))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    let status = retry.status();
    let text = retry.text().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{text}");
    let after = state(&app).await;
    assert_eq!(after["email"], email);
    assert_eq!(after["profile"], email);
    assert_eq!(after["notices"], 1);
    app.close().await;
}
