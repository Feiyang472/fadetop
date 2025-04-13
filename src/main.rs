use anyhow::Error;
use clap::{Parser, command};
use fadetop::app::FadeTopApp;
use py_spy::{Config, config::LockingStrategy};
use remoteprocess::Pid;

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    pid: Pid,

    #[clap(default_value = "100")]
    rate: u64,

    #[clap(default_value = "false")]
    subprocesses: bool,

    #[clap(default_value = "false")]
    native: bool,
}

fn main() -> Result<(), Error> {
    let args = Args::try_parse()?;

    let terminal = ratatui::init();
    let app = FadeTopApp::new((
        args.pid,
        Config {
            blocking: LockingStrategy::NonBlocking,
            sampling_rate: args.rate,
            subprocesses: args.subprocesses,
            native: args.native,
            ..Default::default()
        },
    ));

    let result = app.run(terminal);
    ratatui::restore();
    result
}
