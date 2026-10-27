use super::*;
use axum::http::Method;
use med_tracker::models::_entities::{
    api_change_events, api_idempotency_keys, households, security_audit_events, versions,
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};

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

async fn malformed(fixture: &RequestFixture, server: &TestServer, method: Method) -> Reply {
    let response = server
        .method(
            method,
            &format!("/api/v1/households/{}/admin/settings", fixture.household.id),
        )
        .add_header(
            "authorization",
            format!("Bearer {}", fixture.credential("profile-session")),
        )
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

async fn state(fixture: &RequestFixture) -> Value {
    let versions = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(fixture.household.id))
        .filter(versions::Column::ItemType.eq("Household"))
        .filter(versions::Column::Event.eq("update"))
        .count(&fixture.admin)
        .await
        .unwrap();
    let audits = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(fixture.household.id))
        .filter(security_audit_events::Column::EventType.eq("api/admin/household_settings/updated"))
        .count(&fixture.admin)
        .await
        .unwrap();
    let keys = api_idempotency_keys::Entity::find()
        .filter(api_idempotency_keys::Column::HouseholdId.eq(fixture.household.id))
        .filter(api_idempotency_keys::Column::RequestPath.ends_with("/admin/settings"))
        .count(&fixture.admin)
        .await
        .unwrap();
    let changes = api_change_events::Entity::find()
        .filter(api_change_events::Column::HouseholdId.eq(fixture.household.id))
        .filter(api_change_events::Column::RecordType.eq("Household"))
        .count(&fixture.admin)
        .await
        .unwrap();
    let household = households::Entity::find_by_id(fixture.household.id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    json!({"versions":versions,"audits":audits,"keys":keys,"changes":changes,
        "name":household.name,"timezone":household.timezone,"plan":household.subscription_plan,"slug":household.slug})
}

#[tokio::test]
async fn settings_reads_scope_three_credential_kinds_and_validate_responses() {
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;
    let foreign_id = fixture.foreign.household.id;
    let account_id = fixture.account.id;
    let membership_id = fixture.membership.id;
    let user_id = fixture.user.id;
    let slug = fixture.household.slug.clone();
    let session = fixture.credential("profile-session");
    let app = fixture.credential("profile-app");

    profile::seed_credentials(&fixture).await;
    let session_reference = format!(
        "api_session:{}",
        fixture.session_id("profile-session").await
    );
    let app_reference = format!(
        "api_app_token:{}",
        fixture.app_token_id("profile-app").await
    );
    let oauth_reference = format!("oauth_grant:{}", fixture.authorization.id);
    let mobile = profile::mobile_token(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let mut replies = Vec::new();
    for (name, token) in [
        ("api", session.as_str()),
        ("app", app.as_str()),
        ("mobile", mobile.as_str()),
    ] {
        for (household, expected) in [(household_id, 200), (foreign_id, 403), (i64::MAX, 404)] {
            replies.push((
                name,
                household,
                expected,
                send(&server, Method::GET, household, Some(token), None, None).await,
            ));
        }
    }
    let anonymous = send(&server, Method::GET, household_id, None, None, None).await;
    let audits = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(fixture.household.id))
        .filter(security_audit_events::Column::EventType.eq("api.request"))
        .all(&fixture.admin)
        .await
        .unwrap();
    let audits: Vec<Value> = audits.into_iter()
        .filter(|row| row.metadata["controller"] == "api/v1/admin/settings" && row.metadata["action"] == "show"
            && row.metadata["status"].to_string().trim_matches('"') == "200")
        .map(|row| json!({"request_id":row.request_id,"account":row.actor_account_id,
            "membership":row.actor_membership_id,"metadata":row.metadata,"context":row.audit_context})).collect();
    let after = state(&fixture).await;
    drop(server);
    close_request(fixture, boot).await;
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
            assert_eq!(reply.body["data"]["id"], household_id);
            assert_eq!(reply.body["data"]["slug"], slug.as_str());
            assert_eq!(reply.body["data"]["subscription_plan"], "free");
            let audit = audits
                .iter()
                .find(|row| row["request_id"] == reply.request_id.as_deref().unwrap())
                .unwrap();
            let (method, reference) = match name {
                "api" => ("api_session", session_reference.as_str()),
                "app" => ("api_app_token", app_reference.as_str()),
                _ => ("oauth", oauth_reference.as_str()),
            };
            assert_eq!(
                (audit["account"].as_i64(), audit["membership"].as_i64()),
                (Some(account_id), Some(membership_id))
            );
            assert_eq!(audit["metadata"]["http_method"], "GET");
            assert_eq!(audit["metadata"]["outcome"], "success");
            assert_eq!(audit["context"]["actor_user_id"], user_id);
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
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;

    let session = fixture.credential("profile-session");
    let app = fixture.credential("profile-app");
    let smart = fixture.credential("smart-settings");
    profile::seed_credentials(&fixture).await;

    let mobile = profile::mobile_token(&fixture).await;
    profile::seed_disallowed_grants(&fixture, &fixture.credential("smart-settings")).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let body = json!({"household":{"name":"Denied settings"}});
    let mut replies = Vec::new();
    for method in [Method::GET, Method::PATCH, Method::PUT] {
        for (name, token, expected) in [
            ("invalid", "invalid-settings", 401),
            ("smart", smart.as_str(), 401),
            ("wrong-scope-mobile", mobile.as_str(), 403),
        ] {
            replies.push((
                method.as_str().to_lowercase(),
                name,
                expected,
                send(
                    &server,
                    method.clone(),
                    household_id,
                    Some(token),
                    (method != Method::GET).then_some(&body),
                    None,
                )
                .await,
            ));
        }
    }
    fixture.revoke_all().await;
    for method in [Method::GET, Method::PATCH, Method::PUT] {
        for token in [session.as_str(), app.as_str(), mobile.as_str()] {
            replies.push((
                method.as_str().to_lowercase(),
                "revoked",
                401,
                send(
                    &server,
                    method.clone(),
                    household_id,
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
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_fixture().await;

    let foreign_id = fixture.foreign.household.id;

    let session = fixture.credential("profile-session");
    let app = fixture.credential("profile-app");

    profile::seed_credentials(&fixture).await;

    let mobile = profile::mobile_token(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let body = json!({"household":{"name":"Must not change"}});
    let mut replies = Vec::new();
    for method in [Method::PATCH, Method::PUT] {
        for token in [session.as_str(), app.as_str(), mobile.as_str()] {
            for (household, expected) in [(foreign_id, 403), (i64::MAX, 404)] {
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
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;

    let session = fixture.credential("profile-session");
    let app = fixture.credential("profile-app");

    profile::seed_credentials(&fixture).await;

    let mobile = profile::mobile_token(&fixture).await;
    fixture
        .membership_state(fixture.membership.permissions_version + 1, false)
        .await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let body = json!({"household":{"name":"Stale settings"}});
    let mut replies = Vec::new();
    for method in [Method::GET, Method::PATCH, Method::PUT] {
        for token in [session.as_str(), app.as_str()] {
            replies.push((
                method.as_str().to_lowercase(),
                401,
                send(
                    &server,
                    method.clone(),
                    household_id,
                    Some(token),
                    (method != Method::GET).then_some(&body),
                    None,
                )
                .await,
            ));
        }
    }
    fixture
        .membership_state(fixture.membership.permissions_version, true)
        .await;
    for method in [Method::GET, Method::PATCH, Method::PUT] {
        replies.push((
            method.as_str().to_lowercase(),
            403,
            send(
                &server,
                method.clone(),
                household_id,
                Some(&mobile),
                (method != Method::GET).then_some(&body),
                None,
            )
            .await,
        ));
    }
    let after = state(&fixture).await;
    drop(server);
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;

    let account_id = fixture.account.id;
    let membership_id = fixture.membership.id;
    let user_id = fixture.user.id;

    let session = fixture.credential("profile-session");
    let app = fixture.credential("profile-app");

    profile::seed_credentials(&fixture).await;
    let session_reference = format!(
        "api_session:{}",
        fixture.session_id("profile-session").await
    );
    let app_reference = format!(
        "api_app_token:{}",
        fixture.app_token_id("profile-app").await
    );
    let oauth_reference = format!("oauth_grant:{}", fixture.authorization.id);
    let mobile = profile::mobile_token(&fixture).await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let mut replies = Vec::new();
    for (method, token, body, field, old, new, auth, reference) in [
        (
            Method::PATCH,
            session.as_str(),
            json!({"household":{"name":"Settings one"}}),
            "name",
            "Synthetic household",
            "Settings one",
            "api_session",
            session_reference.as_str(),
        ),
        (
            Method::PUT,
            app.as_str(),
            json!({"household":{"timezone":"UTC"}}),
            "timezone",
            "Europe/London",
            "UTC",
            "api_app_token",
            app_reference.as_str(),
        ),
        (
            Method::PATCH,
            mobile.as_str(),
            json!({"household":{"subscription_plan":"family_plus"}}),
            "subscription_plan",
            "free",
            "family_plus",
            "oauth",
            oauth_reference.as_str(),
        ),
        (
            Method::PUT,
            session.as_str(),
            json!({"household":{"name":"Settings two"}}),
            "name",
            "Settings one",
            "Settings two",
            "api_session",
            session_reference.as_str(),
        ),
        (
            Method::PATCH,
            app.as_str(),
            json!({"household":{"name":"Settings three"}}),
            "name",
            "Settings two",
            "Settings three",
            "api_app_token",
            app_reference.as_str(),
        ),
        (
            Method::PUT,
            mobile.as_str(),
            json!({"household":{"name":"Settings four"}}),
            "name",
            "Settings three",
            "Settings four",
            "oauth",
            oauth_reference.as_str(),
        ),
    ] {
        replies.push((
            method.as_str().to_lowercase(),
            field,
            old,
            new,
            auth,
            reference,
            send(
                &server,
                method,
                household_id,
                Some(token),
                Some(&body),
                None,
            )
            .await,
        ));
    }
    let after = state(&fixture).await;
    let rows = versions::Entity::find()
        .filter(versions::Column::ItemType.eq("Household"))
        .filter(versions::Column::ItemId.eq(fixture.household.id))
        .filter(versions::Column::Event.eq("update"))
        .order_by_asc(versions::Column::Id)
        .all(&fixture.admin)
        .await
        .unwrap();
    let events = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(fixture.household.id))
        .all(&fixture.admin)
        .await
        .unwrap();
    let mut proofs = Vec::new();
    for row in rows {
        for audit in events.iter().filter(|event| {
            event.request_id == row.request_id
                && event.event_type == "api/admin/household_settings/updated"
        }) {
            for request in events.iter().filter(|event| {
                event.request_id == row.request_id && event.event_type == "api.request"
            }) {
                proofs.push(json!({"request_id":row.request_id,"membership":row.actor_membership_id,"user":row.whodunnit,
                    "before":serde_json::from_str::<Value>(row.object.as_deref().unwrap()).unwrap(),
                    "changes":serde_json::from_str::<Value>(row.object_changes.as_deref().unwrap()).unwrap(),
                    "version_context":row.audit_context,"account":audit.actor_account_id,"audit_membership":audit.actor_membership_id,
                    "update_metadata":audit.metadata,"update_context":audit.audit_context,
                    "request_metadata":request.metadata,"request_context":request.audit_context}));
            }
        }
    }
    drop(server);
    close_request(fixture, boot).await;
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
        assert_eq!(proof["membership"], membership_id);
        assert_eq!(proof["audit_membership"], membership_id);
        assert_eq!(proof["account"], account_id);
        assert_eq!(proof["user"], user_id.to_string());
        assert_eq!(
            proof["update_metadata"],
            json!({"target_type":"Household","target_id":household_id,"outcome":"success"})
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
            assert_eq!(context["actor_account_id"], account_id);
            assert_eq!(context["actor_membership_id"], membership_id);
            assert_eq!(context["actor_user_id"], user_id);
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
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;

    let session = fixture.credential("profile-session");

    profile::seed_credentials(&fixture).await;

    let server = server(&boot);
    let before = state(&fixture).await;
    let mut replies = Vec::new();
    for method in [Method::PATCH, Method::PUT] {
        replies.push((
            method.as_str().to_lowercase(),
            400,
            malformed(&fixture, &server, method.clone()).await,
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
                    household_id,
                    Some(session.as_str()),
                    Some(&body),
                    None,
                )
                .await,
            ));
        }
    }
    let after = state(&fixture).await;
    drop(server);
    close_request(fixture, boot).await;
    assert_eq!(before, after);
    for (method, expected, reply) in replies {
        assert_eq!(reply.status, expected, "{method}: {}", reply.body);
        assert_wire(&reply);
        assert_schema(&method, &reply);
        assert!(reply.body.get("data").is_none());
    }
}

async fn keyed_write(method: Method) {
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;

    let session = fixture.credential("profile-session");
    let app = fixture.credential("profile-app");

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
    let mobile_replay = send(
        &server,
        method.clone(),
        household_id,
        Some(&mobile),
        Some(&body),
        Some(&key),
    )
    .await;
    let after_mobile_replay = state(&fixture).await;
    let conflict = send(
        &server,
        method.clone(),
        household_id,
        Some(app.as_str()),
        Some(&changed),
        Some(&key),
    )
    .await;
    let after_conflict = state(&fixture).await;
    fixture.member().await;
    let mut denied = Vec::new();
    for (name, token) in [
        ("api", session.as_str()),
        ("app", app.as_str()),
        ("mobile", mobile.as_str()),
    ] {
        denied.push((
            "demoted",
            name,
            403,
            send(
                &server,
                method.clone(),
                household_id,
                Some(token),
                Some(&body),
                Some(&key),
            )
            .await,
        ));
    }
    let after_demotion = state(&fixture).await;
    fixture
        .membership_state(fixture.membership.permissions_version, true)
        .await;
    for (name, token, expected) in [
        ("api", session.as_str(), 401),
        ("app", app.as_str(), 401),
        ("mobile", mobile.as_str(), 403),
    ] {
        denied.push((
            "withdrawn",
            name,
            expected,
            send(
                &server,
                method.clone(),
                household_id,
                Some(token),
                Some(&body),
                Some(&key),
            )
            .await,
        ));
    }
    let after_withdrawal = state(&fixture).await;
    fixture.revoke_all().await;
    for (name, token) in [
        ("api", session.as_str()),
        ("app", app.as_str()),
        ("mobile", mobile.as_str()),
    ] {
        denied.push((
            "revoked",
            name,
            401,
            send(
                &server,
                method.clone(),
                household_id,
                Some(token),
                Some(&body),
                Some(&key),
            )
            .await,
        ));
    }
    let after_revocation = state(&fixture).await;
    drop(server);
    close_request(fixture, boot).await;
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

    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;

    let session = fixture.credential("profile-session");
    let app = fixture.credential("profile-app");

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
    households::Entity::find_by_id(fixture.household.id)
        .lock_exclusive()
        .one(&blocker)
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
            household_id,
            Some(app.as_str()),
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
            household_id,
            Some(session.as_str()),
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
        household_id,
        Some(session.as_str()),
        Some(&changed),
        Some("settings-concurrent"),
    )
    .await;
    let after = state(&fixture).await;
    drop(server);
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;

    let session = fixture.credential("profile-session");
    let app = fixture.credential("profile-app");

    profile::seed_credentials(&fixture).await;

    let mobile = profile::mobile_token(&fixture).await;
    fixture.member().await;
    let server = server(&boot);
    let before = state(&fixture).await;
    let body = json!({"household":{"name":"Denied settings"}});
    let mut replies = Vec::new();
    for method in [Method::GET, Method::PATCH, Method::PUT] {
        for token in [session.as_str(), app.as_str(), mobile.as_str()] {
            replies.push((
                method.as_str().to_lowercase(),
                send(
                    &server,
                    method.clone(),
                    household_id,
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
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_rate_fixture(&[]).await;
    let household_id = fixture.household.id;
    let session = fixture.credential("profile-session");
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
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;

    let session = fixture.credential("profile-session");

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
                household_id,
                Some(session.as_str()),
                (method != Method::GET).then_some(&body),
                None,
            )
            .await,
        ));
    }
    let after = state(&fixture).await;
    fixture.admin.execute_unprepared("DROP TRIGGER reject_settings_request_audit ON security_audit_events; DROP FUNCTION reject_settings_request_audit()").await.unwrap();
    drop(server);
    close_request(fixture, boot).await;
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
