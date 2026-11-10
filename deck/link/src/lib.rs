//! deck-link – a deck's identity and access to scutoons.com (CYBERDECKS.md §9, #180): handshake, then approval, then
//! data. With the `net` feature (the default): `Identity` (the key pair at first boot), `Link` (request → poll → token
//! renewal over a `Transport`), `HttpTransport` (ureq) – the daemon `deck-link run` (deck-link.service, root). With
//! `handshake` (in `net`; alone for the browser deck, #470): `Handshake`, the protocol's decisions as a sans-IO machine –
//! no HTTP, no files, no key (the caller signs) – and `Session`, its schedule and token over time. Always:
//! `pairing` – the daemon's files the rest of the deck reads (the code for the splash, the token for the stage and
//! `deck-tls`), `format_code`, `pairing_url`.

#[cfg(feature = "handshake")]
pub mod handshake;
#[cfg(feature = "net")]
pub mod http;
#[cfg(feature = "handshake")]
pub mod session;
#[cfg(feature = "net")]
pub mod identity;
#[cfg(feature = "net")]
pub mod link;
pub mod pairing;

#[cfg(feature = "net")]
pub use http::{HttpTransport, DEFAULT_API, DEFAULT_APP};
#[cfg(feature = "net")]
pub use identity::{Identity, DEFAULT_DIR};
#[cfg(feature = "net")]
pub use link::{Link, Reply, Transport, RENEW_AFTER};
#[cfg(feature = "handshake")]
pub use handshake::{Ask, DeckInfo, Handshake, State, Step};
#[cfg(feature = "handshake")]
pub use session::Session;
pub use pairing::{format_code, pairing_url, Shown, TokenSource};
