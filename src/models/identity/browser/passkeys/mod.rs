use super::*;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use webauthn_rs_core::{WebauthnCore, proto::*};

mod audit;
mod authentication;
mod credential;
mod registration;
mod settings;

pub use authentication::{finish as finish_authentication, start as start_authentication};
pub use authentication::{finish_factor, start_factor};
pub use registration::{
    RegistrationInput, finish as finish_registration, start as start_registration,
};
pub use settings::{PasskeySummary, list, remove};

#[derive(Clone, Serialize, Deserialize)]
struct Issued<T> {
    state: T,
    challenge_digest: String,
    issued_at: chrono::NaiveDateTime,
    account_id: Option<i64>,
}

impl<T> Issued<T> {
    fn current_at(&self, now: chrono::NaiveDateTime) -> bool {
        let elapsed = now.signed_duration_since(self.issued_at);
        elapsed >= chrono::Duration::zero() && elapsed < chrono::Duration::minutes(5)
    }
}

pub fn relying_party(origin: &str) -> Result<WebauthnCore, AuthenticationError> {
    let origin = url::Url::parse(origin).map_err(unavailable)?;
    let rp_id = origin
        .host_str()
        .ok_or(AuthenticationError::Unavailable)?
        .to_owned();
    if !matches!(origin.scheme(), "https" | "http") || origin.cannot_be_a_base() {
        return Err(AuthenticationError::Unavailable);
    }
    Ok(WebauthnCore::new_unsafe_experts_only(
        "MedTracker",
        &rp_id,
        vec![origin],
        std::time::Duration::from_secs(300),
        Some(false),
        Some(false),
    ))
}

async fn now(
    transaction: &DatabaseTransaction,
) -> Result<chrono::NaiveDateTime, AuthenticationError> {
    transaction
        .query_one_raw(sql("SELECT timezone('UTC',clock_timestamp()) AS now", []))
        .await
        .map_err(unavailable)?
        .ok_or(AuthenticationError::Unavailable)?
        .try_get("", "now")
        .map_err(unavailable)
}

async fn current<T>(
    transaction: &DatabaseTransaction,
    issued: &Issued<T>,
) -> Result<(), AuthenticationError> {
    if !issued.current_at(now(transaction).await?) {
        return Err(AuthenticationError::Unauthenticated);
    }
    Ok(())
}

async fn consume<T>(
    transaction: &DatabaseTransaction,
    issued: &Issued<T>,
) -> Result<(), AuthenticationError> {
    current(transaction, issued).await?;
    let result = transaction.execute_raw(sql("INSERT INTO account_webauthn_auth_challenges (challenge_digest,created_at) VALUES ($1,timezone('UTC',clock_timestamp())) ON CONFLICT (challenge_digest) DO NOTHING", [issued.challenge_digest.clone().into()])).await.map_err(unavailable)?;
    if result.rows_affected() != 1 {
        return Err(AuthenticationError::Unauthenticated);
    }
    current(transaction, issued).await?;
    transaction.execute_unprepared("DELETE FROM account_webauthn_auth_challenges WHERE created_at < timezone('UTC',clock_timestamp()) - interval '15 minutes'").await.map_err(unavailable)?;
    Ok(())
}

async fn confirm_password(
    transaction: &DatabaseTransaction,
    account_id: i64,
    password: String,
) -> Result<(), AuthenticationError> {
    if password.len() > 1024 {
        return Err(AuthenticationError::Unauthenticated);
    }
    let account = account::Entity::find_by_id(account_id)
        .lock_exclusive()
        .one(transaction)
        .await
        .map_err(unavailable)?
        .ok_or(AuthenticationError::Unauthenticated)?;
    access::verify_account_actor(transaction, account_id)
        .await
        .map_err(super::super::resource::operation_error)?;
    let hash = account
        .password_hash
        .ok_or(AuthenticationError::Unauthenticated)?;
    let valid = tokio::task::spawn_blocking(move || bcrypt::verify(password, &hash))
        .await
        .map_err(unavailable)?
        .map_err(unavailable)?;
    if !valid {
        return Err(AuthenticationError::Unauthenticated);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passkey_challenge_has_an_exact_five_minute_deadline() {
        let issued_at = chrono::DateTime::from_timestamp(1_700_000_000, 0)
            .unwrap()
            .naive_utc();
        let issued = Issued {
            state: (),
            challenge_digest: String::new(),
            issued_at,
            account_id: None,
        };
        assert!(!issued.current_at(issued_at - chrono::Duration::nanoseconds(1)));
        assert!(issued.current_at(issued_at));
        assert!(issued.current_at(
            issued_at + chrono::Duration::minutes(5) - chrono::Duration::nanoseconds(1)
        ));
        assert!(!issued.current_at(issued_at + chrono::Duration::minutes(5)));
        assert!(!issued.current_at(issued_at + chrono::Duration::minutes(6)));
    }
}
