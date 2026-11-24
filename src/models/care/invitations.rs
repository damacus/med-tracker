pub(crate) mod acceptance;
pub use super::person_schedules::sharing::SharingConsent;
mod acceptance_effects;
mod authority;
pub mod browser;
pub mod delivery;
mod input;
pub use input::InvitationInput;
pub(crate) use input::{validate_email, validated_email};
mod issuing;
mod lifecycle;
mod reading;
mod signup;
mod tokens;

use super::administration;
use crate::models::{
    access::TenantTransaction,
    entities::{household_invitation, household_invitation_grant, invitation_mail_attempt},
    errors::OperationError,
};
use crate::views::invitations::InvitationSummary;
use chrono::{NaiveDateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde_json::{Value, json};

pub use acceptance::{accept, accept_browser};
pub use issuing::create;
pub(crate) use issuing::create_for_parent;
pub use lifecycle::{cancel, resend, revoke};
pub use reading::{list, list_api, preview};
pub(crate) use signup::SignupInvitationError;
pub(crate) use signup::accept_signup;
pub(crate) use signup::signup_invitation;

#[derive(Debug)]
pub enum ResendError {
    Operation(OperationError),
    DeliveryUnavailable,
}

pub struct AcceptedInvitation {
    pub household_id: i64,
    pub membership_id: i64,
    pub person_id: i64,
    pub role: String,
}

impl From<OperationError> for ResendError {
    fn from(error: OperationError) -> Self {
        Self::Operation(error)
    }
}

impl From<sea_orm::DbErr> for ResendError {
    fn from(error: sea_orm::DbErr) -> Self {
        Self::Operation(error.into())
    }
}

fn invalid(field: &str, message: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"errors":{field:[message]}}),
    }
}

fn summary(row: &household_invitation::Model, now: NaiveDateTime) -> InvitationSummary {
    InvitationSummary {
        id: row.id,
        email: row.email.clone(),
        membership_role: row.membership_role.clone(),
        pending: row.accepted_at.is_none() && row.revoked_at.is_none() && row.expires_at > now,
        accepted_at: row.accepted_at.map(|at| at.and_utc().to_rfc3339()),
        revoked_at: row.revoked_at.map(|at| at.and_utc().to_rfc3339()),
        expires_at: row.expires_at.and_utc().to_rfc3339(),
    }
}

#[cfg(test)]
mod response_tests {
    use super::*;

    #[test]
    fn invitation_summary_preserves_pending_and_null_lifecycle_fields() {
        let now = chrono::DateTime::UNIX_EPOCH.naive_utc();
        let record = household_invitation::Model {
            id: 12,
            email: "synthetic@example.test".into(),
            membership_role: "member".into(),
            accepted_at: None,
            revoked_at: None,
            expires_at: now + chrono::Duration::days(1),
            created_at: now,
            updated_at: now,
            dependent_sharing: false,
            household_id: 23,
            invited_by_membership_id: 34,
            token_digest: "synthetic".into(),
        };
        let summary = summary(&record, now);
        assert_eq!(summary.id, 12);
        assert!(summary.pending);
        assert_eq!(
            serde_json::to_value(summary).expect("invitation response"),
            json!({"id":12,"email":"synthetic@example.test","membership_role":"member",
                "pending":true,"accepted_at":null,"revoked_at":null,
                "expires_at":"1970-01-02T00:00:00+00:00"})
        );
    }
}
