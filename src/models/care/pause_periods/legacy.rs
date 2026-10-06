use super::*;

async fn source_path(
    tenant: &TenantTransaction,
    kind: Kind,
    id: &str,
) -> Result<Source, OperationError> {
    let portable = if let Ok(id) = id.parse::<i64>() {
        match kind {
            Kind::Schedule => schedule::Entity::find_by_id(id)
                .filter(schedule::Column::HouseholdId.eq(tenant.scope().household_id))
                .one(tenant.transaction())
                .await?
                .map(|row| row.portable_id),
            Kind::Assignment => person_medication::Entity::find_by_id(id)
                .filter(person_medication::Column::HouseholdId.eq(tenant.scope().household_id))
                .one(tenant.transaction())
                .await?
                .map(|row| row.portable_id),
        }
        .ok_or(OperationError::NotFound)?
    } else {
        Uuid::parse_str(id).map_err(|_| OperationError::NotFound)?;
        id.to_owned()
    };
    find_source(tenant, kind, &portable, false).await
}
async fn change(
    tenant: &TenantTransaction,
    id: &str,
    body: &Value,
    provenance: Option<&CredentialProvenance>,
    kind: Kind,
    pause: bool,
) -> Result<(Value, String), OperationError> {
    lock(tenant).await?;
    let source = source_path(tenant, kind, id).await?;
    access::require_person_access(tenant, source.person_id(), PersonAccess::Manage).await?;
    if body.as_object().is_none_or(|body| !body.is_empty()) {
        return Err(invalid("body", "must be empty"));
    }
    let context = AuthContext { tenant, provenance };
    let source = if pause {
        persistence::pause_source(
            tenant.transaction(),
            &context,
            source,
            "reason_not_recorded",
            None,
            &tenant.scope().request_id,
        )
        .await?
        .0
    } else {
        persistence::close_period(
            tenant.transaction(),
            &context,
            source,
            None,
            &tenant.scope().request_id,
        )
        .await?
        .0
    };
    match source {
        Source::Schedule(row) => {
            crate::models::care::treatments::lifecycle::read(tenant, &row.id.to_string()).await
        }
        Source::Assignment(row) => {
            crate::models::care::assignments::read(tenant, &row.id.to_string()).await
        }
    }
}
pub async fn pause_schedule(
    tenant: &TenantTransaction,
    id: &str,
    body: &Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    change(tenant, id, body, provenance, Kind::Schedule, true).await
}
pub async fn resume_schedule(
    tenant: &TenantTransaction,
    id: &str,
    body: &Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    change(tenant, id, body, provenance, Kind::Schedule, false).await
}
pub async fn pause_assignment(
    tenant: &TenantTransaction,
    id: &str,
    body: &Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    change(tenant, id, body, provenance, Kind::Assignment, true).await
}
pub async fn resume_assignment(
    tenant: &TenantTransaction,
    id: &str,
    body: &Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<(Value, String), OperationError> {
    change(tenant, id, body, provenance, Kind::Assignment, false).await
}
