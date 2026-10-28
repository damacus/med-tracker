#[path = "account_authentication.rs"]
mod account_authentication;
#[path = "account_preferences.rs"]
mod account_preferences;
#[path = "account_profile.rs"]
mod account_profile;
#[path = "administration_overview.rs"]
mod administration_overview;
#[path = "avatar.rs"]
mod avatar;
#[path = "bulk_members.rs"]
mod bulk_members;
#[path = "capabilities.rs"]
mod capabilities;
#[path = "../care_api/contract.rs"]
mod contract;
#[path = "favicon.rs"]
mod favicon;
#[path = "household_selection.rs"]
mod household_selection;
#[path = "household_settings.rs"]
mod household_settings;
#[path = "me_ui.rs"]
mod me_ui;
#[path = "people.rs"]
mod people;
#[path = "personal_history.rs"]
mod personal_history;
#[path = "profile.rs"]
mod profile;
#[path = "request_trace.rs"]
mod request_trace;

use crate::scenario::Scenario;
use loco_rs::{
    TestServer,
    app::Hooks,
    boot::{BootResult, StartMode},
    config::Config,
    environment::Environment,
};
#[path = "fixture.rs"]
mod request_fixture;
use request_fixture::{CredentialKind, RequestFixture};
use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionTrait};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

async fn install_browser_session(request: &mut TestServer, boot: &BootResult, account_id: i64) {
    let cookies = crate::session_fixture::browser_cookie(&boot.app_context, account_id).await;
    assert!(
        !cookies.is_empty(),
        "native browser fixture must issue cookies"
    );
    for pair in cookies.split("; ") {
        request.add_cookie(pair.parse().expect("native browser cookie pair"));
    }
}

async fn select_request_household(
    request: &mut TestServer,
    fixture: &RequestFixture,
    boot: &BootResult,
) -> Result<(), Box<dyn std::error::Error>> {
    install_browser_session(request, boot, fixture.account.id).await;
    let chooser = request.get("/households/choose").await;
    let html = chooser.text();
    let csrf =
        administration_overview::authenticity_token(&html).ok_or("household form token missing")?;
    let selected = request
        .post("/households/select")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("household_id", fixture.household.id.to_string().as_str()),
            ("authenticity_token", csrf),
        ])
        .await;
    assert_eq!(selected.status_code(), 303, "{}", selected.text());
    Ok(())
}

async fn boot_request_fixture() -> (RequestFixture, BootResult) {
    let fixture = RequestFixture::new().await;
    let boot = loco_rs::testing::request::boot_test::<med_tracker::app::App>()
        .await
        .unwrap();
    (fixture, boot)
}

async fn boot_request_fixture_with(
    configure: impl FnOnce(&mut Config),
) -> (RequestFixture, BootResult) {
    let fixture = RequestFixture::new().await;
    let mut config = med_tracker::app::App::load_config(&Environment::Test)
        .await
        .unwrap();
    configure(&mut config);
    assert!(!config.database.dangerously_truncate);
    assert!(!config.database.dangerously_recreate);
    let boot = med_tracker::app::App::boot(StartMode::ServerOnly, &Environment::Test, config)
        .await
        .unwrap();
    (fixture, boot)
}

async fn boot_request_rate_fixture(trusted_proxy_ips: &[&str]) -> (RequestFixture, BootResult) {
    boot_request_rate_fixture_with(trusted_proxy_ips, None).await
}

async fn boot_request_rate_fixture_with(
    trusted_proxy_ips: &[&str],
    request_id_enabled: Option<bool>,
) -> (RequestFixture, BootResult) {
    boot_request_fixture_with(|config| {
        config.settings.get_or_insert_with(|| json!({}))["api_rate_limit"] = json!({
            "fixed_now_unix_seconds":1000, "trusted_proxy_ips":trusted_proxy_ips.join(","),
        });
        if let Some(enable) = request_id_enabled {
            config.server.middlewares.request_id =
                Some(loco_rs::controller::middleware::request_id::RequestId { enable });
        }
    })
    .await
}

async fn close_request(fixture: RequestFixture, boot: BootResult) {
    close_boot(boot).await;
    fixture.close().await;
}

async fn boot_shared_fixture() -> (Scenario, BootResult) {
    let scenario = Scenario::new().await;
    let boot = loco_rs::testing::request::boot_test::<med_tracker::app::App>()
        .await
        .unwrap();
    (scenario, boot)
}

async fn boot_shared_fixture_with(configure: impl FnOnce(&mut Config)) -> (Scenario, BootResult) {
    let scenario = Scenario::new().await;
    let mut config = med_tracker::app::App::load_config(&Environment::Test)
        .await
        .unwrap();
    configure(&mut config);
    assert!(!config.database.dangerously_truncate);
    assert!(!config.database.dangerously_recreate);
    assert_eq!(config.database.uri, std::env::var("DATABASE_URL").unwrap());
    let boot = med_tracker::app::App::boot(StartMode::ServerOnly, &Environment::Test, config)
        .await
        .unwrap();
    (scenario, boot)
}

async fn boot_shared_rate_fixture(trusted_proxy_ips: &[&str]) -> (Scenario, BootResult) {
    boot_shared_fixture_with(|config| {
        config.settings.get_or_insert_with(|| json!({}))["api_rate_limit"] = json!({
            "fixed_now_unix_seconds": 1000,
            "trusted_proxy_ips": trusted_proxy_ips.join(","),
        });
    })
    .await
}

async fn close_shared(scenario: Scenario, boot: BootResult) {
    close_boot(boot).await;
    scenario.close().await;
}

async fn close_boot(boot: BootResult) {
    if let Some(queue) = boot.app_context.queue_provider.as_ref() {
        queue.shutdown().unwrap();
    }
    boot.app_context.db.close().await.unwrap();
}

fn server(boot: &BootResult) -> TestServer {
    TestServer::new(
        boot.router
            .clone()
            .unwrap()
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .unwrap()
}

fn operations() -> BTreeMap<(String, String), Value> {
    let source = include_str!("../../docs/api/openapi.v1.yaml");
    let document: Value = serde_yaml_ng::from_str(source).unwrap();
    let mut operations = BTreeMap::new();
    let mut operation_ids = BTreeSet::new();
    for (path, item) in document["paths"].as_object().unwrap() {
        for (method, operation) in item.as_object().unwrap() {
            if ![
                "get", "post", "put", "patch", "delete", "head", "options", "trace",
            ]
            .contains(&method.as_str())
            {
                continue;
            }
            let parameters = item["parameters"]
                .as_array()
                .into_iter()
                .flatten()
                .chain(operation["parameters"].as_array().into_iter().flatten())
                .map(|parameter| {
                    parameter["name"]
                        .as_str()
                        .or_else(|| parameter["$ref"].as_str())
                        .unwrap()
                })
                .collect::<Vec<_>>();
            let mut responses = operation["responses"]
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>();
            responses.sort_unstable();
            let requirements = json!({
                "path": path,
                "method": method.to_uppercase(),
                "operation_id": operation["operationId"],
                "security": operation.get("security").unwrap_or(&json!("inherited")),
                "parameters": parameters,
                "request_body": operation.get("requestBody").is_some(),
                "responses": responses,
            });
            assert!(
                operations
                    .insert((method.to_uppercase(), path.clone()), requirements)
                    .is_none(),
                "Duplicate OpenAPI method and path"
            );
            assert!(
                operation_ids.insert(operation["operationId"].as_str().unwrap().to_owned()),
                "Duplicate OpenAPI operation ID"
            );
        }
    }
    operations
}

fn registered_routes(boot: &BootResult) -> BTreeSet<(String, String)> {
    med_tracker::app::App::routes(&boot.app_context)
        .collect()
        .into_iter()
        .flat_map(|route| {
            route
                .actions
                .into_iter()
                .map(move |method| (method.as_str().to_owned(), route.uri.clone()))
        })
        .collect()
}

fn api_v1_routes(boot: &BootResult) -> BTreeSet<(String, String)> {
    registered_routes(boot)
        .into_iter()
        .filter(|(_, path)| path.starts_with("/api/v1/"))
        .collect()
}

fn normalized(path: &str) -> String {
    path.split('/')
        .map(|part| {
            if part.starts_with('{') && part.ends_with('}') {
                "{}"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

pub mod baseline {
    use super::*;

    #[test]
    fn every_documented_operation_references_shared_rate_limit_response() {
        let document: Value =
            serde_yaml_ng::from_str(include_str!("../../docs/api/openapi.v1.yaml")).unwrap();
        let documented = operations();
        let missing = documented
            .keys()
            .filter(|(method, path)| {
                let method = method.to_lowercase();
                document["paths"][path.as_str()][method.as_str()]["responses"]["429"]["$ref"]
                    != "#/components/responses/RateLimited"
            })
            .collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "Documented API operations without shared RateLimited 429 response: {missing:?}"
        );
    }

    #[test]
    fn every_household_resource_operation_documents_not_found_response() {
        let document: Value =
            serde_yaml_ng::from_str(include_str!("../../docs/api/openapi.v1.yaml")).unwrap();
        let documented = operations();
        let missing = documented
            .iter()
            .filter(|((_, path), _)| path.starts_with("/households/{household_id}/"))
            .filter(|((method, path), _)| {
                let method = method.to_lowercase();
                let response =
                    &document["paths"][path.as_str()][method.as_str()]["responses"]["404"];
                let shared = response["$ref"] == "#/components/responses/NotFound";
                let inline =
                    response.get("$ref").is_none() && response["description"].as_str().is_some();
                let resolved = if shared {
                    contract::resolve(&document, response)
                } else {
                    response
                };
                !(shared || inline)
                    || resolved["content"]["application/json"]["schema"]["$ref"]
                        != "#/components/schemas/ErrorEnvelope"
            })
            .map(|(_, operation)| operation["operation_id"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "Household resource operations without documented ErrorEnvelope 404 response: {missing:?}"
        );
    }

    #[tokio::test]
    async fn inventory_reconciles_collected_loco_routes() {
        let (fixture, boot) = boot_request_fixture().await;
        let registered = api_v1_routes(&boot);
        let documented = operations();
        let documented_routes = documented
            .keys()
            .map(|(method, path)| (method.clone(), normalized(&format!("/api/v1{path}"))))
            .collect::<BTreeSet<_>>();
        let actual_routes = registered
            .iter()
            .map(|(method, path)| (method.clone(), normalized(path)))
            .collect::<BTreeSet<_>>();
        let missing = documented_routes
            .difference(&actual_routes)
            .cloned()
            .collect::<Vec<_>>();
        let extra = actual_routes
            .difference(&documented_routes)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            missing,
            vec![(
                "POST".to_owned(),
                "/api/v1/households/{}/push_subscription/test".to_owned()
            )],
            "Documented operations without a Loco AppRoutes registration changed"
        );
        assert!(
            extra.is_empty(),
            "Undocumented Loco API v1 routes: {extra:?}"
        );
        drop(registered);
        close_request(fixture, boot).await;
    }

    #[tokio::test]
    async fn anonymous_access() {
        let (fixture, boot) = boot_request_fixture().await;
        let row = boot
            .app_context
            .db
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT current_user, row_security_active('medications') AS rls_active",
            ))
            .await
            .unwrap()
            .unwrap();
        let role: String = row.try_get("", "current_user").unwrap();
        assert_eq!(
            role,
            url::Url::parse(&fixture.runtime_uri).unwrap().username()
        );
        assert!(row.try_get::<bool>("", "rls_active").unwrap());
        let request = server(&boot);
        let response = request
            .get(&format!("/api/v1/households/{}/me", fixture.household.id))
            .await;
        assert_eq!(response.status_code(), 401);
        assert_eq!(response.header("cache-control"), "no-store");
        let identity = request
            .get("/api/auth/get-session")
            .add_header("x-forwarded-for", "198.51.100.77")
            .await;
        assert_eq!(identity.status_code(), 200);
        assert_eq!(fixture.effect().await.0, 0);
        drop(request);
        close_request(fixture, boot).await;
    }

    async fn source_buckets(fixture: &RequestFixture) -> BTreeMap<String, i32> {
        use sea_orm::sea_query::{Alias, Query};
        let query = Query::select()
            .columns([Alias::new("bucket"), Alias::new("attempts")])
            .from(Alias::new("identity_rate_limits"))
            .to_owned();
        fixture
            .admin
            .query_all_raw(fixture.admin.get_database_backend().build(&query))
            .await
            .unwrap()
            .into_iter()
            .map(|row| {
                (
                    row.try_get("", "bucket").unwrap(),
                    row.try_get("", "attempts").unwrap(),
                )
            })
            .collect()
    }

    #[tokio::test]
    async fn auth_boundary_rejects_rotating_spoofed_sources() {
        crate::authentication_fixture::reset_source_quota().await;
        let (fixture, boot) = boot_request_fixture().await;
        let bucket = hex::encode(Sha256::digest("source:127.0.0.1".as_bytes()));
        let before = source_buckets(&fixture).await;
        let request = server(&boot);
        for source in ["198.51.100.77", "203.0.113.77"] {
            let response = request
                .post("/api/auth/security/operation/start")
                .add_header("x-forwarded-for", source)
                .json(&json!({}))
                .await;
            assert_eq!(response.status_code(), 400);
        }
        let after = source_buckets(&fixture).await;
        assert_eq!(
            after.get(&bucket),
            Some(&(before.get(&bucket).copied().unwrap_or(0) + 2))
        );
        for source in ["198.51.100.77", "203.0.113.77"] {
            let spoofed = hex::encode(Sha256::digest(format!("source:{source}").as_bytes()));
            assert_eq!(after.get(&spoofed), before.get(&spoofed));
        }
        assert!(
            after
                .keys()
                .all(|key| key == &bucket || before.contains_key(key))
        );
        drop(request);
        close_request(fixture, boot).await;
    }
}

pub mod rate_limit {
    use super::*;
    use axum::{Extension, extract::ConnectInfo, http::Method};
    use std::net::SocketAddr;

    fn export(fixture: &RequestFixture) -> String {
        format!(
            "/api/v1/households/{}/data_exports/health_data_json",
            fixture.household.id
        )
    }

    #[tokio::test]
    async fn missing_socket_peer_returns_json_unavailable_error() {
        let (fixture, boot) = boot_request_rate_fixture(&[]).await;
        let request = TestServer::new(boot.router.clone().unwrap()).unwrap();
        let response = request.get(&export(&fixture)).await;
        assert_eq!(response.status_code(), 503);
        assert_eq!(response.header("content-type"), "application/json");
        assert_eq!(response.header("cache-control"), "no-store");
        let body: Value = response.json();
        assert_eq!(body["error"]["code"], "service_unavailable");
        drop(request);
        close_request(fixture, boot).await;
    }

    #[tokio::test]
    async fn missing_socket_peer_head_response_has_no_body() {
        let (fixture, boot) = boot_request_rate_fixture(&[]).await;
        let request = TestServer::builder()
            .mock_transport()
            .build(boot.router.clone().unwrap())
            .unwrap();
        let response = request.method(Method::HEAD, &export(&fixture)).await;
        assert_eq!(response.status_code(), 503);
        assert_eq!(response.header("cache-control"), "no-store");
        assert!(response.as_bytes().is_empty());
        drop(request);
        close_request(fixture, boot).await;
    }

    #[tokio::test]
    async fn exhausted_export_head_response_has_no_body() {
        let (fixture, boot) = boot_request_rate_fixture(&[]).await;
        let peer = ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 12345)));
        let request = TestServer::builder()
            .mock_transport()
            .build(boot.router.clone().unwrap().layer(Extension(peer)))
            .unwrap();
        for attempt in 1..=10 {
            let response = request.get(&export(&fixture)).await;
            assert_eq!(response.status_code(), 401, "request {attempt}");
        }
        let response = request.method(Method::HEAD, &export(&fixture)).await;
        assert_eq!(response.status_code(), 429);
        assert_eq!(response.header("retry-after"), "20");
        assert_eq!(response.header("cache-control"), "no-store");
        assert!(response.as_bytes().is_empty());
        drop(request);
        close_request(fixture, boot).await;
    }

    #[tokio::test]
    async fn fixed_export_window_rejects_eleventh_request_with_contract_headers() {
        let (fixture, boot) = boot_request_rate_fixture(&[]).await;
        let request = server(&boot);
        for attempt in 1..=10 {
            let response = request.get(&export(&fixture)).await;
            assert_eq!(response.status_code(), 401, "request {attempt}");
        }
        let before = fixture.effect().await;
        let rejected = request.get(&export(&fixture)).await;
        assert_eq!(rejected.status_code(), 429);
        assert_eq!(rejected.header("retry-after"), "20");
        assert_eq!(rejected.header("ratelimit-limit"), "10");
        assert_eq!(rejected.header("ratelimit-remaining"), "0");
        assert_eq!(rejected.header("ratelimit-reset"), "1020");
        assert_eq!(rejected.header("cache-control"), "no-store");
        assert_eq!(rejected.header("content-type"), "application/json");
        assert!(!rejected.headers().contains_key("idempotency-replayed"));
        let request_id = rejected
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let body: Value = rejected.json();
        assert_eq!(body["error"]["code"], "rate_limited");
        assert!(
            body["error"]["message"]
                .as_str()
                .is_some_and(|message| !message.is_empty())
        );
        assert_eq!(fixture.effect().await, before);
        drop(request);
        close_request(fixture, boot).await;
        let document = contract::contract();
        contract::assert_value(
            document,
            &contract::resolve(
                document,
                &document["paths"]["/households/{household_id}/data_exports/{mode}"]["get"]["responses"]
                    ["429"],
            )["content"]["application/json"]["schema"],
            &body,
            "getDataExport 429",
        );
        assert!(request_id.is_some_and(|id| !id.is_empty()));
    }

    #[tokio::test]
    async fn rejected_export_preserves_sanitized_supplied_request_id() {
        let (fixture, boot) = boot_request_rate_fixture(&[]).await;
        let request = server(&boot);
        for attempt in 1..=10 {
            let response = request.get(&export(&fixture)).await;
            assert_eq!(response.status_code(), 401, "request {attempt}");
        }
        let before = fixture.effect().await;
        let rejected = request
            .get(&export(&fixture))
            .add_header("x-request-id", "client-42 / spoof")
            .await;
        let status = rejected.status_code();
        let retry_after = rejected.header("retry-after").to_owned();
        let limit = rejected.header("ratelimit-limit").to_owned();
        let remaining = rejected.header("ratelimit-remaining").to_owned();
        let reset = rejected.header("ratelimit-reset").to_owned();
        let content_type = rejected.header("content-type").to_owned();
        let cache_control = rejected.header("cache-control").to_owned();
        let request_id = rejected
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let body: Value = rejected.json();
        let after = fixture.effect().await;
        drop(request);
        close_request(fixture, boot).await;
        assert_eq!(status, 429);
        assert_eq!(retry_after, "20");
        assert_eq!(limit, "10");
        assert_eq!(remaining, "0");
        assert_eq!(reset, "1020");
        assert_eq!(content_type, "application/json");
        assert_eq!(cache_control, "no-store");
        let document = contract::contract();
        contract::assert_value(
            document,
            &contract::resolve(
                document,
                &document["paths"]["/households/{household_id}/data_exports/{mode}"]["get"]["responses"]
                    ["429"],
            )["content"]["application/json"]["schema"],
            &body,
            "getDataExport 429 supplied request ID",
        );
        assert_eq!(after, before);
        assert_eq!(request_id.as_deref(), Some("client-42spoof"));
    }

    #[tokio::test]
    async fn disabled_request_id_layer_keeps_rejected_export_without_an_id() {
        let (fixture, boot) = boot_request_rate_fixture_with(&[], Some(false)).await;
        let request = server(&boot);
        for attempt in 1..=10 {
            let response = request.get(&export(&fixture)).await;
            assert_eq!(response.status_code(), 401, "request {attempt}");
            assert!(!response.header("x-request-id").is_empty());
        }
        let before = fixture.effect().await;
        let rejected = request.get(&export(&fixture)).await;
        let status = rejected.status_code();
        let retry_after = rejected.header("retry-after").to_owned();
        let limit = rejected.header("ratelimit-limit").to_owned();
        let remaining = rejected.header("ratelimit-remaining").to_owned();
        let reset = rejected.header("ratelimit-reset").to_owned();
        let cache_control = rejected.header("cache-control").to_owned();
        let content_type = rejected.header("content-type").to_owned();
        let request_id = rejected.headers().get("x-request-id").cloned();
        let body: Value = rejected.json();
        let after = fixture.effect().await;
        drop(request);
        close_request(fixture, boot).await;
        assert_eq!(status, 429);
        assert_eq!(retry_after, "20");
        assert_eq!(limit, "10");
        assert_eq!(remaining, "0");
        assert_eq!(reset, "1020");
        assert_eq!(cache_control, "no-store");
        assert_eq!(content_type, "application/json");
        let document = contract::contract();
        contract::assert_value(
            document,
            &contract::resolve(
                document,
                &document["paths"]["/households/{household_id}/data_exports/{mode}"]["get"]["responses"]
                    ["429"],
            )["content"]["application/json"]["schema"],
            &body,
            "getDataExport 429 disabled request ID",
        );
        assert_eq!(after, before);
        assert!(request_id.is_none());
    }

    #[tokio::test]
    async fn head_export_uses_the_get_export_bucket() {
        let (fixture, boot) = boot_request_rate_fixture(&[]).await;
        let token = fixture.token().await;
        let request = server(&boot);
        let first = request
            .method(Method::HEAD, &export(&fixture))
            .add_header("authorization", format!("Bearer {token}"))
            .await;
        assert_eq!(first.status_code(), 200);
        for attempt in 1..=9 {
            let response = request.get(&export(&fixture)).await;
            assert_eq!(response.status_code(), 401, "request {attempt}");
        }
        let rejected = request
            .method(Method::HEAD, &export(&fixture))
            .add_header("authorization", format!("Bearer {token}"))
            .await;
        assert_eq!(rejected.status_code(), 429);
        assert_eq!(rejected.header("ratelimit-limit"), "10");
        drop(request);
        close_request(fixture, boot).await;
    }

    #[tokio::test]
    async fn untrusted_forwarded_sources_share_the_socket_peer_bucket() {
        let (fixture, boot) = boot_request_rate_fixture(&[]).await;
        let request = server(&boot);
        for attempt in 1..=10 {
            let response = request
                .get(&export(&fixture))
                .add_header("x-forwarded-for", format!("198.51.100.{attempt}"))
                .await;
            assert_eq!(response.status_code(), 401, "request {attempt}");
        }
        let rejected = request
            .get(&export(&fixture))
            .add_header("x-forwarded-for", "203.0.113.11")
            .await;
        assert_eq!(rejected.status_code(), 429);
        assert_eq!(rejected.header("ratelimit-limit"), "10");
        drop(request);
        close_request(fixture, boot).await;
    }

    #[tokio::test]
    async fn configured_loopback_proxy_separates_forwarded_client_buckets() {
        let (fixture, boot) = boot_request_rate_fixture(&["127.0.0.1"]).await;
        let request = server(&boot);
        for attempt in 1..=10 {
            let response = request
                .get(&export(&fixture))
                .add_header("x-forwarded-for", "198.51.100.20")
                .await;
            assert_eq!(response.status_code(), 401, "request {attempt}");
        }
        let other_client = request
            .get(&export(&fixture))
            .add_header("x-forwarded-for", "203.0.113.20")
            .await;
        assert_eq!(other_client.status_code(), 401);
        let malformed_chain = request
            .get(&export(&fixture))
            .add_header("x-forwarded-for", "198.51.100.20, invalid")
            .await;
        assert_eq!(malformed_chain.status_code(), 401);
        for attempt in 1..=9 {
            let peer = request.get(&export(&fixture)).await;
            assert_eq!(peer.status_code(), 401, "socket peer request {attempt}");
        }
        let exhausted_peer = request.get(&export(&fixture)).await;
        assert_eq!(exhausted_peer.status_code(), 429);
        let exhausted = request
            .get(&export(&fixture))
            .add_header("x-forwarded-for", "198.51.100.20")
            .await;
        assert_eq!(exhausted.status_code(), 429);
        assert_eq!(exhausted.header("ratelimit-reset"), "1020");
        drop(request);
        close_request(fixture, boot).await;
    }
}

enum StoredRow {
    Session(i64),
    App(i64),
    Grant(i64),
}

async fn overlap_requests_at_row_lock<A, B>(
    fixture: &RequestFixture,
    target: StoredRow,
    wait_query_pattern: &str,
    first: A,
    second: B,
) -> (A::Output, B::Output, bool)
where
    A: std::future::Future,
    B: std::future::Future,
{
    let blocker = fixture.admin.begin().await.unwrap();
    let row = blocker
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_backend_pid() AS pid",
        ))
        .await
        .unwrap()
        .unwrap();
    let blocker_pid = row.try_get::<i32>("", "pid").unwrap();
    use med_tracker::models::_entities::{api_app_tokens, api_sessions, oauth_grants};
    use sea_orm::{EntityTrait, QuerySelect};
    match target {
        StoredRow::Session(id) => {
            api_sessions::Entity::find_by_id(id)
                .lock_exclusive()
                .one(&blocker)
                .await
                .unwrap()
                .unwrap();
        }
        StoredRow::App(id) => {
            api_app_tokens::Entity::find_by_id(id)
                .lock_exclusive()
                .one(&blocker)
                .await
                .unwrap()
                .unwrap();
        }
        StoredRow::Grant(id) => {
            oauth_grants::Entity::find_by_id(id)
                .lock_exclusive()
                .one(&blocker)
                .await
                .unwrap()
                .unwrap();
        }
    }
    let runtime_role = url::Url::parse(&fixture.runtime_uri)
        .unwrap()
        .username()
        .to_owned();
    let release = async {
        let observed = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let row = fixture.admin.query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "SELECT count(DISTINCT pid) AS waiting, count(DISTINCT pid) FILTER (WHERE $2::int=ANY(pg_blocking_pids(pid))) AS held_waiters FROM pg_stat_activity WHERE datname=current_database() AND usename=$1 AND wait_event_type='Lock' AND query LIKE $3 AND cardinality(pg_blocking_pids(pid))>0",
                    [runtime_role.clone().into(), blocker_pid.into(), wait_query_pattern.into()],
                )).await.unwrap().unwrap();
                if row.try_get::<i64>("", "waiting").unwrap() == 2
                    && row.try_get::<i64>("", "held_waiters").unwrap() >= 1
                {
                    break true;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap_or(false);
        blocker.commit().await.unwrap();
        observed
    };
    tokio::join!(first, second, release)
}

pub mod account {
    use super::*;
    use axum::http::Method;
    use med_tracker::models::_entities::{
        account_lockouts, accounts, api_app_tokens, api_sessions, households, oauth_grants,
        security_audit_events, versions,
    };
    use sea_orm::sea_query::Expr;
    use sea_orm::{
        ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter,
        QueryOrder, QuerySelect, Set,
    };

    fn assert_response_schema(method: &str, path: &str, status: u16, body: &Value) {
        let spec_path = match (method, path) {
            ("get", "/api/v1/auth/households") => "/auth/households",
            ("get", "/api/v1/auth/sessions") => "/auth/sessions",
            ("delete", "/api/v1/auth/logout") => "/auth/logout",
            ("delete", path) if path.starts_with("/api/v1/auth/sessions/") => "/auth/sessions/{id}",
            ("get", "/api/v1/auth/profile") => "/auth/profile",
            ("patch", "/api/v1/auth/profile") => "/auth/profile",
            ("get", "/api/v1/auth/profile/avatar") => "/auth/profile/avatar",
            ("put", "/api/v1/auth/profile/avatar") => "/auth/profile/avatar",
            ("delete", "/api/v1/auth/profile/avatar") => "/auth/profile/avatar",
            ("get", "/api/v1/auth/guardian-declarations") => "/auth/guardian-declarations",
            ("patch", path) if path.starts_with("/api/v1/auth/guardian-declarations/") => {
                "/auth/guardian-declarations/{id}"
            }
            _ => panic!("undocumented account request {method} {path}"),
        };
        let document = contract::contract();
        let response = &document["paths"][spec_path][method]["responses"][status.to_string()];
        let response = if response.get("$ref").is_some() {
            contract::resolve(document, response)
        } else {
            response
        };
        contract::assert_value(
            document,
            &response["content"]["application/json"]["schema"],
            body,
            &format!("{method} {spec_path} {status}"),
        );
    }

    pub(super) async fn mobile_token(fixture: &RequestFixture) -> String {
        fixture.token().await
    }
    async fn stored_credentials(fixture: &RequestFixture) {
        let now = chrono::Utc::now().naive_utc();
        let current = fixture
            .session(&fixture.credential("synthetic-current-session"))
            .await;
        let mut current = current.into_active_model();
        current.device_name = Set(Some("Current phone".into()));
        current.created_at = Set(now - chrono::Duration::days(1));
        current.update(&fixture.admin).await.unwrap();
        let expired = fixture
            .session(&fixture.credential("synthetic-expired-session"))
            .await;
        let mut expired = expired.into_active_model();
        expired.device_name = Set(Some("Expired phone".into()));
        expired.created_at = Set(now - chrono::Duration::days(2));
        expired.access_expires_at = Set(now - chrono::Duration::hours(1));
        expired.refresh_expires_at = Set(now - chrono::Duration::minutes(1));
        expired.last_used_at = Set(now - chrono::Duration::days(2));
        expired.update(&fixture.admin).await.unwrap();
        fixture
            .app_token(&fixture.credential("synthetic-app-token"))
            .await;
    }
    async fn foreign_session(fixture: &RequestFixture) -> api_sessions::Model {
        let now = chrono::Utc::now().naive_utc();
        api_sessions::ActiveModel {
            account_id: Set(fixture.foreign.account.id),
            household_membership_id: Set(Some(fixture.foreign.membership.id)),
            access_token_digest: Set(uuid::Uuid::new_v4().to_string()),
            refresh_token_digest: Set(uuid::Uuid::new_v4().to_string()),
            permissions_version: Set(1),
            access_expires_at: Set(now + chrono::Duration::minutes(15)),
            refresh_expires_at: Set(now + chrono::Duration::days(1)),
            last_used_at: Set(now),
            device_name: Set(Some("Foreign device".into())),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&fixture.admin)
        .await
        .unwrap()
    }
    async fn version_count(
        fixture: &RequestFixture,
        item_type: &str,
        account_id: i64,
        event: &str,
    ) -> u64 {
        versions::Entity::find()
            .filter(versions::Column::ItemType.eq(item_type))
            .filter(versions::Column::ItemId.eq(account_id))
            .filter(versions::Column::Event.eq(event))
            .count(&fixture.admin)
            .await
            .unwrap()
    }
    async fn revocation_counts(fixture: &RequestFixture) -> (u64, u64, u64) {
        (
            version_count(
                fixture,
                "AuthenticationToken",
                fixture.account.id,
                "auth_token/api_session/revoked",
            )
            .await,
            version_count(
                fixture,
                "AuthenticationToken",
                fixture.account.id,
                "auth_token/api_app_token/revoked",
            )
            .await,
            version_count(
                fixture,
                "Account",
                fixture.account.id,
                "mobile_oauth.revoked",
            )
            .await,
        )
    }
    async fn current_grant(fixture: &RequestFixture) -> oauth_grants::Model {
        oauth_grants::Entity::find_by_id(fixture.authorization.id)
            .one(&fixture.admin)
            .await
            .unwrap()
            .unwrap()
    }
    async fn aged_state(
        fixture: &RequestFixture,
        ids: [i64; 2],
    ) -> Vec<(i64, chrono::NaiveDateTime)> {
        oauth_grants::Entity::find()
            .filter(oauth_grants::Column::Id.is_in(ids))
            .order_by_asc(oauth_grants::Column::Id)
            .all(&fixture.admin)
            .await
            .unwrap()
            .into_iter()
            .map(|row| (row.id, row.last_used_at.unwrap()))
            .collect()
    }

    #[derive(Clone, Copy)]
    enum DenialChange {
        SessionRevoked,
        AppRevoked,
        AppExpired,
        AppOverage,
        Permissions,
        MembershipRevoked,
        Lockout,
    }
    async fn authority_change(fixture: &RequestFixture, change: DenialChange, restore: bool) {
        let now = chrono::Utc::now().naive_utc();
        match change {
            DenialChange::SessionRevoked => fixture.revoke(CredentialKind::Session, !restore).await,
            DenialChange::AppRevoked => fixture.revoke(CredentialKind::App, !restore).await,
            DenialChange::Permissions => {
                fixture
                    .membership_state(if restore { 1 } else { 2 }, false)
                    .await
            }
            DenialChange::MembershipRevoked => fixture.membership_state(1, !restore).await,
            DenialChange::Lockout => {
                if restore {
                    account_lockouts::Entity::delete_by_id(fixture.account.id)
                        .exec(&fixture.admin)
                        .await
                        .unwrap();
                } else {
                    account_lockouts::ActiveModel {
                        account_id: Set(fixture.account.id),
                        key: Set(fixture.credential("account-route-test")),
                        deadline: Set(now + chrono::Duration::hours(1)),
                        created_at: Set(Some(now)),
                        updated_at: Set(Some(now)),
                        ..Default::default()
                    }
                    .insert(&fixture.admin)
                    .await
                    .unwrap();
                }
            }
            DenialChange::AppExpired => {
                api_app_tokens::Entity::update_many()
                    .col_expr(
                        api_app_tokens::Column::CreatedAt,
                        Expr::value(if restore {
                            now
                        } else {
                            now - chrono::Duration::days(2)
                        }),
                    )
                    .col_expr(
                        api_app_tokens::Column::ExpiresAt,
                        Expr::value(if restore {
                            now.checked_add_months(chrono::Months::new(1)).unwrap()
                        } else {
                            now - chrono::Duration::days(1)
                        }),
                    )
                    .filter(api_app_tokens::Column::AccountId.eq(fixture.account.id))
                    .exec(&fixture.admin)
                    .await
                    .unwrap();
            }
            DenialChange::AppOverage => {
                api_app_tokens::Entity::update_many()
                    .col_expr(
                        api_app_tokens::Column::CreatedAt,
                        Expr::value(if restore {
                            now
                        } else {
                            now.checked_sub_months(chrono::Months::new(13)).unwrap()
                        }),
                    )
                    .filter(api_app_tokens::Column::AccountId.eq(fixture.account.id))
                    .exec(&fixture.admin)
                    .await
                    .unwrap();
            }
        }
    }

    fn account_surface_routes(fixture: &RequestFixture) -> Vec<(Method, String, Option<Value>)> {
        vec![
            (Method::GET, "/api/v1/auth/households".into(), None),
            (Method::GET, "/api/v1/auth/sessions".into(), None),
            (
                Method::DELETE,
                format!("/api/v1/auth/sessions/{}", fixture.authorization.id),
                None,
            ),
            (Method::GET, "/api/v1/auth/profile".into(), None),
            (
                Method::PATCH,
                "/api/v1/auth/profile".into(),
                Some(json!({"profile":{"name":"Surface sweep"}})),
            ),
            (Method::GET, "/api/v1/auth/profile/avatar".into(), None),
            (Method::PUT, "/api/v1/auth/profile/avatar".into(), None),
            (Method::DELETE, "/api/v1/auth/profile/avatar".into(), None),
            (
                Method::GET,
                "/api/v1/auth/guardian-declarations".into(),
                None,
            ),
            (
                Method::PATCH,
                "/api/v1/auth/guardian-declarations/1".into(),
                Some(json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}})),
            ),
        ]
    }

    #[test]
    fn documented_account_scope_denial_uses_declared_forbidden_components() {
        let document: Value =
            serde_yaml_ng::from_str(include_str!("../../docs/api/openapi.v1.yaml")).unwrap();
        for (method, path) in [
            ("get", "/auth/households"),
            ("get", "/auth/sessions"),
            ("delete", "/auth/sessions/{id}"),
            ("get", "/auth/profile"),
            ("patch", "/auth/profile"),
            ("get", "/auth/profile/avatar"),
            ("put", "/auth/profile/avatar"),
            ("delete", "/auth/profile/avatar"),
        ] {
            assert_eq!(
                document["paths"][path][method]["responses"]["403"]["$ref"],
                "#/components/responses/AccountForbidden",
                "{method} {path}"
            );
        }
        for (method, path) in [
            ("get", "/auth/guardian-declarations"),
            ("patch", "/auth/guardian-declarations/{id}"),
        ] {
            assert_eq!(
                document["paths"][path][method]["responses"]["403"]["$ref"],
                "#/components/responses/GuardianDeclarationForbidden",
                "{method} {path}"
            );
        }
        let challenge =
            &document["components"]["responses"]["AccountForbidden"]["headers"]["WWW-Authenticate"];
        assert_eq!(challenge["required"], true);
        assert_eq!(challenge["schema"]["type"], "string");
        assert_eq!(
            challenge["example"],
            "Bearer error=\"insufficient_scope\" scope=\"medtracker\""
        );
        for component in ["AccountForbidden", "GuardianDeclarationForbidden"] {
            let forbidden = &document["components"]["responses"][component];
            assert_eq!(
                forbidden["content"]["application/json"]["schema"]["$ref"],
                "#/components/schemas/ErrorEnvelope",
                "{component}"
            );
            assert_eq!(
                forbidden["headers"]["WWW-Authenticate"]["schema"]["type"], "string",
                "{component}"
            );
        }
    }

    #[tokio::test]
    async fn malformed_delete_session_authenticates_before_identifier_validation() {
        let (fixture, boot) = boot_request_fixture().await;

        let request = server(&boot);
        let token = mobile_token(&fixture).await;
        let mut responses = Vec::new();
        for (label, id) in [
            ("nonnumeric", "not-a-number"),
            ("overflow", "9223372036854775808"),
            ("invalid utf8", "%FF"),
            ("zero", "0"),
            ("negative", "-1"),
        ] {
            let path = format!("/api/v1/auth/sessions/{id}");
            let anonymous = request.method(Method::DELETE, &path).await;
            let authenticated = request
                .method(Method::DELETE, &path)
                .add_header("authorization", format!("Bearer {token}"))
                .await;
            responses.push((label, anonymous, authenticated));
        }

        drop(request);
        close_request(fixture, boot).await;

        let statuses = responses
            .iter()
            .map(|(label, anonymous, authenticated)| {
                (
                    *label,
                    anonymous.status_code().as_u16(),
                    authenticated.status_code().as_u16(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            statuses,
            vec![
                ("nonnumeric", 401, 404),
                ("overflow", 401, 404),
                ("invalid utf8", 401, 404),
                ("zero", 401, 404),
                ("negative", 401, 404),
            ]
        );
        for (_, anonymous, authenticated) in responses {
            assert_eq!(anonymous.header("content-type"), "application/json");
            assert_eq!(anonymous.header("cache-control"), "no-store");
            assert_eq!(anonymous.header("www-authenticate"), "Bearer");
            let request_id_header = anonymous.header("x-request-id");
            let request_id = request_id_header.to_str().unwrap();
            assert!(!request_id.is_empty());
            let body: Value = anonymous.json();
            assert_eq!(body["error"]["code"], "unauthorized");
            assert_eq!(body["error"]["request_id"], request_id);
            assert_response_schema("delete", "/api/v1/auth/sessions/1", 401, &body);

            assert_eq!(authenticated.header("content-type"), "application/json");
            assert_eq!(authenticated.header("cache-control"), "no-store");
            let request_id_header = authenticated.header("x-request-id");
            let request_id = request_id_header.to_str().unwrap();
            assert!(!request_id.is_empty());
            let body: Value = authenticated.json();
            assert_eq!(body["error"]["code"], "not_found");
            assert_eq!(body["error"]["request_id"], request_id);
            assert_response_schema("delete", "/api/v1/auth/sessions/1", 404, &body);
        }
    }

    #[tokio::test]
    async fn account_management_authentication_envelopes_match_each_route() {
        let (fixture, boot) = boot_request_fixture().await;

        let token = mobile_token(&fixture).await;
        let request = server(&boot);
        let mut anonymous = Vec::new();
        for (method, path, body) in account_surface_routes(&fixture) {
            let pending = request.method(method.clone(), &path);
            let pending = match &body {
                Some(body) => pending.json(body),
                None => pending,
            };
            anonymous.push((method.as_str().to_lowercase(), path, pending.await));
        }
        fixture.wrong_scope().await;
        let mut insufficient = Vec::new();
        for (method, path, body) in account_surface_routes(&fixture) {
            let pending = request
                .method(method.clone(), &path)
                .add_header("authorization", format!("Bearer {token}"));
            let pending = match &body {
                Some(body) => pending.json(body),
                None => pending,
            };
            insufficient.push((method.as_str().to_lowercase(), path, pending.await));
        }
        let active = current_grant(&fixture).await.revoked_at.is_none();
        drop(request);
        close_request(fixture, boot).await;

        assert!(active);
        for (method, path, response) in anonymous {
            assert_eq!(response.status_code(), 401, "{method} {path}");
            assert_eq!(response.header("content-type"), "application/json");
            assert_eq!(response.header("cache-control"), "no-store");
            assert_eq!(response.header("www-authenticate"), "Bearer");
            let request_id = response.header("x-request-id").to_str().unwrap().to_owned();
            assert!(!request_id.is_empty());
            let body: Value = response.json();
            assert_eq!(body["error"]["code"], "unauthorized");
            assert_eq!(body["error"]["request_id"], request_id);
            assert_response_schema(&method, &path, 401, &body);
        }
        for (method, path, response) in insufficient {
            assert_eq!(response.status_code(), 403, "{method} {path}");
            assert_eq!(response.header("content-type"), "application/json");
            assert_eq!(response.header("cache-control"), "no-store");
            assert_eq!(
                response.header("www-authenticate"),
                "Bearer error=\"insufficient_scope\" scope=\"medtracker\""
            );
            let request_id = response.header("x-request-id").to_str().unwrap().to_owned();
            assert!(!request_id.is_empty());
            let body: Value = response.json();
            assert_eq!(body["error"]["code"], "forbidden");
            assert_eq!(body["error"]["request_id"], request_id);
            assert_response_schema(&method, &path, 403, &body);
        }
    }

    #[tokio::test]
    async fn account_collections_keep_household_scope_and_session_secrets_out() {
        let (fixture, boot) = boot_request_fixture().await;
        let account_id = fixture.account.id;

        let household_id = fixture.household.id;
        let grant_id = fixture.authorization.id;
        let foreign_household_id = fixture.foreign.household.id;
        let session = fixture.credential("synthetic-current-session");
        let expired_session = fixture.credential("synthetic-expired-session");
        let app = fixture.credential("synthetic-app-token");
        stored_credentials(&fixture).await;
        let current_id = fixture.session_id("synthetic-current-session").await;
        let expired_id = fixture.session_id("synthetic-expired-session").await;
        let (additional, additional_membership) = super::household_selection::home(
            &fixture.scenario,
            "Second household",
            "second-household",
            "member",
            false,
        )
        .await
        .unwrap();
        let additional_id = additional.id;
        let additional_member_id = additional_membership.id;
        let (held, _) = super::household_selection::home(
            &fixture.scenario,
            "Held household",
            "held-household",
            "member",
            false,
        )
        .await
        .unwrap();
        households::Entity::update_many()
            .col_expr(households::Column::LifecycleState, Expr::value("held"))
            .filter(households::Column::Id.eq(held.id))
            .exec(&fixture.admin)
            .await
            .unwrap();
        let foreign_session = foreign_session(&fixture).await;
        let foreign_session_id = foreign_session.id;
        let mobile = mobile_token(&fixture).await;
        let aged_maximum = fixture.credential("synthetic-mobile-over-maximum");
        let aged_inactivity = fixture.credential("synthetic-mobile-over-inactivity");
        let maximum = fixture
            .mobile_login("synthetic-mobile-over-maximum", 31, 0, false)
            .await;
        let inactivity = fixture
            .mobile_login("synthetic-mobile-over-inactivity", 9, 8, false)
            .await;
        let current_refresh = api_sessions::Entity::find_by_id(current_id)
            .one(&fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .refresh_token_digest;
        let request = server(&boot);
        let mut collections = Vec::new();
        for bearer in [session.as_str(), app.as_str()] {
            let households = request
                .get("/api/v1/auth/households")
                .add_header("authorization", format!("Bearer {bearer}"))
                .await;
            let sessions = request
                .get("/api/v1/auth/sessions")
                .add_header("authorization", format!("Bearer {bearer}"))
                .await;
            collections.push((households, sessions));
        }
        let mobile_households = request
            .get("/api/v1/auth/households")
            .add_header("authorization", format!("Bearer {mobile}"))
            .await;
        oauth_grants::Entity::update_many()
            .col_expr(
                oauth_grants::Column::LastUsedAt,
                Expr::value(chrono::Utc::now().naive_utc() - chrono::Duration::days(1)),
            )
            .filter(oauth_grants::Column::Id.eq(fixture.authorization.id))
            .exec(&fixture.admin)
            .await
            .unwrap();
        let before_row = current_grant(&fixture).await;
        let mobile_before = before_row.last_used_at.unwrap();
        let mobile_authenticated = before_row.authenticated_at.unwrap();
        let mobile_sessions = request
            .get("/api/v1/auth/sessions")
            .add_header("authorization", format!("Bearer {mobile}"))
            .await;
        let mobile_after = current_grant(&fixture).await.last_used_at.unwrap();
        let session_settings = &boot.app_context.config.settings.as_ref().unwrap()["session"];
        let inactivity_days = session_settings["inactivity_timeout_days"]
            .as_i64()
            .unwrap();
        let maximum_days = session_settings["max_age_days"].as_i64().unwrap();
        let aged_before = aged_state(&fixture, [maximum.id, inactivity.id]).await;
        let mut aged_denials = Vec::new();
        for (label, bearer) in [
            ("maximum age", aged_maximum.as_str()),
            ("inactivity", aged_inactivity.as_str()),
        ] {
            for (method, path, body) in account_surface_routes(&fixture) {
                let pending = request
                    .method(method.clone(), &path)
                    .add_header("authorization", format!("Bearer {bearer}"));
                let pending = match &body {
                    Some(body) => pending.json(body),
                    None => pending,
                };
                aged_denials.push((label, method.as_str().to_lowercase(), path, pending.await));
            }
        }
        let aged_after = aged_state(&fixture, [maximum.id, inactivity.id]).await;
        drop(request);
        close_request(fixture, boot).await;

        for (households, sessions) in collections {
            assert_eq!(households.status_code(), 200);
            assert_eq!(sessions.status_code(), 200);
            for response in [&households, &sessions] {
                assert_eq!(response.header("content-type"), "application/json");
                assert_eq!(response.header("cache-control"), "no-store");
                assert!(!response.header("x-request-id").to_str().unwrap().is_empty());
            }
            let households: Value = households.json();
            assert_response_schema("get", "/api/v1/auth/households", 200, &households);
            assert_eq!(households["account_id"], account_id);
            assert_eq!(households["data"].as_array().unwrap().len(), 2);
            assert_eq!(households["data"][0]["id"], household_id);
            assert_eq!(households["data"][1]["id"], additional_id);
            assert_eq!(households["data"][1]["membership_id"], additional_member_id);
            assert!(
                households["data"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|row| row["id"] != foreign_household_id)
            );
            let sessions: Value = sessions.json();
            assert_response_schema("get", "/api/v1/auth/sessions", 200, &sessions);
            assert_eq!(sessions["data"].as_array().unwrap().len(), 2);
            assert_eq!(sessions["data"][0]["id"], current_id);
            assert_eq!(sessions["data"][1]["id"], expired_id);
            assert!(
                sessions["data"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|row| row["id"] != foreign_session_id)
            );
            for secret in [
                session.as_str(),
                current_refresh.as_str(),
                expired_session.as_str(),
            ] {
                assert!(!sessions.to_string().contains(secret));
            }
        }
        assert_eq!(mobile_households.status_code(), 200);
        assert_eq!(mobile_sessions.status_code(), 200);
        let mobile_households: Value = mobile_households.json();
        assert_response_schema("get", "/api/v1/auth/households", 200, &mobile_households);
        assert_eq!(mobile_households["data"].as_array().unwrap().len(), 2);
        assert!(
            mobile_households["data"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| row["id"] != foreign_household_id)
        );
        let mobile_sessions: Value = mobile_sessions.json();
        assert_response_schema("get", "/api/v1/auth/sessions", 200, &mobile_sessions);
        assert_eq!(mobile_sessions["data"].as_array().unwrap().len(), 1);
        assert_eq!(mobile_sessions["data"][0]["id"], grant_id);
        assert!(mobile_sessions["data"][0]["household_id"].is_null());
        assert!(mobile_after > mobile_before);
        let stamp = |time| {
            chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(time, chrono::Utc)
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        };
        assert_eq!(
            mobile_sessions["data"][0]["last_used_at"],
            stamp(mobile_after)
        );
        let inactivity_deadline = mobile_after + chrono::Duration::days(inactivity_days);
        let refresh_deadline = if maximum_days == 0 {
            inactivity_deadline
        } else {
            inactivity_deadline.min(mobile_authenticated + chrono::Duration::days(maximum_days))
        };
        assert_eq!(
            mobile_sessions["data"][0]["refresh_token_expires_at"],
            stamp(refresh_deadline)
        );
        assert_eq!(aged_before.len(), 2);
        assert_eq!(aged_after, aged_before);
        for (label, method, path, response) in aged_denials {
            assert_eq!(response.status_code(), 401, "{label}: {method} {path}");
            assert_eq!(response.header("content-type"), "application/json");
            assert_eq!(response.header("cache-control"), "no-store");
            assert_eq!(response.header("www-authenticate"), "Bearer");
            let body: Value = response.json();
            assert_eq!(body["error"]["code"], "unauthorized");
            assert_response_schema(&method, &path, 401, &body);
        }
    }

    #[tokio::test]
    async fn mobile_sessions_hide_foreign_and_expired_login_targets_from_listing_and_revocation() {
        let (fixture, boot) = boot_request_fixture().await;
        let grant_id = fixture.authorization.id;

        let mobile = mobile_token(&fixture).await;
        let foreign = RequestFixture::new().await;
        let foreign_grant = foreign
            .mobile_login("synthetic-foreign-mobile", 0, 0, false)
            .await;
        let foreign_grant_id = foreign_grant.id;
        let maximum = fixture
            .mobile_login("synthetic-aged-mobile-maximum", 31, 0, false)
            .await;
        let maximum_id = maximum.id;
        let inactivity = fixture
            .mobile_login("synthetic-aged-mobile-inactivity", 9, 8, false)
            .await;
        let inactivity_id = inactivity.id;
        let access_expired_grant = fixture
            .mobile_login("synthetic-access-expired-login-active", 0, 0, true)
            .await;
        let access_expired_id = access_expired_grant.id;
        let request = server(&boot);
        let listed = request
            .get("/api/v1/auth/sessions")
            .add_header("authorization", format!("Bearer {mobile}"))
            .await;
        let mut denials = Vec::new();
        for (label, id) in [
            ("foreign account", foreign_grant_id),
            ("maximum login age", maximum_id),
            ("login inactivity", inactivity_id),
        ] {
            denials.push((
                label,
                request
                    .method(Method::DELETE, &format!("/api/v1/auth/sessions/{id}"))
                    .add_header("authorization", format!("Bearer {mobile}"))
                    .await,
            ));
        }
        let access_expired = request
            .method(
                Method::DELETE,
                format!("/api/v1/auth/sessions/{access_expired_id}").as_str(),
            )
            .add_header("authorization", format!("Bearer {mobile}"))
            .await;
        let rows = oauth_grants::Entity::find()
            .filter(oauth_grants::Column::Id.is_in([
                maximum.id,
                inactivity.id,
                foreign_grant.id,
                access_expired_grant.id,
            ]))
            .order_by_asc(oauth_grants::Column::Id)
            .all(&fixture.admin)
            .await
            .unwrap();
        let active = rows
            .into_iter()
            .map(|row| (row.id, row.revoked_at.is_none()))
            .collect::<Vec<_>>();
        let audits = (
            version_count(
                &fixture,
                "Account",
                fixture.account.id,
                "mobile_oauth.revoked",
            )
            .await,
            version_count(
                &fixture,
                "Account",
                foreign.account.id,
                "mobile_oauth.revoked",
            )
            .await,
        );
        foreign.close().await;
        drop(request);
        close_request(fixture, boot).await;

        assert_eq!(listed.status_code(), 200);
        let list: Value = listed.json();
        let listed_ids = list["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|session| session["id"].as_i64().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(listed_ids, BTreeSet::from([grant_id, access_expired_id]));
        let statuses = denials
            .iter()
            .map(|(label, response)| (*label, response.status_code().as_u16()))
            .collect::<Vec<_>>();
        assert_eq!(
            statuses,
            [
                ("foreign account", 404),
                ("maximum login age", 404),
                ("login inactivity", 404)
            ]
        );
        for (label, response) in denials {
            assert_eq!(
                response.header("content-type"),
                "application/json",
                "{label}"
            );
            assert_eq!(response.header("cache-control"), "no-store", "{label}");
            let request_id = response.header("x-request-id").to_str().unwrap().to_owned();
            let body: Value = response.json();
            assert_eq!(body["error"]["code"], "not_found", "{label}");
            assert_eq!(body["error"]["request_id"], request_id, "{label}");
            assert_response_schema("delete", "/api/v1/auth/sessions/1", 404, &body);
        }
        assert_eq!(access_expired.status_code(), 204);
        assert!(access_expired.as_bytes().is_empty());
        assert_eq!(access_expired.header("cache-control"), "no-store");
        let mut expected_active = vec![
            (maximum_id, true),
            (inactivity_id, true),
            (foreign_grant_id, true),
            (access_expired_id, false),
        ];
        expected_active.sort_unstable_by_key(|row| row.0);
        assert_eq!(active, expected_active);
        assert_eq!(audits, (1, 0));
    }

    #[tokio::test]
    async fn account_routes_reject_invalid_revoked_and_stale_credentials_without_writes() {
        let (fixture, boot) = boot_request_fixture().await;
        let grant_id = fixture.authorization.id;
        let session = fixture.credential("synthetic-current-session");
        let expired_session = fixture.credential("synthetic-expired-session");
        let app = fixture.credential("synthetic-app-token");
        stored_credentials(&fixture).await;
        let mobile = mobile_token(&fixture).await;
        let request = server(&boot);
        let mut denied = Vec::new();
        for (label, bearer) in [
            ("invalid", "unknown"),
            ("expired", expired_session.as_str()),
        ] {
            for (method, path, body) in account_surface_routes(&fixture) {
                let pending = request
                    .method(method.clone(), &path)
                    .add_header("authorization", format!("Bearer {bearer}"));
                let pending = match &body {
                    Some(body) => pending.json(body),
                    None => pending,
                };
                denied.push((label, method.as_str().to_lowercase(), path, pending.await));
            }
        }
        for (label, authorization) in [
            ("basic authorization", "Basic cGFzc3dvcmQ="),
            ("missing bearer value", "Bearer"),
            ("empty bearer value", "Bearer "),
        ] {
            for (method, path, body) in account_surface_routes(&fixture) {
                let pending = request
                    .method(method.clone(), &path)
                    .add_header("authorization", authorization);
                let pending = match &body {
                    Some(body) => pending.json(body),
                    None => pending,
                };
                denied.push((label, method.as_str().to_lowercase(), path, pending.await));
            }
        }
        let changes = [
            (
                "revoked session",
                session.as_str(),
                DenialChange::SessionRevoked,
            ),
            ("revoked app token", app.as_str(), DenialChange::AppRevoked),
            ("expired app token", app.as_str(), DenialChange::AppExpired),
            ("overage app token", app.as_str(), DenialChange::AppOverage),
            (
                "stale session permissions",
                session.as_str(),
                DenialChange::Permissions,
            ),
            (
                "stale app token permissions",
                app.as_str(),
                DenialChange::Permissions,
            ),
            (
                "revoked membership",
                session.as_str(),
                DenialChange::MembershipRevoked,
            ),
            ("locked account", session.as_str(), DenialChange::Lockout),
            (
                "locked account app token",
                app.as_str(),
                DenialChange::Lockout,
            ),
            (
                "locked account mobile",
                mobile.as_str(),
                DenialChange::Lockout,
            ),
        ];
        let mut mobile_without_membership = Vec::new();
        for (label, bearer, change) in changes {
            authority_change(&fixture, change, false).await;
            for (method, path, body) in account_surface_routes(&fixture) {
                let pending = request
                    .method(method.clone(), &path)
                    .add_header("authorization", format!("Bearer {bearer}"));
                let pending = match &body {
                    Some(body) => pending.json(body),
                    None => pending,
                };
                denied.push((label, method.as_str().to_lowercase(), path, pending.await));
            }
            if label == "stale session permissions" || label == "revoked membership" {
                mobile_without_membership.push((
                    label,
                    request
                        .get("/api/v1/auth/households")
                        .add_header("authorization", format!("Bearer {mobile}"))
                        .await,
                    request
                        .get("/api/v1/auth/sessions")
                        .add_header("authorization", format!("Bearer {mobile}"))
                        .await,
                ));
            }
            authority_change(&fixture, change, true).await;
        }
        fixture.revoke(CredentialKind::Mobile, true).await;
        for (method, path, body) in account_surface_routes(&fixture) {
            let pending = request
                .method(method.clone(), &path)
                .add_header("authorization", format!("Bearer {mobile}"));
            let pending = match &body {
                Some(body) => pending.json(body),
                None => pending,
            };
            denied.push((
                "revoked mobile",
                method.as_str().to_lowercase(),
                path,
                pending.await,
            ));
        }
        let revocations = version_count(
            &fixture,
            "AuthenticationToken",
            fixture.account.id,
            "auth_token/api_session/revoked",
        )
        .await;
        let active =
            api_sessions::Entity::find_by_id(fixture.session_id("synthetic-current-session").await)
                .one(&fixture.admin)
                .await
                .unwrap()
                .unwrap()
                .revoked_at
                .is_none();
        drop(request);
        close_request(fixture, boot).await;

        assert_eq!(revocations, 0);
        assert!(active);
        for (label, method, path, response) in denied {
            assert_eq!(response.status_code(), 401, "{label}: {method} {path}");
            assert_eq!(response.header("content-type"), "application/json");
            assert_eq!(response.header("cache-control"), "no-store");
            assert_eq!(response.header("www-authenticate"), "Bearer");
            let request_id = response.header("x-request-id").to_str().unwrap().to_owned();
            assert!(!request_id.is_empty());
            let body: Value = response.json();
            assert_eq!(body["error"]["code"], "unauthorized");
            assert_eq!(body["error"]["request_id"], request_id);
            assert_response_schema(&method, &path, 401, &body);
        }
        for (label, households, sessions) in mobile_without_membership {
            assert_eq!(households.status_code(), 200, "{label}: households");
            assert_eq!(sessions.status_code(), 200, "{label}: sessions");
            let household_body: Value = households.json();
            let session_body: Value = sessions.json();
            if label == "revoked membership" {
                assert!(household_body["data"].as_array().unwrap().is_empty());
            }
            assert_eq!(session_body["data"][0]["id"], grant_id);
        }
    }

    #[tokio::test]
    async fn account_routes_share_the_fixed_general_limit() {
        let (fixture, boot) = boot_request_rate_fixture(&[]).await;

        let request = server(&boot);
        for attempt in 1..=300 {
            let response = request.get("/api/v1/capabilities").await;
            assert_eq!(response.status_code(), 200, "request {attempt}");
        }
        let mut denied = Vec::new();
        for (method, path, _) in account_surface_routes(&fixture) {
            denied.push((
                method.as_str().to_lowercase(),
                path.clone(),
                request.method(method, &path).await,
            ));
        }
        denied.push((
            "delete".to_owned(),
            "/api/v1/auth/logout".to_owned(),
            request.method(Method::DELETE, "/api/v1/auth/logout").await,
        ));
        drop(request);
        close_request(fixture, boot).await;
        for (method, path, response) in denied {
            assert_eq!(response.status_code(), 429, "{method} {path}");
            assert_eq!(response.header("content-type"), "application/json");
            assert_eq!(response.header("cache-control"), "no-store");
            assert_eq!(response.header("ratelimit-limit"), "300");
            assert_eq!(response.header("ratelimit-remaining"), "0");
            assert_eq!(response.header("ratelimit-reset"), "1200");
            assert_eq!(response.header("retry-after"), "200");
            let body: Value = response.json();
            assert_eq!(body["error"]["code"], "rate_limited");
            assert_response_schema(&method, &path, 429, &body);
        }
    }

    #[tokio::test]
    async fn account_revoke_and_logout_preserve_namespace_idempotence_and_audit_effects() {
        let (fixture, boot) = boot_request_fixture().await;
        let grant_id = fixture.authorization.id;
        let absent_id = i64::MAX;
        let session = fixture.credential("synthetic-current-session");

        let app = fixture.credential("synthetic-app-token");
        stored_credentials(&fixture).await;
        let current_id = fixture.session_id("synthetic-current-session").await;
        let expired_id = fixture.session_id("synthetic-expired-session").await;
        let mobile = mobile_token(&fixture).await;
        let foreign_session = foreign_session(&fixture).await;
        let foreign_session_id = foreign_session.id;
        // Independent credential sequences can assign the same number to different namespaces.
        let wrong_mobile_id = [current_id, expired_id]
            .into_iter()
            .find(|id| *id != grant_id)
            .unwrap();
        let request = server(&boot);
        let mut not_found = Vec::new();
        for (label, bearer, id) in [
            ("missing", session.as_str(), absent_id),
            ("foreign", session.as_str(), foreign_session_id),
            ("wrong mobile namespace", mobile.as_str(), wrong_mobile_id),
        ] {
            let response = request
                .method(Method::DELETE, &format!("/api/v1/auth/sessions/{id}"))
                .add_header("authorization", format!("Bearer {bearer}"))
                .await;
            assert_eq!(response.status_code(), 404, "{label}");
            not_found.push((label, response));
        }
        let stored_ids = [current_id, expired_id];
        let mut wrong_stored_id = grant_id;
        // Three distinct grants cannot all overlap these two stored-session IDs.
        for label in [
            "synthetic-namespace-spare-one",
            "synthetic-namespace-spare-two",
        ] {
            if !stored_ids.contains(&wrong_stored_id) {
                break;
            }
            wrong_stored_id = fixture.mobile_login(label, 0, 0, false).await.id;
        }
        assert!(!stored_ids.contains(&wrong_stored_id));
        let wrong_stored = request
            .method(
                Method::DELETE,
                &format!("/api/v1/auth/sessions/{wrong_stored_id}"),
            )
            .add_header("authorization", format!("Bearer {app}"))
            .await;
        assert_eq!(wrong_stored.status_code(), 404, "wrong stored namespace");
        not_found.push(("wrong stored namespace", wrong_stored));
        let revoked = request
            .method(
                Method::DELETE,
                format!("/api/v1/auth/sessions/{expired_id}").as_str(),
            )
            .add_header("authorization", format!("Bearer {}", session))
            .await;
        not_found.push((
            "repeat revoke",
            request
                .method(
                    Method::DELETE,
                    format!("/api/v1/auth/sessions/{expired_id}").as_str(),
                )
                .add_header("authorization", format!("Bearer {}", session))
                .await,
        ));
        let absent = request.method(Method::DELETE, "/api/v1/auth/logout").await;
        let unknown = request
            .method(Method::DELETE, "/api/v1/auth/logout")
            .add_header("authorization", "Bearer unknown")
            .await;
        let mut logouts = Vec::new();
        for bearer in [session.as_str(), app.as_str(), mobile.as_str()] {
            let first = request
                .method(Method::DELETE, "/api/v1/auth/logout")
                .add_header("authorization", format!("Bearer {bearer}"))
                .await;
            let replay = request
                .method(Method::DELETE, "/api/v1/auth/logout")
                .add_header("authorization", format!("Bearer {bearer}"))
                .await;
            let after = request
                .get("/api/v1/auth/sessions")
                .add_header("authorization", format!("Bearer {bearer}"))
                .await;
            logouts.push((first, replay, after));
        }
        let effects = revocation_counts(&fixture).await;
        drop(request);
        close_request(fixture, boot).await;

        for response in [&revoked, &absent, &unknown] {
            assert_eq!(response.status_code(), 204);
            assert!(response.as_bytes().is_empty());
            assert_eq!(response.header("cache-control"), "no-store");
            assert!(!response.header("x-request-id").to_str().unwrap().is_empty());
        }
        for (first, replay, after) in logouts {
            for response in [first, replay] {
                assert_eq!(response.status_code(), 204);
                assert!(response.as_bytes().is_empty());
                assert_eq!(response.header("cache-control"), "no-store");
                assert!(!response.header("x-request-id").to_str().unwrap().is_empty());
            }
            assert_eq!(after.status_code(), 401);
        }
        for (label, response) in not_found {
            assert_eq!(response.status_code(), 404, "{label}");
            assert_eq!(response.header("content-type"), "application/json");
            assert_eq!(response.header("cache-control"), "no-store");
            let request_id = response.header("x-request-id").to_str().unwrap().to_owned();
            let body: Value = response.json();
            assert_eq!(body["error"]["code"], "not_found");
            assert_eq!(body["error"]["request_id"], request_id);
            assert_response_schema("delete", "/api/v1/auth/sessions/1", 404, &body);
        }
        assert_eq!(effects, (2, 1, 1));
    }

    #[tokio::test]
    async fn concurrent_account_revocation_and_logout_commit_one_effect_each() {
        let (fixture, boot) = boot_request_fixture().await;
        let session = fixture.credential("synthetic-current-session");

        let app = fixture.credential("synthetic-app-token");
        stored_credentials(&fixture).await;
        let expired_id = fixture.session_id("synthetic-expired-session").await;
        let app_id = fixture.app_token_id("synthetic-app-token").await;
        let mobile = mobile_token(&fixture).await;
        let other_mobile = fixture.credential("synthetic-other-mobile-caller");

        let target = fixture
            .mobile_login("synthetic-mobile-revocation-target", 0, 0, false)
            .await;
        let target_id = target.id;
        let _other_mobile_grant = fixture
            .mobile_login("synthetic-other-mobile-caller", 0, 0, false)
            .await;
        let request = server(&boot);

        let start = std::sync::Arc::new(tokio::sync::Barrier::new(2));
        let (session_first, session_second, session_overlapped) = overlap_requests_at_row_lock(
            &fixture,
            StoredRow::Session(expired_id),
            "UPDATE api_sessions SET revoked_at=%",
            async {
                start.wait().await;
                request
                    .method(
                        Method::DELETE,
                        format!("/api/v1/auth/sessions/{expired_id}").as_str(),
                    )
                    .add_header("authorization", format!("Bearer {}", session))
                    .await
            },
            async {
                start.wait().await;
                request
                    .method(
                        Method::DELETE,
                        format!("/api/v1/auth/sessions/{expired_id}").as_str(),
                    )
                    .add_header("authorization", format!("Bearer {}", app))
                    .await
            },
        )
        .await;

        let start = std::sync::Arc::new(tokio::sync::Barrier::new(2));
        let (mobile_first, mobile_second, mobile_overlapped) = overlap_requests_at_row_lock(
            &fixture,
            StoredRow::Grant(target_id),
            "UPDATE oauth_grants SET revoked_at=%",
            async {
                start.wait().await;
                request
                    .method(
                        Method::DELETE,
                        format!("/api/v1/auth/sessions/{target_id}").as_str(),
                    )
                    .add_header("authorization", format!("Bearer {mobile}"))
                    .await
            },
            async {
                start.wait().await;
                request
                    .method(
                        Method::DELETE,
                        format!("/api/v1/auth/sessions/{target_id}").as_str(),
                    )
                    .add_header("authorization", format!("Bearer {other_mobile}"))
                    .await
            },
        )
        .await;

        let start = std::sync::Arc::new(tokio::sync::Barrier::new(2));
        let (logout_first, logout_second, logout_overlapped) = overlap_requests_at_row_lock(
            &fixture,
            StoredRow::App(app_id),
            "UPDATE api_app_tokens SET revoked_at=%",
            async {
                start.wait().await;
                request
                    .method(Method::DELETE, "/api/v1/auth/logout")
                    .add_header("authorization", format!("Bearer {}", app))
                    .await
            },
            async {
                start.wait().await;
                request
                    .method(Method::DELETE, "/api/v1/auth/logout")
                    .add_header("authorization", format!("Bearer {}", app))
                    .await
            },
        )
        .await;

        let session_revoked =
            api_sessions::Entity::find_by_id(fixture.session_id("synthetic-expired-session").await)
                .one(&fixture.admin)
                .await
                .unwrap()
                .unwrap()
                .revoked_at
                .is_some();
        let mobile_revoked = oauth_grants::Entity::find_by_id(target.id)
            .one(&fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .revoked_at
            .is_some();
        let app_revoked =
            api_app_tokens::Entity::find_by_id(fixture.app_token_id("synthetic-app-token").await)
                .one(&fixture.admin)
                .await
                .unwrap()
                .unwrap()
                .revoked_at
                .is_some();
        let counts = revocation_counts(&fixture).await;
        let persisted = (
            session_revoked,
            mobile_revoked,
            app_revoked,
            counts.0,
            counts.1,
            counts.2,
        );
        drop(request);
        close_request(fixture, boot).await;

        assert!(
            session_overlapped,
            "two API session revocations reached the target lock"
        );
        assert!(
            mobile_overlapped,
            "two mobile revocations reached the target lock"
        );
        assert!(logout_overlapped, "two app logouts reached the target lock");

        for (label, pair) in [
            ("API session", [session_first, session_second]),
            ("mobile grant", [mobile_first, mobile_second]),
        ] {
            let mut statuses = pair
                .iter()
                .map(|response| response.status_code().as_u16())
                .collect::<Vec<_>>();
            statuses.sort_unstable();
            assert_eq!(statuses, [204, 404], "{label}");
            for response in pair {
                assert_eq!(response.header("cache-control"), "no-store", "{label}");
                assert!(!response.header("x-request-id").to_str().unwrap().is_empty());
                if response.status_code() == 204 {
                    assert!(response.as_bytes().is_empty());
                } else {
                    assert_eq!(response.header("content-type"), "application/json");
                    let body: Value = response.json();
                    assert_eq!(body["error"]["code"], "not_found");
                    assert_response_schema("delete", "/api/v1/auth/sessions/1", 404, &body);
                }
            }
        }
        for response in [logout_first, logout_second] {
            assert_eq!(response.status_code(), 204);
            assert!(response.as_bytes().is_empty());
            assert_eq!(response.header("cache-control"), "no-store");
            assert!(!response.header("x-request-id").to_str().unwrap().is_empty());
        }
        assert_eq!(persisted, (true, true, true, 1, 1, 1));
    }

    #[tokio::test]
    async fn integration_grant_cannot_manage_account_sessions_but_can_log_out() {
        let (fixture, boot) = boot_request_fixture().await;
        let account_id = fixture.account.id;
        let person_id = fixture.account.person_id;
        let membership_id = fixture.membership.id;

        let smart = fixture.credential("synthetic-smart-access");
        let integration = fixture.integration(&smart).await;
        let integration_application_id = integration.oauth_application_id;
        let request = server(&boot);
        let mut denied = Vec::new();
        for (method, path, body) in account_surface_routes(&fixture) {
            let pending = request
                .method(method.clone(), &path)
                .add_header("authorization", format!("Bearer {smart}"));
            let pending = match &body {
                Some(body) => pending.json(body),
                None => pending,
            };
            denied.push((method.as_str().to_lowercase(), path, pending.await));
        }
        let active_before = oauth_grants::Entity::find_by_id(integration.id)
            .one(&fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .revoked_at
            .is_none();
        let audit_before = security_audit_events::Entity::find()
            .filter(security_audit_events::Column::HouseholdId.eq(fixture.household.id))
            .filter(security_audit_events::Column::ActorAccountId.eq(fixture.account.id))
            .filter(security_audit_events::Column::EventType.eq("smart_oauth.revoked"))
            .count(&fixture.admin)
            .await
            .unwrap();
        let logout = request
            .method(Method::DELETE, "/api/v1/auth/logout")
            .add_header("authorization", format!("Bearer {smart}"))
            .await;
        let revoked = oauth_grants::Entity::find_by_id(integration.id)
            .one(&fixture.admin)
            .await
            .unwrap()
            .unwrap()
            .revoked_at
            .is_some();
        let audits = security_audit_events::Entity::find()
            .filter(security_audit_events::Column::HouseholdId.eq(fixture.household.id))
            .filter(security_audit_events::Column::ActorAccountId.eq(fixture.account.id))
            .filter(security_audit_events::Column::EventType.eq("smart_oauth.revoked"))
            .order_by_desc(security_audit_events::Column::Id)
            .all(&fixture.admin)
            .await
            .unwrap();
        let audit_first = u64::try_from(audits.len()).unwrap();
        let audit = &audits[0];
        let audit_request_id = audit.request_id.clone();
        let audit_membership = audit.actor_membership_id;
        let audit_metadata = audit.metadata.clone();
        let audit_context = audit.audit_context.clone();
        let replay = request
            .method(Method::DELETE, "/api/v1/auth/logout")
            .add_header("authorization", format!("Bearer {smart}"))
            .await;
        let after = request
            .get("/api/v1/auth/sessions")
            .add_header("authorization", format!("Bearer {smart}"))
            .await;
        let audit_final = security_audit_events::Entity::find()
            .filter(security_audit_events::Column::HouseholdId.eq(fixture.household.id))
            .filter(security_audit_events::Column::ActorAccountId.eq(fixture.account.id))
            .filter(security_audit_events::Column::EventType.eq("smart_oauth.revoked"))
            .count(&fixture.admin)
            .await
            .unwrap();
        drop(request);
        close_request(fixture, boot).await;

        assert!(active_before);
        assert!(revoked);
        assert_eq!(audit_first, audit_before + 1);
        assert_eq!(
            audit_request_id.as_deref(),
            logout.header("x-request-id").to_str().ok()
        );
        assert_eq!(audit_final, audit_first);
        assert_eq!(audit_membership, Some(membership_id));
        assert_eq!(
            audit_metadata["oauth_application_id"],
            integration_application_id
        );
        assert_eq!(audit_metadata["account_id"], account_id);
        assert_eq!(audit_metadata["household_membership_id"], membership_id);
        assert_eq!(audit_metadata["person_id"], person_id);
        assert_eq!(
            audit_context["request_id"],
            audit_request_id.as_deref().unwrap()
        );
        assert!(!audit_metadata.to_string().contains(smart.as_str()));
        assert!(!audit_context.to_string().contains(smart.as_str()));
        for (method, path, response) in denied {
            assert_eq!(response.status_code(), 401, "{method} {path}");
            assert_eq!(response.header("content-type"), "application/json");
            assert_eq!(response.header("cache-control"), "no-store");
            assert_eq!(response.header("www-authenticate"), "Bearer");
            let request_id = response.header("x-request-id").to_str().unwrap().to_owned();
            let body: Value = response.json();
            assert_eq!(body["error"]["code"], "unauthorized");
            assert_eq!(body["error"]["request_id"], request_id);
            assert_response_schema(&method, &path, 401, &body);
        }
        for response in [logout, replay] {
            assert_eq!(response.status_code(), 204);
            assert!(response.as_bytes().is_empty());
            assert_eq!(response.header("cache-control"), "no-store");
            assert!(!response.header("x-request-id").to_str().unwrap().is_empty());
        }
        assert_eq!(after.status_code(), 401);
    }

    #[tokio::test]
    async fn guardian_declaration_listing_rejects_invalid_windows_before_authentication() {
        let (fixture, boot) = boot_request_fixture().await;

        let token = mobile_token(&fixture).await;
        let request = server(&boot);
        let mut responses = Vec::new();
        for suffix in [
            "?limit=0",
            "?limit=101",
            "?limit=not-a-number",
            "?before_id=0",
            "?before_id=-5",
            "?unknown=1",
        ] {
            let path = format!("/api/v1/auth/guardian-declarations{suffix}");
            responses.push((
                suffix,
                request.get(&path).await,
                request
                    .get(&path)
                    .add_header("authorization", format!("Bearer {token}"))
                    .await,
            ));
        }
        drop(request);
        close_request(fixture, boot).await;
        for (suffix, anonymous, authenticated) in responses {
            for response in [&anonymous, &authenticated] {
                assert_eq!(response.status_code(), 422, "{suffix}");
                assert_eq!(response.header("content-type"), "application/json");
                assert_eq!(response.header("cache-control"), "no-store");
                assert!(!response.header("x-request-id").to_str().unwrap().is_empty());
                let body: Value = response.json();
                assert_eq!(body["error"]["code"], "unprocessable_content", "{suffix}");
                assert_response_schema("get", "/api/v1/auth/guardian-declarations", 422, &body);
            }
        }
    }

    #[tokio::test]
    async fn account_surface_lock_timeout_returns_internal_error_envelopes() {
        let (fixture, boot) = boot_request_fixture().await;

        let token = mobile_token(&fixture).await;
        let blocker = fixture.admin.begin().await.unwrap();
        accounts::Entity::find_by_id(fixture.account.id)
            .lock(sea_orm::sea_query::LockType::NoKeyUpdate)
            .one(&blocker)
            .await
            .unwrap();
        let request = server(&boot);
        let guardian_body = json!({"guardian_declaration":{"withdrawn":true,"revision":"1"}});
        let profile_body = json!({"profile":{"name":"Lock timeout"}});
        let (
            profile,
            profile_update,
            avatar,
            avatar_upload,
            avatar_delete,
            declarations,
            withdrawal,
        ) = tokio::join!(
            request
                .get("/api/v1/auth/profile")
                .add_header("authorization", format!("Bearer {token}")),
            request
                .method(Method::PATCH, "/api/v1/auth/profile")
                .add_header("authorization", format!("Bearer {token}"))
                .json(&profile_body),
            request
                .get("/api/v1/auth/profile/avatar")
                .add_header("authorization", format!("Bearer {token}")),
            request
                .put("/api/v1/auth/profile/avatar")
                .add_header("authorization", format!("Bearer {token}")),
            request
                .delete("/api/v1/auth/profile/avatar")
                .add_header("authorization", format!("Bearer {token}")),
            request
                .get("/api/v1/auth/guardian-declarations")
                .add_header("authorization", format!("Bearer {token}")),
            request
                .method(Method::PATCH, "/api/v1/auth/guardian-declarations/1")
                .add_header("authorization", format!("Bearer {token}"))
                .json(&guardian_body),
        );
        blocker.rollback().await.unwrap();
        drop(request);
        close_request(fixture, boot).await;
        for (method, path, response) in [
            ("get", "/api/v1/auth/profile", profile),
            ("patch", "/api/v1/auth/profile", profile_update),
            ("get", "/api/v1/auth/profile/avatar", avatar),
            ("put", "/api/v1/auth/profile/avatar", avatar_upload),
            ("delete", "/api/v1/auth/profile/avatar", avatar_delete),
            ("get", "/api/v1/auth/guardian-declarations", declarations),
            ("patch", "/api/v1/auth/guardian-declarations/1", withdrawal),
        ] {
            assert_eq!(response.status_code(), 500, "{method} {path}");
            assert_eq!(response.header("content-type"), "application/json");
            assert_eq!(response.header("cache-control"), "no-store");
            let request_id = response.header("x-request-id").to_str().unwrap().to_owned();
            assert!(!request_id.is_empty());
            let body: Value = response.json();
            assert_eq!(body["error"]["code"], "internal_error", "{method} {path}");
            assert_eq!(body["error"]["request_id"], request_id);
            assert_response_schema(method, path, 500, &body);
        }
    }
}

#[path = "polish_followups.rs"]
mod polish_followups;
