use super::{
    access, authorization,
    entities::{account, platform_admin},
    errors::OperationError,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QueryResult, Statement, TransactionTrait,
    sea_query::Expr,
};
use serde_json::json;

const PER_PAGE: i64 = 25;
const PLATFORM_LOCK: i32 = 3;

pub(crate) mod expiry;
pub(crate) mod recovery;
pub(crate) mod support;

pub struct RequestMeta {
    pub session_reference: Option<String>,
    pub request_id: Option<String>,
    pub ip: Option<String>,
}

pub(super) struct AuditContext {
    pub account_id: i64,
    pub person_id: i64,
    pub role: &'static str,
    pub household_id: Option<i64>,
    pub session_reference: Option<String>,
    pub request_id: Option<String>,
    pub ip: Option<String>,
}

impl AuditContext {
    pub fn administrator(
        account_id: i64,
        person_id: i64,
        session_reference: Option<&str>,
        request_id: Option<&str>,
        ip: Option<&str>,
    ) -> Self {
        Self {
            account_id,
            person_id,
            role: "platform_admin",
            household_id: None,
            session_reference: session_reference.map(ToOwned::to_owned),
            request_id: request_id.map(ToOwned::to_owned),
            ip: ip.map(ToOwned::to_owned),
        }
    }

    pub fn owner(
        account_id: i64,
        person_id: i64,
        household_id: i64,
        session_reference: Option<&str>,
        request_id: Option<&str>,
        ip: Option<&str>,
    ) -> Self {
        Self {
            role: "household_owner",
            household_id: Some(household_id),
            ..Self::administrator(account_id, person_id, session_reference, request_id, ip)
        }
    }

    pub fn for_household(mut self, household_id: i64) -> Self {
        self.household_id = Some(household_id);
        self
    }
}

impl RequestMeta {
    pub(super) fn administrator(&self, account_id: i64, person_id: i64) -> AuditContext {
        AuditContext::administrator(
            account_id,
            person_id,
            self.session_reference.as_deref(),
            self.request_id.as_deref(),
            self.ip.as_deref(),
        )
    }

    pub(super) fn owner(&self, account_id: i64, person_id: i64, household_id: i64) -> AuditContext {
        AuditContext::owner(
            account_id,
            person_id,
            household_id,
            self.session_reference.as_deref(),
            self.request_id.as_deref(),
            self.ip.as_deref(),
        )
    }
}

pub struct Administrator {
    pub admin: platform_admin::Model,
    pub person_id: i64,
}

pub struct UserEntry {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub status: i32,
    pub sign_in_enabled: bool,
    pub administrator: Option<String>,
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
    transaction
        .execute_unprepared("SELECT set_config('med_tracker.current_account_id','',true),set_config('med_tracker.current_household_id','',true),set_config('med_tracker.current_membership_id','',true),set_config('med_tracker.current_invitation_token_digest','',true)")
        .await?;
    Ok(transaction)
}

async fn authorize(
    transaction: &DatabaseTransaction,
    account_id: i64,
    action: &str,
) -> Result<Administrator, OperationError> {
    account_scope(transaction, account_id).await?;
    let person_id = access::verify_account_actor(transaction, account_id).await?;
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
    Ok(Administrator { admin, person_id })
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
    let query = account::Entity::find().filter(account::Column::Email.ilike(pattern));
    let total = i64::try_from(query.clone().count(transaction).await?)
        .map_err(|_| OperationError::Unavailable)?;
    let pages = (total / PER_PAGE + i64::from(total % PER_PAGE > 0)).max(1);
    let page = page.min(pages);
    let rows = query
        .find_also_related(platform_admin::Entity)
        .order_by_asc(account::Column::Id)
        .paginate(transaction, PER_PAGE.unsigned_abs())
        .fetch_page((page - 1).unsigned_abs())
        .await?;
    let users = rows
        .into_iter()
        .map(|(account, administrator)| UserEntry {
            id: account.id,
            name: account.name.unwrap_or_default(),
            email: account.email,
            status: account.status,
            sign_in_enabled: account.active,
            administrator: administrator.map(|administrator| administrator.status),
        })
        .collect();
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

async fn viable_administrator(
    transaction: &DatabaseTransaction,
    account_id: i64,
) -> Result<bool, OperationError> {
    Ok(viable_administrator_query()
        .filter(platform_admin::Column::AccountId.eq(account_id))
        .one(transaction)
        .await?
        .is_some())
}

async fn viable_administrators(transaction: &DatabaseTransaction) -> Result<i64, OperationError> {
    i64::try_from(viable_administrator_query().count(transaction).await?)
        .map_err(|_| OperationError::Unavailable)
}

fn viable_administrator_query() -> sea_orm::Select<platform_admin::Entity> {
    platform_admin::Entity::find()
        .inner_join(account::Entity)
        .filter(platform_admin::Column::Status.eq("active"))
        .filter(account::Column::Status.eq(2))
        .filter(account::Column::Active.eq(true))
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
        "actor_person_id": audit.person_id,
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
                Some(audit.account_id.to_string()).into(),
                audit.request_id.clone().into(),
                context.into(),
                audit.household_id.into(),
            ],
        ))
        .await
        .map_err(|_| OperationError::Unavailable)?;
    Ok(())
}

fn grantable(target: &account::Model) -> bool {
    target.status == 2 && target.active
}

pub async fn check_rights(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    target_account_id: i64,
    grant: bool,
) -> Result<(), OperationError> {
    authorize(transaction, actor_account_id, "write_platform_users").await?;
    let target = target_account(transaction, target_account_id).await?;
    if grant && !grantable(&target) {
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
    if active && target.status != 2 {
        return Err(OperationError::Validation {
            details: json!({"error":"Only an open verified account can be enabled."}),
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
        if !grantable(&target) {
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
                actor.person_id,
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
            actor.person_id,
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
        if target.status != 2 {
            return Err(OperationError::Validation {
                details: json!({"error":"Only an open verified account can be enabled."}),
            });
        }
        set_sign_in_enabled(transaction, target_account_id, true).await?;
        return record(
            transaction,
            "Account",
            target_account_id,
            "platform/user/activated",
            json!({"account_id":target_account_id,"active":true}),
            &AuditContext::administrator(
                actor_account_id,
                actor.person_id,
                session_reference,
                request_id,
                ip,
            ),
        )
        .await;
    }
    require_remaining_administrator(transaction, target_account_id).await?;
    set_sign_in_enabled(transaction, target_account_id, false).await?;
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
            actor.person_id,
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

async fn set_sign_in_enabled(
    transaction: &DatabaseTransaction,
    account_id: i64,
    active: bool,
) -> Result<(), OperationError> {
    account::Entity::update_many()
        .col_expr(account::Column::Active, Expr::value(active))
        .col_expr(account::Column::UpdatedAt, Expr::value(chrono::Utc::now()))
        .filter(account::Column::Id.eq(account_id))
        .exec(transaction)
        .await?;
    Ok(())
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

fn configured_invite_only(
    settings: Option<&serde_json::Value>,
) -> Result<Option<bool>, OperationError> {
    let Some(value) = crate::models::settings::node(settings, &["registration", "invite_only"])
    else {
        return Ok(None);
    };
    match value {
        serde_json::Value::Bool(flag) => Ok(Some(*flag)),
        serde_json::Value::String(value) if !value.is_empty() => Ok(Some(!matches!(
            value.as_str(),
            "0" | "f" | "F" | "false" | "FALSE" | "off" | "OFF"
        ))),
        serde_json::Value::String(_) => Ok(None),
        _ => Err(OperationError::Unavailable),
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
    settings_config: Option<&serde_json::Value>,
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
        if let Some(locked) = configured_invite_only(settings_config)? {
            defaults.invite_only = locked;
            defaults.invite_only_locked = true;
        }
        return Ok(defaults);
    };
    let stored_invite_only = row
        .try_get::<bool>("", "invite_only")
        .map_err(|_| OperationError::Unavailable)?;
    let (invite_only, invite_only_locked) = match configured_invite_only(settings_config)? {
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
    settings_config: Option<&serde_json::Value>,
) -> Result<(), OperationError> {
    let invalid = || OperationError::Validation {
        details: json!({"error":"The settings values are not allowed."}),
    };
    if let Some(locked) = configured_invite_only(settings_config)?
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
    settings_config: Option<&serde_json::Value>,
) -> Result<(), OperationError> {
    authorize(transaction, actor_account_id, "write_platform_settings").await?;
    valid_settings(
        invite_only,
        medicine_lookup_base_url,
        medicine_lookup_token_url,
        medicine_lookup_source_priority,
        settings_config,
    )
}

#[allow(clippy::too_many_arguments)]
pub async fn change_settings(
    transaction: &DatabaseTransaction,
    actor_account_id: i64,
    invite_only: bool,
    medicine_lookup_base_url: &str,
    medicine_lookup_token_url: &str,
    medicine_lookup_source_priority: &[String],
    meta: &RequestMeta,
    settings_config: Option<&serde_json::Value>,
) -> Result<(), OperationError> {
    settings_lock(transaction).await?;
    let actor = authorize(transaction, actor_account_id, "write_platform_settings").await?;
    valid_settings(
        invite_only,
        medicine_lookup_base_url,
        medicine_lookup_token_url,
        medicine_lookup_source_priority,
        settings_config,
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
        &meta.administrator(actor_account_id, actor.person_id),
    )
    .await
}
