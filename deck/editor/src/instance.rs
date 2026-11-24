//! One deck editor per machine and API (DECK-LINK.md): the first start listens on a loopback port and keeps the port and
//! a secret in its cache (`editor-instance`); a later start – the OS starts one for every `scutoons-deck://` link – hands
//! its link (or nothing: come forward) there and quits. The running one takes what came on its next frame. A check run
//! takes no part. The secret keeps any other caller (a web page reaching 127.0.0.1) out.
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

const FILE: &str = "editor-instance";
/// the longest line taken: a link and the secret
const MAX_LINE: u64 = 8 * 1024;
const WAIT: Duration = Duration::from_secs(2);

/// what a start found
pub enum Claim {
    /// a deck editor runs already and took the start
    Handed,
    /// this one runs: the starts handed to it come through here
    Own(Instance),
}

pub struct Instance {
    rx: Receiver<String>,
    file: PathBuf,
    secret: String,
}

/// a secret for this run (std's hasher keys are random per process)
fn secret() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut out = String::new();
    for i in 0..2u64 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(i ^ u64::from(std::process::id()));
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

/// the running one's port and secret, from the cache
fn running(file: &Path) -> Option<(u16, String)> {
    let text = std::fs::read_to_string(file).ok()?;
    let (port, secret) = text.trim().split_once(' ')?;
    Some((port.parse().ok()?, secret.to_string()))
}

/// the start handed to the running one: true when it answered
fn hand(file: &Path, link: Option<&str>) -> bool {
    let Some((port, secret)) = running(file) else { return false };
    let Ok(mut s) = TcpStream::connect_timeout(&SocketAddr::from((Ipv4Addr::LOCALHOST, port)), WAIT) else { return false };
    let _ = s.set_read_timeout(Some(WAIT));
    if writeln!(s, "{secret} {}", link.unwrap_or_default()).is_err() {
        return false;
    }
    let mut answer = String::new();
    BufReader::new(s).read_line(&mut answer).is_ok() && answer.trim() == "ok"
}

/// one caller's line: the link it hands (empty: come forward), when its secret is this run's
fn take_line(s: TcpStream, secret: &str) -> Option<String> {
    let _ = s.set_read_timeout(Some(WAIT));
    let mut line = String::new();
    BufReader::new((&s).take(MAX_LINE)).read_line(&mut line).ok()?;
    let (got, link) = line.trim_end_matches(['\r', '\n']).split_once(' ')?;
    if got != secret {
        return None;
    }
    let _ = (&s).write_all(b"ok\n");
    Some(link.trim().to_string())
}

/// this start: handed to a running deck editor, else this one listens for the next starts
pub fn claim(cache: &Path, link: Option<&str>) -> Claim {
    let file = cache.join(FILE);
    if hand(&file, link) {
        return Claim::Handed;
    }
    let (tx, rx) = channel();
    let secret = secret();
    match TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) {
        Ok(listener) => {
            let port = listener.local_addr().map_or(0, |a| a.port());
            if let Err(e) = std::fs::write(&file, format!("{port} {secret}")) {
                tracing::warn!("a second start cannot find this editor: {e}");
            }
            let own = secret.clone();
            std::thread::spawn(move || {
                for s in listener.incoming().flatten() {
                    if let Some(link) = take_line(s, &own) {
                        if tx.send(link).is_err() {
                            return;
                        }
                    }
                }
            });
        }
        Err(e) => tracing::warn!("a second start cannot find this editor: {e}"),
    }
    Claim::Own(Instance { rx, file, secret })
}

impl Instance {
    /// the starts handed since the last frame: their links (empty: come forward)
    pub fn take(&self) -> Vec<String> {
        self.rx.try_iter().collect()
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        if running(&self.file).is_some_and(|(_, s)| s == self.secret) {
            let _ = std::fs::remove_file(&self.file);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_start_hands_its_link_to_the_first_and_a_stranger_is_turned_away() {
        let dir = std::env::temp_dir().join(format!("deck-editor-instance-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let Claim::Own(first) = claim(&dir, None) else { panic!("nothing runs yet") };
        assert!(matches!(claim(&dir, Some("scutoons-deck://stage/scene:nave")), Claim::Handed));
        assert!(matches!(claim(&dir, None), Claim::Handed), "a start without a link brings the first forward");
        let (port, _) = running(&dir.join(FILE)).unwrap();
        let mut s = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        writeln!(s, "not-the-secret scutoons-deck://show").unwrap();
        let mut answer = String::new();
        let _ = BufReader::new(s).read_line(&mut answer);
        assert_eq!(answer, "", "no answer without the secret");
        let mut got = Vec::new();
        for _ in 0..50 {
            got.extend(first.take());
            if got.len() >= 2 {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(got, ["scutoons-deck://stage/scene:nave", ""]);
        drop(first);
        assert!(!dir.join(FILE).exists(), "the file leaves with the editor");
        let Claim::Own(_again) = claim(&dir, Some("scutoons-deck://show")) else { panic!("nothing runs any more: this one does") };
        std::fs::remove_dir_all(&dir).ok();
    }
}
