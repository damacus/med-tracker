use better_auth_core::{AuthContext, AuthError, AuthRequest, AuthResult, CreateUser, FieldValue};

use super::schema::AccountSchema;

pub(super) async fn register(
    context: &AuthContext<AccountSchema>,
    request: &AuthRequest,
    input: CreateUser,
    settings: Option<serde_json::Value>,
) -> AuthResult<()> {
    let context = context.clone();
    let store = context.database.clone();
    let request = request.clone();
    let hook_context = better_auth_core::RequestHookContext::from_request(&request)?;
    better_auth_core::with_request_hook_context_value(hook_context, async move {
        better_auth_core::store::transaction(store.as_ref(), move |tx| {
            Box::pin(async move {
                let user = tx.create_user(input).await?;
                let transaction = super::native_hooks::complete_passwordless_profile(
                    &request,
                    &user.id.display_string()?,
                    settings.as_ref(),
                )
                .await?;
                super::native_mail::send_passwordless_registration(&context, &user, &transaction)
                    .await?;
                Ok(())
            })
        })
        .await
    })
    .await
}

pub(super) fn registration_profile(
    request: &AuthRequest,
) -> AuthResult<(
    better_auth_seaorm::TransactionConnection,
    crate::models::identity::signup::AccountProfile,
)> {
    let Some(FieldValue::Function(transaction)) =
        request.server_context(super::native_hooks::TRANSACTION_BINDING)?
    else {
        return Err(AuthError::internal("Signup transaction unavailable"));
    };
    let Some(FieldValue::Function(profile)) =
        request.server_context(super::native_hooks::PROFILE_BINDING)?
    else {
        return Err(AuthError::internal("Signup profile unavailable"));
    };
    Ok((
        transaction
            .binding::<better_auth_seaorm::TransactionConnection>()
            .ok_or_else(|| AuthError::internal("Signup transaction unavailable"))?
            .clone(),
        {
            let profile = profile
                .binding::<crate::models::identity::signup::AccountProfile>()
                .ok_or_else(|| AuthError::internal("Signup profile unavailable"))?;
            crate::models::identity::signup::AccountProfile {
                email: profile.email.clone(),
                name: profile.name.clone(),
                date_of_birth: profile.date_of_birth,
                invitation_token: profile.invitation_token.clone(),
            }
        },
    ))
}
