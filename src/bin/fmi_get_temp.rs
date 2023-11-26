// bin/sjmb.rs

use fmi_get_temp::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut opts = OptsCommon::from_args();
    opts.finish()?;
    opts.start_pgm(env!("CARGO_BIN_NAME"));

    let t = get_temp(&opts).await?;

    let mut mqttoptions = MqttOptions::new(env!("CARGO_BIN_NAME"), &opts.mqtt_host, opts.mqtt_port);
    mqttoptions
        .set_keep_alive(std::time::Duration::from_secs(25))
        .set_clean_session(false);

    println!("{t}");

    if opts.coap_enabled {
        let url = opts.coap_url.clone();
        let key = opts.coap_key.clone();
        let val = t.clone();

        // Spawn a separate task for handling the CoAP stuff to avoid blocking main loop
        tokio::task::spawn_blocking(move || {
            if let Err(e) = coap_send(url, key, val) {
                error!("Message handling error: {e}");
            }
        });
    }

    if opts.mqtt_enabled {
        let msg = format!("{{ \"temperature\": {t} }}");
        info!("Publish MQTT: {} <-- {}", opts.mqtt_topic, msg);
        let (client, mut eventloop) = rumqttc::AsyncClient::new(mqttoptions, 42);
        client
            .publish(opts.mqtt_topic, QoS::AtLeastOnce, false, msg)
            .await?;

        loop {
            let ev = eventloop.poll().await.unwrap();
            debug!("Received = {ev:#?}");
            if let Event::Incoming(p) = ev {
                if let Packet::PubAck(_) = p {
                    debug!("Got ack, exit.");
                    break;
                }
            }
        }
        client.disconnect().await?;
    }
    Ok(())
}
// EOF
