use medtracker_contract_tests::{fixture, Target};
use serde_json::Value;
use std::env;

fn audit_rows_after(id: i64) -> Vec<Value> {
    let url = env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract audit database URL");
    let mut db = postgres::Client::connect(&url, postgres::NoTls).expect("audit database");
    db.query(
        "SELECT id, household_id, actor_account_id, actor_membership_id, request_id, event_type, metadata::text, audit_context::text, row_to_json(security_audit_events)::text AS complete_event FROM security_audit_events WHERE id > $1 ORDER BY id",
        &[&id],
    )
    .expect("audit rows")
    .into_iter()
    .map(|row| {
        serde_json::json!({
            "id": row.get::<_, i64>("id"),
            "household_id": row.get::<_, i64>("household_id"),
            "actor_account_id": row.get::<_, Option<i64>>("actor_account_id"),
            "actor_membership_id": row.get::<_, Option<i64>>("actor_membership_id"),
            "request_id": row.get::<_, Option<String>>("request_id"),
            "event_type": row.get::<_, String>("event_type"),
            "metadata": serde_json::from_str::<Value>(&row.get::<_, String>("metadata")).unwrap(),
            "audit_context": serde_json::from_str::<Value>(&row.get::<_, String>("audit_context")).unwrap(),
            "complete_event": row.get::<_, String>("complete_event")
        })
    })
    .collect()
}

fn latest_audit_id() -> i64 {
    let url = env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract audit database URL");
    let mut db = postgres::Client::connect(&url, postgres::NoTls).expect("audit database");
    db.query_one(
        "SELECT COALESCE(MAX(id), 0) FROM security_audit_events",
        &[],
    )
    .expect("latest audit row")
    .get(0)
}

#[test]
fn medication_reads_write_scoped_success_and_failure_audits_without_bearers() {
    let fixture = fixture();
    let target = Target::from_env();

    for (path, expected_status, action, outcome) in [
        (list_path(fixture.household_id), 200, "index", "success"),
        (
            show_path(fixture.household_id, fixture.managed_medication_id),
            200,
            "show",
            "success",
        ),
        (
            show_path(fixture.household_id, fixture.foreign_medication_id),
            404,
            "show",
            "failure",
        ),
    ] {
        let before = latest_audit_id();
        let response = target.get(&path, Some(&fixture.access_token));
        assert_eq!(response.status().as_u16(), expected_status);
        let events = audit_rows_after(before);
        assert_eq!(events.len(), 1, "one audit row for {path}");
        let event = &events[0];
        assert_eq!(event["household_id"], fixture.household_id);
        assert_eq!(event["actor_account_id"], fixture.account_id);
        assert_eq!(event["actor_membership_id"], fixture.owner_membership_id);
        assert_eq!(event["event_type"], "api.request");
        assert!(!event["request_id"].as_str().unwrap().is_empty());
        assert_eq!(event["metadata"]["http_method"], "GET");
        assert_eq!(event["metadata"]["controller"], "api/v1/medications");
        assert_eq!(event["metadata"]["action"], action);
        assert_eq!(event["metadata"]["status"], expected_status);
        assert_eq!(event["metadata"]["outcome"], outcome);
        assert_eq!(
            event["audit_context"]["authentication_method"],
            "api_session"
        );
        assert_eq!(event["audit_context"]["active_role"], "owner");
        assert_eq!(
            event["audit_context"]["actor_account_id"],
            fixture.account_id
        );
        assert_eq!(
            event["audit_context"]["actor_membership_id"],
            fixture.owner_membership_id
        );
        assert_eq!(event["audit_context"]["actor_user_id"], fixture.user_id);
        assert_eq!(event["audit_context"]["household_id"], fixture.household_id);
        assert_eq!(event["audit_context"]["request_id"], event["request_id"]);
        if expected_status == 200 {
            assert_eq!(event["audit_context"]["policy_class"], "MedicationPolicy");
            assert_eq!(event["audit_context"]["policy_query"], format!("{action}?"));
        } else {
            assert!(event["audit_context"].get("policy_class").is_none());
            assert!(event["audit_context"].get("policy_query").is_none());
        }
        assert_eq!(
            event["audit_context"]["session_reference"],
            format!("api_session:{}", fixture.session_id)
        );
        assert!(!event["complete_event"]
            .as_str()
            .unwrap()
            .contains(&fixture.access_token));
    }

    let before = latest_audit_id();
    assert_eq!(
        target
            .get(
                &list_path(fixture.foreign_household_id),
                Some(&fixture.access_token)
            )
            .status()
            .as_u16(),
        403
    );
    assert!(audit_rows_after(before).is_empty());
}

#[test]
fn authorized_conditional_read_writes_a_success_audit() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = show_path(fixture.household_id, fixture.managed_medication_id);
    let initial = target.get(&path, Some(&fixture.access_token));
    assert_eq!(initial.status().as_u16(), 200);
    let etag = initial.headers()["etag"].to_str().unwrap().to_owned();

    let before = latest_audit_id();
    let unchanged = target.get_with_header(&path, &fixture.access_token, "If-None-Match", &etag);
    assert_eq!(unchanged.status().as_u16(), 304);
    assert!(unchanged.bytes().unwrap().is_empty());
    let events = audit_rows_after(before);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["metadata"]["status"], 304);
    assert_eq!(events[0]["metadata"]["outcome"], "success");
    assert_eq!(events[0]["metadata"]["action"], "show");
}

fn list_path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/medications")
}

fn show_path(household_id: i64, medication_id: i64) -> String {
    format!("{}/{}", list_path(household_id), medication_id)
}

#[test]
fn owner_reads_household_medications_and_foreign_record_is_hidden() {
    let fixture = fixture();
    let target = Target::from_env();
    let list = target.get(
        &list_path(fixture.household_id),
        Some(&fixture.access_token),
    );
    assert_eq!(list.status().as_u16(), 200);
    let body: Value = list.json().unwrap();
    let medications = body["data"].as_array().unwrap();
    assert!(medications
        .iter()
        .any(|medication| medication["id"] == fixture.managed_medication_id));
    assert_eq!(body["meta"]["page"], 1);
    assert_eq!(body["meta"]["per_page"], 20);

    let show = target.get(
        &show_path(fixture.household_id, fixture.managed_medication_id),
        Some(&fixture.access_token),
    );
    assert_eq!(show.status().as_u16(), 200);
    let body: Value = show.json().unwrap();
    assert_eq!(body["data"]["id"], fixture.managed_medication_id);
    assert_eq!(body["data"]["name"], fixture.managed_medication_name);
    assert_eq!(
        body["data"]["portable_id"],
        fixture.managed_medication_portable_id
    );
    assert_eq!(body["data"]["current_supply"], "50.0");
    assert_eq!(body["data"]["reorder_threshold"], "5.0");

    let portable = target.get(
        &format!(
            "{}/{}",
            list_path(fixture.household_id),
            fixture.managed_medication_portable_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(portable.status().as_u16(), 200);
    let body: Value = portable.json().unwrap();
    assert_eq!(body["data"]["id"], fixture.managed_medication_id);

    let foreign = target.get(
        &show_path(fixture.household_id, fixture.foreign_medication_id),
        Some(&fixture.access_token),
    );
    assert_eq!(foreign.status().as_u16(), 404);
    let body: Value = foreign.json().unwrap();
    assert_eq!(body["error"]["code"], "not_found");
}

#[test]
fn delegated_viewer_only_reads_granted_medication() {
    let fixture = fixture();
    let target = Target::from_env();
    let list = target.get(
        &list_path(fixture.household_id),
        Some(&fixture.view_access_token),
    );
    assert_eq!(list.status().as_u16(), 200);
    let body: Value = list.json().unwrap();
    let ids: Vec<i64> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|medication| medication["id"].as_i64().unwrap())
        .collect();
    assert!(ids.contains(&fixture.managed_medication_id));
    assert!(!ids.contains(&fixture.hidden_medication_id));
    assert_eq!(
        target
            .get(
                &show_path(fixture.household_id, fixture.hidden_medication_id),
                Some(&fixture.view_access_token)
            )
            .status()
            .as_u16(),
        404
    );
}

#[test]
fn delegated_creator_reads_own_unlinked_medication() {
    let fixture = fixture();
    let target = Target::from_env();
    let list = target.get(
        &list_path(fixture.medication_read_household_id),
        Some(&fixture.medication_read_delegated_access_token),
    );
    assert_eq!(list.status().as_u16(), 200);
    let body: Value = list.json().unwrap();
    let ids: Vec<i64> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|medication| medication["id"].as_i64().unwrap())
        .collect();
    assert!(ids.contains(&fixture.medication_read_unlinked_id));
    assert!(!ids.contains(&fixture.medication_read_hidden_id));
    assert_eq!(
        target
            .get(
                &show_path(
                    fixture.medication_read_household_id,
                    fixture.medication_read_unlinked_id
                ),
                Some(&fixture.medication_read_delegated_access_token)
            )
            .status()
            .as_u16(),
        200
    );
}

#[test]
fn secondary_membership_authenticates_with_account_user_in_another_household() {
    let fixture = fixture();
    let target = Target::from_env();
    let response = target.get(
        &list_path(fixture.medication_read_household_id),
        Some(&fixture.medication_read_secondary_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().unwrap();
    assert_eq!(body["meta"]["total_count"], 0);
}

#[test]
fn updated_since_filters_before_count_and_rejects_invalid_timestamps() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = list_path(fixture.household_id);
    let old = target.get(
        &format!("{path}?updated_since=2000-01-01T00%3A00%3A00Z"),
        Some(&fixture.access_token),
    );
    assert_eq!(old.status().as_u16(), 200);
    let old_body: Value = old.json().unwrap();
    assert!(old_body["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|medication| { medication["id"] == fixture.managed_medication_id }));

    let future = target.get(
        &format!("{path}?updated_since=2100-01-01T00%3A00%3A00Z"),
        Some(&fixture.access_token),
    );
    assert_eq!(future.status().as_u16(), 200);
    let future_body: Value = future.json().unwrap();
    assert_eq!(future_body["meta"]["total_count"], 0);
    assert!(future_body["data"].as_array().unwrap().is_empty());

    let invalid = target.get(
        &format!("{path}?updated_since=not-a-timestamp"),
        Some(&fixture.access_token),
    );
    assert_eq!(invalid.status().as_u16(), 422);
    let invalid_body: Value = invalid.json().unwrap();
    assert_eq!(invalid_body["error"]["code"], "unprocessable_content");
}

#[test]
fn show_has_representation_etag_and_conditional_get_after_scoped_lookup() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = show_path(fixture.household_id, fixture.managed_medication_id);
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let etag = response.headers()["etag"].to_str().unwrap().to_owned();
    assert!(etag.starts_with('"') && etag.ends_with('"'));
    assert_eq!(etag.len(), 66);

    let portable_path = format!(
        "{}/{}",
        list_path(fixture.household_id),
        fixture.managed_medication_portable_id
    );
    let portable = target.get(&portable_path, Some(&fixture.access_token));
    assert_eq!(portable.status().as_u16(), 200);
    assert_eq!(portable.headers()["etag"], etag);

    let unchanged = target.get_with_header(&path, &fixture.access_token, "If-None-Match", &etag);
    assert_eq!(unchanged.status().as_u16(), 304);
    assert!(unchanged.bytes().unwrap().is_empty());

    for value in [
        format!("W/{etag}"),
        format!("\"stale\", {etag}"),
        "*".to_owned(),
    ] {
        let unchanged =
            target.get_with_header(&path, &fixture.access_token, "If-None-Match", &value);
        assert_eq!(unchanged.status().as_u16(), 304, "{value}");
        assert_eq!(unchanged.headers()["etag"], etag);
        assert!(unchanged.bytes().unwrap().is_empty());
    }

    let stale = target.get_with_header(&path, &fixture.access_token, "If-None-Match", "\"stale\"");
    assert_eq!(stale.status().as_u16(), 200);
    assert_eq!(stale.headers()["etag"], etag);

    let hidden = target.get_with_header(
        &show_path(fixture.household_id, fixture.foreign_medication_id),
        &fixture.access_token,
        "If-None-Match",
        &etag,
    );
    assert_eq!(hidden.status().as_u16(), 404);

    let expired =
        target.get_with_header(&path, &fixture.expired_access_token, "If-None-Match", &etag);
    assert_eq!(expired.status().as_u16(), 401);
}

#[test]
fn invalid_session_and_household_boundary_are_denied() {
    let fixture = fixture();
    let target = Target::from_env();
    assert_eq!(
        target
            .get(&list_path(fixture.household_id), None)
            .status()
            .as_u16(),
        401
    );
    assert_eq!(
        target
            .get(
                &list_path(fixture.household_id),
                Some(&fixture.expired_access_token)
            )
            .status()
            .as_u16(),
        401
    );
    assert_eq!(
        target
            .get(
                &list_path(fixture.household_id),
                Some(&fixture.medication_read_revoked_access_token)
            )
            .status()
            .as_u16(),
        401
    );
    assert_eq!(
        target
            .get(
                &list_path(fixture.household_id),
                Some(&fixture.medication_read_stale_access_token)
            )
            .status()
            .as_u16(),
        401
    );
    assert_eq!(
        target
            .get(
                &list_path(fixture.household_id),
                Some(&fixture.locked_access_token)
            )
            .status()
            .as_u16(),
        401
    );
    assert_eq!(
        target
            .get(
                &list_path(fixture.auth_inactive_household_id),
                Some(&fixture.auth_inactive_access_token)
            )
            .status()
            .as_u16(),
        401
    );
    assert_eq!(
        target
            .get(
                &list_path(fixture.auth_suspended_household_id),
                Some(&fixture.auth_suspended_access_token)
            )
            .status()
            .as_u16(),
        401
    );
    assert_eq!(
        target
            .get(
                &list_path(fixture.household_id),
                Some(&fixture.revocable_access_token)
            )
            .status()
            .as_u16(),
        200
    );
    assert_eq!(
        target
            .get(
                &list_path(fixture.foreign_household_id),
                Some(&fixture.access_token)
            )
            .status()
            .as_u16(),
        403
    );
    let held = &fixture.auth_operational_states["held"];
    assert_eq!(
        target
            .get(
                &list_path(held["household_id"].as_i64().unwrap()),
                held["access_token"].as_str()
            )
            .status()
            .as_u16(),
        403
    );
}
