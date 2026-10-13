use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{SecondsFormat, Utc};
use pbkdf2::pbkdf2_hmac;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const MAX_PLAINTEXT_BYTES: usize = 16 * 1024 * 1024;
const MAX_CIPHERTEXT_BYTES: usize = MAX_PLAINTEXT_BYTES + 16;
const KDF_ROUNDS: u32 = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortableCryptoError {
    InvalidEnvelope,
    TooLarge,
    Unavailable,
}

fn key(passphrase: &str, salt: &str) -> [u8; 32] {
    let mut key = [0; 32];
    pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), salt.as_bytes(), KDF_ROUNDS, &mut key);
    key
}

pub fn encrypt(payload: &Value, passphrase: &str) -> Result<Value, PortableCryptoError> {
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
    let checksum = hex::encode(Sha256::digest(plaintext.as_bytes()));
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

pub fn decrypt(envelope: &Value, passphrase: &str) -> Result<Value, PortableCryptoError> {
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
        || hex::encode(Sha256::digest(plaintext.as_bytes())) != checksum.to_ascii_lowercase()
    {
        return Err(PortableCryptoError::InvalidEnvelope);
    }
    serde_json::from_str(&plaintext).map_err(|_| PortableCryptoError::InvalidEnvelope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn round_trip_restores_the_original_payload() {
        let payload = json!({
            "member": "Alex Example",
            "doses": [{"at": "08:00", "amount": 2}, {"at": "20:00", "amount": 1}],
            "notes": "naïve ünïcode ✓",
            "nested": {"list": [1, 2.5, true, null]},
        });
        let envelope = encrypt(&payload, "correct horse battery staple").unwrap();
        assert_eq!(
            decrypt(&envelope, "correct horse battery staple").unwrap(),
            payload
        );
    }

    #[test]
    fn wrong_passphrase_fails_decryption() {
        let envelope = encrypt(&json!({"dose": 1}), "right").unwrap();
        assert_eq!(
            decrypt(&envelope, "wrong"),
            Err(PortableCryptoError::InvalidEnvelope)
        );
    }

    #[test]
    fn empty_passphrase_is_rejected() {
        assert_eq!(
            encrypt(&json!({"dose": 1}), ""),
            Err(PortableCryptoError::InvalidEnvelope)
        );
        let envelope = encrypt(&json!({"dose": 1}), "right").unwrap();
        assert_eq!(
            decrypt(&envelope, ""),
            Err(PortableCryptoError::InvalidEnvelope)
        );
    }

    #[test]
    fn tampered_ciphertext_is_rejected() {
        let envelope = encrypt(&json!({"dose": 1}), "pw").unwrap();
        let ciphertext = envelope["ciphertext"].as_str().unwrap();
        let mut tampered = envelope.clone();
        tampered["ciphertext"] = Value::String(format!("A{ciphertext}"));
        assert_eq!(
            decrypt(&tampered, "pw"),
            Err(PortableCryptoError::InvalidEnvelope)
        );
    }

    #[test]
    fn tampered_checksum_is_rejected() {
        let mut envelope = encrypt(&json!({"dose": 1}), "pw").unwrap();
        envelope["checksum"] = Value::String("0".repeat(64));
        assert_eq!(
            decrypt(&envelope, "pw"),
            Err(PortableCryptoError::InvalidEnvelope)
        );
    }

    #[test]
    fn rejects_envelopes_with_wrong_markers() {
        for (key, value) in [
            ("format", "medtracker.portable.encrypted.v0"),
            ("cipher", "aes-128-gcm"),
            ("kdf", "argon2"),
        ] {
            let mut envelope = encrypt(&json!({"dose": 1}), "pw").unwrap();
            envelope[key] = json!(value);
            assert_eq!(
                decrypt(&envelope, "pw"),
                Err(PortableCryptoError::InvalidEnvelope),
                "{key}={value}"
            );
        }
    }

    #[test]
    fn rejects_malformed_envelopes() {
        let valid = encrypt(&json!({"dose": 1}), "pw").unwrap();
        for key in ["salt", "checksum", "ciphertext", "encrypted_at"] {
            let mut envelope = valid.clone();
            envelope.as_object_mut().unwrap().remove(key);
            assert_eq!(
                decrypt(&envelope, "pw"),
                Err(PortableCryptoError::InvalidEnvelope),
                "missing {key}"
            );
        }
        let mut extra = valid.clone();
        extra["extra"] = json!(true);
        assert_eq!(
            decrypt(&extra, "pw"),
            Err(PortableCryptoError::InvalidEnvelope)
        );
        let mut bad_salt = valid.clone();
        bad_salt["salt"] = json!("not-hex");
        assert_eq!(
            decrypt(&bad_salt, "pw"),
            Err(PortableCryptoError::InvalidEnvelope)
        );
        let mut bad_date = valid.clone();
        bad_date["encrypted_at"] = json!("yesterday");
        assert_eq!(
            decrypt(&bad_date, "pw"),
            Err(PortableCryptoError::InvalidEnvelope)
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(16))]

        #[test]
        fn round_trip_never_loses_string_payloads(payload in ".{0,512}") {
            let envelope = encrypt(&Value::String(payload.clone()), "pw").unwrap();
            prop_assert_eq!(
                decrypt(&envelope, "pw").unwrap(),
                Value::String(payload)
            );
        }

        #[test]
        fn decrypt_never_panics_on_arbitrary_envelopes(
            ciphertext in "[a-zA-Z0-9+/=.-]{0,256}",
        ) {
            let envelope = json!({
                "format": "medtracker.portable.encrypted.v1",
                "encrypted_at": "2026-01-01T00:00:00Z",
                "cipher": "aes-256-gcm",
                "kdf": "pbkdf2_sha256",
                "salt": "a".repeat(64),
                "checksum": "b".repeat(64),
                "ciphertext": ciphertext,
            });
            let _ = decrypt(&envelope, "pw");
        }
    }
}
