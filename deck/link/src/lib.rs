//! deck-link – a deck's identity and access to scutoons.com (CYBERDECKS.md §9, #180): handshake, then approval, then
//! data. `Identity` (the key pair at first boot), `Link` (request → poll → token renewal over a `Transport`),
//! `HttpTransport` (ureq). The stage's splash shows `Pending`'s code + the QR of `pairing_url`; its library client sends
//! `Ready`'s token – both wired in deck-stage later.

pub mod http;
pub mod identity;
pub mod link;

pub use http::{HttpTransport, DEFAULT_API, DEFAULT_APP};
pub use identity::{Identity, DEFAULT_DIR};
pub use link::{format_code, pairing_url, DeckInfo, Link, Reply, State, Transport, RENEW_AFTER};
