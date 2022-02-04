use std::process::Command;
use xtask_watch::{
    anyhow::Result,
    clap,
};

#[derive(clap::Parser)]
enum Opt {
    Watch {
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
        Opt::Watch {
            command,
            watch,
        } => {
            let command = if !command.is_empty() {
                let mut it = command.iter();

                let mut command = Command::new(it.next().unwrap());
                command.args(it);

                command
            } else {
                let mut command = Command::new("cargo");
                command.arg("check");

                command
            };

            log::info!("starting to watch");
            watch.run(command)?;
        }
    }

    Ok(())
}
