use std::time::Duration;

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    time::timeout,
};

pub async fn read_packet(stream: &mut (impl AsyncRead + Unpin)) -> (u8, Vec<u8>) {
    timeout(Duration::from_secs(3), async {
        let header = stream.read_u8().await.unwrap();
        let mut len = 0usize;
        let mut multiplier = 1;
        for _ in 0..4 {
            let byte = stream.read_u8().await.unwrap();
            len += (byte & 0x7f) as usize * multiplier;
            if byte & 0x80 == 0 {
                assert!(len < 8192);
                let mut body = vec![0; len];
                stream.read_exact(&mut body).await.unwrap();
                return (header, body);
            }
            multiplier *= 128;
        }
        panic!("Invalid MQTT remaining length");
    })
    .await
    .expect("Mock MQTT read deadline exceeded")
}

fn read_string(body: &[u8], offset: &mut usize) -> String {
    let len = u16::from_be_bytes([body[*offset], body[*offset + 1]]) as usize;
    *offset += 2;
    let value = String::from_utf8(body[*offset..*offset + len].to_vec()).unwrap();
    *offset += len;
    value
}

pub async fn broker_roundtrip(
    mut stream: impl AsyncRead + AsyncWrite + Unpin,
    expected_topic: &str,
    expected_temperature: f64,
) -> (String, Vec<String>) {
    timeout(Duration::from_secs(3), async {
        let (header, connect) = read_packet(&mut stream).await;
        assert_eq!(header, 0x10);
        assert_ne!(connect[7] & 2, 0, "Short-lived clients must use a clean session");
        let mut offset = 10;
        let id = read_string(&connect, &mut offset);
        let mut credentials = Vec::new();
        if connect[7] & 0x80 != 0 {
            credentials.push(read_string(&connect, &mut offset));
        }
        if connect[7] & 0x40 != 0 {
            credentials.push(read_string(&connect, &mut offset));
        }
        stream.write_all(&[0x20, 2, 0, 0]).await.unwrap();
        let (header, publish) = read_packet(&mut stream).await;
        assert_eq!(header, 0x33, "Expected retained QoS 1 PUBLISH");
        let mut offset = 0;
        assert_eq!(read_string(&publish, &mut offset), expected_topic);
        let packet_id = &publish[offset..offset + 2];
        let payload: serde_json::Value = serde_json::from_slice(&publish[offset + 2..]).unwrap();
        assert_eq!(payload, serde_json::json!({"temperature": expected_temperature}));
        stream.write_all(&[0x40, 2, packet_id[0], packet_id[1]]).await.unwrap();
        assert_eq!(read_packet(&mut stream).await, (0xe0, Vec::new()));
        (id, credentials)
    })
    .await
    .expect("Mock MQTT exchange deadline exceeded")
}
