use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::{json, Value};

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn path(fixture: &Fixture, suffix: &str) -> String {
    format!("/api/v1/households/{}/admin/{suffix}", fixture.household_id)
}

fn membership(list: &Value, id: i64) -> &Value {
    list["data"]
        .as_array()
        .expect("membership data array")
        .iter()
        .find(|row| row["id"] == id)
        .expect("membership in public list")
}

fn request_id(response: &Response) -> String {
    response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned()
}

fn audit_event(target: &Target, fixture: &Fixture, id: &str, event_type: &str) -> Value {
    let response = target.get(&path(fixture, "audit_logs"), Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    body(response)["data"]
        .as_array()
        .expect("audit data array")
        .iter()
        .find(|row| row["request_id"] == id && row["event_type"] == event_type)
        .cloned()
        .expect("request-correlated audit event")
}

#[test]
fn settings_authority_validation_and_public_readback() {
    let target = Target::from_env();
    let fixture = fixture();
    let settings = path(&fixture, "settings");
    for token in [&fixture.access_token, &fixture.manager_access_token] {
        let response = target.get(&settings, Some(token));
        assert_eq!(response.status().as_u16(), 200);
        assert!(response.headers().contains_key("etag"));
        let data = body(response)["data"].clone();
        assert_eq!(data["id"], fixture.household_id);
        assert_eq!(data["name"], fixture.household_name);
        assert!(data["slug"].is_string());
        assert!(data["timezone"].is_string());
        assert!(data["subscription_plan"].is_string());
        assert!(data["updated_at"].is_string());
    }
    let response = target.get(&settings, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 403);
    assert_eq!(body(response)["error"]["code"], "forbidden");
    let response = target.get(&settings, Some(&fixture.foreign_access_token));
    assert_eq!(response.status().as_u16(), 403);
    let response = target.get(
        &format!(
            "/api/v1/households/{}/admin/settings",
            fixture.foreign_household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 403);
    let response = target.get(
        "/api/v1/households/999999999/admin/settings",
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 404);

    let before = body(target.get(&settings, Some(&fixture.access_token)))["data"].clone();
    let response = target.patch_json(
        &settings,
        &fixture.manager_access_token,
        &json!({"household": {"name": "Contract admin renamed"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let id = request_id(&response);
    assert_eq!(body(response)["data"]["name"], "Contract admin renamed");
    let event = audit_event(
        &target,
        &fixture,
        &id,
        "api/admin/household_settings/updated",
    );
    assert_eq!(event["metadata"]["outcome"], "success");
    assert_eq!(event["metadata"]["target_id"], fixture.household_id);
    let updated = body(target.get(&settings, Some(&fixture.access_token)))["data"].clone();
    assert_eq!(updated["name"], "Contract admin renamed");
    assert_eq!(updated["timezone"], before["timezone"]);

    let response = target.put_json_if_match(
        &settings,
        &fixture.access_token,
        &json!({"household": {"timezone": "Europe/Paris"}}),
        "\"stale\"",
    );
    assert_eq!(response.status().as_u16(), 200);
    let after = body(target.get(&settings, Some(&fixture.access_token)))["data"].clone();
    assert_eq!(after["name"], "Contract admin renamed");
    assert_eq!(after["timezone"], "Europe/Paris");

    for (token, payload, status) in [
        (
            &fixture.access_token,
            json!({"household": {"name": ""}}),
            422,
        ),
        (
            &fixture.manager_access_token,
            json!({"household": {"subscription_plan": "invalid"}}),
            422,
        ),
        (
            &fixture.view_access_token,
            json!({"household": {"name": "Forbidden"}}),
            403,
        ),
    ] {
        let response = target.patch_json(&settings, token, &payload);
        assert_eq!(response.status().as_u16(), status);
        assert!(body(response)["error"]["code"].is_string());
        assert_eq!(
            body(target.get(&settings, Some(&fixture.access_token)))["data"],
            after
        );
    }
    let response = target.put_json(
        &settings,
        &fixture.manager_access_token,
        &json!({"household": {"name": ""}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(
        body(target.get(&settings, Some(&fixture.access_token)))["data"],
        after
    );
    let response = target.put_json(
        &settings,
        &fixture.view_access_token,
        &json!({"household": {"name": "Forbidden"}}),
    );
    assert_eq!(response.status().as_u16(), 403);
    let response = target.patch_json(
        &settings,
        &fixture.foreign_access_token,
        &json!({"household": {"name": "Foreign"}}),
    );
    assert_eq!(response.status().as_u16(), 403);
    assert_eq!(
        body(target.get(&settings, Some(&fixture.access_token)))["data"],
        after
    );
    let response = target.patch_json(
        &settings,
        &fixture.access_token,
        &json!({"household": {"name": before["name"], "timezone": before["timezone"]}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let restored = body(target.get(&settings, Some(&fixture.access_token)))["data"].clone();
    assert_eq!(restored["name"], before["name"]);
    assert_eq!(restored["timezone"], before["timezone"]);
}

#[test]
fn membership_list_authority_envelope_and_non_disclosure() {
    let target = Target::from_env();
    let fixture = fixture();
    let list_path = path(&fixture, "memberships");
    for token in [&fixture.access_token, &fixture.manager_access_token] {
        let response = target.get(&list_path, Some(token));
        assert_eq!(response.status().as_u16(), 200);
        let list = body(response);
        let rows = list["data"].as_array().expect("membership data array");
        assert!(rows.len() >= 4);
        assert!(list.get("meta").is_none());
        assert!(rows.iter().all(|row| row["id"].is_number()
            && row["email"].is_string()
            && row["role"].is_string()
            && row["status"].is_string()
            && row["permissions_version"].is_number()));
        assert_eq!(
            membership(&list, fixture.manager_membership_id)["role"],
            "administrator"
        );
        assert_eq!(
            membership(&list, fixture.view_membership_id)["role"],
            "member"
        );
        assert!(!rows
            .iter()
            .any(|row| row["id"] == fixture.foreign_membership_id));
    }
    let response = target.get(&list_path, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 403);
    let response = target.get(&list_path, Some(&fixture.foreign_access_token));
    assert_eq!(response.status().as_u16(), 403);
}

#[test]
fn membership_patch_and_put_change_access_and_invalidate_target_session() {
    let target = Target::from_env();
    let fixture = fixture();
    let list_path = path(&fixture, "memberships");
    let item = format!("{list_path}/{}", fixture.admin_target_membership_id);
    let before = body(target.get(&list_path, Some(&fixture.access_token)));
    let version = membership(&before, fixture.admin_target_membership_id)["permissions_version"]
        .as_i64()
        .unwrap();
    let response = target.get(
        "/api/v1/auth/households",
        Some(&fixture.admin_target_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let response = target.patch_json(
        &item,
        &fixture.manager_access_token,
        &json!({"household_membership": {"role": "administrator"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let id = request_id(&response);
    assert_eq!(body(response)["data"]["role"], "administrator");
    let event = audit_event(&target, &fixture, &id, "household_membership.role_updated");
    assert_eq!(
        event["metadata"]["target_membership_id"],
        fixture.admin_target_membership_id
    );
    assert_eq!(event["metadata"]["outcome"], "success");
    let list = body(target.get(&list_path, Some(&fixture.access_token)));
    let row = membership(&list, fixture.admin_target_membership_id);
    assert_eq!(row["role"], "administrator");
    assert_eq!(row["permissions_version"], version + 1);
    let response = target.get(
        "/api/v1/auth/households",
        Some(&fixture.admin_target_access_token),
    );
    assert_eq!(response.status().as_u16(), 401);

    let response = target.put_json_if_match(
        &item,
        &fixture.access_token,
        &json!({"household_membership": {"status": "suspended"}}),
        "\"stale\"",
    );
    assert_eq!(response.status().as_u16(), 200);
    let id = request_id(&response);
    assert_eq!(body(response)["data"]["status"], "suspended");
    let event = audit_event(
        &target,
        &fixture,
        &id,
        "household_access.membership_changed",
    );
    assert_eq!(event["metadata"]["outcome"], "success");
    let list = body(target.get(&list_path, Some(&fixture.access_token)));
    let row = membership(&list, fixture.admin_target_membership_id);
    assert_eq!(row["status"], "suspended");
    assert_eq!(row["role"], "administrator");
    assert_eq!(row["permissions_version"], version + 2);
}

#[test]
fn invalid_and_forbidden_membership_updates_preserve_public_state() {
    let target = Target::from_env();
    let fixture = fixture();
    let list_path = path(&fixture, "memberships");
    let item = format!("{list_path}/{}", fixture.admin_invalid_membership_id);
    let before = body(target.get(&list_path, Some(&fixture.access_token)));
    let original = membership(&before, fixture.admin_invalid_membership_id).clone();
    for (token, payload, status) in [
        (
            &fixture.access_token,
            json!({"household_membership": {"role": "invalid"}}),
            422,
        ),
        (
            &fixture.manager_access_token,
            json!({"household_membership": {"role": "owner"}}),
            422,
        ),
        (
            &fixture.view_access_token,
            json!({"household_membership": {"status": "revoked"}}),
            403,
        ),
        (
            &fixture.foreign_access_token,
            json!({"household_membership": {"status": "revoked"}}),
            403,
        ),
    ] {
        let response = target.patch_json(&item, token, &payload);
        assert_eq!(response.status().as_u16(), status);
        assert!(body(response)["error"]["code"].is_string());
        let list = body(target.get(&list_path, Some(&fixture.access_token)));
        assert_eq!(
            membership(&list, fixture.admin_invalid_membership_id),
            &original
        );
    }
    let response = target.put_json(
        &item,
        &fixture.access_token,
        &json!({"household_membership": {"status": "invalid"}}),
    );
    assert_eq!(response.status().as_u16(), 422);
    let response = target.put_json(
        &item,
        &fixture.view_access_token,
        &json!({"household_membership": {"status": "suspended"}}),
    );
    assert_eq!(response.status().as_u16(), 403);
    let response = target.delete(&item, Some(&fixture.view_access_token));
    assert_eq!(response.status().as_u16(), 403);
    let list = body(target.get(&list_path, Some(&fixture.access_token)));
    assert_eq!(
        membership(&list, fixture.admin_invalid_membership_id),
        &original
    );
    let foreign = format!("{list_path}/{}", fixture.foreign_membership_id);
    let response = target.put_json(
        &foreign,
        &fixture.access_token,
        &json!({"household_membership": {"status": "suspended"}}),
    );
    assert_eq!(response.status().as_u16(), 404);
    let response = target.delete(&foreign, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 404);
    let missing = format!("{list_path}/999999999");
    let response = target.patch_json(
        &missing,
        &fixture.access_token,
        &json!({"household_membership": {"status": "suspended"}}),
    );
    assert_eq!(response.status().as_u16(), 404);
}

#[test]
fn deleting_membership_revokes_access_and_last_owner_remains_active() {
    let target = Target::from_env();
    let fixture = fixture();
    let list_path = path(&fixture, "memberships");
    let item = format!("{list_path}/{}", fixture.admin_revoke_membership_id);
    let response = target.get(
        "/api/v1/auth/households",
        Some(&fixture.admin_revoke_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let response = target.delete(&item, Some(&fixture.manager_access_token));
    assert_eq!(response.status().as_u16(), 204);
    let id = request_id(&response);
    let event = audit_event(
        &target,
        &fixture,
        &id,
        "household_access.membership_changed",
    );
    assert_eq!(event["metadata"]["outcome"], "success");
    let list = body(target.get(&list_path, Some(&fixture.access_token)));
    assert_eq!(
        membership(&list, fixture.admin_revoke_membership_id)["status"],
        "revoked"
    );
    let response = target.get(
        "/api/v1/auth/households",
        Some(&fixture.admin_revoke_access_token),
    );
    assert_eq!(response.status().as_u16(), 401);

    let last_list = format!(
        "/api/v1/households/{}/admin/memberships",
        fixture.last_owner_household_id
    );
    let item = format!("{last_list}/{}", fixture.last_owner_membership_id);
    let response = target.delete(&item, Some(&fixture.last_owner_access_token));
    assert_eq!(response.status().as_u16(), 422);
    let id = request_id(&response);
    assert!(body(response)["error"]["errors"]["base"].is_array());
    let audit_path = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.last_owner_household_id
    );
    let response = target.get(&audit_path, Some(&fixture.last_owner_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let audit = body(response);
    let rejection = audit["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            row["request_id"] == id && row["event_type"] == "household_access.membership_changed"
        })
        .expect("request-correlated rejected membership audit");
    assert_eq!(rejection["metadata"]["outcome"], "rejected");
    let response = target.get(&last_list, Some(&fixture.last_owner_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let list = body(response);
    assert_eq!(
        membership(&list, fixture.last_owner_membership_id)["status"],
        "active"
    );
    let response = target.get(
        "/api/v1/auth/households",
        Some(&fixture.last_owner_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
}
