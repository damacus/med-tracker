use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) async fn record_acceptance_effects(
    db: &DatabaseTransaction,
    request_id: &str,
    actor: &auth_sessions::InvitationActor,
    inviter: &membership::Model,
    member: &membership::Model,
    new_person: &person::Model,
    invitation_before: &household_invitation::Model,
    invitation_after: &household_invitation::Model,
    relations: &[carer_relationship::Model],
    now: NaiveDateTime,
) -> Result<(), ApiError> {
    let household_id = member.household_id;
    let grants = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(member.id))
        .filter(grant::Column::RevokedAt.is_null())
        .all(db)
        .await
        .map_err(database_error)?;
    let mut events = vec![security_audit_event::ActiveModel {
        household_id: Set(household_id),
        actor_account_id: Set(Some(inviter.account_id)),
        actor_membership_id: Set(Some(inviter.id)),
        event_type: Set("household_access.membership_created".to_owned()),
        request_id: Set(Some(request_id.to_owned())),
        metadata: Set(json!({
            "target_account_id":actor.account.id,
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
                actor.account.id
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
        .await
        .map_err(database_error)?;
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
        audit_context: Set(json!({"actor_account_id":actor.account.id})),
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
            audit_context: Set(json!({"actor_account_id":actor.account.id})),
            created_at: Set(Some(now)),
            ..Default::default()
        });
    }
    versions.push(version::ActiveModel {
        item_type: Set("HouseholdInvitation".to_owned()),
        item_id: Set(invitation_before.id),
        event: Set("update".to_owned()),
        object: Set(Some(state_row(invitation_before).to_string())),
        object_changes: Set(Some(
            json!({"accepted_at":[null,invitation_after.accepted_at]}).to_string(),
        )),
        whodunnit: Set(None),
        request_id: Set(Some(request_id.to_owned())),
        household_id: Set(Some(household_id)),
        actor_membership_id: Set(None),
        audit_context: Set(json!({"actor_account_id":actor.account.id})),
        created_at: Set(Some(now)),
        ..Default::default()
    });
    version::Entity::insert_many(versions)
        .exec(db)
        .await
        .map_err(database_error)?;
    api_change_event::ActiveModel {
        household_id: Set(household_id),
        household_membership_id: Set(None),
        account_id: Set(Some(actor.account.id)),
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
    }.insert(db).await.map_err(database_error)?;
    Ok(())
}
