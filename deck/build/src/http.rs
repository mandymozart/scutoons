//! Just enough HTTP/1.1 for one JSON route (like the relay's `/status`, hand-rolled on tokio: no framework for three
//! routes): a request head and a `Content-Length` body in, a JSON answer out, `Connection: close`.
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const MAX_HEAD: usize = 16 * 1024;
/// a request that has not arrived whole after this long is dropped
const READ_BUDGET: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    pub path: String,
    /// names in lower case
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReadError {
    /// not HTTP we understand
    Bad(&'static str),
    TooLarge,
    /// the client went away or was too slow
    Gone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub body: String,
}

impl Response {
    pub fn json(status: u16, value: &serde_json::Value) -> Response {
        Response { status, body: value.to_string() }
    }

    pub fn error(status: u16, code: &str) -> Response {
        Response::json(status, &serde_json::json!({ "error": code }))
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        422 => "Unprocessable Entity",
        503 => "Service Unavailable",
        _ => "Internal Server Error",
    }
}

pub async fn write_response<W: AsyncWrite + Unpin>(w: &mut W, r: &Response) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        r.status,
        reason(r.status),
        r.body.len()
    );
    w.write_all(head.as_bytes()).await?;
    w.write_all(r.body.as_bytes()).await?;
    w.flush().await
}

pub async fn read_request<S: AsyncRead + AsyncWrite + Unpin>(s: &mut S, max_body: usize) -> Result<Request, ReadError> {
    tokio::time::timeout(READ_BUDGET, read(s, max_body)).await.map_err(|_| ReadError::Gone)?
}

async fn read<S: AsyncRead + AsyncWrite + Unpin>(s: &mut S, max_body: usize) -> Result<Request, ReadError> {
    let mut buf = Vec::new();
    let head_end = loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        if buf.len() > MAX_HEAD {
            return Err(ReadError::Bad("the request head is too long"));
        }
        let mut chunk = [0u8; 4096];
        match s.read(&mut chunk).await {
            Ok(0) | Err(_) => return Err(ReadError::Gone),
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    };
    let head = std::str::from_utf8(&buf[..head_end]).map_err(|_| ReadError::Bad("the request head is not text"))?;
    let mut lines = head.split("\r\n");
    let mut first = lines.next().unwrap_or("").split(' ');
    let (method, target, version) = (first.next().unwrap_or(""), first.next().unwrap_or(""), first.next().unwrap_or(""));
    if method.is_empty() || target.is_empty() || !version.starts_with("HTTP/1.") {
        return Err(ReadError::Bad("not an HTTP/1.x request"));
    }
    let headers: Vec<(String, String)> = lines.filter_map(|l| l.split_once(':')).map(|(n, v)| (n.trim().to_ascii_lowercase(), v.trim().to_string())).collect();
    let find = |name: &str| headers.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str());
    if find("transfer-encoding").is_some() {
        return Err(ReadError::Bad("chunked bodies are not supported: send Content-Length"));
    }
    let length: usize = match find("content-length") {
        Some(v) => v.parse().map_err(|_| ReadError::Bad("bad Content-Length"))?,
        None => 0,
    };
    if length > max_body {
        return Err(ReadError::TooLarge);
    }
    // curl waits for this before it sends a larger body
    if length > 0 && find("expect").is_some_and(|v| v.eq_ignore_ascii_case("100-continue")) {
        s.write_all(b"HTTP/1.1 100 Continue\r\n\r\n").await.map_err(|_| ReadError::Gone)?;
    }
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < length {
        let mut chunk = [0u8; 16 * 1024];
        match s.read(&mut chunk).await {
            Ok(0) | Err(_) => return Err(ReadError::Gone),
            Ok(n) => body.extend_from_slice(&chunk[..n]),
        }
    }
    body.truncate(length);
    // the path without a query
    let path = target.split('?').next().unwrap_or(target).to_string();
    Ok(Request { method: method.to_string(), path, headers, body })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::duplex;

    async fn parse(raw: &[u8], max: usize) -> Result<Request, ReadError> {
        let (mut client, mut server) = duplex(1 << 20);
        client.write_all(raw).await.unwrap();
        drop(client);
        read_request(&mut server, max).await
    }

    #[tokio::test]
    async fn a_post_with_a_body() {
        let r = parse(b"POST /compile?x=1 HTTP/1.1\r\nHost: b\r\nX-Build-Token: T\r\nContent-Length: 5\r\n\r\nhello", 100).await.unwrap();
        assert_eq!((r.method.as_str(), r.path.as_str(), r.header("x-build-token"), r.body.as_slice()), ("POST", "/compile", Some("T"), &b"hello"[..]));
    }

    #[tokio::test]
    async fn a_get_has_no_body() {
        let r = parse(b"GET /health HTTP/1.0\r\n\r\n", 100).await.unwrap();
        assert_eq!((r.method.as_str(), r.body.len()), ("GET", 0));
    }

    #[tokio::test]
    async fn the_limits_and_the_garbage() {
        assert_eq!(parse(b"POST /compile HTTP/1.1\r\nContent-Length: 101\r\n\r\n", 100).await, Err(ReadError::TooLarge));
        assert_eq!(parse(b"POST / HTTP/1.1\r\nContent-Length: abc\r\n\r\n", 100).await, Err(ReadError::Bad("bad Content-Length")));
        assert!(matches!(parse(b"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n", 100).await, Err(ReadError::Bad(_))));
        assert!(matches!(parse(b"hello\r\n\r\n", 100).await, Err(ReadError::Bad(_))));
        // the body is cut short: the client went away
        assert_eq!(parse(b"POST / HTTP/1.1\r\nContent-Length: 10\r\n\r\nabc", 100).await, Err(ReadError::Gone));
        assert_eq!(parse(b"", 100).await, Err(ReadError::Gone));
        let long = format!("GET /{} HTTP/1.1\r\n", "a".repeat(MAX_HEAD + 10));
        assert!(matches!(parse(long.as_bytes(), 100).await, Err(ReadError::Bad(_))));
    }

    #[tokio::test]
    async fn expect_continue_is_answered_before_the_body_is_read() {
        let (mut client, mut server) = duplex(1 << 16);
        client.write_all(b"POST /compile HTTP/1.1\r\nExpect: 100-continue\r\nContent-Length: 2\r\n\r\n").await.unwrap();
        let reader = tokio::spawn(async move { read_request(&mut server, 100).await });
        let mut answer = [0u8; 25];
        client.read_exact(&mut answer).await.unwrap();
        assert_eq!(&answer, b"HTTP/1.1 100 Continue\r\n\r\n");
        client.write_all(b"{}").await.unwrap();
        assert_eq!(reader.await.unwrap().unwrap().body, b"{}");
    }

    #[tokio::test]
    async fn a_response_is_complete() {
        let mut out = Vec::new();
        write_response(&mut out, &Response::error(401, "unauthorized")).await.unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.starts_with("HTTP/1.1 401 Unauthorized\r\n"));
        assert!(text.contains("Content-Length: 24\r\n"));
        assert!(text.ends_with("\r\n\r\n{\"error\":\"unauthorized\"}"));
    }
}
