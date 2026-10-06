use super::*;

pub(super) fn state(row: &grant::Model) -> Value {
    json!({"household_membership_id":row.household_membership_id,"person_id":row.person_id,"access_level":row.access_level,"relationship_type":row.relationship_type,"expires_at":row.expires_at.map(|time|time.and_utc().to_rfc3339()),"revoked_at":row.revoked_at.map(|time|time.and_utc().to_rfc3339()),"carer_relationship_id":row.carer_relationship_id})
}
async fn values(
    tenant: &TenantTransaction,
    rows: &[grant::Model],
) -> Result<Vec<Value>, OperationError> {
    let people: HashMap<i64, person::Model> = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(person::Column::Id.is_in(rows.iter().map(|row| row.person_id).collect::<Vec<_>>()))
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|row| (row.id, row))
        .collect();
    rows.iter().map(|row|Ok(json!({"id":row.id,"household_membership_id":row.household_membership_id,"person_id":row.person_id,"person_name":people.get(&row.person_id).ok_or(OperationError::Unavailable)?.name,"access_level":row.access_level,"relationship_type":row.relationship_type,"expires_at":row.expires_at.map(|time|time.and_utc().to_rfc3339()),"revoked_at":row.revoked_at.map(|time|time.and_utc().to_rfc3339())}))).collect()
}
pub async fn list(tenant: &TenantTransaction) -> Result<Value, OperationError> {
    authorize(tenant).await?;
    let rows = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(tenant.scope().household_id))
        .order_by_asc(grant::Column::Id)
        .all(tenant.transaction())
        .await?;
    Ok(json!({"data":values(tenant,&rows).await?}))
}
pub async fn create(
    tenant: &TenantTransaction,
    attributes: Value,
    provenance: Option<&CredentialProvenance>,
) -> Result<Value, OperationError> {
    authorize(tenant).await?;
    let parsed = (|| {
        fields(
            &attributes,
            &[
                "household_membership_id",
                "person_id",
                "access_level",
                "relationship_type",
                "expires_at",
            ],
            "person_access_grant",
        )?;
        let member_id = positive_id(&attributes, "household_membership_id")?;
        let person_id = positive_id(&attributes, "person_id")?;
        let level = attributes["access_level"]
            .as_str()
            .filter(|level| matches!(*level, "view" | "record" | "manage"))
            .ok_or_else(|| invalid("access_level", "is invalid"))?;
        let relationship = attributes["relationship_type"]
            .as_str()
            .filter(|kind| {
                matches!(
                    *kind,
                    "self" | "parent" | "family_member" | "carer" | "professional"
                )
            })
            .ok_or_else(|| invalid("relationship_type", "is invalid"))?;
        let expiry = match attributes.get("expires_at") {
            None | Some(Value::Null) => None,
            Some(Value::String(value)) => Some(
                DateTime::parse_from_rfc3339(value)
                    .map_err(|_| invalid("expires_at", "is invalid"))?
                    .naive_utc(),
            ),
            _ => return Err(invalid("expires_at", "is invalid")),
        };
        Ok::<_, OperationError>((member_id, person_id, level, relationship, expiry))
    })();
    let (member_id, person_id, level, relationship, expiry) = match parsed {
        Ok(attributes) => attributes,
        Err(error) => {
            rejected(tenant, None, json!({}), provenance).await?;
            return Err(error);
        }
    };
    let attempted = json!({"household_membership_id":member_id,"person_id":person_id,"access_level":level,"relationship_type":relationship,"expires_at":expiry.map(|time|time.and_utc().to_rfc3339())});
    let member = membership::Entity::find_by_id(member_id)
        .filter(membership::Column::HouseholdId.eq(tenant.scope().household_id))
        .lock_exclusive()
        .one(tenant.transaction())
        .await?;
    let person = person::Entity::find_by_id(person_id)
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .one(tenant.transaction())
        .await?;
    if member.is_none() || person.is_none() {
        rejected(tenant, Some(member_id), attempted, provenance).await?;
        return Err(invalid(
            "base",
            "Grant records must belong to the household",
        ));
    }
    if grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(member_id))
        .filter(grant::Column::PersonId.eq(person_id))
        .filter(grant::Column::RevokedAt.is_null())
        .one(tenant.transaction())
        .await?
        .is_some()
    {
        rejected(tenant, Some(member_id), attempted, provenance).await?;
        return Err(invalid("household_membership_id", "has already been taken"));
    }
    let now = Utc::now().naive_utc();
    let row = grant::ActiveModel {
        household_id: Set(tenant.scope().household_id),
        household_membership_id: Set(member_id),
        person_id: Set(person_id),
        access_level: Set(level.into()),
        relationship_type: Set(relationship.into()),
        expires_at: Set(expiry),
        revoked_at: Set(None),
        granted_by_membership_id: Set(Some(tenant.membership().id)),
        carer_relationship_id: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(tenant.transaction())
    .await?;
    persistence::bump(tenant, member_id).await?;
    record(tenant, &row, None, "success", provenance).await?;
    Ok(json!({"data":values(tenant,&[row]).await?.remove(0)}))
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
        .filter(grant::Column::HouseholdId.eq(tenant.scope().household_id))
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
