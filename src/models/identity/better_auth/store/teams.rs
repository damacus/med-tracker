use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthResult, CreateOrganizationRole, CreateTeam, OrganizationRole, Team, TeamMember,
    UpdateOrganizationRole, UpdateTeam,
    store::{OrganizationRoleKey, OrganizationRoleStore, TeamStore},
};

use super::ClinicalStore;

fn teams_disabled() -> AuthError {
    AuthError::config("Organization teams are not configured for MedTracker")
}

fn dynamic_roles_disabled() -> AuthError {
    AuthError::config("Dynamic organization roles are not configured for MedTracker")
}

#[async_trait]
impl TeamStore for ClinicalStore {
    async fn create_team(&self, _input: CreateTeam) -> AuthResult<Team> {
        Err(teams_disabled())
    }
    async fn get_team(&self, _id: &str) -> AuthResult<Option<Team>> {
        Err(teams_disabled())
    }
    async fn update_team(&self, _id: &str, _update: UpdateTeam) -> AuthResult<Team> {
        Err(teams_disabled())
    }
    async fn delete_team(&self, _id: &str) -> AuthResult<()> {
        Err(teams_disabled())
    }
    async fn list_organization_teams(&self, _organization_id: &str) -> AuthResult<Vec<Team>> {
        Err(teams_disabled())
    }
    async fn count_organization_teams(&self, _organization_id: &str) -> AuthResult<u64> {
        Err(teams_disabled())
    }
    async fn list_user_teams(&self, _user_id: &str) -> AuthResult<Vec<Team>> {
        Err(teams_disabled())
    }
    async fn get_team_member(
        &self,
        _team_id: &str,
        _user_id: &str,
    ) -> AuthResult<Option<TeamMember>> {
        Err(teams_disabled())
    }
    async fn list_team_members(&self, _team_id: &str) -> AuthResult<Vec<TeamMember>> {
        Err(teams_disabled())
    }
    async fn count_team_members(&self, _team_id: &str) -> AuthResult<u64> {
        Err(teams_disabled())
    }
    async fn add_team_member(
        &self,
        _team_id: &better_auth_core::SchemaValue<String>,
        _user_id: &str,
        _maximum: Option<usize>,
    ) -> AuthResult<Option<TeamMember>> {
        Err(teams_disabled())
    }
    async fn remove_team_member(&self, _team_id: &str, _user_id: &str) -> AuthResult<()> {
        Err(teams_disabled())
    }
}

#[async_trait]
impl OrganizationRoleStore for ClinicalStore {
    async fn create_organization_role(
        &self,
        _input: CreateOrganizationRole,
    ) -> AuthResult<OrganizationRole> {
        Err(dynamic_roles_disabled())
    }
    async fn get_organization_role(&self, _id: &str) -> AuthResult<Option<OrganizationRole>> {
        Err(dynamic_roles_disabled())
    }
    async fn find_organization_role(
        &self,
        _organization_id: &str,
        _key: OrganizationRoleKey<'_>,
    ) -> AuthResult<Option<OrganizationRole>> {
        Err(dynamic_roles_disabled())
    }
    async fn query_organization_roles(
        &self,
        _organization_id: &str,
        _names: &[String],
    ) -> AuthResult<Vec<OrganizationRole>> {
        Err(dynamic_roles_disabled())
    }
    async fn count_organization_roles(&self, _organization_id: &str) -> AuthResult<u64> {
        Err(dynamic_roles_disabled())
    }
    async fn list_organization_roles(
        &self,
        _organization_id: &str,
    ) -> AuthResult<Vec<OrganizationRole>> {
        Err(dynamic_roles_disabled())
    }
    async fn update_organization_role(
        &self,
        _id: &str,
        _update: UpdateOrganizationRole,
    ) -> AuthResult<OrganizationRole> {
        Err(dynamic_roles_disabled())
    }
    async fn delete_organization_role(&self, _id: &str) -> AuthResult<()> {
        Err(dynamic_roles_disabled())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unconfigured_organization_extensions_reject_reads_and_writes() {
        let store = ClinicalStore::new(sea_orm::DatabaseConnection::default());
        assert!(matches!(
            store.get_team("1").await,
            Err(AuthError::Config(_))
        ));
        assert!(matches!(
            store.delete_team("1").await,
            Err(AuthError::Config(_))
        ));
        assert!(matches!(
            store.list_user_teams("1").await,
            Err(AuthError::Config(_))
        ));
        assert!(matches!(
            store.get_organization_role("1").await,
            Err(AuthError::Config(_))
        ));
        assert!(matches!(
            store.delete_organization_role("1").await,
            Err(AuthError::Config(_))
        ));
        assert!(matches!(
            store.list_organization_roles("1").await,
            Err(AuthError::Config(_))
        ));
    }
}
