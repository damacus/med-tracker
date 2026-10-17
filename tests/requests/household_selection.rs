use super::*;
use base64::Engine as _;

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

fn authenticity_token(html: &str) -> Option<&str> {
    html.split("name=\"authenticity_token\" value=\"")
        .nth(1)?
        .split('"')
        .next()
}

#[tokio::test]
async fn signed_in_without_active_household_reaches_initial_selection() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=now()").await?;
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
    fixture.admin.execute_unprepared("UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE household_id=72001 AND account_id=71001").await?;
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
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=now()").await?;
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
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=now()").await?;
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
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=now(); INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72002,71001,'Second household','second-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,account_id,role,joined_at,created_at,updated_at) VALUES(74002,72002,71001,'member',now(),now(),now())").await?;
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
    assert_eq!(
        first_session.json::<Value>()["session"]["activeOrganizationId"],
        "72001"
    );
    assert_eq!(
        second_session.json::<Value>()["session"]["activeOrganizationId"],
        "72002"
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
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=now(); INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72002,71001,'Another household','another-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,account_id,role,joined_at,created_at,updated_at) VALUES(74002,72002,71001,'member',now(),now(),now()); INSERT INTO accounts(id,email,password_hash,status,created_at,updated_at) VALUES(71002,'foreign-selector@example.test',crypt('password',gen_salt('bf',4)),2,now(),now()); INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72003,71002,'Foreign household','foreign-selector-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,account_id,role,joined_at,created_at,updated_at) VALUES(74003,72003,71002,'owner',now(),now(),now())").await?;
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
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=now()").await?;
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
        "selection must wait on the revoking membership row"
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
