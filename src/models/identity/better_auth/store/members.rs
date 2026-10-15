use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult, CreateMember, Member,
    store::{ListOrganizationMembersParams, MemberStore},
};
use sea_orm::{ConnectionTrait, FromQueryResult};

use super::{ClinicalStore, clinical_id, database_error, tenant::operation_error};
use crate::models::care::administration;

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
        if super::super::request::authenticated()?.active_household_id != Some(household_id) {
            return Err(AuthError::forbidden(
                "Selected household does not match request",
            ));
        }
        let tenant = self.tenant(household_id).await?;
        administration::authorize(&tenant)
            .await
            .map_err(operation_error)?;
        let field = match input.filter_field.as_deref() {
            None => None,
            Some("id") => Some("m.id"),
            Some("userId" | "user_id") => Some("a.account_id"),
            Some("role") => Some("m.role"),
            _ => return Err(AuthError::validation("Unsupported membership field")),
        };
        let operator = match input.filter_operator.as_deref().unwrap_or("eq") {
            "eq" => "=",
            "ne" => "<>",
            _ => return Err(AuthError::validation("Unsupported membership filter")),
        };
        let sort = match input.sort_by.as_deref().unwrap_or("id") {
            "id" => "m.id",
            "userId" | "user_id" => "a.account_id",
            "role" => "m.role",
            "createdAt" | "created_at" => "m.created_at",
            _ => return Err(AuthError::validation("Unsupported membership sort")),
        };
        let direction = match input.sort_direction.as_deref().unwrap_or("asc") {
            "asc" => "ASC",
            "desc" => "DESC",
            _ => return Err(AuthError::validation("Unsupported sort direction")),
        };
        let mut values: Vec<sea_orm::Value> = vec![household_id.into()];
        let mut condition = String::new();
        if let Some(field) = field {
            let value = input
                .filter_value
                .as_deref()
                .ok_or_else(|| AuthError::validation("Filter value is required"))?;
            values.push(if field == "m.role" {
                value.into()
            } else {
                clinical_id(value)?.into()
            });
            condition = format!(" AND {field} {operator} $2");
        }
        let source = format!(
            "FROM public.household_memberships m JOIN med_tracker.household_accounts($1) a ON a.person_id=m.person_id WHERE m.household_id=$1 AND m.status='active' AND m.revoked_at IS NULL{condition}"
        );
        let count = tenant
            .transaction()
            .query_one_raw(super::statement(
                &format!("SELECT count(*) AS count {source}"),
                values.clone(),
            ))
            .await
            .map_err(database_error)?
            .ok_or_else(|| AuthError::internal("Membership count missing"))?;
        let total: i64 = count.try_get("", "count").map_err(database_error)?;
        let offset_position = values.len() + 1;
        let limit_position = values.len() + 2;
        values.push(
            i64::try_from(input.offset.unwrap_or(0))
                .map_err(|_| AuthError::validation("Offset overflow"))?
                .into(),
        );
        values.push(
            i64::try_from(input.limit.unwrap_or(100).min(100))
                .map_err(|_| AuthError::validation("Limit overflow"))?
                .into(),
        );
        let rows = super::canonical::MemberRow::find_by_statement(super::statement(&format!("SELECT m.id,m.household_id,a.account_id,m.role,m.created_at AT TIME ZONE 'UTC' AS created_at {source} ORDER BY {sort} {direction},m.id OFFSET ${offset_position} LIMIT ${limit_position}"), values)).all(tenant.transaction()).await.map_err(database_error)?;
        tenant.commit().await.map_err(operation_error)?;
        Ok((
            rows.into_iter().map(Into::into).collect(),
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
        let (_, count) = self
            .query_organization_members(&ListOrganizationMembersParams {
                organization_id: org_id.to_owned(),
                limit: Some(0),
                filter_field: owners.then(|| "role".to_owned()),
                filter_value: owners.then(|| "owner".to_owned()),
                ..Default::default()
            })
            .await?;
        i64::try_from(count).map_err(|_| AuthError::internal("Membership count overflow"))
    }
}
