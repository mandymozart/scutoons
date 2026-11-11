//! The deck's menu in the browser deck (#386; the model, its words and its picture are deck-ui `menu.rs`, built from
//! deck-kit's live components – the native deck shows the same): an adapter. The page hands its keys (`menu_key`), the
//! pointer (`menu_pointer`) and the status line's press (`menu_open`); here they become the kit's `Input`, the view's
//! objects become the menu's `Facts`, and the Live page (#478, the native deck's F6) follows the menu. In the browser's
//! full screen Esc is the browser's: M, the status line or Start open the menu (owner 2026-11-11, point 2). The browser
//! deck has no switcher and no settings page yet: their rows say LATER and open empty.
use super::{live_do, SceneView, SPLASH_AT_MOST};
use deck_ui::menu::{Answer, Facts, Input, Page, Pair};
use wasm_bindgen::prelude::*;

impl SceneView {
    /// what the menu says about this deck now
    pub(super) fn menu_facts(&self) -> Facts {
        let s = self.outlet.as_ref().map(|o| (o.name.clone(), o.relay.live()));
        Facts {
            native: false,
            live_page: s.is_some(),
            session: s.as_ref().and_then(|(name, s)| deck_ui::menu::session_line(name, s)),
            who: s.as_ref().map(|(_, s)| deck_ui::live::who(s)).unwrap_or_default(),
            pair: self.pair.clone(),
            loading: !self.show.stage.slides_ready() && self.age < SPLASH_AT_MOST,
            density: self.live_at.0,
            top: self.live_at.1,
            ..Facts::default()
        }
    }

    /// one input for the menu, the Live page following it; whether the menu took it
    fn menu_input(&mut self, i: Input) -> bool {
        self.show.stage.ui_input(i);
        let facts = self.menu_facts();
        if self.menu.input(i, &facts) == Answer::No {
            return false;
        }
        let live = self.menu.own(&facts) == Some(Page::Live);
        match (live, self.outlet.as_ref()) {
            (true, Some(o)) if !self.live.open => self.live.open(&o.relay.live()),
            (false, _) => self.live.open = false,
            _ => {}
        }
        true
    }

    /// the menu's page to the stage, when it changed (a layer of deck-kit's egui screen, #485)
    pub(super) fn menu_paint(&mut self) {
        let facts = self.menu_facts();
        if let Some(layer) = self.menu.paint(&facts, (self.config.width as f32, self.config.height as f32)) {
            self.show.stage.set_menu(layer);
        }
    }

    /// #470 #484: the deck's link as the menu's pairing – signed in (the deck is its user), waiting with a code, paired
    /// (a token, no code), or why it cannot pair
    pub(super) fn set_pair(&mut self, why: &str, code: &str, app: &str, signed: bool) {
        if !signed && !code.is_empty() && code == self.pairing_code {
            return;
        }
        self.pairing_code = code.to_owned();
        self.pair = match (signed, code.is_empty(), why.is_empty()) {
            (true, ..) => Pair::SignedIn,
            (false, false, _) => deck_space::pairing::waiting(code, app),
            (false, true, true) => Pair::Paired(String::new()),
            (false, true, false) => Pair::Offline(why.to_owned()),
        };
    }
}

#[wasm_bindgen]
impl SceneView {
    /// #386: the status line was pressed (or the page's own way to the menu): the menu opens – or closes, or the splash
    /// passes
    pub fn menu_open(&mut self) {
        self.menu_input(Input::Open);
    }

    /// #386: a key of the page (the DOM's `key`); whether it was the menu's or its Live page's. A dialog of the flash side
    /// (#481) has the keys while it shows.
    pub fn menu_key(&mut self, key: &str) -> bool {
        if self.dialog() {
            return false;
        }
        let facts = self.menu_facts();
        let own = self.menu.own(&facts).is_some();
        let input = match key {
            // on the Live page an m is a letter of the name
            "m" | "M" if !own => Some(Input::Open),
            "Escape" if self.menu.open() => Some(Input::Back),
            "Backspace" if !own => Some(Input::Back),
            "ArrowUp" if !own => Some(Input::Up),
            "ArrowDown" if !own => Some(Input::Down),
            "ArrowLeft" => Some(Input::Left),
            "ArrowRight" => Some(Input::Right),
            "Enter" if !own => Some(Input::Enter),
            _ => None,
        };
        if let Some(i) = input {
            if self.menu_input(i) {
                return true;
            }
        }
        if !own {
            // a drawn page keeps the keys
            return self.menu.open();
        }
        self.live_key(key)
    }

    /// #386: the pointer at (x, y) of the canvas, in its pixels: `press` 0 = it moved (hovering chooses), 1 = a press
    /// (a click, a tap), 2 = the secondary press (back). Whether it was the menu's or its Live page's.
    pub fn menu_pointer(&mut self, x: f32, y: f32, press: u8) -> bool {
        // where the deck's dialog or toast is (#481), a press is its own
        if press > 0 && self.face.top().is_some_and(|top| y >= top) {
            return false;
        }
        let facts = self.menu_facts();
        if press == 1 && self.menu.own(&facts) == Some(Page::Live) {
            return self.live_tap(x, y);
        }
        self.menu_input(match press {
            0 => Input::Point(x, y),
            1 => Input::Press(x, y),
            _ => Input::Secondary,
        }) && press > 0
    }
}

impl SceneView {
    /// #478: a key on the open Live page (ArrowUp, ArrowDown, Enter, Backspace, a character)
    fn live_key(&mut self, key: &str) -> bool {
        use deck_ui::live::Key;
        let k = match key {
            _ if !self.live.open => return false,
            "ArrowUp" => Key::Up,
            "ArrowDown" => Key::Down,
            "Enter" => Key::Enter,
            "Backspace" => Key::Back,
            k if k.chars().count() == 1 => Key::Char(k.chars().next().unwrap_or(' ')),
            _ => return false,
        };
        let Some(o) = self.outlet.as_mut() else { return false };
        let did = self.live.key(k, &o.relay.live());
        live_do(o, did);
        true
    }

    /// #478: a press on the open Live page at (x, y) device pixels: a line of it is taken as Enter on it
    fn live_tap(&mut self, x: f32, y: f32) -> bool {
        let (at, s) = self.live_rect();
        let (Some(o), Some(line)) = (self.outlet.as_mut(), deck_space::hud::line_in(at, s, x, y).filter(|_| self.live.open)) else { return false };
        let did = self.live.tap(line, &o.relay.live());
        live_do(o, did);
        true
    }
}
