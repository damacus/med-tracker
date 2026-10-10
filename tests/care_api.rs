#[path = "care_api/account_sessions.rs"]
mod account_sessions;
#[path = "care_api/administration.rs"]
mod administration;
#[path = "care_api/api_session.rs"]
mod api_session;
#[path = "care_api/assignments.rs"]
mod assignments;
#[path = "care_api/avatar.rs"]
mod avatar;
#[path = "care_api/avatar_storage.rs"]
mod avatar_storage;
#[path = "care_api/better_auth_store.rs"]
mod better_auth_store;
#[path = "care_api/browser_push.rs"]
mod browser_push;
#[path = "care_api/browser_push_reminders.rs"]
mod browser_push_reminders;
#[path = "care_api/crud.rs"]
mod crud;
#[path = "care_api/dosages.rs"]
mod dosages;
#[path = "care_api/dose_history.rs"]
mod dose_history;
#[path = "care_api/dose_outcomes.rs"]
mod dose_outcomes;
#[path = "care_api/fixture.rs"]
mod fixture;
#[path = "care_api/health_events.rs"]
mod health_events;
#[path = "care_api/invitations.rs"]
mod invitations;
#[path = "care_api/legacy_pause.rs"]
mod legacy_pause;
#[path = "care_api/locations.rs"]
mod locations;
#[path = "care_api/medication_reads.rs"]
mod medication_reads;
#[path = "care_api/notification_preferences.rs"]
mod notification_preferences;
#[path = "care_api/pause_periods.rs"]
mod pause_periods;
#[path = "care_api/people.rs"]
mod people;
#[path = "care_api/person_carers.rs"]
mod person_carers;
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
#[path = "care_api/saved_status.rs"]
mod saved_status;
#[path = "care_api/schedule_lifecycle.rs"]
mod schedule_lifecycle;
#[path = "care_api/schedule_scope.rs"]
mod schedule_scope;
#[path = "care_api/signup_timezone.rs"]
mod signup_timezone;
#[path = "care_api/stock_orders.rs"]
mod stock_orders;
#[path = "care_api/sync.rs"]
mod sync;
#[path = "care_api/treatment_timestamps.rs"]
mod treatment_timestamps;
#[path = "care_api/treatments.rs"]
mod treatments;

use axum::{body::Body, http::Request};
use fixture::Fixture;
use loco_rs::{
    app::{AppContext, Hooks},
    boot::StartMode,
    config::{Config, QueueConfig},
    environment::Environment,
};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};

struct Application {
    fixture: Fixture,
    context: AppContext,
    origin: String,
    client: reqwest::Client,
    server: tokio::task::JoinHandle<()>,
}

impl Application {
    async fn avatar_bucket_server(
        &self,
        bucket: &str,
    ) -> (String, AppContext, tokio::task::JoinHandle<()>) {
        let mut config = Config::new(&Environment::Test).unwrap();
        config.database.uri = self.fixture.runtime_uri.clone();
        let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
            panic!("PostgreSQL queue required")
        };
        queue.uri = self.fixture.runtime_uri.clone();
        config.settings.get_or_insert_with(|| json!({}))["avatar_storage"] =
            json!({"bucket":bucket});
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
        let fixture = Fixture::new().await;
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
        config.database.uri = fixture.runtime_uri.clone();
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
        med_tracker::models::identity::exchange(&self.fixture.runtime, request)
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

fn take_body() -> Value {
    json!({"medication_take": {
        "client_uuid":"006b49d9-1da2-42f1-800b-8c80867aee1c", "source_type":"person_medication", "source_id":"81001",
        "taken_at":"2026-10-05T10:00:00Z", "dose_amount":"2", "dose_unit":"tablet", "taken_from_medication_id":80001
    }})
}

#[tokio::test]
async fn care_routes_reject_unvalidated_requests_without_clinical_effects() {
    let app = Application::new().await;
    let take = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/medication_takes",
            app.origin
        ))
        .json(&take_body())
        .send()
        .await
        .unwrap();
    let stock = app
        .client
        .patch(format!(
            "{}/api/v1/households/72001/medications/80001/adjust_inventory",
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
    let token = app.token().await;
    let endpoint = format!("{}/api/v1/households/72001/medication_takes", app.origin);
    let first = app
        .client
        .post(&endpoint)
        .bearer_auth(&token)
        .json(&take_body())
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
        .json(&take_body())
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let replay_body: Value = replay.json().await.unwrap_or(Value::Null);
    let replay_effect = app.fixture.effect().await;
    let stock = app
        .client
        .patch(format!(
            "{}/api/v1/households/72001/medications/80001/adjust_inventory",
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
    let audit = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT count(*) AS count FROM versions WHERE item_type='Medication' AND item_id=80001 AND event='adjust inventory (qty: 12, reason: synthetic count)'" )).await.unwrap().unwrap();
    let stock_audits: i64 = audit.try_get("", "count").unwrap();
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
async fn authenticated_foreign_household_and_revoked_person_grant_have_no_effects() {
    let app = Application::new().await;
    let token = app.token().await;
    let foreign = app
        .client
        .post(format!(
            "{}/api/v1/households/99999/medication_takes",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&take_body())
        .send()
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET revoked_at=now() WHERE id=78001")
        .await
        .unwrap();
    let revoked = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/medication_takes",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&take_body())
        .send()
        .await
        .unwrap();
    let statuses = (foreign.status().as_u16(), revoked.status().as_u16());
    let effect = app.fixture.effect().await;
    app.close().await;
    assert_eq!(statuses, (403, 404));
    assert_eq!(effect, (0, "10.00".into(), 0, 0));
}

#[tokio::test]
async fn failed_clinical_audit_rolls_back_effects_but_records_http_attempt() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("CREATE FUNCTION synthetic_care_api_audit_failure() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='MedicationTake' THEN RAISE EXCEPTION 'synthetic clinical audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER synthetic_care_api_audit_failure BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION synthetic_care_api_audit_failure()").await.unwrap();
    let reply = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/medication_takes",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&take_body())
        .send()
        .await
        .unwrap();
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
    let row = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres, "SELECT (SELECT count(*) FROM versions) AS clinical_versions, (SELECT count(*) FROM security_audit_events WHERE event_type='api.request' AND request_id=$1 AND metadata->>'status'='500') AS attempts", [request_id.clone().into()])).await.unwrap().unwrap();
    let versions: i64 = row.try_get("", "clinical_versions").unwrap();
    let attempts: i64 = row.try_get("", "attempts").unwrap();
    app.close().await;
    assert_eq!(status, 500);
    assert_eq!(body["error"]["request_id"], request_id);
    assert!(
        !body
            .to_string()
            .contains("synthetic clinical audit rejection")
    );
    assert_eq!(effect, (0, "10.00".into(), 0, 0));
    assert_eq!(versions, 0);
    assert_eq!(attempts, 1);
}

#[tokio::test]
async fn equivalent_hidden_uuid_collision_preserves_private_conflict_contract() {
    let app = Application::new().await;
    let token = app.token().await;
    app.fixture.admin.execute_unprepared("INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(92001,71001,'Other synthetic household','api-uuid-foreign-household','UTC',now(),now()); INSERT INTO people(id,household_id,name,person_type,has_capacity,created_at,updated_at) VALUES(92002,92001,'Other synthetic adult',0,true,now(),now()); INSERT INTO locations(id,household_id,name,created_at,updated_at) VALUES(92003,92001,'Other synthetic cabinet',now(),now()); INSERT INTO medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at) VALUES(92004,92001,92003,'Other synthetic tablets',10,2,'tablet',now(),now()); INSERT INTO person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at) VALUES(92005,92001,92002,92004,2,'tablet',0,now(),now()); INSERT INTO medication_takes(id,household_id,client_uuid,person_medication_id,dose_amount,dose_unit,taken_at,created_at,updated_at) VALUES(92006,92001,'006B49D9-1DA2-42F1-800B-8C80867AEE1C',92005,2,'tablet','2026-10-05T10:00:00',now(),now())").await.unwrap();
    let before = app.fixture.effect().await;
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/medication_takes",
            app.origin
        ))
        .bearer_auth(&token)
        .json(&take_body())
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let body: Value = response.json().await.unwrap();
    let after = app.fixture.effect().await;
    app.close().await;
    assert_eq!(status, 409);
    assert_eq!(body["error"]["code"], "idempotency_key_unavailable");
    assert!(
        !body
            .to_string()
            .contains("006B49D9-1DA2-42F1-800B-8C80867AEE1C")
    );
    assert_eq!(after, before);
}

#[tokio::test]
async fn stock_removal_api_replays_and_lists_without_second_effect() {
    let app = Application::new().await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/72001/medications/80001/stock_removals",
        app.origin
    );
    let body = json!({"stock_removal":{"quantity":"2","reason":"dropped","submission_id":"206b49d9-1da2-42f1-800b-8c80867aee1c"}});
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
    let changed = app.client.post(&endpoint).bearer_auth(&token).json(&json!({"stock_removal":{"quantity":"3","reason":"dropped","submission_id":"206b49d9-1da2-42f1-800b-8c80867aee1c"}})).send().await.unwrap();
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
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS count FROM versions WHERE item_type='MedicationStockRemoval' AND item_id=80001")).await.unwrap().unwrap();
    let events: i64 = row.try_get("", "count").unwrap();
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
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/medication_takes",
            app.origin
        ))
        .json(&take_body())
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
    let original = app.token_pair().await;
    let request = Request::builder()
        .method("POST")
        .uri("/token")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(
            serde_urlencoded::to_string([
                ("grant_type", "refresh_token"),
                ("client_id", "native"),
                ("refresh_token", original["refresh_token"].as_str().unwrap()),
                ("scope", "offline_access"),
            ])
            .unwrap(),
        ))
        .unwrap();
    let narrowed = med_tracker::models::identity::exchange(&app.fixture.runtime, request)
        .await
        .unwrap();
    let token = narrowed["access_token"].as_str().unwrap();
    let response = app
        .client
        .post(format!(
            "{}/api/v1/households/72001/medication_takes",
            app.origin
        ))
        .bearer_auth(token)
        .json(&take_body())
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
            "{}/api/v1/households/72001/medication_takes",
            app.origin
        ))
        .bearer_auth(token)
        .json(&take_body())
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
    let token = app.token().await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let endpoint = format!(
        "{}/api/v1/households/72001/medications/80001/stock_removals",
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
