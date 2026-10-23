use crate::harness::{self, named};
use med_tracker::models::_entities::people;
use playwright_rs::expect;
use playwright_rs::protocol::{Animations, AriaRole, ScreenshotOptions, Viewport};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn person_editing_preserves_form_and_saved_fields() {
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
                        "{}/households/persistence-fixture/people/73001/edit",
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
                page.get_by_label("Name", true)
                    .fill("Synthetic edited adult", None)
                    .await
                    .unwrap();
                page.get_by_label("Email", true)
                    .fill("Edited@Example.Test", None)
                    .await
                    .unwrap();
                page.get_by_label("Date of Birth", true)
                    .fill("1990-02-03", None)
                    .await
                    .unwrap();
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
                    let directory = std::path::Path::new("docs/screenshots/person-editing");
                    std::fs::create_dir_all(directory).unwrap();
                    page.screenshot_to_file(
                        &directory.join(format!("final-{name}.png")),
                        ScreenshotOptions::builder()
                            .animations(Animations::Disabled)
                            .full_page(true)
                            .build(),
                    )
                    .await
                    .unwrap();
                }
                page.get_by_role(AriaRole::Button, named("Update Person"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.get_by_role(AriaRole::Heading, named("Synthetic edited adult")))
                    .to_be_visible()
                    .await
                    .unwrap();
                let person = people::Entity::find()
                    .filter(people::Column::Id.eq(73001))
                    .one(&journey.admin)
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(person.name, "Synthetic edited adult");
                assert_eq!(person.email.as_deref(), Some("edited@example.test"));
                assert_eq!(person.date_of_birth.unwrap().to_string(), "1990-02-03");
            })
        },
    )
    .await;
}
