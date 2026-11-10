use async_trait::async_trait;
use loco_rs::{
    Error, Result,
    app::{AppContext, Hooks, Initializer},
    bgworker::{BackgroundWorker, Queue},
    boot::{BootResult, StartMode, create_context, run_app},
    config::{Config, QueueConfig},
    controller::{
        AppRoutes,
        middleware::{self, MiddlewareLayer, MiddlewareStackExt},
    },
    environment::Environment,
    task::Tasks,
};
use migration::Migrator;
use sea_orm::{
    ConnectionTrait, Database,
    sea_query::{Alias, Expr, ExprTrait, Func, JoinType, Query},
};
use std::path::Path;
use std::sync::Arc;

pub struct App;

#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        crate::telemetry::build_version()
    }

    fn init_logger(ctx: &AppContext) -> Result<bool> {
        Ok(crate::telemetry::init::<Self>(ctx))
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        let ctx = create_context::<Self>(environment, config).await?;
        let result = async {
            if ctx.environment == Environment::Production {
                crate::models::platform::database_setup::validate_runtime(&ctx).await?;
            }
            loco_rs::db::converge::<Self, Migrator>(&ctx, &ctx.config.database).await?;
            if let (Some(queue), Some(QueueConfig::Postgres(config))) =
                (&ctx.queue_provider, &ctx.config.queue)
            {
                let connection = Database::connect(&config.uri).await?;
                let columns = Query::select()
                    .columns([
                        "id",
                        "name",
                        "task_data",
                        "status",
                        "run_at",
                        "interval",
                        "created_at",
                        "updated_at",
                        "tags",
                        "priority",
                        "attempts",
                        "unique_key",
                    ])
                    .from(Alias::new("pg_loco_queue"))
                    .limit(0)
                    .to_owned();
                connection
                    .query_all_raw(connection.get_database_backend().build(&columns))
                    .await?;
                let column_metadata = Query::select()
                    .columns(["column_name", "data_type", "is_nullable", "column_default"])
                    .from(("information_schema", "columns"))
                    .and_where(Expr::col("table_schema").eq("public"))
                    .and_where(Expr::col("table_name").eq("pg_loco_queue"))
                    .to_owned();
                let column_rows = connection
                    .query_all_raw(connection.get_database_backend().build(&column_metadata))
                    .await?;
                for (name, data_type, nullable, default) in [
                    ("id", "character varying", "NO", None),
                    ("name", "character varying", "NO", None),
                    ("task_data", "jsonb", "NO", None),
                    (
                        "status",
                        "character varying",
                        "NO",
                        Some("'queued'::character varying"),
                    ),
                    ("run_at", "timestamp with time zone", "NO", None),
                    ("interval", "bigint", "YES", None),
                    (
                        "created_at",
                        "timestamp with time zone",
                        "NO",
                        Some("now()"),
                    ),
                    (
                        "updated_at",
                        "timestamp with time zone",
                        "NO",
                        Some("now()"),
                    ),
                    ("tags", "jsonb", "YES", None),
                    ("priority", "integer", "NO", Some("0")),
                    ("attempts", "integer", "NO", Some("0")),
                    ("unique_key", "character varying", "YES", None),
                ] {
                    let mut found = false;
                    for row in &column_rows {
                        if row.try_get::<String>("", "column_name")? == name {
                            found = row.try_get::<String>("", "data_type")? == data_type
                                && row.try_get::<String>("", "is_nullable")? == nullable
                                && row
                                    .try_get::<Option<String>>("", "column_default")?
                                    .as_deref()
                                    == default;
                            break;
                        }
                    }
                    if !found {
                        return Err(Error::string(
                            "Queue schema is not prepared; run database migrations",
                        ));
                    }
                }
                let indexes = Query::select()
                    .column("indexname")
                    .from(("pg_catalog", "pg_indexes"))
                    .and_where(Expr::col("schemaname").eq("public"))
                    .and_where(Expr::col("tablename").eq("pg_loco_queue"))
                    .to_owned();
                let rows = connection
                    .query_all_raw(connection.get_database_backend().build(&indexes))
                    .await?;
                let names = rows
                    .iter()
                    .map(|row| row.try_get::<String>("", "indexname"))
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                for required in [
                    "pg_loco_queue_pkey",
                    "pg_loco_queue_unique_queued",
                    "pg_loco_queue_dequeue",
                    "pg_loco_queue_status_updated_at",
                ] {
                    if !names.iter().any(|name| name == required) {
                        return Err(Error::string(
                            "Queue schema is not prepared; run database migrations",
                        ));
                    }
                }
                // PostgreSQL exposes index predicates and indexed column names
                // through pg_get_expr/pg_get_indexdef; SeaQuery has no typed
                // equivalents for these catalog functions.
                let unique_index = Query::select()
                    .columns([
                        ("i", "indisunique"),
                        ("i", "indisvalid"),
                        ("i", "indnkeyatts"),
                    ])
                    .expr_as(
                        Func::cust(Alias::new("pg_get_expr"))
                            .args([Expr::col(("i", "indpred")), Expr::col(("i", "indrelid"))]),
                        "predicate",
                    )
                    .expr_as(
                        Func::cust(Alias::new("pg_get_indexdef")).args([
                            Expr::col(("i", "indexrelid")),
                            Expr::val(1_i32),
                            Expr::val(true),
                        ]),
                        "first_key",
                    )
                    .expr_as(
                        Func::cust(Alias::new("pg_get_indexdef")).args([
                            Expr::col(("i", "indexrelid")),
                            Expr::val(2_i32),
                            Expr::val(true),
                        ]),
                        "second_key",
                    )
                    .from_as(("pg_catalog", "pg_index"), "i")
                    .join_as(
                        JoinType::InnerJoin,
                        ("pg_catalog", "pg_class"),
                        "idx",
                        Expr::col(("idx", "oid")).equals(("i", "indexrelid")),
                    )
                    .join_as(
                        JoinType::InnerJoin,
                        ("pg_catalog", "pg_class"),
                        "table",
                        Expr::col(("table", "oid")).equals(("i", "indrelid")),
                    )
                    .join_as(
                        JoinType::InnerJoin,
                        ("pg_catalog", "pg_namespace"),
                        "namespace",
                        Expr::col(("namespace", "oid")).equals(("table", "relnamespace")),
                    )
                    .and_where(Expr::col(("idx", "relname")).eq("pg_loco_queue_unique_queued"))
                    .and_where(Expr::col(("table", "relname")).eq("pg_loco_queue"))
                    .and_where(Expr::col(("namespace", "nspname")).eq("public"))
                    .to_owned();
                let index = connection
                    .query_one_raw(connection.get_database_backend().build(&unique_index))
                    .await?
                    .ok_or_else(|| {
                        Error::string("Queue schema is not prepared; run database migrations")
                    })?;
                let unique: bool = index.try_get("", "indisunique")?;
                let valid: bool = index.try_get("", "indisvalid")?;
                let key_count: i16 = index.try_get("", "indnkeyatts")?;
                let first_key: String = index.try_get("", "first_key")?;
                let second_key: String = index.try_get("", "second_key")?;
                let predicate: Option<String> = index.try_get("", "predicate")?;
                if !unique
                    || !valid
                    || key_count != 2
                    || first_key != "name"
                    || second_key != "unique_key"
                    || predicate.as_deref()
                        != Some("(((status)::text = 'queued'::text) AND (unique_key IS NOT NULL))")
                {
                    return Err(Error::string(
                        "Queue schema is not prepared; run database migrations",
                    ));
                }
                let primary_key = Query::select()
                    .column(("key", "column_name"))
                    .from_as(("information_schema", "table_constraints"), "constraint")
                    .join_as(
                        JoinType::InnerJoin,
                        ("information_schema", "key_column_usage"),
                        "key",
                        Expr::col(("constraint", "constraint_name"))
                            .equals(("key", "constraint_name"))
                            .and(
                                Expr::col(("constraint", "constraint_schema"))
                                    .equals(("key", "constraint_schema")),
                            )
                            .and(
                                Expr::col(("constraint", "table_name"))
                                    .equals(("key", "table_name")),
                            )
                            .and(
                                Expr::col(("constraint", "table_schema"))
                                    .equals(("key", "table_schema")),
                            ),
                    )
                    .and_where(Expr::col(("constraint", "table_schema")).eq("public"))
                    .and_where(Expr::col(("constraint", "table_name")).eq("pg_loco_queue"))
                    .and_where(Expr::col(("constraint", "constraint_type")).eq("PRIMARY KEY"))
                    .to_owned();
                let primary = connection
                    .query_all_raw(connection.get_database_backend().build(&primary_key))
                    .await?;
                if primary.len() != 1
                    || primary
                        .first()
                        .map(|row| row.try_get::<String>("", "column_name"))
                        .transpose()?
                        .as_deref()
                        != Some("id")
                {
                    return Err(Error::string(
                        "Queue schema is not prepared; run database migrations",
                    ));
                }
                connection.close().await?;
                if config.dangerously_flush {
                    queue.clear().await?;
                }
            }
            Ok(())
        }
        .await;
        if let Err(error) = result {
            if let Some(queue) = &ctx.queue_provider {
                queue.shutdown()?;
            }
            return Err(error);
        }
        run_app::<Self>(&mode, ctx).await
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
        let mut ctx = ctx;
        crate::telemetry::db::install(&mut ctx.db);
        crate::telemetry::metrics::observe_pool(&ctx.db);
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
        crate::telemetry::queue_metrics::start(ctx);
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
        tasks.register(crate::models::platform::database_setup::DatabaseSetupTask);
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

    async fn on_shutdown(ctx: &AppContext) {
        crate::telemetry::queue_metrics::stop(ctx);
        crate::telemetry::shutdown().await;
    }
}

#[cfg(test)]
mod setup_tests {
    use super::*;

    #[test]
    fn database_setup_is_a_native_loco_task() {
        let mut tasks = Tasks::default();
        App::register_tasks(&mut tasks);
        assert!(tasks.names().iter().any(|name| name == "database-setup"));
    }
}
