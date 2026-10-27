use super::*;
use med_tracker::models::_entities::{household_memberships, security_audit_events};
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

struct Reply {
    route: &'static str,
    credential: &'static str,
    household: i64,
    status: u16,
    body: Value,
    request_id: String,
    cache_control: Option<String>,
    content_type: Option<String>,
    challenge: Option<String>,
}

fn assert_schema(route: &str, status: u16, body: &Value) {
    let path = match route {
        "me" => "/households/{household_id}/me",
        "ui_capabilities" => "/households/{household_id}/ui_capabilities",
        _ => panic!("unknown household route {route}"),
    };
    let document = contract::contract();
    let raw = &document["paths"][path]["get"]["responses"][status.to_string()];
    let response = if raw.get("$ref").is_some() {
        contract::resolve(document, raw)
    } else {
        raw
    };
    contract::assert_value(
        document,
        &response["content"]["application/json"]["schema"],
        body,
        &format!("GET {path} {status}"),
    );
}

async fn mobile_token(fixture: &RequestFixture) -> String {
    fixture.token().await
}

async fn seed_credentials_and_foreign_household(fixture: &RequestFixture) {
    fixture.session(&fixture.credential("me-ui-session")).await;
    fixture.app_token(&fixture.credential("me-ui-app")).await;
}

async fn audit_count(fixture: &RequestFixture) -> usize {
    security_audit_events::Entity::find()
        .filter(security_audit_events::Column::HouseholdId.eq(fixture.household.id))
        .filter(security_audit_events::Column::EventType.eq("api.request"))
        .all(&fixture.admin)
        .await
        .unwrap()
        .into_iter()
        .filter(|event| {
            matches!(
                event.metadata["controller"].as_str(),
                Some("api/v1/me" | "api/v1/ui_capabilities")
            )
        })
        .count()
}

async fn read(
    server: &TestServer,
    route: &'static str,
    credential: &'static str,
    bearer: Option<&str>,
    household: i64,
) -> Reply {
    let path = format!("/api/v1/households/{household}/{route}");
    let response = if let Some(bearer) = bearer {
        server
            .get(&path)
            .add_header("authorization", format!("Bearer {bearer}"))
            .await
    } else {
        server.get(&path).await
    };
    Reply {
        route,
        credential,
        household,
        status: response.status_code().as_u16(),
        request_id: response.header("x-request-id").to_str().unwrap().to_owned(),
        cache_control: response
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        content_type: response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        challenge: response
            .headers()
            .get("www-authenticate")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
        body: response.json(),
    }
}

#[tokio::test]
async fn ui_capabilities_private_success_and_anonymous_denial_are_not_cached() {
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;
    let mobile = mobile_token(&fixture).await;
    let server = server(&boot);
    let success = read(
        &server,
        "ui_capabilities",
        "mobile_oauth",
        Some(&mobile),
        household_id,
    )
    .await;
    let anonymous = read(&server, "ui_capabilities", "anonymous", None, household_id).await;
    drop(server);
    close_request(fixture, boot).await;
    assert_eq!(success.status, 200);
    assert_eq!(anonymous.status, 401);
    assert_eq!(success.cache_control.as_deref(), Some("no-store"));
    assert_eq!(anonymous.cache_control.as_deref(), Some("no-store"));
    assert_schema("ui_capabilities", 200, &success.body);
    assert_schema("ui_capabilities", 401, &anonymous.body);
}

#[tokio::test]
async fn active_credentials_distinguish_own_foreign_and_missing_households() {
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;
    let foreign_id = fixture.foreign.household.id;
    let session = fixture.credential("me-ui-session");
    let app = fixture.credential("me-ui-app");
    seed_credentials_and_foreign_household(&fixture).await;
    let mobile = mobile_token(&fixture).await;
    let server = server(&boot);
    let credentials = [
        ("api_session", session.as_str()),
        ("app_token", app.as_str()),
        ("mobile_oauth", mobile.as_str()),
    ];
    let mut replies = Vec::new();
    for route in ["me", "ui_capabilities"] {
        for (name, bearer) in credentials {
            replies.push(read(&server, route, name, Some(bearer), household_id).await);
        }
    }
    let effects_before = fixture.effect().await;
    let audits_before = audit_count(&fixture).await;
    for route in ["me", "ui_capabilities"] {
        for (name, bearer) in credentials {
            for household in [foreign_id, i64::MAX] {
                replies.push(read(&server, route, name, Some(bearer), household).await);
            }
        }
    }
    let effects_after = fixture.effect().await;
    let audits_after = audit_count(&fixture).await;
    drop(server);
    close_request(fixture, boot).await;
    let statuses = replies
        .iter()
        .map(|reply| (reply.route, reply.credential, reply.household, reply.status))
        .collect::<Vec<_>>();
    let expected = ["me", "ui_capabilities"]
        .into_iter()
        .flat_map(|route| {
            credentials
                .into_iter()
                .map(move |(name, _)| (route, name, household_id, 200))
        })
        .chain(["me", "ui_capabilities"].into_iter().flat_map(|route| {
            credentials.into_iter().flat_map(move |(name, _)| {
                [(route, name, foreign_id, 403), (route, name, i64::MAX, 404)]
            })
        }))
        .collect::<Vec<_>>();
    assert_eq!(statuses, expected);
    assert_eq!(effects_after, effects_before);
    assert_eq!(audits_after, audits_before);
    for reply in replies {
        assert_eq!(reply.content_type.as_deref(), Some("application/json"));
        assert!(!reply.request_id.is_empty());
        assert_eq!(reply.cache_control.as_deref(), Some("no-store"));
        assert_schema(reply.route, reply.status, &reply.body);
        if reply.status == 200 {
            assert!(reply.body["data"].is_object());
        } else {
            assert_eq!(
                reply.body["error"]["code"],
                if reply.status == 403 {
                    "forbidden"
                } else {
                    "not_found"
                }
            );
            assert_eq!(reply.body["error"]["request_id"], reply.request_id);
        }
    }
}

#[tokio::test]
async fn invalid_and_revoked_credentials_do_not_reveal_household_existence() {
    let (fixture, boot) = boot_request_fixture().await;
    let foreign_id = fixture.foreign.household.id;
    let session = fixture.credential("me-ui-session");
    let app = fixture.credential("me-ui-app");
    seed_credentials_and_foreign_household(&fixture).await;
    let mobile = mobile_token(&fixture).await;
    let server = server(&boot);
    let effects_before = fixture.effect().await;
    let audits_before = audit_count(&fixture).await;
    let mut replies = Vec::new();
    for route in ["me", "ui_capabilities"] {
        for household in [foreign_id, i64::MAX] {
            replies.push(read(&server, route, "anonymous", None, household).await);
            replies.push(read(&server, route, "invalid", Some("invalid"), household).await);
        }
    }
    for (name, bearer, kind) in [
        ("api_session", session.as_str(), CredentialKind::Session),
        ("app_token", app.as_str(), CredentialKind::App),
        ("mobile_oauth", mobile.as_str(), CredentialKind::Mobile),
    ] {
        fixture.revoke(kind, true).await;
        for route in ["me", "ui_capabilities"] {
            for household in [foreign_id, i64::MAX] {
                replies.push(read(&server, route, name, Some(bearer), household).await);
            }
        }
        let kind = match name {
            "api_session" => CredentialKind::Session,
            "app_token" => CredentialKind::App,
            _ => CredentialKind::Mobile,
        };
        fixture.revoke(kind, false).await;
    }
    let effects_after = fixture.effect().await;
    let audits_after = audit_count(&fixture).await;
    drop(server);
    close_request(fixture, boot).await;
    assert!(
        replies.iter().all(|reply| reply.status == 401),
        "{:?}",
        replies
            .iter()
            .map(|reply| (reply.route, reply.credential, reply.household, reply.status))
            .collect::<Vec<_>>()
    );
    assert_eq!(audits_after, audits_before);
    assert_eq!(effects_after, effects_before);
    for reply in replies {
        assert_eq!(reply.content_type.as_deref(), Some("application/json"));
        assert!(!reply.request_id.is_empty());
        assert_eq!(reply.challenge.as_deref(), Some("Bearer"));
        assert_eq!(reply.cache_control.as_deref(), Some("no-store"));
        assert_eq!(reply.body["error"]["code"], "unauthorized");
        assert_eq!(reply.body["error"]["request_id"], reply.request_id);
        assert_schema(reply.route, 401, &reply.body);
    }
}

#[tokio::test]
async fn smart_integration_grant_cannot_read_household_me_or_ui() {
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;
    let smart = fixture.credential("synthetic-me-ui-smart-access");
    fixture.integration(&smart).await;
    let effects_before = fixture.effect().await;
    let audits_before = audit_count(&fixture).await;
    let server = server(&boot);
    let replies = [
        read(
            &server,
            "me",
            "smart_integration",
            Some(&smart),
            household_id,
        )
        .await,
        read(
            &server,
            "ui_capabilities",
            "smart_integration",
            Some(&smart),
            household_id,
        )
        .await,
    ];
    let effects_after = fixture.effect().await;
    let audits_after = audit_count(&fixture).await;
    drop(server);
    close_request(fixture, boot).await;
    assert_eq!(
        replies.iter().map(|reply| reply.status).collect::<Vec<_>>(),
        [401, 401]
    );
    assert_eq!(effects_after, effects_before);
    assert_eq!(audits_after, audits_before);
    for reply in replies {
        assert_eq!(reply.content_type.as_deref(), Some("application/json"));
        assert_eq!(reply.cache_control.as_deref(), Some("no-store"));
        assert!(!reply.request_id.is_empty());
        assert!(reply.body.get("data").is_none());
        assert_eq!(reply.challenge.as_deref(), Some("Bearer"));
        assert_eq!(reply.body["error"]["code"], "unauthorized");
        assert_eq!(reply.body["error"]["request_id"], reply.request_id);
        assert_schema(reply.route, 401, &reply.body);
    }
}

#[tokio::test]
async fn mobile_grant_without_medtracker_scope_is_forbidden_on_me_and_ui() {
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;
    let mobile = mobile_token(&fixture).await;
    fixture.wrong_scope().await;
    let effects_before = fixture.effect().await;
    let audits_before = audit_count(&fixture).await;
    let server = server(&boot);
    let replies = [
        read(
            &server,
            "me",
            "mobile_wrong_scope",
            Some(&mobile),
            household_id,
        )
        .await,
        read(
            &server,
            "ui_capabilities",
            "mobile_wrong_scope",
            Some(&mobile),
            household_id,
        )
        .await,
    ];
    let effects_after = fixture.effect().await;
    let audits_after = audit_count(&fixture).await;
    drop(server);
    close_request(fixture, boot).await;
    assert_eq!(
        replies.iter().map(|reply| reply.status).collect::<Vec<_>>(),
        [403, 403]
    );
    assert_eq!(effects_after, effects_before);
    assert_eq!(audits_after, audits_before);
    for reply in replies {
        assert_eq!(reply.content_type.as_deref(), Some("application/json"));
        assert_eq!(reply.cache_control.as_deref(), Some("no-store"));
        assert!(!reply.request_id.is_empty());
        assert_eq!(
            reply.challenge.as_deref(),
            Some("Bearer error=\"insufficient_scope\" scope=\"medtracker\"")
        );
        assert!(reply.body.get("data").is_none());
        assert_eq!(reply.body["error"]["code"], "forbidden");
        assert_eq!(reply.body["error"]["request_id"], reply.request_id);
        assert_schema(reply.route, 403, &reply.body);
    }
}

#[tokio::test]
async fn me_and_ui_reads_share_the_fixed_general_limit_without_rejection_effects() {
    let (fixture, boot) = boot_request_rate_fixture(&[]).await;
    let household_id = fixture.household.id;
    let mobile = mobile_token(&fixture).await;
    let server = server(&boot);
    let effects_initial = fixture.effect().await;
    let audits_initial = audit_count(&fixture).await;
    for attempt in 1..=300 {
        let route = if attempt % 2 == 0 {
            "me"
        } else {
            "ui_capabilities"
        };
        let response = server
            .get(&format!("/api/v1/households/{household_id}/{route}"))
            .add_header("authorization", format!("Bearer {mobile}"))
            .await;
        assert_eq!(response.status_code(), 200, "{route} request {attempt}");
    }
    let effects_before_rejection = fixture.effect().await;
    let audits_before_rejection = audit_count(&fixture).await;
    let mut replies = Vec::new();
    for route in ["me", "ui_capabilities"] {
        let response = server
            .get(&format!("/api/v1/households/{household_id}/{route}"))
            .add_header("authorization", format!("Bearer {mobile}"))
            .await;
        let body: Value = response.json();
        replies.push((
            route,
            response.status_code().as_u16(),
            response.header("content-type").to_owned(),
            response.header("cache-control").to_owned(),
            response.header("x-request-id").to_owned(),
            response.header("retry-after").to_owned(),
            response.header("ratelimit-limit").to_owned(),
            response.header("ratelimit-remaining").to_owned(),
            response.header("ratelimit-reset").to_owned(),
            body,
        ));
    }
    let effects_after = fixture.effect().await;
    let audits_after = audit_count(&fixture).await;
    drop(server);
    close_request(fixture, boot).await;
    assert_eq!(effects_before_rejection, effects_initial);
    assert_eq!(audits_before_rejection, audits_initial + 300);
    assert_eq!(effects_after, effects_before_rejection);
    assert_eq!(audits_after, audits_before_rejection);
    for (
        route,
        status,
        content_type,
        cache_control,
        request_id,
        retry_after,
        limit,
        remaining,
        reset,
        body,
    ) in replies
    {
        assert_eq!(status, 429, "{route}");
        assert_eq!(content_type, "application/json");
        assert_eq!(cache_control, "no-store");
        assert!(!request_id.is_empty());
        assert_eq!(
            (
                retry_after.to_str().unwrap(),
                limit.to_str().unwrap(),
                remaining.to_str().unwrap(),
                reset.to_str().unwrap()
            ),
            ("200", "300", "0", "1200")
        );
        assert_eq!(body["error"]["code"], "rate_limited");
        assert_schema(route, 429, &body);
    }
}

#[tokio::test]
async fn ui_capabilities_rechecks_membership_and_hides_data_on_audit_failure() {
    let (fixture, boot) = boot_request_fixture().await;
    let household_id = fixture.household.id;
    let mobile = mobile_token(&fixture).await;
    let server = server(&boot);
    let initial = read(
        &server,
        "ui_capabilities",
        "mobile_oauth",
        Some(&mobile),
        household_id,
    )
    .await;
    household_memberships::Entity::update_many()
        .col_expr(
            household_memberships::Column::RevokedAt,
            Expr::value(Some(chrono::Utc::now().naive_utc())),
        )
        .filter(household_memberships::Column::Id.eq(fixture.membership.id))
        .exec(&fixture.admin)
        .await
        .unwrap();
    let effects_before_denial = fixture.effect().await;
    let audits_before_denial = audit_count(&fixture).await;
    let denied = read(
        &server,
        "ui_capabilities",
        "mobile_oauth",
        Some(&mobile),
        household_id,
    )
    .await;
    let effects_after_denial = fixture.effect().await;
    let audits_after_denial = audit_count(&fixture).await;
    household_memberships::Entity::update_many()
        .col_expr(
            household_memberships::Column::RevokedAt,
            Expr::value(None::<chrono::NaiveDateTime>),
        )
        .filter(household_memberships::Column::Id.eq(fixture.membership.id))
        .exec(&fixture.admin)
        .await
        .unwrap();
    fixture.admin.execute_unprepared(
        "ALTER TABLE security_audit_events ADD CONSTRAINT reject_ui_audit CHECK (metadata->>'controller' IS DISTINCT FROM 'api/v1/ui_capabilities') NOT VALID"
    ).await.unwrap();
    let effects_before_fault = fixture.effect().await;
    let audits_before_fault = audit_count(&fixture).await;
    let failed = read(
        &server,
        "ui_capabilities",
        "mobile_oauth",
        Some(&mobile),
        household_id,
    )
    .await;
    let effects_after_fault = fixture.effect().await;
    let audits_after_fault = audit_count(&fixture).await;
    fixture
        .admin
        .execute_unprepared("ALTER TABLE security_audit_events DROP CONSTRAINT reject_ui_audit")
        .await
        .unwrap();
    drop(server);
    close_request(fixture, boot).await;
    assert_eq!(initial.status, 200);
    assert_eq!(initial.cache_control.as_deref(), Some("no-store"));
    assert_schema("ui_capabilities", 200, &initial.body);
    assert_eq!(denied.status, 403);
    assert_eq!(denied.cache_control.as_deref(), Some("no-store"));
    assert_eq!(denied.body["error"]["code"], "forbidden");
    assert!(denied.body.get("data").is_none());
    assert_schema("ui_capabilities", 403, &denied.body);
    assert_eq!(effects_after_denial, effects_before_denial);
    assert_eq!(audits_after_denial, audits_before_denial);
    assert_eq!(failed.status, 500);
    assert_eq!(failed.cache_control.as_deref(), Some("no-store"));
    assert_eq!(failed.content_type.as_deref(), Some("application/json"));
    assert!(!failed.request_id.is_empty());
    assert_eq!(failed.body["error"]["code"], "internal_error");
    assert_eq!(failed.body["error"]["request_id"], failed.request_id);
    assert!(failed.body.get("data").is_none());
    assert_eq!(effects_after_fault, effects_before_fault);
    assert_eq!(audits_after_fault, audits_before_fault);
}
