use super::*;
use axum::http::Method;
use med_tracker::models::_entities::{
    accounts, api_change_events, api_idempotency_keys, people, security_audit_events, versions,
};
use sea_orm::{ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

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

pub(super) async fn mobile_token(fixture: &RequestFixture) -> String {
    fixture.token().await
}
pub(super) async fn seed_credentials(fixture: &RequestFixture) {
    fixture
        .session(&fixture.credential("profile-session"))
        .await;
    fixture.app_token(&fixture.credential("profile-app")).await;
}
pub(super) async fn seed_disallowed_grants(fixture: &RequestFixture, smart: &str) {
    fixture.integration(smart).await;
    fixture.wrong_scope().await;
}

fn own_versions(fixture: &RequestFixture) -> sea_orm::Select<versions::Entity> {
    versions::Entity::find().filter(
        Condition::any()
            .add(
                Condition::all()
                    .add(versions::Column::ItemType.eq("Account"))
                    .add(versions::Column::ItemId.eq(fixture.account.id)),
            )
            .add(
                Condition::all()
                    .add(versions::Column::ItemType.eq("Person"))
                    .add(versions::Column::ItemId.eq(fixture.account.person_id)),
            ),
    )
}

pub(super) async fn state(fixture: &RequestFixture) -> Value {
    let versions = own_versions(fixture).count(&fixture.admin).await.unwrap();
    let changes = api_change_events::Entity::find()
        .filter(api_change_events::Column::HouseholdId.eq(fixture.household.id))
        .filter(api_change_events::Column::RecordType.eq("Person"))
        .count(&fixture.admin)
        .await
        .unwrap();
    let keys = api_idempotency_keys::Entity::find()
        .filter(api_idempotency_keys::Column::AccountId.eq(fixture.account.id))
        .filter(api_idempotency_keys::Column::RequestPath.ends_with("/profile"))
        .count(&fixture.admin)
        .await
        .unwrap();
    let audits = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(fixture.household.id))
        .all(&fixture.admin)
        .await
        .unwrap()
        .into_iter()
        .filter(|event| event.metadata["controller"] == "api/v1/profiles")
        .count();
    let person = people::Entity::find_by_id(fixture.account.person_id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let account = accounts::Entity::find_by_id(fixture.account.id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    json!({"versions":versions,"changes":changes,"keys":keys,"audits":audits,
        "birth":person.date_of_birth.map(|date| date.to_string()),"preferences":account.preferences.to_string()})
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
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;
    let foreign_id = fixture.foreign.household.id;
    let account_id = fixture.account.id;
    let person_id = fixture.account.person_id;
    let session = fixture.credential("profile-session");
    let app = fixture.credential("profile-app");
    seed_credentials(&fixture).await;
    let mobile = mobile_token(&fixture).await;
    fixture.birth(None).await;
    let server = server(&boot);
    let missing_birth = send(
        &server,
        Method::GET,
        household_id,
        Some(session.as_str()),
        None,
        None,
    )
    .await;
    fixture.birth(Some("1990-01-01".parse().unwrap())).await;
    let mut replies = Vec::new();
    for (name, token) in [
        ("api", session.as_str()),
        ("app", app.as_str()),
        ("mobile", mobile.as_str()),
    ] {
        for (household, expected) in [(household_id, 200), (foreign_id, 403), (i64::MAX, 404)] {
            let response = send(&server, Method::GET, household, Some(token), None, None).await;
            replies.push((name, household, expected, response));
        }
    }
    let anonymous = send(&server, Method::GET, household_id, None, None, None).await;
    drop(server);
    close_request(fixture, boot).await;
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
            assert_eq!(response.body["data"]["account_id"], account_id.to_string());
            assert_eq!(response.body["data"]["person_id"], person_id.to_string());
        } else {
            assert!(response.body.get("data").is_none());
        }
    }
}

#[tokio::test]
async fn profile_invalid_and_revoked_credentials_do_not_reveal_household() {
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;

    let session = fixture.credential("profile-session");
    let app = fixture.credential("profile-app");
    seed_credentials(&fixture).await;
    let mobile = mobile_token(&fixture).await;
    fixture.revoke_all().await;
    let server = server(&boot);
    let body = json!({"profile":{"time_zone":"Europe/London"}});
    let mut replies = Vec::new();
    for bearer in [
        session.as_str(),
        app.as_str(),
        mobile.as_str(),
        "invalid-profile-token",
    ] {
        for method in [Method::GET, Method::PATCH, Method::PUT] {
            for household in [household_id, i64::MAX] {
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
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_fixture().await;

    let foreign_id = fixture.foreign.household.id;

    seed_credentials(&fixture).await;
    let mobile = mobile_token(&fixture).await;
    let smart = fixture.credential("synthetic-profile-smart");
    seed_disallowed_grants(&fixture, &smart).await;
    let before = state(&fixture).await;
    let server = server(&boot);
    let body = json!({"profile":{"time_zone":"Europe/London"}});
    let mut replies = Vec::new();
    for method in [Method::GET, Method::PATCH, Method::PUT] {
        for (name, bearer, expected) in [
            ("smart", smart.as_str(), 401),
            ("mobile_scope", mobile.as_str(), 403),
        ] {
            let response = send(
                &server,
                method.clone(),
                foreign_id,
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
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;

    let session = fixture.credential("profile-session");
    let app = fixture.credential("profile-app");
    seed_credentials(&fixture).await;
    fixture.birth(Some("1990-01-01".parse().unwrap())).await;
    let mobile = mobile_token(&fixture).await;
    let server = server(&boot);
    let mut results = Vec::new();
    for (method, bearer, zone) in [
        (Method::PATCH, session.as_str(), "Europe/London"),
        (Method::PUT, app.as_str(), "UTC"),
        (Method::PATCH, mobile.as_str(), "America/New_York"),
    ] {
        let body = json!({"profile":{"time_zone":zone}});
        let response = send(
            &server,
            method.clone(),
            household_id,
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
            send(
                &server,
                method,
                household_id,
                Some(&mobile),
                Some(&invalid),
                None,
            )
            .await,
        ));
    }
    let after = state(&fixture).await;
    drop(server);
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;

    let session = fixture.credential("profile-session");
    let app = fixture.credential("profile-app");
    seed_credentials(&fixture).await;
    fixture.birth(Some("1990-01-01".parse().unwrap())).await;
    let server = server(&boot);
    let method_name = method.as_str().to_lowercase();
    let body = json!({"profile":{"time_zone":"Europe/London"}});
    let changed = json!({"profile":{"time_zone":"UTC"}});
    let key = format!("profile-{method}-key");
    let first = send(
        &server,
        method.clone(),
        household_id,
        Some(session.as_str()),
        Some(&body),
        Some(&key),
    )
    .await;
    let after_first = state(&fixture).await;
    let replay = send(
        &server,
        method.clone(),
        household_id,
        Some(app.as_str()),
        Some(&body),
        Some(&key),
    )
    .await;
    let after_replay = state(&fixture).await;
    let conflict = send(
        &server,
        method,
        household_id,
        Some(app.as_str()),
        Some(&changed),
        Some(&key),
    )
    .await;
    let after_conflict = state(&fixture).await;
    drop(server);
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;

    let account_id = fixture.account.id;
    let person_id = fixture.account.person_id;
    let session = fixture.credential("profile-session");
    let app = fixture.credential("profile-app");
    seed_credentials(&fixture).await;
    fixture.birth(Some("1990-01-01".parse().unwrap())).await;
    fixture.view_grant().await;
    let mobile = mobile_token(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let body = json!({"profile":{"time_zone":"Europe/London"}});
    let mut self_replies = Vec::new();
    for method in [Method::PATCH, Method::PUT] {
        for bearer in [session.as_str(), app.as_str(), mobile.as_str()] {
            self_replies.push((
                method.as_str().to_lowercase(),
                send(
                    &server,
                    method.clone(),
                    household_id,
                    Some(bearer),
                    Some(&body),
                    None,
                )
                .await,
            ));
        }
    }
    let after_self = state(&fixture).await;
    let version = own_versions(&fixture)
        .order_by_desc(versions::Column::Id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let saved_version = json!({"item_type":version.item_type,"item_id":version.item_id,"event":version.event,
        "object":version.object.map(|object| serde_json::from_str::<Value>(&object).unwrap()),
        "object_changes":version.object_changes.map(|object| serde_json::from_str::<Value>(&object).unwrap()),
        "request_id":version.request_id});
    fixture
        .membership_state(fixture.membership.permissions_version, true)
        .await;
    let before_revoked = state(&fixture).await;
    let revoked_body = json!({"profile":{"time_zone":"UTC"}});
    let mut revoked_replies = Vec::new();
    for method in [Method::PATCH, Method::PUT] {
        revoked_replies.push((
            method.as_str().to_lowercase(),
            send(
                &server,
                method,
                household_id,
                Some(&mobile),
                Some(&revoked_body),
                None,
            )
            .await,
        ));
    }
    let after_revoked = state(&fixture).await;
    drop(server);
    close_request(fixture, boot).await;
    assert_eq!(self_replies.len(), 6);
    for (method, response) in &self_replies {
        assert_eq!(response.status, 200, "{method}: {}", response.body);
        assert_wire(response);
        assert_schema(method, response);
        assert_eq!(response.body["data"]["account_id"], account_id.to_string());
        assert_eq!(response.body["data"]["person_id"], person_id.to_string());
        assert_eq!(response.body["data"]["time_zone"], "Europe/London");
        assert_eq!(response.body["data"]["date_of_birth"], "1990-01-01");
    }
    let before_preferences: Value =
        serde_json::from_str(before["preferences"].as_str().unwrap()).unwrap();
    let mut expected_preferences = before_preferences.clone();
    expected_preferences["time_zone"] = json!("Europe/London");
    let mut expected = before.clone();
    expected["versions"] = json!(before["versions"].as_i64().unwrap() + 1);
    expected["audits"] = json!(before["audits"].as_i64().unwrap() + 6);
    expected["preferences"] = expected_preferences.clone();
    let mut actual = after_self.clone();
    actual["preferences"] = serde_json::from_str(actual["preferences"].as_str().unwrap()).unwrap();
    assert_eq!(expected, actual);
    assert_eq!(
        saved_version,
        json!({
            "item_type": "Account",
            "item_id": account_id,
            "event": "update",
            "object": {"preferences": before_preferences.clone()},
            "object_changes": {"preferences": [before_preferences, expected_preferences]},
            "request_id": self_replies.first().unwrap().1.request_id,
        })
    );
    assert_eq!(after_self, before_revoked);
    assert_eq!(before_revoked, after_revoked);
    assert_eq!(revoked_replies.len(), 2);
    for (method, response) in revoked_replies {
        assert_eq!(response.status, 403, "{method}: {}", response.body);
        assert_wire(&response);
        assert_schema(&method, &response);
        assert!(response.body.get("data").is_none());
    }
}

#[tokio::test]
async fn profile_rate_limit_rejects_without_mutation() {
    let (fixture, boot) = boot_request_fixture_with(|config| {
        config.settings.get_or_insert_with(|| json!({}))["api_rate_limit"] =
            json!({"fixed_now_unix_seconds":1000,"trusted_proxy_ips":""});
    })
    .await;
    let household_id = fixture.household.id;

    let session = fixture.credential("profile-session");

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
                household_id,
                Some(session.as_str()),
                (method != Method::GET).then_some(&body),
                None,
            )
            .await,
        ));
    }
    let after = state(&fixture).await;
    drop(server);
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;

    let session = fixture.credential("profile-session");

    seed_credentials(&fixture).await;
    fixture.birth(Some("1990-01-01".parse().unwrap())).await;
    fixture.admin.execute_unprepared("CREATE FUNCTION reject_profile_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='api.request' AND NEW.metadata->>'controller'='api/v1/profiles' THEN RAISE EXCEPTION 'Synthetic profile audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_profile_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION reject_profile_audit()").await.unwrap();
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
                household_id,
                Some(session.as_str()),
                (method != Method::GET).then_some(&body),
                None,
            )
            .await,
        ));
    }
    let after = state(&fixture).await;
    fixture.admin.execute_unprepared("DROP TRIGGER reject_profile_audit ON security_audit_events; DROP FUNCTION reject_profile_audit()").await.unwrap();
    drop(server);
    close_request(fixture, boot).await;
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
