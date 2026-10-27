use super::*;
use base64::Engine as _;
use med_tracker::models::_entities::{
    household_invitations, household_memberships, people, person_medications, schedules,
};
use sea_orm::{ActiveModelTrait, Set};

async fn signed_in() -> (RequestFixture, BootResult, TestServer) {
    signed_in_with_zone("UTC").await
}

async fn signed_in_with_zone(zone: &str) -> (RequestFixture, BootResult, TestServer) {
    let (fixture, boot) = boot_request_fixture_with(|config| {
        config.settings.get_or_insert_with(|| json!({}))["time_zone"] = json!(zone);
        config.settings.get_or_insert_with(|| json!({}))["browser_session"] = json!({"key":base64::engine::general_purpose::STANDARD.encode([7_u8;64]),"secure":false});
    }).await;
    let mut request = server(&boot);
    request.save_cookies();
    assert_eq!(
        request
            .post("/api/auth/sign-in/email")
            .add_header("origin", boot.app_context.config.server.full_url())
            .json(&json!({"email":fixture.account.email,"password":"password"}))
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
                ("household_id", fixture.household.id.to_string().as_str()),
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
    let slug = fixture.household.slug.clone();
    let person_id = fixture.account.person_id;
    for lang in ["es", "cy", "ga", "pt", "en"] {
        for suffix in [
            String::new(),
            format!("/{person_id}"),
            "/new".into(),
            format!("/{person_id}/edit"),
        ] {
            let result = request
                .get(&format!("/households/{slug}/people{suffix}?lang={lang}"))
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
        .get(&format!("/households/{slug}/people?lang=es&per_page=1"))
        .await;
    assert!(index.text().contains("/people/new?lang=es"));
    assert!(index.text().contains("per_page=1&amp;lang=es"));
    let form = request
        .get(&format!(
            "/households/{slug}/people/{person_id}/edit?lang=es"
        ))
        .await;
    let html = form.text();
    assert!(html.contains("name=\"lang\" value=\"es\""));
    let result = request
        .post(&format!("/households/{slug}/people/{person_id}"))
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
            .get(&format!("/households/{slug}/people{suffix}"))
            .await;
        assert_eq!(result.status_code(), 422, "{suffix}");
    }
    let fallback = request
        .get(&format!("/households/{slug}/people"))
        .add_header("accept-language", "es")
        .await;
    assert!(fallback.text().contains("<html lang=\"es\""));

    let new_form = request
        .get(&format!("/households/{slug}/people/new?lang=es"))
        .await;
    let new_html = new_form.text();
    for languages in [vec![("lang", "xx")], vec![("lang", "es"), ("lang", "en")]] {
        let mut fields = vec![("authenticity_token", csrf(&new_html))];
        fields.extend(languages);
        assert_eq!(
            request
                .post(&format!("/households/{slug}/people"))
                .form(&fields)
                .await
                .status_code(),
            422
        );
    }
    assert_eq!(
        request
            .post(&format!("/households/{slug}/people"))
            .form(&[("authenticity_token", "invalid"), ("lang", "es")])
            .await
            .status_code(),
        403
    );
    let saved = request
        .post(&format!("/households/{slug}/people"))
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
    close_request(fixture, boot).await;
}

#[tokio::test]
async fn polish_expired_attention_reaches_old_invitations_beyond_new_pending_page() {
    let (fixture, boot, request) = signed_in().await;
    let slug = fixture.household.slug.clone();

    let now = chrono::Utc::now().naive_utc();
    let expired_email = format!("{}@example.test", fixture.credential("old-expired-polish"));
    for index in 0..52 {
        let expired = index == 0;
        household_invitations::ActiveModel {
            household_id: Set(fixture.household.id),
            email: Set(if expired {
                expired_email.clone()
            } else {
                format!("pending-polish-{}-{index}@example.test", fixture.account.id)
            }),
            membership_role: Set("member".into()),
            token_digest: Set(uuid::Uuid::new_v4().to_string()),
            invited_by_membership_id: Set(fixture.membership.id),
            expires_at: Set(now + chrono::Duration::days(if expired { -1 } else { 7 })),
            created_at: Set(now - chrono::Duration::days(if expired { 8 } else { 0 })),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&fixture.admin)
        .await
        .unwrap();
    }
    let overview = request
        .get(&format!("/households/{slug}/admin?lang=es"))
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
    assert!(expired.text().contains(&expired_email));
    assert!(!expired.text().contains("pending-polish-"));
    drop(request);
    close_request(fixture, boot).await;
}

#[tokio::test]
async fn polish_people_counts_keep_source_date_pause_and_visibility_semantics() {
    let (fixture, boot, request) = signed_in_with_zone("Pacific/Kiritimati").await;
    let slug = fixture.household.slug.clone();
    let now = chrono::Utc::now().naive_utc();
    let today = chrono::Utc::now()
        .with_timezone(&chrono_tz::Pacific::Kiritimati)
        .date_naive();
    let person = people::ActiveModel {
        name: Set("Synthetic count person".into()),
        person_type: Set(0),
        has_capacity: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    household_memberships::ActiveModel {
        household_id: Set(fixture.household.id),
        person_id: Set(person.id),
        role: Set("member".into()),
        status: Set("active".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    let second = fixture
        .medication(
            "Synthetic count medicine 2",
            sea_orm::prelude::Decimal::ZERO,
        )
        .await;
    let third = fixture
        .medication(
            "Synthetic count medicine 3",
            sea_orm::prelude::Decimal::ZERO,
        )
        .await;
    for index in 1..=7 {
        let start = match index {
            2 => None,
            3 => Some(today + chrono::Duration::days(1)),
            5 => Some(today - chrono::Duration::days(2)),
            _ => Some(today),
        };
        let end = match index {
            2 => None,
            3 => Some(today + chrono::Duration::days(3)),
            5 => Some(today - chrono::Duration::days(1)),
            _ => Some(today),
        };
        schedules::ActiveModel {
            household_id: Set(fixture.household.id),
            person_id: Set(person.id),
            medication_id: Set(fixture.medication.id),
            active: Set(index != 4),
            start_date: Set(start),
            end_date: Set(end),
            retired_at: Set((index == 6).then_some(now)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&fixture.admin)
        .await
        .unwrap();
    }
    for (index, medication_id) in [fixture.medication.id, second.id, third.id]
        .into_iter()
        .enumerate()
    {
        person_medications::ActiveModel {
            household_id: Set(fixture.household.id),
            person_id: Set(person.id),
            medication_id: Set(medication_id),
            position: Set(0),
            active: Set(index != 1),
            retired_at: Set((index == 2).then_some(now)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&fixture.admin)
        .await
        .unwrap();
    }
    household_memberships::ActiveModel {
        household_id: Set(fixture.foreign.household.id),
        person_id: Set(person.id),
        role: Set("member".into()),
        status: Set("active".into()),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    let foreign_medication = fixture
        .foreign
        .medication("Foreign count medicine", sea_orm::prelude::Decimal::ZERO)
        .await;
    schedules::ActiveModel {
        household_id: Set(fixture.foreign.household.id),
        person_id: Set(person.id),
        medication_id: Set(foreign_medication.id),
        active: Set(true),
        start_date: Set(Some(today)),
        end_date: Set(Some(today)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    person_medications::ActiveModel {
        household_id: Set(fixture.foreign.household.id),
        person_id: Set(person.id),
        medication_id: Set(foreign_medication.id),
        position: Set(0),
        active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await
    .unwrap();
    let result = request.get(&format!("/households/{slug}/people")).await;
    assert_eq!(result.status_code(), 200, "{}", result.text());
    let document = scraper::Html::parse_document(&result.text());
    let selector = scraper::Selector::parse(&format!(
        "[data-person-card='{}'] [data-active-medications]",
        person.id
    ))
    .unwrap();
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
    close_request(fixture, boot).await;
}
