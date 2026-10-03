use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::Serialize;
use sha2::{Digest, Sha256};
use webauthn_rs_core::{
    error::WebauthnError,
    internals::AuthenticatorData,
    proto::{
        Authentication, AuthenticationState, COSEKey, CollectedClientData, Credential,
        CredentialV3, PublicKeyCredential, UserVerificationPolicy,
    },
    WebauthnCore,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasskeyVerifyError {
    Malformed,
    UnexpectedType,
    ChallengeMismatch,
    OriginMismatch,
    RpIdMismatch,
    MissingUserFlags,
    UnsupportedKey,
    SignatureInvalid,
    SignCountReplay,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedPasskey {
    pub credential_id: String,
    pub sign_count: u32,
    pub user_handle: Option<Vec<u8>>,
}

pub fn credential_id(credential_json: &str) -> Result<String, PasskeyVerifyError> {
    Ok(parse_assertion(credential_json)?.id)
}

fn parse_assertion(json: &str) -> Result<PublicKeyCredential, PasskeyVerifyError> {
    let parsed: PublicKeyCredential =
        serde_json::from_str(json).map_err(|_| PasskeyVerifyError::Malformed)?;
    if parsed.type_ != "public-key"
        || decode_b64(&parsed.id)?.as_slice() != parsed.raw_id.as_slice()
    {
        return Err(PasskeyVerifyError::Malformed);
    }
    Ok(parsed)
}

pub fn verify_passkey_assertion(
    credential_json: &str,
    expected_challenge: &str,
    expected_origin: &str,
    rp_id: &str,
    stored_cose_key: &str,
    stored_sign_count: u64,
) -> Result<VerifiedPasskey, PasskeyVerifyError> {
    let parsed = parse_assertion(credential_json)?;
    let response = &parsed.response;
    let authenticator_data = response.authenticator_data.as_slice();
    let client_data = response.client_data_json.as_slice();
    let user_handle = response.user_handle.as_ref().map(|handle| handle.to_vec());
    let client: CollectedClientData =
        serde_json::from_slice(client_data).map_err(|_| PasskeyVerifyError::Malformed)?;
    if client.type_ != "webauthn.get" {
        return Err(PasskeyVerifyError::UnexpectedType);
    }
    if client.challenge.as_slice() != decode_b64(expected_challenge)? {
        return Err(PasskeyVerifyError::ChallengeMismatch);
    }
    let origin = client.origin.as_str();
    if client.cross_origin.unwrap_or(false)
        || origin.strip_suffix('/').unwrap_or(origin) != expected_origin
    {
        return Err(PasskeyVerifyError::OriginMismatch);
    }
    let auth = AuthenticatorData::<Authentication>::try_from(authenticator_data)
        .map_err(|_| PasskeyVerifyError::Malformed)?;
    if !auth.user_present || !auth.user_verified {
        return Err(PasskeyVerifyError::MissingUserFlags);
    }
    if auth.backup_state && !auth.backup_eligible {
        return Err(PasskeyVerifyError::Malformed);
    }
    if authenticator_data[..32] != Sha256::digest(rp_id.as_bytes())[..] {
        return Err(PasskeyVerifyError::RpIdMismatch);
    }
    let sign_count = auth.counter;

    let cose = decode_b64(stored_cose_key).map_err(|_| PasskeyVerifyError::UnsupportedKey)?;
    let cbor: serde_cbor_2::Value =
        serde_cbor_2::from_slice(&cose).map_err(|_| PasskeyVerifyError::UnsupportedKey)?;
    let key = COSEKey::try_from(&cbor).map_err(|_| PasskeyVerifyError::UnsupportedKey)?;
    verify_with_library(
        &parsed,
        expected_challenge,
        expected_origin,
        rp_id,
        key,
        stored_sign_count,
        auth.backup_eligible,
    )?;
    Ok(VerifiedPasskey {
        credential_id: parsed.id,
        sign_count,
        user_handle,
    })
}

fn decode_b64(value: &str) -> Result<Vec<u8>, PasskeyVerifyError> {
    URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| PasskeyVerifyError::Malformed)
}

#[derive(Serialize)]
struct RailsAuthenticationState<'a> {
    credentials: Vec<Credential>,
    policy: UserVerificationPolicy,
    challenge: &'a str,
    appid: Option<String>,
    allow_backup_eligible_upgrade: bool,
}

#[allow(clippy::too_many_arguments)]
fn verify_with_library(
    assertion: &PublicKeyCredential,
    challenge: &str,
    origin: &str,
    rp_id: &str,
    key: COSEKey,
    counter: u64,
    backup_eligible: bool,
) -> Result<(), PasskeyVerifyError> {
    let mut credential: Credential = CredentialV3 {
        cred_id: assertion.raw_id.to_vec(),
        cred: key,
        counter: counter
            .try_into()
            .map_err(|_| PasskeyVerifyError::SignCountReplay)?,
        verified: true,
        registration_policy: UserVerificationPolicy::Required,
    }
    .into();
    credential.backup_eligible = backup_eligible;
    let state: AuthenticationState = serde_json::from_value(
        serde_json::to_value(RailsAuthenticationState {
            credentials: vec![credential],
            policy: UserVerificationPolicy::Required,
            challenge,
            appid: None,
            allow_backup_eligible_upgrade: false,
        })
        .map_err(|_| PasskeyVerifyError::Malformed)?,
    )
    .map_err(|_| PasskeyVerifyError::Malformed)?;
    let origin = url::Url::parse(origin).map_err(|_| PasskeyVerifyError::OriginMismatch)?;
    let webauthn = WebauthnCore::new_unsafe_experts_only(
        "MedTracker",
        rp_id,
        vec![origin],
        std::time::Duration::from_secs(600),
        Some(false),
        Some(false),
    );
    webauthn
        .authenticate_credential(assertion, &state)
        .map(|_| ())
        .map_err(|error| match error {
            WebauthnError::CredentialPossibleCompromise => PasskeyVerifyError::SignCountReplay,
            _ => PasskeyVerifyError::SignatureInvalid,
        })
}
