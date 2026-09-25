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

#[derive(Debug, PartialEq)]
struct TargetState {
    records: Value,
    grants: Value,
    import_audits: Vec<Value>,
}

fn target_state(target: &Target, id: i64, token: &str) -> TargetState {
    let records = snapshot(target, id, token)["records"].clone();
    let grants = collection(target, id, token, "admin/person_access_grants");
    let import_audits = collection(target, id, token, "admin/audit_logs")
        .as_array()
        .expect("audit entries")
        .iter()
        .filter(|event| event["event_type"] == "portable_data.imported")
        .cloned()
        .collect();
    TargetState {
        records,
        grants,
        import_audits,
    }
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

fn assert_imported_record_values(source: &Value, imported: &Value) {
    let mut mismatches = Vec::new();
    for collection in [
        "people",
        "locations",
        "medications",
        "dosage_options",
        "schedules",
        "person_medications",
        "medication_takes",
        "notification_preferences",
    ] {
        for source_row in rows(source, collection) {
            let portable_id = source_row["portable_id"].as_str().expect("portable ID");
            let imported_row = row(imported, collection, portable_id);
            for (field, value) in source_row.as_object().expect("portable record") {
                if field != "updated_at" && field != "etag" && &imported_row[field] != value {
                    mismatches.push(format!(
                        "{collection}/{portable_id}.{field}: {:?} != {:?}",
                        value, imported_row[field]
                    ));
                }
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

fn audit_for_request<'a>(audits: &'a Value, request_id: &str, event_type: &str) -> &'a Value {
    audits
        .as_array()
        .expect("audit entries")
        .iter()
        .find(|event| event["request_id"] == request_id && event["event_type"] == event_type)
        .expect("request-correlated audit event")
}

fn export(target: &Target, fixture: &Fixture) -> (Value, String) {
    let response = target.get_with_header(
        &path(fixture.portable_source_household_id, "portable_export"),
        &fixture.portable_source_access_token,
        HEADER,
        PASSPHRASE,
    );
    assert_eq!(response.status().as_u16(), 200);
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .expect("export request ID")
        .to_owned();
    (body(response)["data"].clone(), request_id)
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

fn assert_rejected_import(
    target: &Target,
    fixture: &Fixture,
    before: &TargetState,
    response: Response,
    status: u16,
) -> Value {
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .expect("denied import request ID")
        .to_owned();
    let payload = assert_failure(response, status);
    let after = target_state(
        target,
        fixture.portable_target_household_id,
        &fixture.portable_target_access_token,
    );
    assert_eq!(&after, before, "failed import changed target state");
    assert!(!after
        .import_audits
        .iter()
        .any(|event| event["request_id"] == request_id));
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
    let target_state_before = target_state(
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
    let source_person = row(
        &source_before,
        "people",
        &fixture.portable_source_person_portable_id,
    );
    assert!(source_person["location_portable_ids"]
        .as_array()
        .unwrap()
        .contains(&Value::from(
            fixture.portable_source_location_portable_id.as_str()
        )));
    assert!(source_person["notification_preference_portable_id"].is_string());
    let dosage = rows(&source_before, "dosage_options")
        .first()
        .expect("source dosage");
    assert_eq!(
        dosage["medication_portable_id"],
        fixture.portable_source_medication_portable_id
    );
    let assignment = rows(&source_before, "person_medications")
        .first()
        .expect("source assignment");
    assert_eq!(
        assignment["person_portable_id"],
        fixture.portable_source_person_portable_id
    );
    assert_eq!(
        assignment["medication_portable_id"],
        fixture.portable_source_medication_portable_id
    );
    assert_eq!(
        assignment["source_dosage_option_portable_id"],
        dosage["portable_id"]
    );
    let source_schedule = row(
        &source_before,
        "schedules",
        &fixture.portable_source_schedule_portable_id,
    );
    assert_eq!(
        source_schedule["source_dosage_option_portable_id"],
        dosage["portable_id"]
    );
    for source_type in ["schedule", "person_medication"] {
        let take = rows(&source_before, "medication_takes")
            .iter()
            .find(|take| take["source_type"] == source_type)
            .expect("take for each source type");
        assert!(take["source_portable_id"].is_string());
        assert_eq!(
            take["taken_from_medication_portable_id"],
            fixture.portable_source_medication_portable_id
        );
        assert_eq!(
            take["taken_from_location_portable_id"],
            fixture.portable_source_location_portable_id
        );
        assert!(take["taken_at"].is_string());
        assert_eq!(take["dose_amount"], "2.0");
    }
    assert_eq!(
        rows(&source_before, "notification_preferences")[0]["person_portable_id"],
        fixture.portable_source_person_portable_id
    );

    let (bundle, export_request_id) = export(&target, &fixture);
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
    let source_audit = collection(
        &target,
        fixture.portable_source_household_id,
        &fixture.portable_source_access_token,
        "admin/audit_logs",
    );
    let export_audit =
        audit_for_request(&source_audit, &export_request_id, "portable_data.exported");
    assert_eq!(
        export_audit["actor_account_id"],
        fixture.portable_source_account_id
    );
    assert_eq!(
        export_audit["actor_membership_id"],
        fixture.portable_source_membership_id
    );
    assert_eq!(export_audit["metadata"]["encrypted"], true);
    assert_eq!(
        export_audit["metadata"]["export_mode"],
        "encrypted_migration_bundle"
    );
    assert_eq!(
        export_audit["metadata"]["record_counts"]["people"],
        rows(&source_before, "people").len()
    );
    assert!(!export_audit.to_string().contains(PASSPHRASE));

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
    assert_eq!(export_audit["metadata"]["record_counts"], plan["counts"]);
    assert_eq!(
        target_state(
            &target,
            fixture.portable_target_household_id,
            &fixture.portable_target_access_token
        ),
        target_state_before
    );

    let wrong_passphrase = assert_rejected_import(
        &target,
        &fixture,
        &target_state_before,
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
    assert!(!wrong_passphrase.to_string().contains("wrong secret"));
    let malformed = assert_rejected_import(
        &target,
        &fixture,
        &target_state_before,
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
    let numeric = assert_rejected_import(
        &target,
        &fixture,
        &target_state_before,
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
    let conflict = assert_rejected_import(
        &target,
        &fixture,
        &target_state_before,
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

    let applied = import(
        &target,
        fixture.portable_target_household_id,
        &fixture.portable_target_app_token,
        "portable_imports",
        PASSPHRASE,
        &bundle,
    );
    assert_eq!(applied.status().as_u16(), 201);
    let apply_request_id = applied.headers()["x-request-id"]
        .to_str()
        .expect("apply request ID")
        .to_owned();
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
    let import_audit = audit_for_request(&audit, &apply_request_id, "portable_data.imported");
    assert_eq!(
        import_audit["actor_account_id"],
        fixture.portable_target_account_id
    );
    assert_eq!(
        import_audit["actor_membership_id"],
        fixture.portable_target_membership_id
    );
    assert_eq!(import_audit["metadata"]["dry_run"], false);
    assert_eq!(import_audit["metadata"]["record_counts"], plan["counts"]);
    assert!(!import_audit.to_string().contains(PASSPHRASE));
    assert_imported_record_values(&source_before, &target_after);
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
    let (bundle, _) = export(&target, &fixture);
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
