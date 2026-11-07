use std::sync::Arc;

use chrono::{Duration, Utc};
use loco_rs::app::AppContext;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tokio::sync::Semaphore;

use super::contracts::{
    ArchivePreparation, ArchivePreparationError, PreparedArchive, SourceInventory, SourceSystem,
    SourceTimeZone,
};
use super::{consents, credential::ImportPrincipal};
use crate::models::{
    _entities::data_import_inspections,
    access::{self, HouseholdScope, TenantTransaction},
    clinical_transfer,
    errors::OperationError,
};

const MAX_ARCHIVE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone)]
pub struct CryptoLimiter(Arc<Semaphore>);

impl CryptoLimiter {
    pub fn new() -> Self {
        Self(Arc::new(Semaphore::new(1)))
    }
}

impl Default for CryptoLimiter {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Upload {
    pub source: SourceSystem,
    pub archive: Vec<u8>,
    pub passphrase: String,
}

#[derive(Debug)]
pub enum InspectionError {
    Preparation(ArchivePreparationError),
    Operation(OperationError),
}

impl From<OperationError> for InspectionError {
    fn from(error: OperationError) -> Self {
        Self::Operation(error)
    }
}

pub struct Inspection {
    pub id: String,
    pub archive_sha256: String,
    pub timezone: SourceTimeZone,
    pub inventory: SourceInventory,
}

pub fn confirmed_timezone(name: &str, confirmed: bool) -> Result<SourceTimeZone, OperationError> {
    if !confirmed {
        return Err(OperationError::Validation {
            details: serde_json::json!({"errors":{"source_timezone":["must be confirmed"]}}),
        });
    }
    SourceTimeZone::parse(name).map_err(|_| OperationError::Validation {
        details: serde_json::json!({"errors":{"source_timezone":["must be a valid time zone"]}}),
    })
}

pub async fn prepare_upload(
    ctx: &AppContext,
    upload: Upload,
) -> Result<PreparedArchive, InspectionError> {
    if upload.archive.len() > MAX_ARCHIVE_BYTES {
        return Err(InspectionError::Preparation(
            ArchivePreparationError::TooLarge,
        ));
    }
    let limiter = ctx
        .shared_store
        .get::<CryptoLimiter>()
        .ok_or_else(|| OperationError::unavailable("import_crypto_limiter"))?;
    let permit = limiter
        .0
        .try_acquire_owned()
        .map_err(|_| OperationError::unavailable("import_crypto_busy"))?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        clinical_transfer::prepare_archive(ArchivePreparation {
            source: upload.source,
            archive: &upload.archive,
            passphrase: &upload.passphrase,
        })
    })
    .await
    .map_err(|_| InspectionError::Operation(OperationError::unavailable("import_crypto_task")))?
    .map_err(InspectionError::Preparation)
}

pub(crate) async fn prepare_assessment(
    ctx: &AppContext,
    upload: Upload,
) -> Result<(PreparedArchive, serde_json::Value), InspectionError> {
    if upload.archive.len() > MAX_ARCHIVE_BYTES {
        return Err(InspectionError::Preparation(
            ArchivePreparationError::TooLarge,
        ));
    }
    let limiter = ctx
        .shared_store
        .get::<CryptoLimiter>()
        .ok_or_else(|| OperationError::unavailable("import_crypto_limiter"))?;
    let permit = limiter
        .0
        .try_acquire_owned()
        .map_err(|_| OperationError::unavailable("import_crypto_busy"))?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let prepared = clinical_transfer::prepare_archive(ArchivePreparation {
            source: upload.source,
            archive: &upload.archive,
            passphrase: &upload.passphrase,
        })
        .map_err(InspectionError::Preparation)?;
        let payload = crate::models::care::portable_crypto::decrypt(
            &prepared.transfer.bundle,
            &upload.passphrase,
        )
        .map_err(|_| {
            InspectionError::Operation(OperationError::unavailable("import_prepared_decrypt"))
        })?;
        Ok((prepared, payload))
    })
    .await
    .map_err(|_| InspectionError::Operation(OperationError::unavailable("import_crypto_task")))?
}

pub async fn inspect(
    ctx: &AppContext,
    scope: &HouseholdScope,
    upload: Upload,
    timezone: SourceTimeZone,
) -> Result<Inspection, InspectionError> {
    inspect_inner(ctx, scope, upload, timezone, None).await
}

pub async fn inspect_checked(
    ctx: &AppContext,
    scope: &HouseholdScope,
    upload: Upload,
    timezone: SourceTimeZone,
    principal: ImportPrincipal<'_>,
) -> Result<Inspection, InspectionError> {
    inspect_inner(ctx, scope, upload, timezone, Some(principal)).await
}

async fn inspect_inner(
    ctx: &AppContext,
    scope: &HouseholdScope,
    upload: Upload,
    timezone: SourceTimeZone,
    principal: Option<ImportPrincipal<'_>>,
) -> Result<Inspection, InspectionError> {
    let prepared = prepare_upload(ctx, upload).await?;
    let tenant = if principal.is_some() {
        consents::begin_for_actor(&ctx.db, scope, ctx.config.settings.as_ref()).await?
    } else {
        access::begin(&ctx.db, scope, ctx.config.settings.as_ref()).await?
    };
    if let Some(principal) = principal {
        principal.revalidate(&tenant).await?;
    }
    access::recheck(&tenant).await?;
    if !access::can_manage_household(&tenant) {
        return Err(OperationError::Forbidden.into());
    }
    let id = uuid::Uuid::new_v4().to_string();
    data_import_inspections::ActiveModel {
        id: Set(id.clone()),
        household_id: Set(scope.household_id),
        account_id: Set(scope.actor.account_id),
        membership_id: Set(tenant.membership().id),
        source_sha256: Set(prepared.archive_sha256.clone()),
        source_timezone: Set(timezone.name().to_owned()),
        expires_at: Set((Utc::now() + Duration::minutes(30)).naive_utc()),
    }
    .insert(tenant.transaction())
    .await
    .map_err(OperationError::from)?;
    tenant.commit().await?;
    Ok(Inspection {
        id,
        archive_sha256: prepared.archive_sha256,
        timezone,
        inventory: prepared.inventory,
    })
}

pub async fn verify(
    tenant: &TenantTransaction,
    id: &str,
    archive_sha256: &str,
    timezone: SourceTimeZone,
) -> Result<(), OperationError> {
    access::recheck(tenant).await?;
    if !access::can_manage_household(tenant) {
        return Err(OperationError::Forbidden);
    }
    let found = data_import_inspections::Entity::find_by_id(id.to_owned())
        .filter(data_import_inspections::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(data_import_inspections::Column::AccountId.eq(tenant.scope().actor.account_id))
        .filter(data_import_inspections::Column::MembershipId.eq(tenant.membership().id))
        .filter(data_import_inspections::Column::SourceSha256.eq(archive_sha256))
        .filter(data_import_inspections::Column::SourceTimezone.eq(timezone.name()))
        .filter(data_import_inspections::Column::ExpiresAt.gt(Utc::now().naive_utc()))
        .one(tenant.transaction())
        .await?;
    if found.is_none() {
        return Err(OperationError::Conflict {
            code: "import_inspection_changed".into(),
            details: serde_json::json!({}),
        });
    }
    access::recheck(tenant).await
}
