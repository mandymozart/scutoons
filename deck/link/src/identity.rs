//! The deck's identity (CYBERDECKS.md §9.1): an Ed25519 key pair made at first boot and kept on the persistence
//! partition. The private key never leaves the deck – no shared secret in the image (anything in the ISO is public).
//! On disk: `<dir>/ed25519.key`, the 32-byte seed, mode 0600 in a 0700 directory, written atomically (a temp file,
//! then a rename) so a power cut never leaves half a key. A live stick without persistence makes a new identity each
//! boot – it simply asks again.

use anyhow::{bail, Context, Result};
use ed25519_dalek::{Signer, SigningKey};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Where the identity lives on a deck (the persistence partition mounts at /var/lib/deck – deck/os tmpfiles.d)
pub const DEFAULT_DIR: &str = "/var/lib/deck/identity";
const KEY_FILE: &str = "ed25519.key";

pub struct Identity {
    key: SigningKey,
}

impl Identity {
    /// The identity in `dir`, made there first when there is none (first boot)
    pub fn load_or_create(dir: &Path) -> Result<Self> {
        let path = dir.join(KEY_FILE);
        if path.exists() {
            return Self::load(&path);
        }
        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed)
            .map_err(|e| anyhow::anyhow!("no randomness for the deck key: {e}"))?;
        write_private(dir, &path, &seed)?;
        Ok(Self {
            key: SigningKey::from_bytes(&seed),
        })
    }

    fn load(path: &Path) -> Result<Self> {
        let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let Ok(seed) = <[u8; 32]>::try_from(bytes.as_slice()) else {
            bail!(
                "{} is not a deck key (32 bytes expected, {} found)",
                path.display(),
                bytes.len()
            );
        };
        Ok(Self {
            key: SigningKey::from_bytes(&seed),
        })
    }

    /// An identity from a seed (tests)
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            key: SigningKey::from_bytes(&seed),
        }
    }

    /// The public key as the server wants it: 32 bytes, lower-case hex
    pub fn public_key_hex(&self) -> String {
        hex(self.key.verifying_key().as_bytes())
    }

    /// The signature of a server challenge (64 bytes, hex) over the domain-separated message
    pub fn sign_challenge(&self, challenge: &str) -> String {
        hex(&self
            .key
            .sign(signed_message(challenge, &self.public_key_hex()).as_bytes())
            .to_bytes())
    }
}

/// What the deck signs (server Decks::SIGNED_PREFIX): bound to this protocol, the challenge and the deck's key
pub fn signed_message(challenge: &str, public_key_hex: &str) -> String {
    format!("scutoons-deck-token:v1:{challenge}:{public_key_hex}")
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn write_private(dir: &Path, path: &Path, seed: &[u8; 32]) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    restrict(dir, 0o700)?;
    let tmp: PathBuf = dir.join(format!("{KEY_FILE}.tmp"));
    {
        let mut file = open_private(&tmp)?;
        file.write_all(seed)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path).with_context(|| format!("saving {}", path.display()))?;
    Ok(())
}

#[cfg(unix)]
fn open_private(path: &Path) -> Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    Ok(fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?)
}

#[cfg(not(unix))]
fn open_private(path: &Path) -> Result<fs::File> {
    Ok(fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?)
}

#[cfg(unix)]
fn restrict(dir: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(dir, fs::Permissions::from_mode(mode))
        .with_context(|| format!("chmod {}", dir.display()))
}

#[cfg(not(unix))]
fn restrict(_dir: &Path, _mode: u32) -> Result<()> {
    Ok(())
}
