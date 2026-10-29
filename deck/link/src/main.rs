//! deck-link – the deck's identity and pairing from the command line (#180):
//!   deck-link id   [--dir D]                        the deck's public key (made at first run – first boot)
//!   deck-link pair [--dir D] [--api URL] [--app URL] [--build B] [--class C]
//!                                                   asks for access, prints the code + pairing link, waits for the
//!                                                   approval (polls every 5 s), then prints "approved" and exits 0
//! The stage uses the library itself (splash + library client); this is for the boot flow and for testing a deck.

use anyhow::{bail, Result};
use deck_link::{
    format_code, pairing_url, DeckInfo, HttpTransport, Identity, Link, State, DEFAULT_API,
    DEFAULT_APP, DEFAULT_DIR,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_default();
    let (mut dir, mut api, mut app) = (
        PathBuf::from(DEFAULT_DIR),
        DEFAULT_API.to_string(),
        DEFAULT_APP.to_string(),
    );
    let mut info = DeckInfo {
        hostname: hostname(),
        ..DeckInfo::default()
    };
    while let Some(flag) = args.next() {
        let value = args.next().unwrap_or_default();
        match flag.as_str() {
            "--dir" => dir = PathBuf::from(value),
            "--api" => api = value,
            "--app" => app = value,
            "--build" => info.build = value,
            "--class" => info.class = value,
            _ => bail!("unknown option {flag}"),
        }
    }
    let identity = Identity::load_or_create(&dir)?;
    match command.as_str() {
        "id" => println!("{}", identity.public_key_hex()),
        "pair" => pair(Link::new(HttpTransport::new(&api), identity, info), &app)?,
        _ => bail!(
            "usage: deck-link id|pair [--dir D] [--api URL] [--app URL] [--build B] [--class C]"
        ),
    }
    Ok(())
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

fn hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .map(|h| h.trim().to_string())
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_default()
}
