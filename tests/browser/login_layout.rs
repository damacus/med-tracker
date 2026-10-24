use playwright_rs::{
    expect,
    protocol::{Animations, AriaRole, ScreenshotOptions, Viewport},
};
use serde_json::{Value, json};

use crate::{
    accessibility_level_a::{assert_no_violations, audit},
    harness,
};

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn login_images_follow_viewport_and_selected_appearance() {
    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| {
        Box::pin(async move {
            let page = &journey.page;
            page.goto(&format!("{}/login", journey.origin), None).await.unwrap();
            page.evaluate::<(), ()>("() => localStorage.setItem('med-tracker-appearance', 'dark')", None).await.unwrap();
            page.reload(None).await.unwrap();
            let saved_source: String = page.evaluate(r#"async () => {
                const image = document.querySelector('[data-login-illustration] img');
                await image.decode();
                return image.currentSrc;
            }"#, None::<&()>).await.unwrap();
            assert!(saved_source.ends_with("login-med-illustration-dark-desktop.webp"));
            let directory = std::path::Path::new("docs/screenshots/login-reference");
            std::fs::create_dir_all(directory).unwrap();
            let mut observations = Vec::new();
            let mut audits = Vec::new();
            for (width, height) in [(1440, 1000), (768, 1000), (390, 844), (375, 844), (320, 844)] {
                page.set_viewport_size(Viewport { width, height }).await.unwrap();
                for appearance in ["light", "dark"] {
                    page.get_by_role(AriaRole::Button, harness::named("Appearance"))
                        .click(None).await.unwrap();
                    page.locator(format!("[data-appearance-choice='{appearance}']"))
                        .click(None).await.unwrap();
                    page.keyboard().press("Escape", None).await.unwrap();
                    let observation: Value = page.evaluate(r#"async () => {
                        const image = document.querySelector('[data-login-illustration] img');
                        if (image) await image.decode();
                        const brand = document.querySelector('.login-brand')?.getBoundingClientRect();
                        const form = document.querySelector('.login-form-panel')?.getBoundingClientRect();
                        const card = getComputedStyle(document.querySelector('.shell-card'));
                        return {
                            source: image?.currentSrc ?? '',
                            decorative: image?.alt === '',
                            loaded: Boolean(image?.naturalWidth),
                            desktop: Boolean(brand && form && brand.right <= form.left && brand.top < form.bottom && form.top < brand.bottom),
                            stacked: Boolean(brand && form && brand.bottom <= form.top),
                            fits: document.documentElement.scrollWidth <= innerWidth,
                            referenceBrand: Boolean(document.querySelector('[data-login-logo="mt"] use[href="/static/auth-icons.svg#brand"]')),
                            benefitIcons: document.querySelectorAll('[data-login-benefit-icon]').length,
                            appHeader: Boolean(document.querySelector('.shell-header')),
                            fieldHeight: document.getElementById('email').getBoundingClientRect().height,
                            actionHeight: document.querySelector('form[action="/login"] button[type="submit"]').getBoundingClientRect().height,
                            footerBelowCard: Boolean(document.querySelector('.login-footer') && document.querySelector('.login-footer').getBoundingClientRect().top >= document.querySelector('.login-surface').getBoundingClientRect().bottom),
                            card: {background: card.backgroundColor, border: card.borderColor, shadow: card.boxShadow}
                        };
                    }"#, None::<&()>).await.unwrap();
                    let size = if width <= 520 { "mobile" } else { "desktop" };
                    observations.push((width, appearance, size, observation));
                    audits.push(audit(page, &format!("login-layout-{width}-{appearance}")).await);
                    if width == 1440 || width == 375 {
                        page.evaluate::<(), ()>("async () => { await document.fonts.ready; window.scrollTo(0, 0); }", None).await.unwrap();
                        page.screenshot_to_file(
                            &directory.join(format!("{size}-{appearance}.png")),
                            ScreenshotOptions::builder().animations(Animations::Disabled).full_page(true).build(),
                        ).await.unwrap();
                    }
                }
            }
            std::fs::create_dir_all("tmp/login-layout").unwrap();
            std::fs::write("tmp/login-layout/observations.json", serde_json::to_vec_pretty(&observations).unwrap()).unwrap();
            for (width, appearance, size, observation) in observations {
                let expected = format!("/static/images/auth/login-med-illustration-{appearance}-{size}.webp");
                assert!(observation["source"].as_str().unwrap().ends_with(&expected), "{width}/{appearance}: {observation}");
                assert_eq!(observation["loaded"], true);
                assert_eq!(observation["decorative"], true);
                assert_eq!(observation["fits"], true);
                assert_eq!(observation["referenceBrand"], true, "missing original MedTracker brand at {width}/{appearance}");
                assert_eq!(observation["benefitIcons"], 4);
                assert_eq!(observation["appHeader"], false);
                assert!(observation["fieldHeight"].as_f64().unwrap() >= if width >= 768 { 56.0 } else { 48.0 });
                assert!(observation["actionHeight"].as_f64().unwrap() >= if width >= 768 { 72.0 } else { 64.0 });
                assert_eq!(observation["footerBelowCard"], true);
                assert_eq!(observation[if width >= 768 { "desktop" } else { "stacked" }], true);
            }
            assert_no_violations(&audits);
            page.get_by_role(AriaRole::Button, harness::named("Appearance")).click(None).await.unwrap();
            page.locator("[data-appearance-choice='system']").click(None).await.unwrap();
            page.keyboard().press("Escape", None).await.unwrap();
            let cdp = page.context().unwrap().new_cdp_session(page).await.unwrap();
            for appearance in ["dark", "light"] {
                cdp.send("Emulation.setEmulatedMedia", Some(json!({"features":[{"name":"prefers-color-scheme","value":appearance}]}))).await.unwrap();
                expect(page.locator("html")).to_have_attribute("data-theme", &format!("neutral-{appearance}")).await.unwrap();
                let source: String = page.evaluate(r#"async () => {
                    const image = document.querySelector('[data-login-illustration] img');
                    await image.decode();
                    return image.currentSrc;
                }"#, None::<&()>).await.unwrap();
                assert!(source.ends_with(&format!("login-med-illustration-{appearance}-mobile.webp")));
            }
            page.get_by_label("Email address", true).fill("persistence@example.test", None).await.unwrap();
            page.get_by_label("Password", true).fill("password", None).await.unwrap();
            page.get_by_role(AriaRole::Button, harness::named("Sign in")).press("Enter", None).await.unwrap();
            expect(page.get_by_role(AriaRole::Button, harness::named("Open household: Synthetic household")))
                .to_be_visible().await.unwrap();
            let security = page.get_by_role(AriaRole::Link, harness::named("Security settings"));
            expect(security.clone()).to_have_attribute("href", "/account/security").await.unwrap();
            security.press("Enter", None).await.unwrap();
            expect(page.get_by_role(AriaRole::Heading, harness::named("Account security")))
                .to_be_visible().await.unwrap();
        })
    }).await;
}
