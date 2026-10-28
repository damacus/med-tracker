use super::{super::browser, ValidatedPrincipal};
use crate::models::{
    care::doses::{CredentialMethod, CredentialProvenance},
    errors::OperationError,
    ids::AccountId,
    profile::personal_history::{self, AccessOutcome},
};
use sea_orm::DatabaseConnection;

pub(crate) struct HistoryActor {
    account_id: AccountId,
    provenance: CredentialProvenance,
}

impl HistoryActor {
    pub(crate) fn browser(principal: &browser::BrowserPrincipal) -> Self {
        Self {
            account_id: AccountId::from_database(principal.account_id()),
            provenance: principal.provenance().clone(),
        }
    }

    pub(crate) fn resource(principal: &ValidatedPrincipal) -> Self {
        Self {
            account_id: AccountId::from_database(principal.account_id()),
            provenance: principal.provenance().clone(),
        }
    }

    pub(crate) fn account_id(&self) -> AccountId {
        self.account_id
    }

    pub(crate) fn provenance(&self) -> &CredentialProvenance {
        &self.provenance
    }

    pub(crate) fn permits_history(&self) -> bool {
        matches!(
            self.provenance.method,
            CredentialMethod::BrowserSession | CredentialMethod::OauthGrant
        )
    }
}

pub(crate) async fn deny_personal_history(
    db: &DatabaseConnection,
    actor: Option<&HistoryActor>,
    request_id: &str,
) -> Result<(), OperationError> {
    let Some(actor) = actor else {
        tracing::info!(
            event_type = "personal_history_denied",
            outcome = "denied",
            reason = "unproved_identity",
            "Personal history read denied without authenticated identity"
        );
        return Ok(());
    };
    let transaction = browser::transaction(db)
        .await
        .map_err(|_| OperationError::Unavailable)?;
    personal_history::record_access(&transaction, actor, request_id, AccessOutcome::Denied).await?;
    transaction.commit().await?;
    Ok(())
}
