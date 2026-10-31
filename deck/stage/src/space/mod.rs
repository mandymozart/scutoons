//! The 3D space (#188): deck-space's (#198 – the same code draws in the browser, deck/space-web), here with what only
//! the deck has – the relay link on a thread (link.rs, the feature `live`) that `Connect::connect` attaches.
pub use deck_space::space::*;

#[cfg(feature = "live")]
pub mod link;

/// Joining a Live session (deck-stage's side of `Space::attach`).
#[cfg(feature = "live")]
pub trait Connect {
    /// Join the session at `relay` (`ws://host:port/live/<code>`; None = the hosted relay `HOSTED_RELAY`, #199) as the
    /// space node's `join` says – a deck streams its being, the conductor gathers the decks'. `token`: the conductor's
    /// session token (a dev relay takes any). `sync`: a deck's sync, which takes the conductor's pushes (#195; None = the
    /// deck doesn't answer them).
    fn connect(&mut self, relay: Option<&str>, token: Option<String>, nick: Option<String>, sync: Option<Box<dyn link::Syncs>>);
}

#[cfg(feature = "live")]
impl Connect for Space {
    fn connect(&mut self, relay: Option<&str>, token: Option<String>, nick: Option<String>, sync: Option<Box<dyn link::Syncs>>) {
        if self.join == Join::Alone {
            return;
        }
        // deck.toml's `relay = ""` = unset
        let relay = relay.filter(|r| !r.trim().is_empty()).unwrap_or(crate::live::config::HOSTED_RELAY);
        let graph = self.wire_graph();
        let sync = sync.filter(|_| self.join == Join::Deck);
        self.attach(link::Link::start(relay, self.join, token, nick, graph, sync).map(|l| Box::new(l) as Box<dyn wire::Wire>));
    }
}
