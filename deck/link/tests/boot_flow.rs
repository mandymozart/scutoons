//! deck-link in the deck's boot flow (#180, deck/os/README.md "Pairing"): the scripts parse (`bash -n`), the units and
//! tmpfiles.d wire the daemon, the run directory and deck-tls the way `pairing` expects, the image builds deck-link, and
//! no admin token is left on a deck (owner 2026-10-31). deck-session's restart and browser logic runs against fakes
//! (#641).

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
    // the enable loop names deck-link (#834 added deck-irq after it)
    assert!(build.lines().any(|l| l.starts_with("for u in ") && l.contains(" deck-link ") && l.trim_end().ends_with("; do")), "deck-link.service enabled");
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

/// deck-session against fakes of `deck`, `chromium`, `cage` and `sleep` (each logs its call; `deck` exits with the next
/// of `codes`, then 75 – the display lost, the session ends). `inside` = the part cage runs. The log, and whether the
/// session's own exit was 75.
fn session(bash: &Path, name: &str, inside: bool, codes: &[i32], deck_toml: &str, with_deck: bool) -> (String, bool) {
    let t = std::env::temp_dir().join(format!("deck-session-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&t);
    for d in ["bin", "state", "run"] {
        std::fs::create_dir_all(t.join(d)).unwrap();
    }
    let fake = |bin: &str, body: &str| std::fs::write(t.join("bin").join(bin), format!("#!/bin/bash\necho \"{bin} $*\" >>\"$T/log\"\n{body}\n")).unwrap();
    if with_deck {
        fake("deck", "n=$(( $(cat \"$T/n\" 2>/dev/null || echo 0) + 1 )); echo $n >\"$T/n\"; c=$(sed -n \"${n}p\" \"$T/codes\"); exit ${c:-75}");
    }
    fake("chromium", "");
    fake("cage", "");
    // the wait for a missing deck never ends by itself: the fake ends the session there
    fake("sleep", if with_deck { "" } else { "kill -TERM $PPID" });
    std::fs::write(t.join("codes"), codes.iter().map(|c| format!("{c}\n")).collect::<String>()).unwrap();
    std::fs::write(t.join("state/deck.toml"), deck_toml).unwrap();
    let script = os().join("rootfs/usr/bin/deck-session");
    let run = "T=$(cygpath -u \"$T\" 2>/dev/null || echo \"$T\"); S=$(cygpath -u \"$S\" 2>/dev/null || echo \"$S\"); chmod +x \"$T\"/bin/*; \
               export T DECK_STATE=\"$T/state\" XDG_RUNTIME_DIR=\"$T/run\" PATH=\"$T/bin:/usr/bin:/bin\"; bash \"$S\" $ARG 2>>\"$T/log\"";
    let out = Command::new(bash)
        .args(["-c", run])
        .env("T", &t)
        .env("S", &script)
        .env("ARG", if inside { "inside" } else { "" })
        .output()
        .unwrap();
    let log = std::fs::read_to_string(t.join("log")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&t);
    (log, out.status.code() == Some(75))
}

/// #641: the deck always plays natively – a crash starts the stage again (the third within a minute waits 30 s), never
/// the browser; the browser opens only on the menu's OPEN CREATOR (exit 12) and the stage comes back after it; an old
/// `kiosk = "browser"` plays the stage; a missing `deck` is said and waited for, never the browser.
#[test]
fn the_session_restarts_the_stage_and_opens_the_browser_only_on_purpose() {
    let Some(bash) = bash() else {
        eprintln!("no bash here – the session test is skipped");
        return;
    };
    let count = |log: &str, what: &str| log.lines().filter(|l| l.starts_with(what)).count();
    let sleeps = |log: &str| log.lines().filter(|l| l.starts_with("sleep ")).map(str::to_owned).collect::<Vec<_>>();

    let (log, ended) = session(&bash, "crash", true, &[1, 139, 1], "kiosk = \"stage\"\n", true);
    assert!(ended, "{log}");
    assert_eq!(count(&log, "deck --"), 4, "three crashes, each the stage again: {log}");
    assert_eq!(sleeps(&log), ["sleep 1", "sleep 1", "sleep 30"], "the third within a minute backs off: {log}");
    assert_eq!(count(&log, "chromium"), 0, "never the browser on a crash: {log}");
    assert!(log.contains("3 crashes within a minute"), "{log}");

    let (log, ended) = session(&bash, "creator", true, &[12], "browser_url = \"\"\n", true);
    assert!(ended, "{log}");
    assert!(log.contains("chromium --kiosk --app=https://creator.scutoons.com"), "OPEN CREATOR: the creator: {log}");
    assert_eq!(count(&log, "deck --"), 2, "the stage again after the browser: {log}");
    let (log, _) = session(&bash, "url", true, &[12], "browser_url = \"https://example.test/x\"\n", true);
    assert!(log.contains("--app=https://example.test/x"), "{log}");

    let (log, ended) = session(&bash, "old", true, &[], "kiosk = \"browser\"\n", true);
    assert!(ended && count(&log, "deck --") == 1 && count(&log, "chromium") == 0, "an old browser mode plays the stage: {log}");
    assert!(log.contains("kiosk = \"browser\" is no mode any more"), "{log}");

    let (log, ended) = session(&bash, "missing", false, &[], "", false);
    assert!(!ended && log.contains("no deck in this image"), "{log}");
    assert_eq!((count(&log, "chromium"), count(&log, "cage")), (0, 0), "no browser, no kiosk without the stage: {log}");

    // deck-set refuses the old mode before it touches deck.toml
    let set = Command::new(&bash).arg(os().join("rootfs/usr/bin/deck-set")).args(["kiosk", "browser"]).output().unwrap();
    assert_eq!(set.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&set.stdout).contains("OPEN CREATOR"));
}
