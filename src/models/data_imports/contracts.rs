use crate::models::ids::{AccountId, HouseholdId, MedicationId, PersonId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

macro_rules! code {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
        #[serde(rename_all = "snake_case")]
        pub(crate) enum $name { $($variant),+ }
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
pub(crate) struct SourceIdentity {
    pub source: SourceSystem,
    pub instance: String,
    pub kind: RecordKind,
    pub id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct DestinationRecord {
    pub id: DestinationId,
    pub href: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub(crate) enum DestinationId {
    Person(PersonId),
    Medication(MedicationId),
    Location(i64),
    DosageOption(i64),
    Schedule(i64),
    Assignment(i64),
    Dose(i64),
    Stock(i64),
    Pause(i64),
    Occurrence(i64),
    HealthEvent(i64),
    Attachment(i64),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct RecordResult {
    pub source: SourceIdentity,
    pub expected: ExpectedRecord,
    pub actual: Option<ActualRecord>,
    pub destination: Option<DestinationRecord>,
    pub reason: Option<ReasonCode>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct RelationshipResult {
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
pub(crate) struct ReportTotals {
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
pub(crate) struct Omission {
    pub item: SourceIdentity,
    pub field: Option<String>,
    pub reason: ReasonCode,
    pub acknowledgement_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct PersonReport {
    pub person: RecordResult,
    pub records: Vec<RecordResult>,
    pub relationships: Vec<RelationshipResult>,
    pub omissions: Vec<Omission>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct ImportReport {
    pub id: String,
    pub state: ImportState,
    pub source: SourceSystem,
    pub source_sha256: String,
    pub created_at: DateTime<Utc>,
    pub totals: ReportTotals,
    pub people: Vec<PersonReport>,
    pub records: Vec<RecordResult>,
    pub relationships: Vec<RelationshipResult>,
    pub omissions: Vec<Omission>,
}

#[derive(Clone)]
pub(crate) struct SourcePreparation<'a> {
    pub source: SourceSystem,
    pub payload: &'a [u8],
    pub manifest: &'a [u8],
    pub passphrase: &'a str,
}

#[derive(Clone, Debug)]
pub(crate) enum PersonDestination {
    Existing(PersonId),
    NewDependent,
}

#[derive(Clone, Debug)]
pub(crate) struct PersonMapping {
    pub source: SourceIdentity,
    pub destination: PersonDestination,
}

// Acceptance must be attributable to the subject or an eligible guardian; a
// household manager's role alone is not consent for another adult.
#[derive(Clone, Debug)]
pub(crate) enum ConsentInput {
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
pub(crate) struct PreviewCommand<'a> {
    pub actor: AccountId,
    pub household: HouseholdId,
    pub source: SourcePreparation<'a>,
    pub mappings: Vec<PersonMapping>,
    pub consents: Vec<ConsentInput>,
    pub acknowledged_omissions: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct ConfirmCommand {
    pub actor: AccountId,
    pub household: HouseholdId,
    pub report_id: String,
    pub preview_token: String,
}

#[cfg(test)]
mod tests {
    use super::*;

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
