//! The dev PC's side: a small static HTTP server for the bundle folder (`deck-update serve`, port 7790) – the decks on
//! the LAN find it by their scan. Flat: only the folder's own files, GET only.

use anyhow::Result;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Serves `dir` on `listener` until the process ends; one thread per request. `log` gets one line per request.
pub fn serve(dir: &Path, listener: TcpListener, log: fn(&str)) -> Result<()> {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let dir = dir.to_path_buf();
        std::thread::spawn(move || {
            if let Some(line) = answer(&dir, stream) {
                log(&line);
            }
        });
    }
    Ok(())
}

/// The file for a request path: one plain file name in `dir`
fn file_for(dir: &Path, path: &str) -> Option<PathBuf> {
    let name = path.split('?').next()?.strip_prefix('/')?;
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") || name.starts_with('.') {
        return None;
    }
    let file = dir.join(name);
    file.is_file().then_some(file)
}

fn answer(dir: &Path, mut stream: TcpStream) -> Option<String> {
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    let peer = stream.peer_addr().map(|a| a.ip().to_string()).unwrap_or_default();
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut request = String::new();
    reader.read_line(&mut request).ok()?;
    // the headers are read and ignored
    let mut header = String::new();
    while reader.read_line(&mut header).ok()? > 2 {
        header.clear();
    }
    let mut words = request.split_whitespace();
    let (method, path) = (words.next()?, words.next()?);
    let file = (method == "GET").then(|| file_for(dir, path)).flatten();
    let Some(file) = file else {
        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return Some(format!("{peer} {method} {path} 404"));
    };
    let bytes = fs::read(&file).ok()?;
    let kind = if path.ends_with(".json") { "application/json" } else { "application/octet-stream" };
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        bytes.len()
    );
    stream.write_all(head.as_bytes()).ok()?;
    stream.write_all(&bytes).ok()?;
    // the manifest is asked every timer run – only the downloads are worth a line
    (!path.ends_with(".json") && !path.ends_with(".sig")).then(|| format!("{peer} fetched {path} ({} KB)", bytes.len() / 1024))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{Fetch, Http};

    #[test]
    fn serves_the_folder_and_nothing_else() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("manifest.json"), b"{\"a\":1}").unwrap();
        fs::write(dir.path().join(".hidden"), b"no").unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let root = dir.path().to_path_buf();
        std::thread::spawn(move || serve(&root, listener, |_| {}));

        let http = Http::new();
        assert_eq!(http.get(&format!("{base}/manifest.json"), 1024).unwrap(), b"{\"a\":1}");
        assert!(http.get(&format!("{base}/manifest.json"), 3).is_err(), "the limit holds");
        for bad in ["/", "/.hidden", "/../Cargo.toml", "/missing.json", "/%2e%2e/x"] {
            assert!(http.get(&format!("{base}{bad}"), 1024).is_err(), "{bad}");
        }
        let dest = dir.path().join("copy");
        let hash = http.download(&format!("{base}/manifest.json"), &dest, 1024).unwrap();
        assert_eq!(hash, crate::manifest::sha256_hex(b"{\"a\":1}"));
    }
}
