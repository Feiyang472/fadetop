use std::{collections::HashMap, env, time::Duration};

use anyhow::Error;
use clap::{CommandFactory, FromArgMatches, Parser, command};
use config::{Value, ValueKind};
use fadetop::app::FadeTopApp;
use fadetop::ser::parse_duration;
use py_spy;
use remoteprocess::Pid;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    pid: Pid,
}

#[derive(Deserialize, Debug)]
struct AppConfig {
    sampling_rate: u64,
    #[serde(deserialize_with = "parse_duration")]
    window_width: Duration,
    subprocesses: bool,
    native: bool,
    // 1/128 max length of string repr of variable
    dump_locals: u64,
    rules: Vec<fadetop::priority::ForgetRules>,
}

fn main() -> Result<(), Error> {
    let config_file =
        env::var("FADETOP_CONFIG").unwrap_or_else(|_| "fadetop_config.toml".to_string());

    let configs = config::Config::builder()
        .set_default("sampling_rate", "100")?
        .set_default("window_width", "100s")?
        .set_default("subprocesses", "true")?
        .set_default("native", "true")?
        .set_default("dump_locals", "1")?
        .set_default(
            "rules",
            Value::new(
                None,
                ValueKind::Array(vec![Value::new(
                    None,
                    ValueKind::Table(HashMap::from([
                        ("type".to_string(), "rectlinear".into()),
                        ("at_least".to_string(), "60s".into()),
                        ("ratio".to_string(), 0.0.into()),
                    ])),
                )]),
            ),
        )?
        .add_source(config::File::with_name(&config_file).required(false))
        .add_source(config::Environment::with_prefix("FADETOP"))
        .build()?
        .try_deserialize::<AppConfig>()?;

    let cmd =
        Args::command().after_help(format!("Fadetop is being run with configs\n{:#?}", configs));

    let args = Args::from_arg_matches_mut(&mut cmd.try_get_matches()?)?;

    let terminal = ratatui::init();
    let app = FadeTopApp::new()
        .with_viewport_window(configs.window_width)
        .with_rules(configs.rules)?;

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
