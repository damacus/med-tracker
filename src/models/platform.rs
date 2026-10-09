use super::{
    access, authorization,
    entities::{account, platform_admin},
    errors::OperationError,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, EntityTrait,
    QueryFilter, QueryResult, Statement, TransactionTrait,
};
use serde_json::json;

const PER_PAGE: i64 = 25;
const PLATFORM_LOCK: i32 = 3;

pub(crate) mod recovery;
pub(crate) mod support;

pub struct RequestMeta {
    pub session_reference: Option<String>,
    pub request_id: Option<String>,
    pub ip: Option<String>,
}

pub(super) struct AuditContext {
    pub account_id: i64,
    pub user_id: i64,
    pub role: &'static str,
    pub household_id: Option<i64>,
    pub session_reference: Option<String>,
    pub request_id: Option<String>,
    pub ip: Option<String>,
}

impl AuditContext {
    pub fn administrator(
        account_id: i64,
        user_id: i64,
        session_reference: Option<&str>,
        request_id: Option<&str>,
        ip: Option<&str>,
    ) -> Self {
        Self {
            account_id,
            user_id,
            role: "platform_admin",
            household_id: None,
            session_reference: session_reference.map(ToOwned::to_owned),
            request_id: request_id.map(ToOwned::to_owned),
            ip: ip.map(ToOwned::to_owned),
        }
    }

    pub fn owner(
        account_id: i64,
        user_id: i64,
        household_id: i64,
        session_reference: Option<&str>,
        request_id: Option<&str>,
        ip: Option<&str>,
    ) -> Self {
        Self {
            role: "household_owner",
            household_id: Some(household_id),
            ..Self::administrator(account_id, user_id, session_reference, request_id, ip)
        }
    }

    pub fn for_household(mut self, household_id: i64) -> Self {
        self.household_id = Some(household_id);
        self
    }
}

impl RequestMeta {
    pub(super) fn administrator(&self, account_id: i64, user_id: i64) -> AuditContext {
        AuditContext::administrator(
            account_id,
            user_id,
            self.session_reference.as_deref(),
            self.request_id.as_deref(),
            self.ip.as_deref(),
        )
    }

    pub(super) fn owner(&self, account_id: i64, user_id: i64, household_id: i64) -> AuditContext {
        AuditContext::owner(
            account_id,
            user_id,
            household_id,
            self.session_reference.as_deref(),
            self.request_id.as_deref(),
            self.ip.as_deref(),
        )
    }
}

pub struct Administrator {
    pub admin: platform_admin::Model,
    pub user_id: i64,
}

pub struct UserEntry {
    pub id: i64,
    pub email: String,
    pub status: i32,
    pub user_active: bool,
    pub administrator: Option<String>,
}

impl UserEntry {
    fn from_row(row: &QueryResult) -> Result<Self, OperationError> {
        Ok(Self {
            id: row
                .try_get("", "id")
                .map_err(|_| OperationError::Unavailable)?,
            email: row
                .try_get("", "email")
                .map_err(|_| OperationError::Unavailable)?,
            status: row
                .try_get("", "status")
                .map_err(|_| OperationError::Unavailable)?,
            user_active: row
                .try_get("", "user_active")
                .map_err(|_| OperationError::Unavailable)?,
            administrator: row
                .try_get("", "administrator")
                .map_err(|_| OperationError::Unavailable)?,
        })
    }
}

pub struct UsersPage {
    pub users: Vec<UserEntry>,
    pub page: i64,
    pub pages: i64,
    pub page_links: Vec<i64>,
    pub per_page: i64,
    pub total: i64,
    pub query: String,
}

pub async fn begin(db: &DatabaseConnection) -> Result<DatabaseTransaction, OperationError> {
    let transaction = db.begin().await.map_err(|_| OperationError::Unavailable)?;
    transaction
        .execute_unprepared("SET LOCAL ROLE med_tracker_app")
        .await
        .map_err(|_| OperationError::Unavailable)?;
    Ok(transaction)
}

async fn authorize(
    transaction: &DatabaseTransaction,
    account_id: i64,
    action: &str,
) -> Result<Administrator, OperationError> {
    let user_id = access::verify_account_actor(transaction, account_id).await?;
    let record = account::Entity::find_by_id(account_id)
        .one(transaction)
        .await
        .map_err(|_| OperationError::Unavailable)?
        .ok_or(OperationError::Unauthenticated)?;
    let Some(admin) = platform_admin::Entity::find()
        .filter(platform_admin::Column::AccountId.eq(account_id))
        .one(transaction)
        .await
        .map_err(|_| OperationError::Unavailable)?
    else {
        return Err(OperationError::Forbidden);
    };
    let allowed = authorization::platform_access(&record, &admin, action);
    if !allowed {
        return Err(OperationError::Forbidden);
    }
    Ok(Administrator { admin, user_id })
}

pub async fn administrator(
    transaction: &DatabaseTransaction,
    account_id: i64,
) -> Result<Administrator, OperationError> {
    authorize(transaction, account_id, "read_platform_users").await
}

pub async fn users(
    transaction: &DatabaseTransaction,
    query: &str,
    page: i64,
) -> Result<UsersPage, OperationError> {
    let search = query.trim().to_owned();
    let escaped = search
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let pattern = format!("%{escaped}%");
    let page = page.max(1);
    let count = transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM accounts WHERE email ILIKE $1 ESCAPE '\\'",
            [pattern.clone().into()],
        ))
        .await
        .map_err(|_| OperationError::Unavailable)?
        .ok_or(OperationError::Unavailable)?;
    let total: i64 = count
        .try_get("", "count")
        .map_err(|_| OperationError::Unavailable)?;
    let pages = (total / PER_PAGE + i64::from(total % PER_PAGE > 0)).max(1);
    let rows = transaction
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT a.id,a.email,a.status,EXISTS(SELECT 1 FROM public.people p JOIN public.users u ON u.person_id=p.id AND u.active WHERE p.account_id=a.id) AS user_active,pa.status AS administrator FROM accounts a LEFT JOIN platform_admins pa ON pa.account_id=a.id WHERE a.email ILIKE $1 ESCAPE '\\' ORDER BY a.id LIMIT $2 OFFSET $3",
            [
                pattern.into(),
                PER_PAGE.into(),
                ((page - 1) * PER_PAGE).into(),
            ],
        ))
        .await
        .map_err(|_| OperationError::Unavailable)?;
    let users = rows
        .iter()
        .map(UserEntry::from_row)
        .collect::<Result<Vec<_>, OperationError>>()?;
    Ok(UsersPage {
        users,
        page,
        pages,
        page_links: page_links(page, pages),
        per_page: PER_PAGE,
        total,
        query: search,
    })
}

fn page_links(page: i64, pages: i64) -> Vec<i64> {
    if pages <= 1 {
        return Vec::new();
    }
    let window = 7.min(pages);
    let current = page.clamp(1, pages);
    let start = (current - window / 2).max(1);
    let end = (start + window - 1).min(pages);
    ((end - window + 1).max(1)..=end).collect()
}

async fn serialize(transaction: &DatabaseTransaction) -> Result<(), OperationError> {
    transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT pg_advisory_xact_lock(1920296809,$1) AS locked",
            [PLATFORM_LOCK.into()],
        ))
        .await
        .map_err(|_| OperationError::Unavailable)?
        .ok_or(OperationError::Unavailable)?;
    Ok(())
}

async fn target_account(
    transaction: &DatabaseTransaction,
    account_id: i64,
) -> Result<account::Model, OperationError> {
    account::Entity::find_by_id(account_id)
        .one(transaction)
        .await
        .map_err(|_| OperationError::Unavailable)?
        .ok_or(OperationError::NotFound)
}

async fn exists(
    transaction: &DatabaseTransaction,
    sql: &str,
    account_id: i64,
) -> Result<bool, OperationError> {
    transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [account_id.into()],
        ))
        .await
        .map_err(|_| OperationError::Unavailable)?
        .ok_or(OperationError::Unavailable)?
        .try_get::<bool>("", "present")
        .map_err(|_| OperationError::Unavailable)
}

async fn has_active_user(
    transaction: &DatabaseTransaction,
    account_id: i64,
) -> Result<bool, OperationError> {
    exists(
        transaction,
        "SELECT EXISTS(SELECT 1 FROM public.people p JOIN public.users u ON u.person_id=p.id AND u.active WHERE p.account_id=$1) AS present",
        account_id,
    )
    .await
}

async fn has_user(
    transaction: &DatabaseTransaction,
    account_id: i64,
) -> Result<bool, OperationError> {
    exists(
        transaction,
        "SELECT EXISTS(SELECT 1 FROM public.people p JOIN public.users u ON u.person_id=p.id WHERE p.account_id=$1) AS present",
        account_id,
    )
    .await
}

async fn viable_administrator(
    transaction: &DatabaseTransaction,
    account_id: i64,
) -> Result<bool, OperationError> {
    exists(
        transaction,
        "SELECT EXISTS(SELECT 1 FROM public.platform_admins pa JOIN public.accounts a ON a.id=pa.account_id WHERE pa.account_id=$1 AND pa.status='active' AND a.status=2 AND EXISTS(SELECT 1 FROM public.people p JOIN public.users u ON u.person_id=p.id AND u.active WHERE p.account_id=a.id)) AS present",
        account_id,
    )
    .await
}

async fn viable_administrators(transaction: &DatabaseTransaction) -> Result<i64, OperationError> {
    transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM public.platform_admins pa JOIN public.accounts a ON a.id=pa.account_id WHERE pa.status='active' AND a.status=2 AND EXISTS(SELECT 1 FROM public.people p JOIN public.users u ON u.person_id=p.id AND u.active WHERE p.account_id=a.id)",
            [],
        ))
        .await
        .map_err(|_| OperationError::Unavailable)?
        .ok_or(OperationError::Unavailable)?
        .try_get::<i64>("", "count")
        .map_err(|_| OperationError::Unavailable)
}

async fn require_remaining_administrator(
    transaction: &DatabaseTransaction,
    target_account_id: i64,
) -> Result<(), OperationError> {
    if viable_administrator(transaction, target_account_id).await?
        && viable_administrators(transaction).await? <= 1
    {
        return Err(OperationError::Conflict {
            code: "last_platform_administrator".to_owned(),
            details: json!({
                "error": "At least one usable platform administrator must remain."
            }),
        });
    }
    Ok(())
}

pub(super) async fn account_scope(
    transaction: &DatabaseTransaction,
    account_id: i64,
) -> Result<(), OperationError> {
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT set_config('med_tracker.current_account_id',$1,true)",
            [account_id.to_string().into()],
        ))
        .await
        .map_err(|_| OperationError::Unavailable)?;
    Ok(())
}

pub(super) async fn household_scope(
    transaction: &DatabaseTransaction,
    household_id: i64,
) -> Result<(), OperationError> {
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT set_config('med_tracker.current_household_id',$1,true),set_config('med_tracker.current_membership_id','',true)",
            [household_id.to_string().into()],
        ))
        .await
        .map_err(|_| OperationError::Unavailable)?;
    Ok(())
}

pub(super) async fn record(
    transaction: &DatabaseTransaction,
    item_type: &str,
    item_id: i64,
    event: &str,
    metadata: serde_json::Value,
    audit: &AuditContext,
) -> Result<(), OperationError> {
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT set_config('med_tracker.current_account_id',$1,true),set_config('med_tracker.current_household_id',$2,true),set_config('med_tracker.current_membership_id','',true)",
            [
                audit.account_id.to_string().into(),
                audit
                    .household_id
                    .map(|id| id.to_string())
                    .unwrap_or_default()
                    .into(),
            ],
        ))
        .await
        .map_err(|_| OperationError::Unavailable)?;
    let context = json!({
        "actor_account_id": audit.account_id,
        "actor_user_id": audit.user_id,
        "active_role": audit.role,
        "authentication_method": "browser_session",
        "session_reference": audit.session_reference,
        "request_id": audit.request_id,
        "ip": audit.ip,
    });
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO versions (item_type,item_id,event,object,whodunnit,request_id,audit_context,household_id,created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,timezone('UTC',clock_timestamp()))",
            [
                item_type.into(),
                item_id.into(),
                event.into(),
                metadata.to_string().into(),
                Some(audit.user_id.to_string()).into(),
                audit.request_id.clone().into(),
                context.into(),
                audit.household_id.into(),
            ],
        ))
        .await
        .map_err(|_| OperationError::Unavailable)?;
    Ok(())
}

async fn grantable(
    transaction: &DatabaseTransaction,
    target: &account::Model,
) -> Result<bool, OperationError> {
    Ok(target.status == 2 && has_active_user(transaction, target.id).await?)
}

pub async fn check_rights(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    target_account_id: i64,
    grant: bool,
) -> Result<(), OperationError> {
    authorize(transaction, actor_account_id, "write_platform_users").await?;
    let target = target_account(transaction, target_account_id).await?;
    if grant && !grantable(transaction, &target).await? {
        return Err(OperationError::Validation {
            details: json!({"error":"The account is not eligible for platform rights."}),
        });
    }
    Ok(())
}

pub async fn check_user(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    target_account_id: i64,
    active: bool,
) -> Result<(), OperationError> {
    authorize(transaction, actor_account_id, "write_platform_users").await?;
    let target = target_account(transaction, target_account_id).await?;
    if active && (target.status != 2 || !has_user(transaction, target_account_id).await?) {
        return Err(OperationError::Validation {
            details: json!({"error":"Only an open verified account with a linked user can be enabled."}),
        });
    }
    Ok(())
}

pub async fn change_rights(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    target_account_id: i64,
    grant: bool,
    session_reference: Option<&str>,
    request_id: Option<&str>,
    ip: Option<&str>,
) -> Result<(), OperationError> {
    serialize(transaction).await?;
    let actor = authorize(transaction, actor_account_id, "write_platform_users").await?;
    let target = target_account(transaction, target_account_id).await?;
    if grant {
        if !grantable(transaction, &target).await? {
            return Err(OperationError::Validation {
                details: json!({"error":"The account is not eligible for platform rights."}),
            });
        }
        transaction
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO public.platform_admins (account_id,status,created_at,updated_at) VALUES ($1,'active',timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp())) ON CONFLICT (account_id) DO UPDATE SET status='active',updated_at=timezone('UTC',clock_timestamp())",
                [target_account_id.into()],
            ))
            .await
            .map_err(|_| OperationError::Unavailable)?;
        record(
            transaction,
            "Account",
            target_account_id,
            "platform/administrator/granted",
            json!({"account_id":target_account_id,"grant":true}),
            &AuditContext::administrator(
                actor_account_id,
                actor.user_id,
                session_reference,
                request_id,
                ip,
            ),
        )
        .await?;
        return Ok(());
    }
    let admin = platform_admin::Entity::find()
        .filter(platform_admin::Column::AccountId.eq(target_account_id))
        .one(transaction)
        .await
        .map_err(|_| OperationError::Unavailable)?
        .ok_or(OperationError::NotFound)?;
    if admin.status != "active" {
        return Ok(());
    }
    require_remaining_administrator(transaction, target_account_id).await?;
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE public.platform_admins SET status='disabled',updated_at=timezone('UTC',clock_timestamp()) WHERE account_id=$1 AND status='active'",
            [target_account_id.into()],
        ))
        .await
        .map_err(|_| OperationError::Unavailable)?;
    record(
        transaction,
        "Account",
        target_account_id,
        "platform/administrator/revoked",
        json!({"account_id":target_account_id,"grant":false}),
        &AuditContext::administrator(
            actor_account_id,
            actor.user_id,
            session_reference,
            request_id,
            ip,
        ),
    )
    .await
}

pub async fn change_user(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    target_account_id: i64,
    active: bool,
    session_reference: Option<&str>,
    request_id: Option<&str>,
    ip: Option<&str>,
) -> Result<(), OperationError> {
    serialize(transaction).await?;
    let actor = authorize(transaction, actor_account_id, "write_platform_users").await?;
    let target = target_account(transaction, target_account_id).await?;
    if active {
        if target.status != 2 || !has_user(transaction, target_account_id).await? {
            return Err(OperationError::Validation {
                details: json!({"error":"Only an open verified account with a linked user can be enabled."}),
            });
        }
        transaction
            .execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "UPDATE public.users SET active=true,updated_at=timezone('UTC',clock_timestamp()) WHERE person_id IN (SELECT id FROM public.people WHERE account_id=$1)",
                [target_account_id.into()],
            ))
            .await
            .map_err(|_| OperationError::Unavailable)?;
        return record(
            transaction,
            "Account",
            target_account_id,
            "platform/user/activated",
            json!({"account_id":target_account_id,"active":true}),
            &AuditContext::administrator(
                actor_account_id,
                actor.user_id,
                session_reference,
                request_id,
                ip,
            ),
        )
        .await;
    }
    require_remaining_administrator(transaction, target_account_id).await?;
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE public.users SET active=false,updated_at=timezone('UTC',clock_timestamp()) WHERE person_id IN (SELECT id FROM public.people WHERE account_id=$1)",
            [target_account_id.into()],
        ))
        .await
        .map_err(|_| OperationError::Unavailable)?;
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT set_config('med_tracker.current_account_id',$1,true)",
            [target_account_id.to_string().into()],
        ))
        .await
        .map_err(|_| OperationError::Unavailable)?;
    transaction
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE public.identity_sessions SET active=false,updated_at=clock_timestamp() WHERE account_id=$1",
            [target_account_id.into()],
        ))
        .await
        .map_err(|_| OperationError::Unavailable)?;
    record(
        transaction,
        "Account",
        target_account_id,
        "platform/user/deactivated",
        json!({"account_id":target_account_id,"active":false}),
        &AuditContext::administrator(
            actor_account_id,
            actor.user_id,
            session_reference,
            request_id,
            ip,
        ),
    )
    .await
}

pub async fn guard_closure(
    transaction: &DatabaseTransaction,
    account_id: i64,
) -> Result<(), OperationError> {
    serialize(transaction).await?;
    require_remaining_administrator(transaction, account_id).await
}

const SETTINGS_LOCK: i32 = 1;
pub const LOOKUP_SOURCES: [&str; 7] = [
    "imported_catalog",
    "local_nhs_dmd",
    "cached_open_products_facts",
    "open_products_facts",
    "curated_catalog",
    "nhs_dmd",
    "supplements",
];

pub struct Settings {
    pub invite_only: bool,
    pub invite_only_locked: bool,
    pub medicine_lookup_base_url: String,
    pub medicine_lookup_token_url: String,
    pub medicine_lookup_source_priority: Vec<String>,
}

fn env_invite_only() -> Result<Option<bool>, OperationError> {
    match std::env::var("INVITE_ONLY") {
        Ok(value) => Ok(Some(!matches!(
            value.as_str(),
            "" | "0" | "f" | "F" | "false" | "FALSE" | "off" | "OFF"
        ))),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(OperationError::Unavailable),
    }
}

fn stored_sources(row: &QueryResult) -> Result<Vec<String>, OperationError> {
    let value: serde_json::Value = row
        .try_get("", "medicine_lookup_source_priority")
        .map_err(|_| OperationError::Unavailable)?;
    Ok(value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default())
}

async fn settings_lock(transaction: &DatabaseTransaction) -> Result<(), OperationError> {
    transaction
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT pg_advisory_xact_lock(1920296809,$1) AS locked",
            [SETTINGS_LOCK.into()],
        ))
        .await
        .map_err(|_| OperationError::Unavailable)?
        .ok_or(OperationError::Unavailable)?;
    Ok(())
}

const DEFAULT_LOOKUP_BASE_URL: &str = "https://ontology.nhs.uk/production1/fhir";
const DEFAULT_LOOKUP_TOKEN_URL: &str = "https://ontology.nhs.uk/authorisation/auth/realms/nhs-digital-terminology/protocol/openid-connect/token";

fn default_settings(invite_only: bool) -> Settings {
    Settings {
        invite_only,
        invite_only_locked: false,
        medicine_lookup_base_url: DEFAULT_LOOKUP_BASE_URL.to_owned(),
        medicine_lookup_token_url: DEFAULT_LOOKUP_TOKEN_URL.to_owned(),
        medicine_lookup_source_priority: [
            "imported_catalog",
            "local_nhs_dmd",
            "cached_open_products_facts",
            "open_products_facts",
            "curated_catalog",
            "nhs_dmd",
            "supplements",
        ]
        .iter()
        .map(|source| source.to_string())
        .collect(),
    }
}

pub async fn settings(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
) -> Result<Settings, OperationError> {
    authorize(transaction, actor_account_id, "read_platform_settings").await?;
    let row = transaction
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT invite_only,medicine_lookup_base_url,medicine_lookup_token_url,medicine_lookup_source_priority FROM app_settings ORDER BY id LIMIT 1".to_owned(),
        ))
        .await
        .map_err(|_| OperationError::Unavailable)?;
    let Some(row) = row else {
        let has_owner = transaction
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                "SELECT public.registration_has_active_owner() AS has_owner".to_owned(),
            ))
            .await
            .map_err(|_| OperationError::Unavailable)?
            .ok_or(OperationError::Unavailable)?
            .try_get::<bool>("", "has_owner")
            .map_err(|_| OperationError::Unavailable)?;
        let mut defaults = default_settings(has_owner);
        if let Some(locked) = env_invite_only()? {
            defaults.invite_only = locked;
            defaults.invite_only_locked = true;
        }
        return Ok(defaults);
    };
    let stored_invite_only = row
        .try_get::<bool>("", "invite_only")
        .map_err(|_| OperationError::Unavailable)?;
    let (invite_only, invite_only_locked) = match env_invite_only()? {
        Some(value) => (value, true),
        None => (stored_invite_only, false),
    };
    Ok(Settings {
        invite_only,
        invite_only_locked,
        medicine_lookup_base_url: row
            .try_get::<String>("", "medicine_lookup_base_url")
            .map_err(|_| OperationError::Unavailable)?,
        medicine_lookup_token_url: row
            .try_get::<String>("", "medicine_lookup_token_url")
            .map_err(|_| OperationError::Unavailable)?,
        medicine_lookup_source_priority: stored_sources(&row)?,
    })
}

fn valid_settings(
    invite_only: bool,
    base_url: &str,
    token_url: &str,
    sources: &[String],
) -> Result<(), OperationError> {
    let invalid = || OperationError::Validation {
        details: json!({"error":"The settings values are not allowed."}),
    };
    if let Some(locked) = env_invite_only()?
        && invite_only != locked
    {
        return Err(invalid());
    }
    let https = |value: &str| {
        url::Url::parse(value).is_ok_and(|url| {
            url.scheme() == "https"
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
        })
    };
    if !https(base_url) || !https(token_url) {
        return Err(invalid());
    }
    if sources.is_empty()
        || !sources
            .iter()
            .all(|source| LOOKUP_SOURCES.contains(&source.as_str()))
    {
        return Err(invalid());
    }
    Ok(())
}

pub async fn check_settings(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    invite_only: bool,
    medicine_lookup_base_url: &str,
    medicine_lookup_token_url: &str,
    medicine_lookup_source_priority: &[String],
) -> Result<(), OperationError> {
    authorize(transaction, actor_account_id, "write_platform_settings").await?;
    valid_settings(
        invite_only,
        medicine_lookup_base_url,
        medicine_lookup_token_url,
        medicine_lookup_source_priority,
    )
}

pub async fn change_settings(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    invite_only: bool,
    medicine_lookup_base_url: &str,
    medicine_lookup_token_url: &str,
    medicine_lookup_source_priority: &[String],
    meta: &RequestMeta,
) -> Result<(), OperationError> {
    settings_lock(transaction).await?;
    let actor = authorize(transaction, actor_account_id, "write_platform_settings").await?;
    valid_settings(
        invite_only,
        medicine_lookup_base_url,
        medicine_lookup_token_url,
        medicine_lookup_source_priority,
    )?;
    let priority = serde_json::to_string(medicine_lookup_source_priority)
        .map_err(|_| OperationError::Unavailable)?;
    let existing = transaction
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id FROM app_settings ORDER BY id LIMIT 1 FOR UPDATE".to_owned(),
        ))
        .await
        .map_err(|_| OperationError::Unavailable)?;
    let id = match existing {
        Some(row) => {
            let id: i64 = row
                .try_get("", "id")
                .map_err(|_| OperationError::Unavailable)?;
            transaction
                .execute_raw(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "UPDATE app_settings SET invite_only=$1,medicine_lookup_base_url=$2,medicine_lookup_token_url=$3,medicine_lookup_source_priority=$4::jsonb,updated_at=now() WHERE id=$5",
                    [
                        invite_only.into(),
                        medicine_lookup_base_url.into(),
                        medicine_lookup_token_url.into(),
                        priority.into(),
                        id.into(),
                    ],
                ))
                .await
                .map_err(|_| OperationError::Unavailable)?;
            id
        }
        None => transaction
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO app_settings(invite_only,medicine_lookup_base_url,medicine_lookup_token_url,medicine_lookup_source_priority,created_at,updated_at) VALUES($1,$2,$3,$4::jsonb,now(),now()) RETURNING id",
                [
                    invite_only.into(),
                    medicine_lookup_base_url.into(),
                    medicine_lookup_token_url.into(),
                    priority.into(),
                ],
            ))
            .await
            .map_err(|_| OperationError::Unavailable)?
            .ok_or(OperationError::Unavailable)?
            .try_get::<i64>("", "id")
            .map_err(|_| OperationError::Unavailable)?,
    };
    record(
        transaction,
        "AppSettings",
        id,
        "platform/settings/updated",
        json!({
            "invite_only": invite_only,
            "medicine_lookup_base_url": medicine_lookup_base_url,
            "medicine_lookup_token_url": medicine_lookup_token_url,
            "medicine_lookup_source_priority": medicine_lookup_source_priority,
        }),
        &meta.administrator(actor_account_id, actor.user_id),
    )
    .await
}
