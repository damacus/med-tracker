use super::*;
use med_tracker::models::_entities as rows;
use sea_orm::TransactionTrait;
use sea_orm::sea_query::{Expr, ExprTrait, Func, Query};
use std::io::Write;
use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};
use tracing::instrument::WithSubscriber;
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

#[derive(Clone, Default)]
struct ImportTimingEvents(
    Arc<Mutex<Vec<serde_json::Value>>>,
    Arc<Mutex<Option<ImportPause>>>,
);

struct ImportPause {
    phase: &'static str,
    reached: tokio::sync::oneshot::Sender<()>,
    resume: std::sync::mpsc::Receiver<()>,
}

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
            || metadata.fields().field("secondary_failure").is_some()
    }

    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut fields = ImportTimingFields::default();
        event.record(&mut fields);
        let pause = {
            let mut pending = self.1.lock().unwrap();
            if pending.as_ref().is_some_and(|pause| {
                fields
                    .0
                    .get("phase")
                    .is_some_and(|phase| phase == pause.phase)
            }) {
                pending.take()
            } else {
                None
            }
        };
        if let Some(pause) = pause {
            let _ = pause.reached.send(());
            tokio::task::block_in_place(|| {
                pause
                    .resume
                    .recv_timeout(std::time::Duration::from_secs(30))
            })
            .unwrap();
        }
        self.0.lock().unwrap().push(fields.0.into());
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}

impl ImportTimingEvents {
    fn pause_at(
        phase: &'static str,
    ) -> (
        Self,
        tokio::sync::oneshot::Receiver<()>,
        std::sync::mpsc::Sender<()>,
    ) {
        let events = Self::default();
        let (reached, waiting) = tokio::sync::oneshot::channel();
        let (resume, resumed) = std::sync::mpsc::channel();
        *events.1.lock().unwrap() = Some(ImportPause {
            phase,
            reached,
            resume: resumed,
        });
        (events, waiting, resume)
    }

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nhs_dmd_reconciled_worker_preserves_specific_failure_without_publishing() {
    crate::reference_fixture::clean_nhs_catalogue().await;
    let app = Application::new().await;
    let previous = med_tracker::models::nhs_dmd::upload(&app.context, "previous.zip", release())
        .await
        .unwrap();
    med_tracker::models::nhs_dmd::run(&app.context, previous)
        .await
        .unwrap();
    for (phase, bytes, expected, earlier_failure) in [
        (
            "total",
            release_with_ampp_xml("<AMPPS><AMPP><AMPP></AMPP></AMPP></AMPPS>"),
            Some("Nested AMPP record in release XML"),
            None,
        ),
        (
            "total",
            release_with_ampp_xml("<AMPPS><AMPP><AMPP></AMPP></AMPP></AMPPS>"),
            None,
            Some("Earlier specific failure"),
        ),
        (
            "parse_gtin",
            release_with_amp("replacement", "5016298210990"),
            None,
            None,
        ),
    ] {
        let id = med_tracker::models::nhs_dmd::upload(&app.context, "stalled.zip", bytes)
            .await
            .unwrap();
        let before = rows::nhs_dmd_imports::Entity::find_by_id(id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        let archive_key = before.archive_key.unwrap();
        let (events, reached, resume) = ImportTimingEvents::pause_at(phase);
        let ctx = app.context.clone();
        let worker = tokio::spawn(async move {
            med_tracker::models::nhs_dmd::run(&ctx, id)
                .with_subscriber(events)
                .await
        });
        tokio::time::timeout(std::time::Duration::from_secs(30), reached)
            .await
            .unwrap()
            .unwrap();
        rows::nhs_dmd_imports::Entity::update_many()
            .col_expr(
                rows::nhs_dmd_imports::Column::UpdatedAt,
                Expr::value(chrono::Utc::now().naive_utc() - chrono::Duration::minutes(31)),
            )
            .filter(rows::nhs_dmd_imports::Column::Id.eq(id))
            .exec(&app.fixture.admin)
            .await
            .unwrap();
        med_tracker::models::nhs_dmd::reconcile(&app.context)
            .await
            .unwrap();
        if let Some(earlier_failure) = earlier_failure {
            rows::nhs_dmd_imports::Entity::update_many()
                .col_expr(
                    rows::nhs_dmd_imports::Column::ErrorMessage,
                    Expr::value(earlier_failure),
                )
                .filter(rows::nhs_dmd_imports::Column::Id.eq(id))
                .exec(&app.fixture.admin)
                .await
                .unwrap();
        }
        let stale = rows::nhs_dmd_imports::Entity::find_by_id(id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stale.status, 5);
        assert_eq!(
            stale.error_message.as_deref(),
            Some(
                earlier_failure
                    .unwrap_or("The import stopped making progress. Upload the release again.")
            )
        );
        resume.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(30), worker)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let failed = rows::nhs_dmd_imports::Entity::find_by_id(id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(failed.status, 5);
        assert_eq!(failed.completed_at, stale.completed_at);
        if let Some(expected) = expected {
            assert_eq!(failed.error_message.as_deref(), Some(expected));
            assert!(
                failed
                    .log
                    .as_ref()
                    .unwrap()
                    .contains("stopped making progress")
            );
            assert!(
                failed
                    .log
                    .as_ref()
                    .unwrap()
                    .contains(failed.error_message.as_ref().unwrap())
            );
        } else {
            assert_eq!(failed.error_message, stale.error_message);
        }
        let store =
            med_tracker::models::nhs_dmd::storage::ArchiveStore::configured(&app.context).unwrap();
        assert!(failed.archive_key.is_none());
        assert!(store.get(&archive_key).await.is_err());
        let diagnostic = failed.error_message;
        med_tracker::models::nhs_dmd::run(&app.context, id)
            .await
            .unwrap();
        let retried = rows::nhs_dmd_imports::Entity::find_by_id(id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retried.status, 5);
        assert_eq!(retried.error_message, diagnostic);
        assert!(
            rows::nhs_dmd_barcodes::Entity::find()
                .filter(rows::nhs_dmd_barcodes::Column::Gtin.eq("5016298210989"))
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            rows::nhs_dmd_barcodes::Entity::find()
                .filter(rows::nhs_dmd_barcodes::Column::Gtin.eq("5016298210990"))
                .one(&app.fixture.admin)
                .await
                .unwrap()
                .is_none()
        );
    }
    app.close().await;
}

fn release() -> Vec<u8> {
    release_with_ampp_xml(
        "<ACTUAL_MEDICINAL_PROD_PACKS><AMPPS><AMPP><APPID>777</APPID><NM>Scanner imported tablets</NM></AMPP></AMPPS></ACTUAL_MEDICINAL_PROD_PACKS>",
    )
}

fn release_with_ampp_xml(ampp: &str) -> Vec<u8> {
    release_with_xml(
        ampp,
        "<GTIN_DETAILS><AMPPS><AMPP><AMPPID>777</AMPPID><GTINDATA><GTIN>5016298210989</GTIN><STARTDT>2020-01-01</STARTDT></GTINDATA></AMPP></AMPPS></GTIN_DETAILS>",
    )
}

fn release_with_xml(ampp: &str, gtin: &str) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, xml) in [("f_ampp2_3000000.xml", ampp), ("f_gtin2_0000000.xml", gtin)] {
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(xml.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[tokio::test]
async fn nhs_dmd_parser_failures_survive_progress_failure() {
    crate::reference_fixture::clean_nhs_catalogue().await;
    let app = Application::new().await;
    let ampp = format!(
        "<AMPPS>{}<AMPP><AMPP></AMPP></AMPP></AMPPS>",
        "<AMPP/>".repeat(250)
    );
    let gtin = format!(
        "<AMPPS><AMPP>{}<GTINDATA><GTINDATA></GTINDATA></GTINDATA></AMPP></AMPPS>",
        "<GTINDATA/>".repeat(250)
    );
    for (phase, bytes, expected) in [
        (
            "parse_ampp",
            release_with_ampp_xml(&ampp),
            "Nested AMPP record in release XML",
        ),
        (
            "parse_gtin",
            release_with_xml(
                "<AMPPS><AMPP><APPID>777</APPID><NM>Synthetic tablets</NM></AMPP></AMPPS>",
                &gtin,
            ),
            "Nested GTINDATA record in GTIN XML",
        ),
    ] {
        let id = med_tracker::models::nhs_dmd::upload(&app.context, "broken.zip", bytes)
            .await
            .unwrap();
        // SeaQuery TableAlterStatement has no add-CHECK API; this fixed fault-injection constraint needs PostgreSQL DDL and exclusive execution.
        app.fixture.admin.execute_unprepared("ALTER TABLE nhs_dmd_imports ADD CONSTRAINT synthetic_progress_failure CHECK (processed_records < 250) NOT VALID").await.unwrap();
        let events = ImportTimingEvents::default();
        let result = med_tracker::models::nhs_dmd::run(&app.context, id)
            .with_subscriber(events.clone())
            .await;
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    sea_orm::sea_query::Table::alter()
                        .table("nhs_dmd_imports")
                        .drop_constraint("synthetic_progress_failure"),
                ),
            )
            .await
            .unwrap();
        result.unwrap();
        let failed = rows::nhs_dmd_imports::Entity::find_by_id(id)
            .one(&app.fixture.admin)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(failed.status, 5);
        assert_eq!(failed.error_message.as_deref(), Some(expected));
        assert!(failed.log.as_ref().unwrap().contains(expected));
        assert!(failed.archive_key.is_none());
        let captured = events.for_run(id);
        assert!(
            captured.iter().any(|event| event["phase"] == phase
                && event["secondary_failure"] == "activity_reporting")
        );
        assert!(
            !serde_json::to_string(&captured)
                .unwrap()
                .contains("synthetic_progress_failure")
        );
    }
    app.close().await;
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
    crate::reference_fixture::clean_nhs_catalogue().await;
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
    crate::reference_fixture::clean_nhs_catalogue().await;
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
    let row = rows::nhs_dmd_imports::Entity::find_by_id(id)
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, 4);
    for (field, count) in [
        (
            "groups",
            rows::nhs_dmd_trade_family_groups::Entity::find()
                .count(&app.fixture.admin)
                .await
                .unwrap(),
        ),
        (
            "families",
            rows::nhs_dmd_trade_families::Entity::find()
                .count(&app.fixture.admin)
                .await
                .unwrap(),
        ),
        (
            "memberships",
            rows::nhs_dmd_amp_trade_families::Entity::find()
                .filter(rows::nhs_dmd_amp_trade_families::Column::AmpCode.eq("A1"))
                .count(&app.fixture.admin)
                .await
                .unwrap(),
        ),
        (
            "releases",
            rows::nhs_dmd_supplementary_releases::Entity::find()
                .count(&app.fixture.admin)
                .await
                .unwrap(),
        ),
    ] {
        assert_eq!(count, 1, "{field}");
    }
    let linkage = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr_as(Expr::col(("f", "code")), "family_code")
                    .expr_as(Expr::col(("g", "code")), "group_code")
                    .column(("m", "amp_code"))
                    .column((
                        rows::nhs_dmd_supplementary_releases::Entity,
                        rows::nhs_dmd_supplementary_releases::Column::ReleasedOn,
                    ))
                    .from_as(rows::nhs_dmd_amp_trade_families::Entity, "m")
                    .join_as(
                        sea_orm::JoinType::InnerJoin,
                        rows::nhs_dmd_trade_families::Entity,
                        "f",
                        Expr::col(("f", "id")).eq(Expr::col(("m", "trade_family_id"))),
                    )
                    .join_as(
                        sea_orm::JoinType::InnerJoin,
                        rows::nhs_dmd_trade_family_groups::Entity,
                        "g",
                        Expr::col(("g", "id")).eq(Expr::col(("f", "trade_family_group_id"))),
                    )
                    .cross_join(rows::nhs_dmd_supplementary_releases::Entity)
                    .and_where(Expr::col(("m", "amp_code")).eq("A1"))
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
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
    crate::reference_fixture::clean_nhs_catalogue().await;
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
    let row = rows::nhs_dmd_barcodes::Entity::find()
        .filter(rows::nhs_dmd_barcodes::Column::Gtin.eq("5016298210989"))
        .one(&app.fixture.admin)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.amp_code.as_deref(), Some("   "));
    assert_eq!(
        rows::nhs_dmd_ampp_relationships::Entity::find()
            .count(&app.fixture.admin)
            .await
            .unwrap(),
        0
    );
    app.close().await;
}

#[tokio::test]
async fn nhs_dmd_failed_replacement_preserves_previous_relationships_and_barcodes() {
    crate::reference_fixture::clean_nhs_catalogue().await;
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
    // SeaQuery TableAlterStatement cannot mark a fault-injection CHECK constraint NOT VALID; these cases require exclusive execution.
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
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_ampp_relationships", "amp_code")))
                    .from("nhs_dmd_ampp_relationships")
                    .cond_where(
                        Expr::col(("nhs_dmd_ampp_relationships", "ampp_code")).eq(Expr::val("777")),
                    )
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
    let old_barcode = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_barcodes", "amp_code")))
                    .from("nhs_dmd_barcodes")
                    .cond_where(
                        Expr::col(("nhs_dmd_barcodes", "gtin")).eq(Expr::val("5016298210989")),
                    )
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
    let failed_run = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_imports", "status")))
                    .from("nhs_dmd_imports")
                    .cond_where(Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(second)))
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
    // SeaQuery TableAlterStatement cannot mark a fault-injection CHECK constraint NOT VALID; these cases require exclusive execution.
    app.fixture
        .admin
        .execute_raw(
            app.fixture.admin.get_database_backend().build(
                &sea_orm::sea_query::Table::alter()
                    .table("nhs_dmd_barcodes")
                    .drop_constraint("reject_replacement_gtin")
                    .to_owned(),
            ),
        )
        .await
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
    crate::reference_fixture::clean_nhs_catalogue().await;
    assert_eq!(std::env::var("MEDTRACKER_OWNED_STORAGE").unwrap(), "1");
    let app = Application::new().await;
    // SeaQuery TableAlterStatement cannot mark a fault-injection CHECK constraint NOT VALID; these cases require exclusive execution.
    app.fixture.admin.execute_unprepared("ALTER TABLE pg_loco_queue ADD CONSTRAINT synthetic_enqueue_failure CHECK(false) NOT VALID").await.unwrap();
    let result =
        med_tracker::models::nhs_dmd::upload(&app.context, "synthetic.zip", release()).await;
    let row = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_imports", "status")))
                    .expr(Expr::col(("nhs_dmd_imports", "archive_key")))
                    .expr(Expr::col(("nhs_dmd_imports", "error_message")))
                    .from("nhs_dmd_imports")
                    .order_by_expr(Expr::col(("nhs_dmd_imports", "id")), sea_orm::Order::Desc)
                    .limit(1)
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
    let status: i32 = row.try_get("", "status").unwrap();
    let key: Option<String> = row.try_get("", "archive_key").unwrap();
    let error: Option<String> = row.try_get("", "error_message").unwrap();
    // SeaQuery TableAlterStatement cannot mark a fault-injection CHECK constraint NOT VALID; these cases require exclusive execution.
    app.fixture
        .admin
        .execute_raw(
            app.fixture.admin.get_database_backend().build(
                &sea_orm::sea_query::Table::alter()
                    .table("pg_loco_queue")
                    .drop_constraint("synthetic_enqueue_failure")
                    .to_owned(),
            ),
        )
        .await
        .unwrap();
    app.close().await;
    assert!(result.is_err());
    assert_eq!(status, 5);
    assert!(key.is_none());
    assert!(error.is_some_and(|value| value.contains("queue")));
}

#[tokio::test]
async fn nhs_dmd_missing_stored_object_marks_import_failed() {
    crate::reference_fixture::clean_nhs_catalogue().await;
    assert_eq!(std::env::var("MEDTRACKER_OWNED_STORAGE").unwrap(), "1");
    let app = Application::new().await;
    let id = med_tracker::models::nhs_dmd::upload(&app.context, "synthetic.zip", release())
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_imports", "archive_key")))
                    .from("nhs_dmd_imports")
                    .cond_where(Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(id)))
                    .to_owned(),
            ),
        )
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
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_imports", "status")))
                    .expr(Expr::col(("nhs_dmd_imports", "error_message")))
                    .from("nhs_dmd_imports")
                    .cond_where(Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(id)))
                    .to_owned(),
            ),
        )
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
    crate::reference_fixture::clean_nhs_catalogue().await;
    assert_eq!(std::env::var("MEDTRACKER_OWNED_STORAGE").unwrap(), "1");
    let app = Application::new().await;
    let id = med_tracker::models::nhs_dmd::upload(&app.context, "synthetic-release.zip", release())
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_imports", "archive_key")))
                    .from("nhs_dmd_imports")
                    .cond_where(Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(id)))
                    .to_owned(),
            ),
        )
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
                .query_one_raw(
                    app.fixture.admin.get_database_backend().build(
                        &Query::select()
                            .expr(Expr::col(("nhs_dmd_imports", "status")))
                            .expr(Expr::col(("nhs_dmd_imports", "archive_key")))
                            .from("nhs_dmd_imports")
                            .cond_where(Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(id)))
                            .to_owned(),
                    ),
                )
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
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_barcodes", "code")))
                    .from("nhs_dmd_barcodes")
                    .cond_where(
                        Expr::col(("nhs_dmd_barcodes", "gtin")).eq(Expr::val("5016298210989")),
                    )
                    .to_owned(),
            ),
        )
        .await
        .unwrap();
    let progress = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_imports", "total_records")))
                    .expr(Expr::col(("nhs_dmd_imports", "processed_records")))
                    .expr(Expr::col(("nhs_dmd_imports", "imported_count")))
                    .expr(Expr::col(("nhs_dmd_imports", "created_count")))
                    .expr(Expr::col(("nhs_dmd_imports", "log")))
                    .from("nhs_dmd_imports")
                    .cond_where(Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(id)))
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
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
    crate::reference_fixture::clean_nhs_catalogue().await;
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
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_imports", "status")))
                    .from("nhs_dmd_imports")
                    .cond_where(Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(good)))
                    .to_owned(),
            ),
        )
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
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_imports", "status")))
                    .from("nhs_dmd_imports")
                    .cond_where(Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(bad)))
                    .to_owned(),
            ),
        )
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
    crate::reference_fixture::clean_nhs_catalogue().await;
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
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_imports", "status")))
                    .from("nhs_dmd_imports")
                    .cond_where(Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(id)))
                    .to_owned(),
            ),
        )
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
                .query_one_raw(
                    app.fixture.admin.get_database_backend().build(
                        &Query::select()
                            .expr(Expr::col(("nhs_dmd_imports", "status")))
                            .expr(Expr::col(("nhs_dmd_imports", "archive_key")))
                            .from("nhs_dmd_imports")
                            .cond_where(Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(id)))
                            .to_owned(),
                    ),
                )
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
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_barcodes", "code")))
                    .from("nhs_dmd_barcodes")
                    .cond_where(
                        Expr::col(("nhs_dmd_barcodes", "gtin")).eq(Expr::val("5016298210989")),
                    )
                    .to_owned(),
            ),
        )
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
    crate::reference_fixture::clean_nhs_catalogue().await;
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
    app.fixture
        .admin
        .execute_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::update()
                    .table("nhs_dmd_imports")
                    .value("status", Expr::val(3_i64))
                    .value(
                        "updated_at",
                        Expr::val(chrono::Utc::now().naive_utc() - chrono::Duration::minutes(31)),
                    )
                    .cond_where(Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(id)))
                    .to_owned(),
            ),
        )
        .await
        .unwrap();
    med_tracker::models::nhs_dmd::reconcile(&app.context)
        .await
        .unwrap();
    let (queue, worker) = restarted_worker(&app).await;
    let drained = tokio::time::timeout(std::time::Duration::from_secs(20), async {
        loop {
            let pending: i64 = app
                .fixture
                .admin
                .query_one_raw(
                    app.fixture.admin.get_database_backend().build(
                        &Query::select()
                            .expr_as(Expr::col(("pg_loco_queue", "id")).count(), "pending")
                            .from("pg_loco_queue")
                            .cond_where(
                                sea_orm::Condition::all()
                                    .add(
                                        Expr::col(("pg_loco_queue", "name"))
                                            .eq(Expr::val("ImportWorker")),
                                    )
                                    .add(
                                        Expr::col(("pg_loco_queue", "status"))
                                            .is_in(["queued", "processing"]),
                                    ),
                            )
                            .to_owned(),
                    ),
                )
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
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_imports", "status")))
                    .expr(Expr::col(("nhs_dmd_imports", "archive_key")))
                    .expr(Expr::col(("nhs_dmd_imports", "error_message")))
                    .from("nhs_dmd_imports")
                    .cond_where(Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(id)))
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
    let imported = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_barcodes", "code")))
                    .from("nhs_dmd_barcodes")
                    .cond_where(
                        Expr::col(("nhs_dmd_barcodes", "gtin")).eq(Expr::val("5016298210989")),
                    )
                    .to_owned(),
            ),
        )
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
    crate::reference_fixture::clean_nhs_catalogue().await;
    let app = Application::new().await;
    let stamp = chrono::Utc::now().naive_utc() - chrono::Duration::minutes(31);
    let seeded_import = rows::nhs_dmd_imports::ActiveModel {
        uploaded_filename: Set("stale.zip".into()),
        status: Set(3),
        created_at: Set(stamp),
        updated_at: Set(stamp),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    med_tracker::models::nhs_dmd::reconcile(&app.context)
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_imports", "status")))
                    .from("nhs_dmd_imports")
                    .cond_where(
                        Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(seeded_import.id)),
                    )
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
    let status: i32 = row.try_get("", "status").unwrap();
    app.close().await;
    assert_eq!(status, 5);
}

#[tokio::test]
async fn nhs_dmd_reconciliation_fails_stalled_import_even_while_worker_owns_lock() {
    crate::reference_fixture::clean_nhs_catalogue().await;
    let app = Application::new().await;
    let stamp = chrono::Utc::now().naive_utc() - chrono::Duration::minutes(31);
    let seeded_import = rows::nhs_dmd_imports::ActiveModel {
        uploaded_filename: Set("stalled.zip".into()),
        status: Set(3),
        created_at: Set(stamp),
        updated_at: Set(stamp),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    let lease = app.fixture.admin.begin().await.unwrap();
    lease
        .query_one_raw(
            lease.get_database_backend().build(
                &Query::select()
                    .expr(Func::cust("pg_advisory_xact_lock").args([Expr::val(7369918_i64)]))
                    .to_owned(),
            ),
        )
        .await
        .unwrap();
    med_tracker::models::nhs_dmd::reconcile(&app.context)
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_imports", "status")))
                    .from("nhs_dmd_imports")
                    .cond_where(
                        Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(seeded_import.id)),
                    )
                    .to_owned(),
            ),
        )
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
    crate::reference_fixture::clean_nhs_catalogue().await;
    let app = Application::new().await;
    let stamp = chrono::Utc::now().naive_utc();
    let seeded_import = rows::nhs_dmd_imports::ActiveModel {
        uploaded_filename: Set("lost.zip".into()),
        status: Set(0),
        created_at: Set(stamp),
        updated_at: Set(stamp),
        ..Default::default()
    }
    .insert(&app.fixture.admin)
    .await
    .unwrap();
    med_tracker::models::nhs_dmd::run(&app.context, seeded_import.id)
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_imports", "status")))
                    .expr(Expr::col(("nhs_dmd_imports", "error_message")))
                    .from("nhs_dmd_imports")
                    .cond_where(
                        Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(seeded_import.id)),
                    )
                    .to_owned(),
            ),
        )
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
    crate::reference_fixture::clean_nhs_catalogue().await;
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
    crate::reference_fixture::clean_nhs_catalogue().await;
    let app = Application::new().await;
    let id = med_tracker::models::nhs_dmd::upload(&app.context, "synthetic.zip", release())
        .await
        .unwrap();
    let row = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_imports", "archive_key")))
                    .from("nhs_dmd_imports")
                    .cond_where(Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(id)))
                    .to_owned(),
            ),
        )
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
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_imports", "status")))
                    .expr(Expr::col(("nhs_dmd_imports", "error_message")))
                    .expr(Expr::col(("nhs_dmd_imports", "archive_key")))
                    .from("nhs_dmd_imports")
                    .cond_where(Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(id)))
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
    let status: i32 = row.try_get("", "status").unwrap();
    let error: String = row.try_get("", "error_message").unwrap();
    let key: Option<String> = row.try_get("", "archive_key").unwrap();
    let catalog = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_barcodes", "code")))
                    .from("nhs_dmd_barcodes")
                    .cond_where(
                        Expr::col(("nhs_dmd_barcodes", "gtin")).eq(Expr::val("5016298210989")),
                    )
                    .to_owned(),
            ),
        )
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
    crate::reference_fixture::clean_nhs_catalogue().await;
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
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_imports", "status")))
                    .expr(Expr::col(("nhs_dmd_imports", "error_message")))
                    .from("nhs_dmd_imports")
                    .cond_where(Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(id)))
                    .to_owned(),
            ),
        )
        .await
        .unwrap()
        .unwrap();
    let status: i32 = row.try_get("", "status").unwrap();
    let error: Option<String> = row.try_get("", "error_message").unwrap();
    let catalog = app
        .fixture
        .admin
        .query_one_raw(
            app.fixture.admin.get_database_backend().build(
                &Query::select()
                    .expr(Expr::col(("nhs_dmd_barcodes", "code")))
                    .from("nhs_dmd_barcodes")
                    .cond_where(
                        Expr::col(("nhs_dmd_barcodes", "gtin")).eq(Expr::val("5016298210989")),
                    )
                    .to_owned(),
            ),
        )
        .await
        .unwrap();
    app.close().await;
    assert_eq!(status, 5);
    assert!(error.is_some());
    assert!(catalog.is_none());
}

#[tokio::test]
async fn nhs_dmd_retries_terminal_cleanup_without_restarting_import() {
    crate::reference_fixture::clean_nhs_catalogue().await;
    for (bytes, expected_status) in [(release(), 4), (b"not a zip release".to_vec(), 5)] {
        let app = Application::new().await;
        let id = med_tracker::models::nhs_dmd::upload(&app.context, "synthetic.zip", bytes)
            .await
            .unwrap();
        // SeaQuery TableAlterStatement cannot mark a fault-injection CHECK constraint NOT VALID; these cases require exclusive execution.
        app.fixture.admin.execute_unprepared("ALTER TABLE nhs_dmd_imports ADD CONSTRAINT synthetic_cleanup_failure CHECK (archive_key IS NOT NULL) NOT VALID").await.unwrap();
        assert!(
            med_tracker::models::nhs_dmd::run(&app.context, id)
                .await
                .is_err()
        );
        let row = app
            .fixture
            .admin
            .query_one_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::select()
                        .expr(Expr::col(("nhs_dmd_imports", "status")))
                        .expr(Expr::col(("nhs_dmd_imports", "archive_key")))
                        .expr(Expr::col(("nhs_dmd_imports", "error_message")))
                        .from("nhs_dmd_imports")
                        .cond_where(Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(id)))
                        .to_owned(),
                ),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<i32>("", "status").unwrap(), expected_status);
        assert!(
            row.try_get::<Option<String>>("", "archive_key")
                .unwrap()
                .is_some()
        );
        let diagnostic: Option<String> = row.try_get("", "error_message").unwrap();
        assert_eq!(diagnostic.is_some(), expected_status == 5);
        app.fixture
            .admin
            .execute_raw(
                app.fixture.admin.get_database_backend().build(
                    &sea_orm::sea_query::Table::alter()
                        .table("nhs_dmd_imports")
                        .drop_constraint("synthetic_cleanup_failure")
                        .to_owned(),
                ),
            )
            .await
            .unwrap();
        med_tracker::models::nhs_dmd::run(&app.context, id)
            .await
            .unwrap();
        let row = app
            .fixture
            .admin
            .query_one_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::select()
                        .expr(Expr::col(("nhs_dmd_imports", "status")))
                        .expr(Expr::col(("nhs_dmd_imports", "archive_key")))
                        .expr(Expr::col(("nhs_dmd_imports", "error_message")))
                        .from("nhs_dmd_imports")
                        .cond_where(Expr::col(("nhs_dmd_imports", "id")).eq(Expr::val(id)))
                        .to_owned(),
                ),
            )
            .await
            .unwrap()
            .unwrap();
        let catalog = app
            .fixture
            .admin
            .query_one_raw(
                app.fixture.admin.get_database_backend().build(
                    &Query::select()
                        .expr(Expr::col(("nhs_dmd_barcodes", "code")))
                        .from("nhs_dmd_barcodes")
                        .cond_where(
                            Expr::col(("nhs_dmd_barcodes", "gtin")).eq(Expr::val("5016298210989")),
                        )
                        .to_owned(),
                ),
            )
            .await
            .unwrap();
        app.close().await;
        assert_eq!(row.try_get::<i32>("", "status").unwrap(), expected_status);
        assert!(
            row.try_get::<Option<String>>("", "archive_key")
                .unwrap()
                .is_none()
        );
        assert!(catalog.is_some());
        assert_eq!(
            row.try_get::<Option<String>>("", "error_message").unwrap(),
            diagnostic
        );
    }
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
