use lopdf::Document;
use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::Value;
use std::env;
use std::fs::{remove_file, write};
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use url::Url;

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn report_path(fixture: &Fixture, kind: &str) -> String {
    format!("/api/v1/households/{}/reports/{kind}", fixture.household_id)
}

fn assert_keys(value: &Value, required: &[&str], allowed: &[&str]) {
    let object = value.as_object().expect("JSON object");
    for key in required {
        assert!(object.contains_key(*key), "missing key {key}");
    }
    for key in object.keys() {
        assert!(allowed.contains(&key.as_str()), "unexpected key {key}");
    }
}

fn assert_timestamp(value: &Value) {
    OffsetDateTime::parse(value.as_str().expect("date-time string"), &Rfc3339)
        .expect("RFC3339 timestamp");
}

fn assert_date(value: &Value) {
    let date = value.as_str().expect("date string");
    assert_eq!(date.len(), 10);
    assert_eq!(date.as_bytes()[4], b'-');
    assert_eq!(date.as_bytes()[7], b'-');
    assert!(date
        .bytes()
        .enumerate()
        .all(|(index, byte)| [4, 7].contains(&index) || byte.is_ascii_digit()));
}

fn assert_report_person(person: &Value) {
    assert_keys(person, &["id", "name"], &["id", "name", "date_of_birth"]);
    assert!(person["id"]
        .as_str()
        .is_some_and(|id| id.parse::<i64>().is_ok_and(|value| value > 0)));
    assert!(person["name"].as_str().is_some());
    if let Some(date_of_birth) = person.get("date_of_birth") {
        if !date_of_birth.is_null() {
            assert_date(date_of_birth);
        }
    }
}

fn assert_error(response: Response, status: u16, code: &str) {
    assert_eq!(response.status().as_u16(), status);
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned();
    let value = body(response);
    assert_keys(&value, &["error"], &["error"]);
    let error = &value["error"];
    assert_keys(
        error,
        &["code", "message", "request_id"],
        &["code", "message", "request_id", "errors"],
    );
    assert_eq!(error["code"], code);
    assert!(error["message"]
        .as_str()
        .is_some_and(|message| !message.is_empty()));
    assert_eq!(error["request_id"], request_id);
    assert!(value.get("data").is_none());
}

fn pdf_text(response: Response) -> String {
    assert_eq!(response.status().as_u16(), 200);
    assert!(response.headers()["cache-control"]
        .to_str()
        .expect("no-store header")
        .contains("no-store"));
    assert_eq!(response.headers()["content-type"], "application/pdf");
    assert!(response.headers()["content-disposition"]
        .to_str()
        .expect("attachment header")
        .contains("attachment"));
    assert!(response.headers()["x-request-id"].to_str().is_ok());
    let bytes = response.bytes().expect("PDF bytes");
    let document = Document::load_mem(&bytes).expect("valid PDF document");
    let page_numbers: Vec<u32> = document.get_pages().keys().copied().collect();
    assert!(!page_numbers.is_empty(), "PDF contains pages");
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let prefix = env::temp_dir().join(format!("medtracker-report-{}-{suffix}", std::process::id()));
    let pdf_path = prefix.with_extension("pdf");
    let image_path = prefix.with_extension("png");
    let _cleanup = TempFiles(vec![pdf_path.clone(), image_path.clone()]);
    write(&pdf_path, &bytes).expect("write temporary PDF");
    let text_output = Command::new("pdftotext")
        .arg(&pdf_path)
        .arg("-")
        .output()
        .expect("start Poppler text extractor");
    assert!(
        text_output.status.success(),
        "Poppler text extraction failed: {}",
        String::from_utf8_lossy(&text_output.stderr)
    );
    let render_output = Command::new("pdftoppm")
        .args([
            "-f",
            "1",
            "-l",
            "1",
            "-singlefile",
            "-scale-to",
            "800",
            "-png",
        ])
        .arg(&pdf_path)
        .arg(prefix.to_str().expect("temporary image prefix"))
        .output()
        .expect("start Poppler PDF renderer");
    assert!(
        render_output.status.success(),
        "Poppler PDF rendering failed: {}",
        String::from_utf8_lossy(&render_output.stderr)
    );
    let image = std::fs::read(&image_path).expect("rendered first-page PNG");
    assert!(image.len() > 8);
    assert_eq!(&image[..8], b"\x89PNG\r\n\x1a\n");
    let text = String::from_utf8(text_output.stdout).expect("Poppler text output is UTF-8");
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

struct TempFiles(Vec<PathBuf>);

impl Drop for TempFiles {
    fn drop(&mut self) {
        for path in &self.0 {
            let _ = remove_file(path);
        }
    }
}

fn report_failure_client() -> (reqwest::blocking::Client, String) {
    let base_url = env::var("CONTRACT_REPORT_FAILURE_BASE_URL")
        .expect("isolated PDF renderer failure API base URL");
    let parsed = Url::parse(&base_url).expect("renderer failure base URL");
    assert_eq!(parsed.scheme(), "http");
    assert_eq!(parsed.host_str(), Some("rust-api-report-fail"));
    assert_eq!(parsed.port(), Some(39996));
    assert!(parsed.username().is_empty() && parsed.password().is_none());
    assert_eq!(parsed.path(), "/");
    assert!(parsed.query().is_none() && parsed.fragment().is_none());
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .expect("renderer failure client");
    (client, base_url.trim_end_matches('/').to_owned())
}

#[test]
fn json_report_responses_match_the_closed_openapi_schemas() {
    let target = Target::from_env();
    let fixture = fixture();

    let health_path = report_path(&fixture, "health_history");
    let health_query = format!(
        "{health_path}?person_id={}&start_date=2026-02-19&end_date=2026-02-26&include_medication_takes=1",
        fixture.managed_person_id
    );
    let health_response = target.get(&health_query, Some(&fixture.access_token));
    assert_eq!(health_response.status().as_u16(), 200);
    assert!(health_response.headers()["cache-control"]
        .to_str()
        .expect("no-store header")
        .contains("no-store"));
    let health = body(health_response);
    assert_keys(&health, &["data"], &["data"]);
    let health_data = &health["data"];
    let health_fields = [
        "person",
        "start_date",
        "end_date",
        "generated_at",
        "current_medicines",
        "chronology",
        "medication_takes",
    ];
    assert_keys(health_data, &health_fields, &health_fields);
    assert_report_person(&health_data["person"]);
    assert_date(&health_data["start_date"]);
    assert_date(&health_data["end_date"]);
    assert_timestamp(&health_data["generated_at"]);
    let medicines = health_data["current_medicines"]
        .as_array()
        .expect("current medicines");
    for medicine in medicines {
        assert_keys(medicine, &["id", "name"], &["id", "name"]);
        assert!(medicine["id"]
            .as_str()
            .is_some_and(|id| id.parse::<i64>().is_ok_and(|value| value > 0)));
        assert!(medicine["name"].as_str().is_some());
    }
    let events = health_data["chronology"].as_array().expect("chronology");
    assert!(
        !events.is_empty(),
        "seeded health events exercise the report schema"
    );
    let event_fields = [
        "id",
        "event_kind",
        "title",
        "started_on",
        "ended_on",
        "ongoing",
        "duration_days",
        "severity",
        "notes",
        "action_taken",
        "medical_help_sought",
        "medication_names",
    ];
    for event in events {
        assert_keys(event, &event_fields, &event_fields);
        assert!(event["id"]
            .as_str()
            .is_some_and(|id| id.parse::<i64>().is_ok_and(|value| value > 0)));
        assert!(matches!(
            event["event_kind"].as_str(),
            Some("illness" | "suspected_side_effect")
        ));
        assert_date(&event["started_on"]);
        if !event["ended_on"].is_null() {
            assert_date(&event["ended_on"]);
        }
        assert!(event["ongoing"].is_boolean());
        assert!(event["duration_days"].is_null() || event["duration_days"].as_i64().is_some());
        assert!(event["duration_days"].is_null() || event["duration_days"].as_i64().unwrap() >= 1);
        assert!(
            event["severity"].is_null()
                || matches!(
                    event["severity"].as_str(),
                    Some("mild" | "moderate" | "severe")
                )
        );
        assert!(event["notes"].is_null() || event["notes"].as_str().is_some());
        assert!(event["action_taken"].is_null() || event["action_taken"].as_str().is_some());
        assert!(event["medical_help_sought"].is_boolean());
        assert!(event["medication_names"].is_array());
    }
    let takes = health_data["medication_takes"]
        .as_array()
        .expect("medication takes");
    assert!(!takes.is_empty(), "fixture has an in-range medication take");
    let take_fields = [
        "taken_at",
        "medication_name",
        "dose_amount",
        "dose_unit",
        "source_type",
        "location_name",
    ];
    for take in takes {
        assert_keys(take, &take_fields, &take_fields);
        assert_timestamp(&take["taken_at"]);
        assert!(take["medication_name"].is_null() || take["medication_name"].is_string());
        assert!(take["dose_amount"].is_null() || take["dose_amount"].is_string());
        assert!(take["dose_unit"].is_null() || take["dose_unit"].is_string());
        assert!(matches!(
            take["source_type"].as_str(),
            Some("scheduled" | "routine" | "as_needed")
        ));
        assert!(take["location_name"].is_null() || take["location_name"].is_string());
    }

    let reviews_path = report_path(&fixture, "medication_reviews");
    let reviews_query = format!("{reviews_path}?person_id={}", fixture.managed_person_id);
    let reviews_response = target.get(&reviews_query, Some(&fixture.view_access_token));
    assert_eq!(reviews_response.status().as_u16(), 200);
    assert!(reviews_response.headers()["cache-control"]
        .to_str()
        .expect("no-store header")
        .contains("no-store"));
    let reviews = body(reviews_response);
    assert_keys(&reviews, &["data"], &["data"]);
    let review_data = &reviews["data"];
    let review_fields = ["person", "generated_at", "prompts"];
    assert_keys(review_data, &review_fields, &review_fields);
    assert_report_person(&review_data["person"]);
    assert_timestamp(&review_data["generated_at"]);
    let prompts = review_data["prompts"].as_array().expect("review prompts");
    assert!(
        !prompts.is_empty(),
        "seeded prompts exercise the report schema"
    );
    let prompt_fields = [
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
    ];
    for prompt in prompts {
        assert_keys(prompt, &prompt_fields, &prompt_fields);
        for id_field in [
            "id",
            "person_id",
            "primary_medication_id",
            "interacting_medication_id",
            "evidence_record_id",
        ] {
            assert!(prompt[id_field]
                .as_str()
                .is_some_and(|id| id.parse::<i64>().is_ok_and(|value| value > 0)));
        }
        for text_field in [
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
        ] {
            assert!(prompt[text_field].as_str().is_some(), "{text_field}");
        }
        assert_date(&prompt["evidence_source_checked_on"]);
        assert_date(&prompt["evidence_source_effective_on"]);
        assert!(matches!(
            prompt["risk_level"].as_str(),
            Some("low" | "moderate" | "high" | "unknown")
        ));
        assert!(matches!(
            prompt["match_confidence"].as_str(),
            Some("low" | "moderate" | "high" | "unknown")
        ));
        assert!(matches!(
            prompt["status"].as_str(),
            Some(
                "needs_review"
                    | "reviewed_with_practitioner"
                    | "expected_prescribed_combination"
                    | "not_relevant"
                    | "hidden_low_signal"
            )
        ));
        for nullable_text in ["practitioner_name", "practitioner_role", "review_note"] {
            assert!(prompt[nullable_text].is_null() || prompt[nullable_text].is_string());
        }
        if !prompt["reviewed_on"].is_null() {
            assert_date(&prompt["reviewed_on"]);
        }
        if !prompt["reviewed_by_membership_id"].is_null() {
            assert!(prompt["reviewed_by_membership_id"]
                .as_str()
                .is_some_and(|id| id.parse::<i64>().is_ok_and(|value| value > 0)));
        }
        assert_timestamp(&prompt["updated_at"]);
    }
}

#[test]
fn each_report_route_rejects_missing_authentication() {
    let fixture = fixture();
    let requests = [
        format!(
            "{}?person_id={}&start_date=2026-02-19&end_date=2026-02-26",
            report_path(&fixture, "health_history"),
            fixture.managed_person_id
        ),
        format!(
            "{}?person_id={}&start_date=2026-02-19&end_date=2026-02-26",
            report_path(&fixture, "health_history.pdf"),
            fixture.managed_person_id
        ),
        format!(
            "{}?person_id={}",
            report_path(&fixture, "medication_reviews"),
            fixture.managed_person_id
        ),
        format!(
            "{}?person_id={}",
            report_path(&fixture, "medication_reviews.pdf"),
            fixture.managed_person_id
        ),
    ];
    let target = Target::from_env();
    for request in requests {
        assert_error(target.get(&request, None), 401, "unauthorized");
    }
}

#[test]
fn pdf_reports_contain_the_selected_health_event_and_review_evidence() {
    let target = Target::from_env();
    let fixture = fixture();
    let health = format!(
        "{}?person_id={}&start_date=2026-02-19&end_date=2026-02-26",
        report_path(&fixture, "health_history.pdf"),
        fixture.managed_person_id
    );
    let health_json = format!(
        "{}?person_id={}&start_date=2026-02-19&end_date=2026-02-26",
        report_path(&fixture, "health_history"),
        fixture.managed_person_id
    );
    let health_data = body(target.get(&health_json, Some(&fixture.access_token)))["data"].clone();
    let health_person_name = health_data["person"]["name"]
        .as_str()
        .expect("health report person name")
        .to_owned();
    let ongoing_event = health_data["chronology"]
        .as_array()
        .expect("health report chronology")
        .iter()
        .find(|event| {
            event["title"] == fixture.managed_health_event_title
                && event["medication_names"]
                    .as_array()
                    .is_some_and(|medications| !medications.is_empty())
        })
        .expect("seeded ongoing health event");
    assert_eq!(ongoing_event["ongoing"], true);
    let linked_medicine = ongoing_event["medication_names"][0]
        .as_str()
        .unwrap_or_else(|| {
            panic!(
                "linked medicine name: {}",
                ongoing_event["medication_names"]
            )
        })
        .to_owned();
    let health_text = pdf_text(target.get(&health, Some(&fixture.access_token)));
    assert!(health_text.contains(&health_person_name));
    assert!(health_text.contains(&fixture.managed_health_event_title));
    assert!(health_text.contains("Duration: Ongoing"));
    assert!(health_text.contains(&linked_medicine));
    assert!(health_text.contains(
        "This report is not a diagnosis or a substitute for professional medical advice."
    ));
    assert!(!health_text.contains("Contract foreign event"));

    let reviews = format!(
        "{}?person_id={}",
        report_path(&fixture, "medication_reviews.pdf"),
        fixture.managed_person_id
    );
    let reviews_json = format!(
        "{}?person_id={}",
        report_path(&fixture, "medication_reviews"),
        fixture.managed_person_id
    );
    let review_data = body(target.get(&reviews_json, Some(&fixture.access_token)))["data"].clone();
    let review_person_name = review_data["person"]["name"]
        .as_str()
        .expect("review report person name");
    let high_risk_prompt = review_data["prompts"]
        .as_array()
        .expect("review report prompts")
        .iter()
        .find(|prompt| prompt["evidence_text"] == "Contract evidence high")
        .expect("seeded high-risk review evidence");
    assert_eq!(high_risk_prompt["risk_level"], "high");
    let primary_medication_name = high_risk_prompt["primary_medication_name"]
        .as_str()
        .expect("review primary medication name")
        .to_owned();
    let source_name = high_risk_prompt["evidence_source_name"]
        .as_str()
        .expect("review evidence source name")
        .to_owned();
    let risk_level = high_risk_prompt["risk_level"]
        .as_str()
        .expect("review risk level");
    let source_instruction = high_risk_prompt["source_instruction"]
        .as_str()
        .expect("review source instruction");
    let review_text = pdf_text(target.get(&reviews, Some(&fixture.access_token)));
    assert!(review_text.contains(review_person_name));
    assert!(review_text.contains(&primary_medication_name));
    assert!(review_text.contains("Contract evidence high"));
    assert!(review_text.contains(&source_name));
    assert!(review_text.contains(&format!("Risk: {risk_level}")));
    assert!(review_text.contains(source_instruction));
    assert!(review_text.contains(
        "This record organises public medicine-label evidence for discussion with a practitioner."
    ));
    assert!(!review_text.contains(&fixture.foreign_person_name));
}

#[test]
fn invalid_filters_are_rejected_on_both_pdf_routes() {
    let target = Target::from_env();
    let fixture = fixture();
    let health = format!(
        "{}?person_id={}&start_date=2026-02-19&end_date=2026-02-26&include_medication_takes=2",
        report_path(&fixture, "health_history.pdf"),
        fixture.managed_person_id
    );
    assert_error(
        target.get(&health, Some(&fixture.access_token)),
        422,
        "unprocessable_content",
    );
    let reviews = format!(
        "{}?person_id={}&status=private-invalid",
        report_path(&fixture, "medication_reviews.pdf"),
        fixture.managed_person_id
    );
    assert_error(
        target.get(&reviews, Some(&fixture.access_token)),
        422,
        "unprocessable_content",
    );
}

#[test]
fn renderer_failure_returns_503_without_success_audit_for_either_pdf_report() {
    let fixture = fixture();
    let (client, base_url) = report_failure_client();
    let mut db = postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        postgres::NoTls,
    )
    .expect("contract database");
    let requests = [
        (
            format!(
                "{}/api/v1/households/{}/reports/health_history.pdf?person_id={}&start_date=2026-02-19&end_date=2026-02-26",
                base_url, fixture.household_id, fixture.managed_person_id
            ),
            "health_history_report.downloaded",
        ),
        (
            format!(
                "{}/api/v1/households/{}/reports/medication_reviews.pdf?person_id={}",
                base_url, fixture.household_id, fixture.managed_person_id
            ),
            "medication_review_report.downloaded",
        ),
    ];
    for (url, event_type) in requests {
        let response = client
            .get(url)
            .bearer_auth(&fixture.access_token)
            .send()
            .expect("isolated renderer failure API must respond");
        assert_eq!(response.status().as_u16(), 503);
        assert!(response.headers()["cache-control"]
            .to_str()
            .expect("no-store header")
            .contains("no-store"));
        assert_eq!(
            response.headers()["content-type"]
                .to_str()
                .unwrap()
                .split(';')
                .next(),
            Some("application/json")
        );
        let request_id = response.headers()["x-request-id"]
            .to_str()
            .expect("request ID")
            .to_owned();
        let value = body(response);
        assert_keys(&value, &["error"], &["error"]);
        let error = &value["error"];
        assert_keys(
            error,
            &["code", "message", "request_id"],
            &["code", "message", "request_id", "errors"],
        );
        assert_eq!(error["code"], "report_unavailable");
        assert_eq!(error["request_id"], request_id);
        assert!(!value
            .to_string()
            .contains(&fixture.managed_health_event_title));
        assert!(!value.to_string().contains("Contract evidence high"));
        let audit_count: i64 = db
            .query_one(
                "SELECT count(*)::bigint FROM security_audit_events WHERE household_id = $1 AND event_type = $2 AND request_id = $3",
                &[&fixture.household_id, &event_type, &request_id],
            )
            .expect("no successful report download audit")
            .get(0);
        assert_eq!(audit_count, 0);
    }
}
