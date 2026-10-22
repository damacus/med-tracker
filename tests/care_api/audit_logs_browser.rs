use super::*;
use std::{error::Error, io};

async fn browser_session(app: &Application) -> Result<String, Box<dyn Error>> {
    app.fixture
        .admin
        .execute_unprepared(
            "INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET completed_at=now()",
        )
        .await?;
    let response = app
        .client
        .post(format!("{}/api/auth/sign-in/email", app.origin))
        .header("origin", app.context.config.server.full_url())
        .json(&json!({"email":"persistence@example.test","password":"password"}))
        .send()
        .await?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(io::Error::other("browser sign-in failed").into());
    }
    let cookies = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|value| {
            value
                .to_str()
                .ok()
                .and_then(|value| value.split(';').next())
                .map(str::to_owned)
                .ok_or_else(|| io::Error::other("invalid browser cookie"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if cookies.is_empty() {
        return Err(io::Error::other("browser sign-in set no cookies").into());
    }
    Ok(cookies.join("; "))
}

fn filter_url(origin: &str, fields: &[(&str, &str)]) -> Result<url::Url, url::ParseError> {
    let mut url = url::Url::parse(&format!(
        "{origin}/households/persistence-fixture/admin/audit_logs"
    ))?;
    url.query_pairs_mut().extend_pairs(fields);
    Ok(url)
}

fn rendered_event_ids(html: &str, selector: &str) -> Result<Vec<i64>, Box<dyn Error>> {
    let selector = scraper::Selector::parse(selector)
        .map_err(|_| io::Error::other("invalid audit event selector"))?;
    let document = scraper::Html::parse_document(html);
    document
        .select(&selector)
        .map(|link| {
            let href = link
                .value()
                .attr("href")
                .ok_or_else(|| io::Error::other("audit event link has no href"))?;
            href.rsplit('/')
                .next()
                .ok_or_else(|| io::Error::other("audit event link has no id"))?
                .split('?')
                .next()
                .ok_or_else(|| io::Error::other("audit event link has no id"))?
                .parse::<i64>()
                .map_err(|_| io::Error::other("audit event link has invalid id").into())
        })
        .collect()
}

const AUDIT_LOCALES: [(&str, &str, &str, &str, &str); 5] = [
    (
        "en",
        "Security audit",
        "New role",
        "Recent activity",
        "Check the following details",
    ),
    (
        "cy",
        "Archwiliad diogelwch",
        "Rôl newydd",
        "Gweithgarwch diweddar",
        "Gwiriwch yr hidlwyr archwilio",
    ),
    (
        "ga",
        "Iniúchadh slándála",
        "Ról nua",
        "Gníomhaíocht le déanaí",
        "Seiceáil na scagairí iniúchta",
    ),
    (
        "pt",
        "Auditoria de segurança",
        "Nova função",
        "Atividade recente",
        "Verifique os filtros da auditoria",
    ),
    (
        "es",
        "Auditoría de seguridad",
        "Nuevo rol",
        "Actividad reciente",
        "Revisa los filtros de auditoría",
    ),
];

#[tokio::test]
async fn audit_browser_locales_show_safe_list_detail_recent_and_invalid_filter()
-> Result<(), Box<dyn Error>> {
    let app = Application::new().await;
    let result = async {
        app.fixture.admin.execute_unprepared(
            "INSERT INTO security_audit_events(id,household_id,actor_account_id,actor_membership_id,event_type,request_id,metadata,audit_context,created_at,updated_at) VALUES(956000,72001,71001,74001,'household_membership.role_updated','550e8400-e29b-41d4-a716-446655440000','{\"target_membership_id\":74002,\"previous_role\":\"member\",\"new_role\":\"administrator\",\"outcome\":\"success\",\"clinical_note\":\"audit-locale-private-clinical\",\"token\":\"audit-locale-private-token\"}'::jsonb,'{}'::jsonb,'2090-01-15 12:00:00','2090-01-15 12:00:00')"
        ).await?;
        let cookie = browser_session(&app).await?;
        let mut responses = Vec::new();
        for (language, title, field, recent, invalid) in AUDIT_LOCALES {
            let list = app.client.get(filter_url(&app.origin, &[("lang", language)])?)
                .header("cookie", &cookie).header("accept-language", "cy").send().await?;
            let list_status = list.status();
            let list_body = list.text().await?;
            let detail = app.client.get(format!("{}/households/persistence-fixture/admin/audit_logs/956000?lang={language}", app.origin))
                .header("cookie", &cookie).header("accept-language", "cy").send().await?;
            let detail_status = detail.status();
            let detail_body = detail.text().await?;
            let overview = app.client.get(format!("{}/households/persistence-fixture/admin?lang={language}", app.origin))
                .header("cookie", &cookie).header("accept-language", "cy").send().await?;
            let overview_body = overview.text().await?;
            let invalid_response = app.client.get(filter_url(&app.origin, &[("lang", language), ("from", "2030-02-30")])?)
                .header("cookie", &cookie).send().await?;
            let invalid_status = invalid_response.status();
            let invalid_body = invalid_response.text().await?;
            responses.push((language, title, field, recent, invalid, list_status, list_body, detail_status, detail_body, overview_body, invalid_status, invalid_body));
        }
        Ok::<_, Box<dyn Error>>(responses)
    }.await;
    app.close().await;
    for (
        language,
        title,
        field,
        recent,
        invalid,
        list_status,
        list_body,
        detail_status,
        detail_body,
        overview_body,
        invalid_status,
        invalid_body,
    ) in result?
    {
        assert_eq!(
            list_status,
            reqwest::StatusCode::OK,
            "{language}: {list_body}"
        );
        assert_eq!(
            detail_status,
            reqwest::StatusCode::OK,
            "{language}: {detail_body}"
        );
        assert!(list_body.contains(&format!("lang=\"{language}\"")));
        assert!(list_body.contains(title), "{language}: list title");
        assert!(detail_body.contains(&format!("lang=\"{language}\"")));
        assert!(detail_body.contains(field), "{language}: safe field label");
        for retained in [
            "household_membership.role_updated",
            "71001",
            "74001",
            "74002",
            "550e8400-e29b-41d4-a716-446655440000",
        ] {
            assert!(
                detail_body.contains(retained),
                "{language}: lost canonical attribution {retained}"
            );
        }
        for body in [&list_body, &detail_body, &overview_body, &invalid_body] {
            assert!(
                !body.contains("audit-locale-private"),
                "{language}: private metadata leaked"
            );
        }
        assert!(overview_body.contains(recent), "{language}: recent section");
        assert!(
            overview_body.contains(&format!("/audit_logs/956000?lang={language}")),
            "{language}: recent link lost locale"
        );
        assert_eq!(invalid_status, reqwest::StatusCode::UNPROCESSABLE_ENTITY);
        assert!(
            invalid_body.contains(invalid),
            "{language}: invalid filter guidance: {}",
            invalid_body.chars().take(8192).collect::<String>()
        );
        if language == "en" {
            assert!(
                invalid_body.contains("From date (UTC) is invalid"),
                "English invalid filter lost its field-specific error"
            );
        }
        assert!(
            invalid_body.contains("2030-02-30"),
            "{language}: invalid draft was discarded"
        );
    }
    Ok(())
}

#[tokio::test]
async fn audit_browser_locales_preserve_filters_and_bounded_cursor_pages()
-> Result<(), Box<dyn Error>> {
    let app = Application::new().await;
    let result = async {
        app.fixture.admin.execute_unprepared(
            "INSERT INTO security_audit_events(id,household_id,actor_account_id,actor_membership_id,event_type,request_id,metadata,audit_context,created_at,updated_at) SELECT event_id,72001,71001,74001,'api.request',NULL,'{}'::jsonb,'{}'::jsonb,'2030-01-15 12:00:00','2030-01-15 12:00:00' FROM generate_series(956100,956150) AS event_id"
        ).await?;
        let cookie = browser_session(&app).await?;
        let mut responses = Vec::new();
        for (language, title, _, _, _) in AUDIT_LOCALES {
            let first = app.client.get(filter_url(&app.origin, &[("lang", language), ("event_type", "api.request"), ("from", "2030-01-15"), ("to", "2030-01-15")])?)
                .header("cookie", &cookie).header("accept-language", "cy").send().await?;
            let first_status = first.status();
            let first_body = first.text().await?;
            let document = scraper::Html::parse_document(&first_body);
            let selector = scraper::Selector::parse("a[rel='next']")
                .map_err(|_| io::Error::other("invalid audit next-page selector"))?;
            let next_href = document.select(&selector).next().and_then(|link| link.value().attr("href"))
                .ok_or_else(|| io::Error::other(format!("{language}: missing audit next-page link ({first_status})")))?;
            let next_url = url::Url::parse(&app.origin)?.join(next_href)?;
            let query = next_url.query_pairs().into_owned().collect::<Vec<_>>();
            let next = app.client.get(next_url).header("cookie", &cookie).header("accept-language", "cy").send().await?;
            let next_status = next.status();
            let next_body = next.text().await?;
            responses.push((language, title, first_status, first_body, query, next_status, next_body));
        }
        Ok::<_, Box<dyn Error>>(responses)
    }.await;
    app.close().await;
    for (language, title, first_status, first_body, query, next_status, next_body) in result? {
        assert_eq!(first_status, reqwest::StatusCode::OK);
        assert_eq!(next_status, reqwest::StatusCode::OK);
        for (key, value) in [
            ("lang", language),
            ("event_type", "api.request"),
            ("from", "2030-01-15"),
            ("to", "2030-01-15"),
        ] {
            assert!(
                query.contains(&(key.to_owned(), value.to_owned())),
                "{language}: lost {key}"
            );
        }
        assert!(
            query
                .iter()
                .any(|(key, value)| key == "cursor" && !value.is_empty())
        );
        for body in [&first_body, &next_body] {
            assert!(body.contains(&format!("lang=\"{language}\"")));
            assert!(body.contains(title));
        }
        for selector in [
            "ul[aria-label] a[href*='/admin/audit_logs/']",
            "table a[href*='/admin/audit_logs/']",
        ] {
            let first = rendered_event_ids(&first_body, selector)?;
            let next = rendered_event_ids(&next_body, selector)?;
            assert_eq!(first.len(), 50, "{language}: first page");
            assert_eq!(next.len(), 1, "{language}: second page");
            assert!(first.iter().all(|id| !next.contains(id)));
        }
    }
    Ok(())
}

#[tokio::test]
async fn audit_browser_locales_show_empty_results_and_safe_malformed_query_recovery()
-> Result<(), Box<dyn Error>> {
    let app = Application::new().await;
    let result = async {
        let cookie = browser_session(&app).await?;
        let mut responses = Vec::new();
        for (language, empty_label) in [
            ("en", "No events match these filters."),
            ("cy", "Nid oes digwyddiadau sy'n cyfateb i'r hidlwyr hyn."),
            ("ga", "Ní mheaitseálann aon imeacht na scagairí seo."),
            ("pt", "Nenhum evento corresponde a estes filtros."),
            ("es", "Ningún evento coincide con estos filtros."),
        ] {
            let empty = app
                .client
                .get(filter_url(
                    &app.origin,
                    &[("lang", language), ("event_type", "unknown.empty")],
                )?)
                .header("cookie", &cookie)
                .send()
                .await?;
            let empty_status = empty.status();
            let empty_body = empty.text().await?;
            let mut invalid = Vec::new();
            for fields in [
                vec![("lang", language), ("lang", "en")],
                vec![("lang", "audit-locale-private-query")],
                vec![
                    ("lang", language),
                    ("unknown", "audit-locale-private-query"),
                ],
            ] {
                let response = app
                    .client
                    .get(filter_url(&app.origin, &fields)?)
                    .header("cookie", &cookie)
                    .header("accept-language", language)
                    .send()
                    .await?;
                let status = response.status();
                let html = response
                    .headers()
                    .get("content-type")
                    .and_then(|value| value.to_str().ok())
                    .is_some_and(|value| value.starts_with("text/html"));
                let no_store = response
                    .headers()
                    .get("cache-control")
                    .and_then(|value| value.to_str().ok())
                    == Some("no-store");
                invalid.push((status, html, no_store, response.text().await?));
            }
            responses.push((language, empty_label, empty_status, empty_body, invalid));
        }
        Ok::<_, Box<dyn Error>>(responses)
    }
    .await;
    app.close().await;
    for (language, empty_label, empty_status, empty_body, invalid) in result? {
        assert_eq!(empty_status, reqwest::StatusCode::OK);
        let text = scraper::Html::parse_document(&empty_body)
            .root_element()
            .text()
            .collect::<String>();
        assert!(
            text.contains(empty_label),
            "{language}: missing useful empty result"
        );
        for (status, html, no_store, body) in invalid {
            assert_eq!(status, reqwest::StatusCode::UNPROCESSABLE_ENTITY);
            assert!(
                html && no_store,
                "{language}: malformed query response must be private HTML"
            );
            assert!(
                body.contains(&format!("lang=\"{language}\"")),
                "{language}: malformed query lost locale"
            );
            assert!(!body.contains("audit-locale-private-query"));
        }
    }
    Ok(())
}

#[tokio::test]
async fn audit_browser_shows_scoped_actor_and_safe_event_detail() -> Result<(), Box<dyn Error>> {
    let app = Application::new().await;
    let result = async {
        app.fixture.admin.execute_unprepared(
            "INSERT INTO security_audit_events(id,household_id,actor_account_id,actor_membership_id,event_type,request_id,metadata,audit_context,created_at,updated_at) VALUES(950001,72001,71001,74001,'household_membership.role_updated','audit-browser-request', '{\"target_account_id\":71002,\"target_membership_id\":74002,\"previous_role\":\"member\",\"new_role\":\"administrator\",\"previous_state\":{\"role\":\"member\",\"status\":\"active\",\"person_id\":73004},\"new_state\":{\"role\":\"administrator\",\"status\":\"active\",\"person_id\":73004},\"outcome\":\"success\",\"secret\":\"synthetic-private-clinical-note\"}'::jsonb,'{\"session_reference\":\"synthetic-private-session\"}'::jsonb,'2030-01-01 12:00:00','2030-01-01 12:00:00')"
        ).await?;
        let cookie = browser_session(&app).await?;
        let list = app.client.get(format!("{}/households/persistence-fixture/admin/audit_logs", app.origin))
            .header("cookie", &cookie).send().await?;
        let list_status = list.status();
        let list_cache = list.headers().get("cache-control").and_then(|value| value.to_str().ok()).map(str::to_owned);
        let list_body = list.text().await?;
        let detail = app.client.get(format!("{}/households/persistence-fixture/admin/audit_logs/950001", app.origin))
            .header("cookie", cookie).send().await?;
        let detail_status = detail.status();
        let detail_cache = detail.headers().get("cache-control").and_then(|value| value.to_str().ok()).map(str::to_owned);
        let detail_body = detail.text().await?;
        Ok::<_, Box<dyn Error>>((list_status, list_cache, list_body, detail_status, detail_cache, detail_body))
    }.await;
    app.close().await;
    let (list_status, list_cache, list_body, detail_status, detail_cache, detail_body) = result?;
    assert_eq!(list_status, reqwest::StatusCode::OK);
    assert_eq!(list_cache.as_deref(), Some("no-store"));
    assert!(list_body.contains("household_membership.role_updated"));
    assert!(list_body.contains("/admin/audit_logs/950001"));
    assert!(!list_body.contains("synthetic-private-clinical-note"));
    assert!(!list_body.contains("synthetic-private-session"));
    assert_eq!(detail_status, reqwest::StatusCode::OK);
    assert_eq!(detail_cache.as_deref(), Some("no-store"));
    assert!(detail_body.contains("household_membership.role_updated"));
    assert!(detail_body.contains("71001"));
    assert!(detail_body.contains("74001"));
    assert!(detail_body.contains("74002"));
    assert!(detail_body.contains("administrator"));
    assert!(!detail_body.contains("audit-browser-request"));
    assert!(!detail_body.contains("synthetic-private-clinical-note"));
    assert!(!detail_body.contains("synthetic-private-session"));
    Ok(())
}

#[tokio::test]
async fn audit_browser_rejects_foreign_events_and_lost_manager_authority()
-> Result<(), Box<dyn Error>> {
    let app = Application::new().await;
    let result = async {
        app.fixture.seed_foreign_household(99999).await;
        app.fixture.admin.execute_unprepared(
            "INSERT INTO security_audit_events(id,household_id,actor_account_id,actor_membership_id,event_type,request_id,metadata,audit_context,created_at,updated_at) VALUES(950002,99999,71999,74999,'api.request','foreign-audit-request','{\"secret\":\"synthetic-foreign-clinical-note\"}'::jsonb,'{}'::jsonb,'2030-01-01 12:00:00','2030-01-01 12:00:00')"
        ).await?;
        let cookie = browser_session(&app).await?;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        let foreign = client
            .get(format!("{}/households/persistence-fixture/admin/audit_logs/950002", app.origin))
            .header("cookie", &cookie)
            .send()
            .await?;
        let foreign_status = foreign.status();
        let foreign_cache = foreign.headers().get("cache-control").and_then(|value| value.to_str().ok()).map(str::to_owned);
        let foreign_body = foreign.text().await?;
        app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='member' WHERE id=74001").await?;
        let member = client
            .get(format!("{}/households/persistence-fixture/admin/audit_logs", app.origin))
            .header("cookie", &cookie)
            .send()
            .await?;
        let member_status = member.status();
        let member_cache = member.headers().get("cache-control").and_then(|value| value.to_str().ok()).map(str::to_owned);
        let member_body = member.text().await?;
        app.fixture.admin.execute_unprepared("UPDATE household_memberships SET role='owner',status='revoked',revoked_at=now() WHERE id=74001").await?;
        let revoked = client
            .get(format!("{}/households/persistence-fixture/admin/audit_logs", app.origin))
            .header("cookie", cookie)
            .send()
            .await?;
        let revoked_status = revoked.status();
        let revoked_cache = revoked.headers().get("cache-control").and_then(|value| value.to_str().ok()).map(str::to_owned);
        let revoked_body = revoked.text().await?;
        Ok::<_, Box<dyn Error>>((foreign_status, foreign_cache, foreign_body, member_status, member_cache, member_body, revoked_status, revoked_cache, revoked_body))
    }.await;
    app.close().await;
    let (
        foreign_status,
        foreign_cache,
        foreign_body,
        member_status,
        member_cache,
        member_body,
        revoked_status,
        revoked_cache,
        revoked_body,
    ) = result?;
    assert_eq!(foreign_status, reqwest::StatusCode::NOT_FOUND);
    assert_eq!(foreign_cache.as_deref(), Some("no-store"));
    assert!(!foreign_body.contains("synthetic-foreign-clinical-note"));
    assert_eq!(member_status, reqwest::StatusCode::FORBIDDEN);
    assert_eq!(member_cache.as_deref(), Some("no-store"));
    assert!(!member_body.contains("950002"));
    assert_ne!(revoked_status, reqwest::StatusCode::OK);
    assert_eq!(revoked_cache.as_deref(), Some("no-store"));
    assert!(!revoked_body.contains("950002"));
    Ok(())
}

#[tokio::test]
async fn audit_browser_filters_and_pages_stable_tied_events() -> Result<(), Box<dyn Error>> {
    let app = Application::new().await;
    let result = async {
        app.fixture.admin.execute_unprepared(
            "INSERT INTO security_audit_events(id,household_id,actor_account_id,actor_membership_id,event_type,request_id,metadata,audit_context,created_at,updated_at) SELECT event_id,72001,71001,74001,'api.request','audit-browser-page-' || event_id,'{\"http_method\":\"GET\",\"controller\":\"api/audit\",\"action\":\"index\",\"outcome\":\"success\",\"status\":200}'::jsonb,'{}'::jsonb,'2030-01-15 12:00:00','2030-01-15 12:00:00' FROM generate_series(951000,951050) AS event_id"
        ).await?;
        app.fixture.admin.execute_unprepared(
            "INSERT INTO security_audit_events(id,household_id,actor_account_id,actor_membership_id,event_type,request_id,metadata,audit_context,created_at,updated_at) SELECT event_id,72001,71001,74001,'api.request','audit-browser-edge-' || event_id,'{}'::jsonb,'{}'::jsonb,stamp::timestamp,stamp::timestamp FROM (VALUES (952000,'2030-01-15 00:00:00'),(952001,'2030-01-15 23:59:59'),(952002,'2030-01-14 23:59:59'),(952003,'2030-01-16 00:00:00')) AS edges(event_id,stamp)"
        ).await?;
        let cookie = browser_session(&app).await?;
        let all_blank = app.client.get(filter_url(&app.origin, &[("event_type", ""), ("from", ""), ("to", "")])?)
            .header("cookie", &cookie).send().await?;
        let all_blank_status = all_blank.status();
        let date_only = app.client.get(filter_url(&app.origin, &[("event_type", ""), ("from", "2030-01-15"), ("to", "2030-01-15")])?)
            .header("cookie", &cookie).send().await?;
        let date_only_status = date_only.status();
        let date_only_body = date_only.text().await?;
        let first = app.client.get(format!("{}/households/persistence-fixture/admin/audit_logs?event_type=api.request&from=2030-01-15&to=2030-01-15", app.origin))
            .header("cookie", &cookie).send().await?;
        let first_status = first.status();
        let first_body = first.text().await?;
        let selector = scraper::Selector::parse("a[rel='next']")
            .map_err(|_| io::Error::other("invalid next-page test selector"))?;
        let document = scraper::Html::parse_document(&first_body);
        let next_href = document.select(&selector).next()
            .and_then(|link| link.value().attr("href"))
            .ok_or_else(|| io::Error::other("missing next-page link"))?
            .to_owned();
        app.fixture.admin.execute_unprepared(
            "INSERT INTO security_audit_events(id,household_id,actor_account_id,actor_membership_id,event_type,request_id,metadata,audit_context,created_at,updated_at) VALUES(951100,72001,71001,74001,'api.request','audit-browser-interleaved','{}'::jsonb,'{}'::jsonb,'2030-01-15 12:00:00','2030-01-15 12:00:00')"
        ).await?;
        let next = app.client.get(format!("{}{}", app.origin, next_href))
            .header("cookie", &cookie).send().await?;
        let next_status = next.status();
        let next_body = next.text().await?;
        let cursor = url::Url::parse(&format!("{}{}", app.origin, next_href))?
            .query_pairs()
            .find(|(name, _)| name == "cursor")
            .map(|(_, value)| value.into_owned())
            .ok_or_else(|| io::Error::other("next-page link omitted cursor"))?;
        let mismatch = app.client.get(filter_url(&app.origin, &[("event_type", "household_membership.role_updated"), ("from", "2030-01-15"), ("to", "2030-01-15"), ("cursor", cursor.as_str())])?)
            .header("cookie", &cookie).send().await?;
        let mismatch_status = mismatch.status();
        let mismatch_cache = mismatch.headers().get("cache-control").and_then(|value| value.to_str().ok()).map(str::to_owned);
        let invalid_date = app.client.get(format!("{}/households/persistence-fixture/admin/audit_logs?from=2030-02-30", app.origin))
            .header("cookie", cookie).send().await?;
        let invalid_date_status = invalid_date.status();
        let invalid_date_cache = invalid_date.headers().get("cache-control").and_then(|value| value.to_str().ok()).map(str::to_owned);
        Ok::<_, Box<dyn Error>>((all_blank_status, date_only_status, date_only_body, first_status, first_body, next_status, next_body, mismatch_status, mismatch_cache, invalid_date_status, invalid_date_cache))
    }.await;
    app.close().await;
    let (
        all_blank_status,
        date_only_status,
        date_only_body,
        first_status,
        first_body,
        next_status,
        next_body,
        mismatch_status,
        mismatch_cache,
        invalid_date_status,
        invalid_date_cache,
    ) = result?;
    assert_eq!(all_blank_status, reqwest::StatusCode::OK);
    assert_eq!(date_only_status, reqwest::StatusCode::OK);
    assert!(date_only_body.contains("/admin/audit_logs/952001"));
    assert!(!date_only_body.contains("/admin/audit_logs/952002"));
    assert!(!date_only_body.contains("/admin/audit_logs/952003"));
    assert_eq!(first_status, reqwest::StatusCode::OK);
    let first_mobile =
        rendered_event_ids(&first_body, "ul[aria-label='Audit events'] > li.card a")?;
    let first_desktop = rendered_event_ids(&first_body, "table tbody tr td a")?;
    assert_eq!(first_mobile.len(), 50);
    assert_eq!(first_mobile, first_desktop);
    assert_eq!(
        first_mobile
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        50
    );
    assert_eq!(first_mobile.first(), Some(&952001));
    assert_eq!(first_mobile.last(), Some(&951002));
    assert!(first_body.contains("/admin/audit_logs/952001"));
    assert!(first_body.contains("/admin/audit_logs/951050"));
    assert!(!first_body.contains("/admin/audit_logs/951000"));
    assert_eq!(next_status, reqwest::StatusCode::OK);
    let next_mobile = rendered_event_ids(&next_body, "ul[aria-label='Audit events'] > li.card a")?;
    let next_desktop = rendered_event_ids(&next_body, "table tbody tr td a")?;
    assert_eq!(next_mobile, vec![951001, 951000, 952000]);
    assert_eq!(next_mobile, next_desktop);
    assert_eq!(
        next_mobile
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3
    );
    assert!(next_body.contains("/admin/audit_logs/952000"));
    assert!(next_body.contains("/admin/audit_logs/951001"));
    assert!(next_body.contains("/admin/audit_logs/951000"));
    assert!(!next_body.contains("/admin/audit_logs/951100"));
    assert!(!next_body.contains("/admin/audit_logs/952002"));
    assert!(!next_body.contains("/admin/audit_logs/952003"));
    assert_eq!(mismatch_status, reqwest::StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(mismatch_cache.as_deref(), Some("no-store"));
    assert_eq!(
        invalid_date_status,
        reqwest::StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(invalid_date_cache.as_deref(), Some("no-store"));
    Ok(())
}

#[tokio::test]
async fn audit_browser_retains_invalid_filter_drafts_after_authorization()
-> Result<(), Box<dyn Error>> {
    let app = Application::new().await;
    let result = async {
        let cookie = browser_session(&app).await?;
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        let invalid_event = client
            .get(filter_url(
                &app.origin,
                &[("event_type", "clinical<script>"), ("from", "2030-01-15")],
            )?)
            .header("cookie", &cookie)
            .send()
            .await?;
        let event_status = invalid_event.status();
        let event_cache = invalid_event
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let event_body = invalid_event.text().await?;
        let invalid_date = client
            .get(filter_url(
                &app.origin,
                &[("event_type", "api.request"), ("from", "2030-02-30")],
            )?)
            .header("cookie", &cookie)
            .send()
            .await?;
        let date_status = invalid_date.status();
        let date_cache = invalid_date
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let date_body = invalid_date.text().await?;
        let anonymous = client
            .get(format!(
                "{}/households/persistence-fixture/admin/audit_logs?from=2030-02-30",
                app.origin
            ))
            .send()
            .await?;
        let anonymous_status = anonymous.status();
        let anonymous_cache = anonymous
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let anonymous_body = anonymous.text().await?;
        Ok::<_, Box<dyn Error>>((
            event_status,
            event_cache,
            event_body,
            date_status,
            date_cache,
            date_body,
            anonymous_status,
            anonymous_cache,
            anonymous_body,
        ))
    }
    .await;
    app.close().await;
    let (
        event_status,
        event_cache,
        event_body,
        date_status,
        date_cache,
        date_body,
        anonymous_status,
        anonymous_cache,
        anonymous_body,
    ) = result?;
    assert_eq!(event_status, reqwest::StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(event_cache.as_deref(), Some("no-store"));
    assert!(event_body.contains("Security audit"));
    assert!(event_body.contains("value=\"clinical&lt;script&gt;\""));
    assert!(event_body.contains("value=\"2030-01-15\""));
    assert!(event_body.contains("form-error-audit-event-type"));
    assert!(!event_body.contains("<script>"));
    assert_eq!(date_status, reqwest::StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(date_cache.as_deref(), Some("no-store"));
    assert!(date_body.contains("value=\"api.request\""));
    assert!(date_body.contains("value=\"2030-02-30\""));
    assert!(date_body.contains("form-error-audit-from"));
    assert_eq!(anonymous_status, reqwest::StatusCode::SEE_OTHER);
    assert_eq!(anonymous_cache.as_deref(), Some("no-store"));
    assert!(!anonymous_body.contains("2030-02-30"));
    Ok(())
}
