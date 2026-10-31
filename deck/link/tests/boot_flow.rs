//! deck-link in the deck's boot flow (#180, deck/os/README.md "Pairing"): the scripts parse (`bash -n`), the units and
//! tmpfiles.d wire the daemon, the run directory and deck-tls the way `pairing` expects, the image builds deck-link, and
//! no admin token is left on a deck (owner 2026-10-31).

use std::path::{Path, PathBuf};
use std::process::Command;

fn os() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../os")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(os().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// a bash that runs scripts (Git Bash on Windows; not WSL's launcher, which may have no distribution)
fn bash() -> Option<PathBuf> {
    let candidates = [
        std::env::var_os("DECK_TEST_BASH").map(PathBuf::from),
        Some(PathBuf::from(r"C:\Program Files\Git\bin\bash.exe")),
        Some(PathBuf::from("/bin/bash")),
        Some(PathBuf::from("bash")),
    ];
    candidates.into_iter().flatten().find(|b| {
        Command::new(b)
            .args(["-c", "echo ok"])
            .output()
            .is_ok_and(|o| o.status.success() && o.stdout.starts_with(b"ok"))
    })
}

#[test]
fn the_deck_scripts_parse() {
    let Some(bash) = bash() else {
        eprintln!("no bash here – the syntax check is skipped");
        return;
    };
    let bin = os().join("rootfs/usr/bin");
    let mut checked = 0;
    for entry in std::fs::read_dir(&bin).unwrap() {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap();
        if !text.starts_with("#!/bin/bash") {
            continue;
        }
        let out = Command::new(&bash).arg("-n").arg(&path).output().unwrap();
        assert!(
            out.status.success(),
            "{}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        checked += 1;
    }
    assert!(checked >= 7, "only {checked} scripts found in {}", bin.display());
}

#[test]
fn the_daemon_the_run_directory_and_deck_tls_fit_together() {
    let unit = read("rootfs/usr/lib/systemd/system/deck-link.service");
    assert!(unit.contains("ExecStart=/usr/bin/deck-link run"));
    assert!(unit.contains("After=deck-firstboot.service"), "the identity lives on the persistence partition, mounted by the boot flow");
    assert!(!unit.contains("User="), "root: the private key stays 0600 root");
    // the run directory: root writes, the group deck reads (setgid → every file group deck)
    let tmpfiles = read("rootfs/usr/lib/tmpfiles.d/deck.conf");
    assert!(tmpfiles.lines().any(|l| l.split_whitespace().collect::<Vec<_>>() == ["d", deck_link::pairing::RUN_DIR, "2750", "root", "deck", "-"]));
    // the boot flow makes the identity and shows the first code
    let firstboot = read("rootfs/usr/bin/deck-firstboot");
    assert!(firstboot.contains("deck-link id") && firstboot.contains("deck-link once"));
    // deck-tls: the deck token from the run directory, the deck route, a timer
    let tls = read("rootfs/usr/bin/deck-tls");
    assert!(tls.contains("/run/deck-link") && tls.contains("/token"));
    assert!(tls.contains("/api/decks/lan-cert?format=pem"));
    assert!(read("rootfs/usr/lib/systemd/system/deck-tls.service").contains("SuccessExitStatus=3"));
    assert!(read("rootfs/usr/lib/systemd/system/deck-tls.timer").contains("OnBootSec="));
    // the start page shows the pairing and the wss names
    let session = read("rootfs/usr/bin/deck-session");
    assert!(session.contains("deck-link status") && session.contains("deck-tls --names"));
    let page = read("rootfs/usr/share/deck/start.html");
    assert!(page.contains("@PAIRING@") && page.contains("@WSS@"));
}

#[test]
fn the_image_builds_and_enables_deck_link() {
    let build = read("container/build.sh");
    assert!(build.contains("-p deck-link"));
    assert!(build.contains("usr/bin/deck-link\""));
    assert!(build.contains("deck-bridge deck-link; do"), "deck-link.service enabled");
    assert!(build.contains("deck-tls.timer"));
    let pkg = std::fs::read_to_string(os().join("../pkg/scutoons-deck/PKGBUILD")).unwrap();
    assert!(pkg.contains("-p deck-link") && pkg.contains("usr/bin/deck-link\""));
}

#[test]
fn no_admin_token_on_a_deck() {
    for rel in [
        "rootfs/usr/bin/deck-tls",
        "rootfs/usr/bin/deck-set",
        "rootfs/usr/share/deck/deck.default.toml",
        "README.md",
    ] {
        let text = read(rel);
        assert!(!text.contains("tls_token"), "{rel} still names tls_token");
        assert!(!text.contains("/api/admin/lan-cert"), "{rel} still uses the admin route");
    }
}
