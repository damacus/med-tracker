use super::*;
#[path = "guardian_declarations.rs"]
mod guardian_declarations;
use med_tracker::models::_entities::{
    accounts, api_change_events, carer_relationships, household_memberships, location_memberships,
    people, person_access_grants, security_audit_events, users, versions,
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect};

use axum::http::Method;

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

async fn send_malformed(server: &TestServer, method: Method, path: &str, bearer: &str) -> Reply {
    let response = server
        .method(method, path)
        .add_header("authorization", format!("Bearer {bearer}"))
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

async fn mobile_token(fixture: &RequestFixture) -> String {
    fixture.token().await
}
async fn seed_credentials_and_foreign_household(fixture: &RequestFixture) {
    fixture.session(&fixture.credential("people-session")).await;
    fixture.app_token(&fixture.credential("people-app")).await;
}
async fn participants(fixture: &RequestFixture) -> Vec<i64> {
    household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(fixture.household.id))
        .select_only()
        .column(household_memberships::Column::PersonId)
        .into_tuple()
        .all(&fixture.admin)
        .await
        .unwrap()
}
async fn people_effect(fixture: &RequestFixture) -> (i64, i64, i64) {
    let ids = participants(fixture).await;
    let people = people::Entity::find()
        .filter(people::Column::Id.is_in(ids))
        .count(&fixture.admin)
        .await
        .unwrap();
    let versions = versions::Entity::find()
        .filter(versions::Column::HouseholdId.eq(fixture.household.id))
        .filter(versions::Column::ItemType.eq("Person"))
        .count(&fixture.admin)
        .await
        .unwrap();
    let changes = api_change_events::Entity::find()
        .filter(api_change_events::Column::HouseholdId.eq(fixture.household.id))
        .filter(api_change_events::Column::RecordType.eq("Person"))
        .count(&fixture.admin)
        .await
        .unwrap();
    (
        i64::try_from(people).unwrap(),
        i64::try_from(versions).unwrap(),
        i64::try_from(changes).unwrap(),
    )
}
async fn request_audits(fixture: &RequestFixture) -> i64 {
    let events = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(fixture.household.id))
        .filter(security_audit_events::Column::EventType.eq("api.request"))
        .all(&fixture.admin)
        .await
        .unwrap();
    i64::try_from(
        events
            .into_iter()
            .filter(|event| event.metadata["controller"] == "api/v1/people")
            .count(),
    )
    .unwrap()
}
async fn people_state(fixture: &RequestFixture) -> Value {
    let (people, versions, changes) = people_effect(fixture).await;
    let grants = person_access_grants::Entity::find()
        .filter(person_access_grants::Column::HouseholdId.eq(fixture.household.id))
        .count(&fixture.admin)
        .await
        .unwrap();
    let links = location_memberships::Entity::find()
        .filter(location_memberships::Column::HouseholdId.eq(fixture.household.id))
        .count(&fixture.admin)
        .await
        .unwrap();
    let events = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(fixture.household.id))
        .all(&fixture.admin)
        .await
        .unwrap();
    let audits = events
        .into_iter()
        .filter(|event| event.metadata["controller"] == "api/v1/people")
        .count();
    let membership = household_memberships::Entity::find_by_id(fixture.membership.id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let person = people::Entity::find_by_id(fixture.account.person_id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    json!({"people":people,"versions":versions,"changes":changes,"grants":grants,"links":links,"audits":audits,
        "permissions":membership.permissions_version,"name":person.name})
}
async fn dependent_effect(fixture: &RequestFixture) -> (i64, i64, i64, i64, i64, i64) {
    let ids = participants(fixture).await;
    let memberships = household_memberships::Entity::find()
        .filter(household_memberships::Column::HouseholdId.eq(fixture.household.id))
        .count(&fixture.admin)
        .await
        .unwrap();
    let declarations = guardian_declarations::Entity::find()
        .filter(guardian_declarations::Column::GuardianPersonId.eq(fixture.account.person_id))
        .filter(guardian_declarations::Column::DeclaredByAccountId.eq(fixture.account.id))
        .filter(guardian_declarations::Column::OriginHouseholdId.eq(fixture.household.id))
        .count(&fixture.admin)
        .await
        .unwrap();
    let carers = carer_relationships::Entity::find()
        .filter(carer_relationships::Column::HouseholdId.eq(fixture.household.id))
        .filter(carer_relationships::Column::CarerId.eq(fixture.account.person_id))
        .filter(carer_relationships::Column::GuardianDeclarationId.is_not_null())
        .filter(carer_relationships::Column::Active.eq(true))
        .count(&fixture.admin)
        .await
        .unwrap();
    let grants = person_access_grants::Entity::find()
        .filter(person_access_grants::Column::HouseholdId.eq(fixture.household.id))
        .count(&fixture.admin)
        .await
        .unwrap();
    let accounts = accounts::Entity::find()
        .filter(accounts::Column::PersonId.is_in(ids.clone()))
        .count(&fixture.admin)
        .await
        .unwrap();
    let users = users::Entity::find()
        .filter(users::Column::PersonId.is_in(ids))
        .count(&fixture.admin)
        .await
        .unwrap();
    (
        i64::try_from(memberships).unwrap(),
        i64::try_from(declarations).unwrap(),
        i64::try_from(carers).unwrap(),
        i64::try_from(grants).unwrap(),
        i64::try_from(accounts).unwrap(),
        i64::try_from(users).unwrap(),
    )
}
async fn assert_created_dependent(fixture: &RequestFixture, body: &Value) {
    let subject = body["data"]["id"]
        .as_i64()
        .or_else(|| body["data"]["id"].as_str().and_then(|id| id.parse().ok()))
        .expect("Successful dependent response must identify its Person");
    let account = accounts::Entity::find_by_id(fixture.account.id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let declarations = guardian_declarations::Entity::find()
        .filter(guardian_declarations::Column::SubjectPersonId.eq(subject))
        .filter(guardian_declarations::Column::GuardianPersonId.eq(fixture.account.person_id))
        .filter(guardian_declarations::Column::DeclaredByAccountId.eq(account.id))
        .filter(guardian_declarations::Column::OriginHouseholdId.eq(fixture.household.id))
        .filter(guardian_declarations::Column::StatementVersion.eq(1))
        .filter(guardian_declarations::Column::Revision.eq(1))
        .filter(guardian_declarations::Column::WithdrawnAt.is_null())
        .all(&fixture.admin)
        .await
        .unwrap();
    let mut linked = 0;
    for declaration in declarations {
        let members = household_memberships::Entity::find()
            .filter(household_memberships::Column::PersonId.eq(subject))
            .filter(household_memberships::Column::HouseholdId.eq(fixture.household.id))
            .filter(household_memberships::Column::Status.eq("active"))
            .filter(household_memberships::Column::RevokedAt.is_null())
            .count(&fixture.admin)
            .await
            .unwrap();
        let carers = carer_relationships::Entity::find()
            .filter(carer_relationships::Column::GuardianDeclarationId.eq(declaration.id))
            .filter(carer_relationships::Column::PatientId.eq(subject))
            .filter(carer_relationships::Column::CarerId.eq(fixture.account.person_id))
            .filter(carer_relationships::Column::HouseholdId.eq(fixture.household.id))
            .filter(carer_relationships::Column::Active.eq(true))
            .count(&fixture.admin)
            .await
            .unwrap();
        if account.person_id == declaration.guardian_person_id {
            linked += members * carers;
        }
    }
    assert_eq!(
        linked, 1,
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
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;
    let foreign_id = fixture.foreign.household.id;
    let person_id = fixture.account.person_id;

    let absent_id = i64::MAX;
    let session = fixture.credential("people-session");
    let app = fixture.credential("people-app");
    seed_credentials_and_foreign_household(&fixture).await;
    let mobile = mobile_token(&fixture).await;
    let server = server(&boot);
    let credentials = [session.as_str(), app.as_str(), mobile.as_str()];
    let path = "/households/{household_id}/people";
    let mut replies = Vec::new();
    for credential in credentials {
        for household in [household_id, foreign_id, i64::MAX] {
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
        &format!("/api/v1/households/{household_id}/people?page=0"),
        Some(session.as_str()),
        None,
        None,
    )
    .await;
    let anonymous = send(
        &server,
        Method::GET,
        &format!("/api/v1/households/{foreign_id}/people"),
        None,
        None,
        None,
    )
    .await;
    let before = people_effect(&fixture).await;
    let invalid_credential = send(
        &server,
        Method::GET,
        &format!("/api/v1/households/{absent_id}/people"),
        Some("invalid"),
        None,
        None,
    )
    .await;
    let after = people_effect(&fixture).await;
    drop(server);
    close_request(fixture, boot).await;
    assert_eq!(
        replies
            .iter()
            .map(|(household, reply)| (*household, reply.status))
            .collect::<Vec<_>>(),
        [(household_id, 200), (foreign_id, 403), (i64::MAX, 404)].repeat(3)
    );
    assert_eq!(
        (invalid.status, anonymous.status, invalid_credential.status),
        (422, 401, 401)
    );
    assert_eq!(before, after);
    for (household, reply) in replies {
        assert_schema("get", path, &reply);
        assert_wire(&reply);
        if household == household_id {
            assert_eq!(reply.body["data"][0]["id"], person_id);
        }
    }
    for reply in [invalid, anonymous, invalid_credential] {
        assert_schema("get", path, &reply);
        assert_wire(&reply);
    }
}

#[tokio::test]
async fn create_person_preserves_atomic_links_and_rejects_denied_writes() {
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;
    let foreign_id = fixture.foreign.household.id;

    let absent_id = i64::MAX;
    let session = fixture.credential("people-session");

    seed_credentials_and_foreign_household(&fixture).await;
    let server = server(&boot);
    let path = &format!("/api/v1/households/{household_id}/people");
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
        &format!("/api/v1/households/{foreign_id}/people"),
        Some(session.as_str()),
        Some(&payload),
        None,
    )
    .await;
    let missing = send(
        &server,
        Method::POST,
        &format!("/api/v1/households/{absent_id}/people"),
        Some(session.as_str()),
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
    let malformed = send_malformed(&server, Method::POST, path, &session).await;
    let invalid = send(&server, Method::POST, path, Some(session.as_str()), Some(&json!({"person":{"name":"Wrong age","date_of_birth":"2020-01-01","person_type":"dependent_adult","has_capacity":false,"initial_guardian_declaration":true}})), None).await;
    let dependent_after_denial = dependent_effect(&fixture).await;
    let after_denial = people_effect(&fixture).await;
    let audits_after_denial = request_audits(&fixture).await;
    let created = send(
        &server,
        Method::POST,
        path,
        Some(session.as_str()),
        Some(&payload),
        None,
    )
    .await;
    let children = people::Entity::find()
        .filter(people::Column::Id.is_in(participants(&fixture).await))
        .filter(people::Column::Name.eq("People TestServer child"))
        .all(&fixture.admin)
        .await
        .unwrap();
    let ids = children.iter().map(|person| person.id).collect::<Vec<_>>();
    let grants = person_access_grants::Entity::find()
        .filter(person_access_grants::Column::PersonId.is_in(ids.clone()))
        .filter(person_access_grants::Column::HouseholdId.eq(fixture.household.id))
        .count(&fixture.admin)
        .await
        .unwrap();
    let locations = location_memberships::Entity::find()
        .filter(location_memberships::Column::PersonId.is_in(ids))
        .filter(location_memberships::Column::HouseholdId.eq(fixture.household.id))
        .count(&fixture.admin)
        .await
        .unwrap();
    let effects = (
        i64::try_from(children.len()).unwrap(),
        i64::try_from(grants).unwrap(),
        i64::try_from(locations).unwrap(),
    );
    let dependent_after = dependent_effect(&fixture).await;
    if created.status == 201 {
        assert_created_dependent(&fixture, &created.body).await;
    }
    drop(server);
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_fixture().await;

    let foreign_id = fixture.foreign.household.id;
    let person_id = fixture.account.person_id;

    seed_credentials_and_foreign_household(&fixture).await;
    let mobile = mobile_token(&fixture).await;
    let smart = fixture.credential("synthetic-people-smart");
    fixture.integration(&smart).await;
    fixture.wrong_scope().await;
    let before = people_effect(&fixture).await;
    let audits_before = request_audits(&fixture).await;
    let server = server(&boot);
    let payload = json!({"person":{"name":"Must not write"}});
    let cases = [
        (
            Method::GET,
            &format!("/api/v1/households/{foreign_id}/people"),
            "/households/{household_id}/people",
            "get",
            false,
        ),
        (
            Method::POST,
            &format!("/api/v1/households/{foreign_id}/people"),
            "/households/{household_id}/people",
            "post",
            true,
        ),
        (
            Method::PATCH,
            &format!("/api/v1/households/{foreign_id}/people/{person_id}"),
            "/households/{household_id}/people/{id}",
            "patch",
            true,
        ),
        (
            Method::PUT,
            &format!("/api/v1/households/{foreign_id}/people/{person_id}"),
            "/households/{household_id}/people/{id}",
            "put",
            true,
        ),
    ];
    let mut replies = Vec::new();
    for (method, route, path, verb, has_body) in cases {
        for (credential, bearer, expected) in [
            ("invalid", "invalid", 401),
            ("smart", smart.as_str(), 401),
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
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;

    let person_id = fixture.account.person_id;
    let minor_id = fixture.minor.id;

    let session = fixture.credential("people-session");

    seed_credentials_and_foreign_household(&fixture).await;
    fixture.view_grant().await;
    fixture.member().await;
    fixture.birth(Some("1990-01-01".parse().unwrap())).await;
    let server = server(&boot);
    for method in [Method::PATCH, Method::PUT] {
        let verb = method.as_str().to_lowercase();
        let own = send(
            &server,
            method,
            &format!("/api/v1/households/{household_id}/people/{person_id}"),
            Some(session.as_str()),
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
            &format!("/api/v1/households/{household_id}/people"),
            "/households/{household_id}/people",
            "post",
        ),
        (
            Method::PATCH,
            &format!("/api/v1/households/{household_id}/people/{minor_id}"),
            "/households/{household_id}/people/{id}",
            "patch",
        ),
        (
            Method::PUT,
            &format!("/api/v1/households/{household_id}/people/{minor_id}"),
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
                Some(session.as_str()),
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
    close_request(fixture, boot).await;
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
            let (fixture, boot) = boot_request_fixture().await;
            let household_id = fixture.household.id;

            let person_id = fixture.account.person_id;

            let app = fixture.credential("people-app");
            seed_credentials_and_foreign_household(&fixture).await;
            if method != Method::POST {
                fixture.birth(Some("1990-01-01".parse().unwrap())).await;
            }
            let mobile = if credential == "mobile" {
                Some(mobile_token(&fixture).await)
            } else {
                None
            };
            let bearer = mobile.as_deref().unwrap_or(app.as_str());
            let route = if method == Method::POST {
                &format!("/api/v1/households/{household_id}/people")
            } else {
                &format!("/api/v1/households/{household_id}/people/{person_id}")
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
            let persisted = people::Entity::find()
                .filter(people::Column::Id.is_in(participants(&fixture).await))
                .filter(people::Column::Name.eq(&name))
                .count(&fixture.admin)
                .await
                .unwrap();
            if method == Method::POST && reply.status == 201 {
                assert_created_dependent(&fixture, &reply.body).await;
            }
            drop(server);
            close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;

    let person_id = fixture.account.person_id;

    let session = fixture.credential("people-session");
    let app = fixture.credential("people-app");
    seed_credentials_and_foreign_household(&fixture).await;
    let mobile = mobile_token(&fixture).await;
    let cases = [
        (
            Method::GET,
            &format!("/api/v1/households/{household_id}/people"),
            "/households/{household_id}/people",
            "get",
            false,
        ),
        (
            Method::POST,
            &format!("/api/v1/households/{household_id}/people"),
            "/households/{household_id}/people",
            "post",
            true,
        ),
        (
            Method::PATCH,
            &format!("/api/v1/households/{household_id}/people/{person_id}"),
            "/households/{household_id}/people/{id}",
            "patch",
            true,
        ),
        (
            Method::PUT,
            &format!("/api/v1/households/{household_id}/people/{person_id}"),
            "/households/{household_id}/people/{id}",
            "put",
            true,
        ),
    ];
    let body = json!({"person":{"name":"Must not write"}});
    let server = server(&boot);
    fixture.revoke_all().await;
    let before = people_effect(&fixture).await;
    let audits_before = request_audits(&fixture).await;
    let mut replies = Vec::new();
    for (method, route, path, verb, has_body) in &cases {
        for (kind, bearer) in [
            ("api_session", session.as_str()),
            ("app_token", app.as_str()),
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
    fixture.revoke(CredentialKind::Session, false).await;
    fixture.revoke(CredentialKind::App, false).await;
    fixture.revoke(CredentialKind::Mobile, false).await;
    fixture
        .membership_state(fixture.membership.permissions_version, true)
        .await;
    for (method, route, path, verb, has_body) in &cases {
        for (kind, bearer, expected) in [
            ("api_session", session.as_str(), 401),
            ("app_token", app.as_str(), 401),
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
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;
    let foreign_id = fixture.foreign.household.id;
    let person_id = fixture.account.person_id;

    let absent_id = i64::MAX;
    let session = fixture.credential("people-session");
    let app = fixture.credential("people-app");
    seed_credentials_and_foreign_household(&fixture).await;
    fixture.birth(Some("1990-01-01".parse().unwrap())).await;
    let server = server(&boot);
    let path = &format!("/api/v1/households/{household_id}/people/{person_id}");
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
        Some(session.as_str()),
        Some(&first),
        Some(&key),
    )
    .await;
    let replay = send(
        &server,
        method.clone(),
        path,
        Some(app.as_str()),
        Some(&first),
        Some(&key),
    )
    .await;
    let before_denial = people_effect(&fixture).await;
    let conflict = send(
        &server,
        method.clone(),
        path,
        Some(session.as_str()),
        Some(&changed),
        Some(&key),
    )
    .await;
    let foreign = send(
        &server,
        method.clone(),
        &format!("/api/v1/households/{foreign_id}/people/{person_id}"),
        Some(session.as_str()),
        Some(&changed),
        None,
    )
    .await;
    let absent = send(
        &server,
        method.clone(),
        &format!("/api/v1/households/{absent_id}/people/{person_id}"),
        Some(session.as_str()),
        Some(&changed),
        None,
    )
    .await;
    let invalid = send(
        &server,
        method.clone(),
        path,
        Some(session.as_str()),
        Some(&json!({"person":{"name":""}})),
        None,
    )
    .await;
    let malformed = send_malformed(&server, method.clone(), path, &session).await;
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
    let person = people::Entity::find_by_id(fixture.account.person_id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let stored = (person.name, person.person_type);
    drop(server);
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_rate_fixture(&[]).await;
    let household_id = fixture.household.id;
    let person_id = fixture.account.person_id;
    let session = fixture.credential("people-session");
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
            &format!("/api/v1/households/{household_id}/people"),
            "/households/{household_id}/people",
            "get",
            false,
        ),
        (
            Method::POST,
            &format!("/api/v1/households/{household_id}/people"),
            "/households/{household_id}/people",
            "post",
            true,
        ),
        (
            Method::PATCH,
            &format!("/api/v1/households/{household_id}/people/{person_id}"),
            "/households/{household_id}/people/{id}",
            "patch",
            true,
        ),
        (
            Method::PUT,
            &format!("/api/v1/households/{household_id}/people/{person_id}"),
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
            Some(session.as_str()),
            has_body.then_some(&body),
            None,
        )
        .await;
        replies.push((contract_path, verb, reply));
    }
    let after = people_effect(&fixture).await;
    drop(server);
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;

    let person_id = fixture.account.person_id;

    let session = fixture.credential("people-session");
    let app = fixture.credential("people-app");
    seed_credentials_and_foreign_household(&fixture).await;
    if method != Method::POST {
        fixture.birth(Some("1990-01-01".parse().unwrap())).await;
    }
    let mobile = if method == Method::POST {
        mobile_token(&fixture).await
    } else {
        String::new()
    };
    let first_bearer = if method == Method::POST {
        mobile.as_str()
    } else {
        session.as_str()
    };
    let second_bearer = if method == Method::POST {
        mobile.as_str()
    } else {
        app.as_str()
    };
    let server = server(&boot);
    let path = if method == Method::POST {
        &format!("/api/v1/households/{household_id}/people")
    } else {
        &format!("/api/v1/households/{household_id}/people/{person_id}")
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
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;

    let person_id = fixture.account.person_id;

    let session = fixture.credential("people-session");

    seed_credentials_and_foreign_household(&fixture).await;
    fixture.birth(Some("1990-01-01".parse().unwrap())).await;
    fixture.admin.execute_unprepared("ALTER TABLE security_audit_events ADD CONSTRAINT reject_people_request_audit CHECK (metadata->>'controller' IS DISTINCT FROM 'api/v1/people') NOT VALID").await.unwrap();
    let server = server(&boot);
    let before = people_state(&fixture).await;
    let body =
        json!({"person":{"name":"Audit rejected People child","date_of_birth":"2020-01-01"}});
    let cases = [
        (
            Method::GET,
            &format!("/api/v1/households/{household_id}/people"),
            "/households/{household_id}/people",
            "get",
            false,
        ),
        (
            Method::POST,
            &format!("/api/v1/households/{household_id}/people"),
            "/households/{household_id}/people",
            "post",
            true,
        ),
        (
            Method::PATCH,
            &format!("/api/v1/households/{household_id}/people/{person_id}"),
            "/households/{household_id}/people/{id}",
            "patch",
            true,
        ),
        (
            Method::PUT,
            &format!("/api/v1/households/{household_id}/people/{person_id}"),
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
            Some(session.as_str()),
            has_body.then_some(&body),
            None,
        )
        .await;
        let after = people_state(&fixture).await;
        replies.push((contract_path, verb, response, after));
    }
    fixture
        .admin
        .execute_unprepared(
            "ALTER TABLE security_audit_events DROP CONSTRAINT reject_people_request_audit",
        )
        .await
        .unwrap();
    drop(server);
    close_request(fixture, boot).await;
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
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;

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
        &format!("/api/v1/households/{household_id}/people"),
        Some(&mobile),
        None,
        None,
    )
    .await;
    let after = people_state(&fixture).await;
    fixture
        .admin
        .execute_unprepared("GRANT SELECT ON public.households TO med_tracker_app")
        .await
        .unwrap();
    drop(server);
    close_request(fixture, boot).await;
    assert_eq!(response.status, 500);
    assert_eq!(response.body["error"]["code"], "internal_error");
    assert!(response.body.get("data").is_none());
    assert_eq!(after, before);
    assert_wire(&response);
    assert_schema("get", "/households/{household_id}/people", &response);
}

#[tokio::test]
async fn people_listing_rechecks_account_closed_during_mobile_authentication() {
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;

    let account_id = fixture.account.id;

    let mobile = mobile_token(&fixture).await;
    fixture.admin.execute_unprepared(&format!("CREATE SEQUENCE public.people_account_closure_reached; CREATE FUNCTION public.close_account_after_mobile_touch() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog, public AS $$ BEGIN PERFORM nextval('public.people_account_closure_reached'); UPDATE public.accounts SET status=3 WHERE id={account_id}; RETURN NEW; END $$; CREATE TRIGGER close_account_after_mobile_touch AFTER UPDATE OF last_used_at ON public.oauth_grants FOR EACH ROW WHEN (NEW.account_id={account_id}) EXECUTE FUNCTION public.close_account_after_mobile_touch()")).await.unwrap();
    let before = people_state(&fixture).await;
    let server = server(&boot);
    let response = send(
        &server,
        Method::GET,
        &format!("/api/v1/households/{household_id}/people"),
        Some(&mobile),
        None,
        None,
    )
    .await;
    let query = sea_orm::sea_query::Query::select()
        .column("is_called")
        .from("people_account_closure_reached")
        .to_owned();
    let reached: bool = fixture
        .admin
        .query_one_raw(fixture.admin.get_database_backend().build(&query))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "is_called")
        .unwrap();
    let status = accounts::Entity::find_by_id(fixture.account.id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap()
        .status;
    let after = people_state(&fixture).await;
    fixture.admin.execute_unprepared("DROP TRIGGER close_account_after_mobile_touch ON oauth_grants; DROP FUNCTION public.close_account_after_mobile_touch(); DROP SEQUENCE public.people_account_closure_reached").await.unwrap();
    drop(server);
    close_request(fixture, boot).await;
    assert!(reached);
    assert_eq!(status, 3);
    assert_eq!(response.status, 401);
    assert_eq!(response.body["error"]["code"], "unauthorized");
    assert!(response.body.get("data").is_none());
    assert_eq!(after, before);
    assert_wire(&response);
    assert_schema("get", "/households/{household_id}/people", &response);
}
