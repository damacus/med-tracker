use super::*;
use sea_orm::DbErr;
use std::{error::Error, io};

fn invitation_document(html: &str) -> Result<(Vec<String>, Option<String>), Box<dyn Error>> {
    let document = scraper::Html::parse_document(html);
    let rows = scraper::Selector::parse("li.card h3")
        .map_err(|_| io::Error::other("invalid invitation row selector"))?;
    let next = scraper::Selector::parse("a[rel='next']")
        .map_err(|_| io::Error::other("invalid invitation next selector"))?;
    Ok((
        document
            .select(&rows)
            .map(|row| row.text().collect::<String>().trim().to_owned())
            .collect(),
        document
            .select(&next)
            .next()
            .and_then(|link| link.value().attr("href"))
            .map(str::to_owned),
    ))
}

#[tokio::test]
async fn invitation_browser_pages_fifty_stably_and_preserves_locale() -> Result<(), Box<dyn Error>>
{
    let app = Application::new().await;
    let result = async {
        let cookie = super::super::platform::sign_in(
            &app,
            71001,
            "persistence@example.test",
        )
        .await;
        app.fixture.admin.execute_unprepared(
            "DELETE FROM household_invitation_grants WHERE household_id=72001; DELETE FROM household_invitations WHERE household_id=72001; INSERT INTO household_invitations(household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) SELECT 72001,'browser-page-'||n||'@example.test','member','browser-page-digest-'||n,74001,now()+interval '7 days','2026-01-01','2026-01-01' FROM generate_series(1,101) n",
        ).await?;
        let first_url = format!(
            "{}/households/persistence-fixture/admin/invitations?lang=es",
            app.origin
        );
        let mut destination = first_url.clone();
        let mut seen = Vec::new();
        for expected_count in [50, 50, 1] {
            let response = app.client.get(&destination)
                .header("cookie", &cookie)
                .header("accept-language", "cy")
                .send().await?;
            assert_eq!(response.status(), reqwest::StatusCode::OK);
            assert_eq!(
                response.headers().get("cache-control").and_then(|value| value.to_str().ok()),
                Some("no-store")
            );
            let body = response.text().await?;
            assert!(body.contains("lang=\"es\""));
            let (emails, next) = invitation_document(&body)?;
            assert_eq!(emails.len(), expected_count, "invitation page must be bounded");
            seen.extend(emails);
            if expected_count == 1 {
                assert!(next.is_none());
            } else {
                let next = next.ok_or_else(|| io::Error::other("missing next invitation page"))?;
                let next = url::Url::parse(&destination)?.join(&next)?;
                assert_eq!(next.origin(), url::Url::parse(&app.origin)?.origin());
                assert_eq!(next.path(), "/households/persistence-fixture/admin/invitations");
                assert!(next.query_pairs().any(|(key, value)| key == "lang" && value == "es"));
                destination = next.to_string();
            }
        }
        let expected = (1..=101).rev()
            .map(|number| format!("browser-page-{number}@example.test"))
            .collect::<Vec<_>>();
        assert_eq!(seen, expected);
        let repeated = app.client.get(first_url).header("cookie", &cookie)
            .send().await?.text().await?;
        assert_eq!(
            invitation_document(&repeated)?.0,
            expected.into_iter().take(50).collect::<Vec<_>>()
        );
        Ok::<_, Box<dyn Error>>(())
    }.await;
    app.close().await;
    result
}

async fn select_first_household(app: &Application) -> Result<String, Box<dyn Error>> {
    let cookie = super::super::platform::sign_in(app, 71001, "persistence@example.test").await;
    let (cookie, html) =
        super::super::platform::form_page(app, &cookie, "/households/choose").await;
    let csrf = super::super::platform::hidden_field(&html, "authenticity_token");
    if csrf.is_empty() {
        return Err(io::Error::other("household selection token missing").into());
    }
    let response = super::super::platform::post_form(
        app,
        &cookie,
        "/households/select",
        &[("household_id", "72001"), ("authenticity_token", &csrf)],
    )
    .await;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        response.url().path(),
        "/households/persistence-fixture/dashboard"
    );
    Ok(super::super::platform::response_cookies(&cookie, &response))
}

async fn session_households(app: &Application) -> Result<Vec<(String, i64)>, DbErr> {
    app.fixture.admin.query_all_raw(Statement::from_string(
        DbBackend::Postgres,
        "SELECT id,active_household_id FROM identity_sessions WHERE account_id=71001 AND active AND purpose='authenticated' ORDER BY id",
    )).await?.into_iter().map(|row| Ok((
        row.try_get::<String>("", "id")?,
        row.try_get::<i64>("", "active_household_id")?,
    ))).collect()
}

#[tokio::test]
async fn invitation_acceptance_pages_keep_locale_and_safe_invalid_link_recovery()
-> Result<(), Box<dyn Error>> {
    let app = Application::new().await;
    let result = async {
        super::seed_acceptance(&app).await;
        let cookie = super::super::platform::sign_in(&app, 71001, "persistence@example.test").await;
        for (locale, heading) in [
            ("en", "Accept invitation"),
            ("cy", "Derbyn gwahoddiad"),
            ("ga", "Glac le cuireadh"),
            ("pt", "Aceitar convite"),
            ("es", "Aceptar invitación"),
        ] {
            let response = app
                .client
                .get(format!(
                    "{}/invitations/accept?token=synthetic-account-invitation&lang={locale}",
                    app.origin
                ))
                .header("accept-language", "cy")
                .header("cookie", &cookie)
                .send()
                .await?;
            assert_eq!(response.status(), reqwest::StatusCode::OK, "{locale}");
            assert_eq!(
                response
                    .headers()
                    .get("cache-control")
                    .and_then(|value| value.to_str().ok()),
                Some("no-store")
            );
            let body = response.text().await?;
            assert!(
                body.contains(&format!("<html lang=\"{locale}\"")),
                "document language {locale}"
            );
            assert!(body.contains(heading), "invitation heading {locale}");
            assert!(
                body.contains(&format!("name=\"lang\" value=\"{locale}\"")),
                "form language {locale}"
            );
        }
        for query in [
            "lang=es",
            "token=synthetic-account-invitation&lang=es&lang=en",
            "token=synthetic-account-invitation&token=other&lang=es",
            "token=synthetic-account-invitation&lang=xx",
            "token=synthetic-account-invitation&lang=es&next=https%3A%2F%2Fevil.example.test",
        ] {
            let response = app
                .client
                .get(format!("{}/invitations/accept?{query}", app.origin))
                .header("accept-language", "es")
                .send()
                .await?;
            assert_eq!(
                response.status(),
                reqwest::StatusCode::UNPROCESSABLE_ENTITY,
                "{query}"
            );
            assert_eq!(
                response
                    .headers()
                    .get("cache-control")
                    .and_then(|value| value.to_str().ok()),
                Some("no-store")
            );
            assert!(
                response
                    .headers()
                    .get("content-type")
                    .and_then(|value| value.to_str().ok())
                    .is_some_and(|value| value.starts_with("text/html"))
            );
            let body = response.text().await?;
            let document = scraper::Html::parse_document(&body);
            let recovery = scraper::Selector::parse("section.card a.btn[href='/']")
                .map_err(|_| io::Error::other("invalid recovery selector"))?;
            assert!(
                document.select(&recovery).next().is_some(),
                "local recovery {query}"
            );
            assert!(
                !body.contains("evil.example.test"),
                "query must not be reflected"
            );
        }
        Ok::<_, Box<dyn Error>>(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn invitation_acceptance_selects_only_the_current_session() -> Result<(), Box<dyn Error>> {
    let app = Application::new().await;
    let result = async {
        super::seed_acceptance(&app).await;
        let first = select_first_household(&app).await?;
        let second = select_first_household(&app).await?;
        let before = session_households(&app).await?;
        assert_eq!(before.len(), 2);
        assert!(before.iter().all(|(_, household)| *household == 72001));
        let (first, preview) = super::super::platform::form_page(
            &app,
            &first,
            "/invitations/accept?token=synthetic-account-invitation",
        ).await;
        let csrf = super::super::platform::hidden_field(&preview, "authenticity_token");
        if csrf.is_empty() {
            return Err(io::Error::other("invitation acceptance token missing").into());
        }
        let response = super::super::platform::post_form(
            &app,
            &first,
            "/invitations/accept",
            &[("token", "synthetic-account-invitation"), ("authenticity_token", &csrf)],
        ).await;
        let accepted_status = response.status();
        let accepted_path = response.url().path().to_owned();
        let first = super::super::platform::response_cookies(&first, &response);
        let after = session_households(&app).await?;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(5))
            .build()?;
        let mut destinations = Vec::new();
        for cookie in [&first, &second] {
            let response = client.get(format!("{}/", app.origin))
                .header("cookie", cookie).send().await?;
            destinations.push((response.status(), response.headers().get("location")
                .and_then(|value| value.to_str().ok()).map(str::to_owned)));
        }
        let navigation = app.client.get(format!(
            "{}/households/synthetic-invited-household/profile/navigation", app.origin
        )).header("cookie", &first).send().await?;
        let navigation_status = navigation.status();
        let navigation: Value = navigation.json().await?;
        let state = app.fixture.admin.query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT (SELECT person_id FROM accounts WHERE id=71001) AS person_id,(SELECT count(*) FROM people WHERE id=73001) AS people,(SELECT count(*) FROM household_memberships WHERE household_id=72002 AND person_id=73001) AS memberships,(SELECT count(*) FROM person_access_grants WHERE household_id=72002 AND person_id=73001) AS self_grants,(SELECT count(*) FROM person_access_grants WHERE household_id=72002 AND person_id=73104) AS dependent_grants,(SELECT count(*) FROM carer_relationships WHERE household_id=72002) AS relationships",
        )).await?.ok_or_else(|| io::Error::other("accepted identity state missing"))?;
        assert_eq!(state.try_get::<i64>("", "person_id")?, 73001);
        for (field, count) in [("people", 1), ("memberships", 1), ("self_grants", 0), ("dependent_grants", 0), ("relationships", 0)] {
            assert_eq!(state.try_get::<i64>("", field)?, count, "canonical {field}");
        }
        assert_eq!(accepted_status, reqwest::StatusCode::OK);
        assert_eq!(
            after.iter().filter(|(_, household)| *household == 72002).count(),
            1,
            "acceptance must select the joined household for exactly the current session"
        );
        assert_eq!(after.len(), 2);
        assert_eq!(
            before.iter().map(|(id, _)| id).collect::<Vec<_>>(),
            after.iter().map(|(id, _)| id).collect::<Vec<_>>()
        );
        assert_eq!(after.iter().filter(|(_, household)| *household == 72001).count(), 1);
        assert_eq!(accepted_path, "/households/synthetic-invited-household/dashboard");
        assert_eq!(destinations, vec![
            (reqwest::StatusCode::SEE_OTHER, Some("/households/synthetic-invited-household/dashboard".into())),
            (reqwest::StatusCode::SEE_OTHER, Some("/households/persistence-fixture/dashboard".into())),
        ]);
        assert_eq!(navigation_status, reqwest::StatusCode::OK);
        assert_eq!(navigation["shell"]["active_household_id"], "72002");
        Ok::<_, Box<dyn Error>>(())
    }.await;
    app.close().await;
    result
}

fn invitation_row(document: &scraper::Html, email: &str) -> Result<String, Box<dyn Error>> {
    let selector = scraper::Selector::parse("li.card")
        .map_err(|_| io::Error::other("invalid invitation card selector"))?;
    document
        .select(&selector)
        .find(|row| row.text().any(|text| text.contains(email)))
        .map(|row| row.html())
        .ok_or_else(|| io::Error::other(format!("missing invitation card for {email}")).into())
}

async fn assert_forbidden_presentation(response: reqwest::Response) -> Result<(), Box<dyn Error>> {
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);
    assert!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/html")),
        "forbidden invitation response must render an HTML recovery page"
    );
    assert_eq!(
        response
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );
    let body = response.text().await?;
    assert!(body.contains("<html lang=\"es\""));
    assert!(body.contains("Ya no tienes acceso a esta página de administración."));
    assert!(body.contains("Vuelve al panel para revisar tu acceso actual al hogar."));
    let document = scraper::Html::parse_document(&body);
    let recovery = scraper::Selector::parse("main section a.btn")
        .map_err(|_| io::Error::other("invalid forbidden recovery-link selector"))?;
    let link = document
        .select(&recovery)
        .next()
        .ok_or_else(|| io::Error::other("missing forbidden recovery link"))?;
    assert_eq!(link.value().attr("href"), Some("/"));
    Ok(())
}

#[tokio::test]
async fn invitation_browser_empty_and_terminal_states_expose_only_permitted_actions()
-> Result<(), Box<dyn Error>> {
    let app = Application::new().await;
    let result = async {
        let cookie = super::super::platform::sign_in(&app, 71001, "persistence@example.test").await;
        app.fixture.admin.execute_unprepared("DELETE FROM household_invitation_grants WHERE household_id=72001; DELETE FROM household_invitations WHERE household_id=72001").await?;
        let path = "/households/persistence-fixture/admin/invitations?lang=es";
        let empty = app.client.get(format!("{}{path}", app.origin))
            .header("cookie", &cookie).send().await?;
        assert_eq!(empty.status(), reqwest::StatusCode::OK);
        let empty = empty.text().await?;
        assert!(empty.contains("<html lang=\"es\""));
        assert!(empty.contains("Invitaciones"));
        assert!(empty.contains("No hay invitaciones"), "translated empty state");
        app.fixture.admin.execute_unprepared("INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,accepted_at,revoked_at,created_at,updated_at) VALUES (87101,72001,'pending-c4@example.test','member','pending-c4',74001,now()+interval '7 days',NULL,NULL,now(),now()),(87102,72001,'expired-c4@example.test','administrator','expired-c4',74001,now()-interval '1 day',NULL,NULL,now()-interval '2 days',now()),(87103,72001,'accepted-c4@example.test','member','accepted-c4',74001,now()+interval '7 days',now(),NULL,now()-interval '3 days',now()),(87104,72001,'revoked-c4@example.test','member','revoked-c4',74001,now()+interval '7 days',NULL,now(),now()-interval '4 days',now())").await?;
        let response = app.client.get(format!("{}{path}", app.origin))
            .header("cookie", &cookie).send().await?;
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body = response.text().await?;
        let document = scraper::Html::parse_document(&body);
        for (email, label, actionable) in [
            ("pending-c4@example.test", "Pendiente", true),
            ("expired-c4@example.test", "Caducada", true),
            ("accepted-c4@example.test", "Aceptada", false),
            ("revoked-c4@example.test", "Revocada", false),
        ] {
            let row = invitation_row(&document, email)?;
            assert!(row.contains(label), "translated status for {email}");
            assert_eq!(row.contains("/resend"), actionable, "resend for {email}");
            assert_eq!(row.contains("/cancel"), actionable, "cancel for {email}");
        }
        assert!(body.contains("name=\"lang\" value=\"es\""));
        Ok::<_, Box<dyn Error>>(())
    }.await;
    app.close().await;
    result
}

#[tokio::test]
async fn invitation_browser_locales_preserve_forms_feedback_and_recovery()
-> Result<(), Box<dyn Error>> {
    let app = Application::new().await;
    let result = async {
        let cookie = super::super::platform::sign_in(&app, 71001, "persistence@example.test").await;
        let path = "/households/persistence-fixture/admin/invitations";
        for (locale, heading) in [
            ("en", "Invitations"),
            ("cy", "Gwahoddiadau"),
            ("ga", "Cuirí"),
            ("pt", "Convites"),
            ("es", "Invitaciones"),
        ] {
            let response = app
                .client
                .get(format!("{}{path}?lang={locale}", app.origin))
                .header("cookie", &cookie)
                .header("accept-language", "cy")
                .send()
                .await?;
            assert_eq!(response.status(), reqwest::StatusCode::OK, "{locale}");
            let body = response.text().await?;
            assert!(
                body.contains(&format!("<html lang=\"{locale}\"")),
                "{locale}"
            );
            assert!(body.contains(heading), "translated title {locale}");
            assert!(
                body.contains(&format!("name=\"lang\" value=\"{locale}\"")),
                "form locale {locale}"
            );
        }
        let (cookie, form) =
            super::super::platform::form_page(&app, &cookie, &format!("{path}?lang=es")).await;
        let csrf = super::super::platform::hidden_field(&form, "authenticity_token");
        let before = manager_effect_counts(&app).await?;
        let invalid = super::super::platform::post_form(
            &app,
            &cookie,
            path,
            &[
                ("authenticity_token", &csrf),
                ("lang", "es"),
                ("email", "invalid-email"),
                ("membership_role", "member"),
            ],
        )
        .await;
        assert_eq!(invalid.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
        let invalid = invalid.text().await?;
        assert!(invalid.contains("invalid-email"), "draft preserved");
        assert!(invalid.contains("<html lang=\"es\""));
        let document = scraper::Html::parse_document(&invalid);
        let field_error = scraper::Selector::parse("#form-error-email")
            .map_err(|_| io::Error::other("invalid email error selector"))?;
        let message = document
            .select(&field_error)
            .next()
            .ok_or_else(|| io::Error::other("missing email validation message"))?
            .text()
            .collect::<String>();
        assert!(
            message.contains("correo electrónico") && message.contains("válido"),
            "translated invalid-email message: {message}"
        );
        assert!(!message.contains("Email is invalid"));
        let rejected = super::super::platform::post_form(
            &app,
            &cookie,
            path,
            &[
                ("authenticity_token", &csrf),
                ("lang", "es"),
                ("email", "retired-care-c4@example.test"),
                ("membership_role", "member"),
                ("relationship_type", "parent"),
                ("access_level", "manage"),
                ("dependent_ids[]", "73002"),
            ],
        )
        .await;
        assert_eq!(rejected.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
        let rejected = rejected.text().await?;
        let document = scraper::Html::parse_document(&rejected);
        let base_error = scraper::Selector::parse("#form-error-invitation-form")
            .map_err(|_| io::Error::other("invalid form error selector"))?;
        let message = document
            .select(&base_error)
            .next()
            .ok_or_else(|| io::Error::other("missing retired-field validation message"))?
            .text()
            .collect::<String>();
        assert!(
            message.contains("autoridad de cuidado"),
            "translated retired-field error: {message}"
        );
        assert!(!message.contains("Household invitations cannot assign care authority"));
        assert_eq!(
            manager_effect_counts(&app).await?,
            before,
            "invalid forms cannot create invitation, mail, version, audit or grant effects"
        );
        let created = super::super::platform::post_form(
            &app,
            &cookie,
            path,
            &[
                ("authenticity_token", &csrf),
                ("lang", "es"),
                ("email", "locale-c4@example.test"),
                ("membership_role", "member"),
            ],
        )
        .await;
        assert_eq!(created.status(), reqwest::StatusCode::OK);
        assert!(
            created
                .url()
                .query()
                .is_some_and(|query| query.contains("lang=es"))
        );
        let created = created.text().await?;
        assert!(created.contains("locale-c4@example.test"));
        assert!(created.contains("en cola"), "queued delivery feedback");
        let refreshed = app
            .client
            .get(format!("{}{path}?lang=es", app.origin))
            .header("cookie", &cookie)
            .send()
            .await?
            .text()
            .await?;
        assert!(refreshed.contains("locale-c4@example.test"));
        Ok::<_, Box<dyn Error>>(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn invitation_browser_stale_actions_recheck_manager_and_invitation_state()
-> Result<(), Box<dyn Error>> {
    let app = Application::new().await;
    let result = async {
        super::seed_acceptance(&app).await;
        let cookie = super::super::platform::sign_in(&app, 71001, "persistence@example.test").await;
        let path = "/households/persistence-fixture/admin/invitations";
        let (cookie, html) = super::super::platform::form_page(&app, &cookie, &format!("{path}?lang=es")).await;
        let csrf = super::super::platform::hidden_field(&html, "authenticity_token");
        let foreign = app.client.get(format!("{}/households/synthetic-invited-household/admin/invitations?lang=es", app.origin))
            .header("cookie", &cookie).send().await?;
        assert_forbidden_presentation(foreign).await?;
        app.fixture.admin.execute_unprepared("INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,accepted_at,created_at,updated_at) VALUES (87111,72001,'stale-c4@example.test','member','stale-c4',74001,now()+interval '7 days',now(),now(),now())").await?;
        for action in ["resend", "cancel"] {
            let response = super::super::platform::post_form(&app, &cookie, &format!("{path}/87111/{action}"), &[("authenticity_token", &csrf),("lang", "es")]).await;
            assert_eq!(response.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY, "accepted invitation {action}");
        }
        app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001").await?;
        let denied = super::super::platform::post_form(&app, &cookie, path, &[("authenticity_token", &csrf),("lang", "es"),("email", "lost-authority-c4@example.test"),("membership_role", "member")]).await;
        assert_forbidden_presentation(denied).await?;
        app.fixture.admin.execute_unprepared("UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE id=74001").await?;
        let before = manager_effect_counts(&app).await?;
        let listed = app.client.get(format!("{}{path}?lang=es", app.origin))
            .header("cookie", &cookie).send().await?;
        assert_forbidden_presentation(listed).await?;
        for (action, endpoint) in [
            ("create", path.to_owned()),
            ("resend", format!("{path}/87111/resend")),
            ("cancel", format!("{path}/87111/cancel")),
        ] {
            let response = super::super::platform::post_form(&app, &cookie, &endpoint, &[("authenticity_token", &csrf),("lang", "es"),("email", "revoked-manager-c4@example.test"),("membership_role", "member")]).await;
            assert_forbidden_presentation(response).await.map_err(|error| io::Error::other(format!("revoked manager {action}: {error}")))?;
        }
        assert_eq!(manager_effect_counts(&app).await?, before, "revoked manager has no invitation, mail, version, audit or grant effects");
        Ok::<_, Box<dyn Error>>(())
    }.await;
    app.close().await;
    result
}

async fn manager_effect_counts(app: &Application) -> Result<Vec<i64>, DbErr> {
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT (SELECT count(*) FROM household_invitations WHERE household_id=72001) AS invitations,(SELECT count(*) FROM pg_loco_queue) AS jobs,(SELECT count(*) FROM versions WHERE household_id=72001 AND item_type='HouseholdInvitation') AS versions,(SELECT count(*) FROM security_audit_events WHERE household_id=72001) AS audits,(SELECT count(*) FROM household_invitation_grants WHERE household_id=72001) AS grants")).await?.ok_or_else(|| DbErr::Custom("missing manager-effect state".into()))?;
    ["invitations", "jobs", "versions", "audits", "grants"]
        .iter()
        .map(|field| row.try_get::<i64>("", field))
        .collect()
}

async fn acceptance_effect_counts(app: &Application) -> Result<Vec<i64>, DbErr> {
    let row = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT (SELECT count(*) FROM household_memberships WHERE household_id=72002 AND person_id=73001) AS memberships,(SELECT count(*) FROM versions v WHERE v.item_type='HouseholdInvitation' AND v.item_id=87001 AND v.event='update' AND v.object_changes::jsonb ? 'accepted_at' AND v.audit_context->>'actor_account_id'='71001' AND v.request_id IS NOT NULL) AS versions,(SELECT count(*) FROM security_audit_events a JOIN versions v ON a.request_id=v.request_id WHERE a.household_id=72002 AND a.actor_account_id=71002 AND a.actor_membership_id=74002 AND a.event_type='household_access.membership_created' AND a.metadata->>'target_account_id'='71001' AND v.item_type='HouseholdInvitation' AND v.item_id=87001 AND v.event='update' AND v.object_changes::jsonb ? 'accepted_at') AS audits,(SELECT count(*) FROM pg_loco_queue) AS jobs")).await?.ok_or_else(|| DbErr::Custom("missing acceptance-effect state".into()))?;
    ["memberships", "versions", "audits", "jobs"]
        .iter()
        .map(|field| row.try_get::<i64>("", field))
        .collect()
}

async fn signup_effect_counts(app: &Application, email: &str) -> Result<Vec<i64>, DbErr> {
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT (SELECT count(*) FROM accounts WHERE email=$1) AS accounts,(SELECT count(*) FROM people p JOIN accounts a ON a.person_id=p.id WHERE a.email=$1) AS people,(SELECT count(*) FROM household_memberships m JOIN people p ON p.id=m.person_id JOIN accounts a ON a.person_id=p.id WHERE a.email=$1) AS memberships,(SELECT count(*) FROM pg_loco_queue q WHERE q.task_data->>'to'=$1) AS jobs,(SELECT count(*) FROM household_invitations WHERE id=87003 AND accepted_at IS NOT NULL) AS accepted",
            [email.into()],
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("missing invited-signup state".into()))?;
    ["accounts", "people", "memberships", "jobs", "accepted"]
        .iter()
        .map(|field| row.try_get::<i64>("", field))
        .collect()
}

#[tokio::test]
async fn invitation_signup_invalid_draft_rerenders_localized_form_without_effects()
-> Result<(), Box<dyn Error>> {
    use sha2::{Digest, Sha256};
    let app = Application::new().await;
    let result = async {
        super::seed_acceptance(&app).await;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(87003,72002,'invited-signup@example.test','administrator',$1,74002,now()+interval '7 days',now(),now())",[hex::encode(Sha256::digest("synthetic-signup-invitation".as_bytes())).into()])).await?;
        let (cookie, preview) = super::super::platform::form_page(
            &app,
            "",
            "/invitations/accept?token=synthetic-signup-invitation&lang=es",
        )
        .await;
        assert!(
            preview.contains("name=\"lang\" value=\"es\""),
            "invited signup form must carry the explicit language"
        );
        let csrf = super::super::platform::hidden_field(&preview, "authenticity_token");
        let before = signup_effect_counts(&app, "invited-signup@example.test").await?;
        assert_eq!(before, vec![0, 0, 0, 0, 0]);
        let form = [
            ("authenticity_token", csrf.as_str()),
            ("credential", "password"),
            ("invitation_token", "synthetic-signup-invitation"),
            ("lang", "es"),
            ("name", ""),
            ("date_of_birth", "1992-05-18"),
            ("email", "invited-signup@example.test"),
            ("password", "short"),
            ("password-confirm", "short"),
        ];
        let invalid = super::super::platform::post_form(&app, &cookie, "/create-account", &form).await;
        assert_eq!(invalid.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            invalid
                .headers()
                .get("cache-control")
                .and_then(|value| value.to_str().ok()),
            Some("no-store")
        );
        let invalid = invalid.text().await?;
        assert!(invalid.contains("<html lang=\"es\""), "locale: {invalid}");
        assert!(
            invalid.contains("Synthetic invited household"),
            "invitation context must render: {invalid}"
        );
        let document = scraper::Html::parse_document(&invalid);
        for (selector, reason) in [
            ("#name[aria-invalid=\"true\"]", "invalid name flag"),
            ("#name-error", "name error description"),
            ("form[action=\"/create-account\"] input[name=\"invitation_token\"][value=\"synthetic-signup-invitation\"]", "invitation token draft"),
            ("form[action=\"/create-account\"] input[name=\"lang\"][value=\"es\"]", "language draft"),
            ("form[action=\"/create-account\"] input[name=\"authenticity_token\"]", "fresh csrf token"),
        ] {
            let selector = scraper::Selector::parse(selector)
                .map_err(|_| io::Error::other("invalid invalid-signup selector"))?;
            assert!(
                document.select(&selector).next().is_some(),
                "missing {reason}"
            );
        }
        assert!(invalid.contains("must be present"), "field errors shown");
        assert!(invalid.contains("value=\"1992-05-18\""), "draft date preserved");
        assert!(
            !invalid.contains("value=\"short\""),
            "password is never preserved"
        );
        assert_eq!(
            signup_effect_counts(&app, "invited-signup@example.test").await?,
            before,
            "invalid invited signup must not create account, person, membership, outbox or acceptance effects"
        );
        let mut corrected = form;
        corrected[4].1 = "Synthetic invited signup";
        corrected[7].1 = "Synthetic-password-12!";
        corrected[8].1 = "Synthetic-password-12!";
        let created =
            super::super::platform::post_form(&app, &cookie, "/create-account", &corrected).await;
        assert_eq!(created.status(), reqwest::StatusCode::OK);
        assert_eq!(
            signup_effect_counts(&app, "invited-signup@example.test").await?,
            vec![1, 1, 1, 1, 1],
            "corrected retry provisions the invited account once"
        );
        Ok::<_, Box<dyn Error>>(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn invitation_signup_dead_or_unsupported_link_recovers_in_the_selected_locale()
-> Result<(), Box<dyn Error>> {
    use sha2::{Digest, Sha256};
    let app = Application::new().await;
    let result = async {
        super::seed_acceptance(&app).await;
        app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(87003,72002,'invited-signup@example.test','member',$1,74002,now()+interval '7 days',now(),now())",[hex::encode(Sha256::digest("synthetic-signup-invitation".as_bytes())).into()])).await?;
        let (cookie, preview) = super::super::platform::form_page(
            &app,
            "",
            "/invitations/accept?token=synthetic-signup-invitation&lang=es",
        )
        .await;
        let csrf = super::super::platform::hidden_field(&preview, "authenticity_token");
        let mut form = vec![
            ("authenticity_token", csrf.clone()),
            ("credential", "password".to_owned()),
            ("invitation_token", "synthetic-signup-invitation".to_owned()),
            ("lang", "es".to_owned()),
            ("name", "Synthetic invited signup".to_owned()),
            ("date_of_birth", "1992-05-18".to_owned()),
            ("email", "invited-signup@example.test".to_owned()),
            ("password", "short".to_owned()),
            ("password-confirm", "short".to_owned()),
        ];
        form[3].1 = "xx".to_owned();
        let public = app.context.config.server.full_url();
        let public_url = url::Url::parse(&public).unwrap();
        let authority = match public_url.port() {
            Some(port) => format!("{}:{port}", public_url.host_str().unwrap()),
            None => public_url.host_str().unwrap().to_owned(),
        };
        let unsupported = app
            .client
            .post(format!("{}/create-account", app.origin))
            .header("cookie", &cookie)
            .header("host", authority)
            .header("origin", public.trim_end_matches('/'))
            .header("accept-language", "es")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(
                serde_urlencoded::to_string(
                    form.iter().map(|(k, v)| (*k, v.as_str())).collect::<Vec<_>>(),
                )
                .unwrap(),
            )
            .send()
            .await?;
        assert_eq!(
            unsupported.status(),
            reqwest::StatusCode::UNPROCESSABLE_ENTITY
        );
        let unsupported = unsupported.text().await?;
        assert!(
            unsupported.contains("<html lang=\"es\""),
            "fallback Accept-Language locale: {unsupported}"
        );
        assert!(unsupported.contains("Enlace de invitación no disponible"));
        form[3].1 = "es".to_owned();
        app.fixture
            .admin
            .execute_unprepared(
                "UPDATE household_invitations SET expires_at=now()-interval '1 second' WHERE id=87003",
            )
            .await?;
        let expired = super::super::platform::post_form(
            &app,
            &cookie,
            "/create-account",
            &form.iter().map(|(k, v)| (*k, v.as_str())).collect::<Vec<_>>(),
        )
        .await;
        assert_eq!(expired.status(), reqwest::StatusCode::NOT_FOUND);
        let expired = expired.text().await?;
        assert!(expired.contains("<html lang=\"es\""));
        assert!(expired.contains("Enlace de invitación no disponible"));
        assert_eq!(
            signup_effect_counts(&app, "invited-signup@example.test").await?,
            vec![0, 0, 0, 0, 0],
            "dead or invalid links never create effects"
        );
        Ok::<_, Box<dyn Error>>(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn invitation_acceptance_recovery_chooser_switches_to_the_joined_household()
-> Result<(), Box<dyn Error>> {
    let app = Application::new().await;
    let result = async {
        super::seed_acceptance(&app).await;
        app.fixture.seed_foreign_household(72009).await;
        app.fixture.admin.execute_unprepared("INSERT INTO households(id,created_by_account_id,name,slug,timezone,status,lifecycle_state,created_at,updated_at) VALUES(72010,71999,'Revoked switch household','revoked-switch-72010','UTC','active','active',now(),now()); INSERT INTO household_memberships(id,household_id,person_id,role,status,revoked_at,joined_at,created_at,updated_at) VALUES(74010,72010,73001,'member','revoked',now(),now()-interval '1 day',now(),now())").await?;
        let cookie = select_first_household(&app).await?;
        let (cookie, preview) = super::super::platform::form_page(
            &app,
            &cookie,
            "/invitations/accept?token=synthetic-account-invitation&lang=es",
        )
        .await;
        let csrf = super::super::platform::hidden_field(&preview, "authenticity_token");
        app.fixture.admin.execute_unprepared("CREATE FUNCTION reject_c4_recovery_switch() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.active_household_id=72002 THEN RAISE EXCEPTION 'synthetic session selection failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_c4_recovery_switch BEFORE UPDATE OF active_household_id ON identity_sessions FOR EACH ROW EXECUTE FUNCTION reject_c4_recovery_switch()").await?;
        let failed = super::super::platform::post_form(
            &app,
            &cookie,
            "/invitations/accept",
            &[
                ("token", "synthetic-account-invitation"),
                ("lang", "es"),
                ("authenticity_token", &csrf),
            ],
        )
        .await;
        assert!(
            !failed.status().is_success(),
            "selection must fail after join"
        );
        let recovery = failed.text().await?;
        assert!(
            recovery.contains("Te has unido al hogar"),
            "honest translated recovery"
        );
        assert!(
            recovery.contains("action=\"/invitations/accept\""),
            "retry form remains available"
        );
        let committed = acceptance_effect_counts(&app).await?;
        assert_eq!(committed.first(), Some(&1), "committed membership");
        let document = scraper::Html::parse_document(&recovery);
        let chooser_link = scraper::Selector::parse("a[href^=\"/households/choose\"]")
            .map_err(|_| io::Error::other("invalid chooser link selector"))?;
        let chooser_href = document
            .select(&chooser_link)
            .next()
            .and_then(|link| link.value().attr("href"))
            .ok_or_else(|| io::Error::other("recovery chooser link missing"))?
            .to_owned();
        assert_eq!(
            chooser_href, "/households/choose?switch",
            "recovery must link to the explicit chooser mode"
        );
        let bounced = app
            .client
            .get(format!("{}/households/choose", app.origin))
            .header("cookie", &cookie)
            .send()
            .await?;
        assert_eq!(
            bounced.url().path(),
            "/households/persistence-fixture/dashboard",
            "plain chooser still forwards to the active household dashboard"
        );
        let (cookie, chooser) =
            super::super::platform::form_page(&app, &cookie, &chooser_href).await;
        assert!(
            chooser.contains("Synthetic invited household")
                && chooser.contains("action=\"/households/select\""),
            "chooser lists the newly joined household: {chooser}"
        );
        let chooser_csrf = super::super::platform::hidden_field(&chooser, "authenticity_token");
        for denied_id in ["72009", "72010", "99999"] {
            let denied = super::super::platform::post_form(
                &app,
                &cookie,
                "/households/select",
                &[
                    ("household_id", denied_id),
                    ("authenticity_token", &chooser_csrf),
                ],
            )
            .await;
            assert!(
                denied.status().is_client_error(),
                "foreign or revoked household {denied_id}: {}",
                denied.status()
            );
        }
        let still_old = app
            .client
            .get(format!("{}/", app.origin))
            .header("cookie", &cookie)
            .send()
            .await?;
        assert_eq!(
            still_old.url().path(),
            "/households/persistence-fixture/dashboard",
            "denied selections never move the active household"
        );
        app.fixture.admin.execute_unprepared("DROP TRIGGER reject_c4_recovery_switch ON identity_sessions; DROP FUNCTION reject_c4_recovery_switch()").await?;
        let selected = super::super::platform::post_form(
            &app,
            &cookie,
            "/households/select",
            &[("household_id", "72002"), ("authenticity_token", &chooser_csrf)],
        )
        .await;
        assert_eq!(selected.status(), reqwest::StatusCode::OK);
        assert_eq!(
            selected.url().path(),
            "/households/synthetic-invited-household/dashboard"
        );
        let refreshed = app
            .client
            .get(format!("{}/", app.origin))
            .header("cookie", &cookie)
            .send()
            .await?;
        assert_eq!(
            refreshed.url().path(),
            "/households/synthetic-invited-household/dashboard",
            "refresh follows the newly selected household"
        );
        assert_eq!(
            acceptance_effect_counts(&app).await?,
            committed,
            "chooser fallback must not repeat acceptance effects"
        );
        Ok::<_, Box<dyn Error>>(())
    }
    .await;
    app.close().await;
    result
}

#[tokio::test]
async fn invitation_acceptance_selection_failure_preserves_join_and_recovers()
-> Result<(), Box<dyn Error>> {
    let app = Application::new().await;
    let result = async {
        super::seed_acceptance(&app).await;
        let cookie = select_first_household(&app).await?;
        let (cookie, preview) = super::super::platform::form_page(&app, &cookie, "/invitations/accept?token=synthetic-account-invitation&lang=es").await;
        let csrf = super::super::platform::hidden_field(&preview, "authenticity_token");
        app.fixture.admin.execute_unprepared("CREATE FUNCTION reject_c4_selection() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.active_household_id=72002 THEN RAISE EXCEPTION 'synthetic session selection failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_c4_selection BEFORE UPDATE OF active_household_id ON identity_sessions FOR EACH ROW EXECUTE FUNCTION reject_c4_selection()").await?;
        let failed = super::super::platform::post_form(&app, &cookie, "/invitations/accept", &[("token", "synthetic-account-invitation"),("lang", "es"),("authenticity_token", &csrf)]).await;
        assert!(!failed.status().is_success(), "selection must fail after join");
        let recovery = failed.text().await?;
        assert!(recovery.contains("Te has unido al hogar"), "honest translated recovery");
        assert!(recovery.contains("name=\"token\" value=\"synthetic-account-invitation\""));
        let committed = acceptance_effect_counts(&app).await?;
        assert_eq!(committed.len(), 4);
        assert_eq!(committed.first(), Some(&1), "committed membership");
        assert_eq!(committed.get(1), Some(&1), "attributed invitation update");
        assert_eq!(committed.get(2), Some(&1), "matching membership-created security audit");
        app.fixture.admin.execute_unprepared("DROP TRIGGER reject_c4_selection ON identity_sessions; DROP FUNCTION reject_c4_selection()").await?;
        let retry = super::super::platform::post_form(&app, &cookie, "/invitations/accept", &[("token", "synthetic-account-invitation"),("lang", "es"),("authenticity_token", &csrf)]).await;
        assert_eq!(retry.status(), reqwest::StatusCode::OK);
        assert_eq!(retry.url().path(), "/households/synthetic-invited-household/dashboard");
        assert_eq!(acceptance_effect_counts(&app).await?, committed, "selection retry does not repeat acceptance effects");
        app.fixture.admin.execute_unprepared("UPDATE household_memberships SET status='revoked',revoked_at=now() WHERE household_id=72002 AND person_id=73001").await?;
        let denied = super::super::platform::post_form(&app, &cookie, "/invitations/accept", &[("token", "synthetic-account-invitation"),("lang", "es"),("authenticity_token", &csrf)]).await;
        assert_eq!(denied.status(), reqwest::StatusCode::NOT_FOUND);
        let denied = denied.text().await?;
        assert!(denied.contains("Enlace de invitación no disponible"));
        assert_eq!(acceptance_effect_counts(&app).await?, committed, "revoked replay must not restore membership or duplicate effects");
        let revoked = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT status,revoked_at IS NOT NULL AS revoked FROM household_memberships WHERE household_id=72002 AND person_id=73001")).await?.ok_or_else(|| io::Error::other("missing revoked membership"))?;
        assert_eq!(revoked.try_get::<String>("", "status")?, "revoked");
        assert!(revoked.try_get::<bool>("", "revoked")?);
        Ok::<_, Box<dyn Error>>(())
    }.await;
    app.close().await;
    result
}
