use super::*;
use crate::models::care::review_prompts::edit;
use axum::http::StatusCode;
use chrono::{Datelike, NaiveDate};
use loco_rs::model::TenantQueryExt;
use sea_orm::DatabaseTransaction;
use serde_json::Map;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
pub(crate) struct AuthContext<'a> {
    pub(crate) tenant: &'a TenantTransaction,
}
fn database_error(_: sea_orm::DbErr) -> OperationError {
    OperationError::Unavailable
}
fn today_in_app_zone(settings: Option<&serde_json::Value>) -> NaiveDate {
    Utc::now()
        .with_timezone(&doses::app_zone(settings))
        .date_naive()
}
pub(crate) async fn person_access(
    _: &DatabaseTransaction,
    context: &AuthContext<'_>,
    id: i64,
    manage: bool,
) -> Result<bool, OperationError> {
    access::can_access_person(
        context.tenant,
        id,
        if manage {
            PersonAccess::Manage
        } else {
            PersonAccess::View
        },
    )
    .await
}
fn sync_error(status: StatusCode) -> OperationError {
    match status {
        StatusCode::FORBIDDEN => OperationError::Forbidden,
        StatusCode::NOT_FOUND => OperationError::NotFound,
        StatusCode::PRECONDITION_REQUIRED | StatusCode::CONFLICT => OperationError::Conflict {
            code: if status == StatusCode::PRECONDITION_REQUIRED {
                "precondition_required"
            } else {
                "sync_conflict"
            }
            .into(),
            details: json!({"error":"A current review version is required"}),
        },
        _ => OperationError::Validation {
            details: json!({"status":status.as_u16(),"code":"unprocessable_content","message":"Review could not be saved"}),
        },
    }
}
pub(super) async fn apply(
    tenant: &TenantTransaction,
    operation: &Operation,
    _provenance: &CredentialProvenance,
) -> Result<Value, OperationError> {
    let context = AuthContext { tenant };
    Ok(apply_sync_operation(
        tenant.transaction(),
        &context,
        operation,
        &tenant.scope().request_id,
    )
    .await?
    .value())
}
pub(super) async fn authorize_replay(
    tenant: &TenantTransaction,
    operation: &Operation,
    saved: &Value,
) -> Result<(), OperationError> {
    let context = AuthContext { tenant };
    authorize_sync_replay(tenant.transaction(), &context, operation, saved).await
}
pub(crate) fn tag(record: &review_prompt::Model) -> String {
    let seconds = record.updated_at.and_utc().timestamp();
    let micros = record.updated_at.and_utc().timestamp_subsec_micros();
    let canonical = format!("MedicationReviewPrompt:{}:{seconds}.{micros:06}", record.id);
    format!("\"{}\"", hex::encode(Sha256::digest(canonical.as_bytes())))
}

pub(crate) async fn actor_adult(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
) -> Result<bool, OperationError> {
    let actor_id = context.tenant.membership().person_id;
    let Some(actor) =
        crate::models::access::person_queries::household_people(context.tenant.scope())
            .filter(person::Column::Id.eq(actor_id))
            .one(db)
            .await
            .map_err(database_error)?
    else {
        return Ok(false);
    };
    let adult_by_age = actor.date_of_birth.is_some_and(|born| {
        let today = today_in_app_zone(context.tenant.settings());
        let mut age = today.year() - born.year();
        if (today.month(), today.day()) < (born.month(), born.day()) {
            age -= 1
        }
        age >= 18
    });
    Ok(actor.person_type == 0 || adult_by_age)
}

pub(crate) async fn visible_prompt(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    id: &str,
) -> Result<Option<review_prompt::Model>, OperationError> {
    let Some(id) = id
        .parse::<i64>()
        .ok()
        .filter(|value| *value > 0 && id == value.to_string())
    else {
        return Ok(None);
    };
    review_prompt::Entity::find_by_id(id)
        .in_tenant(context.tenant.scope().household_id)
        .filter(
            review_prompt::Column::PersonId
                .in_subquery(access::participating_people(context.tenant.membership())),
        )
        .one(db)
        .await
        .map_err(database_error)
}

#[derive(Clone, Copy)]
enum ReviewStatus {
    NeedsReview,
    ReviewedWithPractitioner,
    ExpectedPrescribedCombination,
    NotRelevant,
    HiddenLowSignal,
}

impl ReviewStatus {
    fn parse(value: &str) -> Result<Self, StatusCode> {
        match value {
            "needs_review" => Ok(Self::NeedsReview),
            "reviewed_with_practitioner" => Ok(Self::ReviewedWithPractitioner),
            "expected_prescribed_combination" => Ok(Self::ExpectedPrescribedCombination),
            "not_relevant" => Ok(Self::NotRelevant),
            "hidden_low_signal" => Ok(Self::HiddenLowSignal),
            _ => Err(StatusCode::UNPROCESSABLE_ENTITY),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::NeedsReview => "needs_review",
            Self::ReviewedWithPractitioner => "reviewed_with_practitioner",
            Self::ExpectedPrescribedCombination => "expected_prescribed_combination",
            Self::NotRelevant => "not_relevant",
            Self::HiddenLowSignal => "hidden_low_signal",
        }
    }
}

pub(crate) struct Changes {
    status: Option<ReviewStatus>,
    practitioner_name: Option<String>,
    practitioner_role: Option<String>,
    reviewed_on: Option<Option<NaiveDate>>,
    review_note: Option<String>,
}

impl Changes {
    pub(crate) fn parse(body: &Value) -> Result<Self, StatusCode> {
        let root = body.as_object().ok_or(StatusCode::BAD_REQUEST)?;
        let Some(inner) = root
            .get("medication_review_prompt")
            .and_then(Value::as_object)
        else {
            return Err(StatusCode::BAD_REQUEST);
        };
        if root.len() != 1 {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        Self::parse_attributes(inner)
    }

    pub(crate) fn parse_attributes(inner: &Map<String, Value>) -> Result<Self, StatusCode> {
        if inner.is_empty()
            || inner.keys().any(|field| {
                !matches!(
                    field.as_str(),
                    "status"
                        | "practitioner_name"
                        | "practitioner_role"
                        | "reviewed_on"
                        | "review_note"
                )
            })
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let text = |field: &str| -> Result<Option<String>, StatusCode> {
            match inner.get(field) {
                None => Ok(None),
                Some(Value::String(value)) => Ok(Some(value.clone())),
                _ => Err(StatusCode::UNPROCESSABLE_ENTITY),
            }
        };
        let status = text("status")?
            .as_deref()
            .map(ReviewStatus::parse)
            .transpose()?;
        let reviewed_on = match inner.get("reviewed_on") {
            None => None,
            Some(Value::Null) => Some(None),
            Some(Value::String(date)) if date.is_empty() => Some(None),
            Some(Value::String(date)) => Some(Some(Self::parse_date(date)?)),
            _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
        };
        Ok(Self {
            status,
            practitioner_name: text("practitioner_name")?,
            practitioner_role: text("practitioner_role")?,
            reviewed_on,
            review_note: text("review_note")?,
        })
    }

    fn parse_date(date: &str) -> Result<NaiveDate, StatusCode> {
        let bytes = date.as_bytes();
        if bytes.len() != 10
            || bytes[4] != b'-'
            || bytes[7] != b'-'
            || bytes
                .iter()
                .enumerate()
                .any(|(index, byte)| index != 4 && index != 7 && !byte.is_ascii_digit())
        {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
    }

    pub(crate) fn from_browser_form(
        draft: &HashMap<String, String>,
        record: &review_prompt::Model,
    ) -> Result<Self, StatusCode> {
        let status = Some(ReviewStatus::parse(
            draft
                .get("status")
                .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?,
        )?);
        let reviewed_on = match draft.get("reviewed_on") {
            None => Some(None),
            Some(date) if date.is_empty() => Some(None),
            Some(date) => Some(Some(Self::parse_date(date)?)),
        };
        let text = |field: &str, current: Option<&str>| -> Result<Option<String>, StatusCode> {
            let value = draft.get(field).ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
            Ok((current.is_some() || !value.is_empty()).then(|| value.clone()))
        };
        Ok(Self {
            status,
            practitioner_name: text("practitioner_name", record.practitioner_name.as_deref())?,
            practitioner_role: text("practitioner_role", record.practitioner_role.as_deref())?,
            reviewed_on,
            review_note: text("review_note", record.review_note.as_deref())?,
        })
    }

    pub(crate) fn apply(
        self,
        record: &review_prompt::Model,
        membership_id: i64,
    ) -> Option<(review_prompt::ActiveModel, bool)> {
        let status = self
            .status
            .map(ReviewStatus::as_str)
            .unwrap_or(&record.status);
        let name = self
            .practitioner_name
            .as_deref()
            .or(record.practitioner_name.as_deref());
        let role = self
            .practitioner_role
            .as_deref()
            .or(record.practitioner_role.as_deref());
        let reviewed_on = self.reviewed_on.unwrap_or(record.reviewed_on);
        if matches!(
            status,
            "reviewed_with_practitioner" | "expected_prescribed_combination"
        ) && (name.is_none_or(|value| value.trim().is_empty())
            || role.is_none_or(|value| value.trim().is_empty())
            || reviewed_on.is_none())
        {
            return None;
        }
        let mut changed = record.clone().into_active_model();
        let mut dirty = false;
        if let Some(status) = self.status {
            dirty |= status.as_str() != record.status;
            changed.status = Set(status.as_str().to_owned());
        }
        if let Some(name) = self.practitioner_name {
            dirty |= Some(&name) != record.practitioner_name.as_ref();
            changed.practitioner_name = Set(Some(name));
        }
        if let Some(role) = self.practitioner_role {
            dirty |= Some(&role) != record.practitioner_role.as_ref();
            changed.practitioner_role = Set(Some(role));
        }
        if let Some(date) = self.reviewed_on {
            dirty |= date != record.reviewed_on;
            changed.reviewed_on = Set(date);
        }
        if let Some(note) = self.review_note {
            dirty |= Some(&note) != record.review_note.as_ref();
            changed.review_note = Set(Some(note));
        }
        if matches!(
            status,
            "reviewed_with_practitioner" | "expected_prescribed_combination"
        ) {
            dirty |= Some(membership_id) != record.reviewed_by_membership_id;
            changed.reviewed_by_membership_id = Set(Some(membership_id));
        }
        if dirty {
            changed.updated_at = Set(Utc::now().naive_utc());
        }
        Some((changed, dirty))
    }
}

pub(crate) async fn record_rest_update(
    tenant: &TenantTransaction,
    previous: &review_prompt::Model,
    updated: &review_prompt::Model,
) -> Result<(), OperationError> {
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        actor_account_id: Set(Some(tenant.scope().actor.account_id)),
        actor_membership_id: Set(Some(tenant.membership().id)),
        event_type: Set("medication_review_prompt.updated".to_owned()),
        request_id: Set(None),
        audit_context: Set(json!({})),
        metadata: Set(
            json!({"prompt_id": updated.id, "person_id": updated.person_id,
            "previous_status": previous.status, "status": updated.status}),
        ),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await
    .map_err(database_error)?;
    Ok(())
}

async fn apply_sync_operation(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    operation: &Operation,
    request_id: &str,
) -> Result<SyncResult, OperationError> {
    if operation.action != "update" || !actor_adult(db, context).await? {
        return Err(sync_error(StatusCode::FORBIDDEN));
    }
    let id = operation
        .id
        .as_deref()
        .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
    let record = visible_prompt(db, context, id)
        .await?
        .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
    if !person_access(db, context, record.person_id, true).await? {
        return Err(sync_error(StatusCode::FORBIDDEN));
    }
    crate::models::care::review_prompts::reconcile_person(context.tenant, record.person_id).await?;
    let record = visible_prompt(db, context, id)
        .await?
        .ok_or_else(|| sync_error(StatusCode::NOT_FOUND))?;
    let expected = operation
        .if_match
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| sync_error(StatusCode::PRECONDITION_REQUIRED))?;
    if expected != tag(&record) {
        return Err(sync_error(StatusCode::CONFLICT));
    }
    let mut attributes = operation.attributes.clone();
    let lease = attributes
        .remove("lease_token")
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default();
    edit::require(context.tenant, &record, &lease).await?;
    let changes = Changes::parse_attributes(&attributes).map_err(sync_error)?;
    let (active, dirty) = changes
        .apply(&record, context.tenant.membership().id)
        .ok_or_else(|| sync_error(StatusCode::UNPROCESSABLE_ENTITY))?;
    if !dirty {
        edit::consume(context.tenant, &record, &lease).await?;
        return Ok(SyncResult {
            record_type: "MedicationReviewPrompt",
            record_id: Some(record.id),
            record_portable_id: None,
            etag: Some(tag(&record)),
            replayed: None,
        });
    }
    edit::archive(context.tenant, &record, "review").await?;
    let updated = active.update(db).await.map_err(database_error)?;
    let now = Utc::now().naive_utc();
    security_audit_event::ActiveModel {
        household_id: Set(context.tenant.scope().household_id),
        actor_account_id: Set(Some(context.tenant.scope().actor.account_id)),
        actor_membership_id: Set(Some(context.tenant.membership().id)),
        event_type: Set("medication_review_prompt.updated".to_owned()),
        request_id: Set(Some(request_id.to_owned())),
        audit_context: Set(json!({})),
        metadata: Set(
            json!({"prompt_id": updated.id, "person_id": updated.person_id,
            "previous_status": record.status, "status": updated.status}),
        ),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    edit::consume(context.tenant, &record, &lease).await?;
    Ok(SyncResult {
        record_type: "MedicationReviewPrompt",
        record_id: Some(updated.id),
        record_portable_id: None,
        etag: Some(tag(&updated)),
        replayed: None,
    })
}

async fn authorize_sync_replay(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
    operation: &Operation,
    saved: &Value,
) -> Result<(), OperationError> {
    if operation.action != "update"
        || saved.get("action").and_then(Value::as_str) != Some("update")
        || saved.get("record_type").and_then(Value::as_str) != Some("MedicationReviewPrompt")
        || !actor_adult(db, context).await?
    {
        return Err(OperationError::Forbidden);
    }
    let id = operation.id.as_deref().ok_or(OperationError::Forbidden)?;
    let record = visible_prompt(db, context, id)
        .await?
        .ok_or(OperationError::Forbidden)?;
    if saved.get("record_id").and_then(Value::as_str) != Some(record.id.to_string().as_str())
        || !person_access(db, context, record.person_id, true).await?
    {
        return Err(OperationError::Forbidden);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> review_prompt::Model {
        let day = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        review_prompt::Model {
            id: 1,
            created_at: day.and_hms_opt(0, 0, 0).unwrap(),
            evidence_record_id: 1,
            evidence_source_checked_on: day,
            evidence_source_effective_on: day,
            evidence_source_name: String::new(),
            evidence_source_url: String::new(),
            evidence_source_version: String::new(),
            evidence_text: String::new(),
            household_id: 1,
            interacting_medication_id: 1,
            interacting_medication_name: String::new(),
            match_confidence: String::new(),
            match_reason: String::new(),
            match_type: String::new(),
            matched_term: String::new(),
            person_id: 1,
            practitioner_name: None,
            practitioner_role: None,
            primary_medication_id: 1,
            primary_medication_name: String::new(),
            review_note: None,
            reviewed_by_membership_id: None,
            reviewed_on: None,
            risk_level: String::new(),
            source_instruction: String::new(),
            status: "not_relevant".to_owned(),
            updated_at: day.and_hms_opt(0, 0, 0).unwrap(),
        }
    }

    #[test]
    fn browser_form_preserves_missing_blank_and_strict_date_contract() {
        let stored = record();
        let mut draft = HashMap::from([
            ("status".to_owned(), "not_relevant".to_owned()),
            ("practitioner_name".to_owned(), String::new()),
            ("practitioner_role".to_owned(), String::new()),
            ("review_note".to_owned(), String::new()),
        ]);
        let changes = Changes::from_browser_form(&draft, &stored).unwrap();
        assert!(changes.practitioner_name.is_none());
        assert!(changes.practitioner_role.is_none());
        assert!(changes.review_note.is_none());
        assert_eq!(changes.reviewed_on, Some(None));
        assert!(!changes.apply(&stored, 1).unwrap().1);

        draft.remove("practitioner_name");
        assert_eq!(
            Changes::from_browser_form(&draft, &stored).err(),
            Some(StatusCode::UNPROCESSABLE_ENTITY)
        );
        draft.insert("practitioner_name".to_owned(), String::new());
        draft.insert("reviewed_on".to_owned(), "2026-1-01".to_owned());
        assert_eq!(
            Changes::from_browser_form(&draft, &stored).err(),
            Some(StatusCode::UNPROCESSABLE_ENTITY)
        );
        draft.insert("reviewed_on".to_owned(), String::new());
        draft.insert("status".to_owned(), "reviewed_with_practitioner".to_owned());
        assert!(
            Changes::from_browser_form(&draft, &stored)
                .unwrap()
                .apply(&stored, 1)
                .is_none()
        );

        let mut with_name = stored;
        with_name.practitioner_name = Some("Dr Prior".to_owned());
        draft.insert("status".to_owned(), "not_relevant".to_owned());
        let changes = Changes::from_browser_form(&draft, &with_name).unwrap();
        assert_eq!(changes.practitioner_name.as_deref(), Some(""));
    }

    #[test]
    fn omitted_wire_status_preserves_unknown_stored_status() {
        let mut stored = record();
        stored.status = "legacy_review_state".to_owned();
        let changes =
            Changes::parse(&json!({"medication_review_prompt":{"review_note":"Updated"}})).unwrap();
        let (active, dirty) = changes.apply(&stored, 1).unwrap();
        assert!(dirty);
        assert_eq!(active.status.unwrap(), "legacy_review_state");
    }
}
