use medtracker_contract_tests::{Fixture, Target, fixture};
use reqwest::blocking::{Client, Response};
use serde_json::{Value, json};
use std::env;
use std::thread;
use std::time::Duration;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use url::{Host, Url};

const SUMMARY_FIELDS: [&str; 7] = [
    "id",
    "email",
    "membership_role",
    "pending",
    "accepted_at",
    "revoked_at",
    "expires_at",
];

fn invitations(fixture: &Fixture) -> String {
    format!(
        "/api/v1/households/{}/admin/invitations",
        fixture.household_id
    )
}

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn assert_error(response: Response, status: u16, code: &str) -> Value {
    assert_eq!(response.status().as_u16(), status);
    let request_id = response.headers()["x-request-id"]
        .to_str()
        .expect("request ID")
        .to_owned();
    let value = body(response);
    assert_eq!(value["error"]["code"], code);
    assert_eq!(value["error"]["request_id"], request_id);
    assert!(value["error"]["message"].as_str().is_some());
    assert!(value.get("data").is_none());
    value
}

fn assert_summary(summary: &Value) {
    let fields = summary.as_object().expect("invitation summary object");
    assert_eq!(fields.len(), SUMMARY_FIELDS.len());
    for field in SUMMARY_FIELDS {
        assert!(
            fields.contains_key(field),
            "missing invitation field {field}"
        );
    }
    assert!(summary["id"].as_i64().is_some());
    assert!(
        summary["email"]
            .as_str()
            .is_some_and(|value| value.contains('@'))
    );
    assert!(matches!(
        summary["membership_role"].as_str(),
        Some("administrator" | "member")
    ));
    assert!(summary["pending"].is_boolean());
    for field in ["accepted_at", "revoked_at"] {
        if let Some(timestamp) = summary[field].as_str() {
            OffsetDateTime::parse(timestamp, &Rfc3339).expect("RFC3339 invitation timestamp");
        } else {
            assert!(summary[field].is_null());
        }
    }
    OffsetDateTime::parse(
        summary["expires_at"].as_str().expect("invitation expiry"),
        &Rfc3339,
    )
    .expect("RFC3339 invitation expiry");
    assert!(summary.get("token").is_none());
    assert!(summary.get("token_digest").is_none());
}

fn assert_no_token_shaped_material(value: &Value) {
    match value {
        Value::String(text) => assert!(
            !text
                .as_bytes()
                .windows(64)
                .any(|window| window.iter().all(u8::is_ascii_hexdigit)),
            "public invitation response exposed token-shaped material"
        ),
        Value::Array(items) => items.iter().for_each(assert_no_token_shaped_material),
        Value::Object(fields) => {
            assert!(!fields.contains_key("token") && !fields.contains_key("token_digest"));
            fields.values().for_each(assert_no_token_shaped_material);
        }
        _ => {}
    }
}

fn invitation_audit_count(
    target: &Target,
    fixture: &Fixture,
    invitation_id: i64,
    event_type: &str,
) -> usize {
    let path = format!(
        "/api/v1/households/{}/admin/audit_logs",
        fixture.household_id
    );
    let audit = body(target.get(&path, Some(&fixture.access_token)));
    audit["data"]
        .as_array()
        .expect("audit events")
        .iter()
        .filter(|event| {
            event["event_type"] == event_type && event["metadata"]["target_id"] == invitation_id
        })
        .count()
}

fn audit_database() -> postgres::Client {
    postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        postgres::NoTls,
    )
    .expect("contract database")
}

fn invitation_delivery_state(
    db: &mut postgres::Client,
    invitation_id: i64,
) -> (String, String, String, Option<String>, Option<String>) {
    let row = db
        .query_one(
        "SELECT token_digest, expires_at::text, updated_at::text, accepted_at::text, revoked_at::text FROM household_invitations WHERE id = $1",
        &[&invitation_id],
    )
        .expect("invitation delivery state");
    (row.get(0), row.get(1), row.get(2), row.get(3), row.get(4))
}

fn mailpit() -> (Client, Url) {
    let origin = Url::parse(&env::var("CONTRACT_MAILPIT_URL").expect("Mailpit URL"))
        .expect("valid Mailpit URL");
    assert_eq!(origin.scheme(), "http");
    assert!(
        matches!(origin.host(), Some(Host::Ipv4(address)) if address.is_loopback())
            || matches!(origin.host(), Some(Host::Domain("mail-test")))
    );
    assert_eq!(origin.path(), "/");
    assert!(origin.username().is_empty() && origin.password().is_none());
    assert!(origin.query().is_none() && origin.fragment().is_none());
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .expect("Mailpit client");
    (client, origin)
}

fn message_ids_to(client: &Client, origin: &Url, email: &str) -> Vec<String> {
    let url = origin
        .join("api/v1/messages?start=0&limit=100")
        .expect("Mailpit messages URL");
    let response = client.get(url).send().expect("Mailpit message list");
    assert_eq!(response.status().as_u16(), 200);
    let payload: Value = response.json().expect("Mailpit messages JSON");
    payload["messages"]
        .as_array()
        .expect("Mailpit messages")
        .iter()
        .filter(|message| {
            message["To"].as_array().is_some_and(|recipients| {
                recipients.iter().any(|item| {
                    item["Address"]
                        .as_str()
                        .or_else(|| item["address"].as_str())
                        .is_some_and(|address| address.eq_ignore_ascii_case(email))
                })
            })
        })
        .map(|message| message["ID"].as_str().expect("Mailpit ID").to_owned())
        .collect()
}

fn wait_for_one_message(client: &Client, origin: &Url, email: &str) -> String {
    for _ in 0..30 {
        let ids = message_ids_to(client, origin, email);
        if ids.len() == 1 {
            return ids[0].clone();
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("expected exactly one invitation message");
}

fn message_token(client: &Client, origin: &Url, message_id: &str) -> String {
    assert!(
        message_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    );
    let url = origin
        .join(&format!("api/v1/message/{message_id}"))
        .expect("Mailpit message URL");
    let response = client.get(url).send().expect("Mailpit message");
    assert_eq!(response.status().as_u16(), 200);
    let message: Value = response.json().expect("Mailpit message JSON");
    let text = message["Text"]
        .as_str()
        .or_else(|| message["text"].as_str())
        .expect("invitation email text");
    let accept_url = text
        .lines()
        .find(|line| line.contains("/invitations/accept?token="))
        .expect("invitation acceptance URL");
    let token = Url::parse(accept_url.trim())
        .expect("acceptance URL")
        .query_pairs()
        .find(|(key, _)| key == "token")
        .expect("invitation token")
        .1
        .into_owned();
    assert_eq!(token.len(), 64);
    assert!(token.bytes().all(|byte| byte.is_ascii_hexdigit()));
    token
}

#[test]
fn list_create_and_revoke_use_the_documented_invitation_shapes() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = invitations(&fixture);
    let response = target.get(&path, Some(&fixture.access_token));
    assert_eq!(response.status().as_u16(), 200);
    let listing = body(response);
    assert_eq!(listing.as_object().unwrap().len(), 1);
    let rows = listing["data"].as_array().expect("invitation collection");
    let initial = rows
        .iter()
        .find(|row| row["id"] == fixture.invitation_accept_id)
        .expect("fixture invitation");
    assert_summary(initial);
    assert_no_token_shaped_material(&listing);

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
    assert_error(
        target.get(
            &format!(
                "/api/v1/households/{}/admin/invitations",
                fixture.foreign_household_id
            ),
            Some(&fixture.access_token),
        ),
        403,
        "forbidden",
    );

    let email = format!(
        "openapi-invitation-{}@example.test",
        fixture.invitation_accept_id
    );
    let payload = json!({"household_invitation": {
        "email": email,
        "membership_role": "member"
    }});
    let created = target.post_json_authorized(&path, &fixture.manager_access_token, &payload);
    assert_eq!(created.status().as_u16(), 201);
    let created_body = body(created);
    assert_eq!(created_body.as_object().unwrap().len(), 1);
    assert_summary(&created_body["data"]);
    assert_eq!(created_body["data"]["email"], email);
    assert!(created_body["data"]["pending"].as_bool().unwrap());
    assert!(created_body["data"]["accepted_at"].is_null());
    assert!(created_body["data"]["revoked_at"].is_null());
    assert_no_token_shaped_material(&created_body);
    let id = created_body["data"]["id"].as_i64().unwrap();
    assert_eq!(
        invitation_audit_count(&target, &fixture, id, "api/admin/invitation/created"),
        1
    );

    let unauthorized_create = target.post_json(&path, &payload);
    assert_error(unauthorized_create, 401, "unauthorized");
    assert_error(
        target.post_json_authorized(&path, &fixture.view_access_token, &payload),
        403,
        "forbidden",
    );
    assert_error(
        target.post_json_authorized(
            &path,
            &fixture.access_token,
            &json!({"household_invitation": {"email": "", "membership_role": "member"}}),
        ),
        422,
        "validation_failed",
    );
    assert_error(
        target.post_json_authorized(
            &path,
            &fixture.access_token,
            &json!({"household_invitation": {"email": format!("openapi-invalid-role-{id}@example.test"), "membership_role": "owner"}}),
        ),
        422,
        "validation_failed",
    );
    assert_error(
        target.post_json_authorized(
            &path,
            &fixture.access_token,
            &json!({"household_invitation": {"email": "not-an-email", "membership_role": "member"}}),
        ),
        422,
        "validation_failed",
    );
    assert_error(
        target.post_json_authorized(
            &path,
            &fixture.access_token,
            &json!({"household_invitation": {"email": format!("openapi-unknown-field-{id}@example.test"), "membership_role": "member", "token": "unexpected"}}),
        ),
        422,
        "validation_failed",
    );
    assert_error(
        target.post_json_authorized(
            &path,
            &fixture.access_token,
            &json!({"household_invitation": {"email": fixture.invitation_duplicate_email, "membership_role": "member"}}),
        ),
        422,
        "validation_failed",
    );
    assert_error(
        target.post_json_authorized(&path, &fixture.access_token, &json!({})),
        400,
        "bad_request",
    );

    let listed = body(target.get(&path, Some(&fixture.access_token)));
    let created_again = listed["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id)
        .expect("created invite listed");
    assert_summary(created_again);
    assert_eq!(created_again, &created_body["data"]);
    assert_no_token_shaped_material(&listed);

    let deleted = target.delete(&format!("{path}/{id}"), Some(&fixture.access_token));
    assert_eq!(deleted.status().as_u16(), 204);
    assert!(deleted.bytes().unwrap().is_empty());
    assert_eq!(
        invitation_audit_count(&target, &fixture, id, "api/admin/invitation/revoked"),
        1
    );
    let after = body(target.get(&path, Some(&fixture.access_token)));
    let revoked = after["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id)
        .expect("revoked invite remains listed");
    assert_summary(revoked);
    assert!(!revoked["pending"].as_bool().unwrap());
    assert!(revoked["revoked_at"].is_string());
    assert_error(
        target.delete(&format!("{path}/{id}"), Some(&fixture.view_access_token)),
        403,
        "forbidden",
    );
    assert_error(
        target.delete(
            &format!("{path}/{}", fixture.invitation_authority_id),
            Some(&fixture.foreign_access_token),
        ),
        403,
        "forbidden",
    );
    assert_error(
        target.delete(&format!("{path}/999999999"), Some(&fixture.access_token)),
        404,
        "not_found",
    );
    assert_no_token_shaped_material(&after);
}

#[test]
fn keyed_create_and_revoke_replay_without_duplicate_invitation_effects() {
    let target = Target::from_env();
    let fixture = fixture();
    let collection = invitations(&fixture);
    let nonce = OffsetDateTime::now_utc().unix_timestamp_nanos();
    let email = format!("openapi-keyed-invitation-{nonce}@example.test");
    let payload = json!({"household_invitation": {
        "email": email,
        "membership_role": "member"
    }});
    let create_key = format!("openapi-invitation-create-{nonce}");
    let created =
        target.post_json_with_key(&collection, &fixture.access_token, &create_key, &payload);
    assert_eq!(created.status().as_u16(), 201);
    let created = body(created);
    assert_summary(&created["data"]);
    assert_no_token_shaped_material(&created);
    let id = created["data"]["id"]
        .as_i64()
        .expect("created invitation ID");
    let replay =
        target.post_json_with_key(&collection, &fixture.access_token, &create_key, &payload);
    assert_eq!(replay.status().as_u16(), 201);
    assert_eq!(replay.headers()["idempotency-replayed"], "true");
    assert_eq!(body(replay), created);
    let changed = json!({"household_invitation": {
        "email": format!("openapi-keyed-changed-{nonce}@example.test"),
        "membership_role": "member"
    }});
    assert_error(
        target.post_json_with_key(&collection, &fixture.access_token, &create_key, &changed),
        409,
        "idempotency_key_reused",
    );
    let listed = body(target.get(&collection, Some(&fixture.access_token)));
    assert_eq!(
        listed["data"]
            .as_array()
            .expect("invitation list")
            .iter()
            .filter(|row| row["email"] == email)
            .count(),
        1
    );
    assert_eq!(
        invitation_audit_count(&target, &fixture, id, "api/admin/invitation/created"),
        1
    );

    let invalid_key = format!("openapi-invitation-invalid-{nonce}");
    let invalid = json!({"household_invitation": {
        "email": format!("openapi-keyed-invalid-{nonce}@example.test"),
        "membership_role": "owner"
    }});
    let first_invalid = assert_error(
        target.post_json_with_key(&collection, &fixture.access_token, &invalid_key, &invalid),
        422,
        "validation_failed",
    );
    let replay_invalid =
        target.post_json_with_key(&collection, &fixture.access_token, &invalid_key, &invalid);
    assert_eq!(replay_invalid.headers()["idempotency-replayed"], "true");
    let replay_invalid = assert_error(replay_invalid, 422, "validation_failed");
    assert_ne!(
        first_invalid["error"]["request_id"],
        replay_invalid["error"]["request_id"]
    );
    assert_error(
        target.post_json_with_key(&collection, &fixture.access_token, &invalid_key, &changed),
        409,
        "idempotency_key_reused",
    );
    let after_invalid = body(target.get(&collection, Some(&fixture.access_token)));
    assert!(
        after_invalid["data"]
            .as_array()
            .expect("invitation list after invalid replay")
            .iter()
            .all(|row| row["email"] != invalid["household_invitation"]["email"])
    );

    let other = target.post_json_authorized(&collection, &fixture.access_token, &changed);
    assert_eq!(other.status().as_u16(), 201);
    let other_id = body(other)["data"]["id"]
        .as_i64()
        .expect("second invitation ID");
    let delete_key = format!("openapi-invitation-delete-{nonce}");
    let base = env::var("CONTRACT_BASE_URL").expect("contract base URL");
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .no_proxy()
        .build()
        .expect("HTTP client");
    let delete_keyed = |target_id| {
        client
            .delete(format!(
                "{}{collection}/{target_id}",
                base.trim_end_matches('/')
            ))
            .bearer_auth(&fixture.access_token)
            .header("Idempotency-Key", &delete_key)
            .send()
            .expect("keyed invitation delete response")
    };
    let first_delete = delete_keyed(id);
    assert_eq!(first_delete.status().as_u16(), 204);
    assert!(first_delete.bytes().expect("empty delete body").is_empty());
    let replay_delete = delete_keyed(id);
    assert_eq!(replay_delete.status().as_u16(), 204);
    assert_eq!(replay_delete.headers()["idempotency-replayed"], "true");
    assert!(replay_delete.bytes().expect("empty replay body").is_empty());
    assert_error(delete_keyed(other_id), 409, "idempotency_key_reused");
    assert_eq!(
        invitation_audit_count(&target, &fixture, id, "api/admin/invitation/revoked"),
        1
    );
    assert_eq!(
        invitation_audit_count(&target, &fixture, other_id, "api/admin/invitation/revoked"),
        0
    );
    let cleanup = target.delete(
        &format!("{collection}/{other_id}"),
        Some(&fixture.access_token),
    );
    assert_eq!(cleanup.status().as_u16(), 204);
}

#[test]
fn expired_invitation_resend_rotates_once_and_accepts_the_replacement() {
    let target = Target::from_env();
    let fixture = fixture();
    let (mail_client, mail_origin) = mailpit();
    assert!(
        message_ids_to(
            &mail_client,
            &mail_origin,
            &fixture.invitation_expired_email
        )
        .is_empty()
    );
    let list = invitations(&fixture);
    let item = format!("{list}/{}", fixture.invitation_expired_id);
    let resend = format!("{item}/resend");
    let key = format!("openapi-resend-{}", fixture.invitation_expired_id);
    let old = fixture.invitation_expired_token.clone();
    let mut db = audit_database();
    let membership_count_before: i64 = db
        .query_one(
            "SELECT count(*)::bigint FROM household_memberships WHERE account_id = $1 AND household_id = $2",
            &[&fixture.invitation_expired_account_id, &fixture.household_id],
        )
        .expect("pre-acceptance household membership count")
        .get(0);
    assert_eq!(membership_count_before, 0);
    let resend_audits_before = invitation_audit_count(
        &target,
        &fixture,
        fixture.invitation_expired_id,
        "api/admin/invitation/resent",
    );
    let first = target.post_json_with_key(&resend, &fixture.access_token, &key, &json!({}));
    assert_eq!(first.status().as_u16(), 200);
    assert!(
        first.headers()["cache-control"]
            .to_str()
            .unwrap()
            .contains("no-store")
    );
    let first_body = body(first);
    assert_eq!(first_body.as_object().unwrap().len(), 1);
    assert_eq!(first_body["data"].as_object().unwrap().len(), 3);
    assert_eq!(
        first_body["data"]["invitation_id"],
        fixture.invitation_expired_id.to_string()
    );
    assert_eq!(first_body["data"]["delivery_status"], "queued");
    OffsetDateTime::parse(first_body["data"]["expires_at"].as_str().unwrap(), &Rfc3339)
        .expect("RFC3339 resend expiry");
    assert_no_token_shaped_material(&first_body);

    let message_id = wait_for_one_message(
        &mail_client,
        &mail_origin,
        &fixture.invitation_expired_email,
    );
    let replacement = message_token(&mail_client, &mail_origin, &message_id);
    assert_ne!(replacement, old);
    assert_eq!(
        message_ids_to(
            &mail_client,
            &mail_origin,
            &fixture.invitation_expired_email
        ),
        vec![message_id.clone()]
    );
    assert_eq!(
        invitation_audit_count(
            &target,
            &fixture,
            fixture.invitation_expired_id,
            "api/admin/invitation/resent",
        ),
        resend_audits_before + 1
    );
    let replay = target.post_json_with_key(&resend, &fixture.access_token, &key, &json!({}));
    assert_eq!(replay.status().as_u16(), 200);
    assert_eq!(replay.headers()["idempotency-replayed"], "true");
    assert_eq!(body(replay), first_body);
    assert_eq!(
        message_ids_to(
            &mail_client,
            &mail_origin,
            &fixture.invitation_expired_email
        ),
        vec![message_id]
    );
    assert_eq!(
        invitation_audit_count(
            &target,
            &fixture,
            fixture.invitation_expired_id,
            "api/admin/invitation/resent",
        ),
        resend_audits_before + 1
    );

    let old_attempt = target.post_json_authorized(
        "/api/v1/invitations/accept",
        &fixture.invitation_expired_access_token,
        &json!({"token": old}),
    );
    let old_error = assert_error(old_attempt, 422, "invitation_unavailable");
    assert!(!old_error.to_string().contains(&old));
    assert_eq!(
        db.query_one(
            "SELECT count(*)::bigint FROM household_memberships WHERE account_id = $1 AND household_id = $2",
            &[&fixture.invitation_expired_account_id, &fixture.household_id],
        )
        .expect("membership count after rejected old token")
        .get::<_, i64>(0),
        membership_count_before
    );
    let accepted = target.post_json_authorized(
        "/api/v1/invitations/accept",
        &fixture.invitation_expired_access_token,
        &json!({"token": replacement}),
    );
    assert_eq!(accepted.status().as_u16(), 200);
    let acceptance_request_id = accepted.headers()["x-request-id"]
        .to_str()
        .expect("acceptance request ID")
        .to_owned();
    assert!(
        accepted.headers()["cache-control"]
            .to_str()
            .unwrap()
            .contains("no-store")
    );
    let accepted_body = body(accepted);
    assert_eq!(accepted_body.as_object().unwrap().len(), 1);
    assert_eq!(accepted_body["data"].as_object().unwrap().len(), 4);
    assert_eq!(
        accepted_body["data"]["household_id"],
        fixture.household_id.to_string()
    );
    assert!(
        accepted_body["data"]["membership_id"]
            .as_str()
            .is_some_and(|id| id.parse::<i64>().is_ok())
    );
    assert!(
        accepted_body["data"]["person_id"]
            .as_str()
            .is_some_and(|id| id.parse::<i64>().is_ok())
    );
    assert_eq!(accepted_body["data"]["role"], "member");
    assert_no_token_shaped_material(&accepted_body);
    let accepted_membership_id = accepted_body["data"]["membership_id"]
        .as_str()
        .expect("accepted membership ID")
        .parse::<i64>()
        .expect("numeric membership ID");
    let accepted_person_id = accepted_body["data"]["person_id"]
        .as_str()
        .expect("accepted person ID")
        .parse::<i64>()
        .expect("numeric person ID");
    assert_eq!(
        db.query_one(
            "SELECT count(*)::bigint FROM versions WHERE item_type = 'HouseholdInvitation' AND item_id = $1 AND request_id = $2",
            &[&fixture.invitation_expired_id, &acceptance_request_id],
        )
        .expect("correlated invitation version")
        .get::<_, i64>(0),
        1
    );
    assert_eq!(
        db.query_one(
            "SELECT count(*)::bigint FROM security_audit_events WHERE household_id = $1 AND event_type = 'household_access.membership_created' AND metadata->>'target_membership_id' = $2 AND request_id = $3",
            &[&fixture.household_id, &accepted_membership_id.to_string(), &acceptance_request_id],
        )
        .expect("correlated membership audit")
        .get::<_, i64>(0),
        1
    );
    assert_eq!(
        db.query_one(
            "SELECT count(*)::bigint FROM api_change_events WHERE household_id = $1 AND record_type = 'Person' AND record_id = $2 AND action = 'create' AND request_id = $3",
            &[&fixture.household_id, &accepted_person_id, &acceptance_request_id],
        )
        .expect("correlated person sync event")
        .get::<_, i64>(0),
        1
    );
    let retry = target.post_json_authorized(
        "/api/v1/invitations/accept",
        &fixture.invitation_expired_access_token,
        &json!({"token": replacement}),
    );
    assert_eq!(retry.status().as_u16(), 200);
    assert_eq!(body(retry), accepted_body);
    assert_eq!(
        db.query_one(
            "SELECT count(*)::bigint FROM household_memberships WHERE account_id = $1 AND household_id = $2",
            &[&fixture.invitation_expired_account_id, &fixture.household_id],
        )
        .expect("membership count after accepted replacement")
        .get::<_, i64>(0),
        1
    );
}

#[test]
fn acceptance_requires_matching_user_session_and_rejects_unavailable_tokens() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = "/api/v1/invitations/accept";
    let payload = json!({"token": fixture.invitation_accept_token});
    assert_error(target.post_json(path, &payload), 401, "unauthorized");
    assert_error(
        target.post_json_authorized(path, &fixture.foreign_access_token, &payload),
        422,
        "invitation_unavailable",
    );
    assert_error(
        target.post_json_authorized(path, &fixture.manager_app_token, &payload),
        403,
        "forbidden",
    );
    assert_error(
        target.post_json_authorized(
            path,
            &fixture.invitation_mobile_oauth_token,
            &json!({"token": fixture.invitation_mobile_token}),
        ),
        403,
        "forbidden",
    );
    assert_error(
        target.post_json_authorized(
            path,
            &fixture.invitation_accept_access_token,
            &json!({"token": "unknown-invitation-token"}),
        ),
        422,
        "invitation_unavailable",
    );
    assert_error(
        target.post_json_authorized(path, &fixture.invitation_accept_access_token, &json!({})),
        400,
        "bad_request",
    );
    assert_error(
        target.post_json_authorized(
            path,
            &fixture.invitation_accept_access_token,
            &json!({"token": fixture.invitation_accept_token, "unexpected": true}),
        ),
        422,
        "validation_failed",
    );
    let revoke_path = format!(
        "/api/v1/households/{}/admin/invitations/{}",
        fixture.household_id, fixture.invitation_revoked_id
    );
    let revoke = target.delete(&revoke_path, Some(&fixture.access_token));
    assert_eq!(revoke.status().as_u16(), 204);
    let mut db = audit_database();
    assert!(
        db.query_one(
            "SELECT revoked_at IS NOT NULL FROM household_invitations WHERE id = $1",
            &[&fixture.invitation_revoked_id],
        )
        .expect("revoked invitation state")
        .get::<_, bool>(0)
    );
    let revoked = target.post_json_authorized(
        path,
        &fixture.invitation_revoked_access_token,
        &json!({"token": fixture.invitation_revoked_token}),
    );
    assert_eq!(
        revoked.status().as_u16(),
        422,
        "revoked invitation must be unavailable"
    );
    assert_error(revoked, 422, "invitation_unavailable");
    let expired = target.post_json_authorized(
        path,
        &fixture.invitation_expired_access_token,
        &json!({"token": fixture.invitation_expired_token}),
    );
    assert_error(expired, 422, "invitation_unavailable");
}

#[test]
fn accepted_professional_grant_creates_relationship_and_linked_access_grant() {
    let target = Target::from_env();
    let fixture = fixture();
    let mut db = audit_database();
    let membership_count: i64 = db
        .query_one(
            "SELECT count(*)::bigint FROM household_memberships WHERE household_id = $1 AND account_id = $2",
            &[&fixture.household_id, &fixture.invitation_accept_account_id],
        )
        .expect("target household membership count")
        .get(0);
    let relationship_count: i64 = db
        .query_one(
            "SELECT count(*)::bigint FROM carer_relationships WHERE household_id = $1 AND patient_id = $2",
            &[&fixture.household_id, &fixture.managed_person_id],
        )
        .expect("patient relationship count")
        .get(0);

    let rejected = target.post_json_authorized(
        "/api/v1/invitations/accept",
        &fixture.invitation_accept_access_token,
        &json!({"token": fixture.invitation_accept_token, "unexpected": true}),
    );
    assert_error(rejected, 422, "validation_failed");
    assert_eq!(
        db.query_one(
            "SELECT count(*)::bigint FROM household_memberships WHERE household_id = $1 AND account_id = $2",
            &[&fixture.household_id, &fixture.invitation_accept_account_id],
        )
        .expect("unchanged target household membership count")
        .get::<_, i64>(0),
        membership_count
    );
    assert_eq!(
        db.query_one(
            "SELECT count(*)::bigint FROM carer_relationships WHERE household_id = $1 AND patient_id = $2",
            &[&fixture.household_id, &fixture.managed_person_id],
        )
        .expect("unchanged patient relationship count")
        .get::<_, i64>(0),
        relationship_count
    );

    let accepted = target.post_json_authorized(
        "/api/v1/invitations/accept",
        &fixture.invitation_accept_access_token,
        &json!({"token": fixture.invitation_accept_token}),
    );
    assert_eq!(accepted.status().as_u16(), 200);
    let accepted_body = body(accepted);
    let membership_id = accepted_body["data"]["membership_id"]
        .as_str()
        .expect("accepted membership ID")
        .parse::<i64>()
        .expect("numeric membership ID");
    let person_id = accepted_body["data"]["person_id"]
        .as_str()
        .expect("accepted person ID")
        .parse::<i64>()
        .expect("numeric person ID");
    let membership_person_id: i64 = db
        .query_one(
            "SELECT person_id FROM household_memberships WHERE id = $1 AND account_id = $2 AND household_id = $3",
            &[&membership_id, &fixture.invitation_accept_account_id, &fixture.household_id],
        )
        .expect("accepted membership belongs to invitation account")
        .get(0);
    assert_eq!(membership_person_id, person_id);

    let relationship = db
        .query_one(
            "SELECT id FROM carer_relationships WHERE household_id = $1 AND carer_id = $2 AND patient_id = $3 AND relationship_type = 'professional_carer' AND active = true",
            &[&fixture.household_id, &person_id, &fixture.managed_person_id],
        )
        .expect("active professional carer relationship");
    let relationship_id: i64 = relationship.get(0);
    let patient_grant_id: i64 = db
        .query_one(
            "SELECT id FROM person_access_grants WHERE household_id = $1 AND household_membership_id = $2 AND person_id = $3 AND access_level = 'record' AND relationship_type = 'professional' AND carer_relationship_id = $4 AND revoked_at IS NULL",
            &[&fixture.household_id, &membership_id, &fixture.managed_person_id, &relationship_id],
        )
        .expect("relationship-linked patient access grant")
        .get(0);
    let deleted = target.delete(
        &format!(
            "/api/v1/households/{}/admin/person_access_grants/{patient_grant_id}",
            fixture.household_id
        ),
        Some(&fixture.access_token),
    );
    assert_error(deleted, 422, "validation_failed");
    assert!(db
        .query_one(
            "SELECT revoked_at IS NULL FROM person_access_grants WHERE id = $1 AND carer_relationship_id = $2",
            &[&patient_grant_id, &relationship_id],
        )
        .expect("relationship-owned grant remains active")
        .get::<_, bool>(0));
    assert!(
        db.query_one(
            "SELECT active FROM carer_relationships WHERE id = $1",
            &[&relationship_id],
        )
        .expect("relationship remains active")
        .get::<_, bool>(0)
    );
}

#[test]
fn smtp_failure_rolls_back_invitation_rotation_without_receipt_or_audit() {
    let target = Target::from_env();
    let fixture = fixture();
    let (mail_client, mail_origin) = mailpit();
    let email = format!(
        "openapi-mail-failure-{}@example.test",
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    );
    assert!(message_ids_to(&mail_client, &mail_origin, &email).is_empty());
    let list = invitations(&fixture);
    let created = target.post_json_authorized(
        &list,
        &fixture.access_token,
        &json!({"household_invitation": {
            "email": email,
            "membership_role": "member"
        }}),
    );
    assert_eq!(created.status().as_u16(), 201);
    let invitation_id = body(created)["data"]["id"]
        .as_i64()
        .expect("created invitation ID");
    let item = format!("{list}/{invitation_id}/resend");
    let key = format!("openapi-mail-failure-{invitation_id}");
    let failure_base =
        env::var("CONTRACT_MAIL_FAILURE_BASE_URL").expect("isolated SMTP failure API base URL");
    assert_eq!(failure_base, "http://rust-api-mail-fail:39999");
    let failure_url = format!("{failure_base}{item}");
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .expect("SMTP failure API client");
    let mut db = audit_database();
    let state_before = invitation_delivery_state(&mut db, invitation_id);
    let versions_before: i64 = db
        .query_one(
            "SELECT count(*)::bigint FROM versions WHERE item_type = 'HouseholdInvitation' AND item_id = $1",
            &[&invitation_id],
        )
        .expect("invitation version count")
        .get(0);
    let audit_before: i64 = db
        .query_one(
            "SELECT count(*)::bigint FROM security_audit_events WHERE household_id = $1 AND event_type = 'api/admin/invitation/resent' AND metadata->>'target_id' = $2",
            &[&fixture.household_id, &invitation_id.to_string()],
        )
        .expect("invitation resend audit count")
        .get(0);

    for _ in 0..2 {
        let response = client
            .post(&failure_url)
            .bearer_auth(&fixture.access_token)
            .header("Idempotency-Key", &key)
            .json(&json!({}))
            .send()
            .expect("SMTP failure API must respond");
        assert_ne!(
            response
                .headers()
                .get("idempotency-replayed")
                .and_then(|value| value.to_str().ok()),
            Some("true")
        );
        let error = assert_error(response, 503, "invitation_delivery_unavailable");
        assert_no_token_shaped_material(&error);
        assert!(!error.to_string().contains(&email));
        assert!(!error.to_string().contains("delivery_status"));
        assert!(message_ids_to(&mail_client, &mail_origin, &email).is_empty());
        assert_eq!(
            invitation_delivery_state(&mut db, invitation_id),
            state_before
        );
        assert_eq!(
            db.query_one(
                "SELECT count(*)::bigint FROM versions WHERE item_type = 'HouseholdInvitation' AND item_id = $1",
                &[&invitation_id],
            )
            .expect("unchanged invitation version count")
            .get::<_, i64>(0),
            versions_before
        );
        assert_eq!(
            db.query_one(
                "SELECT count(*)::bigint FROM security_audit_events WHERE household_id = $1 AND event_type = 'api/admin/invitation/resent' AND metadata->>'target_id' = $2",
                &[&fixture.household_id, &invitation_id.to_string()],
            )
            .expect("unchanged invitation resend audit count")
            .get::<_, i64>(0),
            audit_before
        );
    }
}
