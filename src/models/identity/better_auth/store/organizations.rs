use async_trait::async_trait;
use better_auth_core::{AuthError, AuthResult, CreateOrganization, Organization, UpdateOrganization, store::OrganizationStore};

use super::ClinicalStore;

#[async_trait]
impl OrganizationStore for ClinicalStore {
    async fn create_organization(&self, _input: CreateOrganization) -> AuthResult<Organization> {
        Err(AuthError::forbidden("Households are created during account registration"))
    }

    async fn get_organization_by_id(&self, id: &str) -> AuthResult<Option<Organization>> {
        self.canonical_organization(id).await
    }

    async fn get_organization_by_slug(&self, slug: &str) -> AuthResult<Option<Organization>> {
        self.canonical_organization_by_slug(slug).await
    }

    async fn list_organizations_by_ids(&self, ids: &[String]) -> AuthResult<Vec<Organization>> {
        self.canonical_organizations_by_ids(ids).await
    }

    async fn update_organization(&self, id: &str, update: UpdateOrganization) -> AuthResult<Organization> {
        if update.slug.is_some() || update.logo.is_some() || update.metadata.is_some() {
            return Err(AuthError::validation("Only the household name can be changed here"));
        }
        match update.name {
            Some(name) => self.change_canonical_organization(id, &name).await,
            None => self.canonical_organization(id).await?
                .ok_or_else(|| AuthError::not_found("Household not found")),
        }
    }

    async fn delete_organization(&self, _id: &str) -> AuthResult<()> {
        Err(AuthError::forbidden("Household deletion is unavailable"))
    }

    async fn list_user_organizations(&self, user_id: &str) -> AuthResult<Vec<Organization>> {
        self.canonical_user_organizations(user_id).await
    }
}
