#[path = "support/template.rs"]
mod template;

use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

fn wait_for_worker(mut child: Child) -> Output {
    let start = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if start.elapsed() >= Duration::from_secs(90) {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
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
async fn worker() {
    let uri = template::owned_database_url();
    let name = std::env::var("TEMPLATE_REGRESSION_NAME").unwrap();
    let marker = std::env::var("TEMPLATE_REGRESSION_MARKER").unwrap();
    template::prepare(&uri, &name, |database| async move {
        database
            .execute_unprepared(
                "CREATE TABLE guardrail_seed_probe (id integer PRIMARY KEY); INSERT INTO guardrail_seed_probe (id) VALUES (1)",
            )
            .await
            .unwrap();
        std::fs::write(marker, "seeded").unwrap();
        std::thread::sleep(Duration::from_secs(3));
        assert_ne!(std::env::var("TEMPLATE_REGRESSION_PANIC").as_deref(), Ok("1"));
    })
    .await;
}

#[test]
#[ignore]
fn competing_processes_prepare_once() {
    let uri = template::owned_database_url();
    let name = std::env::var("TEMPLATE_REGRESSION_NAME").unwrap();
    let marker = std::env::var("TEMPLATE_REGRESSION_MARKER").unwrap();
    let executable = std::env::current_exe().unwrap();
    let mut first = Command::new(&executable)
        .args(["--ignored", "--exact", "worker", "--nocapture"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    while !std::path::Path::new(&marker).exists() {
        if first.try_wait().unwrap().is_some() {
            let output = first.wait_with_output().unwrap();
            panic!(
                "first worker exited before seed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        if start.elapsed() >= Duration::from_secs(90) {
            first.kill().unwrap();
            let output = first.wait_with_output().unwrap();
            panic!(
                "first worker never reached seed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let second = Command::new(&executable)
        .args(["--ignored", "--exact", "worker", "--nocapture"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let first_output = wait_for_worker(first);
    let second_output = wait_for_worker(second);
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

    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let control = Database::connect(&uri).await.unwrap();
        let row = control.query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT datallowconn FROM pg_database WHERE datname=$1",
            [name.clone().into()],
        )).await.unwrap().unwrap();
        assert!(!row.try_get::<bool>("", "datallowconn").unwrap());

        let suffix = uuid::Uuid::new_v4().simple();
        let clone_name = format!("template_clone_{suffix}");
        let role = format!("template_runtime_{suffix}");
        control.execute_unprepared(&format!("CREATE DATABASE {clone_name} TEMPLATE {name}")).await.unwrap();
        control.execute_unprepared(&format!("CREATE ROLE {role} LOGIN PASSWORD 'password' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS IN ROLE med_tracker_app")).await.unwrap();
        let mut clone_uri = url::Url::parse(&uri).unwrap();
        clone_uri.set_path(&clone_name);
        let clone = Database::connect(clone_uri.to_string()).await.unwrap();
        let seeded = clone.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT count(*) AS seeds FROM guardrail_seed_probe")).await.unwrap().unwrap();
        assert_eq!(seeded.try_get::<i64>("", "seeds").unwrap(), 1);
        clone.close().await.unwrap();

        clone_uri.set_username(&role).unwrap();
        clone_uri.set_password(Some("password")).unwrap();
        let restricted = Database::connect(clone_uri.to_string()).await.unwrap();
        let people = restricted.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT count(*) AS people FROM people")).await.unwrap().unwrap();
        assert_eq!(people.try_get::<i64>("", "people").unwrap(), 0);
        restricted.close().await.unwrap();
        let role_row = control.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT rolsuper, rolbypassrls FROM pg_roles WHERE rolname=$1", [role.clone().into()])).await.unwrap().unwrap();
        assert!(!role_row.try_get::<bool>("", "rolsuper").unwrap());
        assert!(!role_row.try_get::<bool>("", "rolbypassrls").unwrap());
        control.execute_unprepared(&format!("DROP DATABASE {clone_name} WITH (FORCE)")).await.unwrap();
        control.execute_unprepared(&format!("DROP ROLE {role}")).await.unwrap();
        template::prepare(&uri, &name, |_| async { panic!("frozen template should skip seed") }).await;
        let other_name = format!("{name}_other");
        template::prepare(&uri, &other_name, |database| async move {
            database.execute_unprepared("CREATE TABLE other_seed_probe (id integer PRIMARY KEY)").await.unwrap();
        }).await;
        let other_row = control.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT datallowconn FROM pg_database WHERE datname=$1", [other_name.into()])).await.unwrap().unwrap();
        assert!(!other_row.try_get::<bool>("", "datallowconn").unwrap());
        control.close().await.unwrap();
    });

    let failed_name = format!("{name}_failed");
    std::fs::remove_file(&marker).unwrap();
    let failed = Command::new(&executable)
        .args(["--ignored", "--exact", "worker", "--nocapture"])
        .env("TEMPLATE_REGRESSION_NAME", &failed_name)
        .env("TEMPLATE_REGRESSION_PANIC", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let failed_output = wait_for_worker(failed);
    assert!(!failed_output.status.success());
    assert!(std::path::Path::new(&marker).exists());
    let retry = Command::new(&executable)
        .args(["--ignored", "--exact", "worker", "--nocapture"])
        .env("TEMPLATE_REGRESSION_NAME", &failed_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let retry_output = wait_for_worker(retry);
    assert!(
        !retry_output.status.success(),
        "incomplete template was accepted"
    );
    assert!(String::from_utf8_lossy(&retry_output.stderr).contains("datallowconn"));
    runtime.block_on(async {
        let control = Database::connect(&uri).await.unwrap();
        let failed = control
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT datallowconn FROM pg_database WHERE datname=$1",
                [failed_name.into()],
            ))
            .await
            .unwrap()
            .unwrap();
        assert!(failed.try_get::<bool>("", "datallowconn").unwrap());
        let reference = control
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT 1 AS present FROM pg_database WHERE datname='medtracker_reference'",
            ))
            .await
            .unwrap();
        assert!(reference.is_some());
        control.close().await.unwrap();
    });
}
