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
                let identified: bool = compact.first().evaluate("row => [...row.querySelectorAll('[aria-label], img[alt]')].some(node => (node.getAttribute('aria-label') || node.getAttribute('alt')) === 'Alexandra Synthetic Person With A Long Readable Name')", None::<&()>).await.unwrap();
                assert!(identified, "avatar must expose the complete person name");
                let corner: bool = compact.first().evaluate("row => { const card = row.querySelector('.dashboard-timeline-card').getBoundingClientRect(); const avatar = row.querySelector('.dashboard-avatar').getBoundingClientRect(); return avatar.top >= card.top && avatar.top - card.top <= 24 && avatar.right <= card.right && card.right - avatar.right <= 24; }", None::<&()>).await.unwrap();
                assert!(corner, "patient avatar must stay in the card's top-right corner at {width}px");
                let taken = page.locator(".dashboard-timeline-row:has(h3:text-is('Synthetic completed medicine'))");
                let disabled: bool = taken.evaluate("row => [...row.querySelectorAll('button')].some(button => button.disabled && /Taken/.test(button.textContent))", None::<&()>).await.unwrap();
                assert!(disabled, "completed dose must retain a disabled Taken button");
                let aligned: bool = page.evaluate("() => { const open = document.querySelector('.dashboard-timeline-row[data-status=available] [data-dashboard-action]'); const taken = document.querySelector('[data-dashboard-taken]'); return !!open && !!taken && Math.abs(open.getBoundingClientRect().right - taken.getBoundingClientRect().right) < 1; }", None::<&()>).await.unwrap();
                assert!(aligned, "Taken must replace Take in the same action position");
                let menu = compact.first().locator(".dashboard-row-menu");
                let summary = menu.locator("summary");
                summary.press("Enter", None).await.unwrap();
                expect(menu.get_by_role(AriaRole::Link, Some(playwright_rs::protocol::GetByRoleOptions::default().name("More details").exact(false)))).to_be_visible().await.unwrap();
                let skip = menu.get_by_role(AriaRole::Link, Some(playwright_rs::protocol::GetByRoleOptions::default().name("Skip").exact(false)));
                expect(skip.clone()).to_be_visible().await.unwrap();
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
                    const links = [...document.querySelectorAll('#dashboard a[href*=\"record_occurrence_key=\"]')].map(anchor => {
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
            expect(page.get_by_text("Smart Insights", false).first()).to_be_visible().await.unwrap();
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
                        "#dashboard a[href*='record_source_id={}'][href*='record_occurrence_key=']",
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
                    page.locator(".dashboard-timeline-row[data-status=taken] .dashboard-dose")
                        .first(),
                )
                .to_contain_text("3 tablets")
                .await
                .unwrap();
                assert_eq!(
                    page.locator(format!(
                        "#dashboard a[href*='record_source_id={}'][href*='record_occurrence_key=']",
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
                "#dashboard a[href*='record_source_id={}'][href*='record_occurrence_key=']",
                scheduled.id
            );
            for (size, width, height) in [("desktop", 1280, 900), ("mobile", 360, 800)] {
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
