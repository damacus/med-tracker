#[path = "../care_api/fixture.rs"]
mod fixture;

use fixture::Fixture;
use loco_rs::{
    TestServer,
    app::Hooks,
    boot::{BootResult, StartMode},
    config::{Config, QueueConfig},
    environment::Environment,
};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

async fn boot_fixture() -> (Fixture, BootResult) {
    let fixture = Fixture::new().await;
    let mut config = Config::new(&Environment::Test).unwrap();
    config.database.uri = fixture.runtime_uri.clone();
    let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
        panic!("PostgreSQL queue required")
    };
    queue.uri = fixture.runtime_uri.clone();
    let boot = med_tracker::app::App::boot(StartMode::ServerOnly, &Environment::Test, config)
        .await
        .unwrap();
    (fixture, boot)
}

async fn boot_rate_fixture(trusted_proxy_ips: &[&str]) -> (Fixture, BootResult) {
    let fixture = Fixture::new().await;
    let mut config = Config::new(&Environment::Test).unwrap();
    config.database.uri = fixture.runtime_uri.clone();
    let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
        panic!("PostgreSQL queue required")
    };
    queue.uri = fixture.runtime_uri.clone();
    config.settings.get_or_insert_with(|| json!({}))["api_rate_limit"] = json!({
        "fixed_now_unix_seconds": 1000,
        "trusted_proxy_ips": trusted_proxy_ips.join(","),
    });
    let boot = med_tracker::app::App::boot(StartMode::ServerOnly, &Environment::Test, config)
        .await
        .unwrap();
    (fixture, boot)
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
    assert_eq!(
        hex::encode(Sha256::digest(source.as_bytes())),
        "fae2da8212338c04dea75628e35c5892e5e5bbde722d160b3072048c6ea31036",
        "The full API schema or headers changed; refresh the operation inventory and its evidence"
    );
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

fn inventory() -> BTreeMap<(String, String), Value> {
    let mut entries = BTreeMap::new();
    let mut operation_ids = BTreeSet::new();
    for line in include_str!("api_v1_inventory.jsonl").lines() {
        let entry: Value = serde_json::from_str(line).unwrap();
        let key = (
            entry["method"].as_str().unwrap().to_owned(),
            entry["path"].as_str().unwrap().to_owned(),
        );
        assert!(
            operation_ids.insert(entry["operation_id"].as_str().unwrap().to_owned()),
            "Duplicate operation ID: {}",
            entry["operation_id"]
        );
        assert!(
            entries.insert(key.clone(), entry).is_none(),
            "Duplicate method and path: {key:?}"
        );
    }
    entries
}

fn validate_evidence(entry: &Value) -> Result<(), String> {
    let expected = if entry["operation_id"] == "getCurrentProfile" {
        json!({
            "coverage": "partial",
            "evidence": [{
                "binary": "requests",
                "test": "api_v1::baseline::anonymous_access",
                "checks": ["401", "cache-control:no-store", "restricted-runtime-role", "RLS"]
            }]
        })
    } else if entry["operation_id"] == "getDataExport" {
        json!({
            "coverage": "partial",
            "evidence": [
                {
                    "binary": "requests",
                    "test": "api_v1::rate_limit::fixed_export_window_rejects_eleventh_request_with_contract_headers",
                    "checks": ["401", "429", "Retry-After:20", "RateLimit-Limit:10", "RateLimit-Remaining:0", "RateLimit-Reset:1020", "cache-control:no-store", "application/json", "rate_limited", "unchanged-medication-takes-supply-audits-change-events"]
                },
                {
                    "binary": "requests",
                    "test": "api_v1::rate_limit::untrusted_forwarded_sources_share_the_socket_peer_bucket",
                    "checks": ["untrusted-forwarded-source", "429"]
                },
                {
                    "binary": "requests",
                    "test": "api_v1::rate_limit::configured_loopback_proxy_separates_forwarded_client_buckets",
                    "checks": ["trusted-proxy-source", "malformed-forwarded-chain", "429"]
                },
                {
                    "binary": "requests",
                    "test": "api_v1::rate_limit::head_export_uses_the_get_export_bucket",
                    "checks": ["HEAD:200", "GET-HEAD-shared-bucket", "HEAD:429"]
                }
            ]
        })
    } else {
        json!({"coverage": "unverified", "evidence": []})
    };
    if entry["coverage"] != expected["coverage"] || entry["evidence"] != expected["evidence"] {
        return Err(format!(
            "Unsupported executable evidence for {}",
            entry["operation_id"]
        ));
    }
    Ok(())
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

fn resolve<'a>(document: &'a Value, value: &'a Value) -> &'a Value {
    if let Some(reference) = value["$ref"].as_str() {
        document
            .pointer(reference.strip_prefix('#').unwrap())
            .unwrap()
    } else {
        value
    }
}

fn escaped(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn registered_handler(
    source: &str,
    prefix: &str,
    target: &str,
    method: &str,
    symbol: &str,
) -> bool {
    source.match_indices(".add(").any(|(position, _)| {
        let start = position + ".add(".len();
        let mut depth = 1;
        let mut quoted = false;
        let mut escape = false;
        let mut end = start;
        for (offset, character) in source[start..].char_indices() {
            end = start + offset;
            if escape {
                escape = false;
            } else if quoted && character == '\\' {
                escape = true;
            } else if character == '"' {
                quoted = !quoted;
            } else if !quoted && character == '(' {
                depth += 1;
            } else if !quoted && character == ')' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
        }
        if depth != 0 {
            return false;
        }
        let call = &source[start..end];
        let Some(first) = call.find('"') else {
            return false;
        };
        let Some(second) = call[first + 1..].find('"') else {
            return false;
        };
        let route = &call[first + 1..first + 1 + second];
        let route_prefix = if route == "/invitations/accept" {
            "/api/v1"
        } else {
            prefix
        };
        normalized(&format!("{route_prefix}{route}")) == target
            && call.contains(&format!("{method}({symbol})"))
    })
}

fn pointer(document: &Value, value: &Value) {
    let reference = value["pointer"].as_str().unwrap();
    assert!(reference.starts_with("#/"));
    assert!(
        document.pointer(&reference[1..]).is_some(),
        "Unresolved {reference}"
    );
}

fn validate_metadata(document: &Value, entry: &Value) {
    let path = entry["path"].as_str().unwrap();
    let method = entry["method"].as_str().unwrap().to_lowercase();
    let operation = &document["paths"][path][&method];
    let request = operation
        .get("requestBody")
        .map(|body| resolve(document, body));
    let operation_pointer = format!("#/paths/{}/{}", escaped(path), method);
    let request_pointer = operation["requestBody"]["$ref"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{operation_pointer}/requestBody"));
    let expected_request = request
        .and_then(|body| body["content"].as_object())
        .map(|content| {
            content
                .iter()
                .filter(|(_, media)| media.get("schema").is_some())
                .map(|(name, _)| name.as_str())
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    let request_schemas = entry["request_schema"].as_array().unwrap();
    let recorded_request = request_schemas
        .iter()
        .map(|schema| schema["media_type"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(recorded_request, expected_request);
    assert_eq!(recorded_request.len(), request_schemas.len());
    for schema in request_schemas {
        let media = schema["media_type"].as_str().unwrap();
        assert!(request.unwrap()["content"][media].get("schema").is_some());
        assert_eq!(
            schema["pointer"],
            format!("{request_pointer}/content/{}/schema", escaped(media))
        );
        pointer(document, schema);
    }
    let response_schemas = entry["response_schemas"].as_object().unwrap();
    let response_headers = entry["response_headers"].as_object().unwrap();
    let responses = operation["responses"].as_object().unwrap();
    assert_eq!(response_schemas.len(), responses.len());
    assert_eq!(response_headers.len(), responses.len());
    for (status, raw) in responses {
        let response = resolve(document, raw);
        let response_pointer = raw["$ref"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{operation_pointer}/responses/{}", escaped(status)));
        let schemas = response_schemas[status].as_array().unwrap();
        let content = response["content"].as_object();
        let expected_media = content
            .map(|content| {
                content
                    .iter()
                    .filter(|(_, media)| media.get("schema").is_some())
                    .map(|(name, _)| name.as_str())
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        let recorded_media = schemas
            .iter()
            .map(|schema| schema["media_type"].as_str().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(recorded_media, expected_media);
        assert_eq!(recorded_media.len(), schemas.len());
        for schema in schemas {
            let media = schema["media_type"].as_str().unwrap();
            assert!(response["content"][media].get("schema").is_some());
            assert_eq!(
                schema["pointer"],
                format!("{response_pointer}/content/{}/schema", escaped(media))
            );
            pointer(document, schema);
        }
        let headers = response_headers[status].as_array().unwrap();
        let expected_headers = response["headers"]
            .as_object()
            .map(|headers| headers.keys().map(String::as_str).collect::<BTreeSet<_>>())
            .unwrap_or_default();
        let recorded_headers = headers
            .iter()
            .map(|header| header["name"].as_str().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(recorded_headers, expected_headers);
        assert_eq!(recorded_headers.len(), headers.len());
        for header in headers {
            let name = header["name"].as_str().unwrap();
            assert!(response["headers"].get(name).is_some());
            assert_eq!(
                header["pointer"],
                format!("{response_pointer}/headers/{}", escaped(name))
            );
            pointer(document, header);
        }
    }
    assert!(entry["candidate_revision"].is_null());
    assert!(!entry["gaps"].as_array().unwrap().is_empty());
    let handler = entry["handler_source"].as_str();
    if method == "post" && path == "/households/{household_id}/push_subscription/test" {
        assert!(handler.is_none());
    } else {
        let handler = handler.unwrap();
        let (module, symbol) = handler.split_once("::").unwrap();
        let (source, prefix) = match module {
            "src/controllers/api/care.rs" => (
                include_str!("../../src/controllers/api/care.rs"),
                "/api/v1/households",
            ),
            "src/controllers/api/account_sessions.rs" => (
                include_str!("../../src/controllers/api/account_sessions.rs"),
                "/api/v1/auth",
            ),
            "src/controllers/oauth_server/mod.rs" => (
                include_str!("../../src/controllers/oauth_server/mod.rs"),
                "",
            ),
            _ => panic!("Unknown handler module: {module}"),
        };
        assert!(
            registered_handler(
                source,
                prefix,
                &normalized(&format!("/api/v1{path}")),
                &method,
                symbol
            ),
            "Handler symbol not registered for {method} {path}: {handler}"
        );
    }
}

pub mod baseline {
    use super::*;

    #[test]
    fn inventory_tracks_all_documented_operations() {
        let expected = operations();
        let recorded = inventory();
        let document: Value =
            serde_yaml_ng::from_str(include_str!("../../docs/api/openapi.v1.yaml")).unwrap();
        assert_eq!(expected.len(), 122);
        assert_eq!(recorded.len(), expected.len());
        assert_eq!(
            recorded.keys().collect::<Vec<_>>(),
            expected.keys().collect::<Vec<_>>()
        );
        for (key, entry) in &recorded {
            for field in [
                "request_schema",
                "response_schemas",
                "response_headers",
                "handler_source",
                "candidate_revision",
                "gaps",
            ] {
                assert!(
                    entry.get(field).is_some(),
                    "Missing {field} metadata for {key:?}"
                );
            }
            validate_metadata(&document, entry);
            let mut requirements = entry.clone();
            requirements.as_object_mut().unwrap().remove("evidence");
            requirements.as_object_mut().unwrap().remove("coverage");
            for field in [
                "request_schema",
                "response_schemas",
                "response_headers",
                "handler_source",
                "candidate_revision",
                "gaps",
            ] {
                requirements.as_object_mut().unwrap().remove(field);
            }
            assert_eq!(&requirements, &expected[key], "OpenAPI drift at {key:?}");
            assert!(matches!(
                entry["coverage"].as_str(),
                Some("unverified" | "partial" | "complete")
            ));
            assert!(entry["evidence"].as_array().is_some());
            validate_evidence(entry).unwrap();
        }
    }

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
    fn invented_complete_coverage_and_nonexistent_test_are_rejected() {
        let mut entry =
            inventory()[&("GET".to_owned(), "/households/{household_id}/me".to_owned())].clone();
        entry["coverage"] = json!("complete");
        assert!(validate_evidence(&entry).is_err());
        entry["coverage"] = json!("partial");
        entry["evidence"][0]["test"] = json!("api_v1::baseline::invented");
        assert!(validate_evidence(&entry).is_err());
    }

    #[test]
    fn incorrect_schema_pointer_is_rejected() {
        let document: Value =
            serde_yaml_ng::from_str(include_str!("../../docs/api/openapi.v1.yaml")).unwrap();
        let mut entry =
            inventory()[&("GET".to_owned(), "/households/{household_id}/me".to_owned())].clone();
        entry["response_schemas"]["200"][0]["pointer"] =
            json!("#/components/responses/Unauthorized/content/application~1json/schema");
        assert!(std::panic::catch_unwind(|| validate_metadata(&document, &entry)).is_err());
    }

    #[test]
    fn duplicate_response_media_and_headers_are_rejected() {
        let document: Value =
            serde_yaml_ng::from_str(include_str!("../../docs/api/openapi.v1.yaml")).unwrap();
        let mut avatar = inventory()[&(
            "GET".to_owned(),
            "/households/{household_id}/profile/avatar".to_owned(),
        )]
            .clone();
        avatar["response_schemas"]["200"][1] = avatar["response_schemas"]["200"][0].clone();
        assert!(std::panic::catch_unwind(|| validate_metadata(&document, &avatar)).is_err());
        let mut profile =
            inventory()[&("GET".to_owned(), "/households/{household_id}/me".to_owned())].clone();
        profile["response_headers"]["429"][1] = profile["response_headers"]["429"][0].clone();
        assert!(std::panic::catch_unwind(|| validate_metadata(&document, &profile)).is_err());
    }

    #[test]
    fn handler_registered_on_another_path_is_rejected() {
        let document: Value =
            serde_yaml_ng::from_str(include_str!("../../docs/api/openapi.v1.yaml")).unwrap();
        let mut entry = inventory()[&(
            "GET".to_owned(),
            "/households/{household_id}/medications".to_owned(),
        )]
            .clone();
        entry["handler_source"] = json!("src/controllers/api/care.rs::removals::history");
        assert!(std::panic::catch_unwind(|| validate_metadata(&document, &entry)).is_err());
    }

    #[tokio::test]
    async fn inventory_reconciles_collected_loco_routes() {
        let (fixture, boot) = boot_fixture().await;
        let registered = api_v1_routes(&boot);
        let pinned = include_str!("api_v1_routes.jsonl")
            .lines()
            .map(|line| {
                let [method, path]: [String; 2] = serde_json::from_str(line).unwrap();
                (method, path)
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(registered, pinned, "Collected API v1 routes changed");
        let documented = inventory();
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
        println!(
            "API v1 route inventory: {} of {} documented operations registered; missing: {missing:?}; extra: {extra:?}",
            documented.len() - missing.len(),
            documented.len()
        );
        for ((method, path), entry) in &documented {
            if !entry["evidence"].as_array().unwrap().is_empty() {
                assert!(
                    actual_routes
                        .contains(&(method.clone(), normalized(&format!("/api/v1{path}")))),
                    "Evidence claims an unregistered route: {method} {path}"
                );
            }
        }
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

#[tokio::test]
#[ignore = "full API v1 acceptance remains incomplete during migration"]
async fn acceptance() {
    let (fixture, boot) = boot_fixture().await;
    let documented = inventory();
    let registered = api_v1_routes(&boot)
        .into_iter()
        .map(|(method, path)| (method, normalized(&path)))
        .collect::<BTreeSet<_>>();
    let incomplete = documented
        .iter()
        .filter(|((method, path), entry)| {
            let uri = normalized(&format!("/api/v1{path}"));
            !registered.contains(&(method.clone(), uri))
                || entry["coverage"] != "complete"
                || entry["evidence"].as_array().is_none_or(Vec::is_empty)
        })
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    close(fixture, boot).await;
    assert!(
        incomplete.is_empty(),
        "{} API v1 operations lack registered routes or complete executable evidence: {incomplete:?}",
        incomplete.len()
    );
}
