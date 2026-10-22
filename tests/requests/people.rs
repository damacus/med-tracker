use super::*;
use axum::http::Method;
use base64::Engine;

struct Reply {
    status: u16,
    body: Value,
    request_id: Option<String>,
    content_type: Option<String>,
    etag: Option<String>,
    replayed: Option<String>,
    retry_after: Option<String>,
    challenge: Option<String>,
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

fn reply(status: u16, body: Value, headers: &axum::http::HeaderMap) -> Reply {
    Reply {
        status,
        body,
        request_id: header(headers, "x-request-id"),
        content_type: header(headers, "content-type"),
        etag: header(headers, "etag"),
        replayed: header(headers, "idempotency-replayed"),
        retry_after: header(headers, "retry-after"),
        challenge: header(headers, "www-authenticate"),
        limit: header(headers, "ratelimit-limit"),
        remaining: header(headers, "ratelimit-remaining"),
        reset: header(headers, "ratelimit-reset"),
        cache_control: header(headers, "cache-control"),
    }
}

async fn send(
    server: &TestServer,
    method: Method,
    path: &str,
    bearer: Option<&str>,
    body: Option<&Value>,
    key: Option<&str>,
) -> Reply {
    let mut request = server.method(method, path);
    if let Some(token) = bearer {
        request = request.add_header("authorization", format!("Bearer {token}"));
    }
    if let Some(key) = key {
        request = request.add_header("idempotency-key", key);
    }
    if let Some(body) = body {
        request = request.json(body);
    }
    let response = request.await;
    reply(
        response.status_code().as_u16(),
        response.json(),
        response.headers(),
    )
}

async fn send_malformed(server: &TestServer, method: Method, path: &str) -> Reply {
    let response = server
        .method(method, path)
        .add_header("authorization", "Bearer people-session")
        .text("{")
        .content_type("application/json")
        .await;
    reply(
        response.status_code().as_u16(),
        response.json(),
        response.headers(),
    )
}

fn assert_schema(method: &str, path: &str, reply: &Reply) {
    let document = contract::contract();
    let response = &document["paths"][path][method]["responses"][reply.status.to_string()];
    let response = if response.get("$ref").is_some() {
        contract::resolve(document, response)
    } else {
        response
    };
    assert!(
        response.is_object(),
        "{method} {path} {} undocumented",
        reply.status
    );
    contract::assert_value(
        document,
        &response["content"]["application/json"]["schema"],
        &reply.body,
        &format!("{method} {path} {}", reply.status),
    );
}

fn assert_wire(reply: &Reply) {
    assert_eq!(reply.content_type.as_deref(), Some("application/json"));
    let request_id = reply.request_id.as_ref().expect("request ID");
    assert!(!request_id.is_empty());
    if reply.status != 429 && reply.body.get("error").is_some() {
        assert_eq!(reply.body["error"]["request_id"], request_id.as_str());
    }
}

async fn mobile_token(fixture: &Fixture) -> String {
    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/token")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::from(
            serde_urlencoded::to_string([
                ("grant_type", "authorization_code"),
                ("client_id", "native"),
                ("code", "synthetic-legacy-code"),
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

async fn seed_credentials_and_foreign_household(fixture: &Fixture) {
    fixture.admin.execute_unprepared("INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73005,'Foreign member',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,created_at,updated_at) VALUES(71002,73005,'foreign-people@example.test',now(),now()); INSERT INTO households(id,created_by_account_id,name,slug,timezone,status,lifecycle_state,created_at,updated_at) VALUES(72004,71002,'Foreign operational household','foreign-people','UTC','active','active',now(),now());  INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74004,72004,73005,'owner','active',now(),now(),now())").await.unwrap();
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO api_sessions(id,account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,created_at,updated_at) VALUES(88021,71001,74001,$1,'people-refresh',1,now()+interval '15 minutes',now()+interval '1 day',now(),now(),now())",[hex::encode(Sha256::digest(b"people-session")).into()])).await.unwrap();
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO api_app_tokens(id,account_id,household_membership_id,name,token_digest,permissions_version,expires_at,last_used_at,created_at,updated_at) VALUES(88022,71001,74001,'People app',$1,1,now()+interval '1 month',now(),now(),now())",[hex::encode(Sha256::digest(b"people-app")).into()])).await.unwrap();
}

async fn people_effect(fixture: &Fixture) -> (i64, i64, i64) {
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM people WHERE id IN (SELECT person_id FROM household_memberships WHERE household_id=72001)) AS people, (SELECT count(*) FROM versions WHERE item_type='Person') AS versions, (SELECT count(*) FROM api_change_events WHERE record_type='Person') AS changes")).await.unwrap().unwrap();
    (
        row.try_get("", "people").unwrap(),
        row.try_get("", "versions").unwrap(),
        row.try_get("", "changes").unwrap(),
    )
}

async fn request_audits(fixture: &Fixture) -> i64 {
    fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS total FROM security_audit_events WHERE event_type='api.request' AND metadata->>'controller'='api/v1/people'")).await.unwrap().unwrap().try_get("","total").unwrap()
}

async fn people_state(fixture: &Fixture) -> Value {
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM people WHERE id IN (SELECT person_id FROM household_memberships WHERE household_id=72001)) AS people,(SELECT count(*) FROM person_access_grants) AS grants,(SELECT count(*) FROM location_memberships) AS links,(SELECT count(*) FROM versions WHERE item_type='Person') AS versions,(SELECT count(*) FROM api_change_events WHERE record_type='Person') AS changes,(SELECT count(*) FROM security_audit_events WHERE metadata->>'controller'='api/v1/people') AS audits,(SELECT permissions_version FROM household_memberships WHERE id=74001) AS permissions,(SELECT name FROM people WHERE id=73001) AS name")).await.unwrap().unwrap();
    json!({
        "people": row.try_get::<i64>("", "people").unwrap(),
        "grants": row.try_get::<i64>("", "grants").unwrap(),
        "links": row.try_get::<i64>("", "links").unwrap(),
        "versions": row.try_get::<i64>("", "versions").unwrap(),
        "changes": row.try_get::<i64>("", "changes").unwrap(),
        "audits": row.try_get::<i64>("", "audits").unwrap(),
        "permissions": row.try_get::<i32>("", "permissions").unwrap(),
        "name": row.try_get::<String>("", "name").unwrap(),
    })
}

async fn dependent_effect(fixture: &Fixture) -> (i64, i64, i64, i64, i64, i64) {
    let row=fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM household_memberships WHERE household_id=72001) AS memberships,(SELECT count(*) FROM guardian_declarations WHERE guardian_person_id=73001 AND declared_by_account_id=71001 AND origin_household_id=72001) AS declarations,(SELECT count(*) FROM carer_relationships WHERE household_id=72001 AND carer_id=73001 AND guardian_declaration_id IS NOT NULL AND active) AS carers,(SELECT count(*) FROM person_access_grants) AS grants,(SELECT count(*) FROM accounts) AS accounts,(SELECT count(*) FROM users) AS users")).await.unwrap().unwrap();
    (
        row.try_get("", "memberships").unwrap(),
        row.try_get("", "declarations").unwrap(),
        row.try_get("", "carers").unwrap(),
        row.try_get("", "grants").unwrap(),
        row.try_get("", "accounts").unwrap(),
        row.try_get("", "users").unwrap(),
    )
}

async fn assert_created_dependent(fixture: &Fixture, body: &Value) {
    let subject = body["data"]["id"]
        .as_i64()
        .or_else(|| {
            body["data"]["id"]
                .as_str()
                .and_then(|id| id.parse::<i64>().ok())
        })
        .expect("Successful dependent response must identify its Person");
    let row=fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT count(*) AS linked FROM guardian_declarations d JOIN accounts a ON a.id=d.declared_by_account_id AND a.person_id=d.guardian_person_id JOIN household_memberships m ON m.person_id=d.subject_person_id AND m.household_id=d.origin_household_id JOIN carer_relationships c ON c.guardian_declaration_id=d.id AND c.patient_id=d.subject_person_id AND c.carer_id=d.guardian_person_id AND c.household_id=d.origin_household_id WHERE d.subject_person_id=$1 AND d.guardian_person_id=73001 AND a.id=71001 AND d.origin_household_id=72001 AND d.statement_version=1 AND d.revision=1 AND d.withdrawn_at IS NULL AND m.status='active' AND m.revoked_at IS NULL AND c.active",[subject.into()])).await.unwrap().unwrap();
    assert_eq!(
        row.try_get::<i64>("", "linked").unwrap(),
        1,
        "Returned Person must own the active participation and declaration-linked care relationship"
    );
}

fn assert_dependent_effect(
    before: (i64, i64, i64, i64, i64, i64),
    after: (i64, i64, i64, i64, i64, i64),
) {
    assert_eq!(
        after,
        (
            before.0 + 1,
            before.1 + 1,
            before.2 + 1,
            before.3,
            before.4,
            before.5
        )
    );
}

#[test]
fn people_write_routes_document_existing_idempotency_conflict() {
    let document = contract::contract();
    let mut missing = Vec::new();
    for (path, method) in [
        ("/households/{household_id}/people", "post"),
        ("/households/{household_id}/people/{id}", "patch"),
        ("/households/{household_id}/people/{id}", "put"),
    ] {
        let operation = &document["paths"][path][method];
        if operation["responses"]["409"]["$ref"] != "#/components/responses/Conflict" {
            missing.push(format!("{method} {path}: Conflict 409"));
        }
        if !operation["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .any(|parameter| parameter["$ref"] == "#/components/parameters/idempotency_key")
        {
            missing.push(format!("{method} {path}: Idempotency-Key"));
        }
    }
    assert!(
        missing.is_empty(),
        "missing idempotency contract: {missing:?}"
    );
}

#[tokio::test]
async fn people_collection_scopes_credentials_and_validates_queries() {
    let (fixture, boot) = boot_fixture().await;
    seed_credentials_and_foreign_household(&fixture).await;
    let mobile = mobile_token(&fixture).await;
    let server = server(&boot);
    let credentials = ["people-session", "people-app", mobile.as_str()];
    let path = "/households/{household_id}/people";
    let mut replies = Vec::new();
    for credential in credentials {
        for household in [72001, 72004, 99999] {
            let route = format!("/api/v1/households/{household}/people");
            replies.push((
                household,
                send(&server, Method::GET, &route, Some(credential), None, None).await,
            ));
        }
    }
    let invalid = send(
        &server,
        Method::GET,
        "/api/v1/households/72001/people?page=0",
        Some("people-session"),
        None,
        None,
    )
    .await;
    let anonymous = send(
        &server,
        Method::GET,
        "/api/v1/households/72004/people",
        None,
        None,
        None,
    )
    .await;
    let before = people_effect(&fixture).await;
    let invalid_credential = send(
        &server,
        Method::GET,
        "/api/v1/households/99999/people",
        Some("invalid"),
        None,
        None,
    )
    .await;
    let after = people_effect(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(
        replies
            .iter()
            .map(|(household, reply)| (*household, reply.status))
            .collect::<Vec<_>>(),
        [(72001, 200), (72004, 403), (99999, 404)].repeat(3)
    );
    assert_eq!(
        (invalid.status, anonymous.status, invalid_credential.status),
        (422, 401, 401)
    );
    assert_eq!(before, after);
    for (household, reply) in replies {
        assert_schema("get", path, &reply);
        assert_wire(&reply);
        if household == 72001 {
            assert_eq!(reply.body["data"][0]["id"], 73001);
        }
    }
    for reply in [invalid, anonymous, invalid_credential] {
        assert_schema("get", path, &reply);
        assert_wire(&reply);
    }
}

#[tokio::test]
async fn create_person_preserves_atomic_links_and_rejects_denied_writes() {
    let (fixture, boot) = boot_fixture().await;
    seed_credentials_and_foreign_household(&fixture).await;
    let server = server(&boot);
    let path = "/api/v1/households/72001/people";
    let document = contract::contract();
    let payload = json!({"person":{"name":"People TestServer child","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}});
    contract::assert_value(
        document,
        &document["paths"]["/households/{household_id}/people"]["post"]["requestBody"]["content"]["application/json"]
            ["schema"],
        &payload,
        "create person request",
    );
    let dependent_before = dependent_effect(&fixture).await;
    let before_denial = people_effect(&fixture).await;
    let audits_before_denial = request_audits(&fixture).await;
    let denied = send(
        &server,
        Method::POST,
        "/api/v1/households/72004/people",
        Some("people-session"),
        Some(&payload),
        None,
    )
    .await;
    let missing = send(
        &server,
        Method::POST,
        "/api/v1/households/99999/people",
        Some("people-session"),
        Some(&payload),
        None,
    )
    .await;
    let unauthorized = send(
        &server,
        Method::POST,
        path,
        Some("invalid"),
        Some(&payload),
        None,
    )
    .await;
    let malformed = send_malformed(&server, Method::POST, path).await;
    let invalid = send(&server, Method::POST, path, Some("people-session"), Some(&json!({"person":{"name":"Wrong age","date_of_birth":"2020-01-01","person_type":"dependent_adult","has_capacity":false,"initial_guardian_declaration":true}})), None).await;
    let dependent_after_denial = dependent_effect(&fixture).await;
    let after_denial = people_effect(&fixture).await;
    let audits_after_denial = request_audits(&fixture).await;
    let created = send(
        &server,
        Method::POST,
        path,
        Some("people-session"),
        Some(&payload),
        None,
    )
    .await;
    let links = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM people WHERE name='People TestServer child') AS people, (SELECT count(*) FROM person_access_grants WHERE person_id=(SELECT id FROM people WHERE name='People TestServer child')) AS grants, (SELECT count(*) FROM location_memberships WHERE person_id=(SELECT id FROM people WHERE name='People TestServer child')) AS locations")).await.unwrap().unwrap();
    let effects = (
        links.try_get::<i64>("", "people").unwrap(),
        links.try_get::<i64>("", "grants").unwrap(),
        links.try_get::<i64>("", "locations").unwrap(),
    );
    let dependent_after = dependent_effect(&fixture).await;
    if created.status == 201 {
        assert_created_dependent(&fixture, &created.body).await;
    }
    drop(server);
    close(fixture, boot).await;
    assert_eq!(created.status, 201);
    assert_eq!(created.body["data"]["has_capacity"], false);
    assert!(created.etag.as_ref().is_some_and(|etag| !etag.is_empty()));
    assert_schema("post", "/households/{household_id}/people", &created);
    assert_wire(&created);
    assert_eq!(effects, (1, 0, 1));
    assert_eq!(dependent_after_denial, dependent_before);
    assert_dependent_effect(dependent_before, dependent_after);
    assert_eq!(
        invalid.body["error"]["errors"]["person_type"],
        json!(["does not match age"])
    );
    assert_eq!(
        (
            denied.status,
            missing.status,
            unauthorized.status,
            malformed.status,
            invalid.status
        ),
        (403, 404, 401, 400, 422)
    );
    assert_eq!(before_denial, after_denial);
    assert_eq!(audits_after_denial, audits_before_denial + 2);
    for reply in [denied, missing, unauthorized, malformed, invalid] {
        assert_schema("post", "/households/{household_id}/people", &reply);
        assert_wire(&reply);
    }
}

#[tokio::test]
async fn people_methods_reject_invalid_and_wrong_namespace_credentials_before_writes() {
    let (fixture, boot) = boot_fixture().await;
    seed_credentials_and_foreign_household(&fixture).await;
    let mobile = mobile_token(&fixture).await;
    let smart = "synthetic-people-smart-access";
    let digest = base64::engine::general_purpose::URL_SAFE.encode(Sha256::digest(smart.as_bytes()));
    fixture.admin.execute_unprepared("INSERT INTO oauth_applications(id,account_id,client_id,client_kind,name,redirect_uri,scopes,token_endpoint_auth_method,client_secret_hash,created_at,updated_at) VALUES(75003,71001,'smart-people','integration','Synthetic SMART people','https://example.test/callback?tenant=7','launch/patient patient/*.rs offline_access','client_secret_post',crypt('password',gen_salt('bf',4)),now(),now())").await.unwrap();
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO oauth_grants(id,account_id,oauth_application_id,client_kind,household_membership_id,person_id,permissions_version,redirect_uri,scopes,token_hash,expires_in,authenticated_at,last_used_at,device_name,created_at,updated_at) VALUES(76003,71001,75003,'integration',74001,73001,1,'https://example.test/callback?tenant=7','launch/patient patient/*.rs offline_access',$1,now()+interval '15 minutes',now(),now(),'SMART client',now(),now())",[digest.into()])).await.unwrap();
    fixture.admin.execute_unprepared("UPDATE oauth_grants SET access_token_scopes='patient/*.read',access_token_scope_hash=token_hash WHERE id=76001").await.unwrap();
    let before = people_effect(&fixture).await;
    let audits_before = request_audits(&fixture).await;
    let server = server(&boot);
    let payload = json!({"person":{"name":"Must not write"}});
    let cases = [
        (
            Method::GET,
            "/api/v1/households/72004/people",
            "/households/{household_id}/people",
            "get",
            false,
        ),
        (
            Method::POST,
            "/api/v1/households/72004/people",
            "/households/{household_id}/people",
            "post",
            true,
        ),
        (
            Method::PATCH,
            "/api/v1/households/72004/people/73001",
            "/households/{household_id}/people/{id}",
            "patch",
            true,
        ),
        (
            Method::PUT,
            "/api/v1/households/72004/people/73001",
            "/households/{household_id}/people/{id}",
            "put",
            true,
        ),
    ];
    let mut replies = Vec::new();
    for (method, route, path, verb, has_body) in cases {
        for (credential, bearer, expected) in [
            ("invalid", "invalid", 401),
            ("smart", smart, 401),
            ("mobile_wrong_scope", mobile.as_str(), 403),
        ] {
            let reply = send(
                &server,
                method.clone(),
                route,
                Some(bearer),
                has_body.then_some(&payload),
                None,
            )
            .await;
            replies.push((path, verb, credential, expected, reply));
        }
    }
    let after = people_effect(&fixture).await;
    let audits_after = request_audits(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before, after);
    assert_eq!(audits_after, audits_before);
    for (path, verb, credential, expected, reply) in replies {
        assert_eq!(reply.status, expected, "{verb} {path} {credential}");
        assert_schema(verb, path, &reply);
        assert_wire(&reply);
        assert!(reply.body.get("data").is_none());
        assert_eq!(
            reply.challenge.as_deref(),
            Some(if expected == 403 {
                "Bearer error=\"insufficient_scope\" scope=\"medtracker\""
            } else {
                "Bearer"
            })
        );
    }
}

#[tokio::test]
async fn people_write_methods_recheck_current_manage_permission() {
    let (fixture, boot) = boot_fixture().await;
    seed_credentials_and_foreign_household(&fixture).await;
    fixture.admin.execute_unprepared("UPDATE person_access_grants SET access_level='view' WHERE id=78001; UPDATE household_memberships SET role='member' WHERE id=74001; UPDATE people SET date_of_birth='1990-01-01' WHERE id=73001").await.unwrap();
    let server = server(&boot);
    for method in [Method::PATCH, Method::PUT] {
        let verb = method.as_str().to_lowercase();
        let own = send(
            &server,
            method,
            "/api/v1/households/72001/people/73001",
            Some("people-session"),
            Some(&json!({"person":{"name":"Canonical self remains writable"}})),
            None,
        )
        .await;
        assert_eq!(own.status, 200);
        assert_schema(&verb, "/households/{household_id}/people/{id}", &own);
        assert_wire(&own);
    }
    let dependent_before = dependent_effect(&fixture).await;
    let before = people_effect(&fixture).await;
    let audits_before = request_audits(&fixture).await;
    let body = json!({"person":{"name":"Must not write"}});
    let cases = [
        (
            Method::POST,
            "/api/v1/households/72001/people",
            "/households/{household_id}/people",
            "post",
        ),
        (
            Method::PATCH,
            "/api/v1/households/72001/people/73002",
            "/households/{household_id}/people/{id}",
            "patch",
        ),
        (
            Method::PUT,
            "/api/v1/households/72001/people/73002",
            "/households/{household_id}/people/{id}",
            "put",
        ),
    ];
    let mut replies = Vec::new();
    for (method, route, path, verb) in cases {
        replies.push((
            path,
            verb,
            send(
                &server,
                method,
                route,
                Some("people-session"),
                Some(&body),
                None,
            )
            .await,
        ));
    }
    let dependent_after = dependent_effect(&fixture).await;
    let after = people_effect(&fixture).await;
    let audits_after = request_audits(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before, after);
    assert_eq!(dependent_before, dependent_after);
    assert_eq!(audits_after, audits_before + 3);
    for (path, verb, reply) in replies {
        assert_eq!(reply.status, 403, "{verb}");
        assert_schema(verb, path, &reply);
        assert_eq!(reply.body["error"]["code"], "forbidden");
    }
}

#[tokio::test]
async fn app_and_mobile_credentials_can_each_write_all_people_methods() {
    for credential in ["app", "mobile"] {
        for method in [Method::POST, Method::PATCH, Method::PUT] {
            let (fixture, boot) = boot_fixture().await;
            seed_credentials_and_foreign_household(&fixture).await;
            if method != Method::POST {
                fixture
                    .admin
                    .execute_unprepared(
                        "UPDATE people SET date_of_birth='1990-01-01' WHERE id=73001",
                    )
                    .await
                    .unwrap();
            }
            let mobile = if credential == "mobile" {
                Some(mobile_token(&fixture).await)
            } else {
                None
            };
            let bearer = mobile.as_deref().unwrap_or("people-app");
            let route = if method == Method::POST {
                "/api/v1/households/72001/people"
            } else {
                "/api/v1/households/72001/people/73001"
            };
            let contract_path = if method == Method::POST {
                "/households/{household_id}/people"
            } else {
                "/households/{household_id}/people/{id}"
            };
            let name = format!("{credential} {method} person");
            let body = if method == Method::POST {
                json!({"person":{"name":name.clone(),"date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}})
            } else {
                json!({"person":{"name":name.clone()}})
            };
            let dependent_before = dependent_effect(&fixture).await;
            let before = people_effect(&fixture).await;
            let audits_before = request_audits(&fixture).await;
            let server = server(&boot);
            let reply = send(
                &server,
                method.clone(),
                route,
                Some(bearer),
                Some(&body),
                None,
            )
            .await;
            let dependent_after = dependent_effect(&fixture).await;
            let after = people_effect(&fixture).await;
            let audits_after = request_audits(&fixture).await;
            let row = fixture
                .admin
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "SELECT count(*) AS total FROM people WHERE id IN (SELECT person_id FROM household_memberships WHERE household_id=72001) AND name=$1",
                    [name.clone().into()],
                ))
                .await
                .unwrap()
                .unwrap();
            let persisted: i64 = row.try_get("", "total").unwrap();
            if method == Method::POST && reply.status == 201 {
                assert_created_dependent(&fixture, &reply.body).await;
            }
            drop(server);
            close(fixture, boot).await;
            assert_eq!(
                reply.status,
                if method == Method::POST { 201 } else { 200 },
                "{credential} {method}"
            );
            assert_schema(&method.as_str().to_lowercase(), contract_path, &reply);
            assert_wire(&reply);
            assert!(reply.etag.as_ref().is_some_and(|etag| !etag.is_empty()));
            assert_eq!(persisted, 1);
            if method == Method::POST {
                assert_dependent_effect(dependent_before, dependent_after);
            } else {
                assert_eq!(dependent_before, dependent_after);
            }
            assert_eq!(
                after.0,
                before.0 + if method == Method::POST { 1 } else { 0 }
            );
            assert!(after.1 > before.1);
            assert!(after.2 > before.2);
            assert_eq!(audits_after, audits_before + 1);
        }
    }
}

#[tokio::test]
async fn revoked_credentials_and_withdrawn_membership_block_every_people_method() {
    let (fixture, boot) = boot_fixture().await;
    seed_credentials_and_foreign_household(&fixture).await;
    let mobile = mobile_token(&fixture).await;
    let cases = [
        (
            Method::GET,
            "/api/v1/households/72001/people",
            "/households/{household_id}/people",
            "get",
            false,
        ),
        (
            Method::POST,
            "/api/v1/households/72001/people",
            "/households/{household_id}/people",
            "post",
            true,
        ),
        (
            Method::PATCH,
            "/api/v1/households/72001/people/73001",
            "/households/{household_id}/people/{id}",
            "patch",
            true,
        ),
        (
            Method::PUT,
            "/api/v1/households/72001/people/73001",
            "/households/{household_id}/people/{id}",
            "put",
            true,
        ),
    ];
    let body = json!({"person":{"name":"Must not write"}});
    let server = server(&boot);
    fixture.admin.execute_unprepared("UPDATE api_sessions SET revoked_at=now() WHERE id=88021; UPDATE api_app_tokens SET revoked_at=now() WHERE id=88022; UPDATE oauth_grants SET revoked_at=now() WHERE id=76001").await.unwrap();
    let before = people_effect(&fixture).await;
    let audits_before = request_audits(&fixture).await;
    let mut replies = Vec::new();
    for (method, route, path, verb, has_body) in &cases {
        for (kind, bearer) in [
            ("api_session", "people-session"),
            ("app_token", "people-app"),
            ("mobile", mobile.as_str()),
        ] {
            let reply = send(
                &server,
                method.clone(),
                route,
                Some(bearer),
                (*has_body).then_some(&body),
                None,
            )
            .await;
            replies.push(("revoked", path, verb, kind, 401, reply));
        }
    }
    fixture.admin.execute_unprepared("UPDATE api_sessions SET revoked_at=NULL WHERE id=88021; UPDATE api_app_tokens SET revoked_at=NULL WHERE id=88022; UPDATE oauth_grants SET revoked_at=NULL WHERE id=76001; UPDATE household_memberships SET revoked_at=now() WHERE id=74001").await.unwrap();
    for (method, route, path, verb, has_body) in &cases {
        for (kind, bearer, expected) in [
            ("api_session", "people-session", 401),
            ("app_token", "people-app", 401),
            ("mobile", mobile.as_str(), 403),
        ] {
            let reply = send(
                &server,
                method.clone(),
                route,
                Some(bearer),
                (*has_body).then_some(&body),
                None,
            )
            .await;
            replies.push(("membership", path, verb, kind, expected, reply));
        }
    }
    let after = people_effect(&fixture).await;
    let audits_after = request_audits(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before, after);
    assert_eq!(audits_before, audits_after);
    for (boundary, path, verb, kind, expected, reply) in replies {
        assert_eq!(reply.status, expected, "{boundary} {kind} {verb}");
        assert_schema(verb, path, &reply);
        assert_wire(&reply);
        assert!(reply.body.get("data").is_none());
    }
}

async fn update_person_merges_fields_and_rejects_changed_key_replay(method: Method) {
    let (fixture, boot) = boot_fixture().await;
    seed_credentials_and_foreign_household(&fixture).await;
    fixture
        .admin
        .execute_unprepared("UPDATE people SET date_of_birth='1990-01-01' WHERE id=73001")
        .await
        .unwrap();
    let server = server(&boot);
    let path = "/api/v1/households/72001/people/73001";
    let method_name = method.as_str().to_lowercase();
    let first = json!({"person":{"name":format!("People {method} first")}});
    let changed = json!({"person":{"name":format!("People {method} changed")}});
    let document = contract::contract();
    contract::assert_value(
        document,
        &document["paths"]["/households/{household_id}/people/{id}"][method_name.as_str()]["requestBody"]
            ["content"]["application/json"]["schema"],
        &first,
        "person update request",
    );
    let key = format!("people-{method}-key");
    let updated = send(
        &server,
        method.clone(),
        path,
        Some("people-session"),
        Some(&first),
        Some(&key),
    )
    .await;
    let replay = send(
        &server,
        method.clone(),
        path,
        Some("people-app"),
        Some(&first),
        Some(&key),
    )
    .await;
    let before_denial = people_effect(&fixture).await;
    let conflict = send(
        &server,
        method.clone(),
        path,
        Some("people-session"),
        Some(&changed),
        Some(&key),
    )
    .await;
    let foreign = send(
        &server,
        method.clone(),
        "/api/v1/households/72004/people/73001",
        Some("people-session"),
        Some(&changed),
        None,
    )
    .await;
    let absent = send(
        &server,
        method.clone(),
        "/api/v1/households/99999/people/73001",
        Some("people-session"),
        Some(&changed),
        None,
    )
    .await;
    let invalid = send(
        &server,
        method.clone(),
        path,
        Some("people-session"),
        Some(&json!({"person":{"name":""}})),
        None,
    )
    .await;
    let malformed = send_malformed(&server, method.clone(), path).await;
    let unauthorized = send(
        &server,
        method.clone(),
        path,
        Some("invalid"),
        Some(&changed),
        None,
    )
    .await;
    let after_denial = people_effect(&fixture).await;
    let row = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT name,person_type FROM people WHERE id=73001",
        ))
        .await
        .unwrap()
        .unwrap();
    let stored: (String, i32) = (
        row.try_get("", "name").unwrap(),
        row.try_get("", "person_type").unwrap(),
    );
    drop(server);
    close(fixture, boot).await;
    let contract_path = "/households/{household_id}/people/{id}";
    assert_eq!(
        (
            updated.status,
            replay.status,
            conflict.status,
            foreign.status,
            absent.status,
            invalid.status,
            malformed.status,
            unauthorized.status
        ),
        (200, 200, 409, 403, 404, 422, 400, 401),
        "{method}"
    );
    assert_eq!(updated.body, replay.body);
    assert_eq!(replay.replayed.as_deref(), Some("true"));
    assert_eq!(updated.etag, replay.etag);
    assert_eq!(stored.0, format!("People {method} first"));
    assert_eq!(stored.1, 0);
    assert_eq!(before_denial, after_denial);
    for reply in [
        updated,
        replay,
        conflict,
        foreign,
        absent,
        invalid,
        malformed,
        unauthorized,
    ] {
        assert_schema(&method_name, contract_path, &reply);
        assert_wire(&reply);
    }
}

#[tokio::test]
async fn patch_person_merges_fields_and_rejects_changed_key_replay() {
    update_person_merges_fields_and_rejects_changed_key_replay(Method::PATCH).await;
}

#[tokio::test]
async fn put_person_merges_fields_and_rejects_changed_key_replay() {
    update_person_merges_fields_and_rejects_changed_key_replay(Method::PUT).await;
}

#[tokio::test]
async fn all_four_people_methods_share_fixed_general_limit_without_write_effects() {
    let (fixture, boot) = boot_rate_fixture(&[]).await;
    seed_credentials_and_foreign_household(&fixture).await;
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
    let before = people_effect(&fixture).await;
    let body = json!({"person":{"name":"Rate rejected"}});
    let cases = [
        (
            Method::GET,
            "/api/v1/households/72001/people",
            "/households/{household_id}/people",
            "get",
            false,
        ),
        (
            Method::POST,
            "/api/v1/households/72001/people",
            "/households/{household_id}/people",
            "post",
            true,
        ),
        (
            Method::PATCH,
            "/api/v1/households/72001/people/73001",
            "/households/{household_id}/people/{id}",
            "patch",
            true,
        ),
        (
            Method::PUT,
            "/api/v1/households/72001/people/73001",
            "/households/{household_id}/people/{id}",
            "put",
            true,
        ),
    ];
    let mut replies = Vec::new();
    for (method, route, contract_path, verb, has_body) in cases {
        let reply = send(
            &server,
            method,
            route,
            Some("people-session"),
            has_body.then_some(&body),
            None,
        )
        .await;
        replies.push((contract_path, verb, reply));
    }
    let after = people_effect(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before, after);
    for (path, verb, reply) in replies {
        assert_eq!(reply.status, 429, "{verb} {path}");
        assert_eq!(reply.retry_after.as_deref(), Some("200"));
        assert_eq!(reply.limit.as_deref(), Some("300"));
        assert_eq!(reply.remaining.as_deref(), Some("0"));
        assert_eq!(reply.reset.as_deref(), Some("1200"));
        assert_eq!(reply.cache_control.as_deref(), Some("no-store"));
        assert_wire(&reply);
        assert_schema(verb, path, &reply);
    }
}

async fn concurrent_person_key_replays_one_write(method: Method) {
    let (fixture, boot) = boot_fixture().await;
    seed_credentials_and_foreign_household(&fixture).await;
    if method != Method::POST {
        fixture
            .admin
            .execute_unprepared("UPDATE people SET date_of_birth='1990-01-01' WHERE id=73001")
            .await
            .unwrap();
    }
    let mobile = if method == Method::POST {
        mobile_token(&fixture).await
    } else {
        String::new()
    };
    let first_bearer = if method == Method::POST {
        mobile.as_str()
    } else {
        "people-session"
    };
    let second_bearer = if method == Method::POST {
        mobile.as_str()
    } else {
        "people-app"
    };
    let server = server(&boot);
    let path = if method == Method::POST {
        "/api/v1/households/72001/people"
    } else {
        "/api/v1/households/72001/people/73001"
    };
    let body = if method == Method::POST {
        json!({"person":{"name":"Concurrent People child","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}})
    } else {
        json!({"person":{"name":"Concurrent People update"}})
    };
    let changed = if method == Method::POST {
        json!({"person":{"name":"Conflicting People child","date_of_birth":"2020-01-01","person_type":"minor","has_capacity":false,"initial_guardian_declaration":true}})
    } else {
        json!({"person":{"name":"Conflicting People update"}})
    };
    let key = format!("people-overlap-{}", method.as_str().to_lowercase());
    let dependent_before = if method == Method::POST {
        Some(dependent_effect(&fixture).await)
    } else {
        None
    };
    let before = people_effect(&fixture).await;
    let start = std::sync::Arc::new(tokio::sync::Barrier::new(3));
    let (first, second, _) = tokio::join!(
        async {
            start.wait().await;
            send(
                &server,
                method.clone(),
                path,
                Some(first_bearer),
                Some(&body),
                Some(&key),
            )
            .await
        },
        async {
            start.wait().await;
            send(
                &server,
                method.clone(),
                path,
                Some(second_bearer),
                Some(&body),
                Some(&key),
            )
            .await
        },
        async { start.wait().await }
    );
    let dependent_after_overlap = if method == Method::POST {
        Some(dependent_effect(&fixture).await)
    } else {
        None
    };
    let after_overlap = people_effect(&fixture).await;
    let replay = if method == Method::POST {
        Some(
            send(
                &server,
                method.clone(),
                path,
                Some(first_bearer),
                Some(&body),
                Some(&key),
            )
            .await,
        )
    } else {
        None
    };
    let dependent_after_replay = if method == Method::POST {
        Some(dependent_effect(&fixture).await)
    } else {
        None
    };
    let after_replay = people_effect(&fixture).await;
    let conflict = send(
        &server,
        method.clone(),
        path,
        Some(first_bearer),
        Some(&changed),
        Some(&key),
    )
    .await;
    let dependent_after_conflict = if method == Method::POST {
        Some(dependent_effect(&fixture).await)
    } else {
        None
    };
    let after_conflict = people_effect(&fixture).await;
    if method == Method::POST {
        if first.status == 201 {
            assert_created_dependent(&fixture, &first.body).await;
        }
        if second.status == 201 {
            assert_created_dependent(&fixture, &second.body).await;
        }
        if let Some(reply) = &replay
            && reply.status == 201
        {
            assert_created_dependent(&fixture, &reply.body).await;
        }
    }
    drop(server);
    close(fixture, boot).await;
    let expected_status = if method == Method::POST { 201 } else { 200 };
    let expected_people = if method == Method::POST { 1 } else { 0 };
    if method == Method::POST {
        let (winner, contender) = if first.status == 201 {
            (&first, &second)
        } else {
            (&second, &first)
        };
        assert_eq!(winner.status, 201);
        assert!([201, 403].contains(&contender.status), "{method}");
        if contender.status == 201 {
            assert_eq!(winner.body, contender.body);
            assert_eq!(
                [winner.replayed.as_deref(), contender.replayed.as_deref()]
                    .iter()
                    .filter(|marker| **marker == Some("true"))
                    .count(),
                1
            );
        } else {
            assert_eq!(contender.body["error"]["code"], "forbidden");
            assert!(contender.body.get("data").is_none());
        }
        let replay = replay.as_ref().unwrap();
        assert_eq!(replay.status, 201);
        assert_eq!(replay.replayed.as_deref(), Some("true"));
        assert_eq!(replay.body, winner.body);
    } else {
        assert_eq!(
            (first.status, second.status),
            (expected_status, expected_status),
            "{method}"
        );
        assert_eq!(first.body, second.body, "{method}");
        assert_eq!(
            [first.replayed.as_deref(), second.replayed.as_deref()]
                .iter()
                .filter(|marker| **marker == Some("true"))
                .count(),
            1,
            "{method}"
        );
    }
    if let (Some(before), Some(after)) = (dependent_before, dependent_after_overlap) {
        assert_dependent_effect(before, after);
        assert_eq!(dependent_after_replay, Some(after));
        assert_eq!(dependent_after_conflict, Some(after));
    }
    assert_eq!(conflict.status, 409, "{method}");
    assert_eq!(
        (
            after_overlap.0 - before.0,
            after_overlap.1 - before.1,
            after_overlap.2 - before.2
        ),
        (expected_people, 1, 1),
        "{method}"
    );
    assert_eq!(after_replay, after_overlap, "{method}");
    assert_eq!(after_conflict, after_overlap, "{method}");
    let contract_path = if method == Method::POST {
        "/households/{household_id}/people"
    } else {
        "/households/{household_id}/people/{id}"
    };
    let method_name = method.as_str().to_lowercase();
    for reply in [first, second, conflict] {
        assert_schema(&method_name, contract_path, &reply);
        assert_wire(&reply);
    }
    if let Some(replay) = replay {
        assert_schema(&method_name, contract_path, &replay);
        assert_wire(&replay);
    }
}

#[tokio::test]
async fn concurrent_create_person_key_replays_one_write() {
    concurrent_person_key_replays_one_write(Method::POST).await;
}

#[tokio::test]
async fn concurrent_patch_person_key_replays_one_write() {
    concurrent_person_key_replays_one_write(Method::PATCH).await;
}

#[tokio::test]
async fn concurrent_put_person_key_replays_one_write() {
    concurrent_person_key_replays_one_write(Method::PUT).await;
}

#[tokio::test]
async fn people_audit_insert_failure_rolls_back_every_route_without_private_data() {
    let (fixture, boot) = boot_fixture().await;
    seed_credentials_and_foreign_household(&fixture).await;
    fixture
        .admin
        .execute_unprepared("UPDATE people SET date_of_birth='1990-01-01' WHERE id=73001")
        .await
        .unwrap();
    fixture.admin.execute_unprepared("ALTER TABLE security_audit_events ADD CONSTRAINT reject_people_request_audit CHECK (metadata->>'controller' IS DISTINCT FROM 'api/v1/people') NOT VALID").await.unwrap();
    let server = server(&boot);
    let before = people_state(&fixture).await;
    let body =
        json!({"person":{"name":"Audit rejected People child","date_of_birth":"2020-01-01"}});
    let cases = [
        (
            Method::GET,
            "/api/v1/households/72001/people",
            "/households/{household_id}/people",
            "get",
            false,
        ),
        (
            Method::POST,
            "/api/v1/households/72001/people",
            "/households/{household_id}/people",
            "post",
            true,
        ),
        (
            Method::PATCH,
            "/api/v1/households/72001/people/73001",
            "/households/{household_id}/people/{id}",
            "patch",
            true,
        ),
        (
            Method::PUT,
            "/api/v1/households/72001/people/73001",
            "/households/{household_id}/people/{id}",
            "put",
            true,
        ),
    ];
    let mut replies = Vec::new();
    for (method, path, contract_path, verb, has_body) in cases {
        let response = send(
            &server,
            method,
            path,
            Some("people-session"),
            has_body.then_some(&body),
            None,
        )
        .await;
        let after = people_state(&fixture).await;
        replies.push((contract_path, verb, response, after));
    }
    drop(server);
    close(fixture, boot).await;
    for (path, verb, response, after) in &replies {
        assert_eq!(response.status, 500, "{verb} {path}");
        assert_eq!(
            response.body["error"]["code"], "internal_error",
            "{verb} {path}"
        );
        assert!(response.body.get("data").is_none(), "{verb} {path}");
        assert_eq!(after, &before, "{verb} {path} persisted an effect");
        assert_wire(response);
    }
    for (path, verb, response, _) in replies {
        assert_schema(verb, path, &response);
    }
}

#[tokio::test]
async fn people_listing_hides_data_when_restricted_household_read_fails() {
    let (fixture, boot) = boot_fixture().await;
    let mobile = mobile_token(&fixture).await;
    fixture
        .admin
        .execute_unprepared("REVOKE SELECT ON public.households FROM med_tracker_app")
        .await
        .unwrap();
    let before = people_state(&fixture).await;
    let server = server(&boot);
    let response = send(
        &server,
        Method::GET,
        "/api/v1/households/72001/people",
        Some(&mobile),
        None,
        None,
    )
    .await;
    let after = people_state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(response.status, 500);
    assert_eq!(response.body["error"]["code"], "internal_error");
    assert!(response.body.get("data").is_none());
    assert_eq!(after, before);
    assert_wire(&response);
    assert_schema("get", "/households/{household_id}/people", &response);
}

#[tokio::test]
async fn people_listing_rechecks_account_closed_during_mobile_authentication() {
    let (fixture, boot) = boot_fixture().await;
    let mobile = mobile_token(&fixture).await;
    fixture.admin.execute_unprepared("CREATE SEQUENCE public.people_account_closure_reached; CREATE FUNCTION public.close_account_after_mobile_touch() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$ BEGIN PERFORM nextval('public.people_account_closure_reached'); UPDATE public.accounts SET status=3 WHERE id=71001; RETURN NEW; END $$; CREATE TRIGGER close_account_after_mobile_touch AFTER UPDATE OF last_used_at ON public.oauth_grants FOR EACH ROW WHEN (NEW.account_id=71001) EXECUTE FUNCTION public.close_account_after_mobile_touch()").await.unwrap();
    let before = people_state(&fixture).await;
    let server = server(&boot);
    let response = send(
        &server,
        Method::GET,
        "/api/v1/households/72001/people",
        Some(&mobile),
        None,
        None,
    )
    .await;
    let state = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT is_called FROM public.people_account_closure_reached) AS reached,(SELECT status FROM public.accounts WHERE id=71001) AS status")).await.unwrap().unwrap();
    let reached: bool = state.try_get("", "reached").unwrap();
    let status: i32 = state.try_get("", "status").unwrap();
    let after = people_state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert!(reached);
    assert_eq!(status, 3);
    assert_eq!(response.status, 401);
    assert_eq!(response.body["error"]["code"], "unauthorized");
    assert!(response.body.get("data").is_none());
    assert_eq!(after, before);
    assert_wire(&response);
    assert_schema("get", "/households/{household_id}/people", &response);
}
