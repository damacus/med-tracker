use hmac::{Hmac, Mac};
use sha2::Sha256;
use totp_rs::{Algorithm, Builder, Secret};

const BASE32_ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";
const STEP_SECONDS: u64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtpCompatibilityError {
    InvalidStoredKey,
    EmptyHmacSecret,
    InvalidTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretVersion {
    Current,
    Old,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifiedOtp {
    pub matched_step: u64,
    pub secret_version: SecretVersion,
}

pub fn derive_rodauth_otp_secret(
    stored_key: &str,
    hmac_secret: Option<&[u8]>,
) -> Result<String, OtpCompatibilityError> {
    if !matches!(stored_key.len(), 16 | 32)
        || !stored_key
            .bytes()
            .all(|byte| BASE32_ALPHABET.contains(&byte))
    {
        return Err(OtpCompatibilityError::InvalidStoredKey);
    }
    let Some(hmac_secret) = hmac_secret else {
        return Ok(stored_key.to_owned());
    };
    if hmac_secret.is_empty() {
        return Err(OtpCompatibilityError::EmptyHmacSecret);
    }
    let decoded = decode_secret(stored_key)?;
    let mut mac = Hmac::<Sha256>::new_from_slice(hmac_secret)
        .map_err(|_| OtpCompatibilityError::EmptyHmacSecret)?;
    mac.update(decoded.as_bytes());
    let digest = mac.finalize().into_bytes();
    Ok(digest[..stored_key.len()]
        .iter()
        .map(|byte| BASE32_ALPHABET[(byte % 32) as usize] as char)
        .collect())
}

pub fn verify_rodauth_otp(
    stored_key: &str,
    current_hmac_secret: Option<&[u8]>,
    old_hmac_secret: Option<&[u8]>,
    code: &str,
    now: i64,
    last_use: i64,
) -> Result<Option<VerifiedOtp>, OtpCompatibilityError> {
    let now = u64::try_from(now).map_err(|_| OtpCompatibilityError::InvalidTimestamp)?;
    let last_use = u64::try_from(last_use).map_err(|_| OtpCompatibilityError::InvalidTimestamp)?;
    let current_secret = derive_rodauth_otp_secret(stored_key, current_hmac_secret)?;
    if now
        .checked_sub(last_use)
        .is_none_or(|elapsed| elapsed <= STEP_SECONDS)
    {
        return Ok(None);
    }
    let code: String = code
        .chars()
        .filter(|character| !matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{b}' | '\u{c}'))
        .collect();
    if code.len() != 6 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok(None);
    }
    if let Some(matched_step) = matched_step(&current_secret, &code, now, last_use)? {
        return Ok(Some(VerifiedOtp {
            matched_step,
            secret_version: SecretVersion::Current,
        }));
    }
    if let (Some(current), Some(old)) = (current_hmac_secret, old_hmac_secret) {
        if current != old {
            let old_secret = derive_rodauth_otp_secret(stored_key, Some(old))?;
            if let Some(matched_step) = matched_step(&old_secret, &code, now, last_use)? {
                return Ok(Some(VerifiedOtp {
                    matched_step,
                    secret_version: SecretVersion::Old,
                }));
            }
        }
    }
    Ok(None)
}

fn decode_secret(secret: &str) -> Result<Secret, OtpCompatibilityError> {
    Secret::try_from_base32(secret.to_ascii_uppercase())
        .map_err(|_| OtpCompatibilityError::InvalidStoredKey)
}

fn matched_step(
    secret: &str,
    code: &str,
    now: u64,
    last_use: u64,
) -> Result<Option<u64>, OtpCompatibilityError> {
    let verifier = Builder::new()
        .with_algorithm(Algorithm::SHA1)
        .with_digits(6)
        .with_step_duration(STEP_SECONDS)
        .with_skew(0)
        .with_secret(decode_secret(secret)?)
        .build_noncompliant();
    let earliest = now.saturating_sub(STEP_SECONDS) / STEP_SECONDS;
    let latest = (now + STEP_SECONDS) / STEP_SECONDS;
    let after = last_use / STEP_SECONDS;
    let mut matched = None;
    for step in earliest..=latest {
        if step > after && verifier.check(code, step * STEP_SECONDS).is_some() {
            matched = Some(step);
        }
    }
    Ok(matched)
}
