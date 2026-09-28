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
        TaskState::Available => "Available",
        TaskState::Taken => "Taken",
        TaskState::Paused => "Paused",
        TaskState::NotTaken => "Not taken",
        TaskState::Cooldown => "Wait",
        TaskState::OutOfStock => "Out of stock",
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
        <div class="dashboard-task" data-testid=test_id>
            <span class="dashboard-task-time">{row.time}</span>
            <span class="dashboard-task-copy"><strong>{row.medication_name}</strong><small>{row.dose}</small></span>
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
    view! { <svg class="dashboard-icon" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d=path/></svg> }
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
            .unwrap_or_else(|| "None".to_owned())
    };
    let selected_name = page.selected_name.clone();
    let selected_id = page.selected_id.clone();
    let csrf = page.csrf.clone();
    let body = view! {
        <div class="dashboard-shell" data-testid="dashboard">
            <p id="dashboard-unavailable-help" class="dashboard-unavailable-help">"Read-only preview: search, navigation, medication changes, dose recording, refills, and reports are unavailable here."</p>
            <aside class="dashboard-sidebar"><div class="dashboard-brand"><span class="dashboard-brand-mark">"M"</span><strong>"MedTracker"</strong></div>
                <button class="dashboard-search" type="button" aria-disabled="true" aria-describedby="dashboard-unavailable-help" title="Search is not available in this preview">{dashboard_icon("m21 21-4.3-4.3 M19 11a8 8 0 1 1-16 0 8 8 0 0 1 16 0")}"Search"<kbd>"Ctrl K"</kbd></button>
                <nav aria-label="Main navigation"><a aria-current="page" href=prefix.clone()>{dashboard_icon("m3 10 9-7 9 7 M5 9v12h14V9 M9 21v-7h6v7")}"Dashboard"</a>{[("Inventory", "M4 7h16v14H4z M7 3h10v4 M8 11h8 M8 15h8"), ("Locations", "M12 22s7-6 7-12a7 7 0 1 0-14 0c0 6 7 12 7 12z M12 10h.01"), ("People", "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2 M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8 M22 21v-2a4 4 0 0 0-3-3.87 M16 3.13a4 4 0 0 1 0 7.75"), ("Medication Finder", "M4 14 14 4a5 5 0 0 1 7 7L11 21a5 5 0 0 1-7-7z M9 9l6 6"), ("Medicine reviews", "M8 3h8l4 4v14H4V3z M8 12h8 M8 16h6"), ("Reports", "M4 20V4 M4 20h16 M8 16v-5 M13 16V7 M18 16v-8"), ("Administration", "M12 2 4 5v6c0 5 3 8 8 11 5-3 8-6 8-11V5z M9 12l2 2 4-4")].into_iter().map(|(label, path)| view! { <button type="button" aria-disabled="true" aria-describedby="dashboard-unavailable-help" title="This page is not available in this preview">{dashboard_icon(path)}{label}</button> }).collect_view()}</nav>
                <div class="dashboard-sidebar-bottom"><span class="dashboard-identity"><span class="dashboard-avatar" aria-hidden="true">{selected_name.chars().next().unwrap_or('?').to_string()}</span><span><strong>{selected_name.clone()}</strong><small>"Household"</small></span></span><form action="/logout" method="post"><input type="hidden" name="authenticity_token" value=csrf/><button type="submit">"Sign Out"</button></form></div>
            </aside>
            <main class="dashboard-main"><header class="dashboard-top"><div><p class="dashboard-date">{page.date}</p><h1>{page.greeting}</h1></div><div class="dashboard-quick-actions"><button type="button" aria-disabled="true" aria-describedby="dashboard-unavailable-help" title="Adding people is not available in this preview">"Add Person"</button><button type="button" aria-label="Add Medication" aria-disabled="true" aria-describedby="dashboard-unavailable-help" title="Adding medicines is not available in this preview">"Add Medication"</button></div></header>
                <details class="dashboard-selector" data-testid="dashboard-person-selector-disclosure"><summary data-testid="dashboard-person-selector-summary"><span class="dashboard-avatar" aria-hidden="true">{selected_name.chars().next().unwrap_or('?').to_string()}</span><strong>{selected_name}</strong><span class="dashboard-change">"Change person ⌄"</span></summary><nav aria-label="Select person" data-testid="dashboard-person-options"><a href=format!("{prefix}?dashboard_person_id=all") aria-current=(selected_id == "all").then_some("true")>"All Family"</a>{page.selectable_people.into_iter().map(|(id, name)| view! { <a data-testid="dashboard-person-option" href=format!("{prefix}?dashboard_person_id={id}") aria-current=(selected_id == id.to_string()).then_some("true")>{name}</a> }).collect_view()}</nav></details>
                <section class="dashboard-metrics" data-testid="dashboard-metrics" aria-label="Today's summary"><div>{dashboard_icon("M12 3v3 M12 18v3 M3 12h3 M18 12h3 M5.6 5.6l2.1 2.1 M16.3 16.3l2.1 2.1 M5.6 18.4l2.1-2.1 M16.3 7.7l2.1-2.1 M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8") }<span>"NEXT DUE"</span><strong>{next_due}</strong></div><div>{dashboard_icon("M12 2v5 M12 12l4 2 M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20") }<span>"DUE NOW"</span><strong>{page.metrics.due_now}</strong></div><div>{dashboard_icon("M4 5h16v16H4z M8 10l2 2 5-5 M8 17h8") }<span>"TASKS LEFT"</span><strong>{page.metrics.tasks_left}</strong></div></section>
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
