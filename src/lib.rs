// lib.rs

pub mod config;
pub use config::*;

pub mod web_util;
pub use web_util::*;

pub use anyhow::anyhow;
pub use chrono::*;
use coap::client::{CoAPClient, UdpTransport};
pub use log::*;
pub use rumqttc::{Event, EventLoop, MqttOptions, Packet, QoS};
use std::fmt::Display;

pub async fn get_temp(opts: &OptsCommon) -> anyhow::Result<String> {
    let starttime = Utc::now()
        .checked_sub_signed(Duration::minutes(opts.fmi_mins))
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

pub async fn coap_send<S1, S2, S3>(enabled: bool, url: S1, key: S2, value: S3) -> anyhow::Result<()>
where
    S1: AsRef<str> + Display,
    S2: AsRef<str> + Display,
    S3: AsRef<str> + Display,
{
    if !enabled {
        return Ok(());
    }

    let payload = format!("{key} {value}");
    info!("*** CoAP POST {url} <-- {payload}");

    let res = CoAPClient::<UdpTransport>::post_with_timeout(
        url.as_ref(),
        payload.into_bytes(),
        std::time::Duration::new(5, 0),
    )
    .await?;
    info!("<-- {res:?}");
    Ok(())
}

pub async fn mqtt_send<S1, S2>(
    enabled: bool,
    opts: &OptsCommon,
    client_id: S1,
    value: S2,
) -> anyhow::Result<()>
where
    S1: AsRef<str> + Display,
    S2: AsRef<str> + Display,
{
    if !enabled {
        return Ok(());
    }

    let mut mqttoptions = MqttOptions::new(client_id.as_ref(), &opts.mqtt_host, opts.mqtt_port);
    mqttoptions
        .set_keep_alive(std::time::Duration::from_secs(25))
        .set_clean_session(false);

    if let (Some(username), Some(password)) = (&opts.mqtt_username, &opts.mqtt_password) {
        mqttoptions.set_credentials(username, password);
    }

    let msg = format!("{{ \"temperature\": {} }}", value.as_ref());
    info!("Publish MQTT: {} <-- {}", opts.mqtt_topic, msg);
    let (client, mut eventloop) = rumqttc::AsyncClient::new(mqttoptions, 42);
    client
        .publish(&opts.mqtt_topic, QoS::AtLeastOnce, false, msg)
        .await?;

    loop {
        let ev = eventloop.poll().await.unwrap();
        debug!("Received = {ev:#?}");
        if let Event::Incoming(Packet::PubAck(_)) = ev {
            debug!("Got ack, exit.");
            break;
        }
    }
    client.disconnect().await?;

    Ok(())
}
// EOF
