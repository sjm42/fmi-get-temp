// lib.rs

pub use std::{env, fmt::Display, time};

pub use anyhow::anyhow;
pub use chrono::*;
pub use clap::Parser;
pub use tracing::*;

use coap::client::{CoAPClient, UdpTransport};
use rumqttc::{MqttOptions, Packet, QoS};

pub use config::*;
pub use web_util::*;

pub mod config;

pub mod web_util;

pub async fn get_temp(opts: &OptsCommon) -> anyhow::Result<String> {
    let start_time = Utc::now()
        .checked_sub_signed(Duration::try_minutes(opts.fmi_mins).unwrap_or_default())
        .unwrap_or_default()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let url = opts
        .fmi_url
        .replace("###FMI_SID###", &opts.fmi_sid)
        .replace("###START_TIME###", &start_time);

    info!("Getting url {url}");

    let (body, ct) = get_text_body(&url).await?.ok_or(anyhow!("No body!"))?;
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
        .rfind(|n| n.has_tag_name("MeasurementTVP"))
        .ok_or(anyhow!("no TVPs"))?;

    let time = last_tvp
        .children()
        .find(|n| n.is_element() && n.has_tag_name("time"))
        .ok_or(anyhow!("no time"))?
        .children()
        .next_back()
        .ok_or(anyhow!("no time"))?
        .text()
        .ok_or(anyhow!("no time"))?;
    info!("time = {time}");

    let value = last_tvp
        .children()
        .find(|n| n.is_element() && n.has_tag_name("value"))
        .ok_or(anyhow!("no value"))?
        .children()
        .next_back()
        .ok_or(anyhow!("no value"))?
        .text()
        .ok_or(anyhow!("no value"))?;
    info!("value = {value}");

    Ok(value.to_string())
}

pub async fn coap_send(enabled: bool, url: &str, key: &str, value: &str) -> anyhow::Result<()> {
    if !enabled {
        return Ok(());
    }

    let payload = format!("{key} {value}");
    info!("*** CoAP POST {url} <-- {payload}");

    let res =
        CoAPClient::<UdpTransport>::post_with_timeout(url, payload.into_bytes(), time::Duration::new(5, 0)).await?;
    info!("<-- {res:?}");
    Ok(())
}

pub async fn mqtt_send(enabled: bool, opts: &OptsCommon, client_id: &str, value: &str) -> anyhow::Result<()> {
    if !enabled {
        return Ok(());
    }

    let mut mqtt_options = MqttOptions::new(client_id, &opts.mqtt_host, opts.mqtt_port);
    mqtt_options
        .set_keep_alive(time::Duration::from_secs(25))
        .set_clean_session(false);

    if let (Some(username), Some(password)) = (&opts.mqtt_username, &opts.mqtt_password) {
        mqtt_options.set_credentials(username, password);
    }

    let msg = format!("{{ \"temperature\": {value} }}");
    info!("Publish MQTT: {} <-- {}", opts.mqtt_topic, msg);
    let (client, mut eventloop) = rumqttc::AsyncClient::new(mqtt_options, 42);
    client.publish(&opts.mqtt_topic, QoS::AtLeastOnce, true, msg).await?;

    loop {
        let ev = eventloop.poll().await?;
        debug!("Received = {ev:#?}");
        if let rumqttc::Event::Incoming(Packet::PubAck(_)) = ev {
            debug!("Got ack, exit.");
            break;
        }
    }
    client.disconnect().await?;

    Ok(())
}
// EOF
