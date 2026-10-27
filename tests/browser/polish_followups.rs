use crate::harness;
use base64::Engine as _;
use playwright_rs::{
    expect,
    protocol::{FilePayload, Viewport},
};

#[path = "../support/storage_endpoint.rs"]
mod storage_endpoint;

async fn prepare_avatar_storage() {
    let endpoint = storage_endpoint::owned_storage_endpoint();
    let config = aws_sdk_s3::Config::builder()
        .region(aws_sdk_s3::config::Region::new("us-east-1"))
        .credentials_provider(aws_sdk_s3::config::Credentials::new(
            "storage-smoke-access",
            "storage-smoke-secret",
            None,
            None,
            "owned-browser-avatar",
        ))
        .endpoint_url(endpoint)
        .force_path_style(true)
        .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
        .timeout_config(
            aws_sdk_s3::config::timeout::TimeoutConfig::builder()
                .operation_timeout(std::time::Duration::from_secs(4))
                .build(),
        )
        .build();
    let client = aws_sdk_s3::Client::from_conf(config);
    if client
        .head_bucket()
        .bucket("medtracker-fixture")
        .send()
        .await
        .is_err()
    {
        client
            .create_bucket()
            .bucket("medtracker-fixture")
            .send()
            .await
            .unwrap();
    }
}

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
            .unwrap()
    );
    page.evaluate::<(), bool>("() => { window.scrollTo(0,0); return true; }", None)
        .await
        .unwrap();
    let directory = std::path::Path::new("docs/screenshots/polish-followups");
    std::fs::create_dir_all(directory).unwrap();
    page.screenshot_to_file(
        &directory.join(format!("{name}.png")),
        playwright_rs::protocol::ScreenshotOptions::builder()
            .animations(playwright_rs::protocol::Animations::Disabled)
            .build(),
    )
    .await
    .unwrap();
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn polish_profile_avatar_refreshes_all_existing_surfaces_without_navigation() {
    harness::journey(Viewport{width:1440,height:1000},|journey|Box::pin(async move {
        prepare_avatar_storage().await;
        harness::sign_in(journey).await;
        let page=&journey.page;
        page.goto(&format!("{}/households/persistence-fixture/profile",journey.origin),None).await.unwrap();
        expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
        page.evaluate::<(),bool>("() => { window.polishDocument = document; window.polishAccount = document.querySelector('#app-account'); return true; }",None).await.unwrap();
        page.locator("#profile-avatar > summary").click(None).await.unwrap();
        let bytes=base64::engine::general_purpose::STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABAQMAAAAl21bKAAAAA1BMVEX/AAAZ4gk3AAAACklEQVQI12NgAAAAAgAB4iG8MwAAAABJRU5ErkJggg==").unwrap();
        page.locator("#profile-avatar-file").set_input_files_payload(FilePayload::new("synthetic.png","image/png",bytes),None).await.unwrap();
        page.locator("form[action$='/profile/avatar'] button[type=submit]").click(None).await.unwrap();
        expect(page.locator("form[action$='/profile/avatar/remove']")).to_be_visible().await.unwrap();
        expect(page.locator("[data-sidebar-avatar] img")).to_be_visible().await.unwrap();
        assert!(page.evaluate::<(),bool>("async () => { const avatars=[...document.querySelectorAll('[data-profile-avatar], [data-sidebar-avatar]')]; await Promise.all(avatars.map(async avatar => { const image=avatar.querySelector('img'); if(image) await image.decode(); })); return avatars.length>=4 && avatars.every(avatar => { const image=avatar.querySelector('img'); return image && image.complete && image.naturalWidth>0; }) && window.polishDocument===document && window.polishAccount===document.querySelector('#app-account'); }",None).await.unwrap());
        capture(page,"avatar-upload-desktop").await;
        page.set_viewport_size(Viewport{width:390,height:844}).await.unwrap();
        capture(page,"avatar-upload-mobile").await;
        page.set_viewport_size(Viewport{width:1440,height:1000}).await.unwrap();
        page.locator("form[action$='/profile/avatar/remove'] button").click(None).await.unwrap();
        expect(page.locator("form[action$='/profile/avatar/remove']")).to_have_count(0).await.unwrap();
        assert_eq!(page.locator("[data-profile-avatar] img, [data-sidebar-avatar] img").count().await.unwrap(),0);
        assert!(page.evaluate::<(),bool>("() => window.polishDocument===document && window.polishAccount===document.querySelector('#app-account') && [...document.querySelectorAll('[data-profile-avatar], [data-sidebar-avatar]')].every(avatar=>avatar.textContent.trim().length>0)",None).await.unwrap());
        page.locator(".app-sidebar [data-sidebar-account]").click(None).await.unwrap();
        expect(page.locator("#app-account[open]")).to_be_visible().await.unwrap();
        page.keyboard().press("Escape",None).await.unwrap();
        expect(page.locator(".app-sidebar [data-sidebar-account]")).to_be_focused().await.unwrap();
    })).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn polish_people_language_follows_admin_link_and_person_navigation() {
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
                        "{}/households/persistence-fixture/admin?lang=es",
                        journey.origin
                    ),
                    None,
                )
                .await
                .unwrap();
                page.locator(
                    ".admin-action-row[href='/households/persistence-fixture/people?lang=es']",
                )
                .click(None)
                .await
                .unwrap();
                expect(page.locator("html"))
                    .to_have_attribute("lang", "es")
                    .await
                    .unwrap();
                capture(page, "people-es-desktop").await;
                page.set_viewport_size(Viewport {
                    width: 390,
                    height: 844,
                })
                .await
                .unwrap();
                capture(page, "people-es-mobile").await;
                page.locator("[data-person-card='73001'] h2 a")
                    .click(None)
                    .await
                    .unwrap();
                expect(page.locator("html"))
                    .to_have_attribute("lang", "es")
                    .await
                    .unwrap();
                page.locator(
                    "main a[href='/households/persistence-fixture/people/73001/edit?lang=es']",
                )
                .click(None)
                .await
                .unwrap();
                expect(page.locator("html"))
                    .to_have_attribute("lang", "es")
                    .await
                    .unwrap();
                expect(page.locator("#person-form input[name=lang]"))
                    .to_have_value("es")
                    .await
                    .unwrap();
                page.locator(
                    "#person-form a[href='/households/persistence-fixture/people/73001?lang=es']",
                )
                .click(None)
                .await
                .unwrap();
                expect(page.locator("html"))
                    .to_have_attribute("lang", "es")
                    .await
                    .unwrap();
                page.locator("main a[href='/households/persistence-fixture/people?lang=es']")
                    .click(None)
                    .await
                    .unwrap();
                expect(page.locator("html"))
                    .to_have_attribute("lang", "es")
                    .await
                    .unwrap();
            })
        },
    )
    .await;
}
