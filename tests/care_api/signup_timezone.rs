use super::*;

#[tokio::test]
async fn account_creation_initialises_timezone_for_password_and_passkey_signup() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("DELETE FROM app_settings; INSERT INTO app_settings(invite_only,created_at,updated_at) VALUES(false,now(),now())").await.unwrap();
    for (email, name, credential) in [
        (
            "password-owner@example.test",
            "New household owner",
            "password",
        ),
        (
            "passkey-owner@example.test",
            "Passkey household owner",
            "passkey",
        ),
    ] {
        let mut input =
            json!({"email":email,"name":name,"date_of_birth":"1990-01-01","credential":credential});
        if credential == "password" {
            input["password"] = json!("Synthetic-password-123!");
        }
        let response = app
            .client
            .post(format!("{}/api/auth/onboarding/signup", app.origin))
            .header("origin", app.context.config.server.full_url())
            .json(&input)
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body = response.text().await.unwrap();
        assert_eq!(status.as_u16(), 200, "{body}");
        let row = app
            .fixture
            .admin
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT name,preferences->>'time_zone' AS zone FROM accounts WHERE email=$1",
                [email.into()],
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            row.try_get::<Option<String>>("", "name")
                .unwrap()
                .as_deref(),
            Some(name)
        );
        let stored = row.try_get::<String>("", "zone").unwrap();
        let expected = std::env::var("TZ").unwrap_or_else(|_| "UTC".into());
        assert_eq!(stored.trim_start_matches("Etc/"), expected, "{email}");
    }
    app.close().await;
}
