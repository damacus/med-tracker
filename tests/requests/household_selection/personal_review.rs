use super::*;
use med_tracker::models::_entities::{api_change_events, medication_takes, person_medications};
use sea_orm::QueryOrder;

#[tokio::test]
async fn personal_review_clinical_edit_publishes_to_each_active_household() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let (second, membership) =
        home(&fixture.scenario, "Second", "sync-second", "member", false).await?;
    let (revoked, _) = home(&fixture.scenario, "Revoked", "sync-revoked", "member", true).await?;
    fixture.app_token("personal-first").await;
    let mut second_token = fixture
        .app_token("personal-second")
        .await
        .into_active_model();
    second_token.household_membership_id = Set(membership.id);
    second_token.permissions_version = Set(membership.permissions_version);
    second_token.update(&fixture.admin).await?;
    let mut request = server(&boot);
    request.save_cookies();
    browser_session(&fixture, &mut request, &boot).await;
    let html = request.get("/profile").await.text();
    let csrf = authenticity_token(&html).ok_or("missing CSRF")?;
    let saved = request
        .post("/profile")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("authenticity_token", csrf),
            ("setting", "clinical"),
            ("name", "Updated own clinical name"),
            ("date_of_birth", "1990-02-03"),
        ])
        .await;
    assert_eq!(saved.status_code(), 303, "{}", saved.text());
    for (household_id, token, membership_id) in [
        (
            fixture.household.id,
            "personal-first",
            fixture.membership.id,
        ),
        (second.id, "personal-second", membership.id),
    ] {
        let response = request
            .get(&format!(
                "/api/v1/households/{household_id}/sync/changes?cursor=2020-01-01T00:00:00Z"
            ))
            .add_header("authorization", format!("Bearer {token}"))
            .await;
        assert_eq!(response.status_code(), 200, "{}", response.text());
        let body: Value = response.json();
        assert!(
            body["data"]["changes"]
                .as_array()
                .ok_or("missing changes")?
                .iter()
                .any(|event| event["record_type"] == "Person"
                    && event["record_id"] == fixture.account.person_id
                    && event["action"] == "update")
        );
        let events = api_change_events::Entity::find()
            .filter(api_change_events::Column::HouseholdId.eq(household_id))
            .filter(api_change_events::Column::RecordType.eq("Person"))
            .filter(api_change_events::Column::Action.eq("update"))
            .all(&fixture.admin)
            .await?;
        assert_eq!(events.len(), 1);
        let event = events.first().ok_or("missing event")?;
        assert_eq!(event.account_id, Some(fixture.account.id));
        assert_eq!(event.household_membership_id, Some(membership_id));
        assert!(event.record_portable_id.is_some());
    }
    assert_eq!(
        api_change_events::Entity::find()
            .filter(api_change_events::Column::HouseholdId.eq(revoked.id))
            .count(&fixture.admin)
            .await?,
        0
    );
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn personal_review_history_orders_taken_time_and_paginates_ties() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let assignment = person_medications::Entity::find()
        .filter(person_medications::Column::PersonId.eq(fixture.account.person_id))
        .one(&fixture.admin)
        .await?
        .ok_or("assignment missing")?;
    let now = chrono::Utc::now().naive_utc();
    for index in 0..30 {
        medication_takes::ActiveModel {
            household_id: Set(fixture.household.id),
            person_medication_id: Set(Some(assignment.id)),
            taken_from_medication_id: Set(Some(fixture.medication.id)),
            dose_amount: Set(Some(sea_orm::prelude::Decimal::new(index + 1, 0))),
            dose_unit: Set(Some("tablet".into())),
            taken_at: Set(Some(
                now - chrono::Duration::days(if index == 29 { 3 } else { index / 10 }),
            )),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&fixture.admin)
        .await?;
    }
    let expected = medication_takes::Entity::find()
        .filter(medication_takes::Column::PersonMedicationId.eq(assignment.id))
        .order_by_desc(medication_takes::Column::TakenAt)
        .order_by_desc(medication_takes::Column::Id)
        .all(&fixture.admin)
        .await?;
    let mut request = server(&boot);
    request.save_cookies();
    browser_session(&fixture, &mut request, &boot).await;
    let mut path = "/profile/history".to_owned();
    let mut amounts = Vec::new();
    for _ in 0..2 {
        let page = request.get(&path).await;
        assert_eq!(page.status_code(), 200, "{}", page.text());
        let document = scraper::Html::parse_document(&page.text());
        let selector = scraper::Selector::parse("li.card > p:first-of-type")
            .map_err(|_| "invalid selector")?;
        amounts.extend(
            document
                .select(&selector)
                .map(|node| node.text().collect::<String>().trim().to_owned()),
        );
        let next = scraper::Selector::parse("a[href^='/profile/history?']")
            .map_err(|_| "invalid selector")?;
        path = document
            .select(&next)
            .next()
            .and_then(|node| node.value().attr("href"))
            .unwrap_or("")
            .to_owned();
    }
    assert!(path.is_empty());
    assert_eq!(
        amounts,
        expected
            .iter()
            .map(|row| format!("{} tablet", row.dose_amount.unwrap()))
            .collect::<Vec<_>>()
    );
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}

#[tokio::test]
async fn personal_review_shortcuts_are_filtered_in_current_household() -> TestResult {
    let (fixture, boot) = boot_browser_fixture().await;
    let mut request = server(&boot);
    request.save_cookies();
    browser_session(&fixture, &mut request, &boot).await;
    let html = request.get("/profile").await.text();
    let csrf = authenticity_token(&html).ok_or("missing CSRF")?;
    let saved = request
        .post("/profile")
        .add_header("sec-fetch-site", "same-origin")
        .form(&[
            ("authenticity_token", csrf),
            ("setting", "shortcuts"),
            ("shortcut_1", "administration"),
            ("shortcut_2", "people"),
            ("shortcut_3", "profile"),
        ])
        .await;
    assert_eq!(saved.status_code(), 303, "{}", saved.text());
    let endpoint = format!("/households/{}/profile/navigation", fixture.household.slug);
    let owner = request.get(&endpoint).await;
    assert_eq!(owner.status_code(), 200, "{}", owner.text());
    let owner: Value = owner.json();
    assert!(
        owner["links"]
            .as_array()
            .ok_or("links missing")?
            .iter()
            .any(|link| link["id"] == "administration"
                && link["href"] == format!("/households/{}/admin", fixture.household.slug))
    );
    let mut membership = fixture.membership.clone().into_active_model();
    membership.role = Set("member".into());
    membership.update(&fixture.admin).await?;
    let member = request.get(&endpoint).await;
    assert_eq!(member.status_code(), 200, "{}", member.text());
    let member: Value = member.json();
    assert!(
        !member["links"]
            .as_array()
            .ok_or("links missing")?
            .iter()
            .any(|link| link["id"] == "administration")
    );
    assert!(
        member["links"]
            .as_array()
            .ok_or("links missing")?
            .iter()
            .any(|link| link["id"] == "people")
    );
    drop(request);
    close_request(fixture, boot).await;
    Ok(())
}
