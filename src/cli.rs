use anyhow::Error;
use clap::{Parser, Subcommand};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use remoteprocess::Pid;

use crate::{
    app::SamplerOps,
    config::AppConfig,
    processes::{ProcessDiscovery, SystemProcessDiscovery},
    state::ProfilingState,
    subprocess_sampler,
    tabs::process_selection::ProcessSelectionState,
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
    /// List running Python processes and exit
    ListProcesses {
        /// Print JSON (used by the VS Code extension)
        #[arg(long)]
        json: bool,
    },
    /// Run the demo with mock data (no real process attachment)
    #[cfg(feature = "demo")]
    Demo,
}

fn init_tracing() -> tracing_appender::non_blocking::WorkerGuard {
    // Opening the log can fail, e.g. under sudo when /tmp/fadetop.log belongs to
    // another user (fs.protected_regular). Don't let logging stop the app.
    let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
        .filename_prefix("fadetop.log")
        .build(std::env::temp_dir());
    let (non_blocking, guard) = match file_appender {
        Ok(appender) => tracing_appender::non_blocking(appender),
        Err(_) => tracing_appender::non_blocking(std::io::sink()),
    };

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

    if let Some(Command::ListProcesses { json }) = args.command {
        return list_processes(json);
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

    let result = if let Some(pid) = args.pid {
        // Direct attach mode (CLI with PID argument). Create the sampler before
        // taking over the terminal so its errors stay visible.
        let sampler = subprocess_sampler::SubprocessSampler::from_config_and_id(&configs, pid)?;
        let terminal = ratatui::init();
        ProfilingState::new(&configs)
            .run_direct(terminal, sampler)
            .await
    } else {
        // Landing page mode (no PID argument)
        let terminal = ratatui::init();
        ProcessSelectionState::new()
            .run_with_selection::<subprocess_sampler::SubprocessSampler, SystemProcessDiscovery>(
                &configs, terminal,
            )
            .await
    };

    ratatui::restore();
    result
}

fn list_processes(json: bool) -> Result<(), Error> {
    let processes = SystemProcessDiscovery::discover();
    if json {
        println!("{}", serde_json::to_string(&processes)?);
    } else {
        for p in &processes {
            println!("{:>8}  {:<16}  {}", p.pid, p.summary(), p.cmdline);
        }
    }
    Ok(())
}
