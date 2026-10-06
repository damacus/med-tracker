use async_trait::async_trait;
use loco_rs::{
    Error, Result,
    app::{AppContext, Hooks, Initializer},
    bgworker::Queue,
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
            .add_route(crate::controllers::oauth_server::routes())
            .add_route(browser(crate::controllers::browser_routes()))
            .add_route(browser(crate::controllers::auth::routes()))
            .add_route(browser(crate::controllers::oauth_server::browser_routes()))
            .add_route(browser(crate::controllers::medications::routes()))
            .add_route(browser(crate::controllers::locations::routes()))
            .add_route(browser(crate::controllers::people::routes()))
    }

    async fn before_routes(ctx: &AppContext) -> Result<axum::Router<AppContext>> {
        let layers = crate::models::identity::browser::layers(ctx)
            .await
            .map_err(|_| Error::string("Browser session configuration is unavailable"))?;
        if let Some(layers) = layers {
            ctx.shared_store.insert(layers);
        }
        Ok(axum::Router::new())
    }

    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
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
