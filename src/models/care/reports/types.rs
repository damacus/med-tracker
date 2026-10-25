use crate::models::{
    care::sync,
    entities::{person, review_prompt},
};
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, Utc};
use sea_orm::prelude::Decimal;
use serde::{Serialize, Serializer};

#[derive(Clone, Copy)]
pub struct ReportUtc(pub DateTime<Utc>);

impl Serialize for ReportUtc {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_rfc3339())
    }
}

#[derive(Clone, Copy)]
pub struct LocalMinute(pub NaiveDateTime);

impl Serialize for LocalMinute {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.format("%Y-%m-%d %H:%M").to_string())
    }
}

#[derive(Clone)]
pub struct GpTakeTime {
    pub at: DateTime<FixedOffset>,
    pub pdf: bool,
}

impl Serialize for GpTakeTime {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.pdf {
            serializer.serialize_str(&self.at.format("%Y-%m-%d %H:%M").to_string())
        } else {
            serializer.serialize_str(&self.at.to_rfc3339())
        }
    }
}

#[derive(Serialize)]
pub struct GpPerson {
    #[serde(serialize_with = "id_text")]
    pub id: i64,
    pub name: String,
    pub date_of_birth: Option<NaiveDate>,
}

impl From<&person::Model> for GpPerson {
    fn from(row: &person::Model) -> Self {
        Self {
            id: row.id,
            name: row.name.clone(),
            date_of_birth: row.date_of_birth,
        }
    }
}

#[derive(Serialize)]
pub struct GpMedicine {
    #[serde(serialize_with = "id_text")]
    pub id: i64,
    pub name: Option<String>,
}

#[derive(Serialize)]
pub struct GpEvent {
    #[serde(serialize_with = "id_text")]
    pub id: i64,
    pub event_kind: &'static str,
    pub title: String,
    pub started_on: NaiveDate,
    pub ended_on: Option<NaiveDate>,
    pub ongoing: bool,
    pub duration_days: Option<i64>,
    pub severity: Option<&'static str>,
    pub notes: Option<String>,
    pub action_taken: Option<String>,
    pub medical_help_sought: bool,
    pub medication_names: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_range_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct GpTake {
    pub taken_at: GpTakeTime,
    pub medication_name: Option<String>,
    #[serde(serialize_with = "decimal_text")]
    pub dose_amount: Option<Decimal>,
    pub dose_unit: Option<String>,
    pub source_type: &'static str,
    pub location_name: Option<String>,
}

fn decimal_text<S: Serializer>(value: &Option<Decimal>, serializer: S) -> Result<S::Ok, S::Error> {
    value.map(|amount| amount.to_string()).serialize(serializer)
}

fn id_text<S: Serializer>(value: &i64, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&value.to_string())
}

fn optional_id_text<S: Serializer>(value: &Option<i64>, serializer: S) -> Result<S::Ok, S::Error> {
    value.map(|id| id.to_string()).serialize(serializer)
}

#[derive(Serialize)]
pub struct GpHistoryReport {
    pub person: GpPerson,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub generated_at: ReportUtc,
    pub current_medicines: Vec<GpMedicine>,
    pub chronology: Vec<GpEvent>,
    pub medication_takes: Vec<GpTake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_medication_takes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporting_period: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birthday_label: Option<String>,
}

#[derive(Serialize)]
pub struct ReviewPerson {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "optional_id_text")]
    pub id: Option<i64>,
    pub name: String,
}

#[derive(Serialize)]
pub struct ReviewPromptRow {
    #[serde(serialize_with = "id_text")]
    pub id: i64,
    #[serde(serialize_with = "id_text")]
    pub person_id: i64,
    #[serde(serialize_with = "id_text")]
    pub primary_medication_id: i64,
    #[serde(serialize_with = "id_text")]
    pub interacting_medication_id: i64,
    #[serde(serialize_with = "id_text")]
    pub evidence_record_id: i64,
    pub primary_medication_name: String,
    pub interacting_medication_name: String,
    pub evidence_source_name: String,
    pub evidence_source_url: String,
    pub evidence_source_version: String,
    pub matched_term: String,
    pub match_type: String,
    pub source_instruction: String,
    pub match_reason: String,
    pub evidence_text: String,
    pub etag: String,
    pub evidence_source_checked_on: NaiveDate,
    pub evidence_source_effective_on: NaiveDate,
    pub risk_level: String,
    pub match_confidence: String,
    pub status: String,
    pub practitioner_name: Option<String>,
    pub practitioner_role: Option<String>,
    pub review_note: Option<String>,
    pub reviewed_on: Option<NaiveDate>,
    #[serde(serialize_with = "optional_id_text")]
    pub reviewed_by_membership_id: Option<i64>,
    pub updated_at: ReportUtc,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_type_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_instruction_label: Option<String>,
}

impl From<&review_prompt::Model> for ReviewPromptRow {
    fn from(row: &review_prompt::Model) -> Self {
        Self {
            id: row.id,
            person_id: row.person_id,
            primary_medication_id: row.primary_medication_id,
            interacting_medication_id: row.interacting_medication_id,
            evidence_record_id: row.evidence_record_id,
            primary_medication_name: row.primary_medication_name.clone(),
            interacting_medication_name: row.interacting_medication_name.clone(),
            evidence_source_name: row.evidence_source_name.clone(),
            evidence_source_url: row.evidence_source_url.clone(),
            evidence_source_version: row.evidence_source_version.clone(),
            matched_term: row.matched_term.clone(),
            match_type: row.match_type.clone(),
            source_instruction: row.source_instruction.clone(),
            match_reason: row.match_reason.clone(),
            evidence_text: row.evidence_text.clone(),
            etag: sync::review_prompts::tag(row),
            evidence_source_checked_on: row.evidence_source_checked_on,
            evidence_source_effective_on: row.evidence_source_effective_on,
            risk_level: row.risk_level.clone(),
            match_confidence: row.match_confidence.clone(),
            status: row.status.clone(),
            practitioner_name: row.practitioner_name.clone(),
            practitioner_role: row.practitioner_role.clone(),
            review_note: row.review_note.clone(),
            reviewed_on: row.reviewed_on,
            reviewed_by_membership_id: row.reviewed_by_membership_id,
            updated_at: ReportUtc(row.updated_at.and_utc()),
            person_name: None,
            risk_label: None,
            confidence_label: None,
            status_label: None,
            match_type_label: None,
            source_instruction_label: None,
        }
    }
}

#[derive(Serialize)]
pub struct MedicationReviewReport {
    pub person: ReviewPerson,
    pub generated_at: ReportUtc,
    pub prompts: Vec<ReviewPromptRow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub people_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_discuss: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reviewed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_label: Option<String>,
}

#[derive(Serialize)]
pub struct OrdinaryTake {
    pub taken_at: LocalMinute,
    pub source_id: i64,
    pub person_name: String,
    pub medication_name: String,
    #[serde(serialize_with = "decimal_text")]
    pub dose_amount: Option<Decimal>,
    pub dose_unit: Option<String>,
    pub source_type: &'static str,
    pub location_name: &'static str,
}

#[derive(Serialize)]
pub struct NotTakenOutcome {
    pub scheduled_at: NotTakenTime,
    pub person_name: String,
    pub medication_name: String,
    pub reason: Option<String>,
    pub note: Option<String>,
}

pub enum NotTakenTime {
    At(DateTime<FixedOffset>),
    Window(NaiveDate),
}

impl Serialize for NotTakenTime {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::At(at) => serializer.serialize_str(&at.format("%Y-%m-%d %H:%M").to_string()),
            Self::Window(day) => serializer.serialize_str(&day.to_string()),
        }
    }
}

#[derive(Serialize)]
pub struct DailyOutcome {
    pub date: NaiveDate,
    pub day_name: String,
    pub percentage: usize,
    pub expected: usize,
    pub actual: usize,
    pub not_taken: usize,
    pub unexplained_missed: usize,
}

#[derive(Serialize)]
pub struct CycleSummary {
    pub window_starts_on: NaiveDate,
    pub window_ends_on: NaiveDate,
    pub source_id: i64,
    pub person_name: String,
    pub medication_name: String,
    pub expected: usize,
    pub actual: usize,
    pub not_taken: usize,
    pub unexplained_missed: usize,
}

#[derive(Serialize)]
pub struct TodayMedicine {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize)]
pub struct TodayTaken {
    pub person_id: i64,
    pub person_name: String,
    pub medications: Vec<TodayMedicine>,
}

#[derive(Serialize)]
pub struct InventoryAlert {
    pub medication_name: String,
    pub days_left: i64,
    pub doses_left: f64,
    pub low_stock: bool,
}

#[derive(Serialize)]
pub struct Insight {
    pub title: &'static str,
    pub summary: String,
    pub detail: String,
    pub severity: &'static str,
}

#[derive(Serialize)]
pub struct SmartInsights {
    pub state: &'static str,
    pub evidence_summary: String,
    pub insights: Vec<Insight>,
}

#[derive(Serialize)]
pub struct OrdinaryEvent {
    pub title: String,
    pub started_on: NaiveDate,
    pub ended_on: Option<NaiveDate>,
    pub severity: Option<&'static str>,
    pub notes: Option<String>,
    pub action_taken: Option<String>,
    pub medication_names: Vec<String>,
}

#[derive(Serialize)]
pub struct IllnessPattern {
    pub display_title: String,
    pub episode_count: usize,
    pub first_started_on: NaiveDate,
    pub most_recent_started_on: NaiveDate,
    pub average_interval_days: i64,
}

#[derive(Serialize)]
pub struct OrdinarySummary {
    pub expected: usize,
    pub actual: usize,
    pub compliance: usize,
}

#[derive(Serialize)]
pub struct ReportPerson {
    pub id: i64,
    pub id_string: String,
    pub name: String,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum OrdinaryPeople {
    Names(Vec<String>),
    Browser(Vec<ReportPerson>),
}

#[derive(Serialize)]
pub struct OrdinaryHistoryReport {
    pub people: OrdinaryPeople,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub generated_at: ReportUtc,
    pub medication_takes: Vec<OrdinaryTake>,
    pub not_taken_outcomes: Vec<NotTakenOutcome>,
    pub daily_outcomes: Vec<DailyOutcome>,
    pub cycle_summaries: Vec<CycleSummary>,
    pub today_taken: Vec<TodayTaken>,
    pub inventory_alerts: Vec<InventoryAlert>,
    pub smart_insights: SmartInsights,
    pub summary: OrdinarySummary,
    pub suspected_side_effects: Vec<OrdinaryEvent>,
    pub notable_illnesses: Vec<OrdinaryEvent>,
    pub illness_patterns: Vec<IllnessPattern>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manageable_people: Option<Vec<ReportPerson>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_person_id: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_medication_takes: Option<bool>,
}
