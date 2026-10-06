use super::*;
use axum::http::StatusCode;
use chrono::{Datelike, NaiveDate};
use sea_orm::DatabaseTransaction;
use sha2::{Digest, Sha256};
struct AuthContext<'a> {
    tenant: &'a TenantTransaction,
}
fn database_error(_: sea_orm::DbErr) -> OperationError {
    OperationError::Unavailable
}
fn today_in_app_zone() -> NaiveDate {
    Utc::now().with_timezone(&doses::app_zone()).date_naive()
}
async fn person_access(
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
fn tag(record: &review_prompt::Model) -> String {
    let seconds = record.updated_at.and_utc().timestamp();
    let micros = record.updated_at.and_utc().timestamp_subsec_micros();
    let canonical = format!("MedicationReviewPrompt:{}:{seconds}.{micros:06}", record.id);
    format!("\"{}\"", hex::encode(Sha256::digest(canonical.as_bytes())))
}

async fn actor_adult(
    db: &DatabaseTransaction,
    context: &AuthContext<'_>,
) -> Result<bool, OperationError> {
    let Some(actor_id) = context.tenant.membership().person_id else {
        return Ok(false);
    };
    let Some(actor) = person::Entity::find_by_id(actor_id)
        .one(db)
        .await
        .map_err(database_error)?
    else {
        return Ok(false);
    };
    let adult_by_age = actor.date_of_birth.is_some_and(|born| {
        let today = today_in_app_zone();
        let mut age = today.year() - born.year();
        if (today.month(), today.day()) < (born.month(), born.day()) {
            age -= 1
        }
        age >= 18
    });
    Ok(actor.household_id == context.tenant.scope().household_id
        && (actor.person_type == 0 || adult_by_age))
}

async fn visible_prompt(
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
        .filter(review_prompt::Column::HouseholdId.eq(context.tenant.scope().household_id))
        .filter(
            review_prompt::Column::PersonId
                .in_subquery(access::granted_people(context.tenant.membership())),
        )
        .one(db)
        .await
        .map_err(database_error)
}

struct Changes {
    status: Option<String>,
    practitioner_name: Option<String>,
    practitioner_role: Option<String>,
    reviewed_on: Option<NaiveDate>,
    review_note: Option<String>,
}

impl Changes {
    fn parse(body: &Value) -> Result<Self, StatusCode> {
        let root = body.as_object().ok_or(StatusCode::BAD_REQUEST)?;
        let Some(inner) = root
            .get("medication_review_prompt")
            .and_then(Value::as_object)
        else {
            return Err(StatusCode::BAD_REQUEST);
        };
        if root.len() != 1
            || inner.is_empty()
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
        let status = text("status")?;
        if status.as_deref().is_some_and(|status| {
            !matches!(
                status,
                "needs_review"
                    | "reviewed_with_practitioner"
                    | "expected_prescribed_combination"
                    | "not_relevant"
                    | "hidden_low_signal"
            )
        }) {
            return Err(StatusCode::UNPROCESSABLE_ENTITY);
        }
        let reviewed_on = text("reviewed_on")?
            .map(|date| {
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
                NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                    .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)
            })
            .transpose()?;
        Ok(Self {
            status,
            practitioner_name: text("practitioner_name")?,
            practitioner_role: text("practitioner_role")?,
            reviewed_on,
            review_note: text("review_note")?,
        })
    }

    fn apply(
        self,
        record: &review_prompt::Model,
        membership_id: i64,
    ) -> Option<review_prompt::ActiveModel> {
        let status = self.status.clone().unwrap_or_else(|| record.status.clone());
        let name = self
            .practitioner_name
            .as_deref()
            .or(record.practitioner_name.as_deref());
        let role = self
            .practitioner_role
            .as_deref()
            .or(record.practitioner_role.as_deref());
        let reviewed_on = self.reviewed_on.or(record.reviewed_on);
        if matches!(
            status.as_str(),
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
            dirty |= status != record.status;
            changed.status = Set(status);
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
            dirty |= Some(date) != record.reviewed_on;
            changed.reviewed_on = Set(Some(date));
        }
        if let Some(note) = self.review_note {
            dirty |= Some(&note) != record.review_note.as_ref();
            changed.review_note = Set(Some(note));
        }
        if matches!(
            status.as_str(),
            "reviewed_with_practitioner" | "expected_prescribed_combination"
        ) {
            dirty |= Some(membership_id) != record.reviewed_by_membership_id;
            changed.reviewed_by_membership_id = Set(Some(membership_id));
        }
        if dirty {
            changed.updated_at = Set(Utc::now().naive_utc());
        }
        Some(changed)
    }
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
    let expected = operation
        .if_match
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| sync_error(StatusCode::PRECONDITION_REQUIRED))?;
    if expected != tag(&record) {
        return Err(sync_error(StatusCode::CONFLICT));
    }
    let changes = Changes::parse(&json!({"medication_review_prompt": operation.attributes}))
        .map_err(sync_error)?;
    let active = changes
        .apply(&record, context.tenant.membership().id)
        .ok_or_else(|| sync_error(StatusCode::UNPROCESSABLE_ENTITY))?;
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
