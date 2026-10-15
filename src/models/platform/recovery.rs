use super::*;
use crate::models::ids::{AccountId, HouseholdId, HouseholdMembershipId};

pub struct RecoveryMember {
    pub id: HouseholdMembershipId,
    pub name: String,
    pub email: String,
}

pub struct RecoveryHousehold {
    pub id: HouseholdId,
    pub name: String,
    pub slug: String,
    pub has_owner: bool,
    pub members: Vec<RecoveryMember>,
    pub member_page: i64,
    pub member_pages: i64,
    pub member_page_links: Vec<i64>,
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

async fn eligible_members(
    transaction: &DatabaseTransaction,
    household_id: HouseholdId,
    requested_page: i64,
) -> Result<(Vec<RecoveryMember>, i64, i64), OperationError> {
    let rows = transaction
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.platform_recovery_members($1,$2)",
            [household_id.get().into(), requested_page.into()],
        ))
        .await?;
    let total: i64 = rows
        .first()
        .ok_or(OperationError::Unavailable)?
        .try_get("", "total")?;
    let pages = ((total + 49) / 50).max(1);
    let mut members = Vec::new();
    for row in rows {
        if let Some(id) = row.try_get::<Option<i64>>("", "id")? {
            members.push(RecoveryMember {
                id: HouseholdMembershipId::from_database(id),
                name: row.try_get("", "name")?,
                email: row.try_get("", "email")?,
            });
        }
    }
    Ok((members, requested_page.clamp(1, pages), pages))
}

fn household_entry(row: &QueryResult) -> Result<RecoveryHousehold, OperationError> {
    Ok(RecoveryHousehold {
        id: HouseholdId::from_database(row.try_get("", "id")?),
        name: row.try_get("", "name")?,
        slug: row.try_get("", "slug")?,
        has_owner: row.try_get("", "has_owner")?,
        members: Vec::new(),
        member_page: 1,
        member_pages: 1,
        member_page_links: Vec::new(),
    })
}

pub async fn page(
    transaction: &DatabaseTransaction,
    actor_account_id: AccountId,
    search: Option<&str>,
) -> Result<Vec<RecoveryHousehold>, OperationError> {
    authorize(
        transaction,
        actor_account_id.get(),
        "platform_owner_recovery",
    )
    .await?;
    let escaped = search
        .unwrap_or_default()
        .trim()
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    transaction
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.platform_recovery_households($1,NULL)",
            [format!("%{escaped}%").into()],
        ))
        .await?
        .iter()
        .map(household_entry)
        .collect()
}

pub async fn detail(
    transaction: &DatabaseTransaction,
    actor_account_id: AccountId,
    household_id: HouseholdId,
    requested_page: i64,
) -> Result<Option<RecoveryHousehold>, OperationError> {
    authorize(
        transaction,
        actor_account_id.get(),
        "platform_owner_recovery",
    )
    .await?;
    let Some(row) = transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT * FROM med_tracker.platform_recovery_households('%',$1)",
            [household_id.get().into()],
        ))
        .await?
    else {
        return Ok(None);
    };
    let mut household = household_entry(&row)?;
    if !household.has_owner {
        let (members, page, pages) =
            eligible_members(transaction, household_id, requested_page).await?;
        household.members = members;
        household.member_page = page;
        household.member_pages = pages;
        household.member_page_links = page_links(page, pages);
    }
    Ok(Some(household))
}

fn recovery_error(error: sea_orm::DbErr) -> OperationError {
    if let sea_orm::DbErr::Exec(sea_orm::RuntimeErr::SqlxError(ref source)) = error
        && let Some(database) = source.as_database_error()
    {
        match database.code().as_deref() {
            Some("MT001") => return OperationError::Forbidden,
            Some("MT002") => return invalid("The recovery target or reason is unavailable."),
            Some("MT003") => {
                return OperationError::Conflict {
                    code: "household_has_owner".to_owned(),
                    details: json!({"error":"The household already has an owner."}),
                };
            }
            Some("MT004") => return OperationError::NotFound,
            _ => {}
        }
    }
    OperationError::from(error)
}

pub async fn check(
    transaction: &DatabaseTransaction,
    actor_account_id: AccountId,
    household_id: HouseholdId,
    membership_id: HouseholdMembershipId,
    reason: &str,
) -> Result<(), OperationError> {
    authorize(
        transaction,
        actor_account_id.get(),
        "platform_owner_recovery",
    )
    .await?;
    let reason = valid_reason(reason)?;
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT med_tracker.check_household_owner_recovery($1,$2,$3)",
            [
                household_id.get().into(),
                membership_id.get().into(),
                reason.into(),
            ],
        ))
        .await
        .map_err(recovery_error)?;
    Ok(())
}

pub async fn promote(
    transaction: &DatabaseTransaction,
    actor_account_id: AccountId,
    household_id: HouseholdId,
    membership_id: HouseholdMembershipId,
    reason: &str,
    meta: &RequestMeta,
) -> Result<(), OperationError> {
    serialize(transaction).await?;
    authorize(
        transaction,
        actor_account_id.get(),
        "platform_owner_recovery",
    )
    .await?;
    let reason = valid_reason(reason)?;
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT med_tracker.recover_household_owner($1,$2,$3,$4,$5,$6)",
            [
                household_id.get().into(),
                membership_id.get().into(),
                reason.into(),
                meta.session_reference.clone().into(),
                meta.request_id.clone().into(),
                meta.ip.clone().into(),
            ],
        ))
        .await
        .map_err(recovery_error)?;
    Ok(())
}
