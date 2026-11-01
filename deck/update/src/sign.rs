//! Ed25519: the dev PC's (or the release) key signs the manifest, the deck verifies against the keys it trusts.

use crate::manifest::{hex, unhex};
use anyhow::{anyhow, bail, Context, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use std::fs;
use std::path::Path;

/// A new key pair in `private_path` (the 32-byte seed as hex, 0600 where that exists); returns the public key's hex.
/// An existing key is kept – replacing it would lock every deck out.
pub fn keygen(private_path: &Path) -> Result<String> {
    if private_path.exists() {
        return Ok(public_hex(&load_signing(private_path)?));
    }
    let mut seed = [0u8; 32];
    getrandom::getrandom(&mut seed).map_err(|e| anyhow!("no randomness: {e}"))?;
    if let Some(dir) = private_path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(private_path, format!("{}\n", hex(&seed)))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(private_path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(public_hex(&SigningKey::from_bytes(&seed)))
}

pub fn public_hex(key: &SigningKey) -> String {
    hex(key.verifying_key().as_bytes())
}

pub fn load_signing(path: &Path) -> Result<SigningKey> {
    let text = fs::read_to_string(path).with_context(|| format!("reading the update key {}", path.display()))?;
    let seed: [u8; 32] = unhex(&text)?
        .try_into()
        .map_err(|_| anyhow!("{} is not an update key (32 bytes as hex)", path.display()))?;
    Ok(SigningKey::from_bytes(&seed))
}

pub fn sign(key: &SigningKey, bytes: &[u8]) -> String {
    hex(&key.sign(bytes).to_bytes())
}

/// One key per line, `#` comments and blank lines ignored; a line that is no key is an error (never skipped – a typo
/// would otherwise look trusted)
pub fn parse_keys(text: &str) -> Result<Vec<VerifyingKey>> {
    let mut keys = Vec::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if !line.is_empty() {
            keys.push(parse_key(line)?);
        }
    }
    Ok(keys)
}

pub fn parse_key(text: &str) -> Result<VerifyingKey> {
    let bytes: [u8; 32] =
        unhex(text)?.try_into().map_err(|_| anyhow!("a public key is 32 bytes as hex (64 characters)"))?;
    VerifyingKey::from_bytes(&bytes).map_err(|e| anyhow!("not a valid public key: {e}"))
}

/// The keys of every file in `paths` that exists (the image's list and the deck's own)
pub fn trusted(paths: &[&Path]) -> Result<Vec<VerifyingKey>> {
    let mut keys = Vec::new();
    for p in paths {
        match fs::read_to_string(p) {
            Ok(text) => keys.extend(parse_keys(&text).with_context(|| p.display().to_string())?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e).with_context(|| p.display().to_string()),
        }
    }
    Ok(keys)
}

pub fn verify(keys: &[VerifyingKey], bytes: &[u8], signature_hex: &str) -> Result<()> {
    if keys.is_empty() {
        bail!("this deck trusts no update key (deck-update trust <hex>)");
    }
    let sig: [u8; 64] = unhex(signature_hex)?.try_into().map_err(|_| anyhow!("a signature is 64 bytes as hex"))?;
    let sig = Signature::from_bytes(&sig);
    if keys.iter().any(|k| k.verify(bytes, &sig).is_ok()) {
        Ok(())
    } else {
        bail!("the signature matches none of the trusted update keys")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(n: u8) -> SigningKey {
        SigningKey::from_bytes(&[n; 32])
    }

    #[test]
    fn a_signature_verifies_only_for_a_trusted_key() {
        let sig = sign(&key(1), b"manifest");
        let trusted = vec![key(2).verifying_key(), key(1).verifying_key()];
        assert!(verify(&trusted, b"manifest", &sig).is_ok());
        assert!(verify(&trusted, b"manifest!", &sig).is_err(), "other bytes");
        assert!(verify(&[key(2).verifying_key()], b"manifest", &sig).is_err(), "other key");
        assert!(verify(&[], b"manifest", &sig).is_err(), "no keys trusted");
        assert!(verify(&trusted, b"manifest", "00").is_err(), "short signature");
    }

    #[test]
    fn key_files_parse_strictly() {
        let a = public_hex(&key(1));
        assert_eq!(parse_keys(&format!("# dev\n{a}  # the dev PC\n\n")).unwrap().len(), 1);
        assert!(parse_keys("not-a-key").is_err());
        assert!(parse_keys("abcd").is_err());
    }

    #[test]
    fn keygen_keeps_an_existing_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/update.key");
        let first = keygen(&path).unwrap();
        assert_eq!(first.len(), 64);
        assert_eq!(keygen(&path).unwrap(), first, "the same key again");
        assert_eq!(public_hex(&load_signing(&path).unwrap()), first);
    }

    #[test]
    fn trusted_reads_what_exists() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("keys");
        fs::write(&file, format!("{}\n", public_hex(&key(3)))).unwrap();
        assert_eq!(trusted(&[&dir.path().join("none"), &file]).unwrap().len(), 1);
    }
}
