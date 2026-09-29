use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::{SecondsFormat, Utc};
use pbkdf2::pbkdf2_hmac;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const MAX_PLAINTEXT_BYTES: usize = 16 * 1024 * 1024;
#[allow(dead_code)]
const MAX_CIPHERTEXT_BYTES: usize = MAX_PLAINTEXT_BYTES + 16;
const KDF_ROUNDS: u32 = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PortableCryptoError {
    InvalidEnvelope,
    TooLarge,
    Unavailable,
}

fn key(passphrase: &str, salt: &str) -> [u8; 32] {
    let mut key = [0; 32];
    pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), salt.as_bytes(), KDF_ROUNDS, &mut key);
    key
}

pub(super) fn encrypt(payload: &Value, passphrase: &str) -> Result<Value, PortableCryptoError> {
    if passphrase.is_empty() {
        return Err(PortableCryptoError::InvalidEnvelope);
    }
    let plaintext =
        serde_json::to_string(payload).map_err(|_| PortableCryptoError::InvalidEnvelope)?;
    if plaintext.len() > MAX_PLAINTEXT_BYTES {
        return Err(PortableCryptoError::TooLarge);
    }
    let serialized =
        serde_json::to_vec(&plaintext).map_err(|_| PortableCryptoError::InvalidEnvelope)?;
    if serialized.len() > MAX_PLAINTEXT_BYTES {
        return Err(PortableCryptoError::TooLarge);
    }
    let mut salt_bytes = [0_u8; 32];
    let mut iv = [0_u8; 12];
    getrandom::fill(&mut salt_bytes).map_err(|_| PortableCryptoError::Unavailable)?;
    getrandom::fill(&mut iv).map_err(|_| PortableCryptoError::Unavailable)?;
    let salt = salt_bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let cipher = Aes256Gcm::new_from_slice(&key(passphrase, &salt))
        .map_err(|_| PortableCryptoError::Unavailable)?;
    let encrypted = cipher
        .encrypt(&Nonce::from(iv), serialized.as_slice())
        .map_err(|_| PortableCryptoError::Unavailable)?;
    let tag_start = encrypted
        .len()
        .checked_sub(16)
        .ok_or(PortableCryptoError::Unavailable)?;
    let ciphertext = format!(
        "{}--{}--{}",
        STANDARD.encode(&encrypted[..tag_start]),
        STANDARD.encode(iv),
        STANDARD.encode(&encrypted[tag_start..]),
    );
    let checksum = format!("{:x}", Sha256::digest(plaintext.as_bytes()));
    Ok(json!({
        "format": "medtracker.portable.encrypted.v1",
        "encrypted_at": Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        "cipher": "aes-256-gcm",
        "kdf": "pbkdf2_sha256",
        "salt": salt,
        "checksum": checksum,
        "ciphertext": ciphertext,
    }))
}

#[allow(dead_code)]
pub(super) fn decrypt(envelope: &Value, passphrase: &str) -> Result<Value, PortableCryptoError> {
    let object = envelope
        .as_object()
        .ok_or(PortableCryptoError::InvalidEnvelope)?;
    if object.len() != 7
        || envelope["format"] != "medtracker.portable.encrypted.v1"
        || envelope["cipher"] != "aes-256-gcm"
        || envelope["kdf"] != "pbkdf2_sha256"
        || passphrase.is_empty()
    {
        return Err(PortableCryptoError::InvalidEnvelope);
    }
    let salt = envelope["salt"]
        .as_str()
        .ok_or(PortableCryptoError::InvalidEnvelope)?;
    if salt.len() != 64 || !salt.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(PortableCryptoError::InvalidEnvelope);
    }
    let checksum = envelope["checksum"]
        .as_str()
        .ok_or(PortableCryptoError::InvalidEnvelope)?;
    if checksum.len() != 64 || !checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(PortableCryptoError::InvalidEnvelope);
    }
    let encrypted_at = envelope["encrypted_at"]
        .as_str()
        .ok_or(PortableCryptoError::InvalidEnvelope)?;
    chrono::DateTime::parse_from_rfc3339(encrypted_at)
        .map_err(|_| PortableCryptoError::InvalidEnvelope)?;
    let ciphertext = envelope["ciphertext"]
        .as_str()
        .ok_or(PortableCryptoError::InvalidEnvelope)?;
    if ciphertext.len() > MAX_CIPHERTEXT_BYTES.div_ceil(3) * 4 + 64 {
        return Err(PortableCryptoError::TooLarge);
    }
    let mut parts = ciphertext.split("--");
    let encoded_data = parts.next().ok_or(PortableCryptoError::InvalidEnvelope)?;
    let encoded_iv = parts.next().ok_or(PortableCryptoError::InvalidEnvelope)?;
    let encoded_tag = parts.next().ok_or(PortableCryptoError::InvalidEnvelope)?;
    if parts.next().is_some() || encoded_iv.len() != 16 || encoded_tag.len() != 24 {
        return Err(PortableCryptoError::InvalidEnvelope);
    }
    let mut data = STANDARD
        .decode(encoded_data)
        .map_err(|_| PortableCryptoError::InvalidEnvelope)?;
    let iv = STANDARD
        .decode(encoded_iv)
        .map_err(|_| PortableCryptoError::InvalidEnvelope)?;
    let tag = STANDARD
        .decode(encoded_tag)
        .map_err(|_| PortableCryptoError::InvalidEnvelope)?;
    if data.len() > MAX_PLAINTEXT_BYTES {
        return Err(PortableCryptoError::TooLarge);
    }
    if iv.len() != 12 || tag.len() != 16 {
        return Err(PortableCryptoError::InvalidEnvelope);
    }
    data.extend_from_slice(&tag);
    let cipher = Aes256Gcm::new_from_slice(&key(passphrase, salt))
        .map_err(|_| PortableCryptoError::InvalidEnvelope)?;
    let iv: [u8; 12] = iv
        .try_into()
        .map_err(|_| PortableCryptoError::InvalidEnvelope)?;
    let serialized = cipher
        .decrypt(&Nonce::from(iv), data.as_slice())
        .map_err(|_| PortableCryptoError::InvalidEnvelope)?;
    if serialized.len() > MAX_PLAINTEXT_BYTES {
        return Err(PortableCryptoError::TooLarge);
    }
    let plaintext: String =
        serde_json::from_slice(&serialized).map_err(|_| PortableCryptoError::InvalidEnvelope)?;
    if plaintext.len() > MAX_PLAINTEXT_BYTES
        || format!("{:x}", Sha256::digest(plaintext.as_bytes())) != checksum.to_ascii_lowercase()
    {
        return Err(PortableCryptoError::InvalidEnvelope);
    }
    serde_json::from_str(&plaintext).map_err(|_| PortableCryptoError::InvalidEnvelope)
}

#[cfg(test)]
mod tests {
    use super::{decrypt, encrypt, PortableCryptoError};
    use serde_json::json;

    #[test]
    fn envelope_round_trip_and_tamper_rejection() {
        let payload =
            json!({"format": "medtracker.portable.v2", "records": {"people": [{"name": "Áine"}]}});
        let envelope = encrypt(&payload, "correct passphrase").expect("encrypt payload");
        assert_eq!(decrypt(&envelope, "correct passphrase"), Ok(payload));
        assert_eq!(
            decrypt(&envelope, "wrong passphrase"),
            Err(PortableCryptoError::InvalidEnvelope)
        );
        let mut tampered = envelope;
        tampered["checksum"] = json!("0".repeat(64));
        assert_eq!(
            decrypt(&tampered, "correct passphrase"),
            Err(PortableCryptoError::InvalidEnvelope)
        );
    }

    #[test]
    fn malformed_envelopes_fail_without_decryption() {
        let payload = json!({"format": "medtracker.portable.v1", "records": {"people": []}});
        let envelope = encrypt(&payload, "correct passphrase").expect("encrypt payload");
        let mut bad_time = envelope.clone();
        bad_time["encrypted_at"] = json!("not a timestamp");
        assert_eq!(
            decrypt(&bad_time, "correct passphrase"),
            Err(PortableCryptoError::InvalidEnvelope)
        );
        let mut bad_tag = envelope.clone();
        let ciphertext = envelope["ciphertext"].as_str().expect("ciphertext");
        let (prefix, _) = ciphertext.rsplit_once("--").expect("tag delimiter");
        bad_tag["ciphertext"] = json!(format!("{prefix}--AA=="));
        assert_eq!(
            decrypt(&bad_tag, "correct passphrase"),
            Err(PortableCryptoError::InvalidEnvelope)
        );
        let mut modified = envelope;
        let ciphertext = modified["ciphertext"]
            .as_str()
            .expect("ciphertext")
            .to_owned();
        let replacement = if ciphertext.starts_with('A') {
            "B"
        } else {
            "A"
        };
        modified["ciphertext"] = json!(format!("{replacement}{}", &ciphertext[1..]));
        assert_eq!(
            decrypt(&modified, "correct passphrase"),
            Err(PortableCryptoError::InvalidEnvelope)
        );
    }
}
