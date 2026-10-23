use crate::harness::{self, named};
use chrono::{NaiveDate, Utc};
use med_tracker::models::_entities::{
    medication_review_evidence_records as evidence, medication_review_prompts as prompts,
    medications, person_medications,
};
use playwright_rs::{
    expect,
    protocol::{Animations, AriaRole, Page, ScreenshotOptions, Viewport},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
};
use std::path::Path;

async fn seed_review(journey: &harness::Journey) -> prompts::Model {
    let now = Utc::now().naive_utc();
    let day = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
    let aspirin = medications::Entity::find_by_id(80001)
        .one(&journey.admin)
        .await
        .unwrap()
        .unwrap();
    let mut aspirin = aspirin.into_active_model();
    aspirin.name = Set(Some("Aspirin tablets".to_owned()));
    aspirin.update(&journey.admin).await.unwrap();
    medications::ActiveModel {
        id: Set(80003),
        household_id: Set(72001),
        location_id: Set(79001),
        name: Set(Some("Warfarin tablets".to_owned())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&journey.admin)
    .await
    .unwrap();
    person_medications::ActiveModel {
        id: Set(81003),
        household_id: Set(72001),
        person_id: Set(73001),
        medication_id: Set(80003),
        position: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&journey.admin)
    .await
    .unwrap();
    evidence::ActiveModel {
        id: Set(99091),
        source_name: Set("Synthetic reference".to_owned()),
        source_record_id: Set("review-edit-reference".to_owned()),
        source_url: Set("https://example.test/review-edit".to_owned()),
        source_version: Set(Some("1".to_owned())),
        product_name: Set("Aspirin tablets".to_owned()),
        label_section: Set("Warnings".to_owned()),
        evidence_text: Set("Synthetic aspirin and warfarin review evidence.".to_owned()),
        retrieved_on: Set(day),
        source_effective_on: Set(Some(day)),
        risk_level: Set("high".to_owned()),
        match_confidence: Set("high".to_owned()),
        match_status: Set("reviewed_pair".to_owned()),
        candidate_terms: Set(vec!["aspirin".to_owned()]),
        interacting_terms: Set(vec!["warfarin".to_owned()]),
        pharmacologic_classes: Set(Vec::new()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&journey.admin)
    .await
    .unwrap();
    harness::sign_in(journey).await;
    journey
        .page
        .goto(
            &format!(
                "{}/households/persistence-fixture/medicine-reviews",
                journey.origin
            ),
            None,
        )
        .await
        .unwrap();
    expect(journey.page.locator("body[data-navigation-ready]"))
        .to_be_visible()
        .await
        .unwrap();
    prompts::Entity::find()
        .filter(prompts::Column::HouseholdId.eq(72001))
        .filter(prompts::Column::EvidenceRecordId.eq(99091))
        .one(&journey.admin)
        .await
        .unwrap()
        .expect("reconciled review prompt")
}

async fn capture(page: &Page, name: &str) {
    expect(page.locator("body[data-navigation-ready]"))
        .to_be_visible()
        .await
        .unwrap();
    let directory = Path::new("docs/screenshots/typed-review-prompt-editing");
    std::fs::create_dir_all(directory).unwrap();
    for (size, width, height) in [("desktop", 1440, 1000), ("mobile", 390, 844)] {
        page.set_viewport_size(Viewport { width, height })
            .await
            .unwrap();
        let _: serde_json::Value = page
            .evaluate("() => { window.scrollTo(0, 0); return true; }", None::<&()>)
            .await
            .unwrap();
        page.screenshot_to_file(
            &directory.join(format!("{name}-{size}.png")),
            ScreenshotOptions::builder()
                .animations(Animations::Disabled)
                .full_page(true)
                .build(),
        )
        .await
        .unwrap();
    }
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn review_prompt_edit_reference() {
    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| Box::pin(async move {
        let prompt = seed_review(journey).await;
        assert_eq!(prompt.status, "needs_review");
        let page = &journey.page;
        capture(page, "index").await;
        page.locator("main details summary").first().click(None).await.unwrap();
        page.get_by_role(AriaRole::Button, named("Edit review")).click(None).await.unwrap();
        expect(page.locator("[data-review-editor]")).to_be_visible().await.unwrap();
        capture(page, "editing").await;
        page.locator("select[name='status']").select_option("reviewed_with_practitioner", None).await.unwrap();
        page.get_by_role(AriaRole::Button, named("Save review record")).click(None).await.unwrap();
        expect(page.locator("[data-review-editor] .alert-error")).to_be_visible().await.unwrap();
        capture(page, "invalid").await;
        page.locator("select[name='practitioner_role']").select_option("GP", None).await.unwrap();
        page.locator("input[name='practitioner_name']").fill("Dr Synthetic", None).await.unwrap();
        page.locator("input[name='reviewed_on']").fill("2026-10-01", None).await.unwrap();
        page.locator("textarea[name='review_note']").fill("Synthetic decision", None).await.unwrap();
        page.get_by_role(AriaRole::Button, named("Save review record")).click(None).await.unwrap();
        expect(page.locator("main")).to_contain_text("Medicine review record updated.").await.unwrap();
        page.goto(&format!("{}/households/persistence-fixture/medicine-reviews?review_status=all&updated=1", journey.origin), None).await.unwrap();
        page.locator("main details summary").first().click(None).await.unwrap();
        expect(page.locator("main")).to_contain_text("Dr Synthetic").await.unwrap();
        capture(page, "saved").await;
        let saved = prompts::Entity::find_by_id(prompt.id).one(&journey.admin).await.unwrap().unwrap();
        assert_eq!(saved.status, "reviewed_with_practitioner");
        assert_eq!(saved.review_note.as_deref(), Some("Synthetic decision"));
    })).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn review_prompt_clear_only_advances_etag() {
    harness::journey(
        Viewport {
            width: 1440,
            height: 1000,
        },
        |journey| {
            Box::pin(async move {
                let prompt = seed_review(journey).await;
                let mut active = prompt.into_active_model();
                active.status = Set("not_relevant".to_owned());
                active.reviewed_on = Set(Some(NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()));
                let before = active.update(&journey.admin).await.unwrap();
                journey
                    .page
                    .goto(
                        &format!(
                            "{}/households/persistence-fixture/medicine-reviews?review_status=all",
                            journey.origin
                        ),
                        None,
                    )
                    .await
                    .unwrap();
                let page = &journey.page;
                page.locator("main details summary")
                    .first()
                    .click(None)
                    .await
                    .unwrap();
                page.get_by_role(AriaRole::Button, named("Edit review"))
                    .click(None)
                    .await
                    .unwrap();
                page.locator("input[name='reviewed_on']")
                    .fill("", None)
                    .await
                    .unwrap();
                page.get_by_role(AriaRole::Button, named("Save review record"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.locator("main"))
                    .to_contain_text("Medicine review record updated.")
                    .await
                    .unwrap();
                expect(page.locator("body[data-navigation-ready]"))
                    .to_be_visible()
                    .await
                    .unwrap();
                let after = prompts::Entity::find_by_id(before.id)
                    .one(&journey.admin)
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(after.reviewed_on, None);
                assert_ne!(
                    after.updated_at, before.updated_at,
                    "date-clear-only save must advance the review ETag"
                );
            })
        },
    )
    .await;
}
