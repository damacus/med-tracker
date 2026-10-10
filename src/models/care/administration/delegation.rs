use super::*;
use crate::models::access::PersonAccess;
use sea_orm::sea_query::{Expr, ExprTrait};

fn snapshot(row: &carer_relationship::Model) -> Value {
    json!({"id":row.id,"household_id":row.household_id,"carer_id":row.carer_id,"patient_id":row.patient_id,"relationship_type":row.relationship_type,"active":row.active,"created_at":row.created_at,"updated_at":row.updated_at})
}
pub async fn list(tenant: &TenantTransaction) -> Result<Value, OperationError> {
    authorize(tenant).await?;
    let people: HashMap<i64, person::Model> = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|row| (row.id, row))
        .collect();
    let rows = carer_relationship::Entity::find()
        .filter(carer_relationship::Column::HouseholdId.eq(tenant.scope().household_id))
        .order_by_asc(carer_relationship::Column::Id)
        .all(tenant.transaction())
        .await?;
    let data: Vec<Value> = rows
        .iter()
        .map(|row| {
            let mut value = snapshot(row);
            value["carer_name"] = people
                .get(&row.carer_id)
                .map_or(Value::Null, |row| json!(row.name));
            value["patient_name"] = people
                .get(&row.patient_id)
                .map_or(Value::Null, |row| json!(row.name));
            value
        })
        .collect();
    Ok(json!({"data":data}))
}
pub async fn options(tenant: &TenantTransaction) -> Result<Value, OperationError> {
    access::recheck(tenant).await?;
    let grants = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(tenant.membership().id))
        .filter(grant::Column::RevokedAt.is_null())
        .filter(
            sea_orm::Condition::any()
                .add(grant::Column::ExpiresAt.is_null())
                .add(
                    Expr::col(grant::Column::ExpiresAt)
                        .gt(Expr::cust("timezone('UTC', clock_timestamp())")),
                ),
        )
        .all(tenant.transaction())
        .await?;
    let rows = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(
            person::Column::Id.is_in(grants.iter().map(|row| row.person_id).collect::<Vec<_>>()),
        )
        .order_by_asc(person::Column::Name)
        .all(tenant.transaction())
        .await?;
    let patients: Vec<Value> = rows
        .iter()
        .map(|row| json!({"id":row.id,"name":row.name}))
        .collect();
    let carers: Vec<Value> = rows
        .iter()
        .filter(|row| row.has_capacity)
        .map(|row| json!({"id":row.id,"name":row.name}))
        .collect();
    Ok(json!({"carers":carers,"patients":patients}))
}
pub async fn assign(
    tenant: &TenantTransaction,
    carer_id: i64,
    patient_id: i64,
    relationship_type: &str,
    provenance: Option<&CredentialProvenance>,
) -> Result<carer_relationship::Model, OperationError> {
    household::Entity::find_by_id(tenant.scope().household_id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    access::recheck(tenant).await?;
    let carer = person::Entity::find_by_id(carer_id)
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .one(tenant.transaction())
        .await?
        .ok_or_else(|| invalid("carer", "must belong to the relationship household"))?;
    let patient = person::Entity::find_by_id(patient_id)
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .one(tenant.transaction())
        .await?
        .ok_or_else(|| invalid("patient", "must belong to the relationship household"))?;
    if !crate::models::authorization::may_delegate(tenant.membership(), &patient) {
        return Err(OperationError::Forbidden);
    }
    if !can_manage(tenant).await? {
        access::require_person_access(tenant, patient_id, PersonAccess::Manage).await?;
    }
    let (level, kind) = match relationship_type {
        "parent" => ("manage", "parent"),
        "family_member" => ("manage", "family_member"),
        "professional_carer" => ("record", "professional"),
        "self" => ("manage", "self"),
        _ => return Err(invalid("relationship_type", "is invalid")),
    };
    let existing = carer_relationship::Entity::find()
        .filter(carer_relationship::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(carer_relationship::Column::CarerId.eq(carer_id))
        .filter(carer_relationship::Column::PatientId.eq(patient_id))
        .lock_exclusive()
        .one(tenant.transaction())
        .await?;
    let now = Utc::now().naive_utc();
    let before = existing.as_ref().map(snapshot);
    let row = if let Some(row) = existing {
        if row.active && row.relationship_type.as_deref() == Some(relationship_type) {
            row
        } else {
            let mut active: carer_relationship::ActiveModel = row.into();
            active.relationship_type = Set(Some(relationship_type.into()));
            active.active = Set(true);
            active.updated_at = Set(now);
            active.update(tenant.transaction()).await?
        }
    } else {
        carer_relationship::ActiveModel {
            household_id: Set(tenant.scope().household_id),
            carer_id: Set(carer_id),
            patient_id: Set(patient_id),
            relationship_type: Set(Some(relationship_type.into())),
            active: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(tenant.transaction())
        .await?
    };
    if before.as_ref().is_none_or(|value| {
        value["active"] != true || value["relationship_type"] != relationship_type
    }) {
        persistence::record_version(
            tenant,
            "CarerRelationship",
            row.id,
            before,
            snapshot(&row),
            provenance,
        )
        .await?;
    }
    if let Some(account_id) = carer.account_id {
        let member = membership::Entity::find()
            .filter(membership::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(membership::Column::AccountId.eq(account_id))
            .lock_exclusive()
            .one(tenant.transaction())
            .await?;
        let member = if let Some(member) = member {
            memberships::change_for_delegation(tenant, member.id, carer_id, provenance).await?;
            membership::Entity::find_by_id(member.id)
                .one(tenant.transaction())
                .await?
                .ok_or(OperationError::Unavailable)?
        } else {
            let member = membership::ActiveModel {
                account_id: Set(account_id),
                household_id: Set(tenant.scope().household_id),
                person_id: Set(Some(carer_id)),
                role: Set("member".into()),
                status: Set("active".into()),
                permissions_version: Set(1),
                revoked_at: Set(None),
                joined_at: Set(Some(now)),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(tenant.transaction())
            .await?;
            persistence::event(tenant,"household_access.membership_created",json!({"target_account_id":member.account_id,"target_membership_id":member.id,"previous_state":null,"new_state":{"role":member.role,"status":member.status,"person_id":member.person_id},"outcome":"success"}),provenance).await?;
            member
        };
        upsert(
            tenant, member.id, carer_id, "manage", "self", None, provenance,
        )
        .await?;
        if !(carer_id == patient_id && relationship_type == "self") {
            patient_grant(tenant, &row, member.id, level, kind, provenance).await?;
        }
    }
    Ok(row)
}
async fn patient_grant(
    tenant: &TenantTransaction,
    relationship: &carer_relationship::Model,
    member_id: i64,
    level: &str,
    kind: &str,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    let active = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(member_id))
        .filter(grant::Column::PersonId.eq(relationship.patient_id))
        .filter(grant::Column::RevokedAt.is_null())
        .lock_exclusive()
        .one(tenant.transaction())
        .await?;
    if let Some(row) = active {
        if row
            .expires_at
            .is_some_and(|expiry| expiry <= Utc::now().naive_utc())
        {
            grants::revoke_owned(tenant, row, provenance).await?;
        } else if row.carer_relationship_id != Some(relationship.id) {
            let rank = |level: &str| match level {
                "manage" => 3,
                "record" => 2,
                "view" => 1,
                _ => 0,
            };
            if rank(&row.access_level) >= rank(level) && row.expires_at.is_none() {
                return Ok(());
            }
            return Err(invalid(
                "base",
                "existing manual grant does not cover the delegated access level",
            ));
        }
    }
    upsert(
        tenant,
        member_id,
        relationship.patient_id,
        level,
        kind,
        Some(relationship.id),
        provenance,
    )
    .await
}
async fn upsert(
    tenant: &TenantTransaction,
    member_id: i64,
    person_id: i64,
    level: &str,
    kind: &str,
    relationship_id: Option<i64>,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    let query = grant::Entity::find()
        .filter(grant::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(grant::Column::HouseholdMembershipId.eq(member_id))
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
        let query = if let Some(id) = relationship_id {
            query.filter(grant::Column::CarerRelationshipId.eq(id))
        } else {
            query
        };
        query
            .order_by_desc(grant::Column::Id)
            .lock_exclusive()
            .one(tenant.transaction())
            .await?
    };
    let now = Utc::now().naive_utc();
    let before = existing.as_ref().map(grants::state);
    if existing.as_ref().is_some_and(|row| {
        row.access_level == level
            && row.relationship_type == kind
            && row.expires_at.is_none()
            && row.revoked_at.is_none()
            && row.carer_relationship_id == relationship_id
    }) {
        return Ok(());
    }
    let row = if let Some(row) = existing {
        let mut active: grant::ActiveModel = row.into();
        active.access_level = Set(level.into());
        active.relationship_type = Set(kind.into());
        active.expires_at = Set(None);
        active.revoked_at = Set(None);
        active.carer_relationship_id = Set(relationship_id);
        active.updated_at = Set(now);
        active.update(tenant.transaction()).await?
    } else {
        grant::ActiveModel {
            household_id: Set(tenant.scope().household_id),
            household_membership_id: Set(member_id),
            person_id: Set(person_id),
            access_level: Set(level.into()),
            relationship_type: Set(kind.into()),
            expires_at: Set(None),
            revoked_at: Set(None),
            carer_relationship_id: Set(relationship_id),
            granted_by_membership_id: Set(Some(tenant.membership().id)),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(tenant.transaction())
        .await?
    };
    persistence::bump(tenant, member_id).await?;
    grants::record(tenant, &row, before, "success", provenance).await
}
pub async fn deactivate(
    tenant: &TenantTransaction,
    id: i64,
    provenance: Option<&CredentialProvenance>,
) -> Result<carer_relationship::Model, OperationError> {
    authorize(tenant).await?;
    let row = carer_relationship::Entity::find_by_id(id)
        .filter(carer_relationship::Column::HouseholdId.eq(tenant.scope().household_id))
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    if !(row.carer_id == row.patient_id && row.relationship_type.as_deref() == Some("self")) {
        let grants = grant::Entity::find()
            .filter(grant::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(grant::Column::CarerRelationshipId.eq(id))
            .filter(grant::Column::RevokedAt.is_null())
            .lock_exclusive()
            .all(tenant.transaction())
            .await?;
        for grant in grants {
            grants::revoke_owned(tenant, grant, provenance).await?;
        }
    }
    if !row.active {
        return Ok(row);
    }
    let before = snapshot(&row);
    let mut active: carer_relationship::ActiveModel = row.into();
    active.active = Set(false);
    active.updated_at = Set(Utc::now().naive_utc());
    let row = active.update(tenant.transaction()).await?;
    persistence::record_version(
        tenant,
        "CarerRelationship",
        row.id,
        Some(before),
        snapshot(&row),
        provenance,
    )
    .await?;
    Ok(row)
}
pub async fn reactivate(
    tenant: &TenantTransaction,
    id: i64,
    provenance: Option<&CredentialProvenance>,
) -> Result<carer_relationship::Model, OperationError> {
    authorize(tenant).await?;
    let row = carer_relationship::Entity::find_by_id(id)
        .filter(carer_relationship::Column::HouseholdId.eq(tenant.scope().household_id))
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    assign(
        tenant,
        row.carer_id,
        row.patient_id,
        row.relationship_type
            .as_deref()
            .ok_or(OperationError::Unavailable)?,
        provenance,
    )
    .await
}
