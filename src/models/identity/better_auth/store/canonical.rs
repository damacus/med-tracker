use better_auth_core::{AuthError, AuthResult, FieldMap, FieldValue, Organization, wire::UserView};
use chrono::{DateTime, Utc};
use sea_orm::FromQueryResult;

use super::super::request;
use super::{ClinicalStore, clinical_id, context, database_error, statement};

#[derive(FromQueryResult)]
struct UserRow {
    id: i64,
    email: String,
    status: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    name: Option<String>,
    two_factor_enabled: bool,
    passkey_user_handle: Option<String>,
}

impl From<UserRow> for UserView {
    fn from(row: UserRow) -> Self {
        Self {
            field_order: Vec::new(),
            additional_fields: FieldMap::default(),
            visible_fields: None,
            id: row.id.to_string().into(),
            name: row.name.into(),
            email: Some(row.email).into(),
            email_verified: (row.status == 2).into(),
            image: None.into(),
            created_at: row.created_at.into(),
            updated_at: row.updated_at.into(),
            is_anonymous: None.into(),
            phone_number: None.into(),
            phone_number_verified: None.into(),
            username: None.into(),
            display_username: None.into(),
            two_factor_enabled: Some(row.two_factor_enabled).into(),
            role: None.into(),
            banned: (row.status != 2).into(),
            ban_reason: None.into(),
            ban_expires: None.into(),
            metadata: FieldMap::from([(
                "passkey_user_handle".into(),
                row.passkey_user_handle
                    .map_or(FieldValue::Null, FieldValue::from),
            )])
            .into(),
        }
    }
}

#[derive(FromQueryResult)]
struct OrganizationRow {
    id: i64,
    name: String,
    slug: String,
    created_at: DateTime<Utc>,
}

impl From<OrganizationRow> for Organization {
    fn from(row: OrganizationRow) -> Self {
        Self {
            field_order: Vec::new(),
            additional_fields: FieldMap::default(),
            id: row.id.to_string().into(),
            name: row.name.into(),
            slug: row.slug.into(),
            logo: None.into(),
            metadata: None.into(),
            created_at: row.created_at.into(),
        }
    }
}

impl ClinicalStore {
    pub(super) async fn canonical_user_organizations(
        &self,
        user_id: &str,
    ) -> AuthResult<Vec<Organization>> {
        let account_id = clinical_id(user_id)?;
        if request::authenticated()?.account_id != account_id {
            return Err(AuthError::forbidden("Account access denied"));
        }
        let transaction = self.account_transaction().await?;
        let rows = OrganizationRow::find_by_statement(statement("SELECT h.id,h.name,h.slug,h.created_at AT TIME ZONE 'UTC' AS created_at,h.updated_at AT TIME ZONE 'UTC' AS updated_at FROM public.households h JOIN public.household_memberships m ON m.household_id=h.id JOIN public.accounts a ON a.person_id=m.person_id WHERE a.id=$1 AND m.status='active' AND m.revoked_at IS NULL AND h.status='active' AND h.lifecycle_state='active' ORDER BY h.name,h.id", [account_id.into()])).all(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub(super) async fn canonical_user(&self, id: &str) -> AuthResult<Option<UserView>> {
        let transaction = self.transaction().await?;
        let user = self.canonical_user_in(&transaction, id).await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(user)
    }

    pub(crate) async fn canonical_user_in(
        &self,
        transaction: &sea_orm::DatabaseTransaction,
        id: &str,
    ) -> AuthResult<Option<UserView>> {
        let account_id = clinical_id(id)?;
        context(transaction, "med_tracker.current_account_id", id).await?;
        let row=UserRow::find_by_statement(statement("SELECT a.id,a.email::text,a.status,a.created_at AT TIME ZONE 'UTC' AS created_at,a.updated_at AT TIME ZONE 'UTC' AS updated_at,a.name,EXISTS(SELECT 1 FROM public.identity_two_factors o WHERE o.account_id=a.id AND o.verified) AS two_factor_enabled,(SELECT passkey_user_handle::text FROM public.identity_onboarding WHERE account_id=a.id) AS passkey_user_handle FROM public.accounts a WHERE a.id=$1",[account_id.into()])).one(transaction).await.map_err(database_error)?;
        Ok(row.map(Into::into))
    }
}
