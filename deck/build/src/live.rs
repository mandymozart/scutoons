//! "No compile while a show is live" (RIG-EDITING.md §4): the relay beside us in the pod answers `GET /status` with
//! `{ "sessions": n, "live": n }` (B.4's deploy-freeze check); `live > 0` = a session with a conductor. No answer, no
//! JSON, a closed port = not live (the relay may be down; the freeze check of the deploy workflows reads it the same way).
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveCheck {
    addr: String,
    path: String,
}

impl LiveCheck {
    /// `http://127.0.0.1:7700/status` → host:port and path (plain http only – the relay is on our own localhost)
    pub fn parse(url: &str) -> Option<LiveCheck> {
        let rest = url.strip_prefix("http://")?;
        let (authority, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        if authority.is_empty() {
            return None;
        }
        let addr = if authority.contains(':') { authority.to_string() } else { format!("{authority}:80") };
        Some(LiveCheck { addr, path: path.to_string() })
    }

    /// true while the relay reports `live > 0`
    pub async fn is_live(&self) -> bool {
        tokio::time::timeout(Duration::from_secs(2), self.ask()).await.ok().flatten().unwrap_or(false)
    }

    async fn ask(&self) -> Option<bool> {
        let mut stream = TcpStream::connect(&self.addr).await.ok()?;
        let host = self.addr.split(':').next().unwrap_or("localhost");
        stream.write_all(format!("GET {} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nAccept: application/json\r\n\r\n", self.path).as_bytes()).await.ok()?;
        let mut raw = Vec::new();
        stream.take(16 * 1024).read_to_end(&mut raw).await.ok()?;
        let text = String::from_utf8_lossy(&raw);
        let body = text.split_once("\r\n\r\n")?.1;
        let status: serde_json::Value = serde_json::from_str(body.trim()).ok()?;
        Some(status.get("live").and_then(|v| v.as_u64()).unwrap_or(0) > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    /// a relay that answers every request with `body`
    async fn fake_relay(body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            while let Ok((mut s, _)) = listener.accept().await {
                let mut buf = [0u8; 1024];
                let _ = s.read(&mut buf).await;
                let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await;
            }
        });
        format!("http://{addr}/status")
    }

    #[test]
    fn the_url_is_split() {
        assert_eq!(LiveCheck::parse("http://127.0.0.1:7700/status"), Some(LiveCheck { addr: "127.0.0.1:7700".into(), path: "/status".into() }));
        assert_eq!(LiveCheck::parse("http://relay"), Some(LiveCheck { addr: "relay:80".into(), path: "/".into() }));
        assert_eq!(LiveCheck::parse("https://live.scutoons.com/status"), None);
        assert_eq!(LiveCheck::parse("http:///x"), None);
    }

    #[tokio::test]
    async fn live_sessions_mean_a_show() {
        assert!(LiveCheck::parse(&fake_relay(r#"{"sessions":3,"live":1}"#).await).unwrap().is_live().await);
        assert!(!LiveCheck::parse(&fake_relay(r#"{"sessions":3,"live":0}"#).await).unwrap().is_live().await);
    }

    #[tokio::test]
    async fn no_answer_is_not_a_show() {
        assert!(!LiveCheck::parse(&fake_relay("not json").await).unwrap().is_live().await);
        assert!(!LiveCheck::parse(&fake_relay(r#"{"sessions":1}"#).await).unwrap().is_live().await);
        // a port nobody listens on
        let closed = TcpListener::bind("127.0.0.1:0").await.unwrap().local_addr().unwrap();
        assert!(!LiveCheck::parse(&format!("http://{closed}/status")).unwrap().is_live().await);
    }
}
