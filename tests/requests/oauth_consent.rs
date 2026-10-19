use super::*;
use base64::Engine as _;

const CONSENT_QUERY: &str = "response_type=code&response_mode=query&client_id=household-consent&redirect_uri=https%3A%2F%2Fexample.test%2Fcallback&scope=patient%2F*.read&state=household-consent-state&code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM&code_challenge_method=S256";

async fn boot_consent_fixture() -> (Fixture, BootResult) {
    boot_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await
}

fn consent_origin(request: &TestServer) -> String {
    request
        .server_url("/authorize")
        .expect("ConnectInfo transport must use the real HTTP listener")
        .origin()
        .ascii_serialization()
}

async fn prepare_households(fixture: &Fixture) {
    fixture.admin.execute_unprepared("DELETE FROM oauth_grants; UPDATE oauth_applications SET client_id='household-consent',client_kind='integration',redirect_uri='https://example.test/callback',scopes='patient/*.read' WHERE id=75001; INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=now(); INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72002,71001,'Selected second household','selected-second','Europe/London',now(),now()); INSERT INTO people(id,account_id,household_id,name,person_type,has_capacity,created_at,updated_at) VALUES(73004,71001,72002,'Selected second adult',0,true,now(),now()); INSERT INTO household_memberships(id,account_id,household_id,person_id,role,permissions_version,joined_at,created_at,updated_at) VALUES(74002,71001,72002,73004,'administrator',7,now(),now(),now())").await.unwrap();
}

async fn sign_in(request: &TestServer, boot: &BootResult) {
    let response = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    let session = request.get("/api/auth/get-session").await;
    assert_eq!(session.status_code(), 200, "{}", session.text());
    assert_eq!(
        session.json::<Value>()["user"]["email"],
        "persistence@example.test"
    );
}

fn approval_fields(html: &str) -> Vec<(String, String)> {
    let document = scraper::Html::parse_document(html);
    let selector = scraper::Selector::parse("#authorize-form input[type=hidden]").unwrap();
    let mut fields = document
        .select(&selector)
        .map(|element| {
            (
                element.value().attr("name").unwrap().to_owned(),
                element.value().attr("value").unwrap().to_owned(),
            )
        })
        .collect::<Vec<_>>();
    for (name, expected) in [
        ("client_id", "household-consent"),
        ("response_type", "code"),
        ("response_mode", "query"),
        ("redirect_uri", "https://example.test/callback"),
        ("state", "household-consent-state"),
        (
            "code_challenge",
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
        ),
        ("code_challenge_method", "S256"),
    ] {
        assert_eq!(fields.iter().filter(|(key, _)| key == name).count(), 1);
        assert!(
            fields
                .iter()
                .any(|(key, value)| key == name && value == expected)
        );
    }
    let csrf = fields
        .iter()
        .filter(|(name, _)| name == "authenticity_token")
        .collect::<Vec<_>>();
    assert_eq!(csrf.len(), 1);
    assert!(!csrf[0].1.is_empty());
    let scopes =
        scraper::Selector::parse("#authorize-form input[type=checkbox][name='scope[]']").unwrap();
    let offered = document
        .select(&scopes)
        .map(|element| element.value().attr("value").unwrap())
        .collect::<Vec<_>>();
    assert_eq!(offered, ["patient/*.read"]);
    fields.push(("scope[]".into(), offered[0].into()));
    let buttons = scraper::Selector::parse("#authorize-form button[type=submit]").unwrap();
    let approve = document
        .select(&buttons)
        .find(|button| button.text().collect::<String>().trim() == "Allow access")
        .expect("real approval submit control must be present");
    if let Some(name) = approve.value().attr("name") {
        fields.push((name.into(), approve.value().attr("value").unwrap().into()));
    }
    fields
}

#[tokio::test]
async fn oauth_consent_displays_and_grants_the_selected_session_household() {
    let (fixture, boot) = boot_consent_fixture().await;
    prepare_households(&fixture).await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot).await;
    fixture
        .admin
        .execute_unprepared(
            "UPDATE identity_sessions SET active_household_id=72002 WHERE account_id=71001",
        )
        .await
        .unwrap();
    let displayed = request.get(&format!("/authorize?{CONSENT_QUERY}")).await;
    assert_eq!(displayed.status_code(), 200, "{}", displayed.text());
    let html = displayed.text();
    let fields = approval_fields(&html);
    let approved = request
        .post("/authorize")
        .add_header("origin", consent_origin(&request))
        .add_header("sec-fetch-site", "same-origin")
        .form(&fields)
        .await;
    let stored = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT g.household_membership_id,g.person_id,g.permissions_version,a.household_id AS audit_household_id,a.actor_membership_id FROM oauth_grants g JOIN security_audit_events a ON a.metadata->>'oauth_application_id'=g.oauth_application_id::text AND a.actor_account_id=g.account_id AND a.actor_membership_id=g.household_membership_id WHERE g.oauth_application_id=75001 AND a.event_type='smart_oauth.consent_granted'"
    )).await.unwrap();
    let status = approved.status_code();
    drop(request);
    close(fixture, boot).await;
    assert!(html.contains("Household: Selected second household"));
    assert!(html.contains("Person: Selected second adult"));
    assert!(!html.contains("Household: Synthetic household"));
    assert_eq!(status, 303);
    let stored = stored.expect("successful approval must atomically persist its grant and audit");
    assert_eq!(
        stored
            .try_get::<i64>("", "household_membership_id")
            .unwrap(),
        74002
    );
    assert_eq!(stored.try_get::<i64>("", "person_id").unwrap(), 73004);
    assert_eq!(stored.try_get::<i32>("", "permissions_version").unwrap(), 7);
    assert_eq!(
        stored.try_get::<i64>("", "audit_household_id").unwrap(),
        72002
    );
    assert_eq!(
        stored.try_get::<i64>("", "actor_membership_id").unwrap(),
        74002
    );
}

#[tokio::test]
async fn oauth_consent_revoked_displayed_household_cannot_fall_back_on_approval() {
    let (fixture, boot) = boot_consent_fixture().await;
    prepare_households(&fixture).await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot).await;
    fixture
        .admin
        .execute_unprepared(
            "UPDATE identity_sessions SET active_household_id=72001 WHERE account_id=71001",
        )
        .await
        .unwrap();
    let displayed = request.get(&format!("/authorize?{CONSENT_QUERY}")).await;
    assert_eq!(displayed.status_code(), 200, "{}", displayed.text());
    let html = displayed.text();
    assert!(html.contains("Household: Synthetic household"));
    let fields = approval_fields(&html);
    fixture.admin.execute_unprepared("UPDATE household_memberships SET revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74001").await.unwrap();
    let approved = request
        .post("/authorize")
        .add_header("origin", consent_origin(&request))
        .add_header("sec-fetch-site", "same-origin")
        .form(&fields)
        .await;
    let status = approved.status_code();
    let stored = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM oauth_grants WHERE oauth_application_id=75001) AS grants,(SELECT count(*) FROM security_audit_events WHERE event_type='smart_oauth.consent_granted') AS audits"
    )).await.unwrap().unwrap();
    drop(request);
    close(fixture, boot).await;
    assert_eq!(
        status, 400,
        "stale displayed authority must be rejected without choosing another membership"
    );
    assert_eq!(stored.try_get::<i64>("", "grants").unwrap(), 0);
    assert_eq!(stored.try_get::<i64>("", "audits").unwrap(), 0);
}

#[tokio::test]
async fn oauth_consent_selected_membership_without_person_does_not_use_another_household() {
    let (fixture, boot) = boot_consent_fixture().await;
    prepare_households(&fixture).await;
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET person_id=NULL WHERE id=74002")
        .await
        .unwrap();
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot).await;
    fixture
        .admin
        .execute_unprepared(
            "UPDATE identity_sessions SET active_household_id=72002 WHERE account_id=71001",
        )
        .await
        .unwrap();
    let displayed = request.get(&format!("/authorize?{CONSENT_QUERY}")).await;
    let status = displayed.status_code();
    let html = displayed.text();
    let stored = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS grants FROM oauth_grants WHERE oauth_application_id=75001",
        ))
        .await
        .unwrap()
        .unwrap();
    drop(request);
    close(fixture, boot).await;
    assert_eq!(status, 400, "selected membership has no person authority");
    assert!(!html.contains("Household: Synthetic household"));
    assert!(!html.contains("Person: Synthetic adult"));
    assert_eq!(stored.try_get::<i64>("", "grants").unwrap(), 0);
}

#[tokio::test]
async fn oauth_consent_selection_and_permission_changes_invalidate_the_displayed_binding() {
    for mutation in [
        "UPDATE identity_sessions SET active_household_id=72002 WHERE account_id=71001",
        "UPDATE household_memberships SET permissions_version=permissions_version+1 WHERE id=74001",
    ] {
        let (fixture, boot) = boot_consent_fixture().await;
        prepare_households(&fixture).await;
        let mut request = server(&boot);
        request.save_cookies();
        sign_in(&request, &boot).await;
        fixture
            .admin
            .execute_unprepared(
                "UPDATE identity_sessions SET active_household_id=72001 WHERE account_id=71001",
            )
            .await
            .unwrap();
        let displayed = request.get(&format!("/authorize?{CONSENT_QUERY}")).await;
        assert_eq!(displayed.status_code(), 200, "{}", displayed.text());
        let html = displayed.text();
        assert!(html.contains("Household: Synthetic household"));
        assert!(html.contains("Person: Synthetic adult"));
        let fields = approval_fields(&html);
        fixture.admin.execute_unprepared(mutation).await.unwrap();
        let approved = request
            .post("/authorize")
            .add_header("origin", consent_origin(&request))
            .add_header("sec-fetch-site", "same-origin")
            .form(&fields)
            .await;
        let status = approved.status_code();
        let stored = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM oauth_grants WHERE oauth_application_id=75001) AS grants,(SELECT count(*) FROM security_audit_events WHERE event_type='smart_oauth.consent_granted') AS audits"
        )).await.unwrap().unwrap();
        drop(request);
        close(fixture, boot).await;
        assert_eq!(status, 400, "displayed authority changed: {mutation}");
        assert_eq!(stored.try_get::<i64>("", "grants").unwrap(), 0);
        assert_eq!(stored.try_get::<i64>("", "audits").unwrap(), 0);
    }
}

#[tokio::test]
async fn oauth_consent_old_form_cannot_approve_a_new_display_with_identical_protocol_fields() {
    let (fixture, boot) = boot_consent_fixture().await;
    prepare_households(&fixture).await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot).await;
    fixture
        .admin
        .execute_unprepared(
            "UPDATE identity_sessions SET active_household_id=72001 WHERE account_id=71001",
        )
        .await
        .unwrap();
    let first = request.get(&format!("/authorize?{CONSENT_QUERY}")).await;
    assert_eq!(first.status_code(), 200, "{}", first.text());
    let first_html = first.text();
    assert!(first_html.contains("Household: Synthetic household"));
    let old_fields = approval_fields(&first_html);
    fixture
        .admin
        .execute_unprepared(
            "UPDATE identity_sessions SET active_household_id=72002 WHERE account_id=71001",
        )
        .await
        .unwrap();
    let second = request.get(&format!("/authorize?{CONSENT_QUERY}")).await;
    assert_eq!(second.status_code(), 200, "{}", second.text());
    let second_html = second.text();
    assert!(second_html.contains("Household: Selected second household"));
    let _new_fields = approval_fields(&second_html);
    let approved = request
        .post("/authorize")
        .add_header("origin", consent_origin(&request))
        .add_header("sec-fetch-site", "same-origin")
        .form(&old_fields)
        .await;
    let status = approved.status_code();
    let stored = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM oauth_grants WHERE oauth_application_id=75001) AS grants,(SELECT count(*) FROM security_audit_events WHERE event_type='smart_oauth.consent_granted') AS audits"
    )).await.unwrap().unwrap();
    drop(request);
    close(fixture, boot).await;
    assert_eq!(
        status, 400,
        "an old form must not approve the newer displayed household"
    );
    assert_eq!(stored.try_get::<i64>("", "grants").unwrap(), 0);
    assert_eq!(stored.try_get::<i64>("", "audits").unwrap(), 0);
}

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn boot_consent_journey_fixture()
-> Result<(Fixture, BootResult, TestServer), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let (fixture, boot) = boot_fixture_with(|config| {
        config.server.host = "http://localhost".into();
        config.server.port = i32::from(address.port());
        eprintln!(
            "Synthetic consent configured origin: {}",
            config.server.full_url()
        );
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let request = TestServer::new(axum::serve(
        listener,
        boot.router
            .clone()
            .ok_or("boot router missing")?
            .into_make_service_with_connect_info::<std::net::SocketAddr>(),
    ))?;
    assert_eq!(
        request
            .server_address()
            .ok_or("HTTP listener missing")?
            .port(),
        Some(address.port())
    );
    Ok((fixture, boot, request))
}

fn form_value(html: &str, selector: &str) -> Result<String, Box<dyn std::error::Error>> {
    let document = scraper::Html::parse_document(html);
    let selector = scraper::Selector::parse(selector).map_err(|_| "invalid fixture selector")?;
    Ok(document
        .select(&selector)
        .next()
        .and_then(|input| input.value().attr("value"))
        .ok_or("required form value missing")?
        .to_owned())
}

#[tokio::test]
async fn oauth_consent_fresh_login_requires_explicit_household_choice_and_resumes_approval()
-> TestResult {
    let (fixture, boot, mut request) = boot_consent_journey_fixture().await?;
    prepare_households(&fixture).await;
    request.save_cookies();
    let origin = boot.app_context.config.server.full_url();
    let anonymous = request
        .get(&format!("{origin}/authorize?{CONSENT_QUERY}"))
        .await;
    assert_eq!(
        anonymous.request_url().origin().ascii_serialization(),
        origin
    );
    assert_eq!(anonymous.status_code(), 303);
    assert_eq!(anonymous.header("location"), "/login");
    let login = request.get(&format!("{origin}/login")).await;
    assert_eq!(login.status_code(), 200, "{}", login.text());
    let token = form_value(
        &login.text(),
        "form[action='/login'] input[name=authenticity_token]",
    )?;
    let signed_in = request
        .post(&format!("{origin}/login"))
        .add_header("origin", origin.clone())
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("email", "persistence@example.test"),
            ("password", "password"),
            ("authenticity_token", token.as_str()),
        ])
        .await;
    assert_eq!(signed_in.status_code(), 303, "{}", signed_in.text());
    let resumed = signed_in.header("location").to_str()?.to_owned();
    assert!(resumed.starts_with("/authorize?"));
    assert_eq!(
        signed_in.request_url().origin().ascii_serialization(),
        origin
    );
    let session = request.get(&format!("{origin}/api/auth/get-session")).await;
    assert_eq!(session.status_code(), 200);
    assert_eq!(
        session.json::<Value>()["user"]["email"],
        "persistence@example.test"
    );
    let unselected = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT active_household_id FROM identity_sessions WHERE account_id=71001 AND active AND purpose='authenticated'"
    )).await?.ok_or("authenticated session missing")?;
    assert!(
        unselected
            .try_get::<Option<i64>>("", "active_household_id")?
            .is_none()
    );
    let needs_choice = request.get(&format!("{origin}{resumed}")).await;
    assert_eq!(needs_choice.status_code(), 303, "{}", needs_choice.text());
    assert_eq!(needs_choice.header("location"), "/");
    let chooser = request.get(&format!("{origin}/")).await;
    assert_eq!(chooser.status_code(), 200);
    assert!(chooser.text().contains("Selected second household"));
    let token = form_value(
        &chooser.text(),
        "form[action='/households/select'] input[name=authenticity_token]",
    )?;
    let selected = request
        .post(&format!("{origin}/households/select"))
        .add_header("origin", origin.clone())
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("household_id", "72002"),
            ("authenticity_token", token.as_str()),
        ])
        .await;
    assert_eq!(selected.status_code(), 303, "{}", selected.text());
    let resumed = selected.header("location").to_str()?.to_owned();
    assert!(resumed.starts_with("/authorize?"));
    let displayed = request.get(&format!("{origin}{resumed}")).await;
    assert_eq!(displayed.status_code(), 200, "{}", displayed.text());
    let html = displayed.text();
    assert!(html.contains("Household: Selected second household"));
    assert!(html.contains("Person: Selected second adult"));
    let approved = request
        .post(&format!("{origin}/authorize"))
        .add_header("origin", origin.clone())
        .add_header("sec-fetch-site", "same-origin")
        .form(&approval_fields(&html))
        .await;
    assert_eq!(approved.status_code(), 303, "{}", approved.text());
    let callback = url::Url::parse(approved.header("location").to_str()?)?;
    assert_eq!(callback.host_str(), Some("example.test"));
    assert_eq!(callback.path(), "/callback");
    assert!(
        callback
            .query_pairs()
            .any(|(key, value)| key == "code" && !value.is_empty())
    );
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT household_membership_id,person_id,permissions_version FROM oauth_grants WHERE oauth_application_id=75001"
    )).await?.ok_or("consent grant missing")?;
    assert_eq!(row.try_get::<i64>("", "household_membership_id")?, 74002);
    assert_eq!(row.try_get::<i64>("", "person_id")?, 73004);
    assert_eq!(row.try_get::<i32>("", "permissions_version")?, 7);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}
