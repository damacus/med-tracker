use super::*;

fn suggestions_url(app: &Application) -> String {
    format!(
        "{}/api/v1/households/72001/ai_medication_suggestions",
        app.origin
    )
}

async fn upgrade_plan(app: &Application) {
    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE households SET subscription_plan = 'family_plus' WHERE id = 72001",
        )
        .await
        .unwrap();
}

async fn suggestion_audit(app: &Application) -> Option<Value> {
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT event, object, household_id FROM versions WHERE item_type = 'AiMedicationSuggestion'",
        ))
        .await
        .unwrap()?;
    let object: Value =
        serde_json::from_str(&row.try_get::<String>("", "object").unwrap()).unwrap();
    Some(json!({
        "event": row.try_get::<String>("", "event").unwrap(),
        "household_id": row.try_get::<i64>("", "household_id").unwrap(),
        "object": object,
    }))
}

async fn suggestion_access_state(app: &Application) -> Value {
    app.fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT jsonb_build_object(
              'clinical',jsonb_build_object(
                'medications',(SELECT jsonb_agg(to_jsonb(m) ORDER BY m.id) FROM medications m),
                'assignments',(SELECT jsonb_agg(to_jsonb(m) ORDER BY m.id) FROM person_medications m),
                'schedules',(SELECT jsonb_agg(to_jsonb(s) ORDER BY s.id) FROM schedules s),
                'takes',(SELECT jsonb_agg(to_jsonb(t) ORDER BY t.id) FROM medication_takes t),
                'versions',(SELECT jsonb_agg(to_jsonb(v) ORDER BY v.id) FROM versions v WHERE item_type IN ('Medication','PersonMedication','Schedule','MedicationTake')),
                'changes',(SELECT jsonb_agg(to_jsonb(c) ORDER BY c.id) FROM api_change_events c)),
              'suggestions',COALESCE((SELECT jsonb_agg(to_jsonb(v) ORDER BY v.id) FROM versions v WHERE item_type='AiMedicationSuggestion'),'[]'::jsonb),
              'requests',COALESCE((SELECT jsonb_agg(to_jsonb(a) ORDER BY a.id) FROM security_audit_events a WHERE event_type='api.request' AND metadata->>'controller'='api/v1/ai_medication_suggestions'),'[]'::jsonb)
            ) AS state",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "state")
        .unwrap()
}

#[tokio::test]
async fn ai_medication_suggestions_require_authentication_and_access() {
    let app = Application::new_with_ai_help(true).await;
    upgrade_plan(&app).await;
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

    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE household_memberships SET role='member' WHERE id=74001;
             UPDATE person_access_grants SET revoked_at=now() WHERE household_membership_id=74001",
        )
        .await
        .unwrap();
    let token = app.token().await;
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
    assert_eq!(suggestions[0]["whodunnit"], "77001");
    assert_eq!(suggestions[0]["actor_membership_id"], 74001);
    assert_eq!(suggestions[0]["household_id"], 72001);
    let audit = suggestion_audit(&app).await.unwrap();
    assert_eq!(audit["object"]["source_count"], 0);
    assert_eq!(audit["object"]["dose_count"], 0);
    assert_eq!(audit["object"]["error_count"], 1);
    assert_eq!(audit["object"]["result_status"], "error");
    let requests = accepted["requests"].as_array().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0]["actor_account_id"], 71001);
    assert_eq!(requests[0]["actor_membership_id"], 74001);
    assert_eq!(requests[0]["metadata"]["status"], 200);
    assert_eq!(requests[0]["metadata"]["outcome"], "success");
    assert_eq!(requests[0]["request_id"], suggestions[0]["request_id"]);

    app.fixture
        .admin
        .execute_unprepared(
            "UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1
             WHERE id=74001 AND household_id=72001 AND person_id=(SELECT person_id FROM accounts WHERE id=71001)",
        )
        .await
        .unwrap();
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
        .post(format!(
            "{}/api/v1/households/72001/ai_medication_suggestions",
            disabled.origin
        ))
        .bearer_auth(&disabled_token)
        .json(&json!({"medication": {"name": "Synthetic medicine"}}))
        .send()
        .await
        .unwrap();
    Application::assert_status(disabled_response, 404).await;
    disabled.close().await;
}

#[tokio::test]
async fn ai_medication_suggestions_validates_the_identity_shape() {
    let app = Application::new_with_ai_help(true).await;
    upgrade_plan(&app).await;
    let token = app.token().await;

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
}

#[tokio::test]
async fn ai_medication_suggestions_returns_a_draft_and_records_the_suggestion() {
    let app = Application::new_with_ai_help(true).await;
    upgrade_plan(&app).await;
    let token = app.token().await;
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
    assert_eq!(audit["household_id"], 72001);
    assert!(
        audit["object"]["identity_hash"]
            .as_str()
            .is_some_and(|hash| hash.len() == 64)
    );
    assert!(audit["object"]["source_count"].is_number());
    assert!(audit["object"]["dose_count"].is_number());
    assert!(audit["object"]["error_count"].is_number());
    assert!(audit["object"]["result_status"].is_string());
}

const CALPOL_SEED_URL: &str =
    "https://www.calpol.co.uk/our-products/calpol-sixplus-oral-suspension-paracetamol";
const CALPOL_EVIDENCE: &str = "Each 5 ml contains 250 mg paracetamol. Give every 4 to 6 hours as needed. Do not give more than 4 doses in 24 hours.";

fn ai_mock_server() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buffer = [0_u8; 8192];
            let Ok(read) = std::io::Read::read(&mut stream, &mut buffer) else {
                continue;
            };
            let request = String::from_utf8_lossy(&buffer[..read]);
            let (content_type, body) = if request.starts_with("GET /fetch") {
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
    port
}

#[tokio::test]
async fn ai_medication_suggestions_validates_doses_against_fetched_evidence() {
    let port = ai_mock_server();
    let app = Application::new_with_ai_generation(port).await;
    upgrade_plan(&app).await;
    let token = app.token().await;
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
