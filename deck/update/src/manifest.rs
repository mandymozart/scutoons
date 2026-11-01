//! The manifest, the paths a bundle may touch and what a changed file restarts.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

pub const MANIFEST: &str = "manifest.json";
pub const SIGNATURE: &str = "manifest.json.sig";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileEntry {
    /// relative to the root, `/`-separated: `usr/bin/deck-stage`
    pub path: String,
    pub sha256: String,
    pub mode: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    /// for people: `0.1.0.r1234.gabcdef0`
    pub version: String,
    /// unix seconds of the build – the only thing "newer" means
    pub build: u64,
    /// `dev` (the dev PC) or `stable` (the server)
    pub channel: String,
    /// the tarball's file name, next to the manifest
    pub bundle: String,
    pub bundle_sha256: String,
    pub files: Vec<FileEntry>,
    /// a kernel or boot-flow change: the deck reboots (when idle) instead of restarting services
    #[serde(default)]
    pub reboot: bool,
    #[serde(default)]
    pub notes: String,
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn unhex(text: &str) -> Result<Vec<u8>> {
    let text = text.trim();
    if text.len() % 2 != 0 || !text.is_ascii() {
        bail!("not hex: {text:?}");
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| anyhow::anyhow!("not hex: {text:?}")))
        .collect()
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// A relative path with plain components only (no `..`, no absolute path, no drive, no backslash)
pub fn safe(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && !path.contains(':')
        && path.split('/').all(|c| !c.is_empty() && c != "." && c != "..")
}

/// What an update may write: the deck's binaries and scripts, its units, its share directory. Never `etc/` (Wi-Fi
/// keyfiles, PipeWire), never anything else of the base system.
pub fn allowed(path: &str) -> bool {
    if !safe(path) {
        return false;
    }
    let depth = path.matches('/').count();
    (path.starts_with("usr/bin/deck-") && depth == 2)
        || (path.starts_with("usr/lib/systemd/system/deck-") && depth == 4)
        || path == "usr/lib/sysusers.d/deck.conf"
        || path == "usr/lib/tmpfiles.d/deck.conf"
        || path.starts_with("usr/share/deck/")
}

impl Manifest {
    pub fn validate(&self) -> Result<()> {
        if self.build == 0 || self.version.is_empty() || self.channel.is_empty() {
            bail!("the manifest has no version / build / channel");
        }
        if !safe(&self.bundle) || self.bundle.contains('/') {
            bail!("the bundle name {:?} is not a plain file name", self.bundle);
        }
        let hash_ok = |h: &str| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit());
        if !hash_ok(&self.bundle_sha256) {
            bail!("the bundle hash is not a SHA-256");
        }
        if self.files.is_empty() {
            bail!("the manifest lists no files");
        }
        let mut seen = HashSet::new();
        for f in &self.files {
            if !allowed(&f.path) {
                bail!("{} is not a path an update may write", f.path);
            }
            if !hash_ok(&f.sha256) {
                bail!("{}: the hash is not a SHA-256", f.path);
            }
            if f.mode & !0o777 != 0 {
                bail!("{}: mode {:o} is not a plain permission set", f.path, f.mode);
            }
            if !seen.insert(f.path.as_str()) {
                bail!("{} is listed twice", f.path);
            }
        }
        Ok(())
    }
}

/// What the changed files ask of systemd
#[derive(Debug, Default, PartialEq)]
pub struct Actions {
    pub daemon_reload: bool,
    /// in this order (link → relay → bridge → other units → kiosk)
    pub restart: Vec<String>,
    pub reboot: bool,
}

pub fn actions(changed: &[String], reboot: bool) -> Actions {
    let mut daemon_reload = false;
    let mut restart: Vec<String> = Vec::new();
    let mut want = |unit: &str| {
        if !restart.iter().any(|u| u == unit) {
            restart.push(unit.to_string());
        }
    };
    for p in changed {
        match p.as_str() {
            "usr/bin/deck-link" => want("deck-link.service"),
            "usr/bin/deck-relay" => want("deck-relay.service"),
            "usr/bin/deck-bridge" => want("deck-bridge.service"),
            "usr/bin/deck-stage" | "usr/bin/deck-session" => want("deck-kiosk.service"),
            p if p.starts_with("usr/share/deck/") => want("deck-kiosk.service"),
            p if p.starts_with("usr/lib/systemd/system/") => {
                daemon_reload = true;
                let unit = p.rsplit('/').next().unwrap_or("");
                // the update units run us (a restart would cut this run off); the boot steps run once per boot
                let skip = unit.starts_with("deck-update")
                    || unit == "deck-firstboot.service"
                    || unit == "deck-tls.service";
                if unit.ends_with(".service") && !skip {
                    want(unit);
                }
            }
            _ => {}
        }
    }
    let rank = |u: &str| match u {
        "deck-link.service" => 0,
        "deck-relay.service" => 1,
        "deck-bridge.service" => 2,
        "deck-kiosk.service" => 4,
        _ => 3,
    };
    restart.sort_by_key(|u| rank(u));
    Actions { daemon_reload, restart, reboot }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn the_allowed_paths() {
        for ok in [
            "usr/bin/deck-stage",
            "usr/lib/systemd/system/deck-kiosk.service",
            "usr/share/deck/scenes/cathedral.toml",
            "usr/lib/tmpfiles.d/deck.conf",
        ] {
            assert!(allowed(ok), "{ok}");
        }
        for bad in [
            "",
            "/usr/bin/deck-stage",
            "usr/bin/../../etc/passwd",
            "usr/bin/deck-x/../../sh",
            "usr/bin/sh",
            "usr/bin/deck-a/b",
            "etc/NetworkManager/system-connections/deck-hotspot.nmconnection",
            "usr/share/deck/../../etc/shadow",
            "usr\\bin\\deck-stage",
            "C:/usr/bin/deck-stage",
            "usr/lib/systemd/system/sshd.service",
            "usr/lib/systemd/system/deck-x/y.service",
        ] {
            assert!(!allowed(bad), "{bad}");
        }
    }

    #[test]
    fn hex_round_trips() {
        assert_eq!(unhex(&hex(&[0, 1, 254, 255])).unwrap(), vec![0, 1, 254, 255]);
        assert!(unhex("abc").is_err());
        assert!(unhex("zz").is_err());
        assert_eq!(sha256_hex(b"abc").len(), 64);
    }

    pub(crate) fn manifest() -> Manifest {
        Manifest {
            version: "0.1.0".into(),
            build: 10,
            channel: "dev".into(),
            bundle: "bundle.tar.gz".into(),
            bundle_sha256: sha256_hex(b"x"),
            files: vec![FileEntry { path: "usr/bin/deck-stage".into(), sha256: sha256_hex(b"y"), mode: 0o755 }],
            reboot: false,
            notes: String::new(),
        }
    }

    #[test]
    fn a_manifest_is_validated() {
        assert!(manifest().validate().is_ok());
        let mut m = manifest();
        m.files[0].path = "usr/bin/sh".into();
        assert!(m.validate().is_err());
        let mut m = manifest();
        m.bundle = "../x.tar.gz".into();
        assert!(m.validate().is_err());
        let mut m = manifest();
        m.files.push(m.files[0].clone());
        assert!(m.validate().is_err());
        let mut m = manifest();
        m.files[0].mode = 0o4755;
        assert!(m.validate().is_err());
        let mut m = manifest();
        m.build = 0;
        assert!(m.validate().is_err());
    }

    #[test]
    fn changed_files_restart_what_runs_them() {
        let a = actions(&["usr/bin/deck-stage".into(), "usr/share/deck/scenes/a.toml".into()], false);
        assert_eq!(a.restart, vec!["deck-kiosk.service"]);
        assert!(!a.daemon_reload);

        let a = actions(
            &[
                "usr/bin/deck-stage".into(),
                "usr/bin/deck-relay".into(),
                "usr/bin/deck-link".into(),
                "usr/lib/systemd/system/deck-update.service".into(),
                "usr/lib/systemd/system/deck-bridge.service".into(),
                "usr/bin/deck-update".into(),
                "usr/bin/deck-tls".into(),
            ],
            true,
        );
        assert!(a.daemon_reload && a.reboot);
        assert_eq!(a.restart, vec!["deck-link.service", "deck-relay.service", "deck-bridge.service", "deck-kiosk.service"]);

        let a = actions(&["usr/bin/deck-check".into(), "usr/bin/deck-firstboot".into()], false);
        assert_eq!(a, Actions::default());
    }
}
