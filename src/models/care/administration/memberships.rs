use super::*;

fn state(row: &membership::Model) -> Value {
    json!({"role":row.role,"status":row.status,"person_id":row.person_id})
}
pub async fn list(tenant: &TenantTransaction) -> Result<Value, OperationError> {
    authorize(tenant).await?;
    let rows = membership::Entity::find()
        .filter(membership::Column::HouseholdId.eq(tenant.scope().household_id))
        .order_by_asc(membership::Column::Id)
        .all(tenant.transaction())
        .await?;
    Ok(json!({"data":values(tenant,&rows).await?}))
}
async fn values(
    tenant: &TenantTransaction,
    rows: &[membership::Model],
) -> Result<Vec<Value>, OperationError> {
    let accounts: HashMap<i64, account::Model> = account::Entity::find()
        .filter(
            account::Column::Id.is_in(rows.iter().map(|row| row.account_id).collect::<Vec<_>>()),
        )
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|row| (row.id, row))
        .collect();
    let ids: Vec<i64> = rows.iter().filter_map(|row| row.person_id).collect();
    let people: HashMap<i64, person::Model> = person::Entity::find()
        .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
        .filter(person::Column::Id.is_in(ids.clone()))
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|row| (row.id, row))
        .collect();
    let users: HashMap<i64, i64> = user::Entity::find()
        .filter(user::Column::PersonId.is_in(ids))
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|row| (row.person_id, row.id))
        .collect();
    rows.iter().map(|row|Ok(json!({"id":row.id,"account_id":row.account_id,"email":accounts.get(&row.account_id).ok_or(OperationError::Unavailable)?.email,"person_id":row.person_id,"person_name":row.person_id.and_then(|id|people.get(&id).map(|row|&row.name)),"user_id":row.person_id.and_then(|id|users.get(&id).copied()),"role":row.role,"status":row.status,"permissions_version":row.permissions_version,"joined_at":row.joined_at.map(|time|time.and_utc().to_rfc3339())}))).collect()
}
pub async fn change(
    tenant: &TenantTransaction,
    id: i64,
    attributes: Value,
    revoke: bool,
    provenance: Option<&CredentialProvenance>,
) -> Result<Value, OperationError> {
    authorize(tenant).await?;
    change_record(tenant, id, attributes, revoke, provenance).await
}
pub(super) async fn change_for_delegation(
    tenant: &TenantTransaction,
    id: i64,
    person_id: i64,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    change_record(
        tenant,
        id,
        json!({"status":"active","person_id":person_id}),
        false,
        provenance,
    )
    .await?;
    Ok(())
}
async fn change_record(
    tenant: &TenantTransaction,
    id: i64,
    attributes: Value,
    revoke: bool,
    provenance: Option<&CredentialProvenance>,
) -> Result<Value, OperationError> {
    let current = membership::Entity::find_by_id(id)
        .filter(membership::Column::HouseholdId.eq(tenant.scope().household_id))
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    let mut next = current.clone();
    if revoke {
        next.status = "revoked".into();
        if next.revoked_at.is_none() {
            next.revoked_at = Some(Utc::now().naive_utc());
        }
    } else {
        let fields = fields(
            &attributes,
            &["role", "status", "person_id"],
            "household_membership",
        )?;
        if let Some(role) = fields.get("role") {
            next.role = role
                .as_str()
                .filter(|value| matches!(*value, "owner" | "administrator" | "member"))
                .ok_or_else(|| invalid("role", "is invalid"))?
                .into();
        }
        if let Some(status) = fields.get("status") {
            next.status = status
                .as_str()
                .filter(|value| matches!(*value, "active" | "suspended" | "revoked"))
                .ok_or_else(|| invalid("status", "is invalid"))?
                .into();
            if next.status == "active" {
                next.revoked_at = None;
            }
        }
        if let Some(person) = fields.get("person_id") {
            next.person_id = if person.is_null() {
                None
            } else {
                Some(
                    person
                        .as_i64()
                        .filter(|id| *id > 0)
                        .ok_or_else(|| invalid("person_id", "is invalid"))?,
                )
            };
        }
    }
    if let Some(person_id) = next.person_id
        && person::Entity::find_by_id(person_id)
            .filter(person::Column::HouseholdId.eq(tenant.scope().household_id))
            .one(tenant.transaction())
            .await?
            .is_none()
    {
        return rejected(
            tenant,
            &current,
            &next,
            "Person must belong to the same household",
            provenance,
        )
        .await;
    }
    if let Some(person_id) = next.person_id
        && membership::Entity::find()
            .filter(membership::Column::HouseholdId.eq(tenant.scope().household_id))
            .filter(membership::Column::PersonId.eq(person_id))
            .filter(membership::Column::Id.ne(id))
            .one(tenant.transaction())
            .await?
            .is_some()
    {
        return rejected(
            tenant,
            &current,
            &next,
            "Person has already been linked to a membership",
            provenance,
        )
        .await;
    }
    let previous_owner = current.role == "owner" && current.status == "active";
    let next_owner = next.role == "owner" && next.status == "active";
    if previous_owner != next_owner {
        let platform = platform_admin::Entity::find()
            .filter(platform_admin::Column::AccountId.eq(tenant.scope().actor.account_id))
            .filter(platform_admin::Column::Status.eq("active"))
            .one(tenant.transaction())
            .await?
            .is_some();
        if !crate::models::authorization::may_change_owner(
            tenant.membership(),
            tenant.scope().household_id,
            platform,
            next_owner,
        ) {
            return rejected(
                tenant,
                &current,
                &next,
                "Owner change is not authorized",
                provenance,
            )
            .await;
        }
        let home = household::Entity::find_by_id(tenant.scope().household_id)
            .one(tenant.transaction())
            .await?
            .ok_or(OperationError::NotFound)?;
        if previous_owner && home.status == "active" && home.lifecycle_state == "active" {
            let owners = membership::Entity::find()
                .filter(membership::Column::HouseholdId.eq(home.id))
                .filter(membership::Column::Role.eq("owner"))
                .filter(membership::Column::Status.eq("active"))
                .filter(membership::Column::Id.ne(id))
                .count(tenant.transaction())
                .await?;
            if owners == 0 {
                return rejected(
                    tenant,
                    &current,
                    &next,
                    "Last active owner cannot be removed",
                    provenance,
                )
                .await;
            }
        }
    }
    let changed =
        state(&current) != state(&next) || (!revoke && current.revoked_at != next.revoked_at);
    if changed {
        let mut active: membership::ActiveModel = current.clone().into();
        active.role = Set(next.role);
        active.status = Set(next.status);
        active.person_id = Set(next.person_id);
        active.revoked_at = Set(next.revoked_at);
        active.updated_at = Set(Utc::now().naive_utc());
        next = active.update(tenant.transaction()).await?;
        persistence::bump(tenant, id).await?;
        next.permissions_version += 1;
    }
    event(
        tenant,
        &current,
        &next,
        if changed { "success" } else { "no_change" },
        provenance,
    )
    .await?;
    Ok(json!({"data":values(tenant,&[next]).await?.remove(0)}))
}
async fn event(
    tenant: &TenantTransaction,
    previous: &membership::Model,
    next: &membership::Model,
    outcome: &str,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    persistence::event(tenant,if previous.role!=next.role{"household_membership.role_updated"}else{"household_access.membership_changed"},json!({"target_account_id":previous.account_id,"target_membership_id":previous.id,"previous_role":previous.role,"new_role":next.role,"previous_state":state(previous),"new_state":state(next),"outcome":outcome}),provenance).await
}
async fn rejected(
    tenant: &TenantTransaction,
    current: &membership::Model,
    next: &membership::Model,
    message: &str,
    provenance: Option<&CredentialProvenance>,
) -> Result<Value, OperationError> {
    event(tenant, current, next, "rejected", provenance).await?;
    Err(invalid("base", message))
}
