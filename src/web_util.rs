// web_util.rs

use crate::*;

const CONN_TIMEOUT: u64 = 5;
const REQW_TIMEOUT: u64 = 10;
pub const DEFAULT_MAX_BODY_BYTES: u64 = 2 * 1024 * 1024;

fn is_xml_content_type(content_type: &str) -> bool {
    let media_type = content_type.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    if let Some((kind, subtype)) = media_type.split_once('/') {
        return matches!(kind, "text" | "application") && (subtype == "xml" || subtype.ends_with("+xml"));
    }
    false
}

pub async fn get_xml_body(url: &str, max_body_bytes: u64) -> anyhow::Result<String> {
    let (body, content_type) = get_body_with_limit(url, max_body_bytes).await?;
    ensure!(
        is_xml_content_type(&content_type),
        "Expected XML, received Content-Type: {content_type}"
    );
    Ok(body)
}

pub async fn get_text_body(url_s: &str) -> anyhow::Result<Option<(String, String)>> {
    let (body, ct) = get_body(url_s).await?;

    if ct.starts_with("text/") || is_xml_content_type(&ct) {
        Ok(Some((body, ct)))
    } else {
        debug!("Content-type ignored: {ct:?}");
        Ok(None)
    }
}

pub async fn get_body(url_s: &str) -> anyhow::Result<(String, String)> {
    get_body_with_limit(url_s, DEFAULT_MAX_BODY_BYTES).await
}

pub async fn get_body_with_limit(url_s: &str, max_body_bytes: u64) -> anyhow::Result<(String, String)> {
    ensure!(max_body_bytes > 0, "Response size limit must be positive");
    // We want a normalized and valid url, IDN handled etc.
    let url = Url::parse(url_s)?;

    let c = reqwest::ClientBuilder::new()
        .connect_timeout(time::Duration::from_secs(CONN_TIMEOUT))
        .timeout(time::Duration::from_secs(REQW_TIMEOUT))
        .user_agent(format!(
            "Rust/hyper/{} v{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        ))
        .use_rustls_tls()
        .https_only(url.scheme() == "https")
        .build()?;

    let mut resp = c.get(url).send().await?.error_for_status()?;
    let ct = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .context("No Content-Type in response")?
        .to_str()
        .context("Invalid Content-Type in response")?
        .to_owned();

    let mut body = Vec::new();
    while let Some(chunk) = resp.chunk().await? {
        ensure!(
            chunk.len() as u64 <= max_body_bytes.saturating_sub(body.len() as u64),
            "Response exceeds maximum decompressed size of {max_body_bytes} bytes"
        );
        body.extend_from_slice(&chunk);
    }
    let body = String::from_utf8(body).context("Response is not valid UTF-8")?;
    Ok((body, ct))
}

// EOF
