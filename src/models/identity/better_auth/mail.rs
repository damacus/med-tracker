use std::sync::Arc;

use async_trait::async_trait;
use better_auth::{AuthResult, plugins::email_verification::SendVerificationEmail};
use better_auth_core::wire::UserView;
use loco_rs::{
    bgworker::BackgroundWorker,
    mailer::{DEFAULT_MAILER_PRIORITY, Email, MailerWorker},
};
use sea_orm::ConnectionTrait;

use super::{
    ClinicalStore,
    store::{database_error, statement},
};

pub(super) struct VerificationMailer(pub Arc<ClinicalStore>);

pub(super) async fn notice(
    store: &ClinicalStore,
    user: &UserView,
    subject: &str,
    body: &str,
) -> AuthResult<()> {
    let transaction = store.transaction().await?;
    let email = Email {
        to: user
            .email
            .clone()
            .ok_or_else(|| better_auth::AuthError::internal("Account email unavailable"))?,
        subject: subject.into(),
        text: body.into(),
        ..Default::default()
    };
    transaction.execute_raw(statement("INSERT INTO pg_loco_queue(id,task_data,name,run_at,priority) VALUES($1,$2,$3,clock_timestamp(),$4)", [ulid::Ulid::new().to_string().into(), serde_json::to_value(email)?.into(), MailerWorker::class_name().into(), DEFAULT_MAILER_PRIORITY.into()])).await.map_err(database_error)?;
    transaction.commit().await.map_err(database_error)
}

#[async_trait]
impl SendVerificationEmail for VerificationMailer {
    async fn send(&self, user: &UserView, url: &str, token: &str) -> AuthResult<()> {
        use better_auth_core::store::VerificationStore;
        self.0
            .create_verification(better_auth_core::CreateVerification {
                identifier: super::confirmation::identifier(token),
                value: user.id.clone(),
                expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
            })
            .await?;
        let mut confirmation = url::Url::parse(url)
            .map_err(|_| better_auth::AuthError::internal("Verification URL unavailable"))?;
        confirmation.set_path("/verify-account-confirm");
        let transaction = self.0.transaction().await?;
        let email = Email {
            to: user
                .email
                .clone()
                .ok_or_else(|| better_auth::AuthError::validation("Email is required"))?,
            subject: "Verify your account".into(),
            text: format!("Verify your account: {confirmation}"),
            ..Default::default()
        };
        transaction.execute_raw(statement(
            "INSERT INTO pg_loco_queue(id,task_data,name,run_at,priority) VALUES($1,$2,$3,clock_timestamp(),$4)",
            [ulid::Ulid::new().to_string().into(), serde_json::to_value(email)?.into(), MailerWorker::class_name().into(), DEFAULT_MAILER_PRIORITY.into()],
        )).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)
    }
}
