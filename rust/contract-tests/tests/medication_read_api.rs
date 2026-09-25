use medtracker_contract_tests::{fixture, Target};
use serde_json::Value;

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
