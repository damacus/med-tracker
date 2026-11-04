use sea_orm::sea_query::{Expr, ExprTrait, Func, Query};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use std::{error::Error, fs::OpenOptions, io::Write};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn cleanup_stage(stage: &str) {
    let Some(path) = std::env::var_os("MEDTRACKER_CLEANUP_STAGE_FILE") else {
        return;
    };
    let Ok(elapsed) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
        return;
    };
    if let Ok(mut output) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(output, "{} {}", elapsed.as_nanos(), stage);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    assert_eq!(
        std::env::var("MEDTRACKER_OWNED_DATABASE").as_deref(),
        Ok("1")
    );
    let run = uuid::Uuid::parse_str(&std::env::var("MEDTRACKER_TEST_RUN_ID")?)?;
    let database = format!("api_run_{}", run.simple());
    let role = format!("api_runtime_{}", run.simple());
    let marker = format!("medtracker-api-run:{run}");
    let mut control_uri = std::env::var("DATABASE_URL")?;
    let command = std::env::args().nth(1);
    if command.as_deref() == Some("cleanup")
        && std::env::var_os("MEDTRACKER_CLEANUP_STAGE_FILE").is_some()
    {
        let mut uri = url::Url::parse(&control_uri)?;
        uri.query_pairs_mut()
            .append_pair("application_name", "medtracker_cleanup_probe");
        control_uri = uri.into();
    }
    if command.as_deref() == Some("cleanup") {
        cleanup_stage("connect_start");
    }
    let control_result = Database::connect(&control_uri).await;
    if command.as_deref() == Some("cleanup") {
        cleanup_stage("connect_end");
    }
    let control = control_result?;
    match command.as_deref() {
        Some("prepare") => {
            if database_marker(&control, &database).await?.is_some() {
                return Err("Refusing to adopt an existing API test database".into());
            }
            // SeaORM has no database/login-role administration API. These fixed
            // PostgreSQL lifecycle statements contain only UUID-derived names.
            control
                .execute_unprepared(&format!("CREATE DATABASE {database}"))
                .await?;
            control
                .execute_unprepared(&format!("COMMENT ON DATABASE {database} IS '{marker}'"))
                .await?;
            let mut setup_uri = url::Url::parse(&control_uri)?;
            setup_uri.set_path(&database);
            let setup = Database::connect(setup_uri.as_str()).await?;
            loco_rs::db::migrate::<migration::Migrator>(&setup).await?;
            control.execute_unprepared(&format!("CREATE ROLE {role} LOGIN PASSWORD 'password' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS IN ROLE med_tracker_app; COMMENT ON ROLE {role} IS '{marker}'")).await?;
            let mut runtime_uri = setup_uri.clone();
            runtime_uri
                .set_username(&role)
                .map_err(|()| "runtime username")?;
            runtime_uri
                .set_password(Some("password"))
                .map_err(|()| "runtime password")?;
            let mut environment = OpenOptions::new()
                .append(true)
                .open(std::env::var("NEXTEST_ENV")?)?;
            writeln!(
                environment,
                "MEDTRACKER_TEST_SETUP_DATABASE_URL={setup_uri}"
            )?;
            writeln!(environment, "DATABASE_URL={runtime_uri}")?;
            setup.close().await?;
        }
        Some("cleanup") => {
            cleanup_stage("database_marker_start");
            let database_marker_result = database_marker(&control, &database).await;
            cleanup_stage("database_marker_end");
            if let Some(owned) = database_marker_result? {
                if owned != marker {
                    return Err("Refusing to remove an unowned database".into());
                }
                cleanup_stage("drop_database_start");
                let result = control
                    .execute_unprepared(&format!("DROP DATABASE {database} WITH (FORCE)"))
                    .await;
                cleanup_stage("drop_database_end");
                result?;
            }
            let role_query = Query::select()
                .expr_as(
                    Func::cust("shobj_description")
                        .args([Expr::col("oid"), Expr::val("pg_authid")]),
                    "owner",
                )
                .from(("pg_catalog", "pg_roles"))
                .and_where(Expr::col("rolname").eq(&role))
                .to_owned();
            cleanup_stage("role_marker_start");
            let role_marker_result = control
                .query_one_raw(control.get_database_backend().build(&role_query))
                .await;
            cleanup_stage("role_marker_end");
            if let Some(row) = role_marker_result? {
                if row.try_get::<Option<String>>("", "owner")?.as_deref() != Some(marker.as_str()) {
                    return Err("Refusing to remove an unowned role".into());
                }
                cleanup_stage("drop_role_start");
                let result = control
                    .execute_unprepared(&format!("DROP ROLE {role}"))
                    .await;
                cleanup_stage("drop_role_end");
                result?;
            }
        }
        _ => return Err("Use prepare or cleanup".into()),
    }
    if command.as_deref() == Some("cleanup") {
        cleanup_stage("close_start");
    }
    let close_result = control.close().await;
    if command.as_deref() == Some("cleanup") {
        cleanup_stage("close_end");
    }
    close_result?;
    Ok(())
}

async fn database_marker(control: &DatabaseConnection, database: &str) -> Result<Option<String>> {
    let query = Query::select()
        .expr_as(
            Func::cust("shobj_description").args([Expr::col("oid"), Expr::val("pg_database")]),
            "owner",
        )
        .from(("pg_catalog", "pg_database"))
        .and_where(Expr::col("datname").eq(database))
        .to_owned();
    let row = control
        .query_one_raw(control.get_database_backend().build(&query))
        .await?;
    Ok(row
        .map(|row| row.try_get::<Option<String>>("", "owner"))
        .transpose()?
        .map(|owner| owner.unwrap_or_default()))
}
