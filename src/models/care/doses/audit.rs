use super::*;

pub(super) struct StockVersionChange<'a> {
    pub(super) item_type: &'a str,
    pub(super) item_id: i64,
    pub(super) previous: Decimal,
    pub(super) current: Decimal,
    pub(super) event: &'a str,
}

fn domain_audit_context(context: &DoseContext<'_>, request_id: &str) -> Value {
    let mut value = json!({"actor_account_id": context.scope().actor.account_id, "actor_user_id": context.tenant.user_id(),
        "actor_membership_id": context.membership().id, "household_id": context.membership().household_id,
        "active_role": context.membership().role, "permissions_version": context.membership().permissions_version,
        "policy_class": "MedicationTakePolicy", "policy_query": "create?", "request_id": request_id});
    if let Some(provenance) = context.provenance {
        value["authentication_method"] = json!(provenance.method.label());
        value["session_reference"] = json!(format!(
            "{}:{}",
            provenance.method.reference_prefix(),
            provenance.reference
        ));
    }
    value
}

pub(super) async fn stock_version(
    db: &DatabaseTransaction,
    context: &DoseContext<'_>,
    request_id: &str,
    change: StockVersionChange<'_>,
) -> Result<(), ApiError> {
    crate::models::entities::version::ActiveModel {
        item_type: Set(change.item_type.to_owned()), item_id: Set(change.item_id), event: Set(change.event.to_owned()),
        object: Set(Some(json!({"current_supply": decimal_string(change.previous.to_string())}).to_string())),
        object_changes: Set(Some(json!({"current_supply": [decimal_string(change.previous.to_string()), decimal_string(change.current.to_string())]}).to_string())),
        whodunnit: Set(Some(context.tenant.user_id().to_string())), request_id: Set(Some(request_id.to_owned())),
        household_id: Set(Some(context.membership().household_id)), actor_membership_id: Set(Some(context.membership().id)),
        audit_context: Set(domain_audit_context(context, request_id)),
        created_at: Set(Some(Utc::now().naive_utc())), ..Default::default()
    }.insert(db).await.map_err(database_error)?;
    Ok(())
}

pub(super) async fn record_domain_audit(
    db: &DatabaseTransaction,
    context: &DoseContext<'_>,
    request_id: &str,
    take: &medication_take::Model,
    proposed: &ProposedTake,
) -> Result<(), ApiError> {
    let now = Utc::now().naive_utc();
    let mut context_value = domain_audit_context(context, request_id);
    context_value["person_id"] = json!(proposed.source.person_id);
    context_value["medication_id"] = json!(proposed.source.medication_id);
    let snapshot = json!({
        "id": take.id, "portable_id": take.portable_id, "client_uuid": take.client_uuid,
        "household_id": take.household_id, "schedule_id": take.schedule_id,
        "person_medication_id": take.person_medication_id,
        "person_id": proposed.source.person_id, "medication_id": proposed.source.medication_id,
        "taken_from_medication_id": take.taken_from_medication_id,
        "taken_from_location_id": take.taken_from_location_id,
        "dose_amount": take.dose_amount.map(|value| decimal_string(value.to_string())),
        "dose_unit": take.dose_unit,
        "taken_at": take.taken_at.map(|value| value.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
        "created_at": take.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        "updated_at": take.updated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    });
    let changes: serde_json::Map<String, Value> = snapshot
        .as_object()
        .ok_or(OperationError::Unavailable)?
        .iter()
        .map(|(key, value)| (key.clone(), json!([null, value])))
        .collect();
    crate::models::entities::version::ActiveModel {
        item_type: Set("MedicationTake".to_owned()),
        item_id: Set(take.id),
        event: Set("create".to_owned()),
        object: Set(Some(snapshot.to_string())),
        object_changes: Set(Some(Value::Object(changes).to_string())),
        whodunnit: Set(Some(context.tenant.user_id().to_string())),
        request_id: Set(Some(request_id.to_owned())),
        household_id: Set(Some(context.membership().household_id)),
        actor_membership_id: Set(Some(context.membership().id)),
        audit_context: Set(context_value),
        created_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    let person = crate::models::entities::person::Entity::find_by_id(proposed.source.person_id)
        .filter(person::Column::HouseholdId.eq(context.scope().household_id))
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or(OperationError::NotFound)?;
    record_change(
        db,
        context,
        request_id,
        SyncRecord {
            record_type: "MedicationTake",
            record_id: take.id,
            portable_id: &take.portable_id,
            action: "create",
            person_portable_id: Some(&person.portable_id),
        },
    )
    .await?;
    Ok(())
}
