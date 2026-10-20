// lib.rs

pub use std::{borrow::Cow, env, fmt::Display, path::PathBuf, time};

pub use anyhow::{Context, anyhow, ensure};
pub use chrono::{DateTime, Duration, FixedOffset, Utc};
pub use clap::Parser;
pub use coap::client::{CoAPClient, UdpTransport};
pub use rumqttc::{Event as MqttEvent, MqttOptions, Outgoing, Packet, QoS, Transport};
pub use tracing::{Level, debug, error, info, trace, warn};
pub use tracing_subscriber::{EnvFilter, fmt as tracing_fmt};
pub use url::Url;
pub use uuid::Uuid;

#[cfg(test)]
pub use std::sync::{Arc, Mutex};

pub use crate::{config::*, web_util::*};

pub mod config;
pub mod web_util;

pub async fn get_temp(opts: &OptsCommon) -> anyhow::Result<String> {
    opts.validate()?;
    let start_time = Utc::now()
        .checked_sub_signed(Duration::try_minutes(opts.fmi_mins).context("FMI lookback is too large")?)
        .context("FMI lookback exceeds the supported date range")?
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let url = opts
        .fmi_url
        .replace("###FMI_SID###", &opts.fmi_sid)
        .replace("###START_TIME###", &start_time);

    info!("Getting url {url}");

    let body = get_xml_body(&url, opts.fmi_max_body_bytes).await?;
    trace!("FMI response: {body}");
    parse_temperature(
        &body,
        &opts.fmi_sid,
        Utc::now(),
        Duration::try_minutes(opts.fmi_max_age_mins).context("Maximum observation age is too large")?,
    )
}

const WATERML: &str = "http://www.opengis.net/waterml/2.0";
const OM: &str = "http://www.opengis.net/om/2.0";
const OMSO: &str = "http://inspire.ec.europa.eu/schemas/omso/3.0";
const GML: &str = "http://www.opengis.net/gml/3.2";
const XLINK: &str = "http://www.w3.org/1999/xlink";

fn parse_temperature(body: &str, station_id: &str, now: DateTime<Utc>, max_age: Duration) -> anyhow::Result<String> {
    let xml = roxmltree::Document::parse_with_options(
        body,
        roxmltree::ParsingOptions {
            nodes_limit: 100_000,
            ..Default::default()
        },
    )
    .context("Invalid FMI XML")?;
    let mut latest: Option<(DateTime<FixedOffset>, roxmltree::Node<'_, '_>)> = None;
    let mut latest_duplicates = Vec::new();
    for observation in xml
        .descendants()
        .filter(|node| node.has_tag_name((OMSO, "PointTimeSeriesObservation")))
    {
        let station_matches = observation.descendants().any(|node| {
            node.has_tag_name((GML, "identifier"))
                && node.attribute("codeSpace") == Some("http://xml.fmi.fi/namespace/stationcode/fmisid")
                && node.text().is_some_and(|text| text.trim() == station_id)
        });
        let temperature_matches = observation.children().any(|node| {
            node.has_tag_name((OM, "observedProperty"))
                && node
                    .attribute((XLINK, "href"))
                    .and_then(|href| Url::parse(href).ok())
                    .is_some_and(|url| url.query_pairs().any(|(key, value)| key == "param" && value == "t2m"))
        });
        if !station_matches || !temperature_matches {
            continue;
        }
        for point in observation
            .descendants()
            .filter(|node| node.has_tag_name((WATERML, "MeasurementTVP")))
        {
            let timestamp = point_text(point, "time")?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp)
                .with_context(|| format!("Invalid observation timestamp: {timestamp}"))?;
            if latest.as_ref().is_none_or(|(time, _)| timestamp > *time) {
                latest = Some((timestamp, point));
                latest_duplicates.clear();
            } else if latest.as_ref().is_some_and(|(time, _)| timestamp == *time) {
                latest_duplicates.push(point);
            }
        }
    }
    let (timestamp, point) =
        latest.with_context(|| format!("No temperature observations for FMI station {station_id}"))?;
    let age = now.signed_duration_since(timestamp);
    ensure!(
        age <= max_age,
        "Newest observation at {timestamp} exceeds maximum age of {} minutes",
        max_age.num_minutes()
    );
    ensure!(
        age >= -Duration::minutes(5),
        "Newest observation at {timestamp} is more than 5 minutes in the future"
    );
    let value = point_text(point, "value").context("Newest observation has no temperature")?;
    let temperature = parse_finite_temperature(&value).context("Newest observation has an invalid temperature")?;
    for duplicate in latest_duplicates {
        let value = point_text(duplicate, "value").context("Newest observation has no temperature")?;
        let duplicate_temperature =
            parse_finite_temperature(&value).context("Newest observation has an invalid temperature")?;
        ensure!(
            temperature == duplicate_temperature,
            "Conflicting temperatures at {timestamp}"
        );
    }
    info!("time = {timestamp}");
    info!("value = {value}");
    Ok(value)
}

fn point_text(point: roxmltree::Node<'_, '_>, name: &str) -> anyhow::Result<String> {
    let mut fields = point.children().filter(|node| node.has_tag_name((WATERML, name)));
    let field = fields.next().with_context(|| format!("Observation has no {name}"))?;
    ensure!(fields.next().is_none(), "Observation has duplicate {name} fields");
    ensure!(
        !field.children().any(|node| node.is_element()),
        "Invalid observation {name} structure"
    );
    let text: String = field
        .children()
        .filter(|node| node.is_text())
        .filter_map(|node| node.text())
        .collect();
    let text = text.trim();
    ensure!(!text.is_empty(), "Observation has empty {name}");
    Ok(text.to_owned())
}

fn parse_finite_temperature(value: &str) -> anyhow::Result<f64> {
    let temperature: f64 = value.trim().parse().context("Temperature must be a number")?;
    ensure!(temperature.is_finite(), "Temperature must be finite");
    Ok(temperature)
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

/// Publish using exactly `client_id`. CLI defaults and overrides are resolved by the caller.
pub async fn mqtt_send(enabled: bool, opts: &OptsCommon, client_id: &str, value: &str) -> anyhow::Result<()> {
    if !enabled {
        return Ok(());
    }

    opts.validate_mqtt()?;
    validate_mqtt_string("client ID", client_id)?;
    tokio::time::timeout(
        time::Duration::from_secs(opts.mqtt_timeout_secs),
        mqtt_publish(opts, client_id, value),
    )
    .await
    .with_context(|| format!("MQTT operation timed out after {} seconds", opts.mqtt_timeout_secs))?
}

async fn mqtt_publish(opts: &OptsCommon, client_id: &str, value: &str) -> anyhow::Result<()> {
    let temperature = parse_finite_temperature(value)?;
    let mut mqtt_options = MqttOptions::new(client_id, &opts.mqtt_host, opts.mqtt_port);
    mqtt_options
        .set_keep_alive(time::Duration::from_secs(25))
        .set_clean_session(true);

    if opts.mqtt_tls {
        let transport = if let Some(path) = &opts.mqtt_tls_ca {
            let ca = tokio::fs::read(path)
                .await
                .with_context(|| format!("Cannot read MQTT CA bundle {}", path.display()))?;
            Transport::tls(ca, None, None)
        } else {
            Transport::tls_with_default_config()
        };
        mqtt_options.set_transport(transport);
    }

    if let (Some(username), Some(password)) = (&opts.mqtt_username, &opts.mqtt_password) {
        mqtt_options.set_credentials(username, password);
    }

    let msg = serde_json::to_string(&serde_json::json!({ "temperature": temperature }))?;
    let topic = opts.mqtt_topic();
    info!("Publish MQTT: {topic} <-- {msg}");
    let (client, mut eventloop) = rumqttc::AsyncClient::new(mqtt_options, 42);
    client.publish(topic.as_ref(), QoS::AtLeastOnce, true, msg).await?;

    loop {
        let ev = eventloop.poll().await?;
        debug!("Received = {ev:#?}");
        if let MqttEvent::Incoming(Packet::PubAck(_)) = ev {
            debug!("Got ack, exit.");
            break;
        }
    }
    client.disconnect().await?;
    loop {
        if matches!(eventloop.poll().await?, MqttEvent::Outgoing(Outgoing::Disconnect)) {
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/temperature.xml");

    fn fixture_now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-20T08:15:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn parse_fixture(body: &str, station_id: &str) -> anyhow::Result<String> {
        parse_temperature(body, station_id, fixture_now(), Duration::hours(24))
    }

    #[test]
    fn enforces_observation_age_and_clock_skew_at_exact_boundaries() {
        let observed_at = fixture_now();
        for (now, accepted) in [
            (observed_at + Duration::hours(24), true),
            (observed_at + Duration::hours(24) + Duration::milliseconds(1), false),
            (observed_at - Duration::minutes(5), true),
            (observed_at - Duration::minutes(5) - Duration::milliseconds(1), false),
        ] {
            let result = parse_temperature(FIXTURE, "101118", now, Duration::hours(24));
            assert_eq!(result.is_ok(), accepted, "now={now}, result={result:?}");
        }
        assert!(
            parse_temperature(
                FIXTURE,
                "101118",
                observed_at + Duration::hours(36),
                Duration::hours(48)
            )
            .is_ok()
        );
    }

    #[test]
    fn rejects_future_newest_observation_without_using_older_valid_one() {
        let now = fixture_now() - Duration::minutes(6);
        let error = parse_temperature(FIXTURE, "101118", now, Duration::hours(24)).unwrap_err();
        assert!(error.to_string().contains("in the future"));
    }

    #[test]
    fn selects_latest_timestamp_with_offsets_independent_of_xml_order() {
        assert_eq!(parse_fixture(FIXTURE, "101118").unwrap(), "2.5");
        let xml = FIXTURE.replace("2026-10-20T11:15:00+03:00", "2026-10-20T05:15:00-03:00");
        assert_eq!(parse_fixture(&xml, "101118").unwrap(), "2.5");
    }

    #[test]
    fn validates_all_readings_at_the_newest_timestamp() {
        for (value, success) in [("2.50", true), ("3.5", false), ("NaN", false)] {
            let duplicate = format!(
                "<wml2:point><wml2:MeasurementTVP><wml2:time>2026-10-20T08:15:00Z</wml2:time><wml2:value>{value}</wml2:value></wml2:MeasurementTVP></wml2:point>"
            );
            let xml = FIXTURE.replace(
                "</wml2:MeasurementTimeseries>",
                &format!("{duplicate}</wml2:MeasurementTimeseries>"),
            );
            assert_eq!(parse_fixture(&xml, "101118").is_ok(), success, "{value}");
        }
    }

    #[test]
    fn rejects_invalid_newest_value_without_falling_back() {
        for value in ["NaN", "inf", "-inf", "1e999", "text", "0, \"extra\": true", ""] {
            let xml = FIXTURE.replace(">2.5<", &format!(">{value}<"));
            assert!(parse_fixture(&xml, "101118").is_err(), "{value}");
        }
        let xml = FIXTURE.replace("<wml2:value>2.5</wml2:value>", "");
        assert!(parse_fixture(&xml, "101118").is_err());
    }

    #[test]
    fn accepts_finite_negative_values_and_xml_comments() {
        let xml = FIXTURE.replace(">2.5<", "> -2<!-- degrees -->.5 <");
        assert_eq!(parse_fixture(&xml, "101118").unwrap(), "-2.5");
    }

    #[test]
    fn rejects_bad_timestamps_and_mismatching_metadata() {
        let xml = FIXTURE.replace("2026-10-20T11:15:00+03:00", "not-a-timestamp");
        assert!(parse_fixture(&xml, "101118").is_err());
        assert!(parse_fixture(FIXTURE, "100968").is_err());
        assert!(parse_fixture(&FIXTURE.replace("param=t2m", "param=ws_10min"), "101118").is_err());
    }

    #[test]
    fn ignores_other_stations_and_weather_parameters() {
        let member = FIXTURE
            .split_once("<wfs:member>")
            .unwrap()
            .1
            .split_once("</wfs:member>")
            .unwrap()
            .0;
        let wrong_station = member.replace(">101118<", ">100968<").replace(">2.5<", ">99<");
        let wrong_parameter = member.replace("param=t2m", "param=ws_10min").replace(">2.5<", ">88<");
        let xml = FIXTURE.replace(
            "<wfs:member>",
            &format!("<wfs:member>{wrong_station}</wfs:member><wfs:member>{wrong_parameter}</wfs:member><wfs:member>"),
        );
        assert_eq!(parse_fixture(&xml, "101118").unwrap(), "2.5");
    }

    #[test]
    fn xml_entity_declarations_remain_disabled() {
        let xml = FIXTURE
            .replace(
                "<wfs:FeatureCollection",
                "<!DOCTYPE x [<!ENTITY temperature '123'>]><wfs:FeatureCollection",
            )
            .replace(">2.5<", ">&temperature;<");
        assert!(parse_fixture(&xml, "101118").is_err());
    }
}
// EOF
