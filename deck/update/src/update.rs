//! One update run (`deck-update now`, the timer's `auto`) and the boot step (`deck-update boot`).

use crate::apply::{self, System};
use crate::bundle;
use crate::manifest::{actions, Manifest, MANIFEST, SIGNATURE};
use crate::sign;
use crate::source::{self, Fetch, Offer};
use crate::state::{write_atomic, Channel, Config, Paths, Stamp, State, STABLE_EVERY};
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

/// The largest bundle a deck downloads
const MAX_BUNDLE: u64 = 512 << 20;

/// In `current/`: the manifest of the bundle applied before the staged one (what a boot from an older root removes)
const PREVIOUS: &str = "previous-manifest.json";

/// The manifest of the bundle last applied (`current/`), if there is one
fn last_applied(current: &Path) -> Option<Manifest> {
    serde_json::from_slice(&fs::read(current.join(MANIFEST)).ok()?).ok()
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Off,
    NotDue,
    NoSource(Vec<String>),
    UpToDate { version: String, source: String },
    /// `check`: there is one
    Available { from: String, to: String, source: String },
    /// a build that was rolled back once – the timer leaves it
    Bad { version: String },
    Busy { to: String },
    Updated { from: String, to: String, changed: Vec<String>, restarted: Vec<String>, reboot: bool },
    RolledBack { to: String, reason: String },
    /// `boot`: nothing staged, or the root has it already
    Nothing,
    Reapplied { version: String },
}

impl Outcome {
    pub fn message(&self) -> String {
        match self {
            Outcome::Off => "updates are off (deck-set update_channel dev|stable)".into(),
            Outcome::NotDue => "not due yet (the stable channel asks every 6 h; deck-update now asks at once)".into(),
            Outcome::NoSource(errors) => format!("no update source answered:\n  {}", errors.join("\n  ")),
            Outcome::UpToDate { version, source } => format!("up to date: {version} ({source})"),
            Outcome::Available { from, to, source } => format!("{to} is available at {source} (installed: {from})"),
            Outcome::Bad { version } => format!("{version} was rolled back before – deck-update now --force tries it again"),
            Outcome::Busy { to } => format!("{to} is waiting: a show runs (a conductor on the relay) – deck-update now --force"),
            Outcome::Updated { from, to, changed, restarted, reboot } => {
                let mut m = format!("updated {from} → {to}: {} files", changed.len());
                if !restarted.is_empty() {
                    m += &format!(", restarted {}", restarted.join(" "));
                }
                if *reboot {
                    m += ", rebooting";
                }
                m
            }
            Outcome::RolledBack { to, reason } => format!("{to} rolled back: {reason}"),
            Outcome::Nothing => "nothing to re-apply".into(),
            Outcome::Reapplied { version } => format!("re-applied {version} after boot"),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Options {
    /// the timer: throttled, skips builds that were rolled back
    pub auto: bool,
    /// update during a show, try a bad build again
    pub force: bool,
    /// only say what is there
    pub check: bool,
    /// a source for this run only (`--url`)
    pub url: Option<String>,
}

pub struct Updater<'a> {
    pub paths: Paths,
    pub fetch: &'a dyn Fetch,
    pub system: &'a dyn System,
    pub lan: &'a dyn Fn() -> Vec<String>,
    /// how long restarted units get to come up and stay up
    pub wait: apply::Wait,
}

fn label(version: &str, build: u64) -> String {
    if version.is_empty() {
        format!("build {build}")
    } else {
        version.to_string()
    }
}

impl Updater<'_> {
    pub fn config(&self) -> Config {
        Config::parse(&fs::read_to_string(self.paths.deck_toml()).unwrap_or_default())
    }

    pub fn run(&self, opts: &Options, now: u64) -> Result<Outcome> {
        let mut config = self.config();
        if let Some(url) = &opts.url {
            config.url = Some(url.clone());
            if config.channel == Channel::Off {
                config.channel = Channel::Dev;
            }
        }
        if config.channel == Channel::Off {
            return Ok(Outcome::Off);
        }
        let mut state = State::load(&self.paths);
        if opts.auto && config.channel == Channel::Stable && now.saturating_sub(state.checked_at) < STABLE_EVERY {
            return Ok(Outcome::NotDue);
        }
        let key_paths = self.paths.keys();
        let keys = sign::trusted(&[&key_paths[0], &key_paths[1]])?;
        let (bases, scanned) = source::bases(&config, &state, now, !opts.auto, self.lan);
        if scanned {
            state.scanned_at = now;
        }
        let (offer, errors) = source::newest(self.fetch, &bases, &keys, config.channel);
        state.checked_at = now;
        let outcome = self.decide(offer, errors, opts, &mut state);
        let outcome = match outcome {
            Ok(o) => o,
            Err(e) => {
                state.last = format!("failed: {e:#}");
                state.save(&self.paths)?;
                return Err(e);
            }
        };
        state.last = outcome.message();
        state.save(&self.paths)?;
        Ok(outcome)
    }

    fn decide(&self, offer: Option<Offer>, errors: Vec<String>, opts: &Options, state: &mut State) -> Result<Outcome> {
        let Some(offer) = offer else {
            return Ok(Outcome::NoSource(errors));
        };
        state.source = Some(offer.base.clone());
        let m = &offer.manifest;
        let stamp = Stamp::read(&self.paths);
        let from = label(&stamp.version, stamp.build);
        let to = label(&m.version, m.build);
        if m.build <= stamp.build {
            return Ok(Outcome::UpToDate { version: from, source: offer.base.clone() });
        }
        if opts.check {
            return Ok(Outcome::Available { from, to, source: offer.base.clone() });
        }
        if opts.auto && !opts.force && state.bad_builds.contains(&m.build) {
            return Ok(Outcome::Bad { version: to });
        }
        if !opts.force && self.system.busy() {
            return Ok(Outcome::Busy { to });
        }
        let dir = self.paths.dir();
        let download = dir.join("download");
        let _ = fs::remove_dir_all(&download);
        fs::create_dir_all(&download)?;
        let tarball = download.join(&m.bundle);
        let hash = self.fetch.download(&offer.bundle_url(), &tarball, MAX_BUNDLE)?;
        if hash != m.bundle_sha256 {
            bail!("{} does not match the signed manifest", offer.bundle_url());
        }
        let outcome = self.apply(m, &tarball, &from, &to, state)?;
        if matches!(outcome, Outcome::Updated { .. }) {
            stage(&self.paths.current(), &offer.raw, &offer.signature, &tarball, &m.bundle)?;
            // a build forced in after a rollback is good now (the boot re-applies it)
            state.bad_builds.retain(|b| *b != m.build);
        }
        let _ = fs::remove_dir_all(&download);
        Ok(outcome)
    }

    fn apply(&self, m: &Manifest, tarball: &Path, from: &str, to: &str, state: &mut State) -> Result<Outcome> {
        let dir = self.paths.dir();
        let work = dir.join("work");
        let _ = fs::remove_dir_all(&work);
        bundle::unpack(tarball, m, &work)?;
        let old_stamp = fs::read(self.paths.stamp()).ok();
        let old = last_applied(&self.paths.current());
        let installed = apply::install(&self.paths.root, &work, m, &dir.join("previous"), old.as_ref())?;
        let _ = fs::remove_dir_all(&work);
        write_stamp(&self.paths, m)?;
        let acts = actions(&installed.changed, &installed.removed, m.reboot);
        if acts.reboot {
            if acts.daemon_reload {
                self.system.daemon_reload()?;
            }
            self.system.reboot()?;
            return Ok(Outcome::Updated {
                from: from.into(),
                to: to.into(),
                changed: installed.changed,
                restarted: vec![],
                reboot: true,
            });
        }
        match apply::activate(self.system, &acts, &installed.changed, self.wait) {
            Ok(()) => Ok(Outcome::Updated {
                from: from.into(),
                to: to.into(),
                changed: installed.changed,
                restarted: acts.restart,
                reboot: false,
            }),
            Err(e) => {
                apply::rollback(&self.paths.root, &installed)?;
                match old_stamp {
                    Some(b) => write_atomic(&self.paths.stamp(), &b)?,
                    None => {
                        let _ = fs::remove_file(self.paths.stamp());
                    }
                }
                if acts.daemon_reload {
                    self.system.daemon_reload()?;
                }
                let _ = apply::restart_all(self.system, &acts.restart);
                if !state.bad_builds.contains(&m.build) {
                    state.bad_builds.push(m.build);
                }
                Ok(Outcome::RolledBack { to: to.into(), reason: format!("{e:#}") })
            }
        }
    }

    /// At boot, before the deck's services: the staged bundle again when the root is older (a live stick's root
    /// resets at every boot; an installed deck has it already). Verified again – the partition is not trusted more
    /// than the network.
    pub fn boot(&self) -> Result<Outcome> {
        let current = self.paths.current();
        let Ok(raw) = fs::read(current.join(MANIFEST)) else {
            return Ok(Outcome::Nothing);
        };
        let signature = fs::read_to_string(current.join(SIGNATURE)).context("the staged signature")?;
        let key_paths = self.paths.keys();
        let keys = sign::trusted(&[&key_paths[0], &key_paths[1]])?;
        sign::verify(&keys, &raw, signature.trim()).context("the staged bundle")?;
        let m: Manifest = serde_json::from_slice(&raw)?;
        m.validate()?;
        let state = State::load(&self.paths);
        if m.build <= Stamp::read(&self.paths).build || state.bad_builds.contains(&m.build) {
            return Ok(Outcome::Nothing);
        }
        let work = self.paths.dir().join("work");
        let _ = fs::remove_dir_all(&work);
        bundle::unpack(&current.join(&m.bundle), &m, &work)?;
        // the root is older than the staged bundle (a live stick): what the bundle before it listed and this one does
        // not comes off the root as well
        let before: Option<Manifest> = fs::read(current.join(PREVIOUS)).ok().and_then(|b| serde_json::from_slice(&b).ok());
        let installed = apply::install(&self.paths.root, &work, &m, &self.paths.dir().join("previous"), before.as_ref())?;
        let _ = fs::remove_dir_all(&work);
        write_stamp(&self.paths, &m)?;
        if actions(&installed.changed, &installed.removed, false).daemon_reload {
            self.system.daemon_reload()?;
        }
        Ok(Outcome::Reapplied { version: label(&m.version, m.build) })
    }
}

fn write_stamp(paths: &Paths, m: &Manifest) -> Result<()> {
    let stamp = Stamp { version: m.version.clone(), build: m.build };
    write_atomic(&paths.stamp(), &serde_json::to_vec_pretty(&stamp)?)
}

/// The applied bundle into `current` (replacing the last one) for `boot`
fn stage(current: &Path, raw: &[u8], signature: &str, tarball: &Path, name: &str) -> Result<()> {
    let next = current.with_extension("new");
    let _ = fs::remove_dir_all(&next);
    fs::create_dir_all(&next)?;
    fs::copy(tarball, next.join(name))?;
    fs::write(next.join(SIGNATURE), signature)?;
    fs::write(next.join(MANIFEST), raw)?;
    if let Some(before) = last_applied(current) {
        fs::write(next.join(PREVIOUS), serde_json::to_vec(&before)?)?;
    }
    let _ = fs::remove_dir_all(current);
    fs::rename(&next, current)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply::tests::FakeSystem;
    use crate::bundle::{collect, pack, PackOptions};
    use crate::source::tests::FakeFetch;
    use crate::state::SERVER;
    use ed25519_dalek::SigningKey;

    struct Deck {
        _dir: tempfile::TempDir,
        paths: Paths,
        src: std::path::PathBuf,
        key: SigningKey,
    }

    /// a deck root with an old stage, the dev key trusted, and a source tree to pack
    fn deck() -> Deck {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        let paths = Paths::new(&root);
        let key = SigningKey::from_bytes(&[9; 32]);
        fs::create_dir_all(root.join("usr/bin")).unwrap();
        fs::create_dir_all(root.join("usr/share/deck")).unwrap();
        fs::create_dir_all(root.join("var/lib/deck")).unwrap();
        fs::write(root.join("usr/bin/deck"), b"stage v1").unwrap();
        fs::write(root.join("usr/share/deck/update-keys"), sign::public_hex(&key)).unwrap();
        fs::write(paths.stamp(), br#"{"version":"v1","build":100}"#).unwrap();
        let src = dir.path().join("src");
        crate::bundle::tests::fixture(&src);
        Deck { _dir: dir, paths, src, key }
    }

    impl Deck {
        /// packs build `build` and serves it at `base`
        fn release(&self, fetch: &FakeFetch, base: &str, build: u64) -> Manifest {
            let out = self.src.join(format!("out-{build}"));
            let files = collect(&self.src.join("bin"), &self.src.join("rootfs"), Some(&self.src.join("scenes"))).unwrap();
            let opts = PackOptions { version: &format!("v{build}"), build, channel: "dev", reboot: false, notes: "" };
            let m = pack(&files, &opts, &self.key, &out).unwrap();
            fetch.put(&format!("{base}/{MANIFEST}"), fs::read(out.join(MANIFEST)).unwrap());
            fetch.put(&format!("{base}/{SIGNATURE}"), fs::read(out.join(SIGNATURE)).unwrap());
            fetch.put(&format!("{base}/{}", m.bundle), fs::read(out.join(&m.bundle)).unwrap());
            m
        }

        fn updater<'a>(&self, fetch: &'a FakeFetch, sys: &'a FakeSystem, lan: &'a dyn Fn() -> Vec<String>) -> Updater<'a> {
            Updater { paths: self.paths.clone(), fetch, system: sys, lan, wait: apply::Wait::NONE }
        }
    }

    const PC: &str = "http://192.168.1.50:7790";

    #[test]
    fn a_dev_pc_on_the_lan_updates_the_deck() {
        let d = deck();
        let fetch = FakeFetch::default();
        d.release(&fetch, PC, 200);
        let sys = FakeSystem::default();
        let lan = || vec![PC.to_string()];
        let u = d.updater(&fetch, &sys, &lan);

        let check = u.run(&Options { check: true, ..Options::default() }, 1000).unwrap();
        assert_eq!(check, Outcome::Available { from: "v1".into(), to: "v200".into(), source: PC.into() });

        let out = u.run(&Options::default(), 1000).unwrap();
        let Outcome::Updated { changed, restarted, .. } = &out else { panic!("{out:?}") };
        assert!(changed.contains(&"usr/bin/deck".to_string()));
        assert_eq!(restarted, &vec!["deck-relay.service", "deck-kiosk.service"]);
        assert_eq!(fs::read(d.paths.root.join("usr/bin/deck")).unwrap(), b"stage v2");
        assert_eq!(Stamp::read(&d.paths).build, 200);
        assert!(d.paths.current().join(MANIFEST).exists(), "staged for the boot");
        assert_eq!(State::load(&d.paths).source.as_deref(), Some(PC));

        // again: nothing to do, and no scan while the PC answers
        let calls = sys.calls.borrow().len();
        let out = u.run(&Options { auto: true, ..Options::default() }, 1060).unwrap();
        assert_eq!(out, Outcome::UpToDate { version: "v200".into(), source: PC.into() });
        assert_eq!(sys.calls.borrow().len(), calls);
    }

    #[test]
    fn a_show_holds_the_update_back() {
        let d = deck();
        let fetch = FakeFetch::default();
        d.release(&fetch, PC, 200);
        let sys = FakeSystem { busy: true, ..FakeSystem::default() };
        let lan = || vec![PC.to_string()];
        let u = d.updater(&fetch, &sys, &lan);
        assert_eq!(u.run(&Options::default(), 1000).unwrap(), Outcome::Busy { to: "v200".into() });
        assert_eq!(fs::read(d.paths.root.join("usr/bin/deck")).unwrap(), b"stage v1");
        let forced = u.run(&Options { force: true, ..Options::default() }, 1000).unwrap();
        assert!(matches!(forced, Outcome::Updated { .. }), "{forced:?}");
    }

    #[test]
    fn a_stage_that_does_not_come_up_is_rolled_back_once() {
        let d = deck();
        let fetch = FakeFetch::default();
        d.release(&fetch, PC, 200);
        let sys = FakeSystem { down: vec!["deck-kiosk.service".into()], ..FakeSystem::default() };
        let lan = || vec![PC.to_string()];
        let u = d.updater(&fetch, &sys, &lan);
        let out = u.run(&Options::default(), 1000).unwrap();
        assert!(matches!(out, Outcome::RolledBack { .. }), "{out:?}");
        assert_eq!(fs::read(d.paths.root.join("usr/bin/deck")).unwrap(), b"stage v1");
        assert!(!d.paths.root.join("usr/bin/deck-relay").exists(), "a new file is removed again");
        assert_eq!(Stamp::read(&d.paths).build, 100);
        assert!(!d.paths.current().exists(), "nothing staged for the boot");
        // the timer leaves that build alone
        assert_eq!(u.run(&Options { auto: true, ..Options::default() }, 1100).unwrap(), Outcome::Bad { version: "v200".into() });

        // forced in once the stage comes up: good again, and the boot re-applies it
        let up = FakeSystem::default();
        let u = d.updater(&fetch, &up, &lan);
        assert!(matches!(u.run(&Options { force: true, ..Options::default() }, 1200).unwrap(), Outcome::Updated { .. }));
        assert!(State::load(&d.paths).bad_builds.is_empty());
        fs::write(d.paths.stamp(), br#"{"version":"v1","build":100}"#).unwrap();
        assert_eq!(u.boot().unwrap(), Outcome::Reapplied { version: "v200".into() });
    }

    #[test]
    fn untrusted_or_tampered_releases_are_refused() {
        let d = deck();
        let fetch = FakeFetch::default();
        let m = d.release(&fetch, PC, 200);
        // the tarball swapped after signing
        fetch.put(&format!("{PC}/{}", m.bundle), b"not the bundle".to_vec());
        let sys = FakeSystem::default();
        let lan = || vec![PC.to_string()];
        let u = d.updater(&fetch, &sys, &lan);
        assert!(u.run(&Options::default(), 1000).is_err());
        assert_eq!(fs::read(d.paths.root.join("usr/bin/deck")).unwrap(), b"stage v1");
        assert!(State::load(&d.paths).last.starts_with("failed"));

        // a stranger's key
        fs::write(d.paths.root.join("usr/share/deck/update-keys"), sign::public_hex(&SigningKey::from_bytes(&[1; 32]))).unwrap();
        let out = u.run(&Options::default(), 1000).unwrap();
        assert!(matches!(out, Outcome::NoSource(_)), "{out:?}");
    }

    #[test]
    fn off_stable_and_the_server() {
        let d = deck();
        let fetch = FakeFetch::default();
        let sys = FakeSystem::default();
        let lan = || vec![];
        let u = d.updater(&fetch, &sys, &lan);
        fs::write(d.paths.deck_toml(), "update_channel = \"off\"\n").unwrap();
        assert_eq!(u.run(&Options::default(), 1000).unwrap(), Outcome::Off);

        fs::write(d.paths.deck_toml(), "update_channel = \"stable\"\n").unwrap();
        let out = u.run(&Options { auto: true, ..Options::default() }, 100_000).unwrap();
        assert!(matches!(out, Outcome::NoSource(_)));
        assert_eq!(fetch.asked.lock().unwrap()[0], format!("{SERVER}/stable/{MANIFEST}"));
        assert_eq!(u.run(&Options { auto: true, ..Options::default() }, 100_100).unwrap(), Outcome::NotDue);
    }

    #[test]
    fn the_boot_puts_the_staged_bundle_back() {
        let d = deck();
        let fetch = FakeFetch::default();
        d.release(&fetch, PC, 200);
        let sys = FakeSystem::default();
        let lan = || vec![PC.to_string()];
        let u = d.updater(&fetch, &sys, &lan);
        u.run(&Options::default(), 1000).unwrap();
        assert_eq!(u.boot().unwrap(), Outcome::Nothing, "the root has it");

        // a live stick rebooted: the root is the ISO's again
        fs::write(d.paths.root.join("usr/bin/deck"), b"stage v1").unwrap();
        fs::write(d.paths.stamp(), br#"{"version":"v1","build":100}"#).unwrap();
        assert_eq!(u.boot().unwrap(), Outcome::Reapplied { version: "v200".into() });
        assert_eq!(fs::read(d.paths.root.join("usr/bin/deck")).unwrap(), b"stage v2");
    }

    /// 200 installs a scene; 300 is built without it
    fn drop_the_scene(d: &Deck, fetch: &FakeFetch) {
        fs::remove_file(d.src.join("scenes/fireflies.toml")).unwrap();
        d.release(fetch, PC, 300);
    }

    #[test]
    fn a_file_the_bundle_dropped_is_removed() {
        let d = deck();
        let fetch = FakeFetch::default();
        let sys = FakeSystem::default();
        let lan = || vec![PC.to_string()];
        d.release(&fetch, PC, 200);
        let u = d.updater(&fetch, &sys, &lan);
        u.run(&Options::default(), 1000).unwrap();
        let scene = d.paths.root.join("usr/share/deck/scenes/sandbox/fireflies.toml");
        assert!(scene.exists());

        drop_the_scene(&d, &fetch);
        let out = u.run(&Options::default(), 1100).unwrap();
        let Outcome::Updated { changed, restarted, .. } = &out else { panic!("{out:?}") };
        assert_eq!(changed, &vec!["usr/share/deck/scenes/sandbox/fireflies.toml"]);
        assert_eq!(restarted, &vec!["deck-kiosk.service"], "a removed scene restarts the kiosk");
        assert!(!scene.exists());
        assert_eq!(Stamp::read(&d.paths).build, 300);
    }

    #[test]
    fn a_removal_that_does_not_come_up_is_rolled_back() {
        let d = deck();
        let fetch = FakeFetch::default();
        let lan = || vec![PC.to_string()];
        d.release(&fetch, PC, 200);
        d.updater(&fetch, &FakeSystem::default(), &lan).run(&Options::default(), 1000).unwrap();
        let scene = d.paths.root.join("usr/share/deck/scenes/sandbox/fireflies.toml");

        drop_the_scene(&d, &fetch);
        let down = FakeSystem { down: vec!["deck-kiosk.service".into()], ..FakeSystem::default() };
        let out = d.updater(&fetch, &down, &lan).run(&Options::default(), 1100).unwrap();
        assert!(matches!(out, Outcome::RolledBack { .. }), "{out:?}");
        assert_eq!(fs::read(&scene).unwrap(), b"name = \"fireflies\"\n", "the rollback puts the scene back");
        assert_eq!(Stamp::read(&d.paths).build, 200);
    }

    #[test]
    fn the_boot_removes_what_the_bundle_before_dropped() {
        let d = deck();
        let fetch = FakeFetch::default();
        let sys = FakeSystem::default();
        let lan = || vec![PC.to_string()];
        d.release(&fetch, PC, 200);
        let u = d.updater(&fetch, &sys, &lan);
        u.run(&Options::default(), 1000).unwrap();
        drop_the_scene(&d, &fetch);
        u.run(&Options::default(), 1100).unwrap();
        let scene = d.paths.root.join("usr/share/deck/scenes/sandbox/fireflies.toml");
        assert!(!scene.exists());

        // a live stick rebooted: the root is the older one again, with the scene in it
        fs::create_dir_all(scene.parent().unwrap()).unwrap();
        fs::write(&scene, b"name = \"fireflies\"\n").unwrap();
        fs::write(d.paths.stamp(), br#"{"version":"v200","build":200}"#).unwrap();
        assert_eq!(u.boot().unwrap(), Outcome::Reapplied { version: "v300".into() });
        assert!(!scene.exists(), "the bundle before dropped it, so the boot removes it");
    }
}
