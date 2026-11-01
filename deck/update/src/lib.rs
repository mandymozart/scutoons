//! Remote updates for a deck (#237). A release is a *bundle*: `manifest.json` (version, build number, channel, every
//! file's SHA-256 and mode, what the bundle's tarball hashes to), `manifest.json.sig` (Ed25519 over the manifest's exact
//! bytes) and the tarball. The deck trusts the public keys in `/usr/share/deck/update-keys` (the image) and
//! `/var/lib/deck/update-keys` (`deck-update trust <hex>`); anything else is refused. Only files under the deck's own
//! paths ([`manifest::allowed`]) can ever be written – the base system (kernel, packages) is not an update's business.

pub mod apply;
pub mod bundle;
pub mod manifest;
pub mod serve;
pub mod sign;
pub mod source;
pub mod state;
pub mod update;
