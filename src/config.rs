// config.rs

use clap::Parser;
use log::*;
use std::env;

#[derive(Debug, Clone, Parser)]
pub struct OptsCommon {
    #[arg(short, long)]
    pub verbose: bool,
    #[arg(short, long)]
    pub debug: bool,
    #[arg(short, long)]
    pub trace: bool,

    #[arg(
        long,
        default_value = "http://opendata.fmi.fi/wfs/fin?service=WFS&version=2.0.0&request=GetFeature&storedquery_id=fmi::observations::weather::timevaluepair&parameters=t2m&fmisid=###FMI_SID###&starttime=###START_TIME###"
    )]
    pub fmi_url: String,

    // https://www.ilmatieteenlaitos.fi/havaintoasemat
    // Helsinki-Vantaa lentoasema: fmisid 100968
    // Pirkkala lentoasema: fmisid 101118
    // Vaasa lentoasema: fmisid 101462
    // Oulu lentoasema: fmisid 101786
    // Rovaniemi lentoasema: fmisid 101920
    // Kittilä lentoasema: fmisid 101986
    // Salla Naruska: fmisid 101966
    #[arg(long, default_value = "101118")]
    pub fmi_sid: String,

    #[arg(long, default_value_t = 10)]
    pub fmi_mins: i64,

    // MQTT support
    #[arg(long)]
    pub mqtt_enabled: bool,
    #[arg(long, default_value = "localhost")]
    pub mqtt_host: String,
    #[arg(long, default_value_t = 1883)]
    pub mqtt_port: u16,
    #[arg(long, default_value = "fmi_temp/101118")]
    pub mqtt_topic: String,
    #[arg(long)]
    pub mqtt_username: Option<String>,
    #[arg(long)]
    pub mqtt_password: Option<String>,

    // CoAP support
    #[arg(long)]
    pub coap_enabled: bool,
    #[arg(long, default_value = "coap://localhost/store")]
    pub coap_url: String,
    #[arg(long, default_value = "temperature")]
    pub coap_key: String,
}

impl OptsCommon {
    pub fn finish(&mut self) -> anyhow::Result<()> {
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

// EOF
