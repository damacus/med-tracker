use crate::harness;
use playwright_rs::{
    expect,
    protocol::{Animations, ScreenshotOptions, Viewport},
};

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn dashboard_cancellation_restores_focus_and_survives_refresh() {
    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| {
        Box::pin(async move {
            harness::sign_in(journey).await;
            let page = &journey.page;
            let directory = std::path::Path::new("docs/screenshots/dashboard-cancellation");
            std::fs::create_dir_all(directory).unwrap();
            for (size, width, height) in [("desktop", 1440, 1000), ("mobile", 390, 844)] {
                page.set_viewport_size(Viewport { width, height }).await.unwrap();
                for cancel in ["escape", "button"] {
                    page.goto(&format!("{}/households/persistence-fixture/dashboard?dashboard_person_id=73001&dashboard_grouping=time", journey.origin), None).await.unwrap();
                    expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
                    let action = page.locator("#dashboard a[href*='record_source_id=81001']");
                    page.locator("#dashboard details:has(a[href*='record_source_id=81001']) > summary").press("Enter", None).await.unwrap();
                    expect(action.clone()).to_be_visible().await.unwrap();
                    action.press("Enter", None).await.unwrap();
                    let dialog = page.locator("#record-dose-dialog");
                    expect(dialog.clone()).to_be_visible().await.unwrap();
                    expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
                    page.evaluate::<(), ()>("async () => { await document.fonts.ready; }", None).await.unwrap();
                    page.evaluate::<(), ()>("() => { const url = new URL(location.href); url.searchParams.append('tag', 'one'); url.searchParams.append('tag', 'two'); url.hash = 'routine'; history.replaceState({ retained: true }, '', url); }", None).await.unwrap();
                    page.screenshot_to_file(&directory.join(format!("{size}-{cancel}-open.png")), ScreenshotOptions::builder().animations(Animations::Disabled).build()).await.unwrap();
                    if cancel == "escape" {
                        page.keyboard().press("Escape", None).await.unwrap();
                    } else {
                        dialog.locator("[data-dialog-close]").press("Enter", None).await.unwrap();
                    }
                    expect(dialog).to_be_hidden().await.unwrap();
                    expect(page.locator("#dashboard a[href*='record_source_id=81001']")).to_be_focused().await.unwrap();
                    let visible: bool = page.evaluate("() => { const rect = document.activeElement.getBoundingClientRect(); return rect.top >= 60 && rect.bottom <= innerHeight - 90 && document.elementFromPoint(rect.x + rect.width / 2, rect.y + rect.height / 2) === document.activeElement; }", None::<&()>).await.unwrap();
                    assert!(visible, "restored Take action must be visible above the mobile navigation");
                    let state: serde_json::Value = page.evaluate("() => ({ url: location.href, retained: history.state.retained })", None::<&()>).await.unwrap();
                    assert_eq!(state["url"], format!("{}/households/persistence-fixture/dashboard?dashboard_person_id=73001&dashboard_grouping=time&tag=one&tag=two#routine", journey.origin));
                    assert_eq!(state["retained"], true);
                    page.screenshot_to_file(&directory.join(format!("{size}-{cancel}-closed.png")), ScreenshotOptions::builder().animations(Animations::Disabled).build()).await.unwrap();
                    page.reload(None).await.unwrap();
                    expect(page.locator("#dashboard")).to_be_visible().await.unwrap();
                    assert_eq!(page.locator("#record-dose-dialog[open]").count().await.unwrap(), 0);
                    assert!(page.url().contains("dashboard_person_id=73001&dashboard_grouping=time"));
                }
            }
        })
    }).await;
}
