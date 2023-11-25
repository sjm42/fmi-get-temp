// config.rs

use log::*;
use serde::{Deserialize, Serialize};
use std::env;
use std::{fs::File, io::BufReader};
use structopt::StructOpt;

#[derive(Debug, Clone, StructOpt)]
pub struct OptsCommon {
    #[structopt(short, long)]
    pub verbose: bool,
    #[structopt(short, long)]
    pub debug: bool,
    #[structopt(short, long)]
    pub trace: bool,

    #[structopt(short, long, default_value = "$HOME/.config/fmi/config.json")]
    pub fmi_config: String,
}

impl OptsCommon {
    pub fn finish(&mut self) -> anyhow::Result<()> {
        self.fmi_config = shellexpand::full(&self.fmi_config)?.into_owned();
        Ok(())
    }
    pub fn get_loglevel(&self) -> LevelFilter {
        if self.trace {
            LevelFilter::Trace
        } else if self.debug {
            LevelFilter::Debug
        } else if self.verbose {
            LevelFilter::Info
        } else {
            LevelFilter::Error
        }
    }
    pub fn start_pgm(&self, name: &str) {
        env_logger::Builder::new()
            .filter_module(env!("CARGO_PKG_NAME"), self.get_loglevel())
            .filter_module(name, self.get_loglevel())
            .format_timestamp_secs()
            .init();
        info!(
            "Starting up {} v{}...",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        );
        debug!("Git branch: {}", env!("GIT_BRANCH"));
        debug!("Git commit: {}", env!("GIT_COMMIT"));
        debug!("Source timestamp: {}", env!("SOURCE_TIMESTAMP"));
        debug!("Compiler version: {}", env!("RUSTC_VERSION"));
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FmiConfig {
    pub url_temp: String,
    pub fmi_sid: String,
}

impl FmiConfig {
    pub fn new(opts: &OptsCommon) -> anyhow::Result<Self> {
        let file = &opts.fmi_config;
        info!("Reading config file {file}");
        let config: FmiConfig = serde_json::from_reader(BufReader::new(File::open(file)?))?;
        debug!("New FmiConfig:\n{config:#?}");
        Ok(config)
    }
}

// EOF
