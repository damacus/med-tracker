use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ciborium::value::Value as Cbor;
use ring::signature::{
    UnparsedPublicKey, VerificationAlgorithm, ECDSA_P256_SHA256_ASN1, ECDSA_P384_SHA384_ASN1,
    ED25519, RSA_PKCS1_2048_8192_SHA256, RSA_PKCS1_2048_8192_SHA384, RSA_PKCS1_2048_8192_SHA512,
    RSA_PSS_2048_8192_SHA256, RSA_PSS_2048_8192_SHA384, RSA_PSS_2048_8192_SHA512,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

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
    let parsed: Value =
        serde_json::from_str(credential_json).map_err(|_| PasskeyVerifyError::Malformed)?;
    parsed
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(PasskeyVerifyError::Malformed)
}

pub fn verify_passkey_assertion(
    credential_json: &str,
    expected_challenge: &str,
    expected_origin: &str,
    rp_id: &str,
    stored_cose_key: &str,
    stored_sign_count: u64,
) -> Result<VerifiedPasskey, PasskeyVerifyError> {
    let parsed: Value =
        serde_json::from_str(credential_json).map_err(|_| PasskeyVerifyError::Malformed)?;
    let response = parsed
        .get("response")
        .ok_or(PasskeyVerifyError::Malformed)?;
    let credential_id = json_str(&parsed, "id")?.to_owned();
    let authenticator_data = decode_b64(json_str(response, "authenticatorData")?)?;
    let client_data = decode_b64(json_str(response, "clientDataJSON")?)?;
    let signature = decode_b64(json_str(response, "signature")?)?;
    let user_handle = response
        .get("userHandle")
        .and_then(Value::as_str)
        .map(decode_b64)
        .transpose()?;

    let client: Value =
        serde_json::from_slice(&client_data).map_err(|_| PasskeyVerifyError::Malformed)?;
    if client.get("type").and_then(Value::as_str) != Some("webauthn.get") {
        return Err(PasskeyVerifyError::UnexpectedType);
    }
    if client.get("challenge").and_then(Value::as_str) != Some(expected_challenge) {
        return Err(PasskeyVerifyError::ChallengeMismatch);
    }
    if client.get("origin").and_then(Value::as_str) != Some(expected_origin) {
        return Err(PasskeyVerifyError::OriginMismatch);
    }
    if authenticator_data.len() < 37 {
        return Err(PasskeyVerifyError::Malformed);
    }
    let flags = authenticator_data[32];
    if flags & 0x01 == 0 || flags & 0x04 == 0 {
        return Err(PasskeyVerifyError::MissingUserFlags);
    }
    if authenticator_data[..32] != Sha256::digest(rp_id.as_bytes())[..] {
        return Err(PasskeyVerifyError::RpIdMismatch);
    }
    let sign_count = u32::from_be_bytes(authenticator_data[33..37].try_into().unwrap());

    verify_signature(
        stored_cose_key,
        &authenticator_data,
        &client_data,
        &signature,
    )?;

    if !(sign_count == 0 && stored_sign_count == 0) && u64::from(sign_count) <= stored_sign_count {
        return Err(PasskeyVerifyError::SignCountReplay);
    }
    Ok(VerifiedPasskey {
        credential_id,
        sign_count,
        user_handle,
    })
}

fn decode_b64(value: &str) -> Result<Vec<u8>, PasskeyVerifyError> {
    URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| PasskeyVerifyError::Malformed)
}

fn json_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, PasskeyVerifyError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or(PasskeyVerifyError::Malformed)
}

fn verify_signature(
    stored_cose_key: &str,
    authenticator_data: &[u8],
    client_data: &[u8],
    signature: &[u8],
) -> Result<(), PasskeyVerifyError> {
    let cose = URL_SAFE_NO_PAD
        .decode(stored_cose_key)
        .map_err(|_| PasskeyVerifyError::UnsupportedKey)?;
    let key: Cbor =
        ciborium::de::from_reader(&cose[..]).map_err(|_| PasskeyVerifyError::UnsupportedKey)?;
    let Cbor::Map(entries) = key else {
        return Err(PasskeyVerifyError::UnsupportedKey);
    };
    let algorithm = cose_int(&entries, 3).ok_or(PasskeyVerifyError::UnsupportedKey)?;
    let key_bytes = match algorithm {
        -7 => ec_public_key(&entries, 1)?,
        -35 => ec_public_key(&entries, 2)?,
        -8 => okp_public_key(&entries)?,
        -257 | -258 | -259 | -37 | -38 | -39 => rsa_public_key(&entries)?,
        _ => return Err(PasskeyVerifyError::UnsupportedKey),
    };
    let verifier: &dyn VerificationAlgorithm = match algorithm {
        -7 => &ECDSA_P256_SHA256_ASN1,
        -35 => &ECDSA_P384_SHA384_ASN1,
        -8 => &ED25519,
        -257 => &RSA_PKCS1_2048_8192_SHA256,
        -258 => &RSA_PKCS1_2048_8192_SHA384,
        -259 => &RSA_PKCS1_2048_8192_SHA512,
        -37 => &RSA_PSS_2048_8192_SHA256,
        -38 => &RSA_PSS_2048_8192_SHA384,
        _ => &RSA_PSS_2048_8192_SHA512,
    };
    let mut signed = authenticator_data.to_vec();
    signed.extend_from_slice(&Sha256::digest(client_data));
    UnparsedPublicKey::new(verifier, &key_bytes)
        .verify(&signed, signature)
        .map_err(|_| PasskeyVerifyError::SignatureInvalid)
}

fn cose_entry(entries: &[(Cbor, Cbor)], label: i64) -> Option<&Cbor> {
    entries.iter().find_map(|(key, value)| {
        (matches!(key, Cbor::Integer(i) if i128::from(*i) == i128::from(label))).then_some(value)
    })
}

fn cose_int(entries: &[(Cbor, Cbor)], label: i64) -> Option<i64> {
    match cose_entry(entries, label) {
        Some(Cbor::Integer(value)) => i64::try_from(i128::from(*value)).ok(),
        _ => None,
    }
}

fn cose_bytes(entries: &[(Cbor, Cbor)], label: i64) -> Option<&[u8]> {
    match cose_entry(entries, label) {
        Some(Cbor::Bytes(value)) => Some(value),
        _ => None,
    }
}

fn ec_public_key(entries: &[(Cbor, Cbor)], curve: i64) -> Result<Vec<u8>, PasskeyVerifyError> {
    let coordinate = if curve == 1 { 32 } else { 48 };
    let x = cose_bytes(entries, -2).ok_or(PasskeyVerifyError::UnsupportedKey)?;
    let y = cose_bytes(entries, -3).ok_or(PasskeyVerifyError::UnsupportedKey)?;
    if cose_int(entries, 1) != Some(2)
        || cose_int(entries, -1) != Some(curve)
        || x.len() != coordinate
        || y.len() != coordinate
    {
        return Err(PasskeyVerifyError::UnsupportedKey);
    }
    let mut key = Vec::with_capacity(1 + x.len() + y.len());
    key.push(0x04);
    key.extend_from_slice(x);
    key.extend_from_slice(y);
    Ok(key)
}

fn okp_public_key(entries: &[(Cbor, Cbor)]) -> Result<Vec<u8>, PasskeyVerifyError> {
    let x = cose_bytes(entries, -2).ok_or(PasskeyVerifyError::UnsupportedKey)?;
    if cose_int(entries, 1) != Some(1) || cose_int(entries, -1) != Some(6) || x.len() != 32 {
        return Err(PasskeyVerifyError::UnsupportedKey);
    }
    Ok(x.to_vec())
}

fn rsa_public_key(entries: &[(Cbor, Cbor)]) -> Result<Vec<u8>, PasskeyVerifyError> {
    let modulus = cose_bytes(entries, -1).ok_or(PasskeyVerifyError::UnsupportedKey)?;
    let exponent = cose_bytes(entries, -2).ok_or(PasskeyVerifyError::UnsupportedKey)?;
    if cose_int(entries, 1) != Some(3) || modulus.len() < 256 || exponent.is_empty() {
        return Err(PasskeyVerifyError::UnsupportedKey);
    }
    let mut body = der_integer(modulus);
    body.extend_from_slice(&der_integer(exponent));
    let mut der = vec![0x30];
    der_length(&mut der, body.len());
    der.extend_from_slice(&body);
    Ok(der)
}

fn der_integer(bytes: &[u8]) -> Vec<u8> {
    let mut bytes = bytes;
    while bytes.len() > 1 && bytes[0] == 0 {
        bytes = &bytes[1..];
    }
    let padded = bytes[0] & 0x80 != 0;
    let mut out = vec![0x02];
    der_length(&mut out, bytes.len() + usize::from(padded));
    if padded {
        out.push(0);
    }
    out.extend_from_slice(bytes);
    out
}

fn der_length(out: &mut Vec<u8>, length: usize) {
    if length < 128 {
        out.push(length as u8);
    } else if length < 256 {
        out.extend_from_slice(&[0x81, length as u8]);
    } else {
        out.extend_from_slice(&[0x82, (length >> 8) as u8, length as u8]);
    }
}
