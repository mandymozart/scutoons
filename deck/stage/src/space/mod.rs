//! The 3D space (#188): deck-space's (#198 – the same code draws in the browser, deck/space-web), here with what only
//! the deck has – the relay link on a thread (link.rs, the feature `live`) that `Connect::connect` attaches.
pub use deck_space::space::*;

#[cfg(feature = "live")]
pub mod link;

/// Joining a Live session (deck-stage's side of `Space::attach`).
#[cfg(feature = "live")]
pub trait Connect {
    /// Join the session at `relay` (`ws://host:port/live/<code>`) as the space node's `join` says – a deck streams its
    /// being, the conductor gathers the decks'. `token`: the conductor's session token (a dev relay takes any).
    fn connect(&mut self, relay: Option<&str>, token: Option<String>, nick: Option<String>);
}

#[cfg(feature = "live")]
impl Connect for Space {
    fn connect(&mut self, relay: Option<&str>, token: Option<String>, nick: Option<String>) {
        if self.join == Join::Alone {
            return;
        }
        let Some(relay) = relay else {
            self.attach(Err("no relay set (--relay)".into()));
            return;
        };
        let graph = self.wire_graph();
        self.attach(link::Link::start(relay, self.join, token, nick, graph).map(|l| Box::new(l) as Box<dyn wire::Wire>));
    }
}
