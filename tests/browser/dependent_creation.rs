use crate::harness::{self, named};
use med_tracker::models::_entities::{carer_relationships, people};
use playwright_rs::expect;
use playwright_rs::protocol::{Animations, AriaRole, ScreenshotOptions, Viewport};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn dependent_creation_keeps_form_and_guardian_declaration() {
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
                        "{}/households/persistence-fixture/people/new",
                        journey.origin
                    ),
                    None,
                )
                .await
                .unwrap();
                for person_type in ["minor", "dependent_adult", "minor"] {
                    page.locator("#person_type").select_option(person_type, None).await.unwrap();
                    assert!(page.evaluate::<(), bool>(
                        "() => { const capacity = document.getElementById('has_capacity'); return capacity.disabled && !capacity.checked; }",
                        None,
                    ).await.unwrap());
                }
                page.get_by_label("Name", true)
                    .fill("Synthetic typed child", None)
                    .await
                    .unwrap();
                expect(page.locator("body[data-navigation-ready]"))
                    .to_be_visible()
                    .await
                    .unwrap();
                page.get_by_label("Email", true)
                    .fill("Child@Example.Test", None)
                    .await
                    .unwrap();
                page.get_by_label("Date of Birth", true)
                    .fill("2020-01-01", None)
                    .await
                    .unwrap();
                page.locator("#initial_guardian_declaration")
                    .check(None)
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
                    let directory = std::path::Path::new("docs/screenshots/dependent-creation");
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
                page.get_by_label("Date of Birth", true).fill("2999-01-01", None).await.unwrap();
                page.get_by_role(AriaRole::Button, named("Create Person")).click(None).await.unwrap();
                expect(page.locator("[role=alert]")).to_be_visible().await.unwrap();
                assert!(page.evaluate::<(), bool>(
                    "() => { const capacity = document.getElementById('has_capacity'); return capacity.disabled && !capacity.checked; }",
                    None,
                ).await.unwrap());
                page.get_by_label("Date of Birth", true).fill("2020-01-01", None).await.unwrap();
                page.locator("#initial_guardian_declaration").check(None).await.unwrap();
                page.get_by_role(AriaRole::Button, named("Create Person"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.get_by_role(AriaRole::Heading, named("Synthetic typed child")))
                    .to_be_visible()
                    .await
                    .unwrap();
                let person = people::Entity::find()
                    .filter(people::Column::Name.eq("Synthetic typed child"))
                    .one(&journey.admin)
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(person.email.as_deref(), Some("child@example.test"));
                assert_eq!(person.person_type, 1);
                assert!(!person.has_capacity);
                assert_eq!(
                    carer_relationships::Entity::find()
                        .filter(carer_relationships::Column::PatientId.eq(person.id))
                        .filter(carer_relationships::Column::CarerId.eq(73001))
                        .filter(carer_relationships::Column::HouseholdId.eq(72001))
                        .filter(carer_relationships::Column::GuardianDeclarationId.is_not_null())
                        .filter(carer_relationships::Column::Active.eq(true))
                        .count(&journey.admin)
                        .await
                        .unwrap(),
                    1
                );
                page.goto(&format!("{}/households/persistence-fixture/people/73001/edit", journey.origin), None).await.unwrap();
                assert!(page.evaluate::<(), bool>(
                    r#"() => { const capacity = document.getElementById("has_capacity"); return !capacity.disabled && capacity.checked; }"#,
                    None,
                ).await.unwrap());
            })
        },
    )
    .await;
}
