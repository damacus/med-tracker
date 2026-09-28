use medtracker_contract_tests::{Fixture, Target, fixture};
use reqwest::Method;
use reqwest::blocking::{Client, Response};
use serde_json::{Value, json};
use std::env;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration as StdDuration;

fn database() -> postgres::Client {
    postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("audit database URL"),
        postgres::NoTls,
    )
    .expect("contract audit database")
}

struct MembershipStateGuard {
    db: postgres::Client,
    id: i64,
    role: String,
    status: String,
    person_id: Option<i64>,
    revoked_at: Option<String>,
    permissions_version: i64,
    updated_at: String,
}

impl MembershipStateGuard {
    fn new(id: i64) -> Self {
        let mut db = database();
        let row = db
            .query_one(
                "SELECT role, status, person_id, revoked_at::text, permissions_version::bigint, updated_at::text FROM household_memberships WHERE id = $1",
                &[&id],
            )
            .expect("membership before temporary revoke");
        Self {
            db,
            id,
            role: row.get(0),
            status: row.get(1),
            person_id: row.get(2),
            revoked_at: row.get(3),
            permissions_version: row.get(4),
            updated_at: row.get(5),
        }
    }
}

impl Drop for MembershipStateGuard {
    fn drop(&mut self) {
        self.db.execute(
            "UPDATE household_memberships SET role = $2, status = $3, person_id = $4, revoked_at = $5::text::timestamptz, permissions_version = $6::bigint::integer, updated_at = $7::text::timestamp WHERE id = $1",
            &[
                &self.id,
                &self.role,
                &self.status,
                &self.person_id,
                &self.revoked_at,
                &self.permissions_version,
                &self.updated_at,
            ],
        ).expect("restore membership after temporary test change");
    }
}

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
}

fn path(fixture: &Fixture, suffix: &str) -> String {
    format!("/api/v1/households/{}/admin/{suffix}", fixture.household_id)
}

fn list_path(fixture: &Fixture) -> String {
    path(fixture, "memberships")
}

fn membership(list: &Value, id: i64) -> &Value {
    list["data"]
        .as_array()
        .expect("membership data array")
        .iter()
        .find(|row| row["id"] == id)
        .expect("membership in collection")
}

fn assert_membership(row: &Value) {
    let mut keys: Vec<_> = row
        .as_object()
        .expect("membership object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    let mut expected = [
        "account_id",
        "email",
        "id",
        "joined_at",
        "person_id",
        "person_name",
        "permissions_version",
        "role",
        "status",
    ];
    expected.sort_unstable();
    assert_eq!(keys, expected);
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

fn assert_error(response: Response, status: u16) -> Value {
    assert_eq!(response.status().as_u16(), status);
    let error = body(response);
    assert!(error["error"]["code"].as_str().is_some());
    assert!(error["error"]["request_id"].as_str().is_some());
    assert!(error["error"]["message"].as_str().is_some());
    assert!(error.get("data").is_none());
    error
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

fn membership_is_revoked(id: i64) -> bool {
    database()
        .query_one(
            "SELECT revoked_at IS NOT NULL FROM household_memberships WHERE id = $1",
            &[&id],
        )
        .expect("membership revocation timestamp")
        .get(0)
}

fn rate_limited(response: Response) {
    assert_eq!(response.status().as_u16(), 429);
    for header in [
        "retry-after",
        "ratelimit-limit",
        "ratelimit-remaining",
        "ratelimit-reset",
    ] {
        assert!(response.headers().get(header).is_some(), "missing {header}");
    }
    assert_eq!(response.headers()["ratelimit-limit"], "300");
    assert_eq!(response.headers()["ratelimit-remaining"], "0");
    assert_eq!(body(response)["error"]["code"], "rate_limited");
}

fn rate_request(
    client: &Client,
    base_url: &str,
    method: Method,
    path: &str,
    token: &str,
    payload: Option<&Value>,
) -> Response {
    let mut request = client
        .request(
            method,
            format!("{}{}", base_url.trim_end_matches('/'), path),
        )
        .bearer_auth(token);
    if let Some(payload) = payload {
        request = request.json(payload);
    }
    request.send().expect("rate-limit request")
}

fn idempotency_request(
    method: Method,
    path: &str,
    token: &str,
    key: &str,
    payload: Option<&Value>,
) -> Response {
    let base_url = env::var("CONTRACT_BASE_URL").expect("contract API base URL");
    let client = Client::builder()
        .no_proxy()
        .timeout(StdDuration::from_secs(10))
        .build()
        .expect("idempotency client");
    let mut request = client
        .request(
            method,
            format!("{}{}", base_url.trim_end_matches('/'), path),
        )
        .bearer_auth(token)
        .header("Accept", "application/json")
        .header("Idempotency-Key", key);
    if let Some(payload) = payload {
        request = request.json(payload);
    }
    request.send().expect("membership idempotency request")
}

#[test]
fn list_memberships_returns_the_strict_household_collection_to_owner_and_admin() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = list_path(&fixture);

    assert_error(target.get(&path, None), 401);
    for token in [&fixture.access_token, &fixture.manager_access_token] {
        let response = target.get(&path, Some(token));
        assert_eq!(response.status().as_u16(), 200);
        let list = body(response);
        assert!(list.get("meta").is_none());
        let memberships = list["data"].as_array().expect("membership array");
        assert!(!memberships.is_empty());
        for row in memberships {
            assert_membership(row);
        }
        assert_eq!(
            membership(&list, fixture.owner_membership_id)["role"],
            "owner"
        );
        assert_eq!(
            membership(&list, fixture.manager_membership_id)["role"],
            "administrator"
        );
        assert!(
            !memberships
                .iter()
                .any(|row| row["id"] == fixture.foreign_membership_id)
        );
    }
    assert_error(target.get(&path, Some(&fixture.view_access_token)), 403);
    assert_error(target.get(&path, Some(&fixture.foreign_access_token)), 403);
    assert_error(
        target.get(
            &format!(
                "/api/v1/households/{}/admin/memberships",
                fixture.foreign_household_id
            ),
            Some(&fixture.access_token),
        ),
        403,
    );
}

#[test]
fn patch_and_put_merge_fields_bump_permissions_and_invalidate_target_sessions() {
    let target = Target::from_env();
    let fixture = fixture();
    let list = list_path(&fixture);
    let patch_id = fixture.admin_target_membership_id;
    let _restore_patch = MembershipStateGuard::new(patch_id);
    let patch_item = format!("{list}/{patch_id}");
    let before = body(target.get(&list, Some(&fixture.access_token)));
    let before_patch = membership(&before, patch_id).clone();
    let before_patch_version = before_patch["permissions_version"].as_i64().unwrap();
    assert_eq!(before_patch["role"], "member");
    assert_eq!(before_patch["status"], "active");
    assert_eq!(
        target
            .get(
                &format!("/api/v1/households/{}/me", fixture.household_id),
                Some(&fixture.admin_target_access_token)
            )
            .status()
            .as_u16(),
        200
    );

    let patched = target.patch_json(
        &patch_item,
        &fixture.manager_access_token,
        &json!({"household_membership": {"role": "administrator"}}),
    );
    assert_eq!(patched.status().as_u16(), 200);
    let patch_request_id = request_id(&patched);
    let patched = body(patched)["data"].clone();
    assert_membership(&patched);
    assert_eq!(patched["role"], "administrator");
    assert_eq!(patched["status"], "active");
    assert_eq!(patched["permissions_version"], before_patch_version + 1);
    let patch_event = audit_event(
        &target,
        &fixture,
        &patch_request_id,
        "household_membership.role_updated",
    );
    assert_eq!(patch_event["metadata"]["outcome"], "success");
    assert_error(
        target.get(
            &format!("/api/v1/households/{}/me", fixture.household_id),
            Some(&fixture.admin_target_access_token),
        ),
        401,
    );

    let put_id = fixture.admin_manager_put_membership_id;
    let _restore_put = MembershipStateGuard::new(put_id);
    let put_item = format!("{list}/{put_id}");
    let before = body(target.get(&list, Some(&fixture.access_token)));
    let before_put = membership(&before, put_id).clone();
    let before_put_version = before_put["permissions_version"].as_i64().unwrap();
    assert_eq!(before_put["role"], "member");
    assert_eq!(before_put["status"], "active");
    let replaced = target.put_json(
        &put_item,
        &fixture.access_token,
        &json!({"household_membership": {"status": "suspended"}}),
    );
    assert_eq!(replaced.status().as_u16(), 200);
    let put_request_id = request_id(&replaced);
    let replaced = body(replaced)["data"].clone();
    assert_membership(&replaced);
    assert_eq!(replaced["role"], "member");
    assert_eq!(replaced["status"], "suspended");
    assert_eq!(replaced["permissions_version"], before_put_version + 1);
    let put_event = audit_event(
        &target,
        &fixture,
        &put_request_id,
        "household_access.membership_changed",
    );
    assert_eq!(put_event["metadata"]["outcome"], "success");
    assert_error(
        target.get(
            &format!("/api/v1/households/{}/me", fixture.household_id),
            Some(&fixture.admin_manager_put_access_token),
        ),
        401,
    );

    let no_change_id = fixture.admin_invalid_membership_id;
    let no_change_item = format!("{list}/{no_change_id}");
    let current = body(target.get(&list, Some(&fixture.access_token)));
    let no_change = membership(&current, no_change_id).clone();
    let no_change_version = no_change["permissions_version"].as_i64().unwrap();
    let unchanged = target.patch_json(
        &no_change_item,
        &fixture.manager_access_token,
        &json!({"household_membership": {"status": no_change["status"]}}),
    );
    assert_eq!(unchanged.status().as_u16(), 200);
    let unchanged_request_id = request_id(&unchanged);
    assert_eq!(body(unchanged)["data"], no_change);
    let no_change_event = audit_event(
        &target,
        &fixture,
        &unchanged_request_id,
        "household_access.membership_changed",
    );
    assert_eq!(no_change_event["metadata"]["outcome"], "no_change");
    let after_no_change = body(target.get(&list, Some(&fixture.access_token)));
    assert_eq!(
        membership(&after_no_change, no_change_id)["permissions_version"],
        no_change_version
    );
}

#[test]
fn invalid_foreign_last_owner_and_self_revoke_membership_cases_preserve_boundaries() {
    let target = Target::from_env();
    let fixture = fixture();
    let list = list_path(&fixture);
    let id = fixture.admin_invalid_membership_id;
    let item = format!("{list}/{id}");
    let initial = body(target.get(&list, Some(&fixture.access_token)));
    let original = membership(&initial, id).clone();

    for payload in [
        json!({"household_membership": {"role": "owner"}}),
        json!({"household_membership": {"unknown": true}}),
    ] {
        assert_error(
            target.patch_json(&item, &fixture.manager_access_token, &payload),
            422,
        );
        let current = body(target.get(&list, Some(&fixture.access_token)));
        assert_eq!(membership(&current, id), &original);
    }
    assert_error(
        target.put_json(
            &item,
            &fixture.view_access_token,
            &json!({"household_membership": {"status": "suspended"}}),
        ),
        403,
    );
    let malformed = format!("{list}/not-an-integer");
    assert_error(
        target.patch_json(
            &malformed,
            &fixture.access_token,
            &json!({"household_membership": {"status": "suspended"}}),
        ),
        404,
    );
    assert_error(
        target.put_json(
            &malformed,
            &fixture.access_token,
            &json!({"household_membership": {"status": "suspended"}}),
        ),
        404,
    );
    assert_error(target.delete(&malformed, Some(&fixture.access_token)), 404);
    let foreign = format!("{list}/{}", fixture.foreign_membership_id);
    assert_error(
        target.patch_json(
            &foreign,
            &fixture.access_token,
            &json!({"household_membership": {"status": "suspended"}}),
        ),
        404,
    );
    assert_eq!(
        target
            .delete(&foreign, Some(&fixture.access_token))
            .status()
            .as_u16(),
        404
    );

    let revoke_id = fixture.admin_revoke_membership_id;
    let _restore_revoke = MembershipStateGuard::new(revoke_id);
    let revoke_item = format!("{list}/{revoke_id}");
    let before_revoke = body(target.get(&list, Some(&fixture.access_token)));
    let original_revoke = membership(&before_revoke, revoke_id).clone();
    let revoke_version = original_revoke["permissions_version"].as_i64().unwrap();
    assert_eq!(
        target
            .get(
                &format!("/api/v1/households/{}/me", fixture.household_id),
                Some(&fixture.admin_revoke_access_token)
            )
            .status()
            .as_u16(),
        200
    );
    let revoked = target.delete(&revoke_item, Some(&fixture.manager_access_token));
    assert_eq!(revoked.status().as_u16(), 204);
    let revoke_request_id = request_id(&revoked);
    let revoke_event = audit_event(
        &target,
        &fixture,
        &revoke_request_id,
        "household_access.membership_changed",
    );
    assert_eq!(revoke_event["metadata"]["outcome"], "success");
    let after_revoke = body(target.get(&list, Some(&fixture.access_token)));
    let revoked_membership = membership(&after_revoke, revoke_id);
    assert_eq!(revoked_membership["status"], "revoked");
    assert_eq!(
        revoked_membership["permissions_version"],
        revoke_version + 1
    );
    assert!(membership_is_revoked(revoke_id));
    assert_error(
        target.get(
            &format!("/api/v1/households/{}/me", fixture.household_id),
            Some(&fixture.admin_revoke_access_token),
        ),
        401,
    );
    let medication_path = format!(
        "/api/v1/households/{}/medications/{}",
        fixture.household_id, fixture.managed_medication_id
    );
    assert_error(
        target.patch_json(
            &format!("{medication_path}/adjust_inventory"),
            &fixture.admin_revoke_access_token,
            &json!({"adjustment": {"new_quantity": "12.00", "reason": "counted"}}),
        ),
        401,
    );
    assert_error(
        target.patch_json(
            &format!("{medication_path}/mark_as_ordered"),
            &fixture.admin_revoke_access_token,
            &json!({"order_details": {"supplier": "Contract pharmacy", "quantity": "1.00"}}),
        ),
        401,
    );
    assert_error(
        target.patch_json(
            &format!("{medication_path}/mark_as_received"),
            &fixture.admin_revoke_access_token,
            &json!({}),
        ),
        401,
    );
    let reactivated = target.patch_json(
        &revoke_item,
        &fixture.access_token,
        &json!({"household_membership": {"status": "active"}}),
    );
    assert_eq!(reactivated.status().as_u16(), 200);
    let reactivation_request_id = request_id(&reactivated);
    let reactivated = body(reactivated)["data"].clone();
    assert_membership(&reactivated);
    assert_eq!(reactivated["status"], "active");
    assert_eq!(reactivated["role"], original_revoke["role"]);
    assert_eq!(reactivated["permissions_version"], revoke_version + 2);
    assert!(!membership_is_revoked(revoke_id));
    assert_eq!(
        audit_event(
            &target,
            &fixture,
            &reactivation_request_id,
            "household_access.membership_changed",
        )["metadata"]["outcome"],
        "success"
    );
    assert_error(
        target.get(
            &format!("/api/v1/households/{}/me", fixture.household_id),
            Some(&fixture.admin_revoke_access_token),
        ),
        401,
    );

    let last_list = format!(
        "/api/v1/households/{}/admin/memberships",
        fixture.last_owner_household_id
    );
    let last_item = format!("{last_list}/{}", fixture.last_owner_membership_id);
    let last_before = body(target.get(&last_list, Some(&fixture.last_owner_access_token)));
    let last_owner = membership(&last_before, fixture.last_owner_membership_id).clone();
    assert_error(
        target.delete(&last_item, Some(&fixture.last_owner_access_token)),
        422,
    );
    let last_after = body(target.get(&last_list, Some(&fixture.last_owner_access_token)));
    assert_eq!(
        membership(&last_after, fixture.last_owner_membership_id),
        &last_owner
    );

    let manager_item = format!("{list}/{}", fixture.manager_membership_id);
    let _restore_manager = MembershipStateGuard::new(fixture.manager_membership_id);
    assert_eq!(
        target
            .get(
                &format!("/api/v1/households/{}/me", fixture.household_id),
                Some(&fixture.manager_access_token)
            )
            .status()
            .as_u16(),
        200
    );
    let manager_revocation = target.delete(&manager_item, Some(&fixture.manager_access_token));
    assert_eq!(manager_revocation.status().as_u16(), 204);
    let manager_request_id = request_id(&manager_revocation);
    let manager_event = audit_event(
        &target,
        &fixture,
        &manager_request_id,
        "household_access.membership_changed",
    );
    assert_eq!(
        manager_event["metadata"]["target_membership_id"],
        fixture.manager_membership_id
    );
    assert_eq!(manager_event["metadata"]["outcome"], "success");
    assert_error(
        target.get(
            &format!("/api/v1/households/{}/me", fixture.household_id),
            Some(&fixture.manager_access_token),
        ),
        401,
    );
    assert_error(
        target.get(
            &path(&fixture, "app_tokens"),
            Some(&fixture.manager_app_token),
        ),
        401,
    );
}

#[test]
fn concurrent_self_revocation_of_two_owners_preserves_one_active_owner() {
    let fixture = fixture();
    let mut db = database();
    let active_owner_ids: Vec<i64> = db
        .query(
            "SELECT id FROM household_memberships WHERE household_id = $1 AND role = 'owner' AND status = 'active'",
            &[&fixture.household_id],
        )
        .expect("active owner fixture rows")
        .iter()
        .map(|row| row.get(0))
        .collect();
    assert!(active_owner_ids.contains(&fixture.owner_membership_id));
    let _restore_memberships: Vec<_> = active_owner_ids
        .iter()
        .copied()
        .chain([fixture.manager_membership_id])
        .map(MembershipStateGuard::new)
        .collect();
    db.execute(
        "UPDATE household_memberships SET role = 'member' WHERE household_id = $1 AND role = 'owner' AND status = 'active' AND id <> $2",
        &[&fixture.household_id, &fixture.owner_membership_id],
    )
    .expect("reduce fixture to one existing owner");
    db.execute(
        "UPDATE household_memberships SET role = 'owner' WHERE id = $1",
        &[&fixture.manager_membership_id],
    )
    .expect("promote fixture manager for owner race");
    let owners_before: i64 = db
        .query_one(
            "SELECT count(*) FROM household_memberships WHERE household_id = $1 AND role = 'owner' AND status = 'active'",
            &[&fixture.household_id],
        )
        .expect("active owner count before race")
        .get(0);
    assert_eq!(owners_before, 2);

    let membership_path = list_path(&fixture);
    let owner_item = format!("{membership_path}/{}", fixture.owner_membership_id);
    let manager_item = format!("{membership_path}/{}", fixture.manager_membership_id);
    let base_url = env::var("CONTRACT_BASE_URL").expect("API base URL");
    let barrier = Arc::new(Barrier::new(2));
    let mut workers = Vec::new();
    for (path, token) in [
        (owner_item, fixture.access_token.clone()),
        (manager_item, fixture.manager_access_token.clone()),
    ] {
        let barrier = Arc::clone(&barrier);
        let base_url = base_url.clone();
        workers.push(thread::spawn(move || {
            let client = Client::builder().build().expect("membership client");
            barrier.wait();
            client
                .delete(format!("{}{}", base_url.trim_end_matches('/'), path))
                .bearer_auth(token)
                .send()
                .expect("concurrent last-owner response")
        }));
    }
    let mut statuses: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().expect("owner race worker").status().as_u16())
        .collect();
    statuses.sort_unstable();
    assert_eq!(statuses, [204, 422]);
    let owners_after: i64 = db
        .query_one(
            "SELECT count(*) FROM household_memberships WHERE household_id = $1 AND role = 'owner' AND status = 'active'",
            &[&fixture.household_id],
        )
        .expect("active owner count after race")
        .get(0);
    assert_eq!(owners_after, 1);
}

#[test]
fn membership_write_idempotency_replays_each_method_and_rejects_key_reuse() {
    let target = Target::from_env();
    let fixture = fixture();
    let membership_id = fixture.admin_invalid_membership_id;
    let _restore_membership = MembershipStateGuard::new(membership_id);
    let list = list_path(&fixture);
    let item = format!("{list}/{membership_id}");
    let before = target.get(&list, Some(&fixture.access_token));
    assert_eq!(before.status().as_u16(), 200);
    let original = membership(&body(before), membership_id).clone();
    let version = original["permissions_version"].as_i64().unwrap();
    assert_eq!(original["status"], "active");

    assert_error(
        target.patch_json(&item, &fixture.access_token, &json!({})),
        400,
    );
    assert_error(
        target.put_json(&item, &fixture.access_token, &json!({})),
        400,
    );
    assert_eq!(
        membership(
            &body(target.get(&list, Some(&fixture.access_token))),
            membership_id
        ),
        &original
    );

    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after Unix epoch")
        .as_nanos();
    let patch_key = format!("membership-patch-{}-{unique}", fixture.household_id);
    let patch_payload = json!({"household_membership": {"status": "suspended"}});
    let patched = idempotency_request(
        Method::PATCH,
        &item,
        &fixture.access_token,
        &patch_key,
        Some(&patch_payload),
    );
    assert_eq!(patched.status().as_u16(), 200);
    let patched_body = body(patched);
    assert_eq!(patched_body["data"]["status"], "suspended");
    assert_eq!(patched_body["data"]["permissions_version"], version + 1);
    let patch_replay = idempotency_request(
        Method::PATCH,
        &item,
        &fixture.access_token,
        &patch_key,
        Some(&patch_payload),
    );
    assert_eq!(patch_replay.status().as_u16(), 200);
    assert_eq!(patch_replay.headers()["idempotency-replayed"], "true");
    assert_eq!(body(patch_replay), patched_body);
    assert_error(
        idempotency_request(
            Method::PATCH,
            &item,
            &fixture.access_token,
            &patch_key,
            Some(&json!({"household_membership": {"status": "active"}})),
        ),
        409,
    );
    assert_error(
        idempotency_request(
            Method::PATCH,
            &item,
            &fixture.manager_access_token,
            &patch_key,
            Some(&patch_payload),
        ),
        409,
    );
    assert_eq!(
        membership(
            &body(target.get(&list, Some(&fixture.access_token))),
            membership_id
        )["permissions_version"],
        version + 1
    );

    let put_key = format!("membership-put-{}-{unique}", fixture.household_id);
    let put_payload = json!({"household_membership": {"status": "active"}});
    let replaced = idempotency_request(
        Method::PUT,
        &item,
        &fixture.access_token,
        &put_key,
        Some(&put_payload),
    );
    assert_eq!(replaced.status().as_u16(), 200);
    let replaced_body = body(replaced);
    assert_eq!(replaced_body["data"]["status"], "active");
    assert_eq!(replaced_body["data"]["permissions_version"], version + 2);
    let put_replay = idempotency_request(
        Method::PUT,
        &item,
        &fixture.access_token,
        &put_key,
        Some(&put_payload),
    );
    assert_eq!(put_replay.status().as_u16(), 200);
    assert_eq!(put_replay.headers()["idempotency-replayed"], "true");
    assert_eq!(body(put_replay), replaced_body);
    assert_error(
        idempotency_request(
            Method::PUT,
            &item,
            &fixture.access_token,
            &put_key,
            Some(&json!({"household_membership": {"status": "suspended"}})),
        ),
        409,
    );
    assert_eq!(
        membership(
            &body(target.get(&list, Some(&fixture.access_token))),
            membership_id
        )["permissions_version"],
        version + 2
    );

    let delete_key = format!("membership-delete-{}-{unique}", fixture.household_id);
    let deleted = idempotency_request(
        Method::DELETE,
        &item,
        &fixture.access_token,
        &delete_key,
        None,
    );
    assert_eq!(deleted.status().as_u16(), 204);
    let delete_replay = idempotency_request(
        Method::DELETE,
        &item,
        &fixture.access_token,
        &delete_key,
        None,
    );
    assert_eq!(delete_replay.status().as_u16(), 204);
    assert_eq!(delete_replay.headers()["idempotency-replayed"], "true");
    let after_delete = body(target.get(&list, Some(&fixture.access_token)));
    let deleted_membership = membership(&after_delete, membership_id);
    assert_eq!(deleted_membership["status"], "revoked");
    assert_eq!(deleted_membership["permissions_version"], version + 3);
}

#[test]
fn all_four_membership_operations_use_the_shared_rate_limiter() {
    let fixture = fixture();
    let base_url = env::var("CONTRACT_RATE_BASE_URL").expect("non-loopback API base URL");
    let client = Client::builder()
        .no_proxy()
        .timeout(StdDuration::from_secs(5))
        .build()
        .expect("rate-limit client");
    let mut limited = None;
    for _ in 0..601 {
        let response = rate_request(
            &client,
            &base_url,
            Method::GET,
            "/api/v1/capabilities",
            &fixture.access_token,
            None,
        );
        if response.status().as_u16() == 429 {
            limited = Some(response);
            break;
        }
        assert_eq!(response.status().as_u16(), 200);
    }
    rate_limited(limited.expect("shared rate limit response"));

    let list = list_path(&fixture);
    let item = format!("{list}/{}", fixture.admin_invalid_membership_id);
    let payload = json!({"household_membership": {"status": "active"}});
    rate_limited(rate_request(
        &client,
        &base_url,
        Method::GET,
        &list,
        &fixture.access_token,
        None,
    ));
    rate_limited(rate_request(
        &client,
        &base_url,
        Method::PATCH,
        &item,
        &fixture.access_token,
        Some(&payload),
    ));
    rate_limited(rate_request(
        &client,
        &base_url,
        Method::PUT,
        &item,
        &fixture.access_token,
        Some(&payload),
    ));
    rate_limited(rate_request(
        &client,
        &base_url,
        Method::DELETE,
        &item,
        &fixture.access_token,
        None,
    ));
}
