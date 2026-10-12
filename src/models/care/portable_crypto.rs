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
