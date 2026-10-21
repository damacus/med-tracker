use async_trait::async_trait;
use better_auth_core::{
    AuthError, AuthRecordFields, AuthResult, CreateOrganization, FieldMap, Organization,
    UpdateOrganization,
    store::{JoinValue, OrganizationStore},
};

use super::ClinicalStore;

#[async_trait]
impl OrganizationStore for ClinicalStore {
    async fn create_organization(&self, _input: CreateOrganization) -> AuthResult<Organization> {
        Err(AuthError::forbidden(
            "Households are created during account registration",
        ))
    }

    async fn get_organization_by_id(&self, _id: &str) -> AuthResult<Option<Organization>> {
        Err(AuthError::config(
            "Native organization lookup and mutation are not configured",
        ))
    }

    async fn get_organization_by_slug(&self, _slug: &str) -> AuthResult<Option<Organization>> {
        Err(AuthError::config(
            "Native organization lookup and mutation are not configured",
        ))
    }

    async fn list_organizations_by_ids(&self, _ids: &[String]) -> AuthResult<Vec<Organization>> {
        Err(AuthError::config(
            "Native organization lookup and mutation are not configured",
        ))
    }

    async fn update_organization(
        &self,
        _id: &str,
        _update: UpdateOrganization,
    ) -> AuthResult<Organization> {
        Err(AuthError::config(
            "Native organization lookup and mutation are not configured",
        ))
    }

    async fn delete_organization(&self, _id: &str) -> AuthResult<()> {
        Err(AuthError::forbidden("Household deletion is unavailable"))
    }

    async fn list_user_organizations(&self, user_id: &str) -> AuthResult<Vec<JoinValue<FieldMap>>> {
        self.canonical_user_organizations(user_id)
            .await?
            .into_iter()
            .map(|organization| Ok(JoinValue::One(Some(organization.field_values()?))))
            .collect()
    }
}
