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
#[path = "capabilities.rs"]
mod capabilities;
#[path = "../care_api/contract.rs"]
mod contract;
#[path = "../care_api/fixture.rs"]
mod fixture;
#[path = "household_selection.rs"]
mod household_selection;
#[path = "household_settings.rs"]
mod household_settings;
#[path = "me_ui.rs"]
mod me_ui;
#[path = "people.rs"]
mod people;
#[path = "profile.rs"]
mod profile;

use fixture::Fixture;
use loco_rs::{
    TestServer,
    app::Hooks,
    boot::{BootResult, StartMode},
    config::{Config, QueueConfig},
    environment::Environment,
};
use sea_orm::{ConnectionTrait, DbBackend, Statement, TransactionTrait};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

async fn boot_fixture() -> (Fixture, BootResult) {
    boot_fixture_with(|_| {}).await
}

async fn boot_fixture_with(configure: impl FnOnce(&mut Config)) -> (Fixture, BootResult) {
    let fixture = Fixture::new().await;
    let mut config = Config::new(&Environment::Test).unwrap();
    config.database.uri = fixture.runtime_uri.clone();
    let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
        panic!("PostgreSQL queue required")
    };
    queue.uri = fixture.runtime_uri.clone();
    configure(&mut config);
    let boot = med_tracker::app::App::boot(StartMode::ServerOnly, &Environment::Test, config)
        .await
        .unwrap();
    (fixture, boot)
}

async fn boot_rate_fixture(trusted_proxy_ips: &[&str]) -> (Fixture, BootResult) {
    boot_rate_fixture_with(trusted_proxy_ips, None).await
}

async fn boot_rate_fixture_with(
    trusted_proxy_ips: &[&str],
    request_id_enabled: Option<bool>,
) -> (Fixture, BootResult) {
    boot_fixture_with(|config| {
        config.settings.get_or_insert_with(|| json!({}))["api_rate_limit"] = json!({
            "fixed_now_unix_seconds": 1000,
            "trusted_proxy_ips": trusted_proxy_ips.join(","),
        });
        if let Some(enable) = request_id_enabled {
            config.server.middlewares.request_id =
                Some(loco_rs::controller::middleware::request_id::RequestId { enable });
        }
    })
    .await
}

async fn close(fixture: Fixture, boot: BootResult) {
    if let Some(queue) = boot.app_context.queue_provider.as_ref() {
        queue.shutdown().unwrap();
    }
    boot.app_context.db.close().await.unwrap();
    fixture.close().await;
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
        let (fixture, boot) = boot_fixture().await;
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
        close(fixture, boot).await;
    }

    #[tokio::test]
    async fn anonymous_access() {
        let (fixture, boot) = boot_fixture().await;
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
        assert!(role.starts_with("care_runtime_"));
        assert!(row.try_get::<bool>("", "rls_active").unwrap());
        let request = server(&boot);
        let response = request.get("/api/v1/households/72001/me").await;
        assert_eq!(response.status_code(), 401);
        assert_eq!(response.header("cache-control"), "no-store");
        let identity = request
            .get("/api/auth/get-session")
            .add_header("x-forwarded-for", "198.51.100.77")
            .await;
        assert_eq!(identity.status_code(), 200);
        assert_eq!(fixture.effect().await.0, 0);
        drop(request);
        close(fixture, boot).await;
    }

    #[tokio::test]
    async fn auth_boundary_rejects_rotating_spoofed_sources() {
        let (fixture, boot) = boot_fixture().await;
        let request = server(&boot);
        for source in ["198.51.100.77", "203.0.113.77"] {
            let response = request
                .post("/api/auth/security/operation/start")
                .add_header("x-forwarded-for", source)
                .json(&json!({}))
                .await;
            assert_eq!(response.status_code(), 400);
        }
        let buckets = fixture
            .admin
            .query_all_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT bucket, attempts FROM public.identity_rate_limits",
            ))
            .await;
        let buckets = buckets.unwrap();
        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].try_get::<i32>("", "attempts").unwrap(), 2);
        assert_eq!(
            buckets[0].try_get::<String>("", "bucket").unwrap(),
            hex::encode(Sha256::digest("source:127.0.0.1".as_bytes()))
        );
        drop(request);
        close(fixture, boot).await;
    }
}

pub mod rate_limit {
    use super::*;
    use axum::{Extension, extract::ConnectInfo, http::Method};
    use std::net::SocketAddr;

    const EXPORT: &str = "/api/v1/households/72001/data_exports/health_data_json";

    #[tokio::test]
    async fn missing_socket_peer_returns_json_unavailable_error() {
        let (fixture, boot) = boot_rate_fixture(&[]).await;
        let request = TestServer::new(boot.router.clone().unwrap()).unwrap();
        let response = request.get(EXPORT).await;
        assert_eq!(response.status_code(), 503);
        assert_eq!(response.header("content-type"), "application/json");
        assert_eq!(response.header("cache-control"), "no-store");
        let body: Value = response.json();
        assert_eq!(body["error"]["code"], "service_unavailable");
        drop(request);
        close(fixture, boot).await;
    }

    #[tokio::test]
    async fn missing_socket_peer_head_response_has_no_body() {
        let (fixture, boot) = boot_rate_fixture(&[]).await;
        let request = TestServer::builder()
            .mock_transport()
            .build(boot.router.clone().unwrap())
            .unwrap();
        let response = request.method(Method::HEAD, EXPORT).await;
        assert_eq!(response.status_code(), 503);
        assert_eq!(response.header("cache-control"), "no-store");
        assert!(response.as_bytes().is_empty());
        drop(request);
        close(fixture, boot).await;
    }

    #[tokio::test]
    async fn exhausted_export_head_response_has_no_body() {
        let (fixture, boot) = boot_rate_fixture(&[]).await;
        let peer = ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 12345)));
        let request = TestServer::builder()
            .mock_transport()
            .build(boot.router.clone().unwrap().layer(Extension(peer)))
            .unwrap();
        for attempt in 1..=10 {
            let response = request.get(EXPORT).await;
            assert_eq!(response.status_code(), 401, "request {attempt}");
        }
        let response = request.method(Method::HEAD, EXPORT).await;
        assert_eq!(response.status_code(), 429);
        assert_eq!(response.header("retry-after"), "20");
        assert_eq!(response.header("cache-control"), "no-store");
        assert!(response.as_bytes().is_empty());
        drop(request);
        close(fixture, boot).await;
    }

    #[tokio::test]
    async fn fixed_export_window_rejects_eleventh_request_with_contract_headers() {
        let (fixture, boot) = boot_rate_fixture(&[]).await;
        let request = server(&boot);
        for attempt in 1..=10 {
            let response = request.get(EXPORT).await;
            assert_eq!(response.status_code(), 401, "request {attempt}");
        }
        let before = fixture.effect().await;
        let rejected = request.get(EXPORT).await;
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
        close(fixture, boot).await;
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
        let (fixture, boot) = boot_rate_fixture(&[]).await;
        let request = server(&boot);
        for attempt in 1..=10 {
            let response = request.get(EXPORT).await;
            assert_eq!(response.status_code(), 401, "request {attempt}");
        }
        let before = fixture.effect().await;
        let rejected = request
            .get(EXPORT)
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
        close(fixture, boot).await;
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
        let (fixture, boot) = boot_rate_fixture_with(&[], Some(false)).await;
        let request = server(&boot);
        for attempt in 1..=10 {
            let response = request.get(EXPORT).await;
            assert_eq!(response.status_code(), 401, "request {attempt}");
            assert!(!response.header("x-request-id").is_empty());
        }
        let before = fixture.effect().await;
        let rejected = request.get(EXPORT).await;
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
        close(fixture, boot).await;
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
        let (fixture, boot) = boot_rate_fixture(&[]).await;
        let token_request = axum::http::Request::builder()
            .method("POST")
            .uri("/token")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(axum::body::Body::from(
                serde_urlencoded::to_string([
                    ("grant_type", "authorization_code"),
                    ("client_id", "native"),
                    ("code", "synthetic-legacy-code"),
                    ("redirect_uri", "io.damacus.medtracker:/oauth2redirect"),
                    (
                        "code_verifier",
                        "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
                    ),
                ])
                .unwrap(),
            ))
            .unwrap();
        let token = med_tracker::models::identity::exchange(&fixture.runtime, token_request, None)
            .await
            .unwrap()["access_token"]
            .as_str()
            .unwrap()
            .to_owned();
        let request = server(&boot);
        let first = request
            .method(Method::HEAD, EXPORT)
            .add_header("authorization", format!("Bearer {token}"))
            .await;
        assert_eq!(first.status_code(), 200);
        for attempt in 1..=9 {
            let response = request.get(EXPORT).await;
            assert_eq!(response.status_code(), 401, "request {attempt}");
        }
        let rejected = request
            .method(Method::HEAD, EXPORT)
            .add_header("authorization", format!("Bearer {token}"))
            .await;
        assert_eq!(rejected.status_code(), 429);
        assert_eq!(rejected.header("ratelimit-limit"), "10");
        drop(request);
        close(fixture, boot).await;
    }

    #[tokio::test]
    async fn untrusted_forwarded_sources_share_the_socket_peer_bucket() {
        let (fixture, boot) = boot_rate_fixture(&[]).await;
        let request = server(&boot);
        for attempt in 1..=10 {
            let response = request
                .get(EXPORT)
                .add_header("x-forwarded-for", format!("198.51.100.{attempt}"))
                .await;
            assert_eq!(response.status_code(), 401, "request {attempt}");
        }
        let rejected = request
            .get(EXPORT)
            .add_header("x-forwarded-for", "203.0.113.11")
            .await;
        assert_eq!(rejected.status_code(), 429);
        assert_eq!(rejected.header("ratelimit-limit"), "10");
        drop(request);
        close(fixture, boot).await;
    }

    #[tokio::test]
    async fn configured_loopback_proxy_separates_forwarded_client_buckets() {
        let (fixture, boot) = boot_rate_fixture(&["127.0.0.1"]).await;
        let request = server(&boot);
        for attempt in 1..=10 {
            let response = request
                .get(EXPORT)
                .add_header("x-forwarded-for", "198.51.100.20")
                .await;
            assert_eq!(response.status_code(), 401, "request {attempt}");
        }
        let other_client = request
            .get(EXPORT)
            .add_header("x-forwarded-for", "203.0.113.20")
            .await;
        assert_eq!(other_client.status_code(), 401);
        let malformed_chain = request
            .get(EXPORT)
            .add_header("x-forwarded-for", "198.51.100.20, invalid")
            .await;
        assert_eq!(malformed_chain.status_code(), 401);
        for attempt in 1..=9 {
            let peer = request.get(EXPORT).await;
            assert_eq!(peer.status_code(), 401, "socket peer request {attempt}");
        }
        let exhausted_peer = request.get(EXPORT).await;
        assert_eq!(exhausted_peer.status_code(), 429);
        let exhausted = request
            .get(EXPORT)
            .add_header("x-forwarded-for", "198.51.100.20")
            .await;
        assert_eq!(exhausted.status_code(), 429);
        assert_eq!(exhausted.header("ratelimit-reset"), "1020");
        drop(request);
        close(fixture, boot).await;
    }
}

async fn overlap_requests_at_row_lock<A, B>(
    fixture: &Fixture,
    lock_sql: &str,
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
    blocker
        .query_one_raw(Statement::from_string(DbBackend::Postgres, lock_sql))
        .await
        .unwrap()
        .unwrap();
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
    use base64::Engine;

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

    pub(super) async fn mobile_token(fixture: &Fixture) -> String {
        let token_request = axum::http::Request::builder()
            .method("POST")
            .uri("/token")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(axum::body::Body::from(
                serde_urlencoded::to_string([
                    ("grant_type", "authorization_code"),
                    ("client_id", "native"),
                    ("code", "synthetic-legacy-code"),
                    ("redirect_uri", "io.damacus.medtracker:/oauth2redirect"),
                    (
                        "code_verifier",
                        "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
                    ),
                ])
                .unwrap(),
            ))
            .unwrap();
        med_tracker::models::identity::exchange(&fixture.runtime, token_request, None)
            .await
            .unwrap()["access_token"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    async fn stored_credentials(fixture: &Fixture) {
        let session = hex::encode(Sha256::digest(b"synthetic-current-session"));
        let expired = hex::encode(Sha256::digest(b"synthetic-expired-session"));
        let app_token = hex::encode(Sha256::digest(b"synthetic-app-token"));
        fixture.admin.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO api_sessions(id,account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,device_name,created_at,updated_at) VALUES (88001,71001,74001,$1,'synthetic-current-refresh',1,now()+interval '15 minutes',now()+interval '1 day',now(),'Current phone',now()-interval '1 day',now()),(88002,71001,74001,$2,'synthetic-expired-refresh',1,now()-interval '1 hour',now()-interval '1 minute',now()-interval '2 days','Expired phone',now()-interval '2 days',now())",
            [session.into(), expired.into()],
        )).await.unwrap();
        fixture.admin.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO api_app_tokens(id,account_id,household_membership_id,name,token_digest,permissions_version,expires_at,last_used_at,created_at,updated_at) VALUES (88010,71001,74001,'Synthetic app',$1,1,now()+interval '1 month',now(),now(),now())",
            [app_token.into()],
        )).await.unwrap();
    }

    fn account_surface_routes() -> Vec<(Method, &'static str, Option<Value>)> {
        vec![
            (Method::GET, "/api/v1/auth/households", None),
            (Method::GET, "/api/v1/auth/sessions", None),
            (Method::DELETE, "/api/v1/auth/sessions/76001", None),
            (Method::GET, "/api/v1/auth/profile", None),
            (
                Method::PATCH,
                "/api/v1/auth/profile",
                Some(json!({"profile":{"name":"Surface sweep"}})),
            ),
            (Method::GET, "/api/v1/auth/profile/avatar", None),
            (Method::PUT, "/api/v1/auth/profile/avatar", None),
            (Method::DELETE, "/api/v1/auth/profile/avatar", None),
            (Method::GET, "/api/v1/auth/guardian-declarations", None),
            (
                Method::PATCH,
                "/api/v1/auth/guardian-declarations/1",
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
        let (fixture, boot) = boot_fixture().await;
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
        close(fixture, boot).await;

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
        let (fixture, boot) = boot_fixture().await;
        let token = mobile_token(&fixture).await;
        let request = server(&boot);
        let mut anonymous = Vec::new();
        for (method, path, body) in account_surface_routes() {
            let pending = request.method(method.clone(), path);
            let pending = match &body {
                Some(body) => pending.json(body),
                None => pending,
            };
            anonymous.push((method.as_str().to_lowercase(), path, pending.await));
        }
        fixture.admin.execute_unprepared(
            "UPDATE oauth_grants SET access_token_scopes='patient/*.read',access_token_scope_hash=token_hash WHERE id=76001"
        ).await.unwrap();
        let mut insufficient = Vec::new();
        for (method, path, body) in account_surface_routes() {
            let pending = request
                .method(method.clone(), path)
                .add_header("authorization", format!("Bearer {token}"));
            let pending = match &body {
                Some(body) => pending.json(body),
                None => pending,
            };
            insufficient.push((method.as_str().to_lowercase(), path, pending.await));
        }
        let grant = fixture
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT revoked_at IS NULL AS active FROM oauth_grants WHERE id=76001",
            ))
            .await
            .unwrap()
            .unwrap();
        let active: bool = grant.try_get("", "active").unwrap();
        drop(request);
        close(fixture, boot).await;

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
            assert_response_schema(&method, path, 401, &body);
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
            assert_response_schema(&method, path, 403, &body);
        }
    }

    #[tokio::test]
    async fn account_collections_keep_household_scope_and_session_secrets_out() {
        let (fixture, boot) = boot_fixture().await;
        stored_credentials(&fixture).await;
        fixture.admin.execute_unprepared(
            "INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72002,71001,'Second household','second-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,joined_at,created_at,updated_at) VALUES(74002,72002,73001,'member',now(),now(),now()); INSERT INTO households(id,created_by_account_id,name,slug,timezone,status,lifecycle_state,created_at,updated_at) VALUES(72003,71001,'Held household','held-household','UTC','active','held',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,joined_at,created_at,updated_at) VALUES(74003,72003,73001,'member',now(),now(),now())"
        ).await.unwrap();
        fixture.admin.execute_unprepared(
            "INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73005,'Foreign member',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,password_hash,created_at,updated_at) VALUES(71002,73005,'foreign-collection-account@example.test',crypt('password',gen_salt('bf',4)),now(),now()); INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72004,71002,'Foreign active household','foreign-active-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,joined_at,created_at,updated_at) VALUES(74004,72004,73005,'owner',now(),now(),now()); INSERT INTO api_sessions(id,account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,device_name,created_at,updated_at) VALUES(88004,71002,74004,'foreign-collection-digest','foreign-collection-refresh',1,now()+interval '15 minutes',now()+interval '1 day',now(),'Foreign device',now(),now())"
        ).await.unwrap();
        let mobile = mobile_token(&fixture).await;
        let aged_maximum = "synthetic-mobile-over-maximum";
        let aged_inactivity = "synthetic-mobile-over-inactivity";
        let maximum_digest = base64::engine::general_purpose::URL_SAFE
            .encode(Sha256::digest(aged_maximum.as_bytes()));
        let inactivity_digest = base64::engine::general_purpose::URL_SAFE
            .encode(Sha256::digest(aged_inactivity.as_bytes()));
        fixture.admin.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO oauth_grants(id,account_id,oauth_application_id,client_kind,redirect_uri,scopes,token_hash,expires_in,authenticated_at,last_used_at,device_name,created_at,updated_at) VALUES(76004,71001,75001,'mobile','io.damacus.medtracker:/oauth2redirect','medtracker offline_access',$1,now()+interval '15 minutes',now()-interval '31 days',now()-interval '1 minute','Over maximum',now()-interval '31 days',now()),(76005,71001,75001,'mobile','io.damacus.medtracker:/oauth2redirect','medtracker offline_access',$2,now()+interval '15 minutes',now()-interval '9 days',now()-interval '8 days','Over inactivity',now()-interval '9 days',now())",
            [maximum_digest.into(), inactivity_digest.into()],
        )).await.unwrap();
        let request = server(&boot);
        let mut collections = Vec::new();
        for bearer in ["synthetic-current-session", "synthetic-app-token"] {
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
        fixture
            .admin
            .execute_unprepared(
                "UPDATE oauth_grants SET last_used_at=now()-interval '1 day' WHERE id=76001",
            )
            .await
            .unwrap();
        let before_row = fixture
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT last_used_at,authenticated_at FROM oauth_grants WHERE id=76001",
            ))
            .await
            .unwrap()
            .unwrap();
        let mobile_before: chrono::NaiveDateTime = before_row.try_get("", "last_used_at").unwrap();
        let mobile_authenticated: chrono::NaiveDateTime =
            before_row.try_get("", "authenticated_at").unwrap();
        let mobile_sessions = request
            .get("/api/v1/auth/sessions")
            .add_header("authorization", format!("Bearer {mobile}"))
            .await;
        let after_row = fixture
            .admin
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT last_used_at FROM oauth_grants WHERE id=76001",
            ))
            .await
            .unwrap()
            .unwrap();
        let mobile_after: chrono::NaiveDateTime = after_row.try_get("", "last_used_at").unwrap();
        let session_settings = &boot.app_context.config.settings.as_ref().unwrap()["session"];
        let inactivity_days = session_settings["inactivity_timeout_days"]
            .as_i64()
            .unwrap();
        let maximum_days = session_settings["max_age_days"].as_i64().unwrap();
        let aged_before = fixture
            .admin
            .query_all_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT id,last_used_at FROM oauth_grants WHERE id IN (76004,76005) ORDER BY id",
            ))
            .await
            .unwrap()
            .into_iter()
            .map(|row| {
                (
                    row.try_get::<i64>("", "id").unwrap(),
                    row.try_get::<chrono::NaiveDateTime>("", "last_used_at")
                        .unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let mut aged_denials = Vec::new();
        for (label, bearer) in [
            ("maximum age", aged_maximum),
            ("inactivity", aged_inactivity),
        ] {
            for (method, path, body) in account_surface_routes() {
                let pending = request
                    .method(method.clone(), path)
                    .add_header("authorization", format!("Bearer {bearer}"));
                let pending = match &body {
                    Some(body) => pending.json(body),
                    None => pending,
                };
                aged_denials.push((label, method.as_str().to_lowercase(), path, pending.await));
            }
        }
        let aged_after = fixture
            .admin
            .query_all_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT id,last_used_at FROM oauth_grants WHERE id IN (76004,76005) ORDER BY id",
            ))
            .await
            .unwrap()
            .into_iter()
            .map(|row| {
                (
                    row.try_get::<i64>("", "id").unwrap(),
                    row.try_get::<chrono::NaiveDateTime>("", "last_used_at")
                        .unwrap(),
                )
            })
            .collect::<Vec<_>>();
        drop(request);
        close(fixture, boot).await;

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
            assert_eq!(households["account_id"], 71001);
            assert_eq!(households["data"].as_array().unwrap().len(), 2);
            assert_eq!(households["data"][0]["id"], 72001);
            assert_eq!(households["data"][1]["id"], 72002);
            assert_eq!(households["data"][1]["membership_id"], 74002);
            assert!(
                households["data"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|row| row["id"] != 72004)
            );
            let sessions: Value = sessions.json();
            assert_response_schema("get", "/api/v1/auth/sessions", 200, &sessions);
            assert_eq!(sessions["data"].as_array().unwrap().len(), 2);
            assert_eq!(sessions["data"][0]["id"], 88001);
            assert_eq!(sessions["data"][1]["id"], 88002);
            assert!(
                sessions["data"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|row| row["id"] != 88004)
            );
            for secret in [
                "synthetic-current-session",
                "synthetic-current-refresh",
                "synthetic-expired-session",
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
                .all(|row| row["id"] != 72004)
        );
        let mobile_sessions: Value = mobile_sessions.json();
        assert_response_schema("get", "/api/v1/auth/sessions", 200, &mobile_sessions);
        assert_eq!(mobile_sessions["data"].as_array().unwrap().len(), 1);
        assert_eq!(mobile_sessions["data"][0]["id"], 76001);
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
            assert_response_schema(&method, path, 401, &body);
        }
    }

    #[tokio::test]
    async fn mobile_sessions_hide_foreign_and_expired_login_targets_from_listing_and_revocation() {
        let (fixture, boot) = boot_fixture().await;
        let mobile = mobile_token(&fixture).await;
        fixture.admin.execute_unprepared(
            "INSERT INTO people(id,name,created_at,updated_at) VALUES(73998,'Foreign mobile account',now(),now()); INSERT INTO accounts(id,person_id,email,password_hash,created_at,updated_at) VALUES(71002,73998,'foreign-mobile-account@example.test',crypt('password',gen_salt('bf',4)),now(),now()); INSERT INTO oauth_applications(id,account_id,client_id,client_kind,name,redirect_uri,scopes,token_endpoint_auth_method,created_at,updated_at) VALUES(75002,71002,'foreign-native','mobile','Foreign native','io.damacus.medtracker:/oauth2redirect','medtracker offline_access','none',now(),now())"
        ).await.unwrap();
        let foreign_digest = base64::engine::general_purpose::URL_SAFE
            .encode(Sha256::digest(b"synthetic-foreign-mobile"));
        let maximum_digest = base64::engine::general_purpose::URL_SAFE
            .encode(Sha256::digest(b"synthetic-aged-mobile-maximum"));
        let inactivity_digest = base64::engine::general_purpose::URL_SAFE
            .encode(Sha256::digest(b"synthetic-aged-mobile-inactivity"));
        let access_expired_digest = base64::engine::general_purpose::URL_SAFE
            .encode(Sha256::digest(b"synthetic-access-expired-login-active"));
        fixture.admin.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO oauth_grants(id,account_id,oauth_application_id,client_kind,redirect_uri,scopes,token_hash,expires_in,authenticated_at,last_used_at,device_name,created_at,updated_at) VALUES(76008,71002,75002,'mobile','io.damacus.medtracker:/oauth2redirect','medtracker offline_access',$1,now()+interval '15 minutes',now(),now(),'Foreign mobile',now(),now()),(76004,71001,75001,'mobile','io.damacus.medtracker:/oauth2redirect','medtracker offline_access',$2,now()+interval '15 minutes',now()-interval '31 days',now()-interval '1 minute','Over maximum',now()-interval '31 days',now()),(76005,71001,75001,'mobile','io.damacus.medtracker:/oauth2redirect','medtracker offline_access',$3,now()+interval '15 minutes',now()-interval '9 days',now()-interval '8 days','Over inactivity',now()-interval '9 days',now()),(76009,71001,75001,'mobile','io.damacus.medtracker:/oauth2redirect','medtracker offline_access',$4,now()-interval '1 minute',now(),now(),'Access expired, login active',now(),now())",
            [foreign_digest.into(), maximum_digest.into(), inactivity_digest.into(), access_expired_digest.into()],
        )).await.unwrap();
        let request = server(&boot);
        let listed = request
            .get("/api/v1/auth/sessions")
            .add_header("authorization", format!("Bearer {mobile}"))
            .await;
        let mut denials = Vec::new();
        for (label, id) in [
            ("foreign account", 76008),
            ("maximum login age", 76004),
            ("login inactivity", 76005),
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
            .method(Method::DELETE, "/api/v1/auth/sessions/76009")
            .add_header("authorization", format!("Bearer {mobile}"))
            .await;
        let rows = fixture.admin.query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id,revoked_at IS NULL AS active FROM oauth_grants WHERE id IN (76004,76005,76008,76009) ORDER BY id",
        )).await.unwrap();
        let active = rows
            .into_iter()
            .map(|row| {
                (
                    row.try_get::<i64>("", "id").unwrap(),
                    row.try_get::<bool>("", "active").unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let audit = fixture.admin.query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM versions WHERE item_type='Account' AND item_id=71001 AND event='mobile_oauth.revoked') AS own, (SELECT count(*) FROM versions WHERE item_type='Account' AND item_id=71002 AND event='mobile_oauth.revoked') AS foreign",
        )).await.unwrap().unwrap();
        let audits = (
            audit.try_get::<i64>("", "own").unwrap(),
            audit.try_get::<i64>("", "foreign").unwrap(),
        );
        drop(request);
        close(fixture, boot).await;

        assert_eq!(listed.status_code(), 200);
        let list: Value = listed.json();
        let listed_ids = list["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|session| session["id"].as_i64().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(listed_ids, BTreeSet::from([76001, 76009]));
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
        assert_eq!(
            active,
            [(76004, true), (76005, true), (76008, true), (76009, false)]
        );
        assert_eq!(audits, (1, 0));
    }

    #[tokio::test]
    async fn account_routes_reject_invalid_revoked_and_stale_credentials_without_writes() {
        let (fixture, boot) = boot_fixture().await;
        stored_credentials(&fixture).await;
        let mobile = mobile_token(&fixture).await;
        let request = server(&boot);
        let mut denied = Vec::new();
        for (label, bearer) in [
            ("invalid", "unknown"),
            ("expired", "synthetic-expired-session"),
        ] {
            for (method, path, body) in account_surface_routes() {
                let pending = request
                    .method(method.clone(), path)
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
            for (method, path, body) in account_surface_routes() {
                let pending = request
                    .method(method.clone(), path)
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
                "synthetic-current-session",
                "UPDATE api_sessions SET revoked_at=now() WHERE id=88001",
                "UPDATE api_sessions SET revoked_at=NULL WHERE id=88001",
            ),
            (
                "revoked app token",
                "synthetic-app-token",
                "UPDATE api_app_tokens SET revoked_at=now() WHERE id=88010",
                "UPDATE api_app_tokens SET revoked_at=NULL WHERE id=88010",
            ),
            (
                "expired app token",
                "synthetic-app-token",
                "UPDATE api_app_tokens SET created_at=now()-interval '2 days',expires_at=now()-interval '1 day' WHERE id=88010",
                "UPDATE api_app_tokens SET created_at=now(),expires_at=now()+interval '1 month' WHERE id=88010",
            ),
            (
                "overage app token",
                "synthetic-app-token",
                "UPDATE api_app_tokens SET created_at=now()-interval '13 months' WHERE id=88010",
                "UPDATE api_app_tokens SET created_at=now() WHERE id=88010",
            ),
            (
                "stale session permissions",
                "synthetic-current-session",
                "UPDATE household_memberships SET permissions_version=2 WHERE id=74001",
                "UPDATE household_memberships SET permissions_version=1 WHERE id=74001",
            ),
            (
                "stale app token permissions",
                "synthetic-app-token",
                "UPDATE household_memberships SET permissions_version=2 WHERE id=74001",
                "UPDATE household_memberships SET permissions_version=1 WHERE id=74001",
            ),
            (
                "revoked membership",
                "synthetic-current-session",
                "UPDATE household_memberships SET revoked_at=now() WHERE id=74001",
                "UPDATE household_memberships SET revoked_at=NULL WHERE id=74001",
            ),
            (
                "locked account",
                "synthetic-current-session",
                "INSERT INTO account_lockouts(account_id,deadline,key,created_at,updated_at) VALUES(71001,now()+interval '1 hour','account-route-test',now(),now())",
                "DELETE FROM account_lockouts WHERE account_id=71001 AND key='account-route-test'",
            ),
            (
                "locked account app token",
                "synthetic-app-token",
                "INSERT INTO account_lockouts(account_id,deadline,key,created_at,updated_at) VALUES(71001,now()+interval '1 hour','account-route-test',now(),now())",
                "DELETE FROM account_lockouts WHERE account_id=71001 AND key='account-route-test'",
            ),
            (
                "locked account mobile",
                mobile.as_str(),
                "INSERT INTO account_lockouts(account_id,deadline,key,created_at,updated_at) VALUES(71001,now()+interval '1 hour','account-route-test',now(),now())",
                "DELETE FROM account_lockouts WHERE account_id=71001 AND key='account-route-test'",
            ),
        ];
        let mut mobile_without_membership = Vec::new();
        for (label, bearer, change, restore) in changes {
            fixture.admin.execute_unprepared(change).await.unwrap();
            for (method, path, body) in account_surface_routes() {
                let pending = request
                    .method(method.clone(), path)
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
            fixture.admin.execute_unprepared(restore).await.unwrap();
        }
        fixture
            .admin
            .execute_unprepared("UPDATE oauth_grants SET revoked_at=now() WHERE id=76001")
            .await
            .unwrap();
        for (method, path, body) in account_surface_routes() {
            let pending = request
                .method(method.clone(), path)
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
        let row = fixture.admin.query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM versions WHERE event='auth_token/api_session/revoked') AS revocations, (SELECT revoked_at IS NULL FROM api_sessions WHERE id=88001) AS active",
        )).await.unwrap().unwrap();
        let revocations: i64 = row.try_get("", "revocations").unwrap();
        let active: bool = row.try_get("", "active").unwrap();
        drop(request);
        close(fixture, boot).await;

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
            assert_response_schema(&method, path, 401, &body);
        }
        for (label, households, sessions) in mobile_without_membership {
            assert_eq!(households.status_code(), 200, "{label}: households");
            assert_eq!(sessions.status_code(), 200, "{label}: sessions");
            let household_body: Value = households.json();
            let session_body: Value = sessions.json();
            if label == "revoked membership" {
                assert!(household_body["data"].as_array().unwrap().is_empty());
            }
            assert_eq!(session_body["data"][0]["id"], 76001);
        }
    }

    #[tokio::test]
    async fn account_routes_share_the_fixed_general_limit() {
        let (fixture, boot) = boot_rate_fixture(&[]).await;
        let request = server(&boot);
        for attempt in 1..=300 {
            let response = request.get("/api/v1/capabilities").await;
            assert_eq!(response.status_code(), 200, "request {attempt}");
        }
        let mut denied = Vec::new();
        for (method, path, _) in account_surface_routes() {
            denied.push((
                method.as_str().to_lowercase(),
                path,
                request.method(method, path).await,
            ));
        }
        denied.push((
            "delete".to_owned(),
            "/api/v1/auth/logout",
            request.method(Method::DELETE, "/api/v1/auth/logout").await,
        ));
        drop(request);
        close(fixture, boot).await;
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
            assert_response_schema(&method, path, 429, &body);
        }
    }

    #[tokio::test]
    async fn account_revoke_and_logout_preserve_namespace_idempotence_and_audit_effects() {
        let (fixture, boot) = boot_fixture().await;
        stored_credentials(&fixture).await;
        let mobile = mobile_token(&fixture).await;
        fixture.admin.execute_unprepared(
            "INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73004,'Foreign person',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,password_hash,created_at,updated_at) VALUES(71002,73004,'other-session-account@example.test',crypt('password',gen_salt('bf',4)),now(),now()); INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72002,71002,'Foreign household','foreign-session-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,joined_at,created_at,updated_at) VALUES(74002,72002,73004,'owner',now(),now(),now()); INSERT INTO api_sessions(id,account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,created_at,updated_at) VALUES(88004,71002,74002,'foreign-digest','foreign-refresh',1,now()+interval '15 minutes',now()+interval '1 day',now(),now(),now())"
        ).await.unwrap();
        let request = server(&boot);
        let mut not_found = Vec::new();
        for (label, bearer, id) in [
            ("missing", "synthetic-current-session", 99999),
            ("foreign", "synthetic-current-session", 88004),
            ("wrong mobile namespace", mobile.as_str(), 88001),
            ("wrong stored namespace", "synthetic-app-token", 76001),
        ] {
            not_found.push((
                label,
                request
                    .method(Method::DELETE, &format!("/api/v1/auth/sessions/{id}"))
                    .add_header("authorization", format!("Bearer {bearer}"))
                    .await,
            ));
        }
        let revoked = request
            .method(Method::DELETE, "/api/v1/auth/sessions/88002")
            .add_header("authorization", "Bearer synthetic-current-session")
            .await;
        not_found.push((
            "repeat revoke",
            request
                .method(Method::DELETE, "/api/v1/auth/sessions/88002")
                .add_header("authorization", "Bearer synthetic-current-session")
                .await,
        ));
        let absent = request.method(Method::DELETE, "/api/v1/auth/logout").await;
        let unknown = request
            .method(Method::DELETE, "/api/v1/auth/logout")
            .add_header("authorization", "Bearer unknown")
            .await;
        let mut logouts = Vec::new();
        for bearer in [
            "synthetic-current-session",
            "synthetic-app-token",
            mobile.as_str(),
        ] {
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
        let counts = fixture.admin.query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM versions WHERE item_type='AuthenticationToken' AND item_id=71001 AND event='auth_token/api_session/revoked') AS sessions, (SELECT count(*) FROM versions WHERE item_type='AuthenticationToken' AND item_id=71001 AND event='auth_token/api_app_token/revoked') AS app_tokens, (SELECT count(*) FROM versions WHERE item_type='Account' AND item_id=71001 AND event='mobile_oauth.revoked') AS mobile",
        )).await.unwrap().unwrap();
        let effects = (
            counts.try_get::<i64>("", "sessions").unwrap(),
            counts.try_get::<i64>("", "app_tokens").unwrap(),
            counts.try_get::<i64>("", "mobile").unwrap(),
        );
        drop(request);
        close(fixture, boot).await;

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
        let (fixture, boot) = boot_fixture().await;
        stored_credentials(&fixture).await;
        let mobile = mobile_token(&fixture).await;
        let other_mobile = "synthetic-other-mobile-caller";
        let target_mobile = "synthetic-mobile-revocation-target";
        let other_digest = base64::engine::general_purpose::URL_SAFE
            .encode(Sha256::digest(other_mobile.as_bytes()));
        let target_digest = base64::engine::general_purpose::URL_SAFE
            .encode(Sha256::digest(target_mobile.as_bytes()));
        fixture.admin.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO oauth_grants(id,account_id,oauth_application_id,client_kind,redirect_uri,scopes,token_hash,expires_in,authenticated_at,last_used_at,device_name,created_at,updated_at) VALUES(76006,71001,75001,'mobile','io.damacus.medtracker:/oauth2redirect','medtracker offline_access',$1,now()+interval '15 minutes',now(),now(),'Revocation target',now(),now()),(76007,71001,75001,'mobile','io.damacus.medtracker:/oauth2redirect','medtracker offline_access',$2,now()+interval '15 minutes',now(),now(),'Second caller',now(),now())",
            [target_digest.into(), other_digest.into()],
        )).await.unwrap();
        let request = server(&boot);

        let start = std::sync::Arc::new(tokio::sync::Barrier::new(2));
        let (session_first, session_second, session_overlapped) = overlap_requests_at_row_lock(
            &fixture,
            "SELECT id FROM api_sessions WHERE id=88002 FOR UPDATE",
            "UPDATE api_sessions SET revoked_at=%",
            async {
                start.wait().await;
                request
                    .method(Method::DELETE, "/api/v1/auth/sessions/88002")
                    .add_header("authorization", "Bearer synthetic-current-session")
                    .await
            },
            async {
                start.wait().await;
                request
                    .method(Method::DELETE, "/api/v1/auth/sessions/88002")
                    .add_header("authorization", "Bearer synthetic-app-token")
                    .await
            },
        )
        .await;

        let start = std::sync::Arc::new(tokio::sync::Barrier::new(2));
        let (mobile_first, mobile_second, mobile_overlapped) = overlap_requests_at_row_lock(
            &fixture,
            "SELECT id FROM oauth_grants WHERE id=76006 FOR UPDATE",
            "UPDATE oauth_grants SET revoked_at=%",
            async {
                start.wait().await;
                request
                    .method(Method::DELETE, "/api/v1/auth/sessions/76006")
                    .add_header("authorization", format!("Bearer {mobile}"))
                    .await
            },
            async {
                start.wait().await;
                request
                    .method(Method::DELETE, "/api/v1/auth/sessions/76006")
                    .add_header("authorization", format!("Bearer {other_mobile}"))
                    .await
            },
        )
        .await;

        let start = std::sync::Arc::new(tokio::sync::Barrier::new(2));
        let (logout_first, logout_second, logout_overlapped) = overlap_requests_at_row_lock(
            &fixture,
            "SELECT id FROM api_app_tokens WHERE id=88010 FOR UPDATE",
            "UPDATE api_app_tokens SET revoked_at=%",
            async {
                start.wait().await;
                request
                    .method(Method::DELETE, "/api/v1/auth/logout")
                    .add_header("authorization", "Bearer synthetic-app-token")
                    .await
            },
            async {
                start.wait().await;
                request
                    .method(Method::DELETE, "/api/v1/auth/logout")
                    .add_header("authorization", "Bearer synthetic-app-token")
                    .await
            },
        )
        .await;

        let effects = fixture.admin.query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT (SELECT revoked_at IS NOT NULL FROM api_sessions WHERE id=88002) AS session_revoked, (SELECT revoked_at IS NOT NULL FROM oauth_grants WHERE id=76006) AS mobile_revoked, (SELECT revoked_at IS NOT NULL FROM api_app_tokens WHERE id=88010) AS app_revoked, (SELECT count(*) FROM versions WHERE item_type='AuthenticationToken' AND item_id=71001 AND event='auth_token/api_session/revoked') AS session_audits, (SELECT count(*) FROM versions WHERE item_type='Account' AND item_id=71001 AND event='mobile_oauth.revoked') AS mobile_audits, (SELECT count(*) FROM versions WHERE item_type='AuthenticationToken' AND item_id=71001 AND event='auth_token/api_app_token/revoked') AS app_audits",
        )).await.unwrap().unwrap();
        let persisted = (
            effects.try_get::<bool>("", "session_revoked").unwrap(),
            effects.try_get::<bool>("", "mobile_revoked").unwrap(),
            effects.try_get::<bool>("", "app_revoked").unwrap(),
            effects.try_get::<i64>("", "session_audits").unwrap(),
            effects.try_get::<i64>("", "mobile_audits").unwrap(),
            effects.try_get::<i64>("", "app_audits").unwrap(),
        );
        drop(request);
        close(fixture, boot).await;

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
        let (fixture, boot) = boot_fixture().await;
        let smart = "synthetic-smart-access";
        let digest =
            base64::engine::general_purpose::URL_SAFE.encode(Sha256::digest(smart.as_bytes()));
        fixture.admin.execute_unprepared(
            "INSERT INTO oauth_applications(id,account_id,client_id,client_kind,name,redirect_uri,scopes,token_endpoint_auth_method,client_secret_hash,created_at,updated_at) VALUES(75003,71001,'smart-journey','integration','Synthetic SMART journey','https://example.test/callback?tenant=7','launch/patient patient/*.rs offline_access','client_secret_post',crypt('password',gen_salt('bf',4)),now(),now())"
        ).await.unwrap();
        fixture.admin.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO oauth_grants(id,account_id,oauth_application_id,client_kind,household_membership_id,person_id,permissions_version,redirect_uri,scopes,token_hash,expires_in,authenticated_at,last_used_at,device_name,created_at,updated_at) VALUES(76003,71001,75003,'integration',74001,73001,1,'https://example.test/callback?tenant=7','launch/patient patient/*.rs offline_access',$1,now()+interval '15 minutes',now(),now(),'SMART client',now(),now())",
            [digest.into()],
        )).await.unwrap();
        let request = server(&boot);
        let mut denied = Vec::new();
        for (method, path, body) in account_surface_routes() {
            let pending = request
                .method(method.clone(), path)
                .add_header("authorization", format!("Bearer {smart}"));
            let pending = match &body {
                Some(body) => pending.json(body),
                None => pending,
            };
            denied.push((method.as_str().to_lowercase(), path, pending.await));
        }
        let before = fixture.admin.query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT (SELECT revoked_at IS NULL FROM oauth_grants WHERE id=76003) AS active, (SELECT count(*) FROM security_audit_events WHERE household_id=72001 AND actor_account_id=71001 AND event_type='smart_oauth.revoked') AS audits",
        )).await.unwrap().unwrap();
        let active_before: bool = before.try_get("", "active").unwrap();
        let audit_before: i64 = before.try_get("", "audits").unwrap();
        let logout = request
            .method(Method::DELETE, "/api/v1/auth/logout")
            .add_header("authorization", format!("Bearer {smart}"))
            .await;
        let first = fixture.admin.query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT (SELECT revoked_at IS NOT NULL FROM oauth_grants WHERE id=76003) AS revoked, (SELECT count(*) FROM security_audit_events WHERE household_id=72001 AND actor_account_id=71001 AND event_type='smart_oauth.revoked') AS audits, (SELECT max(request_id) FROM security_audit_events WHERE household_id=72001 AND actor_account_id=71001 AND event_type='smart_oauth.revoked') AS audit_request_id",
        )).await.unwrap().unwrap();
        let revoked: bool = first.try_get("", "revoked").unwrap();
        let audit_first: i64 = first.try_get("", "audits").unwrap();
        let audit_request_id: Option<String> = first.try_get("", "audit_request_id").unwrap();
        let audit = fixture.admin.query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT actor_membership_id,metadata,audit_context FROM security_audit_events WHERE household_id=72001 AND actor_account_id=71001 AND event_type='smart_oauth.revoked' LIMIT 1",
        )).await.unwrap().unwrap();
        let audit_membership: Option<i64> = audit.try_get("", "actor_membership_id").unwrap();
        let audit_metadata: Value = audit.try_get("", "metadata").unwrap();
        let audit_context: Value = audit.try_get("", "audit_context").unwrap();
        let replay = request
            .method(Method::DELETE, "/api/v1/auth/logout")
            .add_header("authorization", format!("Bearer {smart}"))
            .await;
        let after = request
            .get("/api/v1/auth/sessions")
            .add_header("authorization", format!("Bearer {smart}"))
            .await;
        let final_row = fixture.admin.query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS audits FROM security_audit_events WHERE household_id=72001 AND actor_account_id=71001 AND event_type='smart_oauth.revoked'",
        )).await.unwrap().unwrap();
        let audit_final: i64 = final_row.try_get("", "audits").unwrap();
        drop(request);
        close(fixture, boot).await;

        assert!(active_before);
        assert!(revoked);
        assert_eq!(audit_first, audit_before + 1);
        assert_eq!(
            audit_request_id.as_deref(),
            logout.header("x-request-id").to_str().ok()
        );
        assert_eq!(audit_final, audit_first);
        assert_eq!(audit_membership, Some(74001));
        assert_eq!(audit_metadata["oauth_application_id"], 75003);
        assert_eq!(audit_metadata["account_id"], 71001);
        assert_eq!(audit_metadata["household_membership_id"], 74001);
        assert_eq!(audit_metadata["person_id"], 73001);
        assert_eq!(
            audit_context["request_id"],
            audit_request_id.as_deref().unwrap()
        );
        assert!(!audit_metadata.to_string().contains(smart));
        assert!(!audit_context.to_string().contains(smart));
        for (method, path, response) in denied {
            assert_eq!(response.status_code(), 401, "{method} {path}");
            assert_eq!(response.header("content-type"), "application/json");
            assert_eq!(response.header("cache-control"), "no-store");
            assert_eq!(response.header("www-authenticate"), "Bearer");
            let request_id = response.header("x-request-id").to_str().unwrap().to_owned();
            let body: Value = response.json();
            assert_eq!(body["error"]["code"], "unauthorized");
            assert_eq!(body["error"]["request_id"], request_id);
            assert_response_schema(&method, path, 401, &body);
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
        let (fixture, boot) = boot_fixture().await;
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
        close(fixture, boot).await;
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
        let (fixture, boot) = boot_fixture().await;
        let token = mobile_token(&fixture).await;
        let blocker = fixture.admin.begin().await.unwrap();
        blocker
            .execute_unprepared("SELECT id FROM accounts WHERE id=71001 FOR NO KEY UPDATE")
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
        close(fixture, boot).await;
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
