use playwright_rs::{
    expect,
    protocol::{Animations, AriaRole, Page, ScreenshotOptions, Viewport},
};

use crate::{
    accessibility_level_a::{assert_no_violations, audit},
    harness,
};

async fn visit(page: &Page, origin: &str, path: &str) {
    let expected = format!("{origin}{path}");
    let response = page.goto(&expected, None).await.unwrap().unwrap();
    assert!(response.ok(), "{path}: HTTP {}", response.status());
    assert_eq!(page.url(), expected, "unexpected redirect for {path}");
}

async fn capture(page: &Page, name: &str) {
    let directory = std::path::Path::new("docs/screenshots/accessibility-level-a");
    std::fs::create_dir_all(directory).unwrap();
    page.screenshot_to_file(
        &directory.join(format!("{name}.png")),
        ScreenshotOptions::builder()
            .animations(Animations::Disabled)
            .build(),
    )
    .await
    .unwrap();
}

async fn core_pages(viewport: Viewport, size: &'static str) {
    harness::journey(viewport, |journey| {
        Box::pin(async move {
            let page = &journey.page;
            let mut reports = Vec::new();
            for (path, name) in [
                ("/login", "login"),
                ("/create-account", "signup"),
                ("/reset-password", "recovery"),
            ] {
                visit(page, &journey.origin, path).await;
                reports.push(audit(page, &format!("{name}-{size}")).await);
            }
            harness::sign_in(journey).await;
            for (path, name) in [
                ("/households/choose?switch=1", "chooser"),
                ("/households/persistence-fixture/dashboard", "dashboard"),
                ("/households/persistence-fixture/medications", "inventory"),
                (
                    "/households/persistence-fixture/medications/new",
                    "medication-form",
                ),
                ("/households/persistence-fixture/people", "people"),
                (
                    "/households/persistence-fixture/people/73001/treatments",
                    "treatments",
                ),
                (
                    "/households/persistence-fixture/people/73001/treatments/schedules/new",
                    "schedule-form",
                ),
                ("/households/persistence-fixture/profile", "profile"),
            ] {
                visit(page, &journey.origin, path).await;
                reports.push(audit(page, &format!("{name}-{size}")).await);
                if name == "chooser" {
                    expect(page.get_by_role(
                        AriaRole::Button,
                        harness::named("Open household: Synthetic household"),
                    ))
                    .to_be_visible()
                    .await
                    .unwrap();
                    capture(page, &format!("chooser-{size}")).await;
                }
            }
            assert_no_violations(&reports);
        })
    })
    .await;
}

async fn keyboard_dialogs_and_errors(viewport: Viewport, size: &'static str) {
    let mobile = viewport.width < 1024;
    harness::journey(viewport, |journey| {
        Box::pin(async move {
            let page = &journey.page;
            visit(page, &journey.origin, "/login").await;
            page.keyboard().press("Tab", None).await.unwrap();
            expect(page.locator(".skip-link"))
                .to_be_focused()
                .await
                .unwrap();
            page.keyboard().press("Enter", None).await.unwrap();
            expect(page.locator("main")).to_be_focused().await.unwrap();
            page.get_by_label("Email address", true)
                .fill("persistence@example.test", None)
                .await
                .unwrap();
            page.get_by_label("Password", true)
                .fill("wrong-password", None)
                .await
                .unwrap();
            page.get_by_role(AriaRole::Button, harness::named("Sign in"))
                .press("Enter", None)
                .await
                .unwrap();
            expect(page.get_by_role(AriaRole::Alert, None))
                .to_be_visible()
                .await
                .unwrap();
            assert_eq!(page.url(), format!("{}/login", journey.origin));
            let mut reports = vec![audit(page, &format!("login-error-{size}")).await];
            capture(page, &format!("login-error-{size}")).await;
            harness::sign_in(journey).await;
            expect(page.locator("body[data-navigation-ready]"))
                .to_be_visible()
                .await
                .unwrap();
            assert!(page.evaluate::<(), bool>(
                r#"() => [...document.querySelectorAll('[aria-haspopup="dialog"][aria-controls]')].every(trigger => { const panel = document.getElementById(trigger.getAttribute('aria-controls')); return panel instanceof HTMLDialogElement || panel?.getAttribute('role') === 'dialog'; })"#,
                None,
            ).await.unwrap(), "a dialog trigger must reference an existing dialog");
            let search_opener = page.locator(if mobile {
                "[data-app-drawer]"
            } else {
                "[data-sidebar-search]"
            });
            search_opener.focus().await.unwrap();
            if mobile {
                page.keyboard().press("Enter", None).await.unwrap();
                expect(page.locator("#app-drawer"))
                    .to_be_visible()
                    .await
                    .unwrap();
                reports.push(audit(page, &format!("menu-open-{size}")).await);
                page.keyboard().press("Tab", None).await.unwrap();
                assert!(
                    page.evaluate::<(), bool>(
                        "() => document.activeElement.closest('#app-drawer') !== null",
                        None,
                    )
                    .await
                    .unwrap()
                );
                page.keyboard().press("Escape", None).await.unwrap();
                expect(search_opener.clone()).to_be_focused().await.unwrap();
            }
            page.keyboard().press("Control+k", None).await.unwrap();
            expect(page.locator("#global_search_query"))
                .to_be_focused()
                .await
                .unwrap();
            reports.push(audit(page, &format!("search-open-{size}")).await);
            page.keyboard().press("Escape", None).await.unwrap();
            expect(page.locator("#global_search_panel"))
                .to_be_hidden()
                .await
                .unwrap();
            expect(search_opener).to_be_focused().await.unwrap();
            visit(
                page,
                &journey.origin,
                "/households/persistence-fixture/medications/80001",
            )
            .await;
            reports.push(audit(page, &format!("medication-detail-{size}")).await);
            let dose = page.locator("[data-dialog-open='record-dose-dialog']");
            dose.focus().await.unwrap();
            page.keyboard().press("Enter", None).await.unwrap();
            expect(page.locator("#record-dose-dialog"))
                .to_be_visible()
                .await
                .unwrap();
            reports.push(audit(page, &format!("dose-dialog-{size}")).await);
            capture(page, &format!("dose-dialog-{size}")).await;
            page.keyboard().press("Escape", None).await.unwrap();
            expect(page.locator("#record-dose-dialog"))
                .to_be_hidden()
                .await
                .unwrap();
            expect(dose).to_be_focused().await.unwrap();
            assert_no_violations(&reports);
        })
    })
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn accessibility_level_a_core_pages_desktop() {
    core_pages(
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
async fn accessibility_level_a_core_pages_mobile() {
    core_pages(
        Viewport {
            width: 390,
            height: 844,
        },
        "mobile",
    )
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn accessibility_level_a_keyboard_desktop() {
    keyboard_dialogs_and_errors(
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
async fn accessibility_level_a_keyboard_mobile() {
    keyboard_dialogs_and_errors(
        Viewport {
            width: 390,
            height: 844,
        },
        "mobile",
    )
    .await;
}
