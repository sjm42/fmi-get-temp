// bin/sjmb.rs

use clap::Parser;
use fmi_get_temp::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut opts = OptsCommon::parse();
    opts.finish()?;
    opts.start_pgm(env!("CARGO_BIN_NAME"));

    let t = get_temp(&opts).await?;

    println!("{t}");

    if opts.coap_enabled {
        coap_send(&opts.coap_url, &opts.coap_key, &t).await?;
    }

    if opts.mqtt_enabled {
        let id = env!("CARGO_BIN_NAME");
        mqtt_send(&opts, id, &t).await?;
    }
    Ok(())
}
// EOF
