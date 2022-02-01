use anyhow::Result;
use std::fs;
use toml;

mod config;

use crate::config::Config;

fn main() -> Result<()> {
    let config = toml::ser::to_string(&Config::new()).unwrap();
    fs::write("project/config.toml", &config).unwrap();

    println!("{}", config);
    Ok(())
}
