//! deck-link – the deck's identity and pairing from the command line (#180, deck/os/README.md "Pairing"):
//!   deck-link id     [--dir D]                       the deck's public key (made at first run – first boot)
//!   deck-link run    [options]                       the daemon (deck-link.service, root): asks for access, keeps the
//!                                                    code / the token in the run directory (/run/deck-link – the
//!                                                    stage's splash and deck-tls read it), renews the token, asks
//!                                                    again after a revoke; prints each change (the journal)
//!   deck-link once   [options]                       one step of it (the boot flow shows its line), exit 1 offline
//!   deck-link status [--run R] [--app URL]           what the run directory says (tty2, the start page)
//!   deck-link pair   [options]                       ask and wait for the approval in the foreground (testing a deck)
//! options: --dir D (identity, /var/lib/deck/identity) · --run R (DECK_LINK_DIR, /run/deck-link) · --api URL · --app URL
//!          (DECK_APP, https://scutoons.com)
//!          · --build B (default /etc/deck-release) · --class C (default machine.toml's class)

use anyhow::{bail, Result};
use deck_link::pairing::{self, Shown};
use deck_link::{
    format_code, pairing_url, DeckInfo, HttpTransport, Identity, Link, State, DEFAULT_API,
    DEFAULT_DIR,
};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const MACHINE_TOML: &str = "/var/lib/deck/machine.toml";
const RELEASE: &str = "/etc/deck-release";

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_default();
    let (mut dir, mut run_dir, mut api, mut app) = (
        PathBuf::from(DEFAULT_DIR),
        pairing::run_dir(),
        DEFAULT_API.to_string(),
        pairing::app_url(),
    );
    let mut info = DeckInfo {
        hostname: hostname(),
        build: read_trimmed(RELEASE),
        class: DeckInfo::class_of_machine(&read_trimmed(MACHINE_TOML)),
    };
    while let Some(flag) = args.next() {
        let value = args.next().unwrap_or_default();
        match flag.as_str() {
            "--dir" => dir = PathBuf::from(value),
            "--run" => run_dir = PathBuf::from(value),
            "--api" => api = value,
            "--app" => app = value,
            "--build" => info.build = value,
            "--class" => info.class = value,
            _ => bail!("unknown option {flag}"),
        }
    }
    if command == "status" {
        match pairing::read_state(&run_dir) {
            Some(shown) => println!("{}", shown.describe(&app)),
            None => println!("deck-link is not running (systemctl status deck-link)"),
        }
        return Ok(());
    }
    let identity = Identity::load_or_create(&dir)?;
    if command == "id" {
        println!("{}", identity.public_key_hex());
        return Ok(());
    }
    let mut link = Link::new(HttpTransport::new(&api), identity, info);
    match command.as_str() {
        "pair" => pair(link, &app)?,
        "once" => {
            let (shown, token) = step(&mut link);
            pairing::write(&run_dir, &shown, token.as_deref())?;
            println!("{}", shown.describe(&app));
            if matches!(shown, Shown::Offline { .. }) {
                std::process::exit(1);
            }
        }
        "run" => run(link, &app, &run_dir),
        _ => bail!("usage: deck-link id|run|once|status|pair [--dir D] [--run R] [--api URL] [--app URL] [--build B] [--class C]"),
    }
    Ok(())
}

/// One step → what the run directory gets
fn step(link: &mut Link<HttpTransport>) -> (Shown, Option<String>) {
    match link.step(Instant::now()) {
        Ok(State::Ready { token, name }) => (Shown::Ready { name }, Some(token)),
        Ok(State::Pending { code }) => (Shown::Pending { code }, None),
        Err(error) => (Shown::Offline { reason: format!("{error:#}") }, None),
    }
}

/// The daemon: every 5 s while a code waits, every minute once paired (the link renews the token when it is due),
/// every 15 s while the API is out of reach. Never returns.
fn run(mut link: Link<HttpTransport>, app: &str, dir: &Path) -> ! {
    let mut said = String::new();
    loop {
        let (shown, token) = step(&mut link);
        if let Err(error) = pairing::write(dir, &shown, token.as_deref()) {
            eprintln!("deck-link: writing {}: {error}", dir.display());
        }
        let line = shown.describe(app);
        if line != said {
            println!("{line}");
            said = line;
        }
        std::thread::sleep(Duration::from_secs(match shown {
            Shown::Pending { .. } => 5,
            Shown::Ready { .. } => 60,
            Shown::Offline { .. } => 15,
        }));
    }
}

fn pair(mut link: Link<HttpTransport>, app: &str) -> Result<()> {
    let mut shown = String::new();
    loop {
        match link.step(Instant::now()) {
            Ok(State::Ready { name, .. }) => {
                println!(
                    "approved{}",
                    if name.is_empty() {
                        String::new()
                    } else {
                        format!(": {name}")
                    }
                );
                return Ok(());
            }
            Ok(State::Pending { code }) if code != shown => {
                println!(
                    "code {}  ·  {}",
                    format_code(&code),
                    pairing_url(app, &code)
                );
                shown = code;
            }
            Ok(State::Pending { .. }) => {}
            Err(error) => eprintln!("deck-link: {error:#} – trying again"),
        }
        std::thread::sleep(Duration::from_secs(5));
    }
}

fn read_trimmed(path: &str) -> String {
    std::fs::read_to_string(path)
        .map(|t| t.trim().to_string())
        .unwrap_or_default()
}

fn hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .map(|h| h.trim().to_string())
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_default()
}
