use super::*;

pub enum VerificationResend {
    Sent,
    RecentlySent,
    UnavailableAccount,
}

pub async fn resend_verification(
    db: &DatabaseConnection,
    email: &str,
    origin: &str,
) -> Result<VerificationResend, SignupError> {
    let transaction = browser::transaction(db).await.map_err(unavailable)?;
    let account = transaction
        .query_one_raw(sql(
            "SELECT id,email::text FROM accounts WHERE email=$1 AND status=1 FOR UPDATE",
            [email.trim().to_lowercase().into()],
        ))
        .await
        .map_err(unavailable)?;
    let Some(account) = account else {
        transaction.rollback().await.map_err(unavailable)?;
        return Ok(VerificationResend::UnavailableAccount);
    };
    let account_id: i64 = account.try_get("", "id").map_err(unavailable)?;
    let email: String = account.try_get("", "email").map_err(unavailable)?;
    let keys = transaction.query_all_raw(sql(
        "SELECT key, email_last_sent > timezone('UTC',clock_timestamp()) - interval '300 seconds' AS recent FROM account_verification_keys WHERE account_id=$1 FOR UPDATE",
        [account_id.into()])).await.map_err(unavailable)?;
    if keys.len() != 1 {
        transaction.rollback().await.map_err(unavailable)?;
        return Ok(VerificationResend::UnavailableAccount);
    }
    if keys[0].try_get::<bool>("", "recent").map_err(unavailable)? {
        transaction.commit().await.map_err(unavailable)?;
        return Ok(VerificationResend::RecentlySent);
    }
    let key: String = keys[0].try_get("", "key").map_err(unavailable)?;
    transaction.execute_raw(sql("UPDATE account_verification_keys SET email_last_sent=timezone('UTC',clock_timestamp()),updated_at=timezone('UTC',clock_timestamp()) WHERE account_id=$1", [account_id.into()])).await.map_err(unavailable)?;
    queue_verification(&transaction, account_id, email, &key, origin).await?;
    transaction.commit().await.map_err(unavailable)?;
    Ok(VerificationResend::Sent)
}
