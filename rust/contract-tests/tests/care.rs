use medtracker_contract_tests::{fixture, Target};
use serde_json::Value;

#[test]
fn current_profile_reads_with_bearer_session() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = format!("/api/v1/households/{}/me", fixture.household_id);
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().expect("JSON profile");
    assert_eq!(body["data"]["id"], fixture.user_id);
    assert_eq!(body["data"]["membership_role"], "owner");
}

#[test]
fn bearer_session_cannot_read_another_household() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = format!("/api/v1/households/{}/me", fixture.foreign_household_id);
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 403);
    let body: Value = response.json().expect("JSON denial");
    assert_eq!(body["error"]["code"], "forbidden");
    assert!(!body.to_string().contains(&fixture.foreign_email));
}
