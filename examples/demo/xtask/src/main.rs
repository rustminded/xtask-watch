use std::process::Command;
use xtask_watch::{
    anyhow::Result,
    clap,
};

#[derive(clap::Parser)]
enum Opt {
    Watch(xtask_watch::Watch),
}

fn main() -> Result<()> {
    let opt: Opt = clap::Parser::parse();

    env_logger::builder()
    .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let mut run_command = Command::new("cargo");
    run_command.arg("check");

    match opt {
        Opt::Watch(watch) => {
            log::info!("starting to watch `project`");
            watch.run(run_command)?;
        }
    }

    Ok(())
}
