use super::*;
use crate::models::entities::{webauthn_key, webauthn_user_id};

pub async fn start(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
    origin: &str,
) -> Result<serde_json::Value, AuthenticationError> {
    begin(db, session, origin, false).await
}

pub async fn start_factor(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
    origin: &str,
) -> Result<serde_json::Value, AuthenticationError> {
    begin(db, session, origin, true).await
}

async fn begin(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
    origin: &str,
    factor: bool,
) -> Result<serde_json::Value, AuthenticationError> {
    let (transaction, account_id, credentials) = if factor {
        let (transaction, pending, _) = otp::pending(db, session).await?;
        let keys = webauthn_key::Entity::find()
            .filter(webauthn_key::Column::AccountId.eq(pending.account_id))
            .all(&transaction)
            .await
            .map_err(unavailable)?;
        if keys.is_empty() {
            return Err(AuthenticationError::Forbidden);
        }
        let credentials = keys
            .iter()
            .filter_map(|key| credential::stored(key).ok())
            .collect::<Vec<_>>();
        if credentials.is_empty() {
            return Err(AuthenticationError::Forbidden);
        }
        (transaction, Some(pending.account_id), credentials)
    } else {
        (transaction(db).await?, None, Vec::new())
    };
    let core = relying_party(origin)?;
    let builder = core
        .new_challenge_authenticate_builder(credentials, Some(UserVerificationPolicy::Required))
        .map_err(unavailable)?;
    let (request, state) = core
        .generate_challenge_authenticate(builder)
        .map_err(unavailable)?;
    let issued = Issued {
        state,
        challenge_digest: hex::encode(Sha256::digest(request.public_key.challenge.as_slice())),
        issued_at: now(&transaction).await?,
        account_id,
    };
    transaction.commit().await.map_err(unavailable)?;
    session.set_store(true);
    session.set("passkey_authentication", issued);
    serde_json::to_value(request).map_err(unavailable)
}

pub async fn finish(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
    origin: &str,
    input: &str,
    request_id: Option<&str>,
) -> Result<(), AuthenticationError> {
    complete(db, session, origin, input, request_id, false).await
}

pub async fn finish_factor(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
    origin: &str,
    input: &str,
    request_id: Option<&str>,
) -> Result<(), AuthenticationError> {
    let result = complete(db, session, origin, input, request_id, true).await;
    if matches!(result, Err(AuthenticationError::Unauthenticated))
        && let Ok((transaction, pending, _)) = otp::pending(db, session).await
    {
        audit::record(
            &transaction,
            pending.account_id,
            "webauthn_verification",
            "failed",
            request_id,
        )
        .await?;
        transaction.commit().await.map_err(unavailable)?;
    }
    result
}

async fn complete(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
    origin: &str,
    input: &str,
    request_id: Option<&str>,
    factor: bool,
) -> Result<(), AuthenticationError> {
    let mut issued = session
        .get::<Issued<AuthenticationState>>("passkey_authentication")
        .ok_or(AuthenticationError::Unauthenticated)?;
    session.remove("passkey_authentication");
    if factor != issued.account_id.is_some() {
        return Err(AuthenticationError::Unauthenticated);
    }
    let assertion: PublicKeyCredential =
        serde_json::from_str(input).map_err(|_| AuthenticationError::Unauthenticated)?;
    if assertion.type_ != "public-key"
        || URL_SAFE_NO_PAD.decode(&assertion.id).ok().as_deref()
            != Some(assertion.raw_id.as_slice())
    {
        return Err(AuthenticationError::Unauthenticated);
    }
    let (transaction, pending) = if factor {
        let (transaction, pending, _) = otp::pending(db, session).await?;
        if issued.account_id != Some(pending.account_id) {
            return Err(AuthenticationError::Unauthenticated);
        }
        (transaction, Some(pending))
    } else {
        (transaction(db).await?, None)
    };
    current(&transaction, &issued).await?;
    let account_id = if let Some(handle) = assertion.response.user_handle.as_ref() {
        let bound = webauthn_user_id::Entity::find()
            .filter(
                webauthn_user_id::Column::WebauthnId.eq(URL_SAFE_NO_PAD.encode(handle.as_slice())),
            )
            .one(&transaction)
            .await
            .map_err(unavailable)?
            .ok_or(AuthenticationError::Unauthenticated)?;
        if issued.account_id.is_some_and(|id| id != bound.account_id) {
            return Err(AuthenticationError::Unauthenticated);
        }
        bound.account_id
    } else {
        issued
            .account_id
            .ok_or(AuthenticationError::Unauthenticated)?
    };
    account::Entity::find_by_id(account_id)
        .lock_exclusive()
        .one(&transaction)
        .await
        .map_err(unavailable)?
        .ok_or(AuthenticationError::Unauthenticated)?;
    let key = webauthn_key::Entity::find()
        .filter(webauthn_key::Column::WebauthnId.eq(&assertion.id))
        .filter(webauthn_key::Column::AccountId.eq(account_id))
        .lock_exclusive()
        .one(&transaction)
        .await
        .map_err(unavailable)?
        .ok_or(AuthenticationError::Unauthenticated)?;
    access::verify_account_actor(&transaction, key.account_id)
        .await
        .map_err(super::super::super::resource::operation_error)?;
    issued
        .state
        .set_allowed_credentials(vec![credential::restore(&key, &assertion)?]);
    let verified = relying_party(origin)?
        .authenticate_credential(&assertion, &issued.state)
        .map_err(|_| AuthenticationError::Unauthenticated)?;
    let count =
        i32::try_from(verified.counter()).map_err(|_| AuthenticationError::Unauthenticated)?;
    consume(&transaction, &issued).await?;
    let now = now(&transaction).await?;
    if let Some(pending) = &pending
        && !pending.current_at(now)
    {
        session.remove("otp_pending");
        return Err(AuthenticationError::Unauthenticated);
    }
    transaction.execute_raw(sql("UPDATE account_webauthn_keys SET sign_count=$1,last_use=timezone('UTC',clock_timestamp()),updated_at=timezone('UTC',clock_timestamp()) WHERE id=$2", [count.into(),key.id.into()])).await.map_err(unavailable)?;
    transaction
        .execute_raw(sql(
            "UPDATE account_otp_keys SET num_failures=0 WHERE id=$1",
            [key.account_id.into()],
        ))
        .await
        .map_err(unavailable)?;
    audit::record(
        &transaction,
        key.account_id,
        "webauthn_verification",
        "succeeded",
        request_id,
    )
    .await?;
    let authenticated_at = pending.map_or(now, |pending| pending.authenticated_at);
    issue_session(transaction, session, key.account_id, authenticated_at, true).await
}
