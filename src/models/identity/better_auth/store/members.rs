use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult, CreateMember, Member,
    store::{ListOrganizationMembersParams, MemberStore},
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};

use super::{ClinicalStore, clinical_id, database_error, tenant::operation_error};
use crate::models::{care::administration, entities::membership};

fn member(row: membership::Model) -> Member {
    Member {
        id: row.id.to_string(),
        organization_id: row.household_id.to_string(),
        user_id: row.account_id.to_string(),
        role: row.role,
        created_at: row.created_at.and_utc(),
    }
}

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
            clinical_id(&current.organization_id)?,
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
            clinical_id(&current.organization_id)?,
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
        input: &ListOrganizationMembersParams,
    ) -> AuthResult<(Vec<Member>, usize)> {
        let household_id = clinical_id(&input.organization_id)?;
        let tenant = self.tenant(household_id).await?;
        administration::authorize(&tenant)
            .await
            .map_err(operation_error)?;
        let mut query = membership::Entity::find()
            .filter(membership::Column::HouseholdId.eq(household_id))
            .filter(membership::Column::Status.eq("active"))
            .filter(membership::Column::RevokedAt.is_null());
        if let Some(field) = input.filter_field.as_deref() {
            let value = input
                .filter_value
                .as_deref()
                .ok_or_else(|| AuthError::validation("Filter value is required"))?;
            let equal = match input.filter_operator.as_deref().unwrap_or("eq") {
                "eq" => true,
                "ne" => false,
                _ => return Err(AuthError::validation("Unsupported membership filter")),
            };
            let condition = match (field, equal) {
                ("id", true) => membership::Column::Id.eq(clinical_id(value)?),
                ("id", false) => membership::Column::Id.ne(clinical_id(value)?),
                ("userId" | "user_id", true) => {
                    membership::Column::AccountId.eq(clinical_id(value)?)
                }
                ("userId" | "user_id", false) => {
                    membership::Column::AccountId.ne(clinical_id(value)?)
                }
                ("role", true) => membership::Column::Role.eq(value),
                ("role", false) => membership::Column::Role.ne(value),
                _ => return Err(AuthError::validation("Unsupported membership field")),
            };
            query = query.filter(condition);
        }
        let total = query
            .clone()
            .count(tenant.transaction())
            .await
            .map_err(database_error)?;
        let sort = match input.sort_by.as_deref().unwrap_or("id") {
            "id" => membership::Column::Id,
            "userId" | "user_id" => membership::Column::AccountId,
            "role" => membership::Column::Role,
            "createdAt" | "created_at" => membership::Column::CreatedAt,
            _ => return Err(AuthError::validation("Unsupported membership sort")),
        };
        query = match input.sort_direction.as_deref().unwrap_or("asc") {
            "asc" => query.order_by_asc(sort),
            "desc" => query.order_by_desc(sort),
            _ => return Err(AuthError::validation("Unsupported sort direction")),
        };
        let rows = query
            .order_by_asc(membership::Column::Id)
            .offset(input.offset.unwrap_or(0) as u64)
            .limit(input.limit.unwrap_or(100).min(100) as u64)
            .all(tenant.transaction())
            .await
            .map_err(database_error)?;
        tenant.commit().await.map_err(operation_error)?;
        Ok((
            rows.into_iter().map(member).collect(),
            usize::try_from(total).map_err(|_| AuthError::internal("Membership count overflow"))?,
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
