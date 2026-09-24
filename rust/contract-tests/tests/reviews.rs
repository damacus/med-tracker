use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::{json, Value};

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn etag(response: &Response) -> String {
    response.headers()["etag"]
        .to_str()
        .expect("ETag")
        .to_owned()
}

fn path(fixture: &Fixture) -> String {
    format!(
        "/api/v1/households/{}/medication_review_prompts",
        fixture.household_id
    )
}

#[test]
fn review_lists_filter_paginate_and_hide_ungranted_people() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = path(&fixture);
    let response = target.get(&base, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert!(response.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("no-store"));
    let list = body(response);
    let rows = list["data"].as_array().unwrap();
    let managed_person_id = fixture.managed_person_id.to_string();
    let managed_prompt_id = fixture.managed_review_prompt_id.to_string();
    let hidden_prompt_id = fixture.hidden_review_prompt_id.to_string();
    let foreign_prompt_id = fixture.foreign_review_prompt_id.to_string();
    let low_signal_prompt_id = fixture.low_signal_review_prompt_id.to_string();
    assert_eq!(rows.len(), 4);
    assert!(rows
        .iter()
        .all(|row| row["person_id"].as_str() == Some(managed_person_id.as_str())));
    assert!(rows
        .iter()
        .any(|row| row["id"].as_str() == Some(managed_prompt_id.as_str())));
    assert!(!rows
        .iter()
        .any(|row| row["id"].as_str() == Some(hidden_prompt_id.as_str())));
    assert!(!rows
        .iter()
        .any(|row| row["id"].as_str() == Some(foreign_prompt_id.as_str())));
    assert!(!rows
        .iter()
        .any(|row| row["id"].as_str() == Some(low_signal_prompt_id.as_str())));
    assert_eq!(list["meta"]["total_count"], 4);

    let first = body(target.get(
        &format!("{base}?page=1&per_page=1"),
        Some(&fixture.access_token),
    ));
    let second = body(target.get(
        &format!("{base}?page=2&per_page=1"),
        Some(&fixture.access_token),
    ));
    assert_eq!(first["meta"]["per_page"], 1);
    assert_eq!(first["meta"]["total_count"], 4);
    assert_ne!(first["data"][0]["id"], second["data"][0]["id"]);

    let high = body(target.get(
        &format!("{base}?priority=discuss_soon"),
        Some(&fixture.access_token),
    ));
    assert_eq!(high["data"].as_array().unwrap().len(), 1);
    assert_eq!(
        high["data"][0]["id"],
        fixture.managed_review_prompt_id.to_string()
    );
    let moderate = body(target.get(
        &format!("{base}?priority=ask_when_convenient"),
        Some(&fixture.access_token),
    ));
    assert_eq!(moderate["data"].as_array().unwrap().len(), 1);
    assert_eq!(
        moderate["data"][0]["id"],
        fixture.second_review_prompt_id.to_string()
    );

    let hidden = body(target.get(
        &format!("{base}?show_hidden=1"),
        Some(&fixture.access_token),
    ));
    assert_eq!(hidden["meta"]["total_count"], 5);
    let hidden_rows = hidden["data"].as_array().unwrap();
    assert!(hidden_rows.iter().any(|row| row["id"].as_str()
        == Some(low_signal_prompt_id.as_str())
        && row["person_id"].as_str() == Some(managed_person_id.as_str())
        && row["status"] == "hidden_low_signal"));
    assert!(!hidden_rows.iter().any(|row| {
        row["id"].as_str() == Some(hidden_prompt_id.as_str())
            || row["id"].as_str() == Some(foreign_prompt_id.as_str())
    }));
    let paged_ids: Vec<String> = (1..=5)
        .map(|page| {
            let response = target.get(
                &format!("{base}?show_hidden=1&page={page}&per_page=1"),
                Some(&fixture.view_access_token),
            );
            assert_eq!(response.status().as_u16(), 200);
            let page_body = body(response);
            assert_eq!(page_body["meta"]["total_count"], 5);
            page_body["data"][0]["id"].as_str().unwrap().to_owned()
        })
        .collect();
    assert_eq!(paged_ids.len(), 5);
    assert_eq!(
        paged_ids
            .iter()
            .filter(|id| *id == &low_signal_prompt_id)
            .count(),
        1
    );
    assert!(!paged_ids.contains(&hidden_prompt_id));
    assert!(!paged_ids.contains(&foreign_prompt_id));
    let invalid = target.get(
        &format!("{base}?priority=private-invalid"),
        Some(&fixture.access_token),
    );
    assert_eq!(invalid.status().as_u16(), 422);
    assert!(!invalid.text().unwrap().contains("private-invalid"));
    let invalid = target.get(
        &format!("{base}?review_status=private-invalid"),
        Some(&fixture.access_token),
    );
    assert_eq!(invalid.status().as_u16(), 422);
    let invalid = target.get(
        &format!("{base}?per_page=invalid"),
        Some(&fixture.access_token),
    );
    assert_eq!(invalid.status().as_u16(), 200);
    assert_eq!(body(invalid)["meta"]["per_page"], 1);
    let foreign = target.get(
        &format!(
            "/api/v1/households/{}/medication_review_prompts",
            fixture.foreign_household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(foreign.status().as_u16(), 403);
}

#[test]
fn review_get_has_bounded_snapshot_and_non_disclosing_access() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = path(&fixture);
    let url = format!("{base}/{}", fixture.managed_review_prompt_id);
    let response = target.get(&url, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert!(response.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("no-store"));
    let tag = etag(&response);
    let row = body(response)["data"].clone();
    assert_eq!(row["id"], fixture.managed_review_prompt_id.to_string());
    assert_eq!(row["person_id"], fixture.managed_person_id.to_string());
    assert_eq!(row["etag"], tag);
    assert_eq!(row["status"], "needs_review");
    assert_eq!(row["risk_level"], "high");
    assert!(row["evidence_text"]
        .as_str()
        .unwrap()
        .contains("Contract evidence"));
    for id in [
        fixture.hidden_review_prompt_id,
        fixture.foreign_review_prompt_id,
    ] {
        let response = target.get(&format!("{base}/{id}"), Some(&fixture.view_access_token));
        assert_eq!(response.status().as_u16(), 404);
        assert_eq!(body(response)["error"]["code"], "not_found");
    }
}

#[test]
fn review_patch_requires_version_valid_context_and_manage_access() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = path(&fixture);
    let url = format!("{base}/{}", fixture.invalid_review_prompt_id);
    let original = target.get(&url, Some(&fixture.access_token));
    let tag = etag(&original);
    let snapshot = body(original)["data"].clone();
    let invalid = json!({"medication_review_prompt": {"status": "reviewed_with_practitioner", "practitioner_name": ""}});
    let response = target.patch_json_if_match(&url, &fixture.access_token, &invalid, &tag);
    assert_eq!(response.status().as_u16(), 422);
    let missing = target.patch_json(
        &url,
        &fixture.access_token,
        &json!({"medication_review_prompt": {"status": "not_relevant"}}),
    );
    assert_eq!(missing.status().as_u16(), 428);
    let stale = target.patch_json_if_match(
        &url,
        &fixture.access_token,
        &json!({"medication_review_prompt": {"status": "not_relevant"}}),
        "stale",
    );
    assert_eq!(stale.status().as_u16(), 409);
    let invalid_put = target.put_json_if_match(&url, &fixture.access_token, &json!({"medication_review_prompt": {"status": "reviewed_with_practitioner", "practitioner_name": ""}}), &tag);
    assert_eq!(invalid_put.status().as_u16(), 422);
    let missing_put = target.put_json(
        &url,
        &fixture.access_token,
        &json!({"medication_review_prompt": {"status": "not_relevant"}}),
    );
    assert_eq!(missing_put.status().as_u16(), 428);
    let stale_put = target.put_json_if_match(
        &url,
        &fixture.access_token,
        &json!({"medication_review_prompt": {"status": "not_relevant"}}),
        "stale",
    );
    assert_eq!(stale_put.status().as_u16(), 409);
    let forbidden = target.patch_json_if_match(
        &url,
        &fixture.view_access_token,
        &json!({"medication_review_prompt": {"status": "not_relevant"}}),
        &tag,
    );
    assert_eq!(forbidden.status().as_u16(), 403);
    let forbidden_put = target.put_json_if_match(
        &url,
        &fixture.view_access_token,
        &json!({"medication_review_prompt": {"status": "not_relevant"}}),
        &tag,
    );
    assert_eq!(forbidden_put.status().as_u16(), 403);
    for id in [
        fixture.hidden_review_prompt_id,
        fixture.foreign_review_prompt_id,
    ] {
        let response = target.patch_json_if_match(
            &format!("{base}/{id}"),
            &fixture.access_token,
            &json!({"medication_review_prompt": {"status": "not_relevant"}}),
            &tag,
        );
        assert_eq!(response.status().as_u16(), 404);
        let response = target.put_json_if_match(
            &format!("{base}/{id}"),
            &fixture.access_token,
            &json!({"medication_review_prompt": {"status": "not_relevant"}}),
            &tag,
        );
        assert_eq!(response.status().as_u16(), 404);
    }
    let after = body(target.get(&url, Some(&fixture.access_token)))["data"].clone();
    assert_eq!(after["status"], "needs_review");
    assert_eq!(after["etag"], tag);
    assert_eq!(after["evidence_text"], snapshot["evidence_text"]);
}

#[test]
fn review_patch_and_put_retain_evidence_and_emit_auditable_updates() {
    let target = Target::from_env();
    let fixture = fixture();
    let base = path(&fixture);
    let url = format!("{base}/{}", fixture.edit_review_prompt_id);
    let initial = target.get(&url, Some(&fixture.access_token));
    let tag = etag(&initial);
    let snapshot = body(initial)["data"].clone();
    let response = target.patch_json_if_match(&url, &fixture.access_token, &json!({"medication_review_prompt": {"status": "reviewed_with_practitioner", "practitioner_name": "Dr Taylor", "practitioner_role": "GP", "reviewed_on": "2026-09-24", "review_note": "Private practitioner context", "evidence_text": "Forged evidence"}}), &tag);
    assert_eq!(response.status().as_u16(), 200);
    let patched_tag = etag(&response);
    let patched = body(response)["data"].clone();
    assert_ne!(patched_tag, tag);
    assert_eq!(patched["status"], "reviewed_with_practitioner");
    assert_eq!(patched["practitioner_name"], "Dr Taylor");
    assert!(!patched["reviewed_by_membership_id"].is_null());
    assert_eq!(patched["evidence_text"], snapshot["evidence_text"]);
    let response = target.put_json_if_match(&url, &fixture.access_token, &json!({"medication_review_prompt": {"status": "not_relevant", "review_note": "Updated note"}}), &patched_tag);
    assert_eq!(response.status().as_u16(), 200);
    let put_tag = etag(&response);
    assert_ne!(put_tag, patched_tag);
    let updated = body(response)["data"].clone();
    assert_eq!(updated["status"], "not_relevant");
    assert_eq!(updated["review_note"], "Updated note");
    assert_eq!(updated["evidence_text"], snapshot["evidence_text"]);
    assert_eq!(updated["etag"], put_tag);
    let retained_response = target.get(&url, Some(&fixture.access_token));
    assert_eq!(etag(&retained_response), put_tag);
    let retained = body(retained_response)["data"].clone();
    assert_eq!(retained, updated);
    let reviewed = body(target.get(
        &format!("{base}?review_status=reviewed"),
        Some(&fixture.access_token),
    ));
    let edit_prompt_id = fixture.edit_review_prompt_id.to_string();
    assert!(reviewed["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"].as_str() == Some(edit_prompt_id.as_str())));
    let audit = body(target.get(
        &format!(
            "/api/v1/households/{}/admin/audit_logs",
            fixture.household_id
        ),
        Some(&fixture.access_token),
    ));
    let updates: Vec<_> = audit["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| {
            event["event_type"] == "medication_review_prompt.updated"
                && event["metadata"]["prompt_id"] == fixture.edit_review_prompt_id
        })
        .collect();
    assert_eq!(updates.len(), 2);
    assert!(updates.iter().any(
        |event| event["metadata"]["previous_status"] == "needs_review"
            && event["metadata"]["status"] == "reviewed_with_practitioner"
    ));
    assert!(updates
        .iter()
        .any(
            |event| event["metadata"]["previous_status"] == "reviewed_with_practitioner"
                && event["metadata"]["status"] == "not_relevant"
        ));
    assert!(!audit.to_string().contains("Private practitioner context"));
    assert!(!audit.to_string().contains("Dr Taylor"));
}
