use super::*;
use crate::models::{
    access::{self, Actor, HouseholdScope},
    authorization,
    care::browser_push::scheduler::household_transaction,
    entities::{household, membership},
};
use loco_rs::{app::AppContext, mailer::MailerWorker};
use sea_orm::sea_query::{Expr, Func, SimpleExpr};
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
    let Some(target) = invitation_mail_attempt::Entity::find_by_id(mail.attempt_id.0)
        .filter(invitation_mail_attempt::Column::HouseholdId.eq(mail.household_id))
        .one(&db)
        .await?
    else {
        db.rollback().await?;
        return Ok(None);
    };
    let invitation = household_invitation::Entity::find_by_id(target.original_invitation_id)
        .filter(household_invitation::Column::HouseholdId.eq(mail.household_id))
        .lock_exclusive()
        .one(&db)
        .await?;
    let row = invitation_mail_attempt::Entity::find_by_id(mail.attempt_id.0)
        .filter(invitation_mail_attempt::Column::HouseholdId.eq(mail.household_id))
        .one(&db)
        .await?
        .ok_or(OperationError::Unavailable)?;
    let attempt = Attempt {
        invitation_id: row.invitation_id,
        original_invitation_id: row.original_invitation_id,
        sender_account_id: row.sender_account_id,
        sender_membership_id: row.sender_membership_id,
        sender_permissions_version: row.sender_permissions_version,
        token_digest: row.token_digest,
        status: AttemptStatus::parse(&row.status)?,
        started_at: row.started_at,
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

fn database_now() -> SimpleExpr {
    Func::cust("timezone")
        .args([Expr::val("UTC"), Func::cust("clock_timestamp").into()])
        .into()
}

async fn finish(
    db: &DatabaseTransaction,
    mail: &InvitationMail,
    category: Option<FailureCategory>,
) -> Result<(), OperationError> {
    invitation_mail_attempt::Entity::update_many()
        .col_expr(
            invitation_mail_attempt::Column::Status,
            Expr::val(if category.is_some() {
                "failed"
            } else {
                "sent_to_provider"
            }),
        )
        .col_expr(
            invitation_mail_attempt::Column::Category,
            Expr::val(category.map(|value| value.key().to_owned())),
        )
        .col_expr(invitation_mail_attempt::Column::FinishedAt, database_now())
        .filter(invitation_mail_attempt::Column::HouseholdId.eq(mail.household_id))
        .filter(invitation_mail_attempt::Column::Id.eq(mail.attempt_id.0))
        .filter(invitation_mail_attempt::Column::Status.eq(AttemptStatus::Queued.key()))
        .exec(db)
        .await?;
    Ok(())
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeliverySettings {
    send_timeout_seconds: Option<u64>,
    from: Option<String>,
}

fn send_settings(
    value: Option<&serde_json::Value>,
) -> Result<(std::time::Duration, Option<String>), OperationError> {
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
    Ok((
        std::time::Duration::from_secs(seconds),
        settings
            .from
            .map(|sender| sender.trim().to_owned())
            .filter(|sender| !sender.is_empty()),
    ))
}

impl InvitationMailWorker {
    async fn start(&self, mail: &InvitationMail) -> Result<(), OperationError> {
        let marker = household_transaction(&self.ctx, mail.household_id).await?;
        invitation_mail_attempt::Entity::update_many()
            .col_expr(invitation_mail_attempt::Column::StartedAt, database_now())
            .filter(invitation_mail_attempt::Column::HouseholdId.eq(mail.household_id))
            .filter(invitation_mail_attempt::Column::Id.eq(mail.attempt_id.0))
            .filter(invitation_mail_attempt::Column::Status.eq(AttemptStatus::Queued.key()))
            .filter(invitation_mail_attempt::Column::StartedAt.is_null())
            .exec(&marker)
            .await?;
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
        let (deadline, sender) = match send_settings(
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
        let row = invitation_mail_attempt::Entity::find_by_id(mail.attempt_id.0)
            .filter(invitation_mail_attempt::Column::HouseholdId.eq(mail.household_id))
            .lock_exclusive()
            .one(&first.db)
            .await?
            .ok_or(OperationError::Unavailable)?;
        if AttemptStatus::parse(&row.status)? != AttemptStatus::Queued {
            first.db.commit().await?;
            return Ok(());
        }
        let category = if !allowed(&first, &mail).await? {
            Some(FailureCategory::Unavailable)
        } else {
            let mut email = mail.email.clone();
            email.from = email.from.or(sender);
            match tokio::time::timeout(deadline, MailerWorker::build(&self.ctx).perform(email))
                .await
            {
                Ok(Ok(())) => None,
                Ok(Err(_)) => Some(FailureCategory::ProviderUnavailable),
                Err(_) => Some(FailureCategory::Uncertain),
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
    use super::send_settings;
    use serde_json::json;
    use std::time::Duration;

    #[test]
    fn invitation_mail_deadline_validates_settings_and_null_fallback() {
        assert_eq!(send_settings(None), Ok((Duration::from_secs(30), None)));
        for value in [json!(null), json!({}), json!({"send_timeout_seconds":null})] {
            assert_eq!(
                send_settings(Some(&value)),
                Ok((Duration::from_secs(30), None))
            );
        }
        for seconds in [1, 30, 300] {
            assert_eq!(
                send_settings(Some(&json!({"send_timeout_seconds":seconds}))),
                Ok((Duration::from_secs(seconds), None))
            );
        }
        for value in [
            json!({"send_timeout_seconds":0}),
            json!({"send_timeout_seconds":301}),
            json!({"send_timeout_seconds":-1}),
            json!({"send_timeout_seconds":"30"}),
            json!({"unrecognised":30}),
        ] {
            assert!(send_settings(Some(&value)).is_err());
        }
    }

    #[test]
    fn invitation_mail_sender_preserves_null_and_empty_fallback() {
        for value in [
            json!({"from":null}),
            json!({"from":""}),
            json!({"from":"  "}),
        ] {
            assert_eq!(
                send_settings(Some(&value)),
                Ok((Duration::from_secs(30), None))
            );
        }
        assert_eq!(
            send_settings(Some(
                &json!({"from":" MedTracker <invitations@medtracker.test> "})
            )),
            Ok((
                Duration::from_secs(30),
                Some("MedTracker <invitations@medtracker.test>".into())
            ))
        );
        assert!(send_settings(Some(&json!({"from":false}))).is_err());
    }
}
