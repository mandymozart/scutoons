//! The deck's menu in the deck's window (#386; the model, its words and its picture are deck-ui `menu.rs`, built from
//! deck-kit's live components): an adapter. Here only winit's keys, mouse and touch and the pad's buttons as the kit's
//! `Input`, the deck's objects read into the menu's `Facts`, and what the menu answers done: the deck's own pages follow
//! it (F4 the settings page, F5 the switcher, F6 the Live page – drawn as before on the HUD), QUIT ends the window, the
//! page goes to the stage (a layer of deck-kit's egui screen, #485). Esc opens the menu when nothing is open; F4 / F5 /
//! F6 still open their page directly. The corner card is gone: deck-link's pairing state (read every 2 s, a small file)
//! is the menu's "Pair this deck".
//! #556: the ballot of a vote this deck may cast in (deck-ui `ballot.rs`, the layer under the menu) gets the inputs the
//! menu leaves – the arrows, Enter, a click, a touch, the pad's d-pad and A – while no page of the deck is open; its
//! cast goes to the Live outlet's relay client.
//! JOIN A SHOW (deck-ui `join.rs`): the code page asks, `join_session` reloads the deck on the outlet scene of that session
//! (the configured relay or the hosted one, the deck's role), `start_frame` hands the relay client's answer to the page.
//! the deck's start (the rules are deck-ui `start.rs`): what the deck hears – the conductor's push or cell, the
//! relay's clock, a MIDI clock in – and whether its conductor is in, read every frame; the menu follows the stand. Play's
//! list: the API's listing of the published scenes (a thread, once) and the copies this deck keeps.
use crate::App;
use deck_ui::join::Typed;
use deck_ui::menu::{Answer, Facts, Input, Menu, Page, Pair};
use deck_ui::start::{self, Begin, Clock, Heard, Listed, Playable};
use std::sync::{Arc, Mutex};
use winit::event::{ElementState, MouseButton, TouchPhase, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};

pub struct Native {
    pub menu: Menu,
    /// #556: the ballot's picture and keys
    pub ballot: deck_ui::ballot::Page,
    /// #556, a check run (`DECK_BALLOT_KEYS=down,enter`, as `--pick` is a check run's click): the keys the ballot gets
    /// once it shows, one every 0.4 s, through the menu's input as a keyboard's; and when the next is due
    keys: std::collections::VecDeque<Input>,
    key_at: Option<f32>,
    /// a check run's keys for the menu (`DECK_MENU_KEYS=down,enter,type:A7K2`), one every 0.4 s from the first frame
    script: std::collections::VecDeque<Result<Input, char>>,
    script_at: f32,
    /// where the pointer is (winit says it on a move, a press has no place)
    at: (f32, f32),
    pair: Pair,
    pair_at: f32,
    /// how the deck came up, whether the person at it left its session (LEAVE), the API's listing (filled by a
    /// thread), what Play lists now
    begin: Begin,
    left: bool,
    listing: Arc<Mutex<Vec<Listed>>>,
    play: Vec<Playable>,
    /// JOIN A SHOW under way: the relay, outlet and scene the deck had before (given back when the join fails)
    before: Option<(Option<String>, Option<String>, Option<std::path::PathBuf>)>,
}

impl Native {
    /// The menu at the start: nothing open – the first frame's stand opens the start menu of an independent deck;
    /// a check run (`--frames`) counts as a named scene unless `DECK_START` is set.
    pub fn new(args: &crate::Args) -> Native {
        use deck_native::live::config::{deck_toml, Settings};
        let settings = Settings::read(&deck_toml()).merged(crate::outlet_settings(args));
        let session = settings.config().and_then(Result::ok).map(|c| c.code);
        let check = args.frames.is_some() && std::env::var_os("DECK_START").is_none();
        let begin = Begin::of(session.as_deref(), args.scene.is_some() || check);
        let listing = Arc::new(Mutex::new(Vec::new()));
        if !check {
            fetch_listing(args.api.clone(), listing.clone());
        }
        let keys = std::env::var("DECK_BALLOT_KEYS").unwrap_or_default();
        let keys = keys.split(',').filter_map(|k| match k.trim() {
            "up" => Some(Input::Up),
            "down" => Some(Input::Down),
            "enter" => Some(Input::Enter),
            _ => None,
        });
        Native {
            menu: Menu::default(),
            ballot: Default::default(),
            keys: keys.collect(),
            key_at: None,
            script: menu_script(&std::env::var("DECK_MENU_KEYS").unwrap_or_default()),
            script_at: 1.0,
            at: (0.0, 0.0),
            pair: Pair::Off,
            pair_at: -10.0,
            begin,
            left: false,
            listing,
            play: Vec::new(),
            before: None,
        }
    }
}

/// a check run's menu keys: up, down, left, right, enter, back, and `type:TEXT` (each character of it, a typed one)
fn menu_script(text: &str) -> std::collections::VecDeque<Result<Input, char>> {
    let mut out = std::collections::VecDeque::new();
    for k in text.split(',').map(str::trim) {
        match k {
            "up" => out.push_back(Ok(Input::Up)),
            "down" => out.push_back(Ok(Input::Down)),
            "left" => out.push_back(Ok(Input::Left)),
            "right" => out.push_back(Ok(Input::Right)),
            "enter" => out.push_back(Ok(Input::Enter)),
            "back" => out.push_back(Ok(Input::Back)),
            _ => out.extend(k.strip_prefix("type:").into_iter().flat_map(str::chars).map(Err)),
        }
    }
    out
}

/// the API's published scenes for Play, once, on a thread (offline: none – Play lists the kept copies)
fn fetch_listing(api: String, into: Arc<Mutex<Vec<Listed>>>) {
    #[cfg(feature = "sync")]
    std::thread::Builder::new()
        .name("play-listing".into())
        .spawn(move || {
            let agent = ureq::AgentBuilder::new().timeout_connect(std::time::Duration::from_secs(5)).timeout_read(std::time::Duration::from_secs(20)).build();
            let (status, body) = deck_native::published::get_with(&agent, &api, "/scenes", deck_link::TokenSource::from_env().get());
            let listed = if status == 200 { deck_space::published::catalog(&body) } else { vec![] };
            tracing::info!("play: {} published scenes listed ({status})", listed.len());
            if let Ok(mut l) = into.lock() {
                *l = listed;
            }
        })
        .ok();
    #[cfg(not(feature = "sync"))]
    let _ = (api, into);
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
            creator: deck_native::handoff::in_session(),
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
            play: self.menu.play.clone(),
        }
    }

    /// what the deck hears from a conductor now: its push (`scene.sync`) or a cell for the outlet, the relay's clock
    /// when this deck follows it, a MIDI clock in
    fn heard(&self) -> Heard {
        #[cfg(feature = "sync")]
        let pushed = self.show.sync.as_ref().is_some_and(|s| s.push_line().is_some());
        #[cfg(not(feature = "sync"))]
        let pushed = false;
        #[cfg(feature = "live")]
        let cell = self.show.live.as_ref().is_some_and(|l| l.status.lock().is_ok_and(|s| !s.cell.is_empty()));
        #[cfg(not(feature = "live"))]
        let cell = false;
        let scene = pushed || cell;
        #[cfg(all(feature = "live", feature = "space"))]
        let relay = self.show.show_link.heard() && !self.show.tempo.leading();
        #[cfg(not(all(feature = "live", feature = "space")))]
        let relay = false;
        let clock = match self.show.tempo.source() {
            crate::tempo::Source::Midi => Clock::Midi,
            crate::tempo::Source::Relay if relay => Clock::Conductor,
            _ => Clock::Own,
        };
        Heard { scene, clock }
    }

    /// JOIN A SHOW: this deck's outlet scene in session `code` – the relay as configured (else the hosted one), the role it
    /// has (none: no role); the page waits for the relay's answer (`join_answer`)
    fn join_session(&mut self, code: &str) {
        use deck_native::live::config::{deck_toml, Choice, Settings, HOSTED_RELAY};
        let file = deck_toml();
        let set = Settings::read(&file).merged(crate::outlet_settings(&self.args));
        let relay = deck_ui::join::relay_url(set.relay.as_deref().unwrap_or(HOSTED_RELAY), code);
        let choice = Choice::read(&file);
        let role = set.outlet.clone().or_else(|| choice.role.clone().filter(|r| !r.is_empty()));
        if choice.role.is_none() {
            // a deck with no role says so (an empty role is none, live/mod.rs `start_outlet`)
            let _ = Choice::keep(&file, "role", "");
        }
        let before = (self.args.relay.clone(), self.args.outlet.clone(), self.args.scene.clone());
        self.args.relay = Some(relay);
        self.args.outlet = Some(role.unwrap_or_else(|| "-".into()));
        self.args.scene = None;
        let old = self.error.take();
        self.reload();
        match self.error.take() {
            Some(e) => {
                (self.args.relay, self.args.outlet, self.args.scene) = before;
                self.error = old;
                self.menu.menu.join_failed(format!("NOT JOINED: {}", e.to_uppercase()));
            }
            None => {
                self.error = old;
                self.menu.before = Some(before);
            }
        }
    }

    /// the relay client's word on the join under way, to the code page: welcomed – the deck is in that session (kept in
    /// deck.toml for its next start); refused or silent – the page says why and the deck goes back to where it was
    fn join_answer(&mut self, time: f32) {
        use deck_ui::join::{Outcome, Reply};
        let s = crate::pages::session(&self.show);
        #[cfg(feature = "live")]
        let refused = self.show.live.as_ref().and_then(|l| l.status.lock().ok()?.problems_relay.clone()).map(|r| r.trim_start_matches("RELAY ").to_string());
        #[cfg(not(feature = "live"))]
        let refused = None;
        match self.menu.menu.settle(time, &Reply { code: s.code, joined: s.joined, refused, link: s.link }) {
            Outcome::Waiting => {}
            Outcome::In(code) => {
                self.menu.before = None;
                self.menu.begin = Begin::Rejoin(code);
                self.menu.left = false;
                let file = deck_native::live::config::deck_toml();
                for (key, value) in [("relay", self.args.relay.clone()), ("outlet", self.args.outlet.clone())] {
                    if let Some(v) = value {
                        let _ = deck_native::live::config::Choice::keep(&file, key, &v);
                    }
                }
            }
            Outcome::Failed(_) => {
                self.give_back_join();
                self.menu.left = true;
                self.switch("idle");
            }
        }
    }

    /// the relay, outlet and scene the deck had before a join that did not hold
    fn give_back_join(&mut self) {
        if let Some((relay, outlet, scene)) = self.menu.before.take() {
            (self.args.relay, self.args.outlet, self.args.scene) = (relay, outlet, scene);
        }
    }

    /// the menu follows the start's stand (once a frame); Play's list while a start page shows (every 2 s)
    fn start_frame(&mut self, refresh: bool, time: f32) {
        if self.menu.menu.joining() {
            self.join_answer(time);
        }
        let conductor = crate::pages::session(&self.show).conductor;
        let stand = start::stand(&self.menu.begin, &self.heard(), conductor, self.menu.left);
        let was = self.menu.menu.top();
        self.menu.menu.follow(&stand);
        if refresh && self.menu.menu.top().is_some_and(Page::start) {
            let cache = self.args.cache.clone().unwrap_or_else(crate::cache::default_dir);
            let listed = self.menu.listing.lock().map(|l| l.clone()).unwrap_or_default();
            self.menu.play = start::play_list(&listed, &deck_native::published::kept(&cache));
        }
        if self.menu.menu.top() != was {
            self.menu_sync(&self.menu_facts());
        }
    }

    /// One input for the menu; whether it took it.
    fn menu_input(&mut self, el: &ActiveEventLoop, i: Input) -> bool {
        if let Some(r) = self.renderer.as_mut() {
            r.ui_input(i);
        }
        let facts = self.menu_facts();
        match self.menu.menu.input(i, &facts) {
            Answer::No => return self.ballot_input(i),
            Answer::Quit => el.exit(),
            Answer::Creator => self.open_creator(el),
            Answer::Play(n) => {
                if let Some(p) = facts.play.get(n) {
                    self.switch(&p.value.clone());
                }
            }
            // out of the session: the idle scene without the outlet; the stand is the start menu's again
            Answer::Leave => {
                self.give_back_join();
                self.menu.left = true;
                self.switch("idle");
            }
            Answer::Join(code) => self.join_session(&code),
            Answer::Yes | Answer::Started => {}
        }
        self.menu_sync(&facts);
        true
    }

    /// #556: an input the menu left, for the ballot – unless a page of the deck is open (its keys are its own); whether
    /// the ballot took it. What it asks (a cast) goes to the Live outlet.
    fn ballot_input(&mut self, i: Input) -> bool {
        if self.page.open || self.switcher.open || self.live.page.open {
            return false;
        }
        let view = crate::pages::ballot(&self.show);
        let (took, did) = self.menu.ballot.input(i, view.as_ref());
        crate::pages::act(&self.show, did);
        took
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
                // #496: Esc drops a picked cast member first (the core says whether it had one), then opens the menu
                Key::Named(NamedKey::Escape) if !self.menu.menu.open() && self.show.space.as_mut().is_some_and(|s| s.key("Escape", false)) => return true,
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
                // the code page: typed characters and Backspace are the code's (Esc goes back)
                Key::Named(NamedKey::Backspace) if self.menu.menu.top() == Some(Page::Join) => return self.menu.menu.typed(Typed::Delete),
                Key::Character(c) if self.menu.menu.top() == Some(Page::Join) && !self.mods.control_key() => return c.chars().fold(false, |_, c| self.menu.menu.typed(Typed::Char(c))),
                // Backspace is the Live page's while it shows (a name is typed there)
                Key::Named(NamedKey::Backspace) if drawn => Input::Back,
                Key::Named(NamedKey::ArrowUp) => Input::Up,
                Key::Named(NamedKey::ArrowDown) => Input::Down,
                Key::Named(NamedKey::ArrowLeft) => Input::Left,
                Key::Named(NamedKey::ArrowRight) => Input::Right,
                Key::Named(NamedKey::Enter) => Input::Enter,
                // a drawn page keeps the other keys, but those that pass every page (F1, F9, the volume)
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

    /// Once a frame: deck-link's pairing state (every 2 s), the pad's buttons, the menu's page to the stage (only when
    /// it changed; a layer of deck-kit's egui screen, #485).
    pub(crate) fn menu_frame(&mut self, el: &ActiveEventLoop, time: f32) {
        let refresh = time - self.menu.pair_at >= 2.0 || time < self.menu.pair_at;
        if refresh {
            self.menu.pair_at = time;
            let state = deck_link::pairing::read_state(&deck_link::pairing::run_dir());
            self.menu.pair = deck_native::pairing::of(state.as_ref(), &deck_link::pairing::app_url());
        }
        self.start_frame(refresh, time);
        #[cfg(feature = "gamepad")]
        {
            // while the ballot shows, the d-pad and A are its own as they are the open menu's
            let open = self.menu.menu.open() || self.menu.ballot.top().is_some();
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
        if let Some(layer) = self.menu.menu.paint(&facts, r.size()) {
            r.set_menu(layer);
        }
        // #556: the ballot, when it changed (a second of its countdown, a cast, the tally, the result, gone)
        let view = crate::pages::ballot(&self.show);
        if let Some(layer) = self.menu.ballot.paint(view.as_ref(), r.size(), deck_outlet::text::mono_per_px()) {
            r.set_vote(layer);
        }
        // a check run's menu keys
        if !self.menu.script.is_empty() && time >= self.menu.script_at {
            self.menu.script_at = time + 0.4;
            match self.menu.script.pop_front() {
                Some(Ok(i)) => {
                    self.menu_input(el, i);
                }
                Some(Err(c)) => {
                    self.menu.menu.typed(Typed::Char(c));
                }
                None => {}
            }
        }
        // a check run's keys, once the ballot shows
        if view.is_some() && !self.menu.keys.is_empty() && time >= *self.menu.key_at.get_or_insert(time + 0.4) {
            self.menu.key_at = Some(time + 0.4);
            if let Some(i) = self.menu.keys.pop_front() {
                self.menu_input(el, i);
            }
        }
    }
}
