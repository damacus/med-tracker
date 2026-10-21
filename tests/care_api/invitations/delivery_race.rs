use super::*;
use loco_rs::{bgworker::BackgroundWorker, mailer::EmailSender};
use med_tracker::models::{
    access::{self, Actor, HouseholdScope},
    care::invitations::{
        self,
        delivery::{InvitationMail, InvitationMailWorker},
    },
};
use sea_orm::TransactionTrait;

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn waiting(app: &Application, query: &str) -> TestResult {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let row = app.fixture.admin.query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query ILIKE $1 AND cardinality(pg_blocking_pids(pid))>0) AS waiting",
                [query.into()],
            )).await?.ok_or("lock observation missing")?;
            if row.try_get::<bool>("", "waiting")? { return Ok::<_,Box<dyn std::error::Error>>(()); }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await??;
    Ok(())
}

async fn queued(app: &Application) -> Result<InvitationMail, Box<dyn std::error::Error>> {
    let tenant = access::begin(
        &app.fixture.runtime,
        &HouseholdScope {
            actor: Actor { account_id: 71001 },
            household_id: 72001,
            request_id: "deterministic-live-invitation-race".into(),
        },
        None,
    )
    .await?;
    invitations::create(
        &tenant,
        json!({"email":"race@example.test","membership_role":"member"}),
        Some(&url::Url::parse(
            "https://synthetic.example.test/invitations/accept",
        )?),
        None,
    )
    .await?;
    tenant.commit().await?;
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT task_data FROM pg_loco_queue ORDER BY id DESC LIMIT 1",
        ))
        .await?
        .ok_or("queued mail missing")?;
    let payload: serde_json::Value = row.try_get("", "task_data")?;
    Ok(serde_json::from_value(payload)?)
}

#[tokio::test]
async fn invitation_live_duplicate_cannot_suppress_a_committed_start() -> TestResult {
    let mut app = Application::new().await;
    app.context.mailer = Some(EmailSender::stub());
    let mail = queued(&app).await?;
    app.fixture.admin.execute_unprepared("CREATE FUNCTION synthetic_invitation_start_gate() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF OLD.started_at IS NULL AND NEW.started_at IS NOT NULL THEN PERFORM pg_advisory_xact_lock(1829712041,1); END IF; RETURN NEW; END $$; CREATE TRIGGER synthetic_invitation_start_gate AFTER UPDATE ON invitation_mail_attempts FOR EACH ROW EXECUTE FUNCTION synthetic_invitation_start_gate()").await?;
    let blocker = app.fixture.admin.begin().await?;
    blocker
        .execute_unprepared("SELECT pg_advisory_xact_lock(1829712041,1)")
        .await?;
    let before = app
        .context
        .mailer
        .as_ref()
        .ok_or("mailer missing")?
        .deliveries()
        .count;
    let first = InvitationMailWorker::build(&app.context);
    let first_mail = mail.clone();
    let first = tokio::spawn(async move { first.perform(first_mail).await });
    waiting(&app, "%UPDATE invitation_mail_attempts SET started_at%").await?;
    let duplicate = InvitationMailWorker::build(&app.context);
    let duplicate_mail = mail.clone();
    let duplicate = tokio::spawn(async move { duplicate.perform(duplicate_mail).await });
    waiting(&app, "%households%FOR UPDATE%").await?;
    blocker.rollback().await?;
    tokio::time::timeout(std::time::Duration::from_secs(5), first).await???;
    tokio::time::timeout(std::time::Duration::from_secs(5), duplicate).await???;
    InvitationMailWorker::build(&app.context)
        .perform(mail)
        .await?;
    let deliveries = app
        .context
        .mailer
        .as_ref()
        .ok_or("mailer missing")?
        .deliveries()
        .count
        - before;
    let receipt = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres, "SELECT status,category,started_at IS NOT NULL AND finished_at IS NOT NULL AS finished FROM invitation_mail_attempts")).await?.ok_or("receipt missing")?;
    app.close().await;
    assert_eq!(
        deliveries, 1,
        "a healthy duplicate must neither suppress nor repeat delivery"
    );
    assert_eq!(receipt.try_get::<String>("", "status")?, "sent_to_provider");
    assert_eq!(receipt.try_get::<Option<String>>("", "category")?, None);
    assert!(receipt.try_get::<bool>("", "finished")?);
    Ok(())
}

#[tokio::test]
async fn invitation_marker_capacity_failure_is_terminal_and_new_invitation_recovers() -> TestResult
{
    let mut app = Application::new_single_connection().await;
    app.context.mailer = Some(EmailSender::stub());
    let mail = queued(&app).await?;
    let worker = InvitationMailWorker::build(&app.context);
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(8),
        worker.perform(mail.clone()),
    )
    .await?;
    assert!(
        result.is_err(),
        "single connection cannot persist the separate durable marker"
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status,category,started_at IS NULL AS unstarted FROM invitation_mail_attempts",
        ))
        .await?
        .ok_or("attempt missing")?;
    assert_eq!(row.try_get::<String>("", "status")?, "failed");
    assert_eq!(row.try_get::<String>("", "category")?, "unavailable");
    assert!(row.try_get::<bool>("", "unstarted")?);
    assert_eq!(
        app.context
            .mailer
            .as_ref()
            .ok_or("mailer missing")?
            .deliveries()
            .count,
        0
    );
    app.context.db = app.fixture.runtime.clone();
    let tenant = access::begin(
        &app.fixture.runtime,
        &HouseholdScope {
            actor: Actor { account_id: 71001 },
            household_id: 72001,
            request_id: "explicit-capacity-recovery".into(),
        },
        None,
    )
    .await?;
    let invitation = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id FROM household_invitations",
        ))
        .await?
        .ok_or("invitation missing")?
        .try_get::<i64>("", "id")?;
    invitations::cancel(&tenant, invitation, None).await?;
    invitations::create(
        &tenant,
        json!({"email":"race@example.test","membership_role":"member"}),
        Some(&url::Url::parse(
            "https://synthetic.example.test/invitations/accept",
        )?),
        None,
    )
    .await?;
    tenant.commit().await?;
    let payload = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT task_data FROM pg_loco_queue ORDER BY id DESC LIMIT 1",
        ))
        .await?
        .ok_or("resend missing")?
        .try_get::<serde_json::Value>("", "task_data")?;
    let worker = InvitationMailWorker::build(&app.context);
    worker.perform(serde_json::from_value(payload)?).await?;
    worker.perform(mail).await?;
    assert_eq!(
        app.context
            .mailer
            .as_ref()
            .ok_or("mailer missing")?
            .deliveries()
            .count,
        1
    );
    let sent = app.fixture.admin.query_one_raw(Statement::from_string(DbBackend::Postgres,"SELECT count(*) AS count FROM invitation_mail_attempts WHERE status='sent_to_provider'")).await?.ok_or("receipt missing")?.try_get::<i64>("","count")?;
    app.close().await;
    assert_eq!(sent, 1);
    Ok(())
}

#[tokio::test]
async fn invitation_stalled_provider_times_out_as_uncertain_without_retry() -> TestResult {
    let mut app =
        Application::new_configured(Some("synthetic-dose-occurrence-signing-key"), |settings| {
            settings["invitation_mail"] = json!({"send_timeout_seconds":1});
        })
        .await;
    let mail = queued(&app).await?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let config = serde_json::from_value(
        json!({"enable":true,"host":"127.0.0.1","port":listener.local_addr()?.port(),"secure":false}),
    )?;
    app.context.mailer = Some(EmailSender::smtp(&config)?);
    let worker = InvitationMailWorker::build(&app.context);
    let mail_copy = mail.clone();
    let sending = tokio::spawn(async move { worker.perform(mail_copy).await });
    let (socket, _) =
        tokio::time::timeout(std::time::Duration::from_secs(2), listener.accept()).await??;
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(3), sending).await;
    drop(socket);
    let completed = match outcome {
        Ok(result) => {
            result??;
            true
        }
        Err(_) => false,
    };
    let receipt = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status,category FROM invitation_mail_attempts",
        ))
        .await?
        .ok_or("receipt missing")?;
    if completed {
        InvitationMailWorker::build(&app.context)
            .perform(mail)
            .await?;
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(100), listener.accept())
                .await
                .is_err()
        );
    }
    app.close().await;
    assert!(
        completed,
        "whole-provider deadline must bound an accepted SMTP socket that sends no greeting"
    );
    assert_eq!(receipt.try_get::<String>("", "status")?, "failed");
    assert_eq!(receipt.try_get::<String>("", "category")?, "uncertain");
    Ok(())
}

#[tokio::test]
async fn invitation_invalid_deadline_records_unavailable_without_starting() -> TestResult {
    let mut app =
        Application::new_configured(Some("synthetic-dose-occurrence-signing-key"), |settings| {
            settings["invitation_mail"] = json!({"send_timeout_seconds":0});
        })
        .await;
    app.context.mailer = Some(EmailSender::stub());
    let mail = queued(&app).await?;
    assert!(
        InvitationMailWorker::build(&app.context)
            .perform(mail)
            .await
            .is_err()
    );
    let row = app
        .fixture
        .admin
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status,category,started_at IS NULL AS unstarted FROM invitation_mail_attempts",
        ))
        .await?
        .ok_or("attempt missing")?;
    let delivered = app
        .context
        .mailer
        .as_ref()
        .ok_or("mailer missing")?
        .deliveries()
        .count;
    app.close().await;
    assert_eq!(delivered, 0);
    assert_eq!(row.try_get::<String>("", "status")?, "failed");
    assert_eq!(row.try_get::<String>("", "category")?, "unavailable");
    assert!(row.try_get::<bool>("", "unstarted")?);
    Ok(())
}
