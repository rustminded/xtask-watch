use std::process::Command;
use xtask_watch::{anyhow::Result, clap};

#[derive(clap::Parser)]
enum Opt {
    #[group(skip)]
    Watch {
        /// Run cargo check.
        #[arg(long)]
        check: bool,

        /// Run cargo test.
        #[arg(long)]
        test: bool,

        /// Run cargo clippy.
        #[arg(long)]
        clippy: bool,

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
        Opt::Watch { check, test, clippy, watch } => {
            log::info!("Starting to watch");
            let mut list = Vec::with_capacity(10);

            if check {
                let mut check = Command::new("cargo");
                check.args(["check", "--workspace"]);
                list.push(check);
            }

            if test {
                let mut test = Command::new("cargo");
                test.args(["test", "--workspace"]);
                list.push(test);
            }

            if clippy {
                let mut clippy = Command::new("cargo");
                clippy.args(["clippy", "--workspace"]);
                list.push(clippy);
            }

            let mut sleep = Command::new("bash");
            sleep.arg("-c");
            sleep.arg("echo sleeping for 10 seconds for testing purposes... don't mind me; \
                sleep 10; echo sleep ended");
            list.push(sleep);

            watch.run(list)?;
        }
    }

    Ok(())
}
