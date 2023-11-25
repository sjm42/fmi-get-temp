// bin/sjmb.rs

use fmi_get_temp::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut opts = OptsCommon::from_args();
    opts.finish()?;
    let fmi = FmiConfig::new(&opts)?;
    opts.start_pgm(env!("CARGO_BIN_NAME"));

    println!("{}", get_temp(&fmi).await?);
    Ok(())
}
// EOF
