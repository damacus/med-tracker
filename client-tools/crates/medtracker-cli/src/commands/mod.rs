use std::io::Read;

use anyhow::{Context, Result};
use medtracker_api_client::{ApiClient, PortableImportMode};
use serde_json::Value;

use crate::args::{
    AuthCommand, BackupCommand, Cli, Command, HouseholdsCommand, PortableCommand, ResourceCommand,
    SyncCommand,
};
use crate::config::{ProfileConfig, load_profile, save_profile};
use crate::output::{format_api_error, print_value};
use crate::secrets::{SavedCredential, delete_access_token, load_access_token, store_access_token};

pub async fn run(cli: Cli) -> Result<()> {
    let saved = profile_for_command(&cli, || load_profile(&cli.profile))?;
    let token = if matches!(
        cli.command,
        Command::Auth {
            command: AuthCommand::Login(_)
        }
    ) {
        None
    } else if let Ok(token) = std::env::var("MEDTRACKER_TOKEN") {
        Some(Credential::Explicit(token))
    } else {
        load_access_token(&cli.profile)?.map(Credential::Saved)
    };
    run_with_profile(cli, saved, token).await
}

fn profile_for_command(
    cli: &Cli,
    load: impl FnOnce() -> Result<Option<ProfileConfig>>,
) -> Result<Option<ProfileConfig>> {
    if cli.base_url.is_some()
        && matches!(
            cli.command,
            Command::Auth {
                command: AuthCommand::Login(_)
            }
        )
    {
        return Ok(None);
    }
    load()
}

enum Credential {
    Saved(SavedCredential),
    Explicit(String),
}

async fn run_with_profile(
    cli: Cli,
    saved: Option<ProfileConfig>,
    token: Option<Credential>,
) -> Result<()> {
    let base_url = cli
        .base_url
        .as_deref()
        .or_else(|| saved.as_ref().map(|config| config.base_url.as_str()))
        .unwrap_or("http://localhost:3000");
    let token = match token {
        Some(Credential::Saved(credential)) => {
            anyhow::ensure!(
                ApiClient::same_origin(base_url, &credential.origin).map_err(format_api_error)?,
                "saved token belongs to another server; select its profile or log in with auth login --token"
            );
            Some(credential.token)
        }
        Some(Credential::Explicit(token)) => Some(token),
        None => None,
    };
    let has_token = token.is_some();
    let client = ApiClient::new(base_url, token).map_err(format_api_error)?;
    let profile = cli.profile.clone();
    let base_url = base_url.to_owned();
    let output = cli.output;

    match cli.command {
        Command::Capabilities => {
            let value: Value = client
                .get_json("/api/v1/capabilities")
                .await
                .map_err(format_api_error)?;
            print_value(&value, output)?;
        }
        Command::Auth { command } => {
            run_auth(command, &profile, &base_url, &client, has_token).await?;
        }
        Command::Households { command } => match command {
            HouseholdsCommand::List => {
                let value = client.households().await.map_err(format_api_error)?;
                print_value(&value, output)?;
            }
        },
        Command::Me(args) => {
            let value = client
                .me(&args.household_id)
                .await
                .map_err(format_api_error)?;
            print_value(&value, output)?;
        }
        Command::Resources { command } => match command {
            ResourceCommand::List(args) => {
                let value = client
                    .list_resource(&args.household_id, args.kind.into())
                    .await
                    .map_err(format_api_error)?;
                print_value(&value, output)?;
            }
        },
        Command::Portable { command } => run_portable(command, &client, output).await?,
        Command::Backup { command } => match command {
            BackupCommand::Export(args) => {
                let value = client
                    .backup_export(&args.household_id, &args.mode)
                    .await
                    .map_err(format_api_error)?;
                print_value(&value, output)?;
            }
        },
        Command::Sync { command } => run_sync(command, &client, output).await?,
    }

    Ok(())
}

async fn run_auth(
    command: AuthCommand,
    profile: &str,
    base_url: &str,
    client: &ApiClient,
    has_token: bool,
) -> Result<()> {
    match command {
        AuthCommand::Login(args) => {
            let authenticated =
                ApiClient::new(base_url, Some(args.token.clone())).map_err(format_api_error)?;
            authenticated.households().await.map_err(format_api_error)?;
            let credential = SavedCredential::new(base_url, &args.token)?;
            store_access_token(profile, &credential)?;
            let path = save_profile(profile, base_url)?;
            println!("authenticated profile={profile} config={}", path.display());
        }
        AuthCommand::Logout => {
            client.logout().await.map_err(format_api_error)?;
            delete_access_token(profile)?;
            println!("logged out profile={profile}");
        }
        AuthCommand::Status => {
            if has_token {
                println!("authenticated profile={profile}");
            } else {
                println!("not authenticated profile={profile}");
            }
        }
    }

    Ok(())
}

async fn run_portable(
    command: PortableCommand,
    client: &ApiClient,
    format: crate::args::OutputFormat,
) -> Result<()> {
    match command {
        PortableCommand::Export(args) => {
            let value = client
                .portable_export(&args.household_id)
                .await
                .map_err(format_api_error)?;
            print_value(&value, format)?;
        }
        PortableCommand::Import(args) => {
            let data = read_json_file(&args.file)?;
            let passphrase = read_passphrase(args.passphrase_stdin)?;
            let value = client
                .portable_import(
                    &args.household_id,
                    data,
                    passphrase,
                    PortableImportMode::Apply,
                )
                .await
                .map_err(format_api_error)?;
            print_value(&value, format)?;
        }
        PortableCommand::DryRun(args) => {
            let data = read_json_file(&args.file)?;
            let passphrase = read_passphrase(args.passphrase_stdin)?;
            let value = client
                .portable_import(
                    &args.household_id,
                    data,
                    passphrase,
                    PortableImportMode::DryRun,
                )
                .await
                .map_err(format_api_error)?;
            print_value(&value, format)?;
        }
    }

    Ok(())
}

async fn run_sync(
    command: SyncCommand,
    client: &ApiClient,
    format: crate::args::OutputFormat,
) -> Result<()> {
    match command {
        SyncCommand::Snapshot(args) => {
            let value = client
                .sync_snapshot(&args.household_id)
                .await
                .map_err(format_api_error)?;
            print_value(&value, format)?;
        }
        SyncCommand::Changes(args) => {
            let value = client
                .sync_changes(&args.household_id, args.since.as_deref())
                .await
                .map_err(format_api_error)?;
            print_value(&value, format)?;
        }
        SyncCommand::Batch(args) => {
            let data = read_json_file(&args.file)?;
            let operations = data
                .get("operations")
                .and_then(Value::as_array)
                .cloned()
                .context("sync batch file must contain an operations array")?;
            let value = client
                .sync_batch(&args.household_id, operations)
                .await
                .map_err(format_api_error)?;
            print_value(&value, format)?;
        }
    }

    Ok(())
}

fn read_json_file(path: &std::path::Path) -> Result<Value> {
    let body = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&body).with_context(|| format!("failed to parse {}", path.display()))
}

fn read_passphrase(enabled: bool) -> Result<Option<String>> {
    if !enabled {
        return Ok(None);
    }

    let mut passphrase = String::new();
    std::io::stdin()
        .read_to_string(&mut passphrase)
        .context("failed to read passphrase from stdin")?;

    Ok(Some(passphrase.trim_end_matches(['\r', '\n']).to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProfileConfig;
    use clap::Parser;
    use wiremock::matchers::{header, method};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn explicit_login_bypasses_corrupt_or_missing_config() -> Result<()> {
        let cli = Cli::try_parse_from([
            "medtracker",
            "--base-url",
            "https://new.example",
            "auth",
            "login",
            "--token",
            "synthetic",
        ])?;
        let saved = profile_for_command(&cli, || anyhow::bail!("corrupt profile config"))?;
        assert!(saved.is_none());
        let saved = profile_for_command(&cli, || Ok(None))?;
        assert!(saved.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn mutable_profile_config_cannot_rebind_saved_credential() -> Result<()> {
        let destination = MockServer::start().await;
        let cli = Cli::try_parse_from(["medtracker", "capabilities"])?;
        let credential = SavedCredential::new("https://trusted.example", "synthetic-token")?;
        let result = run_with_profile(
            cli,
            Some(ProfileConfig {
                base_url: destination.uri(),
            }),
            Some(Credential::Saved(credential)),
        )
        .await;
        let error = result.err().context("mismatched credential was accepted")?;
        assert!(error.to_string().contains("log in"), "{error}");
        assert!(
            destination
                .received_requests()
                .await
                .context("capture disabled")?
                .is_empty()
        );
        Ok(())
    }

    #[tokio::test]
    async fn saved_profile_capabilities_uses_bound_server() -> Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(header("authorization", "Bearer synthetic-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .expect(1)
            .mount(&server)
            .await;
        let cli = Cli::try_parse_from(["medtracker", "capabilities"])?;
        run_with_profile(
            cli,
            Some(ProfileConfig {
                base_url: server.uri(),
            }),
            Some(Credential::Saved(SavedCredential::new(
                &server.uri(),
                "synthetic-token",
            )?)),
        )
        .await?;
        Ok(())
    }

    #[tokio::test]
    async fn saved_profile_allows_same_origin_override() -> Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(header("authorization", "Bearer synthetic-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .expect(1)
            .mount(&server)
            .await;
        let cli = Cli::try_parse_from(["medtracker", "--base-url", &server.uri(), "capabilities"])?;
        run_with_profile(
            cli,
            Some(ProfileConfig {
                base_url: "https://stale-config.example/saved-prefix".into(),
            }),
            Some(Credential::Saved(SavedCredential::new(
                &server.uri(),
                "synthetic-token",
            )?)),
        )
        .await?;
        Ok(())
    }

    #[tokio::test]
    async fn credential_origin_rejects_cross_origin_even_without_config_before_capabilities()
    -> Result<()> {
        let server = MockServer::start().await;
        for saved in [
            None,
            Some(ProfileConfig {
                base_url: "https://saved.example".into(),
            }),
        ] {
            let cli =
                Cli::try_parse_from(["medtracker", "--base-url", &server.uri(), "capabilities"])?;
            let error = run_with_profile(
                cli,
                saved,
                Some(Credential::Saved(SavedCredential::new(
                    "https://trusted.example",
                    "synthetic-token",
                )?)),
            )
            .await
            .err()
            .context("unbound token was accepted")?;
            assert!(error.to_string().contains("log in"), "{error}");
            assert!(
                server
                    .received_requests()
                    .await
                    .context("capture disabled")?
                    .is_empty()
            );
        }
        Ok(())
    }
}
