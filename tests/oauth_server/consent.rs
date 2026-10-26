use super::contract::fixture::Fixture;
use super::fixtures;
use loco_rs::{
    TestServer,
    app::Hooks,
    boot::{BootResult, StartMode},
    config::{Config, QueueConfig},
    environment::Environment,
};
use med_tracker::models::_entities::{
    accounts, household_memberships, households, identity_onboarding, oauth_applications,
    oauth_grants, people, security_audit_events,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait,
    QueryFilter,
};
use serde_json::json;

async fn boot() -> (Fixture, BootResult) {
    let fixture = Fixture::new().await;
    oauth_grants::Entity::delete_by_id(76001)
        .exec(&fixture.admin)
        .await
        .unwrap();
    let application = oauth_applications::Entity::find_by_id(75001)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut application = application.into_active_model();
    application.client_kind = Set("integration".into());
    application.redirect_uri = Set("https://example.test/callback".into());
    application.scopes = Set("patient/*.read".into());
    application.update(&fixture.admin).await.unwrap();
    identity_onboarding::Entity::insert(identity_onboarding::ActiveModel {
        account_id: Set(71001),
        completed_at: Set(Some(chrono::Utc::now().fixed_offset())),
    })
    .on_conflict(
        sea_orm::sea_query::OnConflict::column(identity_onboarding::Column::AccountId)
            .do_nothing()
            .to_owned(),
    )
    .exec(&fixture.admin)
    .await
    .unwrap();
    households::Entity::insert(households::ActiveModel {
        id: Set(72002),
        created_by_account_id: Set(Some(71001)),
        name: Set("Second care home".into()),
        slug: Set("second-care-home".into()),
        timezone: Set("Europe/London".into()),
        created_at: Set(chrono::Utc::now().naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    })
    .exec(&fixture.admin)
    .await
    .unwrap();
    household_memberships::Entity::insert(household_memberships::ActiveModel {
        id: Set(74002),
        household_id: Set(72002),
        person_id: Set(73001),
        role: Set("administrator".into()),
        joined_at: Set(chrono::Utc::now().naive_utc()),
        created_at: Set(chrono::Utc::now().naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    })
    .exec(&fixture.admin)
    .await
    .unwrap();
    let mut config = Config::new(&Environment::Test).unwrap();
    config.settings = Some(
        json!({"browser_session": {"key":base64::Engine::encode(&base64::engine::general_purpose::STANDARD, [7_u8; 64]),"secure":false}}),
    );
    config.database.uri = fixture.runtime_uri.clone();
    let Some(QueueConfig::Postgres(queue)) = config.queue.as_mut() else {
        panic!("PostgreSQL queue required")
    };
    queue.uri = fixture.runtime_uri.clone();
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

fn query() -> String {
    serde_urlencoded::to_string([
        ("client_id", "native"),
        ("response_type", "code"),
        ("redirect_uri", "https://example.test/callback"),
        ("scope", "patient/*.read"),
        ("state", "second-household-test"),
        (
            "code_challenge",
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
        ),
        ("code_challenge_method", "S256"),
    ])
    .unwrap()
}

fn approval_fields(html: &str) -> Vec<(&str, &str)> {
    vec![
        ("authenticity_token", csrf(html)),
        ("consent_nonce", nonce(html)),
        ("client_id", "native"),
        ("response_type", "code"),
        ("redirect_uri", "https://example.test/callback"),
        ("state", "second-household-test"),
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
    fixture.close().await;
}

async fn sign_in(request: &TestServer, boot: &BootResult) {
    sign_in_at(request, boot, "shared sign-in").await;
}

async fn sign_in_at(request: &TestServer, boot: &BootResult, stage: &str) {
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
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

async fn consent(request: &TestServer) -> String {
    let response = request.get(&format!("/authorize?{}", query())).await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    response.text()
}

#[tokio::test]
async fn selected_second_household_is_shown_on_integration_consent() {
    let (fixture, boot) = boot().await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot).await;
    let without_selection = request.get(&format!("/authorize?{}", query())).await;
    assert_eq!(
        without_selection.status_code(),
        303,
        "{}",
        without_selection.text()
    );
    assert_eq!(without_selection.header("location"), "/households/choose");
    let resumed = select(&request, "72002").await;
    assert!(resumed.starts_with("/authorize?"), "{resumed}");
    let consent = consent(&request).await;
    assert!(consent.contains("Household: Second care home"), "{consent}");
    let approved = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&approval_fields(&consent))
        .await;
    assert_eq!(approved.status_code(), 200, "{}", approved.text());
    let grant = oauth_grants::Entity::find()
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(grant.household_membership_id, Some(74002));
    assert_eq!(grant.person_id, Some(73001));
    assert_eq!(grant.permissions_version, Some(1));
    assert_eq!(grant.scopes, "patient/*.read");
    let audits = security_audit_events::Entity::find()
        .filter(security_audit_events::Column::EventType.eq("smart_oauth.consent_granted"))
        .all(&fixture.admin)
        .await
        .unwrap();
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].household_id, 72002);
    let replay = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&approval_fields(&consent))
        .await;
    assert_eq!(replay.status_code(), 400, "{}", replay.text());
    assert_eq!(
        oauth_grants::Entity::find()
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
    let (fixture, boot) = boot().await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot).await;
    select(&request, "72002").await;
    let shown = consent(&request).await;
    assert!(shown.contains("Household: Second care home"));
    assert_eq!(
        select(&request, "72001").await,
        "/households/persistence-fixture/dashboard"
    );
    let approved = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&approval_fields(&shown))
        .await;
    assert_eq!(approved.status_code(), 200, "{}", approved.text());
    let grant = oauth_grants::Entity::find()
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(grant.household_membership_id, Some(74002));
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn newer_consent_page_invalidates_old_form_even_with_identical_oauth_fields() {
    let (fixture, boot) = boot().await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot).await;
    select(&request, "72002").await;
    let old = consent(&request).await;
    select(&request, "72001").await;
    let new = consent(&request).await;
    assert!(new.contains("Household: Synthetic household"));
    assert_ne!(nonce(&old), nonce(&new));
    let rejected = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&approval_fields(&old))
        .await;
    assert_eq!(rejected.status_code(), 400, "{}", rejected.text());
    assert_eq!(
        oauth_grants::Entity::find()
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        security_audit_events::Entity::find()
            .filter(security_audit_events::Column::EventType.eq("smart_oauth.consent_granted"))
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    let approved = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&approval_fields(&new))
        .await;
    assert_eq!(approved.status_code(), 200, "{}", approved.text());
    let grant = oauth_grants::Entity::find()
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(grant.household_membership_id, Some(74001));
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn revoked_displayed_membership_cannot_create_grant_or_audit() {
    let (fixture, boot) = boot().await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot).await;
    select(&request, "72002").await;
    let shown = consent(&request).await;
    let membership = household_memberships::Entity::find_by_id(74002)
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
        .form(&approval_fields(&shown))
        .await;
    assert!(
        rejected.status_code().is_client_error(),
        "{}",
        rejected.text()
    );
    assert_eq!(
        oauth_grants::Entity::find()
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        security_audit_events::Entity::find()
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
    let (fixture, boot) = boot().await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot).await;
    select(&request, "72002").await;
    let old = consent(&request).await;
    let membership = household_memberships::Entity::find_by_id(74002)
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    let mut membership = membership.into_active_model();
    membership.status = Set("revoked".into());
    membership.revoked_at = Set(Some(chrono::Utc::now().naive_utc()));
    membership.update(&fixture.admin).await.unwrap();
    let fresh = request.get(&format!("/authorize?{}", query())).await;
    assert_eq!(fresh.status_code(), 303, "{}", fresh.text());
    assert_eq!(fresh.header("location"), "/households/choose");
    let resumed = select(&request, "72001").await;
    assert!(resumed.starts_with("/authorize?"), "{resumed}");
    let new = consent(&request).await;
    assert!(new.contains("Household: Synthetic household"));
    let old_post = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&approval_fields(&old))
        .await;
    assert_eq!(old_post.status_code(), 400, "{}", old_post.text());
    let approved = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&approval_fields(&new))
        .await;
    assert_eq!(approved.status_code(), 200, "{}", approved.text());
    let grant = oauth_grants::Entity::find()
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(grant.household_membership_id, Some(74001));
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn permission_change_invalidates_displayed_consent() {
    assert_changed_consent_rejected("permissions").await;
}

#[tokio::test]
async fn held_household_invalidates_displayed_consent() {
    assert_changed_consent_rejected("held").await;
}

#[tokio::test]
async fn archived_household_invalidates_displayed_consent() {
    assert_changed_consent_rejected("archived").await;
}

async fn assert_changed_consent_rejected(changed: &str) {
    let (fixture, boot) = boot().await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in_at(&request, &boot, changed).await;
    select(&request, "72002").await;
    let shown = consent(&request).await;
    if changed == "permissions" {
        let membership = household_memberships::Entity::find_by_id(74002)
            .one(&fixture.admin)
            .await
            .unwrap()
            .unwrap();
        let mut membership = membership.into_active_model();
        membership.permissions_version = Set(2);
        membership.update(&fixture.admin).await.unwrap();
    } else {
        let household = households::Entity::find_by_id(72002)
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
        .form(&approval_fields(&shown))
        .await;
    assert!(
        rejected.status_code().is_client_error(),
        "{changed}: {}",
        rejected.text()
    );
    assert_eq!(
        oauth_grants::Entity::find()
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        security_audit_events::Entity::find()
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
    let (fixture, boot) = boot().await;
    let mut first = server(&boot);
    let mut second = server(&boot);
    first.save_cookies();
    second.save_cookies();
    sign_in(&first, &boot).await;
    sign_in(&second, &boot).await;
    select(&first, "72001").await;
    select(&second, "72002").await;
    let first_page = consent(&first).await;
    let second_page = consent(&second).await;
    assert!(first_page.contains("Household: Synthetic household"));
    assert!(second_page.contains("Household: Second care home"));
    let mut substituted = approval_fields(&first_page);
    substituted[0].1 = csrf(&second_page);
    let rejected = second
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&substituted)
        .await;
    assert_eq!(rejected.status_code(), 400, "{}", rejected.text());
    assert_eq!(
        oauth_grants::Entity::find()
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    for (request, page) in [(&first, &first_page), (&second, &second_page)] {
        let response = request
            .post("/authorize")
            .add_header("sec-fetch-site", "same-origin")
            .form(&approval_fields(page))
            .await;
        assert_eq!(response.status_code(), 200, "{}", response.text());
    }
    let grants = oauth_grants::Entity::find()
        .all(&fixture.admin)
        .await
        .unwrap();
    assert_eq!(grants.len(), 2);
    assert!(
        grants
            .iter()
            .any(|grant| grant.household_membership_id == Some(74001))
    );
    assert!(
        grants
            .iter()
            .any(|grant| grant.household_membership_id == Some(74002))
    );
    drop(first);
    drop(second);
    close(fixture, boot).await;
}

#[tokio::test]
async fn mobile_consent_stays_account_bound_without_selected_household() {
    let (fixture, boot) = boot().await;
    fixtures::seed_oauth_clients(&fixture.admin).await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot).await;
    let query = serde_urlencoded::to_string([
        ("client_id", "native-journey"),
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
            ("client_id", "native-journey"),
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
        .one(&fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(grant.account_id, 71001);
    assert_eq!(grant.client_kind, "mobile");
    assert_eq!(grant.household_membership_id, None);
    assert_eq!(grant.person_id, None);
    assert_eq!(grant.scopes, "medtracker");
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn consent_rejects_csrf_protocol_scope_and_nonce_tampering() {
    let (fixture, boot) = boot().await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot).await;
    select(&request, "72002").await;
    let shown = consent(&request).await;
    let mut invalid_csrf = approval_fields(&shown);
    invalid_csrf[0].1 = "invalid";
    let response = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&invalid_csrf)
        .await;
    assert_eq!(response.status_code(), 403, "{}", response.text());
    let mut changed_state = approval_fields(&shown);
    changed_state[5].1 = "changed-state";
    let response = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&changed_state)
        .await;
    assert_eq!(response.status_code(), 400, "{}", response.text());
    let mut invalid_scope = approval_fields(&shown);
    invalid_scope.last_mut().unwrap().1 = "admin";
    let response = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&invalid_scope)
        .await;
    assert_eq!(response.status_code(), 400, "{}", response.text());
    let mut duplicate_nonce = approval_fields(&shown);
    duplicate_nonce.push(("consent_nonce", nonce(&shown)));
    let response = request
        .post("/authorize")
        .add_header("sec-fetch-site", "same-origin")
        .form(&duplicate_nonce)
        .await;
    assert_eq!(response.status_code(), 400, "{}", response.text());
    assert_eq!(
        oauth_grants::Entity::find()
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    let mut denial = approval_fields(&shown);
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
            .count(&fixture.admin)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        security_audit_events::Entity::find()
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
    let (fixture, boot) = boot().await;
    people::Entity::insert(people::ActiveModel {
        id: Set(73050),
        name: Set("New account person".into()),
        date_of_birth: Set(Some(chrono::NaiveDate::from_ymd_opt(1990, 1, 1).unwrap())),
        person_type: Set(0),
        has_capacity: Set(true),
        created_at: Set(chrono::Utc::now().naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    })
    .exec(&fixture.admin)
    .await
    .unwrap();
    accounts::Entity::insert(accounts::ActiveModel {
        id: Set(71050),
        person_id: Set(73050),
        email: Set("new-oauth@example.test".into()),
        status: Set(2),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
        ..Default::default()
    })
    .exec(&fixture.admin)
    .await
    .unwrap();
    identity_onboarding::Entity::insert(identity_onboarding::ActiveModel {
        account_id: Set(71050),
        completed_at: Set(Some(chrono::Utc::now().fixed_offset())),
    })
    .exec(&fixture.admin)
    .await
    .unwrap();
    fixtures::seed_native_password(&fixture.admin, 71050).await;
    let mut request = server(&boot);
    request.save_cookies();
    let unauthenticated = request.get(&format!("/authorize?{}", query())).await;
    assert_eq!(unauthenticated.status_code(), 303);
    assert_eq!(unauthenticated.header("location"), "/login");
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"new-oauth@example.test","password":"password"}))
        .await;
    assert_eq!(signed_in.status_code(), 200, "{}", signed_in.text());
    let chooser_redirect = request.get(&format!("/authorize?{}", query())).await;
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
            ("name", "New OAuth household"),
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
    assert!(shown.text().contains("Household: New OAuth household"));
    drop(request);
    close(fixture, boot).await;
}
