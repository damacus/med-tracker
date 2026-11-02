use anyhow::{Context, Result};
use keyring_core::api::CredentialStore;
use std::sync::Arc;

const SERVICE: &str = "medtracker-cli";

#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct SavedCredential {
    version: u8,
    pub origin: String,
    pub token: String,
}

impl SavedCredential {
    pub fn new(base_url: &str, token: &str) -> Result<Self> {
        Ok(Self {
            version: 1,
            origin: medtracker_api_client::ApiClient::server_origin(base_url)
                .map_err(crate::output::format_api_error)?,
            token: token.to_owned(),
        })
    }

    fn decode(value: &str) -> Result<Self> {
        let invalid = || {
            anyhow::anyhow!(
                "invalid or legacy saved credential; log in again with auth login --token"
            )
        };
        let credential: Self = serde_json::from_str(value).map_err(|_| invalid())?;
        let origin = medtracker_api_client::ApiClient::server_origin(&credential.origin)
            .map_err(|_| invalid())?;
        anyhow::ensure!(
            credential.version == 1 && credential.origin == origin && !credential.token.is_empty(),
            "{}",
            invalid()
        );
        Ok(credential)
    }
}

pub fn load_access_token(profile: &str) -> Result<Option<SavedCredential>> {
    let store = native_store().context("failed to open OS keychain entry")?;
    load_access_token_from_store(store.as_ref(), profile)
}

fn load_access_token_from_store(
    store: &CredentialStore,
    profile: &str,
) -> Result<Option<SavedCredential>> {
    let entry = entry(store, profile)?;
    match entry.get_password() {
        Ok(value) => SavedCredential::decode(&value).map(Some),
        Err(keyring_core::Error::NoEntry) => Ok(None),
        Err(error) => Err(error).context("failed to read token from OS keychain"),
    }
}

pub fn store_access_token(profile: &str, credential: &SavedCredential) -> Result<()> {
    let store = native_store().context("failed to open OS keychain entry")?;
    store_access_token_in_store(store.as_ref(), profile, credential)
}

fn store_access_token_in_store(
    store: &CredentialStore,
    profile: &str,
    credential: &SavedCredential,
) -> Result<()> {
    let value = serde_json::to_string(credential).context("failed to encode saved credential")?;
    entry(store, profile)?
        .set_password(&value)
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
        SERVICE, SavedCredential, delete_access_token_from_store, load_access_token_from_store,
        store_access_token_in_store,
    };
    #[cfg(target_os = "linux")]
    use super::{delete_access_token, load_access_token, store_access_token};

    #[test]
    fn legacy_and_malformed_credentials_require_login() -> anyhow::Result<()> {
        let store = Store::new()?;
        let entry = store.build(SERVICE, "home:access_token", None)?;
        for value in [
            "legacy-synthetic-token",
            "{",
            r#"{"version":2,"origin":"https://example.com","token":"synthetic"}"#,
            r#"{"version":1,"token":"synthetic"}"#,
            r#"{"version":1,"origin":"file:///tmp","token":"synthetic"}"#,
            r#"{"version":1,"origin":"https://example.com","token":""}"#,
        ] {
            entry.set_password(value)?;
            let result = load_access_token_from_store(store.as_ref(), "home");
            assert!(
                result.is_err(),
                "unbound or malformed credential was accepted"
            );
            let error = result
                .err()
                .ok_or_else(|| anyhow::anyhow!("expected rejection"))?;
            assert!(error.to_string().contains("log in"));
            assert!(!format!("{error:?}").contains("synthetic"));
            assert_eq!(entry.get_password()?, value);
        }
        Ok(())
    }

    #[test]
    fn existing_profile_entry_is_read_updated_and_deleted_without_changing_its_identity()
    -> anyhow::Result<()> {
        let store = Store::new()?;
        let existing = store.build(SERVICE, "home:access_token", None)?;
        let initial = SavedCredential::new("https://EXAMPLE.com:443/path", "existing-token")?;
        assert_eq!(initial.origin, "https://example.com");
        existing.set_password(&serde_json::to_string(&initial)?)?;

        assert!(load_access_token_from_store(store.as_ref(), "home")? == Some(initial));
        assert!(load_access_token_from_store(store.as_ref(), "work")?.is_none());

        let updated = SavedCredential::new("https://other.example", "updated-token")?;
        store_access_token_in_store(store.as_ref(), "home", &updated)?;
        assert!(SavedCredential::decode(&existing.get_password()?)? == updated);
        assert!(load_access_token_from_store(store.as_ref(), "home")? == Some(updated));

        delete_access_token_from_store(store.as_ref(), "home")?;
        delete_access_token_from_store(store.as_ref(), "home")?;
        assert!(load_access_token_from_store(store.as_ref(), "home")?.is_none());
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
        let original = SavedCredential::new("https://original.example", "original-token")?;
        entry.set_password(&serde_json::to_string(&original)?)?;
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
        let write = store_access_token_in_store(
            store.as_ref(),
            "home",
            &SavedCredential::new("https://new.example", "synthetic-token")?,
        )
        .err()
        .ok_or_else(|| anyhow::anyhow!("write unexpectedly succeeded"))?;
        assert!(
            write
                .to_string()
                .contains("failed to store token in OS keychain")
        );

        assert!(load_access_token_from_store(store.as_ref(), "home")? == Some(original));

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
            .write_all(
                serde_json::to_string(&SavedCredential::new(
                    "https://example.com",
                    "synthetic-before",
                )?)?
                .as_bytes(),
            )?;
        anyhow::ensure!(
            seed.wait()?.success(),
            "secret-tool could not seed the private store"
        );

        assert_eq!(
            load_access_token(profile)?.map(|credential| credential.token),
            Some("synthetic-before".to_owned())
        );
        let updated = SavedCredential::new("https://other.example", "synthetic-after")?;
        store_access_token(profile, &updated)?;
        let lookup = Command::new("secret-tool")
            .args(["lookup", "service", SERVICE, "username", username])
            .output()?;
        anyhow::ensure!(
            lookup.status.success(),
            "secret-tool could not read the updated item"
        );
        assert!(
            SavedCredential::decode(std::str::from_utf8(&lookup.stdout)?.trim_end())? == updated
        );

        delete_access_token(profile)?;
        delete_access_token(profile)?;
        assert!(load_access_token(profile)?.is_none());
        let missing = Command::new("secret-tool")
            .args(["lookup", "service", SERVICE, "username", username])
            .output()?;
        assert!(!missing.status.success());
        Ok(())
    }
}
