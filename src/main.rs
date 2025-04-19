use std::time::Duration;

use anyhow::Error;
use clap::{Parser, command};
use fadetop::app::FadeTopApp;
use py_spy;
use remoteprocess::Pid;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    pid: Pid,
}

#[derive(Deserialize)]
struct AppConfig {
    sampling_rate: u64,
    window_width_seconds: u64,
    subprocesses: bool,
    native: bool,
    // 1/128 max length of string repr of variable
    dump_locals: u64,
}

fn main() -> Result<(), Error> {
    let args = Args::try_parse()?;
    let configs = config::Config::builder()
        .set_default("sampling_rate", "100")?
        .set_default("window_width_seconds", "100")?
        .set_default("subprocesses", "true")?
        .set_default("native", "true")?
        .set_default("dump_locals", "1")?
        .add_source(config::File::with_name("fadetop_config.toml").required(false))
        .add_source(config::Environment::with_prefix("FADETOP"))
        .build()?
        .try_deserialize::<AppConfig>()?;

    let terminal = ratatui::init();
    let app =
        FadeTopApp::new().with_viewport_window(Duration::from_secs(configs.window_width_seconds));

    let result = app.run(
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
    );
    ratatui::restore();
    result
}
