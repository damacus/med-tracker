use super::IdentityService;
use better_auth_core::{AuthRequest, AuthResult};
use chrono::Utc;
use serde_json::{Value, json};

pub async fn overview(
    service: &IdentityService,
    request: &AuthRequest,
    request_id: String,
) -> AuthResult<Value> {
    super::request::trusted(service, request, request_id, async {
        let (user, current) = service.context().require_authoritative_session(request).await?;
        let transaction = service.store.transaction().await?;
        service.store.lock_security_session(&transaction, &current).await?;
        let sessions = service.context().database.get_user_sessions(&user.id).await?;
        let accounts = service.context().database.get_user_accounts(&user.id).await?;
        let passkeys = service.context().database.list_passkeys_by_user(&user.id).await?;
        let retained = service.store.retained_factor_enabled(&user.id).await?;
        let keys = service.context().database.list_api_keys_by_reference(&user.id).await?;
        let recovery_count = keys.iter().filter(|key| {
            key.config_id == super::onboarding::RECOVERY_CONFIG && key.enabled && key.remaining.is_some_and(|remaining| remaining > 0.0)
                && key.expires_at.as_ref().is_none_or(|date| chrono::DateTime::parse_from_rfc3339(date).is_ok_and(|date| date > Utc::now()))
        }).count();
        let personal_keys = keys.iter().filter(|key| key.config_id == super::personal_keys::CONFIG && key.enabled).map(|key| json!({"id":key.id,"name":key.name,"expires":key.expires_at,"last_used":key.last_request})).collect::<Vec<_>>();
        let result = json!({
            "account_id":user.id,
            "totp":user.two_factor_enabled || retained,
            "password":accounts.iter().any(|account| account.provider_id == "credential" && account.password.is_some()),
            "zitadel":accounts.iter().any(|account| account.provider_id == "zitadel"),
            "provider_available":service.provider.is_some(),
            "recovery_count":recovery_count.to_string(),
            "passkey_count":passkeys.len().to_string(),
            "keys":personal_keys,
            "passkeys":passkeys.into_iter().map(|key| json!({"id":key.id,"name":key.name,"created":key.created_at.format("%d %b %Y").to_string()})).collect::<Vec<_>>(),
            "sessions":sessions.into_iter().map(|session| json!({"id":session.id,"current":session.id==current.id,"created":session.created_at.format("%d %b %Y %H:%M UTC").to_string(),"expires":session.expires_at.format("%d %b %Y %H:%M UTC").to_string()})).collect::<Vec<_>>()
        });
        transaction.commit().await.map_err(super::store::database_error)?;
        Ok(result)
    }).await
}
