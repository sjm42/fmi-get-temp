// bin/fmi_get_temp.rs

use fmi_get_temp::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let me = env!("CARGO_BIN_NAME");
    let opts = OptsCommon::parse();
    opts.validate()?;
    opts.start_pgm(me)?;

    let t = get_temp(&opts).await?;
    println!("{t}");
    let client_id = opts
        .mqtt_client_id
        .clone()
        .unwrap_or_else(|| format!("fmi{}", &Uuid::new_v4().simple().to_string()[..20]));

    // run these concurrently, wait for both
    let (r1, r2) = tokio::join!(
        coap_send(opts.coap_enabled, &opts.coap_url, &opts.coap_key, &t),
        mqtt_send(opts.mqtt_enabled, &opts, &client_id, &t)
    );

    if let Err(e) = r1 {
        error!("CoAP send error: {e}");
    }
    if let Err(e) = r2 {
        return Err(anyhow!("MQTT send error: {e:#}"));
    }

    Ok(())
}
// EOF
