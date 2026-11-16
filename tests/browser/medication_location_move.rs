use crate::{
    accessibility_level_a::{assert_no_violations, audit},
    harness::{self, named},
};
use med_tracker::models::entities::{location, medication, medication_take};
use playwright_rs::{
    expect,
    protocol::{Animations, AriaRole, Page, ScreenshotOptions, Viewport},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, prelude::Decimal};

async fn assert_no_horizontal_overflow(page: &Page) {
    assert!(
        !page
            .evaluate::<(), bool>(
                "() => document.documentElement.scrollWidth > innerWidth",
                None,
            )
            .await
            .unwrap()
    );
}

async fn save_screenshot(page: &Page, name: &str) {
    let directory = std::path::Path::new("docs/screenshots/medication-location-move");
    std::fs::create_dir_all(directory).unwrap();
    page.screenshot_to_file(
        &directory.join(name),
        ScreenshotOptions::builder()
            .animations(Animations::Disabled)
            .full_page(true)
            .build(),
    )
    .await
    .unwrap();
}

async fn medication_location_move(viewport: Viewport, size: &'static str) {
    harness::journey(viewport, |journey| {
        Box::pin(async move {
            let now = chrono::Utc::now().naive_utc();
            let mut house: location::ActiveModel = location::Entity::find_by_id(79001)
                .one(&journey.admin)
                .await
                .unwrap()
                .unwrap()
                .into();
            house.name = Set("House".into());
            house.update(&journey.admin).await.unwrap();

            let utility = location::ActiveModel {
                household_id: Set(72001),
                name: Set("Utility".into()),
                portable_id: Set(format!("utility-{size}")),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&journey.admin)
            .await
            .unwrap();

            medication_take::ActiveModel {
                household_id: Set(72001),
                person_medication_id: Set(Some(81001)),
                dose_amount: Set(Some(Decimal::new(2, 0))),
                dose_unit: Set(Some("tablet".into())),
                taken_at: Set(Some(now - chrono::Duration::hours(3))),
                taken_from_medication_id: Set(Some(80001)),
                taken_from_location_id: Set(Some(79001)),
                portable_id: Set(format!("historical-move-{size}")),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&journey.admin)
            .await
            .unwrap();

            harness::sign_in(journey).await;
            let page = &journey.page;
            let mut reports = Vec::new();

            page.goto(
                &format!(
                    "{}/households/persistence-fixture/medications?location_id=79001",
                    journey.origin
                ),
                None,
            )
            .await
            .unwrap();
            expect(page.locator("body[data-navigation-ready]"))
                .to_be_visible()
                .await
                .unwrap();
            expect(page.locator("#medications"))
                .to_contain_text("Synthetic tablets")
                .await
                .unwrap();

            page.get_by_role(AriaRole::Link, named("Move Synthetic tablets"))
                .click(None)
                .await
                .unwrap();
            expect(page.get_by_role(AriaRole::Heading, named("Move Medication")))
                .to_be_visible()
                .await
                .unwrap();
            expect(page.locator("main"))
                .to_contain_text("Synthetic tablets")
                .await
                .unwrap();
            expect(page.locator("main"))
                .to_contain_text("House")
                .await
                .unwrap();
            expect(page.locator("main"))
                .to_contain_text("10 tablets")
                .await
                .unwrap();
            assert_eq!(
                page.locator("input[name='current_supply'], input[name='dose_amount']")
                    .count()
                    .await
                    .unwrap(),
                0
            );
            page.get_by_label("Destination location", true)
                .focus()
                .await
                .unwrap();
            expect(page.locator("#location_id"))
                .to_be_focused()
                .await
                .unwrap();
            reports.push(audit(page, &format!("medication-move-{size}")).await);
            assert_no_horizontal_overflow(page).await;
            save_screenshot(page, &format!("move-{size}.png")).await;

            page.get_by_role(AriaRole::Link, named("Add Location"))
                .click(None)
                .await
                .unwrap();
            expect(page.get_by_role(AriaRole::Heading, named("New Location")))
                .to_be_visible()
                .await
                .unwrap();
            page.get_by_label("Name", true)
                .fill("Kitchen", None)
                .await
                .unwrap();
            page.get_by_label("Description", true)
                .fill("Kitchen counter", None)
                .await
                .unwrap();
            page.get_by_role(AriaRole::Button, named("Save Location"))
                .click(None)
                .await
                .unwrap();

            let kitchen = location::Entity::find()
                .filter(location::Column::HouseholdId.eq(72001))
                .filter(location::Column::Name.eq("Kitchen"))
                .one(&journey.admin)
                .await
                .unwrap()
                .unwrap();
            expect(page.get_by_label("Destination location", true))
                .to_have_value(&kitchen.id.to_string())
                .await
                .unwrap();

            page.get_by_label("Destination location", true)
                .select_option(utility.id.to_string(), None)
                .await
                .unwrap();
            page.get_by_role(AriaRole::Button, named("Move Medication"))
                .click(None)
                .await
                .unwrap();

            expect(page.locator("main .alert[role='status']"))
                .to_contain_text("Moved Synthetic tablets from House to Utility.")
                .await
                .unwrap();
            expect(page.locator("main"))
                .to_contain_text("Utility")
                .await
                .unwrap();
            expect(page.locator("[data-testid='current-supply']"))
                .to_have_text("10 tablets")
                .await
                .unwrap();
            expect(page.locator("[aria-labelledby='dose-history-heading']"))
                .to_contain_text("House")
                .await
                .unwrap();
            reports.push(audit(page, &format!("medication-moved-{size}")).await);
            assert_no_horizontal_overflow(page).await;
            save_screenshot(page, &format!("moved-{size}.png")).await;

            page.get_by_role(AriaRole::Link, named("Back to House inventory"))
                .click(None)
                .await
                .unwrap();
            expect(page.locator("#medications"))
                .not()
                .to_contain_text("Synthetic tablets")
                .await
                .unwrap();
            expect(page.locator("main .alert[role='status']"))
                .to_contain_text(
                    "Moved Synthetic tablets from House to Utility. It is no longer listed under House.",
                )
                .await
                .unwrap();
            reports.push(audit(page, &format!("medication-house-filtered-{size}")).await);
            assert_no_horizontal_overflow(page).await;
            save_screenshot(page, &format!("house-filtered-{size}.png")).await;

            page.locator("#inventory-location")
                .select_option(utility.id.to_string(), None)
                .await
                .unwrap();
            expect(page.locator("#medications"))
                .to_contain_text("Synthetic tablets")
                .await
                .unwrap();

            page.get_by_role(AriaRole::Link, named("Move Synthetic tablets"))
                .click(None)
                .await
                .unwrap();
            expect(page.get_by_role(AriaRole::Heading, named("Move Medication")))
                .to_be_visible()
                .await
                .unwrap();

            let mut concurrent: medication::ActiveModel = medication::Entity::find_by_id(80001)
                .one(&journey.admin)
                .await
                .unwrap()
                .unwrap()
                .into();
            concurrent.current_supply = Set(Some(Decimal::new(9, 0)));
            concurrent.updated_at = Set(now + chrono::Duration::seconds(5));
            concurrent.update(&journey.admin).await.unwrap();

            page.get_by_label("Destination location", true)
                .select_option(kitchen.id.to_string(), None)
                .await
                .unwrap();
            page.get_by_role(AriaRole::Button, named("Move Medication"))
                .click(None)
                .await
                .unwrap();

            expect(page.locator("main section[role='alert']"))
                .to_contain_text(
                    "Medication changed while this form was open. Review the latest details before saving.",
                )
                .await
                .unwrap();
            expect(page.get_by_label("Destination location", true))
                .to_have_value(&kitchen.id.to_string())
                .await
                .unwrap();
            let error_link = page.locator("#form-error-medication-move-form a");
            error_link.focus().await.unwrap();
            page.keyboard().press("Enter", None).await.unwrap();
            expect(page.locator("#medication-move-form"))
                .to_be_focused()
                .await
                .unwrap();
            reports.push(audit(page, &format!("medication-move-conflict-{size}")).await);
            assert_no_horizontal_overflow(page).await;
            save_screenshot(page, &format!("conflict-{size}.png")).await;

            page.get_by_role(AriaRole::Link, named("Review latest details"))
                .click(None)
                .await
                .unwrap();
            expect(page.get_by_label("Destination location", true))
                .to_have_value(&kitchen.id.to_string())
                .await
                .unwrap();
            expect(page.locator("main"))
                .to_contain_text("9 tablets")
                .await
                .unwrap();

            page.get_by_role(AriaRole::Button, named("Move Medication"))
                .click(None)
                .await
                .unwrap();
            expect(page.locator("main .alert[role='status']"))
                .to_contain_text("Moved Synthetic tablets from Utility to Kitchen.")
                .await
                .unwrap();
            expect(page.locator("main"))
                .to_contain_text("Kitchen")
                .await
                .unwrap();
            expect(page.locator("[data-testid='current-supply']"))
                .to_have_text("9 tablets")
                .await
                .unwrap();
            expect(page.locator("[aria-labelledby='dose-history-heading']"))
                .to_contain_text("House")
                .await
                .unwrap();

            assert_no_violations(&reports);
        })
    })
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn medication_location_move_desktop() {
    medication_location_move(
        Viewport {
            width: 1440,
            height: 1000,
        },
        "desktop",
    )
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn medication_location_move_mobile() {
    medication_location_move(
        Viewport {
            width: 390,
            height: 844,
        },
        "mobile",
    )
    .await;
}
