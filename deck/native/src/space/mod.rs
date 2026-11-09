//! The 3D space (#188): deck-space's (#198 – the same code draws in the browser, deck/space-web), here with what only
//! the deck has – the relay link on a thread (link.rs, the feature `live`) that `Connect::connect` attaches.
pub use deck_space::space::*;

#[cfg(feature = "live")]
pub mod link;

/// Joining a Live session (deck-native's side of `Space::attach`).
#[cfg(feature = "live")]
pub trait Connect {
    /// Join the session at `relay` (`ws://host:port/live/<code>`; None = the hosted relay `HOSTED_RELAY`, #199) as the
    /// space node's `join` says – a deck streams its being, the conductor gathers the decks'. `token`: the conductor's
    /// session token (a dev relay takes any). `sync`: a deck's sync, which takes the conductor's pushes (#195; None = the
    /// deck doesn't answer them). `show`: the tempo clock's side of the link (#321). `far`: the engine's far world nodes
    /// and event outputs (#442, `far_table`; None = no engine).
    fn connect(
        &mut self,
        relay: Option<&str>,
        token: Option<String>,
        nick: Option<String>,
        sync: Option<Box<dyn link::Syncs>>,
        show: Option<std::sync::Arc<crate::tempo::relay::ShowLink>>,
        far: Option<deck_space::far::FarTable>,
    );
}

#[cfg(feature = "live")]
impl Connect for Space {
    fn connect(
        &mut self,
        relay: Option<&str>,
        token: Option<String>,
        nick: Option<String>,
        sync: Option<Box<dyn link::Syncs>>,
        show: Option<std::sync::Arc<crate::tempo::relay::ShowLink>>,
        far: Option<deck_space::far::FarTable>,
    ) {
        if self.join == Join::Alone {
            return;
        }
        // deck.toml's `relay = ""` = unset
        let relay = relay.filter(|r| !r.trim().is_empty()).unwrap_or(crate::live::config::HOSTED_RELAY);
        let graph = self.wire_graph();
        let sync = sync.filter(|_| self.join == Join::Deck);
        self.attach(link::Link::start(relay, self.join, token, nick, graph, sync, show).map(|l| Box::new(l) as Box<dyn wire::Wire>));
        if let Some(t) = far {
            self.wire_far(t);
        }
    }
}

/// #442: the engine's side of far events for the link (deck-space far.rs): the far world nodes' rings and the event
/// outputs' rings out of `handles`, this deck's name from deck.toml, the rig it plays (`subject`). None when the engine
/// has neither.
#[cfg(feature = "live")]
pub fn far_table(handles: &mut crate::audio::Handles, subject: &str) -> Option<deck_space::far::FarTable> {
    use deck_space::far::{FarInlet, FarOutlet, FarTable};
    let tempo = handles.tempo.clone();
    let events = &mut handles.core_mut().events;
    let far = events.take_far();
    let outlets = events.take_outbox_slots();
    if far.is_empty() && outlets.is_empty() {
        return None;
    }
    let name = crate::live::config::Settings::read(&crate::live::config::deck_toml()).name;
    if name.is_none() && !outlets.is_empty() {
        tracing::info!("far events: deck.toml names this deck not (`name = …`), so no other deck can address its outputs");
    }
    let inlets = far
        .into_iter()
        .map(|(address, listen, mut tx)| FarInlet { address, listen, push: Box::new(move |e| tx.push(e).is_ok()) })
        .collect();
    let outlets = outlets.into_iter().map(|(slot, mut rx)| FarOutlet { slot, pop: Box::new(move || rx.pop().ok()) }).collect();
    Some(FarTable { name, subject: subject.to_string(), names: events.names.clone(), inlets, outlets, tempo: Some(tempo) })
}
