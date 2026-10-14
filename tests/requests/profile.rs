use super::*;
use axum::http::Method;
use base64::Engine;

const PROFILE: &str = "/households/{household_id}/profile";

struct Reply {
    status: u16,
    body: Value,
    content_type: Option<String>,
    request_id: Option<String>,
    cache_control: Option<String>,
    replayed: Option<String>,
    challenge: Option<String>,
    retry_after: Option<String>,
    limit: Option<String>,
    remaining: Option<String>,
    reset: Option<String>,
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
        content_type: header(headers, "content-type"),
        request_id: header(headers, "x-request-id"),
        cache_control: header(headers, "cache-control"),
        replayed: header(headers, "idempotency-replayed"),
        challenge: header(headers, "www-authenticate"),
        retry_after: header(headers, "retry-after"),
        limit: header(headers, "ratelimit-limit"),
        remaining: header(headers, "ratelimit-remaining"),
        reset: header(headers, "ratelimit-reset"),
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
    let mut request = server.method(method, &format!("/api/v1/households/{household}/profile"));
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

fn assert_schema(method: &str, response: &Reply) {
    let document = contract::contract();
    let raw = &document["paths"][PROFILE][method]["responses"][response.status.to_string()];
    let described = if raw.get("$ref").is_some() {
        contract::resolve(document, raw)
    } else {
        raw
    };
    assert!(
        described.is_object(),
        "{method} profile {} undocumented",
        response.status
    );
    contract::assert_value(
        document,
        &described["content"]["application/json"]["schema"],
        &response.body,
        &format!("{method} profile {}", response.status),
    );
}

fn assert_wire(response: &Reply) {
    assert_eq!(response.content_type.as_deref(), Some("application/json"));
    assert_eq!(
        response.cache_control.as_deref(),
        Some("no-store"),
        "profile status {} body {}",
        response.status,
        response.body
    );
    let request_id = response.request_id.as_deref().expect("request ID");
    assert!(!request_id.is_empty());
    if response.status != 429 && response.body.get("error").is_some() {
        assert_eq!(response.body["error"]["request_id"], request_id);
    }
}

pub(super) async fn mobile_token(fixture: &Fixture) -> String {
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

pub(super) async fn seed_credentials(fixture: &Fixture) {
    fixture.admin.execute_unprepared("INSERT INTO accounts(id,email,password_hash,created_at,updated_at) VALUES(71002,'foreign-profile@example.test',crypt('password',gen_salt('bf',4)),now(),now()); INSERT INTO households(id,created_by_account_id,name,slug,timezone,status,lifecycle_state,created_at,updated_at) VALUES(72004,71002,'Foreign profile household','foreign-profile','UTC','active','active',now(),now()); INSERT INTO people(id,household_id,account_id,name,person_type,has_capacity,created_at,updated_at) VALUES(73005,72004,71002,'Foreign profile member',0,true,now(),now()); INSERT INTO household_memberships(id,household_id,account_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74004,72004,71002,73005,'owner','active',now(),now(),now())").await.unwrap();
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO api_sessions(id,account_id,household_membership_id,access_token_digest,refresh_token_digest,permissions_version,access_expires_at,refresh_expires_at,last_used_at,created_at,updated_at) VALUES(88031,71001,74001,$1,'profile-refresh',1,now()+interval '15 minutes',now()+interval '1 day',now(),now(),now())",[hex::encode(Sha256::digest(b"profile-session")).into()])).await.unwrap();
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO api_app_tokens(id,account_id,household_membership_id,name,token_digest,permissions_version,expires_at,last_used_at,created_at,updated_at) VALUES(88032,71001,74001,'Profile app',$1,1,now()+interval '1 month',now(),now(),now())",[hex::encode(Sha256::digest(b"profile-app")).into()])).await.unwrap();
}

pub(super) async fn seed_disallowed_grants(fixture: &Fixture, smart: &str) {
    let digest = base64::engine::general_purpose::URL_SAFE.encode(Sha256::digest(smart.as_bytes()));
    fixture.admin.execute_unprepared("INSERT INTO oauth_applications(id,account_id,client_id,client_kind,name,redirect_uri,scopes,token_endpoint_auth_method,client_secret_hash,created_at,updated_at) VALUES(75003,71001,'smart-profile','integration','Synthetic SMART profile','https://example.test/callback','launch/patient patient/*.rs offline_access','client_secret_post',crypt('password',gen_salt('bf',4)),now(),now())").await.unwrap();
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO oauth_grants(id,account_id,oauth_application_id,client_kind,household_membership_id,person_id,permissions_version,redirect_uri,scopes,token_hash,expires_in,authenticated_at,last_used_at,device_name,created_at,updated_at) VALUES(76003,71001,75003,'integration',74001,73001,1,'https://example.test/callback','launch/patient patient/*.rs offline_access',$1,now()+interval '15 minutes',now(),now(),'SMART profile',now(),now())",[digest.into()])).await.unwrap();
    fixture.admin.execute_unprepared("UPDATE oauth_grants SET access_token_scopes='patient/*.read',access_token_scope_hash=token_hash WHERE id=76001").await.unwrap();
}

async fn state(fixture: &Fixture) -> Value {
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT (SELECT count(*) FROM versions WHERE item_type IN ('Account','Person')) AS versions,(SELECT count(*) FROM api_change_events WHERE record_type='Person') AS changes,(SELECT count(*) FROM api_idempotency_keys WHERE request_path LIKE '%/profile') AS keys,(SELECT count(*) FROM security_audit_events WHERE metadata->>'controller'='api/v1/profiles') AS audits,(SELECT date_of_birth::text FROM people WHERE id=73001) AS birth,(SELECT preferences::text FROM accounts WHERE id=71001) AS preferences")).await.unwrap().unwrap();
    json!({"versions":row.try_get::<i64>("","versions").unwrap(),"changes":row.try_get::<i64>("","changes").unwrap(),"keys":row.try_get::<i64>("","keys").unwrap(),"audits":row.try_get::<i64>("","audits").unwrap(),"birth":row.try_get::<Option<String>>("","birth").unwrap(),"preferences":row.try_get::<String>("","preferences").unwrap()})
}

#[test]
fn profile_write_methods_document_existing_keyed_conflict() {
    let document = contract::contract();
    let mut missing = Vec::new();
    for method in ["patch", "put"] {
        let operation = &document["paths"][PROFILE][method];
        if operation["responses"]["409"]["$ref"] != "#/components/responses/Conflict" {
            missing.push(format!("{method}: Conflict 409"));
        }
        if !operation["parameters"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|parameter| parameter["$ref"] == "#/components/parameters/idempotency_key")
        {
            missing.push(format!("{method}: Idempotency-Key"));
        }
    }
    assert!(
        missing.is_empty(),
        "missing profile idempotency contract: {missing:?}"
    );
}

#[tokio::test]
async fn profile_reads_scope_real_credentials_and_serialize_private_response() {
    let (fixture, boot) = boot_fixture().await;
    seed_credentials(&fixture).await;
    let mobile = mobile_token(&fixture).await;
    let server = server(&boot);
    let missing_birth = send(
        &server,
        Method::GET,
        72001,
        Some("profile-session"),
        None,
        None,
    )
    .await;
    fixture
        .admin
        .execute_unprepared("UPDATE people SET date_of_birth='1990-01-01' WHERE id=73001")
        .await
        .unwrap();
    let mut replies = Vec::new();
    for (name, token) in [
        ("api", "profile-session"),
        ("app", "profile-app"),
        ("mobile", mobile.as_str()),
    ] {
        for (household, expected) in [(72001, 200), (72004, 403), (99999, 404)] {
            let response = send(&server, Method::GET, household, Some(token), None, None).await;
            replies.push((name, household, expected, response));
        }
    }
    let anonymous = send(&server, Method::GET, 72001, None, None, None).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(missing_birth.status, 422);
    assert_eq!(missing_birth.body["error"]["code"], "unprocessable_content");
    assert_wire(&missing_birth);
    assert_schema("get", &missing_birth);
    assert_eq!(anonymous.status, 401);
    assert_wire(&anonymous);
    assert_schema("get", &anonymous);
    for (name, household, expected, response) in replies {
        assert_eq!(
            response.status, expected,
            "{name} {household}: {}",
            response.body
        );
        assert_wire(&response);
        assert_schema("get", &response);
        if expected == 200 {
            assert_eq!(response.body["data"]["account_id"], "71001");
            assert_eq!(response.body["data"]["person_id"], "73001");
        } else {
            assert!(response.body.get("data").is_none());
        }
    }
}

#[tokio::test]
async fn profile_invalid_and_revoked_credentials_do_not_reveal_household() {
    let (fixture, boot) = boot_fixture().await;
    seed_credentials(&fixture).await;
    let mobile = mobile_token(&fixture).await;
    fixture.admin.execute_unprepared("UPDATE api_sessions SET revoked_at=now() WHERE id=88031; UPDATE api_app_tokens SET revoked_at=now() WHERE id=88032; UPDATE oauth_grants SET revoked_at=now() WHERE id=76001").await.unwrap();
    let server = server(&boot);
    let body = json!({"profile":{"time_zone":"Europe/London"}});
    let mut replies = Vec::new();
    for bearer in [
        "profile-session",
        "profile-app",
        mobile.as_str(),
        "invalid-profile-token",
    ] {
        for method in [Method::GET, Method::PATCH, Method::PUT] {
            for household in [72001, 99999] {
                replies.push((
                    method.as_str().to_lowercase(),
                    send(
                        &server,
                        method.clone(),
                        household,
                        Some(bearer),
                        (method != Method::GET).then_some(&body),
                        None,
                    )
                    .await,
                ));
            }
        }
    }
    drop(server);
    close(fixture, boot).await;
    for (method, response) in replies {
        assert_eq!(response.status, 401, "{method}");
        assert_eq!(response.challenge.as_deref(), Some("Bearer"));
        assert_wire(&response);
        assert_schema(&method, &response);
        assert!(response.body.get("data").is_none());
    }
}

#[tokio::test]
async fn profile_rejects_smart_namespace_and_insufficient_mobile_scope() {
    let (fixture, boot) = boot_fixture().await;
    seed_credentials(&fixture).await;
    let mobile = mobile_token(&fixture).await;
    let smart = "synthetic-profile-smart";
    seed_disallowed_grants(&fixture, smart).await;
    let before = state(&fixture).await;
    let server = server(&boot);
    let body = json!({"profile":{"time_zone":"Europe/London"}});
    let mut replies = Vec::new();
    for method in [Method::GET, Method::PATCH, Method::PUT] {
        for (name, bearer, expected) in [
            ("smart", smart, 401),
            ("mobile_scope", mobile.as_str(), 403),
        ] {
            let response = send(
                &server,
                method.clone(),
                72004,
                Some(bearer),
                (method != Method::GET).then_some(&body),
                None,
            )
            .await;
            replies.push((method.as_str().to_lowercase(), name, expected, response));
        }
    }
    let after = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(before, after);
    for (method, name, expected, response) in replies {
        assert_eq!(response.status, expected, "{method} {name}");
        assert_eq!(
            response.challenge.as_deref(),
            Some(if expected == 403 {
                "Bearer error=\"insufficient_scope\" scope=\"medtracker\""
            } else {
                "Bearer"
            })
        );
        assert_wire(&response);
        assert_schema(&method, &response);
        assert!(response.body.get("data").is_none());
    }
}

#[tokio::test]
async fn profile_writes_save_allowed_fields_and_reject_security_fields_atomically() {
    let (fixture, boot) = boot_fixture().await;
    seed_credentials(&fixture).await;
    fixture
        .admin
        .execute_unprepared("UPDATE people SET date_of_birth='1990-01-01' WHERE id=73001")
        .await
        .unwrap();
    let mobile = mobile_token(&fixture).await;
    let server = server(&boot);
    let mut results = Vec::new();
    for (method, bearer, zone) in [
        (Method::PATCH, "profile-session", "Europe/London"),
        (Method::PUT, "profile-app", "UTC"),
        (Method::PATCH, mobile.as_str(), "America/New_York"),
    ] {
        let body = json!({"profile":{"time_zone":zone}});
        let response = send(
            &server,
            method.clone(),
            72001,
            Some(bearer),
            Some(&body),
            None,
        )
        .await;
        results.push((method.as_str().to_lowercase(), zone, response));
    }
    let before = state(&fixture).await;
    let invalid = json!({"profile":{"time_zone":"Europe/Paris","role":"owner"}});
    let mut rejected = Vec::new();
    for method in [Method::PATCH, Method::PUT] {
        rejected.push((
            method.as_str().to_lowercase(),
            send(&server, method, 72001, Some(&mobile), Some(&invalid), None).await,
        ));
    }
    let after = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    for (method, zone, response) in results {
        assert_eq!(response.status, 200, "{method}");
        assert_wire(&response);
        assert_schema(&method, &response);
        assert_eq!(response.body["data"]["time_zone"], zone);
    }
    let mut expected = before.clone();
    expected["audits"] = json!(before["audits"].as_i64().unwrap() + 2);
    assert_eq!(expected, after);
    for (method, response) in rejected {
        assert_eq!(response.status, 422, "{method}");
        assert_wire(&response);
        assert_schema(&method, &response);
        assert!(response.body.get("data").is_none());
    }
}

async fn keyed_profile_write(method: Method) {
    let (fixture, boot) = boot_fixture().await;
    seed_credentials(&fixture).await;
    fixture
        .admin
        .execute_unprepared("UPDATE people SET date_of_birth='1990-01-01' WHERE id=73001")
        .await
        .unwrap();
    let server = server(&boot);
    let method_name = method.as_str().to_lowercase();
    let body = json!({"profile":{"time_zone":"Europe/London"}});
    let changed = json!({"profile":{"time_zone":"UTC"}});
    let key = format!("profile-{method}-key");
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
    let conflict = send(
        &server,
        method,
        72001,
        Some("profile-app"),
        Some(&changed),
        Some(&key),
    )
    .await;
    let after_conflict = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    assert_eq!(first.status, 200);
    assert_eq!(replay.status, 200);
    assert_eq!(conflict.status, 409);
    assert_eq!(replay.replayed.as_deref(), Some("true"));
    assert_eq!(replay.body, first.body);
    assert_eq!(after_replay["versions"], after_first["versions"]);
    assert_eq!(after_replay["changes"], after_first["changes"]);
    assert_eq!(after_conflict["versions"], after_first["versions"]);
    assert_eq!(after_conflict["changes"], after_first["changes"]);
    for response in [&first, &replay, &conflict] {
        assert_wire(response);
        assert_schema(&method_name, response);
    }
    assert_eq!(conflict.body["error"]["code"], "idempotency_key_reused");
}

#[tokio::test]
async fn patch_profile_replays_exact_key_and_rejects_changed_body() {
    keyed_profile_write(Method::PATCH).await;
}

#[tokio::test]
async fn put_profile_replays_exact_key_and_rejects_changed_body() {
    keyed_profile_write(Method::PUT).await;
}

#[tokio::test]
async fn profile_permissions_and_membership_recheck_before_mutation() {
    let (fixture, boot) = boot_fixture().await;
    seed_credentials(&fixture).await;
    fixture.admin.execute_unprepared("UPDATE people SET date_of_birth='1990-01-01' WHERE id=73001; UPDATE person_access_grants SET access_level='view' WHERE id=78001").await.unwrap();
    let mobile = mobile_token(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let body = json!({"profile":{"time_zone":"Europe/London"}});
    let mut replies = Vec::new();
    for method in [Method::PATCH, Method::PUT] {
        for bearer in ["profile-session", "profile-app", mobile.as_str()] {
            replies.push((
                method.as_str().to_lowercase(),
                send(
                    &server,
                    method.clone(),
                    72001,
                    Some(bearer),
                    Some(&body),
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
    for method in [Method::PATCH, Method::PUT] {
        replies.push((
            method.as_str().to_lowercase(),
            send(&server, method, 72001, Some(&mobile), Some(&body), None).await,
        ));
    }
    let after = state(&fixture).await;
    drop(server);
    close(fixture, boot).await;
    let mut expected = before.clone();
    expected["audits"] = json!(before["audits"].as_i64().unwrap() + 6);
    assert_eq!(expected, after);
    for (method, response) in replies {
        assert_eq!(response.status, 403, "{method}");
        assert_wire(&response);
        assert_schema(&method, &response);
        assert!(response.body.get("data").is_none());
    }
}

#[tokio::test]
async fn profile_rate_limit_rejects_without_mutation() {
    let (fixture, boot) = boot_fixture_with(|config| {
        config.settings.get_or_insert_with(|| json!({}))["api_rate_limit"] =
            json!({"fixed_now_unix_seconds":1000,"trusted_proxy_ips":""});
    })
    .await;
    seed_credentials(&fixture).await;
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
    let body = json!({"profile":{"time_zone":"Europe/London"}});
    let mut denials = Vec::new();
    for method in [Method::GET, Method::PATCH, Method::PUT] {
        denials.push((
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
    assert_eq!(after, before);
    for (method, denied) in denials {
        assert_eq!(denied.status, 429, "{method}");
        assert_eq!(denied.retry_after.as_deref(), Some("200"));
        assert_eq!(denied.limit.as_deref(), Some("300"));
        assert_eq!(denied.remaining.as_deref(), Some("0"));
        assert_eq!(denied.reset.as_deref(), Some("1200"));
        assert_wire(&denied);
        assert_schema(&method, &denied);
        assert!(denied.body.get("data").is_none());
    }
}

#[tokio::test]
async fn profile_audit_failure_hides_read_and_rolls_back_writes() {
    let (fixture, boot) = boot_fixture().await;
    seed_credentials(&fixture).await;
    fixture.admin.execute_unprepared("UPDATE people SET date_of_birth='1990-01-01' WHERE id=73001; CREATE FUNCTION reject_profile_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='api.request' AND NEW.metadata->>'controller'='api/v1/profiles' THEN RAISE EXCEPTION 'Synthetic profile audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_profile_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION reject_profile_audit()").await.unwrap();
    let server = server(&boot);
    let before = state(&fixture).await;
    let body = json!({"profile":{"time_zone":"Europe/London"}});
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
    assert_eq!(after, before);
    assert_eq!(
        replies
            .iter()
            .map(|(method, response)| (method.as_str(), response.status))
            .collect::<Vec<_>>(),
        [("get", 500), ("patch", 500), ("put", 500)]
    );
    for (_, response) in &replies {
        assert_eq!(response.body["error"]["code"], "internal_error");
        assert!(response.body.get("data").is_none());
        assert_wire(response);
    }
    for (method, response) in replies {
        assert_schema(&method, &response);
    }
}
