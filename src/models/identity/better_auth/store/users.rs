use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult, CreateUser, ListUsersParams, UpdateUser, store::UserStore,
    wire::UserView,
};
use sea_orm::ConnectionTrait;

use super::super::{ClinicalAuthSchema, request};
use super::{ClinicalStore, clinical_id, context, database_error, statement};

#[async_trait]
impl UserStore<ClinicalAuthSchema> for ClinicalStore {
    async fn create_user(&self, input: CreateUser) -> AuthResult<UserView> {
        let transaction = self.transaction().await?;
        let user = self.create_user_in(&transaction, input).await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(user)
    }

    async fn get_user_by_id(&self, id: &str) -> AuthResult<Option<UserView>> {
        self.canonical_user(id).await
    }

    async fn list_users_by_ids(&self, ids: &[String]) -> AuthResult<Vec<UserView>> {
        self.canonical_users_by_ids(ids).await
    }

    async fn get_user_by_email(&self, email: &str) -> AuthResult<Option<UserView>> {
        let transaction = self.transaction().await?;
        let row = transaction.query_one_raw(statement("SELECT id FROM public.accounts WHERE lower(email::text)=lower($1) AND status IN(1,2) ORDER BY id LIMIT 1 FOR UPDATE", [email.trim().into()])).await.map_err(database_error)?;
        let user = match row {
            Some(row) => {
                let id: i64 = row.try_get("", "id").map_err(database_error)?;
                self.canonical_user_in(&transaction, &id.to_string())
                    .await?
            }
            None => None,
        };
        transaction.commit().await.map_err(database_error)?;
        Ok(user)
    }

    async fn get_user_by_username(&self, _username: &str) -> AuthResult<Option<UserView>> {
        Ok(None)
    }

    async fn update_user(&self, id: &str, input: UpdateUser) -> AuthResult<UserView> {
        if input
            .email
            .as_deref()
            .is_some_and(|email| !request::email_change_allowed(id, email))
            || input.name.is_some()
            || input.image.is_some()
            || input.username.is_some()
            || input.display_username.is_some()
            || input.role.is_some()
            || input.banned.is_some()
            || input.ban_reason.is_some()
            || input.ban_expires.is_some()
            || input.metadata.is_some()
        {
            return Err(AuthError::validation(
                "Use the canonical account or person settings to change profile fields",
            ));
        }
        let account_id = clinical_id(id)?;
        let transaction = self.transaction().await?;
        context(&transaction, "med_tracker.current_account_id", id).await?;
        let current = transaction
            .query_one_raw(statement(
                "SELECT status FROM public.accounts WHERE id=$1 FOR UPDATE",
                [account_id.into()],
            ))
            .await
            .map_err(database_error)?
            .ok_or(AuthError::UserNotFound)?;
        let status: i32 = current.try_get("", "status").map_err(database_error)?;
        if status == 1 && input.email_verified == Some(true) {
            transaction.execute_raw(statement("UPDATE public.accounts SET status=2,updated_at=CURRENT_TIMESTAMP WHERE id=$1 AND status=1", [account_id.into()])).await.map_err(database_error)?;
            self.audit(&transaction, account_id, "email_verification", "verified")
                .await?;
        } else if status != 2 {
            return Err(AuthError::Unauthenticated);
        }
        if input.email_verified == Some(false) {
            return Err(AuthError::forbidden(
                "Verified email cannot be cleared through profile update",
            ));
        }
        if let Some(email) = input.email {
            transaction.execute_raw(statement("UPDATE public.accounts SET email=$2,updated_at=clock_timestamp() WHERE id=$1 AND status=2", [account_id.into(), email.clone().into()])).await.map_err(database_error)?;
            transaction.execute_raw(statement("UPDATE public.users SET email_address=$2,updated_at=clock_timestamp() WHERE person_id IN(SELECT id FROM public.people WHERE account_id=$1)", [account_id.into(), email.into()])).await.map_err(database_error)?;
        }
        if let Some(enabled) = input.two_factor_enabled {
            let row = transaction.query_one_raw(statement("SELECT EXISTS(SELECT 1 FROM public.identity_two_factors WHERE account_id=$1 AND verified) AS enabled", [account_id.into()])).await.map_err(database_error)?.ok_or_else(|| AuthError::internal("Factor state unavailable"))?;
            if row.try_get::<bool>("", "enabled").map_err(database_error)? != enabled {
                return Err(AuthError::forbidden(
                    "Factor state must be changed through its verified ceremony",
                ));
            }
            transaction.execute_raw(statement("UPDATE public.identity_onboarding SET legacy_totp_disabled_at=clock_timestamp() WHERE account_id=$1", [account_id.into()])).await.map_err(database_error)?;
        }
        let user = self
            .canonical_user_in(&transaction, id)
            .await?
            .ok_or(AuthError::UserNotFound)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(user)
    }

    async fn delete_user(&self, _id: &str) -> AuthResult<()> {
        Err(AuthError::forbidden(
            "Use the verified account closure operation",
        ))
    }

    async fn list_users(&self, _input: ListUsersParams) -> AuthResult<(Vec<UserView>, usize)> {
        Err(AuthError::forbidden(
            "Global account administration is not enabled",
        ))
    }
}
