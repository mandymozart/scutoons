//! deck-update (#237, deck/os/README.md "Updates").
//!
//! On the deck (root):
//!   deck-update status                 the installed version, the channel, the last run, the trusted keys
//!   deck-update check                  is there a newer build (no download)
//!   deck-update now [--force] [--url U]  update now (--force: also during a show, also a build rolled back before)
//!   deck-update auto                   the timer's run (deck-update.timer): throttled, never during a show
//!   deck-update boot                   at boot: the staged bundle again on a live stick (deck-firstboot runs it)
//!   deck-update trust <hex>            trust another update key (/var/lib/deck/update-keys)
//! On the dev PC (`npm run deck:update` runs these):
//!   deck-update keygen [--key P]       the signing key (default ~/.scutoons/deck-update.key; kept if it exists)
//!   deck-update bundle --bin D --rootfs D [--scenes D] --out D [--version V] [--channel dev] [--reboot] [--notes T] [--key P]
//!                                        (--scenes: the sandbox folder, deck/native/scenes/sandbox – the dev channel only, #427)
//!   deck-update serve D [--port 7790]  serve the bundle folder to the decks on the LAN

use anyhow::{bail, Context, Result};
use deck_update::apply::{Systemd, Wait};
use deck_update::bundle::{self, PackOptions};
use deck_update::manifest::MANIFEST;
use deck_update::sign;
use deck_update::source::{self, Http};
use deck_update::state::{now, Paths, Stamp, State, DEV_PORT};
use deck_update::update::{Options, Outcome, Updater};
use std::net::{TcpListener, UdpSocket};
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = run(&args) {
        eprintln!("deck-update: {e:#}");
        std::process::exit(1);
    }
}

fn flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn value(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn default_key() -> PathBuf {
    if let Some(p) = std::env::var_os("DECK_UPDATE_KEY") {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).unwrap_or_default();
    PathBuf::from(home).join(".scutoons").join("deck-update.key")
}

fn run(args: &[String]) -> Result<()> {
    let cmd = args.first().map(String::as_str).unwrap_or("status");
    let paths = Paths::new("/");
    let http = Http::new();
    let system = Systemd;
    let lan = source::lan;
    let updater = Updater { paths: paths.clone(), fetch: &http, system: &system, lan: &lan, wait: Wait::DECK };
    match cmd {
        "status" => status(&updater),
        "check" | "now" | "auto" => {
            let opts = Options {
                auto: cmd == "auto",
                force: flag(args, "--force"),
                check: cmd == "check",
                url: value(args, "--url"),
            };
            match updater.run(&opts, now()) {
                Ok(outcome) => {
                    println!("{}", outcome.message());
                    // the timer's run never fails the unit for "nothing there"; a manual run says so with its code
                    if cmd != "auto" && matches!(outcome, Outcome::NoSource(_) | Outcome::RolledBack { .. }) {
                        std::process::exit(3);
                    }
                    Ok(())
                }
                Err(e) if cmd == "auto" => {
                    println!("update failed: {e:#}");
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }
        "boot" => {
            match updater.boot() {
                Ok(o) => println!("{}", o.message()),
                // the boot goes on with what the root has
                Err(e) => println!("the staged update was not re-applied: {e:#}"),
            }
            Ok(())
        }
        "trust" => {
            let hex = args.get(1).context("deck-update trust <public key hex>")?;
            sign::parse_key(hex)?;
            let file = paths.own_keys();
            let mut text = std::fs::read_to_string(&file).unwrap_or_default();
            if !text.contains(hex.trim()) {
                text.push_str(&format!("{}\n", hex.trim()));
                deck_update::state::write_atomic(&file, text.as_bytes())?;
            }
            println!("trusted: {} ({})", hex.trim(), file.display());
            Ok(())
        }
        "keygen" => {
            let key = value(args, "--key").map(PathBuf::from).unwrap_or_else(default_key);
            println!("{}", sign::keygen(&key)?);
            eprintln!("(the private key: {} – keep it, never commit it)", key.display());
            Ok(())
        }
        "bundle" => make_bundle(args),
        "serve" => {
            let dir = PathBuf::from(args.get(1).context("deck-update serve <folder>")?);
            let port: u16 = value(args, "--port").map(|p| p.parse()).transpose()?.unwrap_or(DEV_PORT);
            serve(&dir, port)
        }
        "-h" | "--help" | "help" => {
            println!("{}", include_str!("main.rs").lines().skip(2).take_while(|l| l.starts_with("//!")).map(|l| l.trim_start_matches("//!")).collect::<Vec<_>>().join("\n"));
            Ok(())
        }
        other => bail!("unknown command {other:?} (deck-update help)"),
    }
}

fn status(u: &Updater) -> Result<()> {
    let stamp = Stamp::read(&u.paths);
    let state = State::load(&u.paths);
    let config = u.config();
    let key_paths = u.paths.keys();
    let keys = sign::trusted(&[&key_paths[0], &key_paths[1]]).map(|k| k.len()).unwrap_or(0);
    println!("installed: {} (build {})", if stamp.version.is_empty() { "the image's" } else { &stamp.version }, stamp.build);
    println!("channel:   {}{}", config.channel.name(), config.url.map(|u| format!(" · update_url {u}")).unwrap_or_default());
    println!("source:    {}", state.source.as_deref().unwrap_or("none yet"));
    println!("keys:      {keys} trusted");
    println!("last run:  {}", if state.last.is_empty() { "never" } else { &state.last });
    Ok(())
}

fn make_bundle(args: &[String]) -> Result<()> {
    let need = |n: &str| value(args, n).map(PathBuf::from).with_context(|| format!("deck-update bundle needs {n}"));
    let (bin, rootfs, out) = (need("--bin")?, need("--rootfs")?, need("--out")?);
    let scenes = value(args, "--scenes").map(PathBuf::from);
    let key_path = value(args, "--key").map(PathBuf::from).unwrap_or_else(default_key);
    let public = sign::keygen(&key_path)?;
    let key = sign::load_signing(&key_path)?;
    let build = now();
    let version = value(args, "--version").unwrap_or_else(|| format!("build {build}"));
    let channel = value(args, "--channel").unwrap_or_else(|| "dev".into());
    let notes = value(args, "--notes").unwrap_or_default();
    if scenes.is_some() && channel != "dev" {
        bail!("--scenes: the sandboxes go to the dev channel only (#427), not to '{channel}'");
    }
    let files = bundle::collect(&bin, &rootfs, scenes.as_deref())?;
    let opts = PackOptions { version: &version, build, channel: &channel, reboot: flag(args, "--reboot"), notes: &notes };
    let m = bundle::pack(&files, &opts, &key, &out)?;
    // for the first time on a deck that has no deck-update yet (the banner of `serve` shows the commands)
    if bin.join("deck-update").is_file() {
        std::fs::copy(bin.join("deck-update"), out.join("deck-update"))?;
    }
    std::fs::write(out.join("update-key.pub"), format!("{public}\n"))?;
    println!("{} · build {} · {} files → {}", m.version, m.build, m.files.len(), out.join(MANIFEST).display());
    Ok(())
}

/// The PC's address on the LAN: the source address of a route to the internet (a UDP connect sends nothing)
fn lan_address() -> Option<String> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:80").ok()?;
    Some(s.local_addr().ok()?.ip().to_string())
}

fn serve(dir: &Path, port: u16) -> Result<()> {
    if !dir.join(MANIFEST).is_file() {
        bail!("no {MANIFEST} in {} – deck-update bundle first", dir.display());
    }
    let listener = TcpListener::bind(("0.0.0.0", port)).with_context(|| format!("port {port}"))?;
    let host = lan_address().unwrap_or_else(|| "<this PC>".into());
    let base = format!("http://{host}:{port}");
    let public = std::fs::read_to_string(dir.join("update-key.pub")).unwrap_or_default();
    println!("serving {} on {base} – decks on the dev channel find it within a few minutes, or at once:", dir.display());
    println!("  on the deck (tty2 / ssh, root):  deck-update now");
    println!("a deck without deck-update yet (once):");
    println!("  curl -fsS {base}/deck-update -o /usr/bin/deck-update && chmod 755 /usr/bin/deck-update");
    println!("  deck-update trust {} && deck-update now --url {base}", public.trim());
    deck_update::serve::serve(dir, listener, |line| println!("{line}"))
}
