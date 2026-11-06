use clap::{
    ArgAction, ArgGroup, CommandFactory, FromArgMatches, Parser, Subcommand, parser::ValueSource,
};
use loco_rs::{
    app::Hooks,
    boot::{ServeParams, StartMode, create_context, start},
    environment::{Environment, resolve_from_env},
};
use med_tracker::app::App;
use migration::Migrator;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

#[derive(Parser)]
struct Launcher {
    #[arg(short, long, global = true)]
    environment: Option<String>,
    #[arg(long, env = "SCHEDULER_CONFIG", global = true, hide = true)]
    scheduler_config: Option<PathBuf>,
    #[command(subcommand)]
    command: LauncherCommand,
}

#[derive(Subcommand)]
enum LauncherCommand {
    Scheduler {
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        tag: Option<String>,
        #[arg(short = 'c', long = "config")]
        config_path: Option<PathBuf>,
        #[arg(short, long)]
        list: bool,
    },
    #[command(alias = "s", group(ArgGroup::new("start_mode").args(["worker", "server_and_worker", "all"])))]
    Start {
        #[arg(short, long, action, value_delimiter = ',', num_args = 0.., conflicts_with_all = ["server_and_worker", "all"])]
        worker: Option<Vec<String>>,
        #[arg(short, long, action, conflicts_with_all = ["worker", "all"])]
        server_and_worker: bool,
        #[arg(short, long, action, conflicts_with_all = ["worker", "server_and_worker"])]
        all: bool,
        #[arg(long, action, conflicts_with = "all")]
        scheduler: bool,
        #[arg(short, long)]
        binding: Option<String>,
        #[arg(short, long)]
        port: Option<i32>,
        #[arg(short, long, action = ArgAction::SetTrue)]
        no_banner: bool,
    },
    #[command(external_subcommand)]
    Other(Vec<OsString>),
}

fn validate_scheduler(config: &loco_rs::config::Config, path: Option<&Path>) -> Result<(), String> {
    let external = path
        .map(|path| {
            let contents = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
            serde_yaml_ng::from_str::<loco_rs::scheduler::Config>(&contents)
                .map_err(|error| error.to_string())
        })
        .transpose()?;
    if let Some(scheduler) = external.as_ref().or(config.scheduler.as_ref()) {
        for job in scheduler.jobs.values() {
            if !job.shell && job.run.contains('\0') {
                return Err("scheduler command contains a NUL".into());
            }
        }
    }
    Ok(())
}

#[tokio::main]
#[allow(
    clippy::result_large_err,
    reason = "Loco's framework error type contains ordered JSON values"
)]
async fn main() -> loco_rs::Result<()> {
    let launcher = Launcher::command()
        .try_get_matches()
        .ok()
        .and_then(|matches| {
            if matches.value_source("scheduler_config") == Some(ValueSource::CommandLine) {
                None
            } else {
                Launcher::from_arg_matches(&matches).ok()
            }
        });
    let (
        environment,
        scheduler_config,
        worker,
        server_and_worker,
        all,
        scheduler,
        binding,
        port,
        no_banner,
    ) = match launcher {
        Some(Launcher {
            environment,
            scheduler_config,
            command:
                LauncherCommand::Start {
                    worker,
                    server_and_worker,
                    all,
                    scheduler,
                    binding,
                    port,
                    no_banner,
                },
        }) => (
            environment,
            scheduler_config,
            worker,
            server_and_worker,
            all,
            scheduler,
            binding,
            port,
            no_banner,
        ),
        Some(Launcher {
            environment,
            scheduler_config,
            command: LauncherCommand::Scheduler { config_path, .. },
        }) => {
            let environment: Environment = environment.unwrap_or_else(resolve_from_env).into();
            let config = App::load_config(&environment).await?;
            validate_scheduler(
                &config,
                config_path.as_deref().or(scheduler_config.as_deref()),
            )
            .map_err(loco_rs::Error::Message)?;
            return loco_rs::cli::main::<App, Migrator>().await;
        }
        Some(Launcher {
            command: LauncherCommand::Other(args),
            ..
        }) => {
            drop(args);
            return loco_rs::cli::main::<App, Migrator>().await;
        }
        None => return loco_rs::cli::main::<App, Migrator>().await,
    };
    let environment: Environment = environment.unwrap_or_else(resolve_from_env).into();
    let config = App::load_config(&environment).await?;
    if all || scheduler {
        validate_scheduler(&config, scheduler_config.as_deref())
            .map_err(loco_rs::Error::Message)?;
    }
    let initial_context = create_context::<App>(&environment, config).await?;
    if !App::init_logger(&initial_context)? {
        loco_rs::logger::init::<App>(&initial_context.config.logger)?;
    }
    let config = initial_context.config.clone();
    if let Some(queue) = &initial_context.queue_provider {
        queue.shutdown()?;
    }
    drop(initial_context);
    let mode = if all || (server_and_worker && scheduler) {
        StartMode::All
    } else if server_and_worker {
        StartMode::ServerAndWorker
    } else if let Some(tags) = worker {
        if scheduler {
            StartMode::WorkerAndScheduler { tags }
        } else {
            StartMode::WorkerOnly { tags }
        }
    } else if scheduler {
        StartMode::ServerAndScheduler
    } else {
        StartMode::ServerOnly
    };
    let boot = App::boot(mode, &environment, config).await?;
    let server = ServeParams {
        port: port.unwrap_or(boot.app_context.config.server.port),
        binding: binding.unwrap_or_else(|| boot.app_context.config.server.binding.clone()),
    };
    start::<App>(boot, server, no_banner).await
}
