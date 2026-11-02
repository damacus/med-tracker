use super::*;
use crate::models::identity::credentials::CredentialProvenance;
use loco_rs::mailer::Email;
use loco_rs::model::TenantQueryExt;

pub async fn create(
    tenant: &TenantTransaction,
    input: InvitationInput,
    acceptance_url: Option<&url::Url>,
    provenance: Option<&CredentialProvenance>,
) -> Result<Value, OperationError> {
    crate::models::care::sync::lock(tenant).await?;
    administration::authorize(tenant).await?;
    create_authorized(tenant, input, acceptance_url, provenance).await
}

async fn create_authorized(
    tenant: &TenantTransaction,
    input: InvitationInput,
    acceptance_url: Option<&url::Url>,
    provenance: Option<&CredentialProvenance>,
) -> Result<Value, OperationError> {
    let (email, role, dependent_sharing) = input.validated()?;
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
        dependent_sharing: Set(dependent_sharing),
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
        delivery::enqueue(tenant, &row, Email { to: email, subject: "MedTracker household invitation".into(), text: format!("You have been invited to a MedTracker household. Accept your invitation: {target}"), ..Default::default() }).await?;
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

pub(crate) async fn create_for_parent(
    tenant: &TenantTransaction,
    person_id: i64,
    email: &str,
    acceptance_url: &url::Url,
    provenance: Option<&CredentialProvenance>,
) -> Result<Value, OperationError> {
    use sea_orm::QuerySelect;
    crate::models::care::sync::lock(tenant).await?;
    crate::models::care::person_carers::selected(tenant, person_id).await?;
    let email = input::validate_email(email).map_err(OperationError::from)?;
    let pending = household_invitation::Entity::find()
        .in_tenant(tenant.scope().household_id)
        .filter(household_invitation::Column::Email.eq(&email))
        .filter(household_invitation::Column::AcceptedAt.is_null())
        .filter(household_invitation::Column::RevokedAt.is_null())
        .lock_exclusive()
        .one(tenant.transaction())
        .await?;
    if let Some(row) = pending {
        let now = Utc::now().naive_utc();
        if row.membership_role != "member" || row.expires_at <= now {
            return Err(invalid(
                "invitation",
                "has an incompatible or expired invitation",
            ));
        }
        let grants = household_invitation_grant::Entity::find()
            .in_tenant(tenant.scope().household_id)
            .filter(household_invitation_grant::Column::HouseholdInvitationId.eq(row.id))
            .lock_exclusive()
            .all(tenant.transaction())
            .await?;
        if grants.iter().any(|grant| {
            grant.relationship_type != "parent"
                || grant.access_level != "manage"
                || grant.expires_at.is_some()
        }) {
            return Err(invalid(
                "invitation",
                "has an incompatible invitation grant",
            ));
        }
        if !grants.iter().any(|grant| grant.person_id == person_id) {
            let grant = household_invitation_grant::ActiveModel {
                household_id: Set(tenant.scope().household_id),
                household_invitation_id: Set(row.id),
                person_id: Set(person_id),
                access_level: Set("manage".into()),
                relationship_type: Set("parent".into()),
                expires_at: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(tenant.transaction())
            .await?;
            let state = json!({"household_id":grant.household_id,"household_invitation_id":grant.household_invitation_id,
                "person_id":grant.person_id,"access_level":grant.access_level,"relationship_type":grant.relationship_type,"expires_at":grant.expires_at});
            administration::persistence::record_version_as(
                tenant,
                "HouseholdInvitationGrant",
                grant.id,
                "create",
                None,
                state,
                provenance,
            )
            .await?;
            administration::persistence::event(tenant,"household_access.parent_invitation_attached",
                json!({"target_invitation_id":row.id,"target_grant_id":grant.id,"person_id":person_id,"access_level":"manage","relationship_type":"parent","outcome":"success"}),provenance).await?;
        }
        let inviter =
            crate::models::entities::membership::Entity::find_by_id(row.invited_by_membership_id)
                .filter(
                    crate::models::entities::membership::Column::HouseholdId
                        .eq(tenant.scope().household_id),
                )
                .lock_exclusive()
                .one(tenant.transaction())
                .await?
                .ok_or_else(|| invalid("invitation", "has unavailable inviter authority"))?;
        super::authority::authorize(tenant.transaction(), &inviter, &row)
            .await
            .map_err(|error| match error {
                OperationError::Unavailable => error,
                _ => invalid("invitation", "has unavailable inviter authority"),
            })?;
        return Ok(json!({"data":summary(&row,now)}));
    }
    create_authorized(
        tenant,
        InvitationInput::from_form(&email, "member", true),
        Some(acceptance_url),
        provenance,
    )
    .await
}
