use super::member_browser::MembershipId;
use super::*;
use loco_rs::model::TenantQueryExt;

#[derive(Clone, Copy)]
pub enum LifecycleAction {
    Suspend,
    Restore,
    Revoke,
}
impl LifecycleAction {
    pub fn notice(self) -> &'static str {
        match self {
            Self::Suspend => "suspended",
            Self::Restore => "restored",
            Self::Revoke => "revoked",
        }
    }
    fn allowed(self, current: &str) -> bool {
        matches!(
            (self, current),
            (Self::Suspend | Self::Revoke, "active")
                | (Self::Restore | Self::Revoke, "suspended")
                | (Self::Restore, "revoked")
        )
    }
}

#[derive(Clone, Copy)]
pub enum MembershipRole {
    Owner,
    Administrator,
    Member,
}
impl MembershipRole {
    pub fn parse(value: &str) -> Result<Self, OperationError> {
        match value {
            "owner" => Ok(Self::Owner),
            "administrator" => Ok(Self::Administrator),
            "member" => Ok(Self::Member),
            _ => Err(invalid("role", "is invalid")),
        }
    }
    fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Administrator => "administrator",
            Self::Member => "member",
        }
    }
}

#[derive(Clone, Copy)]
enum MembershipStatus {
    Active,
    Suspended,
    Revoked,
}
impl MembershipStatus {
    fn from_action(action: LifecycleAction) -> Self {
        match action {
            LifecycleAction::Suspend => Self::Suspended,
            LifecycleAction::Restore => Self::Active,
            LifecycleAction::Revoke => Self::Revoked,
        }
    }
    fn parse(value: &str) -> Result<Self, OperationError> {
        match value {
            "active" => Ok(Self::Active),
            "suspended" => Ok(Self::Suspended),
            "revoked" => Ok(Self::Revoked),
            _ => Err(invalid("status", "is invalid")),
        }
    }
    fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Revoked => "revoked",
        }
    }
}

#[derive(Default)]
struct Mutation {
    role: Option<MembershipRole>,
    status: Option<MembershipStatus>,
    person_id: Option<i64>,
    revoke: bool,
    browser_action: Option<LifecycleAction>,
}
impl Mutation {
    fn from_api(attributes: &Value, revoke: bool) -> Result<Self, OperationError> {
        if revoke {
            return Ok(Self {
                revoke: true,
                ..Self::default()
            });
        }
        let fields = fields(
            attributes,
            &["role", "status", "person_id"],
            "household_membership",
        )?;
        let role = fields
            .get("role")
            .map(|value| MembershipRole::parse(value.as_str().unwrap_or_default()))
            .transpose()?;
        let status = fields
            .get("status")
            .map(|value| MembershipStatus::parse(value.as_str().unwrap_or_default()))
            .transpose()?;
        let person_id = fields
            .get("person_id")
            .map(|value| {
                value
                    .as_i64()
                    .filter(|id| *id > 0)
                    .ok_or_else(|| invalid("person_id", "is invalid"))
            })
            .transpose()?;
        Ok(Self {
            role,
            status,
            person_id,
            ..Self::default()
        })
    }
}

fn state(row: &membership::Model) -> Value {
    json!({"role":row.role,"status":row.status,"person_id":row.person_id})
}
pub async fn list(tenant: &TenantTransaction) -> Result<Value, OperationError> {
    authorize(tenant).await?;
    let rows = membership::Entity::find()
        .in_tenant(tenant.scope().household_id)
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
            account::Column::PersonId.is_in(
                rows.iter()
                    .filter(|row| row.status == "active" && row.revoked_at.is_none())
                    .map(|row| row.person_id),
            ),
        )
        .all(tenant.transaction())
        .await?
        .into_iter()
        .map(|row| (row.person_id, row))
        .collect();
    let ids: Vec<i64> = rows.iter().map(|row| row.person_id).collect();
    let people: HashMap<i64, person::Model> =
        crate::models::access::person_queries::household_participants(tenant.scope().household_id)
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
    Ok(rows.iter().map(|row|json!({"id":row.id,"account_id":accounts.get(&row.person_id).map(|account|account.id),"email":accounts.get(&row.person_id).map(|account|&account.email),"person_id":row.person_id,"person_name":people.get(&row.person_id).map(|person|&person.name),"user_id":users.get(&row.person_id),"role":row.role,"status":row.status,"permissions_version":row.permissions_version,"joined_at":row.joined_at.and_utc().to_rfc3339()})).collect())
}
pub async fn change(
    tenant: &TenantTransaction,
    id: i64,
    attributes: Value,
    revoke: bool,
    provenance: Option<&CredentialProvenance>,
) -> Result<Value, OperationError> {
    authorize(tenant).await?;
    let mutation = Mutation::from_api(&attributes, revoke)?;
    let next = change_record(tenant, id, mutation, provenance).await?;
    Ok(json!({"data":values(tenant,&[next]).await?.remove(0)}))
}
pub async fn change_browser_role(
    tenant: &TenantTransaction,
    id: MembershipId,
    role: MembershipRole,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    authorize(tenant).await?;
    change_record(
        tenant,
        id.get(),
        Mutation {
            role: Some(role),
            ..Mutation::default()
        },
        provenance,
    )
    .await?;
    Ok(())
}
pub async fn change_browser_status(
    tenant: &TenantTransaction,
    id: MembershipId,
    action: LifecycleAction,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    authorize(tenant).await?;
    change_record(
        tenant,
        id.get(),
        Mutation {
            status: Some(MembershipStatus::from_action(action)),
            revoke: matches!(action, LifecycleAction::Revoke),
            browser_action: Some(action),
            ..Mutation::default()
        },
        provenance,
    )
    .await?;
    Ok(())
}
pub(super) async fn change_for_delegation(
    tenant: &TenantTransaction,
    id: i64,
    person_id: i64,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    if person_id <= 0 {
        return Err(invalid("person_id", "is invalid"));
    }
    change_record(
        tenant,
        id,
        Mutation {
            status: Some(MembershipStatus::Active),
            person_id: Some(person_id),
            ..Mutation::default()
        },
        provenance,
    )
    .await?;
    Ok(())
}
async fn change_record(
    tenant: &TenantTransaction,
    id: i64,
    mutation: Mutation,
    provenance: Option<&CredentialProvenance>,
) -> Result<membership::Model, OperationError> {
    let current = membership::Entity::find_by_id(id)
        .in_tenant(tenant.scope().household_id)
        .lock_exclusive()
        .one(tenant.transaction())
        .await?
        .ok_or(OperationError::NotFound)?;
    if let Some(action) = mutation.browser_action
        && !action.allowed(&current.status)
    {
        return rejected(
            tenant,
            &current,
            &current,
            "Membership state has changed. Refresh the member list",
            provenance,
        )
        .await;
    }
    let mut next = current.clone();
    if mutation.revoke {
        next.status = "revoked".into();
        if next.revoked_at.is_none() {
            next.revoked_at = Some(Utc::now().naive_utc());
        }
    } else {
        if let Some(role) = mutation.role {
            next.role = role.as_str().into();
        }
        if let Some(status) = mutation.status {
            next.status = status.as_str().into();
            if next.status == "active" {
                next.revoked_at = None;
            }
        }
        if let Some(person_id) = mutation.person_id
            && person_id != current.person_id
        {
            return rejected(
                tenant,
                &current,
                &next,
                "Membership identity cannot be changed",
                provenance,
            )
            .await;
        }
    }
    let previous_owner = current.role == "owner" && current.status == "active";
    let next_owner = next.role == "owner" && next.status == "active";
    if previous_owner != next_owner {
        if !crate::models::authorization::may_change_owner(
            tenant.membership(),
            tenant.scope().household_id,
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
                .in_tenant(home.id)
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
    let changed = state(&current) != state(&next)
        || (!mutation.revoke && current.revoked_at != next.revoked_at);
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
    Ok(next)
}
async fn event(
    tenant: &TenantTransaction,
    previous: &membership::Model,
    next: &membership::Model,
    outcome: &str,
    provenance: Option<&CredentialProvenance>,
) -> Result<(), OperationError> {
    let target = account::Entity::find()
        .filter(account::Column::PersonId.eq(previous.person_id))
        .one(tenant.transaction())
        .await?;
    persistence::event(tenant,if previous.role!=next.role{"household_membership.role_updated"}else{"household_access.membership_changed"},json!({"target_account_id":target.map(|account|account.id),"target_membership_id":previous.id,"previous_role":previous.role,"new_role":next.role,"previous_state":state(previous),"new_state":state(next),"outcome":outcome}),provenance).await
}
async fn rejected<T>(
    tenant: &TenantTransaction,
    current: &membership::Model,
    next: &membership::Model,
    message: &str,
    provenance: Option<&CredentialProvenance>,
) -> Result<T, OperationError> {
    event(tenant, current, next, "rejected", provenance).await?;
    Err(invalid("base", message))
}
