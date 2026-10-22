use super::*;

#[tokio::test]
async fn native_password_reset_enrols_numeric_account_without_existing_password_and_rejects_replay()
{
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("DELETE FROM auth_credentials WHERE user_id=71001; INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET completed_at=now()").await.unwrap();
    let sent = app
        .client
        .post(format!("{}/api/auth/request-password-reset", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"email":"persistence@example.test","redirectTo":"/reset-password/confirm"}))
        .send()
        .await
        .unwrap();
    let status = sent.status();
    let body = sent.text().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    let mail:Value=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT task_data FROM pg_loco_queue WHERE task_data->>'subject'='Reset your password' AND task_data->>'to'='persistence@example.test'")).await.unwrap().unwrap().try_get("","task_data").unwrap();
    let link = url::Url::parse(
        mail["text"]
            .as_str()
            .unwrap()
            .split_whitespace()
            .last()
            .unwrap(),
    )
    .unwrap();
    let token = link
        .path_segments()
        .unwrap()
        .next_back()
        .unwrap()
        .to_owned();
    let response = app
        .client
        .post(format!("{}/api/auth/reset-password", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"token":token,"newPassword":"New-native-reset-passphrase-42"}))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    let credential=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS credentials FROM auth_credentials WHERE user_id=71001 AND provider_id='credential' AND password IS NOT NULL")).await.unwrap().unwrap().try_get::<i64>("","credentials").unwrap();
    assert_eq!(credential, 1);
    let replay = app
        .client
        .post(format!("{}/api/auth/reset-password", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"token":token,"newPassword":"Replayed-native-reset-passphrase-42"}))
        .send()
        .await
        .unwrap();
    assert!(replay.status().is_client_error());
    let login=app.client.post(format!("{}/api/auth/sign-in/email",app.origin)).header("origin",app.context.config.server.full_url()).json(&json!({"email":"PERSISTENCE@EXAMPLE.TEST","password":"New-native-reset-passphrase-42"})).send().await.unwrap();
    let status = login.status();
    let body = login.text().await.unwrap();
    assert_eq!(status.as_u16(), 200, "{body}");
    app.close().await;
}
