use super::*;
use crate::models::care::doses::CredentialProvenance;
use loco_rs::mailer::Email;
use loco_rs::model::TenantQueryExt;

pub async fn create(
    tenant: &TenantTransaction,
    attributes: Value,
    acceptance_url: Option<&url::Url>,
    provenance: Option<&CredentialProvenance>,
) -> Result<Value, OperationError> {
    administration::authorize(tenant).await?;
    if attributes
        .get("dependent_ids")
        .is_some_and(|value| value.as_array().is_none_or(|ids| !ids.is_empty()))
    {
        return Err(OperationError::Forbidden);
    }
    create_authorized(tenant, attributes, acceptance_url, provenance).await
}

async fn create_authorized(
    tenant: &TenantTransaction,
    attributes: Value,
    acceptance_url: Option<&url::Url>,
    provenance: Option<&CredentialProvenance>,
) -> Result<Value, OperationError> {
    let (email, role) = input::attributes(&attributes)?;
    if household_invitation::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(household_invitation::Column::Email.eq(&email))
        .filter(household_invitation::Column::AcceptedAt.is_null())
        .filter(household_invitation::Column::RevokedAt.is_null())
        .one(tenant.transaction())
        .await?
        .is_some()
    {
        return Err(invalid("email", "has already been taken"));
    }
    let (token, digest) = tokens::issue()?;
    let now = Utc::now().naive_utc();
    let row = household_invitation::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        email: Set(email.clone()),
        membership_role: Set(role),
        token_digest: Set(digest),
        invited_by_membership_id: Set(tenant.membership().id),
        expires_at: Set(now + chrono::Duration::days(7)),
        accepted_at: Set(None),
        revoked_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;

    if let Some(acceptance_url) = acceptance_url {
        let mut target = acceptance_url.clone();
        target.query_pairs_mut().append_pair("token", &token);
        delivery::enqueue(tenant, Email { to: email, subject: "MedTracker household invitation".into(), text: format!("You have been invited to a MedTracker household. Accept your invitation: {target}"), ..Default::default() }).await?;
    }
    record(tenant, &row, None, "created", provenance).await?;
    Ok(json!({"data":summary(&row,now)}))
}

pub(super) fn state(row: &household_invitation::Model) -> Value {
    json!({"email":row.email,"membership_role":row.membership_role,"accepted_at":row.accepted_at,"revoked_at":row.revoked_at,"expires_at":row.expires_at,"invited_by_membership_id":row.invited_by_membership_id})
}

pub(super) async fn record(
    tenant: &TenantTransaction,
    row: &household_invitation::Model,
    before: Option<Value>,
    action: &str,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    administration::persistence::record_version_as(
        tenant,
        "HouseholdInvitation",
        row.id,
        if action == "resent" {
            "resend"
        } else if before.is_some() {
            "update"
        } else {
            "create"
        },
        before,
        state(row),
        provenance,
    )
    .await?;
    administration::persistence::event(
        tenant,
        &format!("api/admin/invitation/{action}"),
        json!({"target_type":"HouseholdInvitation","target_id":row.id,"outcome":"success"}),
        provenance,
    )
    .await
}
