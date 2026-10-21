use std::sync::Arc;

use async_trait::async_trait;
use better_auth::{AuthResult, plugins::email_verification::SendVerificationEmail};
use better_auth_core::{FieldValue, wire::UserView};
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
    let email = user
        .email
        .typed()?
        .as_ref()
        .ok_or_else(|| better_auth::AuthError::internal("Account email unavailable"))?;
    notice_address(store, email, subject, body).await
}

pub(super) async fn notice_address(
    store: &ClinicalStore,
    address: &str,
    subject: &str,
    body: &str,
) -> AuthResult<()> {
    let transaction = store.transaction().await?;
    let email = Email {
        to: address.into(),
        subject: subject.into(),
        text: body.into(),
        ..Default::default()
    };
    transaction.execute_raw(statement("INSERT INTO pg_loco_queue(id,task_data,name,run_at,priority) VALUES($1,$2,$3,clock_timestamp(),$4)", [ulid::Ulid::generate().to_string().into(), serde_json::to_value(email)?.into(), MailerWorker::class_name().into(), DEFAULT_MAILER_PRIORITY.into()])).await.map_err(database_error)?;
    transaction.commit().await.map_err(database_error)
}

#[async_trait]
impl SendVerificationEmail for VerificationMailer {
    async fn send(&self, user: &FieldValue, url: &str, token: &str) -> AuthResult<()> {
        use better_auth_core::store::VerificationStore;
        let user_id: String = user.model_property("id")?.decode()?;
        let email: String = user.model_property("email")?.decode()?;
        self.0
            .create_verification(better_auth_core::CreateVerification {
                identifier: super::confirmation::identifier(token).into(),
                value: user_id.into(),
                expires_at: (chrono::Utc::now() + chrono::Duration::hours(24)).into(),
                ..Default::default()
            })
            .await?;
        let mut confirmation = url::Url::parse(url)
            .map_err(|_| better_auth::AuthError::internal("Verification URL unavailable"))?;
        confirmation.set_path("/verify-account-confirm");
        notice_address(
            &self.0,
            &email,
            "Verify your account",
            &format!("Verify your account: {confirmation}"),
        )
        .await
    }
}
