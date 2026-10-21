use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult, CreateInvitation, Invitation, InvitationStatus,
    store::{InvitationOrganization, InvitationStore},
};
use chrono::{DateTime, Utc};

use super::ClinicalStore;

#[async_trait]
impl InvitationStore for ClinicalStore {
    async fn create_invitation(&self, _input: CreateInvitation) -> AuthResult<Invitation> {
        Err(AuthError::config(
            "Native organization invitations are not configured",
        ))
    }

    async fn get_invitation_by_id(&self, _id: &str) -> AuthResult<Option<Invitation>> {
        Err(AuthError::config(
            "Native organization invitations are not configured",
        ))
    }

    async fn get_pending_invitation(
        &self,
        _org: &str,
        _email: &str,
    ) -> AuthResult<Option<Invitation>> {
        Err(AuthError::config(
            "Native organization invitations are not configured",
        ))
    }

    async fn update_invitation_status(
        &self,
        _id: &str,
        _status: InvitationStatus,
    ) -> AuthResult<Option<Invitation>> {
        Err(AuthError::config(
            "Native organization invitations are not configured",
        ))
    }

    async fn update_invitation_expiry(
        &self,
        _id: &str,
        _expires_at: DateTime<Utc>,
    ) -> AuthResult<Invitation> {
        Err(AuthError::config(
            "Native organization invitations are not configured",
        ))
    }

    async fn list_organization_invitations(&self, _org: &str) -> AuthResult<Vec<Invitation>> {
        Err(AuthError::config(
            "Native organization invitations are not configured",
        ))
    }
    async fn count_pending_organization_invitations(&self, _org: &str) -> AuthResult<i64> {
        Err(AuthError::config(
            "Native organization invitations are not configured",
        ))
    }
    async fn list_user_invitations(&self, _email: &str) -> AuthResult<Vec<InvitationOrganization>> {
        Err(AuthError::config(
            "Native organization invitations are not configured",
        ))
    }
}
