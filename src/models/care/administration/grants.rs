use super::*;
use loco_rs::model::TenantQueryExt;

pub(super) fn state(row: &grant::Model) -> Value {
    json!({"household_membership_id":row.household_membership_id,"person_id":row.person_id,"access_level":row.access_level,"relationship_type":row.relationship_type,"expires_at":row.expires_at.map(|time|time.and_utc().to_rfc3339()),"revoked_at":row.revoked_at.map(|time|time.and_utc().to_rfc3339()),"carer_relationship_id":row.carer_relationship_id})
}
async fn values(
    tenant: &TenantTransaction,
    rows: &[grant::Model],
) -> Result<Vec<Value>, OperationError> {
    let people: HashMap<i64, person::Model> =
        crate::models::access::person_queries::household_participants(tenant.scope().household_id)
            .filter(
                person::Column::Id.is_in(rows.iter().map(|row| row.person_id).collect::<Vec<_>>()),
            )
            .all(tenant.transaction())
            .await?
            .into_iter()
            .map(|row| (row.id, row))
            .collect();
    Ok(rows.iter().map(|row|json!({"id":row.id,"household_membership_id":row.household_membership_id,"person_id":row.person_id,"person_name":people.get(&row.person_id).map(|person| &person.name),"access_level":row.access_level,"relationship_type":row.relationship_type,"expires_at":row.expires_at.map(|time|time.and_utc().to_rfc3339()),"revoked_at":row.revoked_at.map(|time|time.and_utc().to_rfc3339())})).collect())
}
pub async fn list(tenant: &TenantTransaction) -> Result<Value, OperationError> {
    authorize(tenant).await?;
    let rows = grant::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .order_by_asc(grant::Column::Id)
        .all(tenant.transaction())
        .await?;
    Ok(json!({"data":values(tenant,&rows).await?}))
}
pub enum GrantCreation {
    Rejected,
}

pub async fn create(
    tenant: &TenantTransaction,
    _attributes: Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<GrantCreation, OperationError> {
    authorize(tenant).await?;
    rejected(tenant, None, json!({}), provenance).await?;
    Ok(GrantCreation::Rejected)
}
async fn rejected(
    tenant: &TenantTransaction,
    member_id: Option<i64>,
    attempted: Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    persistence::event(tenant,"household_access.person_grant_changed",json!({"target_membership_id":member_id,"target_grant_id":null,"previous_state":null,"new_state":attempted,"outcome":"rejected"}),provenance).await
}
pub async fn revoke(
    tenant: &TenantTransaction,
    id: i64,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    authorize(tenant).await?;
    let row = grant::Entity::find_by_id(id)
        .in_tenant(tenant.scope().household_id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    if row.carer_relationship_id.is_some() {
        record(tenant, &row, Some(state(&row)), "rejected", provenance).await?;
        return Err(invalid(
            "base",
            "Relationship-owned grants must be revoked through their carer relationship",
        ));
    }
    revoke_owned(tenant, row, provenance).await
}
pub(super) async fn revoke_owned(
    tenant: &TenantTransaction,
    row: grant::Model,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    let before = state(&row);
    let changed = row.revoked_at.is_none();
    let row = if changed {
        let mut active: grant::ActiveModel = row.clone().into();
        active.revoked_at = Set(Some(Utc::now().naive_utc()));
        active.updated_at = Set(Utc::now().naive_utc());
        let row = active.update(tenant.transaction()).await?;
        persistence::bump(tenant, row.household_membership_id).await?;
        row
    } else {
        row
    };
    record(
        tenant,
        &row,
        Some(before),
        if changed { "success" } else { "no_change" },
        provenance,
    )
    .await
}
pub(super) async fn record(
    tenant: &TenantTransaction,
    row: &grant::Model,
    before: Option<Value>,
    outcome: &str,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    persistence::event(tenant,"household_access.person_grant_changed",json!({"target_membership_id":row.household_membership_id,"target_grant_id":row.id,"previous_state":before,"new_state":state(row),"outcome":outcome}),provenance).await
}
