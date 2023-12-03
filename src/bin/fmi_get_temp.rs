// bin/sjmb.rs

use fmi_get_temp::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut opts = OptsCommon::from_args();
    opts.finish()?;
    opts.start_pgm(env!("CARGO_BIN_NAME"));

    let t = get_temp(&opts).await?;

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
        let id = env!("CARGO_BIN_NAME");
        mqtt_send(&opts, id, &t).await?;
    }
    Ok(())
}
// EOF
