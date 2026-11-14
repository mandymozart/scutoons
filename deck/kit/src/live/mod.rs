//! The kit's `live` components (RULES H8, owner 2026-11-11 [ui-kit]): the deck UI the audience and the person at a deck
//! see, in its own style (`tokens.rs`). #386: the HUD's arcade look the deck's menu is built from (`hud.rs`); #481: the
//! reader's dark panel the deck's dialogs and toasts are built from (`panel.rs`). Both reach the screen as layers of the
//! base's egui screen (`base::screen`), the menu under the panel (`MENU`, `PANEL`); #556: under both the ballot of a
//! vote the deck may cast in (`VOTE`, deck-ui `ballot.rs`, built from the panel's parts).
pub mod hud;
pub mod panel;
pub mod tokens;

/// the deck's screen's layers, bottom up: the ballot (#556), the menu (#386), the panel (#481)
pub const VOTE: usize = 0;
pub const MENU: usize = 1;
pub const PANEL: usize = 2;
pub const LAYERS: usize = 3;
