use super::*;
use crate::models::entities::version;
use sea_orm::PaginatorTrait;

fn invalid() -> OperationError {
    OperationError::Validation {
        details: json!({"code":"unprocessable_content","message":"Medication attributes are invalid"}),
    }
}
pub(super) async fn apply(
    tenant: &TenantTransaction,
    operation: &Operation,
    provenance: &CredentialProvenance,
) -> Result<Value, OperationError> {
    let mut attributes = Value::Object(operation.attributes.clone());
    if let Some(Value::String(id)) = attributes.get("location_id") {
        let row = locations::read(tenant, id).await?;
        attributes["location_id"] = json!(row.id);
    }
    if operation.action == "create" {
        let row = medications::crud::create(tenant, attributes, Some(provenance)).await?;
        let snapshot = medications::read_stock_snapshot(tenant, &row.id.to_string()).await?;
        return Ok(result(
            "Medication",
            row.id,
            Some(&row.portable_id),
            Some(snapshot.etag),
            None,
        ));
    }
    let id = operation.id.as_deref().ok_or_else(invalid)?;
    let snapshot = medications::read_stock_snapshot(tenant, id).await?;
    if !matches!(
        operation.action.as_str(),
        "mark_as_ordered" | "mark_as_received"
    ) && !access::can_manage_household(tenant)
    {
        return Err(OperationError::Forbidden);
    }
    if operation.action != "remove_stock" {
        required_etag(operation, &snapshot.etag)?;
    }
    let row = snapshot.medication;
    let mut replayed = None;
    match operation.action.as_str() {
        "delete" => {
            if !operation.attributes.is_empty() {
                return Err(invalid());
            }
            medications::crud::retire(tenant, id, operation.if_match.as_deref(), Some(provenance))
                .await?;
            return Ok(result(
                "Medication",
                row.id,
                Some(&row.portable_id),
                None,
                None,
            ));
        }
        "update" => {
            medications::crud::update(
                tenant,
                id,
                attributes,
                operation.if_match.as_deref(),
                Some(provenance),
            )
            .await?;
        }
        "adjust_inventory" => {
            if operation
                .attributes
                .keys()
                .any(|key| !matches!(key.as_str(), "new_quantity" | "reason"))
            {
                return Err(invalid());
            }
            let quantity = operation
                .attributes
                .get("new_quantity")
                .and_then(Value::as_str)
                .ok_or_else(invalid)?;
            medications::execute_with_options(
                tenant,
                medications::Command::AdjustStock(medications::AdjustStock {
                    medication_id: id.into(),
                    new_quantity: quantity.into(),
                    reason: operation
                        .attributes
                        .get("reason")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                }),
                None,
                Some(provenance),
            )
            .await
            .map_err(|error| match error {
                OperationError::Validation { .. } => invalid(),
                _ => error,
            })?;
        }
        "mark_as_ordered" => {
            orders::mark_as_ordered(
                tenant,
                id,
                &json!({"order_details":attributes}),
                Some(provenance),
            )
            .await?;
        }
        "mark_as_received" => {
            if !operation.attributes.is_empty() {
                return Err(invalid());
            }
            orders::mark_as_received(tenant, id, &json!({}), Some(provenance)).await?;
        }
        "remove_stock" => {
            if operation.attributes.keys().any(|key| {
                !matches!(
                    key.as_str(),
                    "quantity" | "reason" | "note" | "dosage_id" | "submission_id"
                )
            }) {
                return Err(invalid());
            }
            if let Some(Value::String(id)) = attributes.get("dosage_id")
                && id.parse::<i64>().is_err()
            {
                let option = dosage::Entity::find()
                    .filter(dosage::Column::HouseholdId.eq(tenant.scope().household_id))
                    .filter(dosage::Column::PortableId.eq(id))
                    .one(tenant.transaction())
                    .await?
                    .ok_or(OperationError::NotFound)?;
                attributes["dosage_id"] = json!(option.id.to_string());
            }
            let query = version::Entity::find()
                .filter(version::Column::HouseholdId.eq(tenant.scope().household_id))
                .filter(version::Column::ItemType.eq("MedicationStockRemoval"))
                .filter(version::Column::ItemId.eq(row.id))
                .filter(version::Column::Event.eq("stock_removal"));
            let before = query.clone().count(tenant.transaction()).await?;
            let text = |key: &str| {
                attributes
                    .get(key)
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            };
            medications::stock_removals::create(
                tenant,
                medications::stock_removals::RemoveStock {
                    medication_id: id.into(),
                    quantity: text("quantity").ok_or_else(invalid)?,
                    reason: text("reason").ok_or_else(invalid)?,
                    note: text("note"),
                    dosage_id: text("dosage_id"),
                    submission_id: text("submission_id").ok_or_else(invalid)?,
                },
                Some(provenance),
            )
            .await?;
            replayed = Some(before == query.count(tenant.transaction()).await?);
        }
        _ => return Err(invalid()),
    }
    let snapshot = medications::read_stock_snapshot(tenant, id).await?;
    Ok(result(
        "Medication",
        row.id,
        Some(&row.portable_id),
        Some(snapshot.etag),
        replayed,
    ))
}
