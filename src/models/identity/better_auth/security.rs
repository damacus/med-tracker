use super::IdentityService;
use better_auth_core::{AuthRequest, AuthResult, SchemaValue};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};

#[derive(sea_orm::FromQueryResult)]
pub(crate) struct SessionSummary {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(sea_orm::FromQueryResult)]
pub(crate) struct PasskeySummary {
    pub id: i64,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub(crate) struct Inventory {
    pub sessions: Vec<SessionSummary>,
    pub passkeys: Vec<PasskeySummary>,
}

pub(crate) async fn inventory(
    db: &impl sea_orm::ConnectionTrait,
    account_id: i64,
) -> AuthResult<Inventory> {
    use super::schema::{passkey, session};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
    let sessions = session::Entity::find()
        .select_only()
        .columns([
            session::Column::Id,
            session::Column::CreatedAt,
            session::Column::ExpiresAt,
        ])
        .filter(session::Column::UserId.eq(account_id))
        .filter(session::Column::Active.eq(true))
        .filter(session::Column::ExpiresAt.gt(Utc::now()))
        .order_by_asc(session::Column::CreatedAt)
        .order_by_asc(session::Column::Id)
        .into_model::<SessionSummary>()
        .all(db)
        .await
        .map_err(super::policy::database_error)?;
    let passkeys = passkey::Entity::find()
        .select_only()
        .columns([
            passkey::Column::Id,
            passkey::Column::Name,
            passkey::Column::CreatedAt,
        ])
        .filter(passkey::Column::UserId.eq(account_id))
        .order_by_asc(passkey::Column::CreatedAt)
        .order_by_asc(passkey::Column::Id)
        .into_model::<PasskeySummary>()
        .all(db)
        .await
        .map_err(super::policy::database_error)?;
    Ok(Inventory { sessions, passkeys })
}

pub async fn overview(
    service: &IdentityService,
    request: &AuthRequest,
    request_id: String,
) -> AuthResult<Value> {
    super::request::trusted(service, request, request_id, async {
        let (user, current) = service.context().require_authoritative_session(request).await?;
        let transaction = service.policy.transaction().await?;
        service.policy.require_security_session(&transaction, &current).await?;
        let user_id = user.id.display_string()?;
        let inventory = inventory(&transaction, super::clinical_id(&user_id)?).await?;
        let accounts = service.context().database.get_user_accounts(&user_id).await?;
        let personal_keys = super::personal_keys::inventory(&transaction, super::clinical_id(&user_id)?).await?.into_iter().map(|key| json!({"id":key.id,"name":key.name,"expires":key.expires_at,"last_used":key.last_request})).collect::<Vec<_>>();
        let password = accounts.iter().any(|account| account.provider_id == "credential" && matches!(account.password, SchemaValue::Typed(Some(_))));
        let zitadel = accounts.iter().any(|account| account.provider_id == "zitadel");
        let passkeys = inventory.passkeys.into_iter().map(|key| json!({"id":key.id.to_string(),"name":key.name,"created":key.created_at.format("%d %b %Y").to_string()})).collect::<Vec<_>>();
        let current_id=super::clinical_id(&current.id.display_string()?)?;
        let sessions = inventory.sessions.into_iter().map(|session| json!({"id":session.id.to_string(),"current":session.id==current_id,"created":session.created_at.format("%d %b %Y %H:%M UTC").to_string(),"expires":session.expires_at.format("%d %b %Y %H:%M UTC").to_string()})).collect::<Vec<_>>();
        let result = json!({
            "account_id":user_id,
            "totp":user.two_factor_enabled.typed()?.unwrap_or(false),
            "password":password,
            "zitadel":zitadel,
            "provider_available":service.provider.is_some(),
            "passkey_count":passkeys.len().to_string(),
            "keys":personal_keys,
            "passkeys":passkeys,
            "sessions":sessions
        });
        transaction.commit().await.map_err(super::policy::database_error)?;
        Ok(result)
    }).await
}
