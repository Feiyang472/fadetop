use anyhow::Error;
use clap::{CommandFactory, FromArgMatches, Parser, command};
use fadetop::{app::FadeTopApp, config::AppConfig};

use py_spy;
use remoteprocess::Pid;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    pid: Pid,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    let configs = AppConfig::from_configs()?;

    let cmd =
        Args::command().after_help(format!("Fadetop is being run with configs\n{:#?}", configs));

    let args = Args::from_arg_matches_mut(&mut cmd.try_get_matches()?)?;

    let terminal = ratatui::init();
    let app = FadeTopApp::new(configs.clone());

    let result = app
        .run(
            terminal,
            py_spy::sampler::Sampler::new(
                args.pid,
                &py_spy::Config {
                    blocking: py_spy::config::LockingStrategy::NonBlocking,
                    sampling_rate: configs.sampling_rate,
                    subprocesses: configs.subprocesses,
                    native: configs.native,
                    dump_locals: configs.dump_locals,
                    ..Default::default()
                },
            )?,
        )
        .await;
    ratatui::restore();
    result
}
