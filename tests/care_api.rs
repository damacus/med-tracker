#[path = "care_api/account_closure_mail.rs"]
mod account_closure_mail;
#[path = "care_api/account_sessions.rs"]
mod account_sessions;
#[path = "care_api/administration.rs"]
mod administration;
#[path = "care_api/ai_suggestions.rs"]
mod ai_suggestions;
#[path = "care_api/api_session.rs"]
mod api_session;
#[path = "care_api/app_tokens.rs"]
mod app_tokens;
#[path = "care_api/assignments.rs"]
mod assignments;
#[path = "care_api/audit_logs.rs"]
mod audit_logs;
#[path = "care_api/audit_logs_browser.rs"]
mod audit_logs_browser;
#[path = "care_api/canonical_identity.rs"]
mod canonical_identity;

#[path = "care_api/approved_sharing.rs"]
mod approved_sharing;
#[path = "care_api/avatar.rs"]
mod avatar;
#[path = "care_api/avatar_storage.rs"]
mod avatar_storage;
#[path = "care_api/better_auth_feature_stores.rs"]
mod better_auth_feature_stores;
#[path = "care_api/better_auth_invitations.rs"]
mod better_auth_invitations;
#[path = "care_api/better_auth_store.rs"]
mod better_auth_store;
#[path = "care_api/browser_push.rs"]
mod browser_push;
#[path = "care_api/browser_push_reminders.rs"]
mod browser_push_reminders;
#[path = "support/clinical_fixture.rs"]
mod clinical_fixture;
#[path = "care_api/contract.rs"]
mod contract;
#[path = "care_api/contract_tests.rs"]
mod contract_tests;
#[path = "care_api/crud.rs"]
mod crud;
#[path = "care_api/dashboard.rs"]
mod dashboard;
#[path = "care_api/data_exports.rs"]
mod data_exports;
#[path = "care_api/dosages.rs"]
mod dosages;
#[path = "care_api/dose_history.rs"]
mod dose_history;
#[path = "care_api/dose_outcomes.rs"]
mod dose_outcomes;
#[path = "care_api/fixture.rs"]
mod fixture;
#[path = "care_api/fixture_acceptance.rs"]
mod fixture_acceptance;
#[path = "support/fixtures.rs"]
pub mod fixtures;
#[path = "care_api/guardian_declarations.rs"]
mod guardian_declarations;
#[path = "care_api/health_events.rs"]
mod health_events;
#[path = "care_api/invitations.rs"]
mod invitations;
#[path = "care_api/legacy_pause.rs"]
mod legacy_pause;
#[path = "care_api/location_memberships.rs"]
mod location_memberships;
#[path = "care_api/locations.rs"]
mod locations;
#[path = "care_api/me.rs"]
mod me;
#[path = "care_api/medication_lookup.rs"]
mod medication_lookup;
#[path = "care_api/medication_reads.rs"]
mod medication_reads;
#[path = "care_api/mobile_snapshot.rs"]
mod mobile_snapshot;
#[path = "care_api/native_device_tokens.rs"]
mod native_device_tokens;
#[path = "care_api/native_email_change.rs"]
mod native_email_change;
#[path = "care_api/native_oidc.rs"]
mod native_oidc;
#[path = "care_api/native_password_reset.rs"]
mod native_password_reset;
#[path = "care_api/nhs_dmd.rs"]
mod nhs_dmd;
#[path = "care_api/notification_preferences.rs"]
mod notification_preferences;
#[path = "care_api/pause_periods.rs"]
mod pause_periods;
#[path = "care_api/people.rs"]
mod people;
#[path = "care_api/person_carers.rs"]
mod person_carers;
#[path = "care_api/person_medication_reorder.rs"]
mod person_medication_reorder;
#[path = "care_api/person_owned_schedules.rs"]
mod person_owned_schedules;
#[path = "care_api/platform.rs"]
mod platform;
#[path = "care_api/portable_export.rs"]
mod portable_export;
#[path = "care_api/portable_imports.rs"]
mod portable_imports;
#[path = "care_api/profile.rs"]
mod profile;
#[path = "care_api/profile_export.rs"]
mod profile_export;
#[path = "care_api/push_subscriptions.rs"]
mod push_subscriptions;
#[path = "care_api/reports.rs"]
mod reports;
#[path = "care_api/resource_credentials.rs"]
mod resource_credentials;
#[path = "care_api/review_prompts.rs"]
mod review_prompts;
#[path = "care_api/saved_status.rs"]
mod saved_status;
#[path = "care_api/schedule_lifecycle.rs"]
mod schedule_lifecycle;
#[path = "care_api/schedule_scope.rs"]
mod schedule_scope;
#[path = "care_api/scheduler_image.rs"]
mod scheduler_image;
#[path = "support/session_fixture.rs"]
mod session_fixture;
#[path = "care_api/signup.rs"]
mod signup;
#[path = "care_api/signup_timezone.rs"]
mod signup_timezone;
#[path = "care_api/stock_orders.rs"]
mod stock_orders;
#[path = "care_api/support_expiry.rs"]
mod support_expiry;
#[path = "care_api/sync.rs"]
mod sync;
#[path = "requests/storage_guard.rs"]
mod template;
#[path = "care_api/treatment_timestamps.rs"]
mod treatment_timestamps;
#[path = "care_api/treatments.rs"]
mod treatments;
#[path = "care_api/ui_capabilities.rs"]
mod ui_capabilities;

use axum::{body::Body, http::Request};
#[path = "care_api/account_avatar.rs"]
mod account_avatar;
#[path = "support/reference_fixture.rs"]
mod reference_fixture;
#[path = "support/scenario.rs"]
pub mod scenario;
use fixture::Fixture;
use loco_rs::{
    app::{AppContext, Hooks},
    boot::StartMode,
    config::{Config, QueueConfig},
    environment::Environment,
};
use med_tracker::models::_entities::{
    household_memberships, medication_takes, security_audit_events, versions,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter,
};
use serde_json::{Value, json};

struct Application {
    fixture: Fixture,
    context: AppContext,
    origin: String,
    client: reqwest::Client,
    server: tokio::task::JoinHandle<()>,
}

impl Application {
    async fn assert_status(response: reqwest::Response, expected: u16) -> Value {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap();
        assert_eq!(status, expected, "{body}");
        serde_json::from_str(&body).unwrap_or(Value::String(body))
    }

    async fn avatar_bucket_server(
        &self,
        bucket: &str,
    ) -> (String, AppContext, tokio::task::JoinHandle<()>) {
        let mut config = Config::new(&Environment::Test).unwrap();
        assert!(!config.database.auto_migrate);
        assert!(!config.database.dangerously_recreate);
        assert!(!config.database.dangerously_truncate);
        config.database.uri = self.fixture.runtime_uri.clone();
        let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
            panic!("PostgreSQL queue required")
        };
        queue.uri = self.fixture.runtime_uri.clone();
        config.settings.get_or_insert_with(|| json!({}))["avatar_storage"]["bucket"] =
            json!(bucket);
        let boot = med_tracker::app::App::boot(StartMode::ServerOnly, &Environment::Test, config)
            .await
            .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let router = boot.router.unwrap();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .await
            .unwrap()
        });
        (origin, boot.app_context, server)
    }

    async fn new() -> Self {
        Self::new_with_occurrence_key(Some("synthetic-dose-occurrence-signing-key")).await
    }

    async fn new_with_occurrence_key(signing_key: Option<&str>) -> Self {
        Self::new_configured(signing_key, |_| ()).await
    }

    async fn new_with_ai_help(enabled: bool) -> Self {
        Self::new_configured(
            Some("synthetic-dose-occurrence-signing-key"),
            move |settings| {
                settings["ai_medication_help"] = json!({"enabled": enabled});
            },
        )
        .await
    }

    async fn new_with_ai_generation(port: u16) -> Self {
        Self::new_configured(
            Some("synthetic-dose-occurrence-signing-key"),
            move |settings| {
                settings["ai_medication_help"] = json!({
                    "enabled": true,
                    "provider": "openai",
                    "api_key": "synthetic-ai-key",
                    "model": "gpt-4o",
                    "api_base_url": format!("http://127.0.0.1:{port}/v1/"),
                    "source_proxy_origin": format!("http://127.0.0.1:{port}")
                });
            },
        )
        .await
    }

    async fn new_configured(signing_key: Option<&str>, configure: impl FnOnce(&mut Value)) -> Self {
        Self::new_with_pool_size(signing_key, configure, None, None).await
    }

    async fn new_single_connection(acquire_timeout_ms: Option<u64>) -> Self {
        Self::new_with_pool_size(
            Some("synthetic-dose-occurrence-signing-key"),
            |_| (),
            Some(1),
            acquire_timeout_ms,
        )
        .await
    }

    async fn new_with_pool_size(
        signing_key: Option<&str>,
        configure: impl FnOnce(&mut Value),
        pool_size: Option<u32>,
        acquire_timeout_ms: Option<u64>,
    ) -> Self {
        let fixture = Fixture::new().await;
        Self::boot_fixture(
            fixture,
            signing_key,
            configure,
            pool_size,
            acquire_timeout_ms,
        )
        .await
    }
}

impl Application {
    async fn boot_fixture(
        fixture: Fixture,
        signing_key: Option<&str>,
        configure: impl FnOnce(&mut Value),
        pool_size: Option<u32>,
        acquire_timeout_ms: Option<u64>,
    ) -> Self {
        let mut config = Config::new(&Environment::Test).unwrap();
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] = json!({
            "key": "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBw==",
            "secure": false
        });
        let settings = config.settings.get_or_insert_with(|| json!({}));
        if let Some(signing_key) = signing_key {
            settings["dose_occurrences"] = json!({"signing_key": signing_key});
        } else {
            settings.as_object_mut().unwrap().remove("dose_occurrences");
        }
        configure(settings);
        assert!(!config.database.auto_migrate);
        assert!(!config.database.dangerously_recreate);
        assert!(!config.database.dangerously_truncate);
        config.database.uri = fixture.runtime_uri.clone();
        if let Some(acquire_timeout_ms) = acquire_timeout_ms {
            config.database.acquire_timeout = Some(acquire_timeout_ms);
        }
        if let Some(pool_size) = pool_size {
            config.database.min_connections = pool_size;
            config.database.max_connections = pool_size;
        }
        let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
            panic!("PostgreSQL queue required")
        };
        queue.uri = fixture.runtime_uri.clone();
        let boot = med_tracker::app::App::boot(StartMode::ServerOnly, &Environment::Test, config)
            .await
            .unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let router = boot.router.unwrap();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .await
            .unwrap()
        });
        let client = reqwest::Client::builder()
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

    async fn token(&self) -> String {
        self.token_pair().await["access_token"]
            .as_str()
            .unwrap()
            .into()
    }

    async fn token_pair(&self) -> Value {
        let request = Request::builder()
            .method("POST")
            .uri("/token")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(
                serde_urlencoded::to_string([
                    ("grant_type", "authorization_code"),
                    ("client_id", self.fixture.client_id.as_str()),
                    ("code", self.fixture.code.as_str()),
                    ("redirect_uri", "io.damacus.medtracker:/oauth2redirect"),
                    (
                        "code_verifier",
                        "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
                    ),
                ])
                .unwrap(),
            ))
            .unwrap();
        med_tracker::models::identity::exchange(&self.fixture.runtime, request, None)
            .await
            .unwrap()
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

fn shared_take_body(care: &Fixture) -> Value {
    json!({"medication_take": {
        "client_uuid": care.client_uuid.to_string(), "source_type": "person_medication",
        "source_id": care.assignment.id.to_string(), "taken_at": "2026-10-05T10:00:00Z",
        "dose_amount": "2", "dose_unit": "tablet", "taken_from_medication_id": care.medication.id
    }})
}

#[tokio::test]
async fn care_routes_reject_unvalidated_requests_without_clinical_effects() {
    let app = Application::new().await;
    let household_id = app.fixture.scenario.household.id;
    let medication_id = app.fixture.medication.id;
    let take = app
        .client
        .post(format!(
            "{}/api/v1/households/{household_id}/medication_takes",
            app.origin
        ))
        .json(&shared_take_body(&app.fixture))
        .send()
        .await
        .unwrap();
    let stock = app
        .client
        .patch(format!(
            "{}/api/v1/households/{household_id}/medications/{medication_id}/adjust_inventory",
            app.origin
        ))
        .json(&json!({"adjustment":{"new_quantity":"12"}}))
        .send()
        .await
        .unwrap();
    let statuses = (take.status().as_u16(), stock.status().as_u16());
    let effect = app.fixture.effect().await;
    app.close().await;
    assert_eq!(
        statuses,
        (401, 401),
        "Care routes must enforce the real identity boundary"
    );
    assert_eq!(effect, (0, "10.00".into(), 0, 0));
}

#[tokio::test]
async fn authenticated_take_replay_and_stock_adjustment_use_real_loco_routes() {
    let app = Application::new().await;
    let household_id = app.fixture.scenario.household.id;
    let medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/medication_takes",
        app.origin
    );
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&shared_take_body(&app.fixture))
        .send()
        .await
        .unwrap();
    let first_status = first.status().as_u16();
    let first_etag = first.headers().get("etag").cloned();
    let first_body: Value = first.json().await.unwrap_or(Value::Null);
    let taken_effect = app.fixture.effect().await;
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&shared_take_body(&app.fixture))
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let replay_body: Value = replay.json().await.unwrap_or(Value::Null);
    let replay_effect = app.fixture.effect().await;
    let stock = app
        .client
        .patch(format!(
            "{}/api/v1/households/{household_id}/medications/{medication_id}/adjust_inventory",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&json!({"adjustment":{"new_quantity":"12","reason":"synthetic count"}}))
        .send()
        .await
        .unwrap();
    let stock_status = stock.status().as_u16();
    let stock_etag = stock.headers().get("etag").cloned();
    let stock_body: Value = stock.json().await.unwrap_or(Value::Null);
    let final_effect = app.fixture.effect().await;
    let stock_audits = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(household_id))
        .filter(versions::Column::ItemType.eq("Medication"))
        .filter(versions::Column::ItemId.eq(medication_id))
        .filter(versions::Column::Event.eq("adjust inventory (qty: 12, reason: synthetic count)"))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    app.close().await;
    assert_eq!(first_status, 201);
    assert!(first_etag.is_some());
    assert_eq!(taken_effect, (1, "8.00".into(), 1, 1));
    assert_eq!(replay_status, 200);
    assert_eq!(replay_body["data"]["id"], first_body["data"]["id"]);
    assert_eq!(replay_effect, taken_effect);
    assert_eq!(stock_status, 200);
    assert!(stock_etag.is_some());
    assert_eq!(stock_body["data"]["current_supply"], "12.0");
    assert_eq!(final_effect, (1, "12.00".into(), 1, 1));
    assert_eq!(stock_audits, 1);
}

#[tokio::test]
async fn malformed_dose_retry_conflicts_only_for_an_accessible_used_uuid() {
    use med_tracker::models::entities::membership;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let app = Application::new().await;
    let household_id = app.fixture.scenario.household.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/medication_takes",
        app.origin
    );
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&shared_take_body(&app.fixture))
        .send()
        .await
        .unwrap();
    assert_eq!(first.status().as_u16(), 201);
    let committed = app.fixture.effect().await;
    let mut statuses = Vec::new();
    for (field, malformed) in [("taken_at", "not-a-time"), ("dose_amount", "not-a-decimal")] {
        for used in [true, false] {
            let mut body = shared_take_body(&app.fixture);
            body["medication_take"][field] = json!(malformed);
            body["medication_take"]["client_uuid"] = json!(if used {
                app.fixture.client_uuid.to_string().to_uppercase()
            } else {
                uuid::Uuid::new_v4().to_string()
            });
            let response = app
                .client
                .post(&endpoint)
                .bearer_auth(&token)
                .json(&body)
                .send()
                .await
                .unwrap();
            statuses.push(response.status().as_u16());
            assert_eq!(app.fixture.effect().await, committed);
        }
    }
    let member = membership::Entity::find_by_id(app.fixture.scenario.membership.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut revoked: membership::ActiveModel = member.into();
    revoked.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    revoked.update(&app.fixture.admin).await.unwrap();
    let mut denied_body = shared_take_body(&app.fixture);
    denied_body["medication_take"]["taken_at"] = json!("not-a-time");
    let denied = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&denied_body)
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status().as_u16(), 403);
    assert_eq!(app.fixture.effect().await, committed);
    app.close().await;
    assert_eq!(statuses, [409, 422, 409, 422]);
}

#[tokio::test]
async fn authenticated_foreign_household_and_revoked_membership_have_no_effects() {
    let app = Application::new().await;
    let household_id = app.fixture.scenario.household.id;
    let foreign_care = Fixture::new().await;
    let foreign_id = foreign_care.scenario.household.id;
    let token = app.token().await;
    let foreign = app
        .client
        .post(format!(
            "{}/api/v1/households/{foreign_id}/medication_takes",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&shared_take_body(&app.fixture))
        .send()
        .await
        .unwrap();
    app.fixture.revoke().await;
    foreign_care.scenario.close().await;
    let revoked = app
        .client
        .post(format!(
            "{}/api/v1/households/{household_id}/medication_takes",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&shared_take_body(&app.fixture))
        .send()
        .await
        .unwrap();
    let statuses = (foreign.status().as_u16(), revoked.status().as_u16());
    let effect = app.fixture.effect().await;
    app.close().await;
    assert_eq!(statuses, (403, 403));
    assert_eq!(effect, (0, "10.00".into(), 0, 0));
}

#[tokio::test]
async fn failed_clinical_audit_rolls_back_effects_but_records_http_attempt() {
    let app = Application::new().await;
    let household_id = app.fixture.scenario.household.id;
    let token = app.token().await;
    let baseline = app.fixture.versions().await;
    let baseline_versions = baseline.len();
    assert!(baseline_versions > 0);
    assert_eq!(
        baseline_versions,
        baseline
            .iter()
            .filter(|row| row.item_type == "NativeAuthentication")
            .count()
    );
    let function = format!("care_api_audit_failure_{}", uuid::Uuid::new_v4().simple());
    // SeaORM has no trigger or PL/pgSQL function builder. This isolated fault
    // injection requires whole-run exclusivity and removes both objects below.
    app.fixture.admin.execute_unprepared(&format!(
        "CREATE FUNCTION public.{function}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='MedicationTake' THEN RAISE EXCEPTION 'synthetic clinical audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER {function} BEFORE INSERT ON public.versions FOR EACH ROW EXECUTE FUNCTION public.{function}()"
    )).await.unwrap();
    let reply = app
        .client
        .post(format!(
            "{}/api/v1/households/{household_id}/medication_takes",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&shared_take_body(&app.fixture))
        .send()
        .await;
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "DROP TRIGGER {function} ON public.versions; DROP FUNCTION public.{function}()"
        ))
        .await
        .unwrap();
    let reply = reply.unwrap();
    let status = reply.status().as_u16();
    let request_id = reply
        .headers()
        .get("x-request-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let body: Value = reply.json().await.unwrap();
    let effect = app.fixture.effect().await;
    let versions = app.fixture.versions().await.len();
    let attempts = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(household_id))
        .filter(security_audit_events::Column::EventType.eq("api.request"))
        .filter(security_audit_events::Column::RequestId.eq(request_id.clone()))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 500);
    assert_eq!(body["error"]["request_id"], request_id);
    assert!(
        !body
            .to_string()
            .contains("synthetic clinical audit rejection")
    );
    assert_eq!(effect, (0, "10.00".into(), 0, 0));
    assert_eq!(versions, baseline_versions);
    assert_eq!(attempts.len(), 1);
    assert_eq!(
        attempts[0].metadata["status"].to_string().trim_matches('"'),
        "500"
    );
}

#[tokio::test]
async fn equivalent_hidden_uuid_collision_preserves_private_conflict_contract() {
    let app = Application::new().await;
    let household_id = app.fixture.scenario.household.id;
    let token = app.token().await;
    let foreign = Fixture::new().await;
    let historical_uuid = app.fixture.client_uuid.to_string().to_uppercase();
    let now = chrono::Utc::now().naive_utc();
    medication_takes::ActiveModel {
        household_id: Set(foreign.scenario.household.id),
        client_uuid: Set(Some(historical_uuid.clone())),
        person_medication_id: Set(Some(foreign.assignment.id)),
        dose_amount: Set(Some(sea_orm::prelude::Decimal::new(2, 0))),
        dose_unit: Set(Some("tablet".into())),
        taken_at: Set(Some("2026-10-05T10:00:00".parse().unwrap())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let foreign_before = foreign.effect().await;
    let before = app.fixture.effect().await;
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/{household_id}/medication_takes",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&shared_take_body(&app.fixture))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    let after = app.fixture.effect().await;
    assert_eq!(foreign.effect().await, foreign_before);
    foreign.scenario.close().await;
    app.close().await;
    assert_eq!(status, 409);
    assert_eq!(body["error"]["code"], "idempotency_key_unavailable");
    assert!(!body.to_string().contains(&historical_uuid));
    assert_eq!(after, before);
}

#[tokio::test]
async fn stock_removal_api_replays_and_lists_without_second_effect() {
    let app = Application::new().await;
    let household_id = app.fixture.scenario.household.id;
    let medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/medications/{medication_id}/stock_removals",
        app.origin
    );
    let submission_id = uuid::Uuid::new_v4().to_string();
    let body =
        json!({"stock_removal":{"quantity":"2","reason":"dropped","submission_id":submission_id}});
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap();
    let status = first.status().as_u16();
    let first: Value = first.json().await.unwrap_or(Value::Null);
    let replay = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let replay: Value = replay.json().await.unwrap_or(Value::Null);
    let changed = app.client.post(&endpoint).bearer_auth(&token).json(&json!({"stock_removal":{"quantity":"3","reason":"dropped","submission_id":submission_id}})).send().await.unwrap();
    let changed_status = changed.status().as_u16();
    let history = app
        .client
        .get(format!("{endpoint}?page=1&per_page=1"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let history_status = history.status().as_u16();
    let history: Value = history.json().await.unwrap_or(Value::Null);
    let effect = app.fixture.effect().await;
    let events = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(household_id))
        .filter(versions::Column::ItemType.eq("MedicationStockRemoval"))
        .filter(versions::Column::ItemId.eq(medication_id))
        .count(&app.fixture.admin)
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 201);
    assert_eq!(replay_status, 201);
    assert_eq!(first, replay);
    assert_eq!(changed_status, 422);
    assert_eq!(history_status, 200);
    assert_eq!(history["data"][0], first["data"]);
    assert_eq!(history["meta"]["total_count"], 1);
    assert_eq!(effect, (0, "8.00".into(), 0, 0));
    assert_eq!(events, 1);
}

#[tokio::test]
async fn unauthenticated_care_response_has_bearer_challenge() {
    let app = Application::new().await;
    let household_id = app.fixture.scenario.household.id;
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/{household_id}/medication_takes",
            app.origin
        ))
        .json(&shared_take_body(&app.fixture))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let challenge = response
        .headers()
        .get("www-authenticate")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    app.close().await;
    assert_eq!(status, 401);
    assert_eq!(challenge.as_deref(), Some("Bearer"));
}

#[tokio::test]
async fn valid_token_without_clinical_scope_returns_library_challenge() {
    let app = Application::new().await;
    let household_id = app.fixture.scenario.household.id;
    let original = app.token_pair().await;
    let request = Request::builder()
        .method("POST")
        .uri("/token")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(
            serde_urlencoded::to_string([
                ("grant_type", "refresh_token"),
                ("client_id", app.fixture.client_id.as_str()),
                ("refresh_token", original["refresh_token"].as_str().unwrap()),
                ("scope", "offline_access"),
            ])
            .unwrap(),
        ))
        .unwrap();
    let narrowed = med_tracker::models::identity::exchange(&app.fixture.runtime, request, None)
        .await
        .unwrap();
    let token = narrowed["access_token"].as_str().unwrap();
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/{household_id}/medication_takes",
            app.origin
        ))
        .bearer_auth(token)
        .json(&shared_take_body(&app.fixture))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let challenge = response
        .headers()
        .get("www-authenticate")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let effect = app.fixture.effect().await;
    app.close().await;
    assert_eq!(status, 403);
    assert!(challenge.starts_with("Bearer "));
    assert!(challenge.contains("insufficient_scope"));
    assert!(challenge.contains("medtracker"));
    assert_eq!(effect, (0, "10.00".into(), 0, 0));
}

#[tokio::test]
async fn enabled_browser_sessions_do_not_attach_cookies_to_bearer_api() {
    let app = Application::new().await;
    let household_id = app.fixture.scenario.household.id;
    let login = app
        .client
        .get(format!("{}/login", app.origin))
        .send()
        .await
        .unwrap();
    let login_status = login.status().as_u16();
    let token = app.token().await;
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/{household_id}/medication_takes",
            app.origin
        ))
        .bearer_auth(token)
        .json(&shared_take_body(&app.fixture))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let cookies = response.headers().get_all("set-cookie").iter().count();
    let effect = app.fixture.effect().await;
    app.close().await;
    assert_eq!(login_status, 200);
    assert_eq!(status, 201);
    assert_eq!(
        cookies, 0,
        "Bearer API must remain independent of browser cookies"
    );
    assert_eq!(effect, (1, "8.00".into(), 1, 1));
}

#[tokio::test]
async fn stock_removal_api_checks_manager_permission_before_invalid_payload() {
    let app = Application::new().await;
    let household_id = app.fixture.scenario.household.id;
    let medication_id = app.fixture.medication.id;
    let token = app.token().await;
    app.fixture.member().await;
    let endpoint = format!(
        "{}/api/v1/households/{household_id}/medications/{medication_id}/stock_removals",
        app.origin
    );
    let response = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let history = app
        .client
        .get(format!("{endpoint}?page=bad"))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    let history_status = history.status().as_u16();
    let effect = app.fixture.effect().await;
    app.close().await;
    assert_eq!(status, 403);
    assert_eq!(history_status, 403);
    assert_eq!(effect, (0, "10.00".into(), 0, 0));
}

#[tokio::test]
async fn assert_status_returns_the_parsed_json_body() {
    let body: reqwest::Body = "{\"error\":{\"code\":\"synthetic\"}}".into();
    let response: reqwest::Response = axum::http::Response::builder()
        .status(200)
        .body(body)
        .unwrap()
        .into();
    let payload = Application::assert_status(response, 200).await;
    assert_eq!(payload["error"]["code"], "synthetic");
}

#[tokio::test]
#[should_panic(expected = "synthetic explanation")]
async fn assert_status_reports_the_response_body_on_failure() {
    let body: reqwest::Body = "{\"error\":\"synthetic explanation\"}".into();
    let response: reqwest::Response = axum::http::Response::builder()
        .status(422)
        .body(body)
        .unwrap()
        .into();
    Application::assert_status(response, 200).await;
}
