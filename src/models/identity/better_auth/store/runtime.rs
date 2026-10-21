use std::sync::Arc;

use better_auth_core::{
    AuthConfig, AuthError, AuthResult, FieldMap,
    plugin_runtime::ModelFields,
    store::{AuthStore, RuntimeStore, UserAccounts, database_hooks::DatabaseHooks},
};

use super::{super::ClinicalAuthSchema, ClinicalStore};

pub(super) struct RuntimeBindings {
    _config: Arc<AuthConfig>,
    _model_fields: ModelFields,
}

fn mapped_field(logical: &str, physical: Option<&str>) -> bool {
    physical.is_some_and(|name| !name.is_empty() && name != logical)
}

fn validate_schema_mappings(config: &AuthConfig, model_fields: &ModelFields) -> AuthResult<()> {
    for (role, model) in model_fields.active_models() {
        if model_fields.storage_model_name(role, model) != model {
            return Err(AuthError::config(format!(
                "Physical identity model mapping for {model} is not configured"
            )));
        }
        for (logical, field) in model_fields.fields(role).fields() {
            if mapped_field(logical, field.field_name.as_deref()) {
                return Err(AuthError::config(format!(
                    "Physical identity field mapping for {model}.{logical} is not configured"
                )));
            }
        }
    }
    for (model, fields) in [
        ("user", config.user.fields()),
        ("session", config.session.fields()),
        ("account", &config.account.additional_fields),
        ("verification", &config.verification.additional_fields),
    ] {
        for (logical, field) in fields {
            if mapped_field(logical, field.field_name.as_deref()) {
                return Err(AuthError::config(format!(
                    "Physical identity field mapping for {model}.{logical} is not configured"
                )));
            }
        }
    }
    Ok(())
}

impl ClinicalStore {
    pub(super) fn session_defaults(&self) -> AuthResult<FieldMap> {
        self.runtime.as_ref().map_or_else(
            || Ok(FieldMap::new()),
            |runtime| runtime._config.session_default_fields(),
        )
    }

    pub(super) fn validate_user_account_join(&self) -> AuthResult<()> {
        let Some(runtime) = self.runtime.as_ref() else {
            return Ok(());
        };
        let relation =
            UserAccounts::resolve_schema(&runtime._config, &runtime._model_fields, |_, _| false)?;
        if relation.logical_from != "id"
            || relation.logical_to != "userId"
            || relation.from != "id"
            || relation.to != "userId"
            || !relation.many
        {
            return Err(AuthError::config(
                "Clinical user account join is not configured",
            ));
        }
        Ok(())
    }

    fn bind_runtime(
        &self,
        config: Arc<AuthConfig>,
        hooks: Vec<Arc<dyn DatabaseHooks<ClinicalAuthSchema>>>,
        model_fields: ModelFields,
    ) -> AuthResult<Self> {
        if !hooks.is_empty() {
            return Err(AuthError::config(
                "Clinical database hooks are not configured",
            ));
        }
        if model_fields.custom_models().next().is_some() {
            return Err(AuthError::config(
                "Custom identity models are not configured",
            ));
        }
        validate_schema_mappings(&config, &model_fields)?;
        let mut bound = self.clone();
        bound.runtime = Some(Arc::new(RuntimeBindings {
            _config: config,
            _model_fields: model_fields,
        }));
        Ok(bound)
    }
}

impl RuntimeStore<ClinicalAuthSchema> for ClinicalStore {
    fn adapter_id(&self) -> &'static str {
        "medtracker-postgres"
    }

    fn with_runtime(
        &self,
        config: Arc<AuthConfig>,
        hooks: Vec<Arc<dyn DatabaseHooks<ClinicalAuthSchema>>>,
        model_fields: ModelFields,
    ) -> AuthResult<Arc<dyn AuthStore<ClinicalAuthSchema>>> {
        Ok(Arc::new(self.bind_runtime(config, hooks, model_fields)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct UnsupportedHook;

    #[async_trait::async_trait]
    impl DatabaseHooks<ClinicalAuthSchema> for UnsupportedHook {
        fn hook_metadata(&self) -> better_auth_core::observability::database::DatabaseHookMetadata {
            better_auth_core::observability::database::DatabaseHookMetadata {
                methods: &[],
                source: "test",
            }
        }
    }

    #[test]
    fn separate_runtime_builds_do_not_mutate_original_store() {
        let original = ClinicalStore::new(sea_orm::DatabaseConnection::default());
        let mut first_config = AuthConfig::default();
        first_config.secret = "first".into();
        let mut second_config = AuthConfig::default();
        second_config.secret = "second".into();
        let mut first_fields = ModelFields::default();
        first_fields.register_organization_schema(&Default::default(), false, false);
        let first = original
            .bind_runtime(Arc::new(first_config), Vec::new(), first_fields)
            .unwrap();
        let second = original
            .bind_runtime(Arc::new(second_config), Vec::new(), ModelFields::default())
            .unwrap();
        assert_eq!(original.adapter_id(), "medtracker-postgres");
        assert!(original.runtime.is_none());
        assert_eq!(first.runtime.as_ref().unwrap()._config.secret, "first");
        assert_eq!(second.runtime.as_ref().unwrap()._config.secret, "second");
        assert!(
            !first
                .runtime
                .as_ref()
                .unwrap()
                ._model_fields
                .organization_teams_enabled()
        );
        assert!(
            second
                .runtime
                .as_ref()
                .unwrap()
                ._model_fields
                .organization_teams_enabled()
        );
        assert!(!Arc::ptr_eq(
            first.runtime.as_ref().unwrap(),
            second.runtime.as_ref().unwrap()
        ));
    }

    #[test]
    fn unsupported_database_rate_limit_mapping_fails_closed() {
        use better_auth_core::{
            middleware::{RateLimitConfig, RateLimitStorageKind},
            store::schema::SchemaConfiguration,
        };

        let store = ClinicalStore::new(sea_orm::DatabaseConnection::default());
        for (model_name, mapped_field) in [
            (Some("alternate_rate_limits"), None),
            (None, Some("stored_count")),
        ] {
            let mut fields = ModelFields::default();
            let mappings = mapped_field
                .map(|name| std::iter::once(("count".to_owned(), name.to_owned())).collect());
            fields.set_schema_configuration(&SchemaConfiguration {
                config: Arc::new(AuthConfig::default()),
                plugins: Vec::new(),
                metadata: Default::default(),
                secondary_storage: false,
                rate_limit: RateLimitConfig {
                    storage: Some(RateLimitStorageKind::Database),
                    model_name: model_name.map(str::to_owned),
                    fields: mappings,
                    ..Default::default()
                },
            });
            assert!(matches!(
                store.bind_runtime(Arc::new(AuthConfig::default()), Vec::new(), fields),
                Err(AuthError::Config(_))
            ));
            assert!(store.runtime.is_none());
        }
    }

    #[test]
    fn unsupported_custom_model_fails_closed() {
        let store = ClinicalStore::new(sea_orm::DatabaseConnection::default());
        let mut fields = ModelFields::default();
        fields
            .register_custom_model(
                "clinicalExtension",
                Some("custom_table"),
                Default::default(),
            )
            .unwrap();
        assert!(matches!(
            store.bind_runtime(Arc::new(AuthConfig::default()), Vec::new(), fields),
            Err(AuthError::Config(_))
        ));
        assert!(store.runtime.is_none());
    }

    #[test]
    fn unsupported_hook_registration_fails_closed() {
        let store = ClinicalStore::new(sea_orm::DatabaseConnection::default());
        let result = store.bind_runtime(
            Arc::new(AuthConfig::default()),
            vec![Arc::new(UnsupportedHook)],
            ModelFields::default(),
        );
        assert!(matches!(result, Err(AuthError::Config(_))));
        assert!(store.runtime.is_none());
    }
}
