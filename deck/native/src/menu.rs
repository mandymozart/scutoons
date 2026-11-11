//! The deck's menu in the deck's window (#386; the model, its words and its picture are deck-ui `menu.rs`, built from
//! deck-kit's live components): an adapter. Here only winit's keys, mouse and touch and the pad's buttons as the kit's
//! `Input`, the deck's objects read into the menu's `Facts`, and what the menu answers done: the deck's own pages follow
//! it (F4 the settings page, F5 the switcher, F6 the Live page – drawn as before on the HUD), QUIT ends the window, the
//! picture goes to the stage. Esc opens the menu when nothing is open; F4 / F5 / F6 still open their page directly. The
//! corner card is gone: deck-link's pairing state (read every 2 s, a small file) is the menu's "Pair this deck".
use crate::App;
use deck_ui::menu::{Answer, Facts, Input, Menu, Page, Pair};
use winit::event::{ElementState, MouseButton, TouchPhase, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};

pub struct Native {
    pub menu: Menu,
    /// where the pointer is (winit says it on a move, a press has no place)
    at: (f32, f32),
    pair: Pair,
    pair_at: f32,
}

impl Native {
    /// The menu at the start: the splash waits for START only when the deck is in no session (no outlet told) – a check
    /// run (`--frames`) never waits for a key.
    pub fn new(args: &crate::Args) -> Native {
        let session = deck_native::live::config::configured(crate::outlet_settings(args)) || args.frames.is_some();
        Native { menu: Menu::new(session), at: (0.0, 0.0), pair: Pair::Off, pair_at: -10.0 }
    }
}

impl App {
    /// what the menu says about the deck now
    fn menu_facts(&self) -> Facts {
        let s = crate::pages::session(&self.show);
        let outlet = self.args.outlet.clone().unwrap_or_default();
        let scene = self.current_scene().map_or(self.show.scene_name.clone(), |p| p.rsplit(['/', '\\']).next().unwrap_or(p).trim_end_matches(".toml").to_string());
        let h = self.renderer.as_ref().map_or(1080.0, |r| r.size().1);
        Facts {
            native: true,
            live_page: true,
            scenes_page: true,
            settings_page: true,
            session: deck_ui::menu::session_line(&outlet, &s),
            who: deck_ui::live::who(&s),
            scene,
            pair: self.menu.pair.clone(),
            loading: self.loading(),
            density: h / 1080.0,
            top: 0.0,
        }
    }

    /// One input for the menu; whether it took it.
    fn menu_input(&mut self, el: &ActiveEventLoop, i: Input) -> bool {
        let facts = self.menu_facts();
        match self.menu.menu.input(i, &facts) {
            Answer::No => return false,
            Answer::Quit => el.exit(),
            Answer::Yes | Answer::Started => {}
        }
        self.menu_sync(&facts);
        true
    }

    /// The deck's own pages follow the menu: the one it has on top shows, the others close.
    fn menu_sync(&mut self, facts: &Facts) {
        let own = self.menu.menu.own(facts);
        if own == Some(Page::Scenes) {
            if !self.switcher.open {
                self.open_switcher();
            }
        } else {
            self.switcher.open = false;
        }
        self.page.open = own == Some(Page::Settings);
        if (own == Some(Page::Live)) != self.live.page.open {
            self.live.toggle(&self.show);
        }
        self.hud_at = -1.0;
    }

    /// A window event for the menu: true = it was the menu's (the deck's dialog, #481, had its turn before).
    pub(crate) fn menu_event(&mut self, el: &ActiveEventLoop, event: &WindowEvent) -> bool {
        let drawn = self.menu.menu.open() && self.menu.menu.own(&self.menu_facts()).is_none();
        let input = match event {
            WindowEvent::KeyboardInput { event: kev, .. } if kev.state == ElementState::Pressed => match kev.logical_key.as_ref() {
                Key::Named(NamedKey::Escape) if !self.menu.menu.open() => Input::Open,
                Key::Named(NamedKey::Escape) => Input::Back,
                // Shift+F4 is the governor's (main.rs)
                Key::Named(NamedKey::F4 | NamedKey::F5 | NamedKey::F6) if !self.mods.shift_key() => {
                    let page = match kev.logical_key.as_ref() {
                        Key::Named(NamedKey::F4) => Page::Settings,
                        Key::Named(NamedKey::F5) => Page::Scenes,
                        _ => Page::Live,
                    };
                    self.menu.menu.direct(page);
                    self.menu_sync(&self.menu_facts());
                    return true;
                }
                // Backspace is the Live page's while it shows (a name is typed there)
                Key::Named(NamedKey::Backspace) if drawn => Input::Back,
                Key::Named(NamedKey::ArrowUp) => Input::Up,
                Key::Named(NamedKey::ArrowDown) => Input::Down,
                Key::Named(NamedKey::ArrowLeft) => Input::Left,
                Key::Named(NamedKey::ArrowRight) => Input::Right,
                Key::Named(NamedKey::Enter) => Input::Enter,
                // a drawn page keeps the other keys, but those that pass every page (F1, F12, the volume)
                k if drawn && !crate::pages::passes(&kev.logical_key) && !(self.mods.control_key() && k == Key::Character("q")) => return true,
                _ => return false,
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.menu.at = (position.x as f32, position.y as f32);
                // the pointer's move is everybody's (the space's hand, the Live page's lines)
                self.menu_input(el, Input::Point(self.menu.at.0, self.menu.at.1));
                return false;
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => Input::Press(self.menu.at.0, self.menu.at.1),
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Right, .. } => Input::Secondary,
            WindowEvent::Touch(t) if t.phase == TouchPhase::Started => Input::Press(t.location.x as f32, t.location.y as f32),
            _ => return false,
        };
        self.menu_input(el, input)
    }

    /// Once a frame: deck-link's pairing state (every 2 s), the pad's buttons, the menu's picture to the stage (only
    /// when it changed).
    pub(crate) fn menu_frame(&mut self, el: &ActiveEventLoop, time: f32) {
        if time - self.menu.pair_at >= 2.0 || time < self.menu.pair_at {
            self.menu.pair_at = time;
            let state = deck_link::pairing::read_state(&deck_link::pairing::run_dir());
            self.menu.pair = deck_native::pairing::of(state.as_ref(), &deck_link::pairing::app_url());
        }
        #[cfg(feature = "gamepad")]
        {
            let open = self.menu.menu.open();
            let pressed = match (self.show.pad.as_mut(), self.show.space.as_mut()) {
                (Some(pad), Some(space)) => pad.poll(space, &mut self.show.controls, open),
                _ => Vec::new(),
            };
            for i in pressed {
                self.menu_input(el, i);
            }
        }
        #[cfg(not(feature = "gamepad"))]
        let _ = el;
        let facts = self.menu_facts();
        let Some(r) = self.renderer.as_mut() else { return };
        if let Some(picture) = self.menu.menu.paint(&facts, r.size()) {
            r.set_menu(picture.as_ref().map(|p| (p.w, p.h, p.rgba.as_slice())));
        }
    }
}
