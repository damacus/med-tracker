use super::*;

pub async fn create(
    tenant: &TenantTransaction,
    body: Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    authorize(tenant).await?;
    let attrs = attributes(&body, true).ok_or_else(invalid)?;
    let parent = parent(
        tenant,
        attrs.medication_id.as_deref().ok_or_else(invalid)?,
        true,
    )
    .await?;
    access::recheck(tenant).await?;
    let now = Utc::now().naive_utc();
    let active = dosage::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        medication_id: Set(parent.id),
        portable_id: Set(Uuid::new_v4().to_string()),
        amount: Set(attrs.amount.ok_or_else(invalid)?),
        unit: Set(attrs.unit.ok_or_else(invalid)?),
        frequency: Set(attrs.frequency.ok_or_else(invalid)?),
        description: Set(attrs.description),
        default_for_adults: Set(attrs.default_for_adults.unwrap_or(false)),
        default_for_children: Set(attrs.default_for_children.unwrap_or(false)),
        default_max_daily_doses: Set(attrs.default_max_daily_doses.ok_or_else(invalid)?),
        default_min_hours_between_doses: Set(attrs
            .default_min_hours_between_doses
            .ok_or_else(invalid)?),
        default_dose_cycle: Set(attrs.default_dose_cycle.ok_or_else(invalid)?),
        current_supply: Set(attrs.current_supply.unwrap_or(None)),
        reorder_threshold: Set(attrs.reorder_threshold.unwrap_or(None)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    let savepoint = tenant.transaction().begin().await?;
    let result = async {
        let row = active.insert(&savepoint).await.map_err(write_error)?;
        let before = super::super::medications::medication_snapshot(&parent);
        let tracked = row.current_supply.is_some();
        let parent = if tracked {
            inventory::synchronize_inventory(&savepoint, parent, now, true)
                .await?
                .ok_or_else(invalid)?
        } else {
            let mut active = parent.into_active_model();
            active.dose_amount = Set(None);
            active.updated_at = Set(now);
            active.update(&savepoint).await?
        };
        persistence::persist(tenant, &row, None, provenance).await?;
        if tracked {
            administration::persistence::record_version_as(
                tenant,
                "Medication",
                parent.id,
                "api_update",
                Some(before),
                super::super::medications::medication_snapshot(&parent),
                provenance,
            )
            .await?;
        }
        persistence::sync(
            tenant,
            "Medication",
            parent.id,
            &parent.portable_id,
            "update",
            None,
        )
        .await?;
        reading::representation(tenant, &row).await
    }
    .await;
    match result {
        Ok(result) => {
            savepoint.commit().await?;
            Ok(result)
        }
        Err(error) => {
            savepoint.rollback().await?;
            Err(error)
        }
    }
}

pub async fn update(
    tenant: &TenantTransaction,
    id: &str,
    body: Value,
    etag: Option<&str>,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    authorize(tenant).await?;
    let found = record(tenant, id, false).await?;
    parent(tenant, &found.medication_id.to_string(), true).await?;
    access::recheck(tenant).await?;
    let row = record(tenant, id, true).await?;
    if let Some(etag) = etag
        && reading::representation(tenant, &row).await?.1 != etag
    {
        return Err(OperationError::Conflict {
            code: "conflict".into(),
            details: json!({}),
        });
    }
    let attrs = attributes(&body, false).ok_or_else(invalid)?;
    let supply = attrs.current_supply.unwrap_or(row.current_supply);
    let inventory_changed = supply.is_some() || supply != row.current_supply;
    let before = persistence::dosage_snapshot(&row);
    let mut active = row.into_active_model();
    if let Some(value) = attrs.amount {
        active.amount = Set(value);
    }
    if let Some(value) = attrs.unit {
        active.unit = Set(value);
    }
    if let Some(value) = attrs.frequency {
        active.frequency = Set(value);
    }
    if let Some(value) = attrs.description {
        active.description = Set(Some(value));
    }
    if let Some(value) = attrs.default_for_adults {
        active.default_for_adults = Set(value);
    }
    if let Some(value) = attrs.default_for_children {
        active.default_for_children = Set(value);
    }
    if let Some(value) = attrs.default_max_daily_doses {
        active.default_max_daily_doses = Set(value);
    }
    if let Some(value) = attrs.default_min_hours_between_doses {
        active.default_min_hours_between_doses = Set(value);
    }
    if let Some(value) = attrs.default_dose_cycle {
        active.default_dose_cycle = Set(value);
    }
    if let Some(value) = attrs.current_supply {
        active.current_supply = Set(value);
    }
    if let Some(value) = attrs.reorder_threshold {
        active.reorder_threshold = Set(value);
    }
    let now = Utc::now().naive_utc();
    active.updated_at = Set(now);
    let savepoint = tenant.transaction().begin().await?;
    let result = async {
        let row = active.update(&savepoint).await.map_err(write_error)?;
        if !valid_persisted_dosage(&row) {
            return Err(invalid());
        }
        let parent = if inventory_changed {
            let parent = parent(tenant, &row.medication_id.to_string(), false).await?;
            let before = super::super::medications::medication_snapshot(&parent);
            let parent = inventory::synchronize_inventory(&savepoint, parent, now, false)
                .await?
                .ok_or_else(invalid)?;
            Some((before, parent))
        } else {
            None
        };
        persistence::persist(tenant, &row, Some(before), provenance).await?;
        if let Some((before, parent)) = parent {
            administration::persistence::record_version_as(
                tenant,
                "Medication",
                parent.id,
                "api_update",
                Some(before),
                super::super::medications::medication_snapshot(&parent),
                provenance,
            )
            .await?;
            persistence::sync(
                tenant,
                "Medication",
                parent.id,
                &parent.portable_id,
                "update",
                None,
            )
            .await?;
        }
        reading::representation(tenant, &row).await
    }
    .await;
    match result {
        Ok(result) => {
            savepoint.commit().await?;
            Ok(result)
        }
        Err(error) => {
            savepoint.rollback().await?;
            Err(error)
        }
    }
}

fn write_error(error: sea_orm::DbErr) -> OperationError {
    if matches!(
        error.sql_err(),
        Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
    ) {
        invalid()
    } else {
        database_error(error)
    }
}
