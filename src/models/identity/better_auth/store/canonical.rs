use better_auth_core::{AuthError, AuthResult, Member, Organization, wire::UserView};
use chrono::{DateTime, Utc};
use sea_orm::FromQueryResult;
use serde_json::json;

use super::{ClinicalStore, clinical_id, context, database_error, statement};
use super::tenant::operation_error;
use crate::models::care::administration;
use super::super::request;

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
        Self { id:row.id.to_string(),name:row.name,email:Some(row.email),email_verified:row.status==2,image:None,created_at:row.created_at,updated_at:row.updated_at,username:None,display_username:None,two_factor_enabled:row.two_factor_enabled,role:None,banned:row.status!=2,ban_reason:None,ban_expires:None,metadata:json!({"passkey_user_handle":row.passkey_user_handle}) }
    }
}

#[derive(FromQueryResult)]
struct OrganizationRow {
    id: i64,
    name: String,
    slug: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<OrganizationRow> for Organization {
    fn from(row: OrganizationRow) -> Self {
        Self { id:row.id.to_string(),name:row.name,slug:row.slug,logo:None,metadata:None,created_at:row.created_at,updated_at:row.updated_at }
    }
}

#[derive(FromQueryResult)]
struct MemberRow {
    id: i64,
    household_id: i64,
    account_id: i64,
    role: String,
    created_at: DateTime<Utc>,
}

impl From<MemberRow> for Member {
    fn from(row:MemberRow)->Self {
        Self { id:row.id.to_string(),organization_id:row.household_id.to_string(),user_id:row.account_id.to_string(),role:row.role,created_at:row.created_at }
    }
}

impl ClinicalStore {
    pub(super) async fn canonical_users_by_ids(&self, ids: &[String]) -> AuthResult<Vec<UserView>> {
        let ids: Vec<i64> = ids.iter().map(|id| clinical_id(id)).collect::<AuthResult<_>>()?;
        if ids.is_empty() { return Ok(Vec::new()); }
        let actor = request::authenticated()?;
        let household_id = actor.active_household_id.ok_or_else(|| AuthError::bad_request("Choose a household"))?;
        let tenant = self.tenant(household_id).await?;
        administration::authorize(&tenant).await.map_err(operation_error)?;
        let rows = UserRow::find_by_statement(statement("SELECT a.id,a.email::text,a.status,a.created_at AT TIME ZONE 'UTC' AS created_at,a.updated_at AT TIME ZONE 'UTC' AS updated_at,p.name,false AS two_factor_enabled,NULL::text AS passkey_user_handle FROM public.accounts a JOIN public.household_memberships m ON m.account_id=a.id LEFT JOIN public.people p ON p.id=m.person_id AND p.household_id=m.household_id WHERE m.household_id=$1 AND a.id=ANY($2::bigint[]) AND m.status='active' AND m.revoked_at IS NULL ORDER BY a.id", [household_id.into(), ids.into()])).all(tenant.transaction()).await.map_err(database_error)?;
        tenant.commit().await.map_err(operation_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub(super) async fn canonical_user_organizations(&self, user_id: &str) -> AuthResult<Vec<Organization>> {
        let account_id = clinical_id(user_id)?;
        if request::authenticated()?.account_id != account_id { return Err(AuthError::forbidden("Account access denied")); }
        let transaction = self.account_transaction().await?;
        let rows = OrganizationRow::find_by_statement(statement("SELECT h.id,h.name,h.slug,h.created_at AT TIME ZONE 'UTC' AS created_at,h.updated_at AT TIME ZONE 'UTC' AS updated_at FROM public.households h JOIN public.household_memberships m ON m.household_id=h.id WHERE m.account_id=$1 AND m.status='active' AND m.revoked_at IS NULL AND h.status='active' AND h.lifecycle_state='active' ORDER BY h.name,h.id", [account_id.into()])).all(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub(super) async fn canonical_organizations_by_ids(&self, ids: &[String]) -> AuthResult<Vec<Organization>> {
        let ids: Vec<i64> = ids.iter().map(|id| clinical_id(id)).collect::<AuthResult<_>>()?;
        if ids.is_empty() { return Ok(Vec::new()); }
        let actor = request::authenticated()?;
        let transaction = self.account_transaction().await?;
        let rows = OrganizationRow::find_by_statement(statement("SELECT h.id,h.name,h.slug,h.created_at AT TIME ZONE 'UTC' AS created_at,h.updated_at AT TIME ZONE 'UTC' AS updated_at FROM public.households h JOIN public.household_memberships m ON m.household_id=h.id WHERE m.account_id=$1 AND h.id=ANY($2::bigint[]) AND m.status='active' AND m.revoked_at IS NULL AND h.status='active' AND h.lifecycle_state='active' ORDER BY h.id", [actor.account_id.into(), ids.into()])).all(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub(super) async fn canonical_organization_by_slug(&self, slug: &str) -> AuthResult<Option<Organization>> {
        let actor = request::authenticated()?;
        let transaction = self.account_transaction().await?;
        let row = OrganizationRow::find_by_statement(statement("SELECT h.id,h.name,h.slug,h.created_at AT TIME ZONE 'UTC' AS created_at,h.updated_at AT TIME ZONE 'UTC' AS updated_at FROM public.households h JOIN public.household_memberships m ON m.household_id=h.id WHERE m.account_id=$1 AND h.slug=$2 AND m.status='active' AND m.revoked_at IS NULL AND h.status='active' AND h.lifecycle_state='active'", [actor.account_id.into(), slug.into()])).one(&*transaction).await.map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        Ok(row.map(Into::into))
    }

    pub(super) async fn canonical_members(&self, household_id: &str) -> AuthResult<Vec<Member>> {
        let household_id = clinical_id(household_id)?;
        let tenant = self.tenant(household_id).await?;
        administration::authorize(&tenant).await.map_err(operation_error)?;
        let rows = MemberRow::find_by_statement(statement("SELECT id,household_id,account_id,role,created_at AT TIME ZONE 'UTC' AS created_at FROM public.household_memberships WHERE household_id=$1 AND status='active' AND revoked_at IS NULL ORDER BY id", [household_id.into()])).all(tenant.transaction()).await.map_err(database_error)?;
        tenant.commit().await.map_err(operation_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub(super) async fn canonical_member_by_id(&self, member_id: &str) -> AuthResult<Option<Member>> {
        let member_id = clinical_id(member_id)?;
        let household_id = request::authenticated()?.active_household_id.ok_or_else(|| AuthError::bad_request("Choose a household"))?;
        let tenant = self.tenant(household_id).await?;
        let row = MemberRow::find_by_statement(statement("SELECT id,household_id,account_id,role,created_at AT TIME ZONE 'UTC' AS created_at FROM public.household_memberships WHERE household_id=$1 AND id=$2 AND status='active' AND revoked_at IS NULL", [household_id.into(), member_id.into()])).one(tenant.transaction()).await.map_err(database_error)?;
        tenant.commit().await.map_err(operation_error)?;
        Ok(row.map(Into::into))
    }

    pub(super) async fn canonical_user(&self,id:&str)->AuthResult<Option<UserView>> {
        let transaction=self.transaction().await?;
        let user = self.canonical_user_in(&*transaction, id).await?;
        transaction.commit().await.map_err(database_error)?;
        Ok(user)
    }

    pub(crate) async fn canonical_user_in(&self, transaction: &sea_orm::DatabaseTransaction, id: &str) -> AuthResult<Option<UserView>> {
        let account_id=clinical_id(id)?;
        context(&*transaction,"med_tracker.current_account_id",id).await?;
        let row=UserRow::find_by_statement(statement("SELECT a.id,a.email::text,a.status,a.created_at AT TIME ZONE 'UTC' AS created_at,a.updated_at AT TIME ZONE 'UTC' AS updated_at,(SELECT p.name FROM public.people p WHERE p.account_id=a.id ORDER BY p.id LIMIT 1) AS name,EXISTS(SELECT 1 FROM public.identity_two_factors o WHERE o.account_id=a.id AND o.verified) AS two_factor_enabled,(SELECT passkey_user_handle::text FROM public.identity_onboarding WHERE account_id=a.id) AS passkey_user_handle FROM public.accounts a WHERE a.id=$1",[account_id.into()])).one(transaction).await.map_err(database_error)?;
        Ok(row.map(Into::into))
    }

    pub(super) async fn canonical_organization(&self,id:&str)->AuthResult<Option<Organization>> {
        let household_id=clinical_id(id)?;
        let tenant=self.tenant(household_id).await?;
        let row=OrganizationRow::find_by_statement(statement("SELECT id,name,slug,created_at AT TIME ZONE 'UTC' AS created_at,updated_at AT TIME ZONE 'UTC' AS updated_at FROM public.households WHERE id=$1 AND status='active' AND lifecycle_state='active'",[household_id.into()])).one(tenant.transaction()).await.map_err(database_error)?;
        tenant.commit().await.map_err(operation_error)?;
        Ok(row.map(Into::into))
    }

    pub(super) async fn canonical_member(&self,household_id:&str,user_id:&str)->AuthResult<Option<Member>> {
        let household=clinical_id(household_id)?;
        let account=clinical_id(user_id)?;
        let tenant=self.tenant(household).await?;
        let row=MemberRow::find_by_statement(statement("SELECT id,household_id,account_id,role,created_at AT TIME ZONE 'UTC' AS created_at FROM public.household_memberships WHERE household_id=$1 AND account_id=$2 AND status='active' AND revoked_at IS NULL",[household.into(),account.into()])).one(tenant.transaction()).await.map_err(database_error)?;
        tenant.commit().await.map_err(operation_error)?;
        Ok(row.map(Into::into))
    }

    pub(super) async fn change_canonical_member(&self,household_id:i64,member_id:i64,role:Option<&str>)->AuthResult<Option<Member>> {
        let tenant=self.tenant(household_id).await?;
        let attributes=role.map_or_else(||json!({}),|role|json!({"role":role}));
        administration::memberships::change(&tenant,member_id,attributes,role.is_none(),None).await.map_err(operation_error)?;
        let member=MemberRow::find_by_statement(statement("SELECT id,household_id,account_id,role,created_at AT TIME ZONE 'UTC' AS created_at FROM public.household_memberships WHERE id=$1 AND household_id=$2 AND status='active' AND revoked_at IS NULL",[member_id.into(),household_id.into()])).one(tenant.transaction()).await.map_err(database_error)?.map(Into::into);
        tenant.commit().await.map_err(operation_error)?;
        Ok(member)
    }

    pub(super) async fn change_canonical_organization(&self,id:&str,name:&str)->AuthResult<Organization> {
        let household_id=clinical_id(id)?;
        let tenant=self.tenant(household_id).await?;
        administration::settings::update(&tenant,json!({"name":name}),None).await.map_err(operation_error)?;
        let row=OrganizationRow::find_by_statement(statement("SELECT id,name,slug,created_at AT TIME ZONE 'UTC' AS created_at,updated_at AT TIME ZONE 'UTC' AS updated_at FROM public.households WHERE id=$1",[household_id.into()])).one(tenant.transaction()).await.map_err(database_error)?.ok_or_else(||AuthError::not_found("Household not found"))?;
        tenant.commit().await.map_err(operation_error)?;
        Ok(row.into())
    }
}
