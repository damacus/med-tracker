#[path = "support/template.rs"]
mod template;

use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn wait_for_worker(mut child: Child) -> TestResult<Output> {
    let start = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            return Ok(child.wait_with_output()?);
        }
        if start.elapsed() >= Duration::from_secs(90) {
            child.kill()?;
            let output = child.wait_with_output()?;
            panic!(
                "worker exceeded 90 seconds: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[tokio::test]
#[ignore]
async fn worker() -> TestResult {
    let uri = template::owned_database_url();
    let name = std::env::var("TEMPLATE_REGRESSION_NAME")?;
    let marker = std::env::var("TEMPLATE_REGRESSION_MARKER")?;
    template::prepare(&uri, &name, |database| async move {
        database
            .execute_unprepared(
                "CREATE TABLE guardrail_seed_probe (id integer PRIMARY KEY); INSERT INTO guardrail_seed_probe (id) VALUES (1)",
            )
            .await
            .expect("create seed probe");
        std::fs::write(marker, "seeded").expect("write seed marker");
        std::thread::sleep(Duration::from_secs(3));
        assert_ne!(std::env::var("TEMPLATE_REGRESSION_PANIC").as_deref(), Ok("1"));
    })
    .await;
    Ok(())
}

#[test]
fn competing_processes_prepare_once() -> TestResult {
    let uri = template::owned_database_url();
    let tempdir = tempfile::tempdir()?;
    let suffix: String = uuid::Uuid::new_v4()
        .simple()
        .to_string()
        .bytes()
        .map(|byte| {
            let digit = match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                _ => unreachable!(),
            };
            char::from(b'a' + digit)
        })
        .collect();
    let name = std::env::var("TEMPLATE_REGRESSION_NAME")
        .unwrap_or_else(|_| format!("template_guard_{suffix}"));
    let marker = std::env::var("TEMPLATE_REGRESSION_MARKER")
        .unwrap_or_else(|_| tempdir.path().join("seeded").to_string_lossy().into_owned());
    let executable = std::env::current_exe()?;
    let mut first = Command::new(&executable)
        .args(["--ignored", "--exact", "worker", "--nocapture"])
        .env("TEMPLATE_REGRESSION_NAME", &name)
        .env("TEMPLATE_REGRESSION_MARKER", &marker)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let start = Instant::now();
    while !std::path::Path::new(&marker).exists() {
        if first.try_wait()?.is_some() {
            let output = first.wait_with_output()?;
            panic!(
                "first worker exited before seed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        if start.elapsed() >= Duration::from_secs(90) {
            first.kill()?;
            let output = first.wait_with_output()?;
            panic!(
                "first worker never reached seed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let second = Command::new(&executable)
        .args(["--ignored", "--exact", "worker", "--nocapture"])
        .env("TEMPLATE_REGRESSION_NAME", &name)
        .env("TEMPLATE_REGRESSION_MARKER", &marker)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let first_output = wait_for_worker(first);
    let second_output = wait_for_worker(second);
    let first_output = first_output?;
    let second_output = second_output?;
    assert!(
        first_output.status.success(),
        "first worker: {}",
        String::from_utf8_lossy(&first_output.stderr)
    );
    assert!(
        second_output.status.success(),
        "second worker: {}",
        String::from_utf8_lossy(&second_output.stderr)
    );

    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        let control = Database::connect(&uri).await?;
        let row = control.query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT datallowconn FROM pg_database WHERE datname=$1",
            [name.clone().into()],
        )).await?.ok_or("missing query row")?;
        assert!(!row.try_get::<bool>("", "datallowconn")?);

        let suffix = uuid::Uuid::new_v4().simple();
        let clone_name = format!("template_clone_{suffix}");
        let role = format!("template_runtime_{suffix}");
        control.execute_unprepared(&format!("CREATE DATABASE {clone_name} TEMPLATE {name}")).await?;
        control.execute_unprepared(&format!("CREATE ROLE {role} LOGIN PASSWORD 'password' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS IN ROLE med_tracker_app")).await?;
        let mut clone_uri = url::Url::parse(&uri)?;
        clone_uri.set_path(&clone_name);
        let clone = Database::connect(clone_uri.to_string()).await?;
        let seeded = clone.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT count(*) AS seeds FROM guardrail_seed_probe")).await?.ok_or("missing query row")?;
        assert_eq!(seeded.try_get::<i64>("", "seeds")?, 1);
        clone.close().await?;

        clone_uri.set_username(&role).map_err(|()| "set runtime username")?;
        clone_uri.set_password(Some("password")).map_err(|()| "set runtime password")?;
        let restricted = Database::connect(clone_uri.to_string()).await?;
        let people = restricted.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT count(*) AS people FROM people")).await?.ok_or("missing query row")?;
        assert_eq!(people.try_get::<i64>("", "people")?, 0);
        restricted.close().await?;
        let role_row = control.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT rolsuper, rolbypassrls FROM pg_roles WHERE rolname=$1", [role.clone().into()])).await?.ok_or("missing query row")?;
        assert!(!role_row.try_get::<bool>("", "rolsuper")?);
        assert!(!role_row.try_get::<bool>("", "rolbypassrls")?);
        control.execute_unprepared(&format!("DROP DATABASE {clone_name} WITH (FORCE)")).await?;
        control.execute_unprepared(&format!("DROP ROLE {role}")).await?;
        template::prepare(&uri, &name, |_| async { panic!("frozen template should skip seed") }).await;
        let other_name = format!("{name}_other");
        template::prepare(&uri, &other_name, |database| async move {
            database.execute_unprepared("CREATE TABLE other_seed_probe (id integer PRIMARY KEY)").await.expect("create independent probe");
        }).await;
        let other_row = control.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT datallowconn FROM pg_database WHERE datname=$1", [other_name.into()])).await?.ok_or("missing query row")?;
        assert!(!other_row.try_get::<bool>("", "datallowconn")?);
        control.close().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;

    let failed_name = format!("{name}_failed");
    std::fs::remove_file(&marker)?;
    let failed = Command::new(&executable)
        .args(["--ignored", "--exact", "worker", "--nocapture"])
        .env("TEMPLATE_REGRESSION_NAME", &failed_name)
        .env("TEMPLATE_REGRESSION_PANIC", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let failed_output = wait_for_worker(failed)?;
    assert!(!failed_output.status.success());
    assert!(std::path::Path::new(&marker).exists());
    let retry = Command::new(&executable)
        .args(["--ignored", "--exact", "worker", "--nocapture"])
        .env("TEMPLATE_REGRESSION_NAME", &failed_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let retry_output = wait_for_worker(retry)?;
    assert!(
        !retry_output.status.success(),
        "incomplete template was accepted"
    );
    assert!(String::from_utf8_lossy(&retry_output.stderr).contains("datallowconn"));
    runtime.block_on(async {
        let control = Database::connect(&uri).await?;
        let failed = control
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT datallowconn FROM pg_database WHERE datname=$1",
                [failed_name.into()],
            ))
            .await?
            .ok_or("missing query row")?;
        assert!(failed.try_get::<bool>("", "datallowconn")?);
        let reference = control
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT 1 AS present FROM pg_database WHERE datname='medtracker_reference'",
            ))
            .await?;
        assert!(reference.is_some());
        control.close().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;
    Ok(())
}
