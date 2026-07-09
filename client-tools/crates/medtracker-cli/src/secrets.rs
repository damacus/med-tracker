use anyhow::{Context, Result};

const SERVICE: &str = "medtracker-cli";

pub fn load_access_token(profile: &str) -> Result<Option<String>> {
    if let Ok(token) = std::env::var("MEDTRACKER_TOKEN") {
        return Ok(Some(token));
    }

    let entry = keyring::Entry::new(SERVICE, &format!("{profile}:access_token"))
        .context("failed to open OS keychain entry")?;

    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error).context("failed to read token from OS keychain"),
    }
}

pub fn store_access_token(profile: &str, token: &str) -> Result<()> {
    keyring::Entry::new(SERVICE, &format!("{profile}:access_token"))
        .context("failed to open OS keychain entry")?
        .set_password(token)
        .context("failed to store token in OS keychain")
}

pub fn delete_access_token(profile: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, &format!("{profile}:access_token"))
        .context("failed to open OS keychain entry")?;

    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error).context("failed to delete token from OS keychain"),
    }
}

pub fn extract_access_token(value: &serde_json::Value) -> Option<&str> {
    value
        .pointer("/session/access_token")
        .or_else(|| value.pointer("/access_token"))
        .and_then(serde_json::Value::as_str)
}
