use super::*;
use med_tracker::models::platform::expiry;

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
