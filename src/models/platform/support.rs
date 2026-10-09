use super::*;
use crate::models::care::medications::decimal_string;
use crate::models::entities::platform_admin;

pub struct SupportEntry {
    pub id: i64,
    pub household_id: i64,
    pub household_name: String,
    pub requester: String,
    pub reason: String,
    pub state: &'static str,
    pub expires_at: String,
}

pub struct HouseholdOption {
    pub id: i64,
    pub name: String,
}

pub struct SupportPage {
    pub sessions: Vec<SupportEntry>,
    pub households: Vec<HouseholdOption>,
    pub page: i64,
    pub pages: i64,
    pub page_links: Vec<i64>,
}

pub struct SupportAllocation {
    pub person: String,
    pub medication: String,
    pub dose: String,
    pub max_daily: String,
}

pub struct SupportSchedule {
    pub person: String,
    pub medication: String,
    pub frequency: String,
    pub dose: String,
}

pub struct SupportRead {
    pub household_name: String,
    pub slug: String,
    pub expires_at: String,
    pub members: Vec<String>,
    pub people: Vec<String>,
    pub medications: Vec<String>,
    pub allocations: Vec<SupportAllocation>,
    pub schedules: Vec<SupportSchedule>,
}

pub enum EndedBy {
    Administrator,
    Owner,
}

pub enum SupportDecision {
    Applied,
    Expired,
}

struct SessionRow {
    household_id: i64,
    platform_admin_id: i64,
    household_open: bool,
    ended_at: Option<chrono::NaiveDateTime>,
    expired_at: Option<chrono::NaiveDateTime>,
    expires_at: chrono::NaiveDateTime,
    approved_at: Option<chrono::NaiveDateTime>,
    activated_at: Option<chrono::NaiveDateTime>,
    approved_by_account_id: Option<i64>,
    approved_by_membership_id: Option<i64>,
    approved_permissions_version: Option<i32>,
}

fn invalid(detail: &str) -> OperationError {
    OperationError::Validation {
        details: json!({"error":detail}),
    }
}

fn conflict(detail: &str) -> OperationError {
    OperationError::Conflict {
        code: "support_session_state".to_owned(),
        details: json!({"error":detail}),
    }
}

fn valid_reason(reason: &str) -> Result<String, OperationError> {
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > 2000 {
        return Err(invalid("A support reason is required."));
    }
    Ok(reason.to_owned())
}

fn get<T: sea_orm::TryGetable>(row: &QueryResult, column: &str) -> Result<T, OperationError> {
    row.try_get("", column)
        .map_err(|_| OperationError::Unavailable)
}

async fn db_now(
    transaction: &DatabaseTransaction,
) -> Result<chrono::NaiveDateTime, OperationError> {
    transaction
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT clock_timestamp()::timestamp AS now".to_owned(),
        ))
        .await?
        .ok_or(OperationError::Unavailable)
        .and_then(|row| get::<chrono::NaiveDateTime>(&row, "now"))
}

async fn expire_row(
    transaction: &DatabaseTransaction,
    support_id: i64,
    household_id: i64,
    audit: &AuditContext,
) -> Result<(), OperationError> {
    let changed = transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE public.support_access_sessions SET expired_at=clock_timestamp(),updated_at=clock_timestamp() WHERE id=$1 AND expired_at IS NULL AND ended_at IS NULL AND expires_at <= clock_timestamp()",
            [support_id.into()],
        ))
        .await?;
    if changed.rows_affected() == 0 {
        return Ok(());
    }
    record(
        transaction,
        "SupportAccessSession",
        support_id,
        "platform/support/expired",
        json!({"household_id":household_id}),
        audit,
    )
    .await
}

async fn expire(
    transaction: &DatabaseTransaction,
    support_id: i64,
    session: &SessionRow,
    audit: &AuditContext,
) -> Result<bool, OperationError> {
    if session.expires_at > db_now(transaction).await? {
        return Ok(false);
    }
    if session.expired_at.is_none() {
        expire_row(transaction, support_id, session.household_id, audit).await?;
    }
    Ok(true)
}

fn deadline_label(timestamp: chrono::NaiveDateTime) -> String {
    format!("{} UTC", timestamp.format("%Y-%m-%d %H:%M:%S"))
}

fn frequency_label(frequency: &str) -> String {
    let mut label = frequency.replace('_', " ");
    if let Some(first) = label.get(..1) {
        label.replace_range(..1, &first.to_uppercase());
    }
    label
}

fn dose_label(amount: &str, unit: &str) -> String {
    if amount.is_empty() {
        return unit.to_owned();
    }
    if unit.is_empty() {
        return decimal_string(amount.to_owned());
    }
    format!("{} {}", decimal_string(amount.to_owned()), unit)
}

const SESSION_LIST: &str = "SELECT s.id,s.household_id,h.name AS household_name,s.reason,ra.email AS requester,s.ended_at,s.expired_at,s.activated_at,s.approved_at,(s.expires_at <= clock_timestamp()) AS past,s.expires_at FROM public.support_access_sessions s JOIN public.households h ON h.id=s.household_id JOIN public.platform_admins pa ON pa.id=s.platform_admin_id JOIN public.accounts ra ON ra.id=pa.account_id";

fn entry(row: &QueryResult) -> Result<SupportEntry, OperationError> {
    let ended: Option<chrono::NaiveDateTime> = get(row, "ended_at")?;
    let expired_at: Option<chrono::NaiveDateTime> = get(row, "expired_at")?;
    let past: bool = get(row, "past")?;
    let activated = get::<Option<chrono::NaiveDateTime>>(row, "activated_at")?.is_some();
    let approved = get::<Option<chrono::NaiveDateTime>>(row, "approved_at")?.is_some();
    let state = if ended.is_some() {
        "ended"
    } else if expired_at.is_some() || past {
        "expired"
    } else if activated {
        "active"
    } else if approved {
        "approved"
    } else {
        "requested"
    };
    Ok(SupportEntry {
        id: get(row, "id")?,
        household_id: get(row, "household_id")?,
        household_name: get(row, "household_name")?,
        requester: get(row, "requester")?,
        reason: get(row, "reason")?,
        state,
        expires_at: deadline_label(get::<chrono::NaiveDateTime>(row, "expires_at")?),
    })
}

const SESSION_ROW: &str = "SELECT s.household_id,s.platform_admin_id,s.ended_at,s.expired_at,s.expires_at,s.approved_at,s.activated_at,s.approved_by_account_id,s.approved_by_membership_id,s.approved_permissions_version,(h.status='active' AND h.lifecycle_state='active') AS household_open FROM public.support_access_sessions s JOIN public.households h ON h.id=s.household_id WHERE s.id=$1";

async fn session_row(
    transaction: &DatabaseTransaction,
    support_id: i64,
    locked: bool,
) -> Result<SessionRow, OperationError> {
    let sql = if locked {
        format!("{SESSION_ROW} FOR UPDATE OF s")
    } else {
        SESSION_ROW.to_owned()
    };
    let row = transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            &sql,
            [support_id.into()],
        ))
        .await?
        .ok_or(OperationError::NotFound)?;
    Ok(SessionRow {
        household_id: get(&row, "household_id")?,
        platform_admin_id: get(&row, "platform_admin_id")?,
        household_open: get(&row, "household_open")?,
        ended_at: get(&row, "ended_at")?,
        expired_at: get(&row, "expired_at")?,
        expires_at: get(&row, "expires_at")?,
        approved_at: get(&row, "approved_at")?,
        activated_at: get(&row, "activated_at")?,
        approved_by_account_id: get(&row, "approved_by_account_id")?,
        approved_by_membership_id: get(&row, "approved_by_membership_id")?,
        approved_permissions_version: get(&row, "approved_permissions_version")?,
    })
}

async fn household_open(
    transaction: &DatabaseTransaction,
    household_id: i64,
) -> Result<bool, OperationError> {
    transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT (status='active' AND lifecycle_state='active') AS open FROM public.households WHERE id=$1",
            [household_id.into()],
        ))
        .await?
        .ok_or(OperationError::NotFound)
        .and_then(|row| get::<bool>(&row, "open"))
}

async fn open_household(
    transaction: &DatabaseTransaction,
    household_id: i64,
) -> Result<(), OperationError> {
    if !household_open(transaction, household_id).await? {
        return Err(invalid("The household is not open."));
    }
    Ok(())
}

async fn admin_record(
    transaction: &DatabaseTransaction,
    account_id: i64,
) -> Result<Option<platform_admin::Model>, OperationError> {
    platform_admin::Entity::find()
        .filter(platform_admin::Column::AccountId.eq(account_id))
        .one(transaction)
        .await
        .map_err(|_| OperationError::Unavailable)
}

const OWNER_MEMBER: &str = "SELECT m.id,m.permissions_version FROM public.household_memberships m JOIN public.accounts a ON a.id=m.account_id JOIN public.people p ON p.id=m.person_id AND p.account_id=m.account_id JOIN public.users u ON u.person_id=p.id WHERE m.account_id=$1 AND m.household_id=$2 AND m.role='owner' AND m.status='active' AND m.revoked_at IS NULL AND a.status=2 AND p.person_type=0 AND p.has_capacity AND u.active ORDER BY m.id LIMIT 1";

async fn owner_membership(
    transaction: &DatabaseTransaction,
    account_id: i64,
    household_id: i64,
    locked: bool,
) -> Result<Option<(i64, i32)>, OperationError> {
    account_scope(transaction, account_id).await?;
    let sql = if locked {
        format!("{OWNER_MEMBER} FOR UPDATE OF m")
    } else {
        OWNER_MEMBER.to_owned()
    };
    transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            &sql,
            [account_id.into(), household_id.into()],
        ))
        .await?
        .map(|row| {
            Ok((
                get::<i64>(&row, "id")?,
                get::<i32>(&row, "permissions_version")?,
            ))
        })
        .transpose()
}

async fn approval_current(
    transaction: &DatabaseTransaction,
    session: &SessionRow,
) -> Result<bool, OperationError> {
    let (Some(membership_id), Some(version)) = (
        session.approved_by_membership_id,
        session.approved_permissions_version,
    ) else {
        return Ok(false);
    };
    household_scope(transaction, session.household_id).await?;
    transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT EXISTS(SELECT 1 FROM public.household_memberships m JOIN public.accounts a ON a.id=m.account_id JOIN public.people p ON p.id=m.person_id AND p.account_id=m.account_id JOIN public.users u ON u.person_id=p.id WHERE m.id=$1 AND m.household_id=$2 AND m.account_id=$3 AND m.role='owner' AND m.status='active' AND m.revoked_at IS NULL AND m.permissions_version=$4 AND a.status=2 AND p.person_type=0 AND p.has_capacity AND u.active) AS present",
            [
                membership_id.into(),
                session.household_id.into(),
                session.approved_by_account_id.into(),
                version.into(),
            ],
        ))
        .await?
        .ok_or(OperationError::Unavailable)
        .and_then(|row| get::<bool>(&row, "present"))
}

pub async fn page(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    meta: &RequestMeta,
    requested_page: i64,
) -> Result<SupportPage, OperationError> {
    let actor = authorize(transaction, actor_account_id, "support_request").await?;
    let total: i64 = transaction
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM public.support_access_sessions",
        ))
        .await?
        .ok_or(OperationError::Unavailable)
        .and_then(|row| get(&row, "count"))?;
    let pages = (total / 50 + i64::from(total % 50 > 0)).max(1);
    let page = requested_page.clamp(1, pages);
    let rows = transaction
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!("{SESSION_LIST} ORDER BY s.id DESC LIMIT 50 OFFSET $1"),
            [((page - 1) * 50).into()],
        ))
        .await?;
    let mut sessions = Vec::with_capacity(rows.len());
    for row in &rows {
        if get::<bool>(row, "past")?
            && get::<Option<chrono::NaiveDateTime>>(row, "expired_at")?.is_none()
        {
            let household_id: i64 = get(row, "household_id")?;
            let audit = meta
                .administrator(actor_account_id, actor.user_id)
                .for_household(household_id);
            expire_row(transaction, get(row, "id")?, household_id, &audit).await?;
        }
        sessions.push(entry(row)?);
    }
    let households = transaction
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id,name FROM public.households WHERE status='active' AND lifecycle_state='active' ORDER BY id".to_owned(),
        ))
        .await?
        .iter()
        .map(|row| {
            Ok(HouseholdOption {
                id: get(row, "id")?,
                name: get(row, "name")?,
            })
        })
        .collect::<Result<Vec<_>, OperationError>>()?;
    Ok(SupportPage {
        sessions,
        households,
        page,
        pages,
        page_links: page_links(page, pages),
    })
}

async fn owned_households(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
) -> Result<Vec<i64>, OperationError> {
    transaction
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT m.household_id FROM public.household_memberships m JOIN public.accounts a ON a.id=m.account_id JOIN public.people p ON p.id=m.person_id AND p.account_id=m.account_id JOIN public.users u ON u.person_id=p.id WHERE m.account_id=$1 AND m.role='owner' AND m.status='active' AND m.revoked_at IS NULL AND a.status=2 AND p.person_type=0 AND p.has_capacity AND u.active ORDER BY m.household_id",
            [actor_account_id.into()],
        ))
        .await?
        .iter()
        .map(|row| get::<i64>(row, "household_id"))
        .collect()
}

pub async fn support_link(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
) -> Result<bool, OperationError> {
    access::verify_account_actor(transaction, actor_account_id).await?;
    account_scope(transaction, actor_account_id).await?;
    Ok(!owned_households(transaction, actor_account_id)
        .await?
        .is_empty())
}

pub async fn owner_page(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    meta: &RequestMeta,
) -> Result<Vec<SupportEntry>, OperationError> {
    let user_id = access::verify_account_actor(transaction, actor_account_id).await?;
    account_scope(transaction, actor_account_id).await?;
    let ids = owned_households(transaction, actor_account_id).await?;
    if ids.is_empty() {
        return Err(OperationError::Forbidden);
    }
    let rows = transaction
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!("{SESSION_LIST} WHERE s.household_id = ANY($1) ORDER BY s.id DESC"),
            [ids.into()],
        ))
        .await?;
    let mut sessions = Vec::with_capacity(rows.len());
    for row in &rows {
        let household_id: i64 = get(row, "household_id")?;
        if get::<bool>(row, "past")?
            && get::<Option<chrono::NaiveDateTime>>(row, "expired_at")?.is_none()
        {
            let audit = meta.owner(actor_account_id, user_id, household_id);
            expire_row(transaction, get(row, "id")?, household_id, &audit).await?;
        }
        sessions.push(entry(row)?);
    }
    Ok(sessions)
}

pub async fn check_request(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    household_id: i64,
    reason: &str,
) -> Result<(), OperationError> {
    authorize(transaction, actor_account_id, "support_request").await?;
    valid_reason(reason)?;
    open_household(transaction, household_id).await
}

pub async fn request(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    household_id: i64,
    reason: &str,
    meta: &RequestMeta,
) -> Result<(), OperationError> {
    let actor = authorize(transaction, actor_account_id, "support_request").await?;
    let reason = valid_reason(reason)?;
    open_household(transaction, household_id).await?;
    let now = db_now(transaction).await?;
    let id: i64 = transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO public.support_access_sessions (household_id,platform_admin_id,reason,request_id,ip,starts_at,expires_at,created_at,updated_at) VALUES ($1,$2,$3,$4,$5,$6,$6::timestamp + interval '24 hours',$6,$6) RETURNING id",
            [
                household_id.into(),
                actor.admin.id.into(),
                reason.clone().into(),
                meta.request_id.clone().into(),
                meta.ip.clone().into(),
                now.into(),
            ],
        ))
        .await?
        .ok_or(OperationError::Unavailable)
        .and_then(|row| get::<i64>(&row, "id"))?;
    record(
        transaction,
        "SupportAccessSession",
        id,
        "platform/support/requested",
        json!({"household_id":household_id,"platform_admin_id":actor.admin.id,"reason":reason}),
        &meta
            .administrator(actor_account_id, actor.user_id)
            .for_household(household_id),
    )
    .await
}

async fn approve_allowed(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    household_id: i64,
    locked: bool,
) -> Result<Option<(i64, i32)>, OperationError> {
    let account = target_account(transaction, actor_account_id).await?;
    let admin = admin_record(transaction, actor_account_id).await?;
    let owner = owner_membership(transaction, actor_account_id, household_id, locked).await?;
    if !authorization::support_action(
        &account,
        admin.as_ref(),
        household_id,
        owner.is_some(),
        "support_approve",
    ) {
        return Err(OperationError::Forbidden);
    }
    Ok(owner)
}

pub async fn check_approve(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    support_id: i64,
) -> Result<(), OperationError> {
    access::verify_account_actor(transaction, actor_account_id).await?;
    let session = session_row(transaction, support_id, false).await?;
    approve_allowed(transaction, actor_account_id, session.household_id, false).await?;
    Ok(())
}

pub async fn approve(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    support_id: i64,
    meta: &RequestMeta,
) -> Result<SupportDecision, OperationError> {
    access::verify_account_actor(transaction, actor_account_id).await?;
    let session = session_row(transaction, support_id, true).await?;
    if session.approved_at.is_some() || session.activated_at.is_some() {
        return Err(conflict("The support request was already decided."));
    }
    if session.ended_at.is_some() || session.expired_at.is_some() {
        return Err(conflict("The support request is no longer open."));
    }
    if !session.household_open {
        return Err(invalid("The household is not open."));
    }
    let Some((membership_id, permissions_version)) =
        approve_allowed(transaction, actor_account_id, session.household_id, true).await?
    else {
        return Err(OperationError::Forbidden);
    };
    let user_id = access::verify_account_actor(transaction, actor_account_id).await?;
    if !household_open(transaction, session.household_id).await? {
        return Err(invalid("The household is not open."));
    }
    let expiry_audit = meta.owner(actor_account_id, user_id, session.household_id);
    if expire(transaction, support_id, &session, &expiry_audit).await? {
        return Ok(SupportDecision::Expired);
    }
    let now = db_now(transaction).await?;
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE public.support_access_sessions SET approved_at=$5,approved_by_account_id=$2,approved_by_membership_id=$3,approved_permissions_version=$4,updated_at=$5 WHERE id=$1 AND approved_at IS NULL",
            [
                support_id.into(),
                actor_account_id.into(),
                membership_id.into(),
                permissions_version.into(),
                now.into(),
            ],
        ))
        .await?;
    record(
        transaction,
        "SupportAccessSession",
        support_id,
        "platform/support/approved",
        json!({"household_id":session.household_id,"approved_by_membership_id":membership_id}),
        &meta.owner(actor_account_id, user_id, session.household_id),
    )
    .await?;
    Ok(SupportDecision::Applied)
}

pub async fn check_activate(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    support_id: i64,
) -> Result<(), OperationError> {
    let actor = authorize(transaction, actor_account_id, "support_activate").await?;
    let session = session_row(transaction, support_id, false).await?;
    if session.platform_admin_id != actor.admin.id {
        return Err(OperationError::Forbidden);
    }
    Ok(())
}

pub async fn activate(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    support_id: i64,
    meta: &RequestMeta,
) -> Result<SupportDecision, OperationError> {
    let actor = authorize(transaction, actor_account_id, "support_activate").await?;
    let session = session_row(transaction, support_id, true).await?;
    if session.platform_admin_id != actor.admin.id {
        return Err(OperationError::Forbidden);
    }
    if session.approved_at.is_none() {
        return Err(conflict("The support request is not approved."));
    }
    if session.activated_at.is_some() {
        return Err(conflict("The support request is already active."));
    }
    if session.ended_at.is_some() || session.expired_at.is_some() {
        return Err(conflict("The support request is no longer open."));
    }
    let approved = approval_current(transaction, &session).await?;
    let actor = authorize(transaction, actor_account_id, "support_activate").await?;
    if session.platform_admin_id != actor.admin.id {
        return Err(OperationError::Forbidden);
    }
    if !household_open(transaction, session.household_id).await? || !approved {
        return Err(OperationError::Forbidden);
    }
    let expiry_audit = meta
        .administrator(actor_account_id, actor.user_id)
        .for_household(session.household_id);
    if expire(transaction, support_id, &session, &expiry_audit).await? {
        return Ok(SupportDecision::Expired);
    }
    let now = db_now(transaction).await?;
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE public.support_access_sessions SET activated_at=$2,starts_at=$2,expires_at=$2::timestamp + interval '30 minutes',updated_at=$2 WHERE id=$1 AND activated_at IS NULL",
            [support_id.into(), now.into()],
        ))
        .await?;
    record(
        transaction,
        "SupportAccessSession",
        support_id,
        "platform/support/activated",
        json!({"household_id":session.household_id,"platform_admin_id":actor.admin.id}),
        &meta
            .administrator(actor_account_id, actor.user_id)
            .for_household(session.household_id),
    )
    .await?;
    Ok(SupportDecision::Applied)
}

pub async fn end(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    support_id: i64,
    meta: &RequestMeta,
) -> Result<EndedBy, OperationError> {
    access::verify_account_actor(transaction, actor_account_id).await?;
    let session = session_row(transaction, support_id, true).await?;
    let user_id = access::verify_account_actor(transaction, actor_account_id).await?;
    let account = target_account(transaction, actor_account_id).await?;
    let admin = admin_record(transaction, actor_account_id).await?;
    let administrator = admin
        .as_ref()
        .is_some_and(|admin| admin.id == session.platform_admin_id && admin.status == "active");
    let owner = owner_membership(transaction, actor_account_id, session.household_id, false)
        .await?
        .is_some();
    if !(administrator || owner)
        || !authorization::support_action(
            &account,
            admin.as_ref(),
            session.household_id,
            owner,
            "support_end",
        )
    {
        return Err(OperationError::Forbidden);
    }
    let audit = if administrator {
        meta.administrator(actor_account_id, user_id)
    } else {
        meta.owner(actor_account_id, user_id, session.household_id)
    }
    .for_household(session.household_id);
    if session.ended_at.is_none() {
        if expire(transaction, support_id, &session, &audit).await? {
            return Err(OperationError::Forbidden);
        }
        let now = db_now(transaction).await?;
        transaction
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "UPDATE public.support_access_sessions SET ended_at=$2,updated_at=$2 WHERE id=$1 AND ended_at IS NULL",
                [support_id.into(), now.into()],
            ))
            .await?;
        record(
            transaction,
            "SupportAccessSession",
            support_id,
            "platform/support/ended",
            json!({"household_id":session.household_id}),
            &audit,
        )
        .await?;
    }
    Ok(if administrator {
        EndedBy::Administrator
    } else {
        EndedBy::Owner
    })
}

pub async fn read(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    support_id: i64,
    meta: &RequestMeta,
) -> Result<SupportRead, OperationError> {
    let user_id = access::verify_account_actor(transaction, actor_account_id).await?;
    let session = session_row(transaction, support_id, true).await?;
    let Some(_) = admin_record(transaction, actor_account_id)
        .await?
        .filter(|admin| admin.id == session.platform_admin_id)
    else {
        return Err(OperationError::Forbidden);
    };
    if !session.household_open
        || session.ended_at.is_some()
        || session.approved_at.is_none()
        || session.activated_at.is_none()
    {
        return Err(OperationError::Forbidden);
    }
    let audit = meta
        .administrator(actor_account_id, user_id)
        .for_household(session.household_id);
    if expire(transaction, support_id, &session, &audit).await? {
        return Err(OperationError::Forbidden);
    }
    let approved = approval_current(transaction, &session).await?;
    let user_id = access::verify_account_actor(transaction, actor_account_id).await?;
    let account = target_account(transaction, actor_account_id).await?;
    let Some(admin) = admin_record(transaction, actor_account_id)
        .await?
        .filter(|admin| admin.id == session.platform_admin_id)
    else {
        return Err(OperationError::Forbidden);
    };
    if !household_open(transaction, session.household_id).await? {
        return Err(OperationError::Forbidden);
    }
    let audit = meta
        .administrator(actor_account_id, user_id)
        .for_household(session.household_id);
    if expire(transaction, support_id, &session, &audit).await? || !approved {
        return Err(OperationError::Forbidden);
    }
    let now = db_now(transaction).await?;
    if !authorization::support_household_access(
        &account,
        &admin,
        session.household_id,
        session.ended_at.is_some(),
        session.expired_at.is_some(),
        session.expires_at.and_utc().timestamp_micros(),
        now.and_utc().timestamp_micros(),
    ) {
        return Err(OperationError::Forbidden);
    }
    household_scope(transaction, session.household_id).await?;
    let names = async |sql: &str| {
        transaction
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                sql,
                [session.household_id.into()],
            ))
            .await?
            .iter()
            .map(|row| get::<String>(row, "name"))
            .collect::<Result<Vec<String>, OperationError>>()
    };
    let members = names("SELECT p.name AS name FROM public.household_memberships m JOIN public.people p ON p.id=m.person_id WHERE m.household_id=$1 AND m.status='active' AND m.revoked_at IS NULL ORDER BY m.id").await?;
    let people = names("SELECT name FROM public.people WHERE household_id=$1 ORDER BY id").await?;
    let medications =
        names("SELECT name FROM public.medications WHERE household_id=$1 ORDER BY id").await?;
    let allocations = transaction
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT p.name AS person,m.name AS medication,COALESCE(pm.dose_amount::text,'') AS dose_amount,COALESCE(pm.dose_unit,'') AS dose_unit,COALESCE(pm.max_daily_doses::text,'') AS max_daily FROM public.person_medications pm JOIN public.people p ON p.id=pm.person_id JOIN public.medications m ON m.id=pm.medication_id WHERE pm.household_id=$1 AND pm.active AND pm.retired_at IS NULL ORDER BY p.name,m.name",
            [session.household_id.into()],
        ))
        .await?
        .iter()
        .map(|row| {
            Ok(SupportAllocation {
                person: get(row, "person")?,
                medication: get(row, "medication")?,
                dose: dose_label(
                    &get::<String>(row, "dose_amount")?,
                    &get::<String>(row, "dose_unit")?,
                ),
                max_daily: get(row, "max_daily")?,
            })
        })
        .collect::<Result<Vec<_>, OperationError>>()?;
    let schedules = transaction
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT p.name AS person,m.name AS medication,COALESCE(s.frequency,'') AS frequency,COALESCE(s.dose_amount::text,'') AS dose_amount,COALESCE(s.dose_unit,'') AS dose_unit FROM public.schedules s JOIN public.people p ON p.id=s.person_id JOIN public.medications m ON m.id=s.medication_id WHERE s.household_id=$1 AND s.active AND s.retired_at IS NULL ORDER BY p.name,m.name,s.id",
            [session.household_id.into()],
        ))
        .await?
        .iter()
        .map(|row| {
            Ok(SupportSchedule {
                person: get(row, "person")?,
                medication: get(row, "medication")?,
                frequency: frequency_label(&get::<String>(row, "frequency")?),
                dose: dose_label(
                    &get::<String>(row, "dose_amount")?,
                    &get::<String>(row, "dose_unit")?,
                ),
            })
        })
        .collect::<Result<Vec<_>, OperationError>>()?;
    let household = transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT name,slug FROM public.households WHERE id=$1",
            [session.household_id.into()],
        ))
        .await?
        .ok_or(OperationError::NotFound)?;
    Ok(SupportRead {
        household_name: get(&household, "name")?,
        slug: get(&household, "slug")?,
        expires_at: deadline_label(session.expires_at),
        members,
        people,
        medications,
        allocations,
        schedules,
    })
}
