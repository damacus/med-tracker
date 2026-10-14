use super::*;
use med_tracker::models::platform::expiry;
use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};
use tracing::instrument::WithSubscriber;
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

#[derive(Clone, Default)]
struct CanaryEvents(Arc<Mutex<Vec<serde_json::Value>>>);

#[derive(Default)]
struct CanaryFields(serde_json::Map<String, serde_json::Value>);

impl Visit for CanaryFields {
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0.insert(field.name().into(), value.into());
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().into(), value.into());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().into(), format!("{value:?}").into());
    }
}

impl Subscriber for CanaryEvents {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.fields().field("medtracker.canary.kind").is_some()
    }

    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(1)
    }

    fn record(&self, _: &Id, _: &Record<'_>) {}

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut fields = CanaryFields::default();
        event.record(&mut fields);
        self.0.lock().unwrap().push(fields.0.into());
    }

    fn enter(&self, _: &Id) {}

    fn exit(&self, _: &Id) {}
}

async fn platform_admin(app: &Application) {
    app.fixture
        .admin
        .execute_unprepared(
            "INSERT INTO platform_admins(account_id,status,created_at,updated_at) VALUES(71001,'active',now(),now()) ON CONFLICT(account_id) DO UPDATE SET status='active',updated_at=now()",
        )
        .await
        .unwrap();
}

async fn insert_session(
    app: &Application,
    expires_at: &str,
    ended_at: &str,
    expired_at: &str,
) -> i64 {
    app.fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            format!("INSERT INTO support_access_sessions(household_id,platform_admin_id,reason,request_id,ip,starts_at,expires_at,ended_at,expired_at,created_at,updated_at) VALUES(72001,(SELECT id FROM platform_admins WHERE account_id=71001),'Synthetic expiry session','expiry-request','127.0.0.1',now() - interval '1 day',{expires_at},{ended_at},{expired_at},now(),now()) RETURNING id"),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "id")
        .unwrap()
}

async fn session_expired(app: &Application, id: i64) -> bool {
    app.fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT expired_at IS NOT NULL AS expired FROM support_access_sessions WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .and_then(|row| row.try_get::<bool>("", "expired").ok())
        .unwrap_or_default()
}

async fn expiry_audits(app: &Application, id: i64) -> i64 {
    app.fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM versions WHERE item_type='SupportAccessSession' AND item_id=$1 AND event='platform/support/expired'",
            [id.into()],
        ))
        .await
        .unwrap()
        .and_then(|row| row.try_get::<i64>("", "count").ok())
        .unwrap_or_default()
}

#[test]
fn support_expiry_task_is_registered() {
    use loco_rs::app::Hooks;
    let mut tasks = loco_rs::task::Tasks::default();
    med_tracker::app::App::register_tasks(&mut tasks);
    assert!(tasks.names().contains(&"support-access-expire".to_owned()));
}

#[tokio::test]
async fn support_expiry_expires_due_sessions_once() {
    let app = Application::new().await;
    platform_admin(&app).await;
    let id = insert_session(&app, "now() - interval '1 minute'", "NULL", "NULL").await;
    let processed = expiry::expire_due_support_sessions(&app.context)
        .await
        .unwrap();
    let expired = session_expired(&app, id).await;
    let audit = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT object,whodunnit,request_id,household_id,audit_context->>'actor_account_id' AS actor,audit_context->>'active_role' AS role,audit_context->>'actor_user_id' AS actor_user FROM versions WHERE item_type='SupportAccessSession' AND item_id=$1 AND event='platform/support/expired'",
            [id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    let object: String = audit.try_get("", "object").unwrap();
    let whodunnit: Option<String> = audit.try_get("", "whodunnit").unwrap();
    let request_id: Option<String> = audit.try_get("", "request_id").unwrap();
    let household_id: i64 = audit.try_get("", "household_id").unwrap();
    let actor: String = audit.try_get("", "actor").unwrap();
    let role: String = audit.try_get("", "role").unwrap();
    let actor_user: Option<String> = audit.try_get("", "actor_user").unwrap();
    let audits = expiry_audits(&app, id).await;
    app.close().await;
    assert_eq!(processed, 1);
    assert!(expired);
    assert_eq!(audits, 1);
    assert_eq!(object, "{\"household_id\":72001}");
    assert_eq!(household_id, 72001);
    assert_eq!(whodunnit, None);
    assert_eq!(request_id, None);
    assert_eq!(actor, "71001");
    assert_eq!(role, "support_expiry_task");
    assert_eq!(actor_user, None);
}

#[tokio::test]
async fn support_expiry_ignores_ended_future_and_expired() {
    let app = Application::new().await;
    platform_admin(&app).await;
    let ended = insert_session(
        &app,
        "now() - interval '1 minute'",
        "now() - interval '1 minute'",
        "NULL",
    )
    .await;
    let future = insert_session(&app, "now() + interval '1 hour'", "NULL", "NULL").await;
    let expired = insert_session(
        &app,
        "now() - interval '1 minute'",
        "NULL",
        "now() - interval '1 minute'",
    )
    .await;
    let processed = expiry::expire_due_support_sessions(&app.context)
        .await
        .unwrap();
    let states = [
        session_expired(&app, ended).await,
        session_expired(&app, future).await,
    ];
    let audits = [
        expiry_audits(&app, ended).await,
        expiry_audits(&app, future).await,
        expiry_audits(&app, expired).await,
    ];
    app.close().await;
    assert_eq!(processed, 0);
    assert_eq!(states, [false, false]);
    assert_eq!(audits, [0, 0, 0]);
}

#[tokio::test]
async fn support_expiry_is_idempotent() {
    let app = Application::new().await;
    platform_admin(&app).await;
    let id = insert_session(&app, "now() - interval '1 minute'", "NULL", "NULL").await;
    let first = expiry::expire_due_support_sessions(&app.context)
        .await
        .unwrap();
    let second = expiry::expire_due_support_sessions(&app.context)
        .await
        .unwrap();
    let audits = expiry_audits(&app, id).await;
    app.close().await;
    assert_eq!(first, 1);
    assert_eq!(second, 0);
    assert_eq!(audits, 1);
}

#[tokio::test]
async fn support_expiry_continues_past_a_failing_row() {
    let app = Application::new().await;
    platform_admin(&app).await;
    let failing = insert_session(&app, "now() - interval '1 minute'", "NULL", "NULL").await;
    let passing = insert_session(&app, "now() - interval '1 minute'", "NULL", "NULL").await;
    app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION reject_support_expiry_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.item_type='SupportAccessSession' AND NEW.item_id={failing} THEN RAISE EXCEPTION 'synthetic support expiry audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_support_expiry_audit BEFORE INSERT ON versions FOR EACH ROW EXECUTE FUNCTION reject_support_expiry_audit()")).await.unwrap();
    let processed = expiry::expire_due_support_sessions(&app.context)
        .await
        .unwrap();
    let states = [
        session_expired(&app, failing).await,
        session_expired(&app, passing).await,
    ];
    let audits = [
        expiry_audits(&app, failing).await,
        expiry_audits(&app, passing).await,
    ];
    app.close().await;
    assert_eq!(processed, 1);
    assert_eq!(states, [false, true]);
    assert_eq!(audits, [0, 1]);
}

#[test]
fn observability_canary_task_is_registered() {
    use loco_rs::app::Hooks;
    let mut tasks = loco_rs::task::Tasks::default();
    med_tracker::app::App::register_tasks(&mut tasks);
    assert!(tasks.names().contains(&"observability-canary".to_owned()));
}

#[tokio::test]
async fn observability_canary_emits_both_kinds_with_safe_fields() {
    let app = Application::new().await;
    let events = CanaryEvents::default();
    med_tracker::models::observability::emit_canary(&app.context)
        .with_subscriber(events.clone())
        .await
        .unwrap();
    app.close().await;
    let captured = events.0.lock().unwrap().clone();
    assert_eq!(captured.len(), 2);
    let kinds: Vec<&str> = captured
        .iter()
        .map(|event| event["medtracker.canary.kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["application_event", "job"]);
    for event in &captured {
        assert_eq!(event["event.name"], "observability.canary");
        assert_eq!(event["event.outcome"], "success");
        assert_eq!(event["event.dataset"], "medtracker.application");
        assert_eq!(event["medtracker.reason"], "canary_emitted");
        assert_eq!(event["medtracker.schema.version"], 1);
        assert!(event["event.id"].as_str().unwrap().len() == 36);
        assert!(event["workflow.id"].as_str().unwrap().len() == 36);
        assert!(event["attempt.id"].as_str().unwrap().len() == 36);
    }
    assert_eq!(
        captured[0]["workflow.id"].as_str().unwrap(),
        captured[1]["workflow.id"].as_str().unwrap()
    );
    assert_ne!(
        captured[0]["event.id"].as_str().unwrap(),
        captured[1]["event.id"].as_str().unwrap()
    );
}

async fn insert_export(
    app: &Application,
    status: &str,
    expires_at: &str,
    generation_started_at: &str,
) -> i64 {
    app.fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            format!("INSERT INTO household_exports(household_id,requested_by_account_id,requested_at,status,manifest,expires_at,generation_started_at,created_at,updated_at) VALUES(72001,71001,now() - interval '1 day','{status}','{{}}',{expires_at},{generation_started_at},now(),now()) RETURNING id"),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "id")
        .unwrap()
}

async fn attach_artifact(app: &Application, export_id: i64, key: &str, service: &str) -> i64 {
    let blob_id = app
        .fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO active_storage_blobs(byte_size,filename,key,service_name,content_type,created_at) VALUES(4,'export.zip',$1,$2,'application/zip',now()) RETURNING id",
            [key.into(), service.into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<i64>("", "id")
        .unwrap();
    app.fixture
        .admin
        .execute_unprepared(&format!(
            "INSERT INTO active_storage_attachments(blob_id,created_at,household_id,name,record_id,record_type) VALUES({blob_id},now(),72001,'artifact',{export_id},'HouseholdExport')"
        ))
        .await
        .unwrap();
    blob_id
}

async fn export_status(app: &Application, id: i64) -> String {
    app.fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT status FROM household_exports WHERE id=$1",
            [id.into()],
        ))
        .await
        .unwrap()
        .and_then(|row| row.try_get::<String>("", "status").ok())
        .unwrap_or_default()
}

async fn export_audits(app: &Application, export_id: i64) -> i64 {
    app.fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM security_audit_events WHERE event_type='household.export.expired' AND metadata->>'export_id'=$1",
            [export_id.to_string().into()],
        ))
        .await
        .unwrap()
        .and_then(|row| row.try_get::<i64>("", "count").ok())
        .unwrap_or_default()
}

async fn attachment_rows(app: &Application, export_id: i64) -> i64 {
    app.fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM active_storage_attachments WHERE record_type='HouseholdExport' AND record_id=$1 AND name='artifact'",
            [export_id.into()],
        ))
        .await
        .unwrap()
        .and_then(|row| row.try_get::<i64>("", "count").ok())
        .unwrap_or_default()
}

async fn blob_exists(app: &Application, blob_id: i64) -> bool {
    app.fixture
        .admin
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM active_storage_blobs WHERE id=$1",
            [blob_id.into()],
        ))
        .await
        .unwrap()
        .and_then(|row| row.try_get::<i64>("", "count").ok())
        .unwrap_or_default()
        > 0
}

async fn place_retention_hold(app: &Application) {
    app.fixture
        .admin
        .execute_unprepared(
            "INSERT INTO household_retention_holds(approved_by_account_id,household_id,placed_at,reason,review_on,status,created_at,updated_at) VALUES(71001,72001,now(),'Synthetic hold',current_date + 30,'active',now(),now())",
        )
        .await
        .unwrap();
}

fn fs_operator(dir: &tempfile::TempDir) -> opendal::Operator {
    opendal::Operator::new(opendal::services::Fs::default().root(dir.path().to_str().unwrap()))
        .unwrap()
}

fn install_stores(
    app: &Application,
    stores: std::collections::BTreeMap<String, Vec<opendal::Operator>>,
) {
    app.context
        .shared_store
        .insert(med_tracker::models::storage::ArtifactStores(stores));
}

#[test]
fn household_exports_expiry_task_is_registered() {
    use loco_rs::app::Hooks;
    let mut tasks = loco_rs::task::Tasks::default();
    med_tracker::app::App::register_tasks(&mut tasks);
    assert!(
        tasks
            .names()
            .contains(&"household-exports-expire".to_owned())
    );
}

#[tokio::test]
async fn export_expiry_expires_due_ready_export_and_removes_artifact() {
    let app = Application::new().await;
    let dir = tempfile::tempdir().unwrap();
    let operator = fs_operator(&dir);
    operator.write("due-blob-key", vec![9_u8; 4]).await.unwrap();
    install_stores(
        &app,
        [("persistent".to_owned(), vec![operator])]
            .into_iter()
            .collect(),
    );
    let export = insert_export(&app, "ready", "now() - interval '1 minute'", "NULL").await;
    let blob = attach_artifact(&app, export, "due-blob-key", "persistent").await;
    let processed = med_tracker::models::care::hosted_exports::expire_due_exports(&app.context)
        .await
        .unwrap();
    let status = export_status(&app, export).await;
    let attachments = attachment_rows(&app, export).await;
    let blob_exists = blob_exists(&app, blob).await;
    let audits = export_audits(&app, export).await;
    let bytes = dir.path().join("due-blob-key").exists();
    app.close().await;
    assert_eq!(processed, 1);
    assert_eq!(status, "expired");
    assert_eq!(attachments, 0);
    assert!(!blob_exists);
    assert_eq!(audits, 1);
    assert!(!bytes);
}

#[tokio::test]
async fn export_expiry_blocks_during_active_retention_hold() {
    let app = Application::new().await;
    let export = insert_export(&app, "ready", "now() - interval '1 minute'", "NULL").await;
    attach_artifact(&app, export, "held-blob-key", "persistent").await;
    place_retention_hold(&app).await;
    let processed = med_tracker::models::care::hosted_exports::expire_due_exports(&app.context)
        .await
        .unwrap();
    let status = export_status(&app, export).await;
    let attachments = attachment_rows(&app, export).await;
    let audits = export_audits(&app, export).await;
    app.close().await;
    assert_eq!(processed, 0);
    assert_eq!(status, "ready");
    assert_eq!(attachments, 1);
    assert_eq!(audits, 0);
}

#[tokio::test]
async fn export_expiry_ignores_not_due_and_terminal_exports() {
    let app = Application::new().await;
    let future = insert_export(&app, "ready", "now() + interval '1 day'", "NULL").await;
    let expired = insert_export(&app, "expired", "now() - interval '1 minute'", "NULL").await;
    let failed = insert_export(&app, "failed", "now() - interval '1 minute'", "NULL").await;
    let processed = med_tracker::models::care::hosted_exports::expire_due_exports(&app.context)
        .await
        .unwrap();
    let statuses = [
        export_status(&app, future).await,
        export_status(&app, expired).await,
        export_status(&app, failed).await,
    ];
    let audits = [
        export_audits(&app, future).await,
        export_audits(&app, expired).await,
        export_audits(&app, failed).await,
    ];
    app.close().await;
    assert_eq!(processed, 0);
    assert_eq!(statuses, ["ready", "expired", "failed"]);
    assert_eq!(audits, [0, 0, 0]);
}

#[tokio::test]
async fn export_expiry_expires_stale_generating_only_with_artifact() {
    let app = Application::new().await;
    let dir = tempfile::tempdir().unwrap();
    install_stores(
        &app,
        [("persistent".to_owned(), vec![fs_operator(&dir)])]
            .into_iter()
            .collect(),
    );
    let stale = insert_export(&app, "generating", "NULL", "now() - interval '61 minutes'").await;
    attach_artifact(&app, stale, "stale-blob-key", "persistent").await;
    let recent = insert_export(&app, "generating", "NULL", "now() - interval '5 minutes'").await;
    attach_artifact(&app, recent, "recent-blob-key", "persistent").await;
    let detached = insert_export(&app, "generating", "NULL", "now() - interval '61 minutes'").await;
    let processed = med_tracker::models::care::hosted_exports::expire_due_exports(&app.context)
        .await
        .unwrap();
    let statuses = [
        export_status(&app, stale).await,
        export_status(&app, recent).await,
        export_status(&app, detached).await,
    ];
    app.close().await;
    assert_eq!(processed, 1);
    assert_eq!(statuses, ["expired", "generating", "generating"]);
}

#[tokio::test]
async fn export_expiry_is_idempotent_for_household_exports() {
    let app = Application::new().await;
    let export = insert_export(&app, "downloaded", "now() - interval '1 minute'", "NULL").await;
    let first = med_tracker::models::care::hosted_exports::expire_due_exports(&app.context)
        .await
        .unwrap();
    let second = med_tracker::models::care::hosted_exports::expire_due_exports(&app.context)
        .await
        .unwrap();
    let audits = export_audits(&app, export).await;
    app.close().await;
    assert_eq!(first, 1);
    assert_eq!(second, 0);
    assert_eq!(audits, 1);
}

#[tokio::test]
async fn export_expiry_continues_past_a_failing_export() {
    let app = Application::new().await;
    let failing = insert_export(&app, "ready", "now() - interval '1 minute'", "NULL").await;
    let passing = insert_export(&app, "ready", "now() - interval '1 minute'", "NULL").await;
    app.fixture.admin.execute_unprepared(&format!("CREATE FUNCTION reject_export_expiry_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='household.export.expired' AND NEW.metadata->>'export_id'='{failing}' THEN RAISE EXCEPTION 'synthetic export expiry audit rejection'; END IF; RETURN NEW; END $$; CREATE TRIGGER reject_export_expiry_audit BEFORE INSERT ON security_audit_events FOR EACH ROW EXECUTE FUNCTION reject_export_expiry_audit()")).await.unwrap();
    let processed = med_tracker::models::care::hosted_exports::expire_due_exports(&app.context)
        .await
        .unwrap();
    let statuses = [
        export_status(&app, failing).await,
        export_status(&app, passing).await,
    ];
    let audits = [
        export_audits(&app, failing).await,
        export_audits(&app, passing).await,
    ];
    app.close().await;
    assert_eq!(processed, 1);
    assert_eq!(statuses, ["ready", "expired"]);
    assert_eq!(audits, [0, 1]);
}

#[tokio::test]
async fn export_expiry_deletes_bytes_from_every_mirror_backend() {
    let app = Application::new().await;
    let primary = tempfile::tempdir().unwrap();
    let mirror = tempfile::tempdir().unwrap();
    fs_operator(&primary)
        .write("mirror-blob-key", vec![7_u8; 4])
        .await
        .unwrap();
    fs_operator(&mirror)
        .write("mirror-blob-key", vec![7_u8; 4])
        .await
        .unwrap();
    install_stores(
        &app,
        [(
            "persistent_with_s3_mirror".to_owned(),
            vec![fs_operator(&primary), fs_operator(&mirror)],
        )]
        .into_iter()
        .collect(),
    );
    let export = insert_export(&app, "ready", "now() - interval '1 minute'", "NULL").await;
    attach_artifact(&app, export, "mirror-blob-key", "persistent_with_s3_mirror").await;
    let processed = med_tracker::models::care::hosted_exports::expire_due_exports(&app.context)
        .await
        .unwrap();
    let primary_gone = !primary.path().join("mirror-blob-key").exists();
    let mirror_gone = !mirror.path().join("mirror-blob-key").exists();
    app.close().await;
    assert_eq!(processed, 1);
    assert!(primary_gone);
    assert!(mirror_gone);
}

#[test]
fn inventoried_rails_jobs_have_loco_replacements() {
    use loco_rs::app::Hooks;
    let replacements: [(&str, &str); 6] = [
        ("ExpireSupportAccessSessionsJob", "support-access-expire"),
        ("ExpireHouseholdExportsJob", "household-exports-expire"),
        ("ObservabilityCanaryJob", "observability-canary"),
        (
            "MedicationReviewEvidenceRefreshJob",
            "medication-review-evidence-refresh",
        ),
        ("NhsDmdImportReconciliationJob", "nhs_dmd_reconcile"),
        ("ScheduleDailyRemindersJob", "browser-reminders"),
    ];
    let mut tasks = loco_rs::task::Tasks::default();
    med_tracker::app::App::register_tasks(&mut tasks);
    let names = tasks.names();
    let missing: Vec<&str> = replacements
        .iter()
        .filter(|(_, task)| !names.contains(&(*task).to_owned()))
        .map(|(job, _)| *job)
        .collect();
    assert_eq!(missing, Vec::<&str>::new());
}
