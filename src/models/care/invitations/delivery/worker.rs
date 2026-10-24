use super::*;
use crate::models::{
    access::{self, Actor, HouseholdScope},
    authorization,
    care::browser_push::scheduler::household_transaction,
    entities::{household, membership},
};
use loco_rs::app::AppContext;
use sea_orm::{DatabaseTransaction, QuerySelect};

pub struct InvitationMailWorker {
    ctx: AppContext,
}

struct Attempt {
    invitation_id: Option<i64>,
    original_invitation_id: i64,
    sender_account_id: i64,
    sender_membership_id: i64,
    sender_permissions_version: i32,
    token_digest: String,
    status: AttemptStatus,
    started_at: Option<NaiveDateTime>,
}

struct LockedAttempt {
    db: DatabaseTransaction,
    attempt: Attempt,
    invitation: Option<household_invitation::Model>,
}

async fn locked(
    ctx: &AppContext,
    mail: &InvitationMail,
) -> Result<Option<LockedAttempt>, OperationError> {
    let db = household_transaction(ctx, mail.household_id).await?;
    if household::Entity::find_by_id(mail.household_id)
        .lock_exclusive()
        .one(&db)
        .await?
        .is_none()
    {
        db.rollback().await?;
        return Ok(None);
    }
    let target = db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT original_invitation_id FROM invitation_mail_attempts WHERE household_id=$1 AND id=$2",
        [mail.household_id.into(),mail.attempt_id.0.into()],
    )).await?;
    let Some(target) = target else {
        db.rollback().await?;
        return Ok(None);
    };
    let invitation_id: i64 = target.try_get("", "original_invitation_id")?;
    let invitation = household_invitation::Entity::find_by_id(invitation_id)
        .filter(household_invitation::Column::HouseholdId.eq(mail.household_id))
        .lock_exclusive()
        .one(&db)
        .await?;
    let row = db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT invitation_id,original_invitation_id,sender_account_id,sender_membership_id,sender_permissions_version,token_digest,status,started_at FROM invitation_mail_attempts WHERE household_id=$1 AND id=$2",
        [mail.household_id.into(),mail.attempt_id.0.into()],
    )).await?.ok_or(OperationError::Unavailable)?;
    let status: String = row.try_get("", "status")?;
    let attempt = Attempt {
        invitation_id: row.try_get("", "invitation_id")?,
        original_invitation_id: row.try_get("", "original_invitation_id")?,
        sender_account_id: row.try_get("", "sender_account_id")?,
        sender_membership_id: row.try_get("", "sender_membership_id")?,
        sender_permissions_version: row.try_get("", "sender_permissions_version")?,
        token_digest: row.try_get("", "token_digest")?,
        status: AttemptStatus::parse(&status)?,
        started_at: row.try_get("", "started_at")?,
    };
    Ok(Some(LockedAttempt {
        db,
        attempt,
        invitation,
    }))
}

async fn allowed(locked: &LockedAttempt, mail: &InvitationMail) -> Result<bool, OperationError> {
    let Some(invitation) = &locked.invitation else {
        return Ok(false);
    };
    let attempt = &locked.attempt;
    if attempt.invitation_id != Some(invitation.id)
        || attempt.original_invitation_id != invitation.id
        || attempt.token_digest != invitation.token_digest
        || invitation.accepted_at.is_some()
        || invitation.revoked_at.is_some()
        || invitation.expires_at <= Utc::now().naive_utc()
    {
        return Ok(false);
    }
    let scope = HouseholdScope {
        actor: Actor {
            account_id: attempt.sender_account_id,
        },
        household_id: mail.household_id,
        request_id: "invitation-mail".into(),
    };
    let sender = match access::verify_membership(&locked.db, &scope).await {
        Ok(member) => member,
        Err(
            OperationError::Forbidden | OperationError::NotFound | OperationError::Unauthenticated,
        ) => return Ok(false),
        Err(error) => return Err(error),
    };
    if sender.id != attempt.sender_membership_id
        || sender.permissions_version != attempt.sender_permissions_version
        || !authorization::household_manager(&sender, mail.household_id)
    {
        return Ok(false);
    }
    let Some(inviter) = membership::Entity::find_by_id(invitation.invited_by_membership_id)
        .filter(membership::Column::HouseholdId.eq(mail.household_id))
        .one(&locked.db)
        .await?
    else {
        return Ok(false);
    };
    match super::super::authority::authorize(&locked.db, &inviter, invitation).await {
        Ok(()) => Ok(true),
        Err(OperationError::Unavailable) => Err(OperationError::Unavailable),
        Err(_) => Ok(false),
    }
}

async fn finish(
    db: &DatabaseTransaction,
    mail: &InvitationMail,
    category: Option<FailureCategory>,
) -> Result<(), OperationError> {
    db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "UPDATE invitation_mail_attempts SET status=$3,category=$4,finished_at=timezone('UTC',clock_timestamp()) WHERE household_id=$1 AND id=$2 AND status='queued'",
        [mail.household_id.into(),mail.attempt_id.0.into(),if category.is_some() { "failed" } else { "sent_to_provider" }.into(),category.map(|value|value.key().to_owned()).into()],
    )).await?;
    Ok(())
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeliverySettings {
    send_timeout_seconds: Option<u64>,
}

fn send_timeout(value: Option<&serde_json::Value>) -> Result<std::time::Duration, OperationError> {
    let settings: DeliverySettings = match value {
        Some(value) if !value.is_null() => {
            serde_json::from_value(value.clone()).map_err(|_| OperationError::Unavailable)?
        }
        _ => DeliverySettings::default(),
    };
    let seconds = settings.send_timeout_seconds.unwrap_or(30);
    if !(1..=300).contains(&seconds) {
        return Err(OperationError::Unavailable);
    }
    Ok(std::time::Duration::from_secs(seconds))
}

impl InvitationMailWorker {
    async fn start(&self, mail: &InvitationMail) -> Result<(), OperationError> {
        let marker = household_transaction(&self.ctx, mail.household_id).await?;
        marker.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE invitation_mail_attempts SET started_at=timezone('UTC',clock_timestamp()) WHERE household_id=$1 AND id=$2 AND status='queued' AND started_at IS NULL",
            [mail.household_id.into(),mail.attempt_id.0.into()],
        )).await?;
        marker.commit().await?;
        Ok(())
    }

    async fn deliver(&self, mail: InvitationMail) -> Result<(), OperationError> {
        let Some(first) = locked(&self.ctx, &mail).await? else {
            return Ok(());
        };
        if first.attempt.status != AttemptStatus::Queued {
            first.db.commit().await?;
            return Ok(());
        }
        let category = if first.attempt.started_at.is_some() {
            Some(FailureCategory::Uncertain)
        } else if !allowed(&first, &mail).await? {
            Some(FailureCategory::Unavailable)
        } else {
            None
        };
        if let Some(category) = category {
            finish(&first.db, &mail, Some(category)).await?;
            first.db.commit().await?;
            return Ok(());
        }
        let deadline = match send_timeout(
            self.ctx
                .config
                .settings
                .as_ref()
                .and_then(|settings| settings.get("invitation_mail")),
        ) {
            Ok(deadline) => deadline,
            Err(error) => {
                finish(&first.db, &mail, Some(FailureCategory::Unavailable)).await?;
                first.db.commit().await?;
                return Err(error);
            }
        };
        if let Err(error) = self.start(&mail).await {
            finish(&first.db, &mail, Some(FailureCategory::Unavailable)).await?;
            first.db.commit().await?;
            return Err(error);
        }
        let row = first.db.query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT status FROM invitation_mail_attempts WHERE household_id=$1 AND id=$2 FOR UPDATE",
            [mail.household_id.into(),mail.attempt_id.0.into()],
        )).await?.ok_or(OperationError::Unavailable)?;
        if AttemptStatus::parse(&row.try_get::<String>("", "status")?)? != AttemptStatus::Queued {
            first.db.commit().await?;
            return Ok(());
        }
        let category = if !allowed(&first, &mail).await? {
            Some(FailureCategory::Unavailable)
        } else {
            match &self.ctx.mailer {
                Some(mailer) => {
                    match tokio::time::timeout(deadline, mailer.mail(&mail.email)).await {
                        Ok(Ok(())) => None,
                        Ok(Err(_)) => Some(FailureCategory::ProviderUnavailable),
                        Err(_) => Some(FailureCategory::Uncertain),
                    }
                }
                None => Some(FailureCategory::ProviderUnavailable),
            }
        };
        finish(&first.db, &mail, category).await?;
        first.db.commit().await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl BackgroundWorker<InvitationMail> for InvitationMailWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    async fn perform(&self, mail: InvitationMail) -> loco_rs::Result<()> {
        crate::models::canary::work(&self.ctx).await?;
        self.deliver(mail)
            .await
            .map_err(|_| loco_rs::Error::string("Invitation mail attempt unavailable"))
    }
}

#[cfg(test)]
mod tests {
    use super::send_timeout;
    use serde_json::json;
    use std::time::Duration;

    #[test]
    fn invitation_mail_deadline_validates_settings_and_null_fallback() {
        assert_eq!(send_timeout(None), Ok(Duration::from_secs(30)));
        for value in [json!(null), json!({}), json!({"send_timeout_seconds":null})] {
            assert_eq!(send_timeout(Some(&value)), Ok(Duration::from_secs(30)));
        }
        for seconds in [1, 30, 300] {
            assert_eq!(
                send_timeout(Some(&json!({"send_timeout_seconds":seconds}))),
                Ok(Duration::from_secs(seconds))
            );
        }
        for value in [
            json!({"send_timeout_seconds":0}),
            json!({"send_timeout_seconds":301}),
            json!({"send_timeout_seconds":-1}),
            json!({"send_timeout_seconds":"30"}),
            json!({"unrecognised":30}),
        ] {
            assert!(send_timeout(Some(&value)).is_err());
        }
    }
}
