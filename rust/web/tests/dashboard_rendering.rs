use medtracker_web::dashboard::{
    DashboardMetrics, DashboardPage, DashboardPerson, DashboardTaskRow, TaskState, render_dashboard,
};

fn render_person(tasks: Vec<DashboardTaskRow>) -> String {
    let mut page = test_page();
    page.people[0].tasks = tasks;
    render_dashboard(page)
}

fn test_page() -> DashboardPage {
    DashboardPage {
        household_name: "Test household".into(),
        slug: "test".into(),
        csrf: "test-token".into(),
        timezone: chrono_tz::UTC,
        greeting: "Good morning".into(),
        date: "Sunday, Mar 29".into(),
        selected_id: "1".into(),
        selected_name: "Test person".into(),
        mobile_shortcuts: vec!["dashboard".into(), "inventory".into(), "finder".into()],
        household_manager: false,
        people: vec![DashboardPerson {
            id: 1,
            name: "Test person".into(),
            tasks: vec![],
            outcomes: vec![],
        }],
        selectable_people: vec![(1, "Test person".into())],
        metrics: DashboardMetrics {
            due_now: 0,
            tasks_left: 0,
            next_due: None,
        },
        stock: vec![],
        history: vec![],
    }
}

#[test]
fn default_shortcuts_are_ordered_and_unavailable_destinations_are_inert() {
    let html = render_person(vec![]);
    let rail = html
        .split("data-testid=\"dashboard-mobile-rail\"")
        .nth(1)
        .unwrap()
        .split("</aside>")
        .next()
        .unwrap();
    let home = rail.find("Home").unwrap();
    let inventory = rail.find("Inventory").unwrap();
    let finder = rail.find("Medicine Finder").unwrap();
    assert!(home < inventory && inventory < finder);
    assert!(rail.contains("href=\"/households/test/dashboard\""));
    assert!(rail.contains("aria-current=\"page\""));
    assert_eq!(rail.matches("aria-disabled=\"true\"").count(), 2);
    assert!(!rail.contains("href=\"/households/test/medications\""));
}

#[test]
fn sidebar_keeps_its_medication_finder_label_separate_from_the_mobile_rail() {
    let html = render_person(vec![]);
    let sidebar = html
        .split("class=\"dashboard-sidebar\"")
        .nth(1)
        .unwrap()
        .split("</aside>")
        .next()
        .unwrap();
    let rail = html
        .split("data-testid=\"dashboard-mobile-rail\"")
        .nth(1)
        .unwrap()
        .split("</aside>")
        .next()
        .unwrap();
    assert!(sidebar.contains("Medication Finder"));
    assert!(!sidebar.contains("Medicine Finder"));
    assert!(rail.contains("Medicine Finder"));
}

#[test]
fn saved_subset_and_household_role_filter_are_preserved() {
    let mut page = test_page();
    page.mobile_shortcuts = vec!["finder".into(), "dashboard".into()];
    let html = render_dashboard(page);
    let rail = html
        .split("data-testid=\"dashboard-mobile-rail\"")
        .nth(1)
        .unwrap()
        .split("</aside>")
        .next()
        .unwrap();
    assert!(rail.find("Medicine Finder").unwrap() < rail.find("Home").unwrap());
    assert!(!rail.contains("Inventory"));

    let mut page = test_page();
    page.mobile_shortcuts = vec!["administration".into(), "dashboard".into()];
    assert!(
        !render_dashboard(page)
            .split("data-testid=\"dashboard-mobile-rail\"")
            .nth(1)
            .unwrap()
            .split("</aside>")
            .next()
            .unwrap()
            .contains("Administration")
    );

    let mut page = test_page();
    page.mobile_shortcuts = vec!["administration".into(), "dashboard".into()];
    page.household_manager = true;
    assert!(
        render_dashboard(page)
            .split("data-testid=\"dashboard-mobile-rail\"")
            .nth(1)
            .unwrap()
            .split("</aside>")
            .next()
            .unwrap()
            .contains("Administration")
    );
}

#[test]
fn unknown_shortcuts_are_ignored_without_a_fallback_link() {
    let mut page = test_page();
    page.mobile_shortcuts = vec!["unknown".into(), "dashboard".into()];
    let html = render_dashboard(page);
    let rail = html
        .split("data-testid=\"dashboard-mobile-rail\"")
        .nth(1)
        .unwrap()
        .split("</aside>")
        .next()
        .unwrap();
    assert!(!rail.contains("unknown"));
    assert_eq!(rail.matches("<a ").count(), 1);
    assert_eq!(rail.matches("<button ").count(), 0);
}

#[test]
fn dashboard_public_css_and_script_have_content_versioned_urls() {
    let html = render_person(vec![]);
    assert!(html.contains("href=\"/dashboard.css?v="));
    assert!(html.contains("src=\"/dashboard.js?v="));
    assert!(!html.contains("href=\"/dashboard.css\""));
    assert!(!html.contains("src=\"/dashboard.js\""));
}

fn medication(routine: bool) -> DashboardTaskRow {
    DashboardTaskRow {
        medication_name: "Test medicine".into(),
        dose: "1 tablet".into(),
        time: "09:00".into(),
        scheduled_at: None,
        state: TaskState::Available,
        routine,
    }
}

#[test]
fn no_as_needed_dropdown_for_routine_only_person() {
    let html = render_person(vec![medication(true)]);
    assert!(html.contains("Test medicine"));
    assert!(!html.contains("dashboard-as-needed-person"));
    assert!(!html.contains("AS NEEDED"));
}

#[test]
fn no_as_needed_dropdown_for_person_without_tasks() {
    let html = render_person(vec![]);
    assert!(!html.contains("dashboard-as-needed-person"));
}

#[test]
fn as_needed_medicines_keep_the_dropdown() {
    let html = render_person(vec![medication(false)]);
    assert!(html.contains("dashboard-as-needed-person"));
    assert!(html.contains("dashboard-as-needed-task"));
    assert!(html.contains("Test medicine"));
}
