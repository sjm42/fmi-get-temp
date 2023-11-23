// bin/sjmb.rs

use chrono::*;
use log::*;
use structopt::StructOpt;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut opts = OptsCommon::from_args();
    opts.finish()?;
    opts.start_pgm(env!("CARGO_BIN_NAME"));
    Ok(())
}

// EOF
