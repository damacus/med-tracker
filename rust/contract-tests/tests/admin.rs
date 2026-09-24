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

fn assert_membership_envelope(row: &Value) {
    let object = row.as_object().expect("membership object");
    assert_eq!(object.len(), 9);
    for field in [
        "id",
        "account_id",
        "email",
        "person_id",
        "person_name",
        "role",
        "status",
        "permissions_version",
        "joined_at",
    ] {
        assert!(
            object.contains_key(field),
            "missing membership field {field}"
        );
    }
    assert!(row["id"].is_i64());
    assert!(row["account_id"].is_i64());
    assert!(row["email"].is_string());
    assert!(row["person_id"].is_null() || row["person_id"].is_i64());
    assert!(row["person_name"].is_null() || row["person_name"].is_string());
    assert!(row["role"].is_string());
    assert!(row["status"].is_string());
    assert!(row["permissions_version"].is_u64());
    assert!(row["joined_at"].is_null() || row["joined_at"].is_string());
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

    let response = target.patch_json(
        &settings,
        &fixture.access_token,
        &json!({"household": {"name": "Contract owner renamed"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["data"]["name"], "Contract owner renamed");
    assert_eq!(
        body(target.get(&settings, Some(&fixture.manager_access_token)))["data"]["name"],
        "Contract owner renamed"
    );

    let response = target.put_json_if_match(
        &settings,
        &fixture.access_token,
        &json!({"household": {"timezone": "Europe/Paris"}}),
        "\"stale\"",
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["data"]["timezone"], "Europe/Paris");
    let response = target.put_json(
        &settings,
        &fixture.manager_access_token,
        &json!({"household": {"timezone": "Europe/Berlin"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["data"]["timezone"], "Europe/Berlin");
    let after = body(target.get(&settings, Some(&fixture.access_token)))["data"].clone();
    assert_eq!(after["name"], "Contract owner renamed");
    assert_eq!(after["timezone"], "Europe/Berlin");

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
        for row in rows {
            assert_membership_envelope(row);
        }
        assert_eq!(
            membership(&list, fixture.manager_membership_id)["role"],
            "administrator"
        );
        assert_eq!(
            membership(&list, fixture.view_membership_id)["role"],
            "member"
        );
        let unlinked = membership(&list, fixture.grant_target_membership_id);
        assert!(unlinked["person_id"].is_null());
        assert!(unlinked["person_name"].is_null());
        assert!(unlinked["joined_at"].is_string());
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
    let changed = body(response);
    assert_membership_envelope(&changed["data"]);
    assert_eq!(changed["data"]["role"], "administrator");
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

    let put_item = format!("{list_path}/{}", fixture.admin_manager_put_membership_id);
    let before = body(target.get(&list_path, Some(&fixture.access_token)));
    let original = membership(&before, fixture.admin_manager_put_membership_id);
    let original_version = original["permissions_version"].as_i64().unwrap();
    assert_eq!(original["status"], "active");
    let response = target.get(
        "/api/v1/auth/households",
        Some(&fixture.admin_manager_put_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let response = target.put_json_if_match(
        &put_item,
        &fixture.manager_access_token,
        &json!({"household_membership": {"status": "suspended"}}),
        "\"stale\"",
    );
    assert_eq!(response.status().as_u16(), 200);
    let id = request_id(&response);
    let changed = body(response);
    assert_membership_envelope(&changed["data"]);
    assert_eq!(changed["data"]["status"], "suspended");
    let event = audit_event(
        &target,
        &fixture,
        &id,
        "household_access.membership_changed",
    );
    assert_eq!(event["metadata"]["outcome"], "success");
    let list = body(target.get(&list_path, Some(&fixture.access_token)));
    let row = membership(&list, fixture.admin_manager_put_membership_id);
    assert_eq!(row["status"], "suspended");
    assert_eq!(row["role"], "member");
    assert_eq!(row["permissions_version"], original_version + 1);
    let response = target.get(
        "/api/v1/auth/households",
        Some(&fixture.admin_manager_put_access_token),
    );
    assert_eq!(response.status().as_u16(), 401);
}

#[test]
fn owner_can_patch_and_put_distinct_memberships() {
    let target = Target::from_env();
    let fixture = fixture();
    let list_path = path(&fixture, "memberships");
    let patch_item = format!("{list_path}/{}", fixture.admin_owner_patch_membership_id);
    let response = target.patch_json(
        &patch_item,
        &fixture.access_token,
        &json!({"household_membership": {"role": "administrator"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let changed = body(response);
    assert_membership_envelope(&changed["data"]);
    assert_eq!(changed["data"]["role"], "administrator");
    let list = body(target.get(&list_path, Some(&fixture.manager_access_token)));
    assert_eq!(
        membership(&list, fixture.admin_owner_patch_membership_id)["role"],
        "administrator"
    );

    let put_item = format!("{list_path}/{}", fixture.admin_owner_put_membership_id);
    let response = target.get(
        "/api/v1/auth/households",
        Some(&fixture.admin_owner_put_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let response = target.put_json(
        &put_item,
        &fixture.access_token,
        &json!({"household_membership": {"status": "suspended"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let changed = body(response);
    assert_membership_envelope(&changed["data"]);
    assert_eq!(changed["data"]["status"], "suspended");
    let list = body(target.get(&list_path, Some(&fixture.manager_access_token)));
    assert_eq!(
        membership(&list, fixture.admin_owner_put_membership_id)["status"],
        "suspended"
    );
    let response = target.get(
        "/api/v1/auth/households",
        Some(&fixture.admin_owner_put_access_token),
    );
    assert_eq!(response.status().as_u16(), 401);
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
    let foreign_list = format!(
        "/api/v1/households/{}/admin/memberships",
        fixture.foreign_household_id
    );
    let response = target.get(&foreign_list, Some(&fixture.foreign_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let foreign_before = body(response);
    let foreign_original = membership(&foreign_before, fixture.foreign_membership_id).clone();
    let foreign = format!("{list_path}/{}", fixture.foreign_membership_id);
    let response = target.put_json(
        &foreign,
        &fixture.access_token,
        &json!({"household_membership": {"status": "suspended"}}),
    );
    assert_eq!(response.status().as_u16(), 404);
    let response = target.delete(&foreign, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 404);
    let response = target.get(&foreign_list, Some(&fixture.foreign_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let foreign_after = body(response);
    assert_eq!(
        membership(&foreign_after, fixture.foreign_membership_id),
        &foreign_original
    );
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
    let before = body(target.get(&last_list, Some(&fixture.last_owner_access_token)));
    let original = membership(&before, fixture.last_owner_membership_id).clone();
    let audit_path = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.last_owner_household_id
    );
    let rejected = [
        (
            target.patch_json(
                &item,
                &fixture.last_owner_access_token,
                &json!({"household_membership": {"role": "member"}}),
            ),
            "household_membership.role_updated",
        ),
        (
            target.put_json(
                &item,
                &fixture.last_owner_access_token,
                &json!({"household_membership": {"status": "suspended"}}),
            ),
            "household_access.membership_changed",
        ),
        (
            target.delete(&item, Some(&fixture.last_owner_access_token)),
            "household_access.membership_changed",
        ),
    ];
    for (response, event_type) in rejected {
        assert_eq!(response.status().as_u16(), 422);
        let id = request_id(&response);
        assert!(body(response)["error"]["errors"]["base"].is_array());
        let response = target.get(&audit_path, Some(&fixture.last_owner_access_token));
        assert_eq!(response.status().as_u16(), 200);
        let audit = body(response);
        let rejection = audit["data"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["request_id"] == id && row["event_type"] == event_type)
            .expect("request-correlated rejected membership audit");
        assert_eq!(rejection["metadata"]["outcome"], "rejected");
        let current = body(target.get(&last_list, Some(&fixture.last_owner_access_token)));
        assert_eq!(
            membership(&current, fixture.last_owner_membership_id),
            &original
        );
        let response = target.get(
            "/api/v1/auth/households",
            Some(&fixture.last_owner_access_token),
        );
        assert_eq!(response.status().as_u16(), 200);
    }
    let response = target.get(&last_list, Some(&fixture.last_owner_access_token));
    assert_eq!(response.status().as_u16(), 200);
    let list = body(response);
    assert_eq!(
        membership(&list, fixture.last_owner_membership_id),
        &original
    );
    let response = target.get(
        "/api/v1/auth/households",
        Some(&fixture.last_owner_access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
}
