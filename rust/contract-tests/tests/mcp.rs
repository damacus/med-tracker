use medtracker_contract_tests::{fixture, Target};
use reqwest::blocking::Response;
use serde_json::{json, Value};

fn request(target: &Target, token: Option<&str>, id: i64, method: &str, params: Value) -> Response {
    let message = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
    match token {
        Some(token) => target.post_json_authorized("/mcp", token, &message),
        None => target.post_json("/mcp", &message),
    }
}

fn result(response: Response, id: i64) -> Value {
    assert_eq!(response.status().as_u16(), 200);
    let message: Value = response.json().expect("MCP JSON response");
    assert_eq!(message["jsonrpc"], "2.0");
    assert_eq!(message["id"], id);
    assert!(message.get("error").is_none());
    message["result"].clone()
}

fn call_tool(target: &Target, token: &str, id: i64, name: &str, arguments: Value) -> Value {
    result(
        request(
            target,
            Some(token),
            id,
            "tools/call",
            json!({"name": name, "arguments": arguments}),
        ),
        id,
    )
}

fn rpc_error(response: Response, id: i64) -> Value {
    assert_eq!(response.status().as_u16(), 200);
    let message: Value = response.json().expect("MCP JSON error");
    assert_eq!(message["jsonrpc"], "2.0");
    assert_eq!(message["id"], id);
    assert!(message.get("result").is_none());
    assert!(message["error"]["code"].is_number());
    assert!(message["error"]["message"].is_string());
    message["error"].clone()
}

fn assert_snapshot_excludes(
    snapshot: &Value,
    fixture: &medtracker_contract_tests::Fixture,
    view_only: bool,
) {
    let records = &snapshot["snapshot"]["records"];
    let forbidden = [
        (
            "people",
            &fixture.hidden_person_portable_id,
            &fixture.foreign_person_portable_id,
        ),
        (
            "locations",
            &fixture.hidden_location_portable_id,
            &fixture.foreign_location_portable_id,
        ),
        (
            "medications",
            &fixture.hidden_medication_portable_id,
            &fixture.foreign_medication_portable_id,
        ),
        (
            "dosage_options",
            &fixture.hidden_dosage_portable_id,
            &fixture.foreign_dosage_portable_id,
        ),
        (
            "schedules",
            &fixture.hidden_schedule_portable_id,
            &fixture.foreign_schedule_portable_id,
        ),
        (
            "person_medications",
            &fixture.hidden_assignment_portable_id,
            &fixture.foreign_assignment_portable_id,
        ),
        (
            "medication_takes",
            &fixture.hidden_take_portable_id,
            &fixture.foreign_take_portable_id,
        ),
        (
            "notification_preferences",
            &fixture.hidden_preference_portable_id,
            &fixture.foreign_preference_portable_id,
        ),
    ];
    let managed = [
        ("people", &fixture.managed_person_portable_id),
        ("locations", &fixture.primary_location_portable_id),
        ("medications", &fixture.managed_medication_portable_id),
        ("dosage_options", &fixture.managed_dosage_portable_id),
        ("schedules", &fixture.managed_schedule_portable_id),
        (
            "person_medications",
            &fixture.managed_assignment_portable_id,
        ),
        ("medication_takes", &fixture.managed_take_portable_id),
        (
            "notification_preferences",
            &fixture.managed_preference_portable_id,
        ),
    ];
    for (kind, hidden, foreign) in forbidden {
        let rows = records[kind].as_array().expect("snapshot collection");
        assert!(
            rows.iter().all(|row| row["portable_id"] != *foreign),
            "{kind} leaked foreign record"
        );
        if view_only {
            assert!(
                rows.iter().all(|row| row["portable_id"] != *hidden),
                "{kind} leaked hidden record"
            );
        }
    }
    for (kind, id) in managed {
        let rows = records[kind].as_array().expect("snapshot collection");
        assert_eq!(
            rows.iter().any(|row| row["portable_id"] == *id),
            !view_only,
            "{kind} manage scope"
        );
    }
}

#[test]
fn mcp_initializes_and_lists_read_only_capabilities() {
    let target = Target::from_env();
    let fixture = fixture();
    let token = &fixture.manager_app_token;

    let initialized = result(
        request(
            &target,
            Some(token),
            1,
            "initialize",
            json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "MedTracker contract", "version": "1.0"}
            }),
        ),
        1,
    );
    assert_eq!(initialized["serverInfo"]["name"], "med_tracker");
    assert_eq!(initialized["serverInfo"]["title"], "MedTracker MCP");
    assert_eq!(initialized["capabilities"]["tools"]["listChanged"], false);
    assert_eq!(
        initialized["capabilities"]["resources"]["listChanged"],
        false
    );
    assert_eq!(initialized["capabilities"]["prompts"]["listChanged"], false);

    let tools = result(request(&target, Some(token), 2, "tools/list", json!({})), 2);
    let entries = tools["tools"].as_array().expect("MCP tools");
    let mut names: Vec<_> = entries
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "medtracker_current_user",
            "medtracker_health_history_summary",
            "medtracker_household_snapshot",
            "medtracker_inventory_risks",
            "medtracker_today_schedule"
        ]
    );
    assert!(entries.iter().all(|tool| tool["inputSchema"].is_object()));

    let resources = result(
        request(&target, Some(token), 3, "resources/list", json!({})),
        3,
    );
    let resource_entries = resources["resources"].as_array().expect("MCP resources");
    assert_eq!(resource_entries.len(), 1);
    assert_eq!(
        resource_entries[0]["uri"],
        "medtracker://household/snapshot"
    );
    assert_eq!(resource_entries[0]["mimeType"], "application/json");
    let prompts = result(
        request(&target, Some(token), 4, "prompts/list", json!({})),
        4,
    );
    let prompt_entries = prompts["prompts"].as_array().expect("MCP prompts");
    assert_eq!(prompt_entries.len(), 1);
    assert_eq!(prompt_entries[0]["name"], "medtracker_household_review");
}

#[test]
fn mcp_reads_account_and_policy_scoped_household_context() {
    let target = Target::from_env();
    let fixture = fixture();

    let profile = call_tool(
        &target,
        &fixture.access_token,
        10,
        "medtracker_current_user",
        json!({}),
    );
    assert_eq!(
        profile["structuredContent"]["format"],
        "medtracker.mcp.current_user.v1"
    );
    assert_eq!(
        profile["structuredContent"]["user"]["email_address"],
        fixture.primary_email
    );
    assert!(!profile.to_string().contains(&fixture.access_token));

    let snapshot = call_tool(
        &target,
        &fixture.view_access_token,
        11,
        "medtracker_household_snapshot",
        json!({}),
    );
    let payload = &snapshot["structuredContent"];
    assert_eq!(payload["format"], "medtracker.mcp.household_snapshot.v1");
    assert_eq!(payload["snapshot"]["format"], "medtracker.portable.v1");
    assert_snapshot_excludes(payload, &fixture, true);
    assert!(!snapshot.to_string().contains(&fixture.foreign_person_name));
    let owner_snapshot = call_tool(
        &target,
        &fixture.access_token,
        14,
        "medtracker_household_snapshot",
        json!({}),
    );
    assert_snapshot_excludes(&owner_snapshot["structuredContent"], &fixture, false);

    let schedule = call_tool(
        &target,
        &fixture.view_access_token,
        12,
        "medtracker_today_schedule",
        json!({}),
    );
    assert_eq!(
        schedule["structuredContent"]["format"],
        "medtracker.mcp.today_schedule.v1"
    );
    let schedules = schedule["structuredContent"]["schedules"]
        .as_array()
        .expect("visible schedules");
    assert!(schedules
        .iter()
        .any(|row| row["portable_id"] == fixture.managed_schedule_portable_id));
    assert!(schedules.iter().all(|row| {
        row["portable_id"] != fixture.hidden_schedule_portable_id
            && row["portable_id"] != fixture.foreign_schedule_portable_id
    }));
    let taken_today = schedule["structuredContent"]["taken_today"]
        .as_array()
        .expect("taken today groups");
    let managed_taken = taken_today
        .iter()
        .find(|row| row["person_id"] == fixture.managed_person_id)
        .expect("managed person taken today");
    assert!(managed_taken["medications"]
        .as_array()
        .expect("taken medications")
        .iter()
        .any(|row| row["id"] == fixture.historical_medication_id));
    assert!(taken_today
        .iter()
        .all(|row| row["person_id"] != fixture.hidden_person_id
            && row["person_id"] != fixture.foreign_person_id));

    let inventory = call_tool(
        &target,
        &fixture.view_access_token,
        13,
        "medtracker_inventory_risks",
        json!({}),
    );
    assert_eq!(
        inventory["structuredContent"]["format"],
        "medtracker.mcp.inventory_risks.v1"
    );
    let risks = inventory["structuredContent"]["medications"]
        .as_array()
        .expect("inventory risks");
    assert!(risks
        .iter()
        .any(|row| row["portable_id"] == fixture.visible_low_stock_portable_id));
    assert!(risks.iter().all(
        |row| row["portable_id"] != fixture.hidden_low_stock_portable_id
            && row["portable_id"] != fixture.foreign_low_stock_portable_id
    ));
}

#[test]
fn mcp_resources_prompts_and_history_obey_household_scope() {
    let target = Target::from_env();
    let fixture = fixture();

    let resource = result(
        request(
            &target,
            Some(&fixture.view_access_token),
            20,
            "resources/read",
            json!({"uri": "medtracker://household/snapshot"}),
        ),
        20,
    );
    assert_eq!(
        resource["contents"][0]["uri"],
        "medtracker://household/snapshot"
    );
    let snapshot: Value = serde_json::from_str(
        resource["contents"][0]["text"]
            .as_str()
            .expect("JSON resource text"),
    )
    .expect("household snapshot resource");
    assert_eq!(snapshot["format"], "medtracker.mcp.household_snapshot.v1");
    assert_snapshot_excludes(&snapshot, &fixture, true);
    let owner_resource = result(
        request(
            &target,
            Some(&fixture.access_token),
            23,
            "resources/read",
            json!({"uri": "medtracker://household/snapshot"}),
        ),
        23,
    );
    let owner_snapshot: Value = serde_json::from_str(
        owner_resource["contents"][0]["text"]
            .as_str()
            .expect("owner resource text"),
    )
    .expect("owner snapshot resource");
    assert_snapshot_excludes(&owner_snapshot, &fixture, false);

    let prompt = result(
        request(
            &target,
            Some(&fixture.view_access_token),
            21,
            "prompts/get",
            json!({"name": "medtracker_household_review", "arguments": {}}),
        ),
        21,
    );
    assert_eq!(prompt["messages"][0]["role"], "user");
    assert!(prompt["messages"][0]["content"]["text"]
        .as_str()
        .expect("prompt text")
        .contains(&fixture.household_name));
    assert!(!prompt.to_string().contains(&fixture.foreign_person_name));

    let history = call_tool(
        &target,
        &fixture.view_access_token,
        22,
        "medtracker_health_history_summary",
        json!({
            "start_date": "2026-02-25",
            "end_date": "2026-02-26",
            "person_ids": [fixture.managed_person_id, fixture.hidden_person_id, fixture.foreign_person_id]
        }),
    );
    let summary = &history["structuredContent"];
    assert_eq!(
        summary["format"],
        "medtracker.mcp.health_history_summary.v1"
    );
    assert_eq!(summary["start_date"], "2026-02-25");
    assert_eq!(summary["end_date"], "2026-02-26");
    assert!(summary["people"]
        .as_array()
        .expect("history people")
        .iter()
        .any(|person| person["id"] == fixture.managed_person_id));
    assert!(summary["people"]
        .as_array()
        .expect("history people")
        .iter()
        .all(|person| person["id"] == fixture.managed_person_id));
    assert!(!summary.to_string().contains(&fixture.foreign_person_name));
    let takes = summary["medication_takes"]
        .as_array()
        .expect("history takes");
    assert!(takes
        .iter()
        .any(|row| row["person_id"] == fixture.managed_person_id
            && row["medication_name"] == fixture.historical_medication_name));
    assert!(takes
        .iter()
        .all(|row| row["person_id"] == fixture.managed_person_id));
    let effects = summary["suspected_side_effects"]
        .as_array()
        .expect("side effects");
    assert!(effects
        .iter()
        .any(|row| row["person_id"] == fixture.managed_person_id
            && row["title"] == fixture.managed_side_effect_title));
    assert!(effects
        .iter()
        .all(|row| row["person_id"] == fixture.managed_person_id));
    let illnesses = summary["notable_illnesses"].as_array().expect("illnesses");
    assert!(illnesses
        .iter()
        .any(|row| row["person_id"] == fixture.managed_person_id
            && row["title"] == fixture.managed_health_event_title));
    assert!(illnesses
        .iter()
        .all(|row| row["person_id"] == fixture.managed_person_id));
    let patterns = summary["illness_patterns"]
        .as_array()
        .expect("illness patterns");
    assert!(patterns.iter().any(
        |row| row["display_title"] == fixture.managed_health_event_title
            && row["episode_count"] == 2
    ));
    assert!(patterns
        .iter()
        .all(|row| row["display_title"] == fixture.managed_health_event_title));
}

#[test]
fn mcp_rejects_missing_invalid_and_unavailable_requests() {
    let target = Target::from_env();
    let fixture = fixture();

    let missing = request(&target, None, 30, "tools/list", json!({}));
    assert_eq!(missing.status().as_u16(), 401);
    let missing: Value = missing.json().expect("unauthorized JSON");
    assert_eq!(missing["error"]["code"], "unauthorized");
    assert!(missing.get("result").is_none());

    for token in [
        &fixture.portable_revoked_access_token,
        &fixture.portable_locked_access_token,
        &fixture.expired_access_token,
    ] {
        let denied = request(&target, Some(token), 31, "tools/list", json!({}));
        assert_eq!(denied.status().as_u16(), 401);
        let denied: Value = denied.json().expect("denied JSON");
        assert_eq!(denied["error"]["code"], "unauthorized");
        assert!(!denied.to_string().contains(token));
    }

    let unknown_method = rpc_error(
        request(
            &target,
            Some(&fixture.manager_app_token),
            32,
            "unsafe/write",
            json!({}),
        ),
        32,
    );
    assert_eq!(unknown_method["code"], -32601);
    assert!(!unknown_method
        .to_string()
        .contains(&fixture.manager_app_token));
    let missing_tool = rpc_error(
        request(
            &target,
            Some(&fixture.manager_app_token),
            33,
            "tools/call",
            json!({"arguments": {}}),
        ),
        33,
    );
    assert_eq!(missing_tool["code"], -32602);
    assert!(!missing_tool
        .to_string()
        .contains(&fixture.manager_app_token));
    let unknown_tool = rpc_error(
        request(
            &target,
            Some(&fixture.manager_app_token),
            36,
            "tools/call",
            json!({"name": "medtracker_delete_everything", "arguments": {}}),
        ),
        36,
    );
    assert_eq!(unknown_tool["code"], -32602);
    let unknown_prompt = rpc_error(
        request(
            &target,
            Some(&fixture.manager_app_token),
            37,
            "prompts/get",
            json!({"name": "medtracker_unknown_prompt", "arguments": {}}),
        ),
        37,
    );
    assert_eq!(unknown_prompt["code"], -32602);
    let unknown_resource = rpc_error(
        request(
            &target,
            Some(&fixture.manager_app_token),
            34,
            "resources/read",
            json!({"uri": "medtracker://household/unknown"}),
        ),
        34,
    );
    assert_eq!(unknown_resource["code"], -32602);
    assert!(!unknown_resource
        .to_string()
        .contains(&fixture.manager_app_token));

    let invalid_history = call_tool(
        &target,
        &fixture.view_access_token,
        35,
        "medtracker_health_history_summary",
        json!({"start_date": "2026-01-01", "end_date": "2026-07-01"}),
    );
    assert_eq!(invalid_history["isError"], true);
    assert!(invalid_history["content"][0]["text"]
        .as_str()
        .expect("tool error text")
        .contains("180 days"));
    assert!(invalid_history.get("structuredContent").is_none());
}

#[test]
fn mcp_bearer_scope_and_origin_are_checked_per_request() {
    let target = Target::from_env();
    let fixture = fixture();

    let foreign = call_tool(
        &target,
        &fixture.foreign_access_token,
        40,
        "medtracker_household_snapshot",
        json!({}),
    );
    let foreign_people = foreign["structuredContent"]["snapshot"]["records"]["people"]
        .as_array()
        .expect("foreign household people");
    assert!(foreign_people
        .iter()
        .all(|person| person["portable_id"] != fixture.managed_person_portable_id));
    assert!(!foreign
        .to_string()
        .contains(&fixture.managed_medication_name));

    let message = json!({"jsonrpc": "2.0", "id": 41, "method": "tools/list", "params": {}});
    let cross_origin = target.post_json_with_header(
        "/mcp",
        &fixture.manager_app_token,
        "Origin",
        "https://evil.example.com",
        &message,
    );
    assert_eq!(cross_origin.status().as_u16(), 403);
    let denied: Value = cross_origin.json().expect("cross-origin JSON");
    assert_eq!(
        denied["error"]["message"],
        "Forbidden: Invalid Origin header"
    );
}

#[test]
fn mcp_rejects_batches_and_records_a_request_scoped_audit() {
    let target = Target::from_env();
    let fixture = fixture();

    let batch = target.post_json_authorized(
        "/mcp",
        &fixture.manager_app_token,
        &json!([{"jsonrpc": "2.0", "id": 50, "method": "tools/list", "params": {}}]),
    );
    assert_eq!(batch.status().as_u16(), 400);
    let batch: Value = batch.json().expect("batch rejection JSON");
    assert!(batch["error"]["message"]
        .as_str()
        .expect("batch error message")
        .contains("single request object"));

    let listed = request(
        &target,
        Some(&fixture.manager_app_token),
        51,
        "tools/list",
        json!({}),
    );
    let request_id = listed.headers()["x-request-id"]
        .to_str()
        .expect("MCP request ID")
        .to_owned();
    assert!(result(listed, 51)["tools"].is_array());

    let audit_path = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.household_id
    );
    let audits = target.get(&audit_path, Some(&fixture.access_token));
    assert_eq!(audits.status().as_u16(), 200);
    let audits: Value = audits.json().expect("public audit list");
    let event = audits["data"]
        .as_array()
        .expect("audit entries")
        .iter()
        .find(|event| event["request_id"] == request_id && event["event_type"] == "mcp.request")
        .expect("request-correlated MCP audit");
    assert_eq!(event["actor_membership_id"], fixture.manager_membership_id);
    assert_eq!(event["metadata"]["method"], "tools/list");
    assert_eq!(event["metadata"]["outcome"], "ok");
    assert_eq!(event["metadata"]["status"], 200);
    assert!(!event.to_string().contains(&fixture.manager_app_token));
}

#[test]
fn mcp_stateless_transport_handles_notifications_without_a_session() {
    let target = Target::from_env();
    let fixture = fixture();

    let notification = target.post_json_authorized(
        "/mcp",
        &fixture.manager_app_token,
        &json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
    );
    assert_eq!(notification.status().as_u16(), 202);
    assert!(notification
        .bytes()
        .expect("notification response")
        .is_empty());

    let get = target.get("/mcp", Some(&fixture.manager_app_token));
    assert_eq!(get.status().as_u16(), 405);
    let delete = target.delete("/mcp", Some(&fixture.manager_app_token));
    assert_eq!(delete.status().as_u16(), 200);
}
