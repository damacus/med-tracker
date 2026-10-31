use super::*;
use crate::models::entities::{
    account, api_change_event, membership, person, security_audit_event, version,
};

#[derive(Clone, Copy)]
pub(super) enum PersonOrigin {
    Existing(crate::models::care::person_schedules::sharing::ConsentAuthority),
    New,
}

pub(crate) async fn apply(
    db: &impl sea_orm::ConnectionTrait,
    actor: &super::acceptance::AcceptanceActor,
    source: &person::Model,
    inviter: &membership::Model,
    invitation: household_invitation::Model,
    request_id: &str,
    authority: crate::models::care::person_schedules::sharing::ConsentAuthority,
) -> Result<membership::Model, OperationError> {
    apply_to_person(
        db,
        actor,
        source.clone(),
        inviter,
        invitation,
        request_id,
        PersonOrigin::Existing(authority),
    )
    .await
}

pub(super) async fn apply_to_person(
    db: &impl sea_orm::ConnectionTrait,
    actor: &super::acceptance::AcceptanceActor,
    subject: person::Model,
    inviter: &membership::Model,
    invitation: household_invitation::Model,
    request_id: &str,
    origin: PersonOrigin,
) -> Result<membership::Model, OperationError> {
    let household_id = invitation.household_id;
    let owner = account::Entity::find_by_id(actor.account_id)
        .one(db)
        .await?
        .ok_or_else(super::acceptance::unavailable)?;
    if owner.person_id != actor.person_id
        || invitation.dependent_sharing != (subject.id != actor.person_id)
        || !crate::models::authorization::household_manager(inviter, household_id)
    {
        return Err(super::acceptance::unavailable());
    }
    if subject.id != actor.person_id
        && crate::models::care::person_schedules::sharing::guardian(
            db,
            crate::models::ids::PersonId::from_database(subject.id),
        )
        .await?
        .is_none()
    {
        return Err(OperationError::Forbidden);
    }
    let now = Utc::now().naive_utc();
    let existing = membership::Entity::find()
        .filter(membership::Column::HouseholdId.eq(household_id))
        .filter(membership::Column::PersonId.eq(subject.id))
        .one(db)
        .await?;
    let previous_state = existing.as_ref().map(|member| {
        json!({
            "role":member.role,"status":member.status,"person_id":member.person_id,
            "permissions_version":member.permissions_version
        })
    });
    let event = if existing.is_some() {
        "household_access.membership_updated"
    } else {
        "household_access.membership_created"
    };
    let member = if let Some(existing) = existing {
        if !["active", "revoked"].contains(&existing.status.as_str()) {
            return Err(super::acceptance::unavailable());
        }
        let next_version = existing.permissions_version + 1;
        let rejoining = existing.status == "revoked";
        let mut restored: membership::ActiveModel = existing.into();
        restored.status = Set("active".into());
        restored.revoked_at = Set(None);
        restored.permissions_version = Set(next_version);
        if rejoining {
            restored.joined_at = Set(now);
        }
        restored.updated_at = Set(now);
        restored.update(db).await?
    } else {
        membership::ActiveModel {
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
        .await?
    };
    if let PersonOrigin::Existing(authority) = origin {
        crate::models::care::person_schedules::sharing::approve(
            db,
            actor,
            &invitation,
            crate::models::ids::PersonId::from_database(subject.id),
            authority,
            request_id,
        )
        .await?;
    }
    let before = super::issuing::state(&invitation);
    let mut active: household_invitation::ActiveModel = invitation.into();
    active.accepted_at = Set(Some(now));
    active.updated_at = Set(now);
    let invitation = active.update(db).await?;
    let audit_context = json!({
        "actor_account_id":actor.account_id,"actor_person_id":actor.person_id,
        "authentication_method":actor.provenance.as_ref().map(|value| value.method.label()),
        "session_reference":actor.provenance.as_ref().map(|value| format!("{}:{}",value.method.reference_prefix(),value.reference))
    });
    security_audit_event::ActiveModel {
        household_id: Set(household_id),
        actor_account_id: Set(Some(actor.account_id)),
        actor_membership_id: Set(None),
        event_type: Set(event.to_owned()),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(json!({
            "target_account_id":actor.account_id,
            "target_membership_id":member.id,
            "actor_person_id":actor.person_id,
            "invited_by_membership_id":inviter.id,
            "previous_state":previous_state,
            "new_state":{"role":member.role,"status":member.status,"person_id":member.person_id,"permissions_version":member.permissions_version},
            "outcome":"success"
        })),
        audit_context: Set(audit_context.clone()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    version::Entity::insert(version::ActiveModel {
        item_type: Set("HouseholdInvitation".to_owned()),
        item_id: Set(invitation.id),
        event: Set("update".to_owned()),
        object: Set(Some(before.to_string())),
        object_changes: Set(Some(
            json!({"accepted_at":[null,invitation.accepted_at]}).to_string(),
        )),
        whodunnit: Set(Some(actor.account_id.to_string())),
        request_id: Set(Some(request_id.to_owned())),
        household_id: Set(Some(household_id)),
        actor_membership_id: Set(None),
        audit_context: Set(audit_context),
        created_at: Set(Some(now)),
        ..Default::default()
    })
    .exec_without_returning(db)
    .await?;
    let action = match origin {
        PersonOrigin::Existing(_) => "update",
        PersonOrigin::New => {
            version::Entity::insert(version::ActiveModel {
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
                    "actor_person_id":actor.person_id,
                    "request_id":request_id
                })),
                created_at: Set(Some(now)),
                ..Default::default()
            })
            .exec_without_returning(db)
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
