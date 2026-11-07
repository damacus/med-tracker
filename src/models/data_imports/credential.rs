use crate::models::{
    access::TenantTransaction,
    errors::OperationError,
    identity::{
        browser::BrowserPrincipal,
        credentials::CredentialProvenance,
        resource::{AuthenticationError, ValidatedPrincipal},
    },
    ids::AccountId,
};

pub enum ImportPrincipal<'a> {
    Api(&'a ValidatedPrincipal),
    Browser(&'a BrowserPrincipal),
}

impl ImportPrincipal<'_> {
    pub(crate) fn provenance(&self) -> &CredentialProvenance {
        match self {
            Self::Api(principal) => principal.provenance(),
            Self::Browser(principal) => principal.provenance(),
        }
    }

    pub async fn revalidate(&self, tenant: &TenantTransaction) -> Result<(), OperationError> {
        if !tenant
            .holds_exclusive_account(AccountId::from_database(tenant.scope().actor.account_id))
        {
            return Err(OperationError::Forbidden);
        }
        let result = match self {
            Self::Api(principal) => principal.revalidate_household_locked(tenant).await,
            Self::Browser(principal) => principal.revalidate_household_locked(tenant).await,
        };
        result.map_err(|error| match error {
            AuthenticationError::Unauthenticated => OperationError::Unauthenticated,
            AuthenticationError::Forbidden
            | AuthenticationError::NotFound
            | AuthenticationError::InsufficientScope { .. } => OperationError::Forbidden,
            AuthenticationError::Unavailable => OperationError::Unavailable,
        })
    }
}
