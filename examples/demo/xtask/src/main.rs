use std::process::Command;
use xtask_watch::{
    clap,
    clap::Parser,
};

#[derive(Parser)]
enum Opt {
    Watch(xtask_watch::Watch),
}

fn main() {
    let opt: Opt = Parser::parse();

    env_logger::builder()
        .filter(Some("xtask"), log::LevelFilter::Trace)
        .init();

    let mut run_command = Command::new("cargo");
    run_command.arg("check");

    match opt {
        Opt::Watch(watch) => {
            log::info!("starting to watch `project`");
            watch.run(run_command).expect("cannot run watch");
        }
    }
}
