use medtracker_contract_tests::{fixture, Target};
use serde_json::Value;
use std::env;

fn database() -> postgres::Client {
    let url = env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract audit database URL");
    postgres::Client::connect(&url, postgres::NoTls).expect("contract audit database")
}

fn grant_last_used_at(grant_id: i64) -> f64 {
    database()
        .query_one(
            "SELECT EXTRACT(EPOCH FROM last_used_at)::double precision FROM oauth_grants WHERE id = $1",
            &[&grant_id],
        )
        .expect("mobile grant last used")
        .get(0)
}

fn latest_audit_id() -> i64 {
    database()
        .query_one(
            "SELECT COALESCE(MAX(id), 0) FROM security_audit_events",
            &[],
        )
        .expect("latest audit event")
        .get(0)
}

fn audit_events_after(id: i64) -> Vec<Value> {
    database()
        .query(
            "SELECT row_to_json(security_audit_events)::text FROM security_audit_events WHERE id > $1 ORDER BY id",
            &[&id],
        )
        .expect("audit events")
        .into_iter()
        .map(|row| serde_json::from_str::<Value>(&row.get::<_, String>(0)).unwrap())
        .collect()
}

fn list_path(household_id: i64) -> String {
    format!("/api/v1/households/{household_id}/medications")
}

fn show_path(household_id: i64, medication_id: i64) -> String {
    format!("{}/{}", list_path(household_id), medication_id)
}

#[test]
fn current_mobile_oauth_grant_reads_medication_list_and_show() {
    let fixture = fixture();
    let target = Target::from_env();

    let list = target.get(
        &list_path(fixture.household_id),
        Some(&fixture.medication_mobile_oauth_token),
    );
    assert_eq!(list.status().as_u16(), 200);
    let body: Value = list.json().unwrap();
    assert!(body["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|medication| { medication["id"] == fixture.managed_medication_id }));

    let show = target.get(
        &show_path(fixture.household_id, fixture.managed_medication_id),
        Some(&fixture.medication_mobile_oauth_token),
    );
    assert_eq!(show.status().as_u16(), 200);
    let body: Value = show.json().unwrap();
    assert_eq!(body["data"]["id"], fixture.managed_medication_id);
    assert_eq!(body["data"]["name"], fixture.managed_medication_name);
}

#[test]
fn mobile_oauth_uses_current_household_membership_and_medication_visibility() {
    let fixture = fixture();
    let target = Target::from_env();
    let delegated_token = fixture.medication_mobile_oauth_tokens["delegated"]
        .as_str()
        .unwrap();

    let list = target.get(
        &list_path(fixture.medication_read_household_id),
        Some(delegated_token),
    );
    assert_eq!(list.status().as_u16(), 200);
    let body: Value = list.json().unwrap();
    let medications = body["data"].as_array().unwrap();
    assert!(medications
        .iter()
        .any(|row| row["id"] == fixture.medication_read_unlinked_id));
    assert!(!medications
        .iter()
        .any(|row| row["id"] == fixture.medication_read_hidden_id));
    assert_eq!(
        target
            .get(
                &show_path(
                    fixture.medication_read_household_id,
                    fixture.medication_read_unlinked_id
                ),
                Some(delegated_token),
            )
            .status()
            .as_u16(),
        200
    );
    assert_eq!(
        target
            .get(
                &show_path(
                    fixture.medication_read_household_id,
                    fixture.medication_read_hidden_id
                ),
                Some(delegated_token),
            )
            .status()
            .as_u16(),
        404
    );

    let secondary = target.get(
        &list_path(fixture.medication_read_household_id),
        Some(&fixture.medication_mobile_oauth_token),
    );
    assert_eq!(secondary.status().as_u16(), 200);
    let body: Value = secondary.json().unwrap();
    assert_eq!(body["meta"]["total_count"], 0);
}

#[test]
fn mobile_oauth_linked_medication_visibility_requires_an_active_person_grant() {
    let fixture = fixture();
    let target = Target::from_env();
    let tokens = &fixture.medication_mobile_oauth_tokens;
    let path = list_path(fixture.household_id);

    let visible = target.get(&path, tokens["view"].as_str());
    assert_eq!(visible.status().as_u16(), 200);
    let body: Value = visible.json().unwrap();
    let medications = body["data"].as_array().unwrap();
    assert!(medications
        .iter()
        .any(|row| row["id"] == fixture.managed_medication_id));
    assert!(!medications
        .iter()
        .any(|row| row["id"] == fixture.hidden_medication_id));
    assert_eq!(
        target
            .get(
                &show_path(fixture.household_id, fixture.managed_medication_id),
                tokens["view"].as_str(),
            )
            .status()
            .as_u16(),
        200
    );

    let revoked = target.get(&path, tokens["revoked_view"].as_str());
    assert_eq!(revoked.status().as_u16(), 200);
    let body: Value = revoked.json().unwrap();
    assert!(!body["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| { row["id"] == fixture.managed_medication_id }));
    assert_eq!(
        target
            .get(
                &show_path(fixture.household_id, fixture.managed_medication_id),
                tokens["revoked_view"].as_str(),
            )
            .status()
            .as_u16(),
        404
    );
}

#[test]
fn invalid_mobile_oauth_grants_cannot_read_medications() {
    let fixture = fixture();
    let target = Target::from_env();
    let tokens = &fixture.medication_mobile_oauth_tokens;
    for label in ["revoked", "expired", "stale_login", "old_login", "no_scope"] {
        let token = tokens[label].as_str().unwrap();
        let response = target.get(&list_path(fixture.household_id), Some(token));
        assert_eq!(response.status().as_u16(), 401, "{label}");
    }
    for (label, household_id) in [
        ("inactive", fixture.auth_inactive_household_id),
        ("locked", fixture.medication_mobile_locked_household_id),
    ] {
        let token = tokens[label].as_str().unwrap();
        let response = target.get(&list_path(household_id), Some(token));
        assert_eq!(response.status().as_u16(), 401, "{label}");
    }
    let revoked_member = tokens["revoked_membership"].as_str().unwrap();
    assert_eq!(
        target
            .get(
                &list_path(fixture.portable_target_household_id),
                Some(revoked_member)
            )
            .status()
            .as_u16(),
        403
    );
}

#[test]
fn mobile_oauth_audit_identifies_grant_and_does_not_include_bearer() {
    let fixture = fixture();
    let target = Target::from_env();
    let before = latest_audit_id();
    let response = target.get(
        &show_path(fixture.household_id, fixture.managed_medication_id),
        Some(&fixture.medication_mobile_oauth_token),
    );
    assert_eq!(response.status().as_u16(), 200);

    let events = audit_events_after(before);
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert_eq!(event["event_type"], "api.request");
    assert_eq!(event["household_id"], fixture.household_id);
    assert_eq!(event["actor_account_id"], fixture.account_id);
    assert_eq!(event["actor_membership_id"], fixture.owner_membership_id);
    assert_eq!(event["audit_context"]["authentication_method"], "oauth");
    assert_eq!(
        event["audit_context"]["session_reference"],
        format!("oauth_grant:{}", fixture.medication_mobile_oauth_grant_id)
    );
    assert!(!event
        .to_string()
        .contains(&fixture.medication_mobile_oauth_token));
    assert!(!event.to_string().contains("api_session:"));
}

#[test]
fn mobile_oauth_foreign_household_is_forbidden_but_refreshes_activity() {
    let fixture = fixture();
    let target = Target::from_env();
    let before = grant_last_used_at(fixture.medication_mobile_oauth_grant_id);
    let response = target.get(
        &list_path(fixture.foreign_household_id),
        Some(&fixture.medication_mobile_oauth_token),
    );
    assert_eq!(response.status().as_u16(), 403);
    assert!(grant_last_used_at(fixture.medication_mobile_oauth_grant_id) > before);
}

#[test]
fn mobile_oauth_invalid_filter_still_refreshes_activity() {
    let fixture = fixture();
    let target = Target::from_env();
    let before = grant_last_used_at(fixture.medication_mobile_oauth_grant_id);
    let path = format!(
        "{}?updated_since=not-a-timestamp",
        list_path(fixture.household_id)
    );
    let response = target.get(&path, Some(&fixture.medication_mobile_oauth_token));
    assert_eq!(response.status().as_u16(), 422);
    assert!(grant_last_used_at(fixture.medication_mobile_oauth_grant_id) > before);
}
