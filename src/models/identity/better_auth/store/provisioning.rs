use better_auth_core::{AuthError, AuthResult, CreateUser, wire::UserView};
use chrono::NaiveDate;
use sea_orm::DatabaseTransaction;

use super::ClinicalStore;
use crate::models::identity::signup::{self, AccountProfile, SignupError};

impl ClinicalStore {
    pub(super) async fn create_user_in(
        &self,
        transaction: &DatabaseTransaction,
        input: CreateUser,
    ) -> AuthResult<UserView> {
        if input.email_verified == Some(true) || input.role.is_some() {
            return Err(AuthError::forbidden(
                "Account verification and privileges cannot be supplied during registration",
            ));
        }
        let metadata = input
            .metadata
            .as_ref()
            .and_then(better_auth_core::FieldValue::as_object)
            .ok_or_else(|| AuthError::validation("A valid date of birth is required"))?;
        let date_of_birth = metadata
            .get("date_of_birth")?
            .and_then(|value| value.as_str().map(str::to_owned))
            .and_then(|value| NaiveDate::parse_from_str(&value, "%Y-%m-%d").ok())
            .ok_or_else(|| AuthError::validation("A valid date of birth is required"))?;
        let invitation_token = metadata
            .get("invitation_token")?
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| AuthError::validation("Invalid invitation token"))
            })
            .transpose()?;
        let profile = AccountProfile {
            email: input
                .email
                .ok_or_else(|| AuthError::validation("Email is required"))?,
            name: input
                .name
                .typed()?
                .clone()
                .ok_or_else(|| AuthError::validation("Name is required"))?,
            date_of_birth,
            invitation_token,
        };
        let request_id = super::super::request::request_id()
            .ok_or_else(|| AuthError::internal("Identity request context missing"))?;
        let provisioned =
            signup::provision_account_in(transaction, &profile, &request_id, self.settings())
                .await
                .map_err(provisioning_error)?;
        use sea_orm::ConnectionTrait;
        transaction
            .execute_raw(super::statement(
                "INSERT INTO public.identity_onboarding(account_id) VALUES($1)",
                [provisioned.account_id.into()],
            ))
            .await
            .map_err(super::database_error)?;
        self.canonical_user_in(transaction, &provisioned.account_id.to_string())
            .await?
            .ok_or_else(|| AuthError::internal("Provisioned account unavailable"))
    }
}

pub(super) fn provisioning_error(error: SignupError) -> AuthError {
    match error {
        SignupError::Invalid(errors)
            if errors.get("email").is_some_and(|messages| {
                messages
                    .iter()
                    .any(|message| message == "is already registered")
            }) =>
        {
            AuthError::Conflict("Account registration exists".into())
        }
        SignupError::Invalid(errors) => AuthError::validation(
            errors
                .into_iter()
                .map(|(field, messages)| format!("{field}: {}", messages.join(", ")))
                .collect::<Vec<_>>()
                .join("; "),
        ),
        SignupError::RegistrationClosed => AuthError::forbidden("Account registration is closed"),
        SignupError::InvalidKey => AuthError::bad_request("Invitation is unavailable"),
        SignupError::Unavailable => AuthError::internal("Account provisioning unavailable"),
    }
}
