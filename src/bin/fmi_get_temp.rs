// bin/fmi_get_temp.rs

use clap::Parser;

use fmi_get_temp::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let me = env!("CARGO_BIN_NAME");
    let mut opts = OptsCommon::parse();
    opts.finalize()?;
    opts.start_pgm();

    let t = get_temp(&opts).await?;
    println!("{t}");

    // run these concurrently, wait for both
    let (r1, r2) = tokio::join!(
        coap_send(opts.coap_enabled, &opts.coap_url, &opts.coap_key, &t),
        mqtt_send(opts.mqtt_enabled, &opts, me, &t)
    );

    if let Err(e) = r1 {
        error!("CoAP send error: {e}");
    }
    if let Err(e) = r2 {
        error!("MQTT send error: {e}");
    }

    Ok(())
}
// EOF
