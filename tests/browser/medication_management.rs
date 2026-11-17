use crate::{
    accessibility_level_a::{assert_no_violations, audit},
    harness::{self, named},
};
use med_tracker::models::entities::membership as household_membership;
use playwright_rs::{
    expect,
    protocol::{Animations, AriaRole, Page, ScreenshotOptions, Viewport},
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

async fn capture(page: &Page, name: &str, size: &str) {
    let directory = std::path::Path::new("docs/screenshots/medication-management");
    std::fs::create_dir_all(directory).unwrap();
    let full_page = page
        .evaluate::<(), bool>("() => !document.querySelector('dialog[open]')", None)
        .await
        .unwrap();
    if full_page {
        page.evaluate::<(), bool>("() => { window.scrollTo(0, 0); return true; }", None)
            .await
            .unwrap();
    }
    page.screenshot_to_file(
        &directory.join(format!("{name}-{size}.png")),
        ScreenshotOptions::builder()
            .animations(Animations::Disabled)
            .full_page(full_page)
            .build(),
    )
    .await
    .unwrap();
    assert!(
        !page
            .evaluate::<(), bool>(
                "() => document.documentElement.scrollWidth > innerWidth",
                None
            )
            .await
            .unwrap()
    );
    assert_no_violations(&[audit(page, &format!("{name}-{size}")).await]);
}

async fn management(viewport: Viewport, size: &'static str) {
    harness::journey(viewport, |journey| {
        Box::pin(async move {
            harness::sign_in(journey).await;
            let page = &journey.page;
            let path = format!(
                "{}/households/persistence-fixture/medications/80001",
                journey.origin
            );
            page.goto(&path, None).await.unwrap();
            capture(page, "medicine", size).await;
            page.get_by_role(AriaRole::Button, named("Update stock"))
                .click(None)
                .await
                .unwrap();
            let stock = page.get_by_role(AriaRole::Dialog, named("Update stock"));
            expect(stock.clone()).to_be_visible().await.unwrap();
            capture(page, "stock-received", size).await;
            page.keyboard().press("Escape", None).await.unwrap();
            expect(page.get_by_role(AriaRole::Button, named("Update stock")))
                .to_be_focused()
                .await
                .unwrap();
            page.get_by_role(AriaRole::Button, named("Update stock"))
                .click(None)
                .await
                .unwrap();
            stock
                .get_by_label("Quantity to add", true)
                .fill("3", None)
                .await
                .unwrap();
            stock
                .get_by_role(AriaRole::Button, named("Add stock"))
                .click(None)
                .await
                .unwrap();
            expect(page.locator("[data-testid='current-supply']"))
                .to_contain_text("13")
                .await
                .unwrap();
            page.get_by_role(AriaRole::Button, named("Update stock"))
                .click(None)
                .await
                .unwrap();
            stock
                .get_by_label("What changed?", true)
                .select_option("counted", None)
                .await
                .unwrap();
            capture(page, "stock-counted", size).await;
            stock
                .get_by_label("New stock quantity", true)
                .fill("7", None)
                .await
                .unwrap();
            stock
                .get_by_label("Reason", true)
                .fill("Synthetic count", None)
                .await
                .unwrap();
            stock
                .get_by_role(AriaRole::Button, named("Adjust stock"))
                .click(None)
                .await
                .unwrap();
            expect(page.locator("[data-testid='current-supply']"))
                .to_contain_text("7")
                .await
                .unwrap();
            page.get_by_role(AriaRole::Link, named("Edit Medication"))
                .click(None)
                .await
                .unwrap();
            expect(page.locator("[data-medication-inline] #medication-form"))
                .to_be_visible()
                .await
                .unwrap();
            assert_eq!(page.url(), path);
            capture(page, "edit", size).await;
            page.locator("[data-medication-inline] #friendly_name")
                .fill("Updated tablets", None)
                .await
                .unwrap();
            page.get_by_role(AriaRole::Button, named("Save Medication"))
                .click(None)
                .await
                .unwrap();
            expect(page.locator("main h1"))
                .to_contain_text("Updated tablets")
                .await
                .unwrap();
            page.get_by_role(AriaRole::Link, named("Edit Medication"))
                .click(None)
                .await
                .unwrap();
            page.get_by_role(AriaRole::Button, named("Delete Medication"))
                .click(None)
                .await
                .unwrap();
            capture(page, "delete-confirmation", size).await;
            page.get_by_role(AriaRole::Dialog, named("Delete Medication"))
                .get_by_role(AriaRole::Button, named("Cancel"))
                .click(None)
                .await
                .unwrap();
            page.locator("[data-medication-inline] [data-medication-cancel]")
                .click(None)
                .await
                .unwrap();
            page.get_by_role(AriaRole::Link, named("Move Medication"))
                .click(None)
                .await
                .unwrap();
            expect(page.locator("[data-medication-move-dialog] #medication-move-form"))
                .to_be_visible()
                .await
                .unwrap();
            assert_eq!(page.url(), path);
            capture(page, "move", size).await;
            page.locator("[data-medication-move-dialog] [data-medication-close]")
                .click(None)
                .await
                .unwrap();
            page.get_by_role(AriaRole::Link, named("Manage Dose Options"))
                .click(None)
                .await
                .unwrap();
            expect(page.locator("[data-medication-options] h2"))
                .to_contain_text("Dose Options")
                .await
                .unwrap();
            capture(page, "dose-options", size).await;
            page.locator("[data-medication-options]")
                .get_by_role(AriaRole::Link, named("Add dose option"))
                .click(None)
                .await
                .unwrap();
            expect(page.locator("[data-medication-options] form"))
                .to_be_visible()
                .await
                .unwrap();
            capture(page, "dose-option-edit", size).await;
            assert_eq!(page.url(), path);
            let panel = page.locator("[data-medication-options]");
            panel
                .get_by_label("Amount", true)
                .fill("1", None)
                .await
                .unwrap();
            panel
                .get_by_label("Unit", true)
                .select_option("tablet", None)
                .await
                .unwrap();
            panel
                .get_by_label("Description", true)
                .fill("Synthetic option", None)
                .await
                .unwrap();
            panel
                .get_by_label("Current Supply", true)
                .fill("12", None)
                .await
                .unwrap();
            panel
                .get_by_role(AriaRole::Button, named("Save Dose Option"))
                .click(None)
                .await
                .unwrap();
            expect(panel.clone())
                .to_contain_text("Synthetic option")
                .await
                .unwrap();
            page.get_by_role(AriaRole::Button, named("Update stock"))
                .click(None)
                .await
                .unwrap();
            stock
                .get_by_label("What changed?", true)
                .select_option("counted", None)
                .await
                .unwrap();
            capture(page, "stock-option-count", size).await;
            stock
                .locator("[data-medication-panel='options']")
                .click(None)
                .await
                .unwrap();
            expect(panel.get_by_label("Description", true))
                .to_have_value("Synthetic option")
                .await
                .unwrap();
            panel
                .get_by_label("Current Supply", true)
                .fill("11", None)
                .await
                .unwrap();
            panel
                .get_by_label("Description", true)
                .fill("Updated option", None)
                .await
                .unwrap();
            panel
                .get_by_role(AriaRole::Button, named("Save Dose Option"))
                .click(None)
                .await
                .unwrap();
            expect(panel.clone())
                .to_contain_text("Updated option")
                .await
                .unwrap();
            expect(page.locator("[data-testid='current-supply']"))
                .to_contain_text("11")
                .await
                .unwrap();
            panel
                .get_by_role(AriaRole::Button, named("Delete dose option"))
                .click(None)
                .await
                .unwrap();
            panel
                .get_by_role(AriaRole::Dialog, named("Delete Dose Option"))
                .get_by_role(AriaRole::Button, named("Delete"))
                .click(None)
                .await
                .unwrap();
            expect(panel.clone())
                .to_contain_text("No dose options yet.")
                .await
                .unwrap();
            panel
                .locator("[data-medication-cancel]")
                .click(None)
                .await
                .unwrap();
            page.get_by_role(AriaRole::Link, named("Move Medication"))
                .click(None)
                .await
                .unwrap();
            let moving = page.locator("[data-medication-move-dialog]");
            moving
                .get_by_role(AriaRole::Link, named("Add Location"))
                .click(None)
                .await
                .unwrap();
            expect(moving.locator("#location-form"))
                .to_be_visible()
                .await
                .unwrap();
            capture(page, "new-location", size).await;
            moving
                .get_by_label("Name", true)
                .fill("Synthetic destination", None)
                .await
                .unwrap();
            moving
                .get_by_role(AriaRole::Button, named("Save Location"))
                .click(None)
                .await
                .unwrap();
            expect(moving.locator("#medication-move-form"))
                .to_be_visible()
                .await
                .unwrap();
            moving
                .get_by_role(AriaRole::Button, named("Move Medication"))
                .click(None)
                .await
                .unwrap();
            expect(page.locator("main"))
                .to_contain_text("Moved Updated tablets")
                .await
                .unwrap();
            expect(page.locator("[data-testid='current-supply']"))
                .to_contain_text("Untracked")
                .await
                .unwrap();
        })
    })
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task host:test:browser"]
async fn medication_management_desktop() {
    management(
        Viewport {
            width: 1440,
            height: 1000,
        },
        "desktop",
    )
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task host:test:browser"]
async fn medication_management_mobile() {
    management(
        Viewport {
            width: 390,
            height: 844,
        },
        "mobile",
    )
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task host:test:browser"]
async fn medication_management_member_sections() {
    harness::journey(Viewport { width: 390, height: 844 }, |journey| Box::pin(async move {
        harness::sign_in(journey).await;
        let mut membership: household_membership::ActiveModel = household_membership::Entity::find_by_id(74001).one(&journey.admin).await.unwrap().unwrap().into();
        membership.role = Set("member".into());
        membership.update(&journey.admin).await.unwrap();
        let page = &journey.page;
        page.goto(&format!("{}/households/persistence-fixture/medications/80001", journey.origin), None).await.unwrap();
        assert!(page.evaluate::<(), bool>("() => !document.querySelector('[data-medication-inline], [data-medication-options], [data-stock-dialog], #delete-medication-dialog, [data-medication-move-dialog]')", None).await.unwrap());
        expect(page.locator("[data-testid='current-supply']")).to_be_visible().await.unwrap();
        capture(page, "member", "mobile").await;
    })).await;
}
