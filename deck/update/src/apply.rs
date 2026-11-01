//! Installing an unpacked bundle into the root – each file to `<path>.deck-new`, then a rename (a running binary keeps
//! its old inode, a power cut leaves the old file or the new one) – keeping what it replaced for a rollback; and the
//! systemd side (reload, restart, the health check) behind a trait.

use crate::manifest::{allowed, sha256_hex, Actions, Manifest};
use anyhow::{bail, Context, Result};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// What an install changed, so it can be undone
#[derive(Debug, Default)]
pub struct Installed {
    /// paths whose content (or mode) changed, and the `removed` ones
    pub changed: Vec<String>,
    /// files the previous bundle had and this one dropped – taken off the root (their old copy is in `previous`)
    pub removed: Vec<String>,
    /// the old files of `changed`, under their paths (a path not there was new)
    pub previous: PathBuf,
}

/// Copies every manifest file from `unpacked` into `root` when it differs; the replaced files go to `previous`.
/// `old` is the manifest of the bundle applied before (none on a deck's first update): the files it lists and
/// `manifest` does not are removed from `root`, only under the paths `allowed` accepts.
pub fn install(root: &Path, unpacked: &Path, manifest: &Manifest, previous: &Path, old: Option<&Manifest>) -> Result<Installed> {
    let _ = fs::remove_dir_all(previous);
    fs::create_dir_all(previous)?;
    let mut changed = Vec::new();
    for f in &manifest.files {
        let target = root.join(&f.path);
        let current = fs::read(&target).ok();
        let same = current.as_deref().is_some_and(|c| sha256_hex(c) == f.sha256) && mode_ok(&target, f.mode);
        if same {
            continue;
        }
        let bytes = fs::read(unpacked.join(&f.path)).with_context(|| format!("{} is not unpacked", f.path))?;
        if sha256_hex(&bytes) != f.sha256 {
            bail!("{} changed after unpacking", f.path);
        }
        if let Some(old) = &current {
            keep_old(previous, &f.path, &target, old)?;
        }
        place(&target, &bytes, f.mode)?;
        changed.push(f.path.clone());
    }
    let mut removed = Vec::new();
    for path in old.map(|o| dropped(manifest, o)).unwrap_or_default() {
        let target = root.join(&path);
        // a path that is not a file on the root (already gone, or a directory) has nothing to take off
        let Ok(current) = fs::read(&target) else { continue };
        keep_old(previous, &path, &target, &current)?;
        fs::remove_file(&target).with_context(|| path.clone())?;
        changed.push(path.clone());
        removed.push(path);
    }
    Ok(Installed { changed, removed, previous: previous.to_path_buf() })
}

/// The paths `old` lists, `new` does not, and an update may write – nothing else is ever removed
fn dropped(new: &Manifest, old: &Manifest) -> Vec<String> {
    old.files
        .iter()
        .filter(|f| allowed(&f.path) && !new.files.iter().any(|n| n.path == f.path))
        .map(|f| f.path.clone())
        .collect()
}

/// Keeps a copy of the file at `target` (its content and mode) under `previous/<path>` for a rollback
fn keep_old(previous: &Path, path: &str, target: &Path, old: &[u8]) -> Result<()> {
    let keep = previous.join(path);
    fs::create_dir_all(keep.parent().unwrap_or(previous))?;
    fs::write(&keep, old)?;
    set_mode(&keep, mode_of(target).unwrap_or(0o644))?;
    Ok(())
}

/// Puts back what `install` replaced or removed and removes what it added
pub fn rollback(root: &Path, installed: &Installed) -> Result<()> {
    for path in &installed.changed {
        let target = root.join(path);
        let keep = installed.previous.join(path);
        match fs::read(&keep) {
            Ok(old) => place(&target, &old, mode_of(&keep).unwrap_or(0o755))?,
            Err(_) => {
                let _ = fs::remove_file(&target);
            }
        }
    }
    Ok(())
}

fn place(target: &Path, bytes: &[u8], mode: u32) -> Result<()> {
    fs::create_dir_all(target.parent().context("a file without a directory")?)?;
    let tmp = PathBuf::from(format!("{}.deck-new", target.display()));
    {
        let mut f = fs::File::create(&tmp).with_context(|| tmp.display().to_string())?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    set_mode(&tmp, mode)?;
    fs::rename(&tmp, target).with_context(|| target.display().to_string())?;
    Ok(())
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_mode(_: &Path, _: u32) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn mode_of(path: &Path) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).ok().map(|m| m.permissions().mode() & 0o777)
}

#[cfg(not(unix))]
fn mode_of(_: &Path) -> Option<u32> {
    None
}

/// Windows (the tests there) keeps no modes: any mode counts as the wanted one
fn mode_ok(path: &Path, mode: u32) -> bool {
    cfg!(not(unix)) || mode_of(path) == Some(mode)
}

/// The deck's system as the updater touches it
pub trait System {
    fn daemon_reload(&self) -> Result<()>;
    fn restart(&self, unit: &str) -> Result<()>;
    fn enable(&self, unit: &str) -> Result<()>;
    fn is_active(&self, unit: &str) -> bool;
    /// systemd's automatic restarts of the unit (`NRestarts`) – a unit that crashes and comes back is not "up"
    fn restarts(&self, unit: &str) -> u64;
    /// the kiosk's console (tty1) to the front: cage only takes the screen there (seen on the X1 2026-11-01 – started
    /// while tty2 was in front, it exited every 10 s)
    fn show_kiosk(&self);
    fn reboot(&self) -> Result<()>;
    /// a show runs: a conductor is on the deck's relay (LIVE.md B.4's freeze)
    fn busy(&self) -> bool;
}

pub struct Systemd;

impl Systemd {
    fn systemctl(args: &[&str]) -> Result<()> {
        let out = std::process::Command::new("systemctl").args(args).output().context("systemctl")?;
        if !out.status.success() {
            bail!("systemctl {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim());
        }
        Ok(())
    }
}

impl System for Systemd {
    fn daemon_reload(&self) -> Result<()> {
        Self::systemctl(&["daemon-reload"])
    }
    fn restart(&self, unit: &str) -> Result<()> {
        Self::systemctl(&["restart", "--no-block", unit])
    }
    fn enable(&self, unit: &str) -> Result<()> {
        Self::systemctl(&["enable", "--now", unit])
    }
    fn is_active(&self, unit: &str) -> bool {
        Self::systemctl(&["is-active", "--quiet", unit]).is_ok()
    }
    fn restarts(&self, unit: &str) -> u64 {
        std::process::Command::new("systemctl")
            .args(["show", "--value", "-p", "NRestarts", unit])
            .output()
            .ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
            .unwrap_or(0)
    }
    fn show_kiosk(&self) {
        let _ = std::process::Command::new("chvt").arg("1").status();
    }
    fn reboot(&self) -> Result<()> {
        Self::systemctl(&["reboot"])
    }
    fn busy(&self) -> bool {
        relay_live("127.0.0.1:7700") > 0
    }
}

/// How long `activate` waits for restarted units
#[derive(Debug, Clone, Copy)]
pub struct Wait {
    /// the restart must be done by then (a stage that ignores SIGTERM holds its stop up to the unit's TimeoutStopSec)
    pub deadline: Duration,
    /// then every unit up this long without an automatic restart (a stage that dies at start is back after 2 s)
    pub stable: Duration,
}

impl Wait {
    pub const DECK: Wait = Wait { deadline: Duration::from_secs(120), stable: Duration::from_secs(15) };
    /// tests: one look
    pub const NONE: Wait = Wait { deadline: Duration::ZERO, stable: Duration::ZERO };
}

/// `live` of the relay's `GET /status` (`{ "sessions": n, "live": n }`); 0 when it does not answer
pub fn relay_live(addr: &str) -> u64 {
    let ask = || -> Result<u64> {
        let addr = addr.parse()?;
        let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(1))?;
        s.set_read_timeout(Some(Duration::from_secs(2)))?;
        s.write_all(b"GET /status HTTP/1.1\r\nHost: deck\r\nConnection: close\r\n\r\n")?;
        let mut text = String::new();
        s.take(64 * 1024).read_to_string(&mut text)?;
        let body = text.split("\r\n\r\n").nth(1).unwrap_or("");
        let v: serde_json::Value = serde_json::from_str(body.trim())?;
        Ok(v["live"].as_u64().unwrap_or(0))
    };
    ask().unwrap_or(0)
}

/// The update timer keeps itself enabled (a deck that got deck-update by hand gets its timer with the first bundle)
pub const OWN_UNITS: &[&str] = &["deck-update.timer"];

/// Restarts the units (the kiosk with its console in front)
pub fn restart_all(sys: &dyn System, units: &[String]) -> Result<()> {
    if units.iter().any(|u| u == "deck-kiosk.service") {
        sys.show_kiosk();
    }
    for unit in units {
        sys.restart(unit)?;
    }
    Ok(())
}

/// Reload, restart, then wait until every restarted unit is active and stays so for `wait.stable` without an
/// automatic restart; not by `wait.deadline` → Err naming the unit
pub fn activate(sys: &dyn System, actions: &Actions, changed: &[String], wait: Wait) -> Result<()> {
    if actions.daemon_reload {
        sys.daemon_reload()?;
    }
    for unit in OWN_UNITS {
        if changed.iter().any(|p| p.ends_with(&format!("/{unit}"))) {
            sys.enable(unit)?;
        }
    }
    restart_all(sys, &actions.restart)?;
    if actions.restart.is_empty() {
        return Ok(());
    }
    let start = Instant::now();
    // since when all are up, and their restart counts then
    let mut up: Option<(Instant, Vec<u64>)> = None;
    loop {
        let down = actions.restart.iter().find(|u| !sys.is_active(u));
        let counts: Vec<u64> = actions.restart.iter().map(|u| sys.restarts(u)).collect();
        match (&down, &up) {
            (Some(_), _) => up = None,
            (None, Some((_, before))) if *before != counts => up = Some((Instant::now(), counts)),
            (None, None) => up = Some((Instant::now(), counts)),
            (None, Some(_)) => {}
        }
        if let Some((since, _)) = &up {
            if since.elapsed() >= wait.stable {
                return Ok(());
            }
        }
        if start.elapsed() >= wait.deadline + wait.stable {
            let unit = down.unwrap_or(&actions.restart[0]);
            bail!("{unit} did not come up after the update (or kept restarting)");
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::manifest::FileEntry;
    use std::cell::RefCell;

    /// Records calls; `down` units never come up, `flapping` ones keep being restarted by systemd
    #[derive(Default)]
    pub struct FakeSystem {
        pub calls: RefCell<Vec<String>>,
        pub down: Vec<String>,
        pub flapping: Vec<String>,
        pub busy: bool,
        pub polls: std::cell::Cell<u64>,
    }

    impl System for FakeSystem {
        fn daemon_reload(&self) -> Result<()> {
            self.calls.borrow_mut().push("daemon-reload".into());
            Ok(())
        }
        fn restart(&self, unit: &str) -> Result<()> {
            self.calls.borrow_mut().push(format!("restart {unit}"));
            Ok(())
        }
        fn enable(&self, unit: &str) -> Result<()> {
            self.calls.borrow_mut().push(format!("enable {unit}"));
            Ok(())
        }
        fn is_active(&self, unit: &str) -> bool {
            !self.down.iter().any(|d| d == unit)
        }
        fn restarts(&self, unit: &str) -> u64 {
            if self.flapping.iter().any(|f| f == unit) {
                self.polls.set(self.polls.get() + 1);
            }
            self.polls.get()
        }
        fn show_kiosk(&self) {
            self.calls.borrow_mut().push("chvt 1".into());
        }
        fn reboot(&self) -> Result<()> {
            self.calls.borrow_mut().push("reboot".into());
            Ok(())
        }
        fn busy(&self) -> bool {
            self.busy
        }
    }

    fn manifest_for(files: &[(&str, &[u8])]) -> Manifest {
        let mut m = crate::manifest::tests::manifest();
        m.files = files
            .iter()
            .map(|(p, b)| FileEntry { path: p.to_string(), sha256: sha256_hex(b), mode: 0o755 })
            .collect();
        m
    }

    #[test]
    fn install_changes_what_differs_and_rolls_back() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        let unpacked = dir.path().join("unpacked");
        for (p, b) in [("usr/bin/deck-stage", &b"new stage"[..]), ("usr/bin/deck-relay", b"relay"), ("usr/bin/deck-new", b"n")] {
            fs::create_dir_all(unpacked.join(p).parent().unwrap()).unwrap();
            fs::write(unpacked.join(p), b).unwrap();
        }
        fs::create_dir_all(root.join("usr/bin")).unwrap();
        fs::write(root.join("usr/bin/deck-stage"), b"old stage").unwrap();
        fs::write(root.join("usr/bin/deck-relay"), b"relay").unwrap();
        #[cfg(unix)]
        set_mode(&root.join("usr/bin/deck-relay"), 0o755).unwrap();

        let m = manifest_for(&[("usr/bin/deck-stage", b"new stage"), ("usr/bin/deck-relay", b"relay"), ("usr/bin/deck-new", b"n")]);
        let done = install(&root, &unpacked, &m, &dir.path().join("previous"), None).unwrap();
        assert_eq!(done.changed, vec!["usr/bin/deck-stage", "usr/bin/deck-new"], "the relay is the same");
        assert_eq!(fs::read(root.join("usr/bin/deck-stage")).unwrap(), b"new stage");
        assert!(!root.join("usr/bin/deck-stage.deck-new").exists());

        rollback(&root, &done).unwrap();
        assert_eq!(fs::read(root.join("usr/bin/deck-stage")).unwrap(), b"old stage");
        assert!(!root.join("usr/bin/deck-new").exists());
    }

    #[test]
    fn install_removes_what_the_new_manifest_drops() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        let unpacked = dir.path().join("unpacked");
        fs::create_dir_all(unpacked.join("usr/bin")).unwrap();
        fs::write(unpacked.join("usr/bin/deck-stage"), b"stage").unwrap();
        fs::create_dir_all(root.join("usr/bin")).unwrap();
        fs::create_dir_all(root.join("usr/share/deck/scenes")).unwrap();
        fs::create_dir_all(root.join("etc")).unwrap();
        fs::write(root.join("usr/bin/deck-stage"), b"stage").unwrap();
        fs::write(root.join("usr/share/deck/scenes/default.toml"), b"old scene").unwrap();
        fs::write(root.join("etc/keep.conf"), b"keep").unwrap();
        let new = manifest_for(&[("usr/bin/deck-stage", b"stage")]);
        // an old manifest naming a path outside the allowed ones must not take it off the root either
        let old = manifest_for(&[
            ("usr/bin/deck-stage", b"stage"),
            ("usr/share/deck/scenes/default.toml", b"old scene"),
            ("etc/keep.conf", b"keep"),
        ]);
        let previous = dir.path().join("previous");

        // a first update (no previous manifest) removes nothing
        let first = install(&root, &unpacked, &new, &previous, None).unwrap();
        assert!(first.removed.is_empty());
        assert!(root.join("usr/share/deck/scenes/default.toml").exists());

        let done = install(&root, &unpacked, &new, &previous, Some(&old)).unwrap();
        assert_eq!(done.removed, vec!["usr/share/deck/scenes/default.toml"]);
        assert!(done.changed.contains(&"usr/share/deck/scenes/default.toml".to_string()));
        assert!(!root.join("usr/share/deck/scenes/default.toml").exists());
        assert!(root.join("etc/keep.conf").exists(), "outside the allowed paths: never removed");
        assert_eq!(fs::read(previous.join("usr/share/deck/scenes/default.toml")).unwrap(), b"old scene");

        rollback(&root, &done).unwrap();
        assert_eq!(fs::read(root.join("usr/share/deck/scenes/default.toml")).unwrap(), b"old scene");
    }

    #[test]
    fn activate_restarts_and_checks() {
        let actions = Actions { daemon_reload: true, restart: vec!["deck-kiosk.service".into()], reboot: false };
        let changed = vec!["usr/lib/systemd/system/deck-update.timer".to_string()];
        let sys = FakeSystem::default();
        activate(&sys, &actions, &changed, Wait::NONE).unwrap();
        assert_eq!(
            *sys.calls.borrow(),
            vec!["daemon-reload", "enable deck-update.timer", "chvt 1", "restart deck-kiosk.service"]
        );
        let sys = FakeSystem { down: vec!["deck-kiosk.service".into()], ..FakeSystem::default() };
        assert!(activate(&sys, &actions, &changed, Wait::NONE).is_err());

        // up the whole window: fine; up but restarted by systemd in between: not
        let wait = Wait { deadline: Duration::ZERO, stable: Duration::from_millis(600) };
        assert!(activate(&FakeSystem::default(), &actions, &changed, wait).is_ok());
        let flapping = FakeSystem { flapping: vec!["deck-kiosk.service".into()], ..FakeSystem::default() };
        let err = activate(&flapping, &actions, &changed, wait).unwrap_err().to_string();
        assert!(err.contains("deck-kiosk.service"), "{err}");
    }

    #[test]
    fn relay_live_reads_the_status() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let t = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = [0u8; 512];
            let _ = s.read(&mut buf);
            s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"sessions\":2,\"live\":1}").unwrap();
        });
        assert_eq!(relay_live(&addr), 1);
        t.join().unwrap();
        assert_eq!(relay_live("127.0.0.1:1"), 0, "no relay = not busy");
    }
}
