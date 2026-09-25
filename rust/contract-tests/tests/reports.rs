use base64::{engine::general_purpose::STANDARD, Engine};
use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::Value;

fn report_path(fixture: &Fixture, kind: &str) -> String {
    format!("/api/v1/households/{}/reports/{kind}", fixture.household_id)
}

fn export_path(fixture: &Fixture, mode: &str) -> String {
    format!(
        "/api/v1/households/{}/data_exports/{mode}",
        fixture.household_id
    )
}

fn request_id(response: &Response) -> String {
    response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned()
}

fn no_store(response: &Response) {
    assert!(response.headers()["cache-control"]
        .to_str()
        .expect("cache header")
        .contains("no-store"));
}

fn json(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn audits(target: &Target, fixture: &Fixture) -> Vec<Value> {
    let path = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.household_id
    );
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    json(response)["data"]
        .as_array()
        .expect("audit entries")
        .to_vec()
}

fn audit_for_request<'a>(audits: &'a [Value], id: &str, event_type: &str) -> Option<&'a Value> {
    audits
        .iter()
        .find(|row| row["request_id"] == id && row["event_type"] == event_type)
}

fn assert_audited<'a>(audits: &'a [Value], id: &str, event_type: &str, format: &str) -> &'a Value {
    let event = audit_for_request(audits, id, event_type).expect("successful download audit");
    if !format.is_empty() {
        assert_eq!(event["metadata"]["outcome"], "success");
        assert_eq!(event["metadata"]["format"], format);
    }
    event
}

fn assert_error(response: Response, status: u16, code: &str, private_text: &str) -> String {
    assert_eq!(response.status().as_u16(), status);
    if status != 403 {
        no_store(&response);
    }
    let id = request_id(&response);
    let body = json(response);
    assert_eq!(body["error"]["code"], code);
    assert_eq!(body["error"]["request_id"], id);
    assert!(!body.to_string().contains(private_text));
    assert!(body.get("data").is_none());
    id
}

fn assert_pdf(response: Response, filename_suffix: &str) -> String {
    assert_eq!(response.status().as_u16(), 200);
    no_store(&response);
    assert_eq!(response.headers()["content-type"], "application/pdf");
    let disposition = response.headers()["content-disposition"].to_str().unwrap();
    assert!(disposition.contains("attachment"));
    assert!(disposition.contains(filename_suffix));
    let id = request_id(&response);
    let bytes = response.bytes().expect("PDF bytes");
    assert!(bytes.len() > 500, "PDF must be nonempty");
    assert!(bytes.starts_with(b"%PDF-"));
    assert!(bytes.windows(5).any(|window| window == b"%%EOF"));
    id
}

#[test]
fn health_history_json_chronology_dates_and_takes() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = report_path(&fixture, "health_history");
    let query = format!(
        "{path}?person_id={}&start_date=2026-02-19&end_date=2026-02-26",
        fixture.managed_person_portable_id
    );
    let response = target.get(&query, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    no_store(&response);
    let id = request_id(&response);
    let body = json(response);
    let data = &body["data"];
    assert_eq!(data["person"]["id"], fixture.managed_person_id.to_string());
    assert_eq!(data["start_date"], "2026-02-19");
    assert_eq!(data["end_date"], "2026-02-26");
    assert!(data["generated_at"].is_string());
    let chronology = data["chronology"].as_array().expect("chronology");
    let managed_event_id = fixture.managed_health_event_id.to_string();
    let earlier_event_id = fixture.earlier_health_event_id.to_string();
    let managed = chronology
        .iter()
        .position(|row| row["id"].as_str() == Some(managed_event_id.as_str()))
        .expect("fixture event");
    let earlier = chronology
        .iter()
        .position(|row| row["id"].as_str() == Some(earlier_event_id.as_str()))
        .expect("earlier fixture event");
    assert!(managed < earlier, "chronology is newest first");
    assert_eq!(chronology[managed]["started_on"], "2026-02-25");
    assert_eq!(
        chronology[managed]["medication_names"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(data["medication_takes"].as_array().unwrap().len(), 0);
    assert!(!body.to_string().contains("Contract hidden event"));
    assert!(!body.to_string().contains("Contract foreign event"));
    let filtered = format!(
        "{path}?person_id={}&start_date=2026-02-24&end_date=2026-02-26&include_medication_takes=1",
        fixture.managed_person_id
    );
    let response = target.get(&filtered, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let filtered_id = request_id(&response);
    let body = json(response);
    let chronology = body["data"]["chronology"].as_array().unwrap();
    assert!(chronology
        .iter()
        .any(|row| row["id"].as_str() == Some(managed_event_id.as_str())));
    assert!(!chronology
        .iter()
        .any(|row| row["id"].as_str() == Some(earlier_event_id.as_str())));
    assert!(!body["data"]["medication_takes"]
        .as_array()
        .unwrap()
        .is_empty());
    let audits = audits(&target, &fixture);
    assert_audited(&audits, &id, "health_history_report.downloaded", "json");
    assert_audited(
        &audits,
        &filtered_id,
        "health_history_report.downloaded",
        "json",
    );
}

#[test]
fn health_history_pdf_and_denied_requests() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = report_path(&fixture, "health_history");
    let query = format!(
        "{path}.pdf?person_id={}&start_date=2026-02-24&end_date=2026-02-26",
        fixture.managed_person_id
    );
    let id = assert_pdf(
        target.get(&query, Some(&fixture.access_token)),
        "2026-02-24-to-2026-02-26.pdf",
    );
    let mut denied_ids = Vec::new();
    for (person, token, status) in [
        (fixture.managed_person_id, &fixture.view_access_token, 404),
        (fixture.hidden_person_id, &fixture.view_access_token, 404),
        (fixture.foreign_person_id, &fixture.access_token, 404),
    ] {
        let denied = format!("{path}?person_id={person}&start_date=2026-02-24&end_date=2026-02-26");
        let id = assert_error(
            target.get(&denied, Some(token)),
            status,
            "not_found",
            "Contract managed event",
        );
        denied_ids.push(id);
    }
    let foreign = format!(
        "/api/v1/households/{}/reports/health_history?person_id={}",
        fixture.foreign_household_id, fixture.foreign_person_id
    );
    denied_ids.push(assert_error(
        target.get(&foreign, Some(&fixture.access_token)),
        403,
        "forbidden",
        &fixture.foreign_person_name,
    ));
    let audits = audits(&target, &fixture);
    assert_audited(&audits, &id, "health_history_report.downloaded", "pdf");
    for denied in denied_ids {
        assert!(audit_for_request(&audits, &denied, "health_history_report.downloaded").is_none());
    }
}

#[test]
fn health_history_rejects_invalid_filters_without_download_audit() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = report_path(&fixture, "health_history");
    let mut denied_ids = Vec::new();
    for query in [
        String::new(),
        format!(
            "?person_id={}&start_date=private-invalid",
            fixture.managed_person_id
        ),
        format!(
            "?person_id={}&start_date=2026-02-27&end_date=2026-02-26",
            fixture.managed_person_id
        ),
        format!(
            "?person_id={}&start_date=2020-01-01&end_date=2026-02-26",
            fixture.managed_person_id
        ),
        format!(
            "?person_id={}&include_medication_takes=private-invalid",
            fixture.managed_person_id
        ),
    ] {
        let status = if query.is_empty() { 400 } else { 422 };
        let code = if status == 400 {
            "bad_request"
        } else {
            "unprocessable_content"
        };
        let id = assert_error(
            target.get(&format!("{path}{query}"), Some(&fixture.access_token)),
            status,
            code,
            "private-invalid",
        );
        denied_ids.push(id);
    }
    let audits = audits(&target, &fixture);
    for denied in denied_ids {
        assert!(audit_for_request(&audits, &denied, "health_history_report.downloaded").is_none());
    }
}

#[test]
fn medication_review_snapshot_filter_pdf_and_access() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = report_path(&fixture, "medication_reviews");
    let query = format!("{path}?person_id={}", fixture.managed_person_portable_id);
    let response = target.get(&query, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    no_store(&response);
    let id = request_id(&response);
    let body = json(response);
    let data = &body["data"];
    assert_eq!(data["person"]["id"], fixture.managed_person_id.to_string());
    assert!(data["generated_at"].is_string());
    let prompts = data["prompts"].as_array().unwrap();
    assert_eq!(prompts.len(), 4);
    let managed_prompt_id = fixture.managed_review_prompt_id.to_string();
    let managed_person_id = fixture.managed_person_id.to_string();
    assert!(prompts
        .iter()
        .any(|row| row["id"].as_str() == Some(managed_prompt_id.as_str())
            && row["evidence_text"] == "Contract evidence high"));
    assert!(prompts
        .iter()
        .all(|row| row["person_id"].as_str() == Some(managed_person_id.as_str())));
    assert!(!body.to_string().contains(&fixture.foreign_person_name));
    let filtered = format!(
        "{path}?person_id={}&status=hidden_low_signal",
        fixture.managed_person_id
    );
    let response = target.get(&filtered, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let filtered_id = request_id(&response);
    let filtered = json(response);
    assert_eq!(filtered["data"]["prompts"].as_array().unwrap().len(), 1);
    assert_eq!(
        filtered["data"]["prompts"][0]["id"],
        fixture.low_signal_review_prompt_id.to_string()
    );

    let pdf = format!("{path}.pdf?person_id={}", fixture.managed_person_id);
    let pdf_id = assert_pdf(target.get(&pdf, Some(&fixture.access_token)), ".pdf");

    let mut denied_ids = Vec::new();
    for (person, token) in [
        (fixture.hidden_person_id, &fixture.view_access_token),
        (fixture.foreign_person_id, &fixture.access_token),
    ] {
        let denied = format!("{path}?person_id={person}");
        let id = assert_error(
            target.get(&denied, Some(token)),
            404,
            "not_found",
            "Contract evidence high",
        );
        denied_ids.push(id);
    }
    let foreign = format!(
        "/api/v1/households/{}/reports/medication_reviews?person_id={}",
        fixture.foreign_household_id, fixture.foreign_person_id
    );
    denied_ids.push(assert_error(
        target.get(&foreign, Some(&fixture.access_token)),
        403,
        "forbidden",
        &fixture.foreign_person_name,
    ));
    let audits = audits(&target, &fixture);
    assert_audited(&audits, &id, "medication_review_report.downloaded", "json");
    assert_audited(
        &audits,
        &filtered_id,
        "medication_review_report.downloaded",
        "json",
    );
    assert_audited(
        &audits,
        &pdf_id,
        "medication_review_report.downloaded",
        "pdf",
    );
    for denied in denied_ids {
        assert!(
            audit_for_request(&audits, &denied, "medication_review_report.downloaded").is_none()
        );
    }
}

#[test]
fn medication_review_invalid_parameters_have_no_download_audit() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = report_path(&fixture, "medication_reviews");
    let mut denied_ids = Vec::new();
    for query in [
        String::new(),
        format!(
            "?person_id={}&status=private-invalid",
            fixture.managed_person_id
        ),
        format!(
            "?person_id={}&start_date=2026-02-20",
            fixture.managed_person_id
        ),
        format!("?person_id={}&format=xml", fixture.managed_person_id),
    ] {
        let status = if query.is_empty() { 400 } else { 422 };
        let code = if status == 400 {
            "bad_request"
        } else {
            "unprocessable_content"
        };
        let id = assert_error(
            target.get(&format!("{path}{query}"), Some(&fixture.access_token)),
            status,
            code,
            "private-invalid",
        );
        denied_ids.push(id);
    }
    let audits = audits(&target, &fixture);
    for denied in denied_ids {
        assert!(
            audit_for_request(&audits, &denied, "medication_review_report.downloaded").is_none()
        );
    }
}

#[test]
fn data_exports_cover_json_zip_encryption_and_errors() {
    let target = Target::from_env();
    let fixture = fixture();
    let json_path = export_path(&fixture, "health_data_json");
    let response = target.get(&json_path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    no_store(&response);
    let json_id = request_id(&response);
    let data = json(response)["data"].clone();
    assert_eq!(data["format"], "medtracker.health_data.v1");
    assert!(data["records"]["people"]
        .as_array()
        .unwrap()
        .iter()
        .any(|person| person["portable_id"] == fixture.managed_person_portable_id));
    assert!(data["records"]["medications"].is_array());
    assert!(!data.to_string().contains(&fixture.foreign_person_name));
    let response = target.get(&json_path, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let viewer_id = request_id(&response);
    let viewer = json(response);
    let viewer_people = viewer["data"]["records"]["people"].as_array().unwrap();
    assert!(!viewer_people.iter().any(|person| person["portable_id"]
        == fixture.managed_person_portable_id
        || person["portable_id"] == fixture.hidden_person_portable_id));

    let zip_path = export_path(&fixture, "backup_zip");
    let response = target.get(&zip_path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    no_store(&response);
    let zip_id = request_id(&response);
    let zip = json(response)["data"].clone();
    assert_eq!(zip["content_type"], "application/zip");
    assert!(zip["filename"].as_str().unwrap().ends_with(".zip"));
    let bytes = STANDARD
        .decode(zip["base64"].as_str().unwrap())
        .expect("base64 ZIP");
    assert!(bytes.starts_with(b"PK\x03\x04"));
    assert!(bytes.windows(4).any(|window| window == b"PK\x05\x06"));
    let name_len = u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
    let extra_len = u16::from_le_bytes([bytes[28], bytes[29]]) as usize;
    let content_len = u32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]) as usize;
    let content_start = 30 + name_len + extra_len;
    let payload: Value = serde_json::from_slice(&bytes[content_start..content_start + content_len])
        .expect("ZIP contains JSON export");
    assert_eq!(payload["format"], "medtracker.backup.v1");
    assert!(payload["records"]["people"].is_array());
    assert!(!payload.to_string().contains(&fixture.foreign_person_name));
    assert!(!payload.to_string().contains(&fixture.access_token));
    let encrypted_path = export_path(&fixture, "encrypted_migration_bundle");
    let missing = target.get(&encrypted_path, Some(&fixture.access_token));
    let missing_id = assert_error(
        missing,
        422,
        "unprocessable_content",
        "correct horse battery staple",
    );
    let response = target.get_with_header(
        &encrypted_path,
        &fixture.access_token,
        "X-MedTracker-Portable-Passphrase",
        "correct horse battery staple",
    );
    assert_eq!(response.status().as_u16(), 200);
    no_store(&response);
    let encrypted_id = request_id(&response);
    let encrypted = json(response)["data"].clone();
    assert_eq!(encrypted["format"], "medtracker.portable.encrypted.v1");
    assert_eq!(encrypted["cipher"], "aes-256-gcm");
    assert!(!encrypted
        .to_string()
        .contains("correct horse battery staple"));
    assert!(!encrypted.to_string().contains(&fixture.foreign_person_name));
    let unsupported = export_path(&fixture, "private-invalid");
    let unsupported_id = assert_error(
        target.get(&unsupported, Some(&fixture.access_token)),
        422,
        "unprocessable_content",
        "private-invalid",
    );
    let foreign = format!(
        "/api/v1/households/{}/data_exports/health_data_json",
        fixture.foreign_household_id
    );
    let foreign_id = assert_error(
        target.get(&foreign, Some(&fixture.access_token)),
        403,
        "forbidden",
        &fixture.foreign_person_name,
    );
    let audits = audits(&target, &fixture);
    let audit = assert_audited(&audits, &json_id, "portable_data.exported", "");
    assert_eq!(audit["metadata"]["export_mode"], "health_data_json");
    assert_eq!(audit["metadata"]["encrypted"], false);
    assert!(
        audit["metadata"]["record_counts"]["people"]
            .as_u64()
            .unwrap()
            >= 2
    );
    let audit = assert_audited(&audits, &viewer_id, "portable_data.exported", "");
    assert_eq!(audit["metadata"]["export_mode"], "health_data_json");
    let audit = assert_audited(&audits, &zip_id, "portable_data.exported", "");
    assert_eq!(audit["metadata"]["export_mode"], "backup_zip");
    assert_eq!(audit["metadata"]["encrypted"], false);
    let audit = assert_audited(&audits, &encrypted_id, "portable_data.exported", "");
    assert_eq!(
        audit["metadata"]["export_mode"],
        "encrypted_migration_bundle"
    );
    assert_eq!(audit["metadata"]["encrypted"], true);
    assert!(!audit.to_string().contains("correct horse battery staple"));
    for denied in [missing_id, unsupported_id, foreign_id] {
        assert!(audit_for_request(&audits, &denied, "portable_data.exported").is_none());
    }
}
