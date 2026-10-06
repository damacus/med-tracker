use super::*;
use crate::models::care::doses::CredentialProvenance;
use loco_rs::mailer::Email;
use sea_orm::QuerySelect;

async fn current(
    tenant: &TenantTransaction,
    id: i64,
) -> Result<household_invitation::Model, OperationError> {
    administration::authorize(tenant).await?;
    household_invitation::Entity::find_by_id(id)
        .filter(household_invitation::Column::HouseholdId.eq(tenant.scope().household_id))
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)
}

pub async fn cancel(
    tenant: &TenantTransaction,
    id: i64,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    let row = current(tenant, id).await?;
    if row.accepted_at.is_some() || row.revoked_at.is_some() {
        return Err(invalid("base", "Invitation cannot be cancelled"));
    }
    administration::persistence::record_version_as(
        tenant,
        "HouseholdInvitation",
        row.id,
        "destroy",
        Some(super::issuing::state(&row)),
        json!({}),
        provenance,
    )
    .await?;
    household_invitation_grant::Entity::delete_many()
        .filter(household_invitation_grant::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(household_invitation_grant::Column::HouseholdInvitationId.eq(row.id))
        .exec(tenant.transaction())
        .await?;
    household_invitation::Entity::delete_many()
        .filter(household_invitation::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(household_invitation::Column::Id.eq(row.id))
        .exec(tenant.transaction())
        .await?;
    Ok(())
}

pub async fn revoke(
    tenant: &TenantTransaction,
    id: i64,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    let row = current(tenant, id).await?;
    let before = super::issuing::state(&row);
    let now = Utc::now().naive_utc();
    let mut active: household_invitation::ActiveModel = row.into();
    active.revoked_at = Set(Some(now));
    active.updated_at = Set(now);
    let row = active.update(tenant.transaction()).await?;
    super::issuing::record(tenant, &row, Some(before), "revoked", provenance).await
}

pub async fn resend(
    tenant: &TenantTransaction,
    id: i64,
    acceptance_url: &url::Url,
    provenance: Option<&CredentialProvenance>,
) -> Result<Value, ResendError> {
    let row = current(tenant, id).await?;
    if row.accepted_at.is_some() || row.revoked_at.is_some() {
        return Err(invalid("base", "Invitation cannot be resent").into());
    }
    let before = super::issuing::state(&row);
    let (token, digest) = tokens::issue()?;
    let now = Utc::now().naive_utc();
    let mut active: household_invitation::ActiveModel = row.into();
    active.token_digest = Set(digest);
    active.expires_at = Set(now + chrono::Duration::days(7));
    active.updated_at = Set(now);
    let row = active.update(tenant.transaction()).await?;
    let mut target = acceptance_url.clone();
    target.query_pairs_mut().append_pair("token", &token);
    delivery::enqueue(
        tenant,
        Email {
            to: row.email.clone(),
            subject: "MedTracker household invitation".into(),
            text: format!(
                "You have been invited to a MedTracker household. Accept your invitation: {target}"
            ),
            ..Default::default()
        },
    )
    .await
    .map_err(|_| ResendError::DeliveryUnavailable)?;
    super::issuing::record(tenant, &row, Some(before), "resent", provenance).await?;
    Ok(
        json!({"data":{"invitation_id":row.id.to_string(),"expires_at":row.expires_at.and_utc().to_rfc3339(),"delivery_status":"queued"}}),
    )
}
