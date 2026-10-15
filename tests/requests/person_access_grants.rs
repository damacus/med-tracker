use super::*;
use axum::http::Method;

const COLLECTION: &str = "/households/{household_id}/admin/person_access_grants";
const MEMBER: &str = "/households/{household_id}/admin/person_access_grants/{id}";

struct Reply {
    status: u16,
    body: Value,
    empty: bool,
    request_id: Option<String>,
    content_type: Option<String>,
    replayed: Option<String>,
    challenge: Option<String>,
    retry_after: Option<String>,
    limit: Option<String>,
    remaining: Option<String>,
    reset: Option<String>,
    cache_control: Option<String>,
}

fn header(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn collect(status: u16, body: Value, empty: bool, headers: &axum::http::HeaderMap) -> Reply {
    Reply {
        status,
        body,
        empty,
        request_id: header(headers, "x-request-id"),
        content_type: header(headers, "content-type"),
        replayed: header(headers, "idempotency-replayed"),
        challenge: header(headers, "www-authenticate"),
        retry_after: header(headers, "retry-after"),
        limit: header(headers, "ratelimit-limit"),
        remaining: header(headers, "ratelimit-remaining"),
        reset: header(headers, "ratelimit-reset"),
        cache_control: header(headers, "cache-control"),
    }
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
        Some(id) => format!("/api/v1/households/{household}/admin/person_access_grants/{id}"),
        None => format!("/api/v1/households/{household}/admin/person_access_grants"),
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
    let empty = response.as_bytes().is_empty();
    let body = if status == 204 {
        Value::Null
    } else {
        response.json()
    };
    collect(status, body, empty, response.headers())
}

fn assert_wire(reply: &Reply) {
    let request_id = reply.request_id.as_deref().expect("request ID");
    assert!(!request_id.is_empty());
    if reply.status == 204 {
        assert!(reply.empty);
    } else {
        assert_eq!(reply.content_type.as_deref(), Some("application/json"));
        if reply.status != 429 && reply.body.get("error").is_some() {
            assert_eq!(reply.body["error"]["request_id"], request_id);
        }
    }
}

fn assert_schema(method: &str, reply: &Reply) {
    let document = contract::contract();
    let path = if method == "delete" {
        MEMBER
    } else {
        COLLECTION
    };
    let raw = &document["paths"][path][method]["responses"][reply.status.to_string()];
    let described = if raw.get("$ref").is_some() {
        contract::resolve(document, raw)
    } else {
        raw
    };
    assert!(
        described.is_object(),
        "{method} grant {} undocumented: {}",
        reply.status,
        reply.body
    );
    if reply.status != 204 {
        contract::assert_value(
            document,
            &described["content"]["application/json"]["schema"],
            &reply.body,
            &format!("{method} grant {}", reply.status),
        );
    }
}

async fn seed(fixture: &Fixture) {
    profile::seed_credentials(fixture).await;
    fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,password_hash,status,created_at,updated_at) VALUES(71003,'grant-target@example.test',crypt('password',gen_salt('bf',4)),2,now(),now()); INSERT INTO household_memberships(id,account_id,household_id,person_id,role,status,permissions_version,joined_at,created_at,updated_at) VALUES(74002,71003,72001,73002,'member','active',1,now(),now(),now())").await.unwrap();
}

fn body(person_id: i64) -> Value {
    json!({"person_access_grant":{"household_membership_id":74002,"person_id":person_id,"access_level":"manage","relationship_type":"parent"}})
}

async fn state(fixture: &Fixture) -> Value {
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT permissions_version,(SELECT count(*) FROM person_access_grants WHERE household_membership_id=74002) AS grants,(SELECT count(*) FROM person_access_grants WHERE household_membership_id=74002 AND revoked_at IS NULL) AS active,(SELECT count(*) FROM api_idempotency_keys WHERE request_path LIKE '%/admin/person_access_grants%') AS keys,(SELECT count(*) FROM api_change_events) AS changes FROM household_memberships WHERE id=74002"
    )).await.unwrap().unwrap();
    json!({
        "version":row.try_get::<i32>("","permissions_version").unwrap(),
        "grants":row.try_get::<i64>("","grants").unwrap(),
        "active":row.try_get::<i64>("","active").unwrap(),
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
fn person_grant_methods_document_keyed_requests() {
    let document = contract::contract();
    let mut missing = Vec::new();
    for (path, method) in [(COLLECTION, "post"), (MEMBER, "delete")] {
        let operation = &document["paths"][path][method];
        if !operation["parameters"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|item| item["$ref"] == "#/components/parameters/idempotency_key")
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
async fn person_grant_collection_scopes_three_credentials_and_validates_schema() {
    let (fixture, boot) = boot_fixture().await;
    seed(&fixture).await;
    let mobile = profile::mobile_token(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let mut replies = Vec::new();
    for bearer in ["profile-session", "profile-app", mobile.as_str()] {
        for (household, expected) in [(72001, 200), (72004, 403), (99999, 404)] {
            replies.push((
                household,
                expected,
                send(
                    &server,
                    Method::GET,
                    household,
                    None,
                    Some(bearer),
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
    for (household, expected, reply) in replies {
        assert_eq!(reply.status, expected, "{household}: {}", reply.body);
        if household == 72001 {
            assert!(
                reply.body["data"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|row| row["id"] == 78001)
            );
        }
        assert_wire(&reply);
        assert_schema("get", &reply);
    }
    assert_eq!(anonymous.status, 401);
    assert_eq!(anonymous.challenge.as_deref(), Some("Bearer"));
    assert_wire(&anonymous);
    assert_schema("get", &anonymous);
}

#[tokio::test]
async fn person_grant_create_accepts_three_credentials_and_records_scoped_effects() {
    for bearer_kind in ["api", "app", "mobile"] {
        let (fixture, boot) = boot_fixture().await;
        seed(&fixture).await;
        let mobile = profile::mobile_token(&fixture).await;
        let bearer = match bearer_kind {
            "api" => "profile-session",
            "app" => "profile-app",
            _ => mobile.as_str(),
        };
        let server = server(&boot);
        let before = state(&fixture).await;
        let audits_before = audit_count(&fixture).await;
        let reply = send(
            &server,
            Method::POST,
            72001,
            None,
            Some(bearer),
            Some(&body(73002)),
            None,
        )
        .await;
        let after = state(&fixture).await;
        let audits_after = audit_count(&fixture).await;
        let audit_rows = fixture.admin.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT event_type,request_id,actor_account_id,actor_membership_id,metadata,audit_context FROM security_audit_events WHERE household_id=72001 AND request_id=$1",
            [reply.request_id.clone().unwrap().into()],
        )).await.unwrap();
        drop(server);
        close(fixture, boot).await;
        assert_eq!(reply.status, 201, "{bearer_kind}: {}", reply.body);
        assert_eq!(reply.body["data"]["person_id"], 73002);
        assert_eq!(
            after["grants"].as_i64(),
            before["grants"].as_i64().map(|n| n + 1)
        );
        assert_eq!(
            after["active"].as_i64(),
            before["active"].as_i64().map(|n| n + 1)
        );
        assert_eq!(
            after["version"].as_i64(),
            before["version"].as_i64().map(|n| n + 1)
        );
        assert_eq!(after["keys"], before["keys"]);
        assert_eq!(audits_after, audits_before + 2);
        assert_eq!(audit_rows.len(), 2);
        let event_types: Vec<String> = audit_rows
            .iter()
            .map(|row| row.try_get::<String>("", "event_type").unwrap())
            .collect();
        assert!(event_types.contains(&"household_access.person_grant_changed".to_owned()));
        assert!(event_types.contains(&"api.request".to_owned()));
        for row in audit_rows {
            assert_eq!(
                row.try_get::<String>("", "request_id").unwrap(),
                reply.request_id.as_deref().unwrap()
            );
            assert_eq!(row.try_get::<i64>("", "actor_account_id").unwrap(), 71001);
            assert_eq!(
                row.try_get::<i64>("", "actor_membership_id").unwrap(),
                74001
            );
            let context: Value = row.try_get("", "audit_context").unwrap();
            assert_eq!(context["request_id"], reply.request_id.as_deref().unwrap());
            assert_eq!(
                context["authentication_method"],
                match bearer_kind {
                    "api" => "api_session",
                    "app" => "api_app_token",
                    _ => "oauth",
                }
            );
            assert_eq!(
                context["session_reference"],
                match bearer_kind {
                    "api" => "api_session:88031",
                    "app" => "api_app_token:88032",
                    _ => "oauth_grant:76001",
                }
            );
            let metadata: Value = row.try_get("", "metadata").unwrap();
            if metadata.get("target_membership_id").is_some() {
                assert_eq!(metadata["target_membership_id"], 74002);
                assert_eq!(metadata["new_state"]["person_id"], 73002);
                assert_eq!(metadata["outcome"], "success");
            }
        }
        assert_wire(&reply);
        assert_schema("post", &reply);
    }
}

#[tokio::test]
async fn person_grant_delete_revokes_three_credentials_with_empty_body() {
    for bearer_kind in ["api", "app", "mobile"] {
        let (fixture, boot) = boot_fixture().await;
        seed(&fixture).await;
        let mobile = profile::mobile_token(&fixture).await;
        let bearer = match bearer_kind {
            "api" => "profile-session",
            "app" => "profile-app",
            _ => mobile.as_str(),
        };
        let server = server(&boot);
        let created = send(
            &server,
            Method::POST,
            72001,
            None,
            Some(bearer),
            Some(&body(73002)),
            None,
        )
        .await;
        let id = created.body["data"]["id"].as_i64().unwrap().to_string();
        let before = state(&fixture).await;
        let audits_before = audit_count(&fixture).await;
        let first = send(
            &server,
            Method::DELETE,
            72001,
            Some(&id),
            Some(bearer),
            None,
            None,
        )
        .await;
        let middle = state(&fixture).await;
        let second = send(
            &server,
            Method::DELETE,
            72001,
            Some(&id),
            Some(bearer),
            None,
            None,
        )
        .await;
        let after = state(&fixture).await;
        let audits_after = audit_count(&fixture).await;
        let outcomes = fixture.admin.query_all_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT metadata->>'outcome' AS outcome FROM security_audit_events WHERE household_id=72001 AND event_type='household_access.person_grant_changed' AND request_id IN ($1,$2)",
            [first.request_id.clone().unwrap().into(),second.request_id.clone().unwrap().into()],
        )).await.unwrap();
        let listed = send(&server, Method::GET, 72001, None, Some(bearer), None, None).await;
        drop(server);
        close(fixture, boot).await;
        assert_eq!(
            (created.status, first.status, second.status),
            (201, 204, 204)
        );
        for reply in [&first, &second] {
            assert_wire(reply);
            assert_schema("delete", reply);
        }
        assert_eq!(
            middle["active"].as_i64(),
            before["active"].as_i64().map(|n| n - 1)
        );
        assert_eq!(
            middle["version"].as_i64(),
            before["version"].as_i64().map(|n| n + 1)
        );
        assert_eq!(
            middle, after,
            "{bearer_kind} repeated revoke changed the domain"
        );
        assert_eq!(audits_after, audits_before + 4);
        let values: Vec<String> = outcomes
            .iter()
            .map(|row| row.try_get::<String>("", "outcome").unwrap())
            .collect();
        assert!(values.contains(&"success".to_owned()));
        assert!(values.contains(&"no_change".to_owned()));
        assert_eq!(listed.status, 200, "{bearer_kind}: {}", listed.body);
        assert!(listed.body["data"].as_array().unwrap().iter().any(|row|row["id"]==id.parse::<i64>().unwrap() && row["revoked_at"].is_string()));
        assert_wire(&listed);
        assert_schema("get", &listed);
    }
}

#[tokio::test]
async fn person_grant_routes_reject_invalid_revoked_smart_and_scope_credentials() {
    let (fixture, boot) = boot_fixture().await;
    seed(&fixture).await;
    let mobile = profile::mobile_token(&fixture).await;
    profile::seed_disallowed_grants(&fixture, "smart-grants").await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let mut replies = Vec::new();
    for method in [Method::GET, Method::POST, Method::DELETE] {
        for (label, bearer, expected) in [
            ("anonymous", None, 401),
            ("invalid", Some("invalid-grants"), 401),
            ("smart", Some("smart-grants"), 401),
            ("scope", Some(mobile.as_str()), 403),
        ] {
            replies.push((
                method.as_str().to_lowercase(),
                label,
                expected,
                send(
                    &server,
                    method.clone(),
                    72001,
                    (method == Method::DELETE).then_some("78001"),
                    bearer,
                    (method == Method::POST).then_some(&body(73002)),
                    None,
                )
                .await,
            ));
        }
    }
    fixture.admin.execute_unprepared("UPDATE api_sessions SET revoked_at=now() WHERE id=88031; UPDATE api_app_tokens SET revoked_at=now() WHERE id=88032; UPDATE oauth_grants SET revoked_at=now() WHERE id=76001").await.unwrap();
    for method in [Method::GET, Method::POST, Method::DELETE] {
        for bearer in ["profile-session", "profile-app", mobile.as_str()] {
            replies.push((
                method.as_str().to_lowercase(),
                "revoked",
                401,
                send(
                    &server,
                    method.clone(),
                    72001,
                    (method == Method::DELETE).then_some("78001"),
                    Some(bearer),
                    (method == Method::POST).then_some(&body(73002)),
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
        assert!(reply.body.get("data").is_none());
        if expected == 401 {
            assert_eq!(reply.challenge.as_deref(), Some("Bearer"));
        }
        if label == "scope" {
            assert_eq!(
                reply.challenge.as_deref(),
                Some("Bearer error=\"insufficient_scope\" scope=\"medtracker\"")
            );
        }
        assert_wire(&reply);
        assert_schema(&method, &reply);
    }
}

#[tokio::test]
async fn person_grant_routes_reject_locked_and_expired_credentials() {
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
            "api_expired" => fixture.admin.execute_unprepared("UPDATE api_sessions SET access_expires_at=now()-interval '1 minute' WHERE id=88031").await.unwrap(),
            "app_expired" => fixture.admin.execute_unprepared("UPDATE api_app_tokens SET created_at=now()-interval '2 days',expires_at=now()-interval '1 day' WHERE id=88032").await.unwrap(),
            "mobile_expired" => fixture.admin.execute_unprepared("UPDATE oauth_grants SET expires_in=now()-interval '1 minute' WHERE id=76001").await.unwrap(),
            _ => fixture.admin.execute_unprepared("INSERT INTO account_lockouts(account_id,deadline,key,created_at,updated_at) VALUES(71001,now()+interval '1 hour','grant-lockout',now(),now())").await.unwrap(),
        };
        let server = server(&boot);
        let before = state(&fixture).await;
        let audits_before = audit_count(&fixture).await;
        let bearers: Vec<&str> = if condition == "account_locked" {
            vec!["profile-session", "profile-app", mobile.as_str()]
        } else {
            vec![match condition {
                "api_expired" => "profile-session",
                "app_expired" => "profile-app",
                _ => mobile.as_str(),
            }]
        };
        let mut replies = Vec::new();
        for bearer in bearers {
            for method in [Method::GET, Method::POST, Method::DELETE] {
                replies.push((
                    method.as_str().to_lowercase(),
                    send(
                        &server,
                        method.clone(),
                        72001,
                        (method == Method::DELETE).then_some("78001"),
                        Some(bearer),
                        (method == Method::POST).then_some(&body(73002)),
                        Some("denied-grant-key"),
                    )
                    .await,
                ));
            }
        }
        let after = state(&fixture).await;
        let audits_after = audit_count(&fixture).await;
        drop(server);
        close(fixture, boot).await;
        assert_eq!(before, after, "{condition} domain changed");
        assert_eq!(
            audits_before, audits_after,
            "{condition} wrote tenant audit"
        );
        for (method, reply) in replies {
            assert_eq!(reply.status, 401, "{condition} {method}: {}", reply.body);
            assert_eq!(reply.challenge.as_deref(), Some("Bearer"));
            assert!(reply.body.get("data").is_none());
            assert_wire(&reply);
            assert_schema(&method, &reply);
        }
    }
}

#[tokio::test]
async fn person_grant_routes_recheck_manager_and_target_scope() {
    let (fixture, boot) = boot_fixture().await;
    seed(&fixture).await;
    let mobile = profile::mobile_token(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let mut replies = Vec::new();
    for method in [Method::GET, Method::POST, Method::DELETE] {
        let target = (method == Method::DELETE).then_some("78001");
        let input = (method == Method::POST).then_some(body(73002));
        replies.push((
            method.as_str().to_lowercase(),
            403,
            send(
                &server,
                method.clone(),
                72004,
                target,
                Some("profile-session"),
                input.as_ref(),
                None,
            )
            .await,
        ));
        replies.push((
            method.as_str().to_lowercase(),
            404,
            send(
                &server,
                method.clone(),
                99999,
                target,
                Some("profile-session"),
                input.as_ref(),
                None,
            )
            .await,
        ));
    }
    let foreign_target = send(&server,Method::POST,72001,None,Some("profile-session"),Some(&json!({"person_access_grant":{"household_membership_id":74004,"person_id":73005,"access_level":"view","relationship_type":"family_member"}})),None).await;
    let missing = send(
        &server,
        Method::DELETE,
        72001,
        Some("999999"),
        Some("profile-session"),
        None,
        None,
    )
    .await;
    let malformed = send(
        &server,
        Method::DELETE,
        72001,
        Some("bad"),
        Some("profile-session"),
        None,
        None,
    )
    .await;
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    for method in [Method::GET, Method::POST, Method::DELETE] {
        replies.push((
            method.as_str().to_lowercase(),
            403,
            send(
                &server,
                method.clone(),
                72001,
                (method == Method::DELETE).then_some("78001"),
                Some(mobile.as_str()),
                (method == Method::POST).then_some(&body(73002)),
                None,
            )
            .await,
        ));
    }
    let after = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before, after);
    for (method, expected, reply) in replies {
        assert_eq!(reply.status, expected, "{method}: {}", reply.body);
        assert_wire(&reply);
        assert_schema(&method, &reply);
    }
    assert_eq!(
        (foreign_target.status, missing.status, malformed.status),
        (422, 404, 404)
    );
    for reply in [&foreign_target, &missing, &malformed] {
        assert_wire(reply);
        assert_schema(
            if reply.status == 422 {
                "post"
            } else {
                "delete"
            },
            reply,
        );
    }
}

#[tokio::test]
async fn person_grant_creation_validation_duplicate_and_relationship_owned_revoke() {
    let (fixture, boot) = boot_fixture().await;
    seed(&fixture).await;
    fixture.admin.execute_unprepared("INSERT INTO carer_relationships(id,household_id,carer_id,patient_id,relationship_type,active,created_at,updated_at) VALUES(86001,72001,73001,73002,'parent',true,now(),now()); INSERT INTO person_access_grants(id,household_id,household_membership_id,person_id,access_level,relationship_type,carer_relationship_id,created_at,updated_at) VALUES(78002,72001,74001,73002,'manage','parent',86001,now(),now())").await.unwrap();
    let server = server(&boot);
    let before = state(&fixture).await;
    let owned_state_sql = "SELECT m.permissions_version,g.revoked_at IS NOT NULL AS revoked FROM household_memberships m JOIN person_access_grants g ON g.household_membership_id=m.id WHERE g.id=78002";
    let owned_before = fixture
        .admin
        .query_one_raw(Statement::from_string(DbBackend::Postgres, owned_state_sql))
        .await
        .unwrap()
        .unwrap();
    let owned_before = (
        owned_before
            .try_get::<i32>("", "permissions_version")
            .unwrap(),
        owned_before.try_get::<bool>("", "revoked").unwrap(),
    );
    let mut replies = Vec::new();
    let malformed_response = server
        .method(
            Method::POST,
            "/api/v1/households/72001/admin/person_access_grants",
        )
        .add_header("authorization", "Bearer profile-session")
        .content_type("application/json")
        .text("{")
        .await;
    let malformed = collect(
        malformed_response.status_code().as_u16(),
        malformed_response.json(),
        malformed_response.as_bytes().is_empty(),
        malformed_response.headers(),
    );
    let after_malformed = state(&fixture).await;
    for input in [
        json!({"person_access_grant":{"revoked_at":"2000-01-01"}}),
        json!({"person_access_grant":{"household_membership_id":74002,"person_id":73002,"access_level":"owner","relationship_type":"parent"}}),
        json!({"person_access_grant":{"household_membership_id":74002,"person_id":73002,"access_level":"manage","relationship_type":"parent","expires_at":"not-rfc3339"}}),
        json!({"person_access_grant":{"household_membership_id":74002,"access_level":"manage","relationship_type":"parent"}}),
    ] {
        replies.push(
            send(
                &server,
                Method::POST,
                72001,
                None,
                Some("profile-session"),
                Some(&input),
                None,
            )
            .await,
        );
    }
    let owned = send(
        &server,
        Method::DELETE,
        72001,
        Some("78002"),
        Some("profile-session"),
        None,
        None,
    )
    .await;
    let after_owned = state(&fixture).await;
    let owned_after = fixture
        .admin
        .query_one_raw(Statement::from_string(DbBackend::Postgres, owned_state_sql))
        .await
        .unwrap()
        .unwrap();
    let owned_after = (
        owned_after
            .try_get::<i32>("", "permissions_version")
            .unwrap(),
        owned_after.try_get::<bool>("", "revoked").unwrap(),
    );
    let valid = send(&server,Method::POST,72001,None,Some("profile-session"),Some(&json!({"person_access_grant":{"household_membership_id":74002,"person_id":73002,"access_level":"manage","relationship_type":"parent","expires_at":"2099-01-01T00:00:00Z"}})),None).await;
    let duplicate = send(
        &server,
        Method::POST,
        72001,
        None,
        Some("profile-session"),
        Some(&body(73002)),
        None,
    )
    .await;
    let after = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(malformed.status, 400, "{}", malformed.body);
    assert_eq!(malformed.body["error"]["code"], "bad_request");
    assert_eq!(
        malformed.body["error"]["message"],
        "Invalid JSON request body"
    );
    assert!(malformed.body["data"].is_null());
    assert_eq!(after_malformed, before);
    assert_wire(&malformed);
    assert_schema("post", &malformed);
    assert_eq!(replies[0].status, 400);
    for reply in replies.iter().skip(1) {
        assert_eq!(reply.status, 422, "{}", reply.body);
    }
    assert_eq!(owned.status, 422, "{}", owned.body);
    assert_eq!(
        owned.body["error"]["code"], "validation_failed",
        "{}",
        owned.body
    );
    assert_eq!(
        owned.body["error"]["message"], "Validation failed",
        "{}",
        owned.body
    );
    assert_eq!(
        owned.body["error"]["errors"]["base"][0],
        "Relationship-owned grants must be revoked through their carer relationship",
        "{}",
        owned.body
    );
    assert!(owned.body["error"]["details"].is_null(), "{}", owned.body);
    assert_eq!(
        after_owned, before,
        "relationship-owned revoke changed target state"
    );
    assert_eq!(
        owned_after, owned_before,
        "relationship-owned revoke changed actor grant or version"
    );
    assert!(!owned_after.1);
    assert_eq!(valid.status, 201);
    assert_eq!(
        valid.body["data"]["expires_at"],
        "2099-01-01T00:00:00+00:00"
    );
    assert_eq!(duplicate.status, 422);
    assert_eq!(
        after["grants"].as_i64(),
        before["grants"].as_i64().map(|n| n + 1)
    );
    assert_eq!(
        after["version"].as_i64(),
        before["version"].as_i64().map(|n| n + 1)
    );
    for reply in &replies {
        assert_wire(reply);
        assert_schema("post", reply);
    }
    assert_wire(&owned);
    assert_schema("delete", &owned);
    for reply in [&valid, &duplicate] {
        assert_wire(reply);
        assert_schema("post", reply);
    }
}

async fn keyed(method: Method) {
    let (fixture, boot) = boot_fixture().await;
    seed(&fixture).await;
    let mobile = profile::mobile_token(&fixture).await;
    let server = server(&boot);
    let first_body = body(73002);
    let created = if method == Method::DELETE {
        Some(
            send(
                &server,
                Method::POST,
                72001,
                None,
                Some("profile-session"),
                Some(&first_body),
                None,
            )
            .await,
        )
    } else {
        None
    };
    let target = created
        .as_ref()
        .map(|reply| reply.body["data"]["id"].as_i64().unwrap().to_string());
    let original = target.as_deref();
    let input = (method == Method::POST).then_some(&first_body);
    let before = state(&fixture).await;
    let first = send(
        &server,
        method.clone(),
        72001,
        original,
        Some("profile-session"),
        input,
        Some("grant-saved-key"),
    )
    .await;
    let after_first = state(&fixture).await;
    let replay = send(
        &server,
        method.clone(),
        72001,
        original,
        Some("profile-app"),
        input,
        Some("grant-saved-key"),
    )
    .await;
    let mobile_replay = send(
        &server,
        method.clone(),
        72001,
        original,
        Some(mobile.as_str()),
        input,
        Some("grant-saved-key"),
    )
    .await;
    let changed = if method == Method::POST {
        send(
            &server,
            method.clone(),
            72001,
            None,
            Some("profile-session"),
            Some(&body(73001)),
            Some("grant-saved-key"),
        )
        .await
    } else {
        send(
            &server,
            method.clone(),
            72001,
            Some("78001"),
            Some("profile-session"),
            None,
            Some("grant-saved-key"),
        )
        .await
    };
    let after = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    let success = if method == Method::POST { 201 } else { 204 };
    assert_eq!(
        (
            first.status,
            replay.status,
            mobile_replay.status,
            changed.status
        ),
        (success, success, success, 409)
    );
    for reply in [&replay, &mobile_replay] {
        assert_eq!(reply.replayed.as_deref(), Some("true"));
    }
    assert_eq!(
        after_first["version"].as_i64(),
        before["version"].as_i64().map(|n| n + 1)
    );
    assert_eq!(
        after_first["keys"].as_i64(),
        before["keys"].as_i64().map(|n| n + 1)
    );
    assert_eq!(after, after_first);
    for reply in [&first, &replay, &mobile_replay, &changed] {
        assert_wire(reply);
        assert_schema(
            if method == Method::POST {
                "post"
            } else {
                "delete"
            },
            reply,
        );
    }
    if method == Method::POST {
        assert_eq!(first.body, replay.body);
        assert_eq!(first.body, mobile_replay.body);
    }
}

#[tokio::test]
async fn person_grant_create_keyed_replay_and_conflict_commit_once() {
    keyed(Method::POST).await;
}

#[tokio::test]
async fn person_grant_delete_keyed_replay_and_conflict_commit_once() {
    keyed(Method::DELETE).await;
}

#[tokio::test]
async fn person_grant_keyed_writes_observe_real_household_lock_overlap() {
    for method in [Method::POST, Method::DELETE] {
        let (fixture, boot) = boot_fixture().await;
        seed(&fixture).await;
        let server = server(&boot);
        let created = if method == Method::DELETE {
            Some(
                send(
                    &server,
                    Method::POST,
                    72001,
                    None,
                    Some("profile-session"),
                    Some(&body(73002)),
                    None,
                )
                .await,
            )
        } else {
            None
        };
        let target = created
            .as_ref()
            .map(|reply| reply.body["data"]["id"].as_i64().unwrap().to_string());
        let before = state(&fixture).await;
        let input = body(73002);
        let first = send(
            &server,
            method.clone(),
            72001,
            target.as_deref(),
            Some("profile-session"),
            (method == Method::POST).then_some(&input),
            Some("grant-overlap-key"),
        );
        let second = send(
            &server,
            method.clone(),
            72001,
            target.as_deref(),
            Some("profile-app"),
            (method == Method::POST).then_some(&input),
            Some("grant-overlap-key"),
        );
        let (a, b, observed) = super::overlap_requests_at_row_lock(
            &fixture,
            "SELECT id FROM households WHERE id=72001 FOR UPDATE",
            "%households%FOR UPDATE%",
            first,
            second,
        )
        .await;
        let after = state(&fixture).await;
        drop(server);
        close(fixture, boot).await;
        let expected = if method == Method::POST { 201 } else { 204 };
        assert!(
            observed,
            "two restricted household-lock waiters were not observed"
        );
        assert_eq!((a.status, b.status), (expected, expected));
        assert_eq!(
            [a.replayed.as_deref(), b.replayed.as_deref()]
                .iter()
                .filter(|marker| **marker == Some("true"))
                .count(),
            1
        );
        assert_eq!(
            after["version"].as_i64(),
            before["version"].as_i64().map(|n| n + 1)
        );
        assert_eq!(
            after["keys"].as_i64(),
            before["keys"].as_i64().map(|n| n + 1)
        );
        for reply in [&a, &b] {
            assert_wire(reply);
            assert_schema(
                if method == Method::POST {
                    "post"
                } else {
                    "delete"
                },
                reply,
            );
        }
    }
}

#[tokio::test]
async fn person_grant_routes_share_fixed_general_limit_without_domain_effects() {
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
    let mut replies = Vec::new();
    for method in [Method::GET, Method::POST, Method::DELETE] {
        replies.push((
            method.as_str().to_lowercase(),
            send(
                &server,
                method.clone(),
                72001,
                (method == Method::DELETE).then_some("78001"),
                Some("profile-session"),
                (method == Method::POST).then_some(&body(73002)),
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
async fn person_grant_audit_failure_hides_reads_and_rolls_back_writes() {
    let (fixture, boot) = boot_fixture().await;
    seed(&fixture).await;
    let server = server(&boot);
    let seeded = send(
        &server,
        Method::POST,
        72001,
        None,
        Some("profile-session"),
        Some(&body(73002)),
        None,
    )
    .await;
    assert_eq!(seeded.status, 201, "{}", seeded.body);
    let target_id = seeded.body["data"]["id"].as_i64().unwrap();
    let target = target_id.to_string();
    fixture.admin.execute_unprepared("CREATE FUNCTION reject_grant_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='api.request' AND NEW.metadata->>'controller'='api/v1/admin/person_access_grants' THEN RAISE EXCEPTION 'Synthetic grant audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_grant_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION reject_grant_audit()").await.unwrap();
    let before = state(&fixture).await;
    let audits_before = audit_count(&fixture).await;
    let revoked_before = fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT revoked_at IS NOT NULL AS revoked FROM person_access_grants WHERE id=$1",
            [target_id.into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<bool>("", "revoked")
        .unwrap();
    let mut replies = Vec::new();
    for method in [Method::GET, Method::POST, Method::DELETE] {
        replies.push((
            method.as_str().to_lowercase(),
            send(
                &server,
                method.clone(),
                72001,
                (method == Method::DELETE).then_some(target.as_str()),
                Some("profile-session"),
                (method == Method::POST).then_some(&body(73001)),
                Some("grant-fault-key"),
            )
            .await,
        ));
    }
    let after = state(&fixture).await;
    let audits_after = audit_count(&fixture).await;
    let revoked_after = fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT revoked_at IS NOT NULL AS revoked FROM person_access_grants WHERE id=$1",
            [target_id.into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<bool>("", "revoked")
        .unwrap();
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before, after);
    assert_eq!(audits_before, audits_after);
    assert!(!revoked_before);
    assert!(!revoked_after);
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

async fn target_stored(fixture: &Fixture, version: i32, ordinal: i64) -> (String, String) {
    let api = format!("grant-target-api-{ordinal}");
    let app = format!("grant-target-app-{ordinal}");
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO api_sessions(id,account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,created_at,updated_at) VALUES($1,71003,74002,$2,$3,$4,now()+interval '15 minutes',now()+interval '1 day',now(),now(),now())",
        [ordinal.into(),hex::encode(Sha256::digest(api.as_bytes())).into(),format!("target-refresh-{ordinal}").into(),version.into()]
    )).await.unwrap();
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO api_app_tokens(id,account_id,household_membership_id,name,token_digest,permissions_version,expires_at,last_used_at,created_at,updated_at) VALUES($1,71003,74002,'Grant target app',$2,$3,now()+interval '1 month',now(),now(),now())",
        [(ordinal+100).into(),hex::encode(Sha256::digest(app.as_bytes())).into(),version.into()]
    )).await.unwrap();
    (api, app)
}

async fn target_mobile(fixture: &Fixture) -> String {
    fixture.admin.execute_unprepared("INSERT INTO oauth_applications(id,account_id,client_id,client_kind,name,redirect_uri,scopes,token_endpoint_auth_method,created_at,updated_at) VALUES(75004,71003,'grant-target-native','mobile','Grant target native','io.damacus.medtracker:/oauth2redirect','medtracker offline_access','none',now(),now())").await.unwrap();
    fixture.admin.execute_unprepared("INSERT INTO oauth_grants(id,account_id,oauth_application_id,client_kind,code,code_challenge,code_challenge_method,redirect_uri,scopes,expires_in,authenticated_at,last_used_at,device_name,created_at,updated_at) VALUES(76010,71003,75004,'mobile','grant-target-code','E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM','S256','io.damacus.medtracker:/oauth2redirect','medtracker offline_access',timezone('UTC',clock_timestamp())+interval '5 minutes',now(),now(),'Grant target mobile',now(),now())").await.unwrap();
    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/token")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(
            serde_urlencoded::to_string([
                ("grant_type", "authorization_code"),
                ("client_id", "grant-target-native"),
                ("code", "grant-target-code"),
                ("redirect_uri", "io.damacus.medtracker:/oauth2redirect"),
                (
                    "code_verifier",
                    "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
                ),
            ])
            .unwrap(),
        ))
        .unwrap();
    med_tracker::models::identity::exchange(&fixture.runtime, request, None)
        .await
        .unwrap()["access_token"]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn target_person(server: &TestServer, bearer: &str) -> (u16, Value) {
    let response = server
        .get("/api/v1/households/72001/people/73001")
        .add_header("authorization", format!("Bearer {bearer}"))
        .await;
    (response.status_code().as_u16(), response.json())
}

#[tokio::test]
async fn person_grant_target_credentials_follow_create_expiry_and_revoke() {
    for ending in ["expiry", "revoke"] {
        let (fixture, boot) = boot_fixture().await;
        seed(&fixture).await;
        fixture.admin.execute_unprepared("INSERT INTO people(id,household_id,account_id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73007,72001,71003,'Grant target adult','1990-01-01',0,true,now(),now()); INSERT INTO users(id,person_id,email_address,password_digest,active,created_at,updated_at) VALUES(77007,73007,'grant-target@example.test',crypt('password',gen_salt('bf',4)),true,now(),now()); UPDATE household_memberships SET person_id=73007 WHERE id=74002; UPDATE people SET date_of_birth='1980-01-01' WHERE id=73001").await.unwrap();
        let (api_one, app_one) = target_stored(&fixture, 1, 88033).await;
        let mobile = target_mobile(&fixture).await;
        let server = server(&boot);
        let mut before = Vec::new();
        for bearer in [&api_one, &app_one, &mobile] {
            before.push(target_person(&server, bearer).await);
        }
        let grant = send(
            &server,
            Method::POST,
            72001,
            None,
            Some("profile-session"),
            Some(&body(73001)),
            None,
        )
        .await;
        let id = grant.body["data"]["id"].as_i64().unwrap();
        let stale_api = target_person(&server, &api_one).await;
        let stale_app = target_person(&server, &app_one).await;
        let (api_two, app_two) = target_stored(&fixture, 2, 88035).await;
        let mut visible = Vec::new();
        for bearer in [&api_two, &app_two, &mobile] {
            visible.push(target_person(&server, bearer).await);
        }
        let revoked = if ending == "revoke" {
            Some(
                send(
                    &server,
                    Method::DELETE,
                    72001,
                    Some(&id.to_string()),
                    Some("profile-session"),
                    None,
                    None,
                )
                .await,
            )
        } else {
            fixture.admin.execute_unprepared("UPDATE person_access_grants SET expires_at='2000-01-01 00:00:00' WHERE household_membership_id=74002 AND person_id=73001").await.unwrap();
            None
        };
        let old_after_revoke = if ending == "revoke" {
            Some(target_person(&server, &api_two).await)
        } else {
            None
        };
        let (last_api, last_app) = if ending == "revoke" {
            target_stored(&fixture, 3, 88037).await
        } else {
            (api_two, app_two)
        };
        let mut hidden = Vec::new();
        for bearer in [&last_api, &last_app, &mobile] {
            hidden.push(target_person(&server, bearer).await);
        }
        let admin = send(
            &server,
            Method::GET,
            72001,
            None,
            Some("profile-session"),
            None,
            None,
        )
        .await;
        let final_state = state(&fixture).await;
        drop(server);
        close(fixture, boot).await;
        assert_eq!(grant.status, 201, "{}", grant.body);
        assert!(
            before.iter().all(|(status, _)| *status == 404),
            "{ending} before: {before:?}"
        );
        assert!(before.iter().all(|(_, body)| body.get("data").is_none()));
        assert_eq!((stale_api.0, stale_app.0), (401, 401));
        assert!(stale_api.1.get("data").is_none());
        assert!(stale_app.1.get("data").is_none());
        assert!(
            visible.iter().all(|(status, _)| *status == 200),
            "{ending} visible: {visible:?}"
        );
        assert!(
            hidden.iter().all(|(status, _)| *status == 404),
            "{ending} hidden: {hidden:?}"
        );
        assert!(hidden.iter().all(|(_, body)| body.get("data").is_none()));
        assert_eq!(
            old_after_revoke.as_ref().map(|reply| reply.0),
            if ending == "revoke" { Some(401) } else { None }
        );
        assert!(
            old_after_revoke
                .as_ref()
                .is_none_or(|(_, body)| body.get("data").is_none())
        );
        assert_eq!(
            revoked.as_ref().map(|reply| reply.status),
            if ending == "revoke" { Some(204) } else { None }
        );
        assert_eq!(
            final_state["version"],
            if ending == "revoke" { 3 } else { 2 }
        );
        assert_eq!(admin.status, 200);
        assert!(
            admin.body["data"]
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["id"] == id)
        );
        assert_wire(&grant);
        assert_schema("post", &grant);
        assert_wire(&admin);
        assert_schema("get", &admin);
    }
}

#[tokio::test]
async fn person_grant_keyed_writes_recheck_actor_withdrawn_while_waiting() {
    for method in [Method::POST, Method::DELETE] {
        let (fixture, boot) = boot_fixture().await;
        seed(&fixture).await;
        fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,password_hash,status,created_at,updated_at) VALUES(71004,'grant-second-owner@example.test',crypt('password',gen_salt('bf',4)),2,now(),now()); INSERT INTO people(id,household_id,account_id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73008,72001,71004,'Grant second owner','1990-01-01',0,true,now(),now()); INSERT INTO users(id,person_id,email_address,password_digest,active,created_at,updated_at) VALUES(77008,73008,'grant-second-owner@example.test',crypt('password',gen_salt('bf',4)),true,now(),now()); INSERT INTO household_memberships(id,account_id,household_id,person_id,role,status,permissions_version,joined_at,created_at,updated_at) VALUES(74008,71004,72001,73008,'owner','active',1,now(),now(),now())").await.unwrap();
        let server = server(&boot);
        let created = if method == Method::DELETE {
            Some(
                send(
                    &server,
                    Method::POST,
                    72001,
                    None,
                    Some("profile-session"),
                    Some(&body(73002)),
                    None,
                )
                .await,
            )
        } else {
            None
        };
        let target = created
            .as_ref()
            .map(|reply| reply.body["data"]["id"].as_i64().unwrap().to_string());
        let before = state(&fixture).await;
        let audits_before = audit_count(&fixture).await;
        let blocker = fixture.admin.begin().await.unwrap();
        let pid = blocker
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT pg_backend_pid() AS pid",
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get::<i32>("", "pid")
            .unwrap();
        blocker
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT id FROM households WHERE id=72001 FOR UPDATE",
            ))
            .await
            .unwrap()
            .unwrap();
        let role = url::Url::parse(&fixture.runtime_uri)
            .unwrap()
            .username()
            .to_owned();
        let input = body(73002);
        let request = send(
            &server,
            method.clone(),
            72001,
            target.as_deref(),
            Some("profile-session"),
            (method == Method::POST).then_some(&input),
            Some("grant-withdrawn-key"),
        );
        let release = async {
            let observed = tokio::time::timeout(std::time::Duration::from_secs(10),async {
                loop {
                    let row = fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                        "SELECT count(DISTINCT pid) AS waiting FROM pg_stat_activity WHERE datname=current_database() AND usename=$1 AND wait_event_type='Lock' AND query LIKE $3 AND $2::int=ANY(pg_blocking_pids(pid))",
                        [role.clone().into(),pid.into(),"%households%FOR UPDATE%".into()],
                    )).await.unwrap().unwrap();
                    if row.try_get::<i64>("","waiting").unwrap()==1 { break true; }
                    tokio::task::yield_now().await;
                }
            }).await.unwrap_or(false);
            if observed {
                blocker.execute_unprepared("UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74001").await.unwrap();
            }
            blocker.commit().await.unwrap();
            observed
        };
        let (reply, observed) = tokio::join!(request, release);
        let retry = send(
            &server,
            method.clone(),
            72001,
            target.as_deref(),
            Some("profile-session"),
            (method == Method::POST).then_some(&input),
            Some("grant-withdrawn-key"),
        )
        .await;
        let after = state(&fixture).await;
        let audits_after = audit_count(&fixture).await;
        let owners = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT count(*) AS active FROM household_memberships WHERE household_id=72001 AND role='owner' AND status='active' AND revoked_at IS NULL"
        )).await.unwrap().unwrap().try_get::<i64>("","active").unwrap();
        drop(server);
        close(fixture, boot).await;
        assert!(observed, "runtime request never reached held household row");
        assert_eq!((reply.status, retry.status), (403, 401));
        assert_eq!(retry.challenge.as_deref(), Some("Bearer"));
        assert_eq!(after, before, "denied write changed grant/version/key/sync");
        assert_eq!(audits_after, audits_before + 1);
        assert_eq!(owners, 1);
        for denied in [&reply, &retry] {
            assert!(denied.body.get("data").is_none());
            assert_wire(denied);
            assert_schema(
                if method == Method::POST {
                    "post"
                } else {
                    "delete"
                },
                denied,
            );
        }
    }
}
