use super::*;
use chrono::NaiveDate;
use loco_rs::{
    bgworker::BackgroundWorker,
    task::{Task, Vars},
};
use med_tracker::models::_entities as rows;
use med_tracker::models::care::review_prompts::refresh::{
    RefreshRun, RefreshTask, RefreshWorker, import_snapshot,
};
use sea_orm::sea_query::{Expr, ExprTrait, Func, Query};

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
    rows::medication_review_evidence_refresh_runs::ActiveModel {
        status: Set(0),
        created_at: Set(chrono::Utc::now().naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap()
    .id
}

fn review_refresh_snapshot(version: &str, text: &str) -> Value {
    json!({"openfda_last_updated":"2026-10-01","labels":[{"set_id":"synthetic-worker-label","version":version,"effective_time":"20261001","selection_term":"ibuprofen","drug_interactions":[text],"openfda":{"brand_name":["Synthetic tablets"],"substance_name":["ibuprofen"]}}]})
}

struct SeededPrompt {
    prompt: rows::medication_review_prompts::Model,
    evidence: rows::medication_review_evidence_records::Model,
    identity: Option<rows::medication_review_evidence_records::Model>,
    other_medication: Option<rows::medications::Model>,
}

async fn evidence(
    app: &Application,
    product: &str,
) -> rows::medication_review_evidence_records::Model {
    let now = chrono::Utc::now().naive_utc();
    let unique = uuid::Uuid::new_v4();
    rows::medication_review_evidence_records::ActiveModel {
        source_name: Set("Synthetic reference".into()),
        source_record_id: Set(format!("report-{unique}")),
        source_url: Set(format!("https://example.test/evidence/{unique}")),
        product_name: Set(product.into()),
        label_section: Set("Warnings".into()),
        evidence_text: Set("Synthetic evidence.".into()),
        retrieved_on: Set("2026-01-01".parse().unwrap()),
        risk_level: Set("moderate".into()),
        match_confidence: Set("high".into()),
        match_status: Set("reviewed_pair".into()),
        candidate_terms: Set(vec!["synthetic tablets".into()]),
        interacting_terms: Set(vec!["synthetic tablets".into()]),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap()
}

async fn fingerprint(app: &Application, prompt_id: i64, evidence_id: i64) {
    let values = [
        "source_name",
        "source_url",
        "source_version",
        "source_effective_on",
        "evidence_text",
        "risk_level",
        "match_confidence",
    ]
    .map(Expr::col);
    let query = Query::select()
        .expr_as(
            Func::cust("md5").args([Func::cust("jsonb_build_array")
                .args(values)
                .cast_as(sea_orm::sea_query::Alias::new("text"))]),
            "fingerprint",
        )
        .from(rows::medication_review_evidence_records::Entity)
        .and_where(Expr::col("id").eq(evidence_id))
        .to_owned();
    let fingerprint: String = app
        .fixture
        .admin
        .query_one_raw(app.fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "fingerprint")
        .unwrap();
    let update = Query::update()
        .table(rows::medication_review_prompts::Entity)
        .value("evidence_source_fingerprint", fingerprint)
        .and_where(Expr::col("id").eq(prompt_id))
        .to_owned();
    app.fixture
        .admin
        .execute_raw(app.fixture.admin.get_database_backend().build(&update))
        .await
        .unwrap();
}

async fn seed_prompt(app: &Application, status: &str) -> SeededPrompt {
    let evidence = evidence(app, "Synthetic tablets").await;
    let now = chrono::Utc::now().naive_utc();
    let prompt = rows::medication_review_prompts::ActiveModel {
        household_id: Set(app.fixture.household.id),
        person_id: Set(app.fixture.adult.id),
        primary_medication_id: Set(app.fixture.medication.id),
        interacting_medication_id: Set(app.fixture.medication.id),
        evidence_record_id: Set(evidence.id),
        primary_medication_name: Set("Synthetic tablets".into()),
        interacting_medication_name: Set("Synthetic tablets".into()),
        evidence_source_name: Set("Synthetic reference".into()),
        evidence_source_url: Set(evidence.source_url.clone()),
        evidence_source_version: Set("1".into()),
        evidence_source_checked_on: Set("2026-01-01".parse().unwrap()),
        evidence_source_effective_on: Set("2026-01-01".parse().unwrap()),
        evidence_text: Set("Synthetic evidence.".into()),
        match_confidence: Set("high".into()),
        match_reason: Set(
            "A reviewed rule identifies synthetic tablets as the interacting medicine.".into(),
        ),
        match_type: Set("curated".into()),
        matched_term: Set("synthetic tablets".into()),
        risk_level: Set("moderate".into()),
        source_instruction: Set("unclassified".into()),
        status: Set(status.into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    fingerprint(app, prompt.id, evidence.id).await;
    SeededPrompt {
        prompt,
        evidence,
        identity: None,
        other_medication: None,
    }
}

async fn seed_matched_prompt(app: &Application, status: &str) -> SeededPrompt {
    let mut seeded = seed_prompt(app, status).await;
    let mut medication = app.fixture.medication.clone().into_active_model();
    medication.name = Set(Some("Aspirin tablets".into()));
    medication.update(&app.fixture.admin).await.unwrap();
    let now = chrono::Utc::now().naive_utc();
    let other = rows::medications::ActiveModel {
        household_id: Set(app.fixture.household.id),
        location_id: Set(app.fixture.location.id),
        name: Set(Some("Warfarin tablets".into())),
        current_supply: Set(Some(sea_orm::prelude::Decimal::new(10, 0))),
        dose_amount: Set(Some(1.0)),
        dose_unit: Set(Some("tablet".into())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    rows::person_medications::ActiveModel {
        household_id: Set(app.fixture.household.id),
        person_id: Set(app.fixture.adult.id),
        medication_id: Set(other.id),
        dose_amount: Set(Some(sea_orm::prelude::Decimal::ONE)),
        dose_unit: Set(Some("tablet".into())),
        position: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let mut label = seeded.evidence.clone().into_active_model();
    label.source_version = Set(Some("1".into()));
    label.evidence_text = Set("Avoid warfarin with aspirin.".into());
    label.match_status = Set("unreviewed".into());
    label.candidate_terms = Set(vec!["aspirin".into()]);
    label.interacting_terms = Set(vec![]);
    label.risk_level = Set("unknown".into());
    label.match_confidence = Set("unknown".into());
    seeded.evidence = label.update(&app.fixture.admin).await.unwrap();
    let identity = evidence(app, "Warfarin").await;
    let mut identity = identity.into_active_model();
    identity.evidence_text = Set("Warfarin medicine information.".into());
    identity.risk_level = Set("unknown".into());
    identity.match_confidence = Set("unknown".into());
    identity.match_status = Set("unreviewed".into());
    identity.candidate_terms = Set(vec!["warfarin".into()]);
    identity.pharmacologic_classes = Set(vec!["anticoagulant".into()]);
    identity.interacting_terms = Set(vec![]);
    let identity = identity.update(&app.fixture.admin).await.unwrap();
    let mut prompt = seeded.prompt.clone().into_active_model();
    prompt.interacting_medication_id = Set(other.id);
    prompt.primary_medication_name = Set("Aspirin tablets".into());
    prompt.interacting_medication_name = Set("Warfarin tablets".into());
    prompt.evidence_source_version = Set("1".into());
    prompt.evidence_text = Set("Avoid warfarin with aspirin.".into());
    prompt.matched_term = Set("warfarin".into());
    prompt.match_type = Set("ingredient".into());
    prompt.source_instruction = Set("avoid".into());
    prompt.match_reason = Set("The label explicitly names the ingredient warfarin.".into());
    prompt.risk_level = Set("high".into());
    prompt.match_confidence = Set("high".into());
    seeded.prompt = prompt.update(&app.fixture.admin).await.unwrap();
    fingerprint(app, seeded.prompt.id, seeded.evidence.id).await;
    seeded.identity = Some(identity);
    seeded.other_medication = Some(other);
    seeded
}

fn prompts(app: &Application, suffix: &str) -> String {
    let fixture_household_id = app.fixture.household.id;
    format!(
        "{}/api/v1/households/{fixture_household_id}/medication_review_prompts{suffix}",
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
    crate::reference_fixture::clean_review_catalogue().await;
    let app = Application::new().await;
    let token = app.token().await;
    let seeded_91101 = seed_prompt(&app, "needs_review").await;
    let prompt_id_91101 = seeded_91101.prompt.id;
    let shown = app
        .client
        .get(prompts(&app, &format!("/{prompt_id_91101}")))
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
        .patch(prompts(&app, &format!("/{prompt_id_91101}")))
        .bearer_auth(&token)
        .header("if-match", &etag)
        .json(&json!({"medication_review_prompt":{"review_note":"Unleased"}}))
        .send()
        .await
        .unwrap();
    let acquired = app
        .client
        .post(prompts(&app, &format!("/{prompt_id_91101}/lease")))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let acquire_status = acquired.status().as_u16();
    let acquired_body: Value = acquired.json().await.unwrap();
    let lease = acquired_body["data"]["token"].as_str().unwrap_or("");
    let competing = app
        .client
        .post(prompts(&app, &format!("/{prompt_id_91101}/lease")))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let saved = app
        .client
        .patch(prompts(&app, &format!("/{prompt_id_91101}")))
        .bearer_auth(&token)
        .header("if-match", &etag)
        .header("x-review-lease", lease)
        .json(&json!({"medication_review_prompt":{"review_note":"Leased note"}}))
        .send()
        .await
        .unwrap();
    let reacquired = app
        .client
        .post(prompts(&app, &format!("/{prompt_id_91101}/lease")))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let reacquired_status = reacquired.status().as_u16();
    let reacquired_body: Value = reacquired.json().await.unwrap();
    let cancel = app
        .client
        .delete(prompts(&app, &format!("/{prompt_id_91101}/lease")))
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
        .delete(prompts(&app, &format!("/{prompt_id_91101}/lease")))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let remaining = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("medication_review_edit_leases", "token")))
                    .from("medication_review_edit_leases")
                    .cond_where(
                        Expr::col(("medication_review_edit_leases", "prompt_id"))
                            .eq(Expr::val(prompt_id_91101)),
                    )
                    .to_owned(),
            ),
        )
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
    crate::reference_fixture::clean_review_catalogue().await;
    let app = Application::new().await;
    let token = app.token().await;
    let seeded_91252 = seed_prompt(&app, "not_relevant").await;
    let prompt_id_91252 = seeded_91252.prompt.id;
    let shown = app
        .client
        .get(prompts(&app, &format!("/{prompt_id_91252}")))
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
    let lease = edit_token(&app, &token, prompt_id_91252).await;
    let saved = app
        .client
        .patch(prompts(&app, &format!("/{prompt_id_91252}")))
        .bearer_auth(&token)
        .header("if-match", &etag)
        .header("x-review-lease", &lease)
        .json(&json!({"medication_review_prompt":{"status":"not_relevant"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(saved.status().as_u16(), 200);
    assert_eq!(saved.headers().get("etag").unwrap().to_str().unwrap(), etag);
    let count = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr_as(
                        Expr::col(("medication_review_histories", "id")).count(),
                        "total",
                    )
                    .from("medication_review_histories")
                    .cond_where(
                        Expr::col(("medication_review_histories", "prompt_id"))
                            .eq(Expr::val(prompt_id_91252)),
                    )
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "total")
        .unwrap();
    assert_eq!(count, 0);
    let remaining = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("medication_review_edit_leases", "token")))
                    .from("medication_review_edit_leases")
                    .cond_where(
                        Expr::col(("medication_review_edit_leases", "prompt_id"))
                            .eq(Expr::val(prompt_id_91252)),
                    )
                    .to_owned(),
            ),
        )
        .await
        .unwrap();
    assert!(remaining.is_none());
    app.close().await;
}

#[tokio::test]
async fn changed_evidence_reopens_review_and_preserves_the_previous_snapshot() {
    crate::reference_fixture::clean_review_catalogue().await;
    let app = Application::new().await;
    let fixture_adult_id = app.fixture.adult.id;
    let fixture_membership_id = app.fixture.membership.id;
    let token = app.token().await;
    let seeded_91102 = seed_matched_prompt(&app, "reviewed_with_practitioner").await;
    let prompt_id_91102 = seeded_91102.prompt.id;
    let evidence_id_91102 = seeded_91102.evidence.id;
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_prompts")
                        .value("practitioner_name", Expr::val("Dr Test"))
                        .value("practitioner_role", Expr::val("GP"))
                        .value(
                            "reviewed_on",
                            Expr::val("2026-10-01".parse::<chrono::NaiveDate>().unwrap()),
                        )
                        .value("review_note", Expr::val("Discussed"))
                        .value(
                            "reviewed_by_membership_id",
                            Expr::val(fixture_membership_id),
                        )
                        .cond_where(
                            Expr::col(("medication_review_prompts", "id"))
                                .eq(Expr::val(prompt_id_91102)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_evidence_records")
                        .value(
                            "retrieved_on",
                            Expr::val("2026-10-02".parse::<chrono::NaiveDate>().unwrap()),
                        )
                        .cond_where(
                            Expr::col(("medication_review_evidence_records", "id"))
                                .eq(Expr::val(evidence_id_91102)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
    let before: Value = app
        .client
        .get(prompts(&app, &format!("/{prompt_id_91102}")))
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
        .get(prompts(&app, &format!("/{prompt_id_91102}")))
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
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_evidence_records")
                        .value("source_version", Expr::val("2"))
                        .value(
                            "evidence_text",
                            Expr::val("Monitor anticoagulants with aspirin."),
                        )
                        .cond_where(
                            Expr::col(("medication_review_evidence_records", "id"))
                                .eq(Expr::val(evidence_id_91102)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
    let detail_after_change: Value = app
        .client
        .get(prompts(&app, &format!("/{prompt_id_91102}")))
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
        .get(prompts(&app, &format!("/{prompt_id_91102}")))
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
    assert_eq!(snapshot["id"], prompt_id_91102.to_string());
    assert_eq!(snapshot["person_id"], fixture_adult_id.to_string());
    assert_eq!(snapshot["etag"], original_etag);
    assert!(!snapshot.contains_key("evidence_source_fingerprint"));
    assert!(!snapshot.contains_key("household_id"));
}

#[tokio::test]
async fn changed_evidence_without_a_pair_match_retires_the_warning_and_keeps_history() {
    crate::reference_fixture::clean_review_catalogue().await;
    let app = Application::new().await;
    let fixture_membership_id = app.fixture.membership.id;
    let token = app.token().await;
    let seeded_91108 = seed_matched_prompt(&app, "reviewed_with_practitioner").await;
    let prompt_id_91108 = seeded_91108.prompt.id;
    let evidence_id_91108 = seeded_91108.evidence.id;
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_prompts")
                        .value("practitioner_name", Expr::val("Dr Test"))
                        .value("practitioner_role", Expr::val("GP"))
                        .value(
                            "reviewed_on",
                            Expr::val("2026-10-01".parse::<chrono::NaiveDate>().unwrap()),
                        )
                        .value("review_note", Expr::val("Discussed"))
                        .value(
                            "reviewed_by_membership_id",
                            Expr::val(fixture_membership_id),
                        )
                        .cond_where(
                            Expr::col(("medication_review_prompts", "id"))
                                .eq(Expr::val(prompt_id_91108)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_evidence_records")
                        .value("source_version", Expr::val("2"))
                        .value(
                            "evidence_text",
                            Expr::val("This product has no named interacting medicines."),
                        )
                        .cond_where(
                            Expr::col(("medication_review_evidence_records", "id"))
                                .eq(Expr::val(evidence_id_91108)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
    let first: Value = app
        .client
        .get(prompts(&app, &format!("/{prompt_id_91108}")))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let second: Value = app
        .client
        .get(prompts(&app, &format!("/{prompt_id_91108}")))
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
    crate::reference_fixture::clean_review_catalogue().await;
    let app = Application::new().await;
    let fixture_membership_id = app.fixture.membership.id;
    let token = app.token().await;
    let seeded_91109 = seed_prompt(&app, "reviewed_with_practitioner").await;
    let prompt_id_91109 = seeded_91109.prompt.id;
    let evidence_id_91109 = seeded_91109.evidence.id;
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_prompts")
                        .value("practitioner_name", Expr::val("Dr Test"))
                        .value("practitioner_role", Expr::val("GP"))
                        .value(
                            "reviewed_on",
                            Expr::val("2026-10-01".parse::<chrono::NaiveDate>().unwrap()),
                        )
                        .value("review_note", Expr::val("Discussed"))
                        .value(
                            "reviewed_by_membership_id",
                            Expr::val(fixture_membership_id),
                        )
                        .cond_where(
                            Expr::col(("medication_review_prompts", "id"))
                                .eq(Expr::val(prompt_id_91109)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
    let first: Value = app
        .client
        .get(prompts(&app, &format!("/{prompt_id_91109}")))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_evidence_records")
                        .value(
                            "retrieved_on",
                            Expr::val("2026-10-02".parse::<chrono::NaiveDate>().unwrap()),
                        )
                        .cond_where(
                            Expr::col(("medication_review_evidence_records", "id"))
                                .eq(Expr::val(evidence_id_91109)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
    let second: Value = app
        .client
        .get(prompts(&app, &format!("/{prompt_id_91109}")))
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
    crate::reference_fixture::clean_review_catalogue().await;
    let app = Application::new().await;
    let fixture_membership_id = app.fixture.membership.id;
    let token = app.token().await;
    let seeded_91110 = seed_matched_prompt(&app, "reviewed_with_practitioner").await;
    let prompt_id_91110 = seeded_91110.prompt.id;
    let evidence_id_91110 = seeded_91110.evidence.id;
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_prompts")
                        .value("practitioner_name", Expr::val("Dr Test"))
                        .value("practitioner_role", Expr::val("GP"))
                        .value(
                            "reviewed_on",
                            Expr::val("2026-10-01".parse::<chrono::NaiveDate>().unwrap()),
                        )
                        .value("review_note", Expr::val("Discussed"))
                        .value(
                            "reviewed_by_membership_id",
                            Expr::val(fixture_membership_id),
                        )
                        .cond_where(
                            Expr::col(("medication_review_prompts", "id"))
                                .eq(Expr::val(prompt_id_91110)),
                        )
                        .to_owned(),
                ),
            )
            .await?;
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_evidence_records")
                        .value("candidate_terms", Expr::val(vec!["unrelated".to_owned()]))
                        .cond_where(
                            Expr::col(("medication_review_evidence_records", "id"))
                                .eq(Expr::val(evidence_id_91110)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
    let retired: Value = app
        .client
        .get(prompts(&app, &format!("/{prompt_id_91110}")))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_evidence_records")
                        .value("candidate_terms", Expr::val(vec!["aspirin".to_owned()]))
                        .cond_where(
                            Expr::col(("medication_review_evidence_records", "id"))
                                .eq(Expr::val(evidence_id_91110)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
    let restored: Value = app
        .client
        .get(prompts(&app, &format!("/{prompt_id_91110}")))
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
    crate::reference_fixture::clean_review_catalogue().await;
    let app = Application::new().await;
    let fixture_membership_id = app.fixture.membership.id;
    let token = app.token().await;
    let seeded_91111 = seed_matched_prompt(&app, "reviewed_with_practitioner").await;
    let prompt_id_91111 = seeded_91111.prompt.id;
    let evidence_id_91111 = seeded_91111.evidence.id;
    let identity_id_91111 = seeded_91111.identity.as_ref().unwrap().id;
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_evidence_records")
                        .value(
                            "evidence_text",
                            Expr::val("Monitor glimmerase with aspirin."),
                        )
                        .cond_where(
                            Expr::col(("medication_review_evidence_records", "id"))
                                .eq(Expr::val(evidence_id_91111)),
                        )
                        .to_owned(),
                ),
            )
            .await?;
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_evidence_records")
                        .value(
                            "candidate_terms",
                            Expr::val(vec!["warfarin".to_owned(), "glimmerase".to_owned()]),
                        )
                        .cond_where(
                            Expr::col(("medication_review_evidence_records", "id"))
                                .eq(Expr::val(identity_id_91111)),
                        )
                        .to_owned(),
                ),
            )
            .await?;
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_prompts")
                        .value(
                            "evidence_text",
                            Expr::val("Monitor glimmerase with aspirin."),
                        )
                        .value("matched_term", Expr::val("glimmerase"))
                        .value("match_type", Expr::val("ingredient"))
                        .value("source_instruction", Expr::val("monitor_or_adjust"))
                        .value(
                            "match_reason",
                            Expr::val("The label explicitly names the ingredient glimmerase."),
                        )
                        .value("risk_level", Expr::val("moderate"))
                        .value("match_confidence", Expr::val("high"))
                        .value("practitioner_name", Expr::val("Dr Test"))
                        .value("practitioner_role", Expr::val("GP"))
                        .value(
                            "reviewed_on",
                            Expr::val("2026-10-01".parse::<chrono::NaiveDate>().unwrap()),
                        )
                        .value("review_note", Expr::val("Discussed"))
                        .value(
                            "reviewed_by_membership_id",
                            Expr::val(fixture_membership_id),
                        )
                        .cond_where(
                            Expr::col(("medication_review_prompts", "id"))
                                .eq(Expr::val(prompt_id_91111)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
    fingerprint(&app, prompt_id_91111, evidence_id_91111).await;
    let before: Value = app
        .client
        .get(prompts(&app, &format!("/{prompt_id_91111}")))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_evidence_records")
                        .value("candidate_terms", Expr::val(vec!["warfarin".to_owned()]))
                        .cond_where(
                            Expr::col(("medication_review_evidence_records", "id"))
                                .eq(Expr::val(identity_id_91111)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
    let after: Value = app
        .client
        .get(prompts(&app, &format!("/{prompt_id_91111}")))
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
    crate::reference_fixture::clean_review_catalogue().await;
    let app = Application::new().await;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_medication_id = app.fixture.medication.id;
    let token = app.token().await;
    let seeded_91112 = seed_matched_prompt(&app, "reviewed_with_practitioner").await;
    let prompt_id_91112 = seeded_91112.prompt.id;
    let evidence_id_91112 = seeded_91112.evidence.id;
    let matching_medication_id = seeded_91112.other_medication.as_ref().unwrap().id;
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_evidence_records")
                        .value("match_status", Expr::val("reviewed_pair"))
                        .value("candidate_terms", Expr::val(vec!["aspirin".to_owned()]))
                        .value("interacting_terms", Expr::val(vec!["warfarin".to_owned()]))
                        .value("risk_level", Expr::val("high"))
                        .value("match_confidence", Expr::val("high"))
                        .cond_where(
                            Expr::col(("medication_review_evidence_records", "id"))
                                .eq(Expr::val(evidence_id_91112)),
                        )
                        .to_owned(),
                ),
            )
            .await?;
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_prompts")
                        .value("match_type", Expr::val("curated"))
                        .value("matched_term", Expr::val("warfarin"))
                        .value("source_instruction", Expr::val("unclassified"))
                        .value(
                            "match_reason",
                            Expr::val(
                                "A reviewed rule identifies warfarin as the interacting medicine.",
                            ),
                        )
                        .value("practitioner_name", Expr::val("Dr Test"))
                        .value("practitioner_role", Expr::val("GP"))
                        .value(
                            "reviewed_on",
                            Expr::val("2026-10-01".parse::<chrono::NaiveDate>().unwrap()),
                        )
                        .value("review_note", Expr::val("Discussed"))
                        .value(
                            "reviewed_by_membership_id",
                            Expr::val(fixture_membership_id),
                        )
                        .cond_where(
                            Expr::col(("medication_review_prompts", "id"))
                                .eq(Expr::val(prompt_id_91112)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
    fingerprint(&app, prompt_id_91112, evidence_id_91112).await;
    let before: Value = app
        .client
        .get(prompts(&app, &format!("/{prompt_id_91112}")))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_evidence_records")
                        .value("candidate_terms", Expr::val(vec!["warfarin".to_owned()]))
                        .value("interacting_terms", Expr::val(vec!["aspirin".to_owned()]))
                        .cond_where(
                            Expr::col(("medication_review_evidence_records", "id"))
                                .eq(Expr::val(evidence_id_91112)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
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
        .get(prompts(&app, &format!("/{prompt_id_91112}")))
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
    assert_eq!(
        visible[0]["primary_medication_id"],
        matching_medication_id.to_string()
    );
    assert_eq!(
        visible[0]["interacting_medication_id"],
        fixture_medication_id.to_string()
    );
    assert_eq!(visible[0]["matched_term"], "aspirin");
    assert_eq!(visible[0]["status"], "needs_review");
}

#[tokio::test]
async fn review_date_can_be_cleared_for_a_non_practitioner_outcome() {
    crate::reference_fixture::clean_review_catalogue().await;
    let app = Application::new().await;
    let fixture_membership_id = app.fixture.membership.id;
    let token = app.token().await;
    for date in [Value::Null, json!("")] {
        let seeded = seed_prompt(&app, "reviewed_with_practitioner").await;
        let id = seeded.prompt.id;
        async {
            app.fixture
                .admin
                .execute_raw(
                    app.fixture.admin.get_database_backend().build(
                        &Query::update()
                            .table("medication_review_prompts")
                            .value("practitioner_name", Expr::val("Dr Test"))
                            .value("practitioner_role", Expr::val("GP"))
                            .value(
                                "reviewed_on",
                                Expr::val("2026-10-01".parse::<chrono::NaiveDate>().unwrap()),
                            )
                            .value(
                                "reviewed_by_membership_id",
                                Expr::val(fixture_membership_id),
                            )
                            .cond_where(
                                Expr::col(("medication_review_prompts", "id")).eq(Expr::val(id)),
                            )
                            .to_owned(),
                    ),
                )
                .await
        }
        .await
        .unwrap();
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
        assert!(Value::is_null(&body["data"]["reviewed_on"]));
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
    crate::reference_fixture::clean_review_catalogue().await;
    let app = Application::new().await;
    let token = app.token().await;
    let seeded_91105 = seed_prompt(&app, "needs_review").await;
    let prompt_id_91105 = seeded_91105.prompt.id;
    let evidence_id_91105 = seeded_91105.evidence.id;
    let shown: Value = app
        .client
        .get(prompts(&app, &format!("/{prompt_id_91105}")))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let etag = shown["data"]["etag"].as_str().unwrap();
    let lease = edit_token(&app, &token, prompt_id_91105).await;
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_evidence_records")
                        .value("source_version", Expr::val("2"))
                        .value("evidence_text", Expr::val("New evidence before save."))
                        .cond_where(
                            Expr::col(("medication_review_evidence_records", "id"))
                                .eq(Expr::val(evidence_id_91105)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
    let stale = app
        .client
        .patch(prompts(&app, &format!("/{prompt_id_91105}")))
        .bearer_auth(&token)
        .header("if-match", etag)
        .header("x-review-lease", &lease)
        .json(&json!({"medication_review_prompt":{"status":"not_relevant"}}))
        .send()
        .await
        .unwrap();
    let current: Value = app
        .client
        .get(prompts(&app, &format!("/{prompt_id_91105}")))
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
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr_as(
                        Expr::col(("medication_review_edit_leases", "token"))
                            .cast_as(sea_orm::sea_query::Alias::new("text")),
                        "token",
                    )
                    .from("medication_review_edit_leases")
                    .cond_where(
                        Expr::col(("medication_review_edit_leases", "prompt_id"))
                            .eq(Expr::val(prompt_id_91105)),
                    )
                    .to_owned(),
            ),
        )
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
    crate::reference_fixture::clean_review_catalogue().await;
    let app = Application::new().await;
    let fixture_household_id = app.fixture.household.id;
    let fixture_membership_id = app.fixture.membership.id;
    let fixture_grant_id = app.fixture.grant.id;
    let token = app.token().await;
    let seeded_91103 = seed_prompt(&app, "needs_review").await;
    let prompt_id_91103 = seeded_91103.prompt.id;
    let shown: Value = app
        .client
        .get(prompts(&app, &format!("/{prompt_id_91103}")))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let etag = shown["data"]["etag"].as_str().unwrap();
    let lease = edit_token(&app, &token, prompt_id_91103).await;
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_edit_leases")
                        .value(
                            "expires_at",
                            Expr::val(
                                chrono::Utc::now().naive_utc() - chrono::Duration::seconds(1),
                            ),
                        )
                        .cond_where(
                            Expr::col(("medication_review_edit_leases", "prompt_id"))
                                .eq(Expr::val(prompt_id_91103)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
    let expired = app
        .client
        .patch(prompts(&app, &format!("/{prompt_id_91103}")))
        .bearer_auth(&token)
        .header("if-match", etag)
        .header("x-review-lease", &lease)
        .json(&json!({"medication_review_prompt":{"review_note":"Expired"}}))
        .send()
        .await
        .unwrap();
    let new_lease = edit_token(&app, &token, prompt_id_91103).await;
    let sync = app.client.post(format!("{}/api/v1/households/{fixture_household_id}/sync/batches", app.origin))
        .bearer_auth(&token)
        .json(&json!({"batch":{"operations":[{"resource_type":"medication_review_prompt","action":"update","id":prompt_id_91103.to_string(),"if_match":etag,"attributes":{"status":"not_relevant","review_note":"No lease in sync"}}]}}))
        .send().await.unwrap();
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("person_access_grants")
                        .value("access_level", Expr::val("view"))
                        .cond_where(
                            Expr::col(("person_access_grants", "id"))
                                .eq(Expr::val(fixture_grant_id)),
                        )
                        .to_owned(),
                ),
            )
            .await?;
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("household_memberships")
                        .value("revoked_at", Expr::val(chrono::Utc::now().naive_utc()))
                        .cond_where(
                            Expr::col(("household_memberships", "id"))
                                .eq(Expr::val(fixture_membership_id)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
    let revoked = app
        .client
        .patch(prompts(&app, &format!("/{prompt_id_91103}")))
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
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("medication_review_prompts", "status")))
                    .expr(Expr::col(("medication_review_prompts", "review_note")))
                    .from("medication_review_prompts")
                    .cond_where(
                        Expr::col(("medication_review_prompts", "id"))
                            .eq(Expr::val(prompt_id_91103)),
                    )
                    .to_owned(),
            ),
        )
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
    crate::reference_fixture::clean_review_catalogue().await;
    let app = Application::new().await;
    let token = app.token().await;
    let seeded_91104 = seed_prompt(&app, "needs_review").await;
    let prompt_id_91104 = seeded_91104.prompt.id;
    let shown: Value = app
        .client
        .get(prompts(&app, &format!("/{prompt_id_91104}")))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let etag = shown["data"]["etag"].as_str().unwrap();
    let lease = edit_token(&app, &token, prompt_id_91104).await;
    // SeaORM Schema cannot define PostgreSQL trigger functions. These fault cases require whole-run exclusivity.
    app.fixture.admin.execute_unprepared(
        "CREATE FUNCTION synthetic_fail_review_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='medication_review_prompt.updated' THEN RAISE EXCEPTION 'synthetic audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER synthetic_review_audit_failure BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION synthetic_fail_review_audit()",
    ).await.unwrap();
    let failed = app
        .client
        .patch(prompts(&app, &format!("/{prompt_id_91104}")))
        .bearer_auth(&token)
        .header("if-match", etag)
        .header("x-review-lease", &lease)
        .json(&json!({"medication_review_prompt":{"review_note":"Should roll back"}}))
        .send()
        .await
        .unwrap();
    let stored = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .column(("p", "review_note"))
                    .expr_as(
                        Expr::col(("l", "token")).cast_as(sea_orm::sea_query::Alias::new("text")),
                        "token",
                    )
                    .from_as(rows::medication_review_prompts::Entity, "p")
                    .join_as(
                        sea_orm::JoinType::InnerJoin,
                        "medication_review_edit_leases",
                        "l",
                        Expr::col(("l", "prompt_id")).eq(Expr::col(("p", "id"))),
                    )
                    .and_where(Expr::col(("p", "id")).eq(prompt_id_91104))
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
    let note: Option<String> = stored.try_get("", "review_note").unwrap();
    let stored_token: String = stored.try_get("", "token").unwrap();
    app.fixture.admin.execute_unprepared("DROP TRIGGER synthetic_review_audit_failure ON security_audit_events; DROP FUNCTION synthetic_fail_review_audit()").await.unwrap();
    app.close().await;
    assert_eq!(failed.status().as_u16(), 503);
    assert_eq!(note, None);
    assert_eq!(stored_token, lease);
}

#[tokio::test]
async fn review_prompt_index_matches_documented_contract() {
    crate::reference_fixture::clean_review_catalogue().await;
    let app = Application::new().await;
    let fixture_adult_id = app.fixture.adult.id;
    let token = app.token().await;
    let seeded_91001 = seed_prompt(&app, "needs_review").await;
    let prompt_id_91001 = seeded_91001.prompt.id;
    seed_prompt(&app, "hidden_low_signal").await;
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
    assert_eq!(rows[0]["id"], prompt_id_91001.to_string());
    assert_eq!(rows[0]["person_id"], fixture_adult_id.to_string());
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
    crate::reference_fixture::clean_review_catalogue().await;
    let app = Application::new().await;
    let fixture_membership_id = app.fixture.membership.id;
    let token = app.token().await;
    let seeded_91001 = seed_prompt(&app, "needs_review").await;
    let prompt_id_91001 = seeded_91001.prompt.id;
    let shown = app
        .client
        .get(prompts(&app, &format!("/{prompt_id_91001}")))
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
    let lease = edit_token(&app, &token, prompt_id_91001).await;
    let shown_body: Value = shown.json().await.unwrap();
    assert_eq!(shown_status, 200);
    assert_eq!(shown_body["data"]["id"], prompt_id_91001.to_string());
    let missing = app
        .client
        .patch(prompts(&app, &format!("/{prompt_id_91001}")))
        .bearer_auth(&token)
        .json(&json!({"medication_review_prompt":{"review_note":"early note"}}))
        .send()
        .await
        .unwrap();
    let missing_status = missing.status().as_u16();
    let stale = app
        .client
        .patch(prompts(&app, &format!("/{prompt_id_91001}")))
        .bearer_auth(&token)
        .header("if-match", "\"stale\"")
        .json(&json!({"medication_review_prompt":{"review_note":"stale note"}}))
        .send()
        .await
        .unwrap();
    let stale_status = stale.status().as_u16();
    let incomplete = app
        .client
        .patch(prompts(&app, &format!("/{prompt_id_91001}")))
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
        .put(prompts(&app, &format!("/{prompt_id_91001}")))
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
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("security_audit_events", "metadata")))
                    .from("security_audit_events")
                    .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                    .cond_where(
                        Expr::col(("security_audit_events", "event_type"))
                            .eq(Expr::val("medication_review_prompt.updated")),
                    )
                    .order_by_expr(
                        Expr::col(("security_audit_events", "id")),
                        sea_orm::Order::Desc,
                    )
                    .limit(1)
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
    let metadata: Value = event.try_get("", "metadata").unwrap();
    let malformed = app
        .client
        .patch(prompts(&app, &format!("/{prompt_id_91001}")))
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
    assert_eq!(
        updated_body["data"]["reviewed_by_membership_id"],
        fixture_membership_id.to_string()
    );
    assert_eq!(updated_body["data"]["practitioner_name"], "Dr Test");
    assert_eq!(metadata["prompt_id"], prompt_id_91001);
    assert_eq!(metadata["previous_status"], "needs_review");
    assert_eq!(metadata["status"], "reviewed_with_practitioner");
    assert_eq!(malformed_status, 400);
    assert_eq!(malformed_body["error"]["code"], "bad_request");
    assert_eq!(bad_id_status, 404);
}

#[tokio::test]
async fn review_prompt_update_replays_under_an_idempotency_key() {
    crate::reference_fixture::clean_review_catalogue().await;
    let app = Application::new().await;
    let token = app.token().await;
    let seeded_91001 = seed_prompt(&app, "needs_review").await;
    let prompt_id_91001 = seeded_91001.prompt.id;
    let shown = app
        .client
        .get(prompts(&app, &format!("/{prompt_id_91001}")))
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
    let lease = edit_token(&app, &token, prompt_id_91001).await;
    let first = app
        .client
        .patch(prompts(&app, &format!("/{prompt_id_91001}")))
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
        .patch(prompts(&app, &format!("/{prompt_id_91001}")))
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
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr_as(Expr::col(("security_audit_events", "id")).count(), "total")
                    .from("security_audit_events")
                    .and_where(Expr::col("household_id").eq(app.fixture.household.id))
                    .cond_where(
                        Expr::col(("security_audit_events", "event_type"))
                            .eq(Expr::val("medication_review_prompt.updated")),
                    )
                    .to_owned(),
            ),
        )
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
    crate::reference_fixture::clean_review_catalogue().await;
    let app = Application::new().await;
    let token = app.token().await;
    for _ in 0..22 {
        seed_prompt(&app, "needs_review").await;
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
    crate::reference_fixture::clean_review_catalogue().await;
    let app = Application::new().await;
    let fixture_adult_id = app.fixture.adult.id;
    let fixture_minor_id = app.fixture.minor.id;
    let fixture_grant_id = app.fixture.grant.id;
    let token = app.token().await;
    let seeded_91001 = seed_prompt(&app, "needs_review").await;
    let prompt_id_91001 = seeded_91001.prompt.id;
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("person_access_grants")
                        .value("access_level", Expr::val("view"))
                        .cond_where(
                            Expr::col(("person_access_grants", "id"))
                                .eq(Expr::val(fixture_grant_id)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
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
        .get(prompts(&app, &format!("/{prompt_id_91001}")))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let shown_status = shown.status().as_u16();
    let denied = app
        .client
        .patch(prompts(&app, &format!("/{prompt_id_91001}")))
        .bearer_auth(&token)
        .header("if-match", "\"anything\"")
        .json(&json!({"medication_review_prompt":{"review_note":"denied"}}))
        .send()
        .await
        .unwrap();
    let denied_status = denied.status().as_u16();
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_prompts")
                        .value("person_id", Expr::val(fixture_minor_id))
                        .cond_where(
                            Expr::col(("medication_review_prompts", "id"))
                                .eq(Expr::val(prompt_id_91001)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
    let other_person_lease = app
        .client
        .post(prompts(&app, &format!("/{prompt_id_91001}/lease")))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .status()
        .as_u16();
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("people")
                        .value("person_type", Expr::val(1_i32))
                        .value("has_capacity", Expr::val(false))
                        .value(
                            "date_of_birth",
                            Expr::val(
                                chrono::Utc::now()
                                    .date_naive()
                                    .checked_sub_months(chrono::Months::new(120))
                                    .unwrap(),
                            ),
                        )
                        .cond_where(Expr::col(("people", "id")).eq(Expr::val(fixture_adult_id)))
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
    let setup = rows::people::Entity::find_by_id(app.fixture.adult.id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert!(
        setup.person_type == 1
            && !setup.has_capacity
            && setup.date_of_birth.unwrap()
                > chrono::Utc::now()
                    .date_naive()
                    .checked_sub_months(chrono::Months::new(216))
                    .unwrap()
    );
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
    crate::reference_fixture::clean_review_catalogue().await;
    let app = Application::new().await;
    let mut worker_evidence = evidence(&app, "Old synthetic product")
        .await
        .into_active_model();
    worker_evidence.source_name = Set("openFDA / DailyMed SPL".into());
    worker_evidence.source_record_id = Set("synthetic-worker-label".into());
    worker_evidence.source_url = Set("https://example.test/old".into());
    worker_evidence.source_version = Set(Some("1".into()));
    worker_evidence.source_effective_on = Set(Some("2026-09-01".parse().unwrap()));
    worker_evidence.label_section = Set("Drug Interactions".into());
    worker_evidence.evidence_text = Set("Old warning".into());
    worker_evidence.retrieved_on = Set("2026-09-01".parse().unwrap());
    worker_evidence.risk_level = Set("high".into());
    worker_evidence.match_confidence = Set("high".into());
    worker_evidence.match_status = Set("unreviewed".into());
    worker_evidence.candidate_terms = Set(vec!["old".into()]);
    worker_evidence.pharmacologic_classes = Set(vec!["old".into()]);
    worker_evidence.interacting_terms = Set(vec!["old".into()]);
    let worker_evidence = worker_evidence.update(&app.fixture.admin).await.unwrap();
    let run_id = review_refresh_run(&app).await;
    import_snapshot(
        &app.context,
        run_id,
        &review_refresh_snapshot("2", "Changed synthetic warning"),
        NaiveDate::from_ymd_opt(2026, 10, 13).unwrap(),
    )
    .await
    .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col((
                        "medication_review_evidence_records",
                        "risk_level",
                    )))
                    .expr(Expr::col((
                        "medication_review_evidence_records",
                        "match_confidence",
                    )))
                    .expr(Expr::col((
                        "medication_review_evidence_records",
                        "match_status",
                    )))
                    .expr(Expr::col((
                        "medication_review_evidence_records",
                        "interacting_terms",
                    )))
                    .expr(Expr::col((
                        "medication_review_evidence_records",
                        "candidate_terms",
                    )))
                    .from("medication_review_evidence_records")
                    .cond_where(
                        Expr::col(("medication_review_evidence_records", "id"))
                            .eq(Expr::val(worker_evidence.id)),
                    )
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
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
    crate::reference_fixture::clean_review_catalogue().await;
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
    let row = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col((
                        "medication_review_evidence_refresh_runs",
                        "created_count",
                    )))
                    .expr(Expr::col((
                        "medication_review_evidence_refresh_runs",
                        "updated_count",
                    )))
                    .expr(Expr::col((
                        "medication_review_evidence_refresh_runs",
                        "unchanged_count",
                    )))
                    .expr(Expr::col((
                        "medication_review_evidence_refresh_runs",
                        "status",
                    )))
                    .from("medication_review_evidence_refresh_runs")
                    .cond_where(
                        Expr::col(("medication_review_evidence_refresh_runs", "id"))
                            .eq(Expr::val(run_id)),
                    )
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i32>("", "created_count").unwrap(), 1);
    assert_eq!(row.try_get::<i32>("", "updated_count").unwrap(), 0);
    assert_eq!(row.try_get::<i32>("", "unchanged_count").unwrap(), 0);
    assert_eq!(row.try_get::<i32>("", "status").unwrap(), 2);
    let count = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr_as(
                        Expr::col(("medication_review_evidence_records", "id")).count(),
                        "count",
                    )
                    .from("medication_review_evidence_records")
                    .cond_where(
                        Expr::col(("medication_review_evidence_records", "source_record_id"))
                            .eq(Expr::val("synthetic-worker-label")),
                    )
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "count")
        .unwrap();
    assert_eq!(count, 1);
    app.close().await;
}

#[tokio::test]
async fn review_refresh_retry_reopens_changed_prompt_after_evidence_commit() {
    crate::reference_fixture::clean_review_catalogue().await;
    let app = Application::new().await;
    let seeded_91251 = seed_prompt(&app, "reviewed").await;
    let prompt_id_91251 = seeded_91251.prompt.id;
    let evidence_id_91251 = seeded_91251.evidence.id;
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_prompts")
                        .value("practitioner_name", Expr::val("Previous clinician"))
                        .value("practitioner_role", Expr::val("Clinician"))
                        .value(
                            "reviewed_on",
                            Expr::val("2026-09-01".parse::<chrono::NaiveDate>().unwrap()),
                        )
                        .value("review_note", Expr::val("Prior decision"))
                        .cond_where(
                            Expr::col(("medication_review_prompts", "id"))
                                .eq(Expr::val(prompt_id_91251)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_evidence_records")
                        .value("source_name", Expr::val("openFDA / DailyMed SPL"))
                        .value("source_record_id", Expr::val("synthetic-worker-label"))
                        .value("source_version", Expr::val("1"))
                        .value(
                            "source_effective_on",
                            Expr::val("2026-09-01".parse::<chrono::NaiveDate>().unwrap()),
                        )
                        .value("evidence_text", Expr::val("Old warning"))
                        .cond_where(
                            Expr::col(("medication_review_evidence_records", "id"))
                                .eq(Expr::val(evidence_id_91251)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
    // SeaORM Schema cannot define PostgreSQL trigger functions. These fault cases require whole-run exclusivity.
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
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col((
                        "medication_review_evidence_records",
                        "source_version",
                    )))
                    .from("medication_review_evidence_records")
                    .cond_where(
                        Expr::col(("medication_review_evidence_records", "id"))
                            .eq(Expr::val(evidence_id_91251)),
                    )
                    .to_owned(),
            ),
        )
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
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("medication_review_prompts", "status")))
                    .from("medication_review_prompts")
                    .cond_where(
                        Expr::col(("medication_review_prompts", "id"))
                            .eq(Expr::val(prompt_id_91251)),
                    )
                    .to_owned(),
            ),
        )
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
    let result = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .columns([
                        ("r", "status"),
                        ("r", "updated_count"),
                        ("r", "unchanged_count"),
                    ])
                    .expr_as(Expr::col(("p", "status")), "prompt_status")
                    .columns([("p", "practitioner_name"), ("p", "evidence_text")])
                    .from_as(rows::medication_review_evidence_refresh_runs::Entity, "r")
                    .from_as(rows::medication_review_prompts::Entity, "p")
                    .and_where(Expr::col(("r", "id")).eq(run_id))
                    .and_where(Expr::col(("p", "id")).eq(prompt_id_91251))
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
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
    let histories = app
        .fixture
        .admin
        .query_all_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("medication_review_histories", "event_type")))
                    .expr(Expr::col(("medication_review_histories", "snapshot")))
                    .from("medication_review_histories")
                    .cond_where(
                        Expr::col(("medication_review_histories", "prompt_id"))
                            .eq(Expr::val(prompt_id_91251)),
                    )
                    .order_by_expr(
                        Expr::col(("medication_review_histories", "id")),
                        sea_orm::Order::Asc,
                    )
                    .to_owned(),
            ),
        )
        .await
        .unwrap();
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
    crate::reference_fixture::clean_review_catalogue().await;
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
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("medication_review_evidence_refresh_runs", "id")))
                    .from("medication_review_evidence_refresh_runs")
                    .order_by_expr(
                        Expr::col(("medication_review_evidence_refresh_runs", "id")),
                        sea_orm::Order::Desc,
                    )
                    .limit(1)
                    .to_owned(),
            ),
        )
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
            let row = app
                .fixture
                .admin
                .query_one_raw(
                    app.fixture.admin.get_database_backend().build(
                        &Query::select()
                            .expr(Expr::col((
                                "medication_review_evidence_refresh_runs",
                                "status",
                            )))
                            .from("medication_review_evidence_refresh_runs")
                            .cond_where(
                                Expr::col(("medication_review_evidence_refresh_runs", "id"))
                                    .eq(Expr::val(run_id)),
                            )
                            .to_owned(),
                    ),
                )
                .await
                .unwrap()
                .unwrap();
            if row.try_get::<i32>("", "status").unwrap() == 3 {
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
    async {
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::update()
                        .table("medication_review_evidence_refresh_runs")
                        .value("status", Expr::val(1_i32))
                        .value("error_message", Expr::val(Option::<String>::None))
                        .value(
                            "completed_at",
                            Expr::val(Option::<chrono::NaiveDateTime>::None),
                        )
                        .value("updated_at", Expr::val(chrono::Utc::now().naive_utc()))
                        .cond_where(
                            Expr::col(("medication_review_evidence_refresh_runs", "id"))
                                .eq(Expr::val(run_id)),
                        )
                        .to_owned(),
                ),
            )
            .await
    }
    .await
    .unwrap();
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
                .query_one_raw(
                    app.fixture.admin.get_database_backend().build(
                        &Query::select()
                            .expr(Expr::col(("pg_loco_queue", "status")))
                            .from("pg_loco_queue")
                            .cond_where(
                                Expr::col(("pg_loco_queue", "id")).eq(Expr::val(retry_job.clone())),
                            )
                            .to_owned(),
                    ),
                )
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
                .query_one_raw(
                    app.fixture.admin.get_database_backend().build(
                        &Query::select()
                            .expr(Expr::col(("pg_loco_queue", "status")))
                            .from("pg_loco_queue")
                            .cond_where(
                                Expr::col(("pg_loco_queue", "id"))
                                    .eq(Expr::val(duplicate_job.clone())),
                            )
                            .to_owned(),
                    ),
                )
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
    let row = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col((
                        "medication_review_evidence_refresh_runs",
                        "status",
                    )))
                    .expr(Expr::col((
                        "medication_review_evidence_refresh_runs",
                        "created_count",
                    )))
                    .expr(Expr::col((
                        "medication_review_evidence_refresh_runs",
                        "updated_count",
                    )))
                    .expr(Expr::col((
                        "medication_review_evidence_refresh_runs",
                        "unchanged_count",
                    )))
                    .from("medication_review_evidence_refresh_runs")
                    .cond_where(
                        Expr::col(("medication_review_evidence_refresh_runs", "id"))
                            .eq(Expr::val(run_id)),
                    )
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i32>("", "status").unwrap(), 2);
    assert_eq!(row.try_get::<i32>("", "created_count").unwrap(), 1);
    assert_eq!(row.try_get::<i32>("", "updated_count").unwrap(), 0);
    assert_eq!(row.try_get::<i32>("", "unchanged_count").unwrap(), 0);
    let row = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr_as(
                        Expr::col(("medication_review_evidence_records", "id")).count(),
                        "count",
                    )
                    .from("medication_review_evidence_records")
                    .cond_where(
                        Expr::col(("medication_review_evidence_records", "source_record_id"))
                            .eq(Expr::val("synthetic-worker-label")),
                    )
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    app.close().await;
    provider_server.abort();
}
