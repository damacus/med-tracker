use async_trait::async_trait;
use loco_rs::{
    Error, Result,
    app::{AppContext, Hooks, Initializer},
    bgworker::{BackgroundWorker, Queue},
    boot::{BootResult, StartMode, create_app},
    config::Config,
    controller::AppRoutes,
    environment::Environment,
    task::Tasks,
};
use migration::Migrator;
use std::path::Path;

pub struct App;

#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![Box::new(crate::initializers::ViewEngineInitializer)])
    }

    fn routes(ctx: &AppContext) -> AppRoutes {
        let browser = |routes| crate::models::identity::browser::route_layers(ctx, routes);
        AppRoutes::with_default_routes()
            .add_route(crate::controllers::routes())
            .add_route(crate::controllers::api::care::routes())
            .add_route(crate::controllers::api::account_sessions::routes())
            .add_route(crate::controllers::api::care::invitation_routes())
            .add_route(crate::controllers::oauth_server::routes())
            .add_route(browser(crate::controllers::browser_routes()))
            .add_route(browser(crate::controllers::auth::routes()))
            .add_route(browser(crate::controllers::identity_onboarding::routes()))
            .add_route(browser(crate::controllers::oauth_server::browser_routes()))
            .add_route(browser(crate::controllers::medications::routes()))
            .add_route(browser(crate::controllers::medication_orders::routes()))
            .add_route(browser(crate::controllers::dosage_options::routes()))
            .add_route(browser(crate::controllers::locations::routes()))
            .add_route(browser(crate::controllers::people::routes()))
            .add_route(browser(crate::controllers::treatments::routes()))
            .add_route(browser(crate::controllers::reports::routes()))
            .add_route(browser(crate::controllers::dose_occurrences::routes()))
            .add_route(browser(crate::controllers::administration::routes()))
            .add_route(browser(crate::controllers::invitations::routes()))
            .add_route(browser(crate::controllers::invitations::acceptance_routes()))
    }

    async fn before_routes(ctx: &AppContext) -> Result<axum::Router<AppContext>> {
        let layers = crate::models::identity::browser::layers(ctx)
            .await
            .map_err(|_| Error::string("Browser session configuration is unavailable"))?;
        if let Some(layers) = layers {
            let mut config = better_auth::AuthConfig {
                secret: layers.identity_secret.clone(),
                ..Default::default()
            };
            config.session.cookie_secure = layers.secure;
            ctx.shared_store.insert(layers);
            config.base_url = format!(
                "{}/api/auth",
                ctx.config.server.full_url().trim_end_matches('/')
            );
            config.trusted_origins = vec![ctx.config.server.full_url()];
            config.password.require_uppercase = false;
            config.password.require_lowercase = false;
            config.password.require_numbers = false;
            config.password.require_special = false;
            let store = std::sync::Arc::new(
                crate::models::identity::better_auth::ClinicalStore::new(ctx.db.clone()),
            );
            let service = crate::models::identity::better_auth::build(config, store)
                .await
                .map_err(|_| Error::string("Identity service unavailable"))?;
            ctx.shared_store.insert(service.clone());
            return Ok(axum::Router::new().nest(
                "/api/auth",
                crate::models::identity::better_auth::router(service).with_state(()),
            ));
        }
        Ok(axum::Router::new())
    }

    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue
            .register(loco_rs::mailer::MailerWorker::build(ctx))
            .await?;
        Ok(())
    }

    async fn after_routes(router: axum::Router, _ctx: &AppContext) -> Result<axum::Router> {
        Ok(router.layer(axum::middleware::from_fn(
            crate::models::identity::better_auth::http_boundary,
        )))
    }

    fn register_tasks(_tasks: &mut Tasks) {}

    async fn truncate(_ctx: &AppContext) -> Result<()> {
        Err(Error::string(
            "Database truncation is disabled during migration",
        ))
    }

    async fn seed(ctx: &AppContext, base: &Path) -> Result<()> {
        crate::models::seed::seed(ctx, base).await
    }
}
