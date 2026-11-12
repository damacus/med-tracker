use super::*;
use med_tracker::models::_entities as rows;
use sea_orm::QueryOrder;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::sea_query::{Expr, ExprTrait};

fn suggestions_url(app: &Application) -> String {
    let fixture_household_id = app.fixture.household.id;
    format!(
        "{}/api/v1/households/{fixture_household_id}/ai_medication_suggestions",
        app.origin
    )
}

async fn upgrade_plan(app: &Application) {
    let mut household = app.fixture.household.clone().into_active_model();
    household.subscription_plan = Set("family_plus".into());
    household.update(&app.fixture.admin).await.unwrap();
}

async fn suggestion_audit(app: &Application) -> Option<Value> {
    let row = rows::versions::Entity::find()
        .filter(rows::versions::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(rows::versions::Column::ItemType.eq("AiMedicationSuggestion"))
        .one(&app.fixture.admin)
        .await
        .unwrap()?;
    let object: Value = serde_json::from_str(row.object.as_deref().unwrap()).unwrap();
    Some(json!({"event":row.event,"household_id":row.household_id,"object":object}))
}

async fn suggestion_access_state(app: &Application) -> Value {
    let household = app.fixture.household.id;
    macro_rules! records {
        ($entity:ident) => {
            rows::$entity::Entity::find()
                .filter(rows::$entity::Column::HouseholdId.eq(household))
                .order_by_asc(rows::$entity::Column::Id)
                .all(&app.fixture.admin)
                .await
                .unwrap()
        };
    }
    let versions = rows::versions::Entity::find()
        .filter(rows::versions::Column::HouseholdId.eq(household))
        .filter(rows::versions::Column::ItemType.is_in([
            "Medication",
            "PersonMedication",
            "Schedule",
            "MedicationTake",
        ]))
        .order_by_asc(rows::versions::Column::Id)
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let suggestions = rows::versions::Entity::find()
        .filter(rows::versions::Column::HouseholdId.eq(household))
        .filter(rows::versions::Column::ItemType.eq("AiMedicationSuggestion"))
        .order_by_asc(rows::versions::Column::Id)
        .all(&app.fixture.admin)
        .await
        .unwrap();
    let requests = rows::security_audit_events::Entity::find()
        .filter(rows::security_audit_events::Column::HouseholdId.eq(household))
        .filter(rows::security_audit_events::Column::EventType.eq("api.request"))
        .filter(
            Expr::col(rows::security_audit_events::Column::Metadata)
                .cast_json_field("controller")
                .eq("api/v1/ai_medication_suggestions"),
        )
        .order_by_asc(rows::security_audit_events::Column::Id)
        .all(&app.fixture.admin)
        .await
        .unwrap();
    json!({"clinical":{"medications":records!(medications),"assignments":records!(person_medications),"schedules":records!(schedules),"takes":records!(medication_takes),"versions":versions,"changes":records!(api_change_events)},"suggestions":suggestions,"requests":requests})
}

#[tokio::test]
async fn ai_medication_suggestions_require_authentication_and_access() {
    let app = Application::new_with_ai_help(true).await;
    let fixture_account_id = app.fixture.account.id;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    upgrade_plan(&app).await;
    let token = app.token().await;
    Application::assert_status(change_consent(&app, &token, true).await, 200).await;
    assert_eq!(
        app.context.config.settings.as_ref().unwrap()["ai_medication_help"],
        json!({"enabled": true})
    );
    let initial = suggestion_access_state(&app).await;
    let unauthenticated = app
        .client
        .post(suggestions_url(&app))
        .json(&json!({"medication": {"name": "Synthetic medicine"}}))
        .send()
        .await
        .unwrap();
    Application::assert_status(unauthenticated, 401).await;
    let unauthenticated_state = suggestion_access_state(&app).await;
    assert_eq!(unauthenticated_state["clinical"], initial["clinical"]);
    assert_eq!(unauthenticated_state["suggestions"], json!([]));

    app.fixture.member().await;
    rows::person_access_grants::Entity::update_many()
        .col_expr(
            rows::person_access_grants::Column::RevokedAt,
            Expr::val(chrono::Utc::now().naive_utc()),
        )
        .filter(
            rows::person_access_grants::Column::HouseholdMembershipId.eq(app.fixture.membership.id),
        )
        .exec(&app.fixture.admin)
        .await
        .unwrap();
    let participating = app
        .client
        .post(suggestions_url(&app))
        .bearer_auth(&token)
        .json(&json!({"medication": {"name": "Synthetic medicine"}}))
        .send()
        .await
        .unwrap();
    let payload = Application::assert_status(participating, 200).await;
    assert_eq!(
        payload,
        json!({"data":{"medication":{},"doses":[],"sources":[],"errors":["ruby_llm_unconfigured"]}})
    );
    let accepted = suggestion_access_state(&app).await;
    assert_eq!(accepted["clinical"], initial["clinical"]);
    let suggestions = accepted["suggestions"].as_array().unwrap();
    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0]["event"], "ai_medication/suggestion");
    assert_eq!(suggestions[0]["whodunnit"], fixture_account_id.to_string());
    assert_eq!(suggestions[0]["actor_membership_id"], fixture_membership_id);
    assert_eq!(suggestions[0]["household_id"], fixture_household_id);
    let audit = suggestion_audit(&app).await.unwrap();
    assert_eq!(audit["object"]["source_count"], 0);
    assert_eq!(audit["object"]["dose_count"], 0);
    assert_eq!(audit["object"]["error_count"], 1);
    assert_eq!(audit["object"]["result_status"], "error");
    let requests = accepted["requests"].as_array().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0]["actor_account_id"], fixture_account_id);
    assert_eq!(requests[0]["actor_membership_id"], fixture_membership_id);
    assert_eq!(requests[0]["metadata"]["status"], 200);
    assert_eq!(requests[0]["metadata"]["outcome"], "success");
    assert_eq!(requests[0]["request_id"], suggestions[0]["request_id"]);

    let mut member = app.fixture.membership.clone().into_active_model();
    member.status = Set("revoked".into());
    member.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    member.permissions_version = Set(app.fixture.membership.permissions_version + 1);
    member.update(&app.fixture.admin).await.unwrap();
    let denied = app
        .client
        .post(suggestions_url(&app))
        .bearer_auth(&token)
        .json(&json!({"medication": {"name": "Synthetic medicine"}}))
        .send()
        .await
        .unwrap();
    let payload = Application::assert_status(denied, 403).await;
    assert_eq!(payload["error"]["code"], "forbidden");
    let revoked = suggestion_access_state(&app).await;
    assert_eq!(revoked["clinical"], accepted["clinical"]);
    assert_eq!(revoked["suggestions"], accepted["suggestions"]);
    assert_eq!(
        revoked["requests"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|request| request["metadata"]["status"] == 200)
            .count(),
        1
    );
    app.close().await;
}

#[tokio::test]
async fn ai_medication_suggestions_requires_enabled_paid_feature() {
    let app = Application::new_with_ai_help(true).await;
    let token = app.token().await;
    let unpaid = app
        .client
        .post(suggestions_url(&app))
        .bearer_auth(&token)
        .json(&json!({"medication": {"name": "Synthetic medicine"}}))
        .send()
        .await
        .unwrap();
    let payload = Application::assert_status(unpaid, 404).await;
    assert_eq!(payload["error"]["code"], "not_found");

    let disabled = Application::new_with_ai_help(false).await;
    upgrade_plan(&disabled).await;
    let disabled_token = disabled.token().await;
    let disabled_response = disabled
        .client
        .post(suggestions_url(&disabled))
        .bearer_auth(&disabled_token)
        .json(&json!({"medication": {"name": "Synthetic medicine"}}))
        .send()
        .await
        .unwrap();
    Application::assert_status(disabled_response, 404).await;
    disabled.close().await;
    app.close().await;
}

#[tokio::test]
async fn ai_medication_suggestions_validates_the_identity_shape() {
    let app = Application::new_with_ai_help(true).await;
    upgrade_plan(&app).await;
    let token = app.token().await;
    Application::assert_status(change_consent(&app, &token, true).await, 200).await;

    for body in [
        json!({"unlisted": true}),
        json!({"medication": {"name": "Synthetic medicine", "unlisted": "reject"}}),
        json!({"medication": {"name": 42}}),
        json!([1, 2, 3]),
    ] {
        let response = app
            .client
            .post(suggestions_url(&app))
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .unwrap();
        let payload = Application::assert_status(response, 422).await;
        assert_eq!(payload["error"]["code"], "validation_failed");
        assert_eq!(
            payload["error"]["errors"]["medication"],
            json!(["is invalid"])
        );
    }
    app.close().await;
}

#[tokio::test]
async fn ai_medication_suggestions_returns_a_draft_and_records_the_suggestion() {
    let app = Application::new_with_ai_help(true).await;
    let fixture_household_id = app.fixture.household.id;
    upgrade_plan(&app).await;
    let token = app.token().await;
    Application::assert_status(change_consent(&app, &token, true).await, 200).await;
    let response = app
        .client
        .post(suggestions_url(&app))
        .bearer_auth(&token)
        .json(&json!({"medication": {"name": "Synthetic medicine"}}))
        .send()
        .await
        .unwrap();
    let payload = Application::assert_status(response, 200).await;
    assert!(payload["data"]["medication"].is_object());
    assert!(payload["data"]["doses"].is_array());
    assert!(payload["data"]["sources"].is_array());
    assert!(payload["data"]["errors"].is_array());

    let audit = suggestion_audit(&app).await.expect("suggestion audit");
    assert_eq!(audit["event"], "ai_medication/suggestion");
    assert_eq!(audit["household_id"], fixture_household_id);
    assert!(
        audit["object"]["identity_hash"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64)
    );
    assert!(audit["object"]["source_count"].is_number());
    assert!(audit["object"]["dose_count"].is_number());
    assert!(audit["object"]["error_count"].is_number());
    assert!(audit["object"]["result_status"].is_string());
    app.close().await;
}

const CALPOL_SEED_URL: &str =
    "https://www.calpol.co.uk/our-products/calpol-sixplus-oral-suspension-paracetamol";
const CALPOL_EVIDENCE: &str = "Each 5 ml contains 250 mg paracetamol. Give every 4 to 6 hours as needed. Do not give more than 4 doses in 24 hours.";

fn ai_mock_server() -> u16 {
    ai_counting_server().0
}

fn ai_counting_server() -> (u16, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
    ai_gated_server(None)
}

type SourceGate = (
    tokio::sync::oneshot::Sender<()>,
    std::sync::mpsc::Receiver<()>,
);

fn ai_gated_server(
    gate: Option<SourceGate>,
) -> (u16, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
    let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = count.clone();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        let mut gate = gate;
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buffer = [0_u8; 8192];
            let Ok(read) = std::io::Read::read(&mut stream, &mut buffer) else {
                continue;
            };
            let request = String::from_utf8_lossy(&buffer[..read]);
            observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let (content_type, body) = if request.starts_with("GET /fetch") {
                if let Some((entered, release)) = gate.take() {
                    let _ = entered.send(());
                    release
                        .recv_timeout(std::time::Duration::from_secs(4))
                        .unwrap();
                }
                (
                    "text/html",
                    format!(
                        "<html><head><title>CALPOL SixPlus Oral Suspension</title></head><body>{CALPOL_EVIDENCE}</body></html>"
                    ),
                )
            } else {
                let draft = json!({
                    "medication": {"name": "Calpol SixPlus", "description": "Each 5 ml contains 250 mg paracetamol."},
                    "doses": [{
                        "amount": "5", "unit": "ml",
                        "default_max_daily_doses": "4",
                        "default_min_hours_between_doses": "4",
                        "default_dose_cycle": "daily",
                        "evidence": {"url": CALPOL_SEED_URL, "text": CALPOL_EVIDENCE}
                    }],
                    "sources": [{"url": CALPOL_SEED_URL}],
                    "errors": []
                });
                (
                    "application/json",
                    json!({"choices":[{"message":{"content":draft.to_string()}}]}).to_string(),
                )
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = std::io::Write::write_all(&mut stream, response.as_bytes());
        }
    });
    (port, count)
}

#[tokio::test]
async fn ai_consent_withdrawal_stops_later_in_flight_provider_send() {
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let (port, count) = ai_gated_server(Some((entered_tx, release_rx)));
    let app = Application::new_with_ai_generation(port).await;
    upgrade_plan(&app).await;
    let token = app.token().await;
    Application::assert_status(change_consent(&app, &token, true).await, 200).await;
    let request = app
        .client
        .post(suggestions_url(&app))
        .bearer_auth(&token)
        .json(&json!({"medication":{"name":"calpol sixplus"}}));
    let suggestion = tokio::spawn(async move { request.send().await.unwrap() });
    tokio::time::timeout(std::time::Duration::from_secs(3), entered_rx)
        .await
        .unwrap()
        .unwrap();
    let request = app
        .client
        .patch(format!(
            "{}/api/v1/households/{}/admin/settings",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(&token)
        .json(&json!({"household":{"ai_suggestions_enabled":false}}));
    let withdrawal = tokio::spawn(async move { request.send().await.unwrap() });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            use sea_orm::sea_query::{Alias, Query};
            let query = Query::select()
                .expr_as(Expr::col(Alias::new("pid")).count(), Alias::new("waiting"))
                .from(Alias::new("pg_stat_activity"))
                .and_where(Expr::col(Alias::new("wait_event_type")).eq("Lock"))
                .and_where(Expr::col(Alias::new("query")).like("%households%FOR UPDATE%"))
                .to_owned();
            let waiting: i64 = app
                .fixture
                .admin
                .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
                .await
                .unwrap()
                .unwrap()
                .try_get("", "waiting")
                .unwrap();
            if waiting > 0 || withdrawal.is_finished() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    release_tx.send(()).unwrap();
    Application::assert_status(withdrawal.await.unwrap(), 200).await;
    Application::assert_status(suggestion.await.unwrap(), 404).await;
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 1);
    let response = app
        .client
        .post(suggestions_url(&app))
        .bearer_auth(&token)
        .json(&json!({"medication":{"name":"calpol sixplus"}}))
        .send()
        .await
        .unwrap();
    Application::assert_status(response, 404).await;
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 1);
    app.close().await;
}

#[tokio::test]
async fn ai_consent_defaults_off_without_outbound_requests() {
    let (port, count) = ai_counting_server();
    let app = Application::new_with_ai_generation(port).await;
    upgrade_plan(&app).await;
    let token = app.token().await;
    let response = app
        .client
        .post(suggestions_url(&app))
        .bearer_auth(token)
        .json(&json!({"medication":{"name":"calpol sixplus"}}))
        .send()
        .await
        .unwrap();
    Application::assert_status(response, 404).await;
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 0);
    app.close().await;
}

async fn change_consent(app: &Application, token: &str, enabled: bool) -> reqwest::Response {
    app.client
        .patch(format!(
            "{}/api/v1/households/{}/admin/settings",
            app.origin, app.fixture.household.id
        ))
        .bearer_auth(token)
        .json(&json!({"household":{"ai_suggestions_enabled":enabled}}))
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn ai_consent_manager_update_and_withdrawal_stop_outbound_requests() {
    let (port, count) = ai_counting_server();
    let app = Application::new_with_ai_generation(port).await;
    upgrade_plan(&app).await;
    let token = app.token().await;
    let enabled = Application::assert_status(change_consent(&app, &token, true).await, 200).await;
    assert_eq!(enabled["data"]["ai_suggestions_enabled"], true);
    let response = app
        .client
        .post(suggestions_url(&app))
        .bearer_auth(&token)
        .json(&json!({"medication":{"name":"calpol sixplus"}}))
        .send()
        .await
        .unwrap();
    Application::assert_status(response, 200).await;
    assert!(count.load(std::sync::atomic::Ordering::SeqCst) >= 2);
    let withdrawn =
        Application::assert_status(change_consent(&app, &token, false).await, 200).await;
    assert_eq!(withdrawn["data"]["ai_suggestions_enabled"], false);
    let before = count.load(std::sync::atomic::Ordering::SeqCst);
    let response = app
        .client
        .post(suggestions_url(&app))
        .bearer_auth(&token)
        .json(&json!({"medication":{"name":"calpol sixplus"}}))
        .send()
        .await
        .unwrap();
    Application::assert_status(response, 404).await;
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), before);
    app.close().await;
}

#[tokio::test]
async fn ai_consent_settings_preserve_omissions_reject_invalid_and_nonmanagers() {
    let (port, count) = ai_counting_server();
    let app = Application::new_with_ai_generation(port).await;
    let token = app.token().await;
    let endpoint = format!(
        "{}/api/v1/households/{}/admin/settings",
        app.origin, app.fixture.household.id
    );
    let initial = Application::assert_status(
        app.client
            .get(&endpoint)
            .bearer_auth(&token)
            .send()
            .await
            .unwrap(),
        200,
    )
    .await;
    assert_eq!(initial["data"]["ai_suggestions_enabled"], false);
    Application::assert_status(change_consent(&app, &token, true).await, 200).await;
    let repeated = Application::assert_status(change_consent(&app, &token, true).await, 200).await;
    assert_eq!(repeated["data"]["ai_suggestions_enabled"], true);
    let unrelated = app
        .client
        .patch(&endpoint)
        .bearer_auth(&token)
        .json(&json!({"household":{"name":"Synthetic renamed household"}}))
        .send()
        .await
        .unwrap();
    let updated = Application::assert_status(unrelated, 200).await;
    assert_eq!(updated["data"]["ai_suggestions_enabled"], true);
    for invalid in [Value::Null, json!("true"), json!(1)] {
        let response = app
            .client
            .patch(&endpoint)
            .bearer_auth(&token)
            .json(&json!({"household":{"ai_suggestions_enabled":invalid}}))
            .send()
            .await
            .unwrap();
        Application::assert_status(response, 422).await;
    }
    app.fixture.member().await;
    Application::assert_status(change_consent(&app, &token, false).await, 403).await;
    let saved = rows::households::Entity::find_by_id(app.fixture.household.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let saved = serde_json::to_value(saved).unwrap();
    assert_eq!(saved["ai_suggestions_enabled"], true);
    let audits = rows::versions::Entity::find()
        .filter(rows::versions::Column::HouseholdId.eq(app.fixture.household.id))
        .filter(rows::versions::Column::ItemType.eq("Household"))
        .all(&app.fixture.admin)
        .await
        .unwrap();
    assert_eq!(audits.len(), 2);
    assert!(
        audits
            .iter()
            .all(|audit| audit.whodunnit.as_deref() == Some(&app.fixture.account.id.to_string()))
    );
    assert!(audits.iter().any(|audit| {
        audit
            .object
            .as_ref()
            .is_some_and(|object| object.as_str().contains("ai_suggestions_enabled"))
    }));
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 0);
    app.close().await;
}

#[tokio::test]
async fn ai_consent_foreign_and_revoked_callers_send_nothing() {
    let (port, count) = ai_counting_server();
    let app = Application::new_with_ai_generation(port).await;
    upgrade_plan(&app).await;
    let token = app.token().await;
    Application::assert_status(change_consent(&app, &token, true).await, 200).await;
    let foreign = app.fixture.foreign_household().await;
    let endpoint = format!(
        "{}/api/v1/households/{}/ai_medication_suggestions",
        app.origin, foreign.household.id
    );
    let response = app
        .client
        .post(endpoint)
        .bearer_auth(&token)
        .json(&json!({"medication":{"name":"calpol sixplus"}}))
        .send()
        .await
        .unwrap();
    assert!(!response.status().is_success());
    let response = app
        .client
        .patch(format!(
            "{}/api/v1/households/{}/admin/settings",
            app.origin, foreign.household.id
        ))
        .bearer_auth(&token)
        .json(&json!({"household":{"ai_suggestions_enabled":true}}))
        .send()
        .await
        .unwrap();
    assert!(!response.status().is_success());
    let mut member = app.fixture.membership.clone().into_active_model();
    member.status = Set("revoked".into());
    member.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    member.permissions_version = Set(app.fixture.membership.permissions_version + 1);
    member.update(&app.fixture.admin).await.unwrap();
    let response = app
        .client
        .post(suggestions_url(&app))
        .bearer_auth(&token)
        .json(&json!({"medication":{"name":"calpol sixplus"}}))
        .send()
        .await
        .unwrap();
    Application::assert_status(response, 403).await;
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 0);
    foreign.close().await;
    app.close().await;
}

#[tokio::test]
async fn ai_consent_unreadable_database_fails_closed_without_clinical_errors() {
    use sea_orm::TransactionTrait;
    let (port, count) = ai_counting_server();
    let app = Application::new_with_pool_size(Some("synthetic-dose-occurrence-signing-key"), move |settings| {
        settings["ai_medication_help"] = json!({"enabled":true,"provider":"openai","api_key":"synthetic-ai-key","model":"gpt-4o","api_base_url":format!("http://127.0.0.1:{port}/v1/"),"source_proxy_origin":format!("http://127.0.0.1:{port}")});
    }, Some(1), Some(100)).await;
    upgrade_plan(&app).await;
    let token = app.token().await;
    Application::assert_status(change_consent(&app, &token, true).await, 200).await;
    let held = app.context.db.begin().await.unwrap();
    let response = app
        .client
        .post(suggestions_url(&app))
        .bearer_auth(&token)
        .json(&json!({"medication":{"name":"Synthetic private clinical sentinel"}}))
        .send()
        .await
        .unwrap();
    let payload = Application::assert_status(response, 503).await;
    assert!(!payload.to_string().as_str().contains("clinical sentinel"));
    assert!(!payload.to_string().as_str().contains("synthetic-ai-key"));
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 0);
    held.rollback().await.unwrap();
    app.close().await;
}

#[tokio::test]
async fn ai_medication_suggestions_validates_doses_against_fetched_evidence() {
    let port = ai_mock_server();
    let app = Application::new_with_ai_generation(port).await;
    upgrade_plan(&app).await;
    let token = app.token().await;
    Application::assert_status(change_consent(&app, &token, true).await, 200).await;
    let response = app
        .client
        .post(suggestions_url(&app))
        .bearer_auth(&token)
        .json(&json!({"medication": {"name": "calpol sixplus"}}))
        .send()
        .await
        .unwrap();
    let payload = Application::assert_status(response, 200).await;
    let audit = suggestion_audit(&app).await;
    app.close().await;
    let doses = payload["data"]["doses"].as_array().unwrap();
    assert_eq!(doses.len(), 1, "body: {payload}");
    assert_eq!(doses[0]["amount"], "5");
    assert_eq!(doses[0]["unit"], "ml");
    assert_eq!(doses[0]["evidence"]["url"], CALPOL_SEED_URL);
    assert_eq!(payload["data"]["sources"][0]["url"], CALPOL_SEED_URL);
    assert_eq!(audit.unwrap()["object"]["dose_count"], 1);
}
