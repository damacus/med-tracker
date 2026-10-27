use super::*;
use med_tracker::models::_entities::{
    api_change_events, api_sessions, locations, medication_takes, medications, oauth_applications,
    oauth_grants, versions,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, prelude::Decimal, sea_query::Table,
};

async fn oauth_client(scenario: &Scenario, mobile: bool) -> oauth_applications::Model {
    let now = chrono::Utc::now().naive_utc();
    oauth_applications::ActiveModel {
        account_id: Set(Some(scenario.account.id)),
        client_id: Set(format!("capability-{}", uuid::Uuid::new_v4())),
        client_kind: Set(if mobile { "mobile" } else { "integration" }.into()),
        name: Set(if mobile {
            "Other synthetic native"
        } else {
            "Synthetic SMART journey"
        }
        .into()),
        redirect_uri: Set(if mobile {
            "io.damacus.other:/callback"
        } else {
            "https://example.test/callback"
        }
        .into()),
        scopes: Set(if mobile {
            "medtracker offline_access"
        } else {
            "launch/patient patient/*.rs"
        }
        .into()),
        token_endpoint_auth_method: Set(if mobile { "none" } else { "client_secret_post" }.into()),
        client_secret_hash: Set(if mobile {
            None
        } else {
            Some(bcrypt::hash("password", 4).unwrap())
        }),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&scenario.admin)
    .await
    .unwrap()
}

async fn seed_capability_rows(scenario: &Scenario) {
    let application = oauth_client(scenario, true).await;
    let now = chrono::Utc::now().naive_utc();
    oauth_grants::ActiveModel {
        account_id: Set(scenario.account.id),
        oauth_application_id: Set(application.id),
        client_kind: Set("mobile".into()),
        code: Set(Some(uuid::Uuid::new_v4().to_string())),
        code_challenge: Set(Some("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".into())),
        code_challenge_method: Set(Some("S256".into())),
        redirect_uri: Set(Some(application.redirect_uri)),
        scopes: Set(application.scopes),
        expires_in: Set(now + chrono::Duration::minutes(5)),
        authenticated_at: Set(Some(now)),
        last_used_at: Set(Some(now)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&scenario.admin)
    .await
    .unwrap();
    let location = locations::ActiveModel {
        household_id: Set(scenario.household.id),
        name: Set("Capability stock sentinel".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&scenario.admin)
    .await
    .unwrap();
    medications::ActiveModel {
        household_id: Set(scenario.household.id),
        location_id: Set(location.id),
        name: Set(Some("Capability stock sentinel".into())),
        current_supply: Set(Some(Decimal::new(10, 0))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&scenario.admin)
    .await
    .unwrap();
}

async fn credential_counts(scenario: &Scenario) -> (u64, u64) {
    (
        api_sessions::Entity::find()
            .count(&scenario.admin)
            .await
            .unwrap(),
        oauth_grants::Entity::find()
            .count(&scenario.admin)
            .await
            .unwrap(),
    )
}

#[derive(Debug, PartialEq)]
struct GlobalEffects {
    takes: u64,
    supplies: Vec<(i64, Option<Decimal>)>,
    audits: u64,
    changes: u64,
}

async fn global_effects(scenario: &Scenario) -> GlobalEffects {
    // Public requests have no tenant binding. Keep these observations global;
    // callers comparing snapshots must run with whole-run exclusivity.
    GlobalEffects {
        takes: medication_takes::Entity::find()
            .count(&scenario.admin)
            .await
            .unwrap(),
        supplies: medications::Entity::find()
            .select_only()
            .columns([medications::Column::Id, medications::Column::CurrentSupply])
            .order_by_asc(medications::Column::Id)
            .into_tuple()
            .all(&scenario.admin)
            .await
            .unwrap(),
        audits: versions::Entity::find()
            .filter(versions::Column::ItemType.eq("MedicationTake"))
            .filter(versions::Column::Event.eq("create"))
            .count(&scenario.admin)
            .await
            .unwrap(),
        changes: api_change_events::Entity::find()
            .filter(api_change_events::Column::RecordType.eq("MedicationTake"))
            .filter(api_change_events::Column::Action.eq("create"))
            .count(&scenario.admin)
            .await
            .unwrap(),
    }
}

#[tokio::test]
async fn public_capabilities_discovery_matches_contract_and_hides_secrets() {
    let (fixture, boot) = boot_shared_fixture().await;
    seed_capability_rows(&fixture).await;
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
    let mobile = oauth_client(&fixture, true).await;
    let integration = oauth_client(&fixture, false).await;
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
    close_shared(fixture, boot).await;

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
        .find(|client| client["client_id"] == mobile.client_id)
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
            .all(|client| client["client_id"] != integration.client_id)
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
        let (fixture, boot) = boot_shared_fixture_with(|config| {
            config.settings.get_or_insert_with(|| json!({}))["session"] = json!({
                "inactivity_timeout_days": inactivity_days,
                "max_age_days": maximum_age_days,
            });
        })
        .await;
        seed_capability_rows(&fixture).await;
        let request = server(&boot);
        let response = request.get("/api/v1/capabilities").await;
        let status = response.status_code().as_u16();
        let body: Value = response.json();
        drop(request);
        close_shared(fixture, boot).await;
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
    let (fixture, boot) = boot_shared_rate_fixture(&[]).await;
    seed_capability_rows(&fixture).await;
    let request = server(&boot);
    let initial_effects = global_effects(&fixture).await;
    let initial_credentials = credential_counts(&fixture).await;
    for attempt in 1..=300 {
        let response = request.get("/api/v1/capabilities").await;
        assert_eq!(response.status_code(), 200, "request {attempt}");
    }
    let effects_after_success = global_effects(&fixture).await;
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
    let effects_after = global_effects(&fixture).await;
    let credentials_after = credential_counts(&fixture).await;
    drop(request);
    close_shared(fixture, boot).await;

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
    let (fixture, boot) = boot_shared_fixture().await;
    seed_capability_rows(&fixture).await;
    let effects = global_effects(&fixture).await;
    let credentials = credential_counts(&fixture).await;
    let request = server(&boot);
    // The schema fault also requires whole-run exclusivity. Restore the table
    // before inspecting response headers or making assertions.
    let unavailable_table = format!("oauth_unavailable_{}", uuid::Uuid::new_v4().simple());
    let rename = Table::rename()
        .table(
            "oauth_applications",
            sea_orm::sea_query::Alias::new(unavailable_table.clone()),
        )
        .to_owned();
    fixture
        .admin
        .execute_raw(fixture.admin.get_database_backend().build(&rename))
        .await
        .unwrap();
    let unavailable = request.get("/api/v1/capabilities").await;
    let restore = Table::rename()
        .table(
            sea_orm::sea_query::Alias::new(unavailable_table.clone()),
            "oauth_applications",
        )
        .to_owned();
    fixture
        .admin
        .execute_raw(fixture.admin.get_database_backend().build(&restore))
        .await
        .unwrap();
    let status = unavailable.status_code();
    let content_type = unavailable.header("content-type").to_owned();
    let cache_control = unavailable.header("cache-control").to_owned();
    let pragma = unavailable.header("pragma").to_owned();
    let request_id = unavailable.header("x-request-id").to_owned();
    let body: Value = unavailable.json();
    let effects_after = global_effects(&fixture).await;
    let credentials_after = credential_counts(&fixture).await;
    drop(request);
    close_shared(fixture, boot).await;

    assert_eq!(status, 503);
    assert_eq!(content_type, "application/json");
    assert_eq!(cache_control, "no-store");
    assert_eq!(pragma, "no-cache");
    assert!(!request_id.is_empty());
    assert_eq!(body, json!({"error":"temporarily_unavailable"}));
    assert_eq!(effects_after, effects);
    assert_eq!(credentials_after, credentials);
}
