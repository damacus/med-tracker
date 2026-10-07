mod acceptance;
mod acceptance_effects;
mod delivery;
mod input;
pub(crate) use input::validated_email;
mod issuing;
mod lifecycle;
mod reading;
mod signup;
mod tokens;

use super::administration;
use crate::models::{
    access::TenantTransaction,
    entities::{household_invitation, household_invitation_grant},
    errors::OperationError,
};
use chrono::{NaiveDateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde_json::{Value, json};

pub use acceptance::{accept, accept_browser};
pub(crate) use acceptance::AcceptanceActor;
pub(crate) use acceptance_effects::apply as apply_acceptance;
pub use issuing::create;
pub use lifecycle::{cancel, resend, revoke};
pub use reading::{list, list_api, options, preview};
pub use signup::accept_signup;
pub(crate) use signup::signup_invitation;

#[derive(Debug)]
pub enum ResendError {
    Operation(OperationError),
    DeliveryUnavailable,
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

fn summary(row: &household_invitation::Model, now: NaiveDateTime) -> Value {
    json!({"id":row.id,"email":row.email,"membership_role":row.membership_role,"pending":row.accepted_at.is_none() && row.revoked_at.is_none() && row.expires_at>now,"accepted_at":row.accepted_at.map(|at|at.and_utc().to_rfc3339()),"revoked_at":row.revoked_at.map(|at|at.and_utc().to_rfc3339()),"expires_at":row.expires_at.and_utc().to_rfc3339()})
}
