use crate::models::identity::store::digest as token_digest;
use std::sync::Arc;

use async_trait::async_trait;
use better_auth::plugins::{
    ApiKeyPlugin,
    api_key::{ApiKeyVerificationError, VerifyApiKey},
};
use better_auth_core::{
    AuthContext, AuthError, AuthPlugin, AuthRequest, AuthResponse, AuthResult, AuthRoute,
    HttpMethod,
};
use sea_orm::ConnectionTrait;
use serde::Deserialize;
use serde_json::json;

use super::{
    ClinicalAuthSchema, ClinicalStore,
    onboarding::RECOVERY_CONFIG,
    store::{clinical_id, context, database_error, statement},
};

pub(super) struct Recovery(pub Arc<ClinicalStore>);

#[derive(Deserialize)]
struct Input {
    code: String,
}

impl Recovery {
    async fn login(
        &self,
        request: &AuthRequest,
        ctx: &AuthContext<ClinicalAuthSchema>,
    ) -> AuthResult<AuthResponse> {
        let input: Input = request.body_as_json()?;
        if input.code.len() > 128 || !input.code.starts_with("recovery_") {
            return Err(AuthError::InvalidCredentials);
        }
        let plugin = ApiKeyPlugin::builder()
            .config_id(RECOVERY_CONFIG.to_owned())
            .key_length(64)
            .prefix("recovery_".to_owned())
            .store_starting_characters(false)
            .build();
        let key = plugin
            .verify_api_key(
                &VerifyApiKey {
                    key: &input.code,
                    config_id: Some(RECOVERY_CONFIG),
                    permissions: None,
                },
                ctx,
            )
            .await
            .map_err(|error| match error {
                ApiKeyVerificationError::Internal(error) => error,
                ApiKeyVerificationError::Validation(_) => AuthError::InvalidCredentials,
            })?;
        if key.config_id != RECOVERY_CONFIG {
            return Err(AuthError::InvalidCredentials);
        }
        if key
            .metadata
            .as_ref()
            .is_some_and(|metadata| metadata["acknowledged"].as_bool() == Some(false))
        {
            return Err(AuthError::InvalidCredentials);
        }
        let account_id = clinical_id(&key.reference_id)?;
        let user = ctx
            .database
            .get_user_by_id(&key.reference_id)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;
        let transaction = self.0.transaction().await?;
        context(
            &transaction,
            "med_tracker.current_account_id",
            &key.reference_id,
        )
        .await?;
        transaction
            .query_one_raw(statement(
                "SELECT id FROM public.accounts WHERE id=$1 AND status=2 FOR UPDATE",
                [account_id.into()],
            ))
            .await
            .map_err(database_error)?
            .ok_or(AuthError::InvalidCredentials)?;
        if transaction.query_one_raw(statement("SELECT account_id FROM public.identity_onboarding WHERE account_id=$1 AND recovery_saved_at IS NOT NULL", [account_id.into()])).await.map_err(database_error)?.is_none() { return Err(AuthError::InvalidCredentials); }
        transaction
            .execute_raw(statement(
                "DELETE FROM public.identity_sessions WHERE account_id=$1",
                [account_id.into()],
            ))
            .await
            .map_err(database_error)?;
        transaction
            .execute_raw(statement(
                "DELETE FROM public.account_active_session_keys WHERE account_id=$1",
                [account_id.into()],
            ))
            .await
            .map_err(database_error)?;
        transaction.execute_raw(statement("UPDATE public.api_sessions SET revoked_at=timezone('UTC',clock_timestamp()),updated_at=timezone('UTC',clock_timestamp()) WHERE account_id=$1 AND revoked_at IS NULL", [account_id.into()])).await.map_err(database_error)?;
        transaction.execute_raw(statement("UPDATE public.api_app_tokens SET revoked_at=timezone('UTC',clock_timestamp()),updated_at=timezone('UTC',clock_timestamp()) WHERE account_id=$1 AND revoked_at IS NULL", [account_id.into()])).await.map_err(database_error)?;
        transaction.execute_raw(statement("UPDATE public.oauth_grants SET revoked_at=timezone('UTC',clock_timestamp()) WHERE account_id=$1 AND revoked_at IS NULL", [account_id.into()])).await.map_err(database_error)?;
        transaction.execute_raw(statement("DELETE FROM public.identity_api_keys WHERE account_id=$1 AND payload->>'configId'<>$2", [account_id.into(), RECOVERY_CONFIG.into()])).await.map_err(database_error)?;
        self.0
            .audit(&transaction, account_id, "recovery_code", "used")
            .await?;
        let session = ctx
            .session_manager()
            .create_session(&user, None, request.headers.get("user-agent").cloned())
            .await?;
        transaction.execute_raw(statement("UPDATE public.identity_sessions SET additional_fields=additional_fields || jsonb_build_object('authentication_method','recovery') WHERE token=$1 AND account_id=$2", [token_digest(&session.token).into(), account_id.into()])).await.map_err(database_error)?;
        super::mail::notice(&self.0, &user, "Recovery code used", "A recovery code was used to sign in to your MedTracker account. Your other sessions and personal API keys have been revoked. Unused recovery codes remain valid. If this was not you, secure your account now.").await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(
            AuthResponse::json(200, &json!({"status":true}))?.with_header(
                "Set-Cookie",
                better_auth_core::utils::cookie_utils::create_session_cookie(
                    &session.token,
                    &ctx.config,
                ),
            ),
        )
    }
}

#[async_trait]
impl AuthPlugin<ClinicalAuthSchema> for Recovery {
    fn name(&self) -> &'static str {
        "clinical-recovery"
    }
    fn routes(&self) -> Vec<AuthRoute> {
        vec![AuthRoute::post("/recovery/login", "recovery_login")]
    }
    async fn on_request(
        &self,
        request: &AuthRequest,
        ctx: &AuthContext<ClinicalAuthSchema>,
    ) -> AuthResult<Option<AuthResponse>> {
        if request.method == HttpMethod::Post && request.path == "/recovery/login" {
            self.login(request, ctx).await.map(Some)
        } else {
            Ok(None)
        }
    }
}
