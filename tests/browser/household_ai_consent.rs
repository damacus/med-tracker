use crate::harness::{self, named};
use playwright_rs::expect;
use playwright_rs::protocol::{AriaRole, ScreenshotOptions, Viewport};
use sea_orm::EntityTrait;

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn household_ai_consent_is_explicit_accessible_and_withdrawable() {
    harness::journey(
        Viewport {
            width: 1440,
            height: 1000,
        },
        |journey| {
            Box::pin(async move {
                let page = &journey.page;
                harness::sign_in(journey).await;
                let url = format!(
                    "{}/households/persistence-fixture/admin/household/edit",
                    journey.origin
                );
                page.goto(&url, None).await.unwrap();
                let control = page.get_by_label("Allow external AI medication suggestions", true);
                expect(control.clone()).to_be_visible().await.unwrap();
                assert!(
                    !page
                        .evaluate::<(), bool>(
                            "() => document.querySelector('#ai_suggestions_enabled').checked",
                            None
                        )
                        .await
                        .unwrap()
                );
                expect(page.locator("#ai-suggestions-disclosure"))
                    .to_contain_text("configured external AI provider")
                    .await
                    .unwrap();
                let rejected = page.evaluate::<(), u16>("async () => (await fetch(document.querySelector('#administration-form').action, {method:'POST', headers:{'Content-Type':'application/x-www-form-urlencoded'}, body:'name=Synthetic&ai_suggestions_enabled=true'})).status", None).await.unwrap();
                assert_eq!(rejected, 403);
                control.focus().await.unwrap();
                page.keyboard().press("Space", None).await.unwrap();
                expect(control).to_be_checked().await.unwrap();
                page.get_by_label("Household name", true).fill("   ", None).await.unwrap();
                page.get_by_role(AriaRole::Button, named("Save household")).click(None).await.unwrap();
                expect(page.locator("[role=alert]")).to_be_visible().await.unwrap();
                expect(page.get_by_label("Allow external AI medication suggestions", true)).to_be_checked().await.unwrap();
                page.get_by_label("Household name", true).fill("Synthetic consent household", None).await.unwrap();
                page.get_by_role(AriaRole::Button, named("Save household"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.get_by_label("Allow external AI medication suggestions", true))
                    .to_be_checked()
                    .await
                    .unwrap();
                page.goto(&url, None).await.unwrap();
                expect(page.get_by_label("Allow external AI medication suggestions", true))
                    .to_be_checked()
                    .await
                    .unwrap();
                let directory = std::path::Path::new("docs/screenshots/household-ai-consent");
                std::fs::create_dir_all(directory).unwrap();
                let retained = std::path::Path::new("target/household-ai-consent-screenshots");
                std::fs::create_dir_all(retained).unwrap();
                for width in [1440, 390] {
                    page.set_viewport_size(Viewport {
                        width,
                        height: 1000,
                    })
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
                        &directory.join(format!("settings-{width}.png")),
                        ScreenshotOptions::builder().full_page(true).build(),
                    )
                    .await
                    .unwrap();
                    std::fs::copy(directory.join(format!("settings-{width}.png")), retained.join(format!("settings-{width}.png"))).unwrap();
                }
                page.get_by_label("Allow external AI medication suggestions", true)
                    .uncheck(None)
                    .await
                    .unwrap();
                page.get_by_role(AriaRole::Button, named("Save household"))
                    .click(None)
                    .await
                    .unwrap();
                let row = med_tracker::models::_entities::households::Entity::find_by_id(72001)
                    .one(&journey.admin)
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    serde_json::to_value(row).unwrap()["ai_suggestions_enabled"],
                    false
                );
            })
        },
    )
    .await;
}
