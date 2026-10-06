use super::*;
use crate::models::entities::{webauthn_key, webauthn_user_id};

pub struct RegistrationInput {
    pub credential: String,
    pub nickname: String,
    pub password: String,
}

pub async fn start(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
    origin: &str,
) -> Result<serde_json::Value, AuthenticationError> {
    let principal = authenticate(db, session).await?;
    let (transaction, _) = principal.authorization_transaction(db).await?;
    let account = account::Entity::find_by_id(principal.account_id())
        .lock_exclusive()
        .one(&transaction)
        .await
        .map_err(unavailable)?
        .ok_or(AuthenticationError::Unauthenticated)?;
    let existing = webauthn_user_id::Entity::find()
        .filter(webauthn_user_id::Column::AccountId.eq(account.id))
        .one(&transaction)
        .await
        .map_err(unavailable)?;
    let handle = match existing {
        Some(existing) => URL_SAFE_NO_PAD
            .decode(existing.webauthn_id)
            .map_err(unavailable)?,
        None => {
            let handle = uuid::Uuid::new_v4().as_bytes().to_vec();
            transaction.execute_raw(sql("INSERT INTO account_webauthn_user_ids (account_id,webauthn_id,created_at,updated_at) VALUES ($1,$2,timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp()))", [account.id.into(),URL_SAFE_NO_PAD.encode(&handle).into()])).await.map_err(unavailable)?;
            handle
        }
    };
    let keys = webauthn_key::Entity::find()
        .filter(webauthn_key::Column::AccountId.eq(account.id))
        .all(&transaction)
        .await
        .map_err(unavailable)?;
    let excluded = keys
        .into_iter()
        .map(|key| {
            URL_SAFE_NO_PAD
                .decode(key.webauthn_id)
                .map(Into::into)
                .map_err(unavailable)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let core = relying_party(origin)?;
    let builder = core
        .new_challenge_register_builder(&handle, &account.email, &account.email)
        .map_err(unavailable)?
        .user_verification_policy(UserVerificationPolicy::Required)
        .require_resident_key(true)
        .exclude_credentials(Some(excluded));
    let (request, state) = core
        .generate_challenge_register(builder)
        .map_err(unavailable)?;
    let issued = Issued {
        state,
        challenge_digest: hex::encode(Sha256::digest(request.public_key.challenge.as_slice())),
        issued_at: now(&transaction).await?,
        account_id: Some(account.id),
    };
    transaction.commit().await.map_err(unavailable)?;
    session.set("passkey_registration", issued);
    serde_json::to_value(request).map_err(unavailable)
}

pub async fn finish(
    db: &DatabaseConnection,
    session: &Session<SessionPgPool>,
    origin: &str,
    input: RegistrationInput,
    request_id: Option<&str>,
) -> Result<(), AuthenticationError> {
    let nickname = input.nickname.trim();
    if nickname.is_empty() || nickname.chars().count() > 255 {
        return Err(AuthenticationError::Forbidden);
    }
    let issued = session
        .get::<Issued<RegistrationState>>("passkey_registration")
        .ok_or(AuthenticationError::Unauthenticated)?;
    session.remove("passkey_registration");
    let principal = authenticate(db, session).await?;
    if issued.account_id != Some(principal.account_id()) {
        return Err(AuthenticationError::Unauthenticated);
    }
    let (transaction, authenticated_at) = principal.authorization_transaction(db).await?;
    confirm_password(&transaction, principal.account_id(), input.password).await?;
    current(&transaction, &issued).await?;
    let registration: RegisterPublicKeyCredential = serde_json::from_str(&input.credential)
        .map_err(|_| AuthenticationError::Unauthenticated)?;
    let credential = relying_party(origin)?
        .register_credential(&registration, &issued.state, None)
        .map_err(|_| AuthenticationError::Unauthenticated)?;
    let public_key = credential::stored_public_key(&registration)?;
    let count =
        i32::try_from(credential.counter).map_err(|_| AuthenticationError::Unauthenticated)?;
    consume(&transaction, &issued).await?;
    let inserted = transaction.execute_raw(sql("INSERT INTO account_webauthn_keys (account_id,webauthn_id,public_key,sign_count,nickname,created_at,updated_at) VALUES ($1,$2,$3,$4,$5,timezone('UTC',clock_timestamp()),timezone('UTC',clock_timestamp())) ON CONFLICT (webauthn_id,account_id) DO NOTHING", [principal.account_id().into(),URL_SAFE_NO_PAD.encode(credential.cred_id.as_slice()).into(),public_key.into(),count.into(),nickname.into()])).await.map_err(unavailable)?;
    if inserted.rows_affected() != 1 {
        return Err(AuthenticationError::Forbidden);
    }
    audit::record(
        &transaction,
        principal.account_id(),
        "webauthn_credential",
        "created",
        request_id,
    )
    .await?;
    issue_session(
        transaction,
        session,
        principal.account_id(),
        authenticated_at,
        true,
    )
    .await
}
