//! The link's files for the rest of the deck (#180, deck/os/README.md "Pairing"): one daemon pairs – `deck-link run`
//! (root, deck-link.service) – and leaves what it knows in its run directory (`/run/deck-link`, `2750 root:deck` from
//! tmpfiles.d, so every file in it is group `deck`):
//!
//! - `state` – one line: `pending K7Q42M` (the splash shows the code + the QR of `pairing_url`), `ready <name>` or
//!   `offline <why>`; world-readable inside the directory.
//! - `token` – the deck token (`scd_…`, ~1 h), mode 0640: root and the user `deck` (the stage, `deck-tls`) read it. It
//!   is written only when it changes, so its age is the token's age; a reader takes it while it is younger than
//!   `TOKEN_FRESH`. A pending deck (revoked, expired, new) has no token file.
//!
//! The private key never leaves `deck-link` (`/var/lib/deck/identity`, 0600 root). The stage and `deck-tls` only read
//! these files (`TokenSource`), so there is one handshake per deck, not one per program. No dependencies: the stage
//! uses this module without the `net` feature.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// The viewer, where the member pairs (`pairing_url`); `DECK_APP` overrides it (`app_url`)
pub const DEFAULT_APP: &str = "https://scutoons.com";
/// The daemon's run directory on a deck (deck/os tmpfiles.d); `DECK_LINK_DIR` overrides it
pub const RUN_DIR: &str = "/run/deck-link";
const STATE: &str = "state";
const TOKEN: &str = "token";
/// A token file older than this is not used: the server's token lives an hour (Decks::TOKEN_SECONDS), the daemon
/// renews it after 50 minutes (`RENEW_AFTER`)
pub const TOKEN_FRESH: Duration = Duration::from_secs(58 * 60);

/// What the deck shows about its pairing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shown {
    /// waiting for a member: the code (`K7Q42M`)
    Pending { code: String },
    /// approved: the name the member gave the deck (may be empty)
    Ready { name: String },
    /// the API could not be asked (the last token keeps working while it is fresh)
    Offline { reason: String },
}

impl Shown {
    /// The state file's line
    pub fn line(&self) -> String {
        match self {
            Shown::Pending { code } => format!("pending {code}"),
            Shown::Ready { name } => format!("ready {}", one_line(name)).trim_end().to_string(),
            Shown::Offline { reason } => format!("offline {}", one_line(reason)).trim_end().to_string(),
        }
    }

    /// The state file's first line → what it says (None: empty or unknown)
    pub fn parse(text: &str) -> Option<Shown> {
        let line = text.lines().next()?.trim();
        let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
        let rest = rest.trim().to_string();
        match word {
            "pending" if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_alphanumeric()) => {
                Some(Shown::Pending { code: rest })
            }
            "ready" => Some(Shown::Ready { name: rest }),
            "offline" => Some(Shown::Offline { reason: rest }),
            _ => None,
        }
    }

    /// One line for a person (the boot console, the journal, `deck-link status`, the start page)
    pub fn describe(&self, app_url: &str) -> String {
        match self {
            Shown::Pending { code } => format!(
                "pair this deck: code {}  ·  {}",
                format_code(code),
                pairing_url(app_url, code)
            ),
            Shown::Ready { name } if name.is_empty() => "paired".to_string(),
            Shown::Ready { name } => format!("paired: {name}"),
            Shown::Offline { reason } => format!("offline – {reason}"),
        }
    }
}

/// The run directory: `DECK_LINK_DIR`, else `RUN_DIR`
pub fn run_dir() -> PathBuf {
    std::env::var_os("DECK_LINK_DIR")
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(RUN_DIR))
}

/// The viewer the pairing link points to: `DECK_APP`, else `DEFAULT_APP`
pub fn app_url() -> String {
    std::env::var("DECK_APP")
        .ok()
        .filter(|a| !a.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_APP.to_string())
}

/// What the daemon last wrote (None: no daemon, nothing yet)
pub fn read_state(dir: &Path) -> Option<Shown> {
    Shown::parse(&fs::read_to_string(dir.join(STATE)).ok()?)
}

/// The deck token while it is fresh (None: not paired, no daemon, too old, unreadable)
pub fn read_token(dir: &Path, now: SystemTime) -> Option<String> {
    let path = dir.join(TOKEN);
    let written = fs::metadata(&path).ok()?.modified().ok()?;
    if now.duration_since(written).unwrap_or_default() >= TOKEN_FRESH {
        return None;
    }
    let token = fs::read_to_string(&path).ok()?.trim().to_string();
    (!token.is_empty() && !token.contains(char::is_whitespace)).then_some(token)
}

/// The daemon's write: `state` (only when it changed) and the token – written when it changed (Ready), removed when the
/// deck waits for a member (Pending), left alone while the API is out of reach (Offline). Both atomically (a temp file,
/// then a rename); the token 0640.
pub fn write(dir: &Path, shown: &Shown, token: Option<&str>) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    match (shown, token) {
        (Shown::Ready { .. }, Some(token)) => {
            let now = fs::read_to_string(dir.join(TOKEN)).unwrap_or_default();
            if now.trim() != token {
                replace(dir, TOKEN, &format!("{token}\n"), 0o640)?;
            }
        }
        (Shown::Pending { .. }, _) => match fs::remove_file(dir.join(TOKEN)) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
            _ => {}
        },
        _ => {}
    }
    let line = format!("{}\n", shown.line());
    if fs::read_to_string(dir.join(STATE)).ok().as_deref() != Some(line.as_str()) {
        replace(dir, STATE, &line, 0o644)?;
    }
    Ok(())
}

/// Where a program gets the deck token from: `DECK_TOKEN` (a dev deck, tests), else the daemon's files
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenSource {
    None,
    Fixed(String),
    Link(PathBuf),
}

impl TokenSource {
    /// `DECK_TOKEN` when set, else the run directory (`DECK_LINK_DIR` / `RUN_DIR`)
    pub fn from_env() -> Self {
        match std::env::var("DECK_TOKEN") {
            Ok(t) if !t.trim().is_empty() => TokenSource::Fixed(t.trim().to_string()),
            _ => TokenSource::Link(run_dir()),
        }
    }

    /// The token for `Authorization: Bearer` now (None: public content only)
    pub fn get(&self) -> Option<String> {
        match self {
            TokenSource::None => None,
            TokenSource::Fixed(t) => Some(t.clone()),
            TokenSource::Link(dir) => read_token(dir, SystemTime::now()),
        }
    }

    /// The code to pair with while the deck waits for a member
    pub fn pending_code(&self) -> Option<String> {
        match self {
            TokenSource::Link(dir) => match read_state(dir) {
                Some(Shown::Pending { code }) => Some(code),
                _ => None,
            },
            _ => None,
        }
    }
}

/// The code as the splash shows it: K7Q42M → K7Q-42M
pub fn format_code(code: &str) -> String {
    if code.len() == 6 && code.is_ascii() {
        format!("{}-{}", &code[..3], &code[3..])
    } else {
        code.to_string()
    }
}

/// What the splash's QR code opens: the pairing page of the viewer's account page with the code prefilled
pub fn pairing_url(app_url: &str, code: &str) -> String {
    format!(
        "{}/account/decks?deck={code}",
        app_url.trim_end_matches('/')
    )
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn replace(dir: &Path, name: &str, content: &str, mode: u32) -> io::Result<()> {
    let tmp = dir.join(format!(".{name}.tmp"));
    write_mode(&tmp, content, mode)?;
    fs::rename(&tmp, dir.join(name))
}

#[cfg(unix)]
fn write_mode(path: &Path, content: &str, mode: u32) -> io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(mode)
        .open(path)?;
    // the mode above passes the umask; this makes it exact
    file.set_permissions(fs::Permissions::from_mode(mode))?;
    file.write_all(content.as_bytes())?;
    file.sync_all()
}

#[cfg(not(unix))]
fn write_mode(path: &Path, content: &str, _mode: u32) -> io::Result<()> {
    fs::write(path, content)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("deck-link-pairing-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_state_line_round_trips() {
        for shown in [
            Shown::Pending { code: "K7Q42M".into() },
            Shown::Ready { name: "Stage left".into() },
            Shown::Ready { name: String::new() },
            Shown::Offline { reason: "/decks/challenge: timed out".into() },
        ] {
            assert_eq!(Shown::parse(&shown.line()), Some(shown.clone()), "{}", shown.line());
        }
        assert_eq!(Shown::Offline { reason: "a\nb  c".into() }.line(), "offline a b c");
        assert_eq!(Shown::parse(""), None);
        assert_eq!(Shown::parse("pending"), None);
        assert_eq!(Shown::parse("pending K7 Q"), None);
        assert_eq!(Shown::parse("hello"), None);
    }

    #[test]
    fn describe_says_what_to_do() {
        let app = "https://scutoons.com";
        assert_eq!(
            Shown::Pending { code: "K7Q42M".into() }.describe(app),
            "pair this deck: code K7Q-42M  ·  https://scutoons.com/account/decks?deck=K7Q42M"
        );
        assert_eq!(Shown::Ready { name: "X1".into() }.describe(app), "paired: X1");
        assert_eq!(Shown::Ready { name: String::new() }.describe(app), "paired");
    }

    #[test]
    fn the_token_is_written_on_change_removed_when_pending_and_read_while_fresh() {
        let dir = temp("token");
        let ready = Shown::Ready { name: "X1".into() };
        write(&dir, &ready, Some("scd_abc")).unwrap();
        assert_eq!(read_state(&dir), Some(ready.clone()));
        let now = SystemTime::now();
        assert_eq!(read_token(&dir, now).as_deref(), Some("scd_abc"));
        assert_eq!(read_token(&dir, now + TOKEN_FRESH), None, "too old");
        let written = fs::metadata(dir.join(TOKEN)).unwrap().modified().unwrap();
        std::thread::sleep(Duration::from_millis(20));
        write(&dir, &ready, Some("scd_abc")).unwrap();
        assert_eq!(fs::metadata(dir.join(TOKEN)).unwrap().modified().unwrap(), written, "the same token: not rewritten");
        // offline: the token stays (while fresh)
        write(&dir, &Shown::Offline { reason: "no network".into() }, None).unwrap();
        assert_eq!(read_token(&dir, now).as_deref(), Some("scd_abc"));
        // revoked → pending: the token goes
        write(&dir, &Shown::Pending { code: "ABC234".into() }, None).unwrap();
        assert_eq!(read_token(&dir, now), None);
        let source = TokenSource::Link(dir.clone());
        assert_eq!((source.get(), source.pending_code().as_deref()), (None, Some("ABC234")));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            write(&dir, &ready, Some("scd_def")).unwrap();
            assert_eq!(fs::metadata(dir.join(TOKEN)).unwrap().permissions().mode() & 0o777, 0o640);
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn no_daemon_no_token() {
        let dir = temp("none");
        assert_eq!((read_state(&dir), read_token(&dir, SystemTime::now())), (None, None));
        assert_eq!(TokenSource::Fixed("scd_x".into()).get().as_deref(), Some("scd_x"));
        assert_eq!(TokenSource::None.get(), None);
    }
}
