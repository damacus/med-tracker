use assert_cmd::Command;
use predicates::prelude::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn backup_export_returns_unsupported_when_server_capability_is_missing() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/capabilities"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "format": "medtracker.api.capabilities.v1",
                "api_version": "v1",
                "client_tools": {
                    "cli": { "supported": true },
                    "mcp_server": { "supported": true },
                    "diagnostics": ["request_id", "retry_after"]
                },
                "backups": { "health_data_json": false },
                "sync": {}
            }
        })))
        .mount(&server)
        .await;

    let mut command = Command::cargo_bin("medtracker").unwrap();
    command
        .arg("--base-url")
        .arg(server.uri())
        .arg("backup")
        .arg("export")
        .arg("--household-id")
        .arg("household-1")
        .env("MEDTRACKER_TOKEN", "test-token")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported_by_server"));
}
