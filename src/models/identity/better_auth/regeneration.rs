use super::{
    ClinicalAuthSchema, ClinicalStore,
    credentials::RegistrationGrant,
    onboarding::RECOVERY_CONFIG,
    store::{clinical_id, database_error, statement},
};
use async_trait::async_trait;
use better_auth_core::{
    AuthContext, AuthError, AuthPlugin, AuthRequest, AuthResponse, AuthResult, AuthRoute,
    CreateVerification, HttpMethod,
    wire::{SessionView, UserView},
};
use sea_orm::ConnectionTrait;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

pub(super) struct Regeneration(pub Arc<ClinicalStore>);

pub(super) async fn issue(
    store: &ClinicalStore,
    ctx: &AuthContext<ClinicalAuthSchema>,
    user: &UserView,
    session: &SessionView,
) -> AuthResult<AuthResponse> {
    let transaction = store.transaction().await?;
    let result = super::onboarding::issue_codes(store, ctx, &user.id, false).await?;
    let generation = result["generation"]
        .as_str()
        .ok_or_else(|| AuthError::internal("Recovery generation unavailable"))?;
    ctx.database
        .create_verification(CreateVerification {
            identifier: format!("recovery-ack:{generation}"),
            value: serde_json::to_string(&RegistrationGrant {
                account_id: user.id.clone(),
                session_id: session.id.clone(),
                recovery: false,
            })?,
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(30),
        })
        .await?;
    store
        .audit(
            &transaction,
            clinical_id(&user.id)?,
            "recovery_codes",
            "regenerated",
        )
        .await?;
    super::mail::notice(store, user, "Recovery codes replaced", "Your MedTracker recovery codes were replaced. Previous recovery codes no longer work. Save the new codes before leaving the confirmation page.").await?;
    transaction.commit().await.map_err(database_error)?;
    Ok(AuthResponse::json(200, &result)?)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Acknowledge {
    generation: String,
    saved: bool,
}

impl Regeneration {
    async fn acknowledge(
        &self,
        request: &AuthRequest,
        ctx: &AuthContext<ClinicalAuthSchema>,
    ) -> AuthResult<AuthResponse> {
        let input: Acknowledge = request
            .body_as_json()
            .map_err(|_| AuthError::bad_request("Invalid recovery acknowledgement"))?;
        if !input.saved || uuid::Uuid::parse_str(&input.generation).is_err() {
            return Err(AuthError::InvalidCredentials);
        }
        let (user, session) = ctx.require_authoritative_session(request).await?;
        let transaction = self.0.transaction().await?;
        self.0.lock_security_session(&transaction, &session).await?;
        let record = ctx
            .database
            .consume_verification_by_identifier(&format!("recovery-ack:{}", input.generation))
            .await?
            .ok_or(AuthError::InvalidCredentials)?;
        let binding: RegistrationGrant = serde_json::from_str(&record.value)?;
        if binding.account_id != user.id || binding.session_id != session.id {
            return Err(AuthError::InvalidCredentials);
        }
        let updated = transaction.execute_raw(statement("UPDATE public.identity_api_keys SET payload=jsonb_set(payload,'{metadata}',to_jsonb(((payload->>'metadata')::jsonb || jsonb_build_object('acknowledged',true))::text)) WHERE account_id=$1 AND payload->>'configId'=$2 AND (payload->>'metadata')::jsonb->>'generation'=$3", [clinical_id(&user.id)?.into(), RECOVERY_CONFIG.into(), input.generation.into()])).await.map_err(database_error)?.rows_affected();
        if updated != 10 {
            return Err(AuthError::InvalidCredentials);
        }
        self.0
            .audit(
                &transaction,
                clinical_id(&user.id)?,
                "recovery_codes",
                "saved",
            )
            .await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(AuthResponse::json(200, &json!({"status":true}))?)
    }
}

#[async_trait]
impl AuthPlugin<ClinicalAuthSchema> for Regeneration {
    fn name(&self) -> &'static str {
        "clinical-recovery-regeneration"
    }
    fn routes(&self) -> Vec<AuthRoute> {
        vec![AuthRoute::post(
            "/security/recovery/acknowledge",
            "acknowledge_recovery_generation",
        )]
    }
    async fn on_request(
        &self,
        request: &AuthRequest,
        ctx: &AuthContext<ClinicalAuthSchema>,
    ) -> AuthResult<Option<AuthResponse>> {
        if request.method == HttpMethod::Post && request.path == "/security/recovery/acknowledge" {
            self.acknowledge(request, ctx).await.map(Some)
        } else {
            Ok(None)
        }
    }
}
