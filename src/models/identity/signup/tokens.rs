use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use super::{SignupError, unavailable};

pub(super) fn random_key() -> Result<String, SignupError> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(unavailable)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub(super) fn link_token(account_id: i64, stored_key: &str) -> Result<String, SignupError> {
    let secret = current_secret()?;
    link_token_with_secret(account_id, stored_key, &secret)
}

fn link_token_with_secret(
    account_id: i64,
    stored_key: &str,
    secret: &str,
) -> Result<String, SignupError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(unavailable)?;
    mac.update(stored_key.as_bytes());
    Ok(format!(
        "{account_id}_{}",
        URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
    ))
}

pub(super) fn account_id(token: &str) -> Option<i64> {
    let (id, _) = token.split_once('_')?;
    id.parse::<i64>().ok().filter(|id| *id > 0)
}

pub(super) fn matches(token: &str, stored_key: &str) -> Result<bool, SignupError> {
    let Some((_, signature)) = token.split_once('_') else {
        return Ok(false);
    };
    let Ok(signature) = URL_SAFE_NO_PAD.decode(signature) else {
        return Ok(false);
    };
    let current = current_secret()?;
    let old = match std::env::var("RAILS_OLD_SECRET_KEY_BASE") {
        Ok(secret) if !secret.is_empty() => Some(secret),
        Err(std::env::VarError::NotPresent) => None,
        _ => return Err(SignupError::Unavailable),
    };
    for secret in std::iter::once(current).chain(old) {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(unavailable)?;
        mac.update(stored_key.as_bytes());
        if mac.verify_slice(&signature).is_ok() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn current_secret() -> Result<String, SignupError> {
    std::env::var("RAILS_SECRET_KEY_BASE")
        .ok()
        .filter(|secret| !secret.is_empty())
        .ok_or(SignupError::Unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_rodauth_verification_key_representation() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/identity/rails-verification-key.json"
        ))
        .unwrap();
        let id = fixture["account_id"].as_i64().unwrap();
        let stored = fixture["stored_key"].as_str().unwrap();
        let secret = fixture["secret"].as_str().unwrap();
        let expected = fixture["token"].as_str().unwrap();
        assert_eq!(
            link_token_with_secret(id, stored, secret).unwrap(),
            expected
        );
        assert_eq!(account_id(expected), Some(id));
        assert_ne!(
            link_token_with_secret(id, stored, "wrong-synthetic-secret").unwrap(),
            expected
        );
        assert_ne!(
            link_token_with_secret(id, "changed-key", secret).unwrap(),
            expected
        );
        assert!(account_id("invalid-token").is_none());
        assert!(account_id("-1_key").is_none());
    }
}
