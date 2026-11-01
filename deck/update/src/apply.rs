//! Installing an unpacked bundle into the root – each file to `<path>.deck-new`, then a rename (a running binary keeps
//! its old inode, a power cut leaves the old file or the new one) – keeping what it replaced for a rollback; and the
//! systemd side (reload, restart, the health check) behind a trait.

use crate::manifest::{sha256_hex, Actions, Manifest};
use anyhow::{bail, Context, Result};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// What an install changed, so it can be undone
#[derive(Debug, Default)]
pub struct Installed {
    /// paths whose content (or mode) changed
    pub changed: Vec<String>,
    /// the old files of `changed`, under their paths (a path not there was new)
    pub previous: PathBuf,
}

/// Copies every manifest file from `unpacked` into `root` when it differs; the replaced files go to `previous`
pub fn install(root: &Path, unpacked: &Path, manifest: &Manifest, previous: &Path) -> Result<Installed> {
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
            let keep = previous.join(&f.path);
            fs::create_dir_all(keep.parent().unwrap_or(previous))?;
            fs::write(&keep, old)?;
            set_mode(&keep, mode_of(&target).unwrap_or(0o644))?;
        }
        place(&target, &bytes, f.mode)?;
        changed.push(f.path.clone());
    }
    Ok(Installed { changed, previous: previous.to_path_buf() })
}

/// Puts back what `install` replaced and removes what it added
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
    fn reboot(&self) -> Result<()> {
        Self::systemctl(&["reboot"])
    }
    fn busy(&self) -> bool {
        relay_live("127.0.0.1:7700") > 0
    }
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

/// Reload, restart, then wait `settle` and check every restarted unit is running; the first one that is not → Err
pub fn activate(sys: &dyn System, actions: &Actions, changed: &[String], settle: Duration) -> Result<()> {
    if actions.daemon_reload {
        sys.daemon_reload()?;
    }
    for unit in OWN_UNITS {
        if changed.iter().any(|p| p.ends_with(&format!("/{unit}"))) {
            sys.enable(unit)?;
        }
    }
    for unit in &actions.restart {
        sys.restart(unit)?;
    }
    if actions.restart.is_empty() {
        return Ok(());
    }
    // a unit that dies at start is "activating (auto-restart)" or "failed" after a few seconds, not "active"
    let until = Instant::now() + settle;
    loop {
        let down: Vec<&String> = actions.restart.iter().filter(|u| !sys.is_active(u)).collect();
        if Instant::now() >= until {
            if let Some(u) = down.first() {
                bail!("{u} did not come up after the update");
            }
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(500).min(settle));
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::manifest::FileEntry;
    use std::cell::RefCell;

    /// Records calls; `down` units never come up
    #[derive(Default)]
    pub struct FakeSystem {
        pub calls: RefCell<Vec<String>>,
        pub down: Vec<String>,
        pub busy: bool,
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
        let done = install(&root, &unpacked, &m, &dir.path().join("previous")).unwrap();
        assert_eq!(done.changed, vec!["usr/bin/deck-stage", "usr/bin/deck-new"], "the relay is the same");
        assert_eq!(fs::read(root.join("usr/bin/deck-stage")).unwrap(), b"new stage");
        assert!(!root.join("usr/bin/deck-stage.deck-new").exists());

        rollback(&root, &done).unwrap();
        assert_eq!(fs::read(root.join("usr/bin/deck-stage")).unwrap(), b"old stage");
        assert!(!root.join("usr/bin/deck-new").exists());
    }

    #[test]
    fn activate_restarts_and_checks() {
        let actions = Actions { daemon_reload: true, restart: vec!["deck-kiosk.service".into()], reboot: false };
        let changed = vec!["usr/lib/systemd/system/deck-update.timer".to_string()];
        let sys = FakeSystem::default();
        activate(&sys, &actions, &changed, Duration::ZERO).unwrap();
        assert_eq!(
            *sys.calls.borrow(),
            vec!["daemon-reload", "enable deck-update.timer", "restart deck-kiosk.service"]
        );
        let sys = FakeSystem { down: vec!["deck-kiosk.service".into()], ..FakeSystem::default() };
        assert!(activate(&sys, &actions, &changed, Duration::ZERO).is_err());
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
