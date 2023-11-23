// bin/sjmb.rs

use anyhow::anyhow;
use fmi_get_temp::*;
use log::*;
use structopt::StructOpt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut opts = OptsCommon::from_args();
    opts.finish()?;
    opts.start_pgm(env!("CARGO_BIN_NAME"));

    let url = "http://opendata.fmi.fi/wfs/fin?service=WFS&version=2.0.0&request=GetFeature&storedquery_id=fmi::observations::weather::timevaluepair&place=Pirkkala&parameters=t2m&starttime=2023-11-22T14:00:00Z";
    let body = get_text_body(url).await?.ok_or(anyhow!("No body!"))?;
    info!("Getting url {url}\nresult:\n{body:?}",);
    Ok(())
}

// EOF
