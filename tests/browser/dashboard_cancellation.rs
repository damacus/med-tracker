use crate::harness;
use med_tracker::models::_entities::{medication_takes, medications, person_medications};
use playwright_rs::{
    expect,
    protocol::{Animations, ScreenshotOptions, Viewport},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, sea_query::Expr};

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn historical_dose_is_recorded_with_zero_stock() {
    harness::journey(Viewport { width: 1440, height: 1000 }, |journey| {
        Box::pin(async move {
            medications::Entity::update_many().col_expr(medications::Column::CurrentSupply, Expr::value(0))
                .filter(medications::Column::Id.eq(80001)).exec(&journey.admin).await.unwrap();
            person_medications::Entity::update_many().col_expr(person_medications::Column::MinHoursBetweenDoses, Expr::value(6))
                .filter(person_medications::Column::Id.eq(81001)).exec(&journey.admin).await.unwrap();
            harness::sign_in(journey).await;
            let page = &journey.page;
            let day = (chrono::Utc::now().date_naive() - chrono::Duration::days(2)).to_string();
            let directory = std::path::Path::new("docs/screenshots/historical-dose");
            std::fs::create_dir_all(directory).unwrap();
            for (index, clock) in ["08:52", "14:07"].iter().enumerate() {
                page.goto(&format!("{}/households/persistence-fixture/medications/80001?record_source_type=person_medication&record_source_id=81001&record_person_id=73001", journey.origin), None).await.unwrap();
                let form = page.locator("#record-dose-dialog form[data-dose-form]");
                expect(form.clone()).to_be_visible().await.unwrap();
                form.locator("[name=actual_dose_amount]").fill("1.5", None).await.unwrap();
                form.locator("[name=taken_at]").fill(&format!("{day}T{clock}"), None).await.unwrap();
                form.locator("[name=record_mode]").click(None).await.unwrap();
                if index == 0 {
                    for (name, width, height) in [("desktop", 1440, 1000), ("mobile", 390, 844)] {
                        page.set_viewport_size(Viewport { width, height }).await.unwrap();
                        page.screenshot_to_file(&directory.join(format!("{name}.png")), ScreenshotOptions::builder().animations(Animations::Disabled).build()).await.unwrap();
                    }
                }
                form.locator("button[type=submit]").click(None).await.unwrap();
                expect(page.locator("[aria-labelledby=dose-history-heading]")).to_contain_text("Recorded as a past dose").await.unwrap();
            }
            let takes = medication_takes::Entity::find().filter(medication_takes::Column::PersonMedicationId.eq(81001)).all(&journey.admin).await.unwrap();
            assert_eq!(takes.len(), 2);
            assert!(takes.iter().all(|take| take.historical_entry && take.dose_amount == Some(sea_orm::prelude::Decimal::new(15, 1))));
            let medication = medications::Entity::find_by_id(80001).one(&journey.admin).await.unwrap().unwrap();
            assert_eq!(medication.current_supply, Some(sea_orm::prelude::Decimal::ZERO));
        })
    }).await;
}

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
                    if !action.is_visible().await.unwrap() {
                        page.locator("#dashboard details:has(a[href*='record_source_id=81001']) > summary").press("Enter", None).await.unwrap();
                    }
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
