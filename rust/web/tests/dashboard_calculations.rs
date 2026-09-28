use chrono::{TimeZone, Utc};
use medtracker_web::dashboard::{
    DashboardTask, PrnInput, TaskState, calculate_metrics, calculate_prn, is_same_local_day,
};

fn task(state: TaskState, scheduled_at: Option<chrono::DateTime<Utc>>) -> DashboardTask {
    DashboardTask {
        state,
        scheduled_at,
    }
}

#[test]
fn metrics_count_actionable_work_and_select_the_next_due_time() {
    let now = Utc.with_ymd_and_hms(2026, 3, 29, 0, 30, 0).unwrap();
    let tasks = [
        task(TaskState::Available, None),
        task(TaskState::Upcoming, Some(now)),
        task(TaskState::Upcoming, Some(now + chrono::Duration::hours(2))),
        task(TaskState::Cooldown, Some(now + chrono::Duration::hours(1))),
        task(TaskState::OutOfStock, None),
        task(TaskState::Taken, Some(now - chrono::Duration::minutes(5))),
        task(TaskState::MaxReached, None),
    ];

    let metrics = calculate_metrics(&tasks, now);

    assert_eq!(metrics.due_now, 2);
    assert_eq!(metrics.tasks_left, 5);
}

#[test]
fn next_due_uses_the_earliest_future_action_row_when_nothing_is_due() {
    let now = Utc.with_ymd_and_hms(2026, 3, 29, 0, 30, 0).unwrap();
    let next = now + chrono::Duration::minutes(15);
    let tasks = [
        task(TaskState::Upcoming, Some(now + chrono::Duration::hours(2))),
        task(TaskState::Cooldown, Some(next)),
        task(TaskState::Taken, Some(now + chrono::Duration::minutes(5))),
    ];

    let metrics = calculate_metrics(&tasks, now);

    assert_eq!(metrics.due_now, 0);
    assert_eq!(metrics.tasks_left, 2);
    assert_eq!(metrics.next_due, Some(next));
}

#[test]
fn recorded_dose_day_uses_the_configured_timezone_at_midnight() {
    let timezone = chrono_tz::Europe::London;
    let before_midnight = Utc.with_ymd_and_hms(2026, 3, 28, 23, 30, 0).unwrap();
    let dashboard_now = Utc.with_ymd_and_hms(2026, 3, 29, 0, 30, 0).unwrap();
    let same_day = Utc.with_ymd_and_hms(2026, 3, 29, 0, 15, 0).unwrap();

    assert!(!is_same_local_day(before_midnight, dashboard_now, timezone));
    assert!(is_same_local_day(same_day, dashboard_now, timezone));
}

fn prn_projection(
    now: chrono::DateTime<Utc>,
    dose_cycle: &str,
    max_doses: Option<usize>,
    min_hours_between_doses: Option<f64>,
    takes: &[chrono::DateTime<Utc>],
) -> medtracker_web::dashboard::PrnProjection {
    calculate_prn(PrnInput {
        now,
        timezone: chrono_tz::Europe::London,
        paused: false,
        active: true,
        stock_available: true,
        max_doses,
        min_hours_between_doses,
        dose_cycle,
        takes,
    })
}

#[test]
fn prn_availability_resets_at_the_local_daily_boundary() {
    let now = Utc.with_ymd_and_hms(2026, 3, 29, 0, 30, 0).unwrap();
    let yesterday = Utc.with_ymd_and_hms(2026, 3, 28, 23, 50, 0).unwrap();
    let same_day = Utc.with_ymd_and_hms(2026, 3, 29, 0, 10, 0).unwrap();

    let before_reset = prn_projection(now, "daily", Some(1), None, &[yesterday]);
    let after_limit = prn_projection(now, "daily", Some(1), None, &[same_day]);

    assert_eq!(before_reset.state, TaskState::Available);
    assert_eq!(after_limit.state, TaskState::MaxReached);
    assert_eq!(
        after_limit.next_available_at,
        Some(Utc.with_ymd_and_hms(2026, 3, 29, 23, 0, 0).unwrap())
    );
}

#[test]
fn prn_weekly_and_monthly_limits_follow_local_calendar_periods() {
    let now = Utc.with_ymd_and_hms(2026, 3, 29, 0, 30, 0).unwrap();
    let prior_week = Utc.with_ymd_and_hms(2026, 3, 22, 12, 0, 0).unwrap();
    let this_week = Utc.with_ymd_and_hms(2026, 3, 23, 12, 0, 0).unwrap();
    let prior_month = Utc.with_ymd_and_hms(2026, 2, 28, 12, 0, 0).unwrap();
    let this_month = Utc.with_ymd_and_hms(2026, 3, 1, 12, 0, 0).unwrap();

    assert_eq!(
        prn_projection(now, "weekly", Some(1), None, &[prior_week]).state,
        TaskState::Available
    );
    let weekly_limit = prn_projection(now, "weekly", Some(1), None, &[this_week]);
    assert_eq!(weekly_limit.state, TaskState::MaxReached);
    assert_eq!(
        weekly_limit.next_available_at,
        Some(Utc.with_ymd_and_hms(2026, 3, 29, 23, 0, 0).unwrap())
    );

    assert_eq!(
        prn_projection(now, "monthly", Some(1), None, &[prior_month]).state,
        TaskState::Available
    );
    let monthly_limit = prn_projection(now, "monthly", Some(1), None, &[this_month]);
    assert_eq!(monthly_limit.state, TaskState::MaxReached);
    assert_eq!(
        monthly_limit.next_available_at,
        Some(Utc.with_ymd_and_hms(2026, 3, 31, 23, 0, 0).unwrap())
    );
}

#[test]
fn prn_cooldown_uses_the_last_take_before_now() {
    let now = Utc.with_ymd_and_hms(2026, 3, 29, 0, 30, 0).unwrap();
    let last_take = Utc.with_ymd_and_hms(2026, 3, 29, 0, 0, 0).unwrap();
    let future_take = Utc.with_ymd_and_hms(2026, 3, 29, 4, 0, 0).unwrap();
    let projection = prn_projection(now, "daily", None, Some(2.0), &[last_take, future_take]);

    assert_eq!(projection.state, TaskState::Cooldown);
    assert_eq!(
        projection.next_available_at,
        Some(Utc.with_ymd_and_hms(2026, 3, 29, 2, 0, 0).unwrap())
    );
}

#[test]
fn prn_next_availability_is_the_later_of_cycle_reset_and_cooldown_expiry() {
    let now = Utc.with_ymd_and_hms(2026, 3, 29, 6, 30, 0).unwrap();
    let last_take = Utc.with_ymd_and_hms(2026, 3, 29, 6, 0, 0).unwrap();
    let projection = calculate_prn(PrnInput {
        now,
        timezone: chrono_tz::America::Los_Angeles,
        paused: false,
        active: true,
        stock_available: true,
        max_doses: Some(1),
        min_hours_between_doses: Some(4.0),
        dose_cycle: "daily",
        takes: &[last_take],
    });

    assert_eq!(projection.state, TaskState::MaxReached);
    assert_eq!(
        projection.next_available_at,
        Some(Utc.with_ymd_and_hms(2026, 3, 29, 10, 0, 0).unwrap())
    );
}

#[test]
fn prn_pause_activity_and_stock_restrictions_precede_availability() {
    let now = Utc.with_ymd_and_hms(2026, 3, 29, 0, 30, 0).unwrap();
    let paused = calculate_prn(PrnInput {
        now,
        timezone: chrono_tz::Europe::London,
        paused: true,
        active: true,
        stock_available: true,
        max_doses: None,
        min_hours_between_doses: None,
        dose_cycle: "daily",
        takes: &[],
    });
    let inactive = calculate_prn(PrnInput {
        now,
        timezone: chrono_tz::Europe::London,
        paused: false,
        active: false,
        stock_available: true,
        max_doses: None,
        min_hours_between_doses: None,
        dose_cycle: "daily",
        takes: &[],
    });
    let out_of_stock = calculate_prn(PrnInput {
        now,
        timezone: chrono_tz::Europe::London,
        paused: false,
        active: true,
        stock_available: false,
        max_doses: None,
        min_hours_between_doses: None,
        dose_cycle: "daily",
        takes: &[],
    });

    assert_eq!(paused.state, TaskState::Paused);
    assert_eq!(inactive.state, TaskState::Unknown);
    assert_eq!(out_of_stock.state, TaskState::OutOfStock);
}
