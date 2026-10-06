use super::*;

fn value(row: &household::Model) -> Value {
    json!({"id":row.id,"name":row.name,"slug":row.slug,"timezone":row.timezone,"subscription_plan":row.subscription_plan,"updated_at":row.updated_at.and_utc().to_rfc3339()})
}
pub async fn read(tenant: &TenantTransaction) -> Result<Value, OperationError> {
    authorize(tenant).await?;
    let row = household::Entity::find_by_id(tenant.scope().household_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    Ok(json!({"data":value(&row)}))
}
pub async fn update(
    tenant: &TenantTransaction,
    attributes: Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<Value, OperationError> {
    authorize(tenant).await?;
    let fields = fields(
        &attributes,
        &["name", "timezone", "subscription_plan"],
        "household",
    )?;
    let row = household::Entity::find_by_id(tenant.scope().household_id)
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    let before = value(&row);
    let mut active: household::ActiveModel = row.into();
    for (key, value) in fields {
        let text = value
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| invalid(key, "is invalid"))?;
        match key.as_str() {
            "name" => active.name = Set(text.into()),
            "timezone" => active.timezone = Set(text.into()),
            "subscription_plan" => {
                if !matches!(text, "free" | "family_plus") {
                    return Err(invalid(key, "is invalid"));
                }
                active.subscription_plan = Set(text.into());
            }
            _ => return Err(invalid(key, "is invalid")),
        }
    }
    active.updated_at = Set(Utc::now().naive_utc());
    let row = active.update(tenant.transaction()).await?;
    let after = value(&row);
    persistence::record_version(
        tenant,
        "Household",
        row.id,
        Some(before),
        after.clone(),
        provenance,
    )
    .await?;
    persistence::event(
        tenant,
        "api/admin/household_settings/updated",
        json!({"target_type":"Household","target_id":row.id,"outcome":"success"}),
        provenance,
    )
    .await?;
    Ok(json!({"data":after}))
}
