use super::*;
use axum::http::Method;

const PATH: &str = "/households/{household_id}/admin/settings";

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
}

fn header(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn collect(status: u16, body: Value, headers: &axum::http::HeaderMap) -> Reply {
    Reply {
        status,
        body,
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
    bearer: Option<&str>,
    body: Option<&Value>,
    key: Option<&str>,
) -> Reply {
    let mut request = server.method(
        method,
        &format!("/api/v1/households/{household}/admin/settings"),
    );
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
    collect(
        response.status_code().as_u16(),
        response.json(),
        response.headers(),
    )
}

async fn malformed(server: &TestServer, method: Method) -> Reply {
    let response = server
        .method(method, "/api/v1/households/72001/admin/settings")
        .add_header("authorization", "Bearer profile-session")
        .text("{")
        .content_type("application/json")
        .await;
    collect(
        response.status_code().as_u16(),
        response.json(),
        response.headers(),
    )
}

fn assert_schema(method: &str, response: &Reply) {
    let document = contract::contract();
    let raw = &document["paths"][PATH][method]["responses"][response.status.to_string()];
    let described = if raw.get("$ref").is_some() {
        contract::resolve(document, raw)
    } else {
        raw
    };
    assert!(
        described.is_object(),
        "{method} settings {} undocumented: {}",
        response.status,
        response.body
    );
    contract::assert_value(
        document,
        &described["content"]["application/json"]["schema"],
        &response.body,
        &format!("{method} settings {}", response.status),
    );
}

fn assert_wire(response: &Reply) {
    assert_eq!(response.content_type.as_deref(), Some("application/json"));
    let request_id = response.request_id.as_deref().expect("request ID");
    assert!(!request_id.is_empty());
    if response.status != 429 && response.body.get("error").is_some() {
        assert_eq!(response.body["error"]["request_id"], request_id);
    }
}

async fn state(fixture: &Fixture) -> Value {
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM versions WHERE item_type='Household' AND event='update') AS versions,(SELECT count(*) FROM security_audit_events WHERE event_type='api/admin/household_settings/updated') AS audits,(SELECT count(*) FROM api_idempotency_keys WHERE request_path LIKE '%/admin/settings') AS keys,(SELECT count(*) FROM api_change_events WHERE record_type='Household') AS changes,(SELECT name FROM households WHERE id=72001) AS name,(SELECT timezone FROM households WHERE id=72001) AS timezone,(SELECT subscription_plan FROM households WHERE id=72001) AS plan,(SELECT slug FROM households WHERE id=72001) AS slug"
    )).await.unwrap().unwrap();
    json!({"versions":row.try_get::<i64>("","versions").unwrap(),"audits":row.try_get::<i64>("","audits").unwrap(),"keys":row.try_get::<i64>("","keys").unwrap(),"changes":row.try_get::<i64>("","changes").unwrap(),"name":row.try_get::<String>("","name").unwrap(),"timezone":row.try_get::<String>("","timezone").unwrap(),"plan":row.try_get::<String>("","plan").unwrap(),"slug":row.try_get::<String>("","slug").unwrap()})
}

#[tokio::test]
async fn settings_reads_scope_three_credential_kinds_and_validate_responses() {
    let (fixture, boot) = boot_fixture().await;
    profile::seed_credentials(&fixture).await;
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
                send(&server, Method::GET, household, Some(token), None, None).await,
            ));
        }
    }
    let anonymous = send(&server, Method::GET, 72001, None, None, None).await;
    let audits = fixture.admin.query_all_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT request_id,actor_account_id,actor_membership_id,metadata,audit_context FROM security_audit_events WHERE event_type='api.request' AND metadata->>'controller'='api/v1/admin/settings' AND metadata->>'action'='show' AND metadata->>'status'='200'"
    )).await.unwrap();
    let audits: Vec<Value> = audits
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
    let after = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(audits.len(), 3);
    assert_eq!(before, after);
    assert_eq!(anonymous.status, 401);
    assert_wire(&anonymous);
    assert_schema("get", &anonymous);
    for (name, household, expected, reply) in replies {
        assert_eq!(reply.status, expected, "{name} {household}: {}", reply.body);
        assert_wire(&reply);
        assert_schema("get", &reply);
        if expected == 200 {
            assert_eq!(reply.body["data"]["id"], 72001);
            assert_eq!(reply.body["data"]["slug"], "persistence-fixture");
            assert_eq!(reply.body["data"]["subscription_plan"], "free");
            let audit = audits
                .iter()
                .find(|row| row["request_id"] == reply.request_id.as_deref().unwrap())
                .unwrap();
            let (method, reference) = match name {
                "api" => ("api_session", "api_session:88031"),
                "app" => ("api_app_token", "api_app_token:88032"),
                _ => ("oauth", "oauth_grant:76001"),
            };
            assert_eq!(
                (audit["account"].as_i64(), audit["membership"].as_i64()),
                (Some(71001), Some(74001))
            );
            assert_eq!(audit["metadata"]["http_method"], "GET");
            assert_eq!(audit["metadata"]["outcome"], "success");
            assert_eq!(audit["context"]["actor_user_id"], 77001);
            assert_eq!(
                audit["context"]["request_id"],
                reply.request_id.as_deref().unwrap()
            );
            assert_eq!(audit["context"]["authentication_method"], method);
            assert_eq!(audit["context"]["session_reference"], reference);
            assert_eq!(audit["context"]["policy_class"], "HouseholdPolicy");
            assert_eq!(audit["context"]["policy_query"], "show?");
        } else {
            assert!(reply.body.get("data").is_none());
        }
    }
}

#[tokio::test]
async fn settings_methods_reject_invalid_revoked_and_wrong_namespace_credentials() {
    let (fixture, boot) = boot_fixture().await;
    profile::seed_credentials(&fixture).await;
    let mobile = profile::mobile_token(&fixture).await;
    profile::seed_disallowed_grants(&fixture, "smart-settings").await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let body = json!({"household":{"name":"Denied settings"}});
    let mut replies = Vec::new();
    for method in [Method::GET, Method::PATCH, Method::PUT] {
        for (name, token, expected) in [
            ("invalid", "invalid-settings", 401),
            ("smart", "smart-settings", 401),
            ("wrong-scope-mobile", mobile.as_str(), 403),
        ] {
            replies.push((
                method.as_str().to_lowercase(),
                name,
                expected,
                send(
                    &server,
                    method.clone(),
                    72001,
                    Some(token),
                    (method != Method::GET).then_some(&body),
                    None,
                )
                .await,
            ));
        }
    }
    fixture.admin.execute_unprepared("UPDATE api_sessions SET revoked_at=now() WHERE id=88031; UPDATE api_app_tokens SET revoked_at=now() WHERE id=88032; UPDATE oauth_grants SET revoked_at=now() WHERE id=76001").await.unwrap();
    for method in [Method::GET, Method::PATCH, Method::PUT] {
        for token in ["profile-session", "profile-app", mobile.as_str()] {
            replies.push((
                method.as_str().to_lowercase(),
                "revoked",
                401,
                send(
                    &server,
                    method.clone(),
                    72001,
                    Some(token),
                    (method != Method::GET).then_some(&body),
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
    for (method, name, expected, reply) in replies {
        assert_eq!(reply.status, expected, "{method} {name}: {}", reply.body);
        assert_wire(&reply);
        assert_schema(&method, &reply);
        assert!(reply.body.get("data").is_none());
        assert_eq!(
            reply.challenge.as_deref(),
            Some(if name == "wrong-scope-mobile" {
                "Bearer error=\"insufficient_scope\" scope=\"medtracker\""
            } else {
                "Bearer"
            })
        );
    }
}

#[tokio::test]
async fn settings_patch_and_put_distinguish_inaccessible_and_absent_households_without_writes() {
    let (fixture, boot) = boot_fixture().await;
    profile::seed_credentials(&fixture).await;
    let mobile = profile::mobile_token(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let body = json!({"household":{"name":"Must not change"}});
    let mut replies = Vec::new();
    for method in [Method::PATCH, Method::PUT] {
        for token in ["profile-session", "profile-app", mobile.as_str()] {
            for (household, expected) in [(72004, 403), (99999, 404)] {
                replies.push((
                    method.as_str().to_lowercase(),
                    household,
                    expected,
                    send(
                        &server,
                        method.clone(),
                        household,
                        Some(token),
                        Some(&body),
                        None,
                    )
                    .await,
                ));
            }
        }
    }
    let after = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before, after);
    for (method, household, expected, reply) in replies {
        assert_eq!(
            reply.status, expected,
            "{method} {household}: {}",
            reply.body
        );
        assert_wire(&reply);
        assert_schema(&method, &reply);
        assert!(reply.body.get("data").is_none());
    }
}

#[tokio::test]
async fn settings_methods_reject_stale_membership_credentials_before_changes() {
    let (fixture, boot) = boot_fixture().await;
    profile::seed_credentials(&fixture).await;
    let mobile = profile::mobile_token(&fixture).await;
    fixture.admin.execute_unprepared("UPDATE household_memberships SET permissions_version=permissions_version+1 WHERE id=74001").await.unwrap();
    let server = server(&boot);
    let before = state(&fixture).await;
    let body = json!({"household":{"name":"Stale settings"}});
    let mut replies = Vec::new();
    for method in [Method::GET, Method::PATCH, Method::PUT] {
        for token in ["profile-session", "profile-app"] {
            replies.push((
                method.as_str().to_lowercase(),
                401,
                send(
                    &server,
                    method.clone(),
                    72001,
                    Some(token),
                    (method != Method::GET).then_some(&body),
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
    for method in [Method::GET, Method::PATCH, Method::PUT] {
        replies.push((
            method.as_str().to_lowercase(),
            403,
            send(
                &server,
                method.clone(),
                72001,
                Some(&mobile),
                (method != Method::GET).then_some(&body),
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
        if expected == 401 {
            assert_eq!(reply.challenge.as_deref(), Some("Bearer"));
        }
        assert!(reply.body.get("data").is_none());
    }
}

#[tokio::test]
async fn settings_patch_and_put_save_allowed_fields_without_replacing_slug_or_omitted_fields() {
    let (fixture, boot) = boot_fixture().await;
    profile::seed_credentials(&fixture).await;
    let mobile = profile::mobile_token(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let mut replies = Vec::new();
    for (method, token, body, field, old, new, auth, reference) in [
        (
            Method::PATCH,
            "profile-session",
            json!({"household":{"name":"Settings one"}}),
            "name",
            "Synthetic household",
            "Settings one",
            "api_session",
            "api_session:88031",
        ),
        (
            Method::PUT,
            "profile-app",
            json!({"household":{"timezone":"UTC"}}),
            "timezone",
            "Europe/London",
            "UTC",
            "api_app_token",
            "api_app_token:88032",
        ),
        (
            Method::PATCH,
            mobile.as_str(),
            json!({"household":{"subscription_plan":"family_plus"}}),
            "subscription_plan",
            "free",
            "family_plus",
            "oauth",
            "oauth_grant:76001",
        ),
        (
            Method::PUT,
            "profile-session",
            json!({"household":{"name":"Settings two"}}),
            "name",
            "Settings one",
            "Settings two",
            "api_session",
            "api_session:88031",
        ),
        (
            Method::PATCH,
            "profile-app",
            json!({"household":{"name":"Settings three"}}),
            "name",
            "Settings two",
            "Settings three",
            "api_app_token",
            "api_app_token:88032",
        ),
        (
            Method::PUT,
            mobile.as_str(),
            json!({"household":{"name":"Settings four"}}),
            "name",
            "Settings three",
            "Settings four",
            "oauth",
            "oauth_grant:76001",
        ),
    ] {
        replies.push((
            method.as_str().to_lowercase(),
            field,
            old,
            new,
            auth,
            reference,
            send(&server, method, 72001, Some(token), Some(&body), None).await,
        ));
    }
    let after = state(&fixture).await;
    let rows = fixture.admin.query_all_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT v.request_id,v.actor_membership_id,v.whodunnit,v.object::jsonb AS before,v.object_changes::jsonb AS changes,v.audit_context AS version_context,a.actor_account_id,a.actor_membership_id AS audit_membership,a.metadata AS update_metadata,a.audit_context AS update_context,r.metadata AS request_metadata,r.audit_context AS request_context FROM versions v JOIN security_audit_events a ON a.request_id=v.request_id AND a.event_type='api/admin/household_settings/updated' JOIN security_audit_events r ON r.request_id=v.request_id AND r.event_type='api.request' WHERE v.item_type='Household' AND v.item_id=72001 AND v.event='update' ORDER BY v.id"
    )).await.unwrap();
    let proofs: Vec<Value> = rows
        .iter()
        .map(|row| {
            json!({
                "request_id":row.try_get::<String>("","request_id").unwrap(),
                "membership":row.try_get::<i64>("","actor_membership_id").unwrap(),
                "user":row.try_get::<String>("","whodunnit").unwrap(),
                "before":row.try_get::<Value>("","before").unwrap(),
                "changes":row.try_get::<Value>("","changes").unwrap(),
                "version_context":row.try_get::<Value>("","version_context").unwrap(),
                "account":row.try_get::<i64>("","actor_account_id").unwrap(),
                "audit_membership":row.try_get::<i64>("","audit_membership").unwrap(),
                "update_metadata":row.try_get::<Value>("","update_metadata").unwrap(),
                "update_context":row.try_get::<Value>("","update_context").unwrap(),
                "request_metadata":row.try_get::<Value>("","request_metadata").unwrap(),
                "request_context":row.try_get::<Value>("","request_context").unwrap(),
            })
        })
        .collect();
    drop(server);
    close(fixture, boot).await;
    assert_eq!(proofs.len(), 6);
    for (method, field, old, new, auth, reference, reply) in replies {
        assert_eq!(reply.status, 200, "{method}: {}", reply.body);
        assert_wire(&reply);
        assert_schema(&method, &reply);
        let request_id = reply.request_id.as_deref().unwrap();
        let proof = proofs
            .iter()
            .find(|row| row["request_id"] == request_id)
            .unwrap();
        assert_eq!(reply.body["data"][field], new);
        assert_eq!(proof["before"][field], old);
        assert_eq!(proof["changes"][field], json!([old, new]));
        assert!(proof["changes"].get("slug").is_none());
        assert_eq!(proof["membership"], 74001);
        assert_eq!(proof["audit_membership"], 74001);
        assert_eq!(proof["account"], 71001);
        assert_eq!(proof["user"], "77001");
        assert_eq!(
            proof["update_metadata"],
            json!({"target_type":"Household","target_id":72001,"outcome":"success"})
        );
        assert_eq!(
            proof["request_metadata"]["http_method"],
            method.to_uppercase()
        );
        assert_eq!(proof["request_metadata"]["status"], 200);
        for context in [
            &proof["version_context"],
            &proof["update_context"],
            &proof["request_context"],
        ] {
            assert_eq!(context["request_id"], request_id);
            assert_eq!(context["actor_account_id"], 71001);
            assert_eq!(context["actor_membership_id"], 74001);
            assert_eq!(context["actor_user_id"], 77001);
            assert_eq!(context["authentication_method"], auth);
            assert_eq!(context["session_reference"], reference);
        }
    }
    assert_eq!(after["name"], "Settings four");
    assert_eq!(after["timezone"], "UTC");
    assert_eq!(after["plan"], "family_plus");
    assert_eq!(after["slug"], before["slug"]);
    assert_eq!(after["changes"], before["changes"]);
    assert_eq!(
        after["versions"].as_i64().unwrap(),
        before["versions"].as_i64().unwrap() + 6
    );
    assert_eq!(
        after["audits"].as_i64().unwrap(),
        before["audits"].as_i64().unwrap() + 6
    );
}

#[tokio::test]
async fn settings_patch_and_put_validate_bodies_and_leave_domain_state_unchanged() {
    let (fixture, boot) = boot_fixture().await;
    profile::seed_credentials(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let mut replies = Vec::new();
    for method in [Method::PATCH, Method::PUT] {
        replies.push((
            method.as_str().to_lowercase(),
            400,
            malformed(&server, method.clone()).await,
        ));
        for (expected, body) in [
            (400, json!({"household":{"slug":"must-not-change"}})),
            (400, json!({"household":{}})),
            (400, json!({"household":7})),
            (422, json!({"household":{"name":""}})),
            (422, json!({"household":{"name":7}})),
            (422, json!({"household":{"timezone":"  "}})),
            (422, json!({"household":{"timezone":false}})),
            (422, json!({"household":{"subscription_plan":"enterprise"}})),
            (422, json!({"household":{"subscription_plan":7}})),
        ] {
            replies.push((
                method.as_str().to_lowercase(),
                expected,
                send(
                    &server,
                    method.clone(),
                    72001,
                    Some("profile-session"),
                    Some(&body),
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
    for (method, expected, reply) in replies {
        assert_eq!(reply.status, expected, "{method}: {}", reply.body);
        assert_wire(&reply);
        assert_schema(&method, &reply);
        assert!(reply.body.get("data").is_none());
    }
}

async fn keyed_write(method: Method) {
    let (fixture, boot) = boot_fixture().await;
    profile::seed_credentials(&fixture).await;
    let mobile = profile::mobile_token(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let method_name = method.as_str().to_lowercase();
    let key = format!("settings-{method}-key");
    let body = json!({"household":{"name":"Keyed settings"}});
    let changed = json!({"household":{"name":"Changed key body"}});
    let first = send(
        &server,
        method.clone(),
        72001,
        Some("profile-session"),
        Some(&body),
        Some(&key),
    )
    .await;
    let after_first = state(&fixture).await;
    let replay = send(
        &server,
        method.clone(),
        72001,
        Some("profile-app"),
        Some(&body),
        Some(&key),
    )
    .await;
    let after_replay = state(&fixture).await;
    let mobile_replay = send(
        &server,
        method.clone(),
        72001,
        Some(&mobile),
        Some(&body),
        Some(&key),
    )
    .await;
    let after_mobile_replay = state(&fixture).await;
    let conflict = send(
        &server,
        method.clone(),
        72001,
        Some("profile-app"),
        Some(&changed),
        Some(&key),
    )
    .await;
    let after_conflict = state(&fixture).await;
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let mut denied = Vec::new();
    for (name, token) in [
        ("api", "profile-session"),
        ("app", "profile-app"),
        ("mobile", mobile.as_str()),
    ] {
        denied.push((
            "demoted",
            name,
            403,
            send(
                &server,
                method.clone(),
                72001,
                Some(token),
                Some(&body),
                Some(&key),
            )
            .await,
        ));
    }
    let after_demotion = state(&fixture).await;
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET revoked_at=now() WHERE id=74001")
        .await
        .unwrap();
    for (name, token, expected) in [
        ("api", "profile-session", 401),
        ("app", "profile-app", 401),
        ("mobile", mobile.as_str(), 403),
    ] {
        denied.push((
            "withdrawn",
            name,
            expected,
            send(
                &server,
                method.clone(),
                72001,
                Some(token),
                Some(&body),
                Some(&key),
            )
            .await,
        ));
    }
    let after_withdrawal = state(&fixture).await;
    fixture.admin.execute_unprepared("UPDATE api_sessions SET revoked_at=now() WHERE id=88031; UPDATE api_app_tokens SET revoked_at=now() WHERE id=88032; UPDATE oauth_grants SET revoked_at=now() WHERE id=76001").await.unwrap();
    for (name, token) in [
        ("api", "profile-session"),
        ("app", "profile-app"),
        ("mobile", mobile.as_str()),
    ] {
        denied.push((
            "revoked",
            name,
            401,
            send(
                &server,
                method.clone(),
                72001,
                Some(token),
                Some(&body),
                Some(&key),
            )
            .await,
        ));
    }
    let after_revocation = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(
        (
            first.status,
            replay.status,
            mobile_replay.status,
            conflict.status
        ),
        (200, 200, 200, 409)
    );
    assert_eq!(replay.replayed.as_deref(), Some("true"));
    assert_eq!(mobile_replay.replayed.as_deref(), Some("true"));
    assert_eq!(replay.body, first.body);
    assert_eq!(mobile_replay.body, first.body);
    assert_eq!(
        after_first["versions"].as_i64().unwrap(),
        before["versions"].as_i64().unwrap() + 1
    );
    assert_eq!(after_replay, after_first);
    assert_eq!(after_mobile_replay, after_first);
    assert_eq!(after_conflict, after_first);
    assert_eq!(after_demotion, after_first);
    assert_eq!(after_withdrawal, after_first);
    assert_eq!(after_revocation, after_first);
    for reply in [&first, &replay, &mobile_replay, &conflict] {
        assert_wire(reply);
        assert_schema(&method_name, reply);
    }
    assert_eq!(conflict.body["error"]["code"], "idempotency_key_reused");
    for (phase, name, expected, reply) in denied {
        assert_eq!(reply.status, expected, "{phase} {name}: {}", reply.body);
        assert_wire(&reply);
        assert_schema(&method_name, &reply);
        assert!(reply.body.get("data").is_none());
        assert!(reply.replayed.is_none());
        if expected == 401 {
            assert_eq!(reply.challenge.as_deref(), Some("Bearer"));
        }
    }
}

#[tokio::test]
async fn settings_patch_replays_same_key_and_rejects_changed_body() {
    keyed_write(Method::PATCH).await;
}

#[tokio::test]
async fn settings_put_replays_same_key_and_rejects_changed_body() {
    keyed_write(Method::PUT).await;
}

async fn concurrent_keyed_write(method: Method) {
    use sea_orm::TransactionTrait;

    let (fixture, boot) = boot_fixture().await;
    profile::seed_credentials(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let method_name = method.as_str().to_lowercase();
    let body = json!({"household":{"name":"Concurrent settings"}});
    let changed = json!({"household":{"name":"Changed concurrent body"}});
    let blocker = fixture.admin.begin().await.unwrap();
    let blocker_pid: i32 = blocker
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_backend_pid() AS pid",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    blocker
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id FROM households WHERE id=72001 FOR UPDATE",
        ))
        .await
        .unwrap();
    let runtime_role = url::Url::parse(&fixture.runtime_uri)
        .unwrap()
        .username()
        .to_owned();
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
    let first = async {
        barrier.wait().await;
        send(
            &server,
            method.clone(),
            72001,
            Some("profile-app"),
            Some(&body),
            Some("settings-concurrent"),
        )
        .await
    };
    let second = async {
        barrier.wait().await;
        send(
            &server,
            method.clone(),
            72001,
            Some("profile-session"),
            Some(&body),
            Some("settings-concurrent"),
        )
        .await
    };
    let release = async {
        let observed = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let row = fixture.admin.query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "SELECT count(DISTINCT pid) AS waiting,count(DISTINCT pid) FILTER (WHERE $2::int=ANY(pg_blocking_pids(pid))) AS held_waiters FROM pg_stat_activity WHERE datname=current_database() AND usename=$1 AND wait_event_type='Lock' AND query LIKE '%households%' AND query LIKE '%FOR UPDATE%' AND cardinality(pg_blocking_pids(pid))>0",
                    [runtime_role.clone().into(), blocker_pid.into()],
                )).await.unwrap().unwrap();
                if row.try_get::<i64>("", "waiting").unwrap() == 2
                    && row.try_get::<i64>("", "held_waiters").unwrap() >= 1
                {
                    break true;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap_or(false);
        blocker.commit().await.unwrap();
        observed
    };
    let (first, second, observed) = tokio::join!(first, second, release);
    let changed_reply = send(
        &server,
        method,
        72001,
        Some("profile-session"),
        Some(&changed),
        Some("settings-concurrent"),
    )
    .await;
    let after = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert!(
        observed,
        "both runtime settings writes must wait on the same household row"
    );
    assert_eq!(
        (first.status, second.status, changed_reply.status),
        (200, 200, 409)
    );
    assert_eq!(
        usize::from(first.replayed.is_some()) + usize::from(second.replayed.is_some()),
        1
    );
    assert_eq!(first.body, second.body);
    assert_eq!(
        after["versions"].as_i64().unwrap(),
        before["versions"].as_i64().unwrap() + 1
    );
    assert_eq!(
        after["audits"].as_i64().unwrap(),
        before["audits"].as_i64().unwrap() + 1
    );
    for reply in [&first, &second, &changed_reply] {
        assert_wire(reply);
        assert_schema(&method_name, reply);
    }
}

#[tokio::test]
async fn settings_patch_concurrent_keyed_requests_commit_once() {
    concurrent_keyed_write(Method::PATCH).await;
}

#[tokio::test]
async fn settings_put_concurrent_keyed_requests_commit_once() {
    concurrent_keyed_write(Method::PUT).await;
}

#[tokio::test]
async fn settings_current_manage_permission_blocks_writes_without_domain_effects() {
    let (fixture, boot) = boot_fixture().await;
    profile::seed_credentials(&fixture).await;
    let mobile = profile::mobile_token(&fixture).await;
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await
        .unwrap();
    let server = server(&boot);
    let before = state(&fixture).await;
    let body = json!({"household":{"name":"Denied settings"}});
    let mut replies = Vec::new();
    for method in [Method::GET, Method::PATCH, Method::PUT] {
        for token in ["profile-session", "profile-app", mobile.as_str()] {
            replies.push((
                method.as_str().to_lowercase(),
                send(
                    &server,
                    method.clone(),
                    72001,
                    Some(token),
                    (method != Method::GET).then_some(&body),
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
    for (method, reply) in replies {
        assert_eq!(reply.status, 403, "{method}: {}", reply.body);
        assert_wire(&reply);
        assert_schema(&method, &reply);
        assert!(reply.body.get("data").is_none());
    }
}

#[tokio::test]
async fn settings_methods_share_fixed_general_limit_without_domain_effects() {
    let (fixture, boot) = boot_rate_fixture(&[]).await;
    profile::seed_credentials(&fixture).await;
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
    let body = json!({"household":{"name":"Rate-denied settings"}});
    let mut replies = Vec::new();
    for method in [Method::GET, Method::PATCH, Method::PUT] {
        replies.push((
            method.as_str().to_lowercase(),
            send(
                &server,
                method.clone(),
                72001,
                Some("profile-session"),
                (method != Method::GET).then_some(&body),
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
        assert!(reply.body.get("data").is_none());
    }
}

#[tokio::test]
async fn settings_audit_failure_hides_read_and_rolls_back_both_write_methods() {
    let (fixture, boot) = boot_fixture().await;
    profile::seed_credentials(&fixture).await;
    fixture.admin.execute_unprepared("CREATE FUNCTION reject_settings_request_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='api.request' AND NEW.metadata->>'controller'='api/v1/admin/settings' THEN RAISE EXCEPTION 'Synthetic settings request audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_settings_request_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION reject_settings_request_audit()").await.unwrap();
    let server = server(&boot);
    let before = state(&fixture).await;
    let body = json!({"household":{"name":"Must roll back settings"}});
    let mut replies = Vec::new();
    for method in [Method::GET, Method::PATCH, Method::PUT] {
        replies.push((
            method.as_str().to_lowercase(),
            send(
                &server,
                method.clone(),
                72001,
                Some("profile-session"),
                (method != Method::GET).then_some(&body),
                None,
            )
            .await,
        ));
    }
    let after = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before, after);
    for (method, reply) in &replies {
        assert_eq!(reply.status, 500, "{method}: {}", reply.body);
        assert_wire(reply);
        assert_eq!(reply.body["error"]["code"], "internal_error");
        assert!(reply.body.get("data").is_none());
    }
    for (method, reply) in replies {
        assert_schema(&method, &reply);
    }
}
