use chrono::{DateTime, Duration, SecondsFormat, Utc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};

pub const XML: &str = include_str!("../fixtures/temperature.xml");

pub fn xml_at(latest: DateTime<Utc>) -> String {
    [
        ("2026-10-20T08:00:00Z", latest - Duration::minutes(15)),
        ("2026-10-20T11:15:00+03:00", latest),
        ("2026-10-20T07:45:00Z", latest - Duration::minutes(30)),
    ]
    .into_iter()
    .fold(XML.to_owned(), |body, (original, time)| {
        body.replace(original, &time.to_rfc3339_opts(SecondsFormat::Secs, true))
    })
}

pub struct HttpServer {
    pub url: String,
    task: JoinHandle<()>,
}

impl Drop for HttpServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub async fn http_server(headers: &str, body: &[u8]) -> HttpServer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let mut response = format!("HTTP/1.1 200 OK\r\nConnection: close\r\n{headers}\r\n").into_bytes();
    response.extend_from_slice(body);
    let task = tokio::spawn(async move {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                request.push(stream.read_u8().await.unwrap());
                assert!(request.len() < 16_384);
            }
            let _ = stream.write_all(&response).await;
            let _ = stream.shutdown().await;
        })
        .await
        .expect("Mock HTTP server deadline exceeded");
    });
    HttpServer { url, task }
}
