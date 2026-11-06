use crate::models::clinical_transfer::{PreparationError, PreparedTransfer};
use chrono::{DateTime, Utc};
use sea_orm::prelude::Decimal;
use serde::{Deserialize, Serialize};

macro_rules! destination_ids {
    ($($name:ident),+ $(,)?) => {$ (
        #[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
        #[serde(transparent)]
        pub struct $name(pub i64);
    )+};
}

destination_ids!(
    PersonId,
    MedicationId,
    LocationId,
    DosageOptionId,
    ScheduleId,
    AssignmentId,
    DoseId,
    StockId,
    PauseId,
    OccurrenceId,
    HealthEventId,
    AttachmentId
);

macro_rules! code {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }
    };
}

code!(SourceSystem { MedTracker05 });
code!(RecordKind {
    Person,
    Medication,
    Location,
    DosageOption,
    Schedule,
    Assignment,
    Dose,
    Stock,
    Pause,
    Occurrence,
    HealthEvent,
    Attachment,
    Other
});
code!(RelationshipKind {
    PersonMedication,
    SchedulePerson,
    ScheduleMedication,
    AssignmentPerson,
    AssignmentMedication,
    DoseSource,
    DoseTakenFromMedication,
    DoseTakenFromLocation,
    MedicationLocation,
    StockMedication,
    HealthEventMedication,
    PersonLocation,
    Other
});
code!(ExpectedRecord {
    Create,
    Match,
    Skip,
    Reject,
    DecisionRequired
});
code!(ActualRecord {
    Created,
    Matched,
    Skipped,
    Rejected,
    NotApplied
});
code!(ExpectedRelationship {
    Link,
    Omit,
    Reject,
    DecisionRequired
});
code!(ActualRelationship {
    Verified,
    NotLinked,
    Omitted,
    Rejected,
    NotApplied
});
code!(ReasonCode {
    UnsupportedField,
    UnsupportedRecord,
    UnsupportedRelationship,
    ObsoletePersonLocation,
    IdentityConflict,
    ConsentRequired,
    GuardianDeclarationRequired,
    InsufficientAuthority,
    MissingDependency,
    DestinationChanged,
    SourceInvalid,
    AttachmentUnsupported,
    Other
});
code!(ImportState {
    Preview,
    Blocked,
    Ready,
    Applying,
    Completed,
    Failed
});

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceIdentity {
    pub source: SourceSystem,
    pub instance: String,
    pub kind: RecordKind,
    pub id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DestinationRecord {
    pub id: DestinationId,
    pub href: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum DestinationId {
    Person(PersonId),
    Medication(MedicationId),
    Location(LocationId),
    DosageOption(DosageOptionId),
    Schedule(ScheduleId),
    Assignment(AssignmentId),
    Dose(DoseId),
    Stock(StockId),
    Pause(PauseId),
    Occurrence(OccurrenceId),
    HealthEvent(HealthEventId),
    Attachment(AttachmentId),
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct RecordDetails {
    pub name: Option<String>,
    pub dose_amount: Option<Decimal>,
    pub dose_unit: Option<String>,
    pub timing: Option<String>,
    pub stock_quantity: Option<Decimal>,
    pub stock_unit: Option<String>,
    pub occurred_at: Option<DateTime<Utc>>,
    pub source_fields: Vec<SourceField>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceField {
    pub field: String,
    pub source_value: String,
    pub outcome: FieldOutcome,
    pub destination_value: Option<String>,
    pub reason: Option<ReasonCode>,
}

code!(FieldOutcome {
    Preserved,
    Changed,
    Unsupported,
    Omitted
});

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecordResult {
    pub source: SourceIdentity,
    pub expected: ExpectedRecord,
    pub actual: Option<ActualRecord>,
    pub destination: Option<DestinationRecord>,
    pub reason: Option<ReasonCode>,
    pub details: RecordDetails,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RelationshipResult {
    pub kind: RelationshipKind,
    pub source: SourceIdentity,
    pub target: SourceIdentity,
    pub destination_source: Option<DestinationRecord>,
    pub destination_target: Option<DestinationRecord>,
    pub expected: ExpectedRelationship,
    pub actual: Option<ActualRelationship>,
    pub reason: Option<ReasonCode>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReportTotals {
    pub expected_create: u64,
    pub expected_match: u64,
    pub expected_link: u64,
    pub created: u64,
    pub matched: u64,
    pub verified_links: u64,
    pub skipped: u64,
    pub rejected: u64,
    pub unlinked: u64,
    pub decisions_required: u64,
    pub omissions: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Omission {
    pub item: SourceIdentity,
    pub field: Option<String>,
    pub reason: ReasonCode,
    pub acknowledgement_id: String,
    pub acknowledged: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PersonReport {
    pub person: RecordResult,
    pub records: Vec<RecordResult>,
    pub relationships: Vec<RelationshipResult>,
    pub omissions: Vec<Omission>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ImportReport {
    pub id: String,
    pub state: ImportState,
    pub source: SourceSystem,
    pub source_sha256: String,
    pub source_timezone: SourceTimeZone,
    pub created_at: DateTime<Utc>,
    pub totals: ReportTotals,
    pub people: Vec<PersonReport>,
    pub records: Vec<RecordResult>,
    pub relationships: Vec<RelationshipResult>,
    pub omissions: Vec<Omission>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceTimeZone(chrono_tz::Tz);

impl SourceTimeZone {
    pub fn parse(name: &str) -> Result<Self, chrono_tz::ParseError> {
        name.parse().map(Self)
    }

    pub fn name(self) -> &'static str {
        self.0.name()
    }

    pub fn zone(self) -> chrono_tz::Tz {
        self.0
    }
}

impl Serialize for SourceTimeZone {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut value = serializer.serialize_struct("SourceTimeZone", 2)?;
        value.serialize_field("name", self.name())?;
        value.serialize_field("provenance", "operator_confirmed")?;
        value.end()
    }
}

impl<'de> Deserialize<'de> for SourceTimeZone {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Input {
            name: String,
            provenance: Provenance,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case")]
        enum Provenance {
            OperatorConfirmed,
        }
        let input = Input::deserialize(deserializer)?;
        let Provenance::OperatorConfirmed = input.provenance;
        Self::parse(&input.name).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct PreviewToken(pub String);

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct PreviewResult {
    pub report: ImportReport,
    pub confirmation: Option<PreviewToken>,
}

#[derive(Clone)]
pub struct ArchivePreparation<'a> {
    pub source: SourceSystem,
    pub archive: &'a [u8],
    pub passphrase: &'a str,
}

pub struct PreparedArchive {
    pub transfer: PreparedTransfer,
    pub archive_sha256: String,
    pub receipt: PreparationReceipt,
    pub inventory: SourceInventory,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArchivePreparationError {
    InvalidArchive,
    TooLarge,
    UnsupportedEntry,
    DuplicateEntry,
    UnsupportedFields(Vec<SourcePreparationDiagnostic>),
    Preparation(PreparationError),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourcePreparationDiagnostic {
    pub path: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PreparationReceipt {
    pub payload_sha256: String,
    pub manifest_sha256: String,
    pub target_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceInventory {
    pub records: Vec<SourceRecord>,
    pub relationships: Vec<SourceRelationship>,
    pub omissions: Vec<Omission>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceRecord {
    pub identity: SourceIdentity,
    pub details: RecordDetails,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceRelationship {
    pub kind: RelationshipKind,
    pub field: String,
    pub source: SourceIdentity,
    pub target: SourceIdentity,
}

#[derive(Clone)]
pub struct SourcePreparation<'a> {
    pub source: SourceSystem,
    pub payload: &'a [u8],
    pub manifest: &'a [u8],
    pub passphrase: &'a str,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PersonDestination {
    Existing(PersonId),
    NewDependent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PersonMapping {
    pub source: SourceIdentity,
    pub destination: PersonDestination,
}

// Acceptance must be attributable to the subject or an eligible guardian; a
// household manager's role alone is not consent for another adult.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ConsentInput {
    ActingPerson {
        subject: PersonId,
        accepted: bool,
    },
    ActingGuardian {
        subject: PersonId,
        accepted: bool,
    },
    InitialGuardianDeclaration {
        source_person: SourceIdentity,
        accepted: bool,
    },
}

#[derive(Clone)]
pub struct PreviewCommand<'a> {
    pub source: ArchivePreparation<'a>,
    pub source_timezone: SourceTimeZone,
    pub mappings: Vec<PersonMapping>,
    pub consents: Vec<ConsentInput>,
    pub acknowledged_omissions: Vec<String>,
}

#[derive(Clone)]
pub struct ConfirmCommand<'a> {
    pub report_id: String,
    pub preview_token: PreviewToken,
    pub passphrase: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_reports_are_confirmable_and_show_clinical_details() {
        let preview: PreviewResult =
            serde_json::from_str(include_str!("examples/preview.json")).expect("preview document");
        let completed: ImportReport = serde_json::from_str(include_str!("examples/completed.json"))
            .expect("completed document");
        let blocked: PreviewResult =
            serde_json::from_str(include_str!("examples/blocked.json")).expect("blocked document");
        assert!(preview.confirmation.is_some());
        assert!(blocked.confirmation.is_none());
        assert_eq!(preview.report.state, ImportState::Ready);
        assert_eq!(blocked.report.state, ImportState::Blocked);
        assert_eq!(completed.state, ImportState::Completed);
        assert_eq!(
            preview.report.people[0].person.details.name.as_deref(),
            Some("Alex Example")
        );
        assert_eq!(
            completed.people[0].relationships[0].actual,
            Some(ActualRelationship::Verified)
        );
        assert!(!blocked.report.people[0].omissions.is_empty());
        assert!(!blocked.report.people[0].omissions[0].acknowledged);
        assert!(preview.report.people[0].omissions[0].acknowledged);
        assert_eq!(
            preview.report.people[0].records[1].details.dose_amount,
            Some("1.5".parse().expect("decimal"))
        );
        assert_eq!(
            completed.people[0].records[2].details.stock_quantity,
            Some("20".parse().expect("decimal"))
        );
        assert_eq!(completed.totals.verified_links, 1);
        let saved = serde_json::to_string(&preview).expect("serialise preview");
        let reloaded: PreviewResult = serde_json::from_str(&saved).expect("reload preview");
        assert!(reloaded.confirmation == preview.confirmation);
        let completed_json = serde_json::to_string(&completed).expect("serialise completed");
        let completed_again: ImportReport =
            serde_json::from_str(&completed_json).expect("reload completed");
        assert_eq!(completed_again, completed);
        let blocked_json = serde_json::to_string(&blocked).expect("serialise blocked");
        let blocked_again: PreviewResult =
            serde_json::from_str(&blocked_json).expect("reload blocked");
        assert!(blocked_again == blocked);
    }

    #[test]
    fn relationship_report_keeps_expected_and_verified_outcomes_distinct() {
        let report = RelationshipResult {
            kind: RelationshipKind::PersonMedication,
            source: SourceIdentity {
                source: SourceSystem::MedTracker05,
                instance: "canary".into(),
                kind: RecordKind::Person,
                id: "person-1".into(),
            },
            target: SourceIdentity {
                source: SourceSystem::MedTracker05,
                instance: "canary".into(),
                kind: RecordKind::Medication,
                id: "medicine-1".into(),
            },
            destination_source: None,
            destination_target: None,
            expected: ExpectedRelationship::Link,
            actual: Some(ActualRelationship::NotLinked),
            reason: Some(ReasonCode::InsufficientAuthority),
        };
        let json = serde_json::to_string(&report).expect("serialize report");
        assert!(json.contains("\"expected\":\"link\""));
        assert!(json.contains("\"actual\":\"not_linked\""));
        assert!(json.contains("\"reason\":\"insufficient_authority\""));
    }
}
