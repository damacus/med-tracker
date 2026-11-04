use sea_orm::sea_query::{Expr, ExprTrait, Func, Query, SelectStatement};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const VERSION: &str = "pgss-functions-ci-v1";

fn directory() -> Result<PathBuf> {
    let path = PathBuf::from(std::env::var("MEDTRACKER_PGSS_DIR")?);
    if !path.is_absolute() || !path.is_dir() {
        return Err("diagnostic directory unavailable".into());
    }
    Ok(path)
}

async fn connection(uri: &str) -> Result<DatabaseConnection> {
    let mut uri = url::Url::parse(uri)?;
    uri.set_path("/postgres");
    Ok(Database::connect(uri.as_str()).await?)
}

async fn json_rows(db: &DatabaseConnection, query: SelectStatement) -> Result<Vec<Value>> {
    db.query_all_raw(db.get_database_backend().build(&query))
        .await?
        .into_iter()
        .map(|row| row.try_get("", "value").map_err(Into::into))
        .collect()
}

fn encrypt(path: &std::path::Path, payload: &[u8]) -> Result<()> {
    let temp = path.with_extension("age.partial");
    let file = fs::File::create(&temp)?;
    let mut child = Command::new("age")
        .arg("-r")
        .arg(std::env::var("MEDTRACKER_PGSS_RECIPIENT")?)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(file))
        .stderr(Stdio::null())
        .spawn()?;
    let result = child
        .stdin
        .take()
        .ok_or("encryptor stdin unavailable")?
        .write_all(payload);
    let status = child.wait()?;
    if result.is_err() || !status.success() {
        let _ = fs::remove_file(temp);
        return Err("diagnostic encryption failed".into());
    }
    fs::rename(temp, path)?;
    Ok(())
}

pub async fn inventory(control_uri: &str) -> Result<()> {
    let output = Command::new("cargo")
        .args([
            "nextest",
            "list",
            "--locked",
            "--profile",
            "api",
            "--test",
            "care_api",
            "--message-format",
            "json",
        ])
        .stderr(Stdio::null())
        .output()?;
    if !output.status.success() {
        return Err("diagnostic inventory failed".into());
    }
    let list: Value = serde_json::from_slice(&output.stdout)?;
    let cases = list
        .get("rust-suites")
        .and_then(|s| s.get("med-tracker::care_api"))
        .and_then(|s| s.get("testcases"))
        .and_then(Value::as_object)
        .ok_or("test inventory missing")?;
    if cases.is_empty() {
        return Err("test inventory empty".into());
    }
    let mut fingerprints = std::collections::HashSet::new();
    let mut entries = Vec::new();
    let mut insert = Query::insert();
    insert
        .into_table(("diagnostic_ci", "capture_test_hashes"))
        .columns(["fingerprint"]);
    for test in cases.keys() {
        let fingerprint = hex::encode(Sha256::digest(test.as_bytes()))[..16].to_owned();
        if !fingerprints.insert(fingerprint.clone()) {
            return Err("test fingerprint collision".into());
        }
        insert.values_panic([fingerprint.clone().into()]);
        entries.push(
            json!({"binary":"med-tracker::care_api", "testname":test, "fingerprint":fingerprint}),
        );
    }
    let db = connection(control_uri).await?;
    db.execute_raw(db.get_database_backend().build(&insert))
        .await?;
    let payload = json!({"capture_version": VERSION, "github_run_id":std::env::var("GITHUB_RUN_ID")?, "github_sha":std::env::var("GITHUB_SHA")?, "tests":entries});
    encrypt(
        &directory()?.join("inventory.age"),
        &serde_json::to_vec(&payload)?,
    )?;
    db.close().await?;
    Ok(())
}

pub async fn snapshot(control_uri: &str, database: &str, run: uuid::Uuid) -> Result<()> {
    let db = connection(control_uri).await?;
    let database_oid = Query::select()
        .column("oid")
        .from(("pg_catalog", "pg_database"))
        .and_where(Expr::col("datname").eq(database))
        .to_owned();
    let stats = Query::select()
        .expr_as(
            Func::cust("jsonb_build_object").args([
                Expr::val("queryid_text"),
                Expr::col(("s", "queryid")).cast_as("text"),
                Expr::val("stats"),
                Expr::expr(Func::cust("to_jsonb").arg(Expr::col("s"))),
            ]),
            "value",
        )
        .from_as(("diagnostic_ci", "pg_stat_statements"), "s")
        .and_where(Expr::col(("s", "dbid")).in_subquery(database_oid))
        .to_owned();
    let statements = json_rows(&db, stats).await?;
    if statements.is_empty() {
        return Err("diagnostic statistics missing".into());
    }
    let mut run_uri = url::Url::parse(control_uri)?;
    run_uri.set_path(&format!("/{database}"));
    let run_db = Database::connect(run_uri.as_str()).await?;
    let functions = Query::select()
        .expr_as(
            Func::cust("jsonb_build_object").args([
                Expr::val("schema"),
                Expr::col("schemaname"),
                Expr::val("name"),
                Expr::col("funcname"),
                Expr::val("calls"),
                Expr::col("calls"),
                Expr::val("total_time"),
                Expr::col("total_time"),
                Expr::val("self_time"),
                Expr::col("self_time"),
            ]),
            "value",
        )
        .from(("pg_catalog", "pg_stat_user_functions"))
        .to_owned();
    let functions = json_rows(&run_db, functions).await?;
    run_db.close().await?;
    if functions.is_empty() {
        return Err("diagnostic function statistics missing".into());
    }
    let roles = Query::select()
        .expr_as(
            Func::cust("jsonb_build_object").args([
                Expr::val("oid"),
                Expr::col("oid"),
                Expr::val("name"),
                Expr::col("rolname"),
            ]),
            "value",
        )
        .from(("pg_catalog", "pg_roles"))
        .to_owned();
    let info = Query::select()
        .expr_as(Func::cust("to_jsonb").arg(Expr::col("i")), "value")
        .from_as(("diagnostic_ci", "pg_stat_statements_info"), "i")
        .to_owned();
    let mut settings = Query::select();
    for setting in [
        "server_version",
        "shared_preload_libraries",
        "compute_query_id",
        "track_functions",
        "pg_stat_statements.track",
        "pg_stat_statements.track_utility",
        "pg_stat_statements.track_planning",
        "pg_stat_statements.max",
    ] {
        settings.expr_as(
            Func::cust("current_setting").arg(Expr::val(setting)),
            setting,
        );
    }
    let row = db
        .query_one_raw(db.get_database_backend().build(&settings))
        .await?
        .ok_or("settings missing")?;
    let mut captured_settings = serde_json::Map::new();
    for setting in [
        "server_version",
        "shared_preload_libraries",
        "compute_query_id",
        "track_functions",
        "pg_stat_statements.track",
        "pg_stat_statements.track_utility",
        "pg_stat_statements.track_planning",
        "pg_stat_statements.max",
    ] {
        captured_settings.insert(
            setting.to_owned(),
            json!(row.try_get::<String>("", setting)?),
        );
    }
    let info = json_rows(&db, info).await?;
    let baseline_query = Query::select()
        .expr_as(Func::cust("to_jsonb").arg(Expr::col("b")), "value")
        .from_as(("diagnostic_ci", "capture_baseline"), "b")
        .to_owned();
    let baseline = json_rows(&db, baseline_query).await?;
    let final_info = info.first().ok_or("diagnostic final counters missing")?;
    let initial = baseline.first().ok_or("diagnostic baseline missing")?;
    let complete = final_info.get("dealloc") == initial.get("dealloc")
        && final_info.get("stats_reset") == initial.get("stats_reset")
        && final_info.get("dealloc").is_some()
        && final_info.get("stats_reset").is_some();
    let payload = json!({
        "capture_version": VERSION, "github_run_id": std::env::var("GITHUB_RUN_ID")?,
        "github_sha": std::env::var("GITHUB_SHA")?, "fixture_run_id": run.to_string(),
        "database": database, "settings": captured_settings, "roles": json_rows(&db, roles).await?,
        "baseline": baseline, "info": info, "statements": statements, "functions": functions,
    });
    encrypt(
        &directory()?.join("statement-stats.age"),
        &serde_json::to_vec(&payload)?,
    )?;
    if !complete {
        return Err("diagnostic statistics reset or evicted".into());
    }
    fs::write(directory()?.join("snapshot.ok"), b"complete\n")?;
    db.close().await?;
    Ok(())
}

pub async fn sample(control_uri: &str) -> Result<()> {
    let directory = directory()?;
    let db = connection(control_uri).await?;
    let hashes = Query::select()
        .expr_as(
            Func::cust("to_jsonb").arg(Expr::col("fingerprint")),
            "value",
        )
        .from(("diagnostic_ci", "capture_test_hashes"))
        .to_owned();
    let hashes: std::collections::HashSet<String> = json_rows(&db, hashes)
        .await?
        .into_iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or("invalid test fingerprint")
        })
        .collect::<std::result::Result<_, _>>()?;
    if hashes.is_empty() {
        return Err("diagnostic test inventory missing".into());
    }
    let metadata = json!({"capture_version": VERSION, "github_run_id": std::env::var("GITHUB_RUN_ID")?, "github_sha": std::env::var("GITHUB_SHA")?});
    let partial = directory.join("activity.age.partial");
    let file = fs::File::create(&partial)?;
    let mut child = Command::new("age")
        .arg("-r")
        .arg(std::env::var("MEDTRACKER_PGSS_RECIPIENT")?)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(file))
        .stderr(Stdio::null())
        .spawn()?;
    let mut input = child.stdin.take().ok_or("encryptor stdin unavailable")?;
    let result: Result<usize> = async {
        serde_json::to_writer(&mut input, &metadata)?;
        input.write_all(b"\n")?;
        let start = Instant::now();
        let mut samples = 0usize;
        let mut ready = false;
        while !directory.join("stop").exists() && start.elapsed() < Duration::from_secs(1900) {
            let owned = Query::select()
                .column("oid")
                .from(("pg_catalog", "pg_database"))
                .and_where(
                    Expr::expr(
                        Func::cust("shobj_description")
                            .args([Expr::col("oid"), Expr::val("pg_database")]),
                    )
                    .like("medtracker-api-run:%"),
                )
                .to_owned();
            let query = Query::select()
                .expr_as(
                    Func::cust("jsonb_build_object").args([
                        Expr::val("utc"),
                        Expr::expr(Func::cust("clock_timestamp")),
                        Expr::val("pid"),
                        Expr::col("pid"),
                        Expr::val("userid"),
                        Expr::col("usesysid"),
                        Expr::val("dbid"),
                        Expr::col("datid"),
                        Expr::val("application_name"),
                        Expr::col("application_name"),
                        Expr::val("queryid"),
                        Expr::col("query_id"),
                        Expr::val("queryid_text"),
                        Expr::col("query_id").cast_as("text"),
                        Expr::val("query_start"),
                        Expr::col("query_start"),
                        Expr::val("query_age_ms"),
                        Expr::expr(Func::cust("date_part").args([
                            Expr::val("epoch"),
                            Expr::expr(Func::cust("clock_timestamp")).sub(Expr::col("query_start")),
                        ]))
                        .mul(Expr::val(1000)),
                        Expr::val("state"),
                        Expr::col("state"),
                        Expr::val("wait_type"),
                        Expr::col("wait_event_type"),
                        Expr::val("wait"),
                        Expr::col("wait_event"),
                        Expr::val("blockers"),
                        Expr::expr(Func::cust("pg_blocking_pids").arg(Expr::col("pid"))),
                    ]),
                    "value",
                )
                .from(("pg_catalog", "pg_stat_activity"))
                .and_where(Expr::col("datid").in_subquery(owned))
                .and_where(Expr::col("application_name").like("mt\\_diag\\_%"))
                .to_owned();
            let rows = json_rows(&db, query).await?;
            if !ready {
                fs::write(directory.join("sampler.ready"), b"ready\n")?;
                ready = true;
            }
            for row in rows {
                let tag = row
                    .get("application_name")
                    .and_then(Value::as_str)
                    .ok_or("diagnostic tag missing")?;
                let fingerprint = tag.rsplit('_').next().ok_or("diagnostic tag invalid")?;
                if !hashes.contains(fingerprint) {
                    return Err("diagnostic tag not in test inventory".into());
                }
                serde_json::to_writer(&mut input, &row)?;
                input.write_all(b"\n")?;
                samples += 1;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        serde_json::to_writer(
            &mut input,
            &json!({"sample_count": samples, "stopped": directory.join("stop").exists()}),
        )?;
        input.write_all(b"\n")?;
        Ok(samples)
    }
    .await;
    drop(input);
    let samples = match result {
        Ok(samples) => samples,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = fs::remove_file(partial);
            return Err(error);
        }
    };
    let status = child.wait()?;
    if !status.success() || samples == 0 || !directory.join("stop").exists() {
        let _ = fs::remove_file(partial);
        return Err("diagnostic sampling failed".into());
    }
    fs::rename(partial, directory.join("activity.age"))?;
    db.close().await?;
    Ok(())
}
