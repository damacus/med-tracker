use std::io::Read;

use anyhow::{Context, Result};
use medtracker_api_client::{ApiClient, PortableImportMode};
use serde_json::Value;

use crate::args::{
    AuthCommand, BackupCommand, Cli, Command, HouseholdsCommand, PortableCommand, ResourceCommand,
    SyncCommand,
};
use crate::config::save_profile;
use crate::output::{format_api_error, print_value};
use crate::secrets::{
    delete_access_token, extract_access_token, load_access_token, store_access_token,
};

pub async fn run(cli: Cli) -> Result<()> {
    let token = load_access_token(&cli.profile)?;
    let client = ApiClient::new(&cli.base_url, token).map_err(format_api_error)?;
    let profile = cli.profile.clone();
    let base_url = cli.base_url.clone();
    let output = cli.output;

    match cli.command {
        Command::Capabilities => {
            let value: Value = client
                .get_json("/api/v1/capabilities")
                .await
                .map_err(format_api_error)?;
            print_value(&value, output)?;
        }
        Command::Auth { command } => run_auth(command, &profile, &base_url, &client).await?,
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
) -> Result<()> {
    match command {
        AuthCommand::Login(args) => {
            let response = client
                .login(&args.email, &args.password, &args.device_name)
                .await
                .map_err(format_api_error)?;
            if let Some(token) = extract_access_token(&response) {
                store_access_token(profile, token)?;
            }
            let path = save_profile(profile, base_url)?;
            println!("authenticated profile={profile} config={}", path.display());
        }
        AuthCommand::Refresh(args) => {
            let refresh_token = args.refresh_token.context(
                "refresh token must be passed via --refresh-token or MEDTRACKER_REFRESH_TOKEN",
            )?;
            let response = client
                .refresh(&refresh_token)
                .await
                .map_err(format_api_error)?;
            if let Some(token) = extract_access_token(&response) {
                store_access_token(profile, token)?;
            }
            println!("refreshed profile={profile}");
        }
        AuthCommand::Logout => {
            client.logout().await.map_err(format_api_error)?;
            delete_access_token(profile)?;
            println!("logged out profile={profile}");
        }
        AuthCommand::Status => {
            if load_access_token(profile)?.is_some() {
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
