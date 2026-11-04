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
const VERSION: &str = "pgss-cpu-v1";

#[cfg(any(target_os = "linux", test))]
fn parse_test_stat<'a>(
    line: &'a str,
    allowed: &std::collections::HashSet<String>,
) -> Option<(u32, &'a str, u64, u64, u64)> {
    let (pid, rest) = line.split_once(" (")?;
    let (name, fields) = rest.rsplit_once(") ")?;
    if !allowed.contains(name) {
        return None;
    }
    let fields: Vec<_> = fields.split_whitespace().collect();
    Some((
        pid.parse().ok()?,
        name,
        fields.get(19)?.parse().ok()?,
        fields.get(11)?.parse().ok()?,
        fields.get(12)?.parse().ok()?,
    ))
}

#[cfg(target_os = "linux")]
fn postgres_cgroup() -> Result<PathBuf> {
    let container = std::env::var("MEDTRACKER_PGSS_CONTAINER_ID")?;
    if container.len() != 64 || !container.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("invalid diagnostic container id".into());
    }
    let pid: u32 = std::env::var("MEDTRACKER_PGSS_POSTGRES_PID")?.parse()?;
    let memberships = fs::read_to_string(format!("/proc/{pid}/cgroup"))?;
    let relative = memberships
        .lines()
        .find_map(|line| line.strip_prefix("0::/"))
        .ok_or("PostgreSQL cgroup v2 unavailable")?;
    if !relative.contains(&container) {
        return Err("PostgreSQL cgroup does not match service container".into());
    }
    let root = fs::canonicalize("/sys/fs/cgroup")?;
    let path = fs::canonicalize(root.join(relative))?;
    if path == root || !path.starts_with(&root) || !path.join("cpu.stat").is_file() {
        return Err("invalid PostgreSQL cgroup path".into());
    }
    Ok(path)
}

#[cfg(not(target_os = "linux"))]
fn postgres_cgroup() -> Result<PathBuf> {
    Err("CPU capture requires Linux cgroup v2".into())
}

#[cfg(target_os = "linux")]
fn resource_metadata(cgroup: &std::path::Path) -> Result<Value> {
    let host = fs::read_to_string("/proc/stat")?;
    let cpu_count = host
        .lines()
        .filter(|line| {
            line.split_whitespace()
                .next()
                .and_then(|name| name.strip_prefix("cpu"))
                .is_some_and(|suffix| {
                    !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit())
                })
        })
        .count();
    if cpu_count == 0 {
        return Err("host CPU count unavailable".into());
    }
    let quota = fs::read_to_string(cgroup.join("cpu.max"))?;
    let cpuset = fs::read_to_string(cgroup.join("cpuset.cpus.effective"))?;
    Ok(
        json!({"host_cpu_count":cpu_count,"postgres_cpu_max":quota.trim(),"postgres_cpuset":cpuset.trim()}),
    )
}

#[cfg(not(target_os = "linux"))]
fn resource_metadata(_cgroup: &std::path::Path) -> Result<Value> {
    Err("CPU capture requires Linux".into())
}

#[cfg(target_os = "linux")]
fn cpu_sample(
    cgroup: &std::path::Path,
    elapsed: Duration,
    allowed: &std::collections::HashSet<String>,
) -> Result<Value> {
    let stat = fs::read_to_string("/proc/stat")?;
    let aggregate = stat.lines().next().ok_or("host CPU counters missing")?;
    let mut fields = aggregate.split_whitespace();
    if fields.next() != Some("cpu") {
        return Err("host CPU counters invalid".into());
    }
    let host = fields
        .take(8)
        .map(str::parse::<u64>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if host.len() != 8 {
        return Err("host CPU counters incomplete".into());
    }
    let mut pg_cpu = std::collections::BTreeMap::new();
    for line in fs::read_to_string(cgroup.join("cpu.stat"))?.lines() {
        let mut parts = line.split_whitespace();
        if let Some(key @ ("usage_usec" | "user_usec" | "system_usec" | "throttled_usec")) =
            parts.next()
        {
            pg_cpu.insert(
                key.to_owned(),
                parts
                    .next()
                    .ok_or("cgroup CPU value missing")?
                    .parse::<u64>()?,
            );
        }
    }
    if !pg_cpu.contains_key("usage_usec") {
        return Err("cgroup CPU usage missing".into());
    }
    let memory: u64 = fs::read_to_string(cgroup.join("memory.current"))?
        .trim()
        .parse()?;
    let mut processes = Vec::new();
    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        if !entry
            .file_name()
            .to_string_lossy()
            .bytes()
            .all(|b| b.is_ascii_digit())
        {
            continue;
        }
        let Ok(text) = fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        if let Some((pid, comm, start_ticks, user_ticks, system_ticks)) =
            parse_test_stat(&text, allowed)
        {
            processes.push(json!({"pid":pid,"comm":comm,"start_ticks":start_ticks,"user_ticks":user_ticks,"system_ticks":system_ticks}));
        }
    }
    Ok(
        json!({"kind":"cpu", "unix_ms":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis(), "elapsed_ms":elapsed.as_millis(), "host_jiffies":host, "postgres_cpu_usec":pg_cpu, "postgres_memory_bytes":memory, "test_processes":processes}),
    )
}

#[cfg(not(target_os = "linux"))]
fn cpu_sample(
    _cgroup: &std::path::Path,
    _elapsed: Duration,
    _allowed: &std::collections::HashSet<String>,
) -> Result<Value> {
    Err("CPU capture requires Linux".into())
}

#[cfg(test)]
mod cpu_tests {
    use super::parse_test_stat;

    #[test]
    fn parses_only_allowlisted_processes_and_preserves_counter_positions() {
        let line = "42 (care_api-abcdef) S 2 3 4 5 6 7 8 9 10 11 120 130 14 15 16 17 18 19 9876 21";
        let names = ["care_api-abcdef".to_owned()].into_iter().collect();
        assert_eq!(
            parse_test_stat(line, &names),
            Some((42, "care_api-abcdef", 9876, 120, 130))
        );
        assert_eq!(
            parse_test_stat(&line.replace("care_api-abcdef", "other-process"), &names),
            None
        );
        assert_eq!(parse_test_stat("42 (care_api-abcdef) S 2", &names), None);
    }
}

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
            "*",
            "--message-format",
            "json",
        ])
        .stderr(Stdio::null())
        .output()?;
    if !output.status.success() {
        return Err("diagnostic inventory failed".into());
    }
    let list: Value = serde_json::from_slice(&output.stdout)?;
    let suites = list
        .get("rust-suites")
        .and_then(Value::as_object)
        .ok_or("binary inventory missing")?;
    let mut binaries = std::collections::BTreeMap::new();
    for (id, suite) in suites {
        let path = suite
            .get("binary-path")
            .and_then(Value::as_str)
            .ok_or("binary path missing")?;
        let name = std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("binary name invalid")?;
        if !name.is_ascii() {
            return Err("binary name is not ASCII".into());
        }
        let comm = name.chars().take(15).collect::<String>();
        if comm.is_empty() || binaries.insert(comm, id.clone()).is_some() {
            return Err("binary process name collision".into());
        }
    }
    let cases = suites
        .get("med-tracker::care_api")
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
    let mut binary_insert = Query::insert();
    binary_insert
        .into_table(("diagnostic_ci", "capture_binary_names"))
        .columns(["comm"]);
    for comm in binaries.keys() {
        binary_insert.values_panic([comm.clone().into()]);
    }
    db.execute_raw(db.get_database_backend().build(&binary_insert))
        .await?;
    let payload = json!({"capture_version": VERSION, "github_run_id":std::env::var("GITHUB_RUN_ID")?, "github_sha":std::env::var("GITHUB_SHA")?, "tests":entries, "binaries":binaries});
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
        "baseline": baseline, "info": info, "statements": statements,
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
    let cgroup = postgres_cgroup()?;
    let ticks = Command::new("getconf").arg("CLK_TCK").output()?;
    if !ticks.status.success() {
        return Err("clock tick frequency unavailable".into());
    }
    let ticks: u64 = std::str::from_utf8(&ticks.stdout)?.trim().parse()?;
    let resources = resource_metadata(&cgroup)?;
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
    let binary_query = Query::select()
        .expr_as(Func::cust("to_jsonb").arg(Expr::col("comm")), "value")
        .from(("diagnostic_ci", "capture_binary_names"))
        .to_owned();
    let binary_names: std::collections::HashSet<String> = json_rows(&db, binary_query)
        .await?
        .into_iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or("invalid binary inventory")
        })
        .collect::<std::result::Result<_, _>>()?;
    if binary_names.is_empty() {
        return Err("diagnostic binary inventory missing".into());
    }
    let metadata = json!({"capture_version": VERSION, "github_run_id": std::env::var("GITHUB_RUN_ID")?, "github_sha": std::env::var("GITHUB_SHA")?, "clock_ticks_per_second": ticks, "resources": resources});
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
            serde_json::to_writer(
                &mut input,
                &cpu_sample(&cgroup, start.elapsed(), &binary_names)?,
            )?;
            input.write_all(b"\n")?;
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
            for row in json_rows(&db, query).await? {
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
            if !ready {
                input.flush()?;
                fs::write(directory.join("sampler.ready"), b"ready\n")?;
                ready = true;
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
