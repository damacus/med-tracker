use super::*;

async fn application(source_maximum: u32, password_maximum: u32) -> Application {
    Application::new_configured(None, |settings| {
        settings["api_rate_limit"] = json!({"trusted_proxy_ips": "127.0.0.1"});
        settings["identity_rate_limits"] = json!({
            "source_maximum": source_maximum,
            "source_window_seconds": 600,
            "password_maximum": password_maximum,
            "password_window_seconds": 600
        });
    })
    .await
}

fn client_ip() -> String {
    std::net::Ipv6Addr::from(uuid::Uuid::new_v4().as_u128()).to_string()
}

async fn native_attempt(app: &Application, source: &str) -> reqwest::Response {
    let email = format!("proxy-{}@example.test", app.fixture.client_uuid);
    app.client
        .post(format!("{}/api/auth/sign-in/email", app.origin))
        .header("origin", app.context.config.server.full_url())
        .header("x-forwarded-for", source)
        .json(&json!({"email": email, "password": "wrong-password"}))
        .send()
        .await
        .unwrap()
}

async fn browser_attempt(app: &Application, source: &str) -> reqwest::Response {
    let page = app
        .client
        .get(format!("{}/login", app.origin))
        .header("x-forwarded-for", source)
        .send()
        .await
        .unwrap();
    assert_eq!(page.status().as_u16(), 200);
    let cookie = super::platform::response_cookies("", &page);
    let token = super::platform::hidden_field(&page.text().await.unwrap(), "authenticity_token");
    assert!(!token.is_empty());
    let public = app.context.config.server.full_url();
    let url = url::Url::parse(&public).unwrap();
    let authority = match url.port() {
        Some(port) => format!("{}:{port}", url.host_str().unwrap()),
        None => url.host_str().unwrap().to_owned(),
    };
    app.client
        .post(format!("{}/login", app.origin))
        .header("cookie", cookie)
        .header("host", authority)
        .header("origin", public.trim_end_matches('/'))
        .header("x-forwarded-for", source)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(
            serde_urlencoded::to_string([
                (
                    "email",
                    format!("proxy-{}@example.test", app.fixture.client_uuid),
                ),
                ("password", "wrong-password".to_owned()),
                ("authenticity_token", token),
            ])
            .unwrap(),
        )
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn trusted_proxy_authentication_limits_separate_clients() {
    let app = application(2, 100).await;
    let first = client_ip();
    let second = client_ip();
    let mut results = Vec::new();
    for source in [&first, &first, &second, &first, &second] {
        let response = native_attempt(&app, source).await;
        results.push((
            response.status().as_u16(),
            response.headers().get("retry-after").cloned(),
        ));
    }
    app.close().await;
    assert_eq!(
        results
            .iter()
            .map(|(status, _)| *status)
            .collect::<Vec<_>>(),
        [401, 401, 401, 429, 401]
    );
    assert!(
        results[3]
            .1
            .as_ref()
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            > 0
    );
}

#[tokio::test]
async fn native_and_browser_authentication_share_trusted_client_limits() {
    let app = application(2, 100).await;
    let first = client_ip();
    let second = client_ip();
    let mut results = Vec::new();
    for (source, browser) in [
        (&first, false),
        (&first, true),
        (&first, false),
        (&second, true),
        (&second, false),
        (&second, true),
    ] {
        let response = if browser {
            browser_attempt(&app, source).await
        } else {
            native_attempt(&app, source).await
        };
        results.push((
            response.status().as_u16(),
            response.headers().get("retry-after").is_some(),
        ));
    }
    app.close().await;
    assert_eq!(
        results,
        [
            (401, false),
            (401, false),
            (429, true),
            (401, false),
            (401, false),
            (429, true)
        ]
    );
}

#[tokio::test]
async fn trusted_proxy_account_limits_apply_across_client_sources() {
    let app = application(10, 2).await;
    let mut results = Vec::new();
    for _ in 0..3 {
        let response = native_attempt(&app, &client_ip()).await;
        results.push((
            response.status().as_u16(),
            response.headers().get("retry-after").is_some(),
        ));
    }
    app.close().await;
    assert_eq!(results, [(401, false), (401, false), (429, true)]);
}

#[tokio::test]
async fn untrusted_peers_cannot_change_authentication_source_buckets() {
    let app = application(2, 100).await;
    let mut config = app.context.config.clone();
    config.settings.as_mut().unwrap()["api_rate_limit"]["trusted_proxy_ips"] = json!("");
    let boot = med_tracker::app::App::boot(StartMode::ServerOnly, &Environment::Test, config)
        .await
        .unwrap();
    let peer = std::net::SocketAddr::new(client_ip().parse().unwrap(), 1234);
    let server = loco_rs::TestServer::builder()
        .mock_transport()
        .build(
            boot.router
                .unwrap()
                .layer(axum::Extension(axum::extract::ConnectInfo(peer))),
        )
        .unwrap();
    let mut results = Vec::new();
    for _ in 0..3 {
        let source = client_ip();
        let response = server
            .post("/api/auth/sign-in/email")
            .add_header("origin", app.context.config.server.full_url())
            .add_header("x-forwarded-for", &source)
            .add_header("x-real-ip", &source)
            .add_header("x-medtracker-peer", &source)
            .json(&json!({"email": format!("proxy-{}@example.test", app.fixture.client_uuid), "password": "wrong-password"}))
            .await;
        results.push((
            response.status_code().as_u16(),
            response.headers().contains_key("retry-after"),
        ));
    }
    drop(server);
    if let Some(queue) = boot.app_context.queue_provider.as_ref() {
        queue.shutdown().unwrap();
    }
    boot.app_context.db.close().await.unwrap();
    app.close().await;
    assert_eq!(results, [(401, false), (401, false), (429, true)]);
}
