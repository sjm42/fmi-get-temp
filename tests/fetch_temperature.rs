use std::io::Write;

use flate2::{Compression, write::GzEncoder};
use fmi_get_temp::*;

mod common;
use common::*;

#[tokio::test]
async fn fetches_xml_media_types_with_parameters() {
    let xml = xml_at(Utc::now() - Duration::minutes(15));
    for content_type in [
        "text/xml",
        "application/xml; charset=UTF-8",
        "Application/GML+XML; charset=utf-8",
    ] {
        let server = http_server(&format!("Content-Type: {content_type}\r\n"), xml.as_bytes()).await;
        let mut opts = OptsCommon::parse_from(["fmi_get_temp"]);
        opts.fmi_url = server.url.clone();
        assert_eq!(get_temp(&opts).await.unwrap(), "2.5");
    }
}

#[tokio::test]
async fn rejects_stale_and_future_observations_and_honors_configured_maximum_age() {
    for (age_hours, max_age_mins, expected) in [
        (36, 1440, Some("maximum age")),
        (36, 2880, None),
        (-1, 1440, Some("in the future")),
    ] {
        let xml = xml_at(Utc::now() - Duration::hours(age_hours));
        let server = http_server("Content-Type: text/xml\r\n", xml.as_bytes()).await;
        let mut opts = OptsCommon::parse_from(["fmi_get_temp"]);
        opts.fmi_url = server.url.clone();
        opts.fmi_max_age_mins = max_age_mins;
        let result = get_temp(&opts).await;
        if let Some(expected) = expected {
            assert!(result.unwrap_err().to_string().contains(expected));
        } else {
            assert_eq!(result.unwrap(), "2.5");
        }
    }
}

#[tokio::test]
async fn reports_missing_and_unsupported_content_types() {
    for (headers, expected) in [("", "No Content-Type"), ("Content-Type: text/html\r\n", "Expected XML")] {
        let server = http_server(headers, XML.as_bytes()).await;
        let error = get_xml_body(&server.url, DEFAULT_MAX_BODY_BYTES).await.unwrap_err();
        assert!(error.to_string().contains(expected), "{error:#}");
    }
}

#[tokio::test]
async fn enforces_decompressed_size_limit_and_accepts_exact_limit() {
    let mut compressor = GzEncoder::new(Vec::new(), Compression::best());
    compressor.write_all(XML.as_bytes()).unwrap();
    let compressed = compressor.finish().unwrap();
    assert!(compressed.len() < XML.len() / 2);
    let server = http_server(
        "Content-Type: application/xml\r\nContent-Encoding: gzip\r\n",
        &compressed,
    )
    .await;
    let error = get_xml_body(&server.url, (XML.len() / 2) as u64).await.unwrap_err();
    assert!(error.to_string().contains("maximum decompressed size"), "{error:#}");
    let server = http_server("Content-Type: text/xml\r\n", XML.as_bytes()).await;
    assert_eq!(get_xml_body(&server.url, XML.len() as u64).await.unwrap(), XML);
}

#[tokio::test]
async fn enforces_size_limit_for_chunked_bodies() {
    let chunked = format!("{:x}\r\n{XML}\r\n0\r\n\r\n", XML.len());
    let server = http_server(
        "Content-Type: text/xml\r\nTransfer-Encoding: chunked\r\n",
        chunked.as_bytes(),
    )
    .await;
    let error = get_xml_body(&server.url, 32).await.unwrap_err();
    assert!(error.to_string().contains("maximum decompressed size"), "{error:#}");
}

#[tokio::test]
async fn invalid_options_fail_before_http_request() {
    let mut opts = OptsCommon::parse_from(["fmi_get_temp"]);
    opts.fmi_url = "http://127.0.0.1:1/".into();
    opts.fmi_mins = i64::MAX;
    assert!(get_temp(&opts).await.unwrap_err().to_string().contains("lookback"));
}
