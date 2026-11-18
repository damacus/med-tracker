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
fn platform_bootstrap_task_wrapper_uses_selected_environment() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let bin = directory.path().join("bin");
    let config = directory.path().join("config");
    std::fs::create_dir(&bin).unwrap();
    std::fs::create_dir(&config).unwrap();
    for environment in ["production", "development"] {
        std::fs::write(
            config.join(format!("{environment}.yaml")),
            format!("<%= {environment}_sentinel() %>"),
        )
        .unwrap();
    }
    let cargo = bin.join("cargo");
    std::fs::write(&cargo, "#!/usr/bin/env node\nconst {spawnSync}=require('node:child_process');\nconst args=process.argv.slice(2);\nconst result=spawnSync(process.env.MEDTRACKER_BOOTSTRAP_TEST_BINARY,args.slice(args.indexOf('--')+1),{stdio:'inherit'});\nif(result.error) throw result.error;\nprocess.exit(result.status ?? 1);\n").unwrap();
    std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o700)).unwrap();
    let inherited_path = std::env::var_os("PATH").unwrap();
    let paths = std::iter::once(bin).chain(std::env::split_paths(&inherited_path));
    let path = std::env::join_paths(paths).unwrap();
    for selected in [None, Some("development"), Some("production")] {
        let mut command = std::process::Command::new("task");
        command
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .args([
                "host:platform-admin:bootstrap",
                "EMAIL=unavailable@example.test",
                "OPERATOR=process-test",
                "REASON=Environment selection",
            ])
            .env("PATH", &path)
            .env("LOCO_CONFIG_FOLDER", &config)
            .env(
                "MEDTRACKER_BOOTSTRAP_TEST_BINARY",
                env!("CARGO_BIN_EXE_med-tracker"),
            )
            .env_remove("LOCO_ENV")
            .env_remove("RAILS_ENV")
            .env_remove("NODE_ENV");
        if let Some(selected) = selected {
            command.arg(format!("BOOTSTRAP_ENVIRONMENT={selected}"));
        }
        let output = command.output().unwrap();
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success());
        assert!(
            error.contains(&format!("{}_sentinel", selected.unwrap_or("production"))),
            "{error}"
        );
    }
}

#[test]
fn container_scheduler_rejects_invalid_arguments_before_starting() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scheduler.yaml");
    std::fs::write(
        &path,
        "jobs:\n  invalid:\n    run: \"support-access-expire value:bad\\0argument\"\n    schedule: '0 * * * * *'\n",
    )
    .unwrap();
    for arguments in [
        ["scheduler", "--environment", "test"].as_slice(),
        ["start", "--scheduler", "--environment", "test"].as_slice(),
    ] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_med-tracker"))
            .args(arguments)
            .env("SCHEDULER_CONFIG", &path)
            .env(
                "DATABASE_URL",
                "postgres://invalid:invalid@127.0.0.1:1/invalid",
            )
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("scheduler command contains a NUL"),
            "unexpected error: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_med-tracker"))
        .args(["scheduler", "--environment", "test", "--config"])
        .arg(&path)
        .env(
            "DATABASE_URL",
            "postgres://invalid:invalid@127.0.0.1:1/invalid",
        )
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("scheduler command contains a NUL"),
        "unexpected error: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let valid_path = directory.path().join("valid.yaml");
    std::fs::write(
        &valid_path,
        "jobs:\n  valid:\n    run: support-access-expire\n    schedule: '0 * * * * *'\n",
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_med-tracker"))
        .args([
            "start",
            "--scheduler",
            "--environment",
            "test",
            "--scheduler-config",
        ])
        .arg(valid_path)
        .env("SCHEDULER_CONFIG", &path)
        .env(
            "DATABASE_URL",
            "postgres://invalid:invalid@127.0.0.1:1/invalid",
        )
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unexpected argument"),
        "unexpected error: {}",
        String::from_utf8_lossy(&output.stderr)
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
        let config = loco_rs::scheduler::Config {
            jobs: [("invalid".into(), job(run))].into(),
            output: Output::STDOUT,
        };
        assert!(
            Scheduler::new::<App>(&config, &Environment::Test).is_err(),
            "invalid command was accepted: {run:?}"
        );
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
