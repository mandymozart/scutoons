//! The Spores widget (#763, agents/research/redesign/NETWORK.md): one weave of the spore network – spores, the threads
//! woven between them, the canvas's groups and notes – painted and touched the same way in the deck editor and, through
//! wasm, in the creator's Spores tab. The painting (`paint`, `card`) moved here from the editor's Show node view; a host
//! hands the widget its state through `Seen` and maps what was pressed (`Hit`) to its own documents.
pub mod card;
pub mod focus;
pub mod input;
pub mod paint;
pub mod pick;
pub mod state;
pub mod view;
pub mod weave;

pub use state::{At, Motion, State, FLASH};

/// how long a reveal fades in and out, secs
pub const FADE_IN: f32 = 0.35;
pub const FADE_OUT: f32 = 1.1;

/// a press in the widget
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    /// the head: the direction, Fit (frames the selection, else everything)
    Flow(deck_kit::base::graph::Direction),
    Fit,
    /// the empty canvas: the selection goes
    Canvas,
    /// a spore drawn (its index in the graph shown), alone or added
    Node(usize),
    NodeAlso(usize),
    /// a spore's fold
    Fold(usize),
    /// the head's buttons: the selection grouped, a note added
    Group,
    Note,
    /// a group's label, swatch and fold (its index in the canvas)
    GroupName(usize),
    GroupColour(usize),
    GroupFold(usize),
    /// a note (its index in the canvas)
    NoteText(usize),
    /// a port of a spore drawn: the spore, whether an outlet, which of them
    Pin(usize, bool, usize),
}

/// what the painters read of the widget's state
pub trait Seen {
    /// selected, by id
    fn selected(&self, id: &str) -> bool;
    /// a focused spore's flash, 0 → 1 while it runs
    fn flashing(&self, id: &str) -> Option<f32>;
    /// pictures in the cards (a switch: off draws none)
    fn previews(&self) -> bool;
    /// the texture of a picture by its key, once loaded
    fn picture(&self, key: &str) -> Option<u64>;
    /// the host's clock, secs
    fn secs(&self) -> f32;
    /// the pointer as the latent grid follows it (canvas units)
    fn blob(&self) -> Option<(f32, f32)>;
}
