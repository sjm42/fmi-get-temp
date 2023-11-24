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

    let url = "http://opendata.fmi.fi/wfs/fin?service=WFS&version=2.0.0&request=GetFeature&storedquery_id=fmi::observations::weather::timevaluepair&place=Pirkkala&parameters=t2m&starttime=2023-11-23T19:00:00Z";
    info!("Getting url {url}");

    let (body, ct) = get_text_body(url).await?.ok_or(anyhow!("No body!"))?;
    debug!("result:\nContent-type: {ct}\n{body:?}");

    let xml = roxmltree::Document::parse(&body)?;
    debug!("Parsed XML:\n{xml:?}");

    let tvp = xml
        .descendants()
        .find(|n| n.has_tag_name("MeasurementTimeseries") && n.has_children())
        .ok_or(anyhow!("Cannot find time series"))?
        .descendants()
        .filter(|n| n.has_tag_name("MeasurementTVP"))
        .last()
        .ok_or(anyhow!("no measurements"))?;

    let time = tvp
        .children()
        .find(|n| n.is_element() && n.has_tag_name("time"))
        .ok_or(anyhow!("no time"))?
        .children()
        .last()
        .ok_or(anyhow!("no time"))?
        .text()
        .ok_or(anyhow!("no time"))?;
    info!("time = {time}");

    let value = tvp
        .children()
        .find(|n| n.is_element() && n.has_tag_name("value"))
        .ok_or(anyhow!("no value"))?
        .children()
        .last()
        .ok_or(anyhow!("no value"))?
        .text()
        .ok_or(anyhow!("no value"))?;
    info!("value = {value}");

    /*
    for v in tvp.descendants() {
        if v.is_text() {
            info!("tvp node:\n{v:?}");
        }
    }
    */

    Ok(())
}

// EOF
