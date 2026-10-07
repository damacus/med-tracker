use super::*;
use sea_orm::PaginatorTrait;

fn invalid(message: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"code":"unprocessable_content","message":message}),
    }
}
pub(super) async fn apply(
    tenant: &TenantTransaction,
    operation: &Operation,
    provenance: &CredentialProvenance,
) -> Result<Value, OperationError> {
    let (body, etag, replayed) = if operation.action == "create" {
        if operation
            .attributes
            .get("source_id")
            .and_then(Value::as_str)
            .is_some_and(|id| id.parse::<i64>().is_ok())
        {
            return Err(OperationError::NotFound);
        }
        let query = pause_period::Entity::find()
            .filter(pause_period::Column::HouseholdId.eq(tenant.scope().household_id));
        let before = query.clone().count(tenant.transaction()).await?;
        let (body, etag) =
            pause_periods::create(tenant, &envelope(operation), Some(provenance)).await?;
        let replayed = before == query.count(tenant.transaction()).await?;
        (body, etag, replayed)
    } else {
        let id = operation.id.as_deref().ok_or(OperationError::NotFound)?;
        let row = pause_period::Entity::find()
            .filter(pause_period::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(pause_period::Column::PortableId.eq(id))
            .lock_exclusive()
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::NotFound)?;
        pause_periods::authorize_resume(tenant, id).await?;
        let schedules = schedule::Entity::find()
            .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(schedule::Column::Id.is_in(row.schedule_id))
            .all(tenant.transaction())
            .await?
            .into_iter()
            .map(|row| (row.id, row))
            .collect();
        let assignments = person_medication::Entity::find()
            .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(person_medication::Column::Id.is_in(row.person_medication_id))
            .all(tenant.transaction())
            .await?
            .into_iter()
            .map(|row| (row.id, row))
            .collect();
        let (_, etag) = pause_periods::period_values(
            tenant.transaction(),
            std::slice::from_ref(&row),
            &schedules,
            &assignments,
        )
        .await?
        .remove(0);
        required_etag(operation, &etag)?;
        if !operation.attributes.is_empty() {
            return Err(OperationError::Validation {
                details: json!({"code":"unprocessable_content","message":"Attributes are invalid"}),
            });
        }
        let (body, etag) =
            pause_periods::resume(tenant, id, &json!({}), Some(&etag), Some(provenance)).await?;
        (body, etag, row.ended_at.is_some())
    };
    let portable_id = body["data"]["portable_id"]
        .as_str()
        .ok_or(OperationError::Unavailable)?;
    let row = pause_period::Entity::find()
        .filter(pause_period::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(pause_period::Column::PortableId.eq(portable_id))
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    Ok(result(
        "MedicationPausePeriod",
        row.id,
        Some(&row.portable_id),
        Some(etag),
        Some(replayed),
    ))
}

pub(super) async fn source_change(
    tenant: &TenantTransaction,
    operation: &Operation,
    before: &Value,
    provenance: &CredentialProvenance,
) -> Result<Value, OperationError> {
    let id = operation.id.as_deref().ok_or(OperationError::NotFound)?;
    let schedule = operation.resource_type == "schedule";
    let kind = if schedule {
        "Schedule"
    } else {
        "PersonMedication"
    };
    if schedule {
        treatments::lifecycle::authorize_update(tenant, id).await?;
    } else {
        assignments::authorize_update(tenant, id).await?;
    }
    let portable = before["data"]["portable_id"]
        .as_str()
        .ok_or(OperationError::Unavailable)?;
    match operation.action.as_str() {
        "pause" => {
            if operation
                .attributes
                .keys()
                .any(|key| !matches!(key.as_str(), "reason" | "note"))
            {
                return Err(invalid("Attributes are invalid"));
            }
            let mut attrs = operation.attributes.clone();
            attrs.insert("source_type".into(), json!(operation.resource_type));
            attrs.insert("source_id".into(), json!(portable));
            pause_periods::create(
                tenant,
                &json!({"medication_pause_period":attrs}),
                Some(provenance),
            )
            .await
            .map_err(|error| match error {
                OperationError::Validation { .. } => invalid("Attributes are invalid"),
                _ => error,
            })?;
        }
        "resume" => {
            if !operation.attributes.is_empty() {
                return Err(OperationError::Validation {
                    details: json!({"status":400,"code":"bad_request","message":"Invalid request body"}),
                });
            }
            if schedule {
                pause_periods::resume_schedule(tenant, id, &json!({}), Some(provenance)).await?;
            } else {
                pause_periods::resume_assignment(tenant, id, &json!({}), Some(provenance)).await?;
            }
        }
        "reorder" => {
            reorder(tenant, operation, provenance).await?;
        }
        _ => return Err(invalid("Attributes are invalid")),
    }
    let (body, etag) = if schedule {
        treatments::lifecycle::read(tenant, id).await?
    } else {
        assignments::read(tenant, id).await?
    };
    super::operations::wire(kind, body, etag, None)
}

async fn reorder(
    tenant: &TenantTransaction,
    operation: &Operation,
    provenance: &CredentialProvenance,
) -> Result<(), OperationError> {
    let direction = operation
        .attributes
        .get("direction")
        .and_then(Value::as_str);
    if operation.attributes.len() != 1 || !matches!(direction, Some("up" | "down")) {
        return Err(invalid("Direction must be up or down"));
    }
    let id = operation.id.as_deref().ok_or(OperationError::NotFound)?;
    let query = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id));
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(person_medication::Column::Id.eq(id))
    } else {
        query.filter(person_medication::Column::PortableId.eq(id))
    };
    let source = query
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    access::require_person_access(tenant, source.person_id, PersonAccess::Manage).await?;
    let query = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(person_medication::Column::PersonId.eq(source.person_id))
        .filter(person_medication::Column::RetiredAt.is_null());
    let query = if direction == Some("up") {
        query
            .filter(person_medication::Column::Position.lt(source.position))
            .order_by_desc(person_medication::Column::Position)
            .order_by_desc(person_medication::Column::Id)
    } else {
        query
            .filter(person_medication::Column::Position.gt(source.position))
            .order_by_asc(person_medication::Column::Position)
            .order_by_asc(person_medication::Column::Id)
    };
    let Some(adjacent) = query.lock_exclusive().one(tenant.transaction()).await? else {
        return Ok(());
    };
    let person = person::Entity::find_by_id(source.person_id)
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    for (row, position) in [
        (source.clone(), adjacent.position),
        (adjacent, source.position),
    ] {
        let before = json!({"position":row.position});
        let id = row.id;
        let portable = row.portable_id.clone();
        let mut active = row.into_active_model();
        active.position = Set(position);
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(tenant.transaction()).await?;
        crate::models::care::administration::persistence::record_version_as(
            tenant,
            "PersonMedication",
            id,
            "update",
            Some(before),
            json!({"position":position}),
            Some(provenance),
        )
        .await?;
        super::persistence::change(
            tenant,
            "PersonMedication",
            id,
            &portable,
            "update",
            Some(&person.portable_id),
        )
        .await?;
    }
    Ok(())
}
