use super::*;
use crate::models::entities::{
    account, api_change_event, membership, person, security_audit_event, version,
};
use sea_orm::DatabaseTransaction;

pub(super) enum PersonOrigin {
    Existing,
    New { user_id: i64 },
}

pub(crate) async fn apply(
    db: &DatabaseTransaction,
    actor: &super::acceptance::AcceptanceActor,
    source: &person::Model,
    inviter: &membership::Model,
    invitation: household_invitation::Model,
    request_id: &str,
) -> Result<membership::Model, OperationError> {
    apply_to_person(
        db,
        actor,
        source.clone(),
        inviter,
        invitation,
        request_id,
        PersonOrigin::Existing,
    )
    .await
}

pub(super) async fn apply_to_person(
    db: &DatabaseTransaction,
    actor: &super::acceptance::AcceptanceActor,
    subject: person::Model,
    inviter: &membership::Model,
    invitation: household_invitation::Model,
    request_id: &str,
    origin: PersonOrigin,
) -> Result<membership::Model, OperationError> {
    let household_id = invitation.household_id;
    let owner = account::Entity::find_by_id(actor.account_id)
        .filter(account::Column::PersonId.eq(subject.id))
        .one(db)
        .await?
        .ok_or_else(super::acceptance::unavailable)?;
    if owner.person_id != actor.person_id
        || !crate::models::authorization::household_manager(inviter, household_id)
    {
        return Err(super::acceptance::unavailable());
    }
    let inviter_account = account::Entity::find()
        .filter(account::Column::PersonId.eq(inviter.person_id))
        .one(db)
        .await?
        .ok_or_else(super::acceptance::unavailable)?;
    let now = Utc::now().naive_utc();
    let member = membership::ActiveModel {
        household_id: Set(household_id),
        person_id: Set(subject.id),
        permissions_version: Set(1),
        role: Set(invitation.membership_role.clone()),
        status: Set("active".to_owned()),
        revoked_at: Set(None),
        joined_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    let before = super::issuing::state(&invitation);
    let mut active: household_invitation::ActiveModel = invitation.into();
    active.accepted_at = Set(Some(now));
    active.updated_at = Set(now);
    let invitation = active.update(db).await?;
    security_audit_event::ActiveModel {
        household_id: Set(household_id),
        actor_account_id: Set(Some(inviter_account.id)),
        actor_membership_id: Set(Some(inviter.id)),
        event_type: Set("household_access.membership_created".to_owned()),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(json!({
            "target_account_id":actor.account_id,
            "target_membership_id":member.id,
            "previous_state":null,
            "new_state":{"role":member.role,"status":member.status,"person_id":member.person_id,"permissions_version":1},
            "outcome":"success"
        })),
        audit_context: Set(json!({})),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    version::ActiveModel {
        item_type: Set("HouseholdInvitation".to_owned()),
        item_id: Set(invitation.id),
        event: Set("update".to_owned()),
        object: Set(Some(before.to_string())),
        object_changes: Set(Some(
            json!({"accepted_at":[null,invitation.accepted_at]}).to_string(),
        )),
        whodunnit: Set(None),
        request_id: Set(Some(request_id.to_owned())),
        household_id: Set(Some(household_id)),
        actor_membership_id: Set(None),
        audit_context: Set(json!({"actor_account_id":actor.account_id})),
        created_at: Set(Some(now)),
        ..Default::default()
    }
    .insert(db)
    .await?;
    let action = match origin {
        PersonOrigin::Existing => "update",
        PersonOrigin::New { user_id } => {
            version::ActiveModel {
                item_type: Set("Person".to_owned()),
                item_id: Set(subject.id),
                event: Set("create".to_owned()),
                object_changes: Set(Some(
                    json!({
                        "name":[null,subject.name],
                        "person_type":[null,subject.person_type],
                        "has_capacity":[null,subject.has_capacity]
                    })
                    .to_string(),
                )),
                household_id: Set(Some(household_id)),
                request_id: Set(Some(request_id.to_owned())),
                audit_context: Set(json!({
                    "actor_account_id":actor.account_id,
                    "actor_user_id":user_id,
                    "request_id":request_id
                })),
                created_at: Set(Some(now)),
                ..Default::default()
            }
            .insert(db)
            .await?;
            "create"
        }
    };
    api_change_event::ActiveModel {
        household_id: Set(household_id),
        household_membership_id: Set(None),
        account_id: Set(Some(actor.account_id)),
        action: Set(action.to_owned()),
        record_type: Set("Person".to_owned()),
        record_id: Set(subject.id),
        record_portable_id: Set(Some(subject.portable_id.clone())),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(json!({"record_type":"Person","record_id":subject.id,"portable_id":subject.portable_id})),
        occurred_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(member)
}
