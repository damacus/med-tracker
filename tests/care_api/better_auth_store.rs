use super::fixture::Fixture;

#[tokio::test]
async fn development_boot_uses_configured_session_key_without_environment_override() {
    use loco_rs::{app::Hooks, boot::StartMode, config::Config, environment::Environment};
    if std::env::var_os("MEDTRACKER_DEVELOPMENT_BOOT_CHILD").is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "better_auth_store::development_boot_uses_configured_session_key_without_environment_override",
                "--nocapture",
            ])
            .env_remove("MEDTRACKER_SESSION_KEY")
            .env("MEDTRACKER_DEVELOPMENT_BOOT_CHILD", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    assert!(std::env::var_os("MEDTRACKER_SESSION_KEY").is_none());
    let fixture = Fixture::new().await;
    let mut config = Config::new(&Environment::Development).unwrap();
    config.database.uri = fixture.runtime_uri.clone();
    let Some(loco_rs::config::QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
        panic!("PostgreSQL queue required");
    };
    queue.uri = fixture.runtime_uri.clone();
    let boot =
        med_tracker::app::App::boot(StartMode::ServerOnly, &Environment::Development, config)
            .await
            .expect("Committed development session key must initialise Better Auth");
    assert!(
        boot.app_context
            .shared_store
            .get::<med_tracker::models::identity::better_auth::IdentityService>()
            .is_some()
    );
    let mut production_config = boot.app_context.config.clone();
    production_config.settings.as_mut().unwrap()["browser_session"]["key"] =
        serde_json::Value::Null;
    let production = loco_rs::app::AppContext::builder(
        Environment::Production,
        fixture.runtime.clone(),
        production_config,
    )
    .build();
    assert!(
        med_tracker::app::App::before_routes(&production)
            .await
            .is_err()
    );
    fixture.close().await;
}

#[tokio::test]
async fn production_identity_cookies_stay_secure_behind_an_http_proxy() {
    use loco_rs::{
        app::{AppContext, Hooks},
        config::Config,
        environment::Environment,
    };
    if std::env::var_os("MEDTRACKER_PRODUCTION_COOKIE_CHILD").is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "better_auth_store::production_identity_cookies_stay_secure_behind_an_http_proxy",
                "--nocapture",
            ])
            .env_remove("MEDTRACKER_COOKIE_SECURE")
            .env("MEDTRACKER_PRODUCTION_COOKIE_CHILD", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let fixture = Fixture::new().await;
    let mut config = Config::new(&Environment::Test).unwrap();
    config.settings.as_mut().unwrap()["browser_session"]["secure"] = serde_json::json!(true);
    config.database.uri = fixture.runtime_uri.clone();
    assert!(config.server.full_url().starts_with("http://"));
    let context = AppContext::builder(
        Environment::Production,
        fixture.runtime.clone(),
        config.clone(),
    )
    .build();
    let _router = med_tracker::app::App::before_routes(&context)
        .await
        .unwrap();
    let service = context
        .shared_store
        .get::<med_tracker::models::identity::better_auth::IdentityService>()
        .unwrap();
    let mut request =
        better_auth_core::AuthRequest::new(better_auth_core::HttpMethod::Post, "/sign-in/email");
    request
        .headers
        .insert("origin".into(), context.config.server.full_url());
    request
        .headers
        .insert("content-type".into(), "application/json".into());
    request
        .headers
        .insert("x-forwarded-for".into(), "127.0.0.1".into());
    request.body = Some(
        serde_json::to_vec(
            &serde_json::json!({"email": "persistence@example.test", "password": "password"}),
        )
        .unwrap(),
    );
    let response = med_tracker::models::identity::better_auth::dispatch(
        &service,
        request,
        "production-cookie-fixture".into(),
    )
    .await
    .unwrap();
    assert_eq!(response.status, 200);
    assert!(
        response
            .headers
            .get("set-cookie")
            .expect("Sign-in must issue a session cookie")
            .contains("; Secure")
    );
    config.settings.as_mut().unwrap()["browser_session"]["secure"] = serde_json::json!(false);
    let insecure =
        AppContext::builder(Environment::Production, fixture.runtime.clone(), config).build();
    assert!(
        med_tracker::app::App::before_routes(&insecure)
            .await
            .is_err()
    );
    fixture.close().await;
}
