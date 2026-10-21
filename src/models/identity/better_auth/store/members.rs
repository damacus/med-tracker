use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult, CreateMember, Member,
    store::{ListOrganizationMembersParams, MemberStore},
};

use super::ClinicalStore;

#[async_trait]
impl MemberStore for ClinicalStore {
    async fn create_member(&self, _input: CreateMember) -> AuthResult<Member> {
        Err(AuthError::config(
            "Native organization membership is not configured",
        ))
    }

    async fn get_member(
        &self,
        _organization_id: &str,
        _user_id: &str,
    ) -> AuthResult<Option<Member>> {
        Err(AuthError::config(
            "Native organization membership is not configured",
        ))
    }

    async fn get_member_by_id(&self, _id: &str) -> AuthResult<Option<Member>> {
        Err(AuthError::config(
            "Native organization membership is not configured",
        ))
    }

    async fn update_member_role(&self, _member_id: &str, _role: &str) -> AuthResult<Member> {
        Err(AuthError::config(
            "Native organization membership is not configured",
        ))
    }

    async fn delete_member(&self, _member_id: &str) -> AuthResult<()> {
        Err(AuthError::config(
            "Native organization membership is not configured",
        ))
    }

    async fn list_organization_members(&self, _org_id: &str) -> AuthResult<Vec<Member>> {
        Err(AuthError::config(
            "Native organization membership is not configured",
        ))
    }

    async fn query_organization_members(
        &self,
        _input: &ListOrganizationMembersParams,
    ) -> AuthResult<(Vec<Member>, usize)> {
        Err(AuthError::config(
            "Native organization membership is not configured",
        ))
    }

    async fn count_organization_members(&self, _org_id: &str) -> AuthResult<i64> {
        Err(AuthError::config(
            "Native organization membership is not configured",
        ))
    }

    async fn count_organization_owners(&self, _org_id: &str) -> AuthResult<i64> {
        Err(AuthError::config(
            "Native organization membership is not configured",
        ))
    }
}
