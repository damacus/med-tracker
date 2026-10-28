use crate::harness::{self, named};
use med_tracker::models::_entities::{accounts, household_memberships, person_schedules};
use playwright_rs::expect;
use playwright_rs::protocol::{AriaRole, ScreenshotOptions, Viewport};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set, sea_query::Expr,
};

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
