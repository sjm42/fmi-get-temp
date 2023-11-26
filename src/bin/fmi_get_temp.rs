// bin/sjmb.rs

use fmi_get_temp::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut opts = OptsCommon::from_args();
    opts.finish()?;
    opts.start_pgm(env!("CARGO_BIN_NAME"));

    let t = get_temp(&opts).await?;

    let mut mqttoptions = MqttOptions::new("mqtt2coap", &opts.mqtt_host, opts.mqtt_port);
    mqttoptions.set_keep_alive(std::time::Duration::from_secs(25));

    println!("{t}");

    if opts.mqtt_enabled {
        let (client, _eventloop) = rumqttc::AsyncClient::new(mqttoptions, 42);
        client
            .publish(opts.mqtt_topic, QoS::AtLeastOnce, false, t)
            .await?;
    }
    Ok(())
}
// EOF
