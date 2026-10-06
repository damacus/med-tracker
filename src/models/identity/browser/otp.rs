use super::*;
use crate::models::entities::otp_key;
use crate::models::identity::totp;

#[derive(Clone, Serialize, Deserialize)]
struct PasswordVerified {
    account_id: i64,
    authenticated_at: chrono::NaiveDateTime,
}

impl PasswordVerified {
    fn current_at(&self, now: chrono::NaiveDateTime) -> bool {
        let elapsed = now.signed_duration_since(self.authenticated_at);
        elapsed >= chrono::Duration::zero() && elapsed < chrono::Duration::minutes(5)
    }
}

pub fn has_pending_challenge(session: &Session<SessionPgPool>) -> bool {
    session.get::<PasswordVerified>("otp_pending").is_some()
}

pub(super) async fn begin(
    transaction: DatabaseTransaction,
    session: &Session<SessionPgPool>,
    account_id: i64,
    authenticated_at: chrono::NaiveDateTime,
) -> Result<(), AuthenticationError> {
    transaction
        .execute_raw(sql(
            "DELETE FROM account_login_failures WHERE account_id=$1",
            [account_id.into()],
        ))
        .await
        .map_err(unavailable)?;
    transaction.commit().await.map_err(unavailable)?;
    session.renew();
    session.set_store(true);
    session.remove("identity");
    session.set(
        "otp_pending",
        PasswordVerified {
            account_id,
            authenticated_at,
        },
    );
    Ok(())
}

async fn pending(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
) -> Result<(DatabaseTransaction, PasswordVerified, chrono::NaiveDateTime), AuthenticationError> {
    let pending = session
        .get::<PasswordVerified>("otp_pending")
        .ok_or(AuthenticationError::Unauthenticated)?;
    let transaction = transaction(db).await?;
    access::verify_account_actor(&transaction, pending.account_id)
        .await
        .map_err(super::super::resource::operation_error)?;
    let row = transaction
        .query_one_raw(sql("SELECT timezone('UTC',clock_timestamp()) AS now", []))
        .await
        .map_err(unavailable)?
        .ok_or(AuthenticationError::Unavailable)?;
    let now: chrono::NaiveDateTime = row.try_get("", "now").map_err(unavailable)?;
    if !pending.current_at(now) {
        session.remove("otp_pending");
        return Err(AuthenticationError::Unauthenticated);
    }
    Ok((transaction, pending, now))
}

pub async fn challenge(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
) -> Result<(), AuthenticationError> {
    let (transaction, pending, _) = pending(db, session).await?;
    let key = otp_key::Entity::find_by_id(pending.account_id)
        .one(&transaction)
        .await
        .map_err(unavailable)?
        .ok_or(AuthenticationError::Forbidden)?;
    if key.num_failures >= 5 {
        return Err(AuthenticationError::Forbidden);
    }
    transaction.commit().await.map_err(unavailable)?;
    Ok(())
}

pub async fn verify(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
    code: &str,
) -> Result<(), AuthenticationError> {
    let (transaction, pending, _) = pending(db, session).await?;
    let key = otp_key::Entity::find_by_id(pending.account_id)
        .lock_exclusive()
        .one(&transaction)
        .await
        .map_err(unavailable)?
        .ok_or(AuthenticationError::Forbidden)?;
    if key.num_failures >= 5 {
        return Err(AuthenticationError::Forbidden);
    }
    let current_secret = std::env::var("RAILS_SECRET_KEY_BASE").map_err(unavailable)?;
    if current_secret.is_empty() {
        return Err(AuthenticationError::Unavailable);
    }
    let old_secret = match std::env::var("RAILS_OLD_SECRET_KEY_BASE") {
        Ok(secret) if secret.is_empty() => return Err(AuthenticationError::Unavailable),
        Ok(secret) => Some(secret),
        Err(std::env::VarError::NotPresent) => None,
        Err(error) => return Err(unavailable(error)),
    };
    let row = transaction
        .query_one_raw(sql("SELECT timezone('UTC',clock_timestamp()) AS now", []))
        .await
        .map_err(unavailable)?
        .ok_or(AuthenticationError::Unavailable)?;
    let now: chrono::NaiveDateTime = row.try_get("", "now").map_err(unavailable)?;
    if !pending.current_at(now) {
        session.remove("otp_pending");
        return Err(AuthenticationError::Unauthenticated);
    }
    let verified = if code.len() <= 128 {
        totp::verify_rodauth_otp(
            &key.key,
            Some(current_secret.as_bytes()),
            old_secret.as_deref().map(str::as_bytes),
            code,
            now.and_utc().timestamp(),
            key.last_use.and_utc().timestamp(),
        )
        .map_err(unavailable)?
    } else {
        None
    };
    if verified.is_none() {
        transaction
            .execute_raw(sql(
                "UPDATE account_otp_keys SET num_failures=num_failures+1 WHERE id=$1",
                [pending.account_id.into()],
            ))
            .await
            .map_err(unavailable)?;
        transaction.commit().await.map_err(unavailable)?;
        return Err(AuthenticationError::Unauthenticated);
    }
    let result = transaction.execute_raw(sql("UPDATE account_otp_keys SET last_use=timezone('UTC',clock_timestamp()), num_failures=0 WHERE id=$1 AND last_use+interval '30 seconds'<timezone('UTC',clock_timestamp())", [pending.account_id.into()])).await.map_err(unavailable)?;
    if result.rows_affected() != 1 {
        return Err(AuthenticationError::Unauthenticated);
    }
    issue_session(
        transaction,
        session,
        pending.account_id,
        pending.authenticated_at,
        true,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::PasswordVerified;
    use chrono::{DateTime, Duration};

    #[test]
    fn pending_password_expires_at_five_minutes_and_rejects_future_time() {
        let authenticated_at = DateTime::from_timestamp(1_700_000_000, 0)
            .unwrap()
            .naive_utc();
        let pending = PasswordVerified {
            account_id: 71001,
            authenticated_at,
        };
        assert!(pending.current_at(authenticated_at));
        assert!(
            pending.current_at(authenticated_at + Duration::minutes(5) - Duration::nanoseconds(1))
        );
        assert!(!pending.current_at(authenticated_at + Duration::minutes(5)));
        assert!(!pending.current_at(authenticated_at + Duration::days(1)));
        assert!(!pending.current_at(authenticated_at - Duration::nanoseconds(1)));
    }
}
