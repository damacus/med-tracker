use super::{
    ClinicalAuthSchema, ClinicalStore,
    store::{clinical_id, database_error},
};
use async_trait::async_trait;
use better_auth_core::{
    AuthContext, AuthError, AuthPlugin, AuthRequest, AuthResponse, AuthResult, AuthRoute,
    HttpMethod,
};
use serde::Deserialize;
use std::sync::Arc;

pub(super) struct Sessions(pub Arc<ClinicalStore>);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Revoke {
    session_id: String,
}

impl Sessions {
    async fn revoke(
        &self,
        request: &AuthRequest,
        ctx: &AuthContext<ClinicalAuthSchema>,
    ) -> AuthResult<AuthResponse> {
        let input: Revoke = request
            .body_as_json()
            .map_err(|_| AuthError::bad_request("Invalid session operation"))?;
        let (_, session) = ctx.require_authoritative_session(request).await?;
        let transaction = self.0.transaction().await?;
        self.0.lock_security_session(&transaction, &session).await?;
        let target = ctx
            .database
            .get_user_sessions(&session.user_id)
            .await?
            .into_iter()
            .find(|target| target.id == input.session_id)
            .ok_or(AuthError::InvalidCredentials)?;
        self.0
            .delete_user_session_in(&transaction, &session.user_id, &target.id)
            .await?;
        self.0
            .audit(
                &transaction,
                clinical_id(&session.user_id)?,
                "session",
                "revoked",
            )
            .await?;
        let mut response = AuthResponse::json(
            200,
            &serde_json::json!({"status":true,"current":target.id == session.id}),
        )?;
        if target.id == session.id {
            response = response.with_header(
                "Set-Cookie",
                better_auth_core::utils::cookie_utils::create_clear_cookie(
                    &ctx.config.session.cookie_name,
                    &ctx.config,
                ),
            );
        }
        transaction.commit().await.map_err(database_error)?;
        Ok(response)
    }
}

#[async_trait]
impl AuthPlugin<ClinicalAuthSchema> for Sessions {
    fn name(&self) -> &'static str {
        "clinical-session-management"
    }
    fn routes(&self) -> Vec<AuthRoute> {
        vec![AuthRoute::post(
            "/security/session/revoke",
            "revoke_account_session",
        )]
    }
    async fn on_request(
        &self,
        request: &AuthRequest,
        ctx: &AuthContext<ClinicalAuthSchema>,
    ) -> AuthResult<Option<AuthResponse>> {
        if request.method == HttpMethod::Post && request.path == "/security/session/revoke" {
            self.revoke(request, ctx).await.map(Some)
        } else {
            Ok(None)
        }
    }
}
