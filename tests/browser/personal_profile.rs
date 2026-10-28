use crate::harness::{self, named};
use base64::Engine as _;
use med_tracker::models::_entities::{
    accounts, household_memberships, medication_takes, person_medications,
};
use playwright_rs::expect;
use playwright_rs::protocol::{Animations, AriaRole, FilePayload, ScreenshotOptions, Viewport};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, sea_query::Expr};

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn household_free_profile_preserves_editing_and_personal_history() {
    harness::journey(
        Viewport {
            width: 1440,
            height: 1000,
        },
        |journey| {
            Box::pin(async move {
                crate::polish_followups::prepare_avatar_storage().await;
                harness::sign_in(journey).await;
                let account = accounts::Entity::find()
                    .filter(accounts::Column::Email.eq("persistence@example.test"))
                    .one(&journey.admin)
                    .await
                    .unwrap()
                    .unwrap();
                let assignment = person_medications::Entity::find().filter(person_medications::Column::PersonId.eq(account.person_id)).one(&journey.admin).await.unwrap().unwrap();
                let now = chrono::Utc::now().naive_utc();
                medication_takes::ActiveModel {
                    household_id: Set(assignment.household_id), person_medication_id: Set(Some(assignment.id)),
                    taken_from_medication_id: Set(Some(assignment.medication_id)), dose_amount: Set(Some(sea_orm::prelude::Decimal::new(2, 0))), dose_unit: Set(Some("tablet".into())),
                    taken_at: Set(Some(now)), created_at: Set(now), updated_at: Set(now), ..Default::default()
                }.insert(&journey.admin).await.unwrap();
                household_memberships::Entity::update_many()
                    .col_expr(
                        household_memberships::Column::Status,
                        Expr::value("revoked"),
                    )
                    .col_expr(
                        household_memberships::Column::RevokedAt,
                        Expr::value(chrono::Utc::now().naive_utc()),
                    )
                    .filter(household_memberships::Column::PersonId.eq(account.person_id))
                    .exec(&journey.admin)
                    .await
                    .unwrap();
                let page = &journey.page;
                page.goto(&format!("{}/households/choose", journey.origin), None)
                    .await
                    .unwrap();
                let directory = std::path::Path::new("docs/screenshots/personal-profile");
                std::fs::create_dir_all(directory).unwrap();
                for (size, width, height) in [("desktop", 1440, 1000), ("mobile", 390, 844)] {
                    page.set_viewport_size(Viewport { width, height }).await.unwrap();
                    page.screenshot_to_file(&directory.join(format!("arrival-{size}.png")), ScreenshotOptions::builder().full_page(true).build()).await.unwrap();
                }
                page.get_by_role(AriaRole::Link, named("Profile"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.locator("[data-testid=profile-personal-info-card]"))
                    .to_be_visible()
                    .await
                    .unwrap();
                assert_eq!(
                    page.locator("[data-app-drawer-layout]")
                        .count()
                        .await
                        .unwrap(),
                    0
                );
                for name in ["Synthetic account first", "Synthetic account final"] {
                    page.locator("#account-display-name").fill(name, None).await.unwrap();
                    page.locator("#profile-account-name-form button[type=submit]").click(None).await.unwrap();
                    expect(page.locator("#profile-account-name-form:not([data-submitting]) button[type=submit]")).to_be_focused().await.unwrap();
                    assert_eq!(page.evaluate::<(), String>("() => document.querySelector('#account-display-name').value", None).await.unwrap(), name);
                }
                for name in ["Synthetic clinical first", "Synthetic clinical final"] {
                    page.locator("#clinical-name").fill(name, None).await.unwrap();
                    page.locator("#profile-clinical-form button[type=submit]").click(None).await.unwrap();
                    expect(page.locator("#profile-clinical-form:not([data-submitting]) button[type=submit]")).to_be_focused().await.unwrap();
                    assert_eq!(page.evaluate::<(), String>("() => document.querySelector('#clinical-name').value", None).await.unwrap(), name);
                }
                page.locator("#profile-avatar > summary").click(None).await.unwrap();
                let bytes = base64::engine::general_purpose::STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABAQMAAAAl21bKAAAAA1BMVEX/AAAZ4gk3AAAACklEQVQI12NgAAAAAgAB4iG8MwAAAABJRU5ErkJggg==").unwrap();
                page.locator("#profile-avatar-file").set_input_files_payload(FilePayload::new("synthetic.png", "image/png", bytes), None).await.unwrap();
                page.locator("form[action='/profile/avatar'] button[type=submit]").click(None).await.unwrap();
                expect(page.locator("form[action='/profile/avatar/remove']")).to_be_visible().await.unwrap();
                assert!(page.evaluate::<(), bool>("async () => { const image = document.querySelector('[data-profile-avatar] img'); await image.decode(); return image.naturalWidth > 0 && new URL(image.src).pathname === '/profile/avatar'; }", None).await.unwrap());
                page.locator("form[action='/profile/avatar/remove'] button").click(None).await.unwrap();
                expect(page.locator("form[action='/profile/avatar/remove']")).to_have_count(0).await.unwrap();
                page.locator("#profile-avatar > summary").click(None).await.unwrap();
                page.locator("#profile-timezone > summary")
                    .click(None)
                    .await
                    .unwrap();
                page.evaluate::<(), bool>("() => { document.querySelector('#profile-timezone-select').add(new Option('Invalid/Zone', 'Invalid/Zone')); return true; }", None).await.unwrap();
                page.locator("#profile-timezone-select").select_option("Invalid/Zone", None).await.unwrap();
                page.locator("#profile-timezone-form button[type=submit]").click(None).await.unwrap();
                expect(page.locator("#profile-timezone-form [role=alert]")).to_be_visible().await.unwrap();
                expect(page.locator("#profile-timezone-form [role=alert]")).to_be_focused().await.unwrap();
                page.screenshot_to_file(&directory.join("invalid-timezone-mobile.png"), ScreenshotOptions::builder().full_page(true).build()).await.unwrap();
                page.locator("#profile-timezone-select")
                    .select_option("America/New_York", None)
                    .await
                    .unwrap();
                page.locator("#profile-timezone-form button[type=submit]")
                    .click(None)
                    .await
                    .unwrap();
                expect(page.locator("#profile-timezone-summary"))
                    .to_contain_text("America/New_York")
                    .await
                    .unwrap();
                page.reload(None).await.unwrap();
                expect(page.locator("#profile-timezone-summary"))
                    .to_contain_text("America/New_York")
                    .await
                    .unwrap();
                for (size, width, height) in [("desktop", 1440, 1000), ("mobile", 390, 844)] {
                    page.set_viewport_size(Viewport { width, height })
                        .await
                        .unwrap();
                    for theme in ["default-light", "default-dark"] {
                        page.evaluate::<_, ()>(
                            "theme => document.documentElement.setAttribute('data-theme', theme)",
                            Some(&theme),
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
                        page.screenshot_to_file(
                            &directory.join(format!("profile-{size}-{theme}.png")),
                            ScreenshotOptions::builder()
                                .animations(Animations::Disabled)
                                .full_page(true)
                                .build(),
                        )
                        .await
                        .unwrap();
                    }
                }
                page.get_by_role(AriaRole::Link, named("My taken-dose history"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.get_by_role(AriaRole::Heading, named("My taken-dose history")))
                    .to_be_visible()
                    .await
                    .unwrap();
                expect(page.get_by_role(AriaRole::Heading, named("Synthetic tablets"))).to_be_visible().await.unwrap();
                page.screenshot_to_file(
                    &directory.join("history-mobile.png"),
                    ScreenshotOptions::builder().full_page(true).build(),
                )
                .await
                .unwrap();
            })
        },
    )
    .await;
}
