use super::*;

pub async fn create(
    tenant: &TenantTransaction,
    attributes: Value,
    zone: Tz,
    provenance: Option<&CredentialProvenance>,
) -> Result<person::Model, OperationError> {
    authorize_create(tenant).await?;
    let now = Utc::now().naive_utc();
    let mut record = person::Model {
        id: 0,
        account_id: None,
        household_id: tenant.scope().household_id,
        portable_id: uuid::Uuid::new_v4().to_string(),
        name: String::new(),
        email: None,
        person_type: 0,
        date_of_birth: None,
        has_capacity: true,
        created_at: now,
        updated_at: now,
    };
    validation::assign(&mut record, &attributes, true)?;
    let dependent = validation::capacity(&mut record, zone)?;
    let carer = if let Some(id) = tenant.membership().person_id {
        person::Entity::find_by_id(id)
            .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
            .one(tenant.transaction())
            .await?
    } else {
        None
    };
    if !record.has_capacity && (!dependent || carer.is_none()) {
        return Err(invalid("has_capacity", "requires an active carer"));
    }
    let saved = person::ActiveModel {
        account_id: Set(None),
        household_id: Set(record.household_id),
        portable_id: Set(record.portable_id),
        name: Set(record.name),
        email: Set(record.email),
        person_type: Set(record.person_type),
        date_of_birth: Set(record.date_of_birth),
        has_capacity: Set(record.has_capacity),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await
    .map_err(write_error)?;
    let relationship = if dependent {
        Some(
            carer_relationship::ActiveModel {
                household_id: Set(tenant.scope().household_id),
                carer_id: Set(carer.as_ref().ok_or(OperationError::Unavailable)?.id),
                patient_id: Set(saved.id),
                relationship_type: Set(Some("family_member".into())),
                active: Set(true),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(tenant.transaction())
            .await?,
        )
    } else {
        None
    };
    let self_grant = if let Some(carer) = carer.as_ref().filter(|_| dependent) {
        Some(ensure_self_grant(tenant, carer.id, now).await?)
    } else {
        None
    };
    let home = location::Entity::find()
        .filter(location::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(location::Column::Name.eq("Home"))
        .one(tenant.transaction())
        .await?;
    let home = match home {
        Some(home) => home,
        None => {
            let home = location::ActiveModel {
                household_id: Set(tenant.scope().household_id),
                portable_id: Set(uuid::Uuid::new_v4().to_string()),
                name: Set("Home".into()),
                description: Set(Some("Primary home location".into())),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(tenant.transaction())
            .await?;
            persistence::record_version(tenant,"Location",home.id,"create",None,json!({"id":home.id,"household_id":home.household_id,"portable_id":home.portable_id,"name":home.name,"description":home.description,"created_at":home.created_at,"updated_at":home.updated_at}),provenance).await?;
            persistence::change(
                tenant,
                "Location",
                home.id,
                &home.portable_id,
                "create",
                provenance,
            )
            .await?;
            home
        }
    };
    let link = location_membership::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        location_id: Set(home.id),
        person_id: Set(saved.id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    let grant = grant::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        household_membership_id: Set(tenant.membership().id),
        person_id: Set(saved.id),
        access_level: Set("manage".into()),
        relationship_type: Set("family_member".into()),
        granted_by_membership_id: Set(Some(tenant.membership().id)),
        revoked_at: Set(None),
        expires_at: Set(None),
        carer_relationship_id: Set(relationship.as_ref().map(|row| row.id)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    membership::Entity::update_many()
        .col_expr(
            membership::Column::PermissionsVersion,
            Expr::col(membership::Column::PermissionsVersion)
                .add(1 + i32::from(self_grant.as_ref().is_some_and(Option::is_some))),
        )
        .col_expr(membership::Column::UpdatedAt, Expr::value(now))
        .filter(membership::Column::Id.eq(tenant.membership().id))
        .filter(membership::Column::HouseholdId.eq(tenant.scope().household_id))
        .exec(tenant.transaction())
        .await?;
    persistence::record_version(
        tenant,
        "Person",
        saved.id,
        "create",
        None,
        persistence::snapshot(&saved),
        provenance,
    )
    .await?;
    persistence::record_version(tenant,"LocationMembership",link.id,"create",None,json!({"id":link.id,"household_id":link.household_id,"location_id":link.location_id,"person_id":link.person_id,"created_at":link.created_at,"updated_at":link.updated_at}),provenance).await?;
    if let Some(row) = relationship {
        persistence::record_version(tenant,"CarerRelationship",row.id,"create",None,json!({"id":row.id,"household_id":row.household_id,"carer_id":row.carer_id,"patient_id":row.patient_id,"relationship_type":row.relationship_type,"active":row.active,"created_at":row.created_at,"updated_at":row.updated_at}),provenance).await?;
    }
    if let Some(Some((before, row))) = self_grant {
        persistence::grant_event(tenant, &row, before, provenance).await?;
    }
    persistence::grant_event(tenant, &grant, None, provenance).await?;
    persistence::change(
        tenant,
        "Person",
        saved.id,
        &saved.portable_id,
        "create",
        provenance,
    )
    .await?;
    Ok(saved)
}

async fn ensure_self_grant(
    tenant: &TenantTransaction,
    person_id: i64,
    now: chrono::NaiveDateTime,
) -> Result<Option<(Option<Value>, grant::Model)>, OperationError> {
    let query = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(tenant.membership().id))
        .filter(grant::Column::PersonId.eq(person_id));
    let active = query
        .clone()
        .filter(grant::Column::RevokedAt.is_null())
        .lock_exclusive()
        .one(tenant.transaction())
        .await?;
    let existing = if active.is_some() {
        active
    } else {
        query
            .order_by_desc(grant::Column::Id)
            .lock_exclusive()
            .one(tenant.transaction())
            .await?
    };
    if let Some(row) = existing {
        if row.access_level == "manage"
            && row.relationship_type == "self"
            && row.expires_at.is_none()
            && row.revoked_at.is_none()
            && row.carer_relationship_id.is_none()
        {
            return Ok(None);
        }
        let before = persistence::grant_state(&row);
        let mut active: grant::ActiveModel = row.into();
        active.access_level = Set("manage".into());
        active.relationship_type = Set("self".into());
        active.expires_at = Set(None);
        active.revoked_at = Set(None);
        active.carer_relationship_id = Set(None);
        active.updated_at = Set(now);
        Ok(Some((
            Some(before),
            active.update(tenant.transaction()).await?,
        )))
    } else {
        let row = grant::ActiveModel {
            household_id: Set(tenant.scope().household_id),
            household_membership_id: Set(tenant.membership().id),
            person_id: Set(person_id),
            access_level: Set("manage".into()),
            relationship_type: Set("self".into()),
            granted_by_membership_id: Set(Some(tenant.membership().id)),
            revoked_at: Set(None),
            expires_at: Set(None),
            carer_relationship_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(tenant.transaction())
        .await?;
        Ok(Some((None, row)))
    }
}
