use super::IdentityService;
use better_auth_core::{AuthError, AuthRequest, AuthResult, FieldDate, SchemaValue};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};

fn date(value: &SchemaValue<FieldDate>) -> AuthResult<DateTime<Utc>> {
    value
        .typed()?
        .to_datetime()?
        .ok_or_else(|| AuthError::internal("Invalid identity date"))
}

fn optional_date(value: &SchemaValue<Option<FieldDate>>) -> AuthResult<DateTime<Utc>> {
    value
        .typed()?
        .as_ref()
        .ok_or_else(|| AuthError::internal("Invalid identity date"))?
        .to_datetime()?
        .ok_or_else(|| AuthError::internal("Invalid identity date"))
}

pub async fn overview(
    service: &IdentityService,
    request: &AuthRequest,
    request_id: String,
) -> AuthResult<Value> {
    super::request::trusted(service, request, request_id, async {
        let (user, current) = service.context().require_authoritative_session(request).await?;
        let transaction = service.store.transaction().await?;
        service.store.lock_security_session(&transaction, &current).await?;
        let user_id = user.id.typed()?;
        let sessions = service.context().database.get_user_sessions(user_id).await?;
        let accounts = service.context().database.get_user_accounts(user_id).await?;
        let passkeys = service.context().database.list_passkeys_by_user(user_id).await?;
        let retained = service.store.retained_factor_enabled(user_id).await?;
        let recovery_count = service.store.count_current_recovery_api_keys(user_id).await?;
        let personal_keys = service.store.list_enabled_personal_api_keys(user_id).await?.into_iter().map(|key| json!({"id":key.id,"name":key.name,"expires":key.expires_at,"last_used":key.last_request})).collect::<Vec<_>>();
        let password = accounts.iter().any(|account| account.provider_id == "credential" && matches!(account.password, SchemaValue::Typed(Some(_))));
        let zitadel = accounts.iter().any(|account| account.provider_id == "zitadel");
        let passkeys = passkeys.into_iter().map(|key| -> AuthResult<Value> {
            Ok(json!({"id":key.id,"name":key.name,"created":optional_date(&key.created_at)?.format("%d %b %Y").to_string()}))
        }).collect::<AuthResult<Vec<_>>>()?;
        let sessions = sessions.into_iter().map(|session| -> AuthResult<Value> {
            Ok(json!({"id":session.id,"current":session.id==current.id,"created":date(&session.created_at)?.format("%d %b %Y %H:%M UTC").to_string(),"expires":date(&session.expires_at)?.format("%d %b %Y %H:%M UTC").to_string()}))
        }).collect::<AuthResult<Vec<_>>>()?;
        let result = json!({
            "account_id":user_id,
            "totp":user.two_factor_enabled.typed()?.unwrap_or(false) || retained,
            "password":password,
            "zitadel":zitadel,
            "provider_available":service.provider.is_some(),
            "recovery_count":recovery_count.to_string(),
            "passkey_count":passkeys.len().to_string(),
            "keys":personal_keys,
            "passkeys":passkeys,
            "sessions":sessions
        });
        transaction.commit().await.map_err(super::store::database_error)?;
        Ok(result)
    }).await
}
