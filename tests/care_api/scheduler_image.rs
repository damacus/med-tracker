use super::fixture::Fixture;
use med_tracker::models::_entities::{
    platform_admins, security_audit_events, support_access_sessions, versions,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::{process::Command, time::Duration};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn docker(arguments: &[&str]) -> TestResult<String> {
    let output = Command::new("docker").args(arguments).output()?;
    if !output.status.success() {
        return Err(format!(
            "docker {}: {}",
            arguments.first().copied().unwrap_or(""),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

struct Container(String);

impl Drop for Container {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "--force", &self.0])
            .output();
    }
}

async fn seed_session(fixture: &Fixture, platform_admin_id: i64) -> TestResult<i64> {
    let now = chrono::Utc::now().naive_utc();
    let session = support_access_sessions::ActiveModel {
        household_id: Set(fixture.household.id),
        platform_admin_id: Set(platform_admin_id),
        reason: Set("Synthetic shell-free scheduler acceptance".into()),
        starts_at: Set(now - chrono::Duration::hours(2)),
        expires_at: Set(now - chrono::Duration::hours(1)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&fixture.admin)
    .await?;
    Ok(session.id)
}

async fn wait_for_expiry(fixture: &Fixture, id: i64, container: &str) -> TestResult {
    for _ in 0..100 {
        if support_access_sessions::Entity::find_by_id(id)
            .one(&fixture.admin)
            .await?
            .is_some_and(|row| row.expired_at.is_some())
        {
            return Ok(());
        }
        let logs = docker(&["logs", container])?;
        if logs.contains("failed to execute scheduler job") {
            return Err(format!("Native schedule failed: {logs}").into());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Err(format!(
        "Scheduled expiry did not persist: {}",
        docker(&["logs", container])?
    )
    .into())
}

#[tokio::test]
#[ignore = "requires a built scratch image and an owned Docker network; task test:scheduler:image"]
async fn actual_schedule_expires_support_and_audits_once_without_a_shell() -> TestResult {
    let image = std::env::var("LOCO_IMAGE")?;
    let network = std::env::var("MEDTRACKER_IMAGE_NETWORK")?;
    let fixture = Fixture::new().await;
    let result = async {
        let now = chrono::Utc::now().naive_utc();
        let admin = platform_admins::ActiveModel {
            account_id: Set(fixture.account.id), status: Set("active".into()), created_at: Set(now), updated_at: Set(now), ..Default::default()
        }.insert(&fixture.admin).await?;
        let first = seed_session(&fixture, admin.id).await?;
        let directory = tempfile::tempdir()?;
        let config = directory.path().join("scheduler.yaml");
        std::fs::write(&config, "jobs:\n  expiry:\n    run: support-access-expire\n    schedule: '*/1 * * * * *'\n    run_on_start: true\n")?;
        let mut uri = url::Url::parse(&fixture.runtime_uri)?;
        uri.set_host(Some("postgres"))?;
        uri.set_port(Some(5432)).map_err(|()| "set image database port")?;
        let name = format!("scheduler-acceptance-{}", uuid::Uuid::new_v4().simple());
        let guard = Container(name.clone());
        docker(&[
            "run", "--detach", "--name", &name, "--network", &network,
            "--read-only", "--cap-drop=ALL", "--security-opt=no-new-privileges=true",
            "--tmpfs", "/tmp:uid=65532,gid=65532", "--mount",
            &format!("type=bind,src={},dst=/app/scheduler.yaml,readonly", config.display()),
            "--env", &format!("DATABASE_URL={uri}"),
            "--env", "MEDTRACKER_SESSION_KEY=BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBw==",
            "--env", "AUTH_SESSION_SECRET=synthetic-scheduler-image-secret-at-least-32-characters",
            &image, "scheduler", "--environment", "production", "--config", "/app/scheduler.yaml",
        ])?;
        assert_eq!(docker(&["inspect", "--format", "{{.Config.User}}", &name])?.trim(), "65532:65532");
        let shell = Command::new("docker").args(["exec", &name, "/bin/sh", "-c", "exit 0"]).output()?;
        assert!(!shell.status.success(), "Acceptance image must have no /bin/sh");
        let shell_output = format!(
            "{}{}",
            String::from_utf8_lossy(&shell.stdout),
            String::from_utf8_lossy(&shell.stderr)
        );
        assert!(
            shell_output.contains("no such file"),
            "Unexpected shell execution failure: {shell_output}; scheduler logs: {}",
            docker(&["logs", &name])?
        );
        wait_for_expiry(&fixture, first, &name).await?;
        let second = seed_session(&fixture, admin.id).await?;
        wait_for_expiry(&fixture, second, &name).await?;
        tokio::time::sleep(Duration::from_secs(2)).await;
        let versions = versions::Entity::find()
            .filter(versions::Column::HouseholdId.eq(fixture.household.id))
            .filter(versions::Column::ItemType.eq("SupportAccessSession"))
            .filter(versions::Column::Event.eq("platform/support/expired"))
            .all(&fixture.admin).await?;
        let mut subjects: Vec<_> = versions.iter().map(|row| row.item_id).collect();
        subjects.sort_unstable();
        assert_eq!(subjects, [first, second]);
        for row in versions {
            assert_eq!(row.household_id, Some(fixture.household.id));
            assert_eq!(row.whodunnit, None);
            assert_eq!(row.audit_context["actor_account_id"], fixture.account.id);
            assert_eq!(row.audit_context["active_role"], "support_expiry_task");
        }
        let audits = security_audit_events::Entity::find()
            .filter(security_audit_events::Column::HouseholdId.eq(fixture.household.id))
            .filter(security_audit_events::Column::EventType.eq("support_access_session.expired"))
            .all(&fixture.admin).await?;
        let mut subjects: Vec<_> = audits.iter().map(|row| row.metadata["support_access_session_id"].as_i64()).collect();
        subjects.sort_unstable();
        assert_eq!(subjects, [Some(first), Some(second)]);
        for row in audits {
            assert_eq!(row.household_id, fixture.household.id);
            assert_eq!(row.actor_account_id, Some(fixture.account.id));
            assert_eq!(row.actor_membership_id, None);
            assert_eq!(row.audit_context["actor_account_id"], fixture.account.id);
            assert_eq!(row.audit_context["active_role"], "support_expiry_task");
        }
        docker(&["kill", "--signal=SIGINT", &name])?;
        assert_eq!(docker(&["wait", &name])?.trim(), "0");
        drop(guard);
        Ok::<_, Box<dyn std::error::Error>>(())
    }.await;
    fixture.close().await;
    result
}
