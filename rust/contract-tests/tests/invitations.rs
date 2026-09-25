use medtracker_contract_tests::{fixture, Fixture, Target};
use reqwest::blocking::Response;
use serde_json::{json, Value};
use std::env;
use std::thread;
use std::time::Duration;
use url::{Host, Url};

struct Mailpit {
    client: reqwest::blocking::Client,
    origin: Url,
}

impl Mailpit {
    fn from_env() -> Self {
        let origin = Url::parse(&env::var("CONTRACT_MAILPIT_URL").expect("Mailpit URL"))
            .expect("valid Mailpit URL");
        assert_eq!(origin.scheme(), "http");
        assert!(matches!(origin.host(), Some(Host::Ipv4(address)) if address.is_loopback()));
        assert_eq!(origin.path(), "/");
        assert!(origin.username().is_empty() && origin.password().is_none());
        assert!(origin.query().is_none() && origin.fragment().is_none());
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()
            .expect("Mailpit client");
        Self { client, origin }
    }

    fn message_ids_to(&self, recipient: &str) -> Vec<String> {
        let url = self
            .origin
            .join("api/v1/messages?start=0&limit=100")
            .expect("Mailpit messages URL");
        let response = self.client.get(url).send().expect("Mailpit messages");
        assert_eq!(response.status().as_u16(), 200);
        let payload: Value = response.json().expect("Mailpit messages JSON");
        payload["messages"]
            .as_array()
            .expect("Mailpit messages array")
            .iter()
            .filter(|message| {
                message["To"].as_array().is_some_and(|recipients| {
                    recipients.iter().any(|item| {
                        item["Address"]
                            .as_str()
                            .or_else(|| item["address"].as_str())
                            .is_some_and(|address| address.eq_ignore_ascii_case(recipient))
                    })
                })
            })
            .map(|message| {
                message["ID"]
                    .as_str()
                    .expect("Mailpit message ID")
                    .to_owned()
            })
            .collect()
    }

    fn wait_for_one_to(&self, recipient: &str) -> String {
        for _ in 0..30 {
            let ids = self.message_ids_to(recipient);
            if ids.len() == 1 {
                return ids[0].clone();
            }
            thread::sleep(Duration::from_millis(100));
        }
        panic!("expected exactly one delivered invitation");
    }

    fn token_from_message(&self, message_id: &str) -> String {
        assert!(message_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'));
        let url = self
            .origin
            .join(&format!("api/v1/message/{message_id}"))
            .expect("Mailpit message URL");
        let response = self.client.get(url).send().expect("Mailpit message");
        assert_eq!(response.status().as_u16(), 200);
        let message: Value = response.json().expect("Mailpit message JSON");
        let text = message["Text"]
            .as_str()
            .or_else(|| message["text"].as_str())
            .expect("Mailpit message text");
        let accept_url = text
            .lines()
            .find(|line| line.contains("/invitations/accept?token="))
            .expect("invitation acceptance URL");
        let url = Url::parse(accept_url.trim()).expect("valid invitation acceptance URL");
        let token = url
            .query_pairs()
            .find(|(key, _)| key == "token")
            .expect("invitation token")
            .1
            .into_owned();
        assert_eq!(token.len(), 64);
        assert!(token.bytes().all(|byte| byte.is_ascii_hexdigit()));
        token
    }
}

fn invitations(fixture: &Fixture) -> String {
    format!(
        "/api/v1/households/{}/admin/invitations",
        fixture.household_id
    )
}

fn invitation(list: &Value, id: i64) -> &Value {
    list["data"]
        .as_array()
        .expect("invitation collection")
        .iter()
        .find(|item| item["id"] == id)
        .expect("invitation in public collection")
}

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn assert_error(response: Response, status: u16, code: &str) -> Value {
    assert_eq!(response.status().as_u16(), status);
    assert_eq!(
        response.headers()["content-type"]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap(),
        "application/json"
    );
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned();
    let payload = body(response);
    assert_eq!(payload["error"]["code"], code);
    assert_eq!(payload["error"]["request_id"], request_id);
    assert!(payload["error"]["message"].as_str().is_some());
    assert!(payload.get("data").is_none());
    payload
}

fn assert_summary(item: &Value) {
    let fields = item.as_object().expect("invitation object");
    assert_eq!(fields.len(), 7);
    assert!(item["id"].is_i64());
    assert!(item["email"].is_string());
    assert!(item["membership_role"].is_string());
    assert!(item["pending"].is_boolean());
    assert!(item["accepted_at"].is_null() || item["accepted_at"].is_string());
    assert!(item["revoked_at"].is_null() || item["revoked_at"].is_string());
    assert!(item["expires_at"].is_string());
    assert!(item.get("token").is_none());
    assert!(item.get("token_digest").is_none());
}

fn assert_no_tokens(value: &Value, fixture: &Fixture) {
    let payload = value.to_string();
    for token in [
        &fixture.invitation_accept_token,
        &fixture.invitation_expired_token,
        &fixture.invitation_revoked_token,
        &fixture.invitation_rotation_token,
        &fixture.invitation_mobile_token,
    ] {
        assert!(!payload.contains(token));
    }
}

fn assert_no_token_shaped_material(value: &Value) {
    match value {
        Value::String(text) => {
            assert!(
                !text
                    .as_bytes()
                    .windows(64)
                    .any(|window| window.iter().all(u8::is_ascii_hexdigit)),
                "public response exposed token-shaped material"
            );
        }
        Value::Array(items) => items.iter().for_each(assert_no_token_shaped_material),
        Value::Object(fields) => {
            assert!(!fields.contains_key("token") && !fields.contains_key("token_digest"));
            fields.values().for_each(assert_no_token_shaped_material);
        }
        _ => {}
    }
}

fn resend_audit_count(target: &Target, fixture: &Fixture, invitation_id: i64) -> usize {
    let path = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.household_id
    );
    let events = body(target.get(&path, Some(&fixture.access_token)));
    events["data"]
        .as_array()
        .expect("audit events")
        .iter()
        .filter(|event| {
            event["event_type"] == "api/admin/invitation/resent"
                && event["metadata"]["target_id"] == invitation_id
        })
        .count()
}

#[test]
fn task_6b_list_create_delete_validate_and_hide_invitation_secrets() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = invitations(&fixture);
    let initial = body(target.get(&path, Some(&fixture.access_token)));
    assert_summary(invitation(&initial, fixture.invitation_accept_id));
    assert_no_tokens(&initial, &fixture);

    let email = format!(
        "contract-created-{}@example.test",
        fixture.invitation_accept_id
    );
    let response = target.post_json_authorized(
        &path,
        &fixture.manager_access_token,
        &json!({"household_invitation": {"email": email, "membership_role": "member"}}),
    );
    assert_eq!(response.status().as_u16(), 201);
    let created = body(response);
    assert_no_token_shaped_material(&created);
    assert_summary(&created["data"]);
    assert_eq!(created["data"]["email"], email);
    assert_eq!(created["data"]["pending"], true);
    let id = created["data"]["id"].as_i64().unwrap();
    let item = format!("{path}/{id}");
    let listed = body(target.get(&path, Some(&fixture.access_token)));
    assert_no_token_shaped_material(&listed);
    assert_eq!(invitation(&listed, id), &created["data"]);
    assert_no_tokens(&listed, &fixture);
    let audit = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.household_id
    );
    let audit_events = body(target.get(&audit, Some(&fixture.access_token)));
    assert_no_tokens(&audit_events, &fixture);
    assert_no_token_shaped_material(&audit_events);

    let response = target.delete(&item, Some(&fixture.manager_access_token));
    assert_eq!(response.status().as_u16(), 204);
    assert!(response.bytes().unwrap().is_empty());
    let listed = body(target.get(&path, Some(&fixture.access_token)));
    let revoked = invitation(&listed, id);
    assert_eq!(revoked["pending"], false);
    assert!(revoked["revoked_at"].is_string());

    assert_error(target.get(&path, None), 401, "unauthorized");
    assert_error(
        target.get(&path, Some(&fixture.view_access_token)),
        403,
        "forbidden",
    );
    assert_error(
        target.get(&path, Some(&fixture.foreign_access_token)),
        403,
        "forbidden",
    );
    let foreign_path = format!(
        "/api/v1/households/{}/admin/invitations",
        fixture.foreign_household_id
    );
    assert_error(
        target.get(&foreign_path, Some(&fixture.access_token)),
        403,
        "forbidden",
    );
    let denied_create = json!({"household_invitation": {
        "email": format!("contract-denied-{id}@example.test"), "membership_role": "member"
    }});
    assert_error(
        target.post_json_authorized(&path, &fixture.view_access_token, &denied_create),
        403,
        "forbidden",
    );
    assert_error(
        target.post_json_authorized(&path, &fixture.foreign_access_token, &denied_create),
        403,
        "forbidden",
    );
    let foreign_create = target.post_json_authorized(
        &foreign_path,
        &fixture.foreign_access_token,
        &json!({"household_invitation": {
            "email": format!("contract-foreign-invitation-{id}@example.test"),
            "membership_role": "member"
        }}),
    );
    assert_eq!(foreign_create.status().as_u16(), 201);
    let foreign_id = body(foreign_create)["data"]["id"].as_i64().unwrap();
    assert_error(
        target.delete(&format!("{path}/{foreign_id}"), Some(&fixture.access_token)),
        404,
        "not_found",
    );
    let foreign_list = body(target.get(&foreign_path, Some(&fixture.foreign_access_token)));
    assert_eq!(invitation(&foreign_list, foreign_id)["pending"], true);
    let primary_list = body(target.get(&path, Some(&fixture.access_token)));
    assert!(!primary_list["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == foreign_id));
    assert_error(
        target.delete(&item, Some(&fixture.view_access_token)),
        403,
        "forbidden",
    );
    assert_error(
        target.delete(&item, Some(&fixture.foreign_access_token)),
        403,
        "forbidden",
    );
    assert_error(
        target.delete(&format!("{path}/999999999"), Some(&fixture.access_token)),
        404,
        "not_found",
    );
    assert_error(
        target.post_json_authorized(&path, &fixture.access_token, &json!({})),
        400,
        "bad_request",
    );
    let invalid = assert_error(
        target.post_json_authorized(
            &path,
            &fixture.access_token,
            &json!({"household_invitation": {"email": "", "membership_role": "member"}}),
        ),
        422,
        "validation_failed",
    );
    assert!(invalid["error"]["errors"]["email"].is_array());
    let duplicate = assert_error(
        target.post_json_authorized(
            &path,
            &fixture.access_token,
            &json!({"household_invitation": {"email": fixture.invitation_duplicate_email, "membership_role": "member"}}),
        ),
        422,
        "validation_failed",
    );
    assert!(duplicate["error"]["errors"]["email"].is_array());
    let final_list = body(target.get(&path, Some(&fixture.access_token)));
    assert_no_token_shaped_material(&final_list);
    assert_eq!(
        final_list["data"].as_array().unwrap().len(),
        initial["data"].as_array().unwrap().len() + 1
    );
}

#[test]
fn task_6b_resend_renews_expired_replays_once_and_rejects_revoked() {
    let target = Target::from_env();
    let fixture = fixture();
    let mailpit = Mailpit::from_env();
    assert!(mailpit
        .message_ids_to(&fixture.invitation_expired_email)
        .is_empty());
    let list = invitations(&fixture);
    let item = format!("{list}/{}", fixture.invitation_expired_id);
    let resend = format!("{item}/resend");
    let accept = "/api/v1/invitations/accept";
    let denied = assert_error(
        target.post_json_authorized(
            accept,
            &fixture.invitation_expired_access_token,
            &json!({"token": fixture.invitation_expired_token}),
        ),
        422,
        "invitation_unavailable",
    );
    assert!(!denied
        .to_string()
        .contains(&fixture.invitation_expired_token));

    let key = format!("contract-resend-{}", fixture.invitation_expired_id);
    let audit_before = resend_audit_count(&target, &fixture, fixture.invitation_expired_id);
    let response = target.post_json_with_key(&resend, &fixture.access_token, &key, &json!({}));
    assert_eq!(response.status().as_u16(), 200);
    assert!(response.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("no-store"));
    let first = body(response);
    assert_no_token_shaped_material(&first);
    assert_eq!(
        first["data"]["invitation_id"],
        fixture.invitation_expired_id.to_string()
    );
    assert_eq!(first["data"]["delivery_status"], "queued");
    assert!(first["data"]["expires_at"].is_string());
    assert!(!first
        .to_string()
        .contains(&fixture.invitation_expired_token));
    let delivered_id = mailpit.wait_for_one_to(&fixture.invitation_expired_email);
    let replacement_token = mailpit.token_from_message(&delivered_id);
    assert!(replacement_token != fixture.invitation_expired_token);
    assert_eq!(
        resend_audit_count(&target, &fixture, fixture.invitation_expired_id),
        audit_before + 1
    );
    let renewed = body(target.get(&list, Some(&fixture.access_token)));
    assert_no_token_shaped_material(&renewed);
    assert_eq!(
        invitation(&renewed, fixture.invitation_expired_id)["pending"],
        true
    );
    assert_eq!(
        invitation(&renewed, fixture.invitation_expired_id)["expires_at"],
        first["data"]["expires_at"]
    );
    assert_error(
        target.post_json_authorized(
            accept,
            &fixture.invitation_expired_access_token,
            &json!({"token": fixture.invitation_expired_token}),
        ),
        422,
        "invitation_unavailable",
    );
    let replay = target.post_json_with_key(&resend, &fixture.access_token, &key, &json!({}));
    assert_eq!(replay.status().as_u16(), 200);
    assert!(body(replay) == first, "resend replay response changed");
    let after_replay = body(target.get(&list, Some(&fixture.access_token)));
    assert_no_token_shaped_material(&after_replay);
    assert_eq!(
        invitation(&after_replay, fixture.invitation_expired_id),
        invitation(&renewed, fixture.invitation_expired_id)
    );
    assert_eq!(
        resend_audit_count(&target, &fixture, fixture.invitation_expired_id),
        audit_before + 1
    );
    assert_eq!(
        mailpit.message_ids_to(&fixture.invitation_expired_email),
        vec![delivered_id]
    );
    assert_error(
        target.post_json_authorized(&resend, &fixture.manager_app_token, &json!({})),
        403,
        "forbidden",
    );
    assert_error(
        target.post_json_authorized(&resend, &fixture.foreign_access_token, &json!({})),
        403,
        "forbidden",
    );
    assert_error(
        target.post_json_authorized(
            &format!("{list}/999999999/resend"),
            &fixture.access_token,
            &json!({}),
        ),
        404,
        "not_found",
    );
    let response = target.delete(&item, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 204);
    assert_error(
        target.post_json_authorized(
            accept,
            &fixture.invitation_expired_access_token,
            &json!({"token": replacement_token}),
        ),
        422,
        "invitation_unavailable",
    );
    assert_error(
        target.post_json_authorized(&resend, &fixture.access_token, &json!({})),
        422,
        "unprocessable_content",
    );
}

#[test]
fn task_6b_resend_invalidates_a_pending_matching_identity_token() {
    let target = Target::from_env();
    let fixture = fixture();
    let mailpit = Mailpit::from_env();
    assert!(mailpit
        .message_ids_to(&fixture.invitation_rotation_email)
        .is_empty());
    let list = invitations(&fixture);
    let before = body(target.get(&list, Some(&fixture.access_token)));
    let old = invitation(&before, fixture.invitation_rotation_id);
    assert_eq!(old["pending"], true);
    assert_no_tokens(&before, &fixture);
    let resend = format!("{list}/{}/resend", fixture.invitation_rotation_id);
    let response = target.post_json_authorized(&resend, &fixture.manager_access_token, &json!({}));
    assert_eq!(response.status().as_u16(), 200);
    let resend_body = body(response);
    assert_no_token_shaped_material(&resend_body);
    assert_eq!(resend_body["data"]["delivery_status"], "queued");
    let delivered_id = mailpit.wait_for_one_to(&fixture.invitation_rotation_email);
    let replacement_token = mailpit.token_from_message(&delivered_id);
    assert!(replacement_token != fixture.invitation_rotation_token);
    let after = body(target.get(&list, Some(&fixture.access_token)));
    assert_no_token_shaped_material(&after);
    assert_eq!(
        invitation(&after, fixture.invitation_rotation_id)["pending"],
        true
    );
    assert_no_tokens(&after, &fixture);
    let unavailable = assert_error(
        target.post_json_authorized(
            "/api/v1/invitations/accept",
            &fixture.invitation_rotation_access_token,
            &json!({"token": fixture.invitation_rotation_token}),
        ),
        422,
        "invitation_unavailable",
    );
    assert!(!unavailable
        .to_string()
        .contains(&fixture.invitation_rotation_token));
    let households = body(target.get(
        "/api/v1/auth/households",
        Some(&fixture.invitation_rotation_access_token),
    ));
    assert!(!households["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == fixture.household_id));
    let accepted = target.post_json_authorized(
        "/api/v1/invitations/accept",
        &fixture.invitation_rotation_access_token,
        &json!({"token": replacement_token}),
    );
    assert_eq!(accepted.status().as_u16(), 200);
    let accepted_body = body(accepted);
    assert_eq!(
        accepted_body["data"]["household_id"],
        fixture.household_id.to_string()
    );
    assert_no_token_shaped_material(&accepted_body);
    let households = body(target.get(
        "/api/v1/auth/households",
        Some(&fixture.invitation_rotation_access_token),
    ));
    assert_eq!(
        households["data"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["id"] == fixture.household_id)
            .count(),
        1
    );
}

#[test]
fn task_6b_accepts_once_with_grants_and_public_auth_readback() {
    let target = Target::from_env();
    let fixture = fixture();
    let accept = "/api/v1/invitations/accept";
    let payload = json!({"token": fixture.invitation_accept_token});
    let snapshot = format!("/api/v1/households/{}/sync/snapshot", fixture.household_id);
    let cursor = body(target.get(&snapshot, Some(&fixture.access_token)))["data"]["cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let before = body(target.get(
        "/api/v1/auth/households",
        Some(&fixture.invitation_accept_access_token),
    ));
    assert!(!before["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == fixture.household_id));

    let response =
        target.post_json_authorized(accept, &fixture.invitation_accept_access_token, &payload);
    assert_eq!(response.status().as_u16(), 200);
    assert!(response.headers()["cache-control"]
        .to_str()
        .unwrap()
        .contains("no-store"));
    let accepted = body(response);
    assert_no_token_shaped_material(&accepted);
    assert_eq!(
        accepted["data"]["household_id"],
        fixture.household_id.to_string()
    );
    assert_eq!(accepted["data"]["role"], "member");
    assert!(accepted["data"]["membership_id"].is_string());
    assert!(accepted["data"]["person_id"].is_string());
    assert!(!accepted
        .to_string()
        .contains(&fixture.invitation_accept_token));

    let replay =
        target.post_json_authorized(accept, &fixture.invitation_accept_access_token, &payload);
    assert_eq!(replay.status().as_u16(), 200);
    assert!(body(replay) == accepted, "accept replay response changed");
    let after = body(target.get(
        "/api/v1/auth/households",
        Some(&fixture.invitation_accept_access_token),
    ));
    assert_eq!(after["account_id"], fixture.invitation_accept_account_id);
    let matching: Vec<_> = after["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["id"] == fixture.household_id)
        .collect();
    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0]["role"], "member");
    let owner_list = body(target.get(&invitations(&fixture), Some(&fixture.access_token)));
    let summary = invitation(&owner_list, fixture.invitation_accept_id);
    assert_eq!(summary["pending"], false);
    assert!(summary["accepted_at"].is_string());
    assert!(!owner_list
        .to_string()
        .contains(&fixture.invitation_accept_token));
    let feed = format!(
        "/api/v1/households/{}/sync/changes?cursor={cursor}",
        fixture.household_id
    );
    let changes = target.get(&feed, Some(&fixture.access_token));
    assert_eq!(changes.status().as_u16(), 200);
    assert_no_tokens(&body(changes), &fixture);

    let memberships = format!(
        "/api/v1/households/{}/admin/memberships",
        fixture.household_id
    );
    let members = body(target.get(&memberships, Some(&fixture.access_token)));
    let matching: Vec<_> = members["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["account_id"] == fixture.invitation_accept_account_id)
        .collect();
    assert_eq!(matching.len(), 1);
    assert_eq!(
        matching[0]["id"].as_i64().unwrap().to_string(),
        accepted["data"]["membership_id"]
    );
    let grants = format!(
        "/api/v1/households/{}/admin/person_access_grants",
        fixture.household_id
    );
    let granted = body(target.get(&grants, Some(&fixture.access_token)));
    let matching: Vec<_> = granted["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row["household_membership_id"] == matching[0]["id"]
                && row["person_id"] == fixture.managed_person_id
                && row["revoked_at"].is_null()
        })
        .collect();
    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0]["access_level"], "record");
    let resend = format!(
        "{}/{}/resend",
        invitations(&fixture),
        fixture.invitation_accept_id
    );
    assert_error(
        target.post_json_authorized(&resend, &fixture.access_token, &json!({})),
        422,
        "unprocessable_content",
    );
}

#[test]
fn task_6b_revocation_prevents_acceptance_and_keeps_membership_absent() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = format!(
        "{}/{}",
        invitations(&fixture),
        fixture.invitation_revoked_id
    );
    let response = target.delete(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 204);
    assert_error(
        target.post_json_authorized(
            "/api/v1/invitations/accept",
            &fixture.invitation_revoked_access_token,
            &json!({"token": fixture.invitation_revoked_token}),
        ),
        422,
        "invitation_unavailable",
    );
    let households = body(target.get(
        "/api/v1/auth/households",
        Some(&fixture.invitation_revoked_access_token),
    ));
    assert!(!households["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == fixture.household_id));
    let list = body(target.get(&invitations(&fixture), Some(&fixture.access_token)));
    let revoked = invitation(&list, fixture.invitation_revoked_id);
    assert_eq!(revoked["pending"], false);
    assert!(revoked["revoked_at"].is_string());
    assert_no_tokens(&list, &fixture);
}

#[test]
fn task_6b_acceptance_rejects_wrong_identity_app_token_and_unknown_token() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = "/api/v1/invitations/accept";
    let request = json!({"token": fixture.invitation_accept_token});
    assert_error(target.post_json(path, &request), 401, "unauthorized");
    assert_error(
        target.post_json_authorized(path, &fixture.foreign_access_token, &request),
        422,
        "invitation_unavailable",
    );
    assert_error(
        target.post_json_authorized(path, &fixture.manager_app_token, &request),
        403,
        "forbidden",
    );
    let missing = assert_error(
        target.post_json_authorized(
            path,
            &fixture.invitation_accept_access_token,
            &json!({"token": "missing-invitation-token"}),
        ),
        422,
        "invitation_unavailable",
    );
    assert!(!missing
        .to_string()
        .contains(&fixture.invitation_accept_email));
    assert_error(
        target.post_json_authorized(path, &fixture.invitation_accept_access_token, &json!({})),
        400,
        "bad_request",
    );
}

#[test]
fn task_6b_mobile_oauth_acceptance_follows_observed_rails_behavior() {
    let target = Target::from_env();
    let fixture = fixture();
    let request = json!({"token": fixture.invitation_mobile_token});
    let response = target.post_json_authorized(
        "/api/v1/invitations/accept",
        &fixture.invitation_mobile_oauth_token,
        &request,
    );
    assert_eq!(response.status().as_u16(), 200);
    let accepted = body(response);
    assert_no_token_shaped_material(&accepted);
    assert_eq!(
        accepted["data"]["household_id"],
        fixture.household_id.to_string()
    );
    assert_eq!(accepted["data"]["role"], "member");
    let households = body(target.get(
        "/api/v1/auth/households",
        Some(&fixture.invitation_mobile_oauth_token),
    ));
    assert!(households["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == fixture.household_id));
}

#[test]
fn task_6b_resend_rechecks_current_manager_authority_before_cached_replay() {
    let target = Target::from_env();
    let fixture = fixture();
    let resend = format!(
        "{}/{}/resend",
        invitations(&fixture),
        fixture.invitation_authority_id
    );
    let key = format!("contract-authority-{}", fixture.invitation_authority_id);
    let first = target.post_json_with_key(
        &resend,
        &fixture.invitation_authority_access_token,
        &key,
        &json!({}),
    );
    assert_eq!(first.status().as_u16(), 200);
    let first_body = body(first);
    let membership = format!(
        "/api/v1/households/{}/admin/memberships/{}",
        fixture.household_id, fixture.invitation_authority_membership_id
    );
    let demoted = target.patch_json(
        &membership,
        &fixture.access_token,
        &json!({"household_membership": {"role": "member"}}),
    );
    assert_eq!(demoted.status().as_u16(), 200);
    assert_eq!(body(demoted)["data"]["role"], "member");
    let replay = target.post_json_with_key(
        &resend,
        &fixture.invitation_authority_access_token,
        &key,
        &json!({}),
    );
    assert_error(replay, 401, "unauthorized");
    let listed = body(target.get(&invitations(&fixture), Some(&fixture.access_token)));
    assert_eq!(
        invitation(&listed, fixture.invitation_authority_id)["expires_at"],
        first_body["data"]["expires_at"]
    );
}
