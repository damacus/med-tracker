use super::*;
use loco_rs::{
    bgworker::BackgroundWorker,
    mailer::{DEFAULT_MAILER_PRIORITY, Email},
};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde::{Deserialize, Serialize};

mod worker;
pub use worker::InvitationMailWorker;

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AttemptId(i64);

#[derive(Clone, Serialize, Deserialize)]
pub struct InvitationMail {
    pub attempt_id: AttemptId,
    pub household_id: i64,
    #[serde(flatten)]
    pub email: Email,
}

pub(super) async fn enqueue(
    tenant: &TenantTransaction,
    invitation: &household_invitation::Model,
    email: Email,
) -> Result<(), OperationError> {
    let attempt = invitation_mail_attempt::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        invitation_id: Set(Some(invitation.id)),
        original_invitation_id: Set(invitation.id),
        sender_account_id: Set(tenant.scope().actor.account_id),
        sender_membership_id: Set(tenant.membership().id),
        sender_permissions_version: Set(tenant.membership().permissions_version),
        token_digest: Set(invitation.token_digest.clone()),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    let mail = InvitationMail {
        attempt_id: AttemptId(attempt.id),
        household_id: tenant.scope().household_id,
        email,
    };
    let payload = serde_json::to_value(mail).map_err(|_| OperationError::Unavailable)?;
    tenant.transaction().execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO pg_loco_queue(id,task_data,name,run_at,priority) VALUES($1,$2,$3,clock_timestamp(),$4)",
        [ulid::Ulid::generate().to_string().into(),payload.into(),InvitationMailWorker::class_name().into(),DEFAULT_MAILER_PRIORITY.into()],
    )).await?;
    Ok(())
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum AttemptStatus {
    Queued,
    SentToProvider,
    Failed,
}

impl AttemptStatus {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::SentToProvider => "sent_to_provider",
            Self::Failed => "failed",
        }
    }

    pub(super) fn parse(value: &str) -> Result<Self, OperationError> {
        match value {
            "queued" => Ok(Self::Queued),
            "sent_to_provider" => Ok(Self::SentToProvider),
            "failed" => Ok(Self::Failed),
            _ => Err(OperationError::Unavailable),
        }
    }
}

#[derive(Clone, Copy)]
pub enum FailureCategory {
    Unavailable,
    ProviderUnavailable,
    Uncertain,
}

impl FailureCategory {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::Uncertain => "uncertain",
        }
    }

    pub(super) fn parse(value: &str) -> Result<Self, OperationError> {
        match value {
            "unavailable" => Ok(Self::Unavailable),
            "provider_unavailable" => Ok(Self::ProviderUnavailable),
            "uncertain" => Ok(Self::Uncertain),
            _ => Err(OperationError::Unavailable),
        }
    }
}
