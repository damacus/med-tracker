use medtracker_contract_tests::{fixture, Fixture, Target};
use postgres::{Client, NoTls};
use reqwest::blocking::{Client as HttpClient, Response};
use reqwest::Method;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::env;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn rate_limited_response(response: Response) {
    assert_eq!(response.status().as_u16(), 429);
    for header in [
        "retry-after",
        "ratelimit-limit",
        "ratelimit-remaining",
        "ratelimit-reset",
    ] {
        assert!(response.headers().get(header).is_some(), "missing {header}");
    }
    assert_eq!(response.headers()["ratelimit-limit"], "300");
    assert_eq!(response.headers()["ratelimit-remaining"], "0");
    let value: Value = response.json().expect("rate limit JSON");
    assert_eq!(value["error"]["code"], "rate_limited");
}

fn etag(response: &Response) -> String {
    response.headers()["etag"]
        .to_str()
        .expect("ETag")
        .to_owned()
}

fn prompts_path(fixture: &Fixture) -> String {
    format!(
        "/api/v1/households/{}/medication_review_prompts",
        fixture.household_id
    )
}

fn database() -> Client {
    Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        NoTls,
    )
    .expect("contract database connection")
}

fn assert_prompt_schema(prompt: &Value) {
    let expected = BTreeSet::from([
        "id",
        "person_id",
        "primary_medication_id",
        "interacting_medication_id",
        "evidence_record_id",
        "primary_medication_name",
        "interacting_medication_name",
        "evidence_source_name",
        "evidence_source_url",
        "evidence_source_version",
        "matched_term",
        "match_type",
        "source_instruction",
        "match_reason",
        "evidence_text",
        "etag",
        "evidence_source_checked_on",
        "evidence_source_effective_on",
        "risk_level",
        "match_confidence",
        "status",
        "practitioner_name",
        "practitioner_role",
        "review_note",
        "reviewed_on",
        "reviewed_by_membership_id",
        "updated_at",
    ]);
    let actual = prompt
        .as_object()
        .expect("prompt object")
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);
    for field in [
        "id",
        "person_id",
        "primary_medication_id",
        "interacting_medication_id",
        "evidence_record_id",
    ] {
        let value = prompt[field].as_str().expect("decimal string identifier");
        assert!(!value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()));
        assert_ne!(value, "0");
    }
    assert!(prompt["etag"].is_string());
    assert!(prompt["evidence_source_checked_on"].is_string());
    assert!(prompt["evidence_source_effective_on"].is_string());
    assert!(prompt["updated_at"].is_string());
    time::OffsetDateTime::parse(
        prompt["updated_at"].as_str().unwrap(),
        &time::format_description::well_known::Rfc3339,
    )
    .expect("RFC 3339 updated_at");
    assert!(
        prompt["reviewed_by_membership_id"].is_null()
            || prompt["reviewed_by_membership_id"]
                .as_str()
                .is_some_and(|value| {
                    !value.is_empty()
                        && value.bytes().all(|byte| byte.is_ascii_digit())
                        && value != "0"
                })
    );
}

fn assert_collection_schema(collection: &Value) {
    let actual = collection
        .as_object()
        .expect("collection object")
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, BTreeSet::from(["data", "meta"]));
    let meta = collection["meta"].as_object().expect("pagination metadata");
    assert_eq!(
        meta.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        BTreeSet::from(["page", "per_page", "total_count"])
    );
    assert!(collection["meta"]["page"]
        .as_u64()
        .is_some_and(|value| value >= 1));
    assert!(collection["meta"]["per_page"]
        .as_u64()
        .is_some_and(|value| (1..=100).contains(&value)));
    assert!(collection["meta"]["total_count"].as_u64().is_some());
}

struct DisposableReviewEvidence {
    db: Client,
    evidence_ids: Vec<i64>,
    primary_name: String,
    interacting_name: String,
}

impl DisposableReviewEvidence {
    fn active_pair(fixture: &Fixture) -> Self {
        let mut db = database();
        let medication_names = db
            .query_one(
                "SELECT (SELECT name FROM medications WHERE id = $1), (SELECT name FROM medications WHERE id = $2)",
                &[&fixture.managed_medication_id, &fixture.forecast_medication_id],
            )
            .expect("active medication fixture names");
        let primary_name: String = medication_names.get(0);
        let interacting_name: String = medication_names.get(1);
        let active_count: i64 = db
            .query_one(
                "SELECT count(DISTINCT medication_id) FROM person_medications WHERE person_id = $1 AND medication_id = ANY($2) AND active = true AND retired_at IS NULL",
                &[
                    &fixture.managed_person_id,
                    &vec![fixture.managed_medication_id, fixture.forecast_medication_id],
                ],
            )
            .expect("active medication assignments")
            .get(0);
        assert_eq!(active_count, 2, "review sync needs two active medicines");
        Self {
            db,
            evidence_ids: Vec::new(),
            primary_name,
            interacting_name,
        }
    }

    fn new(fixture: &Fixture) -> Self {
        let mut evidence = Self::active_pair(fixture);
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
            .to_string();
        let candidate_terms = vec![evidence.primary_name.clone()];
        let interacting_terms = vec![evidence.interacting_name.clone()];
        let primary_name = evidence.primary_name.clone();
        evidence.insert(
            &format!("contract-openapi-review-{suffix}"),
            &primary_name,
            &candidate_terms,
            &interacting_terms,
            &[],
            "Stored curated contract evidence",
            "reviewed_pair",
        );
        evidence
    }

    fn insert(
        &mut self,
        source_record_id: &str,
        product_name: &str,
        candidate_terms: &[String],
        interacting_terms: &[String],
        pharmacologic_classes: &[String],
        evidence_text: &str,
        match_status: &str,
    ) -> i64 {
        let evidence_id: i64 = self
            .db
            .query_one(
                "INSERT INTO medication_review_evidence_records (source_name, source_record_id, source_url, retrieved_on, product_name, label_section, evidence_text, risk_level, match_confidence, match_status, candidate_terms, interacting_terms, pharmacologic_classes, created_at, updated_at) VALUES ('Contract curated source', $1, 'https://example.test/review-contract', '2026-02-25', $2, 'warnings', $3, 'high', 'high', $4, $5, $6, $7, now(), now()) RETURNING id",
                &[&source_record_id, &product_name, &evidence_text, &match_status, &candidate_terms, &interacting_terms, &pharmacologic_classes],
            )
            .expect("curated evidence record")
            .get(0);
        self.evidence_ids.push(evidence_id);
        evidence_id
    }

    fn prompt_count(&mut self, evidence_id: i64) -> i64 {
        self.db
            .query_one(
                "SELECT count(*) FROM medication_review_prompts WHERE evidence_record_id = $1",
                &[&evidence_id],
            )
            .expect("synced prompt count")
            .get(0)
    }
}

impl Drop for DisposableReviewEvidence {
    fn drop(&mut self) {
        for evidence_id in &self.evidence_ids {
            self.db
                .execute(
                    "DELETE FROM medication_review_prompts WHERE evidence_record_id = $1",
                    &[evidence_id],
                )
                .expect("remove generated review prompts");
            self.db
                .execute(
                    "DELETE FROM medication_review_evidence_records WHERE id = $1",
                    &[evidence_id],
                )
                .expect("remove curated review evidence");
        }
    }
}

fn domain_audits(fixture: &Fixture, prompt_id: i64) -> Vec<Value> {
    database()
        .query(
            "SELECT metadata::text FROM security_audit_events WHERE household_id = $1 AND event_type = 'medication_review_prompt.updated' AND metadata ->> 'prompt_id' = $2 ORDER BY id",
            &[&fixture.household_id, &prompt_id.to_string()],
        )
        .expect("review prompt audit events")
        .into_iter()
        .map(|row| serde_json::from_str(&row.get::<_, String>(0)).expect("audit metadata JSON"))
        .collect()
}

fn patch_with_idempotency_key(
    path: &str,
    token: &str,
    version: &str,
    key: &str,
    payload: &Value,
) -> Response {
    let base = env::var("CONTRACT_BASE_URL").expect("contract API URL");
    assert!(base.starts_with("http://127.0.0.1:") || base.starts_with("http://localhost:"));
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .expect("contract HTTP client");
    client
        .patch(format!("{}{}", base.trim_end_matches('/'), path))
        .bearer_auth(token)
        .header("Accept", "application/json")
        .header("If-Match", version)
        .header("Idempotency-Key", key)
        .json(payload)
        .send()
        .expect("contract API response")
}

struct ViewOnlyGrantGuard {
    db: Client,
    grant_id: i64,
    previous_level: String,
    restored: bool,
}

impl ViewOnlyGrantGuard {
    fn demote_owner(fixture: &Fixture) -> Self {
        let mut db = database();
        let grant = db
            .query_one(
                "SELECT id, access_level FROM person_access_grants WHERE household_membership_id = $1 AND person_id = $2 AND revoked_at IS NULL",
                &[&fixture.owner_membership_id, &fixture.managed_person_id],
            )
            .expect("owner manage grant for review person");
        let grant_id: i64 = grant.get(0);
        let previous_level: String = grant.get(1);
        assert_eq!(previous_level, "manage");
        db.execute(
            "UPDATE person_access_grants SET access_level = 'view', updated_at = now() WHERE id = $1",
            &[&grant_id],
        )
        .expect("temporarily downgrade review grant");
        Self {
            db,
            grant_id,
            previous_level,
            restored: false,
        }
    }

    fn restore(&mut self) {
        if !self.restored {
            self.db
                .execute(
                    "UPDATE person_access_grants SET access_level = $2, updated_at = now() WHERE id = $1",
                    &[&self.grant_id, &self.previous_level],
                )
                .expect("restore review grant");
            self.restored = true;
        }
    }
}

impl Drop for ViewOnlyGrantGuard {
    fn drop(&mut self) {
        self.restore();
    }
}

#[test]
fn list_medication_review_prompts_returns_documented_collection() {
    let target = Target::from_env();
    let fixture = fixture();
    let response = target.get(&prompts_path(&fixture), Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert!(response.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("no-store"));
    let collection = body(response);
    assert!(collection["data"].is_array());
    assert_collection_schema(&collection);
    assert!(!collection["data"].as_array().unwrap().is_empty());
    for prompt in collection["data"].as_array().unwrap() {
        assert_prompt_schema(prompt);
    }
}

#[test]
fn get_medication_review_prompt_returns_snapshot_and_version() {
    let target = Target::from_env();
    let fixture = fixture();
    let url = format!(
        "{}/{}",
        prompts_path(&fixture),
        fixture.managed_review_prompt_id
    );
    let response = target.get(&url, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert!(response.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("no-store"));
    let version = etag(&response);
    let prompt = body(response)["data"].clone();
    assert_prompt_schema(&prompt);
    assert_eq!(prompt["id"], fixture.managed_review_prompt_id.to_string());
    assert_eq!(prompt["person_id"], fixture.managed_person_id.to_string());
    assert_eq!(prompt["etag"], version);
    assert!(prompt["evidence_text"].is_string());
}

#[test]
fn get_prompt_auth_and_non_disclosure_boundaries_are_enforced() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = prompts_path(&fixture);
    assert_eq!(
        target
            .get(
                &format!("{base}/{}", fixture.managed_review_prompt_id),
                None
            )
            .status()
            .as_u16(),
        401
    );
    assert_eq!(
        target
            .get(
                &format!("{base}/not-a-numeric-id"),
                Some(&fixture.view_access_token)
            )
            .status()
            .as_u16(),
        404
    );
    for prompt_id in [
        fixture.hidden_review_prompt_id,
        fixture.foreign_review_prompt_id,
    ] {
        let response = target.get(
            &format!("{base}/{prompt_id}"),
            Some(&fixture.view_access_token),
        );
        assert_eq!(response.status().as_u16(), 404);
    }
}

#[test]
fn listing_detects_ingredient_and_class_matches_but_excludes_no_action() {
    let target = Target::from_env();
    let fixture = fixture();
    let mut evidence = DisposableReviewEvidence::active_pair(&fixture);
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos()
        .to_string();
    let primary_terms = vec![evidence.primary_name.clone()];
    let interacting_terms = vec![evidence.interacting_name.clone()];
    let primary_name = evidence.primary_name.clone();
    let interacting_name = evidence.interacting_name.clone();
    let class_name = "contract review interaction class".to_owned();
    let class_terms = vec![class_name.clone()];
    let ingredient_id = evidence.insert(
        &format!("contract-review-ingredient-{suffix}"),
        &primary_name,
        &primary_terms,
        &[],
        &[],
        &format!("Avoid combining with {interacting_name}."),
        "unreviewed",
    );
    let class_id = evidence.insert(
        &format!("contract-review-class-{suffix}"),
        &primary_name,
        &primary_terms,
        &[],
        &[],
        &format!("Avoid {class_name}.").to_lowercase(),
        "unreviewed",
    );
    let class_identity_id = evidence.insert(
        &format!("contract-review-class-identity-{suffix}"),
        &interacting_name,
        &interacting_terms,
        &[],
        &class_terms,
        &format!(
            "No clinically meaningful interaction with {}.",
            primary_name
        ),
        "unreviewed",
    );
    let no_action_id = evidence.insert(
        &format!("contract-review-no-action-{suffix}"),
        &primary_name,
        &primary_terms,
        &[],
        &[],
        &format!(
            "No clinically meaningful interaction with {}.",
            interacting_name
        ),
        "unreviewed",
    );
    let mixed_text = format!(
        "No clinically meaningful interaction with {interacting_name}. However, avoid combining with {interacting_name}."
    );
    let mixed_id = evidence.insert(
        &format!("contract-review-mixed-{suffix}"),
        &primary_name,
        &primary_terms,
        &[],
        &[],
        &mixed_text,
        "unreviewed",
    );
    let unavoidable_text = format!("Unavoidable use with {interacting_name}.");
    let unavoidable_id = evidence.insert(
        &format!("contract-review-unavoidable-{suffix}"),
        &primary_name,
        &primary_terms,
        &[],
        &[],
        &unavoidable_text,
        "unreviewed",
    );
    let cancer_text = format!("Cancer therapy mentions {interacting_name}.");
    let cancer_id = evidence.insert(
        &format!("contract-review-cancer-{suffix}"),
        &primary_name,
        &primary_terms,
        &[],
        &[],
        &cancer_text,
        "unreviewed",
    );

    let collection = body(target.get(&prompts_path(&fixture), Some(&fixture.access_token)));
    let prompts = collection["data"].as_array().unwrap();
    let by_evidence = |evidence_id: i64| {
        prompts
            .iter()
            .find(|prompt| prompt["evidence_record_id"] == evidence_id.to_string())
    };
    assert_eq!(
        by_evidence(ingredient_id).unwrap()["match_type"],
        "ingredient"
    );
    assert_eq!(by_evidence(class_id).unwrap()["match_type"], "class");
    assert!(by_evidence(class_identity_id).is_none());
    assert!(by_evidence(no_action_id).is_none());
    assert_eq!(
        by_evidence(mixed_id).unwrap()["source_instruction"],
        "avoid"
    );
    assert_eq!(by_evidence(mixed_id).unwrap()["risk_level"], "high");
    assert_eq!(by_evidence(mixed_id).unwrap()["evidence_text"], mixed_text);
    assert_eq!(
        by_evidence(unavoidable_id).unwrap()["source_instruction"],
        "unclassified"
    );
    assert_eq!(
        by_evidence(unavoidable_id).unwrap()["evidence_text"],
        unavoidable_text
    );
    assert_eq!(
        by_evidence(cancer_id).unwrap()["source_instruction"],
        "unclassified"
    );
    assert_eq!(
        by_evidence(cancer_id).unwrap()["evidence_text"],
        cancer_text
    );
}

#[test]
fn all_four_review_prompt_operations_use_the_shared_rate_limit() {
    let fixture = fixture();
    let base = env::var("CONTRACT_RATE_BASE_URL").expect("non-loopback API base URL");
    let client = HttpClient::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("rate limit client");
    let mut limited = false;
    for _ in 0..601 {
        let response = client
            .get(format!("{base}/api/v1/capabilities"))
            .bearer_auth(&fixture.access_token)
            .send()
            .expect("shared rate limit request");
        if response.status().as_u16() == 429 {
            rate_limited_response(response);
            limited = true;
            break;
        }
        assert_eq!(response.status().as_u16(), 200);
    }
    assert!(limited, "expected shared rate limit within 601 requests");

    let collection = prompts_path(&fixture);
    let detail = format!("{collection}/{}", fixture.managed_review_prompt_id);
    let update = format!("{collection}/{}", fixture.edit_review_prompt_id);
    for (method, path, payload) in [
        (Method::GET, collection, None),
        (Method::GET, detail, None),
        (
            Method::PATCH,
            update.clone(),
            Some(json!({"medication_review_prompt": {"review_note": "rate limited"}})),
        ),
        (
            Method::PUT,
            update,
            Some(json!({"medication_review_prompt": {"status": "not_relevant"}})),
        ),
    ] {
        let mut request = client
            .request(method, format!("{}{}", base.trim_end_matches('/'), path))
            .bearer_auth(&fixture.access_token);
        if let Some(payload) = payload {
            request = request.json(&payload).header("If-Match", "stale");
        }
        rate_limited_response(request.send().expect("review prompt rate limit response"));
    }
}

#[test]
fn listing_generates_curated_snapshot_once_and_keeps_it_immutable() {
    let target = Target::from_env();
    let fixture = fixture();
    let mut evidence = DisposableReviewEvidence::new(&fixture);
    let evidence_id_value: i64 = evidence.evidence_ids[0];
    assert_eq!(evidence.prompt_count(evidence_id_value), 0);
    let url = prompts_path(&fixture);

    let first = body(target.get(&url, Some(&fixture.access_token)));
    let evidence_id = evidence_id_value.to_string();
    let generated = first["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|prompt| prompt["evidence_record_id"] == evidence_id)
        .expect("on-demand curated prompt");
    assert_prompt_schema(generated);
    assert_eq!(
        generated["evidence_text"],
        "Stored curated contract evidence"
    );
    let generated = generated.clone();
    assert_eq!(evidence.prompt_count(evidence_id_value), 1);

    let second = body(target.get(&url, Some(&fixture.access_token)));
    let repeated = second["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|prompt| prompt["evidence_record_id"] == evidence_id)
        .expect("same prompt on repeat sync");
    assert_eq!(repeated, &generated);
    assert_eq!(evidence.prompt_count(evidence_id_value), 1);

    evidence
        .db
        .execute(
            "UPDATE medication_review_evidence_records SET evidence_text = 'Changed source evidence', source_version = 'changed' WHERE id = $1",
            &[&evidence_id_value],
        )
        .expect("change source after snapshot capture");
    let after_source_change = body(target.get(&url, Some(&fixture.access_token)));
    let retained = after_source_change["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|prompt| prompt["evidence_record_id"] == evidence_id)
        .expect("retained review snapshot");
    assert_eq!(retained["evidence_text"], generated["evidence_text"]);
    assert_eq!(retained["etag"], generated["etag"]);
    assert_eq!(evidence.prompt_count(evidence_id_value), 1);
}

#[test]
fn list_filters_and_view_scope_follow_the_documented_contract() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = prompts_path(&fixture);
    let view = body(target.get(&base, Some(&fixture.view_access_token)));
    assert!(!view["data"].as_array().unwrap().is_empty());
    assert!(view["data"]
        .as_array()
        .unwrap()
        .iter()
        .all(|prompt| { prompt["person_id"] == fixture.managed_person_id.to_string() }));
    let hidden = body(target.get(
        &format!("{base}?show_hidden=1&review_status=all"),
        Some(&fixture.access_token),
    ));
    assert!(hidden["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|prompt| { prompt["id"] == fixture.low_signal_review_prompt_id.to_string() }));
    let priority = body(target.get(
        &format!("{base}?priority=discuss_soon"),
        Some(&fixture.access_token),
    ));
    assert!(priority["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|prompt| { prompt["id"] == fixture.managed_review_prompt_id.to_string() }));
    let second_page = body(target.get(
        &format!("{base}?page=2&per_page=1"),
        Some(&fixture.access_token),
    ));
    assert_eq!(second_page["meta"]["page"], 2);
    assert_eq!(second_page["meta"]["per_page"], 1);
    assert_eq!(second_page["data"].as_array().unwrap().len(), 1);
    let reviewed = body(target.get(
        &format!("{base}?review_status=reviewed"),
        Some(&fixture.access_token),
    ));
    assert!(!reviewed["data"].as_array().unwrap().is_empty());
    assert!(reviewed["data"].as_array().unwrap().iter().all(|prompt| {
        prompt["status"] != "needs_review" && prompt["status"] != "hidden_low_signal"
    }));
    let invalid_status = target.get(
        &format!("{base}?review_status=unknown"),
        Some(&fixture.access_token),
    );
    assert_eq!(invalid_status.status().as_u16(), 422);
    let invalid_priority = target.get(
        &format!("{base}?priority=unknown"),
        Some(&fixture.access_token),
    );
    assert_eq!(invalid_priority.status().as_u16(), 422);
    let invalid_hidden = target.get(
        &format!("{base}?show_hidden=invalid"),
        Some(&fixture.access_token),
    );
    assert_eq!(invalid_hidden.status().as_u16(), 422);
    assert_eq!(target.get(&base, None).status().as_u16(), 401);
}

#[test]
fn patch_and_put_medication_review_prompt_require_and_return_versions() {
    let target = Target::from_env();
    let fixture = fixture();
    let url = format!(
        "{}/{}",
        prompts_path(&fixture),
        fixture.edit_review_prompt_id
    );

    let before = target.get(&url, Some(&fixture.access_token));
    assert_eq!(before.status().as_u16(), 200);
    let before_tag = etag(&before);
    let before_data = body(before)["data"].clone();
    assert_prompt_schema(&before_data);
    let audits_before_rejections = domain_audits(&fixture, fixture.edit_review_prompt_id);

    let missing_wrapper = target.patch_json(&url, &fixture.access_token, &json!({}));
    assert_eq!(missing_wrapper.status().as_u16(), 400);

    let missing_tag = target.patch_json(
        &url,
        &fixture.access_token,
        &json!({"medication_review_prompt": {"status": "not_relevant"}}),
    );
    assert_eq!(missing_tag.status().as_u16(), 428);
    let stale = target.patch_json_if_match(
        &url,
        &fixture.access_token,
        &json!({"medication_review_prompt": {"status": "not_relevant"}}),
        "stale-version",
    );
    assert_eq!(stale.status().as_u16(), 409);
    let required_fields = target.patch_json_if_match(
        &url,
        &fixture.access_token,
        &json!({"medication_review_prompt": {"status": "reviewed_with_practitioner"}}),
        &before_tag,
    );
    assert_eq!(required_fields.status().as_u16(), 422);
    let blank_practitioner = target.patch_json_if_match(
        &url,
        &fixture.access_token,
        &json!({"medication_review_prompt": {
            "status": "reviewed_with_practitioner",
            "practitioner_name": "",
            "practitioner_role": "GP",
            "reviewed_on": "2026-09-28"
        }}),
        &before_tag,
    );
    assert_eq!(blank_practitioner.status().as_u16(), 422);
    let unknown_attribute = target.patch_json_if_match(
        &url,
        &fixture.access_token,
        &json!({"medication_review_prompt": {"status": "not_relevant", "evidence_text": "forged"}}),
        &before_tag,
    );
    assert_eq!(unknown_attribute.status().as_u16(), 422);
    let unknown_root_attribute = target.patch_json_if_match(
        &url,
        &fixture.access_token,
        &json!({"medication_review_prompt": {"status": "not_relevant"}, "unknown": true}),
        &before_tag,
    );
    assert_eq!(unknown_root_attribute.status().as_u16(), 422);
    let null_reviewed_on = target.patch_json_if_match(
        &url,
        &fixture.access_token,
        &json!({"medication_review_prompt": {
            "status": "reviewed_with_practitioner",
            "practitioner_name": "Dr Contract Reviewer",
            "practitioner_role": "GP",
            "reviewed_on": null
        }}),
        &before_tag,
    );
    assert_eq!(null_reviewed_on.status().as_u16(), 422);
    let view_only = target.patch_json_if_match(
        &url,
        &fixture.view_access_token,
        &json!({"medication_review_prompt": {"status": "not_relevant"}}),
        &before_tag,
    );
    assert_eq!(view_only.status().as_u16(), 403);
    let unauthenticated = target.patch_json_without_auth(
        &url,
        &json!({"medication_review_prompt": {"status": "not_relevant"}}),
    );
    assert_eq!(unauthenticated.status().as_u16(), 401);

    let missing_put_tag = target.put_json(
        &url,
        &fixture.access_token,
        &json!({"medication_review_prompt": {"status": "not_relevant"}}),
    );
    assert_eq!(missing_put_tag.status().as_u16(), 428);
    let stale_put_tag = target.put_json_if_match(
        &url,
        &fixture.access_token,
        &json!({"medication_review_prompt": {"status": "not_relevant"}}),
        "stale-version",
    );
    assert_eq!(stale_put_tag.status().as_u16(), 409);
    let missing_put_fields = target.put_json_if_match(
        &url,
        &fixture.access_token,
        &json!({"medication_review_prompt": {"status": "expected_prescribed_combination"}}),
        &before_tag,
    );
    assert_eq!(missing_put_fields.status().as_u16(), 422);
    let invalid_put_status = target.put_json_if_match(
        &url,
        &fixture.access_token,
        &json!({"medication_review_prompt": {"status": "unknown_status"}}),
        &before_tag,
    );
    assert_eq!(invalid_put_status.status().as_u16(), 422);
    let forged_put_field = target.put_json_if_match(
        &url,
        &fixture.access_token,
        &json!({"medication_review_prompt": {"status": "not_relevant", "match_reason": "forged"}}),
        &before_tag,
    );
    assert_eq!(forged_put_field.status().as_u16(), 422);
    let view_only_put = target.put_json_if_match(
        &url,
        &fixture.view_access_token,
        &json!({"medication_review_prompt": {"status": "not_relevant"}}),
        &before_tag,
    );
    assert_eq!(view_only_put.status().as_u16(), 403);
    let unauthenticated_put = target.put_json_without_auth(
        &url,
        &json!({"medication_review_prompt": {"status": "not_relevant"}}),
    );
    assert_eq!(unauthenticated_put.status().as_u16(), 401);
    assert_eq!(
        domain_audits(&fixture, fixture.edit_review_prompt_id),
        audits_before_rejections
    );
    let unchanged = body(target.get(&url, Some(&fixture.access_token)))["data"].clone();
    assert_eq!(unchanged, before_data);
    assert_eq!(target.get(&url, None).status().as_u16(), 401);

    let patch = target.patch_json_if_match(
        &url,
        &fixture.access_token,
        &json!({"medication_review_prompt": {
            "status": "reviewed_with_practitioner",
            "practitioner_name": "Dr Contract Reviewer",
            "practitioner_role": "GP",
            "reviewed_on": "2026-09-28",
            "review_note": "Reviewed by contract test"
        }}),
        &before_tag,
    );
    assert_eq!(patch.status().as_u16(), 200);
    assert!(patch.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("no-store"));
    let patch_tag = etag(&patch);
    let patched = body(patch)["data"].clone();
    assert_prompt_schema(&patched);
    assert_eq!(patched["etag"], patch_tag);
    assert_eq!(patched["status"], "reviewed_with_practitioner");
    assert_eq!(patched["review_note"], "Reviewed by contract test");
    assert_eq!(patched["evidence_text"], before_data["evidence_text"]);

    let put = target.put_json_if_match(
        &url,
        &fixture.access_token,
        &json!({"medication_review_prompt": {"status": "not_relevant"}}),
        &patch_tag,
    );
    assert_eq!(put.status().as_u16(), 200);
    assert!(put.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("no-store"));
    let put_tag = etag(&put);
    let updated = body(put)["data"].clone();
    assert_prompt_schema(&updated);
    assert_eq!(updated["etag"], put_tag);
    assert_eq!(updated["status"], "not_relevant");
    assert_eq!(updated["evidence_text"], before_data["evidence_text"]);
    assert_eq!(updated["review_note"], patched["review_note"]);
    assert_eq!(
        updated["reviewed_by_membership_id"],
        fixture.owner_membership_id.to_string()
    );

    let audit = domain_audits(&fixture, fixture.edit_review_prompt_id);
    assert_eq!(audit.len(), audits_before_rejections.len() + 2);
    assert_eq!(
        audit[audits_before_rejections.len()]["previous_status"],
        "needs_review"
    );
    assert_eq!(
        audit[audits_before_rejections.len()]["status"],
        "reviewed_with_practitioner"
    );
    assert_eq!(
        audit[audits_before_rejections.len() + 1]["previous_status"],
        "reviewed_with_practitioner"
    );
    assert_eq!(
        audit[audits_before_rejections.len() + 1]["status"],
        "not_relevant"
    );
    for event in audit.iter().skip(audits_before_rejections.len()) {
        assert_eq!(event.as_object().unwrap().len(), 4);
        assert_eq!(event["prompt_id"], fixture.edit_review_prompt_id);
        assert_eq!(event["person_id"], fixture.managed_person_id);
        assert!(!event.to_string().contains("Reviewed by contract test"));
        assert!(!event.to_string().contains("Dr Contract Reviewer"));
    }
}

#[test]
fn simultaneous_updates_with_one_etag_commit_once() {
    let fixture = fixture();
    let url = format!(
        "{}/{}",
        prompts_path(&fixture),
        fixture.invalid_review_prompt_id
    );
    let before = Target::from_env().get(&url, Some(&fixture.access_token));
    assert_eq!(before.status().as_u16(), 200);
    let version = etag(&before);
    let audits_before = domain_audits(&fixture, fixture.invalid_review_prompt_id).len();
    let barrier = Arc::new(Barrier::new(3));
    let mut workers = Vec::new();
    for _ in 0..2 {
        let barrier = Arc::clone(&barrier);
        let url = url.clone();
        let token = fixture.access_token.clone();
        let version = version.clone();
        workers.push(thread::spawn(move || {
            let target = Target::from_env();
            barrier.wait();
            target.patch_json_if_match(
                &url,
                &token,
                &json!({"medication_review_prompt": {"status": "not_relevant", "review_note": "Concurrent review update"}}),
                &version,
            )
        }));
    }
    barrier.wait();
    let statuses = workers
        .into_iter()
        .map(|worker| worker.join().expect("update worker").status().as_u16())
        .collect::<BTreeSet<_>>();
    assert_eq!(statuses, BTreeSet::from([200, 409]));
    assert_eq!(
        domain_audits(&fixture, fixture.invalid_review_prompt_id).len(),
        audits_before + 1
    );
    let updated = body(Target::from_env().get(&url, Some(&fixture.access_token)))["data"].clone();
    assert_eq!(updated["status"], "not_relevant");
    assert_eq!(updated["review_note"], "Concurrent review update");
}

#[test]
fn idempotent_update_rechecks_current_manage_authority_before_replay() {
    let fixture = fixture();
    let prompt_id = fixture.second_review_prompt_id;
    let url = format!("{}/{}", prompts_path(&fixture), prompt_id);
    let before = Target::from_env().get(&url, Some(&fixture.access_token));
    assert_eq!(before.status().as_u16(), 200);
    let version = etag(&before);
    let key = format!(
        "review-prompt-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let payload = json!({"medication_review_prompt": {"status": "not_relevant", "review_note": "Idempotent review"}});
    let audits_before = domain_audits(&fixture, prompt_id).len();

    let first = patch_with_idempotency_key(&url, &fixture.access_token, &version, &key, &payload);
    assert_eq!(first.status().as_u16(), 200);
    assert!(first.headers().get("idempotency-replayed").is_none());
    let first_body = body(first);
    assert_eq!(first_body["data"]["review_note"], "Idempotent review");

    let replay = patch_with_idempotency_key(&url, &fixture.access_token, &version, &key, &payload);
    assert_eq!(replay.status().as_u16(), 200);
    assert_eq!(replay.headers()["idempotency-replayed"], "true");
    assert_eq!(body(replay), first_body);
    assert_eq!(domain_audits(&fixture, prompt_id).len(), audits_before + 1);

    let mut grant = ViewOnlyGrantGuard::demote_owner(&fixture);
    let denied_replay =
        patch_with_idempotency_key(&url, &fixture.access_token, &version, &key, &payload);
    assert_eq!(denied_replay.status().as_u16(), 403);
    grant.restore();
    assert_eq!(domain_audits(&fixture, prompt_id).len(), audits_before + 1);
}
