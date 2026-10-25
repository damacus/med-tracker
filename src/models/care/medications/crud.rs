use loco_rs::model::TenantQueryExt;
mod cascade;
mod dose_mode;
mod input;
mod persistence;
mod validation;

pub use input::{
    CreateInput, Field, LocationId, MedicationCreate, MedicationInput, ScheduleType,
    parse_decimal_text,
};

use super::*;
use crate::models::entities::{api_tombstone, person, person_medication, schedule};
use sea_orm::{Condition, QueryOrder, QuerySelect};
use std::collections::HashMap;

pub async fn create(
    tenant: &TenantTransaction,
    input: impl Into<CreateInput>,
    provenance: Option<&CredentialProvenance>,
) -> Result<medication::Model, OperationError> {
    create_with_schedule_config(tenant, input, None, provenance).await
}

pub(crate) async fn create_with_schedule_config(
    tenant: &TenantTransaction,
    input: impl Into<CreateInput>,
    schedule_config: Option<Value>,
    provenance: Option<&CredentialProvenance>,
) -> Result<medication::Model, OperationError> {
    let input = input.into();
    let context = StockContext { tenant, provenance };
    lock_row(
        tenant.transaction(),
        "households",
        tenant.scope().household_id,
    )
    .await?;
    access::recheck(tenant).await?;
    if !may_create(tenant).await? {
        return Err(OperationError::Forbidden);
    }
    let location_id = match &input {
        CreateInput::Draft(input) => input.location_for_scope(),
        CreateInput::Validated(command) => command.0.location_id.map(|id| id.0),
    };
    validate_location(tenant, location_id).await?;
    let attributes = match input {
        CreateInput::Draft(input) => input.validate(None).map_err(field_error)?,
        CreateInput::Validated(command) => {
            command.validate().map_err(field_error)?;
            command.0
        }
    };
    validate_barcode(tenant, attributes.barcode.as_deref(), None).await?;
    let now = Utc::now().naive_utc();
    let mut active = medication::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        created_by_membership_id: Set(Some(tenant.membership().id)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    attributes.assign(&mut active);
    if let Some(config) = schedule_config {
        active.default_schedule_config = Set(config);
    }
    let saved = active
        .insert(tenant.transaction())
        .await
        .map_err(write_error)?;
    audit::record_version(
        tenant.transaction(),
        &context,
        &tenant.scope().request_id,
        "Medication",
        saved.id,
        "api_create",
        None,
        Some(audit::medication_snapshot(&saved)),
    )
    .await?;
    persistence::change(
        &context,
        "Medication",
        saved.id,
        &saved.portable_id,
        "create",
        json!({}),
    )
    .await?;
    Ok(saved)
}

pub async fn update(
    tenant: &TenantTransaction,
    id: &str,
    input: MedicationInput,
    etag: Option<&str>,
    provenance: Option<&CredentialProvenance>,
) -> Result<medication::Model, OperationError> {
    let context = StockContext { tenant, provenance };
    let found = locked_manager(tenant, id, etag).await?;
    validate_location(tenant, input.location_for_scope()).await?;
    let attributes = input.validate(Some(&found)).map_err(field_error)?;
    validate_barcode(tenant, attributes.barcode.as_deref(), Some(found.id)).await?;
    let switching =
        found.dose_amount.is_none() && attributes.dose_amount.is_some_and(|value| value.is_some());
    if switching
        && schedule::Entity::find()
            .in_tenant(tenant.scope().household_id)
            .filter(schedule::Column::MedicationId.eq(found.id))
            .one(tenant.transaction())
            .await
            .map_err(database_error)?
            .is_some()
    {
        return Err(field_error((
            "dose_amount",
            "cannot switch dose mode while schedules exist",
        )));
    }
    if found.dose_amount.is_none() && input.only_null_dose() {
        return Ok(found);
    }
    let before = audit::medication_snapshot(&found);
    let mut active: medication::ActiveModel = found.into();
    attributes.assign(&mut active);
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active
        .update(tenant.transaction())
        .await
        .map_err(write_error)?;
    if switching {
        dose_mode::sync_single_dose_mode(&context, updated.id).await?;
    }
    audit::record_version(
        tenant.transaction(),
        &context,
        &tenant.scope().request_id,
        "Medication",
        updated.id,
        "api_update",
        Some(before),
        Some(audit::medication_snapshot(&updated)),
    )
    .await?;
    persistence::change(
        &context,
        "Medication",
        updated.id,
        &updated.portable_id,
        "update",
        json!({}),
    )
    .await?;
    Ok(updated)
}

pub async fn retire(
    tenant: &TenantTransaction,
    id: &str,
    etag: Option<&str>,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    retire_with_event(tenant, id, etag, provenance, "api_destroy").await
}

async fn retire_with_event(
    tenant: &TenantTransaction,
    id: &str,
    etag: Option<&str>,
    provenance: Option<&CredentialProvenance>,
    event: &str,
) -> Result<(), OperationError> {
    let context = StockContext { tenant, provenance };
    let found = locked_manager(tenant, id, etag).await?;
    let before = audit::medication_snapshot(&found);
    if !cascade::delete_medication_tree(
        tenant.transaction(),
        &context,
        &tenant.scope().request_id,
        &found,
    )
    .await?
    {
        return Err(validation(
            "Medication cannot be deleted while retained records exist",
        ));
    }
    audit::record_version(
        tenant.transaction(),
        &context,
        &tenant.scope().request_id,
        "Medication",
        found.id,
        event,
        Some(before),
        None,
    )
    .await
}

async fn locked_manager(
    tenant: &TenantTransaction,
    id: &str,
    etag: Option<&str>,
) -> Result<medication::Model, OperationError> {
    lock_row(
        tenant.transaction(),
        "households",
        tenant.scope().household_id,
    )
    .await?;
    access::recheck(tenant).await?;
    let found = visible_medication(tenant, id)
        .await?
        .ok_or(OperationError::NotFound)?;
    if !access::can_manage_household(tenant) {
        return Err(OperationError::Forbidden);
    }
    lock_row(tenant.transaction(), "medications", found.id).await?;
    let found = visible_medication(tenant, id)
        .await?
        .ok_or(OperationError::NotFound)?;
    if let Some(etag) = etag.filter(|v| !v.is_empty())
        && snapshot(tenant.transaction(), found.clone(), tenant.settings())
            .await?
            .etag
            != etag
    {
        return Err(conflict());
    }
    Ok(found)
}

async fn may_create(tenant: &TenantTransaction) -> Result<bool, OperationError> {
    if access::can_manage_household(tenant) {
        return Ok(true);
    }
    access::has_person_access(tenant, crate::models::access::PersonAccess::Manage).await
}

pub async fn can_create(tenant: &TenantTransaction) -> Result<bool, OperationError> {
    access::recheck(tenant).await?;
    may_create(tenant).await
}

async fn validate_location(
    tenant: &TenantTransaction,
    location_id: Option<i64>,
) -> Result<(), OperationError> {
    if !validation::valid_location(
        tenant.transaction(),
        tenant.scope().household_id,
        location_id,
    )
    .await?
    {
        return Err(OperationError::NotFound);
    }
    Ok(())
}

async fn validate_barcode(
    tenant: &TenantTransaction,
    barcode: Option<&str>,
    existing_id: Option<i64>,
) -> Result<(), OperationError> {
    if validation::barcode_conflict(tenant.transaction(), barcode, existing_id).await? {
        return Err(field_error(("barcode", "has already been taken")));
    }
    Ok(())
}

fn field_error((field, message): (&str, &str)) -> OperationError {
    OperationError::Validation {
        details: json!({"error":"Validation failed","errors":{field:[message]}}),
    }
}

fn write_error(error: sea_orm::DbErr) -> OperationError {
    if matches!(error.sql_err(),Some(sea_orm::SqlErr::UniqueConstraintViolation(name)) if name.contains("index_medications_on_barcode"))
    {
        field_error(("barcode", "has already been taken"))
    } else {
        database_error(error)
    }
}
