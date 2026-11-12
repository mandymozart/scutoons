//! The bundle on the dev PC (`pack`: the Linux binaries + deck/os/rootfs's deck paths + the scenes → a signed manifest
//! and one tar.gz) and on the deck (`unpack`: every entry checked against the signed manifest before it lands).

use crate::manifest::{allowed, hex, sha256_hex, FileEntry, Manifest, MANIFEST, SIGNATURE};
use crate::sign;
use anyhow::{bail, Context, Result};
use ed25519_dalek::SigningKey;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Nothing a deck installs is anywhere near this (the stage is ~30 MB)
pub const MAX_FILE: u64 = 256 << 20;

/// One file going into a bundle
#[derive(Debug, Clone, PartialEq)]
pub struct SourceFile {
    pub path: String,
    pub source: PathBuf,
    pub mode: u32,
}

/// cargo's binary names → where the image installs them (deck/os/container/build.sh)
const BINARIES: &[(&str, &str)] = &[
    ("relay", "usr/bin/deck-relay"),
    ("bridge", "usr/bin/deck-bridge"),
    ("deck-check", "usr/bin/deck-check"),
    ("deck", "usr/bin/deck"),
    ("deck-link", "usr/bin/deck-link"),
    ("deck-update", "usr/bin/deck-update"),
];

/// What goes into a bundle: the binaries found in `bin` (cargo's release dir), every allowed path of `rootfs`
/// (deck/os/rootfs), on the dev channel the sandbox scenes (#427: `*.toml` of deck/native/scenes/sandbox → the deck's
/// /usr/share/deck/scenes/sandbox, its dev mode). Sorted by path.
pub fn collect(bin: &Path, rootfs: &Path, scenes: Option<&Path>) -> Result<Vec<SourceFile>> {
    let mut files = Vec::new();
    for (name, path) in BINARIES {
        let source = bin.join(name);
        if source.is_file() {
            files.push(SourceFile { path: path.to_string(), source, mode: 0o755 });
        }
    }
    if files.is_empty() {
        bail!("no deck binaries in {} (relay, deck, …)", bin.display());
    }
    walk(rootfs, rootfs, &mut files)?;
    for scenes in scenes {
        for entry in fs::read_dir(scenes).with_context(|| scenes.display().to_string())? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".toml") && entry.path().is_file() {
                files.push(SourceFile { path: format!("usr/share/deck/scenes/sandbox/{name}"), source: entry.path(), mode: 0o644 });
            }
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    files.dedup_by(|a, b| a.path == b.path);
    Ok(files)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<SourceFile>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| dir.display().to_string())? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, out)?;
            continue;
        }
        let rel = path.strip_prefix(root)?.to_string_lossy().replace('\\', "/");
        if allowed(&rel) {
            // Windows keeps no modes: the scripts in usr/bin are executable, the rest is data
            let mode = if rel.starts_with("usr/bin/") { 0o755 } else { 0o644 };
            out.push(SourceFile { path: rel, source: path, mode });
        }
    }
    Ok(())
}

pub struct PackOptions<'a> {
    pub version: &'a str,
    pub build: u64,
    pub channel: &'a str,
    pub reboot: bool,
    pub notes: &'a str,
}

/// Writes `bundle-<build>.tar.gz`, `manifest.json` and `manifest.json.sig` into `out` (older bundles there are removed)
pub fn pack(files: &[SourceFile], opts: &PackOptions, key: &SigningKey, out: &Path) -> Result<Manifest> {
    fs::create_dir_all(out)?;
    let bundle = format!("bundle-{}.tar.gz", opts.build);
    let part = out.join(format!("{bundle}.part"));
    let mut entries = Vec::new();
    {
        let gz = GzEncoder::new(fs::File::create(&part)?, Compression::default());
        let mut tar = tar::Builder::new(gz);
        for f in files {
            if !allowed(&f.path) {
                bail!("{} is not a path an update may write", f.path);
            }
            let bytes = fs::read(&f.source).with_context(|| f.source.display().to_string())?;
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(f.mode);
            header.set_mtime(opts.build);
            header.set_uid(0);
            header.set_gid(0);
            header.set_entry_type(tar::EntryType::Regular);
            tar.append_data(&mut header, &f.path, bytes.as_slice())?;
            entries.push(FileEntry { path: f.path.clone(), sha256: sha256_hex(&bytes), mode: f.mode });
        }
        tar.into_inner()?.finish()?;
    }
    let bundle_sha256 = hex(&Sha256::digest(fs::read(&part)?));
    let manifest = Manifest {
        version: opts.version.to_string(),
        build: opts.build,
        channel: opts.channel.to_string(),
        bundle: bundle.clone(),
        bundle_sha256,
        files: entries,
        reboot: opts.reboot,
        notes: opts.notes.to_string(),
    };
    manifest.validate()?;
    for old in fs::read_dir(out)? {
        let name = old?.file_name().to_string_lossy().to_string();
        if name.starts_with("bundle-") && name.ends_with(".tar.gz") {
            fs::remove_file(out.join(name))?;
        }
    }
    fs::rename(&part, out.join(&bundle))?;
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    // the signature first: a deck that sees the new manifest finds a matching signature
    fs::write(out.join(SIGNATURE), sign::sign(key, &bytes))?;
    fs::write(out.join(MANIFEST), &bytes)?;
    Ok(manifest)
}

/// Unpacks `bundle` into `dest` (files under their paths). Every entry must be a regular file the signed manifest
/// lists, with its hash; every listed file must be there; the tarball itself must hash to the manifest's value.
pub fn unpack(bundle: &Path, manifest: &Manifest, dest: &Path) -> Result<()> {
    let actual = hex(&Sha256::digest(fs::read(bundle).with_context(|| bundle.display().to_string())?));
    if actual != manifest.bundle_sha256 {
        bail!("the bundle does not match the manifest (sha256 {actual})");
    }
    let want: HashMap<&str, &FileEntry> = manifest.files.iter().map(|f| (f.path.as_str(), f)).collect();
    let mut seen = 0usize;
    let mut archive = tar::Archive::new(GzDecoder::new(fs::File::open(bundle)?));
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().replace('\\', "/");
        let Some(f) = want.get(path.as_str()) else {
            bail!("the bundle holds {path}, which the manifest does not list");
        };
        if entry.header().entry_type() != tar::EntryType::Regular {
            bail!("{path} is not a regular file");
        }
        if entry.size() > MAX_FILE {
            bail!("{path} is too large");
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.by_ref().take(MAX_FILE + 1).read_to_end(&mut bytes)?;
        if sha256_hex(&bytes) != f.sha256 {
            bail!("{path} does not match its hash");
        }
        let target = dest.join(&path);
        fs::create_dir_all(target.parent().unwrap_or(dest))?;
        fs::write(&target, &bytes)?;
        seen += 1;
    }
    if seen != manifest.files.len() {
        bail!("the bundle holds {seen} of the manifest's {} files", manifest.files.len());
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn fixture(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
        let bin = dir.join("bin");
        let rootfs = dir.join("rootfs");
        let scenes = dir.join("scenes");
        for d in [&bin, &rootfs.join("usr/bin"), &rootfs.join("usr/lib/systemd/system"), &rootfs.join("etc"), &scenes] {
            fs::create_dir_all(d).unwrap();
        }
        fs::write(bin.join("relay"), b"relay v2").unwrap();
        fs::write(bin.join("deck"), b"stage v2").unwrap();
        fs::write(bin.join("not-ours"), b"x").unwrap();
        fs::write(rootfs.join("usr/bin/deck-session"), b"#!/bin/sh\n").unwrap();
        fs::write(rootfs.join("usr/lib/systemd/system/deck-kiosk.service"), b"[Unit]\n").unwrap();
        fs::write(rootfs.join("etc/secret.conf"), b"no").unwrap();
        fs::write(scenes.join("fireflies.toml"), b"name = \"fireflies\"\n").unwrap();
        (bin, rootfs, scenes)
    }

    #[test]
    fn collect_takes_only_the_deck_paths() {
        let dir = tempfile::tempdir().unwrap();
        let (bin, rootfs, scenes) = fixture(dir.path());
        let paths: Vec<_> = collect(&bin, &rootfs, Some(&scenes)).unwrap().into_iter().map(|f| (f.path, f.mode)).collect();
        assert_eq!(
            paths,
            vec![
                ("usr/bin/deck".to_string(), 0o755),
                ("usr/bin/deck-relay".to_string(), 0o755),
                ("usr/bin/deck-session".to_string(), 0o755),
                ("usr/lib/systemd/system/deck-kiosk.service".to_string(), 0o644),
                ("usr/share/deck/scenes/sandbox/fireflies.toml".to_string(), 0o644),
            ]
        );
        // no sandboxes (a stable bundle): the binaries and the rootfs alone
        assert_eq!(collect(&bin, &rootfs, None).unwrap().len(), paths.len() - 1);
    }

    #[test]
    fn pack_then_unpack_round_trips_and_tampering_is_caught() {
        let dir = tempfile::tempdir().unwrap();
        let (bin, rootfs, scenes) = fixture(dir.path());
        let files = collect(&bin, &rootfs, &scenes).unwrap();
        let key = SigningKey::from_bytes(&[7; 32]);
        let out = dir.path().join("out");
        let opts = PackOptions { version: "0.1.0.r1", build: 1000, channel: "dev", reboot: false, notes: "" };
        let m = pack(&files, &opts, &key, &out).unwrap();

        let raw = fs::read(out.join(MANIFEST)).unwrap();
        let sig = fs::read_to_string(out.join(SIGNATURE)).unwrap();
        sign::verify(&[key.verifying_key()], &raw, &sig).unwrap();
        assert_eq!(serde_json::from_slice::<Manifest>(&raw).unwrap(), m);

        let dest = dir.path().join("dest");
        unpack(&out.join(&m.bundle), &m, &dest).unwrap();
        assert_eq!(fs::read(dest.join("usr/bin/deck")).unwrap(), b"stage v2");

        // a manifest that promises other content
        let mut wrong = m.clone();
        wrong.files[0].sha256 = sha256_hex(b"other");
        assert!(unpack(&out.join(&m.bundle), &wrong, &dir.path().join("d2")).is_err());
        // a manifest missing a file the tarball holds
        let mut short = m.clone();
        short.files.pop();
        assert!(unpack(&out.join(&m.bundle), &short, &dir.path().join("d3")).is_err());
        // another tarball
        let mut other = m.clone();
        other.bundle_sha256 = sha256_hex(b"x");
        assert!(unpack(&out.join(&m.bundle), &other, &dir.path().join("d4")).is_err());

        // a second pack replaces the old tarball
        let opts = PackOptions { build: 1001, ..opts };
        pack(&files, &opts, &key, &out).unwrap();
        assert!(!out.join("bundle-1000.tar.gz").exists());
        assert!(out.join("bundle-1001.tar.gz").exists());
    }
}
