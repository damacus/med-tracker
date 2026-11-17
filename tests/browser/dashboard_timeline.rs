use crate::harness;
use chrono::Timelike;
use med_tracker::models::_entities::{accounts, medication_takes, medications, schedules};
use med_tracker::models::entities::dose_occurrence;
use playwright_rs::{
    expect,
    protocol::{Animations, AriaRole, ScreenshotOptions, Viewport},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
    prelude::Decimal,
};

pub(super) async fn seed_schedule(
    journey: &harness::Journey,
    medication_id: i64,
    person_id: i64,
    times: &[String],
) -> schedules::Model {
    schedules::Entity::insert(schedules::ActiveModel {
        household_id: Set(72001),
        person_id: Set(person_id),
        medication_id: Set(medication_id),
        active: Set(true),
        start_date: Set(chrono::NaiveDate::from_ymd_opt(2020, 1, 1)),
        end_date: Set(chrono::NaiveDate::from_ymd_opt(2099, 12, 31)),
        dose_amount: Set(Some(Decimal::new(2, 0))),
        dose_unit: Set(Some("tablet".into())),
        schedule_type: Set(0),
        origin_time_zone: Set(Some("UTC".into())),
        schedule_config: Set(serde_json::json!({ "times": times })),
        portable_id: Set(uuid::Uuid::new_v4().to_string()),
        created_at: Set(chrono::Utc::now().naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    })
    .exec_with_returning(&journey.admin)
    .await
    .expect("seed dashboard timeline schedule")
}

pub(super) async fn seed_medication(journey: &harness::Journey, id: i64, name: &str) {
    let mut account = accounts::Entity::find_by_id(71001)
        .one(&journey.admin)
        .await
        .unwrap()
        .unwrap();
    account.preferences["time_zone"] = serde_json::json!("UTC");
    account
        .into_active_model()
        .update(&journey.admin)
        .await
        .unwrap();
    medications::Entity::insert(medications::ActiveModel {
        id: Set(id),
        household_id: Set(72001),
        location_id: Set(79001),
        name: Set(Some(name.into())),
        current_supply: Set(Some(Decimal::new(10, 0))),
        dose_amount: Set(Some(2.0)),
        dose_unit: Set(Some("tablet".into())),
        created_at: Set(chrono::Utc::now().naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    })
    .exec(&journey.admin)
    .await
    .expect("seed dashboard timeline medication");
}

fn screenshot(name: &str) -> (std::path::PathBuf, ScreenshotOptions) {
    let directory = std::path::Path::new("docs/screenshots/dashboard-timeline");
    std::fs::create_dir_all(directory).unwrap();
    (
        directory.join(name),
        ScreenshotOptions::builder()
            .animations(Animations::Disabled)
            .build(),
    )
}

fn due_before_now(times: &[String]) -> usize {
    let now = chrono::Utc::now();
    let elapsed = now.hour() as i64 * 60 + now.minute() as i64;
    times
        .iter()
        .filter(|time| {
            time.split_once(':').is_some_and(|(hour, minute)| {
                hour.parse::<i64>().unwrap_or(0) * 60 + minute.parse::<i64>().unwrap_or(0)
                    <= elapsed
            })
        })
        .count()
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn dashboard_mobile_next_action_stays_above_dock_and_preserves_dose_flow() {
    harness::journey(Viewport { width: 390, height: 844 }, |journey| {
        Box::pin(async move {
            schedules::Entity::update_many()
                .col_expr(schedules::Column::Active, sea_orm::sea_query::Expr::value(false))
                .filter(schedules::Column::HouseholdId.eq(72001))
                .filter(schedules::Column::PersonId.eq(73001))
                .exec(&journey.admin).await.unwrap();
            seed_medication(journey, 80011, "Synthetic next action medicine with a long readable formulation and strength").await;
            for _ in 0..2 {
                seed_schedule(journey, 80011, 73001, &["00:00".into()]).await;
            }
            harness::sign_in(journey).await;
            let page = &journey.page;
            let path = format!("{}/households/persistence-fixture/dashboard?dashboard_person_id=73001&tag=one&tag=two#routine", journey.origin);
            let mut first_key = String::new();
            for (size, width, height) in [("mobile", 390, 844), ("reflow", 320, 700), ("desktop", 1280, 1000)] {
                page.set_viewport_size(Viewport { width, height }).await.unwrap();
                page.goto(&path, None).await.unwrap();
                expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
                page.evaluate::<(), ()>("async () => { await document.fonts.ready; }", None).await.unwrap();
                let tray = page.locator("[data-dashboard-next-action]");
                if size == "desktop" {
                    expect(tray).to_be_hidden().await.unwrap();
                    let (file, options) = screenshot("desktop-library.png");
                    page.screenshot_to_file(&file, options).await.unwrap();
                    let panel = page.locator("[data-dashboard-responsive-panel]").first();
                    let panel_link = panel.locator("a[href]").first();
                    panel_link.focus().await.unwrap();
                    page.set_viewport_size(Viewport { width: 390, height: 844 }).await.unwrap();
                    expect(panel_link.clone()).to_be_visible().await.unwrap();
                    expect(panel_link).to_be_focused().await.unwrap();
                    let remains_open: bool = panel.evaluate("panel => panel.open", None::<&()>).await.unwrap();
                    assert!(remains_open, "resizing to mobile must not collapse a panel containing keyboard focus");
                    continue;
                }
                expect(tray.clone()).to_be_visible().await.unwrap();
                let glass: bool = tray.evaluate("tray => { const backdrop = document.querySelector('.dashboard-next-action-backdrop'); const style = getComputedStyle(backdrop); const r = backdrop.getBoundingClientRect(); const card = tray.getBoundingClientRect(); return backdrop.getAttribute('aria-hidden') === 'true' && style.pointerEvents === 'none' && (style.backdropFilter.includes('blur(') || style.webkitBackdropFilter?.includes('blur(')) && r.top < card.top && parseInt(style.zIndex) < parseInt(getComputedStyle(tray).zIndex); }", None::<&()>).await.unwrap();
                assert!(glass, "mobile tray must have a non-interactive blurred transition above the card");
                let next: serde_json::Value = page.evaluate("() => { const tray = document.querySelector('[data-dashboard-next-action]'); const link = tray.querySelector('a[href*=record_occurrence_key]'); const key = new URL(link.href).searchParams.get('record_occurrence_key'); const first = [...document.querySelectorAll('.dashboard-timeline-row[data-status=available]')].find(row => row.dataset.occurrenceKey); const visible = [...document.querySelectorAll('a[href*=record_occurrence_key]')].filter(a => a.getClientRects().length && new URL(a.href).searchParams.get('record_occurrence_key') === key); return {key, expected: first?.dataset.occurrenceKey, count: visible.length}; }", None::<&()>).await.unwrap();
                assert_eq!(next["key"], next["expected"], "tray must select the first due scheduled occurrence");
                assert_eq!(next["count"], 1, "selected occurrence must have exactly one visible record action");
                first_key = next["key"].as_str().unwrap().to_owned();
                if size == "mobile" {
                    let (file, options) = screenshot("mobile-next-action-top.png");
                    page.screenshot_to_file(&file, options).await.unwrap();
                    page.set_viewport_size(Viewport { width: 390, height: 654 }).await.unwrap();
                    let reference_fits: bool = tray.evaluate("tray => { const r = tray.getBoundingClientRect(); const dock = document.querySelector('.dock').getBoundingClientRect(); return r.top >= 60 && r.bottom <= dock.top && document.documentElement.scrollWidth <= innerWidth; }", None::<&()>).await.unwrap();
                    assert!(reference_fits, "next action must fit the reference viewport above the dock");
                    let (file, options) = screenshot("mobile-next-action-reference.png");
                    page.screenshot_to_file(&file, options).await.unwrap();
                    page.set_viewport_size(Viewport { width, height }).await.unwrap();
                }
                page.evaluate::<(), ()>("() => { window.scrollTo(0, document.documentElement.scrollHeight); }", None).await.unwrap();
                let clear: bool = tray.evaluate("tray => { const r = tray.getBoundingClientRect(); const dock = document.querySelector('.dock').getBoundingClientRect(); return r.top >= 60 && r.bottom <= dock.top && r.left >= 0 && r.right <= innerWidth && r.height > 0; }", None::<&()>).await.unwrap();
                assert!(clear, "tray must remain visible above the dock at the page end ({width}px)");
                let (file, options) = screenshot(&format!("mobile-next-action-{size}.png"));
                page.screenshot_to_file(&file, options).await.unwrap();
                for cancel in ["escape", "button"] {
                    let opener = tray.locator("a[href*='record_occurrence_key=']");
                    opener.press("Enter", None).await.unwrap();
                    let dialog = page.locator("#record-dose-dialog");
                    expect(dialog.clone()).to_be_visible().await.unwrap();
                    page.evaluate::<(), ()>("() => { const url = new URL(location.href); url.searchParams.append('tag', 'one'); url.searchParams.append('tag', 'two'); url.hash = 'routine'; history.replaceState({retained: true}, '', url); }", None).await.unwrap();
                    if cancel == "escape" { page.keyboard().press("Escape", None).await.unwrap(); }
                    else { dialog.locator("[data-dialog-close]").press("Enter", None).await.unwrap(); }
                    expect(dialog).to_be_hidden().await.unwrap();
                    expect(opener.clone()).to_be_focused().await.unwrap();
                    let restored: bool = opener.evaluate("a => { const r = a.getBoundingClientRect(); const dock = document.querySelector('.dock').getBoundingClientRect(); const url = new URL(location.href); return r.top >= 60 && r.bottom <= dock.top && url.searchParams.getAll('tag').join(',') === 'one,two' && url.hash === '#routine' && !url.searchParams.has('record_occurrence_key') && !url.searchParams.has('record_occurrence_etag') && history.state.retained === true; }", None::<&()>).await.unwrap();
                    assert!(restored, "cancel must preserve navigation state and focus the visible tray opener");
                }
                page.evaluate::<(), ()>("async () => { document.documentElement.style.fontSize = '200%'; await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))); }", None).await.unwrap();
                let reflow: serde_json::Value = tray.evaluate("tray => { const r = tray.getBoundingClientRect(); const dock = document.querySelector('.dock').getBoundingClientRect(); const props = getComputedStyle(document.querySelector('#dashboard')); return { fits: document.documentElement.scrollWidth <= innerWidth && r.top >= 0 && r.bottom <= dock.top, viewport: {width: innerWidth, height: innerHeight}, docWidth: document.documentElement.scrollWidth, trayRect: r.toJSON(), dockRect: dock.toJSON(), scrollHeight: document.documentElement.scrollHeight, clientHeight: document.documentElement.clientHeight, props: {trayHeight: props.getPropertyValue('--dashboard-tray-height'), dockHeight: props.getPropertyValue('--dashboard-dock-height')} }; }", None::<&()>).await.unwrap();
                let controls_clear: bool = tray.evaluate("tray => { const dock = document.querySelector('.dock').getBoundingClientRect(); return [...tray.querySelectorAll('a[data-dashboard-action], .dashboard-row-menu > summary')].every(control => { const r = control.getBoundingClientRect(); return r.top >= 0 && r.bottom <= dock.top && r.left >= 0 && r.right <= innerWidth && control.contains(document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)); }); }", None::<&()>).await.unwrap();
                let (file, options) = screenshot(&format!("mobile-next-action-{size}-200-percent.png"));
                page.screenshot_to_file(&file, options).await.unwrap();
                assert_eq!(reflow["fits"], true, "long next action must reflow at 200% text without dock overlap: {reflow}");
                assert!(controls_clear, "the enlarged tray actions must remain unobscured: {reflow}");
                page.evaluate::<(), ()>("async () => { document.documentElement.style.fontSize = ''; await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))); }", None).await.unwrap();
            }
            page.set_viewport_size(Viewport { width: 390, height: 844 }).await.unwrap();
            page.goto(&path, None).await.unwrap();
            for remaining in [1, 0] {
                let tray = page.locator("[data-dashboard-next-action]");
                expect(tray.clone()).to_be_visible().await.unwrap();
                let action = tray.locator("a[href*='record_occurrence_key=']");
                if remaining == 0 {
                    let key: String = action.evaluate("a => new URL(a.href).searchParams.get('record_occurrence_key')", None::<&()>).await.unwrap();
                    assert_ne!(key, first_key, "recording must advance to the next due occurrence");
                }
                action.press("Enter", None).await.unwrap();
                let form = page.locator("#record-dose-dialog form[data-dose-form]");
                expect(form.clone()).to_be_visible().await.unwrap();
                form.locator("[name=actual_dose_amount]").fill("2", None).await.unwrap();
                form.locator("button[type=submit]").press("Enter", None).await.unwrap();
                expect(page.locator("#dashboard")).to_be_visible().await.unwrap();
            }
            expect(page.locator("[data-dashboard-next-action]")).to_be_hidden().await.unwrap();
            let medicine = medications::Entity::find_by_id(80011).one(&journey.admin).await.unwrap().unwrap();
            assert_eq!(medicine.current_supply, Some(Decimal::new(6, 0)), "two dose submissions deduct stock exactly once each");
            let count = medication_takes::Entity::find().filter(medication_takes::Column::TakenFromMedicationId.eq(80011)).all(&journey.admin).await.unwrap().len();
            assert_eq!(count, 2, "cancelling must not create doses");
        })
    }).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn dashboard_compact_cards_keep_identity_and_keyboard_secondary_actions() {
    harness::journey(Viewport { width: 1280, height: 1000 }, |journey| {
        Box::pin(async move {
            use med_tracker::models::_entities::people;
            let person_name = "Alexandra Synthetic Person With A Long Readable Name";
            people::Entity::update_many()
                .col_expr(people::Column::Name, sea_orm::sea_query::Expr::value(person_name))
                .filter(people::Column::Id.eq(73001))
                .exec(&journey.admin).await.unwrap();
            seed_medication(journey, 80008, "Synthetic compact medicine").await;
            seed_schedule(journey, 80008, 73001, &["00:00".into(), "00:01".into(), "00:02".into()]).await;
            seed_medication(journey, 80009, "Synthetic medicine with a long formulation name and a clearly readable strength").await;
            seed_schedule(journey, 80009, 73001, &["23:59".into()]).await;
            seed_medication(journey, 80010, "Synthetic completed medicine").await;
            let completed = seed_schedule(journey, 80010, 73001, &["00:00".into()]).await;
            medication_takes::Entity::insert(medication_takes::ActiveModel {
                household_id: Set(72001), schedule_id: Set(Some(completed.id)),
                dose_amount: Set(Some(Decimal::new(2, 0))), dose_unit: Set(Some("tablet".into())),
                taken_at: Set(Some(chrono::Utc::now().naive_utc())),
                portable_id: Set(uuid::Uuid::new_v4().to_string()),
                created_at: Set(chrono::Utc::now().naive_utc()), updated_at: Set(chrono::Utc::now().naive_utc()),
                ..Default::default()
            }).exec(&journey.admin).await.unwrap();
            harness::sign_in(journey).await;
            let page = &journey.page;
            for (size, width, height) in [("desktop", 1280, 1000), ("mobile", 360, 800)] {
                page.set_viewport_size(Viewport { width, height }).await.unwrap();
                page.goto(&format!("{}/households/persistence-fixture/dashboard?dashboard_person_id=all", journey.origin), None).await.unwrap();
                expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
                page.evaluate::<(), ()>("async () => { await document.fonts.ready; }", None).await.unwrap();
                let compact = page.locator(".dashboard-timeline-row:has(h3:text-is('Synthetic compact medicine'))");
                assert_eq!(compact.count().await.unwrap(), 3);
                let geometry: serde_json::Value = page.evaluate("() => { const rows = [...document.querySelectorAll('.dashboard-timeline-row')].filter(row => row.querySelector('h3')?.textContent === 'Synthetic compact medicine'); return { heights: rows.map(row => row.getBoundingClientRect().height), fits: document.documentElement.scrollWidth <= innerWidth, patientLinks: [...document.querySelectorAll('.dashboard-timeline-card a[href], .dashboard-prn-card a[href]')].filter(link => /[/]people[/][0-9]+$/.test(new URL(link.href).pathname)).length, addPerson: document.querySelectorAll('#dashboard a[href$=\"/people/new\"]').length }; }", None::<&()>).await.unwrap();
                assert_eq!(geometry["fits"], true, "cards must reflow at {width}px");
                assert_eq!(geometry["patientLinks"], 0, "card identity must not navigate away");
                assert_eq!(geometry["addPerson"], 0, "dashboard must not offer Add person");
                if size == "desktop" {
                    assert!(geometry["heights"].as_array().unwrap().iter().all(|height| height.as_f64().unwrap() <= 150.0), "ordinary desktop rows must stay compact: {geometry}");
                }
                let timeline_line: serde_json::Value = page.evaluate("() => { const timeline = document.querySelector('.dashboard-timeline'); const row = timeline.querySelector('.dashboard-timeline-row .timeline-middle').getBoundingClientRect(); const axis = row.x + row.width / 2; const offsets = [...timeline.querySelectorAll('.timeline-middle, li > hr')].map(node => { const r = node.getBoundingClientRect(); return Math.abs(r.x + r.width / 2 - axis); }); return { hasNow: !!timeline.querySelector('.dashboard-timeline-now'), maxOffset: Math.max(...offsets) }; }", None::<&()>).await.unwrap();
                assert_eq!(timeline_line["hasNow"], true, "fixture must include the Now marker");
                assert!(timeline_line["maxOffset"].as_f64().unwrap() < 1.0, "all markers and vertical rails must share one timeline axis at {width}px: {timeline_line}");
                let identified: bool = compact.first().evaluate("row => [...row.querySelectorAll('[aria-label], img[alt]')].some(node => (node.getAttribute('aria-label') || node.getAttribute('alt')) === 'Alexandra Synthetic Person With A Long Readable Name')", None::<&()>).await.unwrap();
                assert!(identified, "avatar must expose the complete person name");
                let corner: bool = compact.first().evaluate("row => { const card = row.querySelector('.dashboard-timeline-card').getBoundingClientRect(); const avatar = row.querySelector('.dashboard-identity').getBoundingClientRect(); return avatar.top >= card.top && avatar.top - card.top <= 24 && avatar.right <= card.right && card.right - avatar.right <= 24; }", None::<&()>).await.unwrap();
                assert!(corner, "patient avatar must stay in the card's top-right corner at {width}px");
                let taken = page.locator(".dashboard-timeline-row:has(h3:text-is('Synthetic completed medicine'))");
                let disabled: bool = taken.evaluate("row => [...row.querySelectorAll('button')].some(button => button.disabled && /Taken/.test(button.textContent))", None::<&()>).await.unwrap();
                assert!(disabled, "completed dose must retain a disabled Taken button");
                if size == "desktop" {
                    let aligned: bool = page.evaluate("() => { const open = document.querySelector('.dashboard-timeline-row[data-status=available] [data-dashboard-action]'); const taken = document.querySelector('[data-dashboard-taken]'); return !!open && !!taken && Math.abs(open.getBoundingClientRect().right - taken.getBoundingClientRect().right) < 1; }", None::<&()>).await.unwrap();
                    assert!(aligned, "Taken must replace Take in the same desktop action position");
                } else {
                    expect(page.locator("[data-dashboard-next-action]")).to_be_visible().await.unwrap();
                }
                let menu = compact.first().locator(".dashboard-row-menu");
                let summary = menu.locator("summary");
                summary.press("Enter", None).await.unwrap();
                expect(menu.get_by_role(AriaRole::Link, Some(playwright_rs::protocol::GetByRoleOptions::default().name("More details").exact(false)))).to_be_visible().await.unwrap();
                let skip = menu.get_by_role(AriaRole::Link, Some(playwright_rs::protocol::GetByRoleOptions::default().name("Skip").exact(false)));
                expect(skip.clone()).to_be_visible().await.unwrap();
                menu.evaluate::<(), ()>("async node => { await Promise.all(node.getAnimations({subtree: true}).map(animation => animation.finished.catch(() => {}))); }", None).await.unwrap();
                let opacity: String = menu.locator(".dropdown-content").evaluate("node => getComputedStyle(node).opacity", None::<&()>).await.unwrap();
                assert_eq!(opacity, "1", "open menu must finish its appearance animation before contrast auditing");
                page.add_script_tag(Some(playwright_rs::protocol::AddScriptTagOptions::builder().content(std::fs::read_to_string("node_modules/axe-core/axe.min.js").unwrap()).build())).await.unwrap();
                let violations: serde_json::Value = page.evaluate("async () => { const r = await axe.run(document.querySelector('#dashboard'), {runOnly: {type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa']}}); return r.violations; }", None::<&()>).await.unwrap();
                assert_eq!(violations, serde_json::json!([]), "open menu accessibility at {width}px: {violations}");
                let (path, options) = screenshot(&format!("compact-{size}-menu.png"));
                page.screenshot_to_file(&path, options).await.unwrap();
                skip.focus().await.unwrap();
                page.keyboard().press("Escape", None).await.unwrap();
                expect(summary.clone()).to_be_focused().await.unwrap();
                let visible: bool = summary.evaluate("node => { const r = node.getBoundingClientRect(); return !node.parentElement.open && r.top >= 60 && r.bottom <= innerHeight - 90 && node.contains(document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)); }", None::<&()>).await.unwrap();
                assert!(visible, "Escape must close the row menu and restore unobscured focus at {width}px");
                let (path, options) = screenshot(&format!("compact-{size}.png"));
                page.screenshot_to_file(&path, options).await.unwrap();
            }
        })
    }).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn dashboard_timeline_lists_every_today_occurrence_once_in_order() {
    harness::journey(Viewport { width: 1280, height: 900 }, |journey| {
        Box::pin(async move {
            seed_medication(journey, 80003, "Synthetic timeline medicine").await;
            let times = ["00:00", "12:00", "23:59"].map(str::to_string);
            let scheduled = seed_schedule(journey, 80003, 73001, &times).await;
            medication_takes::Entity::insert(medication_takes::ActiveModel {
                household_id: Set(72001),
                schedule_id: Set(Some(scheduled.id)),
                dose_amount: Set(Some(Decimal::new(3, 0))),
                dose_unit: Set(Some("tablet".into())),
                taken_at: Set(Some(chrono::Utc::now().naive_utc())),
                portable_id: Set(uuid::Uuid::new_v4().to_string()),
                created_at: Set(chrono::Utc::now().naive_utc()),
                updated_at: Set(chrono::Utc::now().naive_utc()),
                ..Default::default()
            })
            .exec(&journey.admin)
            .await
            .unwrap();
            harness::sign_in(journey).await;
            let page = &journey.page;
            page.goto(&format!("{}/households/persistence-fixture/dashboard", journey.origin), None).await.unwrap();
            expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();

            let timeline: serde_json::Value = page.evaluate(
                "() => {
                    const rows = [...document.querySelectorAll('#dashboard [data-dashboard-task]')];
                    const links = [...document.querySelectorAll('#dashboard a[href*=\"record_occurrence_key=\"]')].filter(anchor => anchor.getClientRects().length).map(anchor => {
                        const url = new URL(anchor.href);
                        return {
                            key: url.searchParams.get('record_occurrence_key'),
                            source: url.searchParams.get('record_source_id'),
                            kind: url.searchParams.get('record_source_type'),
                            person: url.searchParams.get('record_person_id'),
                        };
                    });
                    return {
                        rows: rows.length,
                        slots: rows.filter(row => row.textContent.includes('Synthetic timeline medicine')).length,
                        times: rows
                            .filter(row => row.textContent.includes('Synthetic timeline medicine'))
                            .map(row => (row.querySelector('time') || {}).textContent || null),
                        links,
                    };
                }",
                None::<&()>,
            ).await.unwrap();
            let slots = timeline["slots"].as_u64().unwrap() as usize;
            assert_eq!(slots, 3, "each of the three daily slots must render once: {timeline}");
            let links: Vec<_> = timeline["links"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|link| link["source"].as_str() == Some(scheduled.id.to_string().as_str()))
                .collect();
            let keys: std::collections::HashSet<_> =
                links.iter().filter_map(|link| link["key"].as_str()).collect();
            assert_eq!(links.len(), keys.len(), "occurrence keys must be unique per slot: {links:?}");
            assert!(
                links.iter().all(|link| link["kind"] == "schedule" && link["person"] == "73001"),
                "default view is the signed-in person's timeline: {links:?}"
            );
            let due = due_before_now(&times);
            assert!(
                links.len() <= due,
                "future occurrences stay visible but cannot be recorded yet: {links:?}"
            );
            let rendered: Vec<_> = timeline["times"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|time| time.as_str().map(str::to_owned))
                .collect();
            assert_eq!(rendered, times.to_vec(), "every scheduled time must be present in chronological order: {timeline}");

            expect(page.locator("[data-dashboard-taken]:disabled").first()).to_be_visible().await.unwrap();
            expect(page.get_by_text("As needed", true).first()).to_be_visible().await.unwrap();
            expect(page.locator("#dashboard a[href*='record_source_type=person_medication']")).to_be_visible().await.unwrap();
            assert_eq!(
                page.locator("#dashboard a[href*='dashboard_date=']").count().await.unwrap(),
                0,
                "the timeline must not offer date navigation"
            );
            expect(page.locator("#dashboard-insights-title")).to_be_visible().await.unwrap();
            expect(page.locator("#dashboard .dashboard-stock").get_by_role(AriaRole::Link, harness::named("Inventory"))).to_be_visible().await.unwrap();

            for (size, viewport) in [("desktop", Viewport { width: 1280, height: 900 }), ("mobile", Viewport { width: 360, height: 800 })] {
                page.set_viewport_size(viewport).await.unwrap();
                let overflow: bool = page.evaluate(
                    "() => document.documentElement.scrollWidth > document.documentElement.clientWidth",
                    None::<&()>,
                ).await.unwrap();
                assert!(!overflow, "horizontal overflow at {size} viewport");
                let (path, options) = screenshot(&format!("timeline-{size}.png"));
                page.screenshot_to_file(&path, options).await.unwrap();
            }

            page.goto(&format!("{}/households/persistence-fixture/dashboard?dashboard_person_id=all", journey.origin), None).await.unwrap();
            expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
            let (path, options) = screenshot("timeline-everyone.png");
            page.screenshot_to_file(&path, options).await.unwrap();
        })
    }).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn dashboard_timeline_records_a_dose_against_its_occurrence() {
    harness::journey(
        Viewport {
            width: 1280,
            height: 900,
        },
        |journey| {
            Box::pin(async move {
                seed_medication(journey, 80004, "Synthetic recorded medicine").await;
                let times = ["00:00", "23:59"].map(str::to_string);
                let scheduled = seed_schedule(journey, 80004, 73001, &times).await;
                harness::sign_in(journey).await;
                let page = &journey.page;
                page.goto(
                    &format!(
                        "{}/households/persistence-fixture/dashboard",
                        journey.origin
                    ),
                    None,
                )
                .await
                .unwrap();
                expect(page.locator("body[data-navigation-ready]"))
                    .to_be_visible()
                    .await
                    .unwrap();
                let action = page
                    .locator(format!(
                        "#dashboard a[href*='record_source_id={}'][href*='record_occurrence_key=']:visible",
                        scheduled.id
                    ))
                    .first();
                action.press("Enter", None).await.unwrap();
                let dialog = page.locator("#record-dose-dialog");
                let form = dialog.locator("form[data-dose-form]");
                expect(form.clone()).to_be_visible().await.unwrap();
                let (path, options) = screenshot("record-dialog.png");
                page.screenshot_to_file(&path, options).await.unwrap();
                form.locator("[name=actual_dose_amount]")
                    .fill("3", None)
                    .await
                    .unwrap();
                form.locator("button[type=submit]")
                    .click(None)
                    .await
                    .unwrap();
                expect(page.locator("#dashboard"))
                    .to_be_visible()
                    .await
                    .unwrap();

                let takes = medication_takes::Entity::find()
                    .filter(medication_takes::Column::ScheduleId.eq(scheduled.id))
                    .all(&journey.admin)
                    .await
                    .unwrap();
                assert_eq!(
                    takes.len(),
                    1,
                    "one dose recorded against the schedule: {takes:?}"
                );
                assert_eq!(
                    takes[0].dose_amount,
                    Some(Decimal::new(3, 0)),
                    "the actual amount given is preserved"
                );
                assert!(!takes[0].historical_entry);
                let medication = medications::Entity::find_by_id(80004)
                    .one(&journey.admin)
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    medication.current_supply,
                    Some(Decimal::new(7, 0)),
                    "stock is deducted once"
                );

                expect(
                    page.locator(".dashboard-timeline-row[data-status=taken] .dashboard-row-content > p")
                        .first(),
                )
                .to_contain_text("3 tablets")
                .await
                .unwrap();
                assert_eq!(
                    page.locator(format!(
                        "#dashboard a[href*='record_source_id={}'][href*='record_occurrence_key=']:visible",
                        scheduled.id
                    ))
                    .count()
                    .await
                    .unwrap(),
                    due_before_now(&times) - 1,
                    "only the still-open due occurrences keep record actions"
                );
                let (path, options) = screenshot("recorded.png");
                page.screenshot_to_file(&path, options).await.unwrap();
            })
        },
    )
    .await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn dashboard_timeline_cancelling_record_dialog_restores_occurrence_focus() {
    harness::journey(Viewport { width: 1280, height: 900 }, |journey| {
        Box::pin(async move {
            seed_medication(journey, 80005, "Synthetic focus medicine").await;
            let mut source = seed_schedule(journey, 80005, 73001, &[]).await.into_active_model();
            source.max_daily_doses = Set(Some(2));
            let scheduled = source.update(&journey.admin).await.unwrap();
            harness::sign_in(journey).await;
            let page = &journey.page;
            let selector = format!(
                "#dashboard a[href*='record_source_id={}'][href*='record_occurrence_key=']:visible",
                scheduled.id
            );
            for (size, width, height) in [("desktop", 1280, 900)] {
                page.set_viewport_size(Viewport { width, height }).await.unwrap();
                for cancel in ["escape", "button"] {
                    page.goto(&format!("{}/households/persistence-fixture/dashboard", journey.origin), None).await.unwrap();
                    expect(page.locator("body[data-navigation-ready]")).to_be_visible().await.unwrap();
                    let links = page.locator(&selector);
                    assert_eq!(links.count().await.unwrap(), 2, "both untimed daily slots must be available");
                    let action = links.nth(1);
                    let href: String = action.evaluate("anchor => anchor.href", None::<&()>).await.unwrap();
                    action.press("Enter", None).await.unwrap();
                    let dialog = page.locator("#record-dose-dialog");
                    expect(dialog.clone()).to_be_visible().await.unwrap();
                    if cancel == "escape" {
                        page.keyboard().press("Escape", None).await.unwrap();
                    } else {
                        dialog.locator("[data-dialog-close]").press("Enter", None).await.unwrap();
                    }
                    expect(dialog).to_be_hidden().await.unwrap();
                    let focused: String = page.evaluate("() => document.activeElement instanceof HTMLAnchorElement ? document.activeElement.href : ''", None::<&()>).await.unwrap();
                    assert_eq!(focused, href, "focus must return to the second occurrence's action");
                    let clear: bool = page.evaluate("() => { const a = document.activeElement; const r = a.getBoundingClientRect(); return r.top >= 60 && r.bottom <= innerHeight - 90 && document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2) === a; }", None::<&()>).await.unwrap();
                    assert!(clear, "the restored occurrence action must be clear of fixed navigation");
                    let (path, options) = screenshot(&format!("focus-{size}-{cancel}.png"));
                    page.screenshot_to_file(&path, options).await.unwrap();
                }
            }
        })
    }).await;
}

#[tokio::test]
#[ignore = "browser journeys run through task test:browser"]
async fn dashboard_timeline_not_taken_targets_its_exact_occurrence() {
    harness::journey(
        Viewport {
            width: 360,
            height: 800,
        },
        |journey| {
            Box::pin(async move {
                seed_medication(journey, 80006, "Synthetic outcome medicine").await;
                let times = ["00:00", "23:59"].map(str::to_string);
                let source = seed_schedule(journey, 80006, 73001, &times).await;
                harness::sign_in(journey).await;
                let page = &journey.page;
                page.goto(
                    &format!(
                        "{}/households/persistence-fixture/dashboard",
                        journey.origin
                    ),
                    None,
                )
                .await
                .unwrap();
                expect(page.locator("body[data-navigation-ready]"))
                    .to_be_visible()
                    .await
                    .unwrap();
                let menu = page.locator(format!(
                    ".dashboard-timeline-row:has(a[href*='/schedules/{}/doses?'][href*='#dose-occurrence-']) .dashboard-row-menu",
                    source.id
                )).first();
                menu.locator("summary").press("Enter", None).await.unwrap();
                let action = menu.get_by_role(AriaRole::Link, Some(playwright_rs::protocol::GetByRoleOptions::default().name("Skip").exact(true)));
                action.press("Enter", None).await.unwrap();
                let target = page.locator("article:target");
                expect(target.clone()).to_be_visible().await.unwrap();
                expect(target.clone()).to_be_focused().await.unwrap();
                let clear: bool = target
                    .evaluate(
                        "article => article.getBoundingClientRect().top >= 60",
                        None::<&()>,
                    )
                    .await
                    .unwrap();
                assert!(
                    clear,
                    "the selected outcome must be clear of the fixed header"
                );
                target
                    .locator("button[data-dialog-open^='dose-not_taken-']")
                    .press("Enter", None)
                    .await
                    .unwrap();
                let form = page.locator("dialog[open] form[data-outcome-action=not_taken]");
                expect(form.clone()).to_be_visible().await.unwrap();
                form.locator("select[name=reason]")
                    .select_option("refused", None)
                    .await
                    .unwrap();
                form.locator("button[type=submit]")
                    .press("Enter", None)
                    .await
                    .unwrap();
                expect(page.locator("[data-dose-outcome]").first())
                    .to_contain_text("Not taken")
                    .await
                    .unwrap();
                let outcomes = dose_occurrence::Entity::find()
                    .filter(dose_occurrence::Column::ScheduleId.eq(source.id))
                    .all(&journey.admin)
                    .await
                    .unwrap();
                assert_eq!(outcomes.len(), 1);
                assert_eq!(outcomes[0].position, 1);
                assert_eq!(outcomes[0].outcome, "not_taken");
                assert_eq!(outcomes[0].reason.as_deref(), Some("refused"));
                let medicine = medications::Entity::find_by_id(80006)
                    .one(&journey.admin)
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(medicine.current_supply, Some(Decimal::new(10, 0)));
                assert_eq!(
                    medication_takes::Entity::find()
                        .filter(medication_takes::Column::ScheduleId.eq(source.id))
                        .all(&journey.admin)
                        .await
                        .unwrap()
                        .len(),
                    0
                );
            })
        },
    )
    .await;
}
