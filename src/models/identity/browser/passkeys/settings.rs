use super::*;
use crate::models::entities::webauthn_key;

#[derive(Serialize)]
pub struct PasskeySummary {
    pub id: String,
    pub nickname: String,
    pub needs_replacement: bool,
}

pub async fn list(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
) -> Result<Vec<PasskeySummary>, AuthenticationError> {
    let principal = authenticate(db, session).await?;
    let (transaction, _) = principal.authorization_transaction(db).await?;
    let keys = webauthn_key::Entity::find()
        .filter(webauthn_key::Column::AccountId.eq(principal.account_id()))
        .all(&transaction)
        .await
        .map_err(unavailable)?;
    let result = keys
        .into_iter()
        .map(|key| PasskeySummary {
            needs_replacement: credential::stored(&key).is_err(),
            id: key.webauthn_id,
            nickname: key.nickname.unwrap_or_else(|| "Passkey".into()),
        })
        .collect();
    transaction.commit().await.map_err(unavailable)?;
    Ok(result)
}

pub async fn remove(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
    credential_id: &str,
    password: String,
    request_id: Option<&str>,
) -> Result<(), AuthenticationError> {
    let principal = authenticate(db, session).await?;
    let (transaction, _) = principal.authorization_transaction(db).await?;
    confirm_password(&transaction, principal.account_id(), password).await?;
    let result = transaction
        .execute_raw(sql(
            "DELETE FROM account_webauthn_keys WHERE account_id=$1 AND webauthn_id=$2",
            [principal.account_id().into(), credential_id.into()],
        ))
        .await
        .map_err(unavailable)?;
    if result.rows_affected() != 1 {
        return Err(AuthenticationError::Forbidden);
    }
    transaction.execute_raw(sql("DELETE FROM account_recovery_codes WHERE id=$1 AND NOT EXISTS (SELECT 1 FROM account_otp_keys WHERE id=$1) AND NOT EXISTS (SELECT 1 FROM account_webauthn_keys WHERE account_id=$1)", [principal.account_id().into()])).await.map_err(unavailable)?;
    audit::record(
        &transaction,
        principal.account_id(),
        "webauthn_credential",
        "revoked",
        request_id,
    )
    .await?;
    transaction
        .execute_raw(sql(
            "DELETE FROM account_active_session_keys WHERE account_id=$1 AND session_id=$2",
            [
                principal.account_id().into(),
                principal.identity.registry_key.clone().into(),
            ],
        ))
        .await
        .map_err(unavailable)?;
    transaction.commit().await.map_err(unavailable)?;
    session.destroy();
    Ok(())
}
