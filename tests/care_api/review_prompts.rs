use super::*;

async fn seed_prompt(app: &Application, id: i64, status: &str) {
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "INSERT INTO medication_review_evidence_records(id,source_name,source_record_id,source_url,product_name,label_section,evidence_text,retrieved_on,risk_level,match_confidence,created_at,updated_at) VALUES ({id}00,'Synthetic reference','report-{id}','https://example.test/evidence/{id}','Synthetic tablets','Warnings','Synthetic evidence.','2026-01-01','moderate','high',now(),now())"
        ))
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "INSERT INTO medication_review_prompts(id,household_id,person_id,primary_medication_id,interacting_medication_id,evidence_record_id,primary_medication_name,interacting_medication_name,evidence_source_name,evidence_source_url,evidence_source_version,evidence_source_checked_on,evidence_source_effective_on,evidence_text,match_confidence,match_reason,match_type,matched_term,risk_level,source_instruction,status,created_at,updated_at) VALUES ({id},72001,73001,80001,80001,{id}00,'Synthetic tablets','Synthetic tablets','Synthetic reference','https://example.test/evidence/{id}','1','2026-01-01','2026-01-01','Synthetic evidence.','high','Synthetic reason','ingredient','synthetic','moderate','unclassified','{status}',now(),now())"
        ))
        .await
        .unwrap();
}

fn prompts(app: &Application, suffix: &str) -> String {
    format!(
        "{}/api/v1/households/72001/medication_review_prompts{suffix}",
        app.origin
    )
}

#[tokio::test]
async fn review_prompt_index_matches_documented_contract() {
    let app = Application::new().await;
    let token = app.token().await;
    seed_prompt(&app, 91001, "needs_review").await;
    seed_prompt(&app, 91002, "hidden_low_signal").await;
    let response = app
        .client
        .get(prompts(&app, ""))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let status = response.status().as_u16();
    let no_store = response
        .headers()
        .get("cache-control")
        .is_some_and(|value| value == "no-store");
    let body: Value = response.json().await.unwrap();
    let hidden = app
        .client
        .get(prompts(&app, "?show_hidden=1"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    let reviewed = app
        .client
        .get(prompts(&app, "?review_status=reviewed"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    let invalid = app
        .client
        .get(prompts(&app, "?review_status=bogus"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let invalid_status = invalid.status().as_u16();
    app.close().await;
    assert_eq!(status, 200);
    assert!(no_store);
    let rows = body["data"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], "91001");
    assert_eq!(rows[0]["person_id"], "73001");
    assert_eq!(rows[0]["status"], "needs_review");
    assert!(rows[0]["etag"].is_string());
    assert_eq!(body["meta"]["page"], 1);
    assert_eq!(body["meta"]["per_page"], 20);
    assert_eq!(body["meta"]["total_count"], 1);
    assert_eq!(hidden["data"].as_array().unwrap().len(), 2);
    assert_eq!(hidden["meta"]["total_count"], 2);
    assert_eq!(reviewed["data"].as_array().unwrap().len(), 0);
    assert_eq!(invalid_status, 422);
}

#[tokio::test]
async fn review_prompt_show_and_update_preserve_version_and_audit() {
    let app = Application::new().await;
    let token = app.token().await;
    seed_prompt(&app, 91001, "needs_review").await;
    let shown = app
        .client
        .get(prompts(&app, "/91001"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let shown_status = shown.status().as_u16();
    let etag = shown
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let shown_body: Value = shown.json().await.unwrap();
    assert_eq!(shown_status, 200);
    assert_eq!(shown_body["data"]["id"], "91001");
    let missing = app
        .client
        .patch(prompts(&app, "/91001"))
        .bearer_auth(&token)
        .json(&json!({"medication_review_prompt":{"review_note":"early note"}}))
        .send()
        .await
        .unwrap();
    let missing_status = missing.status().as_u16();
    let stale = app
        .client
        .patch(prompts(&app, "/91001"))
        .bearer_auth(&token)
        .header("if-match", "\"stale\"")
        .json(&json!({"medication_review_prompt":{"review_note":"stale note"}}))
        .send()
        .await
        .unwrap();
    let stale_status = stale.status().as_u16();
    let incomplete = app
        .client
        .patch(prompts(&app, "/91001"))
        .bearer_auth(&token)
        .header("if-match", &etag)
        .json(&json!({"medication_review_prompt":{"status":"reviewed_with_practitioner"}}))
        .send()
        .await
        .unwrap();
    let incomplete_status = incomplete.status().as_u16();
    let updated = app
        .client
        .put(prompts(&app, "/91001"))
        .bearer_auth(&token)
        .header("if-match", &etag)
        .json(&json!({"medication_review_prompt":{"status":"reviewed_with_practitioner","practitioner_name":"Dr Test","practitioner_role":"GP","reviewed_on":"2026-10-01","review_note":"Synthetic note"}}))
        .send()
        .await
        .unwrap();
    let updated_status = updated.status().as_u16();
    let next_etag = updated
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let updated_body: Value = updated.json().await.unwrap();
    let event = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT metadata FROM security_audit_events WHERE event_type='medication_review_prompt.updated' ORDER BY id DESC LIMIT 1"))
        .await
        .unwrap()
        .unwrap();
    let metadata: Value = event.try_get("", "metadata").unwrap();
    let malformed = app
        .client
        .patch(prompts(&app, "/91001"))
        .bearer_auth(&token)
        .header("if-match", &next_etag)
        .header("content-type", "application/json")
        .body("not json")
        .send()
        .await
        .unwrap();
    let malformed_status = malformed.status().as_u16();
    let malformed_body: Value = malformed.json().await.unwrap();
    let bad_id = app
        .client
        .get(prompts(&app, "/abc"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let bad_id_status = bad_id.status().as_u16();
    app.close().await;
    assert_eq!(missing_status, 428);
    assert_eq!(stale_status, 409);
    assert_eq!(incomplete_status, 422);
    assert_eq!(updated_status, 200);
    assert_ne!(etag, next_etag);
    assert_eq!(updated_body["data"]["status"], "reviewed_with_practitioner");
    assert_eq!(updated_body["data"]["reviewed_by_membership_id"], "74001");
    assert_eq!(updated_body["data"]["practitioner_name"], "Dr Test");
    assert_eq!(metadata["prompt_id"], 91001);
    assert_eq!(metadata["previous_status"], "needs_review");
    assert_eq!(metadata["status"], "reviewed_with_practitioner");
    assert_eq!(malformed_status, 400);
    assert_eq!(malformed_body["error"]["code"], "bad_request");
    assert_eq!(bad_id_status, 404);
}

#[tokio::test]
async fn review_prompt_update_replays_under_an_idempotency_key() {
    let app = Application::new().await;
    let token = app.token().await;
    seed_prompt(&app, 91001, "needs_review").await;
    let shown = app
        .client
        .get(prompts(&app, "/91001"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let etag = shown
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let first = app
        .client
        .patch(prompts(&app, "/91001"))
        .bearer_auth(&token)
        .header("if-match", &etag)
        .header("idempotency-key", "synthetic-review-update")
        .json(&json!({"medication_review_prompt":{"review_note":"Keyed note"}}))
        .send()
        .await
        .unwrap();
    let first_status = first.status().as_u16();
    let replay = app
        .client
        .patch(prompts(&app, "/91001"))
        .bearer_auth(&token)
        .header("if-match", &etag)
        .header("idempotency-key", "synthetic-review-update")
        .json(&json!({"medication_review_prompt":{"review_note":"Keyed note"}}))
        .send()
        .await
        .unwrap();
    let replay_status = replay.status().as_u16();
    let replay_marker = replay.headers().get("idempotency-replayed").is_some();
    let events = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) AS total FROM security_audit_events WHERE event_type='medication_review_prompt.updated'"))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "total")
        .unwrap();
    app.close().await;
    assert_eq!(first_status, 200);
    assert_eq!(replay_status, 200);
    assert!(replay_marker);
    assert_eq!(events, 1);
}

#[tokio::test]
async fn review_prompt_index_paginates_beyond_the_first_page() {
    let app = Application::new().await;
    let token = app.token().await;
    for id in 91010..91032 {
        seed_prompt(&app, id, "needs_review").await;
    }
    let first = app
        .client
        .get(prompts(&app, ""))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    let second = app
        .client
        .get(prompts(&app, "?page=2"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    app.close().await;
    assert_eq!(first["data"].as_array().unwrap().len(), 20);
    assert_eq!(first["meta"]["total_count"], 22);
    let rows = second["data"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(second["meta"]["page"], 2);
    assert_eq!(second["meta"]["total_count"], 22);
}

#[tokio::test]
async fn review_prompts_require_adult_actor_and_manage_access() {
    let app = Application::new().await;
    let token = app.token().await;
    seed_prompt(&app, 91001, "needs_review").await;
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET access_level='view' WHERE id=78001")
        .await
        .unwrap();
    let listed = app
        .client
        .get(prompts(&app, ""))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let listed_status = listed.status().as_u16();
    let listed_body: Value = listed.json().await.unwrap();
    let shown = app
        .client
        .get(prompts(&app, "/91001"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let shown_status = shown.status().as_u16();
    let denied = app
        .client
        .patch(prompts(&app, "/91001"))
        .bearer_auth(&token)
        .header("if-match", "\"anything\"")
        .json(&json!({"medication_review_prompt":{"review_note":"denied"}}))
        .send()
        .await
        .unwrap();
    let denied_status = denied.status().as_u16();
    app.fixture
        .admin
        .execute_unprepared("UPDATE people SET person_type=1,has_capacity=false WHERE id=73001")
        .await
        .unwrap();
    let minor = app
        .client
        .get(prompts(&app, ""))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let minor_status = minor.status().as_u16();
    app.close().await;
    assert_eq!(listed_status, 200);
    assert_eq!(listed_body["data"].as_array().unwrap().len(), 1);
    assert_eq!(shown_status, 200);
    assert_eq!(denied_status, 403);
    assert_eq!(minor_status, 403);
}
