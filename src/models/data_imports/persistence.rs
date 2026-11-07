use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, QuerySelect,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub use super::contracts::SourceTimeZone;
use super::contracts::{
    ConsentInput, ImportReport, ImportState, PersonMapping, PreparationReceipt, PreviewToken,
    SourceInventory,
};
use super::identity::RecipientConsent;
use crate::models::{
    _entities::data_import_runs,
    access::TenantTransaction,
    care::{portable_crypto, portable_imports::planning::DestinationExpectation},
    errors::OperationError,
};

#[derive(Clone, PartialEq)]
pub struct PreviewContext(PreviewContextDocument);

#[derive(Clone, Deserialize, PartialEq, Serialize)]
struct PreviewContextDocument {
    expectation: Option<DestinationExpectation>,
    recipient_consents: Vec<RecipientConsent>,
    #[serde(default)]
    inventory: Option<SourceInventory>,
    #[serde(default)]
    token_verifier: Option<String>,
}

impl PreviewContext {
    pub(crate) fn new(
        expectation: Option<DestinationExpectation>,
        recipient_consents: Vec<RecipientConsent>,
        inventory: Option<SourceInventory>,
        token_verifier: Option<String>,
    ) -> Self {
        Self(PreviewContextDocument {
            expectation,
            recipient_consents,
            inventory,
            token_verifier,
        })
    }

    pub(crate) fn expectation(&self) -> Option<&DestinationExpectation> {
        self.0.expectation.as_ref()
    }

    pub(crate) fn recipient_consents(&self) -> &[RecipientConsent] {
        &self.0.recipient_consents
    }

    pub(crate) fn inventory(&self) -> Option<&SourceInventory> {
        self.0.inventory.as_ref()
    }

    pub(crate) fn token_verifier(&self) -> Option<&str> {
        self.0.token_verifier.as_deref()
    }
}

#[derive(Deserialize, Serialize)]
struct ReceiptEnvelope {
    receipt: PreparationReceipt,
    context: Option<PreviewContextDocument>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StoredReceipt {
    Envelope(ReceiptEnvelope),
    Legacy(PreparationReceipt),
}

#[derive(Clone, PartialEq)]
pub struct EncryptedPrepared(Vec<u8>);

impl EncryptedPrepared {
    pub fn from_prepared(
        prepared: &crate::models::clinical_transfer::PreparedTransfer,
    ) -> Result<Self, portable_crypto::PortableCryptoError> {
        serde_json::to_vec(&prepared.bundle)
            .map(Self)
            .map_err(|_| portable_crypto::PortableCryptoError::InvalidEnvelope)
    }

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
    pub context: Option<PreviewContext>,
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
        receipt: Set(document(&ReceiptEnvelope {
            receipt: run.receipt.clone(),
            context: run.context.as_ref().map(|context| PreviewContextDocument {
                expectation: context.expectation().cloned(),
                recipient_consents: context.recipient_consents().to_vec(),
                inventory: context.inventory().cloned(),
                token_verifier: context.token_verifier().map(str::to_owned),
            }),
        })?),
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
    let (receipt, context) = match read_document(row.receipt)? {
        StoredReceipt::Envelope(envelope) => {
            (envelope.receipt, envelope.context.map(PreviewContext))
        }
        StoredReceipt::Legacy(receipt) => (receipt, None),
    };
    Ok(Some(StoredRun {
        created_by_membership_id: row.created_by_membership_id,
        snapshot: NewRun {
            report,
            receipt,
            context,
            encrypted_prepared: EncryptedPrepared(row.encrypted_prepared),
            source_timezone,
            mappings: read_document(row.mappings)?,
            consents: read_document(row.consents)?,
            acknowledged_omissions: read_document(row.acknowledged_omissions)?,
            confirmation: row.confirmation.map(PreviewToken),
        },
    }))
}

pub async fn list_reports(
    tenant: &TenantTransaction,
    after_id: Option<&str>,
    limit: u64,
) -> Result<Vec<ImportReport>, OperationError> {
    if !(1..=100).contains(&limit) {
        return Err(OperationError::Validation {
            details: serde_json::json!({"errors": {"limit": ["must be between 1 and 100"]}}),
        });
    }
    let mut query = data_import_runs::Entity::find()
        .select_only()
        .column(data_import_runs::Column::Report)
        .filter(data_import_runs::Column::HouseholdId.eq(tenant.scope().household_id))
        .order_by_asc(data_import_runs::Column::Id)
        .limit(limit);
    if let Some(id) = after_id {
        query = query.filter(data_import_runs::Column::Id.gt(id.to_owned()));
    }
    let documents: Vec<(serde_json::Value,)> = query.into_tuple().all(tenant.transaction()).await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        care::portable_imports::planning::{NameExpectation, SlotExpectation, SourceCollection},
        data_imports::contracts::{RecordKind, SourceIdentity, SourceSystem},
        entities::membership,
        ids::{AccountId, HouseholdId},
    };

    #[test]
    fn receipt_document_roundtrips_private_context_and_accepts_legacy_receipts() {
        let receipt = PreparationReceipt {
            payload_sha256: "payload".into(),
            manifest_sha256: "manifest".into(),
            target_sha256: "target".into(),
        };
        let expectation = DestinationExpectation {
            household_id: HouseholdId::from_database(1),
            actor_id: AccountId::from_database(2),
            actor_membership: membership::Model {
                id: 3,
                created_at: Default::default(),
                household_id: 1,
                joined_at: Default::default(),
                permissions_version: 0,
                person_id: 4,
                revoked_at: None,
                role: "owner".into(),
                status: "active".into(),
                updated_at: Default::default(),
            },
            aliases: vec![],
            slots: vec![SlotExpectation {
                collection: SourceCollection::Medications,
                portable_id: "source-medication".into(),
                occupied: false,
                name: Some(NameExpectation {
                    key: "medicine".into(),
                    matching_portable_ids: vec!["existing".into()],
                }),
            }],
        };
        let recipient = RecipientConsent {
            source_person: SourceIdentity {
                source: SourceSystem::MedTracker05,
                instance: "source-instance".into(),
                kind: RecordKind::Person,
                id: "source-person".into(),
            },
            request_id: "request-id".into(),
        };
        let context = PreviewContext::new(
            Some(expectation.clone()),
            vec![recipient.clone()],
            None,
            None,
        );
        let encoded = document(&ReceiptEnvelope {
            receipt: receipt.clone(),
            context: Some(context.0.clone()),
        })
        .unwrap();
        let StoredReceipt::Envelope(decoded) = read_document(encoded).unwrap() else {
            panic!("expected context envelope");
        };
        let reloaded = PreviewContext(decoded.context.unwrap());
        assert_eq!(decoded.receipt, receipt);
        assert_eq!(reloaded.expectation(), Some(&expectation));
        assert!(reloaded.recipient_consents() == [recipient]);

        let legacy = document(&receipt).unwrap();
        let StoredReceipt::Legacy(reloaded_receipt) = read_document(legacy).unwrap() else {
            panic!("expected legacy receipt");
        };
        assert_eq!(reloaded_receipt, receipt);
        let blocked = PreviewContext::new(None, vec![], None, None);
        assert!(blocked.expectation().is_none());
    }

    #[test]
    fn preview_context_retains_inventory_and_accepts_older_documents() {
        let inventory = super::super::contracts::SourceInventory {
            records: vec![],
            relationships: vec![],
            omissions: vec![],
        };
        let context = PreviewContext::new(
            None,
            vec![],
            Some(inventory.clone()),
            Some("verifier".into()),
        );
        let encoded = document(&context.0).unwrap();
        let decoded: PreviewContextDocument = read_document(encoded).unwrap();
        assert_eq!(decoded.inventory, Some(inventory));
        assert_eq!(decoded.token_verifier.as_deref(), Some("verifier"));

        let old = serde_json::json!({"expectation": null, "recipient_consents": []});
        let decoded: PreviewContextDocument = read_document(old).unwrap();
        assert!(decoded.inventory.is_none());
        assert!(decoded.token_verifier.is_none());
    }
}
