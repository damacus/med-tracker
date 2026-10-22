use crate::harness::{self, named};
use playwright_rs::expect;
use playwright_rs::protocol::{AriaRole, FulfillOptions, ScreenshotOptions, Viewport};
use sea_orm::ConnectionTrait;

async fn capture(page: &playwright_rs::protocol::Page, name: &str) {
    expect(page.locator("body[data-navigation-ready]"))
        .to_be_visible()
        .await
        .unwrap();
    assert!(
        !page
            .evaluate::<(), bool>(
                "() => document.documentElement.scrollWidth > innerWidth",
                None
            )
            .await
            .unwrap(),
        "overflow in {name}"
    );
    if page
        .evaluate::<(), bool>("() => innerWidth < 1024", None)
        .await
        .unwrap()
    {
        expect(page.locator("[data-app-drawer]"))
            .to_be_visible()
            .await
            .unwrap();
    } else {
        expect(page.locator(".app-sidebar [data-section=people][aria-current=page]"))
            .to_be_visible()
            .await
            .unwrap();
    }
    let directory = std::path::Path::new("docs/screenshots/shared-layout");
    std::fs::create_dir_all(directory).unwrap();
    page.screenshot_to_file(
        &directory.join(format!("people-{name}.png")),
        ScreenshotOptions::default(),
    )
    .await
    .unwrap();
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn people_polish_cards_and_medication_journey() {
    harness::journey(
        Viewport {
            width: 1440,
            height: 1000,
        },
        |journey| {
            Box::pin(async move {
                let page = &journey.page;
                journey.admin.execute_unprepared("INSERT INTO medications(id,household_id,location_id,name,dose_unit,created_at,updated_at) VALUES(80011,72001,79001,'Synthetic second medicine','tablet',now(),now()); UPDATE people SET date_of_birth=CASE id WHEN 73001 THEN '1990-02-03'::date WHEN 73002 THEN '2020-05-12'::date ELSE '1980-01-14'::date END WHERE id IN(73001,73002,73003)").await.unwrap();
                harness::sign_in(journey).await;
                page.goto(
                    &format!("{}/households/persistence-fixture/people", journey.origin),
                    None,
                )
                .await
                .unwrap();
                expect(page.get_by_role(AriaRole::Link, named("New Person")))
                    .to_be_visible()
                    .await
                    .unwrap();
                expect(page.locator("[data-person-card='73001'] [data-active-medications]"))
                    .to_be_visible()
                    .await
                    .unwrap();
                capture(page, "desktop").await;
                page.locator("[data-person-card='73001']").get_by_role(AriaRole::Link,named("View Medications")).click(None).await.unwrap();
                expect(page.locator("#current-medications-heading")).to_be_visible().await.unwrap();
                page.get_by_role(AriaRole::Link,named("Add Medication")).click(None).await.unwrap();
                expect(page.locator("main a[href*='/people/73001/treatments/assignments/new']")).to_be_visible().await.unwrap();
                page.goto(&format!("{}/households/persistence-fixture/people",journey.origin),None).await.unwrap();

                for width in [390, 320] {
                    page.set_viewport_size(Viewport { width, height: 844 })
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
                    capture(page, &format!("cards-{width}")).await;
                }
                let card = page.locator("[data-person-card='73001']");
                card.get_by_role(AriaRole::Link, named("View Medications"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.locator("#current-medications-heading"))
                    .to_be_visible()
                    .await
                    .unwrap();
                capture(page, "detail-320").await;
                page.set_viewport_size(Viewport {width:1440,height:1000}).await.unwrap();
                expect(page.locator("aside dl")).to_contain_text("3 February 1990").await.unwrap();
                capture(page, "detail-desktop").await;
                page.set_viewport_size(Viewport {width:390,height:844}).await.unwrap();
                capture(page, "detail-390").await;
                page.set_viewport_size(Viewport {width:320,height:844}).await.unwrap();
                page.get_by_role(AriaRole::Link, named("Add Medication"))
                    .click(None)
                    .await
                    .unwrap();
                page.locator("main a[href*='/people/73001/treatments/assignments/new']").click(None).await.unwrap();
                expect(page.locator("main")).to_contain_text("Synthetic adult").await.unwrap();
                page.get_by_label("Medication",true).select_option("80011",None).await.unwrap();
                page.get_by_label("Dose amount",true).fill("1",None).await.unwrap();
                page.get_by_label("Dose unit",true).select_option("tablet",None).await.unwrap();
                page.get_by_role(AriaRole::Button,named("Assign medication")).click(None).await.unwrap();
                expect(page.get_by_role(AriaRole::Heading,named("Treatments for Synthetic adult"))).to_be_visible().await.unwrap();
                expect(page.locator("main")).to_contain_text("Synthetic second medicine").await.unwrap();
                page.goto(&format!("{}/households/persistence-fixture/people/73001",journey.origin),None).await.unwrap();
                expect(page.locator("#current-medications-heading")).to_be_visible().await.unwrap();
                page.goto(
                    &format!(
                        "{}/households/persistence-fixture/people/new",
                        journey.origin
                    ),
                    None,
                )
                .await
                .unwrap();
                page.get_by_label("Name", true)
                    .fill("Synthetic draft child", None)
                    .await
                    .unwrap();
                page.get_by_label("Date of Birth", true)
                    .fill("2999-01-01", None)
                    .await
                    .unwrap();
                page.locator("#initial_guardian_declaration")
                    .check(None)
                    .await
                    .unwrap();
                page.get_by_role(AriaRole::Button, named("Create Person"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.locator("#name"))
                    .to_have_value("Synthetic draft child")
                    .await
                    .unwrap();
                expect(page.locator("[role=alert]"))
                    .to_be_visible()
                    .await
                    .unwrap();
                expect(page.locator("#person_type")).to_be_focused().await.unwrap();
                capture(page, "error-320").await;
                page.evaluate::<(),bool>("() => { window.scrollTo(0,0); return true; }",None).await.unwrap();
                capture(page, "error-summary-320").await;
                page.set_viewport_size(Viewport {
                    width: 1440,
                    height: 1000,
                })
                .await
                .unwrap();
                page.goto(
                    &format!(
                        "{}/households/persistence-fixture/people/73001/edit",
                        journey.origin
                    ),
                    None,
                )
                .await
                .unwrap();
                capture(page, "edit-desktop").await;
            })
        },
    )
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn people_polish_counts_consent_and_visibility() {
    harness::journey(Viewport {width:1440,height:1000}, |journey| {
        Box::pin(async move {
            let page=&journey.page;
            journey.admin.execute_unprepared("INSERT INTO medications(id,household_id,location_id,name,created_at,updated_at) VALUES(80011,72001,79001,'Synthetic paused medicine',now(),now()); UPDATE people SET date_of_birth='1990-02-03',email='synthetic@example.test' WHERE id=73001; UPDATE people SET email='accountless@example.test' WHERE id=73002; INSERT INTO person_medications(household_id,person_id,medication_id,position,active,retired_at,created_at,updated_at) VALUES(72001,73001,80011,1,false,NULL,now(),now()),(72001,73001,80001,2,true,now(),now(),now()); INSERT INTO schedules(household_id,person_id,medication_id,active,start_date,end_date,created_at,updated_at) VALUES(72001,73001,80001,true,current_date-1,current_date+1,now(),now()),(72001,73001,80001,true,current_date+1,current_date+5,now(),now()),(72001,73001,80001,true,current_date-5,current_date-1,now(),now()),(72001,73001,80001,false,current_date-1,current_date+1,now(),now());").await.unwrap();
            harness::sign_in(journey).await;
            page.goto(&format!("{}/households/persistence-fixture/people",journey.origin),None).await.unwrap();
            expect(page.locator("[data-person-card='73001'] [data-active-medications]")).to_have_text("2 active medications").await.unwrap();
            expect(page.locator("[data-person-card='73001'] time")).to_have_text("3 February 1990").await.unwrap();
            assert_eq!(page.locator("[data-person-card='73002'] img").count().await.unwrap(),0);
            assert_eq!(page.locator("[data-person-card='73005']").count().await.unwrap(),0);
            assert_eq!(page.locator("[data-person-card='73002'] a[href*='add-medication']").count().await.unwrap(),0);
            assert_eq!(page.locator("main img[src*='gravatar']").count().await.unwrap(),0);
            journey.admin.execute_unprepared("UPDATE accounts SET preferences=jsonb_set(preferences,'{gravatar_enabled}','true') WHERE id=71001").await.unwrap();
            page.route("https://www.gravatar.com/**",|route|async move {route.fulfill(FulfillOptions::builder().content_type("image/svg+xml").body_string("<svg xmlns='http://www.w3.org/2000/svg' width='48' height='48'><rect width='48' height='48' fill='#86a998'/></svg>").build()).await}).await.unwrap();
            page.reload(None).await.unwrap();
            expect(page.locator("[data-person-card='73001'] img[src*='gravatar']")).to_be_visible().await.unwrap();
            assert_eq!(page.locator("[data-person-card='73002'] img[src*='gravatar']").count().await.unwrap(),0);
            journey.admin.execute_unprepared("INSERT INTO active_storage_blobs(id,key,filename,content_type,byte_size,service_name,created_at) VALUES(79901,'synthetic-accountless-avatar','synthetic.png','image/png',1,'s3_with_persistent_mirror',now()); INSERT INTO active_storage_attachments(name,record_type,record_id,blob_id,household_id,created_at) VALUES('avatar','Person',73002,79901,72001,now()); UPDATE accounts SET preferences=jsonb_set(preferences,'{gravatar_enabled}','false') WHERE id=71001").await.unwrap();
            page.route("**/people/73002/avatar",|route|async move {route.fulfill(FulfillOptions::builder().content_type("image/svg+xml").body_string("<svg xmlns='http://www.w3.org/2000/svg' width='48' height='48'><rect width='48' height='48' fill='#86a998'/></svg>").build()).await}).await.unwrap();
            page.reload(None).await.unwrap();
            expect(page.locator("[data-person-card='73002'] img[src$='/73002/avatar']")).to_be_visible().await.unwrap();
            assert_eq!(page.locator("main img[src*='gravatar']").count().await.unwrap(),0);
            page.goto(&format!("{}/households/persistence-fixture/people?per_page=1",journey.origin),None).await.unwrap();
            page.get_by_role(AriaRole::Link,named("Next page")).click(None).await.unwrap();
            expect(page.locator("[data-person-card='73002']")).to_be_visible().await.unwrap();
            page.get_by_role(AriaRole::Link,named("Previous page")).click(None).await.unwrap();
            expect(page.locator("[data-person-card='73001']")).to_be_visible().await.unwrap();
            journey.admin.execute_unprepared("UPDATE household_memberships SET status='revoked',revoked_at=now(),permissions_version=permissions_version+1 WHERE id=74001").await.unwrap();
            page.goto(&format!("{}/households/persistence-fixture/people",journey.origin),None).await.unwrap();
            assert_eq!(page.locator("[data-person-card]").count().await.unwrap(),0);
        })
    }).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn people_polish_empty_page_is_usable() {
    harness::journey(
        Viewport {
            width: 1440,
            height: 1000,
        },
        |journey| {
            Box::pin(async move {
                let page = &journey.page;
                harness::sign_in(journey).await;
                page.goto(
                    &format!(
                        "{}/households/persistence-fixture/people?page=2",
                        journey.origin
                    ),
                    None,
                )
                .await
                .unwrap();
                expect(page.locator(".people-empty"))
                    .to_contain_text("No people to show")
                    .await
                    .unwrap();
                expect(page.get_by_role(AriaRole::Link, named("Previous page")))
                    .to_be_visible()
                    .await
                    .unwrap();
                capture(page, "empty-desktop").await;
                page.set_viewport_size(Viewport {
                    width: 390,
                    height: 844,
                })
                .await
                .unwrap();
                capture(page, "empty-390").await;
                page.locator(".people-empty")
                    .get_by_role(AriaRole::Link, named("New Person"))
                    .click(None)
                    .await
                    .unwrap();
                expect(page.locator("#person-form"))
                    .to_be_visible()
                    .await
                    .unwrap();
                capture(page, "new-390").await;
            })
        },
    )
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn people_polish_render_locales() {
    use loco_rs::{controller::views::ViewRenderer, prelude::TeraView};
    let view = TeraView::build().unwrap();
    for (lang, source) in [
        ("en", include_str!("../../assets/reports/locales/en.yml")),
        ("cy", include_str!("../../assets/reports/locales/cy.yml")),
        ("ga", include_str!("../../assets/reports/locales/ga.yml")),
        ("es", include_str!("../../assets/reports/locales/es.yml")),
        ("pt", include_str!("../../assets/reports/locales/pt.yml")),
    ] {
        let all: serde_json::Value = serde_yaml_ng::from_str(source).unwrap();
        let labels = &all[lang];
        let data = serde_json::json!({"slug":"synthetic","title":"People","lang":lang,"labels":labels,"care_warning_labels":labels["care_warning"],"authenticity_token":"synthetic","allow_palette":true,"appearances":[],"palettes":[],"can_create":true,"people":[{"id":1,"name":"Synthetic adult","care_warning":null,"age_present":false,"date_of_birth_display":null,"avatar_initials":"SA","avatar_attached":false,"gravatar_url":null,"can_manage":true,"active_medication_label":"1 active medication"}],"previous_page":null,"next_page":null,"per_page":20});
        view.render("people/index.html", data)
            .unwrap_or_else(|error| panic!("{lang}: {error:?}"));
    }
}
