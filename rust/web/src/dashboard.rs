use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;
use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskState {
    Upcoming,
    Available,
    Taken,
    Paused,
    NotTaken,
    Cooldown,
    OutOfStock,
    MaxReached,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DashboardTask {
    pub state: TaskState,
    pub scheduled_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DashboardMetrics {
    pub next_due: Option<DateTime<Utc>>,
    pub due_now: usize,
    pub tasks_left: usize,
}

pub fn calculate_metrics(tasks: &[DashboardTask], now: DateTime<Utc>) -> DashboardMetrics {
    let due_now = tasks
        .iter()
        .filter(|task| {
            task.state == TaskState::Available
                || (task.state == TaskState::Upcoming
                    && task.scheduled_at.is_none_or(|scheduled| scheduled <= now))
        })
        .count();
    let tasks_left = tasks
        .iter()
        .filter(|task| {
            !matches!(
                task.state,
                TaskState::Taken | TaskState::MaxReached | TaskState::NotTaken | TaskState::Unknown
            )
        })
        .count();
    let next_due = tasks
        .iter()
        .filter(|task| {
            !matches!(
                task.state,
                TaskState::Taken | TaskState::MaxReached | TaskState::NotTaken | TaskState::Unknown
            )
        })
        .filter(|task| {
            !(task.state == TaskState::Available
                || (task.state == TaskState::Upcoming
                    && task.scheduled_at.is_none_or(|scheduled| scheduled <= now)))
        })
        .filter_map(|task| task.scheduled_at)
        .min();
    DashboardMetrics {
        next_due,
        due_now,
        tasks_left,
    }
}

pub fn is_same_local_day(timestamp: DateTime<Utc>, now: DateTime<Utc>, timezone: Tz) -> bool {
    timestamp.with_timezone(&timezone).date_naive() == now.with_timezone(&timezone).date_naive()
}

pub struct PrnInput<'a> {
    pub now: DateTime<Utc>,
    pub timezone: Tz,
    pub paused: bool,
    pub active: bool,
    pub stock_available: bool,
    pub max_doses: Option<usize>,
    pub min_hours_between_doses: Option<f64>,
    pub dose_cycle: &'a str,
    pub takes: &'a [DateTime<Utc>],
}

pub struct PrnProjection {
    pub state: TaskState,
    pub next_available_at: Option<DateTime<Utc>>,
}

pub fn calculate_prn(input: PrnInput<'_>) -> PrnProjection {
    if input.paused {
        return PrnProjection {
            state: TaskState::Paused,
            next_available_at: None,
        };
    }
    if !input.active {
        return PrnProjection {
            state: TaskState::Unknown,
            next_available_at: None,
        };
    }
    if !input.stock_available {
        return PrnProjection {
            state: TaskState::OutOfStock,
            next_available_at: None,
        };
    }
    let local_now = input.now.with_timezone(&input.timezone);
    let cycle_takes = input
        .takes
        .iter()
        .filter(|taken| {
            let local_taken = taken.with_timezone(&input.timezone);
            match input.dose_cycle {
                "weekly" => local_taken.iso_week() == local_now.iso_week(),
                "monthly" => {
                    local_taken.year() == local_now.year()
                        && local_taken.month() == local_now.month()
                }
                _ => local_taken.date_naive() == local_now.date_naive(),
            }
        })
        .count();
    let interval_next = input.min_hours_between_doses.and_then(|hours| {
        input
            .takes
            .iter()
            .filter(|taken| **taken <= input.now)
            .max()
            .map(|last| *last + Duration::seconds((hours * 3600.0).ceil() as i64))
            .filter(|next| *next > input.now)
    });
    if input.max_doses.is_some_and(|max| cycle_takes >= max) {
        let next_date = match input.dose_cycle {
            "weekly" => {
                local_now.date_naive()
                    + Duration::days(7 - local_now.weekday().num_days_from_monday() as i64)
            }
            "monthly" => {
                let (year, month) = if local_now.month() == 12 {
                    (local_now.year() + 1, 1)
                } else {
                    (local_now.year(), local_now.month() + 1)
                };
                NaiveDate::from_ymd_opt(year, month, 1).unwrap()
            }
            _ => local_now.date_naive() + Duration::days(1),
        };
        let next = input
            .timezone
            .from_local_datetime(&next_date.and_hms_opt(0, 0, 0).unwrap())
            .earliest()
            .map(|time| time.with_timezone(&Utc))
            .into_iter()
            .chain(interval_next)
            .max();
        return PrnProjection {
            state: TaskState::MaxReached,
            next_available_at: next,
        };
    }
    if let Some(next) = interval_next {
        return PrnProjection {
            state: TaskState::Cooldown,
            next_available_at: Some(next),
        };
    }
    PrnProjection {
        state: TaskState::Available,
        next_available_at: None,
    }
}

#[derive(Clone)]
pub struct DashboardPerson {
    pub id: i64,
    pub name: String,
    pub tasks: Vec<DashboardTaskRow>,
    pub outcomes: Vec<DashboardTaskRow>,
}

#[derive(Clone)]
pub struct DashboardTaskRow {
    pub medication_name: String,
    pub dose: String,
    pub time: String,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub state: TaskState,
    pub routine: bool,
}

#[derive(Clone)]
pub struct DashboardStock {
    pub name: String,
    pub amount: String,
    pub unit: String,
    pub low: bool,
    pub out: bool,
}

#[derive(Clone)]
pub struct DashboardHistory {
    pub person_name: String,
    pub medication_name: String,
    pub dose: String,
    pub time: String,
}

pub struct DashboardPage {
    pub household_name: String,
    pub slug: String,
    pub csrf: String,
    pub timezone: Tz,
    pub greeting: String,
    pub date: String,
    pub selected_id: String,
    pub selected_name: String,
    pub people: Vec<DashboardPerson>,
    pub selectable_people: Vec<(i64, String)>,
    pub metrics: DashboardMetrics,
    pub stock: Vec<DashboardStock>,
    pub history: Vec<DashboardHistory>,
}

fn status_name(state: TaskState) -> &'static str {
    match state {
        TaskState::Upcoming => "Upcoming",
        TaskState::Available => "Available now",
        TaskState::Taken => "Taken",
        TaskState::Paused => "Paused",
        TaskState::NotTaken => "Not taken",
        TaskState::Cooldown => "Wait",
        TaskState::OutOfStock => "Out of Stock",
        TaskState::MaxReached => "Dose limit reached",
        TaskState::Unknown => "Status unavailable",
    }
}

fn task_row(row: DashboardTaskRow) -> impl IntoView {
    let test_id = if row.routine {
        "dashboard-routine-task"
    } else {
        "dashboard-as-needed-task"
    };
    let status = status_name(row.state);
    view! {
        <div class="dashboard-task" data-testid=test_id data-state=status>
            <div class="dashboard-task-leading"><span class="dashboard-task-time">{row.time}</span>
            <span class="dashboard-task-copy"><strong>{row.medication_name}</strong><small>{row.dose}</small></span></div>
            <span class="dashboard-task-action"><button type="button" aria-disabled="true" aria-describedby="dashboard-unavailable-help" title="Dose recording is not available in this preview">"Take"</button><span class="dashboard-status">{status}</span></span>
        </div>
    }
}

fn person_card(person: DashboardPerson) -> impl IntoView {
    let initial = person.name.chars().next().unwrap_or('?').to_string();
    let routine: Vec<_> = person
        .tasks
        .iter()
        .filter(|row| row.routine)
        .cloned()
        .collect();
    let as_needed: Vec<_> = person
        .tasks
        .iter()
        .filter(|row| !row.routine)
        .cloned()
        .collect();
    let no_routine = routine.is_empty();
    view! {
        <article class="dashboard-person-card">
            <div class="dashboard-person-heading"><span class="dashboard-avatar" aria-hidden="true">{initial}</span><div><h3>{person.name}</h3><p>{if no_routine { "No routine tasks awaiting a dose" } else { "Today's medication tasks" }}</p></div><span class="dashboard-person-count">{routine.len().to_string()}</span></div>
            <div class="dashboard-routine" inner_html=if no_routine { view! { <p class="dashboard-routine-empty">"No routine tasks awaiting a dose"</p> }.to_html() } else { routine.into_iter().map(task_row).collect_view().to_html() }></div>
            {(!person.outcomes.is_empty()).then(|| view! { <div class="dashboard-outcomes" data-testid="dashboard-not-taken-outcome">{person.outcomes.into_iter().map(task_row).collect_view()}</div> })}
            <details class="dashboard-prn" data-testid="dashboard-as-needed-person"><summary>"AS NEEDED"</summary><div inner_html=if as_needed.is_empty() { view! { <p>"No as-needed medicines"</p> }.to_html() } else { as_needed.into_iter().map(task_row).collect_view().to_html() }></div></details>
        </article>
    }
}

fn dashboard_icon(path: &'static str) -> impl IntoView {
    view! { <svg class="dashboard-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d=path/></svg> }
}

fn dashboard_material_icon(path: &'static str) -> impl IntoView {
    view! { <svg class="dashboard-icon" aria-hidden="true" viewBox="0 -960 960 960" fill="currentColor" stroke="none"><path d=path/></svg> }
}

pub fn render_dashboard(page: DashboardPage) -> String {
    let prefix = format!("/households/{}/dashboard", page.slug);
    let next_due = if page.metrics.due_now > 0 {
        "Now".to_owned()
    } else {
        page.metrics
            .next_due
            .map(|time| {
                time.with_timezone(&page.timezone)
                    .format("%H:%M")
                    .to_string()
            })
            .unwrap_or_else(|| "None today".to_owned())
    };
    let selected_name = page.selected_name.clone();
    let selected_id = page.selected_id.clone();
    let csrf = page.csrf.clone();
    let body = view! {
        <div class="dashboard-shell" data-testid="dashboard">
            <p id="dashboard-unavailable-help" class="dashboard-unavailable-help">"Read-only preview: search, navigation, medication changes, dose recording, refills, and reports are unavailable here."</p>
            <header class="dashboard-mobile-topbar"><button id="dashboard-menu-trigger" type="button" aria-label="Open menu" aria-controls="dashboard-navigation" aria-expanded="false">{dashboard_icon("M4 6h16 M4 12h16 M4 18h16")}</button><a href=prefix.clone()>"MedTracker"</a><button type="button" aria-label="Search" aria-disabled="true" aria-describedby="dashboard-unavailable-help">{dashboard_icon("m21 21-4.3-4.3 M19 11a8 8 0 1 1-16 0 8 8 0 0 1 16 0")}</button></header>
            <dialog id="dashboard-navigation" class="dashboard-drawer" aria-label="Navigation menu"><header class="dashboard-drawer-header"><h2>"MedTracker"</h2><button type="button" aria-label="Close menu" autofocus>{dashboard_icon("M6 6l12 12 M18 6 6 18")}</button></header></dialog>
            <aside class="dashboard-sidebar"><div class="dashboard-brand"><span class="dashboard-brand-mark">"M"</span><strong>"MedTracker"</strong></div>
                <button class="dashboard-search" type="button" aria-disabled="true" aria-describedby="dashboard-unavailable-help" title="Search is not available in this preview">{dashboard_icon("m21 21-4.3-4.3 M19 11a8 8 0 1 1-16 0 8 8 0 0 1 16 0")}"Search"<kbd>"Ctrl K"</kbd></button>
                <nav aria-label="Main navigation"><a aria-current="page" href=prefix.clone()>{dashboard_icon("m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z M9 22V12h6v10")}"Dashboard"</a>{[("Inventory", "M620-163 450-333l56-56 114 114 226-226 56 56-282 282Zm220-397h-80v-200h-80v120H280v-120h-80v560h240v80H200q-33 0-56.5-23.5T120-200v-560q0-33 23.5-56.5T200-840h167q11-35 43-57.5t70-22.5q40 0 71.5 22.5T594-840h166q33 0 56.5 23.5T840-760v200ZM480-760q17 0 28.5-11.5T520-800q0-17-11.5-28.5T480-840q-17 0-28.5 11.5T440-800q0 17 11.5 28.5T480-760Z"), ("Locations", "m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z M9 22V12h6v10"), ("People", "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2 M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8 M22 21v-2a4 4 0 0 0-3-3.87 M16 3.13a4 4 0 0 1 0 7.75"), ("Medication Finder", "m21 21-4.3-4.3 M3 11a8 8 0 1 0 16 0a8 8 0 1 0 -16 0"), ("Medicine reviews", "M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z M14 2v4a2 2 0 0 0 2 2h4 M10 9H8 M16 13H8 M16 17H8"), ("Reports", "M12 8v4 M12 16h.01 M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0"), ("Administration", "M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z M9 12a3 3 0 1 0 6 0a3 3 0 1 0 -6 0")].into_iter().map(|(label, path)| view! { <button type="button" aria-disabled="true" aria-describedby="dashboard-unavailable-help" title="This page is not available in this preview">{if label == "Inventory" { dashboard_material_icon(path).into_any() } else { dashboard_icon(path).into_any() }}{label}</button> }).collect_view()}</nav>
                <div class="dashboard-sidebar-bottom"><span class="dashboard-identity"><span class="dashboard-avatar" aria-hidden="true">{selected_name.chars().next().unwrap_or('?').to_string()}</span><span><strong>{selected_name.clone()}</strong><small>"Household"</small></span></span><form action="/logout" method="post"><input type="hidden" name="authenticity_token" value=csrf/><button type="submit"><span class="dashboard-desktop-label">"Sign Out"</span><span class="dashboard-mobile-label">"Logout"</span></button></form></div>
            </aside>
            <main class="dashboard-main"><header class="dashboard-top"><div><p class="dashboard-date">{page.date}</p><h1>{page.greeting}</h1></div><div class="dashboard-quick-actions"><button type="button" aria-disabled="true" aria-describedby="dashboard-unavailable-help" title="Adding people is not available in this preview">"Add Person"</button><button type="button" aria-label="Add Medication" aria-disabled="true" aria-describedby="dashboard-unavailable-help" title="Adding medicines is not available in this preview">"Add Medication"</button></div></header>
                <details class="dashboard-selector" data-testid="dashboard-person-selector-disclosure"><summary data-testid="dashboard-person-selector-summary"><span class="dashboard-avatar" aria-hidden="true">{selected_name.chars().next().unwrap_or('?').to_string()}</span><strong>{selected_name}</strong><span class="dashboard-change">"Change person ⌄"</span></summary><nav aria-label="Select person" data-testid="dashboard-person-options"><a href=format!("{prefix}?dashboard_person_id=all") aria-current=(selected_id == "all").then_some("true")>"All Family"</a>{page.selectable_people.into_iter().map(|(id, name)| view! { <a data-testid="dashboard-person-option" href=format!("{prefix}?dashboard_person_id={id}") aria-current=(selected_id == id.to_string()).then_some("true")>{name}</a> }).collect_view()}</nav></details>
                <section class="dashboard-metrics" data-testid="dashboard-metrics" aria-label="Today's summary"><div>{dashboard_icon("M12 6v6l4 2 M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0") }<span>"NEXT DUE"</span><strong>{next_due}</strong></div><div>{dashboard_material_icon("M200-640h560v-80H200v80Zm0 0v-80 80Zm0 560q-33 0-56.5-23.5T120-160v-560q0-33 23.5-56.5T200-800h40v-80h80v80h320v-80h80v80h40q33 0 56.5 23.5T840-720v227q-19-9-39-15t-41-9v-43H200v400h252q7 22 16.5 42T491-80H200Zm378.5-18.5Q520-157 520-240t58.5-141.5Q637-440 720-440t141.5 58.5Q920-323 920-240T861.5-98.5Q803-40 720-40T578.5-98.5ZM787-145l28-28-75-75v-112h-40v128l87 87Z") }<span>"DUE NOW"</span><strong>{page.metrics.due_now}</strong></div><div>{dashboard_icon("m9 12 2 2 4-4 M2 12a10 10 0 1 0 20 0a10 10 0 1 0 -20 0") }<span>"TASKS LEFT"</span><strong>{page.metrics.tasks_left}</strong></div></section>
                <div class="dashboard-columns"><section class="dashboard-schedule"><h2>"Today's Schedule"</h2><div inner_html=if page.people.iter().all(|person| person.tasks.is_empty() && person.outcomes.is_empty()) { view! { <p class="dashboard-empty">"No medication tasks for this selection."</p> }.to_html() } else { page.people.into_iter().map(person_card).collect_view().to_html() }></div></section>
                    <section class="dashboard-stock"><h2>"Stock Inventory"</h2><div class="dashboard-stock-card"><div inner_html=if page.stock.is_empty() { view! { <p>"No stock for this selection."</p> }.to_html() } else { page.stock.into_iter().map(|stock| { let bar_class = if stock.out { "dashboard-stock-bar out" } else if stock.low { "dashboard-stock-bar low" } else { "dashboard-stock-bar" }; view! { <div class="dashboard-stock-item"><div><strong>{stock.name}</strong><span>{stock.amount}" "{stock.unit}" left"</span></div><div class=bar_class></div></div> } }).collect_view().to_html() }></div><button type="button" aria-disabled="true" aria-describedby="dashboard-unavailable-help" title="Ordering refills is not available in this preview">"ORDER REFILLS"</button></div></section>
                    {(!page.history.is_empty()).then(|| view! { <section class="dashboard-history" data-testid="dashboard-today-dose-history"><h2>"Previous Doses Today"</h2>{page.history.into_iter().map(|row| view! { <div class="dashboard-history-row"><span><strong>{row.medication_name}</strong><small>{row.person_name}" · "{row.dose}</small></span><time>{row.time}</time></div> }).collect_view()}</section> })}
                    <section class="dashboard-insights"><h2>"Smart Insights"</h2><div><span class="dashboard-insights-icon" aria-hidden="true">"⌁"</span><h3>"Insights coming soon"</h3><p>"Your medication insights will appear here when this feature is available."</p><button type="button" aria-disabled="true" aria-describedby="dashboard-unavailable-help" title="Reports are not available in this preview">"VIEW FULL REPORT"</button></div></section>
                </div><footer>"v0.1.0"</footer>
            </main>
        </div>
    }.to_html();
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><meta name=\"theme-color\" content=\"#7daa92\"><meta name=\"csrf-token\" content=\"{}\"><link rel=\"stylesheet\" href=\"/dashboard.css\"><link rel=\"manifest\" href=\"/manifest.webmanifest\"><script defer src=\"/dashboard.js\"></script><title>Dashboard | MedTracker</title></head><body>{body}</body></html>",
        page.csrf
    )
}
