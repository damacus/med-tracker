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
        "08948b972a6a759122fdec88da3ceffe076173aaf014a6abed3c25b60ee61273",
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
                .values()
                .filter(|media| media.get("schema").is_some())
                .count()
        })
        .unwrap_or(0);
    let request_schemas = entry["request_schema"].as_array().unwrap();
    assert_eq!(request_schemas.len(), expected_request);
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
        assert_eq!(
            schemas.len(),
            content
                .map(|content| content
                    .values()
                    .filter(|media| media.get("schema").is_some())
                    .count())
                .unwrap_or(0)
        );
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
        assert_eq!(
            headers.len(),
            response["headers"]
                .as_object()
                .map_or(0, serde_json::Map::len)
        );
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
        let source = match module {
            "src/controllers/api/care.rs" => include_str!("../../src/controllers/api/care.rs"),
            "src/controllers/api/account_sessions.rs" => {
                include_str!("../../src/controllers/api/account_sessions.rs")
            }
            "src/controllers/oauth_server/mod.rs" => {
                include_str!("../../src/controllers/oauth_server/mod.rs")
            }
            _ => panic!("Unknown handler module: {module}"),
        };
        assert!(
            source.contains(&format!("{method}({symbol})")),
            "Handler symbol not registered: {handler}"
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
        assert_eq!(expected.len(), 119);
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
