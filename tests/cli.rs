use std::{process::Output, time::Duration};

use fmi_get_temp::{Duration as ChronoDuration, Utc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    process::Command,
    time::{error::Elapsed, timeout},
};

mod common;
use common::*;
#[path = "support/mqtt.rs"]
mod mqtt;
use mqtt::*;

async fn run_cli(args: Vec<String>, rust_log: Option<&str>) -> Output {
    run_cli_with_deadline(args, rust_log, Duration::from_secs(10))
        .await
        .expect("CLI test deadline exceeded")
}

async fn run_cli_with_deadline(
    args: Vec<String>,
    rust_log: Option<&str>,
    deadline: Duration,
) -> Result<Output, Elapsed> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fmi_get_temp"));
    command.args(args).env_remove("RUST_LOG").kill_on_drop(true);
    if let Some(filter) = rust_log {
        command.env("RUST_LOG", filter);
    }
    timeout(deadline, command.output()).await.map(|result| result.unwrap())
}

#[tokio::test]
async fn mqtt_failure_returns_nonzero_and_keeps_diagnostics_off_stdout() {
    let xml = xml_at(Utc::now() - ChronoDuration::minutes(15));
    let server = http_server("Content-Type: application/xml\r\n", xml.as_bytes()).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let broker = tokio::spawn(async move {
        let (stream, _) = timeout(Duration::from_secs(3), listener.accept())
            .await
            .unwrap()
            .unwrap();
        drop(stream);
    });
    let output = run_cli(
        vec![
            "--fmi-url".into(),
            server.url.clone(),
            "--mqtt-enabled".into(),
            "--mqtt-host".into(),
            "127.0.0.1".into(),
            "--mqtt-port".into(),
            port.to_string(),
        ],
        None,
    )
    .await;
    assert!(!output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "2.5\n");
    assert!(String::from_utf8(output.stderr).unwrap().contains("MQTT send error"));
    timeout(Duration::from_secs(3), broker).await.unwrap().unwrap();
}

#[tokio::test]
async fn cli_verbosity_and_rust_log_write_only_to_stderr() {
    for filter in [None, Some("warn,fmi_get_temp=info")] {
        let xml = xml_at(Utc::now() - ChronoDuration::minutes(15));
        let server = http_server("Content-Type: text/xml\r\n", xml.as_bytes()).await;
        let output = run_cli(vec!["--fmi-url".into(), server.url.clone(), "--verbose".into()], filter).await;
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(output.stdout, b"2.5\n");
        assert!(String::from_utf8(output.stderr).unwrap().contains("Starting up"));
    }
}

#[tokio::test]
async fn cli_resolves_topics_and_unique_or_explicit_client_ids() {
    let mut generated_ids = Vec::new();
    for (sid, topic, client_id) in [
        ("101118", None, None),
        ("101118", None, None),
        ("100968", None, None),
        ("100968", Some("fmi_temp/101118"), Some("configured-id")),
    ] {
        let xml = xml_at(Utc::now() - ChronoDuration::minutes(15)).replace(">101118<", &format!(">{sid}<"));
        let server = http_server("Content-Type: application/xml\r\n", xml.as_bytes()).await;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let expected_topic = topic.map(str::to_owned).unwrap_or_else(|| format!("fmi_temp/{sid}"));
        let broker = tokio::spawn(async move {
            let (stream, _) = timeout(Duration::from_secs(3), listener.accept())
                .await
                .unwrap()
                .unwrap();
            broker_roundtrip(stream, &expected_topic, 2.5).await
        });
        let mut args = vec![
            "--fmi-url".into(),
            server.url.clone(),
            "--fmi-sid".into(),
            sid.into(),
            "--mqtt-enabled".into(),
            "--mqtt-host".into(),
            "127.0.0.1".into(),
            "--mqtt-port".into(),
            port.to_string(),
        ];
        if let Some(topic) = topic {
            args.extend(["--mqtt-topic".into(), topic.into()]);
        }
        if let Some(id) = client_id {
            args.extend(["--mqtt-client-id".into(), id.into()]);
        }
        let output = run_cli(args, None).await;
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(output.stdout, b"2.5\n");
        let (actual_id, _) = timeout(Duration::from_secs(3), broker).await.unwrap().unwrap();
        if let Some(id) = client_id {
            assert_eq!(actual_id, id);
        } else {
            assert!(actual_id.len() <= 23);
            assert!(actual_id.chars().all(|c| c.is_ascii_alphanumeric()));
            assert!(!generated_ids.contains(&actual_id));
            generated_ids.push(actual_id);
        }
    }
}

#[tokio::test]
async fn cli_deadline_terminates_child_process_and_closes_its_connection() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut stream, _) = timeout(Duration::from_secs(3), listener.accept())
            .await
            .unwrap()
            .unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            request.push(stream.read_u8().await.unwrap());
        }
        // Keep the HTTP response open until the CLI helper's shorter deadline kills its child.
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/xml\r\nContent-Length: 1000\r\n\r\n")
            .await
            .unwrap();
        let mut byte = [0];
        assert_eq!(
            timeout(Duration::from_secs(3), stream.read(&mut byte))
                .await
                .unwrap()
                .unwrap(),
            0
        );
    });
    let result = run_cli_with_deadline(vec!["--fmi-url".into(), url], None, Duration::from_secs(1)).await;
    assert!(result.is_err());
    timeout(Duration::from_secs(3), server).await.unwrap().unwrap();
}

#[tokio::test]
async fn invalid_rust_log_is_reported_before_fetching() {
    let output = run_cli(Vec::new(), Some("fmi_get_temp=not-a-level")).await;
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr).unwrap().contains("Invalid RUST_LOG"));
}
