use super::*;
use crate::models::entities::webauthn_key;
use webauthn_rs_core::internals::AuthenticatorData;

pub(super) fn restore(
    key: &webauthn_key::Model,
    assertion: &PublicKeyCredential,
) -> Result<Credential, AuthenticationError> {
    let mut credential = stored(key)?;
    let observed = AuthenticatorData::<Authentication>::try_from(
        assertion.response.authenticator_data.as_slice(),
    )
    .map_err(|_| AuthenticationError::Unauthenticated)?;
    credential.backup_eligible = observed.backup_eligible;
    credential.backup_state = observed.backup_state;
    Ok(credential)
}

pub(super) fn stored(key: &webauthn_key::Model) -> Result<Credential, AuthenticationError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(&key.public_key)
        .map_err(|_| AuthenticationError::Unauthenticated)?;
    let value: serde_cbor_2::Value =
        serde_cbor_2::from_slice(&bytes).map_err(|_| AuthenticationError::Unauthenticated)?;
    let public_key = COSEKey::try_from(&value).map_err(|_| AuthenticationError::Unauthenticated)?;
    if !COSEAlgorithm::secure_algs().contains(&public_key.type_) {
        return Err(AuthenticationError::Unauthenticated);
    }
    Ok(CredentialV3 {
        cred_id: URL_SAFE_NO_PAD
            .decode(&key.webauthn_id)
            .map_err(|_| AuthenticationError::Unauthenticated)?,
        cred: public_key,
        counter: key
            .sign_count
            .try_into()
            .map_err(|_| AuthenticationError::Unauthenticated)?,
        verified: true,
        registration_policy: UserVerificationPolicy::Required,
    }
    .into())
}

pub(super) fn stored_public_key(
    registration: &RegisterPublicKeyCredential,
) -> Result<String, AuthenticationError> {
    let object: serde_cbor_2::Value =
        serde_cbor_2::from_slice(registration.response.attestation_object.as_slice())
            .map_err(|_| AuthenticationError::Unauthenticated)?;
    let serde_cbor_2::Value::Map(object) = object else {
        return Err(AuthenticationError::Unauthenticated);
    };
    let Some(serde_cbor_2::Value::Bytes(auth_data)) =
        object.get(&serde_cbor_2::Value::Text("authData".into()))
    else {
        return Err(AuthenticationError::Unauthenticated);
    };
    let parsed = AuthenticatorData::<Registration>::try_from(auth_data.as_slice())
        .map_err(|_| AuthenticationError::Unauthenticated)?;
    let key = parsed
        .acd
        .ok_or(AuthenticationError::Unauthenticated)?
        .credential_pk;
    Ok(URL_SAFE_NO_PAD.encode(serde_cbor_2::to_vec(&key).map_err(unavailable)?))
}
