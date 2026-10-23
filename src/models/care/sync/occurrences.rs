use super::*;
use crate::models::care::dose_occurrences;
use loco_rs::model::TenantQueryExt;
use sea_orm::PaginatorTrait;

fn invalid() -> OperationError {
    OperationError::Validation {
        details: json!({"code":"invalid_occurrence","message":"Occurrence is unavailable"}),
    }
}
fn source_error(error: OperationError) -> OperationError {
    match error {
        OperationError::NotFound => invalid(),
        _ => error,
    }
}
pub(super) async fn apply(
    tenant: &TenantTransaction,
    operation: &Operation,
    secret: Option<&std::sync::Arc<[u8]>>,
    provenance: &CredentialProvenance,
) -> Result<Value, OperationError> {
    let secret = secret.ok_or(OperationError::Unavailable)?;
    if operation.action == "create" {
        let changes_before = api_change_event::Entity::find()
            .in_tenant(tenant.scope().household_id)
            .filter(api_change_event::Column::RecordType.eq("MedicationDoseOccurrence"))
            .count(tenant.transaction())
            .await?;
        if operation.attributes.keys().any(|key| {
            !matches!(
                key.as_str(),
                "source_type" | "source_id" | "occurrence_key" | "outcome" | "reason" | "note"
            )
        }) || operation.attributes.get("outcome").and_then(Value::as_str) != Some("not_taken")
        {
            return Err(invalid());
        }
        let text = |key: &str| {
            operation
                .attributes
                .get(key)
                .and_then(Value::as_str)
                .ok_or_else(invalid)
        };
        let kind = text("source_type")?;
        let id = text("source_id")?;
        let key = text("occurrence_key")?;
        let written = dose_occurrences::change(
            tenant,
            (kind, id),
            "not_taken",
            dose_occurrences::ChangeInput::not_taken(
                key,
                operation.attributes.get("reason"),
                operation.attributes.get("note"),
            ),
            None,
            secret,
            Some(provenance),
        )
        .await
        .map_err(source_error)?;
        let changes_after = api_change_event::Entity::find()
            .in_tenant(tenant.scope().household_id)
            .filter(api_change_event::Column::RecordType.eq("MedicationDoseOccurrence"))
            .count(tenant.transaction())
            .await?;
        return Ok(result(
            "MedicationDoseOccurrence",
            written.row.id,
            Some(&written.row.portable_id),
            Some(written.etag),
            Some(changes_after == changes_before),
        ));
    }
    if operation.attributes.len() != 1
        || operation.attributes.get("outcome").and_then(Value::as_str) != Some("open")
    {
        return Err(invalid());
    }
    let id = operation.id.as_deref().ok_or(OperationError::NotFound)?;
    let query = dose_occurrence::Entity::find().in_tenant(tenant.scope().household_id);
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(dose_occurrence::Column::Id.eq(id))
    } else {
        query.filter(dose_occurrence::Column::PortableId.eq(id))
    };
    let row = query
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    let (kind, source_id) = if let Some(id) = row.schedule_id {
        ("schedule", id)
    } else {
        (
            "person_medication",
            row.person_medication_id.ok_or_else(invalid)?,
        )
    };
    dose_occurrences::authorize(tenant, kind, &source_id.to_string(), "reopen")
        .await
        .map_err(source_error)?;
    if row.outcome != "not_taken" {
        return Err(invalid());
    }
    required_etag(operation, &dose_occurrences::record_etag(&row))?;
    let key = dose_occurrences::key_for_row(tenant, &row, secret)
        .await
        .map_err(source_error)?;
    let written = dose_occurrences::change(
        tenant,
        (kind, &source_id.to_string()),
        "reopen",
        dose_occurrences::ChangeInput::reopen(&key),
        operation.if_match.as_deref(),
        secret,
        Some(provenance),
    )
    .await
    .map_err(source_error)?;
    Ok(result(
        "MedicationDoseOccurrence",
        row.id,
        Some(&row.portable_id),
        Some(written.etag),
        Some(false),
    ))
}
