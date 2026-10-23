use super::settings;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use loco_rs::{
    app::AppContext,
    task::{Task, TaskInfo, Vars},
};
use sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbErr,
    sea_query::{Alias, Expr, ExprTrait, Query},
};
use serde_json::Value;
use std::{sync::Arc, time::Duration};

mod fixtures;
mod smoke;

pub const SCOPE: &str = "medtracker-canary";
const HOST: &str = "medtracker-canary-rw.medtracker-canary.svc.cluster.local";

#[derive(Clone)]
pub(crate) struct Canary {
    pub control: DatabaseConnection,
    pub image: String,
    pub smoke_key: String,
}

pub(crate) fn enabled(ctx: &AppContext) -> bool {
    settings::flag(ctx.config.settings.as_ref(), &["canary", "enabled"]) == Some(true)
}

fn invalid() -> DbErr {
    DbErr::Custom("Synthetic canary configuration or control state is unavailable".into())
}

pub(crate) fn text<'a>(ctx: &'a AppContext, key: &str) -> Result<&'a str, DbErr> {
    settings::text(ctx.config.settings.as_ref(), &["canary", key]).ok_or_else(invalid)
}

fn target(uri: &str, database: &str) -> bool {
    url::Url::parse(uri).is_ok_and(|url| {
        matches!(url.scheme(), "postgres" | "postgresql")
            && url.host_str() == Some(HOST)
            && url.path() == format!("/{database}")
            && url.query_pairs().all(|(key, _)| {
                matches!(key.as_ref(), "sslmode" | "sslrootcert" | "application_name")
            })
    })
}

fn validate(ctx: &AppContext) -> Result<(), DbErr> {
    let settings = ctx.config.settings.as_ref();
    let control = text(ctx, "control_database_url")?;
    if text(ctx, "scope")? != SCOPE
        || !target(control, "medtracker_canary_control")
        || !(target(&ctx.config.database.uri, "medtracker_canary")
            || target(&ctx.config.database.uri, "medtracker_canary_control"))
        || text(ctx, "image")?
            .split_once("@sha256:")
            .is_none_or(|(_, digest)| {
                digest.len() != 64 || !digest.bytes().all(|c| c.is_ascii_hexdigit())
            })
        || text(ctx, "smoke_key")?.len() < 32
        || matches!(&ctx.config.queue, Some(loco_rs::config::QueueConfig::Postgres(queue)) if queue.reaper.is_some())
        || ctx
            .config
            .mailer
            .as_ref()
            .is_some_and(|mailer| !mailer.stub)
        || settings::flag(settings, &["ai_medication_help", "enabled"]) == Some(true)
        || ["browser_push", "avatar_storage", "dmd_storage", "zitadel"]
            .iter()
            .any(|section| {
                settings
                    .and_then(|s| s.get(section))
                    .and_then(Value::as_object)
                    .is_some_and(|values| {
                        values.iter().any(|(key, value)| {
                            key != "force_path_style"
                                && key != "region"
                                && value.as_str().is_some_and(|s| !s.trim().is_empty())
                        })
                    })
            })
    {
        return Err(invalid());
    }
    Ok(())
}

pub(crate) async fn configure(ctx: &AppContext) -> Result<(), DbErr> {
    if !enabled(ctx) {
        return Ok(());
    }
    validate(ctx)?;
    let control = text(ctx, "control_database_url")?;
    let mut options = sea_orm::ConnectOptions::new(control);
    options
        .max_connections(2)
        .min_connections(0)
        .connect_timeout(Duration::from_secs(3))
        .sqlx_logging(false);
    ctx.shared_store.insert(Arc::new(Canary {
        control: Database::connect(options).await?,
        image: text(ctx, "image")?.into(),
        smoke_key: text(ctx, "smoke_key")?.into(),
    }));
    Ok(())
}

fn phase_allows(phase: &str, heartbeat: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    phase == "ready"
        || (phase == "verifying"
            && heartbeat <= now
            && now - heartbeat <= chrono::Duration::seconds(30))
}

#[derive(serde::Serialize)]
struct PublicStatus {
    phase: String,
    last_success: Option<DateTime<Utc>>,
    next_reset: Option<DateTime<Utc>>,
}

impl Canary {
    async fn status(&self) -> Result<PublicStatus, DbErr> {
        let query = Query::select()
            .columns([
                Alias::new("phase"),
                Alias::new("updated_at"),
                Alias::new("image"),
                Alias::new("last_success"),
                Alias::new("next_reset"),
            ])
            .from(Alias::new("canary_control"))
            .and_where(Expr::col(Alias::new("scope")).eq(SCOPE))
            .to_owned();
        let row = self.control.query_one(&query).await?.ok_or_else(invalid)?;
        let phase: String = row.try_get("", "phase")?;
        let heartbeat = row.try_get("", "updated_at")?;
        let image: String = row.try_get("", "image")?;
        if image != self.image || !phase_allows(&phase, heartbeat, Utc::now()) {
            return Err(invalid());
        }
        Ok(PublicStatus {
            phase,
            last_success: row.try_get("", "last_success")?,
            next_reset: row.try_get("", "next_reset")?,
        })
    }
}

pub(crate) async fn work(ctx: &AppContext) -> Result<(), DbErr> {
    if !enabled(ctx) {
        return Ok(());
    }
    let state = ctx.shared_store.get::<Arc<Canary>>().ok_or_else(invalid)?;
    loop {
        if state.status().await.is_ok() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

pub(crate) async fn boundary(
    State(ctx): State<AppContext>,
    request: Request,
    next: Next,
) -> Response {
    if !enabled(&ctx) {
        return next.run(request).await;
    }
    let Some(state) = ctx.shared_store.get::<Arc<Canary>>() else {
        return unavailable();
    };
    let Ok(status) = state.status().await else {
        return unavailable();
    };
    if status.phase == "verifying" {
        use hmac::{Hmac, KeyInit, Mac};
        use sha2::Sha256;
        let provided = request
            .headers()
            .get("x-medtracker-canary-smoke")
            .and_then(|s| s.to_str().ok())
            .unwrap_or_default();
        let Ok(mut expected) =
            <Hmac<Sha256> as KeyInit>::new_from_slice(state.smoke_key.as_bytes())
        else {
            return unavailable();
        };
        expected.update(state.smoke_key.as_bytes());
        let tag = expected.finalize().into_bytes();
        let Ok(mut actual) = <Hmac<Sha256> as KeyInit>::new_from_slice(state.smoke_key.as_bytes())
        else {
            return unavailable();
        };
        actual.update(provided.as_bytes());
        if actual.verify_slice(&tag).is_err() {
            return unavailable();
        }
    }
    if request.uri().path() == "/canary/status" && request.method() == axum::http::Method::GET {
        return ([("cache-control", "no-store")], axum::Json(status)).into_response();
    }
    next.run(request).await
}

fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        [("cache-control", "no-store"), ("retry-after", "30")],
        "Synthetic canary is being prepared",
    )
        .into_response()
}

pub struct CapabilitiesTask;
#[async_trait::async_trait]
impl Task for CapabilitiesTask {
    fn task(&self) -> TaskInfo {
        TaskInfo { name: "canary-capabilities".into(), detail: "Validate synthetic canary seed, gate and no-send configuration without application writes".into() }
    }
    async fn run(&self, ctx: &AppContext, _: &Vars) -> loco_rs::Result<()> {
        if !enabled(ctx) || ctx.shared_store.get::<Arc<Canary>>().is_none() {
            return Err(loco_rs::Error::string("Canary is not configured"));
        }
        let state = ctx
            .shared_store
            .get::<Arc<Canary>>()
            .ok_or_else(|| loco_rs::Error::string("Canary state unavailable"))?;
        let query = Query::select()
            .column(Alias::new("scope"))
            .from(Alias::new("canary_control"))
            .and_where(Expr::col(Alias::new("scope")).eq(SCOPE))
            .to_owned();
        if state.control.query_one(&query).await?.is_none() {
            return Err(loco_rs::Error::string("Canary control scope unavailable"));
        }
        Ok(())
    }
}

pub struct SeedTask;
#[async_trait::async_trait]
impl Task for SeedTask {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "canary-seed".into(),
            detail: "Transactionally initialise the dedicated synthetic canary".into(),
        }
    }
    async fn run(&self, ctx: &AppContext, _: &Vars) -> loco_rs::Result<()> {
        fixtures::seed(ctx)
            .await
            .map_err(|_| loco_rs::Error::string("Synthetic canary seed refused or failed"))
    }
}

pub struct SmokeTask;
#[async_trait::async_trait]
impl Task for SmokeTask {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "canary-smoke".into(),
            detail: "Sign in and inspect synthetic care through the internal running service"
                .into(),
        }
    }
    async fn run(&self, ctx: &AppContext, _: &Vars) -> loco_rs::Result<()> {
        smoke::run(ctx)
            .await
            .map_err(|_| loco_rs::Error::string("Synthetic canary smoke failed"))
    }
}

pub(crate) struct MailerWorker {
    ctx: AppContext,
}
#[async_trait::async_trait]
impl loco_rs::bgworker::BackgroundWorker<loco_rs::mailer::Email> for MailerWorker {
    fn build(ctx: &AppContext) -> Self {
        Self { ctx: ctx.clone() }
    }
    fn class_name() -> String {
        "MailerWorker".into()
    }
    fn queue() -> Option<String> {
        Some("mailer".into())
    }
    async fn perform(&self, email: loco_rs::mailer::Email) -> loco_rs::Result<()> {
        work(&self.ctx).await?;
        loco_rs::mailer::MailerWorker::build(&self.ctx)
            .perform(email)
            .await
    }
}

#[cfg(test)]
mod acceptance;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_dedicated_target_and_non_redirecting_database_options_are_accepted() {
        assert!(target(
            &format!("postgres://user:password@{HOST}/medtracker_canary?sslmode=verify-full"),
            "medtracker_canary"
        ));
        for uri in [
            format!("postgres://user:password@{HOST}/medtracker"),
            "postgres://user:password@localhost/medtracker_canary".to_string(),
            format!("postgres://user:password@{HOST}/medtracker_canary?host=production"),
        ] {
            assert!(!target(&uri, "medtracker_canary"));
        }
    }
    #[test]
    fn verifying_requires_current_image_and_fresh_heartbeat() {
        let now = chrono::Utc::now();
        assert!(phase_allows("ready", now - chrono::Duration::days(2), now));
        assert!(phase_allows(
            "verifying",
            now - chrono::Duration::seconds(30),
            now
        ));
        assert!(!phase_allows(
            "verifying",
            now - chrono::Duration::seconds(31),
            now
        ));
        assert!(!phase_allows("failed", now, now));
        assert!(!phase_allows(
            "verifying",
            now + chrono::Duration::seconds(1),
            now
        ));
    }
}
