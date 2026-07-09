use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "medtracker")]
#[command(about = "Operate MedTracker through the hosted API")]
pub struct Cli {
    #[arg(
        long,
        env = "MEDTRACKER_BASE_URL",
        default_value = "http://localhost:3000"
    )]
    pub base_url: String,
    #[arg(long, env = "MEDTRACKER_PROFILE", default_value = "default")]
    pub profile: String,
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub output: OutputFormat,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Json,
    Table,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Capabilities,
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    Households {
        #[command(subcommand)]
        command: HouseholdsCommand,
    },
    Me(HouseholdArgs),
    Resources {
        #[command(subcommand)]
        command: ResourceCommand,
    },
    Portable {
        #[command(subcommand)]
        command: PortableCommand,
    },
    Backup {
        #[command(subcommand)]
        command: BackupCommand,
    },
    Sync {
        #[command(subcommand)]
        command: SyncCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    Login(LoginArgs),
    Refresh(RefreshArgs),
    Logout,
    Status,
}

#[derive(Debug, Args)]
pub struct LoginArgs {
    #[arg(long, env = "MEDTRACKER_EMAIL")]
    pub email: String,
    #[arg(long, env = "MEDTRACKER_PASSWORD")]
    pub password: String,
    #[arg(long, default_value = "medtracker-cli")]
    pub device_name: String,
}

#[derive(Debug, Args)]
pub struct RefreshArgs {
    #[arg(long, env = "MEDTRACKER_REFRESH_TOKEN")]
    pub refresh_token: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum HouseholdsCommand {
    List,
}

#[derive(Debug, Args)]
pub struct HouseholdArgs {
    #[arg(long, env = "MEDTRACKER_HOUSEHOLD_ID")]
    pub household_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ResourceKindArg {
    People,
    Locations,
    Medications,
    DosageOptions,
    HealthEvents,
    Schedules,
    PersonMedications,
    MedicationTakes,
}

#[derive(Debug, Subcommand)]
pub enum ResourceCommand {
    List(ResourceListArgs),
}

#[derive(Debug, Args)]
pub struct ResourceListArgs {
    #[arg(long, env = "MEDTRACKER_HOUSEHOLD_ID")]
    pub household_id: String,
    #[arg(long, value_enum)]
    pub kind: ResourceKindArg,
}

#[derive(Debug, Subcommand)]
pub enum PortableCommand {
    Export(HouseholdArgs),
    Import(PortableImportArgs),
    DryRun(PortableImportArgs),
}

#[derive(Debug, Args)]
pub struct PortableImportArgs {
    #[arg(long, env = "MEDTRACKER_HOUSEHOLD_ID")]
    pub household_id: String,
    #[arg(long)]
    pub file: std::path::PathBuf,
    #[arg(long)]
    pub passphrase_stdin: bool,
}

#[derive(Debug, Subcommand)]
pub enum BackupCommand {
    Export(BackupExportArgs),
}

#[derive(Debug, Args)]
pub struct BackupExportArgs {
    #[arg(long, env = "MEDTRACKER_HOUSEHOLD_ID")]
    pub household_id: String,
    #[arg(long, default_value = "health_data_json")]
    pub mode: String,
}

#[derive(Debug, Subcommand)]
pub enum SyncCommand {
    Snapshot(HouseholdArgs),
    Changes(SyncChangesArgs),
    Batch(SyncBatchArgs),
}

#[derive(Debug, Args)]
pub struct SyncChangesArgs {
    #[arg(long, env = "MEDTRACKER_HOUSEHOLD_ID")]
    pub household_id: String,
    #[arg(long)]
    pub since: Option<String>,
}

#[derive(Debug, Args)]
pub struct SyncBatchArgs {
    #[arg(long, env = "MEDTRACKER_HOUSEHOLD_ID")]
    pub household_id: String,
    #[arg(long)]
    pub file: std::path::PathBuf,
}

impl From<ResourceKindArg> for medtracker_api_client::ResourceKind {
    fn from(value: ResourceKindArg) -> Self {
        match value {
            ResourceKindArg::People => Self::People,
            ResourceKindArg::Locations => Self::Locations,
            ResourceKindArg::Medications => Self::Medications,
            ResourceKindArg::DosageOptions => Self::DosageOptions,
            ResourceKindArg::HealthEvents => Self::HealthEvents,
            ResourceKindArg::Schedules => Self::Schedules,
            ResourceKindArg::PersonMedications => Self::PersonMedications,
            ResourceKindArg::MedicationTakes => Self::MedicationTakes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn help_lists_expected_command_groups() {
        let mut command = Cli::command();
        let help = command.render_long_help().to_string();

        for group in [
            "capabilities",
            "auth",
            "households",
            "me",
            "resources",
            "portable",
            "backup",
            "sync",
        ] {
            assert!(help.contains(group), "help did not include {group}");
        }
    }

    #[test]
    fn portable_import_does_not_accept_passphrase_argument() {
        let error = Cli::try_parse_from([
            "medtracker",
            "portable",
            "import",
            "--household-id",
            "1",
            "--file",
            "bundle.json",
            "--passphrase",
            "secret",
        ])
        .unwrap_err();

        assert!(error.to_string().contains("unexpected argument"));
    }
}
