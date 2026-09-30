use medtracker_web::dashboard::{
    DashboardMetrics, DashboardPage, DashboardPerson, DashboardTaskRow, TaskState, render_dashboard,
};

fn render_person(tasks: Vec<DashboardTaskRow>) -> String {
    render_dashboard(DashboardPage {
        household_name: "Test household".into(),
        slug: "test".into(),
        csrf: "test-token".into(),
        timezone: chrono_tz::UTC,
        greeting: "Good morning".into(),
        date: "Sunday, Mar 29".into(),
        selected_id: "1".into(),
        selected_name: "Test person".into(),
        people: vec![DashboardPerson {
            id: 1,
            name: "Test person".into(),
            tasks,
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
    })
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
