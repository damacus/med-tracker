use super::*;
use crate::models::{
    _entities::household_exports as exports,
    access::{self, Actor, HouseholdScope},
};
use crate::views::portable::{ExportRead, ExportStatus};
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use chrono::Duration;
use loco_rs::{app::AppContext, bgworker::BackgroundWorker};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel, QueryFilter,
    QuerySelect, Set,
    sea_query::{Alias, Expr, Func, Query},
};
use serde::Serialize;

#[derive(Clone, Deserialize, Serialize)]
pub struct ExportJob {
    id: i64,
    household_id: i64,
    account_id: i64,
}

impl ExportJob {
    fn associated_data(&self) -> String {
        format!(
            "medtracker.export.v1:{}:{}:{}",
            self.household_id, self.account_id, self.id
        )
    }
}

pub struct ExportWorker(AppContext);

fn cipher(ctx: &AppContext) -> Result<Aes256Gcm, OperationError> {
    let key = ctx
        .config
        .settings
        .as_ref()
        .and_then(|settings| settings["portable_export_wrapping_key"].as_str())
        .and_then(|key| hex::decode(key).ok())
        .ok_or_else(|| OperationError::unavailable("configuration"))?;
    Aes256Gcm::new_from_slice(&key).map_err(|_| OperationError::unavailable("configuration"))
}

fn limit(ctx: &AppContext) -> Result<i32, OperationError> {
    let value = ctx
        .config
        .settings
        .as_ref()
        .and_then(|settings| settings.get("portable_export_concurrency"));
    match value {
        None | Some(Value::Null) => Ok(1),
        Some(value) => value
            .as_i64()
            .and_then(|n| i32::try_from(n).ok())
            .filter(|n| (1..=32).contains(n))
            .ok_or_else(|| OperationError::unavailable("configuration")),
    }
}

pub async fn submit(
    ctx: &AppContext,
    tenant: &TenantTransaction,
    query: ExportQuery,
    passphrase: &str,
    migration_bundle: bool,
) -> Result<i64, OperationError> {
    access::recheck(tenant).await?;
    export_format(&query)?;
    limit(ctx)?;
    let cipher = cipher(ctx)?;
    let now = Utc::now().naive_utc();
    let transaction = tenant.transaction();
    use sea_orm::TransactionTrait;
    let savepoint = transaction.begin().await?;
    let row = exports::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        requested_by_account_id: Set(tenant.scope().actor.account_id),
        status: Set("requested".into()),
        manifest: Set(json!({})),
        requested_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        expires_at: Set(Some(now + Duration::hours(24))),
        ..Default::default()
    }
    .insert(&savepoint)
    .await?;
    let job = ExportJob {
        id: row.id,
        household_id: row.household_id,
        account_id: row.requested_by_account_id,
    };
    let mut nonce = [0; 12];
    getrandom::fill(&mut nonce).map_err(|_| OperationError::unavailable("randomness"))?;
    let nonce_value = Nonce::from(nonce);
    let wrapped = cipher
        .encrypt(
            &nonce_value,
            Payload {
                msg: passphrase.as_bytes(),
                aad: job.associated_data().as_bytes(),
            },
        )
        .map_err(|_| OperationError::unavailable("encryption"))?;
    let mut row = row.into_active_model();
    row.manifest = Set(
        json!({"nonce": hex::encode(nonce), "wrapped": hex::encode(wrapped), "query": query, "migration_bundle": migration_bundle}),
    );
    row.update(&savepoint).await?;
    let args = serde_json::to_value(&job).map_err(|_| OperationError::Unavailable)?;
    let insert = Query::insert()
        .into_table(Alias::new("pg_loco_queue"))
        .columns(["id", "task_data", "name", "run_at", "priority"].map(Alias::new))
        .values_panic([
            ulid::Ulid::generate().to_string().into(),
            args.into(),
            ExportWorker::class_name().into(),
            now.into(),
            0.into(),
        ])
        .to_owned();
    savepoint.execute(&insert).await?;
    savepoint.commit().await?;
    Ok(job.id)
}

pub(crate) async fn read(
    tenant: &TenantTransaction,
    id: i64,
    download: bool,
) -> Result<ExportRead, OperationError> {
    if download {
        sync::lock(tenant).await?;
    }
    access::recheck(tenant).await?;
    let query = exports::Entity::find_by_id(id)
        .filter(exports::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(exports::Column::RequestedByAccountId.eq(tenant.scope().actor.account_id));
    if !download {
        let (status, expires_at) = query
            .select_only()
            .columns([exports::Column::Status, exports::Column::ExpiresAt])
            .into_tuple::<(String, Option<chrono::NaiveDateTime>)>()
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::NotFound)?;
        let status = if expires_at.is_some_and(|until| until <= Utc::now().naive_utc()) {
            "expired".to_owned()
        } else {
            status
        };
        return Ok(ExportRead::Status(ExportStatus {
            id: id.to_string(),
            status,
        }));
    }
    let row = query
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    let expired = row
        .expires_at
        .is_some_and(|until| until <= Utc::now().naive_utc());
    if expired || row.status != "ready" || row.manifest["scope"] != export_scope(tenant).await? {
        return Err(OperationError::NotFound);
    }
    row.manifest
        .get("artifact")
        .cloned()
        .map(ExportRead::Artifact)
        .ok_or(OperationError::Unavailable)
}

async fn export_scope(tenant: &TenantTransaction) -> Result<Value, OperationError> {
    let people = sync::export_people(tenant)
        .await?
        .into_iter()
        .map(|person| person.id)
        .collect::<Vec<_>>();
    Ok(
        json!({"people": people, "membership": tenant.membership().id,
        "permissions_version": tenant.membership().permissions_version}),
    )
}

#[async_trait::async_trait]
impl BackgroundWorker<ExportJob> for ExportWorker {
    fn build(ctx: &AppContext) -> Self {
        Self(ctx.clone())
    }

    fn max_retries() -> u32 {
        3
    }

    async fn perform(&self, job: ExportJob) -> loco_rs::Result<()> {
        run(&self.0, &job)
            .await
            .map_err(|_| loco_rs::Error::string("Portable export job failed"))
    }
}

async fn run(ctx: &AppContext, job: &ExportJob) -> Result<(), OperationError> {
    let concurrency = limit(ctx)?;
    let scope = HouseholdScope {
        actor: Actor {
            account_id: job.account_id,
        },
        household_id: job.household_id,
        request_id: uuid::Uuid::new_v4().to_string(),
    };
    loop {
        let tenant = match access::begin(&ctx.db, &scope, ctx.config.settings.as_ref()).await {
            Ok(tenant) => tenant,
            Err(
                OperationError::Forbidden
                | OperationError::NotFound
                | OperationError::Unauthenticated,
            ) => return Ok(()),
            Err(error) => return Err(error),
        };
        let db = tenant.transaction();
        let mut acquired = false;
        for slot in 0..concurrency {
            let query = Query::select()
                .expr_as(
                    Func::cust(Alias::new("pg_try_advisory_xact_lock"))
                        .args([Expr::val(0x4d54_4558_i32), Expr::val(slot)]),
                    Alias::new("acquired"),
                )
                .to_owned();
            let result = db
                .query_one(&query)
                .await?
                .ok_or(OperationError::Unavailable)?;
            if result.try_get::<bool>("", "acquired")? {
                acquired = true;
                break;
            }
        }
        if !acquired {
            tenant.rollback().await?;
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            continue;
        }
        super::sync::lock(&tenant).await?;
        access::recheck(&tenant).await?;
        let Some(row) = exports::Entity::find_by_id(job.id)
            .filter(exports::Column::HouseholdId.eq(job.household_id))
            .filter(exports::Column::RequestedByAccountId.eq(job.account_id))
            .lock_exclusive()
            .one(db)
            .await?
        else {
            return tenant.rollback().await;
        };
        if row.status != "requested" {
            return tenant.rollback().await;
        }
        let now = Utc::now().naive_utc();
        let authority = export_scope(&tenant).await?;
        let outcome = if row.expires_at.is_some_and(|until| until <= now) {
            Err(validation("Portable export expired"))
        } else {
            generate(ctx, &tenant, job, &row.manifest).await
        };
        let mut update = row.into_active_model();
        update.updated_at = Set(Utc::now().naive_utc());
        match outcome {
            Ok(artifact) => {
                update.status = Set("ready".into());
                update.ready_at = Set(Some(Utc::now().naive_utc()));
                update.manifest = Set(json!({"artifact": artifact, "scope": authority}));
            }
            Err(OperationError::Validation { .. }) => {
                update.status = Set("failed".into());
                update.failed_at = Set(Some(Utc::now().naive_utc()));
                update.failure_code = Set(Some("export_invalid".into()));
                update.manifest = Set(json!({}));
            }
            Err(error) => return Err(error),
        }
        update.update(db).await?;
        return tenant.commit().await;
    }
}

async fn generate(
    ctx: &AppContext,
    tenant: &TenantTransaction,
    job: &ExportJob,
    manifest: &Value,
) -> Result<Value, OperationError> {
    let read_hex = |name: &str| {
        manifest[name]
            .as_str()
            .and_then(|value| hex::decode(value).ok())
            .ok_or(OperationError::Unavailable)
    };
    let nonce: [u8; 12] = read_hex("nonce")?
        .try_into()
        .map_err(|_| OperationError::Unavailable)?;
    let bytes = cipher(ctx)?
        .decrypt(
            &Nonce::from(nonce),
            Payload {
                msg: &read_hex("wrapped")?,
                aad: job.associated_data().as_bytes(),
            },
        )
        .map_err(|_| OperationError::unavailable("decryption"))?;
    let passphrase = String::from_utf8(bytes).map_err(|_| OperationError::Unavailable)?;
    if manifest["migration_bundle"] == true {
        encrypted_migration_bundle(tenant, &passphrase).await
    } else {
        let query = serde_json::from_value(manifest["query"].clone())
            .map_err(|_| OperationError::Unavailable)?;
        portable_export(tenant, &query, Some(&passphrase)).await
    }
}
