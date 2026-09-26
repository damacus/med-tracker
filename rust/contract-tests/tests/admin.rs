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

fn assert_api_error(response: Response, status: u16, code: &str) -> Value {
    assert_eq!(response.status().as_u16(), status);
    assert_eq!(
        response.headers()["content-type"]
            .to_str()
            .expect("JSON content type")
            .split(';')
            .next()
            .unwrap(),
        "application/json"
    );
    let request_id = request_id(&response);
    let payload = body(response);
    assert_eq!(payload["error"]["code"], code);
    assert_eq!(payload["error"]["request_id"], request_id);
    assert!(payload["error"]["message"]
        .as_str()
        .is_some_and(|message| !message.is_empty()));
    assert!(payload.get("data").is_none());
    payload
}

fn app_token(list: &Value, id: i64) -> &Value {
    list["data"]
        .as_array()
        .expect("app token array")
        .iter()
        .find(|row| row["id"] == id)
        .expect("app token in public list")
}

fn assert_token_summary(row: &Value) {
    let object = row.as_object().expect("app token summary");
    assert_eq!(object.len(), 6);
    assert!(row["id"].is_i64());
    assert!(row["name"].is_string());
    assert!(row["last_used_at"].is_string());
    assert!(row["expires_at"].is_string());
    assert!(row["revoked_at"].is_null() || row["revoked_at"].is_string());
    assert!(row["permissions_version"].is_u64());
    assert!(row.get("token").is_none());
    assert!(row.get("token_digest").is_none());
}

#[test]
fn task_6a2_app_tokens_issue_once_list_without_secret_and_revoke_bearer_access() {
    let target = Target::from_env();
    let fixture = fixture();
    let tokens = path(&fixture, "app_tokens");
    let response = target.post_json_authorized(
        &tokens,
        &fixture.access_token,
        &json!({"api_app_token": {"name": "Contract owner CLI"}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let create_id = request_id(&response);
    let created = body(response);
    let issued = &created["data"];
    let id = issued["id"].as_i64().expect("new app token ID");
    let raw = issued["token"].as_str().expect("one-time raw token");
    assert!(raw.starts_with("mt_app_"));
    assert_eq!(issued.as_object().unwrap().len(), 7);
    assert_eq!(issued["name"], "Contract owner CLI");
    assert!(issued["expires_at"].is_string());
    assert!(issued["revoked_at"].is_null());

    let household_me = format!("/api/v1/households/{}/me", fixture.household_id);
    let response = target.get(&household_me, Some(raw));
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["data"]["membership_role"], "owner");
    let response = target.get(&tokens, Some(raw));
    assert_eq!(response.status().as_u16(), 200);
    let own_list = body(response);
    let summary = app_token(&own_list, id);
    assert_token_summary(summary);
    assert_eq!(summary["name"], "Contract owner CLI");
    assert_eq!(summary["expires_at"], issued["expires_at"]);
    assert!(!own_list.to_string().contains(raw));
    let response = target.get(&tokens, Some(&fixture.manager_access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert!(!body(response)["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == id));

    let created_audit = audit_event(
        &target,
        &fixture,
        &create_id,
        "auth_token/api_app_token/created",
    );
    assert_eq!(created_audit["actor_account_id"], fixture.account_id);
    assert_eq!(
        created_audit["actor_membership_id"],
        fixture.owner_membership_id
    );
    assert!(!created_audit.to_string().contains(raw));
    let item = format!("{tokens}/{id}");
    let response = target.delete(&item, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 204);
    let delete_id = request_id(&response);
    let revoked_audit = audit_event(
        &target,
        &fixture,
        &delete_id,
        "auth_token/api_app_token/revoked",
    );
    assert!(!revoked_audit.to_string().contains(raw));
    let response = target.get(&household_me, Some(raw));
    assert_api_error(response, 401, "unauthorized");
    let response = target.delete(&item, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 204);
    let response = target.get(&tokens, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let revoked = body(response);
    assert!(app_token(&revoked, id)["revoked_at"].is_string());
    assert!(!revoked.to_string().contains(raw));
}

#[test]
fn task_6a2_app_token_rechecks_current_membership_authority() {
    let target = Target::from_env();
    let fixture = fixture();
    let tokens = path(&fixture, "app_tokens");
    let response = target.post_json_authorized(
        &tokens,
        &fixture.token_authority_access_token,
        &json!({"api_app_token": {"name": "Authority-bound manager token"}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let issued = body(response);
    let raw = issued["data"]["token"].as_str().unwrap();
    let response = target.get(&tokens, Some(raw));
    assert_eq!(response.status().as_u16(), 200);
    let member = format!(
        "{}/{}",
        path(&fixture, "memberships"),
        fixture.token_authority_membership_id
    );
    let response = target.patch_json(
        &member,
        &fixture.access_token,
        &json!({"household_membership": {"role": "member"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(body(response)["data"]["role"], "member");
    assert_api_error(target.get(&tokens, Some(raw)), 401, "unauthorized");
    assert_api_error(
        target.get(&tokens, Some(&fixture.token_authority_access_token)),
        401,
        "unauthorized",
    );
}

#[test]
fn task_6a2_app_token_authority_and_failure_envelopes() {
    let target = Target::from_env();
    let fixture = fixture();
    let tokens = path(&fixture, "app_tokens");
    for actor in [
        &fixture.access_token,
        &fixture.manager_access_token,
        &fixture.manager_app_token,
    ] {
        let response = target.get(&tokens, Some(actor));
        assert_eq!(response.status().as_u16(), 200);
        assert!(body(response)["data"].is_array());
    }
    let manager_list = body(target.get(&tokens, Some(&fixture.manager_app_token)));
    assert_token_summary(app_token(&manager_list, fixture.manager_app_token_id));
    assert!(!manager_list
        .to_string()
        .contains(&fixture.manager_app_token));
    assert!(!manager_list["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| { row["id"] == fixture.revoked_owner_app_token_id }));
    let owner_list = body(target.get(&tokens, Some(&fixture.access_token)));
    assert!(app_token(&owner_list, fixture.revoked_owner_app_token_id)["revoked_at"].is_string());
    assert!(!owner_list["data"].as_array().unwrap().iter().any(|row| {
        row["id"] == fixture.manager_app_token_id || row["id"] == fixture.foreign_app_token_id
    }));

    assert_api_error(target.get(&tokens, None), 401, "unauthorized");
    assert_api_error(
        target.get(&tokens, Some(&fixture.view_access_token)),
        403,
        "forbidden",
    );
    assert_api_error(
        target.get(&tokens, Some(&fixture.foreign_access_token)),
        403,
        "forbidden",
    );
    let payload = json!({"api_app_token": {"name": "Forbidden app token"}});
    assert_api_error(
        target.post_json_authorized(&tokens, &fixture.view_access_token, &payload),
        403,
        "forbidden",
    );
    assert_api_error(
        target.post_json_authorized(&tokens, &fixture.foreign_access_token, &payload),
        403,
        "forbidden",
    );
    let manager_item = format!("{tokens}/{}", fixture.manager_app_token_id);
    let foreign_item = format!("{tokens}/{}", fixture.foreign_app_token_id);
    assert_api_error(
        target.delete(&manager_item, Some(&fixture.access_token)),
        404,
        "not_found",
    );
    assert_api_error(
        target.delete(&foreign_item, Some(&fixture.access_token)),
        404,
        "not_found",
    );
    assert_api_error(
        target.delete(&manager_item, Some(&fixture.view_access_token)),
        403,
        "forbidden",
    );
    assert_api_error(
        target.delete(&manager_item, Some(&fixture.foreign_access_token)),
        403,
        "forbidden",
    );
    assert_api_error(
        target.delete(&format!("{tokens}/999999999"), Some(&fixture.access_token)),
        404,
        "not_found",
    );
    let foreign_tokens = format!(
        "/api/v1/households/{}/admin/app_tokens",
        fixture.foreign_household_id
    );
    assert_api_error(
        target.get(&foreign_tokens, Some(&fixture.access_token)),
        403,
        "forbidden",
    );
    let response = target.get(&foreign_tokens, Some(&fixture.foreign_access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert_token_summary(app_token(&body(response), fixture.foreign_app_token_id));

    let response = target.post_json_authorized(
        &tokens,
        &fixture.manager_app_token,
        &json!({"api_app_token": {"name": "Manager bearer child"}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let child = body(response);
    let child_id = child["data"]["id"].as_i64().unwrap();
    let child_item = format!("{tokens}/{child_id}");
    assert_api_error(
        target.delete(&child_item, Some(&fixture.access_token)),
        404,
        "not_found",
    );
    let response = target.delete(&child_item, Some(&fixture.manager_app_token));
    assert_eq!(response.status().as_u16(), 204);
    let response = target.get(&tokens, Some(&fixture.manager_access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert!(app_token(&body(response), child_id)["revoked_at"].is_string());
}

#[test]
fn task_6a2_app_token_validation_and_duplicate_names() {
    let target = Target::from_env();
    let fixture = fixture();
    let tokens = path(&fixture, "app_tokens");
    let before = body(target.get(&tokens, Some(&fixture.access_token)))["data"].clone();
    assert_api_error(
        target.post_json_authorized(&tokens, &fixture.access_token, &json!({})),
        400,
        "bad_request",
    );
    let response = target.post_json_authorized(
        &tokens,
        &fixture.access_token,
        &json!({"api_app_token": {"name": " "}}),
    );
    let error = assert_api_error(response, 422, "validation_failed")["error"].clone();
    assert!(error["errors"]["name"].is_array());
    for expiry in [Value::Null, json!("forever"), json!("2999-01-01T00:00:00Z")] {
        let response = target.post_json_authorized(
            &tokens,
            &fixture.access_token,
            &json!({"api_app_token": {"name": "Invalid lifetime", "expires_at": expiry}}),
        );
        let error = assert_api_error(response, 422, "validation_failed")["error"].clone();
        assert!(error["errors"]["expires_at"].is_array());
    }
    let after = body(target.get(&tokens, Some(&fixture.access_token)))["data"].clone();
    assert_eq!(after, before);

    let request = json!({"api_app_token": {"name": "Repeated display name"}});
    let first = target.post_json_authorized(&tokens, &fixture.access_token, &request);
    assert_eq!(first.status().as_u16(), 201);
    let first_id = body(first)["data"]["id"].as_i64().unwrap();
    let second = target.post_json_authorized(&tokens, &fixture.access_token, &request);
    assert_eq!(second.status().as_u16(), 201);
    let second_id = body(second)["data"]["id"].as_i64().unwrap();
    assert_ne!(first_id, second_id);
}

#[test]
fn task_6a2_audit_access_order_query_bounds_and_sensitive_redaction() {
    let target = Target::from_env();
    let fixture = fixture();
    let audit = path(&fixture, "audit_logs");
    assert_api_error(target.get(&audit, None), 401, "unauthorized");
    assert_api_error(
        target.get(&audit, Some(&fixture.view_access_token)),
        403,
        "forbidden",
    );
    assert_api_error(
        target.get(&audit, Some(&fixture.foreign_access_token)),
        403,
        "forbidden",
    );
    let foreign_audit = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.foreign_household_id
    );
    assert_api_error(
        target.get(&foreign_audit, Some(&fixture.access_token)),
        403,
        "forbidden",
    );
    let response = target.get(&foreign_audit, Some(&fixture.foreign_access_token));
    assert_eq!(response.status().as_u16(), 200);
    assert!(body(response)["data"].is_array());

    let snapshot = format!("/api/v1/households/{}/sync/snapshot", fixture.household_id);
    let response = target.get(&snapshot, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let cursor = body(response)["data"]["cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let secret = "Private clinical note for contract 6A2";
    let health = format!("/api/v1/households/{}/health_events", fixture.household_id);
    let response = target.post_json_authorized(
        &health,
        &fixture.access_token,
        &json!({"health_event": {"person_id": fixture.managed_person_portable_id,
            "event_kind": "illness", "title": secret, "started_on": "2026-02-25"}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let health_request_id = request_id(&response);
    let raw_response = target.post_json_authorized(
        &path(&fixture, "app_tokens"),
        &fixture.access_token,
        &json!({"api_app_token": {"name": "Audit redaction probe"}}),
    );
    assert_eq!(raw_response.status().as_u16(), 201);
    let token_request_id = request_id(&raw_response);
    let raw = body(raw_response)["data"]["token"]
        .as_str()
        .unwrap()
        .to_owned();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let later_response = target.post_json_authorized(
        &path(&fixture, "app_tokens"),
        &fixture.access_token,
        &json!({"api_app_token": {"name": "Audit order probe"}}),
    );
    assert_eq!(later_response.status().as_u16(), 201);
    let later_request_id = request_id(&later_response);
    let later_raw = body(later_response)["data"]["token"]
        .as_str()
        .unwrap()
        .to_owned();

    for actor in [
        &fixture.access_token,
        &fixture.manager_access_token,
        &fixture.manager_app_token,
    ] {
        let response = target.get(&audit, Some(actor));
        assert_eq!(response.status().as_u16(), 200);
        let result = body(response);
        let rows = result["data"].as_array().expect("audit data array");
        assert!(!rows.is_empty());
        assert!(rows.len() <= 100);
        for pair in rows.windows(2) {
            let newer = pair[0]["created_at"]
                .as_str()
                .expect("newer audit timestamp");
            let older = pair[1]["created_at"]
                .as_str()
                .expect("older audit timestamp");
            assert!(newer >= older, "audit collection must be newest first");
        }
        assert!(result.get("meta").is_none());
        assert!(!result.to_string().contains(secret));
        assert!(!result.to_string().contains(&raw));
        assert!(!result.to_string().contains(&later_raw));
        let (earlier_position, token_event) = rows
            .iter()
            .enumerate()
            .find(|(_, row)| {
                row["request_id"] == token_request_id
                    && row["event_type"] == "auth_token/api_app_token/created"
            })
            .expect("token issuance audit with request correlation");
        let (later_position, later_event) = rows
            .iter()
            .enumerate()
            .find(|(_, row)| {
                row["request_id"] == later_request_id
                    && row["event_type"] == "auth_token/api_app_token/created"
            })
            .expect("later token issuance audit with request correlation");
        assert!(
            later_event["created_at"].as_str().unwrap()
                > token_event["created_at"].as_str().unwrap()
        );
        assert!(later_position < earlier_position);
        assert_eq!(token_event["actor_account_id"], fixture.account_id);
        assert_eq!(
            token_event["actor_membership_id"],
            fixture.owner_membership_id
        );
        assert_eq!(token_event["metadata"]["token_type"], "api_app_token");
        assert!(rows
            .iter()
            .any(|row| row["request_id"] == health_request_id));
    }
    let response = target.get(
        &format!("{audit}?page=999&per_page=1&event_type=absent"),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 200);
    let queried = body(response);
    assert!(queried["data"].as_array().unwrap().len() > 1);
    assert!(queried["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["request_id"] == token_request_id));
    assert!(queried.get("meta").is_none());

    let feed = format!(
        "/api/v1/households/{}/sync/changes?cursor={cursor}",
        fixture.household_id
    );
    let response = target.get(&feed, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let changes = body(response);
    assert!(!changes.to_string().contains(&raw));
    assert!(!changes.to_string().contains(&later_raw));
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
    assert_api_error(
        target.patch_json(
            &foreign,
            &fixture.access_token,
            &json!({"household_membership": {"status": "suspended"}}),
        ),
        404,
        "not_found",
    );
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
