use super::*;

pub struct RecoveryMember {
    pub id: i64,
    pub name: String,
    pub email: String,
}

pub struct RecoveryHousehold {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub has_owner: bool,
    pub members: Vec<RecoveryMember>,
}

fn invalid(detail: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"error":detail}),
    }
}

fn valid_reason(reason: &str) -> Result<String, OperationError> {
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > 2000 {
        return Err(invalid("A recovery reason is required."));
    }
    Ok(reason.to_owned())
}

async fn open_household(
    transaction: &DatabaseTransaction,
    household_id: i64,
    locked: bool,
) -> Result<(), OperationError> {
    let sql = if locked {
        "SELECT status,lifecycle_state FROM public.households WHERE id=$1 FOR UPDATE"
    } else {
        "SELECT status,lifecycle_state FROM public.households WHERE id=$1"
    };
    let row = transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [household_id.into()],
        ))
        .await?
        .ok_or(OperationError::NotFound)?;
    let status: String = row
        .try_get("", "status")
        .map_err(|_| OperationError::Unavailable)?;
    let lifecycle: String = row
        .try_get("", "lifecycle_state")
        .map_err(|_| OperationError::Unavailable)?;
    if status != "active" || lifecycle != "active" {
        return Err(invalid("The household is not open."));
    }
    Ok(())
}

async fn usable_owner(
    transaction: &DatabaseTransaction,
    household_id: i64,
) -> Result<bool, OperationError> {
    exists(
        transaction,
        "SELECT EXISTS(SELECT 1 FROM public.household_memberships m JOIN public.accounts a ON a.id=m.account_id JOIN public.people p ON p.id=m.person_id AND p.account_id=m.account_id JOIN public.users u ON u.person_id=p.id WHERE m.household_id=$1 AND m.role='owner' AND m.status='active' AND m.revoked_at IS NULL AND a.status=2 AND p.person_type=0 AND p.has_capacity AND u.active) AS present",
        household_id,
    )
    .await
}

async fn eligible_members(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    household_id: i64,
) -> Result<Vec<RecoveryMember>, OperationError> {
    let rows = transaction
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT m.id AS id,p.name AS name,COALESCE(u.email_address,'') AS email FROM public.household_memberships m JOIN public.accounts a ON a.id=m.account_id JOIN public.people p ON p.id=m.person_id AND p.account_id=m.account_id JOIN public.users u ON u.person_id=p.id WHERE m.household_id=$1 AND m.account_id<>$2 AND m.role<>'owner' AND m.status='active' AND m.revoked_at IS NULL AND a.status=2 AND p.person_type=0 AND p.has_capacity AND u.active ORDER BY m.id",
            [household_id.into(), actor_account_id.into()],
        ))
        .await?;
    rows.iter()
        .map(|row| {
            Ok(RecoveryMember {
                id: row
                    .try_get("", "id")
                    .map_err(|_| OperationError::Unavailable)?,
                name: row
                    .try_get("", "name")
                    .map_err(|_| OperationError::Unavailable)?,
                email: row
                    .try_get("", "email")
                    .map_err(|_| OperationError::Unavailable)?,
            })
        })
        .collect()
}

async fn eligible_target(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    household_id: i64,
    membership_id: i64,
) -> Result<(), OperationError> {
    let row = transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT m.account_id,m.household_id,m.role,m.status,m.revoked_at FROM public.household_memberships m WHERE m.id=$1",
            [membership_id.into()],
        ))
        .await?
        .ok_or(OperationError::NotFound)?;
    let member_household: i64 = row
        .try_get("", "household_id")
        .map_err(|_| OperationError::Unavailable)?;
    if member_household != household_id {
        return Err(OperationError::NotFound);
    }
    let member_account: i64 = row
        .try_get("", "account_id")
        .map_err(|_| OperationError::Unavailable)?;
    if member_account == actor_account_id {
        return Err(OperationError::Forbidden);
    }
    let role: String = row
        .try_get("", "role")
        .map_err(|_| OperationError::Unavailable)?;
    let status: String = row
        .try_get("", "status")
        .map_err(|_| OperationError::Unavailable)?;
    let revoked: Option<chrono::NaiveDateTime> = row
        .try_get("", "revoked_at")
        .map_err(|_| OperationError::Unavailable)?;
    if role == "owner" || status != "active" || revoked.is_some() {
        return Err(invalid("The member is not eligible for ownership."));
    }
    let eligible = transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM public.household_memberships m JOIN public.accounts a ON a.id=m.account_id JOIN public.people p ON p.id=m.person_id AND p.account_id=m.account_id JOIN public.users u ON u.person_id=p.id WHERE m.id=$1 AND a.status=2 AND p.person_type=0 AND p.has_capacity AND u.active) AS present",
            [membership_id.into()],
        ))
        .await?
        .ok_or(OperationError::Unavailable)?
        .try_get::<bool>("", "present")
        .map_err(|_| OperationError::Unavailable)?;
    if !eligible {
        return Err(invalid("The member is not eligible for ownership."));
    }
    Ok(())
}

fn household_entry(row: &QueryResult) -> Result<RecoveryHousehold, OperationError> {
    Ok(RecoveryHousehold {
        id: row
            .try_get("", "id")
            .map_err(|_| OperationError::Unavailable)?,
        name: row
            .try_get("", "name")
            .map_err(|_| OperationError::Unavailable)?,
        slug: row
            .try_get("", "slug")
            .map_err(|_| OperationError::Unavailable)?,
        has_owner: false,
        members: Vec::new(),
    })
}

pub async fn page(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    search: Option<&str>,
) -> Result<Vec<RecoveryHousehold>, OperationError> {
    authorize(transaction, actor_account_id, "platform_owner_recovery").await?;
    let statement = match search.map(str::trim).filter(|term| !term.is_empty()) {
        Some(term) => Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT id,name,slug FROM public.households WHERE status='active' AND lifecycle_state='active' AND (name ILIKE '%'||$1||'%' OR slug ILIKE '%'||$1||'%') ORDER BY name,id LIMIT 50",
            [term.to_owned().into()],
        ),
        None => Statement::from_string(
            DbBackend::Postgres,
            "SELECT id,name,slug FROM public.households WHERE status='active' AND lifecycle_state='active' ORDER BY name,id LIMIT 50".to_owned(),
        ),
    };
    transaction
        .query_all_raw(statement)
        .await?
        .iter()
        .map(household_entry)
        .collect()
}

pub async fn detail(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    household_id: i64,
) -> Result<Option<RecoveryHousehold>, OperationError> {
    authorize(transaction, actor_account_id, "platform_owner_recovery").await?;
    let Some(row) = transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT id,name,slug FROM public.households WHERE id=$1 AND status='active' AND lifecycle_state='active'",
            [household_id.into()],
        ))
        .await?
    else {
        return Ok(None);
    };
    household_scope(transaction, household_id).await?;
    let mut household = household_entry(&row)?;
    household.has_owner = usable_owner(transaction, household_id).await?;
    if !household.has_owner {
        household.members = eligible_members(transaction, actor_account_id, household_id).await?;
    }
    Ok(Some(household))
}

pub async fn check(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    household_id: i64,
    membership_id: i64,
    reason: &str,
) -> Result<(), OperationError> {
    authorize(transaction, actor_account_id, "platform_owner_recovery").await?;
    valid_reason(reason)?;
    open_household(transaction, household_id, false).await?;
    household_scope(transaction, household_id).await?;
    if usable_owner(transaction, household_id).await? {
        return Err(invalid("The household already has an owner."));
    }
    eligible_target(transaction, actor_account_id, household_id, membership_id).await
}

pub async fn promote(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    household_id: i64,
    membership_id: i64,
    reason: &str,
    meta: &RequestMeta,
) -> Result<(), OperationError> {
    serialize(transaction).await?;
    let actor = authorize(transaction, actor_account_id, "platform_owner_recovery").await?;
    let reason = valid_reason(reason)?;
    open_household(transaction, household_id, true).await?;
    household_scope(transaction, household_id).await?;
    if usable_owner(transaction, household_id).await? {
        return Err(OperationError::Conflict {
            code: "household_has_owner".to_owned(),
            details: json!({"error":"The household already has an owner."}),
        });
    }
    transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT id FROM public.household_memberships WHERE id=$1 FOR UPDATE",
            [membership_id.into()],
        ))
        .await?
        .ok_or(OperationError::NotFound)?;
    eligible_target(transaction, actor_account_id, household_id, membership_id).await?;
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE public.household_memberships SET role='owner',permissions_version=permissions_version+1,updated_at=timezone('UTC',clock_timestamp()) WHERE id=$1",
            [membership_id.into()],
        ))
        .await?;
    record(
        transaction,
        "HouseholdMembership",
        membership_id,
        "platform/owner_recovered",
        json!({
            "household_id": household_id,
            "membership_id": membership_id,
            "account_id": actor_account_id,
            "reason": reason,
        }),
        &meta
            .administrator(actor_account_id, actor.user_id)
            .for_household(household_id),
    )
    .await
}
