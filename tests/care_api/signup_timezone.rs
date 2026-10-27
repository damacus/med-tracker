use super::*;
use med_tracker::models::_entities::accounts;

#[tokio::test]
async fn account_creation_initialises_timezone_for_password_and_passkey_signup() {
    crate::authentication_fixture::reset_source_quota().await;
    let app = Application::new().await;
    let registration = super::signup::open_registration(&app).await;
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
        let email = format!("{}-{email}", uuid::Uuid::new_v4());
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
        let row = accounts::Entity::find()
            .filter(accounts::Column::Email.eq(&email))
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.name.as_deref(), Some(name));
        let stored = row.preferences["time_zone"].as_str().unwrap();
        let expected = std::env::var("TZ").unwrap_or_else(|_| "UTC".into());
        assert_eq!(stored.trim_start_matches("Etc/"), expected, "{email}");
    }
    super::signup::restore_registration(&app, registration).await;
    app.close().await;
}
