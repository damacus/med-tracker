use async_trait::async_trait;
use better_auth::plugins::passkey::{
    PasskeyAuthenticationHook, PasskeyAuthenticationVerification, PasskeyEndpoint,
    PasskeyRegistrationHook, PasskeyRegistrationResult, PasskeyRegistrationUser,
    PasskeyRegistrationVerification,
};
use better_auth_core::{AuthError, AuthResult};

pub(super) struct RequiredUserVerification;

#[async_trait]
impl PasskeyRegistrationHook for RequiredUserVerification {
    async fn after_verification(
        &self,
        _ctx: PasskeyEndpoint<'_>,
        verification: &PasskeyRegistrationVerification,
        _user: &PasskeyRegistrationUser,
        _client_data: &serde_json::Value,
        _context: Option<&str>,
    ) -> AuthResult<PasskeyRegistrationResult> {
        if !verification.registration_info.user_verified {
            return Err(AuthError::forbidden(
                "Verify your identity with this passkey",
            ));
        }
        Ok(PasskeyRegistrationResult::default())
    }
}

#[async_trait]
impl PasskeyAuthenticationHook for RequiredUserVerification {
    async fn after_verification(
        &self,
        _ctx: PasskeyEndpoint<'_>,
        verification: &PasskeyAuthenticationVerification,
        _client_data: &serde_json::Value,
    ) -> AuthResult<()> {
        if !verification.authentication_info.user_verified {
            return Err(AuthError::forbidden(
                "Verify your identity with this passkey",
            ));
        }
        Ok(())
    }
}
