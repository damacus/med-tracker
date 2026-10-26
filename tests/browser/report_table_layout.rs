use crate::harness;
use playwright_rs::{expect, protocol::Viewport};

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn report_outcomes_keep_dates_readable_and_scroll_by_keyboard() {
    harness::journey(Viewport { width: 390, height: 844 }, |journey| {
        Box::pin(async move {
            harness::sign_in(journey).await;
            let page = &journey.page;
            page.goto(&format!("{}/households/persistence-fixture/reports", journey.origin), None).await.unwrap();
            expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
            for (size, width, height) in [("mobile",390,844), ("narrow",320,844), ("desktop",1440,1000)] {
                page.set_viewport_size(Viewport { width, height }).await.unwrap();
                let readable: bool = page.evaluate(r#"() => {
                    const cells = [...document.querySelectorAll('[aria-labelledby="daily-title"] tbody th')];
                    return cells.every(cell => {
                        const range = document.createRange();
                        range.selectNodeContents(cell);
                        return range.getBoundingClientRect().height <= parseFloat(getComputedStyle(cell).lineHeight) + 1;
                    }) && document.documentElement.scrollWidth <= innerWidth;
                }"#, None::<&()>).await.unwrap();
                assert!(readable, "dates must stay on one line without widening the page at {size}");
                let table = page.locator("[aria-labelledby='daily-title'] [role='region']");
                table.press("ArrowRight", None).await.unwrap();
                let keyboard_access: bool = page.evaluate(r#"async () => {
                    const region = document.querySelector('[aria-labelledby="daily-title"] [role="region"]');
                    const deadline = performance.now() + 1000;
                    while (region.scrollWidth > region.clientWidth && region.scrollLeft === 0 && performance.now() < deadline) {
                        await new Promise(requestAnimationFrame);
                    }
                    return document.activeElement === region && (region.scrollWidth <= region.clientWidth || region.scrollLeft > 0);
                }"#, None::<&()>).await.unwrap();
                assert!(keyboard_access, "outcomes must be keyboard-scrollable at {size}");
                let restrained_focus: bool = page.evaluate(r#"() => {
                    const style = getComputedStyle(document.activeElement);
                    return style.outlineStyle === 'none' && style.boxShadow !== 'none';
                }"#, None::<&()>).await.unwrap();
                assert!(restrained_focus, "keyboard focus must remain visible without framing the whole table at {size}");
                page.evaluate::<(), bool>("() => { document.querySelector('[aria-labelledby=\"daily-title\"] [role=\"region\"]').scrollLeft = 0; document.getElementById('daily-title').scrollIntoView({block:'center'}); return true; }", None).await.unwrap();
                harness::capture(page, &format!("report-outcomes-{size}")).await;
            }
        })
    }).await;
}
