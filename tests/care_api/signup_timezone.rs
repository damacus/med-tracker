use super::*;
use med_tracker::models::identity::signup::{self, AccountProfile, SignupInput};
use sea_orm::TransactionTrait;

#[tokio::test]
async fn account_creation_initialises_timezone_for_password_and_passkey_signup() {
    let fixture = Fixture::new().await;
    fixture.admin.execute_unprepared("DELETE FROM app_settings; INSERT INTO app_settings(invite_only,created_at,updated_at) VALUES(false,now(),now())").await.unwrap();
    signup::create(
        &fixture.runtime,
        &SignupInput {
            invitation_token: String::new(),
            email: "password-owner@example.test".into(),
            name: "New household owner".into(),
            date_of_birth: "1990-01-01".into(),
            password: "Synthetic-password-123!".into(),
            password_confirm: "Synthetic-password-123!".into(),
        },
        "https://medtracker.example.test",
        Some("password-signup-timezone"),
    )
    .await
    .unwrap();
    let transaction = fixture.runtime.begin().await.unwrap();
    signup::provision_account_in(
        &transaction,
        &AccountProfile {
            email: "passkey-owner@example.test".into(),
            name: "Passkey household owner".into(),
            date_of_birth: chrono::NaiveDate::from_ymd_opt(1990, 1, 1).unwrap(),
            invitation_token: None,
        },
        "passkey-signup-timezone",
    )
    .await
    .unwrap();
    transaction.commit().await.unwrap();
    for email in ["password-owner@example.test", "passkey-owner@example.test"] {
        let row = fixture
            .admin
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT preferences->>'time_zone' AS zone FROM accounts WHERE email=$1",
                [email.into()],
            ))
            .await
            .unwrap()
            .unwrap();
        let stored = row.try_get::<String>("", "zone").unwrap();
        let expected = std::env::var("TZ").unwrap_or_else(|_| "UTC".into());
        assert_eq!(stored.trim_start_matches("Etc/"), expected, "{email}");
    }
    fixture.close().await;
}
