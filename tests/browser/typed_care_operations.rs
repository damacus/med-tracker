use crate::harness::{self, named};
use med_tracker::models::_entities::person_medications;
use playwright_rs::{
    expect,
    protocol::{Animations, AriaRole, Page, ScreenshotOptions, Viewport},
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use std::path::Path;

async fn capture(page: &Page, name: &str) {
    expect(page.locator("body[data-navigation-ready]"))
        .to_be_visible()
        .await
        .unwrap();
    let directory = Path::new("docs/screenshots/typed-care-operations");
    std::fs::create_dir_all(directory).unwrap();
    page.screenshot_to_file(
        &directory.join(format!("{name}.png")),
        ScreenshotOptions::builder()
            .animations(Animations::Disabled)
            .full_page(true)
            .build(),
    )
    .await
    .unwrap();
}

async fn capture_both(page: &Page, name: &str) {
    for (size, width, height) in [("desktop", 1440, 1000), ("mobile", 390, 844)] {
        page.set_viewport_size(Viewport { width, height })
            .await
            .unwrap();
        capture(page, &format!("{name}-{size}")).await;
    }
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn typed_pause_browser_reference() {
    harness::journey(
        Viewport {
            width: 1440,
            height: 1000,
        },
        |journey| {
            Box::pin(async move {
                harness::sign_in(journey).await;
                let page = &journey.page;
                page.goto(
                    &format!(
                        "{}/households/persistence-fixture/people/73001/treatments",
                        journey.origin
                    ),
                    None,
                )
                .await
                .unwrap();
                let assignment =
                    page.locator("section[aria-labelledby='assignments-heading'] article");
                expect(assignment.clone())
                    .to_contain_text("Synthetic tablets")
                    .await
                    .unwrap();
                assignment
                    .get_by_text("Pause options", true)
                    .click(None)
                    .await
                    .unwrap();
                assignment
                    .get_by_label("Pause reason", true)
                    .select_option("clinician_advice", None)
                    .await
                    .unwrap();
                assignment
                    .get_by_label("Pause note", true)
                    .fill("Synthetic temporary pause", None)
                    .await
                    .unwrap();
                assignment
                    .get_by_role(AriaRole::Button, named("Pause treatment"))
                    .click(None)
                    .await
                    .unwrap();
                expect(assignment.clone())
                    .to_contain_text("Synthetic temporary pause")
                    .await
                    .unwrap();
                capture_both(page, "paused").await;
                assignment
                    .get_by_role(AriaRole::Button, named("Resume treatment"))
                    .click(None)
                    .await
                    .unwrap();
                expect(assignment.clone())
                    .to_contain_text("Active")
                    .await
                    .unwrap();
                expect(assignment.get_by_role(AriaRole::Button, named("Resume treatment")))
                    .to_have_count(0)
                    .await
                    .unwrap();
                capture_both(page, "resumed").await;
            })
        },
    )
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn typed_occurrence_browser_reference() {
    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| Box::pin(async move {
        person_medications::ActiveModel {
            id: Set(81001),
            administration_kind: Set(0),
            ..Default::default()
        }
        .update(&journey.admin)
        .await
        .unwrap();
        harness::sign_in(journey).await;
        let page = &journey.page;
        page.goto(&format!("{}/households/persistence-fixture/people/73001/treatments/assignments/81001/doses", journey.origin), None).await.unwrap();
        let row = page.locator("article[aria-label^='Dose ']").first();
        expect(row.clone()).to_be_visible().await.unwrap();
        row.get_by_role(AriaRole::Button, named("Record not taken")).click(None).await.unwrap();
        row.get_by_label("Reason", true).select_option("refused", None).await.unwrap();
        row.get_by_label("Note (optional)", true).fill("Synthetic missed dose", None).await.unwrap();
        row.get_by_role(AriaRole::Button, named("Confirm not taken")).click(None).await.unwrap();
        expect(row.clone()).to_contain_text("Synthetic missed dose").await.unwrap();
        capture_both(page, "not-taken").await;
        row.get_by_role(AriaRole::Button, named("Reopen dose")).click(None).await.unwrap();
        row.get_by_role(AriaRole::Button, named("Confirm reopening")).click(None).await.unwrap();
        expect(row.locator("[data-dose-outcome]")).to_contain_text("Open").await.unwrap();
        capture_both(page, "reopened").await;
    })).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn typed_health_event_browser_reference() {
    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| Box::pin(async move {
        harness::sign_in(journey).await;
        let page = &journey.page;
        page.goto(&format!("{}/households/persistence-fixture/people/73001/health_events/new?event_kind=suspected_side_effect&locale=en", journey.origin), None).await.unwrap();
        page.get_by_label("Title", true).fill("Synthetic reaction", None).await.unwrap();
        page.get_by_label("Start date", true).fill("2026-10-01", None).await.unwrap();
        page.locator("input[name='medication_80001']").check(None).await.unwrap();
        page.get_by_role(AriaRole::Button, named("Save")).click(None).await.unwrap();
        expect(page.locator("main")).to_contain_text("Synthetic reaction").await.unwrap();
        capture_both(page, "health-created").await;
        page.locator("main article").get_by_role(AriaRole::Link, named("Edit")).click(None).await.unwrap();
        page.get_by_label("Title", true).fill("Synthetic edited reaction", None).await.unwrap();
        page.get_by_role(AriaRole::Button, named("Save")).click(None).await.unwrap();
        expect(page.locator("main")).to_contain_text("Synthetic edited reaction").await.unwrap();
        capture_both(page, "health-edited").await;
    })).await;
}
