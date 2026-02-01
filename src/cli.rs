use anyhow::Error;
use clap::{Parser, Subcommand};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use remoteprocess::Pid;

use crate::{
    app::SamplerOps, config::AppConfig, processes::SystemProcessDiscovery, state::ProfilingState,
    subprocess_sampler, tabs::process_selection::ProcessSelectionState,
};

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    pid: Option<Pid>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Internal: run as subprocess worker (hidden from help)
    #[command(hide = true)]
    SubprocessWorker,
    /// Show the current configuration and exit
    ShowConfig,
    /// Run the demo with mock data (no real process attachment)
    #[cfg(feature = "demo")]
    Demo,
}

fn init_tracing() -> tracing_appender::non_blocking::WorkerGuard {
    let file_appender = tracing_appender::rolling::never("/tmp", "fadetop.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .init();

    guard
}

pub async fn run() -> Result<(), Error> {
    let _guard = init_tracing();
    let args = Args::parse();

    tracing::debug!(?args, "Parsed command line arguments");

    // Handle subprocess worker mode (no config loading, no terminal)
    if let Some(Command::SubprocessWorker) = args.command {
        tracing::info!("Starting subprocess worker");
        return subprocess_sampler::run_worker();
    }

    let configs = AppConfig::from_configs()?;

    // Handle show-config command
    if let Some(Command::ShowConfig) = args.command {
        println!("{:#?}", configs);
        return Ok(());
    }

    // Handle demo command
    #[cfg(feature = "demo")]
    if let Some(Command::Demo) = args.command {
        return crate::demo::run().await;
    }

    let mut terminal = ratatui::init();

    let result = if let Some(pid) = args.pid {
        // Direct attach mode (CLI with PID argument)
        ProfilingState::new(&configs)
            .run_attached(
                &mut terminal,
                subprocess_sampler::SubprocessSampler::from_config_and_id(&configs, pid)?,
            )
            .await
    } else {
        // Landing page mode (no PID argument)
        ProcessSelectionState::new()
            .run_with_selection::<subprocess_sampler::SubprocessSampler, SystemProcessDiscovery>(
                &configs, terminal,
            )
            .await
    };

    ratatui::restore();
    result
}
