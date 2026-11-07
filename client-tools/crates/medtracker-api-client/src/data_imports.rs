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
codes!(ConsentDecision { Approved, Rejected });
codes!(ConsentStatus {
    Pending,
    Approved,
    Rejected
});

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
    NotificationPreference(i64),
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ConsentBinding {
    pub source_sha256: String,
    pub source_person: SourceIdentity,
    pub target_person: i64,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ConsentRequestResult {
    pub id: String,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PersonalConsentRequest {
    pub id: String,
    pub household_id: i64,
    pub requester_account_id: i64,
    pub binding: ConsentBinding,
    pub decision: ConsentStatus,
}
#[derive(Clone, Debug, Serialize)]
pub struct ConsentDecisionRequest {
    pub binding: ConsentBinding,
    pub decision: ConsentDecision,
}
/// Contains a service-issued token and archive passphrase.
#[derive(Clone, Serialize)]
pub struct ConfirmationRequest {
    pub preview_token: String,
    pub passphrase: String,
}
#[derive(Deserialize)]
struct ConfirmationResponse {
    report: ImportReport,
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
    #[doc = "# Errors\nThe request fails, is rejected, or its response cannot be decoded."]
    pub async fn create_data_import_consent(
        &self,
        household_id: &str,
        binding: &ConsentBinding,
    ) -> Result<ConsentRequestResult, ApiError> {
        self.post_json_data(
            &format!("/api/v1/households/{household_id}/data_imports/consents"),
            binding,
        )
        .await
    }
    #[doc = "# Errors\nThe request fails, is rejected, or its response cannot be decoded."]
    pub async fn data_import_consent(
        &self,
        household_id: &str,
        id: &str,
    ) -> Result<PersonalConsentRequest, ApiError> {
        self.get_data(&format!(
            "/api/v1/households/{household_id}/data_imports/consents/{id}"
        ))
        .await
    }
    #[doc = "# Errors\nThe request fails, is rejected, or its response cannot be decoded."]
    pub async fn decide_data_import_consent(
        &self,
        household_id: &str,
        id: &str,
        request: &ConsentDecisionRequest,
    ) -> Result<PersonalConsentRequest, ApiError> {
        self.post_json_data(
            &format!("/api/v1/households/{household_id}/data_imports/consents/{id}/decision"),
            request,
        )
        .await
    }
    /// Returns the persisted report; inspect its state for completed or failed apply.
    #[doc = "# Errors\nThe request fails, is rejected, or its response cannot be decoded."]
    pub async fn confirm_data_import(
        &self,
        household_id: &str,
        id: &str,
        request: &ConfirmationRequest,
    ) -> Result<ImportReport, ApiError> {
        let response: ConfirmationResponse = self
            .post_json_data(
                &format!("/api/v1/households/{household_id}/data_imports/{id}/confirm"),
                request,
            )
            .await?;
        Ok(response.report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn mount_consent_and_confirmation(
        server: &MockServer,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        use wiremock::matchers::body_json;
        let root = "/api/v1/households/42/data_imports";
        let source =
            json!({"source":"med_tracker05","instance":"synthetic","kind":"person","id":"p1"});
        let binding =
            json!({"source_sha256":"a".repeat(64),"source_person":source,"target_person":7});
        let personal = |decision| json!({"id":"consent-1","household_id":42,"requester_account_id":11,"binding":binding,"decision":decision});
        Mock::given(method("POST"))
            .and(path(format!("{root}/consents")))
            .and(header("authorization", "Bearer synthetic-token"))
            .and(body_json(binding.clone()))
            .respond_with(
                ResponseTemplate::new(201).set_body_json(json!({"data":{"id":"consent-1"}})),
            )
            .mount(server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("{root}/consents/consent-1")))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"data":personal("pending")})),
            )
            .mount(server)
            .await;
        for decision in ["approved", "rejected"] {
            Mock::given(method("POST"))
                .and(path(format!("{root}/consents/consent-1/decision")))
                .and(body_json(json!({"binding":binding,"decision":decision})))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(json!({"data":personal(decision)})),
                )
                .mount(server)
                .await;
        }
        let completed: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/clinical-import-completed.json"))?;
        let failed: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/clinical-import-failed.json"))?;
        let mut ready: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/clinical-import-blocked.json"))?;
        *ready.get_mut("state").ok_or("missing state")? = json!("ready");
        for (id, report) in [("run-completed", completed.clone()), ("run-failed", failed)] {
            Mock::given(method("POST"))
                .and(path(format!("{root}/{id}/confirm")))
                .and(header("authorization", "Bearer synthetic-token"))
                .and(body_json(
                    json!({"preview_token":"synthetic-token","passphrase":"synthetic-passphrase"}),
                ))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(json!({"data":{"report":report}})),
                )
                .mount(server)
                .await;
        }
        Mock::given(method("GET"))
            .and(path(format!("{root}/run-ready")))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"data":{"report":ready,"confirmation":"synthetic-ready-token"}}),
            ))
            .mount(server)
            .await;
        Mock::given(method("POST")).and(path(format!("{root}/run-stale/confirm")))
            .respond_with(ResponseTemplate::new(409).set_body_json(json!({"error":{"code":"conflict","message":"Stale preview","request_id":"req-stale"}}))).mount(server).await;
        let denied_root = "/api/v1/households/43/data_imports";
        for (verb, endpoint) in [
            ("POST", format!("{denied_root}/consents")),
            ("GET", format!("{denied_root}/consents/consent-1")),
            ("POST", format!("{denied_root}/consents/consent-1/decision")),
        ] {
            Mock::given(method(verb)).and(path(endpoint)).respond_with(
                ResponseTemplate::new(403).set_body_json(json!({"error":{"code":"forbidden","message":"Denied","request_id":"req-denied","trace_id":"trace-denied"}}))
            ).mount(server).await;
        }
        Ok(binding)
    }

    #[tokio::test]
    async fn consent_and_confirmation_contracts() -> Result<(), Box<dyn std::error::Error>> {
        let server = MockServer::start().await;
        let binding = mount_consent_and_confirmation(&server).await?;
        let client = ApiClient::new(server.uri(), Some("synthetic-token".into()))?;
        let binding: ConsentBinding = serde_json::from_value(binding)?;
        assert_eq!(
            client.create_data_import_consent("42", &binding).await?.id,
            "consent-1"
        );
        assert_eq!(
            client
                .data_import_consent("42", "consent-1")
                .await?
                .decision,
            ConsentStatus::Pending
        );
        for (decision, expected) in [
            (ConsentDecision::Approved, ConsentStatus::Approved),
            (ConsentDecision::Rejected, ConsentStatus::Rejected),
        ] {
            let request = ConsentDecisionRequest {
                binding: binding.clone(),
                decision,
            };
            assert_eq!(
                client
                    .decide_data_import_consent("42", "consent-1", &request)
                    .await?
                    .decision,
                expected
            );
        }
        let confirmation = ConfirmationRequest {
            preview_token: "synthetic-token".into(),
            passphrase: "synthetic-passphrase".into(),
        };
        assert_eq!(
            client
                .confirm_data_import("42", "run-completed", &confirmation)
                .await?
                .state,
            ImportState::Completed
        );
        assert_eq!(
            client
                .confirm_data_import("42", "run-completed", &confirmation)
                .await?
                .state,
            ImportState::Completed
        );
        assert_eq!(
            client
                .confirm_data_import("42", "run-failed", &confirmation)
                .await?
                .state,
            ImportState::Failed
        );
        assert_eq!(
            client
                .data_import_report("42", "run-ready")
                .await?
                .confirmation
                .as_deref(),
            Some("synthetic-ready-token")
        );
        let error = client
            .confirm_data_import("42", "run-stale", &confirmation)
            .await
            .err()
            .ok_or("expected stale preview error")?;
        assert!(
            matches!(error, ApiError::Server { status: 409, ref code, ref request_id, .. } if code == "conflict" && request_id.as_deref() == Some("req-stale"))
        );
        let denied_decision = ConsentDecisionRequest {
            binding: binding.clone(),
            decision: ConsentDecision::Rejected,
        };
        let errors = [
            client
                .create_data_import_consent("43", &binding)
                .await
                .err(),
            client.data_import_consent("43", "consent-1").await.err(),
            client
                .decide_data_import_consent("43", "consent-1", &denied_decision)
                .await
                .err(),
        ];
        for error in errors {
            assert!(
                matches!(error, Some(ApiError::Server { status: 403, ref code, ref request_id, ref trace_id, .. }) if code == "forbidden" && request_id.as_deref() == Some("req-denied") && trace_id.as_deref() == Some("trace-denied"))
            );
        }
        Ok(())
    }

    async fn mount_import_routes(server: &MockServer) -> Result<(), Box<dyn std::error::Error>> {
        let prefix = "/api/v1/households/42/data_imports";
        let inspection_response: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/clinical-import-inspection.json"))?;
        let blocked: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/clinical-import-blocked.json"))?;
        let completed: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/clinical-import-completed.json"))?;
        let failed: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/clinical-import-failed.json"))?;
        Mock::given(method("POST"))
            .and(path(format!("{prefix}/inspections")))
            .and(header("authorization", "Bearer synthetic-token"))
            .respond_with(
                ResponseTemplate::new(201).set_body_json(json!({"data":inspection_response})),
            )
            .mount(server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!("{prefix}/previews")))
            .respond_with(
                ResponseTemplate::new(201)
                    .set_body_json(json!({"data":{"report":blocked,"confirmation":null}})),
            )
            .mount(server)
            .await;
        Mock::given(method("GET"))
            .and(path(prefix))
            .and(query_param("after_id", "cursor +"))
            .and(query_param("limit", "25"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                json!({"data":{"reports":[blocked,completed,failed],"next_cursor":null}}),
            ))
            .mount(server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("{prefix}/run-1")))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"data":{"report":completed,"confirmation":null}})),
            )
            .mount(server)
            .await;
        Ok(())
    }

    fn check_blocked_report(
        blocked_report: &ImportReport,
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(blocked_report.state, ImportState::Blocked);
        let person = blocked_report.people.first().ok_or("missing person")?;
        assert_eq!(person.person.expected, ExpectedRecord::DecisionRequired);
        assert_eq!(person.person.actual, None);
        assert_eq!(
            person.relationships.first().ok_or("missing edge")?.actual,
            None
        );
        assert_eq!(
            blocked_report
                .relationships
                .first()
                .ok_or("missing blocked edge")?
                .actual,
            None
        );
        assert_eq!(
            blocked_report.issues.first().ok_or("missing issue")?.reason,
            ReasonCode::ConsentRequired
        );
        assert!(
            !person
                .omissions
                .first()
                .ok_or("missing omission")?
                .acknowledged
        );
        Ok(())
    }

    fn check_completed_report(
        completed_report: &ImportReport,
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(completed_report.state, ImportState::Completed);
        let person = completed_report.people.first().ok_or("missing person")?;
        assert_eq!(person.person.actual, Some(ActualRecord::Matched));
        assert_eq!(
            person
                .relationships
                .first()
                .ok_or("missing verified edge")?
                .actual,
            Some(ActualRelationship::Verified)
        );
        assert_eq!(
            completed_report
                .relationships
                .first()
                .ok_or("missing omitted edge")?
                .actual,
            Some(ActualRelationship::Omitted)
        );
        let schedule = person.records.first().ok_or("missing schedule")?;
        assert_eq!(schedule.details.stock_quantity.as_deref(), Some("20.25"));
        let preference = person.records.get(1).ok_or("missing preference")?;
        assert_eq!(
            preference.destination.as_ref().map(|d| &d.id),
            Some(&DestinationId::NotificationPreference(91))
        );
        assert!(
            person
                .omissions
                .first()
                .ok_or("missing omission")?
                .acknowledged
        );
        assert_eq!(
            completed_report
                .issues
                .first()
                .ok_or("missing issue")?
                .reason,
            ReasonCode::UnsupportedField
        );
        Ok(())
    }

    fn check_inventory(inventory: &SourceInventory) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(inventory.records.len(), 2);
        assert_eq!(
            inventory
                .relationships
                .first()
                .ok_or("missing source edge")?
                .kind,
            RelationshipKind::PersonMedication
        );
        assert_eq!(
            inventory
                .omissions
                .first()
                .ok_or("missing omission")?
                .acknowledgement_id,
            "omit-1"
        );
        assert_eq!(
            inventory
                .records
                .get(1)
                .ok_or("missing schedule")?
                .details
                .dose_amount
                .as_deref(),
            Some("1.50")
        );
        Ok(())
    }

    #[tokio::test]
    async fn four_routes_use_the_documented_wire_contract() -> Result<(), Box<dyn std::error::Error>>
    {
        let server = MockServer::start().await;
        mount_import_routes(&server).await?;
        let identity =
            json!({"source":"med_tracker05","instance":"synthetic","kind":"person","id":"p1"});
        let client = ApiClient::new(server.uri(), Some("synthetic-token".into()))?;
        let inspection = InspectionRequest {
            source: SourceSystem::MedTracker05,
            archive_base64: "YWJj".into(),
            passphrase: "synthetic-passphrase".into(),
            source_timezone: "Europe/London".into(),
            timezone_confirmed: true,
        };
        let inventory = client
            .inspect_data_import("42", &inspection)
            .await?
            .inventory;
        check_inventory(&inventory)?;
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
        let blocked_report = client.preview_data_import("42", &preview).await?.report;
        check_blocked_report(&blocked_report)?;
        let page = client
            .data_import_reports("42", Some("cursor +"), Some(25))
            .await?;
        assert_eq!(page.reports.len(), 3);
        let failed_report = page.reports.get(2).ok_or("missing failed report")?;
        assert_eq!(failed_report.state, ImportState::Failed);
        assert_eq!(
            failed_report
                .relationships
                .first()
                .ok_or("missing unlinked edge")?
                .actual,
            Some(ActualRelationship::NotLinked)
        );
        let detail = client.data_import_report("42", "run-1").await?;
        assert!(detail.confirmation.is_none());
        let completed_report = detail.report;
        check_completed_report(&completed_report)?;
        let requests = server.received_requests().await.ok_or("capture disabled")?;
        let request = requests
            .iter()
            .find(|r| r.url.path().ends_with("/previews"))
            .ok_or("missing preview")?;
        let body: serde_json::Value = serde_json::from_slice(&request.body)?;
        assert_eq!(
            body.get("mappings"),
            Some(&json!([{"source":identity,"destination":{"Existing":7}}]))
        );
        assert_eq!(
            body.get("declarations"),
            Some(&json!([{"ActingPerson":{"subject":7,"accepted":true}}]))
        );
        assert_eq!(
            body.get("recipient_consents"),
            Some(&json!([{"source_person":identity,"request_id":"consent-1"}]))
        );
        assert_eq!(
            body.get("acknowledged_omissions"),
            Some(&json!(["omission-1"]))
        );
        assert_eq!(
            body.get("passphrase").and_then(serde_json::Value::as_str),
            Some("synthetic-passphrase")
        );
        Ok(())
    }
}
