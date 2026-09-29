use medtracker_contract_tests::{Fixture, Target, fixture};
use reqwest::blocking::{Client, Response};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const GRANT_FIELDS: [&str; 8] = [
    "id",
    "household_membership_id",
    "person_id",
    "person_name",
    "access_level",
    "relationship_type",
    "expires_at",
    "revoked_at",
];

fn body(response: Response) -> Value {
    response.json().expect("JSON response")
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
    let value: Value = response.json().expect("rate limit JSON");
    assert_eq!(value["error"]["code"], "rate_limited");
}

fn grants_path(fixture: &Fixture) -> String {
    format!(
        "/api/v1/households/{}/admin/person_access_grants",
        fixture.household_id
    )
}

fn audit_database() -> postgres::Client {
    postgres::Client::connect(
        &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
        postgres::NoTls,
    )
    .expect("contract database")
}

fn membership_version(db: &mut postgres::Client, membership_id: i64) -> i64 {
    db.query_one(
        "SELECT permissions_version::bigint FROM household_memberships WHERE id = $1",
        &[&membership_id],
    )
    .expect("membership permissions version")
    .get(0)
}

struct MembershipVersionGuard {
    db: postgres::Client,
    id: i64,
    version: i64,
}

impl MembershipVersionGuard {
    fn new(id: i64) -> Self {
        let mut db = audit_database();
        let version = membership_version(&mut db, id);
        Self { db, id, version }
    }
}

impl Drop for MembershipVersionGuard {
    fn drop(&mut self) {
        self.db
            .execute(
                "UPDATE household_memberships SET permissions_version = $2::bigint::integer WHERE id = $1",
                &[&self.id, &self.version],
            )
            .expect("restore grant target permissions version");
    }
}

fn grant_audit_outcomes(
    db: &mut postgres::Client,
    household_id: i64,
    membership_id: i64,
) -> HashMap<String, i64> {
    db.query(
        "SELECT metadata->>'outcome', count(*)::bigint FROM security_audit_events WHERE household_id = $1 AND event_type = 'household_access.person_grant_changed' AND metadata->>'target_membership_id' = $2 GROUP BY metadata->>'outcome'",
        &[&household_id, &membership_id.to_string()],
    )
    .expect("grant audit outcomes")
    .into_iter()
    .map(|row| (row.get(0), row.get(1)))
    .collect()
}

fn outcome_count(outcomes: &HashMap<String, i64>, outcome: &str) -> i64 {
    outcomes.get(outcome).copied().unwrap_or_default()
}

fn grant_audit_new_state(db: &mut postgres::Client, household_id: i64, request_id: &str) -> Value {
    let serialized: String = db
        .query_one(
            "SELECT (metadata->'new_state')::text FROM security_audit_events WHERE household_id = $1 AND event_type = 'household_access.person_grant_changed' AND request_id = $2",
            &[&household_id, &request_id],
        )
        .expect("rejected grant audit event")
        .get(0);
    serde_json::from_str(&serialized).expect("structured grant audit state")
}

struct RelationshipOwnedGrant {
    db: postgres::Client,
    grant_id: i64,
    relationship_id: i64,
}

impl RelationshipOwnedGrant {
    fn new(fixture: &Fixture) -> Self {
        let mut db = postgres::Client::connect(
            &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("contract database URL"),
            postgres::NoTls,
        )
        .expect("contract database");
        let relationship_id: i64 = db
            .query_one(
                "INSERT INTO carer_relationships (household_id, carer_id, patient_id, relationship_type, active, created_at, updated_at) VALUES ($1, $2, $3, 'parent', true, now(), now()) RETURNING id",
                &[&fixture.household_id, &fixture.managed_person_id, &fixture.hidden_person_id],
            )
            .expect("disposable carer relationship")
            .get(0);
        let grant_id: i64 = db
            .query_one(
                "INSERT INTO person_access_grants (household_id, household_membership_id, person_id, access_level, relationship_type, carer_relationship_id, granted_by_membership_id, created_at, updated_at) VALUES ($1, $2, $3, 'manage', 'parent', $4, $5, now(), now()) RETURNING id",
                &[&fixture.household_id, &fixture.owner_membership_id, &fixture.hidden_person_id, &relationship_id, &fixture.owner_membership_id],
            )
            .expect("disposable relationship-owned grant")
            .get(0);
        Self {
            db,
            grant_id,
            relationship_id,
        }
    }
}

impl Drop for RelationshipOwnedGrant {
    fn drop(&mut self) {
        self.db
            .execute(
                "DELETE FROM person_access_grants WHERE id = $1",
                &[&self.grant_id],
            )
            .expect("remove disposable grant");
        self.db
            .execute(
                "DELETE FROM carer_relationships WHERE id = $1",
                &[&self.relationship_id],
            )
            .expect("remove disposable relationship");
    }
}

struct ForeignGrant {
    db: postgres::Client,
    id: i64,
}

impl ForeignGrant {
    fn new(fixture: &Fixture) -> Self {
        let mut db = audit_database();
        let id: i64 = db
            .query_one(
                "SELECT id FROM person_access_grants WHERE household_id = $1 AND household_membership_id = $2 AND person_id = $3 AND revoked_at IS NULL ORDER BY id LIMIT 1",
                &[&fixture.foreign_household_id, &fixture.foreign_membership_id, &fixture.foreign_person_id],
            )
            .expect("existing foreign grant")
            .get(0);
        Self { db, id }
    }
}

#[test]
fn list_create_and_revoke_person_grant() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = grants_path(&fixture);

    let list = target.get(&path, Some(&fixture.access_token));
    assert_eq!(list.status().as_u16(), 200);
    assert!(body(list)["data"].is_array());

    let payload = json!({"person_access_grant": {
        "household_membership_id": fixture.grant_target_membership_id,
        "person_id": fixture.managed_person_id,
        "access_level": "view",
        "relationship_type": "family_member"
    }});
    let created = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(created.status().as_u16(), 201);
    let grant = body(created)["data"].clone();
    assert_eq!(grant.as_object().unwrap().len(), GRANT_FIELDS.len());
    for field in GRANT_FIELDS {
        assert!(grant.get(field).is_some(), "missing {field}");
    }
    assert_eq!(
        grant["household_membership_id"],
        fixture.grant_target_membership_id
    );
    assert_eq!(grant["person_id"], fixture.managed_person_id);
    assert!(
        grant["person_name"]
            .as_str()
            .is_some_and(|name| !name.is_empty())
    );
    assert_eq!(grant["access_level"], "view");
    assert_eq!(grant["relationship_type"], "family_member");
    assert!(grant["expires_at"].is_null());
    assert!(grant["revoked_at"].is_null());
    let grant_id = grant["id"].as_i64().expect("numeric grant ID");

    let listed = body(target.get(&path, Some(&fixture.access_token)));
    assert!(
        listed["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["id"] == grant_id)
    );

    let deleted = target.delete(&format!("{path}/{grant_id}"), Some(&fixture.access_token));
    assert_eq!(deleted.status().as_u16(), 204);
    let after_revoke = body(target.get(&path, Some(&fixture.access_token)));
    let revoked = after_revoke["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == grant_id)
        .expect("revoked grant remains in administrative list");
    assert!(revoked["revoked_at"].is_string());
}

#[test]
fn grant_collection_has_strict_shape_and_manager_access_boundaries() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = grants_path(&fixture);

    for token in [&fixture.access_token, &fixture.manager_access_token] {
        let response = target.get(&path, Some(token));
        assert_eq!(response.status().as_u16(), 200);
        let value = body(response);
        assert_eq!(value.as_object().unwrap().len(), 1);
        for grant in value["data"].as_array().unwrap() {
            assert_eq!(grant.as_object().unwrap().len(), GRANT_FIELDS.len());
            for field in GRANT_FIELDS {
                assert!(grant.get(field).is_some(), "missing {field}");
            }
            assert!(grant["id"].as_i64().is_some());
            assert!(grant["household_membership_id"].as_i64().is_some());
            assert!(grant["person_id"].as_i64().is_some());
            assert!(grant["person_name"].as_str().is_some());
            assert!(matches!(
                grant["access_level"].as_str(),
                Some("view" | "record" | "manage")
            ));
            assert!(matches!(
                grant["relationship_type"].as_str(),
                Some("self" | "parent" | "family_member" | "carer" | "professional")
            ));
            assert!(grant["expires_at"].is_string() || grant["expires_at"].is_null());
            assert!(grant["revoked_at"].is_string() || grant["revoked_at"].is_null());
        }
    }

    for token in [None, Some(fixture.view_access_token.as_str())] {
        let response = target.get(&path, token);
        assert_eq!(
            response.status().as_u16(),
            if token.is_none() { 401 } else { 403 }
        );
    }
    let response = target.get(
        &format!(
            "/api/v1/households/{}/admin/person_access_grants",
            fixture.foreign_household_id
        ),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 403);
}

#[test]
fn grant_validation_and_duplicate_rejection_preserve_existing_grants() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = grants_path(&fixture);
    let payload = json!({"person_access_grant": {
        "household_membership_id": fixture.grant_target_membership_id,
        "person_id": fixture.managed_person_id,
        "access_level": "record",
        "relationship_type": "professional",
        "expires_at": "2030-01-01T00:00:00Z"
    }});
    let response = target.post_json_authorized(&path, &fixture.manager_access_token, &payload);
    assert_eq!(response.status().as_u16(), 201);
    let grant = body(response)["data"].clone();
    let grant_id = grant["id"].as_i64().expect("grant id");
    assert_eq!(grant["access_level"], "record");
    assert!(
        grant["expires_at"]
            .as_str()
            .unwrap()
            .starts_with("2030-01-01T00:00:00")
    );

    let unauthorized = target.post_json(&path, &payload);
    assert_eq!(unauthorized.status().as_u16(), 401);
    let member_denied = target.post_json_authorized(&path, &fixture.view_access_token, &payload);
    assert_eq!(member_denied.status().as_u16(), 403);

    let duplicate = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(duplicate.status().as_u16(), 422);
    assert_eq!(body(duplicate)["error"]["code"], "validation_failed");

    let mut invalid_inputs = vec![
        json!({"person_access_grant": {"household_membership_id": fixture.grant_target_membership_id, "person_id": fixture.managed_person_id, "access_level": "owner", "relationship_type": "professional"}}),
        json!({"person_access_grant": {"household_membership_id": fixture.grant_target_membership_id, "person_id": fixture.managed_person_id, "access_level": "view", "relationship_type": "friend"}}),
        json!({"person_access_grant": {"household_membership_id": fixture.grant_target_membership_id, "person_id": fixture.managed_person_id, "access_level": "view", "relationship_type": "professional", "expires_at": "not-a-timestamp"}}),
        json!({"person_access_grant": {"household_membership_id": fixture.grant_target_membership_id, "person_id": fixture.foreign_person_id, "access_level": "view", "relationship_type": "professional"}}),
    ];
    invalid_inputs.push(json!({"person_access_grant": {
        "household_membership_id": fixture.grant_target_membership_id,
        "person_id": fixture.managed_person_id,
        "access_level": "view",
        "relationship_type": "professional",
        "unexpected": true
    }}));
    for invalid in invalid_inputs {
        let response = target.post_json_authorized(&path, &fixture.access_token, &invalid);
        assert_eq!(response.status().as_u16(), 422);
    }

    let listed = body(target.get(&path, Some(&fixture.access_token)));
    let matching: Vec<&Value> = listed["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["id"] == grant_id)
        .collect();
    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0]["access_level"], "record");
    assert!(matching[0]["revoked_at"].is_null());
    let deleted = target.delete(&format!("{path}/{grant_id}"), Some(&fixture.access_token));
    assert_eq!(deleted.status().as_u16(), 204);
    let unauthorized_delete = target.delete(&format!("{path}/{grant_id}"), None);
    assert_eq!(unauthorized_delete.status().as_u16(), 401);
}

#[test]
fn grant_writes_replay_by_key_and_recheck_the_current_account() {
    let target = Target::from_env();
    let fixture = fixture();
    let _restore_version = MembershipVersionGuard::new(fixture.admin_target_membership_id);
    let path = grants_path(&fixture);
    let payload = json!({"person_access_grant": {
        "household_membership_id": fixture.admin_target_membership_id,
        "person_id": fixture.managed_person_id,
        "access_level": "view",
        "relationship_type": "family_member"
    }});
    let mut db = audit_database();
    let version_before = membership_version(&mut db, fixture.admin_target_membership_id);
    let audits_before = grant_audit_outcomes(
        &mut db,
        fixture.household_id,
        fixture.admin_target_membership_id,
    );
    let key = "person-grant-contract-replay";
    let first = target.post_json_with_key(&path, &fixture.access_token, key, &payload);
    assert_eq!(first.status().as_u16(), 201);
    assert!(first.headers().get("idempotency-replayed").is_none());
    let first_body: Value = first.json().expect("created grant JSON");
    let grant_id = first_body["data"]["id"].as_i64().unwrap();
    assert_eq!(
        membership_version(&mut db, fixture.admin_target_membership_id),
        version_before + 1
    );
    let after_first = grant_audit_outcomes(
        &mut db,
        fixture.household_id,
        fixture.admin_target_membership_id,
    );
    assert_eq!(
        outcome_count(&after_first, "success"),
        outcome_count(&audits_before, "success") + 1
    );

    let replay = target.post_json_with_key(&path, &fixture.access_token, key, &payload);
    assert_eq!(replay.status().as_u16(), 201);
    assert_eq!(replay.headers()["idempotency-replayed"], "true");
    assert_eq!(body(replay), first_body);
    assert_eq!(
        membership_version(&mut db, fixture.admin_target_membership_id),
        version_before + 1
    );
    assert_eq!(
        grant_audit_outcomes(
            &mut db,
            fixture.household_id,
            fixture.admin_target_membership_id,
        ),
        after_first
    );

    let changed = json!({"person_access_grant": {
        "household_membership_id": fixture.admin_target_membership_id,
        "person_id": fixture.managed_person_id,
        "access_level": "manage",
        "relationship_type": "family_member"
    }});
    let conflict = target.post_json_with_key(&path, &fixture.access_token, key, &changed);
    assert_eq!(conflict.status().as_u16(), 409);
    let conflict_request_id = conflict.headers()["x-request-id"]
        .to_str()
        .expect("conflict request ID")
        .to_owned();
    let conflict_body: Value = conflict.json().expect("conflict JSON");
    assert_eq!(conflict_body["error"]["request_id"], conflict_request_id);

    let other_account =
        target.post_json_with_key(&path, &fixture.manager_access_token, key, &payload);
    assert_eq!(other_account.status().as_u16(), 409);
    assert_eq!(
        membership_version(&mut db, fixture.admin_target_membership_id),
        version_before + 1
    );
    assert_eq!(
        grant_audit_outcomes(
            &mut db,
            fixture.household_id,
            fixture.admin_target_membership_id,
        ),
        after_first
    );

    let deleted = target.delete(&format!("{path}/{grant_id}"), Some(&fixture.access_token));
    assert_eq!(deleted.status().as_u16(), 204);
    assert_eq!(
        membership_version(&mut db, fixture.admin_target_membership_id),
        version_before + 2
    );
    let after_revoke = grant_audit_outcomes(
        &mut db,
        fixture.household_id,
        fixture.admin_target_membership_id,
    );
    assert_eq!(
        outcome_count(&after_revoke, "success"),
        outcome_count(&after_first, "success") + 1
    );
    let revoked_at: String = db
        .query_one(
            "SELECT revoked_at::text FROM person_access_grants WHERE id = $1",
            &[&grant_id],
        )
        .expect("revoked grant timestamp")
        .get(0);
    let repeated = target.delete(&format!("{path}/{grant_id}"), Some(&fixture.access_token));
    assert_eq!(repeated.status().as_u16(), 204);
    assert_eq!(
        membership_version(&mut db, fixture.admin_target_membership_id),
        version_before + 2
    );
    let revoked_at_after: String = db
        .query_one(
            "SELECT revoked_at::text FROM person_access_grants WHERE id = $1",
            &[&grant_id],
        )
        .expect("unchanged grant timestamp")
        .get(0);
    assert_eq!(revoked_at_after, revoked_at);
    let after_repeat = grant_audit_outcomes(
        &mut db,
        fixture.household_id,
        fixture.admin_target_membership_id,
    );
    assert_eq!(
        outcome_count(&after_repeat, "success"),
        outcome_count(&after_revoke, "success")
    );
    assert_eq!(
        outcome_count(&after_repeat, "no_change"),
        outcome_count(&after_revoke, "no_change") + 1
    );
}

#[test]
fn grant_changes_invalidate_the_granted_members_current_credential() {
    let target = Target::from_env();
    let fixture = fixture();
    let _restore_version = MembershipVersionGuard::new(fixture.admin_target_membership_id);
    let path = grants_path(&fixture);
    let profile = format!("/api/v1/households/{}/me", fixture.household_id);
    let before = target.get(&profile, Some(&fixture.admin_target_access_token));
    assert_eq!(before.status().as_u16(), 200);

    let payload = json!({"person_access_grant": {
        "household_membership_id": fixture.admin_target_membership_id,
        "person_id": fixture.managed_person_id,
        "access_level": "view",
        "relationship_type": "family_member"
    }});
    let created = target.post_json_authorized(&path, &fixture.access_token, &payload);
    assert_eq!(created.status().as_u16(), 201);
    let grant_id = body(created)["data"]["id"].as_i64().unwrap();
    let stale = target.get(&profile, Some(&fixture.admin_target_access_token));
    assert_eq!(stale.status().as_u16(), 401);

    let deleted = target.delete(&format!("{path}/{grant_id}"), Some(&fixture.access_token));
    assert_eq!(deleted.status().as_u16(), 204);
    let still_stale = target.get(&profile, Some(&fixture.admin_target_access_token));
    assert_eq!(still_stale.status().as_u16(), 401);
}

#[test]
fn relationship_owned_grants_cannot_be_revoked_directly() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = grants_path(&fixture);
    let mut grant = RelationshipOwnedGrant::new(&fixture);
    let version_before: i64 = grant
        .db
        .query_one(
            "SELECT permissions_version::bigint FROM household_memberships WHERE id = $1",
            &[&fixture.owner_membership_id],
        )
        .unwrap()
        .get(0);

    let response = target.delete(
        &format!("{path}/{}", grant.grant_id),
        Some(&fixture.access_token),
    );
    assert_eq!(response.status().as_u16(), 422);
    assert_eq!(body(response)["error"]["code"], "validation_failed");
    let grant_state = grant
        .db
        .query_one(
            "SELECT revoked_at::text FROM person_access_grants WHERE id = $1",
            &[&grant.grant_id],
        )
        .unwrap();
    let revoked_at: Option<String> = grant_state.get(0);
    assert!(revoked_at.is_none());
    let version_after: i64 = grant
        .db
        .query_one(
            "SELECT permissions_version::bigint FROM household_memberships WHERE id = $1",
            &[&fixture.owner_membership_id],
        )
        .unwrap()
        .get(0);
    assert_eq!(version_after, version_before);
}

#[test]
fn grant_boundaries_accept_expired_rows_and_reject_unauthorized_or_unaddressable_deletes() {
    let target = Target::from_env();
    let fixture = fixture();
    let path = grants_path(&fixture);
    let mut db = audit_database();
    let version_before = membership_version(&mut db, fixture.grant_target_membership_id);
    let expired_payload = json!({"person_access_grant": {
        "household_membership_id": fixture.grant_target_membership_id,
        "person_id": fixture.hidden_person_id,
        "access_level": "view",
        "relationship_type": "family_member",
        "expires_at": "2000-01-01T00:00:00Z"
    }});
    let expired = target.post_json_authorized(&path, &fixture.access_token, &expired_payload);
    assert_eq!(expired.status().as_u16(), 201);
    let grant = body(expired)["data"].clone();
    let grant_id = grant["id"].as_i64().expect("expired grant ID");
    assert!(grant["revoked_at"].is_null());
    let expiry = OffsetDateTime::parse(
        grant["expires_at"].as_str().expect("past expiry timestamp"),
        &Rfc3339,
    )
    .expect("RFC3339 past expiry");
    assert!(expiry < OffsetDateTime::now_utc());
    let listed = body(target.get(&path, Some(&fixture.access_token)));
    let listed_expired = listed["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == grant_id)
        .expect("expired grant remains listed");
    assert_eq!(listed_expired["expires_at"], grant["expires_at"]);
    assert!(listed_expired["revoked_at"].is_null());
    let inactive: bool = db
        .query_one(
            "SELECT expires_at < now() AND revoked_at IS NULL FROM person_access_grants WHERE id = $1",
            &[&grant_id],
        )
        .expect("expired unrevoked grant state")
        .get(0);
    assert!(inactive);
    assert_eq!(
        membership_version(&mut db, fixture.grant_target_membership_id),
        version_before + 1
    );

    let missing_wrapper = target.post_json_authorized(&path, &fixture.access_token, &json!({}));
    assert_eq!(missing_wrapper.status().as_u16(), 400);
    let after_missing_wrapper: bool = db
        .query_one(
            "SELECT revoked_at IS NULL FROM person_access_grants WHERE id = $1",
            &[&grant_id],
        )
        .expect("grant unchanged after malformed request")
        .get(0);
    assert!(after_missing_wrapper);
    assert_eq!(
        membership_version(&mut db, fixture.grant_target_membership_id),
        version_before + 1
    );

    let denied_delete = target.delete(
        &format!("{path}/{grant_id}"),
        Some(&fixture.view_access_token),
    );
    assert_eq!(denied_delete.status().as_u16(), 403);
    assert!(
        db.query_one(
            "SELECT revoked_at IS NULL FROM person_access_grants WHERE id = $1",
            &[&grant_id],
        )
        .expect("grant unchanged after denied delete")
        .get::<_, bool>(0)
    );

    let mut foreign = ForeignGrant::new(&fixture);
    let hidden_foreign = target.delete(
        &format!("{path}/{}", foreign.id),
        Some(&fixture.access_token),
    );
    assert_eq!(hidden_foreign.status().as_u16(), 404);
    assert!(
        foreign
            .db
            .query_one(
                "SELECT revoked_at IS NULL FROM person_access_grants WHERE id = $1",
                &[&foreign.id],
            )
            .expect("foreign grant remains unchanged")
            .get::<_, bool>(0)
    );
    for id in ["not-a-number", "999999999"] {
        let missing = target.delete(&format!("{path}/{id}"), Some(&fixture.access_token));
        assert_eq!(missing.status().as_u16(), 404);
    }
    assert_eq!(
        membership_version(&mut db, fixture.grant_target_membership_id),
        version_before + 1
    );

    let revoked = target.delete(&format!("{path}/{grant_id}"), Some(&fixture.access_token));
    assert_eq!(revoked.status().as_u16(), 204);
    assert_eq!(
        membership_version(&mut db, fixture.grant_target_membership_id),
        version_before + 2
    );
}

#[test]
fn concurrent_duplicate_grant_creates_apply_one_access_change() {
    let fixture = fixture();
    let path = grants_path(&fixture);
    let payload = json!({"person_access_grant": {
        "household_membership_id": fixture.grant_target_membership_id,
        "person_id": fixture.hidden_person_id,
        "access_level": "view",
        "relationship_type": "family_member"
    }});
    let mut db = audit_database();
    let version_before = membership_version(&mut db, fixture.grant_target_membership_id);
    let audits_before = grant_audit_outcomes(
        &mut db,
        fixture.household_id,
        fixture.grant_target_membership_id,
    );
    let barrier = Arc::new(Barrier::new(3));
    let mut workers = Vec::new();
    for _ in 0..2 {
        let barrier = Arc::clone(&barrier);
        let path = path.clone();
        let token = fixture.access_token.clone();
        let payload = payload.clone();
        workers.push(thread::spawn(move || {
            let target = Target::from_env();
            barrier.wait();
            let response = target.post_json_authorized(&path, &token, &payload);
            (response.status().as_u16(), body(response))
        }));
    }
    barrier.wait();
    let mut outcomes: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().expect("grant create worker"))
        .collect();
    outcomes.sort_by_key(|(status, _)| *status);
    assert_eq!(
        outcomes
            .iter()
            .map(|(status, _)| *status)
            .collect::<Vec<_>>(),
        [201, 422]
    );
    let created = outcomes.iter().find(|(status, _)| *status == 201).unwrap();
    let grant_id = created.1["data"]["id"].as_i64().unwrap();
    let rejected = outcomes.iter().find(|(status, _)| *status == 422).unwrap();
    let rejected_request_id = rejected.1["error"]["request_id"]
        .as_str()
        .expect("rejected create request ID");
    let attempted = grant_audit_new_state(&mut db, fixture.household_id, rejected_request_id);
    assert_eq!(
        attempted.as_object().unwrap().len(),
        7,
        "audit must contain only parsed grant state"
    );
    assert_eq!(
        attempted["household_membership_id"],
        fixture.grant_target_membership_id
    );
    assert_eq!(attempted["person_id"], fixture.hidden_person_id);
    assert_eq!(attempted["access_level"], "view");
    assert_eq!(attempted["relationship_type"], "family_member");
    assert!(attempted["expires_at"].is_null());
    assert!(attempted["revoked_at"].is_null());
    assert!(attempted["carer_relationship_id"].is_null());

    let listed = body(Target::from_env().get(&path, Some(&fixture.access_token)));
    assert_eq!(
        listed["data"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| {
                row["household_membership_id"] == fixture.grant_target_membership_id
                    && row["person_id"] == fixture.hidden_person_id
                    && row["revoked_at"].is_null()
            })
            .count(),
        1
    );
    assert_eq!(
        membership_version(&mut db, fixture.grant_target_membership_id),
        version_before + 1
    );
    let after = grant_audit_outcomes(
        &mut db,
        fixture.household_id,
        fixture.grant_target_membership_id,
    );
    assert_eq!(
        outcome_count(&after, "success"),
        outcome_count(&audits_before, "success") + 1
    );
    assert_eq!(
        outcome_count(&after, "rejected"),
        outcome_count(&audits_before, "rejected") + 1
    );
    let deleted =
        Target::from_env().delete(&format!("{path}/{grant_id}"), Some(&fixture.access_token));
    assert_eq!(deleted.status().as_u16(), 204);
}

#[test]
fn all_person_grant_operations_use_the_shared_rate_limit() {
    let fixture = fixture();
    let base = env::var("CONTRACT_RATE_BASE_URL").expect("non-loopback API base URL");
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("rate limit client");
    let mut limited = None;
    for _ in 0..601 {
        let response = client
            .get(format!(
                "{}/api/v1/capabilities",
                base.trim_end_matches('/')
            ))
            .bearer_auth(&fixture.access_token)
            .send()
            .expect("capabilities request");
        if response.status().as_u16() == 429 {
            limited = Some(response);
            break;
        }
        assert_eq!(response.status().as_u16(), 200);
    }
    rate_limited(limited.expect("shared API rate limit"));

    let path = grants_path(&fixture);
    let url = format!("{}{}", base.trim_end_matches('/'), path);
    let payload = json!({"person_access_grant": {
        "household_membership_id": fixture.grant_target_membership_id,
        "person_id": fixture.managed_person_id,
        "access_level": "view",
        "relationship_type": "family_member"
    }});
    for request in [
        client.get(&url).bearer_auth(&fixture.access_token).send(),
        client
            .post(&url)
            .bearer_auth(&fixture.access_token)
            .json(&payload)
            .send(),
        client
            .delete(format!("{url}/1"))
            .bearer_auth(&fixture.access_token)
            .send(),
    ] {
        rate_limited(request.expect("grant operation request"));
    }
}
