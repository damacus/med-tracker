use medtracker_contract_tests::{fixture, Target};
use serde_json::{json, Value};

fn assert_denial(response: reqwest::blocking::Response, status: u16, code: &str) {
    assert_eq!(response.status().as_u16(), status);
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned();
    let body: Value = response.json().expect("JSON denial");
    assert_eq!(body["error"]["code"], code);
    assert_eq!(body["error"]["request_id"], request_id);
    assert!(body.get("data").is_none());
}

#[test]
fn capabilities_are_public() {
    let response = Target::from_env().get("/api/v1/capabilities", None);
    assert_eq!(response.status().as_u16(), 200);
    assert!(response
        .headers()
        .get("cache-control")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("no-store"));
    let body: Value = response.json().expect("JSON capabilities");
    assert_eq!(body["data"]["format"], "medtracker.api.capabilities.v1");
    assert_eq!(body["data"]["api_version"], "v1");
    assert!(body["data"]["authentication"]["methods"]
        .as_array()
        .unwrap()
        .contains(&Value::from("oauth_bearer")));
    assert_eq!(
        body["data"]["authentication"]["hosted_mobile"],
        "rodauth_authorization_code_pkce"
    );
    assert_eq!(
        body["data"]["authentication"]["mobile_oauth"]["household_binding"],
        "account"
    );
    assert_eq!(body["data"]["profile"]["online_only"], true);
    assert_eq!(body["data"]["administration"]["household"], true);
    assert!(body["data"]["authentication"]
        .get("password_login")
        .is_none());
}

#[test]
fn bearer_lists_only_its_operational_household() {
    let fixture = fixture();
    let response = Target::from_env().get("/api/v1/auth/households", Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().expect("JSON households");
    assert_eq!(body["account_id"], fixture.account_id);
    let households = body["data"].as_array().expect("household array");
    assert_eq!(households.len(), 1);
    assert_eq!(households[0]["id"], fixture.household_id);
    assert_eq!(households[0]["name"], fixture.household_name);
    assert_eq!(households[0]["role"], "owner");
    assert!(!body.to_string().contains(&fixture.foreign_email));
}

#[test]
fn bearer_lists_its_sessions_without_token_material() {
    let fixture = fixture();
    let response = Target::from_env().get("/api/v1/auth/sessions", Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().expect("JSON sessions");
    let sessions = body["data"].as_array().expect("sessions array");
    let session = sessions
        .iter()
        .find(|item| item["id"] == fixture.session_id)
        .expect("current session");
    assert_eq!(session["household_id"], fixture.household_id);
    assert_eq!(session["device_name"], "contract-tests");
    assert!(session["access_token_expires_at"].is_string());
    assert!(session["refresh_token_expires_at"].is_string());
    assert!(!body.to_string().contains(&fixture.access_token));
}

#[test]
fn unknown_expired_and_locked_bearers_are_denied() {
    let fixture = fixture();
    let target = Target::from_env();
    for token in [
        "mt_unknown_access_token",
        &fixture.expired_access_token,
        &fixture.locked_access_token,
    ] {
        for path in ["/api/v1/auth/households", "/api/v1/auth/sessions"] {
            let response = target.get(path, Some(token));
            assert_eq!(response.status().as_u16(), 401, "{path}");
            let body: Value = response.json().expect("JSON denial");
            assert_eq!(body["error"]["code"], "unauthorized");
        }
    }
}

#[test]
fn selected_session_revocation_invalidates_its_bearer() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = format!("/api/v1/auth/sessions/{}", fixture.revocable_session_id);
    let response = target.delete(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 204);
    let response = target.get(
        "/api/v1/auth/sessions",
        Some(&fixture.revocable_access_token),
    );
    assert_eq!(response.status().as_u16(), 401);
    let body: Value = response.json().expect("JSON denial");
    assert_eq!(body["error"]["code"], "unauthorized");
}

#[test]
fn logout_revokes_the_current_bearer_and_is_idempotent_without_one() {
    let fixture = fixture();
    let target = Target::from_env();
    let response = target.delete("/api/v1/auth/logout", Some(&fixture.logout_access_token));
    assert_eq!(response.status().as_u16(), 204);
    let response = target.get("/api/v1/auth/sessions", Some(&fixture.logout_access_token));
    assert_eq!(response.status().as_u16(), 401);
    let response = target.delete("/api/v1/auth/logout", None);
    assert_eq!(response.status().as_u16(), 204);
}

#[test]
fn task_6n_deactivated_user_cannot_use_prior_session_or_app_token() {
    let fixture = fixture();
    let target = Target::from_env();
    let path = format!(
        "/api/v1/households/{}/me",
        fixture.auth_deactivated_household_id
    );
    for token in [
        &fixture.auth_deactivated_access_token,
        &fixture.auth_deactivated_app_token,
    ] {
        assert_denial(target.get(&path, Some(token)), 401, "unauthorized");
        assert_denial(
            target.get("/api/v1/auth/households", Some(token)),
            401,
            "unauthorized",
        );
    }
}

#[test]
fn task_6n_inactive_user_cannot_use_still_issued_session() {
    let fixture = fixture();
    let path = format!(
        "/api/v1/households/{}/me",
        fixture.auth_inactive_household_id
    );
    assert_denial(
        Target::from_env().get(&path, Some(&fixture.auth_inactive_access_token)),
        401,
        "unauthorized",
    );
}

#[test]
fn task_6n_nonoperational_households_and_suspended_membership_deny_prior_session() {
    let fixture = fixture();
    let target = Target::from_env();
    for state in ["held", "offboarded", "purged"] {
        let context = &fixture.auth_operational_states[state];
        let household_id = context["household_id"].as_i64().expect("household ID");
        let token = context["access_token"].as_str().expect("access token");
        let path = format!("/api/v1/households/{household_id}/medications");
        assert_denial(target.get(&path, Some(token)), 401, "unauthorized");
    }
    let path = format!(
        "/api/v1/households/{}/me",
        fixture.auth_suspended_household_id
    );
    assert_denial(
        target.get(&path, Some(&fixture.auth_suspended_access_token)),
        401,
        "unauthorized",
    );
}

#[test]
fn task_6n_absent_requested_membership_is_forbidden() {
    let fixture = fixture();
    let path = format!("/api/v1/households/{}/me", fixture.foreign_household_id);
    assert_denial(
        Target::from_env().get(&path, Some(&fixture.access_token)),
        403,
        "forbidden",
    );
}

#[test]
fn task_6n_role_change_invalidates_prior_session_and_app_token() {
    let fixture = fixture();
    let target = Target::from_env();
    let me_path = format!("/api/v1/households/{}/me", fixture.auth_role_household_id);
    for token in [
        &fixture.auth_role_member_access_token,
        &fixture.auth_role_member_app_token,
        &fixture.auth_role_member_oauth_token,
    ] {
        assert_eq!(target.get(&me_path, Some(token)).status().as_u16(), 200);
    }
    let path = format!(
        "/api/v1/households/{}/admin/memberships/{}",
        fixture.auth_role_household_id, fixture.auth_role_member_membership_id
    );
    let response = target.patch_json(
        &path,
        &fixture.auth_role_owner_access_token,
        &json!({"household_membership": {"role": "administrator"}}),
    );
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().expect("updated membership");
    assert_eq!(body["data"]["role"], "administrator");
    for token in [
        &fixture.auth_role_member_access_token,
        &fixture.auth_role_member_app_token,
        &fixture.auth_role_member_oauth_token,
    ] {
        assert_denial(target.get(&me_path, Some(token)), 401, "unauthorized");
    }
    assert_eq!(
        target
            .get(&me_path, Some(&fixture.auth_role_owner_access_token))
            .status()
            .as_u16(),
        200
    );
}
