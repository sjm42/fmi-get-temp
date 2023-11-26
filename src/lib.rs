// lib.rs

pub mod config;
pub use config::*;

pub mod web_util;
pub use web_util::*;

pub use anyhow::anyhow;
pub use chrono::*;
pub use log::*;
pub use rumqttc::{Event, EventLoop, MqttOptions, Packet, QoS};
pub use structopt::StructOpt;

pub async fn get_temp(opts: &OptsCommon) -> anyhow::Result<String> {
    let starttime = Utc::now()
        .checked_sub_signed(Duration::minutes(15))
        .unwrap()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let url = opts
        .fmi_url
        .replace("###FMI_SID###", &opts.fmi_sid)
        .replace("###START_TIME###", &starttime);

    info!("Getting url {url}");

    let (body, ct) = get_text_body(url).await?.ok_or(anyhow!("No body!"))?;
    debug!("result:\nContent-type: {ct}\n{body:?}");

    let xml = roxmltree::Document::parse(&body)?;
    debug!("Parsed XML:\n{xml:?}");

    let ser = xml
        .descendants()
        .find(|n| n.has_tag_name("MeasurementTimeseries") && n.has_children())
        .ok_or(anyhow!("Cannot find time series"))?;
    let num_tvp = ser.children().filter(|n| n.is_element()).count();
    info!("#tvp: {num_tvp}");

    let last_tvp = ser
        .descendants()
        .filter(|n| n.has_tag_name("MeasurementTVP"))
        .last()
        .ok_or(anyhow!("no TVPs"))?;

    let time = last_tvp
        .children()
        .find(|n| n.is_element() && n.has_tag_name("time"))
        .ok_or(anyhow!("no time"))?
        .children()
        .last()
        .ok_or(anyhow!("no time"))?
        .text()
        .ok_or(anyhow!("no time"))?;
    info!("time = {time}");

    let value = last_tvp
        .children()
        .find(|n| n.is_element() && n.has_tag_name("value"))
        .ok_or(anyhow!("no value"))?
        .children()
        .last()
        .ok_or(anyhow!("no value"))?
        .text()
        .ok_or(anyhow!("no value"))?;
    info!("value = {value}");

    Ok(value.to_string())
}

// EOF
