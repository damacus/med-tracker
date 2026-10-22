use super::*;
use base64::Engine as _;

async fn signed_in() -> (Fixture, BootResult, TestServer) {
    signed_in_with_zone("UTC").await
}

async fn signed_in_with_zone(zone: &str) -> (Fixture, BootResult, TestServer) {
    let (fixture, boot) = boot_fixture_with(|config| {
        config.settings.get_or_insert_with(|| json!({}))["time_zone"] = json!(zone);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] = json!({"key":base64::engine::general_purpose::STANDARD.encode([7_u8;64]),"secure":false});
    }).await;
    fixture.admin.execute_unprepared("INSERT INTO identity_onboarding(account_id,completed_at) VALUES(71001,now()) ON CONFLICT(account_id) DO UPDATE SET completed_at=now()").await.unwrap();
    let mut request = server(&boot);
    request.save_cookies();
    assert_eq!(
        request
            .post("/api/auth/sign-in/email")
            .add_header("origin", boot.app_context.config.server.full_url())
            .json(&json!({"email":"persistence@example.test","password":"password"}))
            .await
            .status_code(),
        200
    );
    let chooser = request.get("/households/choose").await;
    let token = csrf(&chooser.text()).to_owned();
    assert_eq!(
        request
            .post("/households/select")
            .form(&[
                ("household_id", "72001"),
                ("authenticity_token", token.as_str())
            ])
            .await
            .status_code(),
        303
    );
    (fixture, boot, request)
}
fn csrf(html: &str) -> &str {
    html.split("name=\"authenticity_token\" value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
}

#[tokio::test]
async fn polish_people_explicit_language_survives_routes_forms_and_invalid_input() {
    let (fixture, boot, request) = signed_in().await;
    for lang in ["es", "cy", "ga", "pt", "en"] {
        for suffix in ["", "/73001", "/new", "/73001/edit"] {
            let result = request
                .get(&format!(
                    "/households/persistence-fixture/people{suffix}?lang={lang}"
                ))
                .add_header("accept-language", "en")
                .await;
            assert_eq!(result.status_code(), 200, "{}", result.text());
            assert!(
                result.text().contains(&format!("<html lang=\"{lang}\"")),
                "{suffix}: {}",
                result.text()
            );
        }
    }
    let index = request
        .get("/households/persistence-fixture/people?lang=es&per_page=1")
        .await;
    assert!(index.text().contains("/people/new?lang=es"));
    assert!(index.text().contains("per_page=1&amp;lang=es"));
    let form = request
        .get("/households/persistence-fixture/people/73001/edit?lang=es")
        .await;
    let html = form.text();
    assert!(html.contains("name=\"lang\" value=\"es\""));
    let result = request
        .post("/households/persistence-fixture/people/73001")
        .form(&[
            ("authenticity_token", csrf(&html)),
            ("lang", "es"),
            ("name", ""),
            ("date_of_birth", "2020-01-01"),
            ("person_type", "minor"),
        ])
        .await;
    assert_eq!(result.status_code(), 422, "{}", result.text());
    assert!(result.text().contains("<html lang=\"es\""));
    for suffix in ["?lang=xx", "?lang=es&lang=en"] {
        let result = request
            .get(&format!("/households/persistence-fixture/people{suffix}"))
            .await;
        assert_eq!(result.status_code(), 422, "{suffix}");
    }
    let fallback = request
        .get("/households/persistence-fixture/people")
        .add_header("accept-language", "es")
        .await;
    assert!(fallback.text().contains("<html lang=\"es\""));

    let new_form = request
        .get("/households/persistence-fixture/people/new?lang=es")
        .await;
    let new_html = new_form.text();
    for languages in [vec![("lang", "xx")], vec![("lang", "es"), ("lang", "en")]] {
        let mut fields = vec![("authenticity_token", csrf(&new_html))];
        fields.extend(languages);
        assert_eq!(
            request
                .post("/households/persistence-fixture/people")
                .form(&fields)
                .await
                .status_code(),
            422
        );
    }
    assert_eq!(
        request
            .post("/households/persistence-fixture/people")
            .form(&[("authenticity_token", "invalid"), ("lang", "es")])
            .await
            .status_code(),
        403
    );
    let saved = request
        .post("/households/persistence-fixture/people")
        .form(&[
            ("authenticity_token", csrf(&new_html)),
            ("lang", "es"),
            ("name", "Synthetic locale child"),
            ("date_of_birth", "2020-01-01"),
            ("person_type", "minor"),
            ("initial_guardian_declaration", "true"),
        ])
        .await;
    assert_eq!(saved.status_code(), 303, "{}", saved.text());
    let location = saved.header("location");
    let destination = location.to_str().unwrap();
    assert!(destination.ends_with("?lang=es"), "{destination}");
    assert!(
        request
            .get(destination)
            .await
            .text()
            .contains("<html lang=\"es\"")
    );
    let detail = request.get(destination).await;
    let parsed = scraper::Html::parse_document(&detail.text());
    let links = scraper::Selector::parse("a[href]").unwrap();
    let carers = parsed
        .select(&links)
        .filter_map(|link| link.value().attr("href"))
        .find(|href| href.ends_with("/carer_relationships?locale=es"))
        .unwrap();
    let carer_page = request.get(carers).await;
    assert_eq!(carer_page.status_code(), 200, "{}", carer_page.text());
    assert!(carer_page.text().contains("<html lang=\"es\""));
    assert!(!detail.text().contains("/carer_relationships/new"));
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn polish_expired_attention_reaches_old_invitations_beyond_new_pending_page() {
    let (fixture, boot, request) = signed_in().await;
    fixture.admin.execute_unprepared(r"
INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) SELECT 98700+n,72001,'pending-polish-'||n||'@example.test','member','pending-polish-'||n,74001,now()+interval '7 days',now(),now() FROM generate_series(1,51) n;
INSERT INTO household_invitations(id,household_id,email,membership_role,token_digest,invited_by_membership_id,expires_at,created_at,updated_at) VALUES(98699,72001,'old-expired-polish@example.test','member','expired-polish',74001,now()-interval '1 day',now()-interval '8 days',now())
").await.unwrap();
    let overview = request
        .get("/households/persistence-fixture/admin?lang=es")
        .await;
    let html = overview.text();
    let doc = scraper::Html::parse_document(&html);
    let selector = scraper::Selector::parse("#expired-invitations a").unwrap();
    let href = doc
        .select(&selector)
        .next()
        .unwrap()
        .value()
        .attr("href")
        .unwrap();
    assert!(href.contains("status=expired"), "{href}");
    assert!(href.contains("lang=es"), "{href}");
    let expired = request.get(href).await;
    assert_eq!(expired.status_code(), 200, "{}", expired.text());
    assert!(expired.text().contains("old-expired-polish@example.test"));
    assert!(!expired.text().contains("pending-polish-"));
    drop(request);
    close(fixture, boot).await;
}

#[tokio::test]
async fn polish_people_counts_keep_source_date_pause_and_visibility_semantics() {
    let (fixture, boot, request) = signed_in_with_zone("Pacific/Kiritimati").await;
    fixture.admin.execute_unprepared(r"
INSERT INTO people(id,name,person_type,has_capacity,created_at,updated_at) VALUES(97301,'Synthetic count person',0,true,now(),now());
INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(97401,72001,97301,'member','active',now(),now(),now());
INSERT INTO medications(id,household_id,location_id,name,dose_unit,created_at,updated_at) SELECT 97700+n,72001,79001,'Synthetic count medicine '||n,'tablet',now(),now() FROM generate_series(2,3)n;
INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,retired_at,created_at,updated_at) SELECT 97500+n,72001,97301,80001,n<>4,CASE WHEN n=2 THEN NULL WHEN n=3 THEN (now() at time zone 'Pacific/Kiritimati')::date+1 WHEN n=5 THEN (now() at time zone 'Pacific/Kiritimati')::date-2 ELSE (now() at time zone 'Pacific/Kiritimati')::date END,CASE WHEN n=2 THEN NULL WHEN n=3 THEN (now() at time zone 'Pacific/Kiritimati')::date+3 WHEN n=5 THEN (now() at time zone 'Pacific/Kiritimati')::date-1 ELSE (now() at time zone 'Pacific/Kiritimati')::date END,CASE WHEN n=6 THEN now() ELSE NULL END,now(),now() FROM generate_series(1,7)n;
INSERT INTO person_medications(id,household_id,person_id,medication_id,position,active,retired_at,created_at,updated_at) SELECT 97600+n,72001,97301,CASE WHEN n=1 THEN 80001 ELSE 97700+n END,0,n<>2,CASE WHEN n=3 THEN now() ELSE NULL END,now(),now() FROM generate_series(1,3)n
").await.unwrap();
    fixture.admin.execute_unprepared(r"
INSERT INTO households(id,created_by_account_id,name,slug,timezone,created_at,updated_at) VALUES(97801,71001,'Other synthetic count household','other-count-household','UTC',now(),now());
INSERT INTO household_memberships(id,household_id,person_id,role,status,joined_at,created_at,updated_at) VALUES(97802,97801,97301,'member','active',now(),now(),now());
INSERT INTO locations(id,household_id,name,created_at,updated_at) VALUES(97803,97801,'Other synthetic cupboard',now(),now());
INSERT INTO medications(id,household_id,location_id,name,dose_unit,created_at,updated_at) VALUES(97804,97801,97803,'Foreign count medicine','tablet',now(),now());
INSERT INTO schedules(id,household_id,person_id,medication_id,active,start_date,end_date,created_at,updated_at) VALUES(97805,97801,97301,97804,true,(now() at time zone 'Pacific/Kiritimati')::date,(now() at time zone 'Pacific/Kiritimati')::date,now(),now());
INSERT INTO person_medications(id,household_id,person_id,medication_id,position,active,created_at,updated_at) VALUES(97806,97801,97301,97804,0,true,now(),now());
").await.unwrap();
    let result = request.get("/households/persistence-fixture/people").await;
    assert_eq!(result.status_code(), 200, "{}", result.text());
    let document = scraper::Html::parse_document(&result.text());
    let selector =
        scraper::Selector::parse("[data-person-card='97301'] [data-active-medications]").unwrap();
    assert_eq!(
        document
            .select(&selector)
            .next()
            .unwrap()
            .text()
            .collect::<String>()
            .trim(),
        "3 active medications"
    );
    drop(request);
    close(fixture, boot).await;
}
