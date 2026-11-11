//! The kit's `live` components (RULES H8, owner 2026-11-11 [ui-kit]): the deck UI the audience and the person at a deck
//! see, in its own style (`tokens.rs`). #386: the HUD's arcade look the deck's menu is built from (`hud.rs`); #481: the
//! reader's dark panel the deck's dialogs and toasts are built from (`panel.rs`). Both reach the screen as layers of the
//! base's egui screen (`base::screen`), the menu under the panel (`MENU`, `PANEL`).
pub mod hud;
pub mod panel;
pub mod tokens;

/// the deck's screen's layers, bottom up: the menu (#386), the panel (#481)
pub const MENU: usize = 0;
pub const PANEL: usize = 1;
pub const LAYERS: usize = 2;
