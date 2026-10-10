use super::*;
use crate::models::entities::{
    api_change_event, carer_relationship, grant, membership, person, security_audit_event, version,
};
use sea_orm::{DatabaseTransaction, IntoActiveModel, PaginatorTrait};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub(crate) async fn apply(
    db: &DatabaseTransaction,
    actor: &super::acceptance::AcceptanceActor,
    source: &person::Model,
    inviter: &membership::Model,
    invitation: household_invitation::Model,
    request_id: &str,
) -> Result<membership::Model, OperationError> {
    let household_id = invitation.household_id;
    let now = Utc::now().naive_utc();
    let new_person = person::ActiveModel {
        account_id: Set(Some(actor.account_id)),
        household_id: Set(household_id),
        portable_id: Set(Uuid::new_v4().to_string()),
        name: Set(source.name.clone()),
        email: Set(None),
        date_of_birth: Set(source.date_of_birth),
        person_type: Set(source.person_type),
        has_capacity: Set(source.has_capacity),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    apply_to_person(db, actor, new_person, inviter, invitation, request_id).await
}

pub(super) async fn apply_to_person(
    db: &DatabaseTransaction,
    actor: &super::acceptance::AcceptanceActor,
    new_person: person::Model,
    inviter: &membership::Model,
    invitation: household_invitation::Model,
    request_id: &str,
) -> Result<membership::Model, OperationError> {
    let household_id = invitation.household_id;
    if !crate::models::authorization::household_manager(inviter, household_id)
        && (new_person.person_type != 0 || !new_person.has_capacity)
    {
        return Err(super::acceptance::unavailable());
    }
    let now = Utc::now().naive_utc();
    let new_membership = membership::ActiveModel {
        account_id: Set(actor.account_id),
        household_id: Set(household_id),
        person_id: Set(Some(new_person.id)),
        permissions_version: Set(1),
        role: Set(invitation.membership_role.clone()),
        status: Set("active".to_owned()),
        revoked_at: Set(None),
        joined_at: Set(Some(now)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    grant::ActiveModel {
        household_id: Set(household_id),
        household_membership_id: Set(new_membership.id),
        person_id: Set(new_person.id),
        access_level: Set("manage".to_owned()),
        relationship_type: Set("self".to_owned()),
        expires_at: Set(None),
        revoked_at: Set(None),
        granted_by_membership_id: Set(Some(new_membership.id)),
        carer_relationship_id: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    let additional = household_invitation_grant::Entity::find()
        .filter(household_invitation_grant::Column::HouseholdInvitationId.eq(invitation.id))
        .filter(household_invitation_grant::Column::HouseholdId.eq(household_id))
        .all(db)
        .await?;
    let mut latest_grants = HashMap::new();
    for row in additional {
        let entry = latest_grants
            .entry(row.person_id)
            .or_insert_with(|| row.clone());
        if row.id > entry.id {
            *entry = row;
        }
    }
    let additional = latest_grants.into_values().collect::<Vec<_>>();
    let mut created_relations = Vec::new();
    if !additional.is_empty() {
        let patient_ids = additional
            .iter()
            .map(|row| row.person_id)
            .collect::<HashSet<_>>();
        let patients = person::Entity::find()
            .filter(person::Column::HouseholdId.eq(household_id))
            .filter(person::Column::Id.is_in(patient_ids.iter().copied()))
            .all(db)
            .await?;
        if patients.len() != patient_ids.len() {
            return Err(super::acceptance::unavailable());
        }
        let relationships = additional
            .iter()
            .filter_map(|row| {
                let relation_type = match row.relationship_type.as_str() {
                    "parent" => "parent",
                    "family_member" => "family_member",
                    "carer" | "professional" => "professional_carer",
                    _ => return None,
                };
                Some(carer_relationship::ActiveModel {
                    household_id: Set(household_id),
                    carer_id: Set(new_person.id),
                    patient_id: Set(row.person_id),
                    relationship_type: Set(Some(relation_type.to_owned())),
                    active: Set(true),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                })
            })
            .collect::<Vec<_>>();
        if !relationships.is_empty() {
            carer_relationship::Entity::insert_many(relationships)
                .exec(db)
                .await?;
        }
        let relationships = carer_relationship::Entity::find()
            .filter(carer_relationship::Column::HouseholdId.eq(household_id))
            .filter(carer_relationship::Column::CarerId.eq(new_person.id))
            .filter(carer_relationship::Column::PatientId.is_in(patient_ids.iter().copied()))
            .all(db)
            .await?;
        created_relations = relationships.clone();
        let relation_ids = relationships
            .into_iter()
            .map(|row| (row.patient_id, row.id))
            .collect::<HashMap<_, _>>();
        let grants = additional
            .into_iter()
            .map(|row| {
                let relation_id = if matches!(
                    row.relationship_type.as_str(),
                    "parent" | "family_member" | "carer" | "professional"
                ) {
                    relation_ids.get(&row.person_id).copied()
                } else {
                    None
                };
                let grant_type =
                    if matches!(row.relationship_type.as_str(), "carer" | "professional") {
                        "professional".to_owned()
                    } else {
                        row.relationship_type
                    };
                grant::ActiveModel {
                    household_id: Set(household_id),
                    household_membership_id: Set(new_membership.id),
                    person_id: Set(row.person_id),
                    access_level: Set(row.access_level),
                    relationship_type: Set(grant_type),
                    expires_at: Set(row.expires_at),
                    revoked_at: Set(None),
                    granted_by_membership_id: Set(Some(inviter.id)),
                    carer_relationship_id: Set(relation_id),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                }
            })
            .collect::<Vec<_>>();
        grant::Entity::insert_many(grants).exec(db).await?;
    }
    let grant_count = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(new_membership.id))
        .filter(grant::Column::RevokedAt.is_null())
        .count(db)
        .await?;
    let mut member_active = new_membership.into_active_model();
    member_active.permissions_version =
        Set(i32::try_from(grant_count).map_err(|_| OperationError::Unavailable)? + 1);
    member_active.updated_at = Set(now);
    let new_membership = member_active.update(db).await?;
    let invitation_before = invitation.clone();
    let mut active: household_invitation::ActiveModel = invitation.into();
    active.accepted_at = Set(Some(now));
    active.updated_at = Set(now);
    let invitation_after = active.update(db).await?;
    record_acceptance_effects(
        db,
        request_id,
        actor,
        inviter,
        &new_membership,
        &new_person,
        &invitation_before,
        &invitation_after,
        &created_relations,
        now,
    )
    .await?;
    Ok(new_membership)
}

#[allow(clippy::too_many_arguments)]
async fn record_acceptance_effects(
    db: &DatabaseTransaction,
    request_id: &str,
    actor: &super::acceptance::AcceptanceActor,
    inviter: &membership::Model,
    member: &membership::Model,
    new_person: &person::Model,
    invitation_before: &household_invitation::Model,
    invitation_after: &household_invitation::Model,
    relations: &[carer_relationship::Model],
    now: NaiveDateTime,
) -> Result<(), OperationError> {
    let household_id = member.household_id;
    let grants = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(member.id))
        .filter(grant::Column::RevokedAt.is_null())
        .all(db)
        .await?;
    let mut events = vec![security_audit_event::ActiveModel {
        household_id: Set(household_id),
        actor_account_id: Set(Some(inviter.account_id)),
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
    }];
    for row in &grants {
        let self_grant = row.person_id == new_person.id;
        events.push(security_audit_event::ActiveModel {
            household_id: Set(household_id),
            actor_account_id: Set(Some(if self_grant {
                actor.account_id
            } else {
                inviter.account_id
            })),
            actor_membership_id: Set(Some(if self_grant { member.id } else { inviter.id })),
            event_type: Set("household_access.person_grant_changed".to_owned()),
            request_id: Set(Some(request_id.to_owned())),
            metadata: Set(json!({
                "target_membership_id":member.id,
                "target_grant_id":row.id,
                "previous_state":null,
                "new_state":{
                    "household_membership_id":member.id,
                    "person_id":row.person_id,
                    "access_level":row.access_level,
                    "relationship_type":row.relationship_type,
                    "expires_at":row.expires_at.map(|at| at.and_utc().to_rfc3339()),
                    "revoked_at":null,
                    "carer_relationship_id":row.carer_relationship_id
                },
                "outcome":"success"
            })),
            audit_context: Set(json!({})),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        });
    }
    security_audit_event::Entity::insert_many(events)
        .exec(db)
        .await?;
    let mut versions = vec![version::ActiveModel {
        item_type: Set("Person".to_owned()),
        item_id: Set(new_person.id),
        event: Set("create".to_owned()),
        object: Set(None),
        object_changes: Set(Some(json!({"name":[null,new_person.name],"person_type":[null,new_person.person_type],"has_capacity":[null,new_person.has_capacity]}).to_string())),
        whodunnit: Set(None),
        request_id: Set(Some(request_id.to_owned())),
        household_id: Set(Some(household_id)),
        actor_membership_id: Set(None),
        audit_context: Set(json!({"actor_account_id":actor.account_id})),
        created_at: Set(Some(now)),
        ..Default::default()
    }];
    for relation in relations {
        versions.push(version::ActiveModel {
            item_type: Set("CarerRelationship".to_owned()),
            item_id: Set(relation.id),
            event: Set("create".to_owned()),
            object: Set(None),
            object_changes: Set(Some(json!({"carer_id":[null,relation.carer_id],"patient_id":[null,relation.patient_id],"relationship_type":[null,relation.relationship_type]}).to_string())),
            whodunnit: Set(None),
            request_id: Set(Some(request_id.to_owned())),
            household_id: Set(Some(household_id)),
            actor_membership_id: Set(None),
            audit_context: Set(json!({"actor_account_id":actor.account_id})),
            created_at: Set(Some(now)),
            ..Default::default()
        });
    }
    versions.push(version::ActiveModel {
        item_type: Set("HouseholdInvitation".to_owned()),
        item_id: Set(invitation_before.id),
        event: Set("update".to_owned()),
        object: Set(Some(super::issuing::state(invitation_before).to_string())),
        object_changes: Set(Some(
            json!({"accepted_at":[null,invitation_after.accepted_at]}).to_string(),
        )),
        whodunnit: Set(None),
        request_id: Set(Some(request_id.to_owned())),
        household_id: Set(Some(household_id)),
        actor_membership_id: Set(None),
        audit_context: Set(json!({"actor_account_id":actor.account_id})),
        created_at: Set(Some(now)),
        ..Default::default()
    });
    version::Entity::insert_many(versions).exec(db).await?;
    api_change_event::ActiveModel {
        household_id: Set(household_id),
        household_membership_id: Set(None),
        account_id: Set(Some(actor.account_id)),
        action: Set("create".to_owned()),
        record_type: Set("Person".to_owned()),
        record_id: Set(new_person.id),
        record_portable_id: Set(Some(new_person.portable_id.clone())),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(json!({"record_type":"Person","record_id":new_person.id,"portable_id":new_person.portable_id})),
        occurred_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }.insert(db).await?;
    Ok(())
}
