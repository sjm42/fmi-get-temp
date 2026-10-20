use std::{fs, path::PathBuf, sync::Arc, time::Duration};

use fmi_get_temp::*;
use rcgen::{CertifiedKey, generate_simple_self_signed};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    time::timeout,
};
use tokio_rustls::{
    TlsAcceptor,
    rustls::{ServerConfig, pki_types::PrivatePkcs8KeyDer},
};

#[path = "support/mqtt.rs"]
mod mqtt;
use mqtt::*;

#[tokio::test]
async fn preserves_exact_library_id_credentials_and_sends_a_real_disconnect() {
    let mut listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut opts = OptsCommon::parse_from(["fmi_get_temp"]);
    opts.mqtt_host = "127.0.0.1".into();
    opts.mqtt_port = listener.local_addr().unwrap().port();
    opts.mqtt_timeout_secs = 3;
    opts.mqtt_username = Some("alice".into());
    opts.mqtt_password = Some("password".into());
    opts.mqtt_client_id = Some("cli-option-does-not-override-library-argument".into());
    for _ in 0..2 {
        let broker = tokio::spawn(async move {
            let (stream, _) = timeout(Duration::from_secs(3), listener.accept())
                .await
                .unwrap()
                .unwrap();
            let result = broker_roundtrip(stream, "fmi_temp/101118", -2.5).await;
            (listener, result)
        });
        mqtt_send(true, &opts, "caller-supplied-id", "-2.5").await.unwrap();
        let (returned_listener, (id, credentials)) = timeout(Duration::from_secs(3), broker).await.unwrap().unwrap();
        listener = returned_listener;
        assert_eq!(id, "caller-supplied-id");
        assert_eq!(credentials, ["alice", "password"]);
    }
}

#[tokio::test]
async fn rejects_invalid_library_client_ids_before_connecting() {
    let opts = OptsCommon::parse_from(["fmi_get_temp"]);
    for id in ["", "bad\0id", &"a".repeat(65_536)] {
        let error = mqtt_send(true, &opts, id, "-2.5").await.unwrap_err();
        assert!(error.to_string().contains("client ID"));
    }
}

#[tokio::test]
async fn deadline_covers_missing_publish_acknowledgement() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut opts = OptsCommon::parse_from(["fmi_get_temp"]);
    opts.mqtt_host = "127.0.0.1".into();
    opts.mqtt_port = listener.local_addr().unwrap().port();
    opts.mqtt_timeout_secs = 1;
    let broker = tokio::spawn(async move {
        let (mut stream, _) = timeout(Duration::from_secs(3), listener.accept())
            .await
            .unwrap()
            .unwrap();
        read_packet(&mut stream).await;
        stream.write_all(&[0x20, 2, 0, 0]).await.unwrap();
        let (header, _) = read_packet(&mut stream).await;
        assert_eq!(header, 0x33);
        let mut byte = [0];
        assert_eq!(stream.read(&mut byte).await.unwrap(), 0);
    });
    let error = mqtt_send(true, &opts, "fmi_get_temp", "-2.5").await.unwrap_err();
    assert!(error.to_string().contains("timed out after 1 seconds"), "{error:#}");
    timeout(Duration::from_secs(3), broker).await.unwrap().unwrap();
}

#[tokio::test]
async fn rejects_invalid_numeric_payload_before_connecting() {
    let opts = OptsCommon::parse_from(["fmi_get_temp"]);
    for value in ["NaN", "inf", "1e999", "0, \"injected\":true"] {
        let error = mqtt_send(true, &opts, "fmi_get_temp", value).await.unwrap_err();
        assert!(error.to_string().contains("Temperature"), "{error:#}");
    }
}

struct CaFile(PathBuf);

impl Drop for CaFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn tls_config() -> (TlsAcceptor, CaFile) {
    let CertifiedKey { cert, signing_key } = generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let path = env::temp_dir().join(format!("fmi-test-ca-{}.pem", Uuid::new_v4()));
    fs::write(&path, cert.pem()).unwrap();
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![cert.der().clone()],
            PrivatePkcs8KeyDer::from(signing_key.serialize_der()).into(),
        )
        .unwrap();
    (TlsAcceptor::from(Arc::new(config)), CaFile(path))
}

#[tokio::test]
async fn mqtt_tls_accepts_configured_ca_and_rejects_untrusted_certificates() {
    let (acceptor, ca) = tls_config();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut opts = OptsCommon::parse_from(["fmi_get_temp"]);
    opts.mqtt_host = "localhost".into();
    opts.mqtt_port = listener.local_addr().unwrap().port();
    opts.mqtt_tls = true;
    opts.mqtt_timeout_secs = 3;
    let server_acceptor = acceptor.clone();
    let broker = tokio::spawn(async move {
        let (stream, _) = timeout(Duration::from_secs(3), listener.accept())
            .await
            .unwrap()
            .unwrap();
        assert!(server_acceptor.accept(stream).await.is_err());
        listener
    });
    let error = mqtt_send(true, &opts, "fmi_get_temp", "-2.5").await.unwrap_err();
    assert!(!error.to_string().contains("timed out"), "{error:#}");
    let listener = timeout(Duration::from_secs(3), broker).await.unwrap().unwrap();

    opts.mqtt_tls_ca = Some(ca.0.clone());
    let broker = tokio::spawn(async move {
        let (stream, _) = timeout(Duration::from_secs(3), listener.accept())
            .await
            .unwrap()
            .unwrap();
        let stream = acceptor.accept(stream).await.unwrap();
        broker_roundtrip(stream, "fmi_temp/101118", -2.5).await
    });
    mqtt_send(true, &opts, "fmi_get_temp", "-2.5").await.unwrap();
    timeout(Duration::from_secs(3), broker).await.unwrap().unwrap();
}

#[tokio::test]
async fn https_rejects_self_signed_server_certificate() {
    let (acceptor, _ca) = tls_config();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("https://localhost:{}/", listener.local_addr().unwrap().port());
    let server = tokio::spawn(async move {
        let (stream, _) = timeout(Duration::from_secs(3), listener.accept())
            .await
            .unwrap()
            .unwrap();
        assert!(acceptor.accept(stream).await.is_err());
    });
    let error = get_xml_body(&url, 1024).await.unwrap_err();
    assert!(
        format!("{error:#}").to_ascii_lowercase().contains("certificate"),
        "{error:#}"
    );
    timeout(Duration::from_secs(3), server).await.unwrap().unwrap();
}
