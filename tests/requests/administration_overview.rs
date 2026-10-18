use super::*;
use base64::Engine as _;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn authenticity_token(html: &str) -> Option<&str> {
    html.split("name=\"authenticity_token\" value=\"")
        .nth(1)?
        .split('"')
        .next()
}

async fn select_household(
    request: &TestServer,
    fixture: &Fixture,
    boot: &BootResult,
) -> TestResult {
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,recovery_saved_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET recovery_saved_at=now()").await?;
    let signed_in = request
        .post("/api/auth/sign-in/email")
        .add_header("origin", boot.app_context.config.server.full_url())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .await;
    assert_eq!(signed_in.status_code(), 200, "{}", signed_in.text());
    let chooser = request.get("/households/choose").await;
    let chooser_html = chooser.text();
    let token = authenticity_token(&chooser_html).ok_or("household form token missing")?;
    let selected = request
        .post("/households/select")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[("household_id", "72001"), ("authenticity_token", token)])
        .await;
    assert_eq!(selected.status_code(), 303, "{}", selected.text());
    Ok(())
}

#[tokio::test]
async fn administration_overview_shows_only_current_household_status() -> TestResult {
    let (fixture, boot) = boot_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    fixture.admin.execute_unprepared(
        "INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(87001,72001,'pending-admin@example.test','member','overview-pending',74001,now()+interval '7 days',now(),now()),(87002,72001,'expired-admin@example.test','member','overview-expired',74001,now()-interval '1 day',now(),now()); INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72002,71001,'Other household','other-household','UTC',now(),now()); INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(73004,'Other person',0,true,now(),now()),(73005,'Foreign account person',0,true,now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,joined_at,created_at,updated_at) VALUES(74002,72002,73001,'owner',now(),now(),now()),(74004,72002,73004,'member',now(),now(),now()); INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(87003,72002,'other-admin@example.test','member','overview-other',74002,now()+interval '7 days',now(),now()); INSERT INTO accounts(id,person_id,email,password_hash,status,created_at,updated_at) VALUES(71002,73005,'foreign-admin@example.test',crypt('password',gen_salt('bf',4)),2,now(),now()); INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72003,71002,'Foreign household','foreign-admin-household','UTC',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,joined_at,created_at,updated_at) VALUES(74003,72003,73005,'owner',now(),now(),now())"
    ).await?;
    let identity = fixture.admin.query_one_raw(Statement::from_string(
        DbBackend::Postgres,
        "SELECT a.person_id,count(m.id) AS households FROM accounts a JOIN household_memberships m ON m.person_id=a.person_id WHERE a.id=71001 AND m.household_id IN(72001,72002) GROUP BY a.person_id",
    )).await?.ok_or("shared canonical Person fixture missing")?;
    let shared_person = identity.try_get::<i64>("", "person_id")?;
    let shared_households = identity.try_get::<i64>("", "households")?;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&request, &fixture, &boot).await?;
    let overview = request.get("/households/persistence-fixture/admin").await;
    let overview_status = overview.status_code();
    let overview_cache = overview.header("cache-control").to_owned();
    let html = overview.text();
    let selected_other = request.get("/households/other-household/admin").await;
    let other_status = selected_other.status_code();
    let other_html = selected_other.text();
    let foreign = request
        .get("/households/foreign-admin-household/admin")
        .await;
    let foreign_status = foreign.status_code().as_u16();
    fixture
        .admin
        .execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001")
        .await?;
    let denied = request.get("/households/persistence-fixture/admin").await;
    let denied_status = denied.status_code();
    fixture.admin.execute_unprepared("UPDATE household_memberships SET role='administrator',status='revoked',revoked_at=now() WHERE id=74001").await?;
    let revoked = request.get("/households/persistence-fixture/admin").await;
    let revoked_status = revoked.status_code().as_u16();
    drop(request);
    close(fixture, boot).await;
    assert_eq!(overview_status, 200, "{html}");
    assert_eq!(overview_cache, "no-store");
    assert!(html.contains("Synthetic household administration"));
    assert!(html.contains("3 active members"));
    assert!(html.contains("3 people"));
    assert!(html.contains("1 pending invitation"));
    assert!(!html.contains("Other household"));
    assert!(!html.contains("other-admin@example.test"));
    assert_eq!(other_status, 200, "{other_html}");
    assert!(other_html.contains("Other household administration"));
    assert!(other_html.contains("2 active members"));
    assert!(other_html.contains("2 people"));
    assert!(other_html.contains("1 pending invitation"));
    assert!(!other_html.contains("Synthetic household"));
    assert_eq!(shared_person, 73001);
    assert_eq!(shared_households, 2);
    assert!([403, 404].contains(&foreign_status));
    assert_eq!(denied_status, 403);
    assert!([403, 404].contains(&revoked_status));
    Ok(())
}

#[tokio::test]
async fn administration_overview_shows_five_recent_scoped_safe_events() -> TestResult {
    let (fixture, boot) = boot_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let mut request = server(&boot);
    request.save_cookies();
    let result = async {
        select_household(&request, &fixture, &boot).await?;
        fixture
            .admin
            .execute_unprepared("DELETE FROM security_audit_events WHERE household_id=72001")
            .await?;
        let empty = request.get("/households/persistence-fixture/admin").await;
        let empty_status = empty.status_code();
        let empty_html = empty.text();
        fixture.admin.execute_unprepared(
            "INSERT INTO security_audit_events(id,household_id,actor_account_id,actor_membership_id,event_type,request_id,metadata,audit_context,created_at,updated_at) SELECT 953000+number,72001,71001,74001,'household_membership.role_updated','recent-primary-' || number,'{\"target_membership_id\":74001,\"new_role\":\"administrator\",\"secret\":\"private-clinical-note\"}'::jsonb,'{\"session_reference\":\"private-session\"}'::jsonb,'2030-01-15 12:00:00','2030-01-15 12:00:00' FROM generate_series(1,6) AS number; INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(72002,71001,'Other household','other-household','UTC',now(),now()); INSERT INTO security_audit_events(id,household_id,event_type,request_id,metadata,audit_context,created_at,updated_at) VALUES(953007,72002,'household_membership.role_updated','recent-foreign','{\"secret\":\"foreign-clinical-note\"}'::jsonb,'{}'::jsonb,'2030-01-16 12:00:00','2030-01-16 12:00:00')"
        ).await?;
        let populated = request.get("/households/persistence-fixture/admin").await;
        let populated_status = populated.status_code();
        let populated_cache = populated.header("cache-control").to_owned();
        let populated_html = populated.text();
        Ok::<_, Box<dyn std::error::Error>>((empty_status, empty_html, populated_status, populated_cache, populated_html))
    }.await;
    drop(request);
    close(fixture, boot).await;
    let (empty_status, empty_html, populated_status, populated_cache, populated_html) = result?;
    assert_eq!(empty_status, 200, "{empty_html}");
    assert!(empty_html.contains("Recent activity"));
    assert!(empty_html.contains("No recent administrative activity"));
    assert_eq!(populated_status, 200, "{populated_html}");
    assert_eq!(populated_cache, "no-store");
    assert!(populated_html.contains("Recent activity"));
    assert_eq!(populated_html.matches("View audit event").count(), 5);
    assert!(populated_html.contains("/admin/audit_logs/953006"));
    assert!(!populated_html.contains("/admin/audit_logs/953001"));
    assert!(!populated_html.contains("/admin/audit_logs/953007"));
    assert!(populated_html.contains("Account #71001, membership #74001"));
    assert!(populated_html.contains("Household role changed"));
    assert!(!populated_html.contains("private-clinical-note"));
    assert!(!populated_html.contains("private-session"));
    assert!(!populated_html.contains("foreign-clinical-note"));
    Ok(())
}

#[tokio::test]
async fn navigation_exposes_platform_tools_only_while_platform_authority_is_active() -> TestResult {
    let (fixture, boot) = boot_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&request, &fixture, &boot).await?;
    let path = "/households/persistence-fixture/profile/navigation";
    let ordinary = request.get(path).await;
    let ordinary_status = ordinary.status_code();
    let ordinary_body = ordinary.text();
    fixture.admin.execute_unprepared("INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71001,'active',now(),now())").await?;
    let elevated = request.get(path).await;
    let elevated_status = elevated.status_code();
    let elevated_body = elevated.text();
    fixture
        .admin
        .execute_unprepared("UPDATE platform_admins SET status='disabled' WHERE account_id=71001")
        .await?;
    let revoked = request.get(path).await;
    let revoked_status = revoked.status_code();
    let revoked_body = revoked.text();
    drop(request);
    close(fixture, boot).await;
    assert_eq!(ordinary_status, 200, "{ordinary_body}");
    assert!(!ordinary_body.contains("/platform/users"));
    assert_eq!(elevated_status, 200, "{elevated_body}");
    assert!(elevated_body.contains("/platform/users"));
    assert_eq!(revoked_status, 200, "{revoked_body}");
    assert!(!revoked_body.contains("/platform/users"));
    Ok(())
}

#[tokio::test]
async fn administration_overview_locales_preserve_language_and_household_navigation() -> TestResult
{
    let (fixture, boot) = boot_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&request, &fixture, &boot).await?;
    let mut results = Vec::new();
    for language in ["en", "cy", "ga", "pt", "es"] {
        let response = request
            .get(&format!(
                "/households/persistence-fixture/admin?lang={language}"
            ))
            .add_header("accept-language", "en")
            .await;
        results.push((language, response.status_code(), response.text()));
    }
    let header = request
        .get("/households/persistence-fixture/admin")
        .add_header("accept-language", "fr,es;q=0.9,en;q=0.1")
        .await
        .text();
    let invalid = request
        .get("/households/persistence-fixture/admin?lang=es&lang=en")
        .await
        .status_code();
    drop(request);
    close(fixture, boot).await;
    for (language, status, html) in results {
        assert_eq!(status, 200, "{language}: {html}");
        assert!(
            html.contains(&format!("<html lang=\"{language}\"")),
            "{language}: missing document language"
        );
        assert!(
            html.contains(&format!("/admin/users?lang={language}")),
            "{language}: member link lost language"
        );
        assert!(
            html.contains(&format!("/admin/invitations?lang={language}")),
            "{language}: invitation link lost language"
        );
        assert!(html.contains("Synthetic household"));
        assert!(
            html.contains("/admin/audit_logs"),
            "audit navigation is available in this delivery"
        );
    }
    assert!(header.contains("<html lang=\"es\""));
    assert_eq!(invalid, 422);
    Ok(())
}

#[tokio::test]
async fn administration_locale_selection_survives_shared_links_and_forms() -> TestResult {
    let (fixture, boot) = boot_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&request, &fixture, &boot).await?;
    let mut results = Vec::new();
    for language in ["en", "cy", "ga", "pt", "es"] {
        for path in [
            "users",
            "users/77001/edit",
            "household/edit",
            "carer_relationships",
            "carer_relationships/new",
            "invitations",
        ] {
            let response = request
                .get(&format!(
                    "/households/persistence-fixture/admin/{path}?lang={language}"
                ))
                .add_header("accept-language", "cy")
                .await;
            results.push((language, path, response.status_code(), response.text()));
        }
    }
    drop(request);
    close(fixture, boot).await;
    for (language, path, status, html) in results {
        assert_eq!(status, 200, "{language} {path}: {html}");
        assert!(
            html.contains(&format!("<html lang=\"{language}\"")),
            "{language} {path}"
        );
        for link in html
            .split("href=\"")
            .skip(1)
            .filter_map(|value| value.split('"').next())
        {
            if link.starts_with("/households/persistence-fixture/admin") {
                assert!(
                    link.ends_with(&format!("?lang={language}")),
                    "{language} {path}: {link}"
                );
            }
        }
        if html.contains("method=\"post\"") {
            assert!(
                html.contains(&format!("name=\"lang\" value=\"{language}\"")),
                "{language} {path}: form lost language"
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn administration_locale_invalid_links_return_safe_translated_html() -> TestResult {
    let (fixture, boot) = boot_fixture_with(|config| {
        let key = base64::engine::general_purpose::STANDARD.encode([7_u8; 64]);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] =
            json!({"key":key,"secure":false});
    })
    .await;
    let mut request = server(&boot);
    request.save_cookies();
    select_household(&request, &fixture, &boot).await?;
    let mut results = Vec::new();
    for (language, message) in [
        ("en", "This administration link is invalid."),
        ("cy", "ddolen weinyddu hon yn annilys."),
        ("ga", "Tá an nasc riaracháin seo neamhbhailí."),
        ("pt", "Esta ligação de administração é inválida."),
        ("es", "Este enlace de administración no es válido."),
    ] {
        for path in [
            "admin?lang=es&lang=en",
            "admin?lang=private-sentinel",
            "admin/invitations?lang=es&lang=en",
        ] {
            let response = request
                .get(&format!("/households/persistence-fixture/{path}"))
                .add_header("accept-language", language)
                .await;
            results.push((
                language,
                message,
                response.status_code(),
                response.header("content-type").to_owned(),
                response.text(),
            ));
        }
    }
    drop(request);
    close(fixture, boot).await;
    for (language, message, status, content_type, html) in results {
        assert_eq!(status, 422);
        assert!(content_type.to_str()?.starts_with("text/html"));
        assert!(html.contains(&format!("<html lang=\"{language}\"")));
        assert!(
            html.contains(message),
            "{language}: missing invalid-link recovery copy"
        );
        assert!(html.contains("class=\"btn btn-neutral mt-4\" href=\"/\""));
        assert!(!html.contains("private-sentinel"));
    }
    Ok(())
}
