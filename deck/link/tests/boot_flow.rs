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

/// #251: a deck that knows a network and its key boots straight to the stage – no Wi-Fi countdown, no handshake in the
/// boot flow, nothing of the deck waiting for network-online.target (networkd's wait-online held it 2 min on the X1)
#[test]
fn a_known_deck_boots_straight_to_the_stage() {
    let firstboot = read("rootfs/usr/bin/deck-firstboot");
    assert!(firstboot.contains("connection show") && firstboot.contains("connecting in the background"));
    assert!(firstboot.contains("had_key") && firstboot.contains("\"$had_key\" = 0"), "the handshake only on the first key");
    assert!(!read("rootfs/usr/bin/deck-wifi").contains("wifi_wait"), "no countdown");
    for unit in ["deck-link.service", "deck-update.service", "deck-kiosk.service"] {
        let text = read(&format!("rootfs/usr/lib/systemd/system/{unit}"));
        assert!(!text.lines().any(|l| l.starts_with("After=") && l.contains("network-online")), "{unit}");
    }
    let build = read("container/build.sh");
    for masked in ["systemd-networkd.service", "systemd-networkd-wait-online.service"] {
        assert!(build.contains(&format!("ln -sf /dev/null \"$PROFILE/airootfs/etc/systemd/system/{masked}\"")), "{masked}");
    }
}

/// #262: the image does not hard-enable serial-getty@ttyS0 (a deck without a serial port restarted it 672 times on the X1).
/// The generator starts it only with console=ttyS0 on the kernel line, which the live entries carry (profiledef's
/// KERNEL_PARAMS) and the installed deck's does not – the safe entry keeps console=tty1 only.
#[test]
fn the_serial_console_is_not_hard_enabled() {
    let build = read("container/build.sh");
    assert!(!build.contains("getty.target.wants/serial-getty@ttyS0"), "no hard enable of ttyS0");
    assert!(build.contains("serial-getty generator"), "the reason stays next to the build step");
    assert!(read("profile/profiledef.deck.sh").contains("console=ttyS0,115200"), "the live kernel line has the serial console");
    let live = read("profile/efiboot/loader/entries/01-scutoonsdeck.conf");
    assert!(live.contains("%KERNEL_PARAMS%"), "the live entry takes the kernel parameters");
}

/// mkarchiso sets modes from profiledef's file_permissions – a program missing there shipped without its execute bit
/// (deck-link on the X1's s7 image, 2026-11-01: deck-link.service "Permission denied" from boot on, #237)
#[test]
fn every_program_the_image_installs_is_executable() {
    let build = read("container/build.sh");
    let profile = read("profile/profiledef.deck.sh");
    let mut programs: Vec<String> = build
        .lines()
        .filter_map(|l| l.split("airootfs/usr/bin/").nth(1))
        .map(|rest| rest.trim_end_matches('"').to_string())
        .filter(|p| p.starts_with("deck-") && !p.contains('*') && !p.contains('"'))
        .collect();
    // the scripts of rootfs/usr/bin land there too
    for entry in std::fs::read_dir(os().join("rootfs/usr/bin")).unwrap() {
        programs.push(entry.unwrap().file_name().to_string_lossy().to_string());
    }
    assert!(programs.iter().any(|p| p == "deck-link") && programs.iter().any(|p| p == "deck-update"), "{programs:?}");
    for p in programs {
        assert!(profile.contains(&format!("[\"/usr/bin/{p}\"]=\"0:0:755\"")), "{p} is not 755 in profiledef.deck.sh");
    }
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
