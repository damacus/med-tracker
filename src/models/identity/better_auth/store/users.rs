use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult, CreateUser, FieldValue, ListUsersParams, UpdateUser,
    store::{AccountStore, JoinValue, UserAccounts, UserStore},
    wire::UserView,
};
use sea_orm::{ConnectionTrait, DatabaseTransaction};

use super::super::{ClinicalAuthSchema, request};
use super::{ClinicalStore, clinical_id, context, database_error, statement};

impl ClinicalStore {
    async fn update_user_in(
        &self,
        transaction: &DatabaseTransaction,
        id: &str,
        input: UpdateUser,
    ) -> AuthResult<UserView> {
        if input
            .email
            .as_deref()
            .is_some_and(|email| !request::email_change_allowed(id, email))
            || !input.name.is_undefined()
            || !input.image.is_undefined()
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
        context(transaction, "med_tracker.current_account_id", id).await?;
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
            self.audit(transaction, account_id, "email_verification", "verified")
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
            transaction.execute_raw(statement("UPDATE public.users SET email_address=$2,updated_at=clock_timestamp() WHERE person_id IN(SELECT person_id FROM public.accounts WHERE id=$1)", [account_id.into(), email.into()])).await.map_err(database_error)?;
        }
        self.canonical_user_in(transaction, id)
            .await?
            .ok_or(AuthError::UserNotFound)
    }
}

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

    async fn list_users_by_ids(&self, _ids: &[String], _limit: f64) -> AuthResult<Vec<UserView>> {
        Err(AuthError::config(
            "Native organization user batches are not configured",
        ))
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

    async fn get_user_with_accounts(&self, email: &str) -> AuthResult<Option<UserAccounts>> {
        let Some(user) = self.get_user_by_email(email).await? else {
            return Ok(None);
        };
        let accounts =
            AccountStore::<ClinicalAuthSchema>::get_user_accounts(self, user.id.typed()?).await?;
        Ok(Some(UserAccounts::new(user, JoinValue::Many(accounts))))
    }

    async fn get_user_by_username(&self, _username: &str) -> AuthResult<Option<UserView>> {
        Ok(None)
    }

    async fn get_user_by_phone_number(&self, _phone_number: &str) -> AuthResult<Option<UserView>> {
        Err(AuthError::forbidden("Phone sign-in is unavailable"))
    }

    async fn update_user_optional(
        &self,
        id: &str,
        input: UpdateUser,
    ) -> AuthResult<Option<UserView>> {
        match self.update_user(id, input).await {
            Ok(user) => Ok(Some(user)),
            Err(AuthError::UserNotFound) => Ok(None),
            Err(error) => Err(error),
        }
    }

    async fn update_user(&self, id: &str, input: UpdateUser) -> AuthResult<UserView> {
        let transaction = self.transaction().await?;
        let user = self.update_user_in(&transaction, id, input).await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(user)
    }

    async fn update_user_by_field_value(
        &self,
        field: &str,
        value: &FieldValue,
        update: UpdateUser,
    ) -> AuthResult<Option<UserView>> {
        if field != "id" && field != "email" {
            return Err(AuthError::config("Unsupported user field selector"));
        }
        let selector = value
            .as_str()
            .ok_or_else(|| AuthError::validation("User selector must be a native string"))?;
        if field == "id" {
            return self.update_user_optional(selector, update).await;
        }
        let transaction = self.transaction().await?;
        let row = transaction.query_one_raw(statement("SELECT id FROM public.accounts WHERE lower(email::text)=lower($1) AND status IN(1,2) ORDER BY id LIMIT 1 FOR UPDATE", [selector.trim().into()])).await.map_err(database_error)?;
        let Some(row) = row else {
            transaction.commit().await.map_err(database_error)?;
            return Ok(None);
        };
        let account_id: i64 = row.try_get("", "id").map_err(database_error)?;
        let user = self
            .update_user_in(&transaction, &account_id.to_string(), update)
            .await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(Some(user))
    }

    async fn delete_user(&self, _id: &str) -> AuthResult<()> {
        Err(AuthError::forbidden(
            "Use the verified account closure operation",
        ))
    }

    async fn verify_user_and_revoke_unproven_access(
        &self,
        _user_id: &str,
    ) -> AuthResult<Option<UserView>> {
        Err(AuthError::config(
            "Magic link and email OTP authentication are not configured",
        ))
    }

    async fn create_user_fields_optional(
        &self,
        _fields: better_auth_core::FieldMap,
    ) -> AuthResult<Option<UserView>> {
        Err(AuthError::config(
            "Native user registration is not configured",
        ))
    }

    async fn list_users(&self, _input: ListUsersParams) -> AuthResult<(Vec<UserView>, usize)> {
        Err(AuthError::forbidden(
            "Global account administration is not enabled",
        ))
    }
}
