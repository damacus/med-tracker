use super::*;
use crate::app::App;
use loco_rs::task::Task;
use loco_rs::{
    app::{Hooks, Initializer},
    boot::{StartMode, create_context},
    config::Config,
    environment::Environment,
};
use sea_orm::{ConnectionTrait, EntityTrait, sea_query::ExprTrait};
use serde_json::json;

#[path = "../../../tests/support/db.rs"]
mod db;
#[path = "../../../tests/support/template.rs"]
mod template;

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL provided by task test:canary"]
async fn seeded_care_full_boot_gate_and_replay() {
    let uri = template::owned_database_url();
    template::prepare(&uri, "loco_canary_empty", |_| async {}).await;
    let database =
        db::OwnedDatabase::clone(&uri, "loco_canary_empty", "canary_db_", "canary_role_").await;
    let control = sea_orm::Database::connect(&uri).await.unwrap();
    control.execute_unprepared("CREATE TABLE canary_control(scope text PRIMARY KEY,run_id text,phase text,image text,updated_at timestamptz,last_success timestamptz,next_reset timestamptz)").await.unwrap();
    let image = format!("ghcr.io/damacus/medtracker@sha256:{}", "a".repeat(64));
    control
        .execute_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            "INSERT INTO canary_control VALUES($1,'test-run','seeding',$2,now(),now()-interval '1 hour',now()+interval '6 days')",
            [SCOPE.into(), image.clone().into()],
        ))
        .await
        .unwrap();
    let mut preflight_config = Config::new(&Environment::Production).unwrap();
    preflight_config.database.uri = uri.clone();
    if let Some(loco_rs::config::QueueConfig::Postgres(queue)) = preflight_config.queue.as_mut() {
        queue.uri = uri.clone();
        queue.reaper = None;
    }
    let mut preflight = create_context::<App>(&Environment::Production, preflight_config)
        .await
        .unwrap();
    preflight.config.database.uri =
        format!("postgres://preflight:password@{HOST}/medtracker_canary_control");
    preflight.config.settings.as_mut().unwrap()["canary"] = json!({"enabled":true,"scope":SCOPE,"image":image,"smoke_key":"synthetic-smoke-key-independent-32-bytes","control_database_url":format!("postgres://status:password@{HOST}/medtracker_canary_control")});
    validate(&preflight).unwrap();
    if let Some(loco_rs::config::QueueConfig::Postgres(queue)) = preflight.config.queue.as_mut() {
        queue.reaper = Some(loco_rs::config::ReaperConfig {
            age_minutes: 10,
            interval_seconds: 60,
        });
    }
    assert!(validate(&preflight).is_err());
    if let Some(loco_rs::config::QueueConfig::Postgres(queue)) = preflight.config.queue.as_mut() {
        queue.reaper = None;
    }
    preflight.config.mailer = Some(serde_json::from_value(json!({"stub":false})).unwrap());
    assert!(validate(&preflight).is_err());
    preflight.config.mailer = None;

    preflight.shared_store.insert(Arc::new(Canary {
        control: control.clone(),
        image: image.clone(),
        smoke_key: "synthetic-smoke-key-independent-32-bytes".into(),
    }));
    CapabilitiesTask
        .run(&preflight, &Vars::default())
        .await
        .unwrap();
    let safe_settings = preflight.config.settings.clone();
    for unsafe_canary in [
        json!({"scope":"production"}),
        json!({"image":"mutable:beta"}),
        json!({"smoke_key":"short"}),
        json!({"control_database_url":"postgres://production/medtracker"}),
    ] {
        preflight.config.settings = safe_settings.clone();
        for (key, value) in unsafe_canary.as_object().unwrap() {
            preflight.config.settings.as_mut().unwrap()["canary"][key] = value.clone();
        }
        assert!(validate(&preflight).is_err());
    }
    for section in ["browser_push", "avatar_storage", "dmd_storage", "zitadel"] {
        preflight.config.settings = safe_settings.clone();
        preflight.config.settings.as_mut().unwrap()[section] =
            json!({"endpoint":"https://external.example"});
        assert!(validate(&preflight).is_err());
    }
    preflight.config.settings = safe_settings.clone();
    preflight.config.settings.as_mut().unwrap()["ai_medication_help"]["enabled"] = json!(true);
    assert!(validate(&preflight).is_err());
    preflight.db.close().await.unwrap();
    let mut config = Config::new(&Environment::Test).unwrap();
    let mut admin_uri = url::Url::parse(&database.runtime_uri).unwrap();
    let source = url::Url::parse(&uri).unwrap();
    admin_uri.set_username(source.username()).unwrap();
    admin_uri.set_password(source.password()).unwrap();
    config.database.uri = admin_uri.to_string();
    if let Some(loco_rs::config::QueueConfig::Postgres(queue)) = config.queue.as_mut() {
        queue.uri = admin_uri.to_string();
    }
    let mut seed_ctx = create_context::<App>(&Environment::Test, config)
        .await
        .unwrap();
    let epoch = Utc::now();
    seed_ctx.config.database.uri = format!("postgres://setup:password@{HOST}/medtracker_canary");
    seed_ctx.config.settings = Some(
        json!({"canary":{"enabled":true,"scope":SCOPE,"image":image,"epoch":epoch.to_rfc3339(),"run_id":"test-run"}}),
    );
    seed_ctx.shared_store.insert(Arc::new(Canary {
        control: control.clone(),
        image: image.clone(),
        smoke_key: "synthetic-smoke-key-independent-32-bytes".into(),
    }));
    crate::models::_entities::people::Entity::insert(
        crate::models::_entities::people::ActiveModel {
            id: sea_orm::Set(99999),
            name: sea_orm::Set("Unrecognised population".into()),
            person_type: sea_orm::Set(0),
            has_capacity: sea_orm::Set(true),
            created_at: sea_orm::Set(epoch.naive_utc()),
            updated_at: sea_orm::Set(epoch.naive_utc()),
            ..Default::default()
        },
    )
    .exec(&database.admin)
    .await
    .unwrap();
    assert!(fixtures::seed(&seed_ctx).await.is_err());
    assert_eq!(
        crate::models::_entities::people::Entity::find()
            .all(&database.admin)
            .await
            .unwrap()
            .len(),
        1
    );
    let markers = sea_orm::sea_query::Query::select()
        .column(sea_orm::sea_query::Alias::new("table_name"))
        .from((
            sea_orm::sea_query::Alias::new("information_schema"),
            sea_orm::sea_query::Alias::new("tables"),
        ))
        .and_where(
            sea_orm::sea_query::Expr::col(sea_orm::sea_query::Alias::new("table_name"))
                .eq("canary_seed"),
        )
        .to_owned();
    assert!(database.admin.query_one(&markers).await.unwrap().is_none());
    crate::models::_entities::people::Entity::delete_by_id(99999)
        .exec(&database.admin)
        .await
        .unwrap();
    seed_ctx.environment = Environment::Production;
    assert!(
        App::seed(&seed_ctx, std::path::Path::new("src/fixtures"))
            .await
            .is_err()
    );
    fixtures::seed(&seed_ctx).await.unwrap();
    fixtures::seed(&seed_ctx).await.unwrap();
    assert_eq!(
        crate::models::_entities::households::Entity::find()
            .all(&database.admin)
            .await
            .unwrap()
            .len(),
        3
    );
    assert!(
        crate::models::_entities::push_subscriptions::Entity::find()
            .all(&database.admin)
            .await
            .unwrap()
            .is_empty()
    );
    let mut config = Config::new(&Environment::Test).unwrap();
    config.database.uri = database.runtime_uri.clone();
    if let Some(loco_rs::config::QueueConfig::Postgres(queue)) = config.queue.as_mut() {
        queue.uri = database.runtime_uri.clone();
        queue.reaper = None;
    }
    let key =
        "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBw==";
    config.settings.as_mut().unwrap()["browser_session"] = json!({"key":key,"secure":false});
    let boot = App::boot(StartMode::ServerOnly, &Environment::Test, config)
        .await
        .unwrap();
    let mut ctx = boot.app_context.clone();
    ctx.config.settings.as_mut().unwrap()["canary"] =
        json!({"enabled":true,"smoke_key":"synthetic-smoke-key-independent-32-bytes"});
    ctx.shared_store.insert(Arc::new(Canary {
        control: control.clone(),
        image: image.clone(),
        smoke_key: "synthetic-smoke-key-independent-32-bytes".into(),
    }));
    async fn notice_page(
        loco_rs::controller::views::ViewEngine(view): loco_rs::controller::views::ViewEngine<
            loco_rs::controller::views::engines::TeraView,
        >,
    ) -> axum::response::Html<String> {
        use loco_rs::controller::views::ViewRenderer;
        axum::response::Html(
            view.render("layouts/base.html", json!({"title":"Synthetic canary","appearances":[{"id":"light","label":"Light"},{"id":"dark","label":"Dark"},{"id":"system","label":"System"}],"palettes":[]}))
                .unwrap(),
        )
    }
    let notice_router = crate::initializers::ViewEngineInitializer
        .after_routes(
            axum::Router::new().route("/__test/canary-notice", axum::routing::get(notice_page)),
            &ctx,
        )
        .await
        .unwrap();
    let mut ordinary = ctx.clone();
    ordinary.config.settings.as_mut().unwrap()["canary"]["enabled"] = json!(false);
    let ordinary_router = crate::initializers::ViewEngineInitializer
        .after_routes(
            axum::Router::new().route("/__test/ordinary", axum::routing::get(notice_page)),
            &ordinary,
        )
        .await
        .unwrap();
    let router = boot
        .router
        .unwrap()
        .merge(notice_router)
        .merge(ordinary_router)
        .layer(axum::middleware::from_fn_with_state(ctx.clone(), boundary));
    let server = loco_rs::TestServer::new(router.clone()).unwrap();
    assert_eq!(
        server.get("/login").await.status_code(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    control
        .execute_unprepared("UPDATE canary_control SET phase='verifying',updated_at=now()")
        .await
        .unwrap();
    assert_eq!(
        server.get("/login").await.status_code(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        server
            .get("/login")
            .add_header("x-medtracker-canary-smoke", "wrong-key")
            .await
            .status_code(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        server
            .get("/login")
            .add_header(
                "x-medtracker-canary-smoke",
                "synthetic-smoke-key-independent-32-bytes"
            )
            .await
            .status_code(),
        StatusCode::OK
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = url::Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
    let serving = tokio::spawn(async move {
        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
    });
    smoke::journey(&ctx, &url).await.unwrap();
    work(&ctx).await.unwrap();
    control
        .execute_unprepared("UPDATE canary_control SET updated_at=now()-interval '31 seconds'")
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(50), work(&ctx))
            .await
            .is_err()
    );
    let waiting_ctx = ctx.clone();
    let waiting = tokio::spawn(async move { work(&waiting_ctx).await });
    control
        .execute_unprepared("UPDATE canary_control SET phase='ready',updated_at=now()")
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(3), waiting)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(server.get("/login").await.status_code(), StatusCode::OK);
    let notice = server.get("/__test/canary-notice").await.text();
    assert!(notice.contains("All edits are deleted at the weekly reset"));
    assert!(notice.contains("/static/js/canary.js"));
    assert!(!notice.contains("synthetic-smoke-key-independent"));
    let ordinary = server.get("/__test/ordinary").await.text();
    assert!(!ordinary.contains("data-canary-notice"));
    assert!(!ordinary.contains("/static/js/canary.js"));
    let status = server.get("/canary/status").await;
    assert_eq!(status.headers().get("cache-control").unwrap(), "no-store");
    let status = status.json::<serde_json::Value>();
    assert_eq!(status.as_object().unwrap().len(), 3);
    assert_eq!(status["phase"], "ready");
    assert!(status["last_success"].is_string());
    assert!(status["next_reset"].is_string());
    playwright_rs::install_browsers(Some(&["chromium"]))
        .await
        .unwrap();
    let playwright = playwright_rs::Playwright::launch().await.unwrap();
    let browser = playwright.chromium().launch().await.unwrap();
    for (name, width, height) in [("canary-desktop", 1440, 1000), ("canary-mobile", 390, 844)] {
        let context = browser
            .new_context_with_options(
                playwright_rs::protocol::BrowserContextOptions::builder()
                    .viewport(playwright_rs::protocol::Viewport { width, height })
                    .build(),
            )
            .await
            .unwrap();
        let page = context.new_page().await.unwrap();
        page.goto(url.join("__test/canary-notice").unwrap().as_ref(), None)
            .await
            .unwrap();
        playwright_rs::expect(page.locator("[data-canary-next]"))
            .to_contain_text("Europe/London")
            .await
            .unwrap();
        let overflow: bool = page
            .evaluate(
                "() => document.documentElement.scrollWidth > window.innerWidth",
                None::<&()>,
            )
            .await
            .unwrap();
        assert!(!overflow);
        std::fs::create_dir_all("docs/screenshots/browser-journeys").unwrap();
        page.screenshot_to_file(
            &std::path::Path::new("docs/screenshots/browser-journeys").join(format!("{name}.png")),
            playwright_rs::protocol::ScreenshotOptions::builder()
                .animations(playwright_rs::protocol::Animations::Disabled)
                .build(),
        )
        .await
        .unwrap();
        context.close().await.unwrap();
    }
    browser.close().await.unwrap();
    serving.abort();

    control
        .execute_unprepared("DROP TABLE canary_control")
        .await
        .unwrap();
    seed_ctx.db.close().await.unwrap();
    ctx.db.close().await.unwrap();
    control.close().await.unwrap();
    database.close().await;
}
