use super::*;
use subtle::ConstantTimeEq;

async fn enrolled_factors(
    transaction: &DatabaseTransaction,
    account_id: i64,
) -> Result<(bool, bool), AuthenticationError> {
    let row = transaction
        .query_one_raw(sql(
            "SELECT EXISTS (SELECT 1 FROM account_otp_keys WHERE id=$1) AS otp, EXISTS (SELECT 1 FROM account_webauthn_keys WHERE account_id=$1) AS passkey",
            [account_id.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(AuthenticationError::Unavailable)?;
    Ok((
        row.try_get("", "otp").map_err(unavailable)?,
        row.try_get("", "passkey").map_err(unavailable)?,
    ))
}

pub async fn challenge(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
) -> Result<bool, AuthenticationError> {
    let (transaction, pending, _) = otp::pending(db, session).await?;
    let (otp, passkey) = enrolled_factors(&transaction, pending.account_id).await?;
    if !otp && !passkey {
        return Err(AuthenticationError::Forbidden);
    }
    transaction.commit().await.map_err(unavailable)?;
    Ok(otp)
}

pub async fn verify(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
    code: &str,
) -> Result<(), AuthenticationError> {
    let (transaction, pending, _) = otp::pending(db, session).await?;
    let (otp, passkey) = enrolled_factors(&transaction, pending.account_id).await?;
    if !otp && !passkey {
        return Err(AuthenticationError::Forbidden);
    }
    let rows = transaction
        .query_all_raw(sql(
            "SELECT code FROM account_recovery_codes WHERE id=$1",
            [pending.account_id.into()],
        ))
        .await
        .map_err(unavailable)?;
    let mut matched = false;
    for row in rows {
        let stored: String = row.try_get("", "code").map_err(unavailable)?;
        matched |= bool::from(stored.as_bytes().ct_eq(code.as_bytes()));
    }
    if !matched {
        return Err(AuthenticationError::Unauthenticated);
    }
    let consumed = transaction
        .execute_raw(sql(
            "DELETE FROM account_recovery_codes WHERE id=$1 AND code=$2",
            [pending.account_id.into(), code.into()],
        ))
        .await
        .map_err(unavailable)?;
    if consumed.rows_affected() != 1 {
        return Err(AuthenticationError::Unauthenticated);
    }
    let now = transaction
        .query_one_raw(sql("SELECT timezone('UTC',clock_timestamp()) AS now", []))
        .await
        .map_err(unavailable)?
        .ok_or(AuthenticationError::Unavailable)?
        .try_get("", "now")
        .map_err(unavailable)?;
    if !pending.current_at(now) {
        session.remove("otp_pending");
        return Err(AuthenticationError::Unauthenticated);
    }
    access::verify_account_actor(&transaction, pending.account_id)
        .await
        .map_err(super::super::resource::operation_error)?;
    let (otp, passkey) = enrolled_factors(&transaction, pending.account_id).await?;
    if !otp && !passkey {
        return Err(AuthenticationError::Forbidden);
    }
    transaction
        .execute_raw(sql(
            "UPDATE account_otp_keys SET num_failures=0 WHERE id=$1",
            [pending.account_id.into()],
        ))
        .await
        .map_err(unavailable)?;
    issue_session(
        transaction,
        session,
        pending.account_id,
        pending.authenticated_at,
        true,
    )
    .await
}
