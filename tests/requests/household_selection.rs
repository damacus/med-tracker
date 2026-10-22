use super::*;
use base64::Engine as _;
use sha2::Digest as _;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

async fn boot_browser_fixture() -> (Fixture, BootResult) {
    boot_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await
}

async fn sign_in(request: &TestServer, boot: &BootResult) {
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(signed_in.status_code(), 200, "{}", signed_in.text());
}

async fn sign_in_account_only(request: &TestServer, boot: &BootResult) {
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"household-entry@example.test","password":"password"}))
        .await;
    assert_eq!(signed_in.status_code(), 200, "{}", signed_in.text());
}

#[tokio::test]
async fn verified_account_without_user_or_membership_creates_first_household() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    fixture.admin.execute_unprepared("DELETE FROM app_settings; INSERT INTO app_settings(invite_only,created_at,updated_at) VALUES(true,now(),now())").await?;
    fixture.admin.execute_unprepared("INSERT INTO people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73050,'Household entrant','1990-01-01',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,status,preferences,created_at,updated_at) VALUES(71050,73050,'household-entry@example.test',2,'{\"time_zone\":\"Europe/London\"}',now(),now()); INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71050,now())").await?;
    crate::fixtures::seed_native_password(&fixture.admin, 71050).await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in_account_only(&request, &boot).await;
    let chooser = request.get("/households/choose").await;
    assert_eq!(chooser.status_code(), 200, "{}", chooser.text());
    let html = chooser.text();
    assert!(html.contains("action=\"/households\""));
    let token = authenticity_token(&html).ok_or("creation token missing")?;
    let created = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("name", "New care home"), ("authenticity_token", token)])
        .await;
    assert_eq!(created.status_code(), 303, "{}", created.text());
    let response_request_id = created.header("x-request-id").to_str()?.to_owned();
    let audit = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT request_id, audit_context->>'request_id' AS context_request_id FROM security_audit_events WHERE actor_account_id=71050 AND event_type='household_access.membership_created'")).await?.ok_or("creation audit missing")?;
    assert_eq!(
        audit.try_get::<String>("", "request_id")?,
        response_request_id
    );
    assert_eq!(
        audit.try_get::<String>("", "context_request_id")?,
        response_request_id
    );
    let location_header = created.header("location");
    let location = location_header.to_str()?;
    let dashboard = request.get(location).await;
    assert_eq!(dashboard.status_code(), 200, "{}", dashboard.text());
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) AS memberships FROM household_memberships m JOIN households h ON h.id=m.household_id WHERE m.person_id=73050 AND m.role='owner' AND m.status='active' AND h.name='New care home' AND h.timezone='Europe/London'")).await?.ok_or("membership count missing")?;
    assert_eq!(row.try_get::<i64>("", "memberships")?, 1);
    let identity = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM accounts WHERE person_id=73050) AS accounts, (SELECT count(*) FROM people WHERE id=73050) AS people, (SELECT count(*) FROM users WHERE person_id=73050) AS users, (SELECT count(*) FROM versions WHERE item_type='Person' AND item_id=73050 AND event='create') AS person_events")).await?.ok_or("identity count missing")?;
    assert_eq!(identity.try_get::<i64>("", "accounts")?, 1);
    assert_eq!(identity.try_get::<i64>("", "people")?, 1);
    assert_eq!(identity.try_get::<i64>("", "users")?, 0);
    assert_eq!(identity.try_get::<i64>("", "person_events")?, 0);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn held_or_archived_household_membership_allows_new_selectable_household() -> TestResult {
    for inactive_state in ["held", "archived"] {
        let (fixture, boot) = boot_browser_fixture().await;
        fixture.admin.execute_unprepared("INSERT INTO people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73050,'Household entrant','1990-01-01',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,status,created_at,updated_at) VALUES(71050,73050,'household-entry@example.test',2,now(),now()); INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71050,now()); INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72050,71050,'Previous household','previous-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(74050,72050,73050,'owner','active',now(),now(),now())").await?;
        crate::fixtures::seed_native_password(&fixture.admin, 71050).await;
        let inactive_sql = if inactive_state == "held" {
            "UPDATE households SET lifecycle_state='held' WHERE id=72050"
        } else {
            "UPDATE households SET status='archived' WHERE id=72050"
        };
        fixture.admin.execute_unprepared(inactive_sql).await?;
        let mut request = server(&boot);
        request.save_cookies();
        sign_in_account_only(&request, &boot).await;
        let chooser = request.get("/households/choose").await;
        assert_eq!(chooser.status_code(), 200, "{}", chooser.text());
        let html = chooser.text();
        assert!(html.contains("Create a household"));
        let csrf = authenticity_token(&html).ok_or("creation token missing")?;
        let created = request
            .post("/households")
            .add_header("sec-fetch-site", "same-origin")
            .form(&[
                ("name", "New selectable home"),
                ("authenticity_token", csrf),
            ])
            .await;
        assert_eq!(
            created.status_code(),
            303,
            "{inactive_state}: {}",
            created.text()
        );
        let dashboard = request.get(created.header("location").to_str()?).await;
        assert_eq!(
            dashboard.status_code(),
            200,
            "{inactive_state}: {}",
            dashboard.text()
        );
        let old = request
            .get("/households/previous-household/dashboard")
            .await;
        assert!(
            old.status_code().is_client_error(),
            "{inactive_state}: {}",
            old.text()
        );
        drop(request);
        close(fixture, boot).await;
    }
    Ok(())
}

#[tokio::test]
async fn existing_member_invalid_creation_post_does_not_render_empty_chooser() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET completed_at=now()").await?;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot).await;
    let chooser = request.get("/households/choose?switch=1").await;
    assert_eq!(chooser.status_code(), 200, "{}", chooser.text());
    let chooser_html = chooser.text();
    let csrf = authenticity_token(&chooser_html).ok_or("selection token missing")?;
    let rejected = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("name", " "), ("authenticity_token", csrf)])
        .await;
    assert_eq!(rejected.status_code(), 403, "{}", rejected.text());
    let chooser = request.get("/households/choose?switch=1").await;
    assert_eq!(chooser.status_code(), 200, "{}", chooser.text());
    assert!(chooser.text().contains("action=\"/households/select\""));
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn supplementary_plane_name_above_browser_utf16_limit_is_accepted() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73050,'Household entrant','1990-01-01',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,status,created_at,updated_at) VALUES(71050,73050,'household-entry@example.test',2,now(),now()); INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71050,now())").await?;
    crate::fixtures::seed_native_password(&fixture.admin, 71050).await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in_account_only(&request, &boot).await;
    let html = request.get("/households/choose").await.text();
    let csrf = authenticity_token(&html).ok_or("creation token missing")?;
    let name = format!("Care {}", "😀".repeat(51));
    assert!(name.chars().count() <= 100);
    assert!(name.encode_utf16().count() > 100);
    let created = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("name", name.as_str()), ("authenticity_token", csrf)])
        .await;
    assert_eq!(created.status_code(), 303, "{}", created.text());
    let row = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT name FROM households WHERE created_by_account_id=71050",
        ))
        .await?
        .ok_or("household missing")?;
    assert_eq!(row.try_get::<String>("", "name")?, name);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn verified_account_without_user_or_membership_accepts_invitation() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    fixture.admin.execute_unprepared("DELETE FROM users WHERE person_id=73001; INSERT INTO people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73050,'Household entrant','1990-01-01',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,status,created_at,updated_at) VALUES(71050,73050,'household-entry@example.test',2,now(),now()); INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71050,now()); INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72050,71050,'Departed household','departed-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,revoked_at,joined_at,created_at,updated_at) VALUES(74050,72050,73050,'member','revoked',now(),now()-interval '1 day',now(),now())").await?;
    crate::fixtures::seed_native_password(&fixture.admin, 71050).await;
    let invitation = "account-only-household-invitation";
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO household_invitations(household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(72001,'household-entry@example.test','member',$1,74001,now()+interval '7 days',now(),now())",
        [hex::encode(sha2::Sha256::digest(invitation.as_bytes())).into()],
    )).await?;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in_account_only(&request, &boot).await;
    let preview = request
        .get(&format!("/invitations/accept?token={invitation}"))
        .await;
    assert_eq!(preview.status_code(), 200, "{}", preview.text());
    let html = preview.text();
    let csrf = authenticity_token(&html).ok_or("invitation token missing")?;
    let accepted = request
        .post("/invitations/accept")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("token", invitation), ("authenticity_token", csrf)])
        .await;
    assert_eq!(accepted.status_code(), 303, "{}", accepted.text());
    assert_eq!(
        accepted.header("location"),
        "/households/persistence-fixture/dashboard"
    );
    let dashboard = request
        .get("/households/persistence-fixture/dashboard")
        .await;
    assert_eq!(dashboard.status_code(), 200, "{}", dashboard.text());
    let departed = request
        .get("/households/departed-household/dashboard")
        .await;
    assert!(
        departed.status_code().is_client_error(),
        "{}",
        departed.text()
    );
    let replay = request
        .post("/invitations/accept")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("token", invitation), ("authenticity_token", csrf)])
        .await;
    assert_eq!(replay.status_code(), 303, "{}", replay.text());
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM users WHERE person_id IN (73001,73050)) AS users, (SELECT count(*) FROM household_memberships WHERE household_id=72001 AND person_id=73050 AND status='active') AS memberships, (SELECT count(*) FROM versions WHERE item_type='Person' AND item_id=73050 AND event='create') AS person_events")).await?.ok_or("invitation state missing")?;
    assert_eq!(row.try_get::<i64>("", "users")?, 0);
    assert_eq!(row.try_get::<i64>("", "memberships")?, 1);
    assert_eq!(row.try_get::<i64>("", "person_events")?, 0);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn first_household_creation_rejects_invalid_input_csrf_and_expired_session() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73050,'Household entrant','1990-01-01',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,status,created_at,updated_at) VALUES(71050,73050,'household-entry@example.test',2,now(),now()); INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71050,now())").await?;
    crate::fixtures::seed_native_password(&fixture.admin, 71050).await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in_account_only(&request, &boot).await;
    let chooser = request.get("/households/choose").await;
    let html = chooser.text();
    let csrf = authenticity_token(&html).ok_or("creation token missing")?;
    let invalid_csrf = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("name", "Rejected household"),
            ("authenticity_token", "invalid"),
        ])
        .await;
    assert_eq!(invalid_csrf.status_code(), 403, "{}", invalid_csrf.text());
    let overlong = "x".repeat(101);
    for name in [" ", "A\nB", overlong.as_str()] {
        let invalid = request
            .post("/households")
            .add_header("sec-fetch-site", "same-origin")
            .form(&[("name", name), ("authenticity_token", csrf)])
            .await;
        assert_eq!(invalid.status_code(), 422, "{}", invalid.text());
        assert!(invalid.text().contains("Enter a household name"));
    }
    fixture.admin.execute_unprepared("UPDATE identity_sessions SET expires_at=now()-interval '1 second' WHERE account_id=71050").await?;
    let expired = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("name", "Rejected household"), ("authenticity_token", csrf)])
        .await;
    assert!(
        expired.status_code().is_client_error(),
        "{}",
        expired.text()
    );
    let row = fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS households FROM households WHERE created_by_account_id=71050",
        ))
        .await?
        .ok_or("household count missing")?;
    assert_eq!(row.try_get::<i64>("", "households")?, 0);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn concurrent_first_household_submissions_create_only_one_membership() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73050,'Household entrant','1990-01-01',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,status,created_at,updated_at) VALUES(71050,73050,'household-entry@example.test',2,now(),now()); INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71050,now())").await?;
    crate::fixtures::seed_native_password(&fixture.admin, 71050).await;
    let mut first = server(&boot);
    first.save_cookies();
    let mut second = server(&boot);
    second.save_cookies();
    sign_in_account_only(&first, &boot).await;
    sign_in_account_only(&second, &boot).await;
    let first_html = first.get("/households/choose").await.text();
    let second_html = second.get("/households/choose").await.text();
    let first_csrf = authenticity_token(&first_html).ok_or("first creation token missing")?;
    let second_csrf = authenticity_token(&second_html).ok_or("second creation token missing")?;
    let (one, two) = tokio::join!(
        first
            .post("/households")
            .add_header("sec-fetch-site", "same-origin")
            .form(&[("name", "First home"), ("authenticity_token", first_csrf)]),
        second
            .post("/households")
            .add_header("sec-fetch-site", "same-origin")
            .form(&[("name", "Second home"), ("authenticity_token", second_csrf)])
    );
    let statuses = [one.status_code().as_u16(), two.status_code().as_u16()];
    assert!(
        statuses.contains(&303),
        "{statuses:?}: {} / {}",
        one.text(),
        two.text()
    );
    assert!(
        statuses.contains(&403),
        "{statuses:?}: {} / {}",
        one.text(),
        two.text()
    );
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM households WHERE created_by_account_id=71050) AS households, (SELECT count(*) FROM household_memberships WHERE person_id=73050 AND status='active') AS memberships, (SELECT count(*) FROM security_audit_events WHERE actor_account_id=71050 AND event_type='household_access.membership_created') AS audits")).await?.ok_or("creation count missing")?;
    assert_eq!(row.try_get::<i64>("", "households")?, 1);
    assert_eq!(row.try_get::<i64>("", "memberships")?, 1);
    assert_eq!(row.try_get::<i64>("", "audits")?, 1);
    drop(first);
    drop(second);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn revoked_last_membership_allows_new_household_without_restoring_access() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73050,'Household entrant','1990-01-01',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,status,created_at,updated_at) VALUES(71050,73050,'household-entry@example.test',2,now(),now()); INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71050,now())").await?;
    crate::fixtures::seed_native_password(&fixture.admin, 71050).await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in_account_only(&request, &boot).await;
    let chooser = request.get("/households/choose").await.text();
    let csrf = authenticity_token(&chooser).ok_or("creation token missing")?;
    let first = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("name", "First home"), ("authenticity_token", csrf)])
        .await;
    assert_eq!(first.status_code(), 303, "{}", first.text());
    let first_header = first.header("location");
    let first_path = first_header.to_str()?;
    let duplicate = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("name", "Duplicate home"), ("authenticity_token", csrf)])
        .await;
    assert_eq!(duplicate.status_code(), 403, "{}", duplicate.text());
    fixture.admin.execute_unprepared("UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE person_id=73050").await?;
    let denied = request.get(first_path).await;
    assert!(denied.status_code().is_client_error(), "{}", denied.text());
    let empty = request.get("/households/choose").await;
    assert_eq!(empty.status_code(), 200, "{}", empty.text());
    let empty_html = empty.text();
    let new_csrf = authenticity_token(&empty_html).ok_or("new creation token missing")?;
    let second = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("name", "Second home"), ("authenticity_token", new_csrf)])
        .await;
    assert_eq!(second.status_code(), 303, "{}", second.text());
    let second_header = second.header("location");
    let second_dashboard = request.get(second_header.to_str()?).await;
    assert_eq!(
        second_dashboard.status_code(),
        200,
        "{}",
        second_dashboard.text()
    );
    let still_denied = request.get(first_path).await;
    assert!(
        still_denied.status_code().is_client_error(),
        "{}",
        still_denied.text()
    );
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) FILTER(WHERE status='active') AS active, count(*) FILTER(WHERE status='revoked') AS revoked FROM household_memberships WHERE person_id=73050")).await?.ok_or("membership count missing")?;
    assert_eq!(row.try_get::<i64>("", "active")?, 1);
    assert_eq!(row.try_get::<i64>("", "revoked")?, 1);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn first_household_creation_rolls_back_when_audit_fails() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73050,'Household entrant','1990-01-01',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,status,created_at,updated_at) VALUES(71050,73050,'household-entry@example.test',2,now(),now()); INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71050,now()); CREATE FUNCTION reject_first_household_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.actor_account_id=71050 AND NEW.event_type='household_access.membership_created' THEN RAISE EXCEPTION 'synthetic audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_first_household_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION reject_first_household_audit()").await?;
    crate::fixtures::seed_native_password(&fixture.admin, 71050).await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in_account_only(&request, &boot).await;
    let chooser = request.get("/households/choose").await.text();
    let csrf = authenticity_token(&chooser).ok_or("creation token missing")?;
    let failed = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("name", "Should roll back"), ("authenticity_token", csrf)])
        .await;
    assert_eq!(failed.status_code(), 503, "{}", failed.text());
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM households WHERE created_by_account_id=71050) AS households, (SELECT count(*) FROM household_memberships WHERE person_id=73050) AS memberships, (SELECT count(*) FROM security_audit_events WHERE actor_account_id=71050 AND event_type='household_access.membership_created') AS audits")).await?.ok_or("rollback state missing")?;
    assert_eq!(row.try_get::<i64>("", "households")?, 0);
    assert_eq!(row.try_get::<i64>("", "memberships")?, 0);
    assert_eq!(row.try_get::<i64>("", "audits")?, 0);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn committed_household_survives_current_session_selection_failure() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73050,'Household entrant','1990-01-01',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,status,created_at,updated_at) VALUES(71050,73050,'household-entry@example.test',2,now(),now()); INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71050,now()); CREATE FUNCTION reject_first_household_selection() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.account_id=71050 AND NEW.active_household_id IS DISTINCT FROM OLD.active_household_id THEN RAISE EXCEPTION 'synthetic selection failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_first_household_selection BEFORE UPDATE ON identity_sessions FOR EACH ROW EXECUTE FUNCTION reject_first_household_selection()").await?;
    crate::fixtures::seed_native_password(&fixture.admin, 71050).await;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in_account_only(&request, &boot).await;
    let chooser = request.get("/households/choose").await.text();
    let csrf = authenticity_token(&chooser).ok_or("creation token missing")?;
    let created = request
        .post("/households")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("name", "Recoverable home"), ("authenticity_token", csrf)])
        .await;
    assert_eq!(created.status_code(), 303, "{}", created.text());
    assert_eq!(created.header("location"), "/households/choose");
    let recovery = request.get("/households/choose").await;
    assert_eq!(recovery.status_code(), 200, "{}", recovery.text());
    assert!(
        recovery
            .text()
            .contains("Your household was created. Select it below")
    );
    assert!(recovery.text().contains("Recoverable home"));
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM households WHERE created_by_account_id=71050) AS households, (SELECT count(*) FROM household_memberships WHERE person_id=73050 AND status='active') AS memberships, (SELECT count(*) FROM identity_sessions WHERE account_id=71050 AND active_household_id IS NOT NULL) AS selected")).await?.ok_or("selection state missing")?;
    assert_eq!(row.try_get::<i64>("", "households")?, 1);
    assert_eq!(row.try_get::<i64>("", "memberships")?, 1);
    assert_eq!(row.try_get::<i64>("", "selected")?, 0);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn account_only_invitation_rechecks_csrf_expiry_and_inviter_authority() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    fixture.admin.execute_unprepared("DELETE FROM users WHERE person_id=73001; INSERT INTO people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73050,'Household entrant','1990-01-01',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,status,created_at,updated_at) VALUES(71050,73050,'household-entry@example.test',2,now(),now()); INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71050,now())").await?;
    crate::fixtures::seed_native_password(&fixture.admin, 71050).await;
    let invitation = "account-only-household-invitation";
    fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO household_invitations(household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(72001,'household-entry@example.test','member',$1,74001,now()+interval '7 days',now(),now())",
        [hex::encode(sha2::Sha256::digest(invitation.as_bytes())).into()],
    )).await?;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in_account_only(&request, &boot).await;
    let preview = request
        .get(&format!("/invitations/accept?token={invitation}"))
        .await;
    assert_eq!(preview.status_code(), 200, "{}", preview.text());
    let html = preview.text();
    let csrf = authenticity_token(&html).ok_or("invitation token missing")?;
    let rejected = request
        .post("/invitations/accept")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("token", invitation), ("authenticity_token", "invalid")])
        .await;
    assert_eq!(rejected.status_code(), 403, "{}", rejected.text());
    fixture.admin.execute_unprepared("UPDATE household_invitations SET expires_at=now()-interval '1 second' WHERE email='household-entry@example.test'").await?;
    let expired = request
        .post("/invitations/accept")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("token", invitation), ("authenticity_token", csrf)])
        .await;
    assert_eq!(expired.status_code(), 404, "{}", expired.text());
    fixture.admin.execute_unprepared("UPDATE household_invitations SET expires_at=now()+interval '7 days' WHERE email='household-entry@example.test'; UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE id=74001").await?;
    let revoked_inviter = request
        .post("/invitations/accept")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("token", invitation), ("authenticity_token", csrf)])
        .await;
    assert_eq!(
        revoked_inviter.status_code(),
        404,
        "{}",
        revoked_inviter.text()
    );
    let row = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT (SELECT count(*) FROM household_memberships WHERE person_id=73050) AS memberships, (SELECT count(*) FROM household_invitations WHERE email='household-entry@example.test' AND accepted_at IS NOT NULL) AS accepted")).await?.ok_or("invitation denial state missing")?;
    assert_eq!(row.try_get::<i64>("", "memberships")?, 0);
    assert_eq!(row.try_get::<i64>("", "accepted")?, 0);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

fn authenticity_token(html: &str) -> Option<&str> {
    html.split("name=\"authenticity_token\" value=\"")
        .nth(1)?
        .split('"')
        .next()
}

#[tokio::test]
async fn signed_in_without_active_household_reaches_initial_selection() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET completed_at=now()").await?;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot).await;
    let home = request.get("/").await;
    assert_eq!(home.status_code(), 303, "{}", home.text());
    assert_eq!(home.header("location"), "/households/choose");
    assert_eq!(home.header("cache-control"), "no-store");
    let chooser = request.get("/households/choose").await;
    assert_eq!(chooser.status_code(), 200, "{}", chooser.text());
    assert_eq!(chooser.header("cache-control"), "no-store");
    assert!(chooser.text().contains("action=\"/households/select\""));
    assert!(chooser.text().contains("name=\"authenticity_token\""));
    assert!(chooser.text().contains("name=\"household_id\""));
    let chooser_html = chooser.text();
    let token = authenticity_token(&chooser_html).ok_or("household form token missing")?;
    let selected = request
        .post("/households/select")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("household_id", "72001"), ("authenticity_token", token)])
        .await;
    assert_eq!(selected.status_code(), 303, "{}", selected.text());
    assert_eq!(
        selected.header("location"),
        "/households/persistence-fixture/dashboard"
    );
    let chosen_again = request.get("/households/choose").await;
    assert_eq!(chosen_again.status_code(), 303, "{}", chosen_again.text());
    assert_eq!(
        chosen_again.header("location"),
        "/households/persistence-fixture/dashboard"
    );
    assert_eq!(chosen_again.header("cache-control"), "no-store");
    fixture.admin.execute_unprepared("UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE household_id=72001 AND person_id=(SELECT person_id FROM accounts WHERE id=71001)").await?;
    let revoked_home = request.get("/").await;
    assert_eq!(revoked_home.status_code(), 303, "{}", revoked_home.text());
    assert_eq!(revoked_home.header("location"), "/households/choose");
    assert_eq!(revoked_home.header("cache-control"), "no-store");
    let empty_chooser = request.get("/households/choose").await;
    assert_eq!(empty_chooser.status_code(), 200, "{}", empty_chooser.text());
    assert!(
        empty_chooser
            .text()
            .contains("You do not have an active household membership.")
    );
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn household_selection_requires_csrf_and_does_not_change_session() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET completed_at=now()").await?;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot).await;
    let rejected = request
        .post("/households/select")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("household_id", "72001"), ("authenticity_token", "invalid")])
        .await;
    assert_eq!(rejected.status_code(), 403, "{}", rejected.text());
    assert_eq!(rejected.header("cache-control"), "no-store");
    let missing = request
        .post("/households/select")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("household_id", "72001")])
        .await;
    assert!(
        missing.status_code().is_client_error(),
        "{}",
        missing.text()
    );
    assert_eq!(missing.header("cache-control"), "no-store");
    let home = request.get("/households/choose").await;
    let html = home.text();
    let token = authenticity_token(&html).ok_or("household form token missing")?;
    let foreign = request
        .post("/households/select")
        .add_header("origin", "https://foreign.example.test")
        .add_header("sec-fetch-site", "cross-site")
        .form(&[("household_id", "72001"), ("authenticity_token", token)])
        .await;
    assert_eq!(foreign.status_code(), 403, "{}", foreign.text());
    assert_eq!(foreign.header("cache-control"), "no-store");
    let missing_origin = request
        .post("/households/select")
        .add_header("sec-fetch-site", "cross-site")
        .form(&[("household_id", "72001"), ("authenticity_token", token)])
        .await;
    assert_eq!(
        missing_origin.status_code(),
        403,
        "{}",
        missing_origin.text()
    );
    assert_eq!(missing_origin.header("cache-control"), "no-store");
    let state = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) AS selected FROM identity_sessions WHERE account_id=71001 AND active_household_id IS NOT NULL")).await?.ok_or("identity session must be present")?;
    assert_eq!(state.try_get::<i64>("", "selected")?, 0);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn household_selection_requires_current_account_and_session() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET completed_at=now()").await?;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot).await;
    let home = request.get("/households/choose").await;
    let token = authenticity_token(&home.text())
        .ok_or("household form token missing")?
        .to_owned();
    for mutation in [
        "UPDATE identity_sessions SET expires_at=now()-interval '1 second' WHERE account_id=71001",
        "UPDATE identity_sessions SET expires_at=now()+interval '1 hour' WHERE account_id=71001; UPDATE accounts SET status=1 WHERE id=71001",
    ] {
        fixture.admin.execute_unprepared(mutation).await?;
        let denied = request
            .post("/households/select")
            .add_header("sec-fetch-site", "same-origin")
            .form(&[
                ("household_id", "72001"),
                ("authenticity_token", token.as_str()),
            ])
            .await;
        assert!(denied.status_code().is_client_error(), "{}", denied.text());
        assert_eq!(denied.header("cache-control"), "no-store");
    }
    let state = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) AS selected FROM identity_sessions WHERE account_id=71001 AND active_household_id IS NOT NULL"
    )).await?.ok_or("identity session must be present")?;
    assert_eq!(state.try_get::<i64>("", "selected")?, 0);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn household_selection_is_independent_for_two_browser_sessions() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET completed_at=now(); INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72002,71001,'Second household','second-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,joined_at,created_at,updated_at) VALUES(74002,72002,(SELECT person_id FROM accounts WHERE id=71001),'member',now(),now(),now())").await?;
    let mut first = server(&boot);
    first.save_cookies();
    let mut second = server(&boot);
    second.save_cookies();
    sign_in(&first, &boot).await;
    sign_in(&second, &boot).await;

    let first_home = first.get("/households/choose").await;
    let first_html = first_home.text();
    let first_token =
        authenticity_token(&first_html).ok_or("first household form token missing")?;
    let first_selected = first
        .post("/households/select")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("household_id", "72001"),
            ("authenticity_token", first_token),
        ])
        .await;
    assert_eq!(
        first_selected.status_code(),
        303,
        "{}",
        first_selected.text()
    );
    assert_eq!(
        first_selected.header("location"),
        "/households/persistence-fixture/dashboard"
    );
    let first_home_after_selection = first.get("/").await;
    assert_eq!(
        first_home_after_selection.status_code(),
        303,
        "{}",
        first_home_after_selection.text()
    );
    assert_eq!(
        first_home_after_selection.header("location"),
        "/households/persistence-fixture/dashboard"
    );
    let first_dashboard = first.get("/households/persistence-fixture/dashboard").await;
    assert_eq!(
        first_dashboard.status_code(),
        200,
        "{}",
        first_dashboard.text()
    );
    let navigation = first
        .get("/households/persistence-fixture/profile/navigation")
        .await;
    assert_eq!(navigation.status_code(), 200, "{}", navigation.text());
    assert_eq!(navigation.header("cache-control"), "no-store");
    let shell = &navigation.json::<Value>()["shell"];
    assert_eq!(shell["active_household_id"], "72001");
    assert_eq!(shell["households"].as_array().map(Vec::len), Some(2));
    assert!(shell["households"].as_array().is_some_and(|households| {
        households
            .iter()
            .any(|household| household["id"] == 72002 && household["name"] == "Second household")
    }));

    let second_home = second.get("/households/choose").await;
    let second_html = second_home.text();
    let second_token =
        authenticity_token(&second_html).ok_or("second household form token missing")?;
    let second_selected = second
        .post("/households/select")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("household_id", "72002"),
            ("authenticity_token", second_token),
        ])
        .await;
    assert_eq!(
        second_selected.status_code(),
        303,
        "{}",
        second_selected.text()
    );
    assert_eq!(
        second_selected.header("location"),
        "/households/second-household/dashboard"
    );
    let second_home_after_selection = second.get("/").await;
    assert_eq!(
        second_home_after_selection.status_code(),
        303,
        "{}",
        second_home_after_selection.text()
    );
    assert_eq!(
        second_home_after_selection.header("location"),
        "/households/second-household/dashboard"
    );
    let second_inventory = second.get("/households/second-household/medications").await;
    assert_eq!(
        second_inventory.status_code(),
        200,
        "{}",
        second_inventory.text()
    );

    let first_session = first.get("/api/auth/get-session").await;
    let second_session = second.get("/api/auth/get-session").await;
    assert_eq!(first_session.status_code(), 200, "{}", first_session.text());
    assert_eq!(
        second_session.status_code(),
        200,
        "{}",
        second_session.text()
    );
    let selections = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) FILTER (WHERE active_household_id=72001) AS first, count(*) FILTER (WHERE active_household_id=72002) AS second FROM identity_sessions WHERE account_id=71001 AND active"
    )).await?.ok_or("selected sessions must be present")?;
    assert_eq!(selections.try_get::<i64>("", "first")?, 1);
    assert_eq!(selections.try_get::<i64>("", "second")?, 1);
    drop(first);
    drop(second);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn household_selection_rechecks_membership_and_household_state() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET completed_at=now(); INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72002,71001,'Another household','another-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,joined_at,created_at,updated_at) VALUES(74002,72002,(SELECT person_id FROM accounts WHERE id=71001),'member',now(),now(),now()); INSERT INTO people(id,name,date_of_birth,person_type,has_capacity,created_at,updated_at) VALUES(73992,'Foreign household selector','1990-01-01',0,true,now(),now()); INSERT INTO accounts(id,person_id,email,status,created_at,updated_at) VALUES(71002,73992,'foreign-selector@example.test',2,now(),now()); INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72003,71002,'Foreign household','foreign-selector-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,joined_at,created_at,updated_at) VALUES(74003,72003,(SELECT person_id FROM accounts WHERE id=71002),'owner',now(),now(),now())").await?;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot).await;
    let home = request.get("/households/choose").await;
    let html = home.text();
    let token = authenticity_token(&html)
        .ok_or("household form token missing")?
        .to_owned();
    let selected = request
        .post("/households/select")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("household_id", "72001"), ("authenticity_token", &token)])
        .await;
    assert_eq!(selected.status_code(), 303, "{}", selected.text());

    for (id, setup) in [
        ("99999", "SELECT 1"),
        ("invalid", "SELECT 1"),
        ("0", "SELECT 1"),
        ("-1", "SELECT 1"),
        ("72003", "SELECT 1"),
        (
            "72002",
            "UPDATE households SET lifecycle_state='held' WHERE id=72002",
        ),
        (
            "72002",
            "UPDATE households SET lifecycle_state='active' WHERE id=72002; UPDATE household_memberships SET status='suspended' WHERE id=74002",
        ),
        (
            "72002",
            "UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE id=74002",
        ),
    ] {
        fixture.admin.execute_unprepared(setup).await?;
        let rejected = request
            .post("/households/select")
            .add_header("sec-fetch-site", "same-origin")
            .form(&[("household_id", id), ("authenticity_token", &token)])
            .await;
        assert!(
            rejected.status_code().is_client_error(),
            "{id}: {} {}",
            rejected.status_code(),
            rejected.text()
        );
        assert_eq!(rejected.header("cache-control"), "no-store");
    }
    let state = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) FILTER (WHERE active_household_id=72001) AS retained, count(*) FILTER (WHERE active_household_id<>72001) AS changed FROM identity_sessions WHERE account_id=71001 AND active"
    )).await?.ok_or("identity session must be present")?;
    assert_eq!(state.try_get::<i64>("", "retained")?, 1);
    assert_eq!(state.try_get::<i64>("", "changed")?, 0);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn household_revocation_serializes_with_selection() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET completed_at=now()").await?;
    let mut request = server(&boot);
    request.save_cookies();
    sign_in(&request, &boot).await;
    let home = request.get("/households/choose").await;
    let token = authenticity_token(&home.text())
        .ok_or("household form token missing")?
        .to_owned();

    let blocker = fixture.admin.begin().await?;
    let blocker_pid: i32 = blocker
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_backend_pid() AS pid",
        ))
        .await?
        .ok_or("blocker backend must exist")?
        .try_get("", "pid")?;
    blocker
        .execute_unprepared("SELECT id FROM household_memberships WHERE id=74001 FOR UPDATE")
        .await?;
    let selection = async {
        request
            .post("/households/select")
            .add_header("sec-fetch-site", "same-origin")
            .form(&[
                ("household_id", "72001"),
                ("authenticity_token", token.as_str()),
            ])
            .await
    };
    let release = async {
        let observed = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let row = fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
                    "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE '%household_memberships%' AND query LIKE '%FOR SHARE%' AND $1::int=ANY(pg_blocking_pids(pid))) AS waiting",
                    [blocker_pid.into()],
                )).await?.ok_or("lock observation must return one row")?;
                if row.try_get::<bool>("", "waiting")? {
                    break Ok::<_,Box<dyn std::error::Error>>(true);
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        }).await.unwrap_or(Ok(false))?;
        blocker
            .execute_unprepared(
                "UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE id=74001",
            )
            .await?;
        blocker.commit().await?;
        Ok::<_, Box<dyn std::error::Error>>(observed)
    };
    let (denied, observed) = tokio::join!(selection, release);
    assert!(
        observed?,
        "selection must wait on the revoking membership row; actual HTTP {}",
        denied.status_code()
    );
    assert!(denied.status_code().is_client_error(), "{}", denied.text());
    assert_eq!(denied.header("cache-control"), "no-store");
    let state = fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,
        "SELECT count(*) AS selected FROM identity_sessions WHERE account_id=71001 AND active_household_id IS NOT NULL"
    )).await?.ok_or("identity session must be present")?;
    assert_eq!(state.try_get::<i64>("", "selected")?, 0);
    drop(request);
    close(fixture, boot).await;
    Ok(())
}
