use super::*;

fn verify_url(app: &Application, token: &str) -> url::Url {
    let mut url = url::Url::parse(&format!("{}/api/auth/verify-email", app.origin)).unwrap();
    url.query_pairs_mut().append_pair("token", token);
    url
}
async fn state(app: &Application) -> Value {
    app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT jsonb_build_object('email',(SELECT email FROM accounts WHERE id=71001),'profile',(SELECT email_address FROM users WHERE person_id=73001),'sessions',(SELECT count(*) FROM identity_sessions),'audit',(SELECT count(*) FROM versions WHERE item_type='NativeAuthentication'),'notices',(SELECT count(*) FROM pg_loco_queue WHERE task_data->>'subject'='Email address changed')) AS state")).await.unwrap().unwrap().try_get("","state").unwrap()
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
    let mail:Value=app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT task_data FROM pg_loco_queue WHERE task_data->>'to'=$1 AND task_data->>'subject'='Verify your new email address'",[email.into()])).await.unwrap().unwrap().try_get("","task_data").unwrap();
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
    let app = Application::new().await;
    let cookie = super::platform::sign_in(&app, 71001, "persistence@example.test").await;
    app.fixture.seed_foreign_household(72002).await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE people SET date_of_birth='1985-02-03' WHERE id=73999")
        .await
        .unwrap();
    let foreign = super::platform::sign_in(&app, 71999, "other-care-household@example.test").await;
    let token = issued_change(&app, &cookie, "changed-native@example.test").await;
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
    assert_eq!(after["email"], "changed-native@example.test");
    assert_eq!(after["profile"], "changed-native@example.test");
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
    app.close().await;
}
#[tokio::test]
async fn native_email_change_notice_failure_rolls_back_account_profile_and_subject_audit() {
    let app = Application::new().await;
    let cookie = super::platform::sign_in(&app, 71001, "persistence@example.test").await;
    let token = issued_change(&app, &cookie, "rollback-native@example.test").await;
    let before = state(&app).await;
    app.fixture.admin.execute_unprepared("CREATE FUNCTION reject_native_email_notice() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic native email notice failure'; END $$; CREATE TRIGGER reject_native_email_notice BEFORE INSERT ON pg_loco_queue FOR EACH ROW WHEN(NEW.task_data->>'subject'='Email address changed') EXECUTE FUNCTION reject_native_email_notice()").await.unwrap();
    let rejected = app
        .client
        .get(verify_url(&app, &token))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert!(rejected.status().is_server_error());
    assert_eq!(state(&app).await, before);
    app.fixture.admin.execute_unprepared("DROP TRIGGER reject_native_email_notice ON pg_loco_queue; DROP FUNCTION reject_native_email_notice()").await.unwrap();
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
    assert_eq!(after["email"], "rollback-native@example.test");
    assert_eq!(after["profile"], "rollback-native@example.test");
    assert_eq!(after["notices"], 1);
    app.close().await;
}
