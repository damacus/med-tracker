use super::*;

pub(super) async fn apply(
    tenant: &TenantTransaction,
    operation: &Operation,
    provenance: &CredentialProvenance,
) -> Result<Value, OperationError> {
    let schedule = operation.resource_type == "schedule";
    let kind = if schedule {
        "Schedule"
    } else {
        "PersonMedication"
    };
    if operation.action == "create" {
        let (body, etag) = if schedule {
            treatments::create(tenant, &envelope(operation), Some(provenance)).await?
        } else {
            assignments::create(tenant, &envelope(operation), Some(provenance)).await?
        };
        return super::operations::wire(kind, body, etag, (!schedule).then_some(false));
    }
    let id = operation.id.as_deref().ok_or(OperationError::NotFound)?;
    if schedule {
        treatments::lifecycle::authorize_update(tenant, id).await?;
    } else {
        assignments::authorize_update(tenant, id).await?;
    }
    let before = if schedule {
        treatments::lifecycle::read(tenant, id).await?
    } else {
        assignments::read(tenant, id).await?
    };
    required_etag(operation, &before.1)?;
    if operation.action == "update" {
        let (body, etag) = if schedule {
            treatments::lifecycle::update(
                tenant,
                id,
                &envelope(operation),
                operation.if_match.as_deref(),
                Some(provenance),
            )
            .await?
        } else {
            assignments::update(
                tenant,
                id,
                &envelope(operation),
                operation.if_match.as_deref(),
                Some(provenance),
            )
            .await?
        };
        let unchanged = before.0 == body;
        return super::operations::wire(kind, body, etag, (!schedule).then_some(unchanged));
    }
    if matches!(operation.action.as_str(), "pause" | "resume" | "reorder") {
        return super::pauses::source_change(tenant, operation, &before.0, provenance).await;
    }
    if !operation.attributes.is_empty() {
        return Err(OperationError::Validation {
            details: json!({"status":if schedule {400}else{422},"code":if schedule {"bad_request"}else{"unprocessable_content"},"message":if schedule {"Invalid request body"}else{"Person medication is invalid"}}),
        });
    }
    let record_id = before.0["data"]["id"]
        .as_i64()
        .ok_or(OperationError::Unavailable)?;
    let now = Utc::now().naive_utc();
    let (person_id, portable_id, clinical) = if schedule {
        treatments::lifecycle::authorize_update(tenant, id).await?;
        let row = schedule::Entity::find_by_id(record_id)
            .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
            .lock_exclusive()
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::NotFound)?;
        let clinical = treatments::representation(&row).0["data"].clone();
        let person_id = row.person_id;
        let portable_id = row.portable_id.clone();
        let mut active = row.into_active_model();
        active.active = Set(false);
        active.retired_at = Set(Some(now));
        active.updated_at = Set(now);
        active.update(tenant.transaction()).await?;
        (person_id, portable_id, clinical)
    } else {
        assignments::authorize_update(tenant, id).await?;
        let row = person_medication::Entity::find_by_id(record_id)
            .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
            .lock_exclusive()
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::NotFound)?;
        let clinical = assignments::clinical_snapshot(&row);
        let person_id = row.person_id;
        let portable_id = row.portable_id.clone();
        let mut active = row.into_active_model();
        active.active = Set(false);
        active.retired_at = Set(Some(now));
        active.updated_at = Set(now);
        active.update(tenant.transaction()).await?;
        (person_id, portable_id, clinical)
    };
    crate::models::care::administration::persistence::record_version_as(
        tenant,
        kind,
        record_id,
        "destroy",
        Some(clinical),
        json!({}),
        Some(provenance),
    )
    .await?;
    let person = person::Entity::find_by_id(person_id)
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    super::persistence::change(
        tenant,
        kind,
        record_id,
        &portable_id,
        "update",
        Some(&person.portable_id),
    )
    .await?;
    super::persistence::tombstone(tenant, kind, record_id, &portable_id, &person.portable_id)
        .await?;
    Ok(result(kind, record_id, Some(&portable_id), None, None))
}
