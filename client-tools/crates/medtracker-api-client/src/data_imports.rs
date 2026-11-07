//! Clinical data import inspection, preview and reports.
use serde::{Deserialize, Serialize};

use crate::{ApiClient, ApiError};

macro_rules! codes {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }
    };
}
codes!(SourceSystem { MedTracker05 });
codes!(RecordKind {
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
    NotificationPreference,
    Attachment,
    Other
});
codes!(RelationshipKind {
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
codes!(ExpectedRecord {
    Create,
    Match,
    Skip,
    Reject,
    DecisionRequired
});
codes!(ActualRecord {
    Created,
    Matched,
    Skipped,
    Rejected,
    NotApplied
});
codes!(ExpectedRelationship {
    Link,
    Omit,
    Reject,
    DecisionRequired
});
codes!(ActualRelationship {
    Verified,
    NotLinked,
    Omitted,
    Rejected,
    NotApplied
});
codes!(ReasonCode {
    UnsupportedField,
    UnsupportedRecord,
    UnsupportedRelationship,
    ObsoletePersonLocation,
    IdentityConflict,
    ConsentRequired,
    GuardianDeclarationRequired,
    InsufficientAuthority,
    MissingDependency,
    HistoricalActorUnavailable,
    DestinationChanged,
    SourceInvalid,
    AttachmentUnsupported,
    Other
});
codes!(ImportState {
    Preview,
    Blocked,
    Ready,
    Applying,
    Completed,
    Failed
});
codes!(FieldOutcome {
    Preserved,
    Changed,
    Unsupported,
    Omitted
});
codes!(TimeZoneProvenance { OperatorConfirmed });

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceIdentity {
    pub source: SourceSystem,
    pub instance: String,
    pub kind: RecordKind,
    pub id: String,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum DestinationId {
    Person(i64),
    Medication(i64),
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
pub struct DestinationRecord {
    pub id: DestinationId,
    pub href: Option<String>,
}
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct RecordDetails {
    pub name: Option<String>,
    pub dose_amount: Option<String>,
    pub dose_unit: Option<String>,
    pub timing: Option<String>,
    pub stock_quantity: Option<String>,
    pub stock_unit: Option<String>,
    pub occurred_at: Option<String>,
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
    #[serde(default)]
    pub field: Option<String>,
    pub source: SourceIdentity,
    pub target: SourceIdentity,
    pub destination_source: Option<DestinationRecord>,
    pub destination_target: Option<DestinationRecord>,
    pub expected: ExpectedRelationship,
    pub actual: Option<ActualRelationship>,
    pub reason: Option<ReasonCode>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
pub struct ImportIssue {
    pub source: Option<SourceIdentity>,
    pub field: Option<String>,
    pub reason: ReasonCode,
    pub detail: String,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ImportReport {
    pub id: String,
    pub state: ImportState,
    pub source: SourceSystem,
    pub source_sha256: String,
    pub source_timezone: SourceTimeZone,
    pub created_at: String,
    pub totals: ReportTotals,
    pub people: Vec<PersonReport>,
    pub records: Vec<RecordResult>,
    pub relationships: Vec<RelationshipResult>,
    pub omissions: Vec<Omission>,
    #[serde(default)]
    pub issues: Vec<ImportIssue>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SourceTimeZone {
    pub name: String,
    pub provenance: TimeZoneProvenance,
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
    pub reason: Option<ReasonCode>,
    pub source: SourceIdentity,
    pub target: SourceIdentity,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PersonDestination {
    Existing(i64),
    NewDependent,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PersonMapping {
    pub source: SourceIdentity,
    pub destination: PersonDestination,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ConsentInput {
    ActingPerson {
        subject: i64,
        accepted: bool,
    },
    ActingGuardian {
        subject: i64,
        accepted: bool,
    },
    InitialGuardianDeclaration {
        source_person: SourceIdentity,
        accepted: bool,
    },
}
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecipientConsent {
    pub source_person: SourceIdentity,
    pub request_id: String,
}

/// Contains an encrypted archive and its passphrase; deliberately does not implement Debug.
#[derive(Clone, Serialize)]
pub struct InspectionRequest {
    pub source: SourceSystem,
    pub archive_base64: String,
    pub passphrase: String,
    pub source_timezone: String,
    pub timezone_confirmed: bool,
}
/// Contains an encrypted archive, passphrase and consent references.
#[derive(Clone, Serialize)]
pub struct PreviewRequest {
    pub inspection_id: String,
    pub source: SourceSystem,
    pub archive_base64: String,
    pub passphrase: String,
    pub source_timezone: String,
    pub timezone_confirmed: bool,
    pub mappings: Vec<PersonMapping>,
    pub declarations: Vec<ConsentInput>,
    pub recipient_consents: Vec<RecipientConsent>,
    pub acknowledged_omissions: Vec<String>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Inspection {
    pub id: String,
    pub archive_sha256: String,
    pub timezone: SourceTimeZone,
    pub inventory: SourceInventory,
}
/// Confirmation is a secret service-issued token.
#[derive(Clone, Deserialize)]
pub struct PreviewResult {
    pub report: ImportReport,
    pub confirmation: Option<String>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct ReportPage {
    pub reports: Vec<ImportReport>,
    pub next_cursor: Option<String>,
}

impl ApiClient {
    #[doc = "# Errors\nThe request fails, is rejected, or its response cannot be decoded."]
    pub async fn inspect_data_import(
        &self,
        household_id: &str,
        request: &InspectionRequest,
    ) -> Result<Inspection, ApiError> {
        self.post_json_data(
            &format!("/api/v1/households/{household_id}/data_imports/inspections"),
            request,
        )
        .await
    }
    #[doc = "# Errors\nThe request fails, is rejected, or its response cannot be decoded."]
    pub async fn preview_data_import(
        &self,
        household_id: &str,
        request: &PreviewRequest,
    ) -> Result<PreviewResult, ApiError> {
        self.post_json_data(
            &format!("/api/v1/households/{household_id}/data_imports/previews"),
            request,
        )
        .await
    }
    #[doc = "# Errors\nThe request fails, is rejected, or its response cannot be decoded."]
    pub async fn data_import_reports(
        &self,
        household_id: &str,
        after_id: Option<&str>,
        limit: Option<u64>,
    ) -> Result<ReportPage, ApiError> {
        let limit = limit.map(|value| value.to_string());
        let mut query = Vec::new();
        if let Some(cursor) = after_id {
            query.push(("after_id", cursor));
        }
        if let Some(ref value) = limit {
            query.push(("limit", value.as_str()));
        }
        self.get_data_with_query(
            &format!("/api/v1/households/{household_id}/data_imports"),
            &query,
        )
        .await
    }
    #[doc = "# Errors\nThe request fails, is rejected, or its response cannot be decoded."]
    pub async fn data_import_report(
        &self,
        household_id: &str,
        id: &str,
    ) -> Result<PreviewResult, ApiError> {
        self.get_data(&format!(
            "/api/v1/households/{household_id}/data_imports/{id}"
        ))
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn four_routes_use_the_documented_wire_contract() -> Result<(), Box<dyn std::error::Error>>
    {
        let server = MockServer::start().await;
        let prefix = "/api/v1/households/42/data_imports";
        let identity =
            json!({"source":"med_tracker05","instance":"synthetic","kind":"person","id":"p1"});
        let report = json!({
            "id":"run-1","state":"ready","source":"med_tracker05","source_sha256":"abc",
            "source_timezone":{"name":"Europe/London","provenance":"operator_confirmed"},
            "created_at":"2026-11-06T12:00:00Z",
            "totals":{"expected_create":1,"expected_match":0,"expected_link":0,"created":0,"matched":0,"verified_links":0,"skipped":0,"rejected":0,"unlinked":0,"decisions_required":0,"omissions":0},
            "people":[],"records":[],"relationships":[],"omissions":[],"issues":[]
        });
        Mock::given(method("POST")).and(path(format!("{prefix}/inspections")))
            .and(header("authorization", "Bearer synthetic-token"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"data":{"id":"inspection-1","archive_sha256":"abc","timezone":{"name":"Europe/London","provenance":"operator_confirmed"},"inventory":{"records":[],"relationships":[],"omissions":[]}}}))).mount(&server).await;
        Mock::given(method("POST"))
            .and(path(format!("{prefix}/previews")))
            .respond_with(
                ResponseTemplate::new(201).set_body_json(
                    json!({"data":{"report":report,"confirmation":"synthetic-token"}}),
                ),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(prefix))
            .and(query_param("after_id", "cursor +"))
            .and(query_param("limit", "25"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"data":{"reports":[report],"next_cursor":null}})),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("{prefix}/run-1")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"data":{"report":report,"confirmation":null}})),
            )
            .mount(&server)
            .await;
        let client = ApiClient::new(server.uri(), Some("synthetic-token".into()))?;
        let inspection = InspectionRequest {
            source: SourceSystem::MedTracker05,
            archive_base64: "YWJj".into(),
            passphrase: "synthetic-passphrase".into(),
            source_timezone: "Europe/London".into(),
            timezone_confirmed: true,
        };
        assert_eq!(
            client.inspect_data_import("42", &inspection).await?.id,
            "inspection-1"
        );
        let source = SourceIdentity {
            source: SourceSystem::MedTracker05,
            instance: "synthetic".into(),
            kind: RecordKind::Person,
            id: "p1".into(),
        };
        let preview = PreviewRequest {
            inspection_id: "inspection-1".into(),
            source: inspection.source,
            archive_base64: inspection.archive_base64.clone(),
            passphrase: inspection.passphrase.clone(),
            source_timezone: inspection.source_timezone.clone(),
            timezone_confirmed: true,
            mappings: vec![PersonMapping {
                source: source.clone(),
                destination: PersonDestination::Existing(7),
            }],
            declarations: vec![ConsentInput::ActingPerson {
                subject: 7,
                accepted: true,
            }],
            recipient_consents: vec![RecipientConsent {
                source_person: source,
                request_id: "consent-1".into(),
            }],
            acknowledged_omissions: vec!["omission-1".into()],
        };
        assert_eq!(
            client
                .preview_data_import("42", &preview)
                .await?
                .report
                .state,
            ImportState::Ready
        );
        assert_eq!(
            client
                .data_import_reports("42", Some("cursor +"), Some(25))
                .await?
                .reports
                .len(),
            1
        );
        assert!(
            client
                .data_import_report("42", "run-1")
                .await?
                .confirmation
                .is_none()
        );
        let requests = server.received_requests().await.ok_or("capture disabled")?;
        let request = requests
            .iter()
            .find(|r| r.url.path().ends_with("/previews"))
            .ok_or("missing preview")?;
        let body: serde_json::Value = serde_json::from_slice(&request.body)?;
        assert_eq!(
            body["mappings"],
            json!([{"source":identity,"destination":{"Existing":7}}])
        );
        assert_eq!(
            body["declarations"],
            json!([{"ActingPerson":{"subject":7,"accepted":true}}])
        );
        assert_eq!(
            body["recipient_consents"],
            json!([{"source_person":identity,"request_id":"consent-1"}])
        );
        assert_eq!(body["acknowledged_omissions"], json!(["omission-1"]));
        assert_eq!(body["passphrase"], "synthetic-passphrase");
        Ok(())
    }
}
