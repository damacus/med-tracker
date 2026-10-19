use super::*;
use sea_orm::TransactionTrait;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};
use tracing::instrument::WithSubscriber;
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

#[derive(Clone, Default)]
struct ImportTimingEvents(Arc<Mutex<Vec<serde_json::Value>>>);

#[derive(Default)]
struct ImportTimingFields(serde_json::Map<String, serde_json::Value>);

impl Visit for ImportTimingFields {
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0.insert(field.name().into(), value.into());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0.insert(field.name().into(), value.into());
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.0.insert(field.name().into(), serde_json::json!(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().into(), value.into());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().into(), format!("{value:?}").into());
    }
}

impl Subscriber for ImportTimingEvents {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.fields().field("duration_ms").is_some()
    }

    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut fields = ImportTimingFields::default();
        event.record(&mut fields);
        self.0.lock().unwrap().push(fields.0.into());
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}

impl ImportTimingEvents {
    fn for_run(&self, run_id: i64) -> Vec<serde_json::Value> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter(|event| event["run_id"] == run_id)
            .cloned()
            .collect()
    }
}

fn release() -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, xml) in [
        (
            "f_ampp2_3000000.xml",
            "<ACTUAL_MEDICINAL_PROD_PACKS><AMPPS><AMPP><APPID>777</APPID><NM>Scanner imported tablets</NM></AMPP></AMPPS></ACTUAL_MEDICINAL_PROD_PACKS>",
        ),
        (
            "f_gtin2_0000000.xml",
            "<GTIN_DETAILS><AMPPS><AMPP><AMPPID>777</AMPPID><GTINDATA><GTIN>5016298210989</GTIN><STARTDT>2020-01-01</STARTDT></GTINDATA></AMPP></AMPPS></GTIN_DETAILS>",
        ),
    ] {
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(xml.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn release_with_amp(amp: &str, gtin: &str) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    writer
        .start_file(
            "f_ampp2_3000000.xml",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    writer
        .write_all(format!("<AMPPS><AMPP><APPID>777</APPID><NM>Scanner imported tablets</NM><APID>{amp}</APID></AMPP></AMPPS>").as_bytes())
        .unwrap();
    writer
        .start_file(
            "f_gtin2_0000000.xml",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    writer
        .write_all(format!("<AMPPS><AMPP><AMPPID>777</AMPPID><GTINDATA><GTIN>{gtin}</GTIN></GTINDATA></AMPP></AMPPS>").as_bytes())
        .unwrap();
    writer.finish().unwrap().into_inner()
}

fn release_with_supplementary() -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, xml) in [
        (
            "f_ampp2_3000000.xml",
            "<AMPPS><AMPP><APPID>777</APPID><NM>Scanner imported tablets</NM><APID>A1</APID></AMPP></AMPPS>",
        ),
        (
            "f_gtin2_0000000.xml",
            "<AMPPS><AMPP><AMPPID>777</AMPPID><GTINDATA><GTIN>5016298210989</GTIN></GTINDATA></AMPP></AMPPS>",
        ),
        (
            "f_trade_family_group2_0311026.xml",
            "<GROUPS><TRADE_FAMILY_GROUP><TFGID>G1</TFGID><NM>Group</NM></TRADE_FAMILY_GROUP></GROUPS>",
        ),
        (
            "f_trade_family2_0311026.xml",
            "<FAMILIES><TRADE_FAMILY><TFID>F1</TFID><NM>Family</NM><TFGID>G1</TFGID></TRADE_FAMILY></FAMILIES>",
        ),
        (
            "f_amp_trade_family2_0311026.xml",
            "<MEMBERSHIPS><AMP_TRADE_FAMILY><APID>A1</APID><TFID>F1</TFID></AMP_TRADE_FAMILY></MEMBERSHIPS>",
        ),
    ] {
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(xml.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[tokio::test]
async fn booted_application_registers_named_storage_stores() {
    let app = Application::new().await;
    assert!(med_tracker::models::profile::avatar::Storage::configured(&app.context).is_ok());
    assert!(med_tracker::models::nhs_dmd::storage::ArchiveStore::configured(&app.context).is_ok());
    app.close().await;
}

#[tokio::test]
async fn nhs_dmd_import_emits_numeric_phase_and_total_timings_for_success_and_failure() {
    let timings = ImportTimingEvents::default();
    let _subscriber = tracing::subscriber::set_default(timings.clone());
    let app = Application::new().await;
    let success = med_tracker::models::nhs_dmd::upload(&app.context, "success.zip", release())
        .await
        .unwrap();
    med_tracker::models::nhs_dmd::run(&app.context, success)
        .with_subscriber(timings.clone())
        .await
        .unwrap();
    let failure = med_tracker::models::nhs_dmd::upload(
        &app.context,
        "failure.zip",
        b"not a zip release".to_vec(),
    )
    .await
    .unwrap();
    med_tracker::models::nhs_dmd::run(&app.context, failure)
        .with_subscriber(timings.clone())
        .await
        .unwrap();
    app.close().await;

    let successful = timings.for_run(success);
    for phase in [
        "extract",
        "count",
        "parse_ampp",
        "parse_supplementary",
        "parse_gtin",
        "persist",
        "commit",
        "total",
    ] {
        let event = successful
            .iter()
            .find(|event| event["phase"] == phase)
            .unwrap_or_else(|| panic!("missing {phase} timing: {successful:?}"));
        assert_eq!(event["outcome"], "success");
        assert!(event["duration_ms"].is_number(), "{event:?}");
        assert!(event["duration_ms"].as_f64().unwrap() >= 0.0);
    }
    let failed = timings.for_run(failure);
    for phase in ["extract", "total"] {
        let event = failed
            .iter()
            .find(|event| event["phase"] == phase)
            .unwrap_or_else(|| panic!("missing {phase} failure timing: {failed:?}"));
        assert_eq!(event["outcome"], "failure");
        assert!(event["duration_ms"].is_number());
    }
    assert_eq!(failed.len(), 2, "{failed:?}");
}

#[tokio::test]
async fn nhs_dmd_synthetic_supplementary_release_publishes_trade_relationships() {
    let app = Application::new().await;
    let id = med_tracker::models::nhs_dmd::upload(
        &app.context,
        "supplementary.zip",
        release_with_supplementary(),
    )
    .await
    .unwrap();
    med_tracker::models::nhs_dmd::run(&app.context, id)
        .await
        .unwrap();
    let row = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT i.status, (SELECT count(*) FROM nhs_dmd_trade_family_groups) AS groups, (SELECT count(*) FROM nhs_dmd_trade_families) AS families, (SELECT count(*) FROM nhs_dmd_amp_trade_families WHERE amp_code = 'A1') AS memberships, (SELECT count(*) FROM nhs_dmd_supplementary_releases) AS releases FROM nhs_dmd_imports i WHERE i.id = $1",
        [id.into()],
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i32>("", "status").unwrap(), 4);
    for field in ["groups", "families", "memberships", "releases"] {
        assert_eq!(row.try_get::<i64>("", field).unwrap(), 1, "{field}");
    }
    let linkage = app.fixture.admin.query_one_raw(Statement::from_string(
        DbBackend::Postgres,
        "SELECT f.code AS family_code, g.code AS group_code, m.amp_code, r.released_on FROM nhs_dmd_amp_trade_families m JOIN nhs_dmd_trade_families f ON f.id=m.trade_family_id JOIN nhs_dmd_trade_family_groups g ON g.id=f.trade_family_group_id CROSS JOIN nhs_dmd_supplementary_releases r WHERE m.amp_code='A1'",
    )).await.unwrap().unwrap();
    assert_eq!(linkage.try_get::<String>("", "family_code").unwrap(), "F1");
    assert_eq!(linkage.try_get::<String>("", "group_code").unwrap(), "G1");
    assert_eq!(linkage.try_get::<String>("", "amp_code").unwrap(), "A1");
    assert_eq!(
        linkage
            .try_get::<chrono::NaiveDate>("", "released_on")
            .unwrap(),
        chrono::NaiveDate::from_ymd_opt(2026, 10, 31).unwrap()
    );
    app.close().await;
}

#[tokio::test]
async fn nhs_dmd_blank_amp_code_omits_relationship_but_preserves_barcode_value() {
    let app = Application::new().await;
    let id = med_tracker::models::nhs_dmd::upload(
        &app.context,
        "blank-amp.zip",
        release_with_amp("   ", "5016298210989"),
    )
    .await
    .unwrap();
    med_tracker::models::nhs_dmd::run(&app.context, id)
        .await
        .unwrap();
    let row = app.fixture.admin.query_one_raw(Statement::from_string(
        DbBackend::Postgres,
        "SELECT b.amp_code, (SELECT count(*) FROM nhs_dmd_ampp_relationships) AS relationships FROM nhs_dmd_barcodes b WHERE b.gtin='5016298210989'",
    )).await.unwrap().unwrap();
    assert_eq!(row.try_get::<String>("", "amp_code").unwrap(), "   ");
    assert_eq!(row.try_get::<i64>("", "relationships").unwrap(), 0);
    app.close().await;
}

#[tokio::test]
async fn nhs_dmd_failed_replacement_preserves_previous_relationships_and_barcodes() {
    let app = Application::new().await;
    let first = med_tracker::models::nhs_dmd::upload(
        &app.context,
        "first.zip",
        release_with_amp("111", "5016298210989"),
    )
    .await
    .unwrap();
    med_tracker::models::nhs_dmd::run(&app.context, first)
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared("ALTER TABLE nhs_dmd_barcodes ADD CONSTRAINT reject_replacement_gtin CHECK (gtin <> '5016298210990') NOT VALID").await.unwrap();
    let second = med_tracker::models::nhs_dmd::upload(
        &app.context,
        "second.zip",
        release_with_amp("222", "5016298210990"),
    )
    .await
    .unwrap();
    med_tracker::models::nhs_dmd::run(&app.context, second)
        .await
        .unwrap();
    let old_relationship = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT amp_code FROM nhs_dmd_ampp_relationships WHERE ampp_code = '777'",
        ))
        .await
        .unwrap()
        .unwrap();
    let old_barcode = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT amp_code FROM nhs_dmd_barcodes WHERE gtin = '5016298210989'",
        ))
        .await
        .unwrap()
        .unwrap();
    let failed_run = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT status FROM nhs_dmd_imports WHERE id = $1",
            [second.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    app.close().await;
    assert_eq!(failed_run.try_get::<i32>("", "status").unwrap(), 5);
    assert_eq!(
        old_relationship.try_get::<String>("", "amp_code").unwrap(),
        "111"
    );
    assert_eq!(
        old_barcode
            .try_get::<Option<String>>("", "amp_code")
            .unwrap()
            .as_deref(),
        Some("111")
    );
}

fn nested_release(valid: bool) -> Vec<u8> {
    let mut nested = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    nested
        .start_file(
            "f_gtin2_0000000.xml",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    nested.write_all(b"<GTIN_DETAILS><AMPPS><AMPP><AMPPID>777</AMPPID><GTINDATA><GTIN>5016298210989</GTIN><STARTDT>2020-01-01</STARTDT></GTINDATA></AMPP></AMPPS></GTIN_DETAILS>").unwrap();
    let nested = nested.finish().unwrap().into_inner();
    let mut outer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    outer
        .start_file(
            "f_ampp2_3000000.xml",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    outer.write_all(b"<ACTUAL_MEDICINAL_PROD_PACKS><AMPPS><AMPP><APPID>777</APPID><NM>Scanner imported tablets</NM></AMPP></AMPPS></ACTUAL_MEDICINAL_PROD_PACKS>").unwrap();
    outer
        .start_file("weekly_GTIN.zip", zip::write::SimpleFileOptions::default())
        .unwrap();
    outer
        .write_all(if valid {
            &nested
        } else {
            b"invalid nested ZIP"
        })
        .unwrap();
    outer.finish().unwrap().into_inner()
}

fn leaked_nested_directories() -> std::collections::HashSet<std::path::PathBuf> {
    std::fs::read_dir(std::env::temp_dir())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("dmd-gtin-"))
        .map(|entry| entry.path())
        .collect()
}

async fn restarted_worker(
    app: &Application,
) -> (
    std::sync::Arc<loco_rs::bgworker::Queue>,
    tokio::task::JoinHandle<loco_rs::Result<()>>,
) {
    let mut config = Config::new(&Environment::Test).unwrap();
    let Some(QueueConfig::Postgres(queue_config)) = config.queue.as_mut() else {
        panic!("PostgreSQL queue required")
    };
    queue_config.uri = app.fixture.runtime_uri.clone();
    let queue = std::sync::Arc::new(
        loco_rs::bgworker::pg::create_provider(queue_config)
            .await
            .unwrap(),
    );
    med_tracker::app::App::connect_workers(&app.context, &queue)
        .await
        .unwrap();
    let running = queue.clone();
    let worker = tokio::spawn(async move { running.run(vec![]).await });
    (queue, worker)
}

async fn stop_worker(
    queue: std::sync::Arc<loco_rs::bgworker::Queue>,
    worker: tokio::task::JoinHandle<loco_rs::Result<()>>,
) {
    queue.shutdown().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), worker)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn nhs_dmd_queue_failure_marks_import_failed_and_cleans_archive() {
    assert_eq!(std::env::var("MEDTRACKER_OWNED_STORAGE").unwrap(), "1");
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("ALTER TABLE pg_loco_queue ADD CONSTRAINT synthetic_enqueue_failure CHECK(false) NOT VALID").await.unwrap();
    let result =
        med_tracker::models::nhs_dmd::upload(&app.context, "synthetic.zip", release()).await;
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status,archive_key,error_message FROM nhs_dmd_imports ORDER BY id DESC LIMIT 1",
        ))
        .await
        .unwrap()
        .unwrap();
    let status: i32 = row.try_get("", "status").unwrap();
    let key: Option<String> = row.try_get("", "archive_key").unwrap();
    let error: Option<String> = row.try_get("", "error_message").unwrap();
    app.close().await;
    assert!(result.is_err());
    assert_eq!(status, 5);
    assert!(key.is_none());
    assert!(error.is_some_and(|value| value.contains("queue")));
}

#[tokio::test]
async fn nhs_dmd_missing_stored_object_marks_import_failed() {
    assert_eq!(std::env::var("MEDTRACKER_OWNED_STORAGE").unwrap(), "1");
    let app = Application::new().await;
    let id = med_tracker::models::nhs_dmd::upload(&app.context, "synthetic.zip", release())
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT archive_key FROM nhs_dmd_imports WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let key: String = row.try_get("", "archive_key").unwrap();
    med_tracker::models::nhs_dmd::storage::ArchiveStore::configured(&app.context)
        .unwrap()
        .delete(&key)
        .await
        .unwrap();
    let _ = med_tracker::models::nhs_dmd::run(&app.context, id).await;
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT status,error_message FROM nhs_dmd_imports WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let status: i32 = row.try_get("", "status").unwrap();
    let error: Option<String> = row.try_get("", "error_message").unwrap();
    app.close().await;
    assert_eq!(status, 5);
    assert!(error.is_some_and(|value| value.contains("archive")));
}

#[tokio::test]
async fn nhs_dmd_queue_imports_verified_archive_and_cleans_it_after_completion() {
    assert_eq!(std::env::var("MEDTRACKER_OWNED_STORAGE").unwrap(), "1");
    let app = Application::new().await;
    let id = med_tracker::models::nhs_dmd::upload(&app.context, "synthetic-release.zip", release())
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT archive_key FROM nhs_dmd_imports WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let key: String = row.try_get("", "archive_key").unwrap();
    let store =
        med_tracker::models::nhs_dmd::storage::ArchiveStore::configured(&app.context).unwrap();
    assert_eq!(store.get(&key).await.unwrap(), release());
    let object = crate::avatar_storage::owned_client()
        .head_object()
        .bucket("scanner-fixture")
        .key(&key)
        .send()
        .await
        .unwrap();
    assert_eq!(object.content_type().unwrap(), "application/zip");
    let queue = app.context.queue_provider.as_ref().unwrap().clone();
    med_tracker::app::App::connect_workers(&app.context, &queue)
        .await
        .unwrap();
    let running = queue.clone();
    let worker = tokio::spawn(async move { running.run(vec![]).await });
    let completed = tokio::time::timeout(std::time::Duration::from_secs(20), async {
        loop {
            let row = app
                .fixture
                .admin
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "SELECT status,archive_key FROM nhs_dmd_imports WHERE id=$1",
                    [id.into()],
                ))
                .await
                .unwrap()
                .unwrap();
            let status: i32 = row.try_get("", "status").unwrap();
            let key: Option<String> = row.try_get("", "archive_key").unwrap();
            if status >= 4 && key.is_none() {
                break status;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await;
    queue.shutdown().unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), worker)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let found = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT code FROM nhs_dmd_barcodes WHERE gtin='5016298210989'",
        ))
        .await
        .unwrap();
    let progress = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT total_records,processed_records,imported_count,created_count,log FROM nhs_dmd_imports WHERE id=$1",[id.into()])).await.unwrap().unwrap();
    let missing = store.get(&key).await.is_err();
    app.close().await;
    assert_eq!(completed.unwrap(), 4);
    assert_eq!(found.unwrap().try_get::<String>("", "code").unwrap(), "777");
    assert_eq!(progress.try_get::<i32>("", "total_records").unwrap(), 2);
    assert_eq!(progress.try_get::<i32>("", "processed_records").unwrap(), 2);
    assert_eq!(progress.try_get::<i32>("", "imported_count").unwrap(), 1);
    assert_eq!(progress.try_get::<i32>("", "created_count").unwrap(), 1);
    assert!(
        progress
            .try_get::<String>("", "log")
            .unwrap()
            .contains("Extracting release archive\nCounting release records\n")
    );
    assert!(missing);
}

#[tokio::test]
async fn nhs_dmd_nested_gtin_temp_files_are_removed_after_success_and_failure() {
    assert_eq!(std::env::var("MEDTRACKER_OWNED_STORAGE").unwrap(), "1");
    let app = Application::new().await;
    let before = leaked_nested_directories();
    let good = med_tracker::models::nhs_dmd::upload(
        &app.context,
        "nested-release.zip",
        nested_release(true),
    )
    .await
    .unwrap();
    med_tracker::models::nhs_dmd::run(&app.context, good)
        .await
        .unwrap();
    let completed = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT status FROM nhs_dmd_imports WHERE id=$1",
            [good.into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i32>("", "status")
        .unwrap();
    let after_success = leaked_nested_directories();
    app.close().await;
    for path in after_success.difference(&before) {
        let _ = std::fs::remove_dir_all(path);
    }
    assert_eq!(completed, 4);
    assert_eq!(
        after_success, before,
        "nested GTIN extraction leaked after success"
    );

    let app = Application::new().await;
    let before = leaked_nested_directories();
    let bad = med_tracker::models::nhs_dmd::upload(
        &app.context,
        "invalid-nested-release.zip",
        nested_release(false),
    )
    .await
    .unwrap();
    let _ = med_tracker::models::nhs_dmd::run(&app.context, bad).await;
    let failed = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT status FROM nhs_dmd_imports WHERE id=$1",
            [bad.into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i32>("", "status")
        .unwrap();
    let after_failure = leaked_nested_directories();
    app.close().await;
    for path in after_failure.difference(&before) {
        let _ = std::fs::remove_dir_all(path);
    }
    assert_eq!(failed, 5);
    assert_eq!(
        after_failure, before,
        "nested GTIN extraction leaked after failure"
    );
}

#[tokio::test]
async fn nhs_dmd_queued_release_survives_worker_restart() {
    assert_eq!(std::env::var("MEDTRACKER_OWNED_STORAGE").unwrap(), "1");
    let app = Application::new().await;
    let original = app.context.queue_provider.as_ref().unwrap().clone();
    med_tracker::app::App::connect_workers(&app.context, &original)
        .await
        .unwrap();
    let running = original.clone();
    let first_worker = tokio::spawn(async move { running.run(vec![]).await });
    stop_worker(original, first_worker).await;
    let id = med_tracker::models::nhs_dmd::upload(&app.context, "persisted-release.zip", release())
        .await
        .unwrap();
    let queued = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT status FROM nhs_dmd_imports WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(queued.try_get::<i32>("", "status").unwrap(), 0);
    let (queue, worker) = restarted_worker(&app).await;
    let completed = tokio::time::timeout(std::time::Duration::from_secs(20), async {
        loop {
            let row = app
                .fixture
                .admin
                .query_one_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "SELECT status,archive_key FROM nhs_dmd_imports WHERE id=$1",
                    [id.into()],
                ))
                .await
                .unwrap()
                .unwrap();
            if row.try_get::<i32>("", "status").unwrap() == 4
                && row
                    .try_get::<Option<String>>("", "archive_key")
                    .unwrap()
                    .is_none()
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await;
    stop_worker(queue, worker).await;
    let imported = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT code FROM nhs_dmd_barcodes WHERE gtin='5016298210989'",
        ))
        .await
        .unwrap();
    app.close().await;
    completed.unwrap();
    assert_eq!(
        imported.unwrap().try_get::<String>("", "code").unwrap(),
        "777"
    );
}

#[tokio::test]
async fn nhs_dmd_interrupted_release_fails_without_restart_rerun() {
    assert_eq!(std::env::var("MEDTRACKER_OWNED_STORAGE").unwrap(), "1");
    let app = Application::new().await;
    let original = app.context.queue_provider.as_ref().unwrap().clone();
    med_tracker::app::App::connect_workers(&app.context, &original)
        .await
        .unwrap();
    let running = original.clone();
    let first_worker = tokio::spawn(async move { running.run(vec![]).await });
    stop_worker(original, first_worker).await;
    let id =
        med_tracker::models::nhs_dmd::upload(&app.context, "interrupted-release.zip", release())
            .await
            .unwrap();
    app.fixture.admin.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"UPDATE nhs_dmd_imports SET status=3,updated_at=now()-interval '31 minutes' WHERE id=$1",[id.into()])).await.unwrap();
    med_tracker::models::nhs_dmd::reconcile(&app.context)
        .await
        .unwrap();
    let (queue, worker) = restarted_worker(&app).await;
    let drained = tokio::time::timeout(std::time::Duration::from_secs(20), async {
        loop {
            let pending: i64 = app
                .fixture
                .admin
                .query_one_raw(Statement::from_string(
                    DbBackend::Postgres,
                    "SELECT count(*) AS pending FROM pg_loco_queue WHERE name='ImportWorker' AND status IN ('queued','processing')",
                ))
                .await
                .unwrap()
                .unwrap()
                .try_get("", "pending")
                .unwrap();
            if pending == 0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await;
    stop_worker(queue, worker).await;
    drained.unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT status,archive_key,error_message FROM nhs_dmd_imports WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let imported = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT code FROM nhs_dmd_barcodes WHERE gtin='5016298210989'",
        ))
        .await
        .unwrap();
    app.close().await;
    assert_eq!(row.try_get::<i32>("", "status").unwrap(), 5);
    assert!(
        row.try_get::<Option<String>>("", "archive_key")
            .unwrap()
            .is_none()
    );
    assert!(
        row.try_get::<Option<String>>("", "error_message")
            .unwrap()
            .unwrap()
            .contains("stopped making progress")
    );
    assert!(imported.is_none());
}

#[tokio::test]
async fn nhs_dmd_reconciliation_fails_stale_imports_and_keeps_fresh_work_active() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO nhs_dmd_imports(id,uploaded_filename,status,created_at,updated_at) VALUES(94001,'stale.zip',3,now()-interval '31 minutes',now()-interval '31 minutes')").await.unwrap();
    med_tracker::models::nhs_dmd::reconcile(&app.context)
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status FROM nhs_dmd_imports WHERE id=94001",
        ))
        .await
        .unwrap()
        .unwrap();
    let status: i32 = row.try_get("", "status").unwrap();
    app.close().await;
    assert_eq!(status, 5);
}

#[tokio::test]
async fn nhs_dmd_reconciliation_fails_stalled_import_even_while_worker_owns_lock() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO nhs_dmd_imports(id,uploaded_filename,status,created_at,updated_at) VALUES(94001,'stalled.zip',3,now()-interval '31 minutes',now()-interval '31 minutes')").await.unwrap();
    let lease = app.fixture.admin.begin().await.unwrap();
    lease
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_advisory_xact_lock(7369918)",
        ))
        .await
        .unwrap();
    med_tracker::models::nhs_dmd::reconcile(&app.context)
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status FROM nhs_dmd_imports WHERE id=94001",
        ))
        .await
        .unwrap()
        .unwrap();
    let status: i32 = row.try_get("", "status").unwrap();
    lease.rollback().await.unwrap();
    app.close().await;
    assert_eq!(status, 5);
}

#[tokio::test]
async fn nhs_dmd_missing_archive_fails_without_blocking_future_uploads() {
    let app = Application::new().await;
    app.fixture.admin.execute_unprepared("INSERT INTO nhs_dmd_imports(id,uploaded_filename,status,created_at,updated_at) VALUES(94001,'lost.zip',0,now(),now())").await.unwrap();
    med_tracker::models::nhs_dmd::run(&app.context, 94001)
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status,error_message FROM nhs_dmd_imports WHERE id=94001",
        ))
        .await
        .unwrap()
        .unwrap();
    let status: i32 = row.try_get("", "status").unwrap();
    let error: Option<String> = row.try_get("", "error_message").unwrap();
    app.close().await;
    assert_eq!(status, 5);
    assert!(error.is_some_and(|value| value.contains("archive")));
}

#[tokio::test]
async fn nhs_dmd_rejects_concurrent_upload_and_allows_the_next_release_after_completion() {
    let app = Application::new().await;
    let first = med_tracker::models::nhs_dmd::upload(&app.context, "first.zip", release())
        .await
        .unwrap();
    let second = med_tracker::models::nhs_dmd::upload(&app.context, "second.zip", release()).await;
    assert!(matches!(
        second,
        Err(med_tracker::models::errors::OperationError::Conflict { .. })
    ));
    med_tracker::models::nhs_dmd::run(&app.context, first)
        .await
        .unwrap();
    let third = med_tracker::models::nhs_dmd::upload(&app.context, "third.zip", release()).await;
    app.close().await;
    assert!(third.is_ok());
}

#[tokio::test]
async fn nhs_dmd_rejects_tampered_archive_before_catalogue_writes() {
    let app = Application::new().await;
    let id = med_tracker::models::nhs_dmd::upload(&app.context, "synthetic.zip", release())
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT archive_key FROM nhs_dmd_imports WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let key: String = row.try_get("", "archive_key").unwrap();
    med_tracker::models::nhs_dmd::storage::ArchiveStore::configured(&app.context)
        .unwrap()
        .put(&key, b"tampered".to_vec())
        .await
        .unwrap();
    med_tracker::models::nhs_dmd::run(&app.context, id)
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT status,error_message,archive_key FROM nhs_dmd_imports WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let status: i32 = row.try_get("", "status").unwrap();
    let error: String = row.try_get("", "error_message").unwrap();
    let key: Option<String> = row.try_get("", "archive_key").unwrap();
    let catalog = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT code FROM nhs_dmd_barcodes WHERE gtin='5016298210989'",
        ))
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 5);
    assert!(error.contains("integrity"));
    assert!(key.is_none());
    assert!(catalog.is_none());
}

#[tokio::test]
async fn nhs_dmd_bad_zip_fails_and_does_not_publish_partial_catalogue() {
    let app = Application::new().await;
    let id = med_tracker::models::nhs_dmd::upload(
        &app.context,
        "broken.zip",
        b"not a zip archive".to_vec(),
    )
    .await
    .unwrap();
    med_tracker::models::nhs_dmd::run(&app.context, id)
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT status,error_message FROM nhs_dmd_imports WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let status: i32 = row.try_get("", "status").unwrap();
    let error: Option<String> = row.try_get("", "error_message").unwrap();
    let catalog = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT code FROM nhs_dmd_barcodes WHERE gtin='5016298210989'",
        ))
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 5);
    assert!(error.is_some());
    assert!(catalog.is_none());
}

#[tokio::test]
async fn nhs_dmd_retries_terminal_cleanup_without_restarting_import() {
    let app = Application::new().await;
    let id = med_tracker::models::nhs_dmd::upload(&app.context, "synthetic.zip", release())
        .await
        .unwrap();
    app.fixture.admin.execute_unprepared("ALTER TABLE nhs_dmd_imports ADD CONSTRAINT synthetic_cleanup_failure CHECK (archive_key IS NOT NULL) NOT VALID").await.unwrap();
    assert!(
        med_tracker::models::nhs_dmd::run(&app.context, id)
            .await
            .is_err()
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT status,archive_key FROM nhs_dmd_imports WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i32>("", "status").unwrap(), 4);
    assert!(
        row.try_get::<Option<String>>("", "archive_key")
            .unwrap()
            .is_some()
    );
    app.fixture
        .admin
        .execute_unprepared("ALTER TABLE nhs_dmd_imports DROP CONSTRAINT synthetic_cleanup_failure")
        .await
        .unwrap();
    med_tracker::models::nhs_dmd::run(&app.context, id)
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT status,archive_key FROM nhs_dmd_imports WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let catalog = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT code FROM nhs_dmd_barcodes WHERE gtin='5016298210989'",
        ))
        .await
        .unwrap();
    app.close().await;
    assert_eq!(row.try_get::<i32>("", "status").unwrap(), 4);
    assert!(
        row.try_get::<Option<String>>("", "archive_key")
            .unwrap()
            .is_none()
    );
    assert!(catalog.is_some());
}

#[test]
fn nhs_dmd_reconciliation_is_scheduled_for_every_runtime() {
    for environment in [
        Environment::Development,
        Environment::Test,
        Environment::Production,
    ] {
        let config = Config::new(&environment).unwrap();
        let value = serde_json::to_value(config).unwrap();
        assert_eq!(
            value["scheduler"]["jobs"]["nhs_dmd_reconcile"]["run"],
            "nhs_dmd_reconcile"
        );
        assert_eq!(
            value["scheduler"]["jobs"]["nhs_dmd_reconcile"]["schedule"],
            "0 */5 * * * *"
        );
    }
}
