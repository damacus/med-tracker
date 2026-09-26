use medtracker_contract_tests::{fixture, Fixture, Target};
use postgres::{Client, NoTls};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

static NEXT_TOKEN: AtomicU64 = AtomicU64::new(0);

fn token(prefix: &str) -> String {
    format!(
        "{prefix}-{}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT_TOKEN.fetch_add(1, Ordering::Relaxed)
    )
}

struct Disposable {
    db: Client,
    sessions: Vec<i64>,
    app_tokens: Vec<i64>,
    grants: Vec<i64>,
    applications: Vec<i64>,
    alternate_households: Vec<(i64, i64)>,
    refreshes: HashMap<i64, String>,
}

impl Disposable {
    fn new() -> Self {
        Self {
            db: Client::connect(
                &env::var("CONTRACT_AUDIT_DATABASE_URL").expect("database URL"),
                NoTls,
            )
            .expect("database"),
            sessions: Vec::new(),
            app_tokens: Vec::new(),
            grants: Vec::new(),
            applications: Vec::new(),
            alternate_households: Vec::new(),
            refreshes: HashMap::new(),
        }
    }

    fn session(&mut self, account_id: i64, membership_id: i64, id: Option<i64>) -> (i64, String) {
        let raw_access = token("mt-contract-session");
        let refresh = token("mt-contract-refresh");
        let row = self.db.query_one("INSERT INTO api_sessions (id, account_id, household_membership_id, access_token_digest, refresh_token_digest, permissions_version, device_name, access_expires_at, refresh_expires_at, last_used_at, created_at, updated_at) VALUES (COALESCE($1, nextval(pg_get_serial_sequence('api_sessions', 'id'))), $2, $3, encode(digest($4, 'sha256'), 'hex'), encode(digest($5, 'sha256'), 'hex'), (SELECT permissions_version FROM household_memberships WHERE id = $3), 'Contract device', now() + interval '1 hour', now() + interval '30 days', now(), now(), now()) RETURNING id", &[&id, &account_id, &membership_id, &raw_access, &refresh]).expect("session insert");
        let id: i64 = row.get(0);
        self.sessions.push(id);
        self.refreshes.insert(id, refresh);
        (id, raw_access)
    }

    fn app_token(&mut self, account_id: i64, membership_id: i64) -> (i64, String) {
        let token = token("mt_app_contract");
        let row = self.db.query_one("INSERT INTO api_app_tokens (account_id, household_membership_id, token_digest, permissions_version, name, expires_at, last_used_at, created_at, updated_at) VALUES ($1, $2, encode(digest($3, 'sha256'), 'hex'), (SELECT permissions_version FROM household_memberships WHERE id = $2), 'Contract app token', now() + interval '1 day', now(), now(), now()) RETURNING id", &[&account_id, &membership_id, &token]).expect("app token insert");
        let id: i64 = row.get(0);
        self.app_tokens.push(id);
        (id, token)
    }

    fn mobile(&mut self, fixture: &Fixture, account_id: i64, id: Option<i64>) -> (i64, String) {
        let token = token("contract-mobile");
        let row = self.db.query_one("INSERT INTO oauth_grants (id, account_id, oauth_application_id, client_kind, scopes, expires_in, authenticated_at, last_used_at, token_hash, created_at, updated_at) SELECT COALESCE($1, nextval(pg_get_serial_sequence('oauth_grants', 'id'))), $2, oauth_application_id, 'mobile', 'medtracker offline_access', now() + interval '1 hour', now(), now(), translate(encode(digest($3, 'sha256'), 'base64'), '+/', '-_'), now(), now() FROM oauth_grants WHERE id = $4 RETURNING id", &[&id, &account_id, &token, &fixture.medication_mobile_oauth_grant_id]).expect("mobile grant insert");
        let id: i64 = row.get(0);
        self.grants.push(id);
        (id, token)
    }

    fn integration(&mut self, fixture: &Fixture) -> (i64, String) {
        let client_id = token("contract-client");
        let application: i64 = self.db.query_one("INSERT INTO oauth_applications (client_id, client_kind, name, redirect_uri, scopes, token_endpoint_auth_method, created_at, updated_at) VALUES ($1, 'integration', 'Contract integration', 'https://example.invalid/callback', 'patient/*.rs', 'none', now(), now()) RETURNING id", &[&client_id]).expect("integration client").get(0);
        self.applications.push(application);
        let token = token("contract-integration");
        let id: i64 = self.db.query_one("INSERT INTO oauth_grants (account_id, oauth_application_id, client_kind, household_membership_id, person_id, permissions_version, scopes, expires_in, token_hash, created_at, updated_at) VALUES ($1, $2, 'integration', $3, $4, (SELECT permissions_version FROM household_memberships WHERE id = $3), 'patient/*.rs', now() + interval '1 hour', translate(encode(digest($5, 'sha256'), 'base64'), '+/', '-_'), now(), now()) RETURNING id", &[&fixture.account_id, &application, &fixture.owner_membership_id, &fixture.user_person_id, &token]).expect("integration grant").get(0);
        self.grants.push(id);
        (id, token)
    }

    fn foreign_account(&mut self, fixture: &Fixture) -> i64 {
        self.db
            .query_one(
                "SELECT account_id FROM household_memberships WHERE id = $1",
                &[&fixture.foreign_membership_id],
            )
            .expect("foreign membership")
            .get(0)
    }

    fn alternate_membership(&mut self, fixture: &Fixture) -> (i64, i64) {
        let slug = token("contract-sessions-household");
        let household_id: i64 = self.db.query_one("INSERT INTO households (created_by_account_id, name, slug, timezone, created_at, updated_at) VALUES ($1, 'Contract second household', $2, 'UTC', now(), now()) RETURNING id", &[&fixture.account_id, &slug]).expect("alternate household").get(0);
        let membership_id: i64 = self.db.query_one("INSERT INTO household_memberships (account_id, household_id, role, status, joined_at, created_at, updated_at) VALUES ($1, $2, 'owner', 'active', now(), now(), now()) RETURNING id", &[&fixture.account_id, &household_id]).expect("alternate membership").get(0);
        self.alternate_households
            .push((household_id, membership_id));
        (household_id, membership_id)
    }

    fn revoked(&mut self, table: &str, id: i64) -> bool {
        let sql = format!("SELECT revoked_at IS NOT NULL FROM {table} WHERE id = $1");
        self.db
            .query_one(&sql, &[&id])
            .expect("revocation state")
            .get(0)
    }

    fn active_refresh_exists(&mut self, id: i64) -> bool {
        let refresh = self.refreshes.get(&id).expect("refresh token");
        self.db.query_one("SELECT EXISTS (SELECT 1 FROM api_sessions WHERE id = $1 AND refresh_token_digest = encode(digest($2, 'sha256'), 'hex') AND revoked_at IS NULL AND refresh_expires_at > now())", &[&id, refresh]).expect("active refresh lookup").get(0)
    }

    fn version_count(&mut self, account_id: i64, event: &str) -> i64 {
        self.db.query_one("SELECT count(*) FROM versions WHERE item_type = 'AuthenticationToken' AND item_id = $1 AND event = $2", &[&account_id, &event]).expect("version count").get(0)
    }

    fn oauth_version_count(&mut self, grant_id: i64) -> i64 {
        self.db.query_one("SELECT count(*) FROM versions WHERE item_type = 'OauthGrant' AND item_id = $1 AND event = 'mobile_oauth.revoked'", &[&grant_id]).expect("OAuth version count").get(0)
    }
}

impl Drop for Disposable {
    fn drop(&mut self) {
        for id in &self.sessions {
            self.db
                .execute("DELETE FROM api_sessions WHERE id = $1", &[id])
                .expect("delete session");
        }
        for id in &self.app_tokens {
            self.db
                .execute("DELETE FROM api_app_tokens WHERE id = $1", &[id])
                .expect("delete app token");
        }
        for id in &self.grants {
            self.db
                .execute("DELETE FROM oauth_grants WHERE id = $1", &[id])
                .expect("delete mobile grant");
        }
        for id in &self.applications {
            self.db
                .execute("DELETE FROM oauth_applications WHERE id = $1", &[id])
                .expect("delete integration application");
        }
        for (household_id, membership_id) in &self.alternate_households {
            self.db.execute("DELETE FROM security_audit_events WHERE household_id = $1 AND event_type = 'auth_token/api_session/revoked'", &[household_id]).expect("delete alternate audit event");
            self.db.execute("DELETE FROM versions WHERE household_id = $1 AND event = 'auth_token/api_session/revoked'", &[household_id]).expect("delete alternate audit version");
            self.db
                .execute(
                    "DELETE FROM household_memberships WHERE id = $1",
                    &[membership_id],
                )
                .expect("delete alternate membership");
            self.db
                .execute("DELETE FROM households WHERE id = $1", &[household_id])
                .expect("delete alternate household");
        }
    }
}

fn keys(value: &Value) -> Vec<&str> {
    value
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect()
}

fn assert_session(row: &Value) {
    let mut fields = keys(row);
    assert!(row["id"].as_u64().is_some_and(|id| id > 0));
    assert!(row["device_name"].is_string() || row["device_name"].is_null());
    if row.get("household_id").is_some() {
        assert!(
            row["household_id"].is_null() || row["household_id"].as_u64().is_some_and(|id| id > 0)
        );
        fields.retain(|field| *field != "household_id");
    }
    assert_eq!(
        fields,
        [
            "access_token_expires_at",
            "created_at",
            "device_name",
            "id",
            "last_used_at",
            "refresh_token_expires_at"
        ]
    );
    for field in [
        "last_used_at",
        "access_token_expires_at",
        "refresh_token_expires_at",
        "created_at",
    ] {
        OffsetDateTime::parse(row[field].as_str().expect("timestamp"), &Rfc3339)
            .expect("RFC3339 timestamp");
    }
}

fn assert_error(response: reqwest::blocking::Response, status: u16) {
    assert_eq!(response.status().as_u16(), status);
    let body: Value = response.json().expect("error JSON");
    assert_eq!(keys(&body), ["error"]);
    assert!(keys(&body["error"])
        .iter()
        .all(|field| ["code", "message", "request_id", "errors"].contains(field)));
    for field in ["code", "message", "request_id"] {
        assert!(body["error"][field]
            .as_str()
            .is_some_and(|text| !text.is_empty()));
    }
}

fn assert_empty_204(response: reqwest::blocking::Response) {
    assert_eq!(response.status().as_u16(), 204);
    assert_eq!(response.text().expect("response body"), "");
}

#[test]
fn list_sessions_scopes_account_and_nonrevoked_rows() {
    let fixture = fixture();
    let mut disposable = Disposable::new();
    let target = Target::from_env();
    let (active_id, actor) =
        disposable.session(fixture.account_id, fixture.owner_membership_id, None);
    let (expired_id, expired) =
        disposable.session(fixture.account_id, fixture.owner_membership_id, None);
    disposable
        .db
        .execute(
            "UPDATE api_sessions SET access_expires_at = now() - interval '1 minute' WHERE id = $1",
            &[&expired_id],
        )
        .unwrap();
    let (revoked_id, revoked) =
        disposable.session(fixture.account_id, fixture.owner_membership_id, None);
    disposable
        .db
        .execute(
            "UPDATE api_sessions SET revoked_at = now() WHERE id = $1",
            &[&revoked_id],
        )
        .unwrap();
    let foreign_account = disposable.foreign_account(&fixture);
    let (foreign_id, _) = disposable.session(foreign_account, fixture.foreign_membership_id, None);
    let response = target.get("/api/v1/auth/sessions", Some(&actor));
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().unwrap();
    assert_eq!(keys(&body), ["data"]);
    let rows = body["data"].as_array().expect("sessions");
    for row in rows {
        assert_session(row);
    }
    let ids: Vec<i64> = rows.iter().map(|row| row["id"].as_i64().unwrap()).collect();
    assert!(ids.contains(&active_id) && ids.contains(&expired_id));
    assert!(!ids.contains(&revoked_id) && !ids.contains(&foreign_id));
    assert_error(target.get("/api/v1/auth/sessions", None), 401);
    for credential in ["invalid-token", expired.as_str(), revoked.as_str()] {
        assert_error(target.get("/api/v1/auth/sessions", Some(credential)), 401);
    }
    let (_, app) = disposable.app_token(fixture.account_id, fixture.owner_membership_id);
    let app_body: Value = target
        .get("/api/v1/auth/sessions", Some(&app))
        .json()
        .unwrap();
    assert!(app_body["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == active_id));
    let (old_app_id, old_app) =
        disposable.app_token(fixture.account_id, fixture.owner_membership_id);
    disposable.db.execute("UPDATE api_app_tokens SET created_at = now() - interval '13 months', expires_at = now() + interval '1 day' WHERE id = $1", &[&old_app_id]).unwrap();
    assert_error(target.get("/api/v1/auth/sessions", Some(&old_app)), 401);
}

#[test]
fn delete_session_revokes_only_own_selected_session() {
    let fixture = fixture();
    let mut disposable = Disposable::new();
    let target = Target::from_env();
    let (_, actor) = disposable.session(fixture.account_id, fixture.owner_membership_id, None);
    let (selected_id, selected) =
        disposable.session(fixture.account_id, fixture.owner_membership_id, None);
    let foreign_account = disposable.foreign_account(&fixture);
    let (foreign_id, _) = disposable.session(foreign_account, fixture.foreign_membership_id, None);
    assert_error(
        target.delete(&format!("/api/v1/auth/sessions/{foreign_id}"), Some(&actor)),
        404,
    );
    assert!(!disposable.revoked("api_sessions", foreign_id));
    assert_error(
        target.delete("/api/v1/auth/sessions/999999999999", Some(&actor)),
        404,
    );
    for malformed in ["abc", "+1"] {
        assert_error(
            target.delete(&format!("/api/v1/auth/sessions/{malformed}"), Some(&actor)),
            404,
        );
    }
    let audit_before =
        disposable.version_count(fixture.account_id, "auth_token/api_session/revoked");
    assert_empty_204(target.delete(
        &format!("/api/v1/auth/sessions/{selected_id}"),
        Some(&actor),
    ));
    assert!(disposable.revoked("api_sessions", selected_id));
    assert!(!disposable.active_refresh_exists(selected_id));
    assert_eq!(
        disposable.version_count(fixture.account_id, "auth_token/api_session/revoked"),
        audit_before + 1
    );
    assert_error(target.get("/api/v1/auth/sessions", Some(&selected)), 401);
    assert_error(
        target.delete(
            &format!("/api/v1/auth/sessions/{selected_id}"),
            Some(&actor),
        ),
        404,
    );
    assert_eq!(
        disposable.version_count(fixture.account_id, "auth_token/api_session/revoked"),
        audit_before + 1
    );
    let (app_target_id, _) =
        disposable.session(fixture.account_id, fixture.owner_membership_id, None);
    let (_, app) = disposable.app_token(fixture.account_id, fixture.owner_membership_id);
    assert_empty_204(target.delete(
        &format!("/api/v1/auth/sessions/{app_target_id}"),
        Some(&app),
    ));
    assert!(disposable.revoked("api_sessions", app_target_id));
}

#[test]
fn list_and_delete_reject_missing_or_invalid_credentials() {
    let fixture = fixture();
    let mut disposable = Disposable::new();
    let target = Target::from_env();
    let (selected_id, _) =
        disposable.session(fixture.account_id, fixture.owner_membership_id, None);
    let (unbound_id, unbound_token) =
        disposable.session(fixture.account_id, fixture.owner_membership_id, None);
    disposable
        .db
        .execute(
            "UPDATE api_sessions SET household_membership_id = NULL WHERE id = $1",
            &[&unbound_id],
        )
        .unwrap();
    let (old_app_id, old_app) =
        disposable.app_token(fixture.account_id, fixture.owner_membership_id);
    disposable.db.execute("UPDATE api_app_tokens SET created_at = now() - interval '13 months', expires_at = now() + interval '1 day' WHERE id = $1", &[&old_app_id]).unwrap();
    let path = format!("/api/v1/auth/sessions/{selected_id}");
    let mobile = &fixture.medication_mobile_oauth_tokens;
    let credentials = [
        "invalid-token",
        fixture.expired_access_token.as_str(),
        fixture.medication_read_revoked_access_token.as_str(),
        fixture.medication_read_stale_access_token.as_str(),
        fixture.auth_deactivated_access_token.as_str(),
        fixture.auth_inactive_access_token.as_str(),
        fixture.auth_suspended_access_token.as_str(),
        fixture.portable_revoked_access_token.as_str(),
        fixture.auth_deactivated_app_token.as_str(),
        old_app.as_str(),
        unbound_token.as_str(),
        mobile["expired"].as_str().expect("expired mobile token"),
        mobile["revoked"].as_str().expect("revoked mobile token"),
        mobile["inactive"].as_str().expect("inactive mobile token"),
        mobile["locked"].as_str().expect("locked mobile token"),
        mobile["no_scope"].as_str().expect("unscoped mobile token"),
        mobile["stale_login"]
            .as_str()
            .expect("stale login mobile token"),
        mobile["old_login"]
            .as_str()
            .expect("maximum age mobile token"),
    ];
    assert_error(target.get("/api/v1/auth/sessions", None), 401);
    assert_error(target.delete(&path, None), 401);
    for credential in credentials {
        assert_error(target.get("/api/v1/auth/sessions", Some(credential)), 401);
        assert_error(target.delete(&path, Some(credential)), 401);
    }
    assert!(!disposable.revoked("api_sessions", selected_id));
    let portable_mobile = mobile["revoked_membership"]
        .as_str()
        .expect("portable mobile token");
    assert_eq!(
        target
            .get("/api/v1/auth/sessions", Some(portable_mobile))
            .status()
            .as_u16(),
        200
    );
}

#[test]
fn selected_session_audit_uses_target_household() {
    let fixture = fixture();
    let mut disposable = Disposable::new();
    let target = Target::from_env();
    let (_, actor) = disposable.session(fixture.account_id, fixture.owner_membership_id, None);
    let (household_id, membership_id) = disposable.alternate_membership(&fixture);
    let (selected_id, _) = disposable.session(fixture.account_id, membership_id, None);
    assert_empty_204(target.delete(
        &format!("/api/v1/auth/sessions/{selected_id}"),
        Some(&actor),
    ));
    let version_household: i64 = disposable.db.query_one("SELECT household_id FROM versions WHERE item_type = 'AuthenticationToken' AND item_id = $1 AND event = 'auth_token/api_session/revoked' ORDER BY id DESC LIMIT 1", &[&fixture.account_id]).expect("target version").get(0);
    assert_eq!(version_household, household_id);
    let event_count: i64 = disposable.db.query_one("SELECT count(*) FROM security_audit_events WHERE household_id = $1 AND event_type = 'auth_token/api_session/revoked'", &[&household_id]).expect("target security audit").get(0);
    assert_eq!(event_count, 1);
}

#[test]
fn mobile_sessions_use_separate_namespace_even_when_ids_collide() {
    let fixture = fixture();
    let mut disposable = Disposable::new();
    let target = Target::from_env();
    let collision = 1_000_000_000_i64
        + (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            % 1_000_000) as i64;
    let (api_id, _) = disposable.session(
        fixture.account_id,
        fixture.owner_membership_id,
        Some(collision),
    );
    let (_, mobile_actor) = disposable.mobile(&fixture, fixture.account_id, None);
    let (grant_id, _) = disposable.mobile(&fixture, fixture.account_id, Some(collision));
    let foreign_account = disposable.foreign_account(&fixture);
    let (foreign_grant_id, _) = disposable.mobile(&fixture, foreign_account, None);
    let (expired_grant_id, _) = disposable.mobile(&fixture, fixture.account_id, None);
    disposable.db.execute("UPDATE oauth_grants SET authenticated_at = now() - interval '31 days', last_used_at = now() - interval '31 days' WHERE id = $1", &[&expired_grant_id]).unwrap();
    assert_eq!(api_id, grant_id);
    let response = target.get("/api/v1/auth/sessions", Some(&mobile_actor));
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().unwrap();
    assert_eq!(keys(&body), ["data"]);
    for row in body["data"].as_array().unwrap() {
        assert_session(row);
    }
    assert!(body["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == grant_id));
    assert!(!body["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == foreign_grant_id));
    assert_error(
        target.delete(
            &format!("/api/v1/auth/sessions/{foreign_grant_id}"),
            Some(&mobile_actor),
        ),
        404,
    );
    assert!(!disposable.revoked("oauth_grants", foreign_grant_id));
    assert_error(
        target.delete(
            &format!("/api/v1/auth/sessions/{expired_grant_id}"),
            Some(&mobile_actor),
        ),
        404,
    );
    assert!(!disposable.revoked("oauth_grants", expired_grant_id));
    let audit_before = disposable.oauth_version_count(grant_id);
    assert_empty_204(target.delete(
        &format!("/api/v1/auth/sessions/{grant_id}"),
        Some(&mobile_actor),
    ));
    assert!(disposable.revoked("oauth_grants", grant_id));
    assert_eq!(disposable.oauth_version_count(grant_id), audit_before + 1);
    assert!(!disposable.revoked("api_sessions", api_id));
}

#[test]
fn logout_revokes_session_app_token_and_mobile_grant_idempotently() {
    let fixture = fixture();
    let mut disposable = Disposable::new();
    let target = Target::from_env();
    let (session_id, session) =
        disposable.session(fixture.account_id, fixture.owner_membership_id, None);
    let (app_id, app) = disposable.app_token(fixture.account_id, fixture.owner_membership_id);
    let (grant_id, mobile) = disposable.mobile(&fixture, fixture.account_id, None);
    let (integration_id, integration) = disposable.integration(&fixture);
    assert_error(target.get("/api/v1/auth/sessions", Some(&integration)), 401);
    assert_error(
        target.delete(
            &format!("/api/v1/auth/sessions/{session_id}"),
            Some(&integration),
        ),
        401,
    );
    let session_events =
        disposable.version_count(fixture.account_id, "auth_token/api_session/revoked");
    let app_events =
        disposable.version_count(fixture.account_id, "auth_token/api_app_token/revoked");
    let mobile_events = disposable.oauth_version_count(grant_id);
    let integration_events = disposable.oauth_version_count(integration_id);
    for credential in [&session, &app, &mobile, &integration] {
        assert_empty_204(target.delete("/api/v1/auth/logout", Some(credential)));
        assert_empty_204(target.delete("/api/v1/auth/logout", Some(credential)));
    }
    assert!(disposable.revoked("api_sessions", session_id));
    assert!(!disposable.active_refresh_exists(session_id));
    assert!(disposable.revoked("api_app_tokens", app_id));
    assert!(disposable.revoked("oauth_grants", grant_id));
    assert!(disposable.revoked("oauth_grants", integration_id));
    assert_eq!(
        disposable.version_count(fixture.account_id, "auth_token/api_session/revoked"),
        session_events + 1
    );
    assert_eq!(
        disposable.version_count(fixture.account_id, "auth_token/api_app_token/revoked"),
        app_events + 1
    );
    assert_eq!(disposable.oauth_version_count(grant_id), mobile_events + 1);
    assert_eq!(
        disposable.oauth_version_count(integration_id),
        integration_events + 1
    );
    for credential in [&session, &app, &mobile, &integration] {
        assert_error(target.get("/api/v1/auth/sessions", Some(credential)), 401);
    }
    assert_empty_204(target.delete("/api/v1/auth/logout", None));
    assert_empty_204(target.delete("/api/v1/auth/logout", Some("invalid-token")));
}

#[test]
fn concurrent_selected_revoke_and_logout_record_one_audit_each() {
    let fixture = fixture();
    let mut disposable = Disposable::new();
    let (_, actor) = disposable.session(fixture.account_id, fixture.owner_membership_id, None);
    let (selected_id, _) =
        disposable.session(fixture.account_id, fixture.owner_membership_id, None);
    let audit_before =
        disposable.version_count(fixture.account_id, "auth_token/api_session/revoked");
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let barrier = barrier.clone();
            let actor = actor.clone();
            std::thread::spawn(move || {
                let target = Target::from_env();
                barrier.wait();
                target
                    .delete(
                        &format!("/api/v1/auth/sessions/{selected_id}"),
                        Some(&actor),
                    )
                    .status()
                    .as_u16()
            })
        })
        .collect();
    barrier.wait();
    let mut statuses: Vec<u16> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    statuses.sort_unstable();
    assert_eq!(statuses, [204, 404]);
    assert_eq!(
        disposable.version_count(fixture.account_id, "auth_token/api_session/revoked"),
        audit_before + 1
    );

    let (logout_id, logout_actor) =
        disposable.session(fixture.account_id, fixture.owner_membership_id, None);
    let audit_before =
        disposable.version_count(fixture.account_id, "auth_token/api_session/revoked");
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let barrier = barrier.clone();
            let actor = logout_actor.clone();
            std::thread::spawn(move || {
                let target = Target::from_env();
                barrier.wait();
                target
                    .delete("/api/v1/auth/logout", Some(&actor))
                    .status()
                    .as_u16()
            })
        })
        .collect();
    barrier.wait();
    let statuses: Vec<u16> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(statuses, [204, 204]);
    assert!(disposable.revoked("api_sessions", logout_id));
    assert_eq!(
        disposable.version_count(fixture.account_id, "auth_token/api_session/revoked"),
        audit_before + 1
    );
}

#[test]
fn z_rate_limit_covers_all_three_session_operations_without_revocation() {
    let fixture = fixture();
    let mut disposable = Disposable::new();
    let (actor_id, actor) =
        disposable.session(fixture.account_id, fixture.owner_membership_id, None);
    let (target_id, _) = disposable.session(fixture.account_id, fixture.owner_membership_id, None);
    let base = env::var("CONTRACT_RATE_BASE_URL").expect("nonloopback API URL");
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let started = Instant::now();
    let mut limited = false;
    for _ in 0..601 {
        let response = client
            .get(format!("{base}/api/v1/auth/sessions"))
            .bearer_auth(&actor)
            .send()
            .unwrap();
        if response.status().as_u16() == 429 {
            assert_eq!(response.headers()["ratelimit-limit"], "300");
            assert_eq!(response.headers()["ratelimit-remaining"], "0");
            assert!(response.headers().get("retry-after").is_some());
            assert!(response.headers().get("ratelimit-reset").is_some());
            let body: Value = response.json().unwrap();
            assert_eq!(keys(&body), ["error"]);
            assert_eq!(keys(&body["error"]), ["code", "message"]);
            assert_eq!(body["error"]["code"], "rate_limited");
            limited = true;
            break;
        }
        assert_eq!(response.status().as_u16(), 200);
    }
    assert!(limited && started.elapsed() < Duration::from_secs(60));
    for path in [
        format!("{base}/api/v1/auth/sessions/{target_id}"),
        format!("{base}/api/v1/auth/logout"),
    ] {
        let response = client.delete(path).bearer_auth(&actor).send().unwrap();
        assert_eq!(response.status().as_u16(), 429);
    }
    assert!(!disposable.revoked("api_sessions", actor_id));
    assert!(!disposable.revoked("api_sessions", target_id));
}
