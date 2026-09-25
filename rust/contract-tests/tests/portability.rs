use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::{json, Value};

const PASSPHRASE: &str = "contract portable secret";
const HEADER: &str = "X-MedTracker-Portable-Passphrase";

fn path(id: i64, action: &str) -> String {
    format!("/api/v1/households/{id}/{action}")
}

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

#[track_caller]
fn snapshot(target: &Target, id: i64, token: &str) -> Value {
    let response = target.get(&path(id, "mobile_snapshot"), Some(token));
    assert_eq!(
        response.status().as_u16(),
        200,
        "snapshot household {id} at {}",
        std::panic::Location::caller()
    );
    body(response)["data"].clone()
}

fn collection(target: &Target, id: i64, token: &str, action: &str) -> Value {
    let response = target.get(&path(id, action), Some(token));
    assert_eq!(response.status().as_u16(), 200, "{action}");
    body(response)["data"].clone()
}

fn import_audit_count(target: &Target, id: i64, token: &str) -> usize {
    collection(target, id, token, "admin/audit_logs")
        .as_array()
        .expect("audit entries")
        .iter()
        .filter(|event| event["event_type"] == "portable_data.imported")
        .count()
}

fn rows<'a>(snapshot: &'a Value, collection: &str) -> &'a [Value] {
    snapshot["records"][collection]
        .as_array()
        .expect("portable collection")
}

fn row<'a>(snapshot: &'a Value, collection: &str, id: &str) -> &'a Value {
    rows(snapshot, collection)
        .iter()
        .find(|record| record["portable_id"] == id)
        .expect("portable record in public snapshot")
}

fn export(target: &Target, fixture: &Fixture) -> Value {
    let response = target.get_with_header(
        &path(fixture.portable_source_household_id, "portable_export"),
        &fixture.portable_source_access_token,
        HEADER,
        PASSPHRASE,
    );
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"].clone()
}

fn import(
    target: &Target,
    id: i64,
    token: &str,
    action: &str,
    passphrase: &str,
    bundle: &Value,
) -> Response {
    target.post_json_with_header(
        &path(id, action),
        token,
        HEADER,
        passphrase,
        &json!({"bundle": bundle}),
    )
}

fn assert_failure(response: Response, status: u16) -> Value {
    assert_eq!(response.status().as_u16(), status);
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .unwrap()
        .to_owned();
    let payload = body(response);
    assert!(!payload.to_string().contains(PASSPHRASE));
    if payload.get("error").is_some() {
        assert_eq!(payload["error"]["request_id"], request_id);
        assert!(payload["error"]["code"].is_string());
    } else {
        assert_eq!(payload["data"]["applied"], false);
        assert!(
            payload["data"]["errors"]
                .as_array()
                .is_some_and(|v| !v.is_empty())
                || payload["data"]["conflicts"]
                    .as_array()
                    .is_some_and(|v| !v.is_empty())
        );
    }
    payload
}

#[test]
fn portable_bundle_dry_run_apply_and_public_readback() {
    let target = Target::from_env();
    let fixture = fixture();
    let source_before = snapshot(
        &target,
        fixture.portable_source_household_id,
        &fixture.portable_source_access_token,
    );
    let target_before = snapshot(
        &target,
        fixture.portable_target_household_id,
        &fixture.portable_target_access_token,
    );
    let source_reviews_before = collection(
        &target,
        fixture.portable_source_household_id,
        &fixture.portable_source_access_token,
        "medication_review_prompts",
    );
    let source_grants_before = collection(
        &target,
        fixture.portable_source_household_id,
        &fixture.portable_source_access_token,
        "admin/person_access_grants",
    );
    assert!(!source_reviews_before.as_array().unwrap().is_empty());
    assert!(!source_grants_before.as_array().unwrap().is_empty());
    let initial_import_audits = import_audit_count(
        &target,
        fixture.portable_target_household_id,
        &fixture.portable_target_access_token,
    );
    assert_eq!(
        row(
            &source_before,
            "people",
            &fixture.portable_source_person_portable_id
        )["name"],
        fixture.portable_source_person_name
    );
    assert_eq!(
        row(
            &source_before,
            "medications",
            &fixture.portable_source_medication_portable_id
        )["location_portable_id"],
        fixture.portable_source_location_portable_id
    );
    assert_eq!(
        row(
            &source_before,
            "schedules",
            &fixture.portable_source_schedule_portable_id
        )["person_portable_id"],
        fixture.portable_source_person_portable_id
    );
    assert!(!rows(&source_before, "medication_takes").is_empty());
    assert!(!rows(&source_before, "notification_preferences").is_empty());

    let bundle = export(&target, &fixture);
    assert_eq!(bundle["format"], "medtracker.portable.encrypted.v1");
    assert_eq!(bundle["cipher"], "aes-256-gcm");
    assert_eq!(bundle["kdf"], "pbkdf2_sha256");
    assert_eq!(bundle["checksum"].as_str().unwrap().len(), 64);
    assert!(bundle["salt"].as_str().is_some_and(|v| !v.is_empty()));
    assert!(bundle["ciphertext"].as_str().is_some_and(|v| !v.is_empty()));
    assert!(!bundle
        .to_string()
        .contains(&fixture.portable_source_person_name));
    assert!(!bundle.to_string().contains(PASSPHRASE));

    let response = import(
        &target,
        fixture.portable_target_household_id,
        &fixture.portable_target_access_token,
        "portable_imports/dry_run",
        PASSPHRASE,
        &bundle,
    );
    assert_eq!(response.status().as_u16(), 200);
    let plan = body(response)["data"].clone();
    assert_eq!(plan["applied"], false);
    for collection in [
        "people",
        "locations",
        "medications",
        "dosage_options",
        "schedules",
        "medication_takes",
    ] {
        assert_eq!(
            plan["counts"][collection],
            rows(&source_before, collection).len()
        );
    }
    assert!(
        plan["conflicts"].as_array().unwrap().is_empty(),
        "{}",
        plan["conflicts"][0]["record_type"]
    );
    assert!(plan["errors"].as_array().unwrap().is_empty());
    assert_eq!(
        snapshot(
            &target,
            fixture.portable_target_household_id,
            &fixture.portable_target_access_token
        )["records"],
        target_before["records"]
    );
    assert_eq!(
        import_audit_count(
            &target,
            fixture.portable_target_household_id,
            &fixture.portable_target_access_token
        ),
        initial_import_audits
    );

    assert_failure(
        import(
            &target,
            fixture.portable_target_household_id,
            &fixture.portable_target_access_token,
            "portable_imports",
            "wrong secret",
            &bundle,
        ),
        422,
    );
    let malformed = assert_failure(
        import(
            &target,
            fixture.portable_target_household_id,
            &fixture.portable_target_access_token,
            "portable_imports",
            PASSPHRASE,
            &fixture.portable_malformed_bundle,
        ),
        422,
    );
    assert!(malformed["error"]["message"]
        .as_str()
        .unwrap()
        .contains("array"));
    let numeric = assert_failure(
        import(
            &target,
            fixture.portable_target_household_id,
            &fixture.portable_target_access_token,
            "portable_imports",
            PASSPHRASE,
            &fixture.portable_numeric_bundle,
        ),
        422,
    );
    assert!(numeric["data"]["errors"]
        .to_string()
        .contains("Rails numeric IDs"));
    let conflict = assert_failure(
        import(
            &target,
            fixture.portable_target_household_id,
            &fixture.portable_target_access_token,
            "portable_imports",
            PASSPHRASE,
            &fixture.portable_conflict_bundle,
        ),
        422,
    );
    assert_eq!(conflict["data"]["conflicts"][0]["record_type"], "locations");
    assert_eq!(
        snapshot(
            &target,
            fixture.portable_target_household_id,
            &fixture.portable_target_access_token
        )["records"],
        target_before["records"]
    );
    assert_eq!(
        import_audit_count(
            &target,
            fixture.portable_target_household_id,
            &fixture.portable_target_access_token
        ),
        initial_import_audits
    );

    let applied = import(
        &target,
        fixture.portable_target_household_id,
        &fixture.portable_target_app_token,
        "portable_imports",
        PASSPHRASE,
        &bundle,
    );
    assert_eq!(applied.status().as_u16(), 201);
    let applied = body(applied);
    assert_eq!(applied["data"]["applied"], true);
    assert_eq!(applied["data"]["counts"], plan["counts"]);
    assert!(!applied.to_string().contains(PASSPHRASE));
    assert_failure(
        target.get(
            &path(fixture.portable_target_household_id, "mobile_snapshot"),
            Some(&fixture.portable_target_access_token),
        ),
        401,
    );
    let target_after = snapshot(
        &target,
        fixture.portable_target_household_id,
        &fixture.portable_target_mobile_token,
    );
    let audit = collection(
        &target,
        fixture.portable_target_household_id,
        &fixture.portable_target_mobile_token,
        "admin/audit_logs",
    );
    assert!(!audit.to_string().contains(PASSPHRASE));
    assert!(audit.as_array().unwrap().iter().any(|event| {
        event["event_type"] == "portable_data.imported" && event["metadata"]["dry_run"] == false
    }));
    for collection in [
        "people",
        "locations",
        "medications",
        "dosage_options",
        "schedules",
        "medication_takes",
        "notification_preferences",
    ] {
        for source_row in rows(&source_before, collection) {
            let portable_id = source_row["portable_id"].as_str().expect("portable ID");
            assert_eq!(
                row(&target_after, collection, portable_id)["portable_id"],
                portable_id
            );
        }
    }
    assert_eq!(
        row(
            &target_after,
            "schedules",
            &fixture.portable_source_schedule_portable_id
        )["person_portable_id"],
        fixture.portable_source_person_portable_id
    );
    assert_eq!(
        row(
            &target_after,
            "schedules",
            &fixture.portable_source_schedule_portable_id
        )["medication_portable_id"],
        fixture.portable_source_medication_portable_id
    );
    assert_eq!(
        row(
            &target_after,
            "medications",
            &fixture.portable_source_medication_portable_id
        )["location_portable_id"],
        fixture.portable_source_location_portable_id
    );
    assert_eq!(
        snapshot(
            &target,
            fixture.portable_source_household_id,
            &fixture.portable_source_access_token
        )["records"],
        source_before["records"]
    );
    assert_eq!(
        collection(
            &target,
            fixture.portable_source_household_id,
            &fixture.portable_source_access_token,
            "medication_review_prompts"
        ),
        source_reviews_before
    );
    assert_eq!(
        collection(
            &target,
            fixture.portable_source_household_id,
            &fixture.portable_source_access_token,
            "admin/person_access_grants"
        ),
        source_grants_before
    );

    let replay = import(
        &target,
        fixture.portable_target_household_id,
        &fixture.portable_target_mobile_token,
        "portable_imports",
        PASSPHRASE,
        &bundle,
    );
    assert_eq!(replay.status().as_u16(), 201);
    let replayed = snapshot(
        &target,
        fixture.portable_target_household_id,
        &fixture.portable_target_mobile_token,
    );
    for collection in [
        "people",
        "locations",
        "medications",
        "dosage_options",
        "schedules",
        "medication_takes",
        "notification_preferences",
    ] {
        assert_eq!(
            rows(&replayed, collection).len(),
            rows(&target_after, collection).len()
        );
    }
}

#[test]
fn portable_endpoints_enforce_current_household_and_account_authority() {
    let target = Target::from_env();
    let fixture = fixture();
    let bundle = export(&target, &fixture);
    let before = snapshot(
        &target,
        fixture.portable_target_household_id,
        &fixture.portable_target_mobile_token,
    );
    assert_failure(
        target.get(
            &path(fixture.portable_source_household_id, "portable_export"),
            None,
        ),
        401,
    );
    assert_failure(
        target.get(
            &path(fixture.portable_source_household_id, "portable_export"),
            Some(&fixture.portable_source_access_token),
        ),
        422,
    );
    assert_failure(
        target.get(
            &format!(
                "{}?passphrase={PASSPHRASE}",
                path(fixture.portable_source_household_id, "portable_export")
            ),
            Some(&fixture.portable_source_access_token),
        ),
        422,
    );
    for action in ["portable_imports/dry_run", "portable_imports"] {
        assert_failure(
            target.post_json_authorized(
                &path(fixture.portable_target_household_id, action),
                &fixture.portable_target_mobile_token,
                &json!({"bundle": bundle}),
            ),
            422,
        );
    }
    assert_failure(
        target.get_with_header(
            &path(fixture.portable_target_household_id, "portable_export"),
            &fixture.portable_source_access_token,
            HEADER,
            PASSPHRASE,
        ),
        403,
    );
    for token in [
        &fixture.portable_revoked_access_token,
        &fixture.portable_locked_access_token,
    ] {
        assert_failure(
            import(
                &target,
                fixture.portable_target_household_id,
                token,
                "portable_imports/dry_run",
                PASSPHRASE,
                &bundle,
            ),
            401,
        );
    }
    assert_failure(
        import(
            &target,
            fixture.portable_target_household_id,
            &fixture.portable_member_access_token,
            "portable_imports",
            PASSPHRASE,
            &bundle,
        ),
        403,
    );
    assert_failure(
        import(
            &target,
            fixture.portable_target_household_id,
            &fixture.portable_source_access_token,
            "portable_imports",
            PASSPHRASE,
            &bundle,
        ),
        403,
    );
    assert_eq!(
        snapshot(
            &target,
            fixture.portable_target_household_id,
            &fixture.portable_target_mobile_token
        )["records"],
        before["records"]
    );
}
