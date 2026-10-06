use super::*;

pub(super) async fn apply(
    context: &StockContext<'_>,
    medication: medication::Model,
    quantity: Decimal,
    mut payload: Value,
    dosage_id: Option<String>,
) -> Result<Value, OperationError> {
    let db = context.tenant.transaction();
    let (previous, remaining, unit) = if let Some(id) = dosage_id {
        let id = id.parse::<i64>().map_err(|_| invalid())?;
        db.query_one_raw(Statement::from_sql_and_values(DbBackend::Postgres,"SELECT id FROM dosages WHERE id=$1 AND household_id=$2 AND medication_id=$3 FOR UPDATE",[id.into(),context.tenant.scope().household_id.into(),medication.id.into()])).await.map_err(database_error)?.ok_or_else(invalid)?;
        let option = dosage::Entity::find_by_id(id)
            .filter(dosage::Column::HouseholdId.eq(context.tenant.scope().household_id))
            .filter(dosage::Column::MedicationId.eq(medication.id))
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(invalid)?;
        let previous = option.current_supply.ok_or_else(invalid)?;
        if previous < quantity {
            return Err(invalid());
        }
        let remaining = previous - quantity;
        let unit = option.unit.clone();
        let portable = option.portable_id.clone();
        let mut active: dosage::ActiveModel = option.into();
        active.current_supply = Set(Some(remaining));
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(db).await.map_err(database_error)?;
        record(
            context,
            "MedicationDosageOption",
            id,
            "update",
            Some(json!({"current_supply":format_quantity(previous)})),
            Some(json!({"current_supply":format_quantity(remaining)})),
        )
        .await?;
        change(context, "MedicationDosageOption", id, &portable).await?;
        let tracked = dosage::Entity::find()
            .filter(dosage::Column::HouseholdId.eq(context.tenant.scope().household_id))
            .filter(dosage::Column::MedicationId.eq(medication.id))
            .filter(dosage::Column::CurrentSupply.is_not_null())
            .all(db)
            .await
            .map_err(database_error)?;
        let total: Decimal = tracked.iter().filter_map(|v| v.current_supply).sum();
        let threshold: Decimal = tracked.iter().filter_map(|v| v.reorder_threshold).sum();
        if total >= Decimal::from(100_000_000) || threshold >= Decimal::from(100_000_000) {
            return Err(invalid());
        }
        let restock = medication
            .supply_at_last_restock
            .map_or(total, |v| v.max(total));
        let mut active: medication::ActiveModel = medication.clone().into();
        active.current_supply = Set(Some(total));
        active.reorder_threshold = Set(threshold);
        active.supply_at_last_restock = Set(Some(restock));
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(db).await.map_err(database_error)?;
        record(context,"Medication",medication.id,"update",Some(json!({"current_supply":medication.current_supply.map(format_quantity),"reorder_threshold":format_quantity(medication.reorder_threshold),"supply_at_last_restock":medication.supply_at_last_restock.map(format_quantity)})),Some(json!({"current_supply":format_quantity(total),"reorder_threshold":format_quantity(threshold),"supply_at_last_restock":format_quantity(restock)}))).await?;
        change(
            context,
            "Medication",
            medication.id,
            &medication.portable_id,
        )
        .await?;
        (previous, remaining, unit)
    } else {
        let tracked = dosage::Entity::find()
            .filter(dosage::Column::HouseholdId.eq(context.tenant.scope().household_id))
            .filter(dosage::Column::MedicationId.eq(medication.id))
            .filter(dosage::Column::CurrentSupply.is_not_null())
            .one(db)
            .await
            .map_err(database_error)?
            .is_some();
        if tracked {
            return Err(invalid());
        }
        let previous = medication.current_supply.ok_or_else(invalid)?;
        if previous < quantity {
            return Err(invalid());
        }
        let remaining = previous - quantity;
        let unit = medication.dose_unit.clone().unwrap_or_default();
        let mut active: medication::ActiveModel = medication.clone().into();
        active.current_supply = Set(Some(remaining));
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(db).await.map_err(database_error)?;
        record(
            context,
            "Medication",
            medication.id,
            "update",
            Some(json!({"current_supply":format_quantity(previous)})),
            Some(json!({"current_supply":format_quantity(remaining)})),
        )
        .await?;
        change(
            context,
            "Medication",
            medication.id,
            &medication.portable_id,
        )
        .await?;
        (previous, remaining, unit)
    };
    let unit = if [
        "tablet", "capsule", "gummy", "sachet", "spray", "drop", "pad", "ml",
    ]
    .contains(&unit.as_str())
    {
        unit
    } else {
        "units".into()
    };
    payload["previous_quantity"] = json!(format_quantity(previous));
    payload["remaining_quantity"] = json!(format_quantity(remaining));
    payload["unit"] = json!(unit);
    let event = record(
        context,
        "MedicationStockRemoval",
        medication.id,
        "stock_removal",
        Some(payload),
        None,
    )
    .await?;
    Ok(row(&event))
}

async fn record(
    context: &StockContext<'_>,
    item_type: &str,
    item_id: i64,
    event: &str,
    before: Option<Value>,
    after: Option<Value>,
) -> Result<version::Model, OperationError> {
    let tenant = context.tenant;
    let mut audit_context = json!({"actor_account_id":tenant.scope().actor.account_id,"actor_user_id":tenant.user_id(),"actor_membership_id":tenant.membership().id,"household_id":tenant.scope().household_id,"request_id":tenant.scope().request_id,"active_role":tenant.membership().role,"permissions_version":tenant.membership().permissions_version,"policy_class":"MedicationPolicy","policy_query":"update?"});
    if let Some(provenance) = context.provenance {
        let (method, prefix) = match provenance.method {
            CredentialMethod::ApiSession => ("api_session", "api_session"),
            CredentialMethod::ApiAppToken => ("api_app_token", "api_app_token"),
            CredentialMethod::OauthGrant => ("oauth", "oauth_grant"),
            CredentialMethod::BrowserSession => ("browser_session", "browser_session"),
        };
        audit_context["authentication_method"] = json!(method);
        audit_context["session_reference"] = json!(format!("{prefix}:{}", provenance.reference));
    }
    let changes = after.as_ref().map(|new| {
        let mut changes = serde_json::Map::new();
        if let Some(fields) = new.as_object() {
            for (key, value) in fields {
                let old = before
                    .as_ref()
                    .and_then(|v| v.get(key))
                    .cloned()
                    .unwrap_or(Value::Null);
                if old != *value {
                    changes.insert(key.clone(), json!([old, value]));
                }
            }
        }
        Value::Object(changes).to_string()
    });
    version::ActiveModel {
        item_type: Set(item_type.into()),
        item_id: Set(item_id),
        event: Set(event.into()),
        object: Set(before.map(|v| v.to_string())),
        object_changes: Set(changes),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        household_id: Set(Some(tenant.scope().household_id)),
        actor_membership_id: Set(Some(tenant.membership().id)),
        whodunnit: Set(Some(tenant.user_id().to_string())),
        audit_context: Set(audit_context),
        created_at: Set(Some(Utc::now().naive_utc())),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await
    .map_err(database_error)
}

async fn change(
    context: &StockContext<'_>,
    kind: &str,
    id: i64,
    portable: &str,
) -> Result<(), OperationError> {
    let tenant = context.tenant;
    let now = Utc::now().naive_utc();
    api_change_event::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        household_membership_id: Set(Some(tenant.membership().id)),
        account_id: Set(Some(tenant.scope().actor.account_id)),
        action: Set("update".into()),
        record_type: Set(kind.into()),
        record_id: Set(id),
        record_portable_id: Set(Some(portable.into())),
        request_id: Set(Some(tenant.scope().request_id.clone())),
        metadata: Set(json!({"record_type":kind,"record_id":id,"portable_id":portable})),
        occurred_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await
    .map_err(database_error)?;
    Ok(())
}
