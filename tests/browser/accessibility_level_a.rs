use playwright_rs::{
    expect,
    protocol::{Animations, AriaRole, Page, ScreenshotOptions, Viewport},
};
use serde_json::Value;

use crate::harness;

pub async fn audit(page: &Page, name: &str) -> Value {
    if page.locator("body.app-household").count().await.unwrap() > 0 {
        expect(page.locator("body[data-navigation-ready]"))
            .to_be_visible()
            .await
            .unwrap();
    }
    let engine = std::fs::read_to_string("node_modules/axe-core/axe.min.js").unwrap();
    page.add_script_tag(Some(
        playwright_rs::protocol::AddScriptTagOptions::builder()
            .content(engine)
            .build(),
    ))
    .await
    .unwrap();
    let report: Value = page.evaluate(
        "async () => { const r = await axe.run(document, {runOnly: {type: 'tag', values: ['wcag2a', 'wcag21a', 'wcag22a']}}); return {url: location.pathname, violations: r.violations, incomplete: r.incomplete}; }",
        None::<&()>,
    ).await.unwrap();
    let directory = std::path::Path::new("tmp/accessibility-level-a");
    std::fs::create_dir_all(directory).unwrap();
    std::fs::write(
        directory.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    std::fs::write(
        directory.join(format!("{name}.aria.yml")),
        page.locator("body").aria_snapshot(None).await.unwrap(),
    )
    .unwrap();
    report
}

pub fn assert_no_violations(reports: &[Value]) {
    let failures: Vec<_> = reports
        .iter()
        .filter(|report| !report["violations"].as_array().unwrap().is_empty())
        .collect();
    assert!(
        failures.is_empty(),
        "Level A violations: {}",
        serde_json::to_string_pretty(&failures).unwrap()
    );
}

async fn navigation(viewport: Viewport, size: &'static str) {
    let desktop = viewport.width >= 1024;
    harness::journey(viewport, |journey| Box::pin(async move {
        harness::sign_in(journey).await;
        let page = &journey.page;
        let report = audit(page, &format!("navigation-{size}")).await;
        expect(page.get_by_role(AriaRole::Button, harness::named("SA Synthetic adult My Account"))).to_be_visible().await.unwrap();
        let directory = std::path::Path::new("docs/screenshots/accessibility-level-a");
        std::fs::create_dir_all(directory).unwrap();
        page.screenshot_to_file(&directory.join(format!("navigation-{size}.png")), ScreenshotOptions::builder().animations(Animations::Disabled).build()).await.unwrap();
        let trigger = page.locator(if desktop { "[data-sidebar-account]" } else { "[data-app-account]" });
        trigger.focus().await.unwrap();
        page.keyboard().press("Enter", None).await.unwrap();
        expect(page.get_by_role(AriaRole::Dialog, harness::named("My Account"))).to_be_visible().await.unwrap();
        page.keyboard().press("Escape", None).await.unwrap();
        expect(trigger).to_be_focused().await.unwrap();
        assert_no_violations(&[report]);
        page.evaluate::<(), bool>("() => { const button = document.createElement('button'); button.id = 'a11y-negative-control'; button.textContent = 'Visible audit label'; button.setAttribute('aria-label', 'Different accessible name'); document.querySelector('main').append(button); return true; }", None).await.unwrap();
        let negative = audit(page, &format!("negative-control-{size}")).await;
        assert!(negative["violations"].as_array().unwrap().iter().any(|violation| violation["id"] == "label-content-name-mismatch"), "audit failed to detect a visible-label mismatch");
    })).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn accessibility_level_a_navigation_desktop() {
    navigation(
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
async fn accessibility_level_a_navigation_mobile() {
    navigation(
        Viewport {
            width: 390,
            height: 844,
        },
        "mobile",
    )
    .await;
}
