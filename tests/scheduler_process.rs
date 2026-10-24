#![cfg(unix)]

use loco_rs::{
    environment::Environment,
    scheduler::{Job, Output, Scheduler},
};
use med_tracker::app::App;
use std::path::Path;

fn job(run: &str) -> Job {
    Job {
        run: run.into(),
        shell: false,
        run_on_start: false,
        cron: "0 * * * * *".into(),
        tags: None,
        output: None,
    }
}

#[test]
fn invalid_native_commands_fail_before_spawning() {
    for run in [
        "",
        "   ",
        "''",
        "'",
        "support-access-expire \"unterminated",
        "support-access-expire 'unterminated",
        "support-access-expire\0",
        "support-access-expire value:bad\0argument",
    ] {
        let config = loco_rs::scheduler::Config {
            jobs: [("invalid".into(), job(run))].into(),
            output: Output::STDOUT,
        };
        assert!(Scheduler::new::<App>(&config, &Environment::Test).is_err());
    }
}

#[test]
fn scheduler_validates_the_parsed_task_name_and_keeps_filters() {
    let mut configured = job("'support-access-expire' 'label:two words'");
    configured.tags = Some(vec!["expiry".into()]);
    let mut other = job("browser-reminders");
    other.tags = Some(vec!["reminders".into()]);
    let config = loco_rs::scheduler::Config {
        jobs: [("quoted".into(), configured), ("other".into(), other)].into(),
        output: Output::STDOUT,
    };
    let scheduler = Scheduler::new::<App>(&config, &Environment::Test).unwrap();
    assert_eq!(
        scheduler
            .clone()
            .by_spec(&loco_rs::scheduler::Spec {
                name: Some("quoted".into()),
                tag: None
            })
            .jobs
            .len(),
        1
    );
    assert_eq!(
        scheduler
            .by_spec(&loco_rs::scheduler::Spec {
                name: None,
                tag: Some("expiry".into())
            })
            .jobs
            .len(),
        1
    );
}

#[test]
fn production_jobs_are_registered_native_tasks() {
    let config =
        loco_rs::config::Config::from_folder(&Environment::Production, Path::new("config"))
            .unwrap();
    let configured = config.scheduler.unwrap();
    assert_eq!(configured.jobs.len(), 4);
    for task in configured.jobs.values() {
        assert!(!task.shell, "Production schedules require native execution");
    }
    let scheduler = Scheduler::new::<App>(&configured, &Environment::Production).unwrap();
    let mut names: Vec<_> = scheduler
        .jobs
        .values()
        .map(|task| task.run.as_str())
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "browser-reminders",
            "medication-review-evidence-refresh",
            "nhs_dmd_reconcile",
            "support-access-expire"
        ]
    );
}

#[test]
fn explicit_shell_jobs_keep_environment_and_exit_errors() {
    let directory = tempfile::tempdir().unwrap();
    let report = directory.path().join("environment");
    let mut shell = job(&format!(
        "printf '%s' \"$LOCO_ENV\" > '{}'",
        report.display()
    ));
    shell.shell = true;
    shell
        .prepare_command(Path::new("unused"), &Output::STDOUT, &Environment::Test)
        .run()
        .unwrap();
    assert_eq!(std::fs::read_to_string(report).unwrap(), "test");
    shell.run = "exit 37".into();
    assert!(
        shell
            .prepare_command(Path::new("unused"), &Output::STDOUT, &Environment::Test)
            .run()
            .is_err()
    );
}
