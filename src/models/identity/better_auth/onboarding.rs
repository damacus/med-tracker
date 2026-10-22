use super::{
    ClinicalAuthSchema, IdentityPolicy, IdentityService,
    policy::{clinical_id, context, database_error, statement},
};
use async_trait::async_trait;
use better_auth_core::{
    AuthContext, AuthError, AuthPlugin, AuthRequest, AuthResponse, AuthResult, AuthRoute,
    CreateUser, FieldValue, HttpMethod,
};
use sea_orm::ConnectionTrait;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

pub(super) struct Onboarding(pub Arc<IdentityPolicy>);

#[derive(Deserialize)]
struct Signup {
    name: String,
    email: String,
    date_of_birth: chrono::NaiveDate,
    invitation_token: Option<String>,
    credential: String,
    password: Option<String>,
}

pub(super) async fn signup(
    service: &IdentityService,
    request: &AuthRequest,
) -> AuthResult<AuthResponse> {
    let input: Signup = request
        .body_as_json()
        .map_err(|_| AuthError::validation("Enter a valid account registration"))?;
    let metadata = json!({"name":input.name,"date_of_birth":input.date_of_birth,"invitation_token":input.invitation_token});
    match input.credential.as_str() {
        "password" => {
            let password = input
                .password
                .ok_or_else(|| AuthError::validation("Password is required"))?;
            super::validate_new_password(&password).map_err(AuthError::validation)?;
            let (sender, receiver) =
                std::sync::mpsc::sync_channel::<super::native_mail::SignupDelivery>(2);
            let native = request.clone();
            native.set_server_context(
                super::native_mail::SIGNUP_DELIVERY,
                better_auth_core::FieldFunction::bound(sender).into(),
            )?;
            let response=service.call_endpoint(HttpMethod::Post, "/sign-up/email", better_auth::server_api::EndpointInput {
                request: Some(native), headers: Some(request.headers.clone()),
                body: Some(json!({"name":input.name,"email":input.email,"password":password,"metadata":metadata,"callbackURL":"/auth/passkey/setup"})), ..Default::default()
            }).await?;
            let (mut created, mut queued) = (false, false);
            for outcome in receiver.try_iter() {
                match outcome {
                    super::native_mail::SignupDelivery::Created => created = true,
                    super::native_mail::SignupDelivery::Queued => queued = true,
                    super::native_mail::SignupDelivery::Failed => {
                        return Err(AuthError::internal(
                            "Registration verification delivery unavailable",
                        ));
                    }
                }
            }
            if created && !queued {
                return Err(AuthError::internal(
                    "Registration verification delivery unavailable",
                ));
            }
            Ok(response)
        }
        "passkey" if input.password.as_deref().is_none_or(str::is_empty) => {
            super::passwordless_registration::register(
                service.context(),
                request,
                CreateUser {
                    name: Some(input.name).into(),
                    email: Some(input.email),
                    metadata: Some(FieldValue::from_json(metadata)?),
                    ..Default::default()
                },
                service.policy.settings().cloned(),
            )
            .await?;
            AuthResponse::json(200, &json!({"status":true}))
        }
        _ => Err(AuthError::validation("Choose password or passkey")),
    }
}

impl Onboarding {
    async fn complete(
        &self,
        request: &AuthRequest,
        ctx: &AuthContext<ClinicalAuthSchema>,
    ) -> AuthResult<AuthResponse> {
        let (user, session) = ctx.require_authoritative_session(request).await?;
        if !*user.email_verified.typed()?
            || session
                .additional_fields
                .get("clinical_session_purpose")
                .and_then(FieldValue::as_str)
                != Some("enrolment")
        {
            return Err(AuthError::Unauthenticated);
        }
        let account = clinical_id(&user.id.display_string()?)?;
        let transaction = self.0.transaction().await?;
        context(
            &transaction,
            "med_tracker.current_account_id",
            &account.to_string(),
        )
        .await?;
        let row = transaction.query_one_raw(statement("SELECT a.id FROM accounts a WHERE a.id=$1 AND a.status=2 AND (EXISTS(SELECT 1 FROM auth_credentials WHERE user_id=a.id AND (provider_id<>'credential' OR password IS NOT NULL)) OR EXISTS(SELECT 1 FROM identity_passkeys WHERE account_id=a.id)) FOR UPDATE", [account.into()])).await.map_err(database_error)?;
        if row.is_none() {
            return Err(AuthError::forbidden(
                "Add a sign-in method before continuing",
            ));
        }
        transaction.execute_raw(statement("UPDATE identity_onboarding SET completed_at=clock_timestamp() WHERE account_id=$1 AND completed_at IS NULL", [account.into()])).await.map_err(database_error)?;
        transaction.execute_raw(statement("UPDATE identity_sessions SET purpose='authenticated' WHERE account_id=$1 AND token=$2 AND purpose='enrolment' AND active", [account.into(), session.token.typed()?.clone().into()])).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        AuthResponse::json(200, &json!({"status":true}))
    }
}

#[async_trait]
impl AuthPlugin<ClinicalAuthSchema> for Onboarding {
    fn name(&self) -> &'static str {
        "medtracker-onboarding"
    }
    fn routes(&self) -> Vec<AuthRoute> {
        vec![AuthRoute::post(
            "/onboarding/complete",
            "complete_onboarding",
        )]
    }
    async fn on_request(
        &self,
        request: &AuthRequest,
        ctx: &AuthContext<ClinicalAuthSchema>,
    ) -> AuthResult<Option<AuthResponse>> {
        if request.method != HttpMethod::Post {
            return Ok(None);
        }
        match request.path() {
            "/onboarding/complete" => self.complete(request, ctx).await.map(Some),
            _ => Ok(None),
        }
    }
}
