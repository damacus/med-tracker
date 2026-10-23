use crate::harness::{self, named};
use med_tracker::models::entities::{location, medication, person_medication, schedule};
use playwright_rs::{
    expect,
    protocol::{Animations, AriaRole, ScreenshotOptions, Viewport},
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

async fn storage_location_deletion(viewport: Viewport, size: &'static str) {
    harness::journey(viewport, |journey| Box::pin(async move {
        let medication_before = medication::Entity::find_by_id(80001).one(&journey.admin).await.unwrap();
        let assignments_before = person_medication::Entity::find().all(&journey.admin).await.unwrap();
        let schedules_before = schedule::Entity::find().all(&journey.admin).await.unwrap();
        let now = chrono::Utc::now().naive_utc();
        location::ActiveModel {
            id: Set(79991), household_id: Set(72001), name: Set("Empty storage label".into()),
            portable_id: Set("empty-storage-label".into()), created_at: Set(now), updated_at: Set(now),
            ..Default::default()
        }.insert(&journey.admin).await.unwrap();
        harness::sign_in(journey).await;
        let page = &journey.page;
        page.goto(&format!("{}/households/persistence-fixture/locations/79001", journey.origin), None).await.unwrap();
        expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
        page.get_by_role(AriaRole::Button, named("Delete Location")).click(None).await.unwrap();
        let dialog = page.get_by_role(AriaRole::Dialog, named("Delete Location"));
        expect(dialog.clone()).to_be_visible().await.unwrap();
        let directory = std::path::Path::new("docs/screenshots/storage-location-deletion");
        std::fs::create_dir_all(directory).unwrap();
        page.screenshot_to_file(&directory.join(format!("confirmation-{size}.png")), ScreenshotOptions::builder().animations(Animations::Disabled).full_page(true).build()).await.unwrap();
        expect(page.get_by_text("Only empty locations without administration history can be deleted. Medications and care records are never deleted with a location.", true)).to_be_visible().await.unwrap();
        page.get_by_role(AriaRole::Button, named("Cancel")).click(None).await.unwrap();
        expect(page.locator("[data-dialog-open='delete-location']")).to_be_focused().await.unwrap();
        page.get_by_role(AriaRole::Button, named("Delete Location")).click(None).await.unwrap();
        page.locator("#delete-location button[type='submit']").click(None).await.unwrap();
        expect(page.get_by_text("cannot be deleted while medications are stored here; move them to another location first", false)).to_be_visible().await.unwrap();
        assert_eq!(medication::Entity::find_by_id(80001).one(&journey.admin).await.unwrap(), medication_before);
        assert_eq!(person_medication::Entity::find().all(&journey.admin).await.unwrap(), assignments_before);
        assert_eq!(schedule::Entity::find().all(&journey.admin).await.unwrap(), schedules_before);
        assert!(!page.evaluate::<(),bool>("() => document.documentElement.scrollWidth > innerWidth",None).await.unwrap());
        page.screenshot_to_file(&directory.join(format!("rejected-{size}.png")), ScreenshotOptions::builder().animations(Animations::Disabled).full_page(true).build()).await.unwrap();
        page.goto(&format!("{}/households/persistence-fixture/locations/79991", journey.origin), None).await.unwrap();
        page.get_by_role(AriaRole::Button, named("Delete Location")).click(None).await.unwrap();
        page.locator("#delete-location button[type='submit']").click(None).await.unwrap();
        expect(page.get_by_role(AriaRole::Heading, named("Locations"))).to_be_visible().await.unwrap();
        assert!(location::Entity::find_by_id(79991).one(&journey.admin).await.unwrap().is_none());
    })).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn storage_location_deletion_desktop() {
    storage_location_deletion(
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
async fn storage_location_deletion_mobile() {
    storage_location_deletion(
        Viewport {
            width: 390,
            height: 844,
        },
        "mobile",
    )
    .await;
}
