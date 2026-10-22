use super::*;
use chrono::NaiveDate;
use loco_rs::{
    bgworker::BackgroundWorker,
    task::{Task, Vars},
};
use med_tracker::models::care::review_prompts::refresh::{
    RefreshRun, RefreshTask, RefreshWorker, import_snapshot,
};

#[test]
fn review_lease_delete_contract_allows_idempotent_omitted_token() {
    let spec: Value =
        serde_yaml_ng::from_str(include_str!("../../docs/api/openapi.v1.yaml")).unwrap();
    let parameters =
        spec["paths"]["/households/{household_id}/medication_review_prompts/{id}/lease"]["delete"]
            ["parameters"]
            .as_array()
            .unwrap();
    let token = parameters
        .iter()
        .find(|parameter| parameter["name"] == "X-Review-Lease")
        .unwrap();
    assert_eq!(token["required"], false);
    assert_eq!(token["schema"]["format"], "uuid");
}

async fn review_refresh_run(app: &Application) -> i64 {
    app.fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "INSERT INTO medication_review_evidence_refresh_runs(status,created_at,updated_at) VALUES (0,now(),now()) RETURNING id",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "id")
        .unwrap()
}

fn review_refresh_snapshot(version: &str, text: &str) -> Value {
    json!({"openfda_last_updated":"2026-10-01","labels":[
        {"set_id":"synthetic-worker-label","version":version,"effective_time":"20261001",
         "selection_term":"ibuprofen","drug_interactions":[text],
         "openfda":{"brand_name":["Synthetic tablets"],"substance_name":["ibuprofen"]}}
    ]})
}

async fn seed_prompt(app: &Application, id: i64, status: &str) {
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "INSERT INTO medication_review_evidence_records(id,source_name,source_record_id,source_url,product_name,label_section,evidence_text,retrieved_on,risk_level,match_confidence,match_status,candidate_terms,interacting_terms,created_at,updated_at) VALUES ({id}00,'Synthetic reference','report-{id}','https://example.test/evidence/{id}','Synthetic tablets','Warnings','Synthetic evidence.','2026-01-01','moderate','high','reviewed_pair',ARRAY['synthetic tablets']::varchar[],ARRAY['synthetic tablets']::varchar[],now(),now())"
        ))
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "INSERT INTO medication_review_prompts(id,household_id,person_id,primary_medication_id,interacting_medication_id,evidence_record_id,primary_medication_name,interacting_medication_name,evidence_source_name,evidence_source_url,evidence_source_version,evidence_source_checked_on,evidence_source_effective_on,evidence_text,match_confidence,match_reason,match_type,matched_term,risk_level,source_instruction,status,created_at,updated_at) VALUES ({id},72001,73001,80001,80001,{id}00,'Synthetic tablets','Synthetic tablets','Synthetic reference','https://example.test/evidence/{id}','1','2026-01-01','2026-01-01','Synthetic evidence.','high','A reviewed rule identifies synthetic tablets as the interacting medicine.','curated','synthetic tablets','moderate','unclassified','{status}',now(),now())"
        ))
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared(&format!(
        "UPDATE medication_review_prompts p SET evidence_source_fingerprint=md5(jsonb_build_array(e.source_name,e.source_url,e.source_version,e.source_effective_on,e.evidence_text,e.risk_level,e.match_confidence)::text) FROM medication_review_evidence_records e WHERE p.id={id} AND e.id=p.evidence_record_id"
    )).await.unwrap();
}

async fn seed_matched_prompt(app: &Application, id: i64, status: &str) {
    seed_prompt(app, id, status).await;
    app.fixture.admin.execute_unprepared(&format!(
        "UPDATE medications SET name='Aspirin tablets' WHERE id=80001; INSERT INTO medications(id,household_id,location_id,name,current_supply,dose_amount,dose_unit,created_at,updated_at) VALUES (80003,72001,79001,'Warfarin tablets',10,1,'tablet',now(),now()); INSERT INTO person_medications(id,household_id,person_id,medication_id,dose_amount,dose_unit,position,created_at,updated_at) VALUES (81003,72001,73001,80003,1,'tablet',1,now(),now()); UPDATE medication_review_evidence_records SET source_version='1',evidence_text='Avoid warfarin with aspirin.',match_status='unreviewed',candidate_terms=ARRAY['aspirin']::varchar[],interacting_terms=ARRAY[]::varchar[],risk_level='unknown',match_confidence='unknown' WHERE id={id}00; INSERT INTO medication_review_evidence_records(id,source_name,source_record_id,source_url,product_name,label_section,evidence_text,retrieved_on,risk_level,match_confidence,candidate_terms,pharmacologic_classes,created_at,updated_at) VALUES ({id}01,'Synthetic reference','identity-{id}','https://example.test/evidence/identity-{id}','Warfarin','Warnings','Warfarin medicine information.','2026-01-01','unknown','unknown',ARRAY['warfarin']::varchar[],ARRAY['anticoagulant']::varchar[],now(),now()); UPDATE medication_review_prompts SET primary_medication_id=80001,interacting_medication_id=80003,primary_medication_name='Aspirin tablets',interacting_medication_name='Warfarin tablets',evidence_source_version='1',evidence_text='Avoid warfarin with aspirin.',matched_term='warfarin',match_type='ingredient',source_instruction='avoid',match_reason='The label explicitly names the ingredient warfarin.',risk_level='high',match_confidence='high' WHERE id={id}; UPDATE medication_review_prompts p SET evidence_source_fingerprint=md5(jsonb_build_array(e.source_name,e.source_url,e.source_version,e.source_effective_on,e.evidence_text,e.risk_level,e.match_confidence)::text) FROM medication_review_evidence_records e WHERE p.id={id} AND e.id=p.evidence_record_id"
    )).await.unwrap();
}

fn prompts(app: &Application, suffix: &str) -> String {
    format!(
        "{}/api/v1/households/72001/medication_review_prompts{suffix}",
        app.origin
    )
}

async fn edit_token(app: &Application, bearer: &str, id: i64) -> String {
    let response = app
        .client
        .post(prompts(app, &format!("/{id}/lease")))
        .bearer_auth(bearer)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), 201);
    let body: Value = response.json().await.unwrap();
    body["data"]["token"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn review_edit_requires_a_persisted_exclusive_lease_and_releases_after_save() {
    let app = Application::new().await;
    let token = app.token().await;
    seed_prompt(&app, 91101, "needs_review").await;
    let shown = app
        .client
        .get(prompts(&app, "/91101"))
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
    let without_lease = app
        .client
        .patch(prompts(&app, "/91101"))
        .bearer_auth(&token)
        .header("if-match", &etag)
        .json(&json!({"medication_review_prompt":{"review_note":"Unleased"}}))
        .send()
        .await
        .unwrap();
    let acquired = app
        .client
        .post(prompts(&app, "/91101/lease"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let acquire_status = acquired.status().as_u16();
    let acquired_body: Value = acquired.json().await.unwrap();
    let lease = acquired_body["data"]["token"].as_str().unwrap_or("");
    let competing = app
        .client
        .post(prompts(&app, "/91101/lease"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let saved = app
        .client
        .patch(prompts(&app, "/91101"))
        .bearer_auth(&token)
        .header("if-match", &etag)
        .header("x-review-lease", lease)
        .json(&json!({"medication_review_prompt":{"review_note":"Leased note"}}))
        .send()
        .await
        .unwrap();
    let reacquired = app
        .client
        .post(prompts(&app, "/91101/lease"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let reacquired_status = reacquired.status().as_u16();
    let reacquired_body: Value = reacquired.json().await.unwrap();
    let cancel = app
        .client
        .delete(prompts(&app, "/91101/lease"))
        .bearer_auth(&token)
        .header(
            "x-review-lease",
            reacquired_body["data"]["token"].as_str().unwrap(),
        )
        .send()
        .await
        .unwrap();
    let repeated_cancel = app
        .client
        .delete(prompts(&app, "/91101/lease"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let remaining = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT token FROM medication_review_edit_leases WHERE prompt_id=91101",
        ))
        .await
        .unwrap();
    app.close().await;
    assert_eq!(without_lease.status().as_u16(), 409);
    assert_eq!(acquire_status, 201);
    assert!(!lease.is_empty());
    assert_eq!(competing.status().as_u16(), 409);
    assert_eq!(saved.status().as_u16(), 200);
    assert_eq!(reacquired_status, 201);
    assert_eq!(cancel.status().as_u16(), 200);
    assert_eq!(repeated_cancel.status().as_u16(), 200);
    assert!(remaining.is_none());
}

#[tokio::test]
async fn review_save_with_unchanged_fields_releases_lease_without_history() {
    let app = Application::new().await;
    let token = app.token().await;
    seed_prompt(&app, 91252, "not_relevant").await;
    let shown = app
        .client
        .get(prompts(&app, "/91252"))
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
    let lease = edit_token(&app, &token, 91252).await;
    let saved = app
        .client
        .patch(prompts(&app, "/91252"))
        .bearer_auth(&token)
        .header("if-match", &etag)
        .header("x-review-lease", &lease)
        .json(&json!({"medication_review_prompt":{"status":"not_relevant"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(saved.status().as_u16(), 200);
    assert_eq!(saved.headers().get("etag").unwrap().to_str().unwrap(), etag);
    let count = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*)::bigint AS total FROM medication_review_histories WHERE prompt_id=91252"
    )).await.unwrap().unwrap().try_get::<i64>("", "total").unwrap();
    assert_eq!(count, 0);
    let remaining = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT token FROM medication_review_edit_leases WHERE prompt_id=91252",
        ))
        .await
        .unwrap();
    assert!(remaining.is_none());
    app.close().await;
}

#[tokio::test]
async fn changed_evidence_reopens_review_and_preserves_the_previous_snapshot() {
    let app = Application::new().await;
    let token = app.token().await;
    seed_matched_prompt(&app, 91102, "reviewed_with_practitioner").await;
    app.fixture.admin.execute_unprepared(
        "UPDATE medication_review_prompts SET practitioner_name='Dr Test',practitioner_role='GP',reviewed_on='2026-10-01',review_note='Discussed',reviewed_by_membership_id=74001 WHERE id=91102",
    ).await.unwrap();
    app.fixture.admin.execute_unprepared(
        "UPDATE medication_review_evidence_records SET retrieved_on='2026-10-02' WHERE id=9110200",
    ).await.unwrap();
    let before: Value = app
        .client
        .get(prompts(&app, "/91102"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let original_etag = before["data"]["etag"].as_str().unwrap().to_owned();
    let unchanged = app
        .client
        .get(prompts(&app, ""))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(unchanged.status().as_u16(), 200);
    let after_timestamp: Value = app
        .client
        .get(prompts(&app, "/91102"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        after_timestamp["data"]["status"],
        "reviewed_with_practitioner"
    );
    assert_eq!(after_timestamp["data"]["etag"], original_etag);
    app.fixture.admin.execute_unprepared(
        "UPDATE medication_review_evidence_records SET source_version='2',evidence_text='Monitor anticoagulants with aspirin.' WHERE id=9110200",
    ).await.unwrap();
    let detail_after_change: Value = app
        .client
        .get(prompts(&app, "/91102"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(detail_after_change["data"]["status"], "needs_review");
    assert_ne!(detail_after_change["data"]["etag"], original_etag);
    let refreshed = app
        .client
        .get(prompts(&app, ""))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(refreshed.status().as_u16(), 200);
    let after: Value = app
        .client
        .get(prompts(&app, "/91102"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    app.close().await;
    assert_eq!(after["data"]["status"], "needs_review");
    assert_eq!(
        after["data"]["evidence_text"],
        "Monitor anticoagulants with aspirin."
    );
    assert_eq!(after["data"]["matched_term"], "anticoagulants");
    assert_eq!(after["data"]["match_type"], "class");
    assert_eq!(after["data"]["source_instruction"], "monitor_or_adjust");
    assert_eq!(
        after["data"]["match_reason"],
        "The label explicitly names the class anticoagulants."
    );
    assert_eq!(after["data"]["risk_level"], "moderate");
    assert_eq!(after["data"]["match_confidence"], "moderate");
    assert_ne!(after["data"]["etag"], original_etag);
    assert_eq!(
        after["history"][0]["snapshot"]["practitioner_name"],
        "Dr Test"
    );
    assert_eq!(after["history"][0]["snapshot"]["review_note"], "Discussed");
    assert_eq!(
        after["history"][0]["snapshot"]["evidence_text"],
        "Avoid warfarin with aspirin."
    );
    assert_eq!(after["history"][0]["snapshot"]["matched_term"], "warfarin");
    assert_eq!(after["history"][0]["snapshot"]["match_type"], "ingredient");
    assert_eq!(
        after["history"][0]["snapshot"]["source_instruction"],
        "avoid"
    );
    let snapshot = after["history"][0]["snapshot"].as_object().unwrap();
    assert_eq!(snapshot["id"], "91102");
    assert_eq!(snapshot["person_id"], "73001");
    assert_eq!(snapshot["etag"], original_etag);
    assert!(!snapshot.contains_key("evidence_source_fingerprint"));
    assert!(!snapshot.contains_key("household_id"));
}

#[tokio::test]
async fn changed_evidence_without_a_pair_match_retires_the_warning_and_keeps_history() {
    let app = Application::new().await;
    let token = app.token().await;
    seed_matched_prompt(&app, 91108, "reviewed_with_practitioner").await;
    app.fixture.admin.execute_unprepared(
        "UPDATE medication_review_prompts SET practitioner_name='Dr Test',practitioner_role='GP',reviewed_on='2026-10-01',review_note='Discussed',reviewed_by_membership_id=74001 WHERE id=91108",
    ).await.unwrap();
    app.fixture.admin.execute_unprepared(
        "UPDATE medication_review_evidence_records SET source_version='2',evidence_text='This product has no named interacting medicines.' WHERE id=9110800",
    ).await.unwrap();
    let first: Value = app
        .client
        .get(prompts(&app, "/91108"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let second: Value = app
        .client
        .get(prompts(&app, "/91108"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let default_list: Value = app
        .client
        .get(prompts(&app, ""))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    app.close().await;
    assert_eq!(first["data"]["status"], "hidden_low_signal");
    assert_ne!(first["data"]["source_instruction"], "avoid");
    assert_ne!(first["data"]["matched_term"], "warfarin");
    assert_ne!(first["data"]["match_type"], "ingredient");
    assert_ne!(
        first["data"]["match_reason"],
        "The label explicitly names the ingredient warfarin."
    );
    assert_eq!(
        first["history"][0]["snapshot"]["source_instruction"],
        "avoid"
    );
    assert_eq!(first["history"][0]["snapshot"]["review_note"], "Discussed");
    assert_eq!(second["history"].as_array().unwrap().len(), 1);
    assert!(default_list["data"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn changed_evidence_keeps_an_equivalent_legacy_review_untouched() {
    let app = Application::new().await;
    let token = app.token().await;
    seed_prompt(&app, 91109, "reviewed_with_practitioner").await;
    app.fixture.admin.execute_unprepared(
        "UPDATE medication_review_prompts SET practitioner_name='Dr Test',practitioner_role='GP',reviewed_on='2026-10-01',review_note='Discussed',reviewed_by_membership_id=74001 WHERE id=91109",
    ).await.unwrap();
    let first: Value = app
        .client
        .get(prompts(&app, "/91109"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared(
        "UPDATE medication_review_evidence_records SET retrieved_on='2026-10-02' WHERE id=9110900",
    ).await.unwrap();
    let second: Value = app
        .client
        .get(prompts(&app, "/91109"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    app.close().await;
    assert_eq!(first["data"]["status"], "reviewed_with_practitioner");
    assert_eq!(second["data"]["status"], "reviewed_with_practitioner");
    assert_eq!(second["data"]["etag"], first["data"]["etag"]);
    assert_eq!(second["data"]["practitioner_name"], "Dr Test");
    assert!(second["history"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn changed_evidence_match_inputs_alone_retire_and_restore_a_warning() {
    let app = Application::new().await;
    let token = app.token().await;
    seed_matched_prompt(&app, 91110, "reviewed_with_practitioner").await;
    app.fixture.admin.execute_unprepared(
        "UPDATE medication_review_prompts SET practitioner_name='Dr Test',practitioner_role='GP',reviewed_on='2026-10-01',review_note='Discussed',reviewed_by_membership_id=74001 WHERE id=91110; UPDATE medication_review_evidence_records SET candidate_terms=ARRAY['unrelated']::varchar[] WHERE id=9111000",
    ).await.unwrap();
    let retired: Value = app
        .client
        .get(prompts(&app, "/91110"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared(
        "UPDATE medication_review_evidence_records SET candidate_terms=ARRAY['aspirin']::varchar[] WHERE id=9111000",
    ).await.unwrap();
    let restored: Value = app
        .client
        .get(prompts(&app, "/91110"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    app.close().await;
    assert_eq!(retired["data"]["status"], "hidden_low_signal");
    assert_eq!(
        retired["history"][0]["snapshot"]["review_note"],
        "Discussed"
    );
    assert_eq!(restored["data"]["status"], "needs_review");
    assert_eq!(restored["data"]["matched_term"], "warfarin");
    assert_eq!(restored["data"]["source_instruction"], "avoid");
    assert_eq!(restored["history"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn changed_evidence_supporting_identity_alone_retires_a_warning() {
    let app = Application::new().await;
    let token = app.token().await;
    seed_matched_prompt(&app, 91111, "reviewed_with_practitioner").await;
    app.fixture.admin.execute_unprepared(
        "UPDATE medication_review_evidence_records SET evidence_text='Monitor glimmerase with aspirin.' WHERE id=9111100; UPDATE medication_review_evidence_records SET candidate_terms=ARRAY['warfarin','glimmerase']::varchar[] WHERE id=9111101; UPDATE medication_review_prompts SET evidence_text='Monitor glimmerase with aspirin.',matched_term='glimmerase',match_type='ingredient',source_instruction='monitor_or_adjust',match_reason='The label explicitly names the ingredient glimmerase.',risk_level='moderate',match_confidence='high',practitioner_name='Dr Test',practitioner_role='GP',reviewed_on='2026-10-01',review_note='Discussed',reviewed_by_membership_id=74001 WHERE id=91111; UPDATE medication_review_prompts p SET evidence_source_fingerprint=md5(jsonb_build_array(e.source_name,e.source_url,e.source_version,e.source_effective_on,e.evidence_text,e.risk_level,e.match_confidence)::text) FROM medication_review_evidence_records e WHERE p.id=91111 AND e.id=p.evidence_record_id",
    ).await.unwrap();
    let before: Value = app
        .client
        .get(prompts(&app, "/91111"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared(
        "UPDATE medication_review_evidence_records SET candidate_terms=ARRAY['warfarin']::varchar[] WHERE id=9111101",
    ).await.unwrap();
    let after: Value = app
        .client
        .get(prompts(&app, "/91111"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    app.close().await;
    assert_eq!(before["data"]["status"], "reviewed_with_practitioner");
    assert_eq!(after["data"]["status"], "hidden_low_signal");
    assert_eq!(
        after["history"][0]["snapshot"]["matched_term"],
        "glimmerase"
    );
    assert_eq!(after["history"][0]["snapshot"]["review_note"], "Discussed");
}

#[tokio::test]
async fn changed_evidence_reversed_curated_pair_retires_the_old_direction() {
    let app = Application::new().await;
    let token = app.token().await;
    seed_matched_prompt(&app, 91112, "reviewed_with_practitioner").await;
    app.fixture.admin.execute_unprepared(
        "UPDATE medication_review_evidence_records SET match_status='reviewed_pair',candidate_terms=ARRAY['aspirin']::varchar[],interacting_terms=ARRAY['warfarin']::varchar[],risk_level='high',match_confidence='high' WHERE id=9111200; UPDATE medication_review_prompts SET match_type='curated',matched_term='warfarin',source_instruction='unclassified',match_reason='A reviewed rule identifies warfarin as the interacting medicine.',practitioner_name='Dr Test',practitioner_role='GP',reviewed_on='2026-10-01',review_note='Discussed',reviewed_by_membership_id=74001 WHERE id=91112; UPDATE medication_review_prompts p SET evidence_source_fingerprint=md5(jsonb_build_array(e.source_name,e.source_url,e.source_version,e.source_effective_on,e.evidence_text,e.risk_level,e.match_confidence)::text) FROM medication_review_evidence_records e WHERE p.id=91112 AND e.id=p.evidence_record_id",
    ).await.unwrap();
    let before: Value = app
        .client
        .get(prompts(&app, "/91112"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared(
        "UPDATE medication_review_evidence_records SET candidate_terms=ARRAY['warfarin']::varchar[],interacting_terms=ARRAY['aspirin']::varchar[] WHERE id=9111200",
    ).await.unwrap();
    let list: Value = app
        .client
        .get(prompts(&app, ""))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let old: Value = app
        .client
        .get(prompts(&app, "/91112"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    app.close().await;
    assert_eq!(before["data"]["status"], "reviewed_with_practitioner");
    assert_eq!(old["data"]["status"], "hidden_low_signal");
    assert_eq!(old["history"][0]["snapshot"]["review_note"], "Discussed");
    let visible = list["data"].as_array().unwrap();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0]["primary_medication_id"], "80003");
    assert_eq!(visible[0]["interacting_medication_id"], "80001");
    assert_eq!(visible[0]["matched_term"], "aspirin");
    assert_eq!(visible[0]["status"], "needs_review");
}

#[tokio::test]
async fn review_date_can_be_cleared_for_a_non_practitioner_outcome() {
    let app = Application::new().await;
    let token = app.token().await;
    for (id, date) in [(91106, Value::Null), (91107, json!(""))] {
        seed_prompt(&app, id, "reviewed_with_practitioner").await;
        app.fixture.admin.execute_unprepared(&format!(
            "UPDATE medication_review_prompts SET practitioner_name='Dr Test',practitioner_role='GP',reviewed_on='2026-10-01',reviewed_by_membership_id=74001 WHERE id={id}"
        )).await.unwrap();
        let shown = app
            .client
            .get(prompts(&app, &format!("/{id}")))
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
        let lease = edit_token(&app, &token, id).await;
        let incomplete = app.client.patch(prompts(&app, &format!("/{id}")))
            .bearer_auth(&token).header("if-match", &etag).header("x-review-lease", &lease)
            .json(&json!({"medication_review_prompt":{"status":"reviewed_with_practitioner","reviewed_on":date}}))
            .send().await.unwrap();
        assert_eq!(incomplete.status().as_u16(), 422);
        let response = app
            .client
            .patch(prompts(&app, &format!("/{id}")))
            .bearer_auth(&token)
            .header("if-match", &etag)
            .header("x-review-lease", &lease)
            .json(&json!({"medication_review_prompt":{"status":"not_relevant","reviewed_on":date}}))
            .send()
            .await
            .unwrap();
        let status = response.status().as_u16();
        let body: Value = response.json().await.unwrap();
        assert_eq!(status, 200, "{body}");
        assert!(body["data"]["reviewed_on"].is_null());
        let detail: Value = app
            .client
            .get(prompts(&app, &format!("/{id}")))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(
            detail["history"][0]["snapshot"]["reviewed_on"],
            "2026-10-01"
        );
    }
    app.close().await;
}

#[tokio::test]
async fn changed_evidence_blocks_an_edit_started_before_refresh() {
    let app = Application::new().await;
    let token = app.token().await;
    seed_prompt(&app, 91105, "needs_review").await;
    let shown: Value = app
        .client
        .get(prompts(&app, "/91105"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let etag = shown["data"]["etag"].as_str().unwrap();
    let lease = edit_token(&app, &token, 91105).await;
    app.fixture.admin.execute_unprepared(
        "UPDATE medication_review_evidence_records SET source_version='2',evidence_text='New evidence before save.' WHERE id=9110500",
    ).await.unwrap();
    let stale = app
        .client
        .patch(prompts(&app, "/91105"))
        .bearer_auth(&token)
        .header("if-match", etag)
        .header("x-review-lease", &lease)
        .json(&json!({"medication_review_prompt":{"status":"not_relevant"}}))
        .send()
        .await
        .unwrap();
    let current: Value = app
        .client
        .get(prompts(&app, "/91105"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let lease_row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT token::text AS token FROM medication_review_edit_leases WHERE prompt_id=91105",
        ))
        .await
        .unwrap()
        .unwrap();
    let retained: String = lease_row.try_get("", "token").unwrap();
    app.close().await;
    assert_eq!(stale.status().as_u16(), 409);
    assert_eq!(current["data"]["status"], "needs_review");
    assert_eq!(
        current["data"]["evidence_text"],
        "New evidence before save."
    );
    assert_eq!(retained, lease);
}

#[tokio::test]
async fn expired_review_lease_or_revoked_membership_cannot_be_used_by_api_or_sync() {
    let app = Application::new().await;
    let token = app.token().await;
    seed_prompt(&app, 91103, "needs_review").await;
    let shown: Value = app
        .client
        .get(prompts(&app, "/91103"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let etag = shown["data"]["etag"].as_str().unwrap();
    let lease = edit_token(&app, &token, 91103).await;
    app.fixture.admin.execute_unprepared(
        "UPDATE medication_review_edit_leases SET expires_at=timezone('UTC',clock_timestamp())-interval '1 second' WHERE prompt_id=91103",
    ).await.unwrap();
    let expired = app
        .client
        .patch(prompts(&app, "/91103"))
        .bearer_auth(&token)
        .header("if-match", etag)
        .header("x-review-lease", &lease)
        .json(&json!({"medication_review_prompt":{"review_note":"Expired"}}))
        .send()
        .await
        .unwrap();
    let new_lease = edit_token(&app, &token, 91103).await;
    let sync = app.client.post(format!("{}/api/v1/households/72001/sync/batches", app.origin))
        .bearer_auth(&token)
        .json(&json!({"batch":{"operations":[{"resource_type":"medication_review_prompt","action":"update","id":"91103","if_match":etag,"attributes":{"status":"not_relevant","review_note":"No lease in sync"}}]}}))
        .send().await.unwrap();
    app.fixture
        .admin
        .execute_unprepared("UPDATE person_access_grants SET access_level='view' WHERE id=78001; UPDATE household_memberships SET revoked_at=now() WHERE id=74001")
        .await
        .unwrap();
    let revoked = app
        .client
        .patch(prompts(&app, "/91103"))
        .bearer_auth(&token)
        .header("if-match", etag)
        .header("x-review-lease", &new_lease)
        .json(&json!({"medication_review_prompt":{"review_note":"Revoked"}}))
        .send()
        .await
        .unwrap();
    let stored = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status,review_note FROM medication_review_prompts WHERE id=91103",
        ))
        .await
        .unwrap()
        .unwrap();
    let status: String = stored.try_get("", "status").unwrap();
    let note: Option<String> = stored.try_get("", "review_note").unwrap();
    app.close().await;
    assert_eq!(expired.status().as_u16(), 409);
    assert_eq!(sync.status().as_u16(), 409);
    assert_eq!(revoked.status().as_u16(), 403);
    assert_eq!(status, "needs_review");
    assert_eq!(note, None);
}

#[tokio::test]
async fn failed_review_audit_rolls_back_mutation_and_keeps_the_lease() {
    let app = Application::new().await;
    let token = app.token().await;
    seed_prompt(&app, 91104, "needs_review").await;
    let shown: Value = app
        .client
        .get(prompts(&app, "/91104"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let etag = shown["data"]["etag"].as_str().unwrap();
    let lease = edit_token(&app, &token, 91104).await;
    app.fixture.admin.execute_unprepared(
        "CREATE FUNCTION synthetic_fail_review_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='medication_review_prompt.updated' THEN RAISE EXCEPTION 'synthetic audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER synthetic_review_audit_failure BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION synthetic_fail_review_audit()",
    ).await.unwrap();
    let failed = app
        .client
        .patch(prompts(&app, "/91104"))
        .bearer_auth(&token)
        .header("if-match", etag)
        .header("x-review-lease", &lease)
        .json(&json!({"medication_review_prompt":{"review_note":"Should roll back"}}))
        .send()
        .await
        .unwrap();
    let stored = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT p.review_note,l.token::text AS token FROM medication_review_prompts p JOIN medication_review_edit_leases l ON l.prompt_id=p.id WHERE p.id=91104"))
        .await.unwrap().unwrap();
    let note: Option<String> = stored.try_get("", "review_note").unwrap();
    let stored_token: String = stored.try_get("", "token").unwrap();
    app.close().await;
    assert_eq!(failed.status().as_u16(), 500);
    assert_eq!(note, None);
    assert_eq!(stored_token, lease);
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
    let lease = edit_token(&app, &token, 91001).await;
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
        .header("x-review-lease", &lease)
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
        .header("x-review-lease", &lease)
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
    let lease = edit_token(&app, &token, 91001).await;
    let first = app
        .client
        .patch(prompts(&app, "/91001"))
        .bearer_auth(&token)
        .header("if-match", &etag)
        .header("x-review-lease", &lease)
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
        .header("x-review-lease", &lease)
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
async fn review_prompts_require_adult_actor_and_other_person_manage_access() {
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
        .execute_unprepared("UPDATE medication_review_prompts SET person_id=73002 WHERE id=91001")
        .await
        .unwrap();
    let other_person_lease = app
        .client
        .post(prompts(&app, "/91001/lease"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    app.fixture
        .admin
        .execute_unprepared("UPDATE people SET person_type=1,has_capacity=false,date_of_birth=(CURRENT_DATE-interval '10 years')::date WHERE id=73001")
        .await
        .unwrap();
    let setup=app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT date_of_birth>(CURRENT_DATE-interval '18 years')::date AND person_type=1 AND NOT has_capacity AS minor FROM people WHERE id=73001")).await.unwrap().unwrap();
    assert!(setup.try_get::<bool>("", "minor").unwrap());
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
    assert_eq!(denied_status, 409);
    assert_eq!(other_person_lease, 403);
    assert_eq!(minor_status, 403);
}

#[tokio::test]
async fn review_refresh_resets_non_curated_matching_fields() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared(
        "INSERT INTO medication_review_evidence_records(id,source_name,source_record_id,source_url,source_version,source_effective_on,product_name,label_section,evidence_text,retrieved_on,risk_level,match_confidence,match_status,candidate_terms,pharmacologic_classes,interacting_terms,created_at,updated_at) VALUES (9876501,'openFDA / DailyMed SPL','synthetic-worker-label','https://example.test/old','1','2026-09-01','Old synthetic product','Drug Interactions','Old warning','2026-09-01','high','high','unreviewed',ARRAY['old']::varchar[],ARRAY['old']::varchar[],ARRAY['old']::varchar[],now(),now())"
    ).await.unwrap();
    let run_id = review_refresh_run(&app).await;
    import_snapshot(
        &app.context,
        run_id,
        &review_refresh_snapshot("2", "Changed synthetic warning"),
        NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
    )
    .await
    .unwrap();
    let row = app.fixture.admin.query_one_raw(Statement::from_string(
        DbBackend::Postgres,
        "SELECT risk_level,match_confidence,match_status,interacting_terms,candidate_terms FROM medication_review_evidence_records WHERE id=9876501",
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<String>("", "risk_level").unwrap(), "unknown");
    assert_eq!(
        row.try_get::<String>("", "match_confidence").unwrap(),
        "unknown"
    );
    assert_eq!(
        row.try_get::<String>("", "match_status").unwrap(),
        "unreviewed"
    );
    assert!(
        row.try_get::<Vec<String>>("", "interacting_terms")
            .unwrap()
            .is_empty()
    );
    assert_ne!(
        row.try_get::<Vec<String>>("", "candidate_terms").unwrap(),
        vec!["old"]
    );
    app.close().await;
}

#[tokio::test]
async fn review_refresh_retry_and_concurrent_delivery_keep_the_first_completed_run() {
    let app = Application::new().await;
    let run_id = review_refresh_run(&app).await;
    let date = NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();
    assert!(
        import_snapshot(&app.context, run_id, &json!({"labels":[]}), date)
            .await
            .is_err()
    );
    let snapshot = review_refresh_snapshot("1", "Synthetic warning");
    let (first, second) = tokio::join!(
        import_snapshot(&app.context, run_id, &snapshot, date),
        import_snapshot(&app.context, run_id, &snapshot, date)
    );
    assert!(first.is_ok() || second.is_ok());
    import_snapshot(&app.context, run_id, &snapshot, date)
        .await
        .unwrap();
    let row = app.fixture.admin.query_one_raw(Statement::from_string(
        DbBackend::Postgres,
        format!("SELECT created_count,updated_count,unchanged_count,status FROM medication_review_evidence_refresh_runs WHERE id={run_id}"),
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i32>("", "created_count").unwrap(), 1);
    assert_eq!(row.try_get::<i32>("", "updated_count").unwrap(), 0);
    assert_eq!(row.try_get::<i32>("", "unchanged_count").unwrap(), 0);
    assert_eq!(row.try_get::<i32>("", "status").unwrap(), 2);
    let count = app.fixture.admin.query_one_raw(Statement::from_string(
        DbBackend::Postgres,
        "SELECT count(*)::bigint AS count FROM medication_review_evidence_records WHERE source_record_id='synthetic-worker-label'",
    )).await.unwrap().unwrap().try_get::<i64>("", "count").unwrap();
    assert_eq!(count, 1);
    app.close().await;
}

#[tokio::test]
async fn review_refresh_retry_reopens_changed_prompt_after_evidence_commit() {
    let app = Application::new().await;
    seed_prompt(&app, 91251, "reviewed").await;
    app.fixture.admin.execute_unprepared(
        "UPDATE medication_review_prompts SET practitioner_name='Previous clinician',practitioner_role='Clinician',reviewed_on='2026-09-01',review_note='Prior decision' WHERE id=91251"
    ).await.unwrap();
    app.fixture.admin.execute_unprepared(
        "UPDATE medication_review_evidence_records SET source_name='openFDA / DailyMed SPL',source_record_id='synthetic-worker-label',source_version='1',source_effective_on='2026-09-01',evidence_text='Old warning' WHERE id=9125100"
    ).await.unwrap();
    app.fixture.admin.execute_unprepared(
        "CREATE FUNCTION review_refresh_reconcile_block() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic blocked reconcile'; END $$"
    ).await.unwrap();
    app.fixture.admin.execute_unprepared(
        "CREATE TRIGGER review_refresh_reconcile_block BEFORE UPDATE ON medication_review_prompts FOR EACH ROW EXECUTE FUNCTION review_refresh_reconcile_block()"
    ).await.unwrap();
    let run_id = review_refresh_run(&app).await;
    let snapshot = review_refresh_snapshot("2", "Changed synthetic warning");
    let date = NaiveDate::from_ymd_opt(2026, 10, 13).unwrap();
    assert!(
        import_snapshot(&app.context, run_id, &snapshot, date)
            .await
            .is_err()
    );
    let evidence = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT source_version FROM medication_review_evidence_records WHERE id=9125100",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        evidence
            .try_get::<Option<String>>("", "source_version")
            .unwrap()
            .as_deref(),
        Some("2")
    );
    let pending = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status FROM medication_review_prompts WHERE id=91251",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pending.try_get::<String>("", "status").unwrap(), "reviewed");
    app.fixture
        .admin
        .execute_unprepared(
            "DROP TRIGGER review_refresh_reconcile_block ON medication_review_prompts",
        )
        .await
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared("DROP FUNCTION review_refresh_reconcile_block()")
        .await
        .unwrap();
    import_snapshot(&app.context, run_id, &snapshot, date)
        .await
        .unwrap();
    let result = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        format!("SELECT r.status,r.updated_count,r.unchanged_count,p.status AS prompt_status,p.practitioner_name,p.evidence_text FROM medication_review_evidence_refresh_runs r CROSS JOIN medication_review_prompts p WHERE r.id={run_id} AND p.id=91251")
    )).await.unwrap().unwrap();
    assert_eq!(result.try_get::<i32>("", "status").unwrap(), 2);
    assert_eq!(result.try_get::<i32>("", "updated_count").unwrap(), 0);
    assert_eq!(result.try_get::<i32>("", "unchanged_count").unwrap(), 1);
    assert_eq!(
        result.try_get::<String>("", "prompt_status").unwrap(),
        "needs_review"
    );
    assert_eq!(
        result
            .try_get::<Option<String>>("", "practitioner_name")
            .unwrap(),
        None
    );
    assert_eq!(
        result.try_get::<String>("", "evidence_text").unwrap(),
        "Changed synthetic warning"
    );
    let histories = app.fixture.admin.query_all_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT event_type,snapshot FROM medication_review_histories WHERE prompt_id=91251 ORDER BY id"
    )).await.unwrap();
    assert_eq!(histories.len(), 1);
    assert_eq!(
        histories[0].try_get::<String>("", "event_type").unwrap(),
        "evidence"
    );
    let prior: Value = histories[0].try_get("", "snapshot").unwrap();
    assert_eq!(prior["practitioner_name"], "Previous clinician");
    assert_eq!(prior["review_note"], "Prior decision");
    app.close().await;
}

#[tokio::test]
async fn review_refresh_queued_job_survives_worker_restart_and_duplicate_delivery() {
    let requests = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let label = json!({"meta":{"last_updated":"2026-10-01"},"results":[
        {"set_id":"synthetic-worker-label","version":"1","effective_time":"20261001",
         "drug_interactions":["Synthetic warning"],"openfda":{"product_type":["HUMAN OTC DRUG"],
         "substance_name":["ibuprofen"],"brand_name":["Synthetic tablets"]}}
    ]});
    let provider = axum::Router::new().route(
        "/drug/label.json",
        axum::routing::get(move || {
            let label = label.clone();
            let requests = requests.clone();
            async move {
                if requests.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                    (
                        axum::http::StatusCode::SERVICE_UNAVAILABLE,
                        axum::Json(json!({})),
                    )
                } else {
                    (axum::http::StatusCode::OK, axum::Json(label))
                }
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/drug/label.json", listener.local_addr().unwrap());
    let provider_server =
        tokio::spawn(async move { axum::serve(listener, provider).await.unwrap() });
    let app = Application::new_configured(Some("synthetic-dose-occurrence-signing-key"), move |settings| {
        settings["review_refresh_fixture"] = json!({"endpoint":endpoint,"manifest":{"selection":["ibuprofen"],"targeted_selection":[]}});
    }).await;
    assert!(
        app.context
            .config
            .settings
            .as_ref()
            .unwrap()
            .get("review_refresh_fixture")
            .is_some()
    );
    let first_queue = app.context.queue_provider.as_ref().unwrap().clone();
    med_tracker::app::App::connect_workers(&app.context, &first_queue)
        .await
        .unwrap();
    let running = first_queue.clone();
    let first_worker = tokio::spawn(async move { running.run(vec![]).await });
    first_queue.shutdown().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), first_worker)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    RefreshTask
        .run(&app.context, &Vars::default())
        .await
        .unwrap();
    let run_id: i64 = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id FROM medication_review_evidence_refresh_runs ORDER BY id DESC LIMIT 1",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "id")
        .unwrap();
    let mut config = Config::new(&Environment::Test).unwrap();
    let Some(QueueConfig::Postgres(queue_config)) = config.queue.as_mut() else {
        panic!("PostgreSQL queue required")
    };
    queue_config.uri = app.fixture.runtime_uri.clone();
    let queue = std::sync::Arc::new(
        loco_rs::bgworker::pg::create_provider(queue_config)
            .await
            .unwrap(),
    );
    med_tracker::app::App::connect_workers(&app.context, &queue)
        .await
        .unwrap();
    let running = queue.clone();
    let worker = tokio::spawn(async move { running.run(vec![]).await });
    tokio::time::timeout(std::time::Duration::from_secs(20), async {
        loop {
            let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
                format!("SELECT status FROM medication_review_evidence_refresh_runs WHERE id={run_id}")
            )).await.unwrap().unwrap();
            if row.try_get::<i32>("", "status").unwrap() == 3 { break; }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }).await.unwrap();
    queue.shutdown().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), worker)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    app.fixture.admin.execute_unprepared(&format!(
        "UPDATE medication_review_evidence_refresh_runs SET status=1,error_message=NULL,completed_at=NULL,updated_at=now() WHERE id={run_id}"
    )).await.unwrap();
    let retry_job = RefreshWorker::perform_later(&app.context, RefreshRun { run_id })
        .await
        .unwrap();
    let mut config = Config::new(&Environment::Test).unwrap();
    let Some(QueueConfig::Postgres(queue_config)) = config.queue.as_mut() else {
        panic!("PostgreSQL queue required")
    };
    queue_config.uri = app.fixture.runtime_uri.clone();
    let queue = std::sync::Arc::new(
        loco_rs::bgworker::pg::create_provider(queue_config)
            .await
            .unwrap(),
    );
    med_tracker::app::App::connect_workers(&app.context, &queue)
        .await
        .unwrap();
    let running = queue.clone();
    let worker = tokio::spawn(async move { running.run(vec![]).await });
    tokio::time::timeout(std::time::Duration::from_secs(20), async {
        loop {
            let row = app
                .fixture
                .admin
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "SELECT status FROM pg_loco_queue WHERE id=$1",
                    [retry_job.clone().into()],
                ))
                .await
                .unwrap()
                .unwrap();
            if row.try_get::<String>("", "status").unwrap() == "completed" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    queue.shutdown().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), worker)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let duplicate_job = RefreshWorker::perform_later(&app.context, RefreshRun { run_id })
        .await
        .unwrap();
    let mut config = Config::new(&Environment::Test).unwrap();
    let Some(QueueConfig::Postgres(queue_config)) = config.queue.as_mut() else {
        panic!("PostgreSQL queue required")
    };
    queue_config.uri = app.fixture.runtime_uri.clone();
    let queue = std::sync::Arc::new(
        loco_rs::bgworker::pg::create_provider(queue_config)
            .await
            .unwrap(),
    );
    med_tracker::app::App::connect_workers(&app.context, &queue)
        .await
        .unwrap();
    let running = queue.clone();
    let worker = tokio::spawn(async move { running.run(vec![]).await });
    tokio::time::timeout(std::time::Duration::from_secs(20), async {
        loop {
            let row = app
                .fixture
                .admin
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "SELECT status FROM pg_loco_queue WHERE id=$1",
                    [duplicate_job.clone().into()],
                ))
                .await
                .unwrap()
                .unwrap();
            if row.try_get::<String>("", "status").unwrap() == "completed" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    queue.shutdown().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), worker)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        format!("SELECT status,created_count,updated_count,unchanged_count FROM medication_review_evidence_refresh_runs WHERE id={run_id}")
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i32>("", "status").unwrap(), 2);
    assert_eq!(row.try_get::<i32>("", "created_count").unwrap(), 1);
    assert_eq!(row.try_get::<i32>("", "updated_count").unwrap(), 0);
    assert_eq!(row.try_get::<i32>("", "unchanged_count").unwrap(), 0);
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*)::bigint AS count FROM medication_review_evidence_records WHERE source_record_id='synthetic-worker-label'"
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    app.close().await;
    provider_server.abort();
}
