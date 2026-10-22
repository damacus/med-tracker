use std::sync::Arc;

use better_auth::plugins::{
    email_verification::EmailVerificationCallbacks,
    password_management::PasswordManagementCallbacks,
};
use better_auth_core::{AuthError, AuthResult, FieldValue, background::BackgroundFuture};
use loco_rs::{
    bgworker::BackgroundWorker,
    mailer::{DEFAULT_MAILER_PRIORITY, Email, MailerWorker},
};
use sea_orm::{ConnectionTrait, DatabaseConnection};

use super::{
    policy::{database_error, statement},
    schema::AccountSchema,
};

async fn enqueue(db: &impl ConnectionTrait, payload: serde_json::Value) -> AuthResult<()> {
    db.execute_raw(statement("INSERT INTO pg_loco_queue(id,task_data,name,run_at,priority) VALUES($1,$2,$3,clock_timestamp(),$4)", [ulid::Ulid::generate().to_string().into(), payload.into(), MailerWorker::class_name().into(), DEFAULT_MAILER_PRIORITY.into()])).await.map_err(database_error)?;
    Ok(())
}

pub(super) const SIGNUP_DELIVERY: &str = "medtracker.signup-delivery";

pub(super) enum SignupDelivery {
    Created,
    Queued,
    Failed,
}

fn signup_sender(
    request: &better_auth_core::AuthRequest,
) -> AuthResult<Option<std::sync::mpsc::SyncSender<SignupDelivery>>> {
    let Some(original) = request.original_request() else {
        return Ok(None);
    };
    let Some(FieldValue::Function(binding)) = original.server_context(SIGNUP_DELIVERY)? else {
        return Ok(None);
    };
    binding
        .binding::<std::sync::mpsc::SyncSender<SignupDelivery>>()
        .cloned()
        .map(Some)
        .ok_or_else(|| AuthError::internal("Registration delivery context unavailable"))
}

pub(super) fn signup_created(request: &better_auth_core::AuthRequest) -> AuthResult<()> {
    if let Some(sender) = signup_sender(request)? {
        sender
            .try_send(SignupDelivery::Created)
            .map_err(|_| AuthError::internal("Registration delivery unavailable"))?;
    }
    Ok(())
}

pub(super) fn verification(db: DatabaseConnection) -> EmailVerificationCallbacks<AccountSchema> {
    let db = Arc::new(db);
    EmailVerificationCallbacks::send(move |message, endpoint| {
        let user = message.user_view()?;
        let email = Email {
            to: user
                .email
                .typed()?
                .clone()
                .ok_or_else(|| AuthError::internal("Account email unavailable"))?,
            subject: if endpoint
                .input_request()
                .is_some_and(|request| request.path() == "/change-email")
            {
                "Verify your new email address"
            } else {
                "Verify your account"
            }
            .into(),
            text: format!("Verify your account: {}", confirmation_url(&message.url)?),
            ..Default::default()
        };
        let payload = serde_json::to_value(email)?;
        if endpoint.transaction.is_some() {
            let request = endpoint
                .input_request()
                .ok_or_else(|| AuthError::internal("Signup request unavailable"))?;
            let Some(FieldValue::Function(binding)) =
                request.server_context(super::native_hooks::TRANSACTION_BINDING)?
            else {
                return Err(AuthError::internal("Signup transaction unavailable"));
            };
            let tx = binding
                .binding::<better_auth_seaorm::TransactionConnection>()
                .ok_or_else(|| AuthError::internal("Signup transaction unavailable"))?
                .clone();
            let sender = signup_sender(request)?;
            Ok(Some(BackgroundFuture::new(async move {
                let result = enqueue(&tx, payload).await;
                if let Some(sender) = sender {
                    sender
                        .try_send(if result.is_ok() {
                            SignupDelivery::Queued
                        } else {
                            SignupDelivery::Failed
                        })
                        .map_err(|_| AuthError::internal("Registration delivery unavailable"))?;
                }
                result
            })))
        } else {
            let db = db.clone();
            Ok(Some(BackgroundFuture::new(async move {
                enqueue(db.as_ref(), payload).await
            })))
        }
    })
}

pub(super) fn password(db: DatabaseConnection) -> PasswordManagementCallbacks<AccountSchema> {
    let db = Arc::new(db);
    PasswordManagementCallbacks::send_reset_password(move |message, _| {
        let user = &message.user;
        let email = Email {
            to: user
                .email
                .typed()?
                .clone()
                .ok_or_else(|| AuthError::internal("Account email unavailable"))?,
            subject: "Reset your password".into(),
            text: format!("Reset your password: {}", message.url),
            ..Default::default()
        };
        let payload = serde_json::to_value(email)?;
        let db = db.clone();
        Ok(Some(BackgroundFuture::new(async move {
            enqueue(db.as_ref(), payload).await
        })))
    })
}

pub(super) async fn send_passwordless_registration(
    context: &better_auth_core::AuthContext<AccountSchema>,
    user: &better_auth_core::wire::UserView,
    transaction: &better_auth_seaorm::TransactionConnection,
) -> AuthResult<()> {
    let (sender, receiver) = std::sync::mpsc::sync_channel::<Email>(1);
    let mut local = context.clone();
    local
        .extensions
        .insert(Arc::new(EmailVerificationCallbacks::<AccountSchema>::send(
            move |message, _| {
                let user = message.user_view()?;
                let email = Email {
                    to: user
                        .email
                        .typed()?
                        .clone()
                        .ok_or_else(|| AuthError::internal("Account email unavailable"))?,
                    subject: "Verify your account".into(),
                    text: format!("Verify your account: {}", confirmation_url(&message.url)?),
                    ..Default::default()
                };
                sender.try_send(email).map_err(|_| {
                    AuthError::internal("Registration verification delivery unavailable")
                })?;
                Ok(None)
            },
        )));
    better_auth::plugins::EmailVerificationPlugin::new()
        .send_on_sign_in(true)
        .send_verification_on_sign_in(user, Some("/auth/passkey/setup"), &local)
        .await?;
    let email = receiver
        .try_recv()
        .map_err(|_| AuthError::internal("Registration verification was not generated"))?;
    enqueue(transaction, serde_json::to_value(email)?).await
}

fn confirmation_url(generated: &str) -> AuthResult<url::Url> {
    let mut link = url::Url::parse(generated)
        .map_err(|_| AuthError::internal("Verification URL unavailable"))?;
    link.set_path("/verify-account-confirm");
    Ok(link)
}
