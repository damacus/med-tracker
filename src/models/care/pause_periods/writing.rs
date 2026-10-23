use super::*;
use loco_rs::model::TenantQueryExt;
pub(super) fn source_identity(body: &Value) -> Result<(Kind, &str), OperationError> {
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
    attempt: PauseSourceAttempt,
) -> Result<PauseSource, OperationError> {
    lock(tenant).await?;
    let selector = attempt.into_result()?;
    let source = find_source(tenant, selector.kind.kind(), &selector.id, false).await?;
    access::require_person_access(tenant, source.person_id(), PersonAccess::Manage).await?;
    Ok(selector)
}
pub async fn authorize_source(
    tenant: &TenantTransaction,
    kind: &str,
    id: &str,
) -> Result<(), OperationError> {
    lock(tenant).await?;
    let kind = PauseSourceKind::parse(kind)?;
    Uuid::parse_str(id).map_err(|_| invalid("source_id", "must be a portable identifier"))?;
    let source = find_source(tenant, kind.kind(), id, false).await?;
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
        .in_tenant(tenant.scope().household_id)
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
    input: &PauseCreate,
    provenance: Option<&CredentialProvenance>,
) -> Result<PauseWritten, OperationError> {
    lock(tenant).await?;
    let source = find_source(tenant, input.source_kind.kind(), &input.source_id, false).await?;
    access::require_person_access(tenant, source.person_id(), PersonAccess::Manage).await?;
    let context = AuthContext { tenant, provenance };
    let (source, row) = persistence::pause_source(
        tenant.transaction(),
        &context,
        source,
        input.reason.name(),
        input.note.clone(),
        &tenant.scope().request_id,
    )
    .await?;
    let (body, etag) = reading::project(tenant, &row, &source).await?;
    Ok(PauseWritten { row, body, etag })
}
pub async fn resume(
    tenant: &TenantTransaction,
    input: &ResumeInput,
    etag: Option<&str>,
    provenance: Option<&CredentialProvenance>,
) -> Result<PauseWritten, OperationError> {
    authorize_resume(tenant, &input.id).await?;
    let row = period(tenant, &input.id).await?;
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
    let row = row.ok_or(OperationError::Unavailable)?;
    let (body, etag) = reading::project(tenant, &row, &source).await?;
    Ok(PauseWritten { row, body, etag })
}
