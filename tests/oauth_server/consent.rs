use crate::scenario::Scenario;
use loco_rs::{
    TestServer,
    app::Hooks,
    boot::{BootResult, StartMode},
    config::{Config, QueueConfig},
    environment::Environment,
};
use med_tracker::models::_entities::{
    household_memberships, households, oauth_applications, oauth_grants, security_audit_events,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait,
    QueryFilter,
};
use serde_json::json;

struct Fixture {
    scenario: Scenario,
    second_household: households::Model,
    second_membership: household_memberships::Model,
    application: oauth_applications::Model,
    state: String,
}

impl std::ops::Deref for Fixture {
    type Target = Scenario;
    fn deref(&self) -> &Self::Target {
        &self.scenario
    }
}

async fn client(
    scenario: &Scenario,
    kind: &str,
    redirect: &str,
    scopes: &str,
) -> oauth_applications::Model {
    let now = chrono::Utc::now().naive_utc();
    oauth_applications::ActiveModel {
        account_id: Set(Some(scenario.account.id)),
        client_id: Set(format!("consent-{}", uuid::Uuid::new_v4())),
        client_kind: Set(kind.into()),
        name: Set("Synthetic consent client".into()),
        redirect_uri: Set(redirect.into()),
        scopes: Set(scopes.into()),
        token_endpoint_auth_method: Set("none".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&scenario.admin)
    .await
    .unwrap()
}

async fn boot() -> (Fixture, BootResult) {
    let scenario = Scenario::new().await;
    let mut membership = scenario.membership.clone().into_active_model();
    membership.role = Set("administrator".into());
    membership.update(&scenario.admin).await.unwrap();
    let application = client(
        &scenario,
        "integration",
        "https://example.test/callback",
        "patient/*.read",
    )
    .await;
    let now = chrono::Utc::now().naive_utc();
    let second_household = households::ActiveModel {
        created_by_account_id: Set(Some(scenario.account.id)),
        name: Set("Second care home".into()),
        slug: Set(format!("second-care-home-{}", uuid::Uuid::new_v4())),
        timezone: Set("Europe/London".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&scenario.admin)
    .await
    .unwrap();
    let second_membership = household_memberships::ActiveModel {
        household_id: Set(second_household.id),
        person_id: Set(scenario.account.person_id),
        role: Set("administrator".into()),
        status: Set("active".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&scenario.admin)
    .await
    .unwrap();
    let fixture = Fixture {
        scenario,
        second_household,
        second_membership,
        application,
        state: uuid::Uuid::new_v4().to_string(),
    };
    let mut config = Config::new(&Environment::Test).unwrap();
    config.settings = Some(
        json!({"browser_session": {"key":base64::Engine::encode(&base64::engine::general_purpose::STANDARD, [7_u8; 64]),"secure":false}}),
    );
    config.database.uri = std::env::var("DATABASE_URL").unwrap();
    assert!(!config.database.auto_migrate);
    assert!(!config.database.dangerously_recreate);
    assert!(!config.database.dangerously_truncate);
    let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
        panic!("PostgreSQL queue required")
    };
    queue.uri = config.database.uri.clone();
    let boot = med_tracker::app::App::boot(StartMode::ServerOnly, &Environment::Test, config)
        .await
        .unwrap();
    (fixture, boot)
}

fn server(boot: &BootResult) -> TestServer {
    TestServer::new(
        boot.router
            .clone()
            .unwrap()
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .unwrap()
}

fn csrf(html: &str) -> &str {
    html.split("name=\"authenticity_token\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
}

fn nonce(html: &str) -> &str {
    html.split("name=\"consent_nonce\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
}

fn query(fixture: &Fixture) -> String {
    serde_urlencoded::to_string([
        ("client_id", fixture.application.client_id.as_str()),
        ("response_type", "code"),
        ("redirect_uri", "https://example.test/callback"),
        ("scope", "patient/*.read"),
        ("state", fixture.state.as_str()),
        (
            "code_challenge",
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
        ),
        ("code_challenge_method", "S256"),
    ])
    .unwrap()
}

fn approval_fields<'a>(fixture: &'a Fixture, html: &'a str) -> Vec<(&'static str, &'a str)> {
    vec![
        ("authenticity_token", csrf(html)),
        ("consent_nonce", nonce(html)),
        ("client_id", fixture.application.client_id.as_str()),
        ("response_type", "code"),
        ("redirect_uri", "https://example.test/callback"),
        ("state", fixture.state.as_str()),
        (
            "code_challenge",
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
        ),
        ("code_challenge_method", "S256"),
        ("scope[]", "patient/*.read"),
    ]
}

async fn close(fixture: Fixture, boot: BootResult) {
    if let Some(queue) = boot.app_context.queue_provider.as_ref() {
        queue.shutdown().unwrap();
    }
    boot.app_context.db.close().await.unwrap();
    fixture.scenario.close().await;
}

async fn sign_in(request: &TestServer, boot: &BootResult, fixture: &Fixture) {
    sign_in_at(request, boot, fixture, "shared sign-in").await;
}

async fn sign_in_at(request: &TestServer, boot: &BootResult, fixture: &Fixture, stage: &str) {
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":fixture.account.email,"password":"password"}))
        .await;
    let request_id = signed_in
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("absent");
    let content_type = signed_in
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("absent");
    let error_code = serde_json::from_str::<serde_json::Value>(&signed_in.text())
        .ok()
        .and_then(|body| {
            body.get("code")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "absent".into());
    assert_eq!(
        signed_in.status_code(),
        200,
        "stage={stage} request_id={request_id} content_type={content_type} error_code={error_code}"
    );
}

async fn select(request: &TestServer, household_id: &str) -> String {
    let chooser = request.get("/households/choose?switch=1").await;
    assert_eq!(chooser.status_code(), 200, "{}", chooser.text());
    let selected = request
        .post("/households/select")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("household_id", household_id),
            ("authenticity_token", csrf(&chooser.text())),
        ])
        .await;
    assert_eq!(selected.status_code(), 303, "{}", selected.text());
    selected.header("location").to_str().unwrap().to_owned()
}

async fn consent(request: &TestServer, fixture: &Fixture) -> String {
    let response = request.get(&format!("/authorize?{}", query(fixture))).await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    response.text()
}

#[tokio::test]
async fn selected_second_household_is_shown_on_integration_consent() {
    crate::authentication_fixture::reset_source_quota().await;
    let (fixture, boot) = boot().await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot, &fixture).await;
    let without_selection = request
        .get(&format!("/authorize?{}", query(&fixture)))
        .await;
    assert_eq!(
        without_selection.status_code(),
        303,
        "{}",
        without_selection.text()
    );
    assert_eq!(without_selection.header("location"), "/households/choose");
    let resumed = select(&request, &fixture.second_household.id.to_string()).await;
    assert!(resumed.starts_with("/authorize?"), "{resumed}");
    let consent = consent(&request, &fixture).await;
    assert!(consent.contains("Household: Second care home"), "{consent}");
    let approved = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&approval_fields(&fixture, &consent))
        .await;
    assert_eq!(approved.status_code(), 200, "{}", approved.text());
    let grant = oauth_grants::Entity::find()
        .filter(oauth_grants::Column::AccountId.eq(fixture.account.id))
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        grant.household_membership_id,
        Some(fixture.second_membership.id)
    );
    assert_eq!(grant.person_id, Some(fixture.account.person_id));
    assert_eq!(
        grant.permissions_version,
        Some(fixture.second_membership.permissions_version)
    );
    assert_eq!(grant.scopes, "patient/*.read");
    let audits = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::ActorAccountId.eq(fixture.account.id))
        .filter(
            security_audit_events::Column::HouseholdId
                .is_in([fixture.household.id, fixture.second_household.id]),
        )
        .filter(security_audit_events::Column::EventType.eq("smart_oauth.consent_granted"))
        .all(&fixture.admin)
        .await
        .unwrap();
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].household_id, fixture.second_household.id);
    let replay = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&approval_fields(&fixture, &consent))
        .await;
    assert_eq!(replay.status_code(), 400, "{}", replay.text());
    assert_eq!(
        oauth_grants::Entity::find()
            .filter(oauth_grants::Column::AccountId.eq(fixture.account.id))
            .count(&fixture.admin)
            .await
            .unwrap(),
        1
    );
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn consent_keeps_its_displayed_household_after_session_switch() {
    crate::authentication_fixture::reset_source_quota().await;
    let (fixture, boot) = boot().await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot, &fixture).await;
    select(&request, &fixture.second_household.id.to_string()).await;
    let shown = consent(&request, &fixture).await;
    assert!(shown.contains("Household: Second care home"));
    assert_eq!(
        select(&request, &fixture.household.id.to_string()).await,
        format!("/households/{}/dashboard", fixture.household.slug)
    );
    let approved = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&approval_fields(&fixture, &shown))
        .await;
    assert_eq!(approved.status_code(), 200, "{}", approved.text());
    let grant = oauth_grants::Entity::find()
        .filter(oauth_grants::Column::AccountId.eq(fixture.account.id))
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        grant.household_membership_id,
        Some(fixture.second_membership.id)
    );
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn newer_consent_page_invalidates_old_form_even_with_identical_oauth_fields() {
    crate::authentication_fixture::reset_source_quota().await;
    let (fixture, boot) = boot().await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot, &fixture).await;
    select(&request, &fixture.second_household.id.to_string()).await;
    let old = consent(&request, &fixture).await;
    select(&request, &fixture.household.id.to_string()).await;
    let new = consent(&request, &fixture).await;
    assert!(new.contains("Household: Synthetic household"));
    assert_ne!(nonce(&old), nonce(&new));
    let rejected = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&approval_fields(&fixture, &old))
        .await;
    assert_eq!(rejected.status_code(), 400, "{}", rejected.text());
    assert_eq!(
        oauth_grants::Entity::find()
            .filter(oauth_grants::Column::AccountId.eq(fixture.account.id))
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        security_audit_events::Entity::find()
            .filter(security_audit_events::Column::ActorAccountId.eq(fixture.account.id))
            .filter(
                security_audit_events::Column::HouseholdId
                    .is_in([fixture.household.id, fixture.second_household.id])
            )
            .filter(security_audit_events::Column::EventType.eq("smart_oauth.consent_granted"))
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    let approved = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&approval_fields(&fixture, &new))
        .await;
    assert_eq!(approved.status_code(), 200, "{}", approved.text());
    let grant = oauth_grants::Entity::find()
        .filter(oauth_grants::Column::AccountId.eq(fixture.account.id))
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(grant.household_membership_id, Some(fixture.membership.id));
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn revoked_displayed_membership_cannot_create_grant_or_audit() {
    crate::authentication_fixture::reset_source_quota().await;
    let (fixture, boot) = boot().await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot, &fixture).await;
    select(&request, &fixture.second_household.id.to_string()).await;
    let shown = consent(&request, &fixture).await;
    let membership = household_memberships::Entity::find_by_id(fixture.second_membership.id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut membership = membership.into_active_model();
    membership.status = Set("revoked".into());
    membership.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    membership.update(&fixture.admin).await.unwrap();
    let rejected = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&approval_fields(&fixture, &shown))
        .await;
    assert!(
        rejected.status_code().is_client_error(),
        "{}",
        rejected.text()
    );
    assert_eq!(
        oauth_grants::Entity::find()
            .filter(oauth_grants::Column::AccountId.eq(fixture.account.id))
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        security_audit_events::Entity::find()
            .filter(security_audit_events::Column::ActorAccountId.eq(fixture.account.id))
            .filter(
                security_audit_events::Column::HouseholdId
                    .is_in([fixture.household.id, fixture.second_household.id])
            )
            .filter(security_audit_events::Column::EventType.eq("smart_oauth.consent_granted"))
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn fresh_authorization_after_revocation_can_choose_another_household() {
    crate::authentication_fixture::reset_source_quota().await;
    let (fixture, boot) = boot().await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot, &fixture).await;
    select(&request, &fixture.second_household.id.to_string()).await;
    let old = consent(&request, &fixture).await;
    let membership = household_memberships::Entity::find_by_id(fixture.second_membership.id)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut membership = membership.into_active_model();
    membership.status = Set("revoked".into());
    membership.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    membership.update(&fixture.admin).await.unwrap();
    let fresh = request
        .get(&format!("/authorize?{}", query(&fixture)))
        .await;
    assert_eq!(fresh.status_code(), 303, "{}", fresh.text());
    assert_eq!(fresh.header("location"), "/households/choose");
    let resumed = select(&request, &fixture.household.id.to_string()).await;
    assert!(resumed.starts_with("/authorize?"), "{resumed}");
    let new = consent(&request, &fixture).await;
    assert!(new.contains("Household: Synthetic household"));
    let old_post = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&approval_fields(&fixture, &old))
        .await;
    assert_eq!(old_post.status_code(), 400, "{}", old_post.text());
    let approved = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&approval_fields(&fixture, &new))
        .await;
    assert_eq!(approved.status_code(), 200, "{}", approved.text());
    let grant = oauth_grants::Entity::find()
        .filter(oauth_grants::Column::AccountId.eq(fixture.account.id))
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(grant.household_membership_id, Some(fixture.membership.id));
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn permission_change_invalidates_displayed_consent() {
    crate::authentication_fixture::reset_source_quota().await;
    assert_changed_consent_rejected("permissions").await;
}

#[tokio::test]
async fn held_household_invalidates_displayed_consent() {
    crate::authentication_fixture::reset_source_quota().await;
    assert_changed_consent_rejected("held").await;
}

#[tokio::test]
async fn archived_household_invalidates_displayed_consent() {
    crate::authentication_fixture::reset_source_quota().await;
    assert_changed_consent_rejected("archived").await;
}

async fn assert_changed_consent_rejected(changed: &str) {
    let (fixture, boot) = boot().await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in_at(&request, &boot, &fixture, changed).await;
    select(&request, &fixture.second_household.id.to_string()).await;
    let shown = consent(&request, &fixture).await;
    if changed == "permissions" {
        let membership = household_memberships::Entity::find_by_id(fixture.second_membership.id)
            .one(&fixture.admin)
            .await
            .unwrap()
            .unwrap();
        let mut membership = membership.into_active_model();
        membership.permissions_version = Set(fixture.second_membership.permissions_version + 1);
        membership.update(&fixture.admin).await.unwrap();
    } else {
        let household = households::Entity::find_by_id(fixture.second_household.id)
            .one(&fixture.admin)
            .await
            .unwrap()
            .unwrap();
        let mut household = household.into_active_model();
        if changed == "held" {
            household.lifecycle_state = Set("held".into());
        } else {
            household.status = Set("archived".into());
        }
        household.update(&fixture.admin).await.unwrap();
    }
    let rejected = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&approval_fields(&fixture, &shown))
        .await;
    assert!(
        rejected.status_code().is_client_error(),
        "{changed}: {}",
        rejected.text()
    );
    assert_eq!(
        oauth_grants::Entity::find()
            .filter(oauth_grants::Column::AccountId.eq(fixture.account.id))
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        security_audit_events::Entity::find()
            .filter(security_audit_events::Column::ActorAccountId.eq(fixture.account.id))
            .filter(
                security_audit_events::Column::HouseholdId
                    .is_in([fixture.household.id, fixture.second_household.id])
            )
            .filter(security_audit_events::Column::EventType.eq("smart_oauth.consent_granted"))
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn two_browser_sessions_keep_distinct_household_consent() {
    crate::authentication_fixture::reset_source_quota().await;
    let (fixture, boot) = boot().await;
    let mut first = server(&boot);
    let mut second = server(&boot);
    first.save_cookies();
    second.save_cookies();
    sign_in(&first, &boot, &fixture).await;
    sign_in(&second, &boot, &fixture).await;
    select(&first, &fixture.household.id.to_string()).await;
    select(&second, &fixture.second_household.id.to_string()).await;
    let first_page = consent(&first, &fixture).await;
    let second_page = consent(&second, &fixture).await;
    assert!(first_page.contains("Household: Synthetic household"));
    assert!(second_page.contains("Household: Second care home"));
    let mut substituted = approval_fields(&fixture, &first_page);
    substituted[0].1 = csrf(&second_page);
    let rejected = second
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&substituted)
        .await;
    assert_eq!(rejected.status_code(), 400, "{}", rejected.text());
    assert_eq!(
        oauth_grants::Entity::find()
            .filter(oauth_grants::Column::AccountId.eq(fixture.account.id))
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    for (request, page) in [(&first, &first_page), (&second, &second_page)] {
        let response = request
            .post("/authorize")
            .add_header("sec-fetch-site", "same-origin")
            .form(&approval_fields(&fixture, page))
            .await;
        assert_eq!(response.status_code(), 200, "{}", response.text());
    }
    let grants = oauth_grants::Entity::find()
        .filter(oauth_grants::Column::AccountId.eq(fixture.account.id))
        .all(&fixture.admin)
        .await
        .unwrap();
    assert_eq!(grants.len(), 2);
    assert!(
        grants
            .iter()
            .any(|grant| grant.household_membership_id == Some(fixture.membership.id))
    );
    assert!(
        grants
            .iter()
            .any(|grant| grant.household_membership_id == Some(fixture.second_membership.id))
    );
    drop(first);
    drop(second);
    close(fixture, boot).await;
}

#[tokio::test]
async fn mobile_consent_stays_account_bound_without_selected_household() {
    crate::authentication_fixture::reset_source_quota().await;
    let (fixture, boot) = boot().await;
    let mobile = client(
        &fixture,
        "mobile",
        "io.damacus.medtracker:/oauth2redirect",
        "medtracker offline_access",
    )
    .await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot, &fixture).await;
    let query = serde_urlencoded::to_string([
        ("client_id", mobile.client_id.as_str()),
        ("response_type", "code"),
        ("redirect_uri", "io.damacus.medtracker:/oauth2redirect"),
        ("scope", "medtracker offline_access"),
        ("state", "mobile-no-household"),
        (
            "code_challenge",
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
        ),
        ("code_challenge_method", "S256"),
    ])
    .unwrap();
    let page = request.get(&format!("/authorize?{query}")).await;
    assert_eq!(page.status_code(), 200, "{}", page.text());
    assert!(!page.text().contains("Household:"));
    assert!(!page.text().contains("Person:"));
    let approved = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("authenticity_token", csrf(&page.text())),
            ("consent_nonce", nonce(&page.text())),
            ("client_id", mobile.client_id.as_str()),
            ("response_type", "code"),
            ("redirect_uri", "io.damacus.medtracker:/oauth2redirect"),
            ("state", "mobile-no-household"),
            (
                "code_challenge",
                "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
            ),
            ("code_challenge_method", "S256"),
            ("scope[]", "medtracker"),
        ])
        .await;
    assert_eq!(approved.status_code(), 200, "{}", approved.text());
    let grant = oauth_grants::Entity::find()
        .filter(oauth_grants::Column::AccountId.eq(fixture.account.id))
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(grant.account_id, fixture.account.id);
    assert_eq!(grant.client_kind, "mobile");
    assert_eq!(grant.household_membership_id, None);
    assert_eq!(grant.person_id, None);
    assert_eq!(grant.scopes, "medtracker");
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn consent_rejects_csrf_protocol_scope_and_nonce_tampering() {
    crate::authentication_fixture::reset_source_quota().await;
    let (fixture, boot) = boot().await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot, &fixture).await;
    select(&request, &fixture.second_household.id.to_string()).await;
    let shown = consent(&request, &fixture).await;
    let mut invalid_csrf = approval_fields(&fixture, &shown);
    invalid_csrf[0].1 = "invalid";
    let response = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&invalid_csrf)
        .await;
    assert_eq!(response.status_code(), 403, "{}", response.text());
    let mut changed_state = approval_fields(&fixture, &shown);
    changed_state[5].1 = "changed-state";
    let response = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&changed_state)
        .await;
    assert_eq!(response.status_code(), 400, "{}", response.text());
    let mut invalid_scope = approval_fields(&fixture, &shown);
    invalid_scope.last_mut().unwrap().1 = "admin";
    let response = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&invalid_scope)
        .await;
    assert_eq!(response.status_code(), 400, "{}", response.text());
    let mut duplicate_nonce = approval_fields(&fixture, &shown);
    duplicate_nonce.push(("consent_nonce", nonce(&shown)));
    let response = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&duplicate_nonce)
        .await;
    assert_eq!(response.status_code(), 400, "{}", response.text());
    assert_eq!(
        oauth_grants::Entity::find()
            .filter(oauth_grants::Column::AccountId.eq(fixture.account.id))
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    let mut denial = approval_fields(&fixture, &shown);
    denial.pop();
    denial.push(("consent_action", "deny"));
    let response = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&denial)
        .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    assert_eq!(
        oauth_grants::Entity::find()
            .filter(oauth_grants::Column::AccountId.eq(fixture.account.id))
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        security_audit_events::Entity::find()
            .filter(security_audit_events::Column::ActorAccountId.eq(fixture.account.id))
            .filter(
                security_audit_events::Column::HouseholdId
                    .is_in([fixture.household.id, fixture.second_household.id])
            )
            .filter(security_audit_events::Column::EventType.eq("smart_oauth.consent_granted"))
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn first_household_creation_resumes_pending_integration_consent() {
    crate::authentication_fixture::reset_source_quota().await;
    let (fixture, boot) = boot().await;
    let newcomer = Scenario::new().await;
    household_memberships::Entity::delete_by_id(newcomer.membership.id)
        .exec(&fixture.admin)
        .await
        .unwrap();
    assert_eq!(
        household_memberships::Entity::find()
            .filter(household_memberships::Column::PersonId.eq(newcomer.account.person_id))
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    let household_name = format!("New OAuth household {}", uuid::Uuid::new_v4());
    let mut request = server(&boot);
    request.save_cookies();
    let unauthenticated = request
        .get(&format!("/authorize?{}", query(&fixture)))
        .await;
    assert_eq!(unauthenticated.status_code(), 303);
    assert_eq!(unauthenticated.header("location"), "/login");
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":newcomer.account.email,"password":"password"}))
        .await;
    assert_eq!(signed_in.status_code(), 200, "{}", signed_in.text());
    let chooser_redirect = request
        .get(&format!("/authorize?{}", query(&fixture)))
        .await;
    assert_eq!(
        chooser_redirect.status_code(),
        303,
        "{}",
        chooser_redirect.text()
    );
    assert_eq!(chooser_redirect.header("location"), "/households/choose");
    let chooser = request.get("/households/choose").await;
    assert_eq!(chooser.status_code(), 200, "{}", chooser.text());
    let created = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("name", household_name.as_str()),
            ("authenticity_token", csrf(&chooser.text())),
        ])
        .await;
    assert_eq!(created.status_code(), 303, "{}", created.text());
    assert!(
        created
            .header("location")
            .to_str()
            .unwrap()
            .starts_with("/authorize?")
    );
    let shown = request
        .get(created.header("location").to_str().unwrap())
        .await;
    assert_eq!(shown.status_code(), 200, "{}", shown.text());
    assert!(
        shown
            .text()
            .contains(&format!("Household: {household_name}"))
    );
    drop(request);
    newcomer.close().await;
    close(fixture, boot).await;
}
