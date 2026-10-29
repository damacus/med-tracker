use async_trait::async_trait;
use loco_rs::{
    Error, Result,
    app::{AppContext, Hooks, Initializer},
    bgworker::{BackgroundWorker, Queue},
    boot::{BootResult, StartMode, create_app},
    config::Config,
    controller::{
        AppRoutes,
        middleware::{self, MiddlewareLayer, MiddlewareStackExt},
    },
    environment::Environment,
    task::Tasks,
};
use migration::Migrator;
use std::path::Path;
use std::sync::Arc;

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
            .add_route(browser(crate::controllers::searches::routes()))
            .add_route(browser(crate::controllers::dashboard::routes()))
            .add_route(browser(crate::controllers::nhs_dmd::routes()))
            .add_route(browser(crate::controllers::medication_orders::routes()))
            .add_route(browser(crate::controllers::dosage_options::routes()))
            .add_route(browser(crate::controllers::locations::routes()))
            .add_route(browser(crate::controllers::people::routes()))
            .add_route(browser(crate::controllers::profile::routes()))
            .add_route(browser(
                crate::controllers::profile::account_profile::routes(),
            ))
            .add_route(browser(crate::controllers::guardian_declarations::routes()))
            .add_route(browser(crate::controllers::profile::avatar::routes()))
            .add_route(browser(crate::controllers::treatments::routes()))
            .add_route(browser(crate::controllers::reports::routes()))
            .add_route(browser(crate::controllers::review_prompts::routes()))
            .add_route(browser(crate::controllers::dose_occurrences::routes()))
            .add_route(browser(crate::controllers::administration::routes()))
            .add_route(browser(crate::controllers::invitations::routes()))
            .add_route(browser(crate::controllers::invitations::acceptance_routes()))
            .add_route(browser(crate::controllers::platform::routes()))
    }

    fn middlewares(ctx: &AppContext) -> Vec<Box<dyn MiddlewareLayer>> {
        let mut middlewares = middleware::default_middleware_stack(ctx);
        middlewares.delete("request_id");
        middlewares
    }

    async fn after_context(ctx: AppContext) -> Result<AppContext> {
        crate::models::canary::configure(&ctx).await?;
        let rate_limit = crate::models::rate_limit::RateLimitState::from_settings(
            ctx.config.settings.as_ref(),
            &ctx.environment,
        )
        .map_err(|error| Error::string(&error.to_string()))?;
        ctx.shared_store.insert(Arc::new(rate_limit));
        if let Some(push) =
            crate::models::care::browser_push::Service::from_context(&ctx).map_err(Error::string)?
        {
            ctx.shared_store.insert(push);
        }
        if let Some((storage, operators)) =
            crate::models::storage::configured(ctx.config.settings.as_ref())
        {
            ctx.shared_store.insert(operators);
            return Ok(ctx.into_builder().storage(storage.into()).build());
        }
        Ok(ctx)
    }

    async fn before_routes(ctx: &AppContext) -> Result<axum::Router<AppContext>> {
        let layers = crate::models::identity::browser::layers(ctx)
            .await
            .map_err(|_| Error::string("Browser session configuration is unavailable"))?;
        if let Some(layers) = layers {
            let mut config = better_auth::AuthConfig::default();
            config.secret = layers.identity_secret.clone();
            config.advanced.use_secure_cookies = Some(layers.secure);
            ctx.shared_store.insert(layers);
            let origin = crate::models::settings::public_origin(ctx)
                .map_err(|error| Error::string(&error.to_string()))?;
            config.base_url = origin
                .join("/api/auth")
                .map_err(|error| Error::string(&error.to_string()))?
                .to_string()
                .into();
            config.trusted_origins = Some(vec![origin.origin().ascii_serialization()].into());
            config.password.require_uppercase = false;
            config.password.require_lowercase = false;
            config.password.require_numbers = false;
            config.password.require_special = false;
            let store = std::sync::Arc::new(
                crate::models::identity::better_auth::IdentityPolicy::new(ctx.db.clone())
                    .with_settings(ctx.config.settings.clone()),
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
            .register(crate::models::nhs_dmd::ImportWorker::build(ctx))
            .await?;
        queue
            .register(crate::models::care::review_prompts::refresh::RefreshWorker::build(ctx))
            .await?;
        queue
            .register(crate::models::care::browser_push::scheduler::ReminderWorker::build(ctx))
            .await?;
        queue
            .register(crate::models::care::invitations::delivery::InvitationMailWorker::build(ctx))
            .await?;
        queue
            .register(crate::models::canary::MailerWorker::build(ctx))
            .await?;
        queue
            .register(crate::models::profile::avatar::AvatarRetirementWorker::build(ctx))
            .await?;
        Ok(())
    }

    async fn after_routes(router: axum::Router, ctx: &AppContext) -> Result<axum::Router> {
        let rate_limit = ctx
            .shared_store
            .get::<Arc<crate::models::rate_limit::RateLimitState>>()
            .ok_or_else(|| Error::string("API rate-limit state is unavailable"))?;
        let router = router
            .layer(axum::middleware::from_fn(
                crate::models::identity::better_auth::http_boundary,
            ))
            .layer(axum::middleware::from_fn_with_state(
                rate_limit,
                crate::controllers::api::rate_limit::boundary,
            ));
        let router = router.layer(axum::middleware::from_fn_with_state(
            ctx.clone(),
            crate::models::canary::boundary,
        ));
        let router = if ctx
            .config
            .server
            .middlewares
            .request_id
            .as_ref()
            .is_none_or(|request_id| request_id.enable)
        {
            router.layer(axum::middleware::from_fn(
                middleware::request_id::request_id_middleware,
            ))
        } else {
            router
        };
        Ok(router.layer(axum::middleware::from_fn(crate::request_trace::boundary)))
    }

    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(crate::models::canary::CapabilitiesTask);
        tasks.register(crate::models::canary::SeedTask);
        tasks.register(crate::models::canary::SmokeTask);
        tasks.register(crate::models::nhs_dmd::ReconcileTask);
        tasks.register(crate::models::care::review_prompts::refresh::RefreshTask);
        tasks.register(crate::models::care::browser_push::scheduler::ReminderTask);
        tasks.register(crate::models::platform::expiry::SupportExpiryTask);
    }

    async fn truncate(_ctx: &AppContext) -> Result<()> {
        Err(Error::string(
            "Database truncation is disabled during migration",
        ))
    }

    async fn seed(ctx: &AppContext, base: &Path) -> Result<()> {
        crate::models::seed::seed(ctx, base).await
    }
}
