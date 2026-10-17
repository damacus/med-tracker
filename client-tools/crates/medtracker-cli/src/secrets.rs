use anyhow::{Context, Result};
use keyring_core::api::CredentialStore;
use std::sync::Arc;

const SERVICE: &str = "medtracker-cli";

pub fn load_access_token(profile: &str) -> Result<Option<String>> {
    if let Ok(token) = std::env::var("MEDTRACKER_TOKEN") {
        return Ok(Some(token));
    }

    let store = native_store().context("failed to open OS keychain entry")?;
    load_access_token_from_store(store.as_ref(), profile)
}

fn load_access_token_from_store(store: &CredentialStore, profile: &str) -> Result<Option<String>> {
    let entry = entry(store, profile)?;

    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring_core::Error::NoEntry) => Ok(None),
        Err(error) => Err(error).context("failed to read token from OS keychain"),
    }
}

pub fn store_access_token(profile: &str, token: &str) -> Result<()> {
    let store = native_store().context("failed to open OS keychain entry")?;
    store_access_token_in_store(store.as_ref(), profile, token)
}

fn store_access_token_in_store(store: &CredentialStore, profile: &str, token: &str) -> Result<()> {
    entry(store, profile)?
        .set_password(token)
        .context("failed to store token in OS keychain")
}

pub fn delete_access_token(profile: &str) -> Result<()> {
    let store = native_store().context("failed to open OS keychain entry")?;
    delete_access_token_from_store(store.as_ref(), profile)
}

fn delete_access_token_from_store(store: &CredentialStore, profile: &str) -> Result<()> {
    let entry = entry(store, profile)?;

    match entry.delete_credential() {
        Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
        Err(error) => Err(error).context("failed to delete token from OS keychain"),
    }
}

fn entry(store: &CredentialStore, profile: &str) -> Result<keyring_core::Entry> {
    store
        .build(SERVICE, &format!("{profile}:access_token"), None)
        .context("failed to open OS keychain entry")
}

#[cfg(target_os = "macos")]
fn native_store() -> keyring_core::Result<Arc<CredentialStore>> {
    Ok(apple_native_keyring_store::keychain::Store::new()?)
}

#[cfg(all(
    unix,
    not(any(target_os = "macos", target_os = "ios", target_os = "android"))
))]
fn native_store() -> keyring_core::Result<Arc<CredentialStore>> {
    Ok(dbus_secret_service_keyring_store::Store::new()?)
}

#[cfg(target_os = "windows")]
fn native_store() -> keyring_core::Result<Arc<CredentialStore>> {
    Ok(windows_native_keyring_store::Store::new()?)
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "windows",
    all(
        unix,
        not(any(target_os = "macos", target_os = "ios", target_os = "android"))
    )
)))]
fn native_store() -> keyring_core::Result<Arc<CredentialStore>> {
    Err(keyring_core::Error::NotSupportedByStore(String::from(
        "no native credential store is configured for this platform",
    )))
}

#[cfg(test)]
mod tests {
    use keyring_core::api::CredentialStoreApi;
    use keyring_core::mock::Store;

    use super::{
        SERVICE, delete_access_token_from_store, load_access_token_from_store,
        store_access_token_in_store,
    };
    #[cfg(target_os = "linux")]
    use super::{delete_access_token, load_access_token, store_access_token};

    #[test]
    fn existing_profile_entry_is_read_updated_and_deleted_without_changing_its_identity()
    -> anyhow::Result<()> {
        let store = Store::new()?;
        let existing = store.build(SERVICE, "home:access_token", None)?;
        existing.set_password("existing-token")?;

        assert_eq!(
            load_access_token_from_store(store.as_ref(), "home")?,
            Some(String::from("existing-token"))
        );
        assert_eq!(load_access_token_from_store(store.as_ref(), "work")?, None);

        store_access_token_in_store(store.as_ref(), "home", "updated-token")?;
        assert_eq!(existing.get_password()?, "updated-token");

        delete_access_token_from_store(store.as_ref(), "home")?;
        delete_access_token_from_store(store.as_ref(), "home")?;
        assert_eq!(load_access_token_from_store(store.as_ref(), "home")?, None);
        assert!(matches!(
            existing.get_password(),
            Err(keyring_core::Error::NoEntry)
        ));
        Ok(())
    }

    #[test]
    fn credential_read_write_and_delete_failures_keep_their_operation_context() -> anyhow::Result<()>
    {
        let store = Store::new()?;
        let entry = store.build(SERVICE, "home:access_token", None)?;
        let credential = entry
            .as_any()
            .downcast_ref::<keyring_core::mock::Cred>()
            .ok_or_else(|| anyhow::anyhow!("mock store returned an unexpected credential"))?;
        credential.set_error(keyring_core::Error::Invalid(
            String::from("credential"),
            String::from("synthetic failure"),
        ));

        let read = load_access_token_from_store(store.as_ref(), "home")
            .err()
            .ok_or_else(|| anyhow::anyhow!("read unexpectedly succeeded"))?;
        assert!(
            read.to_string()
                .contains("failed to read token from OS keychain")
        );

        credential.set_error(keyring_core::Error::Invalid(
            String::from("credential"),
            String::from("synthetic failure"),
        ));
        let write = store_access_token_in_store(store.as_ref(), "home", "synthetic-token")
            .err()
            .ok_or_else(|| anyhow::anyhow!("write unexpectedly succeeded"))?;
        assert!(
            write
                .to_string()
                .contains("failed to store token in OS keychain")
        );

        credential.set_error(keyring_core::Error::Invalid(
            String::from("credential"),
            String::from("synthetic failure"),
        ));
        let delete = delete_access_token_from_store(store.as_ref(), "home")
            .err()
            .ok_or_else(|| anyhow::anyhow!("delete unexpectedly succeeded"))?;
        assert!(
            delete
                .to_string()
                .contains("failed to delete token from OS keychain")
        );
        Ok(())
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "requires the private GNOME Secret Service session started by the Task target"]
    fn existing_secret_tool_item_is_interoperable() -> anyhow::Result<()> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        anyhow::ensure!(
            std::env::var_os("MEDTRACKER_PRIVATE_SECRET_SERVICE").is_some(),
            "run this test through task client-tools:test:secret-service"
        );
        let profile = "private-session";
        let username = "private-session:access_token";
        let mut seed = Command::new("secret-tool")
            .args([
                "store",
                "--collection=default",
                "--label=MedTracker synthetic test",
                "service",
                SERVICE,
                "username",
                username,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        seed.stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("secret-tool stdin was unavailable"))?
            .write_all(b"synthetic-before")?;
        anyhow::ensure!(
            seed.wait()?.success(),
            "secret-tool could not seed the private store"
        );

        assert_eq!(
            load_access_token(profile)?.as_deref(),
            Some("synthetic-before")
        );
        store_access_token(profile, "synthetic-after")?;
        let lookup = Command::new("secret-tool")
            .args(["lookup", "service", SERVICE, "username", username])
            .output()?;
        anyhow::ensure!(
            lookup.status.success(),
            "secret-tool could not read the updated item"
        );
        assert_eq!(lookup.stdout, b"synthetic-after");

        delete_access_token(profile)?;
        delete_access_token(profile)?;
        assert_eq!(load_access_token(profile)?, None);
        let missing = Command::new("secret-tool")
            .args(["lookup", "service", SERVICE, "username", username])
            .output()?;
        assert!(!missing.status.success());
        Ok(())
    }
}
