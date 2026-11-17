use crate::harness;
use playwright_rs::{
    expect,
    protocol::{Animations, ScreenshotOptions, Viewport},
};

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn dashboard_timeline_accessibility_and_reflow() {
    harness::journey(Viewport { width: 1280, height: 1000 }, |journey| {
        Box::pin(async move {
            crate::dashboard_timeline::seed_medication(journey, 80007, "Synthetic medicine with a long formulation name and a clearly readable strength").await;
            let times = ["00:00", "12:00", "23:59"].map(str::to_string);
            crate::dashboard_timeline::seed_schedule(journey, 80007, 73001, &times).await;
            harness::sign_in(journey).await;
            let page = &journey.page;
            let directory = std::path::Path::new("docs/screenshots/dashboard-timeline");
            std::fs::create_dir_all(directory).unwrap();
            let engine = std::fs::read_to_string("node_modules/axe-core/axe.min.js").unwrap();
            for (size, width, height) in [("desktop", 1280, 1000), ("mobile", 390, 844), ("reflow", 320, 700)] {
                page.set_viewport_size(Viewport { width, height }).await.unwrap();
                page.goto(&format!("{}/households/persistence-fixture/dashboard?dashboard_person_id=all", journey.origin), None).await.unwrap();
                expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
                expect(page.locator("#dashboard ol[data-dashboard-timeline]")).to_be_visible().await.unwrap();
                for appearance in ["light", "dark"] {
                    harness::device_appearance(page, appearance).await;
                    page.evaluate::<(), ()>("async () => { await document.fonts.ready; }", None).await.unwrap();
                    page.wait_for_function("() => document.getAnimations().every(animation => animation.playState !== 'running' || !Number.isFinite(animation.effect?.getComputedTiming().endTime))", None).await.unwrap();
                    let fits = page.evaluate::<(), bool>("() => document.documentElement.scrollWidth <= innerWidth && [...document.querySelectorAll('#dashboard a.btn')].filter(a => a.getClientRects().length).every(a => a.getBoundingClientRect().height >= 44)", None).await.unwrap();
                    assert!(fits, "dashboard must reflow and retain touch targets at {width}px in {appearance}");
                    page.add_script_tag(Some(playwright_rs::protocol::AddScriptTagOptions::builder().content(engine.clone()).build())).await.unwrap();
                    let violations: serde_json::Value = page.evaluate("async () => { const r = await axe.run(document.querySelector('#dashboard'), {runOnly: {type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa']}}); return r.violations; }", None::<&()>).await.unwrap();
                    assert_eq!(violations, serde_json::json!([]), "dashboard accessibility violations at {width}px in {appearance}: {violations}");
                    let menu = page.locator(".dashboard-timeline-row .dashboard-row-menu").first();
                    menu.locator("summary").press("Enter", None).await.unwrap();
                    menu.evaluate::<(), ()>("async node => { await Promise.all(node.getAnimations({subtree: true}).map(animation => animation.finished.catch(() => {}))); }", None).await.unwrap();
                    let opacity: String = menu.locator(".dropdown-content").evaluate("node => getComputedStyle(node).opacity", None::<&()>).await.unwrap();
                    assert_eq!(opacity, "1", "open menu must finish its appearance animation before contrast auditing");
                    let violations: serde_json::Value = page.evaluate("async () => { const r = await axe.run(document.querySelector('#dashboard'), {runOnly: {type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa']}}); return r.violations; }", None::<&()>).await.unwrap();
                    assert_eq!(violations, serde_json::json!([]), "open secondary-menu accessibility violations at {width}px in {appearance}: {violations}");
                    page.keyboard().press("Escape", None).await.unwrap();
                    expect(menu.locator("summary")).to_be_focused().await.unwrap();
                    if size != "reflow" {
                        page.screenshot_to_file(&directory.join(format!("{size}-{appearance}.png")), ScreenshotOptions::builder().animations(Animations::Disabled).full_page(true).build()).await.unwrap();
                    }
                }
            }
            page.set_viewport_size(Viewport { width: 360, height: 800 }).await.unwrap();
            page.evaluate::<(), ()>("() => { localStorage.setItem('med-tracker-theme', 'serene-sage'); localStorage.setItem('med-tracker-appearance', 'dark'); }", None).await.unwrap();
            page.reload(None).await.unwrap();
            expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
            expect(page.locator("html")).to_have_attribute("data-theme", "serene-sage-dark").await.unwrap();
            page.add_script_tag(Some(playwright_rs::protocol::AddScriptTagOptions::builder().content(engine).build())).await.unwrap();
            let violations: serde_json::Value = page.evaluate("async () => { const r = await axe.run(document.querySelector('#dashboard'), {runOnly: {type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa']}}); return r.violations; }", None::<&()>).await.unwrap();
            assert_eq!(violations, serde_json::json!([]), "saved-palette accessibility violations: {violations}");
            page.screenshot_to_file(&directory.join("mobile-saved-theme.png"), ScreenshotOptions::builder().animations(Animations::Disabled).full_page(true).build()).await.unwrap();
        })
    }).await;
}
