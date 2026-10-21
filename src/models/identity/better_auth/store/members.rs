use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult, CreateMember, Member,
    store::{ListOrganizationMembersParams, MemberStore},
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect, QueryTrait};

use super::{
    ClinicalStore, clinical_id, clinical_schema_id, database_error, tenant::operation_error,
};
use crate::models::{
    care::administration,
    entities::{account, membership},
};

#[async_trait]
impl MemberStore for ClinicalStore {
    async fn create_member(&self, _input: CreateMember) -> AuthResult<Member> {
        Err(AuthError::forbidden(
            "Membership requires atomic invitation acceptance or account registration",
        ))
    }

    async fn get_member(&self, organization_id: &str, user_id: &str) -> AuthResult<Option<Member>> {
        self.canonical_member(organization_id, user_id).await
    }

    async fn get_member_by_id(&self, id: &str) -> AuthResult<Option<Member>> {
        self.canonical_member_by_id(id).await
    }

    async fn update_member_role(&self, member_id: &str, role: &str) -> AuthResult<Member> {
        let current = self
            .canonical_member_by_id(member_id)
            .await?
            .ok_or_else(|| AuthError::not_found("Membership not found"))?;
        self.change_canonical_member(
            clinical_schema_id(&current.organization_id)?,
            clinical_id(member_id)?,
            Some(role),
        )
        .await?
        .ok_or_else(|| AuthError::not_found("Membership not found"))
    }

    async fn delete_member(&self, member_id: &str) -> AuthResult<()> {
        let current = self
            .canonical_member_by_id(member_id)
            .await?
            .ok_or_else(|| AuthError::not_found("Membership not found"))?;
        self.change_canonical_member(
            clinical_schema_id(&current.organization_id)?,
            clinical_id(member_id)?,
            None,
        )
        .await?;
        Ok(())
    }

    async fn list_organization_members(&self, org_id: &str) -> AuthResult<Vec<Member>> {
        self.canonical_members(org_id).await
    }

    async fn query_organization_members(
        &self,
        _input: &ListOrganizationMembersParams,
    ) -> AuthResult<(Vec<Member>, usize)> {
        Err(AuthError::config(
            "Native organization member queries are not configured",
        ))
    }

    async fn count_organization_members(&self, org_id: &str) -> AuthResult<i64> {
        self.count_members(org_id, false).await
    }

    async fn count_organization_owners(&self, org_id: &str) -> AuthResult<i64> {
        self.count_members(org_id, true).await
    }
}

impl ClinicalStore {
    async fn count_members(&self, org_id: &str, owners: bool) -> AuthResult<i64> {
        let household_id = clinical_id(org_id)?;
        let tenant = self.tenant(household_id).await?;
        administration::authorize(&tenant)
            .await
            .map_err(operation_error)?;
        let mut query = membership::Entity::find()
            .filter(
                membership::Column::PersonId.in_subquery(
                    account::Entity::find()
                        .select_only()
                        .column(account::Column::PersonId)
                        .into_query(),
                ),
            )
            .filter(membership::Column::HouseholdId.eq(household_id))
            .filter(membership::Column::Status.eq("active"))
            .filter(membership::Column::RevokedAt.is_null());
        if owners {
            query = query.filter(membership::Column::Role.eq("owner"));
        }
        let count = query
            .count(tenant.transaction())
            .await
            .map_err(database_error)?;
        tenant.commit().await.map_err(operation_error)?;
        i64::try_from(count).map_err(|_| AuthError::internal("Membership count overflow"))
    }
}
