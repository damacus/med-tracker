use super::*;
fn source_identity(body: &Value) -> Result<(Kind, &str), OperationError> {
    let attrs = body
        .get("medication_pause_period")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("medication_pause_period", "is invalid"))?;
    let kind = attrs
        .get("source_type")
        .and_then(Value::as_str)
        .and_then(Kind::parse)
        .ok_or_else(|| invalid("source_type", "is invalid"))?;
    let id = attrs
        .get("source_id")
        .and_then(Value::as_str)
        .filter(|id| Uuid::parse_str(id).is_ok())
        .ok_or_else(|| invalid("source_id", "must be a portable identifier"))?;
    Ok((kind, id))
}
pub async fn authorize_create(
    tenant: &TenantTransaction,
    body: &Value,
) -> Result<(), OperationError> {
    lock(tenant).await?;
    let (kind, id) = source_identity(body)?;
    let source = find_source(tenant, kind, id, false).await?;
    access::require_person_access(tenant, source.person_id(), PersonAccess::Manage).await
}
async fn period(
    tenant: &TenantTransaction,
    id: &str,
) -> Result<pause_period::Model, OperationError> {
    if Uuid::parse_str(id).is_err() {
        return Err(OperationError::NotFound);
    }
    pause_period::Entity::find()
        .filter(pause_period::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(pause_period::Column::PortableId.eq(id))
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)
}
pub async fn authorize_resume(tenant: &TenantTransaction, id: &str) -> Result<(), OperationError> {
    lock(tenant).await?;
    let row = period(tenant, id).await?;
    let source = source_for_period(tenant, &row).await?;
    access::require_person_access(tenant, source.person_id(), PersonAccess::Manage).await
}
pub async fn create(
    tenant: &TenantTransaction,
    body: &Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    authorize_create(tenant, body).await?;
    let (kind, id) = source_identity(body)?;
    let outer = body
        .as_object()
        .ok_or_else(|| invalid("medication_pause_period", "is invalid"))?;
    let attrs = outer
        .get("medication_pause_period")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("medication_pause_period", "is invalid"))?;
    if outer.len() != 1
        || attrs.keys().any(|key| {
            !matches!(
                key.as_str(),
                "source_type" | "source_id" | "reason" | "note"
            )
        })
    {
        return Err(invalid(
            "medication_pause_period",
            "contains an unsupported field",
        ));
    }
    let reason = attrs
        .get("reason")
        .and_then(Value::as_str)
        .filter(|reason| {
            matches!(
                *reason,
                "out_of_supply"
                    | "temporarily_not_needed"
                    | "clinician_advice"
                    | "side_effects"
                    | "other"
            )
        })
        .ok_or_else(|| invalid("reason", "is invalid"))?;
    let note = match attrs.get("note") {
        None | Some(Value::Null) => None,
        Some(Value::String(note)) => Some(note.clone()),
        _ => return Err(invalid("note", "must be a string")),
    };
    let source = find_source(tenant, kind, id, false).await?;
    let context = AuthContext { tenant, provenance };
    let (source, row) = persistence::pause_source(
        tenant.transaction(),
        &context,
        source,
        reason,
        note,
        &tenant.scope().request_id,
    )
    .await?;
    reading::project(tenant, &row, &source).await
}
pub async fn resume(
    tenant: &TenantTransaction,
    id: &str,
    body: &Value,
    etag: Option<&str>,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    authorize_resume(tenant, id).await?;
    if body.as_object().is_none_or(|value| !value.is_empty()) {
        return Err(invalid("body", "must be empty"));
    }
    let row = period(tenant, id).await?;
    let source = source_for_period(tenant, &row).await?;
    let before = reading::project(tenant, &row, &source).await?;
    if etag.is_some_and(|etag| !etag.is_empty() && etag != before.1) {
        return Err(OperationError::Conflict {
            code: "conflict".into(),
            details: json!({"error":"Record has changed since it was last read"}),
        });
    }
    let context = AuthContext { tenant, provenance };
    let (source, row) = persistence::close_period(
        tenant.transaction(),
        &context,
        source,
        Some(row),
        &tenant.scope().request_id,
    )
    .await?;
    reading::project(tenant, &row.ok_or(OperationError::Unavailable)?, &source).await
}
