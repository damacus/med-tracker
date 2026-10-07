use super::*;

fn matches_id(id: Option<&str>, saved: &Value) -> bool {
    id.is_some_and(|id| {
        saved["record_id"].as_str() == Some(id) || saved["record_portable_id"].as_str() == Some(id)
    })
}
fn superseding<'a>(
    operations: &'a [Operation],
    results: &'a [Value],
    index: usize,
) -> Option<(&'a Operation, &'a Value)> {
    let operation = &operations[index];
    let result = &results[index];
    if operation.action == "delete" || !matches_id(operation.id.as_deref(), result) {
        return None;
    }
    operations
        .iter()
        .zip(results)
        .enumerate()
        .skip(index + 1)
        .find_map(|(later_index, (later, saved))| {
            (later.resource_type == operation.resource_type
                && later.action == "delete"
                && saved["index"].as_u64() == Some(later_index as u64)
                && saved["action"] == "delete"
                && saved["record_type"] == result["record_type"]
                && saved["record_id"] == result["record_id"]
                && saved["record_portable_id"] == result["record_portable_id"]
                && matches_id(later.id.as_deref(), saved))
            .then_some((later, saved))
        })
}
pub async fn authorize_replay(
    tenant: &TenantTransaction,
    body: &Value,
    saved: &Value,
    original_request_id: Option<&str>,
    provenance: &CredentialProvenance,
) -> Result<(), OperationError> {
    let operations = input::parse(body)?;
    if saved["data"]["applied"].as_bool() != Some(true) {
        return Err(OperationError::Forbidden);
    }
    let results = saved["data"]["results"]
        .as_array()
        .filter(|rows| rows.len() == operations.len())
        .ok_or(OperationError::Forbidden)?;
    for (index, (operation, result)) in operations.iter().zip(results).enumerate() {
        let kind = match operation.resource_type.as_str() {
            "medication" => "Medication",
            "medication_dosage_option" => "MedicationDosageOption",
            "medication_take" => "MedicationTake",
            "medication_dose_occurrence" => "MedicationDoseOccurrence",
            "medication_pause_period" => "MedicationPausePeriod",
            "person" => "Person",
            "health_event" => "HealthEvent",
            "location" => "Location",
            "medication_review_prompt" => "MedicationReviewPrompt",
            "schedule" => "Schedule",
            "person_medication" => "PersonMedication",
            _ => return Err(OperationError::Forbidden),
        };
        if result["index"].as_u64() != Some(index as u64)
            || result["action"].as_str() != Some(&operation.action)
            || result["record_type"].as_str() != Some(kind)
        {
            return Err(OperationError::Forbidden);
        }
        let (operation, saved) =
            superseding(&operations, results, index).unwrap_or((operation, result));
        authorize(tenant, operation, saved, original_request_id, provenance)
            .await
            .map_err(|error| match error {
                OperationError::Unavailable => error,
                _ => OperationError::Forbidden,
            })?;
    }
    Ok(())
}
async fn deleted(
    tenant: &TenantTransaction,
    kind: &str,
    id: i64,
    portable: &str,
) -> Result<Value, OperationError> {
    let rows = api_tombstone::Entity::find()
        .filter(api_tombstone::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(api_tombstone::Column::RecordType.eq(kind))
        .filter(api_tombstone::Column::RecordPortableId.eq(portable))
        .all(tenant.transaction())
        .await?;
    rows.into_iter()
        .find(|row| row.metadata["record_id"].as_i64() == Some(id))
        .map(|row| row.metadata)
        .ok_or(OperationError::Forbidden)
}
async fn person_identifier(
    tenant: &TenantTransaction,
    id: &str,
) -> Result<person::Model, OperationError> {
    let query =
        person::Entity::find().filter(person::Column::HouseholdId.eq(tenant.scope().household_id));
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(person::Column::Id.eq(id))
    } else {
        query.filter(person::Column::PortableId.eq(id))
    };
    query
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::Forbidden)
}
async fn source(
    tenant: &TenantTransaction,
    kind: &str,
    id: i64,
) -> Result<(i64, String, bool), OperationError> {
    if kind == "schedule" {
        let row = schedule::Entity::find_by_id(id)
            .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::Forbidden)?;
        Ok((row.person_id, row.portable_id, row.retired_at.is_some()))
    } else {
        let row = person_medication::Entity::find_by_id(id)
            .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::Forbidden)?;
        Ok((row.person_id, row.portable_id, row.retired_at.is_some()))
    }
}
async fn authorize(
    tenant: &TenantTransaction,
    operation: &Operation,
    saved: &Value,
    original_request_id: Option<&str>,
    provenance: &CredentialProvenance,
) -> Result<(), OperationError> {
    let id = saved["record_id"]
        .as_str()
        .and_then(|value| value.parse::<i64>().ok())
        .ok_or(OperationError::Forbidden)?;
    let portable = saved["record_portable_id"].as_str();
    if operation.id.is_some() && !matches_id(operation.id.as_deref(), saved) {
        return Err(OperationError::Forbidden);
    }
    match operation.resource_type.as_str() {
        "health_event" => {
            return health_events::authorize_replay(
                tenant,
                operation,
                saved,
                original_request_id.ok_or(OperationError::Forbidden)?,
                provenance,
            )
            .await;
        }
        "medication_review_prompt" => {
            return review_prompts::authorize_replay(tenant, operation, saved).await;
        }
        "location" => {
            locations::authorize(tenant).await?;
            if operation.action == "delete" {
                deleted(
                    tenant,
                    "Location",
                    id,
                    portable.ok_or(OperationError::Forbidden)?,
                )
                .await?;
            } else {
                let row = locations::read(tenant, &id.to_string()).await?;
                if Some(row.portable_id.as_str()) != portable {
                    return Err(OperationError::Forbidden);
                }
            }
        }
        "medication" => {
            if operation.action == "delete" {
                if !access::can_manage_household(tenant) {
                    return Err(OperationError::Forbidden);
                }
                deleted(
                    tenant,
                    "Medication",
                    id,
                    portable.ok_or(OperationError::Forbidden)?,
                )
                .await?;
            } else {
                let row = medications::read_stock_snapshot(tenant, &id.to_string())
                    .await?
                    .medication;
                if Some(row.portable_id.as_str()) != portable {
                    return Err(OperationError::Forbidden);
                }
                if operation.action == "create" {
                    if !medications::crud::can_create(tenant).await? {
                        return Err(OperationError::Forbidden);
                    }
                } else if !matches!(
                    operation.action.as_str(),
                    "mark_as_ordered" | "mark_as_received"
                ) && !access::can_manage_household(tenant)
                {
                    return Err(OperationError::Forbidden);
                }
            }
        }
        "medication_dosage_option" => {
            if !dosages::can_manage(tenant).await? {
                return Err(OperationError::Forbidden);
            }
            let (body, _) = dosages::read(tenant, &id.to_string()).await?;
            if body["data"]["portable_id"].as_str() != portable {
                return Err(OperationError::Forbidden);
            }
        }
        "person" => {
            let row = person_identifier(tenant, &id.to_string()).await?;
            if Some(row.portable_id.as_str()) != portable {
                return Err(OperationError::Forbidden);
            }
            if operation.action == "create" {
                people::authorize_create(tenant).await?;
            }
            people::authorize_update(tenant, &id.to_string()).await?;
        }
        "medication_take" => {
            let row = medication_take::Entity::find_by_id(id)
                .filter(medication_take::Column::HouseholdId.eq(tenant.scope().household_id))
                .one(tenant.transaction())
                .await?
                .ok_or(OperationError::Forbidden)?;
            if Some(row.portable_id.as_str()) != portable {
                return Err(OperationError::Forbidden);
            }
            let (kind, source_id) = if let Some(id) = row.schedule_id {
                ("schedule", id)
            } else {
                (
                    "person_medication",
                    row.person_medication_id.ok_or(OperationError::Forbidden)?,
                )
            };
            let (person, source_portable, _) = source(tenant, kind, source_id).await?;
            access::require_person_access(tenant, person, PersonAccess::Record).await?;
            if operation
                .attributes
                .get("source_type")
                .and_then(Value::as_str)
                != Some(kind)
                || !operation
                    .attributes
                    .get("source_id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| id == source_portable || id == source_id.to_string())
            {
                return Err(OperationError::Forbidden);
            }
        }
        "schedule" | "person_medication" => {
            let (person, source_portable, retired) =
                source(tenant, &operation.resource_type, id).await?;
            if Some(source_portable.as_str()) != portable {
                return Err(OperationError::Forbidden);
            }
            access::require_person_access(tenant, person, PersonAccess::Manage).await?;
            if operation.action == "delete" {
                if !retired {
                    return Err(OperationError::Forbidden);
                }
                deleted(
                    tenant,
                    if operation.resource_type == "schedule" {
                        "Schedule"
                    } else {
                        "PersonMedication"
                    },
                    id,
                    &source_portable,
                )
                .await?;
            }
            if let Some(person_id) = operation
                .attributes
                .get("person_id")
                .and_then(Value::as_str)
                && person_identifier(tenant, person_id).await?.id != person
            {
                return Err(OperationError::Forbidden);
            }
            if let Some(medication_id) = operation
                .attributes
                .get("medication_id")
                .and_then(Value::as_str)
            {
                medications::read_stock_snapshot(tenant, medication_id).await?;
            }
        }
        "medication_pause_period" => {
            let row = pause_period::Entity::find_by_id(id)
                .filter(pause_period::Column::HouseholdId.eq(tenant.scope().household_id))
                .one(tenant.transaction())
                .await?
                .ok_or(OperationError::Forbidden)?;
            if Some(row.portable_id.as_str()) != portable {
                return Err(OperationError::Forbidden);
            }
            let (kind, source_id) = if let Some(id) = row.schedule_id {
                ("schedule", id)
            } else {
                (
                    "person_medication",
                    row.person_medication_id.ok_or(OperationError::Forbidden)?,
                )
            };
            let (person, source_portable, _) = source(tenant, kind, source_id).await?;
            access::require_person_access(tenant, person, PersonAccess::Manage).await?;
            if operation.action == "create"
                && (operation
                    .attributes
                    .get("source_type")
                    .and_then(Value::as_str)
                    != Some(kind)
                    || operation
                        .attributes
                        .get("source_id")
                        .and_then(Value::as_str)
                        != Some(&source_portable))
            {
                return Err(OperationError::Forbidden);
            }
        }
        "medication_dose_occurrence" => {
            let row = dose_occurrence::Entity::find_by_id(id)
                .filter(dose_occurrence::Column::HouseholdId.eq(tenant.scope().household_id))
                .one(tenant.transaction())
                .await?
                .ok_or(OperationError::Forbidden)?;
            if Some(row.portable_id.as_str()) != portable {
                return Err(OperationError::Forbidden);
            }
            let (kind, source_id) = if let Some(id) = row.schedule_id {
                ("schedule", id)
            } else {
                (
                    "person_medication",
                    row.person_medication_id.ok_or(OperationError::Forbidden)?,
                )
            };
            let (person, source_portable, _) = source(tenant, kind, source_id).await?;
            access::require_person_access(
                tenant,
                person,
                if operation.action == "create" {
                    PersonAccess::Record
                } else {
                    PersonAccess::Manage
                },
            )
            .await?;
            if operation.action == "create"
                && (operation
                    .attributes
                    .get("source_type")
                    .and_then(Value::as_str)
                    != Some(kind)
                    || !operation
                        .attributes
                        .get("source_id")
                        .and_then(Value::as_str)
                        .is_some_and(|id| id == source_portable || id == source_id.to_string()))
            {
                return Err(OperationError::Forbidden);
            }
        }
        _ => return Err(OperationError::Forbidden),
    }
    Ok(())
}
