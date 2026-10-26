use med_tracker::models::_entities::{medication_takes, schedules};
use playwright_rs::{
    expect,
    protocol::{AriaRole, Viewport},
};
use sea_orm::{ActiveModelTrait, Set};
use serde_json::{Value, json};

use crate::harness::{self, named};

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
                page.evaluate::<(), bool>("() => { document.querySelector('[aria-labelledby=\"daily-title\"] [role=\"region\"]').scrollLeft = 0; document.getElementById('daily-title').scrollIntoView({block:'center'}); return true; }", None).await.unwrap();
                harness::capture(page, &format!("report-outcomes-{size}")).await;
            }
        })
    }).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn report_clarity_distinguishes_empty_days_from_completed_days() {
    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| {
        Box::pin(async move {
            let day = chrono::NaiveDate::from_ymd_opt(2026, 7, 2).unwrap();
            let now = day.and_hms_opt(7, 0, 0).unwrap();
            let schedule = schedules::ActiveModel {
                household_id: Set(72001), person_id: Set(73001), medication_id: Set(80001),
                active: Set(true), dose_amount: Set(Some(sea_orm::prelude::Decimal::new(2, 0))),
                dose_unit: Set(Some("tablet".into())), dose_cycle: Set(Some(0)),
                max_daily_doses: Set(Some(1)), schedule_type: Set(0),
                schedule_config: Set(json!({"times":["08:00"]})),
                start_date: Set(Some(day)), end_date: Set(Some(day)),
                portable_id: Set(uuid::Uuid::new_v4().to_string()),
                created_at: Set(now), updated_at: Set(now), ..Default::default()
            }.insert(&journey.admin).await.unwrap();
            medication_takes::ActiveModel {
                household_id: Set(72001), schedule_id: Set(Some(schedule.id)),
                dose_amount: Set(Some(sea_orm::prelude::Decimal::new(2, 0))),
                dose_unit: Set(Some("tablet".into())), taken_at: Set(Some(now)),
                taken_from_medication_id: Set(Some(80001)),
                portable_id: Set(uuid::Uuid::new_v4().to_string()),
                created_at: Set(now), updated_at: Set(now), ..Default::default()
            }.insert(&journey.admin).await.unwrap();
            harness::sign_in(journey).await;
            let page = &journey.page;
            page.goto(&format!("{}/households/persistence-fixture/reports?person_id=73001&start_date=2026-07-01&end_date=2026-07-02", journey.origin), None).await.unwrap();
            expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
            let rows: Value = page.evaluate("() => [...document.querySelectorAll('section[aria-labelledby=\"daily-title\"] tbody tr')].map(row => [...row.children].map(cell => cell.textContent.trim()))", None::<&()>).await.unwrap();
            assert_eq!(rows, json!([["2026-07-01","0","0","0","0"],["2026-07-02","1","1","0","0"]]));
            let summary = page.locator("section[aria-label='Report summary']");
            expect(summary.clone()).to_contain_text("100%").await.unwrap();
            expect(summary).to_contain_text("1/1").await.unwrap();
            let display: Value = page.evaluate(r#"() => {
                const summary = document.querySelector('section[aria-label="Report summary"]').textContent;
                const chart = document.querySelector('[aria-labelledby="timeline-title"] [role="img"]');
                const description = document.getElementById(chart.getAttribute('aria-describedby'));
                const marker = chart.querySelector('[data-testid="report-no-doses-marker"]');
                return {
                    periodLabels: summary.includes('Period compliance') && summary.includes('Doses logged in period'),
                    emptyMarkers: chart.querySelectorAll('[data-testid="report-no-doses-marker"]').length,
                    completedBars: chart.querySelectorAll('[data-testid="report-compliance-bar"]').length,
                    emptyMeaning: Boolean(marker?.title.includes('2026-07-01: No doses expected')),
                    described: Boolean(description && !chart.contains(description) && description.textContent.includes('2026-07-01: No doses expected') && description.textContent.includes('2026-07-02: 100%'))
                };
            }"#, None::<&()>).await.unwrap();
            assert_eq!(display, json!({"periodLabels":true,"emptyMarkers":1,"completedBars":1,"emptyMeaning":true,"described":true}));
            for appearance in ["light", "dark"] {
                page.set_viewport_size(Viewport { width: 1440, height: 1000 }).await.unwrap();
                page.locator("[data-sidebar-account]").click(None).await.unwrap();
                page.get_by_role(AriaRole::Button, named("Appearance")).click(None).await.unwrap();
                page.locator(format!("[data-appearance-choice='{appearance}']")).click(None).await.unwrap();
                page.keyboard().press("Escape", None).await.unwrap();
                for (size, width, height) in [("desktop",1440,1000),("mobile",390,844)] {
                    page.set_viewport_size(Viewport { width, height }).await.unwrap();
                    if appearance == "light" {
                        page.evaluate::<(), bool>("() => { document.querySelector('section[aria-label=\"Report summary\"]').scrollIntoView({block:'center'}); return true; }", None).await.unwrap();
                        harness::capture(page, &format!("report-period-summary-{size}")).await;
                    }
                    page.evaluate::<(), bool>("() => { document.getElementById('timeline-title').scrollIntoView({block:'center'}); return true; }", None).await.unwrap();
                    harness::capture(page, &format!("report-empty-day-{appearance}-{size}")).await;
                    assert!(!page.evaluate::<(), bool>("() => document.documentElement.scrollWidth > innerWidth", None).await.unwrap());
                }
            }
        })
    }).await;
}
