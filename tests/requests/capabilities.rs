use super::*;

async fn credential_counts(fixture: &Fixture) -> (i64, i64) {
    let row = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM api_sessions) AS sessions, (SELECT count(*) FROM oauth_grants) AS grants".to_owned(),
        ))
        .await
        .unwrap()
        .unwrap();
    (
        row.try_get("", "sessions").unwrap(),
        row.try_get("", "grants").unwrap(),
    )
}

#[tokio::test]
async fn public_capabilities_discovery_matches_contract_and_hides_secrets() {
    let (fixture, boot) = boot_fixture().await;
    let request = server(&boot);
    let response = request.get("/api/v1/capabilities").await;
    let status = response.status_code();
    let body: Value = response.json();
    assert_eq!(status, 200);
    let data = &body["data"];
    let advertised = json!({
        "app_tokens": data["administration"]["app_tokens"],
        "audit_logs": data["administration"]["audit_logs"],
        "review_actions": data["medication_reviews"]["actions"],
        "portable_formats": data["portable_formats"],
        "encrypted_migration_bundle": data["backups"]["encrypted_migration_bundle"],
        "unencrypted_zip": data["backups"]["unencrypted_zip"],
        "health_data_json": data["backups"]["health_data_json"],
        "dry_run_import": data["sync"]["dry_run_import"],
    });
    assert_eq!(
        advertised,
        json!({
            "app_tokens": true,
            "audit_logs": true,
            "review_actions": ["index", "show", "update"],
            "portable_formats": [
                "medtracker.portable.v1",
                "medtracker.portable.encrypted.v1",
                "medtracker.portable.v2",
                "medtracker.portable.v3"
            ],
            "encrypted_migration_bundle": true,
            "unencrypted_zip": true,
            "health_data_json": true,
            "dry_run_import": true,
        })
    );
    assert_eq!(data["client_tools"]["cli"]["supported"], false);
    assert_eq!(data["client_tools"]["mcp_server"]["supported"], false);
    assert_eq!(data["fhir"]["resources"], json!([]));
    assert_eq!(
        body["data"]["profile"],
        json!({
            "actions": ["show", "update"],
            "online_only": true,
            "avatar": {
                "actions": ["show", "update", "destroy"],
                "max_bytes": 5 * 1024 * 1024,
                "content_types": ["image/png", "image/jpeg", "image/webp"]
            }
        })
    );
    fixture.admin.execute_unprepared("INSERT INTO oauth_applications(id, account_id, client_id, client_kind, name, redirect_uri, scopes, token_endpoint_auth_method, created_at, updated_at) VALUES(75002,71001,'other-native','mobile','Other synthetic native','io.damacus.other:/callback','medtracker offline_access','none',now(),now())").await.unwrap();
    fixture.admin.execute_unprepared("INSERT INTO oauth_applications(id, account_id, client_id, client_kind, name, redirect_uri, scopes, token_endpoint_auth_method, client_secret_hash, created_at, updated_at) VALUES(75003,71001,'smart-journey','integration','Synthetic SMART journey','https://example.test/callback','launch/patient patient/*.rs','client_secret_post',crypt('password',gen_salt('bf',4)),now(),now())").await.unwrap();
    let response = request.get("/api/v1/capabilities").await;
    let status = response.status_code();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let cache_control = response
        .headers()
        .get("cache-control")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let pragma = response
        .headers()
        .get("pragma")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body: Value = response.json();
    drop(request);
    close(fixture, boot).await;

    assert_eq!(status, 200);
    assert_eq!(
        body["data"]["authentication"]["mobile_oauth"]["inactivity_timeout_days"],
        7
    );
    assert_eq!(
        body["data"]["authentication"]["mobile_oauth"]["maximum_age_days"],
        30
    );
    let clients = body["data"]["authentication"]["mobile_oauth"]["clients"]
        .as_array()
        .unwrap();
    let client = clients
        .iter()
        .find(|client| client["client_id"] == "other-native")
        .unwrap();
    assert_eq!(client["name"], "Other synthetic native");
    assert_eq!(
        client["redirect_uris"],
        json!(["io.damacus.other:/callback"])
    );
    assert_eq!(client["scopes"], json!(["medtracker", "offline_access"]));
    assert!(
        clients
            .iter()
            .all(|client| client["client_id"] != "smart-journey")
    );
    for client in clients {
        assert!(client.get("client_secret").is_none());
        assert!(client.get("client_secret_hash").is_none());
    }
    assert_eq!(content_type.as_deref(), Some("application/json"));
    assert_eq!(cache_control.as_deref(), Some("no-store"));
    assert_eq!(pragma.as_deref(), Some("no-cache"));
    assert!(request_id.is_some_and(|value| !value.is_empty()));
    let document = contract::contract();
    contract::assert_value(
        document,
        &document["components"]["schemas"]["CapabilitiesResponse"],
        &body,
        "capabilities",
    );
}

#[tokio::test]
async fn configured_mobile_lifetimes_include_disabled_maximum_age() {
    let mut observed = Vec::new();
    for (inactivity_days, maximum_age_days) in [(3, 14), (3, 0)] {
        let (fixture, boot) = boot_fixture_with(|config| {
            config.settings.get_or_insert_with(|| json!({}))["session"] = json!({
                "inactivity_timeout_days": inactivity_days,
                "max_age_days": maximum_age_days,
            });
        })
        .await;
        let request = server(&boot);
        let response = request.get("/api/v1/capabilities").await;
        let status = response.status_code().as_u16();
        let body: Value = response.json();
        drop(request);
        close(fixture, boot).await;
        observed.push((
            maximum_age_days,
            status,
            body["data"]["authentication"]["mobile_oauth"]["inactivity_timeout_days"].clone(),
            body["data"]["authentication"]["mobile_oauth"]["maximum_age_days"].clone(),
        ));
    }
    assert_eq!(
        observed,
        vec![(14, 200, json!(3), json!(14)), (0, 200, json!(3), json!(0))]
    );
}

#[tokio::test]
async fn capability_requests_use_the_fixed_general_limit() {
    let (fixture, boot) = boot_rate_fixture(&[]).await;
    let request = server(&boot);
    let initial_effects = fixture.effect().await;
    let initial_credentials = credential_counts(&fixture).await;
    for attempt in 1..=300 {
        let response = request.get("/api/v1/capabilities").await;
        assert_eq!(response.status_code(), 200, "request {attempt}");
    }
    let effects_after_success = fixture.effect().await;
    let credentials_after_success = credential_counts(&fixture).await;
    let rejected = request.get("/api/v1/capabilities").await;
    let status = rejected.status_code();
    let content_type = rejected.header("content-type").to_owned();
    let cache_control = rejected.header("cache-control").to_owned();
    let request_id = rejected.header("x-request-id").to_owned();
    let retry_after = rejected.header("retry-after").to_owned();
    let limit = rejected.header("ratelimit-limit").to_owned();
    let remaining = rejected.header("ratelimit-remaining").to_owned();
    let reset = rejected.header("ratelimit-reset").to_owned();
    let body: Value = rejected.json();
    let effects_after = fixture.effect().await;
    let credentials_after = credential_counts(&fixture).await;
    drop(request);
    close(fixture, boot).await;

    assert_eq!(status, 429);
    assert_eq!(content_type, "application/json");
    assert_eq!(cache_control, "no-store");
    assert!(!request_id.is_empty());
    assert_eq!(retry_after, "200");
    assert_eq!(limit, "300");
    assert_eq!(remaining, "0");
    assert_eq!(reset, "1200");
    let document = contract::contract();
    contract::assert_value(
        document,
        &contract::resolve(
            document,
            &document["paths"]["/capabilities"]["get"]["responses"]["429"],
        )["content"]["application/json"]["schema"],
        &body,
        "getCapabilities 429",
    );
    assert_eq!(body["error"]["code"], "rate_limited");
    assert_eq!(effects_after_success, initial_effects);
    assert_eq!(credentials_after_success, initial_credentials);
    assert_eq!(effects_after, initial_effects);
    assert_eq!(credentials_after, initial_credentials);
}

#[tokio::test]
async fn capability_database_lookup_failure_uses_existing_unavailable_envelope() {
    let (fixture, boot) = boot_fixture().await;
    let effects = fixture.effect().await;
    let credentials = credential_counts(&fixture).await;
    fixture
        .admin
        .execute_unprepared(
            "ALTER TABLE oauth_applications RENAME TO oauth_applications_unavailable",
        )
        .await
        .unwrap();
    let request = server(&boot);
    let unavailable = request.get("/api/v1/capabilities").await;
    fixture
        .admin
        .execute_unprepared(
            "ALTER TABLE oauth_applications_unavailable RENAME TO oauth_applications",
        )
        .await
        .unwrap();
    let status = unavailable.status_code();
    let content_type = unavailable.header("content-type").to_owned();
    let cache_control = unavailable.header("cache-control").to_owned();
    let pragma = unavailable.header("pragma").to_owned();
    let request_id = unavailable.header("x-request-id").to_owned();
    let body: Value = unavailable.json();
    let effects_after = fixture.effect().await;
    let credentials_after = credential_counts(&fixture).await;
    drop(request);
    close(fixture, boot).await;

    assert_eq!(status, 503);
    assert_eq!(content_type, "application/json");
    assert_eq!(cache_control, "no-store");
    assert_eq!(pragma, "no-cache");
    assert!(!request_id.is_empty());
    assert_eq!(body, json!({"error":"temporarily_unavailable"}));
    assert_eq!(effects_after, effects);
    assert_eq!(credentials_after, credentials);
}
