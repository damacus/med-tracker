use async_trait::async_trait;
use better_auth_core::{AuthError, AuthResult, CreateAccount, FieldFunction, FieldMap, FieldValue};
use better_auth_seaorm::{DatabaseHookUpdate, HookControl, SeaOrmHookContext, SeaOrmHooks};
use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};

use super::{
    policy::{clinical_id, database_error, statement},
    schema::AccountSchema,
};
use crate::models::identity::signup::{self, AccountProfile, SignupError};

pub(super) const TRANSACTION_BINDING: &str = "medtracker.signup-transaction";
pub(super) const PROFILE_BINDING: &str = "medtracker.signup-profile";

pub(super) struct DomainHooks {
    pub settings: Option<serde_json::Value>,
}

#[better_auth::database_hooks]
#[async_trait]
impl SeaOrmHooks<AccountSchema> for DomainHooks {
    async fn before_create_user(
        &self,
        fields: &mut FieldMap,
        ctx: &SeaOrmHookContext<'_, AccountSchema>,
    ) -> AuthResult<DatabaseHookUpdate<FieldMap>> {
        let tx = ctx
            .tx
            .ok_or_else(|| AuthError::internal("Signup transaction unavailable"))?;
        let request = &ctx
            .request
            .as_ref()
            .ok_or_else(|| AuthError::internal("Signup request unavailable"))?
            .request;
        let native_state = better_auth::plugins::oauth::get_oauth_state(request)?;
        let registration = if let Some(state) = native_state.as_ref() {
            state.model_property("profile")?
        } else {
            fields
                .get("metadata")
                .cloned()
                .unwrap_or(FieldValue::Undefined)
        };
        let metadata = registration
            .as_object()
            .ok_or_else(|| AuthError::validation("A valid date of birth is required"))?;
        let birth = metadata
            .get("date_of_birth")?
            .and_then(|v| v.as_str().map(str::to_owned))
            .and_then(|v| chrono::NaiveDate::parse_from_str(&v, "%Y-%m-%d").ok())
            .ok_or_else(|| AuthError::validation("A valid date of birth is required"))?;
        let invitation_token = match metadata.get("invitation_token")? {
            None | Some(FieldValue::Null | FieldValue::Undefined) => None,
            Some(FieldValue::String(value)) => Some(value),
            _ => return Err(AuthError::validation("Invalid invitation token")),
        };
        let mut profile = AccountProfile {
            name: metadata
                .get("name")?
                .and_then(|value| value.as_str().map(str::to_owned))
                .or_else(|| {
                    fields
                        .get("name")
                        .and_then(FieldValue::as_str)
                        .map(str::to_owned)
                })
                .ok_or_else(|| AuthError::validation("Name is required"))?,
            email: fields
                .get("email")
                .and_then(FieldValue::as_str)
                .ok_or_else(|| AuthError::validation("Email is required"))?
                .into(),
            date_of_birth: birth,
            invitation_token,
        };
        let provider_email = profile.email.clone();
        let verified = fields.get("emailVerified").and_then(FieldValue::as_bool) == Some(true);
        let person_id = signup::prepare_native_profile(tx, &mut profile, self.settings.as_ref())
            .await
            .map_err(signup_error)?;
        if verified && profile.email != provider_email {
            return Err(AuthError::validation(
                "The verified provider email must match the invitation",
            ));
        }
        let _ = fields.insert(
            "metadata".into(),
            FieldValue::from_json(serde_json::json!({}))?,
        );
        let _ = fields.insert("name".into(), profile.name.clone().into());
        let _ = fields.insert("person_id".into(), person_id.into());
        let _ = fields.insert("email".into(), profile.email.clone().into());
        let _ = fields.insert("status".into(), 1.into());
        let _ = fields.insert(
            "preferences".into(),
            FieldValue::from_json(
                signup::initial_preferences(self.settings.as_ref()).map_err(signup_error)?,
            )?,
        );
        request.set_server_context(TRANSACTION_BINDING, FieldFunction::bound(tx.clone()).into())?;
        request.set_server_context(PROFILE_BINDING, FieldFunction::bound(profile).into())?;
        super::native_mail::signup_created(request)?;
        Ok(DatabaseHookUpdate::Continue)
    }

    async fn before_create_account(
        &self,
        account: &mut CreateAccount,
        ctx: &SeaOrmHookContext<'_, AccountSchema>,
    ) -> AuthResult<HookControl> {
        let Some(request) = ctx.request.as_ref() else {
            return Ok(HookControl::Continue);
        };
        if let Some(transaction) = ctx.tx {
            request.request.set_server_context(
                TRANSACTION_BINDING,
                FieldFunction::bound(transaction.clone()).into(),
            )?;
        }
        let Some(FieldValue::Function(binding)) =
            request.request.server_context(PROFILE_BINDING)?
        else {
            return Ok(HookControl::Continue);
        };
        let profile = binding
            .binding::<AccountProfile>()
            .ok_or_else(|| AuthError::internal("Signup profile unavailable"))?;
        let tx = ctx
            .tx
            .ok_or_else(|| AuthError::internal("Signup transaction unavailable"))?;
        signup::complete_native_profile(
            tx,
            clinical_id(&account.user_id.display_string()?)?,
            profile,
            super::request::request_id().as_deref(),
            self.settings.as_ref(),
        )
        .await
        .map_err(signup_error)?;
        tx.execute_raw(statement(
            "UPDATE accounts SET status=2 WHERE id=$1 AND status=1 AND email_verified",
            [clinical_id(&account.user_id.display_string()?)?.into()],
        ))
        .await
        .map_err(database_error)?;
        request
            .request
            .set_server_context(PROFILE_BINDING, FieldValue::Undefined)?;
        Ok(HookControl::Continue)
    }

    async fn before_update_user(
        &self,
        field: &str,
        value: &FieldValue,
        _update: &mut FieldMap,
        ctx: &SeaOrmHookContext<'_, AccountSchema>,
    ) -> AuthResult<DatabaseHookUpdate<FieldMap>> {
        let selector = match field {
            "id" => statement(
                "SELECT id,status FROM accounts WHERE id=$1",
                [clinical_id(
                    &better_auth_core::SchemaValue::<String>::from_field(value.clone())
                        .display_string()?,
                )?
                .into()],
            ),
            "email" => statement(
                "SELECT id,status FROM accounts WHERE email=$1",
                [value.as_str().ok_or(AuthError::InvalidCredentials)?.into()],
            ),
            _ => return Err(AuthError::InvalidCredentials),
        };
        let row = if let Some(tx) = ctx.tx {
            tx.query_one_raw(selector).await
        } else {
            ctx.db.query_one_raw(selector).await
        }
        .map_err(database_error)?
        .ok_or(AuthError::Unauthenticated)?;
        let status: i32 = row.try_get("", "status").map_err(database_error)?;
        if !matches!(status, 1 | 2) {
            return Err(AuthError::Unauthenticated);
        }
        let account_id = row.try_get("", "id").map_err(database_error)?;
        if let Some(tx) = ctx.tx {
            crate::models::access::verify_account_enabled(tx, account_id).await
        } else {
            crate::models::access::verify_account_enabled(ctx.db, account_id).await
        }
        .map_err(super::policy::operation_error)?;
        Ok(DatabaseHookUpdate::Continue)
    }

    async fn before_create_session(
        &self,
        fields: &mut FieldMap,
        ctx: &SeaOrmHookContext<'_, AccountSchema>,
    ) -> AuthResult<DatabaseHookUpdate<FieldMap>> {
        let id = better_auth_core::SchemaValue::<String>::from_field(
            fields
                .get("userId")
                .ok_or_else(|| AuthError::internal("Session account unavailable"))?
                .clone(),
        )
        .display_string()?;
        let account_id = clinical_id(&id)?;
        let purpose = if let Some(tx) = ctx.tx {
            eligible(tx, account_id).await?
        } else {
            eligible_in(ctx.db, account_id).await?
        };
        let _ = fields.insert("clinical_session_purpose".into(), purpose.into());
        let path = ctx.request.as_ref().map(|request| request.request.path());
        let method = match path {
            Some("/callback/zitadel") => Some("zitadel"),
            Some("/passkey/verify-authentication") => Some("passkey"),
            Some("/sign-in/email") => Some("password"),
            _ => None,
        };
        if let Some(method) = method {
            let _ = fields.insert("authentication_method".into(), method.into());
        }
        Ok(DatabaseHookUpdate::Continue)
    }
}

async fn eligible_in(db: &DatabaseConnection, account_id: i64) -> AuthResult<&'static str> {
    let tx = db.begin().await.map_err(database_error)?;
    let purpose = eligible(&tx, account_id).await?;
    tx.commit().await.map_err(database_error)?;
    Ok(purpose)
}

async fn eligible(tx: &impl ConnectionTrait, account_id: i64) -> AuthResult<&'static str> {
    tx.execute_raw(statement(
        "SELECT set_config('med_tracker.current_account_id',$1,true)",
        [account_id.to_string().into()],
    ))
    .await
    .map_err(database_error)?;
    crate::models::access::verify_account_enabled(tx, account_id)
        .await
        .map_err(super::policy::operation_error)?;
    let row = tx.query_one_raw(statement(
        "SELECT a.status,(o.account_id IS NULL OR o.completed_at IS NOT NULL) AS ready FROM accounts a JOIN people p ON p.id=a.person_id LEFT JOIN identity_onboarding o ON o.account_id=a.id WHERE a.id=$1 AND a.status IN (1,2) AND p.date_of_birth <= (CURRENT_DATE - INTERVAL '18 years')::date AND NOT EXISTS(SELECT 1 FROM account_lockouts l WHERE l.account_id=a.id AND l.deadline>CURRENT_TIMESTAMP)",
        [account_id.into()],
    )).await.map_err(database_error)?.ok_or(AuthError::Unauthenticated)?;
    let status: i32 = row.try_get("", "status").map_err(database_error)?;
    let ready: bool = row.try_get("", "ready").map_err(database_error)?;
    Ok(if status == 2 && ready {
        "authenticated"
    } else {
        "enrolment"
    })
}

pub(super) fn signup_error(error: SignupError) -> AuthError {
    match error {
        SignupError::RegistrationClosed => AuthError::forbidden("Account registration is closed"),
        SignupError::Invalid(errors) => AuthError::validation(
            errors
                .into_iter()
                .map(|(field, errors)| format!("{field}: {}", errors.join(", ")))
                .collect::<Vec<_>>()
                .join("; "),
        ),
        SignupError::InvalidKey => AuthError::bad_request("Invitation is unavailable"),
        SignupError::Unavailable => AuthError::internal("Account setup unavailable"),
    }
}

pub(super) async fn complete_passwordless_profile(
    request: &better_auth_core::AuthRequest,
    user_id: &str,
    settings: Option<&serde_json::Value>,
) -> AuthResult<better_auth_seaorm::TransactionConnection> {
    let (transaction, profile) = super::passwordless_registration::registration_profile(request)?;
    signup::complete_native_profile(
        &transaction,
        clinical_id(user_id)?,
        &profile,
        super::request::request_id().as_deref(),
        settings,
    )
    .await
    .map_err(signup_error)?;
    request.set_server_context(PROFILE_BINDING, FieldValue::Undefined)?;
    Ok(transaction)
}
