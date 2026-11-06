use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, QuerySelect,
};
use serde::{Serialize, de::DeserializeOwned};

pub use super::contracts::SourceTimeZone;
use super::contracts::{
    ConsentInput, ImportReport, ImportState, PersonMapping, PreparationReceipt, PreviewToken,
};
use crate::models::{
    _entities::data_import_runs, access::TenantTransaction, care::portable_crypto,
    errors::OperationError,
};

#[derive(Clone, PartialEq)]
pub struct EncryptedPrepared(Vec<u8>);

impl EncryptedPrepared {
    pub fn encrypt(
        prepared: &serde_json::Value,
        passphrase: &str,
    ) -> Result<Self, portable_crypto::PortableCryptoError> {
        let envelope = portable_crypto::encrypt(prepared, passphrase)?;
        let bytes = serde_json::to_vec(&envelope)
            .map_err(|_| portable_crypto::PortableCryptoError::InvalidEnvelope)?;
        Ok(Self(bytes))
    }

    pub fn decrypt(
        &self,
        passphrase: &str,
    ) -> Result<serde_json::Value, portable_crypto::PortableCryptoError> {
        let envelope = serde_json::from_slice(&self.0)
            .map_err(|_| portable_crypto::PortableCryptoError::InvalidEnvelope)?;
        portable_crypto::decrypt(&envelope, passphrase)
    }
}

pub struct NewRun {
    pub report: ImportReport,
    pub receipt: PreparationReceipt,
    pub encrypted_prepared: EncryptedPrepared,
    pub source_timezone: SourceTimeZone,
    pub mappings: Vec<PersonMapping>,
    pub consents: Vec<ConsentInput>,
    pub acknowledged_omissions: Vec<String>,
    pub confirmation: Option<PreviewToken>,
}

pub struct StoredRun {
    pub created_by_membership_id: i64,
    pub snapshot: NewRun,
}

fn document<T: Serialize>(value: &T) -> Result<serde_json::Value, OperationError> {
    serde_json::to_value(value).map_err(|_| OperationError::unavailable("import_document_encode"))
}

fn read_document<T: DeserializeOwned>(value: serde_json::Value) -> Result<T, OperationError> {
    serde_json::from_value(value).map_err(|_| OperationError::unavailable("import_document_decode"))
}

pub async fn create(tenant: &TenantTransaction, run: &NewRun) -> Result<(), OperationError> {
    if run.report.source_timezone != run.source_timezone
        || !matches!(
            run.report.state,
            ImportState::Preview | ImportState::Blocked | ImportState::Ready
        )
        || (run.report.state == ImportState::Ready) != run.confirmation.is_some()
    {
        return Err(OperationError::unavailable("import_snapshot_invalid"));
    }
    data_import_runs::ActiveModel {
        id: Set(run.report.id.clone()),
        household_id: Set(tenant.scope().household_id),
        created_by_membership_id: Set(tenant.membership().id),
        source_timezone: Set(run.source_timezone.name().to_owned()),
        encrypted_prepared: Set(run.encrypted_prepared.0.clone()),
        report: Set(document(&run.report)?),
        receipt: Set(document(&run.receipt)?),
        mappings: Set(document(&run.mappings)?),
        consents: Set(document(&run.consents)?),
        acknowledged_omissions: Set(document(&run.acknowledged_omissions)?),
        confirmation: Set(run.confirmation.as_ref().map(|token| token.0.clone())),
    }
    .insert(tenant.transaction())
    .await?;
    Ok(())
}

pub async fn find(
    tenant: &TenantTransaction,
    id: &str,
) -> Result<Option<StoredRun>, OperationError> {
    let Some(row) = data_import_runs::Entity::find_by_id(id.to_owned())
        .one(tenant.transaction())
        .await?
    else {
        return Ok(None);
    };
    let source_timezone = SourceTimeZone::parse(&row.source_timezone)
        .map_err(|_| OperationError::unavailable("import_timezone_decode"))?;
    let report: ImportReport = read_document(row.report)?;
    if report.source_timezone != source_timezone {
        return Err(OperationError::unavailable("import_timezone_mismatch"));
    }
    Ok(Some(StoredRun {
        created_by_membership_id: row.created_by_membership_id,
        snapshot: NewRun {
            report,
            receipt: read_document(row.receipt)?,
            encrypted_prepared: EncryptedPrepared(row.encrypted_prepared),
            source_timezone,
            mappings: read_document(row.mappings)?,
            consents: read_document(row.consents)?,
            acknowledged_omissions: read_document(row.acknowledged_omissions)?,
            confirmation: row.confirmation.map(PreviewToken),
        },
    }))
}

// Stage 42 must add bounded pagination before exposing report listing through an API.
pub async fn list_reports(tenant: &TenantTransaction) -> Result<Vec<ImportReport>, OperationError> {
    let documents: Vec<(serde_json::Value,)> = data_import_runs::Entity::find()
        .select_only()
        .column(data_import_runs::Column::Report)
        .filter(data_import_runs::Column::HouseholdId.eq(tenant.scope().household_id))
        .order_by_asc(data_import_runs::Column::Id)
        .into_tuple()
        .all(tenant.transaction())
        .await?;
    documents
        .into_iter()
        .map(|(document,)| read_document(document))
        .collect()
}

pub async fn transition(
    tenant: &TenantTransaction,
    expected: ImportState,
    report: &ImportReport,
    confirmation: Option<&PreviewToken>,
) -> Result<(), OperationError> {
    let Some(row) = data_import_runs::Entity::find_by_id(report.id.clone())
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
    else {
        return Err(OperationError::NotFound);
    };
    let previous: ImportReport = read_document(row.report.clone())?;
    let mut unchanged_preview = previous.clone();
    unchanged_preview.state = report.state;
    let permitted = matches!(
        (expected, report.state),
        (ImportState::Ready, ImportState::Applying)
            | (
                ImportState::Applying,
                ImportState::Completed | ImportState::Failed
            )
            | (ImportState::Ready, ImportState::Blocked)
    );
    if previous.state != expected
        || !permitted
        || previous.source != report.source
        || previous.source_sha256 != report.source_sha256
        || previous.source_timezone != report.source_timezone
        || previous.created_at != report.created_at
        || (report.state == ImportState::Applying
            && (unchanged_preview != *report
                || row.confirmation.as_deref() != confirmation.map(|token| token.0.as_str())
                || row.confirmation.is_none()))
    {
        return Err(OperationError::Conflict {
            code: "import_snapshot_changed".into(),
            details: serde_json::json!({}),
        });
    }
    let mut update = row.into_active_model();
    update.report = Set(document(report)?);
    update.confirmation = Set(None);
    update.update(tenant.transaction()).await?;
    Ok(())
}
