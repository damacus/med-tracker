#![cfg(unix)]

use loco_rs::{
    environment::Environment,
    scheduler::{Job, Output, Scheduler},
};
use med_tracker::app::App;
use std::{
    io,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

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

fn binary() -> PathBuf {
    static PROBE: OnceLock<tempfile::TempDir> = OnceLock::new();
    PROBE
        .get_or_init(|| {
            let directory = tempfile::tempdir().unwrap();
            let compiled = Command::new("rustc")
                .args(["tests/fixtures/scheduler_probe.rs", "-o"])
                .arg(directory.path().join("probe"))
                .output()
                .unwrap();
            assert!(
                compiled.status.success(),
                "{}",
                String::from_utf8_lossy(&compiled.stderr)
            );
            directory
        })
        .path()
        .join("probe")
}

#[test]
fn native_task_preserves_arguments_environment_and_literal_metacharacters() {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("binary with spaces");
    std::os::unix::fs::symlink(binary(), &executable).unwrap();
    let marker = directory.path().join("injected");
    let report = directory.path().join("report");
    let run = format!(
        "support-access-expire '{}' 'label:two  spaces' 'quote:it\"s' '$(touch {})' ';' '&' '>' '*.yaml' '$HOME' ''",
        report.display(),
        marker.display()
    );
    let description = job(&run)
        .prepare_command(&executable, &Output::STDOUT, &Environment::Test)
        .unwrap();
    assert!(description.run().unwrap().status.success());
    let expected = [
        "task",
        "support-access-expire",
        report.to_str().unwrap(),
        "label:two  spaces",
        "quote:it\"s",
        &format!("$(touch {})", marker.display()),
        ";",
        "&",
        ">",
        "*.yaml",
        "$HOME",
        "",
        "test",
        &std::env::var("PATH").unwrap(),
    ]
    .join("\0");
    assert_eq!(std::fs::read_to_string(report).unwrap(), expected);
    assert!(
        !marker.exists(),
        "Native task arguments must never execute shell syntax"
    );
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
    ] {
        let description = job(run).prepare_command(&binary(), &Output::STDOUT, &Environment::Test);
        assert_eq!(description.unwrap_err().kind(), io::ErrorKind::InvalidInput);
        let config = loco_rs::scheduler::Config {
            jobs: [("invalid".into(), job(run))].into(),
            output: Output::STDOUT,
        };
        assert!(Scheduler::new::<App>(&config, &Environment::Test).is_err());
    }
}

#[test]
fn native_task_does_not_execute_unquoted_shell_operators() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("injected");
    let report = directory.path().join("report");
    let run = format!(
        "support-access-expire '{}' ; touch '{}'",
        report.display(),
        marker.display()
    );
    job(&run)
        .prepare_command(&binary(), &Output::STDOUT, &Environment::Test)
        .unwrap()
        .run()
        .unwrap();
    assert!(
        !marker.exists(),
        "Unquoted shell operators must remain native task arguments"
    );
    let arguments = std::fs::read_to_string(report).unwrap();
    assert!(arguments.contains("\0;\0touch\0"));
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
fn native_process_reports_nonzero_exit() {
    let failed = job("fail synthetic-secret:do-not-log")
        .prepare_command(&binary(), &Output::STDOUT, &Environment::Test)
        .unwrap();
    let error = failed.run().unwrap_err();
    assert!(error.to_string().contains("37"), "{error}");
    assert!(!error.to_string().contains("synthetic-secret"));
}

#[test]
fn missing_native_executable_preserves_error_kind_without_argument_values() {
    let directory = tempfile::tempdir().unwrap();
    let description = job("support-access-expire synthetic-secret:do-not-log")
        .prepare_command(
            &directory.path().join("missing"),
            &Output::STDOUT,
            &Environment::Test,
        )
        .unwrap();
    let error = description.run().unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
    assert!(!error.to_string().contains("synthetic-secret"));
}

#[test]
fn output_modes_control_actual_child_stdout_and_stderr() {
    for (mode, visible) in [("stdout", true), ("silent", false)] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "scheduler_output_child",
                "--ignored",
                "--nocapture",
            ])
            .env("MEDTRACKER_SCHEDULER_OUTPUT", mode)
            .env("MEDTRACKER_SCHEDULER_PROBE", binary())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).contains("synthetic scheduler stdout"),
            visible
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stderr).contains("synthetic scheduler stderr"),
            visible
        );
    }
}

#[test]
#[ignore = "subprocess fixture run by output_modes_control_actual_child_stdout_and_stderr"]
fn scheduler_output_child() {
    let directory = tempfile::tempdir().unwrap();
    let report = directory.path().join("report");
    let mut configured = job(&format!("support-access-expire '{}'", report.display()));
    configured.output = Some(
        if std::env::var("MEDTRACKER_SCHEDULER_OUTPUT").unwrap() == "silent" {
            Output::Silent
        } else {
            Output::STDOUT
        },
    );
    let executable = PathBuf::from(std::env::var_os("MEDTRACKER_SCHEDULER_PROBE").unwrap());
    configured
        .prepare_command(&executable, &Output::STDOUT, &Environment::Test)
        .unwrap()
        .run()
        .unwrap();
    assert!(report.exists());
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
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(std::fs::read_to_string(report).unwrap(), "test");
    shell.run = "exit 37".into();
    assert!(
        shell
            .prepare_command(Path::new("unused"), &Output::STDOUT, &Environment::Test)
            .unwrap()
            .run()
            .is_err()
    );
}
