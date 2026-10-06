use super::*;

async fn lock_household(tenant: &TenantTransaction) -> Result<(), OperationError> {
    household::Entity::find_by_id(tenant.scope().household_id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    access::recheck(tenant).await
}
async fn person(tenant: &TenantTransaction, id: &str) -> Result<person::Model, OperationError> {
    let query =
        person::Entity::find().filter(person::Column::HouseholdId.eq(tenant.scope().household_id));
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(person::Column::Id.eq(id))
    } else {
        query.filter(person::Column::PortableId.eq(id))
    };
    let row = query
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    if !access::can_access_person(tenant, row.id, PersonAccess::View).await? {
        return Err(OperationError::NotFound);
    }
    Ok(row)
}
async fn medicine(
    tenant: &TenantTransaction,
    id: &str,
) -> Result<medication::Model, OperationError> {
    let query = access::medication_scope(tenant);
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(medication::Column::Id.eq(id))
    } else {
        query.filter(medication::Column::PortableId.eq(id))
    };
    query
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)
}
async fn option(
    tenant: &TenantTransaction,
    id: Option<&str>,
) -> Result<Option<dosage::Model>, OperationError> {
    let Some(id) = id else { return Ok(None) };
    let query =
        dosage::Entity::find().filter(dosage::Column::HouseholdId.eq(tenant.scope().household_id));
    let query = if let Ok(id) = id.parse::<i64>() {
        query.filter(dosage::Column::Id.eq(id))
    } else {
        query.filter(dosage::Column::PortableId.eq(id))
    };
    let row = query
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    medicine(tenant, &row.medication_id.to_string()).await?;
    Ok(Some(row))
}
pub async fn authorize_create(
    tenant: &TenantTransaction,
    body: &Value,
) -> Result<(), OperationError> {
    lock_household(tenant).await?;
    let id = body
        .get("person_medication")
        .and_then(|value| value.get("person_id"))
        .and_then(Value::as_str)
        .filter(|value| valid_identifier(value))
        .ok_or_else(invalid)?;
    let row = person(tenant, id).await?;
    access::require_person_access(tenant, row.id, PersonAccess::Manage).await
}
pub async fn authorize_update(tenant: &TenantTransaction, id: &str) -> Result<(), OperationError> {
    lock_household(tenant).await?;
    let row = reading::find(tenant, id, true).await?;
    access::require_person_access(tenant, row.person_id, PersonAccess::Manage).await
}
pub async fn create(
    tenant: &TenantTransaction,
    body: &Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    authorize_create(tenant, body).await?;
    let attrs = Attributes::parse(body, true).map_err(|_| invalid())?;
    let person = person(tenant, attrs.person_id.as_deref().ok_or_else(invalid)?).await?;
    let medication = medicine(tenant, attrs.medication_id.as_deref().ok_or_else(invalid)?).await?;
    if person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(person_medication::Column::PersonId.eq(person.id))
        .filter(person_medication::Column::MedicationId.eq(medication.id))
        .filter(person_medication::Column::RetiredAt.is_null())
        .one(tenant.transaction())
        .await?
        .is_some()
    {
        return Err(invalid());
    }
    let selected = option(tenant, attrs.source_dosage_option_id.as_deref()).await?;
    let (amount, unit, source) = dosing::resolved_dose(
        tenant.transaction(),
        tenant,
        &attrs,
        &person,
        &medication,
        None,
        selected.as_ref(),
    )
    .await?
    .ok_or_else(invalid)?;
    let position = person_medication::Entity::find()
        .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(person_medication::Column::PersonId.eq(person.id))
        .order_by_desc(person_medication::Column::Position)
        .one(tenant.transaction())
        .await?
        .map_or(1, |row| row.position + 1);
    let now = Utc::now().naive_utc();
    let kind = attrs.administration_kind.unwrap_or(1);
    let hours = attrs.min_hours_between_doses.unwrap_or(None);
    let hours = if kind == 0 && attrs.max_daily_doses == Some(1) && hours == Some(24) {
        None
    } else {
        hours
    };
    let row = person_medication::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        portable_id: Set(Uuid::new_v4().to_string()),
        person_id: Set(person.id),
        medication_id: Set(medication.id),
        active: Set(true),
        retired_at: Set(None),
        administration_kind: Set(kind),
        dose_amount: Set(Some(amount)),
        dose_unit: Set(Some(unit)),
        source_dosage_option_id: Set(source),
        notes: Set(attrs.notes),
        max_daily_doses: Set(attrs.max_daily_doses),
        min_hours_between_doses: Set(hours),
        dose_cycle: Set(attrs.dose_cycle),
        position: Set(position),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    finish(tenant, row, None, "create", &person.portable_id, provenance).await
}
pub async fn update(
    tenant: &TenantTransaction,
    id: &str,
    body: &Value,
    etag: Option<&str>,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    authorize_update(tenant, id).await?;
    let row = reading::find(tenant, id, true).await?;
    let before = reading::project(tenant, row.clone()).await?;
    let clinical_before = clinical_snapshot(&row);
    if etag.is_some_and(|etag| !etag.is_empty() && etag != before.1) {
        return Err(OperationError::Conflict {
            code: "conflict".into(),
            details: json!({"error":"Record has changed since it was last read"}),
        });
    }
    let attrs = Attributes::parse(body, false).map_err(|_| invalid())?;
    if let Some(id) = attrs.person_id.as_deref()
        && person(tenant, id).await?.id != row.person_id
    {
        return Err(invalid());
    }
    let person = person(tenant, &row.person_id.to_string()).await?;
    let medication = medicine(
        tenant,
        attrs
            .medication_id
            .as_deref()
            .unwrap_or(&row.medication_id.to_string()),
    )
    .await?;
    if medication.id != row.medication_id
        && person_medication::Entity::find()
            .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(person_medication::Column::PersonId.eq(row.person_id))
            .filter(person_medication::Column::MedicationId.eq(medication.id))
            .filter(person_medication::Column::RetiredAt.is_null())
            .one(tenant.transaction())
            .await?
            .is_some()
    {
        return Err(invalid());
    }
    let selected = if attrs.source_dosage_option_id.is_some() {
        option(tenant, attrs.source_dosage_option_id.as_deref()).await?
    } else if let Some(id) = row.source_dosage_option_id {
        dosage::Entity::find_by_id(id)
            .filter(dosage::Column::HouseholdId.eq(tenant.scope().household_id))
            .one(tenant.transaction())
            .await?
    } else {
        None
    };
    let (amount, unit, source) = dosing::resolved_dose(
        tenant.transaction(),
        tenant,
        &attrs,
        &person,
        &medication,
        Some(&row),
        selected.as_ref(),
    )
    .await?
    .ok_or_else(invalid)?;
    let kind = attrs.administration_kind.unwrap_or(row.administration_kind);
    let maximum = attrs.max_daily_doses.or(row.max_daily_doses);
    let minimum = attrs
        .min_hours_between_doses
        .unwrap_or(row.min_hours_between_doses);
    let minimum = if kind == 0 && maximum == Some(1) && minimum == Some(24) {
        None
    } else {
        minimum
    };
    if medication.id == row.medication_id
        && row.dose_amount == Some(amount)
        && row.dose_unit.as_deref() == Some(unit.as_str())
        && row.source_dosage_option_id == source
        && row.administration_kind == kind
        && row.notes == attrs.notes.clone().or(row.notes.clone())
        && row.max_daily_doses == maximum
        && row.min_hours_between_doses == minimum
        && row.dose_cycle == attrs.dose_cycle.or(row.dose_cycle)
    {
        return Ok(before);
    }
    let mut active = row.into_active_model();
    active.medication_id = Set(medication.id);
    active.dose_amount = Set(Some(amount));
    active.dose_unit = Set(Some(unit));
    active.source_dosage_option_id = Set(source);
    if let Some(value) = attrs.notes {
        active.notes = Set(Some(value));
    }
    if let Some(value) = attrs.administration_kind {
        active.administration_kind = Set(value);
    }
    if let Some(value) = attrs.max_daily_doses {
        active.max_daily_doses = Set(Some(value));
    }
    if let Some(value) = attrs.min_hours_between_doses {
        active.min_hours_between_doses = Set(value);
    }
    if let Some(value) = attrs.dose_cycle {
        active.dose_cycle = Set(Some(value));
    }
    active.min_hours_between_doses = Set(minimum);
    active.updated_at = Set(Utc::now().naive_utc());
    let row = active.update(tenant.transaction()).await?;
    finish(
        tenant,
        row,
        Some(clinical_before),
        "update",
        &person.portable_id,
        provenance,
    )
    .await
}
async fn finish(
    tenant: &TenantTransaction,
    row: person_medication::Model,
    before: Option<Value>,
    action: &str,
    person_portable: &str,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    let result = reading::project(tenant, row.clone()).await?;
    administration::persistence::record_version_as(
        tenant,
        "PersonMedication",
        row.id,
        action,
        before,
        clinical_snapshot(&row),
        provenance,
    )
    .await?;
    let now = Utc::now().naive_utc();
    api_change_event::ActiveModel{household_id:Set(tenant.scope().household_id),household_membership_id:Set(Some(tenant.membership().id)),account_id:Set(Some(tenant.scope().actor.account_id)),action:Set(action.into()),record_type:Set("PersonMedication".into()),record_id:Set(row.id),record_portable_id:Set(Some(row.portable_id.clone())),request_id:Set(Some(tenant.scope().request_id.clone())),metadata:Set(json!({"record_type":"PersonMedication","record_id":row.id,"portable_id":row.portable_id,"person_portable_id":person_portable})),occurred_at:Set(now),created_at:Set(now),updated_at:Set(now),..Default::default()}.insert(tenant.transaction()).await?;
    Ok(result)
}

fn clinical_snapshot(row: &person_medication::Model) -> Value {
    json!({
        "id": row.id,
        "household_id": row.household_id,
        "portable_id": row.portable_id,
        "person_id": row.person_id,
        "medication_id": row.medication_id,
        "active": row.active,
        "retired_at": row.retired_at,
        "administration_kind": row.administration_kind,
        "dose_amount": row.dose_amount.map(|value| crate::models::care::medications::decimal_string(value.to_string())),
        "dose_unit": row.dose_unit,
        "source_dosage_option_id": row.source_dosage_option_id,
        "notes": row.notes,
        "max_daily_doses": row.max_daily_doses,
        "min_hours_between_doses": row.min_hours_between_doses,
        "dose_cycle": row.dose_cycle,
        "position": row.position,
        "created_at": row.created_at,
        "updated_at": row.updated_at,
    })
}
