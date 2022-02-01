use std::process;
use xtask_watch::{
    anyhow::{ensure, Context, Result},
    clap,
};

#[derive(clap::Parser)]
enum Opt {
    Build,
    Watch(xtask_watch::Watch),
}

fn main() -> Result<()> {
    let opt: Opt = clap::Parser::parse();

    env_logger::builder()
        .filter(Some("xtask"), log::LevelFilter::Trace)
        .init();

    let mut run_command = process::Command::new("cargo");
    run_command.args(["run", "--package", "project"]);

    match opt {
        Opt::Build => {
            log::info!("running `project`");
            ensure!(
                run_command
                    .status()
                    .context("could not start cargo")?
                    .success(),
                "run command failed"
            );
        }
        Opt::Watch(watch) => {
            log::info!("starting to watch `project`");
            watch.run(run_command)?;
        }
    }

    Ok(())
}
