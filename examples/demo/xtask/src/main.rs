use std::process::Command;
use xtask_watch::{anyhow::Result, clap};

#[derive(clap::Parser)]
enum Opt {
    #[group(skip)]
    Watch {
        /// Command executed when changes are detected.
        ///
        /// If nothing is provided, `cargo check` will be executed.
        command: Vec<String>,

        #[clap(flatten)]
        watch: xtask_watch::Watch,
    },
}

fn main() -> Result<()> {
    let opt: Opt = clap::Parser::parse();

    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    match opt {
        Opt::Watch { command, watch } => {
            log::info!("starting to watch");
            if !command.is_empty() {
                let mut it = command.iter();

                let mut command = Command::new(it.next().unwrap());
                command.args(it);

                watch.run(command)?;
            } else {
                let mut check = Command::new("cargo");
                check.arg("check");

                let mut test = Command::new("cargo");
                test.arg("test");

                let mut sleep = Command::new("bash");
                sleep.arg("-c");
                sleep.arg("echo sleeping for 10 seconds...; sleep 10; echo sleep ended");

                watch.run([check, test, sleep])?;
            }
        }
    }

    Ok(())
}
