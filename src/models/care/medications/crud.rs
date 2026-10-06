mod cascade;
mod dose_mode;
mod persistence;
mod validation;

use super::*;
use crate::models::entities::{api_tombstone, grant, person, person_medication, schedule};
use sea_orm::sea_query::{Expr, ExprTrait};
use sea_orm::{Condition, QueryOrder, QuerySelect};
use std::collections::HashMap;

pub async fn create(
    tenant: &TenantTransaction,
    attributes: Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<medication::Model, OperationError> {
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
    validate(tenant, &attributes, None).await?;
    let now = Utc::now().naive_utc();
    let mut active = medication::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        created_by_membership_id: Set(Some(tenant.membership().id)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    validation::assign_attributes(&mut active, &attributes).map_err(field_error)?;
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
    attributes: Value,
    etag: Option<&str>,
    provenance: Option<&CredentialProvenance>,
) -> Result<medication::Model, OperationError> {
    let context = StockContext { tenant, provenance };
    let found = locked_manager(tenant, id, etag).await?;
    validate(tenant, &attributes, Some(&found)).await?;
    let switching =
        found.dose_amount.is_none() && attributes.get("dose_amount").is_some_and(|v| !v.is_null());
    if switching
        && schedule::Entity::find()
            .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
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
    if found.dose_amount.is_none()
        && attributes
            .as_object()
            .is_some_and(|v| v.len() == 1 && v.get("dose_amount") == Some(&Value::Null))
    {
        return Ok(found);
    }
    let before = audit::medication_snapshot(&found);
    let mut active: medication::ActiveModel = found.into();
    validation::assign_attributes(&mut active, &attributes).map_err(field_error)?;
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
        "api_destroy",
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
    if !matches!(tenant.membership().role.as_str(), "owner" | "administrator") {
        return Err(OperationError::Forbidden);
    }
    lock_row(tenant.transaction(), "medications", found.id).await?;
    let found = visible_medication(tenant, id)
        .await?
        .ok_or(OperationError::NotFound)?;
    if let Some(etag) = etag.filter(|v| !v.is_empty())
        && snapshot(tenant.transaction(), found.clone()).await?.etag != etag
    {
        return Err(conflict());
    }
    Ok(found)
}

async fn may_create(tenant: &TenantTransaction) -> Result<bool, OperationError> {
    if matches!(tenant.membership().role.as_str(), "owner" | "administrator") {
        return Ok(true);
    }
    grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(tenant.membership().id))
        .filter(grant::Column::AccessLevel.eq("manage"))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(
                    Expr::col(grant::Column::ExpiresAt)
                        .gt(Expr::cust("timezone('UTC', clock_timestamp())")),
                ),
        )
        .one(tenant.transaction())
        .await
        .map(|row| row.is_some())
        .map_err(database_error)
}

pub async fn can_create(tenant: &TenantTransaction) -> Result<bool, OperationError> {
    access::recheck(tenant).await?;
    may_create(tenant).await
}

async fn validate(
    tenant: &TenantTransaction,
    attributes: &Value,
    existing: Option<&medication::Model>,
) -> Result<(), OperationError> {
    if !validation::valid_location(
        tenant.transaction(),
        tenant.scope().household_id,
        attributes,
    )
    .await?
    {
        return Err(OperationError::NotFound);
    }
    validation::validate_attributes(attributes, existing).map_err(field_error)?;
    if validation::barcode_conflict(tenant.transaction(), attributes, existing.map(|v| v.id))
        .await?
    {
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
