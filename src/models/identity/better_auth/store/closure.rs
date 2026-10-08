use super::super::request;
use super::{ClinicalStore, clinical_id, context, database_error, statement};
use better_auth_core::{AuthError, AuthResult};
use sea_orm::ConnectionTrait;

impl ClinicalStore {
    pub(crate) async fn close_account(&self, id: &str) -> AuthResult<()> {
        let actor = request::authenticated()?;
        let account_id = clinical_id(id)?;
        if actor.account_id != account_id {
            return Err(AuthError::forbidden("Account access denied"));
        }
        let transaction = self.account_transaction().await?;
        let user = self
            .canonical_user_in(&transaction, id)
            .await?
            .ok_or(AuthError::Unauthenticated)?;
        let homes = transaction.query_all_raw(statement("SELECT household_id FROM public.household_memberships WHERE account_id=$1 ORDER BY household_id", [account_id.into()])).await.map_err(database_error)?;
        for home in homes {
            let household_id: i64 = home.try_get("", "household_id").map_err(database_error)?;
            context(
                &transaction,
                "med_tracker.current_household_id",
                &household_id.to_string(),
            )
            .await?;
            transaction
                .query_one_raw(statement(
                    "SELECT id FROM public.households WHERE id=$1 FOR UPDATE",
                    [household_id.into()],
                ))
                .await
                .map_err(database_error)?;
            let row = transaction.query_one_raw(statement("SELECT EXISTS(SELECT 1 FROM public.household_memberships m JOIN public.households h ON h.id=m.household_id WHERE m.household_id=$1 AND m.account_id=$2 AND m.role='owner' AND m.status='active' AND m.revoked_at IS NULL AND h.status='active' AND h.lifecycle_state='active') AND NOT EXISTS(SELECT 1 FROM public.household_memberships WHERE household_id=$1 AND account_id<>$2 AND role='owner' AND status='active' AND revoked_at IS NULL) AS sole_owner", [household_id.into(), account_id.into()])).await.map_err(database_error)?.ok_or_else(|| AuthError::internal("Household ownership unavailable"))?;
            if row
                .try_get::<bool>("", "sole_owner")
                .map_err(database_error)?
            {
                return Err(AuthError::forbidden(
                    "Transfer household ownership before closing your account",
                ));
            }
            transaction.execute_raw(statement("UPDATE public.users SET active=false,updated_at=timezone('UTC',clock_timestamp()) WHERE person_id IN(SELECT id FROM public.people WHERE account_id=$1 AND household_id=$2)", [account_id.into(), household_id.into()])).await.map_err(database_error)?;
            transaction.execute_raw(statement("UPDATE public.person_access_grants SET revoked_at=timezone('UTC',clock_timestamp()),updated_at=timezone('UTC',clock_timestamp()) WHERE household_id=$1 AND household_membership_id IN(SELECT id FROM public.household_memberships WHERE household_id=$1 AND account_id=$2) AND revoked_at IS NULL", [household_id.into(), account_id.into()])).await.map_err(database_error)?;
            transaction.execute_raw(statement("UPDATE public.household_memberships SET status='revoked',revoked_at=timezone('UTC',clock_timestamp()),permissions_version=permissions_version+1,updated_at=timezone('UTC',clock_timestamp()) WHERE household_id=$1 AND account_id=$2 AND revoked_at IS NULL", [household_id.into(), account_id.into()])).await.map_err(database_error)?;
        }
        self.audit(&transaction, account_id, "account", "closed")
            .await?;
        super::super::mail::notice(self, &user, "Account closed", "Your MedTracker account was closed. Your sign-in methods, sessions and personal API keys are no longer usable. Shared care records and audit history are preserved.").await?;
        transaction.execute_raw(statement("UPDATE public.api_sessions SET revoked_at=timezone('UTC',clock_timestamp()),updated_at=timezone('UTC',clock_timestamp()) WHERE account_id=$1 AND revoked_at IS NULL", [account_id.into()])).await.map_err(database_error)?;
        transaction.execute_raw(statement("UPDATE public.api_app_tokens SET revoked_at=timezone('UTC',clock_timestamp()),updated_at=timezone('UTC',clock_timestamp()) WHERE account_id=$1 AND revoked_at IS NULL", [account_id.into()])).await.map_err(database_error)?;
        transaction.execute_raw(statement("UPDATE public.oauth_grants SET revoked_at=timezone('UTC',clock_timestamp()) WHERE account_id=$1 AND revoked_at IS NULL", [account_id.into()])).await.map_err(database_error)?;
        transaction.execute_raw(statement("UPDATE public.identity_api_keys SET payload=jsonb_set(payload,'{enabled}','false'::jsonb) WHERE account_id=$1", [account_id.into()])).await.map_err(database_error)?;
        transaction
            .execute_raw(statement(
                "DELETE FROM public.account_active_session_keys WHERE account_id=$1",
                [account_id.into()],
            ))
            .await
            .map_err(database_error)?;
        transaction.execute_raw(statement("UPDATE public.identity_sessions SET active=false,updated_at=clock_timestamp() WHERE account_id=$1", [account_id.into()])).await.map_err(database_error)?;
        transaction.execute_raw(statement("UPDATE public.accounts SET status=3,updated_at=clock_timestamp() WHERE id=$1 AND status=2", [account_id.into()])).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)
    }
}
