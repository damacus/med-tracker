use crate::harness::{self, named};
use med_tracker::models::_entities::{
    accounts, household_memberships, person_schedules, schedules,
};
use playwright_rs::expect;
use playwright_rs::protocol::{AriaRole, ScreenshotOptions, Viewport};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set, sea_query::Expr,
};

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn person_owned_schedule_browser_adoption_retains_application_zone() {
    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| Box::pin(async move {
        let actor = accounts::Entity::find().filter(accounts::Column::Email.eq("persistence@example.test"))
            .one(&journey.admin).await.unwrap().unwrap();
        let mut account = actor.clone().into_active_model();
        account.preferences = Set(serde_json::json!({"time_zone":"Asia/Tokyo"}));
        account.update(&journey.admin).await.unwrap();
        let now = chrono::Utc::now().naive_utc();
        let source = schedules::ActiveModel {
            household_id: Set(72001), person_id: Set(actor.person_id), medication_id: Set(80001),
            active: Set(true), dose_amount: Set(Some("2".parse().unwrap())), dose_unit: Set(Some("tablet".into())),
            dose_cycle: Set(Some(0)), max_daily_doses: Set(Some(4)),
            start_date: Set(Some("2026-01-01".parse().unwrap())), end_date: Set(Some("2027-12-31".parse().unwrap())),
            schedule_type: Set(0), schedule_config: Set(serde_json::json!({"times":["08:00"]})),
            created_at: Set(now), updated_at: Set(now), ..Default::default()
        }.insert(&journey.admin).await.unwrap();
        harness::sign_in(journey).await;
        let page = &journey.page;
        page.goto(&format!("{}/households/persistence-fixture/people/73001/treatments/schedules/{}/edit", journey.origin, source.id), None).await.unwrap();
        expect(page.locator("#adoption-zone-help")).to_contain_text("Saving adopts the application's time zone").await.unwrap();
        page.get_by_label("Notes", true).fill("Unrelated note edit", None).await.unwrap();
        let artifacts = std::path::Path::new("target/2684-task2-fix1-screenshots");
        std::fs::create_dir_all(artifacts).unwrap();
        for (name, width, height) in [("desktop", 1440, 1000), ("mobile", 390, 844)] {
            page.set_viewport_size(Viewport {width, height}).await.unwrap();
            assert!(!page.evaluate::<(), bool>("() => document.documentElement.scrollWidth > innerWidth", None).await.unwrap());
            page.locator("#origin-time-zone").focus().await.unwrap();
            let scroll = if name == "mobile" {
                "() => { const field = document.querySelector('#origin-time-zone'); window.scrollTo(0, field.getBoundingClientRect().top + window.scrollY - 150); }"
            } else {
                "() => { window.scrollTo(0, 0); }"
            };
            page.evaluate::<(), ()>(scroll, None).await.unwrap();
            page.screenshot_to_file(&artifacts.join(format!("browser-adoption-{name}.png")), ScreenshotOptions::builder().full_page(name == "desktop").build()).await.unwrap();
        }
        page.get_by_role(AriaRole::Button, named("Edit schedule")).click(None).await.unwrap();
        expect(page.locator("main")).to_contain_text("Unrelated note edit").await.unwrap();
        let stored = schedules::Entity::find_by_id(source.id).one(&journey.admin).await.unwrap().unwrap();
        assert_eq!(stored.origin_time_zone.as_deref(), Some("UTC"));
        assert_eq!(stored.portable_id, source.portable_id);
        let plan = person_schedules::Entity::find_by_id(stored.person_schedule_id.unwrap()).one(&journey.admin).await.unwrap().unwrap();
        assert_eq!(plan.origin_time_zone, "UTC");
        assert_eq!(plan.timezone_provenance, "adopted_application_zone");
        page.goto(&format!("{}/households/persistence-fixture/people/73001/treatments/schedules/new", journey.origin), None).await.unwrap();
        assert_eq!(page.evaluate::<(), String>("() => document.querySelector('#origin-time-zone').value", None).await.unwrap(), "Asia/Tokyo");
    })).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn person_owned_schedule_remains_editable_after_last_departure() {
    harness::journey(
        Viewport {
            width: 1440,
            height: 1000,
        },
        |journey| {
            Box::pin(async move {
                let actor = accounts::Entity::find()
                    .filter(accounts::Column::Email.eq("persistence@example.test"))
                    .one(&journey.admin)
                    .await
                    .unwrap()
                    .unwrap();
                let mut account = actor.clone().into_active_model();
                account.preferences = Set(serde_json::json!({"time_zone":"Europe/London"}));
                account.update(&journey.admin).await.unwrap();
                harness::sign_in(journey).await;
                let page = &journey.page;
                page.goto(
                    &format!(
                        "{}/households/persistence-fixture/people/73001/treatments/schedules/new",
                        journey.origin
                    ),
                    None,
                )
                .await
                .unwrap();
                page.get_by_label("Medication", true)
                    .select_option("80001", None)
                    .await
                    .unwrap();
                page.get_by_label("Dose amount", true)
                    .fill("2", None)
                    .await
                    .unwrap();
                page.get_by_label("Dose unit", true)
                    .select_option("tablet", None)
                    .await
                    .unwrap();
                page.get_by_label("Start date", true)
                    .fill("2026-01-01", None)
                    .await
                    .unwrap();
                page.get_by_label("End date", true)
                    .fill("2027-12-31", None)
                    .await
                    .unwrap();
                page.locator("#origin-time-zone")
                    .fill("Europe/London", None)
                    .await
                    .unwrap();
                page.get_by_role(AriaRole::Button, named("Add schedule"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.locator("main"))
                    .to_contain_text("Europe/London")
                    .await
                    .unwrap();
                let plan = person_schedules::Entity::find()
                    .filter(person_schedules::Column::PersonId.eq(actor.person_id))
                    .one(&journey.admin)
                    .await
                    .unwrap()
                    .unwrap();
                household_memberships::Entity::update_many()
                    .col_expr(
                        household_memberships::Column::Status,
                        Expr::value("revoked"),
                    )
                    .col_expr(
                        household_memberships::Column::RevokedAt,
                        Expr::value(chrono::Utc::now().naive_utc()),
                    )
                    .filter(household_memberships::Column::PersonId.eq(actor.person_id))
                    .exec(&journey.admin)
                    .await
                    .unwrap();
                page.goto(&format!("{}/profile/schedules", journey.origin), None)
                    .await
                    .unwrap();
                expect(page.get_by_role(AriaRole::Heading, named("My treatment plans")))
                    .to_be_visible()
                    .await
                    .unwrap();
                let amount = format!("#amount-{}", plan.id);
                page.locator(&amount).fill("3", None).await.unwrap();
                page.get_by_role(AriaRole::Button, named("Save plan"))
                    .click(None)
                    .await
                    .unwrap();
                page.reload(None).await.unwrap();
                assert_eq!(
                    page.evaluate::<(), String>(
                        "() => document.querySelector('[name=dose_amount]').value",
                        None
                    )
                    .await
                    .unwrap(),
                    "3"
                );
                page.get_by_role(AriaRole::Button, named("Retire plan"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.locator("dialog[open] button[data-dialog-close]"))
                    .to_be_focused()
                    .await
                    .unwrap();
                page.locator("dialog[open]")
                    .press("Escape", None)
                    .await
                    .unwrap();
                expect(page.locator("button[data-dialog-open^='retire-']"))
                    .to_be_focused()
                    .await
                    .unwrap();
                page.get_by_role(AriaRole::Button, named("Pause schedule"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.get_by_role(AriaRole::Button, named("Resume schedule")))
                    .to_be_visible()
                    .await
                    .unwrap();
                page.get_by_role(AriaRole::Button, named("Resume schedule"))
                    .click(None)
                    .await
                    .unwrap();
                let directory = std::path::Path::new("docs/screenshots/person-owned-schedules");
                let artifacts = std::path::Path::new("target/2684-task2-screenshots");
                std::fs::create_dir_all(directory).unwrap();
                std::fs::create_dir_all(artifacts).unwrap();
                for (name, width, height) in [("desktop", 1440, 1000), ("mobile", 390, 844)] {
                    page.set_viewport_size(Viewport { width, height })
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
                    page.locator(&amount).focus().await.unwrap();
                    page.locator(&amount).press("Tab", None).await.unwrap();
                    expect(page.locator("select[name=dose_unit]"))
                        .to_be_focused()
                        .await
                        .unwrap();
                    let screenshot = directory.join(format!("personal-continuity-{name}.png"));
                    page.screenshot_to_file(
                        &screenshot,
                        ScreenshotOptions::builder().full_page(true).build(),
                    )
                    .await
                    .unwrap();
                    std::fs::copy(&screenshot, artifacts.join(screenshot.file_name().unwrap()))
                        .unwrap();
                }
            })
        },
    )
    .await;
}
