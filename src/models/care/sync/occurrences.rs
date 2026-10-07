use super::*;
use crate::models::care::dose_occurrences;
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
            .filter(api_change_event::Column::HouseholdId.eq(tenant.scope().household_id))
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
        let mut attrs = Map::new();
        attrs.insert("key".into(), json!(text("occurrence_key")?));
        for field in ["reason", "note"] {
            if let Some(value) = operation.attributes.get(field) {
                attrs.insert(field.into(), value.clone());
            }
        }
        let (body, etag) = dose_occurrences::change(
            tenant,
            (kind, id),
            "not_taken",
            &json!({"dose_occurrence":attrs}),
            None,
            secret,
            Some(provenance),
        )
        .await
        .map_err(source_error)?;
        let source_id = body["data"]["source_id"]
            .as_i64()
            .ok_or(OperationError::Unavailable)?;
        let date = body["data"]["window_starts_on"]
            .as_str()
            .and_then(|date| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok())
            .ok_or(OperationError::Unavailable)?;
        let position = body["data"]["position"]
            .as_i64()
            .and_then(|position| i32::try_from(position).ok())
            .ok_or(OperationError::Unavailable)?;
        let query = dose_occurrence::Entity::find()
            .filter(dose_occurrence::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(dose_occurrence::Column::WindowStartsOn.eq(date))
            .filter(dose_occurrence::Column::Position.eq(position));
        let query = match kind {
            "schedule" => query.filter(dose_occurrence::Column::ScheduleId.eq(source_id)),
            "person_medication" => {
                query.filter(dose_occurrence::Column::PersonMedicationId.eq(source_id))
            }
            _ => return Err(invalid()),
        };
        let row = query
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::Unavailable)?;
        let changes_after = api_change_event::Entity::find()
            .filter(api_change_event::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(api_change_event::Column::RecordType.eq("MedicationDoseOccurrence"))
            .count(tenant.transaction())
            .await?;
        return Ok(result(
            "MedicationDoseOccurrence",
            row.id,
            Some(&row.portable_id),
            Some(etag),
            Some(changes_after == changes_before),
        ));
    }
    if operation.attributes.len() != 1
        || operation.attributes.get("outcome").and_then(Value::as_str) != Some("open")
    {
        return Err(invalid());
    }
    let id = operation.id.as_deref().ok_or(OperationError::NotFound)?;
    let query = dose_occurrence::Entity::find()
        .filter(dose_occurrence::Column::HouseholdId.eq(tenant.scope().household_id));
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
    let data = dose_occurrences::list(
        tenant,
        kind,
        &source_id.to_string(),
        dose_occurrences::RangeQuery {
            start_date: Some(row.window_starts_on.to_string()),
            end_date: Some(row.window_starts_on.to_string()),
        },
        secret,
    )
    .await
    .map_err(source_error)?;
    let occurrence = data["data"]
        .as_array()
        .and_then(|rows| {
            rows.iter().find(|value| {
                value["window_starts_on"] == json!(row.window_starts_on.to_string())
                    && value["position"] == json!(row.position)
            })
        })
        .ok_or_else(invalid)?;
    let key = occurrence["key"].as_str().ok_or_else(invalid)?;
    let (_, etag) = dose_occurrences::change(
        tenant,
        (kind, &source_id.to_string()),
        "reopen",
        &json!({"dose_occurrence":{"key":key}}),
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
        Some(etag),
        Some(false),
    ))
}
