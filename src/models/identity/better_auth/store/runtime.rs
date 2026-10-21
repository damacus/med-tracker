use std::sync::Arc;

use better_auth_core::{
    AuthConfig, AuthError, AuthResult, FieldMap,
    plugin_runtime::ModelFields,
    store::{AuthStore, RuntimeStore, database_hooks::DatabaseHooks},
};

use super::{super::ClinicalAuthSchema, ClinicalStore};

impl ClinicalStore {
    pub(super) fn session_defaults(&self) -> AuthResult<FieldMap> {
        self.runtime.as_ref().map_or_else(
            || Ok(FieldMap::new()),
            |config| config.session_default_fields(),
        )
    }
}

impl RuntimeStore<ClinicalAuthSchema> for ClinicalStore {
    fn with_runtime(
        &self,
        config: Arc<AuthConfig>,
        hooks: Vec<Arc<dyn DatabaseHooks<ClinicalAuthSchema>>>,
        _model_fields: ModelFields,
    ) -> AuthResult<Arc<dyn AuthStore<ClinicalAuthSchema>>> {
        if !hooks.is_empty() {
            return Err(AuthError::config(
                "Clinical database hooks are not configured",
            ));
        }
        let mut bound = self.clone();
        bound.runtime = Some(config);
        Ok(Arc::new(bound))
    }
}
