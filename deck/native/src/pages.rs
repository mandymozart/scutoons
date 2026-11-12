//! The HUD's pages in the deck's window (#250 F5 the scene switcher, #220 F4 the settings page, #478 F6 the Live page): an
//! open page takes the keys – winit's key named as the page's own here; what a key does is the page's (switcher.rs,
//! quality.rs, deck-ui live.rs). The Live page also takes the pointer: a press on one of its HUD lines (deck-space
//! `hud::line_at`). Its session and what it asks for are the Live outlet's (live/mod.rs).
use deck_native::quality as settings;
use deck_native::switcher;
use deck_ui::live;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::keyboard::{Key, NamedKey};

/// What a key is to an open page.
pub enum Press<K> {
    Key(K),
    Close,
    None,
}

/// the keys an open page lets through: F1 (the HUD), F12 (the web), the pages' own F-keys, the volume
pub fn passes(k: &Key) -> bool {
    matches!(k.as_ref(), Key::Named(NamedKey::F1 | NamedKey::F12 | NamedKey::F4 | NamedKey::F5 | NamedKey::F6 | NamedKey::AudioVolumeUp | NamedKey::AudioVolumeDown | NamedKey::AudioVolumeMute))
}

pub fn switcher_key(k: &Key) -> Press<switcher::Key> {
    match k.as_ref() {
        Key::Named(NamedKey::ArrowUp) => Press::Key(switcher::Key::Up),
        Key::Named(NamedKey::ArrowDown) => Press::Key(switcher::Key::Down),
        Key::Named(NamedKey::Enter) => Press::Key(switcher::Key::Enter),
        // #427: "set as start" – apart from Enter's load
        Key::Character("s" | "S") => Press::Key(switcher::Key::Start),
        Key::Named(NamedKey::Escape) => Press::Close,
        _ => Press::None,
    }
}

pub fn settings_key(k: &Key) -> Press<settings::Key> {
    Press::Key(match k.as_ref() {
        Key::Named(NamedKey::ArrowUp) => settings::Key::Up,
        Key::Named(NamedKey::ArrowDown) => settings::Key::Down,
        Key::Named(NamedKey::ArrowLeft) => settings::Key::Left,
        Key::Named(NamedKey::ArrowRight) => settings::Key::Right,
        Key::Named(NamedKey::Enter | NamedKey::Space) => settings::Key::Toggle,
        Key::Character("+" | "=") => settings::Key::More,
        Key::Character("-") => settings::Key::Less,
        Key::Character("p" | "P") => settings::Key::Pixels,
        Key::Character("r" | "R") => settings::Key::Reset,
        Key::Named(NamedKey::Escape) => return Press::Close,
        _ => return Press::None,
    })
}

fn live_key(k: &Key) -> Press<live::Key> {
    Press::Key(match k.as_ref() {
        Key::Named(NamedKey::ArrowUp) => live::Key::Up,
        Key::Named(NamedKey::ArrowDown) => live::Key::Down,
        Key::Named(NamedKey::Enter) => live::Key::Enter,
        Key::Named(NamedKey::Backspace) => live::Key::Back,
        Key::Named(NamedKey::Space) => live::Key::Char(' '),
        Key::Character(s) => match s.chars().next() {
            Some(c) => live::Key::Char(c),
            None => return Press::None,
        },
        Key::Named(NamedKey::Escape) => return Press::Close,
        _ => return Press::None,
    })
}

/// #478: the Live page (F6) and where the pointer is
#[derive(Default)]
pub struct LivePage {
    pub page: live::Page,
    at: (f32, f32),
}

/// the Live outlet's session, or none (not in a session)
pub fn session(show: &crate::Show) -> live::Session {
    #[cfg(feature = "live")]
    if let Some(l) = &show.live {
        return l.session();
    }
    let _ = show;
    live::Session::default()
}

impl LivePage {
    /// F6: open (on the role it is in) or close
    pub fn toggle(&mut self, show: &crate::Show) {
        match self.page.open {
            true => self.page.open = false,
            false => self.page.open(&session(show)),
        }
    }

    pub fn lines(&self, show: &crate::Show) -> (Vec<String>, usize) {
        self.page.lines(&session(show))
    }

    /// An event while the page is open: a key or a press on its lines (`size`: the window's pixels). Whether it took it.
    pub fn event(&mut self, show: &crate::Show, e: &WindowEvent, size: (f32, f32)) -> bool {
        let s = session(show);
        let did = match e {
            WindowEvent::KeyboardInput { event: k, .. } if k.state == ElementState::Pressed && !passes(&k.logical_key) => match live_key(&k.logical_key) {
                Press::Key(key) => self.page.key(key, &s),
                Press::Close => {
                    self.page.open = false;
                    None
                }
                Press::None => None,
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.at = (position.x as f32, position.y as f32);
                return false;
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                let Some(line) = deck_space::hud::line_at(self.at.0, self.at.1, size.0, size.1) else { return false };
                self.page.tap(line, &s)
            }
            _ => return false,
        };
        #[cfg(feature = "live")]
        if let (Some(l), Some(d)) = (&show.live, did) {
            l.act(d);
        }
        #[cfg(not(feature = "live"))]
        let _ = did;
        true
    }
}
