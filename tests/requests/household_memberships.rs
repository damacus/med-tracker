use super::*;
use axum::http::Method;
use base64::Engine;

const COLLECTION: &str = "/households/{household_id}/admin/memberships";
const MEMBER: &str = "/households/{household_id}/admin/memberships/{id}";

struct Reply {
    status: u16,
    body: Value,
    request_id: Option<String>,
    content_type: Option<String>,
    replayed: Option<String>,
    challenge: Option<String>,
    retry_after: Option<String>,
    limit: Option<String>,
    remaining: Option<String>,
    reset: Option<String>,
    cache_control: Option<String>,
    empty_body: bool,
}

fn header(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

async fn seed(fixture: &Fixture) {
    profile::seed_credentials(fixture).await;
    fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,password_hash,status,created_at,updated_at) VALUES(71003,'membership-target@example.test',crypt('password',gen_salt('bf',4)),2,now(),now()); INSERT INTO household_memberships(id,account_id,household_id,person_id,role,status,permissions_version,joined_at,created_at,updated_at) VALUES(74002,71003,72001,73002,'member','active',1,now(),now(),now())").await.unwrap();
}

async fn send(
    server: &TestServer,
    method: Method,
    household: i64,
    target: Option<&str>,
    bearer: Option<&str>,
    body: Option<&Value>,
    key: Option<&str>,
) -> Reply {
    let path = match target {
        Some(id) => format!("/api/v1/households/{household}/admin/memberships/{id}"),
        None => format!("/api/v1/households/{household}/admin/memberships"),
    };
    let mut request = server.method(method, &path);
    if let Some(bearer) = bearer {
        request = request.add_header("authorization", format!("Bearer {bearer}"));
    }
    if let Some(key) = key {
        request = request.add_header("idempotency-key", key);
    }
    if let Some(body) = body {
        request = request.json(body);
    }
    let response = request.await;
    let status = response.status_code().as_u16();
    let empty_body = response.as_bytes().is_empty();
    let body = if status == 204 {
        Value::Null
    } else {
        response.json()
    };
    collect(status, body, response.headers(), empty_body)
}

fn collect(status: u16, body: Value, headers: &axum::http::HeaderMap, empty_body: bool) -> Reply {
    Reply {
        status,
        request_id: header(headers, "x-request-id"),
        content_type: header(headers, "content-type"),
        replayed: header(headers, "idempotency-replayed"),
        challenge: header(headers, "www-authenticate"),
        retry_after: header(headers, "retry-after"),
        limit: header(headers, "ratelimit-limit"),
        remaining: header(headers, "ratelimit-remaining"),
        reset: header(headers, "ratelimit-reset"),
        cache_control: header(headers, "cache-control"),
        empty_body,
        body,
    }
}

async fn send_raw(server: &TestServer, method: Method) -> Reply {
    let response = server
        .method(method, "/api/v1/households/72001/admin/memberships/74002")
        .add_header("authorization", "Bearer profile-session")
        .add_header("content-type", "application/json")
        .text("{")
        .await;
    collect(
        response.status_code().as_u16(),
        response.json(),
        response.headers(),
        response.as_bytes().is_empty(),
    )
}

fn assert_wire(reply: &Reply) {
    let request_id = reply.request_id.as_deref().expect("request ID");
    assert!(!request_id.is_empty());
    if reply.status == 204 {
        assert!(reply.empty_body);
    } else {
        assert_eq!(reply.content_type.as_deref(), Some("application/json"));
        if reply.status != 429 && reply.body.get("error").is_some() {
            assert_eq!(reply.body["error"]["request_id"], request_id);
        }
    }
}

fn assert_schema(method: &str, reply: &Reply) {
    let document = contract::contract();
    let path = if method == "get" { COLLECTION } else { MEMBER };
    let raw = &document["paths"][path][method]["responses"][reply.status.to_string()];
    let described = if raw.get("$ref").is_some() {
        contract::resolve(document, raw)
    } else {
        raw
    };
    assert!(
        described.is_object(),
        "{method} membership {} undocumented: {}",
        reply.status,
        reply.body
    );
    if reply.status != 204 {
        contract::assert_value(
            document,
            &described["content"]["application/json"]["schema"],
            &reply.body,
            &format!("{method} membership {}", reply.status),
        );
    }
}

async fn state(fixture: &Fixture) -> Value {
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT role,status,person_id,permissions_version,revoked_at IS NOT NULL AS revoked,(SELECT count(*) FROM api_idempotency_keys WHERE request_path LIKE '%/admin/memberships/%') AS keys,(SELECT count(*) FROM api_change_events) AS changes FROM household_memberships WHERE id=74002"
    )).await.unwrap().unwrap();
    json!({
        "role":row.try_get::<String>("","role").unwrap(),
        "status":row.try_get::<String>("","status").unwrap(),
        "person_id":row.try_get::<Option<i64>>("","person_id").unwrap(),
        "permissions":row.try_get::<i32>("","permissions_version").unwrap(),
        "revoked":row.try_get::<bool>("","revoked").unwrap(),
        "keys":row.try_get::<i64>("","keys").unwrap(),
        "changes":row.try_get::<i64>("","changes").unwrap(),
    })
}

async fn audit_count(fixture: &Fixture) -> i64 {
    fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS total FROM security_audit_events WHERE household_id=72001",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "total")
        .unwrap()
}

#[test]
fn membership_methods_document_existing_keyed_replay() {
    let document = contract::contract();
    let mut missing = Vec::new();
    for method in ["patch", "put", "delete"] {
        let operation = &document["paths"][MEMBER][method];
        if !operation["parameters"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|parameter| parameter["$ref"] == "#/components/parameters/idempotency_key")
        {
            missing.push(format!("{method}: Idempotency-Key"));
        }
        if operation["responses"]["409"]["$ref"] != "#/components/responses/Conflict" {
            missing.push(format!("{method}: Conflict 409"));
        }
    }
    assert!(missing.is_empty(), "{}", missing.join(", "));
}

#[tokio::test]
async fn membership_collection_scopes_three_credentials_and_validates_schema() {
    let (fixture, boot) = boot_fixture().await;
    seed(&fixture).await;
    let mobile = profile::mobile_token(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let mut replies = Vec::new();
    for (name, token) in [
        ("api", "profile-session"),
        ("app", "profile-app"),
        ("mobile", mobile.as_str()),
    ] {
        for (household, expected) in [(72001, 200), (72004, 403), (99999, 404)] {
            replies.push((
                name,
                household,
                expected,
                send(
                    &server,
                    Method::GET,
                    household,
                    None,
                    Some(token),
                    None,
                    None,
                )
                .await,
            ));
        }
    }
    let anonymous = send(&server, Method::GET, 72001, None, None, None, None).await;
    let after = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before, after);
    assert_eq!(anonymous.status, 401);
    assert_wire(&anonymous);
    assert_schema("get", &anonymous);
    for (name, household, expected, reply) in replies {
        assert_eq!(reply.status, expected, "{name} {household}: {}", reply.body);
        assert_wire(&reply);
        assert_schema("get", &reply);
        if expected == 200 {
            let rows = reply.body["data"].as_array().unwrap();
            assert_eq!(rows.len(), 2);
            assert_eq!(
                (rows[0]["id"].as_i64(), rows[1]["id"].as_i64()),
                (Some(74001), Some(74002))
            );
            assert_eq!(rows[1]["account_id"], 71003);
            assert_eq!(rows[1]["email"], "membership-target@example.test");
            assert_eq!(rows[1]["person_id"], 73002);
            assert_eq!(rows[1]["role"], "member");
            assert_eq!(rows[1]["permissions_version"], 1);
        } else {
            assert!(reply.body.get("data").is_none());
        }
    }
}

#[tokio::test]
async fn membership_write_methods_save_current_target_and_return_method_schemas() {
    let (fixture, boot) = boot_fixture().await;
    seed(&fixture).await;
    let mobile = profile::mobile_token(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let mut writes = Vec::new();
    for method in [Method::PATCH, Method::PUT] {
        for (name, token) in [
            ("api", "profile-session"),
            ("app", "profile-app"),
            ("mobile", mobile.as_str()),
        ] {
            let next = if writes.len() % 2 == 0 {
                "suspended"
            } else {
                "active"
            };
            let body = json!({"household_membership":{"status":next}});
            writes.push((
                method.as_str().to_lowercase(),
                name,
                next,
                send(
                    &server,
                    method.clone(),
                    72001,
                    Some("74002"),
                    Some(token),
                    Some(&body),
                    None,
                )
                .await,
            ));
        }
    }
    let after_writes = state(&fixture).await;
    let removed = send(
        &server,
        Method::DELETE,
        72001,
        Some("74002"),
        Some("profile-session"),
        None,
        None,
    )
    .await;
    let after_delete = state(&fixture).await;
    let audit_rows = fixture.admin.query_all_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT request_id,actor_account_id,actor_membership_id,metadata,audit_context FROM security_audit_events WHERE event_type='household_access.membership_changed' AND metadata->>'target_membership_id'='74002' ORDER BY id"
    )).await.unwrap();
    let audits: Vec<Value> = audit_rows
        .iter()
        .map(|row| {
            json!({
                "request_id":row.try_get::<String>("","request_id").unwrap(),
                "account":row.try_get::<i64>("","actor_account_id").unwrap(),
                "membership":row.try_get::<i64>("","actor_membership_id").unwrap(),
                "metadata":row.try_get::<Value>("","metadata").unwrap(),
                "context":row.try_get::<Value>("","audit_context").unwrap(),
            })
        })
        .collect();
    drop(server);
    close(fixture, boot).await;
    assert_eq!(audits.len(), 7);
    for (method, name, next, reply) in writes {
        assert_eq!(reply.status, 200, "{method} {name}: {}", reply.body);
        assert_wire(&reply);
        assert_schema(&method, &reply);
        assert_eq!(reply.body["data"]["status"], next);
        assert_eq!(reply.body["data"]["role"], "member");
        assert_eq!(reply.body["data"]["person_id"], 73002);
        let audit = audits
            .iter()
            .find(|audit| audit["request_id"] == reply.request_id.as_deref().unwrap())
            .unwrap();
        assert_eq!(audit["account"], 71001);
        assert_eq!(audit["membership"], 74001);
        assert_eq!(audit["metadata"]["target_account_id"], 71003);
        assert_eq!(audit["metadata"]["target_membership_id"], 74002);
        assert_eq!(audit["metadata"]["new_state"]["status"], next);
        assert_eq!(audit["metadata"]["outcome"], "success");
        assert_eq!(
            audit["context"]["request_id"],
            reply.request_id.as_deref().unwrap()
        );
        assert_eq!(
            audit["context"]["authentication_method"],
            match name {
                "api" => "api_session",
                "app" => "api_app_token",
                _ => "oauth",
            }
        );
        assert_eq!(
            audit["context"]["session_reference"],
            match name {
                "api" => "api_session:88031",
                "app" => "api_app_token:88032",
                _ => "oauth_grant:76001",
            }
        );
    }
    assert_eq!(after_writes["status"], "active");
    assert_eq!(after_writes["role"], before["role"]);
    assert_eq!(after_writes["person_id"], before["person_id"]);
    assert_eq!(after_writes["changes"], before["changes"]);
    assert_eq!(
        after_writes["permissions"].as_i64().unwrap(),
        before["permissions"].as_i64().unwrap() + 6
    );
    assert_eq!(removed.status, 204, "{}", removed.body);
    assert_wire(&removed);
    assert_schema("delete", &removed);
    assert_eq!(removed.body, Value::Null);
    let removed_audit = audits
        .iter()
        .find(|audit| audit["request_id"] == removed.request_id.as_deref().unwrap())
        .unwrap();
    assert_eq!(removed_audit["metadata"]["target_membership_id"], 74002);
    assert_eq!(removed_audit["metadata"]["new_state"]["status"], "revoked");
    assert_eq!(
        removed_audit["context"]["authentication_method"],
        "api_session"
    );
    assert_eq!(after_delete["status"], "revoked");
    assert_eq!(after_delete["revoked"], true);
    assert_eq!(after_delete["changes"], before["changes"]);
    assert_eq!(
        after_delete["permissions"].as_i64().unwrap(),
        after_writes["permissions"].as_i64().unwrap() + 1
    );
}

#[tokio::test]
async fn membership_delete_revokes_target_with_each_credential() {
    for kind in ["api", "app", "mobile"] {
        let (fixture, boot) = boot_fixture().await;
        seed(&fixture).await;
        fixture.admin.execute_unprepared("INSERT INTO people(id,household_id,account_id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73007,72001,71003,'Membership target adult','1990-01-01',0,true,now(),now()); INSERT INTO users(id,person_id,email_address,password_digest,active,created_at,updated_at) VALUES(77007,73007,'membership-target@example.test',crypt('password',gen_salt('bf',4)),true,now(),now()); UPDATE household_memberships SET person_id=73007 WHERE id=74002").await.unwrap();
        let target_token = match kind {
            "api" => {
                fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO api_sessions(id,account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,created_at,updated_at) VALUES(88033,71003,74002,$1,'membership-target-refresh',1,now()+interval '15 minutes',now()+interval '1 day',now(),now(),now())",[hex::encode(Sha256::digest(b"membership-target-session")).into()])).await.unwrap();
                "membership-target-session"
            }
            "app" => {
                fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO api_app_tokens(id,account_id,household_membership_id,name,token_digest,permissions_version,expires_at,last_used_at,created_at,updated_at) VALUES(88034,71003,74002,'Membership target app',$1,1,now()+interval '1 month',now(),now(),now())",[hex::encode(Sha256::digest(b"membership-target-app")).into()])).await.unwrap();
                "membership-target-app"
            }
            _ => {
                fixture.admin.execute_unprepared("INSERT INTO oauth_applications(id,account_id,client_id,client_kind,name,redirect_uri,scopes,token_endpoint_auth_method,created_at,updated_at) VALUES(75004,71003,'membership-target-native','mobile','Membership target native','io.damacus.medtracker:/oauth2redirect','medtracker offline_access','none',now(),now())").await.unwrap();
                let digest = base64::engine::general_purpose::URL_SAFE
                    .encode(Sha256::digest(b"membership-target-mobile"));
                fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO oauth_grants(id,account_id,oauth_application_id,client_kind,redirect_uri,scopes,token_hash,expires_in,authenticated_at,last_used_at,device_name,created_at,updated_at) VALUES(76010,71003,75004,'mobile','io.damacus.medtracker:/oauth2redirect','medtracker offline_access',$1,now()+interval '15 minutes',now(),now(),'Membership target mobile',now(),now())",[digest.into()])).await.unwrap();
                "membership-target-mobile"
            }
        };
        let mobile = profile::mobile_token(&fixture).await;
        let token = match kind {
            "api" => "profile-session",
            "app" => "profile-app",
            _ => mobile.as_str(),
        };
        let server = server(&boot);
        let target_before = server
            .get("/api/v1/households/72001/me")
            .add_header("authorization", format!("Bearer {target_token}"))
            .await;
        let target_before_status = target_before.status_code().as_u16();
        let target_before_body: Value = target_before.json();
        let before = state(&fixture).await;
        let reply = send(
            &server,
            Method::DELETE,
            72001,
            Some("74002"),
            Some(token),
            None,
            None,
        )
        .await;
        let after = state(&fixture).await;
        let audits_after_delete = audit_count(&fixture).await;
        let target_after = server
            .get("/api/v1/households/72001/me")
            .add_header("authorization", format!("Bearer {target_token}"))
            .await;
        let target_after_status = target_after.status_code().as_u16();
        let target_after_body: Value = target_after.json();
        let after_denial = state(&fixture).await;
        let audits_after_denial = audit_count(&fixture).await;
        drop(server);
        close(fixture, boot).await;
        assert_eq!(target_before_status, 200, "{kind}: {target_before_body}");
        assert_eq!(reply.status, 204, "{kind}: {}", reply.body);
        assert_wire(&reply);
        assert_schema("delete", &reply);
        assert_eq!(after["status"], "revoked");
        assert_eq!(after["revoked"], true);
        assert_eq!(
            after["permissions"].as_i64().unwrap(),
            before["permissions"].as_i64().unwrap() + 1
        );
        assert_eq!(after["changes"], before["changes"]);
        assert_eq!(
            target_after_status,
            if kind == "mobile" { 403 } else { 401 },
            "{kind}: {target_after_body}"
        );
        assert_eq!(
            target_after_body["error"]["code"],
            if kind == "mobile" {
                "forbidden"
            } else {
                "unauthorized"
            }
        );
        assert!(target_after_body.get("data").is_none());
        assert_eq!(
            after_denial, after,
            "{kind} denied household read changed membership state"
        );
        assert_eq!(
            audits_after_denial, audits_after_delete,
            "{kind} denied household read wrote an audit"
        );
    }
}

#[tokio::test]
async fn membership_status_change_invalidates_its_own_live_session() {
    let (fixture, boot) = boot_fixture().await;
    seed(&fixture).await;
    fixture.admin.execute_unprepared("INSERT INTO people(id,household_id,account_id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73007,72001,71003,'Membership target adult','1990-01-01',0,true,now(),now()); INSERT INTO users(id,person_id,email_address,password_digest,active,created_at,updated_at) VALUES(77007,73007,'membership-target@example.test',crypt('password',gen_salt('bf',4)),true,now(),now()); UPDATE household_memberships SET person_id=73007 WHERE id=74002").await.unwrap();
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO api_sessions(id,account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,created_at,updated_at) VALUES(88033,71003,74002,$1,'membership-target-refresh',1,now()+interval '15 minutes',now()+interval '1 day',now(),now(),now())",
        [hex::encode(Sha256::digest(b"membership-target-session")).into()]
    )).await.unwrap();
    let server = server(&boot);
    let before = server
        .get("/api/v1/auth/sessions")
        .add_header("authorization", "Bearer membership-target-session")
        .await;
    let before_status = before.status_code().as_u16();
    let change = send(
        &server,
        Method::PATCH,
        72001,
        Some("74002"),
        Some("profile-session"),
        Some(&json!({"household_membership":{"status":"suspended"}})),
        None,
    )
    .await;
    let after = server
        .get("/api/v1/auth/sessions")
        .add_header("authorization", "Bearer membership-target-session")
        .await;
    let after_status = after.status_code().as_u16();
    let after_body: Value = after.json();
    let target = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before_status, 200);
    assert_eq!(change.status, 200, "{}", change.body);
    assert_wire(&change);
    assert_schema("patch", &change);
    assert_eq!(target["status"], "suspended");
    assert_eq!(target["permissions"], 2);
    assert_eq!(after_status, 401, "{after_body}");
    assert_eq!(after_body["error"]["code"], "unauthorized");
}

#[tokio::test]
async fn membership_writes_distinguish_foreign_absent_and_invalid_targets_without_changes() {
    let (fixture, boot) = boot_fixture().await;
    seed(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let body = json!({"household_membership":{"role":"administrator"}});
    let mut replies = Vec::new();
    for method in [Method::PATCH, Method::PUT, Method::DELETE] {
        for (household, id, expected) in [
            (72004, "74004", 403),
            (99999, "74002", 404),
            (72001, "74004", 404),
            (72001, "99999", 404),
            (72001, "abc", 404),
        ] {
            replies.push((
                method.as_str().to_lowercase(),
                household,
                id,
                expected,
                send(
                    &server,
                    method.clone(),
                    household,
                    Some(id),
                    Some("profile-session"),
                    (method != Method::DELETE).then_some(&body),
                    None,
                )
                .await,
            ));
        }
    }
    let after = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before, after);
    for (method, household, id, expected, reply) in replies {
        assert_eq!(
            reply.status, expected,
            "{method} {household}/{id}: {}",
            reply.body
        );
        assert_wire(&reply);
        assert_schema(&method, &reply);
        assert!(reply.body.get("data").is_none());
    }
}

#[tokio::test]
async fn membership_updates_validate_fields_and_preserve_last_owner() {
    let (fixture, boot) = boot_fixture().await;
    seed(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let mut replies = Vec::new();
    for method in [Method::PATCH, Method::PUT] {
        for (expected, body) in [
            (400, json!({"household_membership":{}})),
            (
                400,
                json!({"household_membership":{"permissions_version":900}}),
            ),
            (422, json!({"household_membership":{"role":"invalid"}})),
            (422, json!({"household_membership":{"status":false}})),
            (422, json!({"household_membership":{"person_id":99999}})),
            (422, json!({"household_membership":{"role":"owner"}})),
        ] {
            replies.push((
                method.as_str().to_lowercase(),
                expected,
                send(
                    &server,
                    method.clone(),
                    72001,
                    Some("74002"),
                    Some("profile-session"),
                    Some(&body),
                    None,
                )
                .await,
            ));
        }
        replies.push((
            method.as_str().to_lowercase(),
            400,
            send_raw(&server, method).await,
        ));
    }
    let after = state(&fixture).await;
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='owner' WHERE id=74001")
        .await
        .unwrap();
    let mut last_owner = Vec::new();
    for (method, body) in [
        (
            Method::PATCH,
            Some(json!({"household_membership":{"role":"member"}})),
        ),
        (
            Method::PUT,
            Some(json!({"household_membership":{"role":"administrator"}})),
        ),
        (Method::DELETE, None),
    ] {
        last_owner.push((
            method.as_str().to_lowercase(),
            send(
                &server,
                method,
                72001,
                Some("74001"),
                Some("profile-session"),
                body.as_ref(),
                None,
            )
            .await,
        ));
    }
    let owner = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT role,status,permissions_version,revoked_at IS NOT NULL AS revoked FROM household_memberships WHERE id=74001")).await.unwrap().unwrap();
    let owner_state = json!({"role":owner.try_get::<String>("","role").unwrap(),"status":owner.try_get::<String>("","status").unwrap(),"permissions":owner.try_get::<i32>("","permissions_version").unwrap(),"revoked":owner.try_get::<bool>("","revoked").unwrap()});
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before, after);
    for (method, expected, reply) in replies {
        assert_eq!(reply.status, expected, "{method}: {}", reply.body);
        assert_wire(&reply);
        assert_schema(&method, &reply);
        assert!(reply.body.get("data").is_none());
    }
    for (method, reply) in last_owner {
        assert_eq!(reply.status, 422, "{method}: {}", reply.body);
        assert_eq!(
            reply.body["error"]["errors"]["base"][0],
            "Last active owner cannot be removed"
        );
        assert_wire(&reply);
        assert_schema(&method, &reply);
    }
    assert_eq!(
        owner_state,
        json!({"role":"owner","status":"active","permissions":1,"revoked":false})
    );
}

#[tokio::test]
async fn membership_put_ignores_protected_input_when_allowed_status_is_present() {
    let (fixture, boot) = boot_fixture().await;
    seed(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let audits_before = audit_count(&fixture).await;
    let reply = send(
        &server,
        Method::PUT,
        72001,
        Some("74002"),
        Some("profile-session"),
        Some(&json!({"household_membership":{"status":"suspended","permissions_version":900}})),
        Some("membership-protected-input"),
    )
    .await;
    let after = state(&fixture).await;
    let audits_after = audit_count(&fixture).await;
    let audit_rows = fixture.admin.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT event_type,request_id,actor_account_id,actor_membership_id,metadata,audit_context FROM security_audit_events WHERE household_id=72001 AND request_id=$1 ORDER BY id",
        [reply.request_id.clone().unwrap().into()],
    )).await.unwrap();
    let audits: Vec<Value> = audit_rows
        .iter()
        .map(|row| {
            json!({
                "event_type":row.try_get::<String>("", "event_type").unwrap(),
                "request_id":row.try_get::<String>("", "request_id").unwrap(),
                "account":row.try_get::<i64>("", "actor_account_id").unwrap(),
                "membership":row.try_get::<i64>("", "actor_membership_id").unwrap(),
                "metadata":row.try_get::<Value>("", "metadata").unwrap(),
                "context":row.try_get::<Value>("", "audit_context").unwrap(),
            })
        })
        .collect();
    drop(server);
    close(fixture, boot).await;
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert_wire(&reply);
    assert_schema("put", &reply);
    assert_eq!(reply.body["data"]["status"], "suspended");
    assert_eq!(reply.body["data"]["permissions_version"], 2);
    assert_eq!(after["status"], "suspended");
    assert_eq!(after["permissions"], 2);
    assert_eq!(after["role"], before["role"]);
    assert_eq!(after["person_id"], before["person_id"]);
    assert_eq!(after["revoked"], before["revoked"]);
    assert_eq!(
        after["keys"].as_i64(),
        before["keys"].as_i64().map(|count| count + 1)
    );
    assert_eq!(after["changes"], before["changes"]);
    assert_eq!(audits_after, audits_before + 2);
    assert_eq!(audits.len(), 2);
    let change = audits
        .iter()
        .find(|audit| audit["event_type"] == "household_access.membership_changed")
        .unwrap();
    let request = audits
        .iter()
        .find(|audit| audit["event_type"] == "api.request")
        .unwrap();
    for audit in [change, request] {
        assert_eq!(audit["request_id"], reply.request_id.as_deref().unwrap());
        assert_eq!(audit["account"], 71001);
        assert_eq!(audit["membership"], 74001);
        assert_eq!(
            audit["context"]["request_id"],
            reply.request_id.as_deref().unwrap()
        );
    }
    assert_eq!(change["metadata"]["target_membership_id"], 74002);
    assert_eq!(change["metadata"]["new_state"]["status"], "suspended");
    assert_eq!(change["metadata"]["outcome"], "success");
    assert_eq!(request["metadata"]["http_method"], "PUT");
    assert_eq!(
        request["metadata"]["controller"],
        "api/v1/admin/memberships"
    );
    assert_eq!(request["metadata"]["action"], "update");
    assert_eq!(request["metadata"]["outcome"], "success");
    assert_eq!(request["metadata"]["status"], 200);
}

#[tokio::test]
async fn membership_routes_reject_invalid_revoked_smart_and_insufficient_scope_credentials() {
    let (fixture, boot) = boot_fixture().await;
    seed(&fixture).await;
    let mobile = profile::mobile_token(&fixture).await;
    profile::seed_disallowed_grants(&fixture, "smart-memberships").await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let body = json!({"household_membership":{"status":"suspended"}});
    let mut replies = Vec::new();
    for method in [Method::GET, Method::PATCH, Method::PUT, Method::DELETE] {
        for (label, token, expected) in [
            ("invalid", "invalid-memberships", 401),
            ("smart", "smart-memberships", 401),
            ("wrong-scope", mobile.as_str(), 403),
        ] {
            replies.push((
                method.as_str().to_lowercase(),
                label,
                expected,
                send(
                    &server,
                    method.clone(),
                    72001,
                    (method != Method::GET).then_some("74002"),
                    Some(token),
                    matches!(method, Method::PATCH | Method::PUT).then_some(&body),
                    None,
                )
                .await,
            ));
        }
    }
    fixture.admin.execute_unprepared("UPDATE api_sessions SET revoked_at=now() WHERE id=88031; UPDATE api_app_tokens SET revoked_at=now() WHERE id=88032; UPDATE oauth_grants SET revoked_at=now() WHERE id=76001").await.unwrap();
    for method in [Method::GET, Method::PATCH, Method::PUT, Method::DELETE] {
        for token in ["profile-session", "profile-app", mobile.as_str()] {
            replies.push((
                method.as_str().to_lowercase(),
                "revoked",
                401,
                send(
                    &server,
                    method.clone(),
                    72001,
                    (method != Method::GET).then_some("74002"),
                    Some(token),
                    matches!(method, Method::PATCH | Method::PUT).then_some(&body),
                    None,
                )
                .await,
            ));
        }
    }
    let after = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before, after);
    for (method, label, expected, reply) in replies {
        assert_eq!(reply.status, expected, "{method} {label}: {}", reply.body);
        assert_wire(&reply);
        assert_schema(&method, &reply);
        assert!(reply.body.get("data").is_none());
        if expected == 401 {
            assert_eq!(reply.challenge.as_deref(), Some("Bearer"));
        } else if label == "wrong-scope" {
            assert_eq!(
                reply.challenge.as_deref(),
                Some("Bearer error=\"insufficient_scope\" scope=\"medtracker\"")
            );
        }
    }
}

#[tokio::test]
async fn membership_routes_recheck_current_manager_and_withdrawn_membership() {
    let (fixture, boot) = boot_fixture().await;
    seed(&fixture).await;
    let mobile = profile::mobile_token(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let body = json!({"household_membership":{"role":"administrator"}});
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let mut replies = Vec::new();
    for method in [Method::GET, Method::PATCH, Method::PUT, Method::DELETE] {
        for token in ["profile-session", "profile-app", mobile.as_str()] {
            replies.push((
                method.as_str().to_lowercase(),
                "demoted",
                403,
                send(
                    &server,
                    method.clone(),
                    72001,
                    (method != Method::GET).then_some("74002"),
                    Some(token),
                    matches!(method, Method::PATCH | Method::PUT).then_some(&body),
                    None,
                )
                .await,
            ));
        }
    }
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET revoked_at=now() WHERE id=74001")
        .await
        .unwrap();
    for method in [Method::GET, Method::PATCH, Method::PUT, Method::DELETE] {
        for (token, expected) in [
            ("profile-session", 401),
            ("profile-app", 401),
            (mobile.as_str(), 403),
        ] {
            replies.push((
                method.as_str().to_lowercase(),
                "withdrawn",
                expected,
                send(
                    &server,
                    method.clone(),
                    72001,
                    (method != Method::GET).then_some("74002"),
                    Some(token),
                    matches!(method, Method::PATCH | Method::PUT).then_some(&body),
                    None,
                )
                .await,
            ));
        }
    }
    let after = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before, after);
    for (method, label, expected, reply) in replies {
        assert_eq!(reply.status, expected, "{method} {label}: {}", reply.body);
        assert_wire(&reply);
        assert_schema(&method, &reply);
        assert!(reply.body.get("data").is_none());
    }
}

async fn keyed_write(method: Method) {
    let (fixture, boot) = boot_fixture().await;
    seed(&fixture).await;
    let mobile = profile::mobile_token(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let name = method.as_str().to_lowercase();
    let key = format!("membership-{name}-key");
    let body = json!({"household_membership":{"status":"suspended"}});
    let changed = json!({"household_membership":{"status":"revoked"}});
    let submitted = (method != Method::DELETE).then_some(&body);
    let first = send(
        &server,
        method.clone(),
        72001,
        Some("74002"),
        Some("profile-session"),
        submitted,
        Some(&key),
    )
    .await;
    let after_first = state(&fixture).await;
    let replay = send(
        &server,
        method.clone(),
        72001,
        Some("74002"),
        Some("profile-app"),
        submitted,
        Some(&key),
    )
    .await;
    let mobile_replay = send(
        &server,
        method.clone(),
        72001,
        Some("74002"),
        Some(&mobile),
        submitted,
        Some(&key),
    )
    .await;
    let after_replay = state(&fixture).await;
    let conflict = send(
        &server,
        method.clone(),
        72001,
        Some(if method == Method::DELETE {
            "74001"
        } else {
            "74002"
        }),
        Some("profile-app"),
        (method != Method::DELETE).then_some(&changed),
        Some(&key),
    )
    .await;
    let after_conflict = state(&fixture).await;
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let denied = send(
        &server,
        method,
        72001,
        Some("74002"),
        Some("profile-session"),
        submitted,
        Some(&key),
    )
    .await;
    let after_denial = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    let status = if name == "delete" { 204 } else { 200 };
    assert_eq!(
        (
            first.status,
            replay.status,
            mobile_replay.status,
            conflict.status,
            denied.status
        ),
        (status, status, status, 409, 403)
    );
    assert_eq!(replay.replayed.as_deref(), Some("true"));
    assert_eq!(mobile_replay.replayed.as_deref(), Some("true"));
    assert_eq!(first.body, replay.body);
    assert_eq!(first.body, mobile_replay.body);
    assert_eq!(
        after_first["permissions"].as_i64().unwrap(),
        before["permissions"].as_i64().unwrap() + 1
    );
    assert_eq!(after_replay, after_first);
    assert_eq!(after_conflict, after_first);
    assert_eq!(after_denial, after_first);
    for reply in [&first, &replay, &mobile_replay, &conflict, &denied] {
        assert_wire(reply);
        assert_schema(&name, reply);
    }
    assert_eq!(conflict.body["error"]["code"], "idempotency_key_reused");
}

#[tokio::test]
async fn patch_membership_keyed_replay_commits_once() {
    keyed_write(Method::PATCH).await;
}

#[tokio::test]
async fn put_membership_keyed_replay_commits_once() {
    keyed_write(Method::PUT).await;
}

#[tokio::test]
async fn delete_membership_keyed_replay_commits_once() {
    keyed_write(Method::DELETE).await;
}

async fn overlap(method: Method) {
    let (fixture, boot) = boot_fixture().await;
    seed(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let name = method.as_str().to_lowercase();
    let key = format!("membership-{name}-overlap");
    let body = json!({"household_membership":{"status":"suspended"}});
    let (first, second, observed) = super::overlap_requests_at_row_lock(
        &fixture,
        "SELECT id FROM households WHERE id=72001 FOR UPDATE",
        "%households%FOR UPDATE%",
        async {
            send(
                &server,
                method.clone(),
                72001,
                Some("74002"),
                Some("profile-session"),
                (method != Method::DELETE).then_some(&body),
                Some(&key),
            )
            .await
        },
        async {
            send(
                &server,
                method.clone(),
                72001,
                Some("74002"),
                Some("profile-app"),
                (method != Method::DELETE).then_some(&body),
                Some(&key),
            )
            .await
        },
    )
    .await;
    let after = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert!(observed, "{name}: two household lock waiters");
    let status = if name == "delete" { 204 } else { 200 };
    assert_eq!(
        (first.status, second.status),
        (status, status),
        "{} / {}",
        first.body,
        second.body
    );
    assert_eq!(
        usize::from(first.replayed.as_deref() == Some("true"))
            + usize::from(second.replayed.as_deref() == Some("true")),
        1
    );
    assert!(
        [first.replayed.as_deref(), second.replayed.as_deref()]
            .into_iter()
            .all(|marker| marker.is_none() || marker == Some("true"))
    );
    assert_eq!(first.body, second.body);
    assert_eq!(
        after["permissions"].as_i64().unwrap(),
        before["permissions"].as_i64().unwrap() + 1
    );
    assert_eq!(
        after["keys"].as_i64().unwrap(),
        before["keys"].as_i64().unwrap() + 1
    );
    for reply in [&first, &second] {
        assert_wire(reply);
        assert_schema(&name, reply);
    }
}

#[tokio::test]
async fn patch_membership_overlap_replays_one_write() {
    overlap(Method::PATCH).await;
}

#[tokio::test]
async fn put_membership_overlap_replays_one_write() {
    overlap(Method::PUT).await;
}

#[tokio::test]
async fn delete_membership_overlap_replays_one_write() {
    overlap(Method::DELETE).await;
}

#[tokio::test]
async fn membership_routes_share_fixed_general_rate_limit_without_domain_effects() {
    let (fixture, boot) = boot_rate_fixture(&[]).await;
    seed(&fixture).await;
    let server = server(&boot);
    for _ in 0..300 {
        assert_eq!(
            server
                .get("/api/v1/capabilities")
                .await
                .status_code()
                .as_u16(),
            200
        );
    }
    let before = state(&fixture).await;
    let body = json!({"household_membership":{"status":"suspended"}});
    let mut replies = Vec::new();
    for method in [Method::GET, Method::PATCH, Method::PUT, Method::DELETE] {
        replies.push((
            method.as_str().to_lowercase(),
            send(
                &server,
                method.clone(),
                72001,
                (method != Method::GET).then_some("74002"),
                Some("profile-session"),
                matches!(method, Method::PATCH | Method::PUT).then_some(&body),
                None,
            )
            .await,
        ));
    }
    let after = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before, after);
    for (method, reply) in replies {
        assert_eq!(reply.status, 429, "{method}: {}", reply.body);
        assert_eq!(reply.retry_after.as_deref(), Some("200"));
        assert_eq!(reply.limit.as_deref(), Some("300"));
        assert_eq!(reply.remaining.as_deref(), Some("0"));
        assert_eq!(reply.reset.as_deref(), Some("1200"));
        assert_eq!(reply.cache_control.as_deref(), Some("no-store"));
        assert_wire(&reply);
        assert_schema(&method, &reply);
    }
}

#[tokio::test]
async fn membership_audit_failure_hides_reads_and_rolls_back_writes() {
    let (fixture, boot) = boot_fixture().await;
    seed(&fixture).await;
    fixture.admin.execute_unprepared("CREATE FUNCTION reject_membership_request_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='api.request' AND NEW.metadata->>'controller'='api/v1/admin/memberships' THEN RAISE EXCEPTION 'Synthetic membership request audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_membership_request_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION reject_membership_request_audit()").await.unwrap();
    let server = server(&boot);
    let before = state(&fixture).await;
    let audits_before = audit_count(&fixture).await;
    let body = json!({"household_membership":{"status":"suspended"}});
    let mut replies = Vec::new();
    for method in [Method::GET, Method::PATCH, Method::PUT, Method::DELETE] {
        replies.push((
            method.as_str().to_lowercase(),
            send(
                &server,
                method.clone(),
                72001,
                (method != Method::GET).then_some("74002"),
                Some("profile-session"),
                matches!(method, Method::PATCH | Method::PUT).then_some(&body),
                Some("membership-audit-fault"),
            )
            .await,
        ));
    }
    let after = state(&fixture).await;
    let audits_after = audit_count(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before, after);
    assert_eq!(audits_before, audits_after);
    for (method, reply) in &replies {
        assert_eq!(reply.status, 500, "{method}: {}", reply.body);
        assert_eq!(reply.body["error"]["code"], "internal_error");
        assert!(reply.body.get("data").is_none());
        assert_wire(reply);
    }
    for (method, reply) in replies {
        assert_schema(&method, &reply);
    }
}

#[tokio::test]
async fn membership_routes_reject_locked_and_expired_credentials_before_domain_access() {
    for condition in [
        "api_expired",
        "app_expired",
        "mobile_expired",
        "account_locked",
    ] {
        let (fixture, boot) = boot_fixture().await;
        seed(&fixture).await;
        let mobile = profile::mobile_token(&fixture).await;
        match condition {
            "api_expired" => fixture.admin.execute_unprepared(
                "UPDATE api_sessions SET access_expires_at=now()-interval '1 minute' WHERE id=88031"
            ).await.unwrap(),
            "app_expired" => fixture.admin.execute_unprepared(
                "UPDATE api_app_tokens SET created_at=now()-interval '2 days',expires_at=now()-interval '1 day' WHERE id=88032"
            ).await.unwrap(),
            "mobile_expired" => fixture.admin.execute_unprepared(
                "UPDATE oauth_grants SET expires_in=now()-interval '1 minute' WHERE id=76001"
            ).await.unwrap(),
            _ => fixture.admin.execute_unprepared(
                "INSERT INTO account_lockouts(account_id,deadline,key,created_at,updated_at) VALUES(71001,now()+interval '1 hour','membership-route-lockout',now(),now())"
            ).await.unwrap(),
        };
        let server = server(&boot);
        let before = state(&fixture).await;
        let audits_before = audit_count(&fixture).await;
        let mut replies = Vec::new();
        let bearers: Vec<&str> = if condition == "account_locked" {
            vec!["profile-session", "profile-app", mobile.as_str()]
        } else {
            vec![match condition {
                "api_expired" => "profile-session",
                "app_expired" => "profile-app",
                _ => mobile.as_str(),
            }]
        };
        let body = json!({"household_membership":{"status":"suspended"}});
        for bearer in bearers {
            for method in [Method::GET, Method::PATCH, Method::PUT, Method::DELETE] {
                replies.push((
                    method.as_str().to_lowercase(),
                    bearer.to_owned(),
                    send(
                        &server,
                        method.clone(),
                        72001,
                        (method != Method::GET).then_some("74002"),
                        Some(bearer),
                        matches!(method, Method::PATCH | Method::PUT).then_some(&body),
                        Some("membership-denied-saved-key"),
                    )
                    .await,
                ));
            }
        }
        let after = state(&fixture).await;
        let audits_after = audit_count(&fixture).await;
        drop(server);
        close(fixture, boot).await;
        assert_eq!(after, before, "{condition} changed target/key/change state");
        assert_eq!(
            audits_after, audits_before,
            "{condition} wrote a target audit"
        );
        for (method, bearer, reply) in replies {
            assert_eq!(
                reply.status, 401,
                "{condition} {method} {bearer}: {}",
                reply.body
            );
            assert_eq!(reply.body["error"]["code"], "unauthorized");
            assert!(reply.body.get("data").is_none());
            assert_eq!(reply.challenge.as_deref(), Some("Bearer"));
            assert_wire(&reply);
            assert_schema(&method, &reply);
        }
    }
}

#[tokio::test]
async fn membership_keyed_writes_recheck_actor_withdrawn_while_waiting_for_household_lock() {
    for method in [Method::PATCH, Method::PUT, Method::DELETE] {
        let (fixture, boot) = boot_fixture().await;
        seed(&fixture).await;
        fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,password_hash,status,created_at,updated_at) VALUES(71004,'membership-second-owner@example.test',crypt('password',gen_salt('bf',4)),2,now(),now()); INSERT INTO people(id,household_id,account_id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73008,72001,71004,'Membership second owner','1990-01-01',0,true,now(),now()); INSERT INTO users(id,person_id,email_address,password_digest,active,created_at,updated_at) VALUES(77008,73008,'membership-second-owner@example.test',crypt('password',gen_salt('bf',4)),true,now(),now()); INSERT INTO household_memberships(id,account_id,household_id,person_id,role,status,permissions_version,joined_at,created_at,updated_at) VALUES(74008,71004,72001,73008,'owner','active',1,now(),now(),now())").await.unwrap();
        let server = server(&boot);
        let before = state(&fixture).await;
        let audits_before = audit_count(&fixture).await;
        let blocker = fixture.admin.begin().await.unwrap();
        let row = blocker
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT pg_backend_pid() AS pid",
            ))
            .await
            .unwrap()
            .unwrap();
        let blocker_pid = row.try_get::<i32>("", "pid").unwrap();
        blocker
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT id FROM households WHERE id=72001 FOR UPDATE",
            ))
            .await
            .unwrap()
            .unwrap();
        let runtime_role = url::Url::parse(&fixture.runtime_uri)
            .unwrap()
            .username()
            .to_owned();
        let body = json!({"household_membership":{"status":"suspended"}});
        let name = method.as_str().to_lowercase();
        let key = format!("membership-withdrawn-{name}");
        let request = send(
            &server,
            method.clone(),
            72001,
            Some("74002"),
            Some("profile-session"),
            (method != Method::DELETE).then_some(&body),
            Some(&key),
        );
        let release = async {
            let observed = tokio::time::timeout(std::time::Duration::from_secs(10), async {
                loop {
                    let row = fixture.admin.query_one_raw(Statement::from_sql_and_values(
                        DbBackend::Postgres,
                        "SELECT count(DISTINCT pid) AS waiting, count(DISTINCT pid) FILTER (WHERE $2::int=ANY(pg_blocking_pids(pid))) AS held_waiters FROM pg_stat_activity WHERE datname=current_database() AND usename=$1 AND wait_event_type='Lock' AND query LIKE $3 AND cardinality(pg_blocking_pids(pid))>0",
                        [runtime_role.clone().into(), blocker_pid.into(), "%households%FOR UPDATE%".into()],
                    )).await.unwrap().unwrap();
                    if row.try_get::<i64>("", "waiting").unwrap() == 1
                        && row.try_get::<i64>("", "held_waiters").unwrap() == 1 {
                        break true;
                    }
                    tokio::task::yield_now().await;
                }
            }).await.unwrap_or(false);
            if observed {
                blocker.execute_unprepared(
                    "UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74001"
                ).await.unwrap();
            }
            blocker.commit().await.unwrap();
            observed
        };
        let (reply, observed) = tokio::join!(request, release);
        let actor = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT status,permissions_version,revoked_at IS NOT NULL AS revoked FROM household_memberships WHERE id=74001"
        )).await.unwrap().unwrap();
        let actor_state = json!({
            "status":actor.try_get::<String>("", "status").unwrap(),
            "version":actor.try_get::<i32>("", "permissions_version").unwrap(),
            "revoked":actor.try_get::<bool>("", "revoked").unwrap(),
        });
        let owner = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT role,status,permissions_version,revoked_at IS NOT NULL AS revoked FROM household_memberships WHERE id=74008"
        )).await.unwrap().unwrap();
        let owner_state = json!({
            "role":owner.try_get::<String>("", "role").unwrap(),
            "status":owner.try_get::<String>("", "status").unwrap(),
            "version":owner.try_get::<i32>("", "permissions_version").unwrap(),
            "revoked":owner.try_get::<bool>("", "revoked").unwrap(),
        });
        let retry = send(
            &server,
            method.clone(),
            72001,
            Some("74002"),
            Some("profile-session"),
            (method != Method::DELETE).then_some(&body),
            Some(&key),
        )
        .await;
        let after = state(&fixture).await;
        let audits_after = audit_count(&fixture).await;
        let audit_rows = fixture.admin.query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT event_type,request_id,actor_account_id,actor_membership_id,metadata,audit_context FROM security_audit_events WHERE household_id=72001 AND request_id=$1",
            [reply.request_id.clone().unwrap().into()],
        )).await.unwrap();
        let audits: Vec<Value> = audit_rows
            .iter()
            .map(|row| {
                json!({
                    "event_type":row.try_get::<String>("", "event_type").unwrap(),
                    "request_id":row.try_get::<String>("", "request_id").unwrap(),
                    "account":row.try_get::<i64>("", "actor_account_id").unwrap(),
                    "membership":row.try_get::<i64>("", "actor_membership_id").unwrap(),
                    "metadata":row.try_get::<Value>("", "metadata").unwrap(),
                    "context":row.try_get::<Value>("", "audit_context").unwrap(),
                })
            })
            .collect();
        drop(server);
        close(fixture, boot).await;
        assert!(
            observed,
            "{name}: restricted runtime never waited on held household row"
        );
        assert_eq!(
            actor_state,
            json!({"status":"revoked","version":2,"revoked":true})
        );
        assert_eq!(
            owner_state,
            json!({"role":"owner","status":"active","version":1,"revoked":false})
        );
        assert_eq!(reply.status, 403, "{name}: {}", reply.body);
        assert_eq!(retry.status, 401, "{name}: {}", retry.body);
        assert_eq!(retry.challenge.as_deref(), Some("Bearer"));
        assert_eq!(
            after, before,
            "{name}: denied write changed target/key/change state"
        );
        assert_eq!(audits_after, audits_before + 1);
        assert_eq!(audits.len(), 1, "{name}: expected one denial audit");
        let audit = &audits[0];
        assert_eq!(audit["event_type"], "api.request");
        assert_eq!(audit["request_id"], reply.request_id.as_deref().unwrap());
        assert_eq!(audit["account"], 71001);
        assert_eq!(audit["membership"], 74001);
        assert_eq!(audit["metadata"]["http_method"], method.as_str());
        assert_eq!(audit["metadata"]["controller"], "api/v1/admin/memberships");
        assert_eq!(
            audit["metadata"]["action"],
            if method == Method::DELETE {
                "destroy"
            } else {
                "update"
            }
        );
        assert_eq!(audit["metadata"]["outcome"], "failure");
        assert_eq!(audit["metadata"]["status"], 403);
        assert_eq!(
            audit["context"]["request_id"],
            reply.request_id.as_deref().unwrap()
        );
        assert_eq!(audit["context"]["authentication_method"], "api_session");
        assert_eq!(audit["context"]["session_reference"], "api_session:88031");
        for denied in [&reply, &retry] {
            assert!(denied.body.get("data").is_none());
            assert_wire(denied);
            assert_schema(&name, denied);
        }
    }
}
