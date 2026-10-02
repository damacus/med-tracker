use super::*;

async fn accept_pending(
    db: &DatabaseTransaction,
    request_id: &str,
    actor: &auth_sessions::InvitationActor,
    headers: &HeaderMap,
    invitation: household_invitation::Model,
) -> Result<membership::Model, ApiError> {
    let household_id = invitation.household_id;
    let expected_digest = invitation.token_digest.clone();
    tenant_setting(db, "med_tracker.current_household_id", household_id)
        .await
        .map_err(database_error)?;
    tenant_setting(db, "med_tracker.current_account_id", actor.account.id)
        .await
        .map_err(database_error)?;
    let home = household::Entity::find_by_id(household_id)
        .lock_exclusive()
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(invitation_unavailable)?;
    if home.status != "active" || home.lifecycle_state != "active" {
        return Err(invitation_unavailable());
    }
    let current_actor = auth_sessions::invitation_actor(db, headers).await?;
    if current_actor.account.id != actor.account.id || current_actor.session.id != actor.session.id
    {
        return Err(invitation_unavailable());
    }
    let invitation = household_invitation::Entity::find_by_id(invitation.id)
        .filter(household_invitation::Column::HouseholdId.eq(household_id))
        .lock_exclusive()
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(invitation_unavailable)?;
    if invitation.email != actor.account.email.to_lowercase()
        || invitation.token_digest != expected_digest
        || invitation.revoked_at.is_some()
        || invitation.expires_at <= Utc::now().naive_utc()
    {
        return Err(invitation_unavailable());
    }
    if invitation.accepted_at.is_some() {
        return membership::Entity::find()
            .filter(membership::Column::HouseholdId.eq(household_id))
            .filter(membership::Column::AccountId.eq(actor.account.id))
            .filter(membership::Column::Status.eq("active"))
            .filter(membership::Column::RevokedAt.is_null())
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(invitation_unavailable);
    }
    let inviter = membership::Entity::find_by_id(invitation.invited_by_membership_id)
        .one(db)
        .await
        .map_err(database_error)?
        .ok_or_else(invitation_unavailable)?;
    if inviter.household_id != household_id
        || inviter.status != "active"
        || inviter.revoked_at.is_some()
        || !matches!(inviter.role.as_str(), "owner" | "administrator")
    {
        return Err(invitation_unavailable());
    }
    let existing = membership::Entity::find()
        .filter(membership::Column::HouseholdId.eq(household_id))
        .filter(membership::Column::AccountId.eq(actor.account.id))
        .one(db)
        .await
        .map_err(database_error)?;
    if existing.is_some() {
        return Err(invitation_unavailable());
    }
    let now = Utc::now().naive_utc();
    let new_person = person::ActiveModel {
        account_id: Set(Some(actor.account.id)),
        household_id: Set(household_id),
        portable_id: Set(Uuid::new_v4().to_string()),
        name: Set(actor.person.name.clone()),
        email: Set(None),
        date_of_birth: Set(actor.person.date_of_birth),
        person_type: Set(actor.person.person_type),
        has_capacity: Set(actor.person.has_capacity),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(database_error)?;
    let new_membership = membership::ActiveModel {
        account_id: Set(actor.account.id),
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
    .await
    .map_err(database_error)?;
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
    .await
    .map_err(database_error)?;
    let additional = household_invitation_grant::Entity::find()
        .filter(household_invitation_grant::Column::HouseholdInvitationId.eq(invitation.id))
        .filter(household_invitation_grant::Column::HouseholdId.eq(household_id))
        .all(db)
        .await
        .map_err(database_error)?;
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
            .await
            .map_err(database_error)?;
        if patients.len() != patient_ids.len() {
            return Err(invitation_unavailable());
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
                .await
                .map_err(database_error)?;
        }
        let relationships = carer_relationship::Entity::find()
            .filter(carer_relationship::Column::HouseholdId.eq(household_id))
            .filter(carer_relationship::Column::CarerId.eq(new_person.id))
            .filter(carer_relationship::Column::PatientId.is_in(patient_ids.iter().copied()))
            .all(db)
            .await
            .map_err(database_error)?;
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
        grant::Entity::insert_many(grants)
            .exec(db)
            .await
            .map_err(database_error)?;
    }
    let grant_count = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(new_membership.id))
        .filter(grant::Column::RevokedAt.is_null())
        .count(db)
        .await
        .map_err(database_error)?;
    let mut member_active = new_membership.into_active_model();
    member_active.permissions_version =
        Set(i32::try_from(grant_count).map_err(|_| ApiError::internal())? + 1);
    member_active.updated_at = Set(now);
    let new_membership = member_active.update(db).await.map_err(database_error)?;
    let invitation_before = invitation.clone();
    let mut active: household_invitation::ActiveModel = invitation.into();
    active.accepted_at = Set(Some(now));
    active.updated_at = Set(now);
    let invitation_after = active.update(db).await.map_err(database_error)?;
    record_acceptance_effects(
        db,
        request_id,
        actor,
        &inviter,
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

pub(crate) async fn accept(
    state: State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let result = accept_inner(state, headers, payload).await;
    let mut response = match result {
        Ok(response) => response,
        Err(error) => error.into_response(),
    };
    no_store(&mut response);
    Ok(response)
}

async fn accept_inner(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let db = state.db.begin().await.map_err(database_error)?;
    restricted_role(&db).await.map_err(database_error)?;
    let actor = auth_sessions::invitation_actor(&db, &headers).await?;
    let Json(body) = payload.map_err(|_| {
        acceptance_error(
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid request body",
        )
    })?;
    let outer = body.as_object().ok_or_else(|| {
        acceptance_error(
            StatusCode::BAD_REQUEST,
            "bad_request",
            "Invalid request body",
        )
    })?;
    let token = outer
        .get("token")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            acceptance_error(
                StatusCode::BAD_REQUEST,
                "bad_request",
                "Invalid request body",
            )
        })?;
    if outer.len() != 1 {
        return Err(acceptance_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            "Invalid request body",
        ));
    }
    let digest = format!("{:x}", Sha256::digest(token.as_bytes()));
    let request_id = Uuid::new_v4().to_string();
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT set_config('med_tracker.current_invitation_token_digest', $1, true)",
        [digest.clone().into()],
    ))
    .await
    .map_err(database_error)?;
    let invitation = household_invitation::Entity::find()
        .filter(household_invitation::Column::TokenDigest.eq(&digest))
        .filter(household_invitation::Column::AcceptedAt.is_null())
        .filter(household_invitation::Column::RevokedAt.is_null())
        .filter(household_invitation::Column::ExpiresAt.gt(Utc::now().naive_utc()))
        .one(&db)
        .await
        .map_err(database_error)?;
    let result = if let Some(invitation) = invitation {
        accept_pending(&db, &request_id, &actor, &headers, invitation).await?
    } else {
        accepted_retry(&db, &actor, &headers, &digest)
            .await?
            .ok_or_else(invitation_unavailable)?
    };
    db.commit().await.map_err(database_error)?;
    let mut response = (StatusCode::OK, Json(accepted_body(&result))).into_response();
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).map_err(|_| ApiError::internal())?,
    );
    no_store(&mut response);
    Ok(response)
}
