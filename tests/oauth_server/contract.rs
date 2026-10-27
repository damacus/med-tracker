use crate::scenario::Scenario;
use med_tracker::models::_entities::{
    api_change_events, locations, medication_takes, medications, oauth_applications, oauth_grants,
    versions,
};
use med_tracker::models::entities::active_session_key;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait,
    QueryFilter, prelude::Decimal,
};

struct Fixture {
    scenario: Scenario,
    application: oauth_applications::Model,
    grant: oauth_grants::Model,
    medication_id: i64,
    legacy_session_key: String,
}

impl std::ops::Deref for Fixture {
    type Target = Scenario;
    fn deref(&self) -> &Self::Target {
        &self.scenario
    }
}

async fn mobile_client(scenario: &Scenario, redirect: &str) -> oauth_applications::Model {
    let now = chrono::Utc::now().naive_utc();
    oauth_applications::ActiveModel {
        account_id: Set(Some(scenario.account.id)),
        client_id: Set(uuid::Uuid::new_v4().to_string()),
        client_kind: Set("mobile".into()),
        name: Set("Synthetic native contract client".into()),
        redirect_uri: Set(redirect.into()),
        scopes: Set("medtracker offline_access".into()),
        token_endpoint_auth_method: Set("none".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&scenario.admin)
    .await
    .unwrap()
}

impl Fixture {
    async fn new() -> Self {
        let scenario = Scenario::new().await;
        let application = mobile_client(&scenario, REDIRECT).await;
        let now = chrono::Utc::now().naive_utc();
        let grant = oauth_grants::ActiveModel {
            account_id: Set(scenario.account.id),
            oauth_application_id: Set(application.id),
            client_kind: Set("mobile".into()),
            code: Set(Some(uuid::Uuid::new_v4().to_string())),
            code_challenge: Set(Some("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".into())),
            code_challenge_method: Set(Some("S256".into())),
            redirect_uri: Set(Some(application.redirect_uri.clone())),
            scopes: Set(application.scopes.clone()),
            expires_in: Set(now + chrono::Duration::minutes(5)),
            authenticated_at: Set(Some(now - chrono::Duration::days(2))),
            last_used_at: Set(Some(now - chrono::Duration::days(2))),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&scenario.admin)
        .await
        .unwrap();
        let location = locations::ActiveModel {
            household_id: Set(scenario.household.id),
            name: Set("Contract stock sentinel".into()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&scenario.admin)
        .await
        .unwrap();
        let medication = medications::ActiveModel {
            household_id: Set(scenario.household.id),
            created_by_membership_id: Set(Some(scenario.membership.id)),
            location_id: Set(location.id),
            name: Set(Some("Contract stock sentinel".into())),
            current_supply: Set(Some(Decimal::new(10, 0))),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&scenario.admin)
        .await
        .unwrap();
        Self {
            scenario,
            application,
            grant,
            medication_id: medication.id,
            legacy_session_key: uuid::Uuid::new_v4().to_string(),
        }
    }

    async fn effect(&self) -> (u64, String, u64, u64) {
        let takes = medication_takes::Entity::find()
            .filter(medication_takes::Column::HouseholdId.eq(self.household.id))
            .count(&self.admin)
            .await
            .unwrap();
        let medication = medications::Entity::find_by_id(self.medication_id)
            .one(&self.admin)
            .await
            .unwrap()
            .unwrap();
        let audits = versions::Entity::find()
            .filter(versions::Column::HouseholdId.eq(self.household.id))
            .filter(versions::Column::ItemType.eq("MedicationTake"))
            .filter(versions::Column::Event.eq("create"))
            .count(&self.admin)
            .await
            .unwrap();
        let changes = api_change_events::Entity::find()
            .filter(api_change_events::Column::HouseholdId.eq(self.household.id))
            .filter(api_change_events::Column::RecordType.eq("MedicationTake"))
            .filter(api_change_events::Column::Action.eq("create"))
            .count(&self.admin)
            .await
            .unwrap();
        (
            takes,
            medication.current_supply.unwrap().to_string(),
            audits,
            changes,
        )
    }

    async fn close(self) {
        self.scenario.close().await;
    }
}

use loco_rs::{
    app::{AppContext, Hooks},
    boot::StartMode,
    config::{Config, QueueConfig},
    environment::Environment,
};
use serde_json::{Value, json};

const REDIRECT: &str = "io.damacus.medtracker:/oauth2redirect";
const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

#[tokio::test]
async fn all_registered_redirects_remain_available() {
    let app = Application::new().await;
    let mut application = app.fixture.application.clone().into_active_model();
    application.redirect_uri = Set(
        "io.damacus.medtracker:/oauth2redirect io.damacus.medtracker.dev:/oauth2redirect".into(),
    );
    application.update(&app.fixture.admin).await.unwrap();
    let query = serde_urlencoded::to_string([
        ("client_id", app.fixture.application.client_id.as_str()),
        ("response_type", "code"),
        ("redirect_uri", "io.damacus.medtracker.dev:/oauth2redirect"),
        ("scope", "medtracker offline_access"),
        (
            "code_challenge",
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
        ),
        ("code_challenge_method", "S256"),
    ])
    .unwrap();
    let response = app
        .client
        .get(format!("{}/authorize?{query}", app.origin))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let location = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let exchange_status = app.redeem().await.status().as_u16();
    app.close().await;
    assert_eq!(status, 303);
    assert_eq!(location.as_deref(), Some("/login"));
    assert_eq!(exchange_status, 200);
}

#[tokio::test]
async fn unregistered_scope_is_rejected_before_sign_in() {
    let app = Application::new().await;
    let query = serde_urlencoded::to_string([
        ("client_id", app.fixture.application.client_id.as_str()),
        ("response_type", "code"),
        ("redirect_uri", REDIRECT),
        ("scope", "admin"),
        ("response_mode", "query"),
        (
            "code_challenge",
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
        ),
        ("code_challenge_method", "S256"),
    ])
    .unwrap();
    let response = app
        .client
        .get(format!("{}/authorize?{query}", app.origin))
        .send()
        .await
        .unwrap();
    let location = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    app.close().await;
    let callback =
        url::Url::parse(location.as_deref().unwrap()).expect("Registered OAuth error redirect");
    assert!(
        callback
            .query_pairs()
            .any(|(key, value)| key == "error" && value == "invalid_scope")
    );
}

struct Application {
    fixture: Fixture,
    context: AppContext,
    origin: String,
    client: reqwest::Client,
    server: tokio::task::JoinHandle<()>,
}

impl Application {
    async fn new() -> Self {
        let fixture = Fixture::new().await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://localhost:{}", listener.local_addr().unwrap().port());
        let mut config = Config::new(&Environment::Test).unwrap();
        config.server.host = "http://localhost".into();
        config.server.port = i32::from(listener.local_addr().unwrap().port());
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] = json!({
            "key": "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBw==",
            "secure": false
        });
        config.database.uri = std::env::var("DATABASE_URL").unwrap();
        assert!(!config.database.auto_migrate);
        assert!(!config.database.dangerously_recreate);
        assert!(!config.database.dangerously_truncate);
        let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
            panic!("PostgreSQL queue required")
        };
        queue.uri = config.database.uri.clone();
        let boot = med_tracker::app::App::boot(StartMode::ServerOnly, &Environment::Test, config)
            .await
            .unwrap();
        let store = axum_session::SessionStore::<axum_session_sqlx::SessionPgPool>::new(
            None,
            axum_session::SessionConfig::default(),
        )
        .await
        .unwrap();
        let legacy_fixture = axum::Router::new()
            .route(
                "/test/legacy-principal",
                axum::routing::get(legacy_browser_principal),
            )
            .with_state((
                fixture.runtime.clone(),
                fixture.account.id,
                fixture.legacy_session_key.clone(),
            ))
            .layer(axum_session::SessionLayer::new(store));
        let router = boot.router.unwrap().merge(legacy_fixture);
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .await
            .unwrap()
        });
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        Self {
            fixture,
            context: boot.app_context,
            origin,
            client,
            server,
        }
    }

    async fn form(&self, path: &str, fields: &[(&str, &str)]) -> reqwest::Response {
        self.client
            .post(format!("{}{path}", self.origin))
            .header("content-type", "application/x-www-form-urlencoded")
            .body(serde_urlencoded::to_string(fields).unwrap())
            .send()
            .await
            .unwrap()
    }

    async fn redeem(&self) -> reqwest::Response {
        self.form(
            "/token",
            &[
                ("grant_type", "authorization_code"),
                ("client_id", self.fixture.application.client_id.as_str()),
                ("code", self.fixture.grant.code.as_deref().unwrap()),
                ("redirect_uri", REDIRECT),
                ("code_verifier", VERIFIER),
            ],
        )
        .await
    }

    async fn close(self) {
        self.server.abort();
        let _ = self.server.await;
        if let Some(queue) = self.context.queue_provider.as_ref() {
            queue.shutdown().unwrap();
        }
        self.context.db.close().await.unwrap();
        self.fixture.close().await;
    }
}

#[tokio::test]
async fn authorisation_code_discovery_matches_native_client_contract() {
    let app = Application::new().await;
    let response = app
        .client
        .get(format!(
            "{}/.well-known/oauth-authorization-server",
            app.origin
        ))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    let origin = app.origin.clone();
    app.close().await;
    assert_eq!(status.as_u16(), 200);
    let body: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["issuer"], origin);
    for (key, path) in [
        ("authorization_endpoint", "/authorize"),
        ("token_endpoint", "/token"),
        ("revocation_endpoint", "/revoke"),
    ] {
        let endpoint = reqwest::Url::parse(body[key].as_str().unwrap()).unwrap();
        assert_eq!(endpoint.origin().ascii_serialization(), origin);
        assert_eq!(endpoint.path(), path);
        assert!(endpoint.query().is_none() && endpoint.fragment().is_none());
    }
    assert!(
        body["code_challenge_methods_supported"]
            .as_array()
            .unwrap()
            .contains(&json!("S256"))
    );
    for grant in ["authorization_code", "refresh_token"] {
        assert!(
            body["grant_types_supported"]
                .as_array()
                .unwrap()
                .contains(&json!(grant))
        );
    }
}

#[tokio::test]
async fn authorisation_code_capabilities_publish_the_registered_public_mobile_client() {
    let app = Application::new().await;
    let response = app
        .client
        .get(format!("{}/api/v1/capabilities", app.origin))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let cache_control = response.headers().get("cache-control").cloned();
    let origin = app.origin.clone();
    let text = response.text().await.unwrap();
    let client_id = app.fixture.application.client_id.clone();
    app.close().await;
    assert_eq!(status, 200);
    let body: Value = serde_json::from_str(&text).unwrap();
    let contract: Value =
        serde_yaml_ng::from_str(include_str!("../../docs/api/openapi.v1.yaml")).unwrap();
    let schema = &contract["components"]["schemas"]["Capabilities"];
    for required in schema["required"].as_array().unwrap() {
        let name = required.as_str().unwrap();
        assert!(
            body["data"].get(name).is_some(),
            "missing native capability: {name}"
        );
    }
    for name in body["data"].as_object().unwrap().keys() {
        assert!(
            schema["properties"].get(name).is_some(),
            "unknown native capability: {name}"
        );
    }
    assert_eq!(cache_control.unwrap(), "no-store");
    assert_eq!(body["data"]["format"], "medtracker.api.capabilities.v1");
    assert_eq!(body["data"]["api_version"], "v1");
    let authentication = &body["data"]["authentication"];
    assert_eq!(
        authentication["methods"],
        json!(["oauth_bearer", "api_app_token"])
    );
    assert_eq!(
        authentication["hosted_mobile"],
        "rodauth_authorization_code_pkce"
    );
    let mobile = &authentication["mobile_oauth"];
    assert_eq!(
        mobile["discovery_url"],
        format!("{origin}/.well-known/oauth-authorization-server")
    );
    assert_eq!(mobile["inactivity_timeout_days"], 7);
    assert_eq!(mobile["maximum_age_days"], 30);
    assert_eq!(mobile["household_binding"], "account");
    let clients = mobile["clients"].as_array().unwrap();
    let native = clients
        .iter()
        .find(|client| client["client_id"] == client_id)
        .unwrap();
    assert!(
        native["redirect_uris"]
            .as_array()
            .unwrap()
            .contains(&json!(REDIRECT))
    );
    for scope in ["medtracker", "offline_access"] {
        assert!(native["scopes"].as_array().unwrap().contains(&json!(scope)));
    }
    assert!(native.get("client_secret").is_none());
    assert!(native.get("client_secret_hash").is_none());
}

#[tokio::test]
async fn authorisation_code_native_form_is_cookie_free_and_not_cacheable() {
    let app = Application::new().await;
    let response = app.redeem().await;
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let body = response.text().await.unwrap();
    let clinical_effect = app.fixture.effect().await;
    app.close().await;
    assert_eq!(status, 200);
    let pair: Value = serde_json::from_str(&body).unwrap();
    assert!(!pair["access_token"].as_str().unwrap().is_empty());
    assert!(!pair["refresh_token"].as_str().unwrap().is_empty());
    assert_eq!(
        pair["token_type"].as_str().unwrap().to_lowercase(),
        "bearer"
    );
    assert_eq!(headers["cache-control"], "no-store");
    assert_eq!(headers["pragma"], "no-cache");
    assert_eq!(headers.get_all("set-cookie").iter().count(), 0);
    assert_eq!(clinical_effect, (0, "10.00".into(), 0, 0));
}

#[tokio::test]
async fn authorisation_code_concurrent_http_redemption_has_one_success() {
    let app = Application::new().await;
    let (first, second) = tokio::join!(app.redeem(), app.redeem());
    let statuses = [first.status().as_u16(), second.status().as_u16()];
    let valid = oauth_grants::Entity::find_by_id(app.fixture.grant.id)
        .filter(oauth_grants::Column::Code.is_null())
        .filter(oauth_grants::Column::TokenHash.is_not_null())
        .filter(oauth_grants::Column::RefreshTokenHash.is_not_null())
        .filter(oauth_grants::Column::RevokedAt.is_null())
        .count(&app.fixture.admin)
        .await
        .unwrap();
    app.close().await;
    assert_eq!(statuses.iter().filter(|status| **status == 200).count(), 1);
    assert_eq!(statuses.iter().filter(|status| **status == 400).count(), 1);
    assert_eq!(valid, 1);
}

#[tokio::test]
async fn refresh_rotation_invalidates_the_original_refresh_through_http() {
    let app = Application::new().await;
    let response = app.redeem().await;
    let status = response.status().as_u16();
    if status != 200 {
        app.close().await;
        assert_eq!(status, 200);
        return;
    }
    let pair: Value = response.json().await.unwrap();
    let old = pair["refresh_token"].as_str().unwrap();
    let fields = [
        ("grant_type", "refresh_token"),
        ("client_id", app.fixture.application.client_id.as_str()),
        ("refresh_token", old),
    ];
    let (first, second) = tokio::join!(app.form("/token", &fields), app.form("/token", &fields));
    let statuses = [first.status().as_u16(), second.status().as_u16()];
    let replay = app.form("/token", &fields).await.status().as_u16();
    app.close().await;
    assert_eq!(statuses.iter().filter(|status| **status == 200).count(), 1);
    assert_eq!(statuses.iter().filter(|status| **status == 400).count(), 1);
    assert_eq!(replay, 400);
}

#[tokio::test]
async fn token_revocation_wrong_hint_falls_back_and_unknown_tokens_are_idempotent() {
    let app = Application::new().await;
    let response = app.redeem().await;
    let status = response.status().as_u16();
    if status != 200 {
        app.close().await;
        assert_eq!(status, 200);
        return;
    }
    let pair: Value = response.json().await.unwrap();
    let refresh = pair["refresh_token"].as_str().unwrap();
    let revoked = app
        .form(
            "/revoke",
            &[
                ("client_id", app.fixture.application.client_id.as_str()),
                ("token", refresh),
                ("token_type_hint", "access_token"),
            ],
        )
        .await;
    let revoked_status = revoked.status().as_u16();
    let cookies = revoked.headers().get_all("set-cookie").iter().count();
    let replay = app
        .form(
            "/token",
            &[
                ("grant_type", "refresh_token"),
                ("client_id", app.fixture.application.client_id.as_str()),
                ("refresh_token", refresh),
            ],
        )
        .await
        .status()
        .as_u16();
    let unknown = app
        .form(
            "/revoke",
            &[
                ("client_id", app.fixture.application.client_id.as_str()),
                ("token", "unknown-synthetic-token"),
                ("token_type_hint", "not-a-supported-hint"),
            ],
        )
        .await
        .status()
        .as_u16();
    app.close().await;
    assert_eq!(revoked_status, 200);
    assert_eq!(unknown, 200);
    assert_eq!(replay, 400);
    assert_eq!(cookies, 0);
}

#[tokio::test]
async fn token_revocation_is_bound_to_the_authenticated_registered_client() {
    let app = Application::new().await;
    let other_client = mobile_client(&app.fixture.scenario, "io.damacus.other:/callback").await;
    let response = app.redeem().await;
    let status = response.status().as_u16();
    if status != 200 {
        app.close().await;
        assert_eq!(status, 200);
        return;
    }
    let pair: Value = response.json().await.unwrap();
    let refresh = pair["refresh_token"].as_str().unwrap();
    let other = app
        .form(
            "/revoke",
            &[
                ("client_id", other_client.client_id.as_str()),
                ("token", refresh),
            ],
        )
        .await
        .status()
        .as_u16();
    let rightful = app
        .form(
            "/token",
            &[
                ("grant_type", "refresh_token"),
                ("client_id", app.fixture.application.client_id.as_str()),
                ("refresh_token", refresh),
            ],
        )
        .await
        .status()
        .as_u16();
    app.close().await;
    assert_eq!(other, 200);
    assert_eq!(rightful, 200);
}

#[tokio::test]
async fn token_revocation_preserves_legacy_json_as_well_as_standard_form_requests() {
    let app = Application::new().await;
    let form = app
        .form(
            "/revoke",
            &[
                ("client_id", app.fixture.application.client_id.as_str()),
                ("token", "unknown-synthetic-token"),
            ],
        )
        .await
        .status()
        .as_u16();
    let json = app
        .client
        .post(format!("{}/revoke", app.origin))
        .json(&json!({"client_id": app.fixture.application.client_id, "token": "unknown-synthetic-token"}))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.close().await;
    assert_eq!(form, 200);
    assert_eq!(json, 200);
}

#[tokio::test]
async fn authorisation_code_duplicate_client_id_is_rejected_without_consuming_code() {
    let app = Application::new().await;
    let response = app
        .form(
            "/token",
            &[
                ("grant_type", "authorization_code"),
                ("client_id", app.fixture.application.client_id.as_str()),
                ("client_id", "other-native"),
                ("code", app.fixture.grant.code.as_deref().unwrap()),
                ("redirect_uri", REDIRECT),
                ("code_verifier", VERIFIER),
            ],
        )
        .await;
    let rejected = response.status().as_u16();
    let rightful = app.redeem().await.status().as_u16();
    app.close().await;
    assert_eq!(rejected, 400);
    assert_eq!(rightful, 200);
}

#[tokio::test]
async fn authorisation_code_unregistered_redirect_never_receives_a_code_or_redirect() {
    let app = Application::new().await;
    let query = serde_urlencoded::to_string([
        ("response_type", "code"),
        ("client_id", app.fixture.application.client_id.as_str()),
        ("redirect_uri", "https://attacker.example/callback"),
        ("scope", "medtracker offline_access"),
        ("state", "synthetic-state"),
        (
            "code_challenge",
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
        ),
        ("code_challenge_method", "S256"),
    ])
    .unwrap();
    let response = app
        .client
        .get(format!("{}/authorize?{query}", app.origin))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let destination = response
        .headers()
        .get("location")
        .map(|header| header.to_str().unwrap().to_owned());
    app.close().await;
    assert_ne!(status, 404);
    assert!(destination.is_none_or(|location| !location.starts_with("https://attacker.example")));
}

async fn legacy_browser_principal(
    session: axum_session::Session<axum_session_sqlx::SessionPgPool>,
    axum::extract::State((db, account_id, registry_key)): axum::extract::State<(
        sea_orm::DatabaseConnection,
        i64,
        String,
    )>,
) -> axum::http::StatusCode {
    session.set(
        "identity",
        json!({
            "account_id": account_id, "registry_key": registry_key,
            "additional_factor_verified": true, "better_auth": false
        }),
    );
    match med_tracker::models::identity::browser::authenticate(&db, &session, None).await {
        Ok(_) => axum::http::StatusCode::OK,
        Err(_) => axum::http::StatusCode::UNAUTHORIZED,
    }
}

#[tokio::test]
async fn legacy_browser_principal_cannot_authenticate_after_cutover() {
    let app = Application::new().await;
    let now = chrono::Utc::now().naive_utc();
    active_session_key::ActiveModel {
        account_id: Set(app.fixture.account.id),
        session_id: Set(app.fixture.legacy_session_key.clone()),
        created_at: Set(now),
        last_use: Set(now),
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let status = app
        .client
        .get(format!("{}/test/legacy-principal", app.origin))
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.close().await;
    assert_eq!(status, 401);
}
