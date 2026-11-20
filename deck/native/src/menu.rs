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
//! the deck's start (the rules are deck-ui `start.rs`): what the deck hears – the conductor's push or cell, the
//! relay's clock, a MIDI clock in – and whether its conductor is in, read every frame; the menu follows the stand. Play's
//! list: the API's listing of the published scenes and the sandbox's published shows (a thread, once) and the copies this
//! deck keeps. A show picked there plays on this deck as its own conductor (deck-show alone.rs): its scenes fetched
//! first (a thread), then its rows' cells go to the piece side; the show page's GO fires the next row, LEAVE ends it.
use crate::App;
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
    /// a check run (`DECK_MENU_KEYS=enter,down`): the keys the start menu gets once it shows, one every 0.4 s
    menu_keys: std::collections::VecDeque<Input>,
    menu_key_at: Option<f32>,
    /// the show this deck conducts itself
    #[cfg(feature = "live")]
    own: Own,
}

/// the API's answer listing the shows, the show while its scenes are fetched, the show playing, the cells for the piece
/// side, the show's time base
#[cfg(feature = "live")]
#[derive(Default)]
struct Own {
    shows: Arc<Mutex<String>>,
    fetching: Option<(deck_show::alone::Alone, std::sync::mpsc::Receiver<()>)>,
    playing: Option<deck_show::alone::Alone>,
    cells: Vec<protocol::Cell>,
    started: Option<std::time::Instant>,
}

#[cfg(feature = "live")]
impl Own {
    fn ms(&mut self) -> u64 {
        self.started.get_or_insert_with(std::time::Instant::now).elapsed().as_millis() as u64
    }

    fn active(&self) -> bool {
        self.fetching.is_some() || self.playing.is_some()
    }
}

/// a check run's keys (`up,down,enter,open`) as the kit's inputs
fn keys_of(var: &str) -> std::collections::VecDeque<Input> {
    let keys = std::env::var(var).unwrap_or_default();
    keys.split(',')
        .filter_map(|k| match k.trim() {
            "up" => Some(Input::Up),
            "down" => Some(Input::Down),
            "enter" => Some(Input::Enter),
            "open" => Some(Input::Open),
            _ => None,
        })
        .collect()
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
        #[cfg(feature = "live")]
        let own = Own::default();
        if !check {
            fetch_listing(args.api.clone(), listing.clone());
            #[cfg(feature = "live")]
            fetch_shows(args.api.clone(), own.shows.clone());
        }
        Native {
            menu: Menu::default(),
            ballot: Default::default(),
            keys: keys_of("DECK_BALLOT_KEYS"),
            key_at: None,
            at: (0.0, 0.0),
            pair: Pair::Off,
            pair_at: -10.0,
            begin,
            left: false,
            listing,
            play: Vec::new(),
            menu_keys: keys_of("DECK_MENU_KEYS"),
            menu_key_at: None,
            #[cfg(feature = "live")]
            own,
        }
    }

    /// the cells of the show this deck conducts itself, for the piece side (once a frame)
    pub fn take_cells(&mut self) -> Vec<protocol::Cell> {
        #[cfg(feature = "live")]
        return std::mem::take(&mut self.own.cells);
        #[cfg(not(feature = "live"))]
        Vec::new()
    }
}

/// the sandbox's published shows for Play, once, on a thread: the API's answer kept whole (a pick plays from it); a check
/// run reads it from the file `DECK_SHOWS`
#[cfg(feature = "live")]
fn fetch_shows(api: String, into: Arc<Mutex<String>>) {
    if let Some(file) = std::env::var_os("DECK_SHOWS") {
        if let (Ok(body), Ok(mut s)) = (std::fs::read_to_string(file), into.lock()) {
            *s = body;
        }
        return;
    }
    #[cfg(feature = "sync")]
    std::thread::Builder::new()
        .name("play-shows".into())
        .spawn(move || {
            let agent = ureq::AgentBuilder::new().timeout_connect(std::time::Duration::from_secs(5)).timeout_read(std::time::Duration::from_secs(20)).build();
            let path = deck_show::alone::path(start::DEMO_TOON);
            let (status, body) = deck_native::published::get_with(&agent, &api, &path, deck_link::TokenSource::from_env().get());
            tracing::info!("play: {} published shows listed ({status})", deck_show::alone::listing(&body).len());
            if let (200, Ok(mut s)) = (status, into.lock()) {
                *s = body;
            }
        })
        .ok();
    #[cfg(not(feature = "sync"))]
    let _ = (api, into);
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
            #[cfg(feature = "live")]
            showing: self.menu.own.playing.as_ref().map(deck_show::alone::Alone::showing),
            #[cfg(not(feature = "live"))]
            showing: None,
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

    /// the menu follows the start's stand (once a frame); Play's list while a start page shows (every 2 s)
    fn start_frame(&mut self, refresh: bool) {
        let conductor = crate::pages::session(&self.show).conductor;
        let stand = start::stand(&self.menu.begin, &self.heard(), conductor, self.menu.left);
        let was = self.menu.menu.top();
        self.menu.menu.follow(&stand);
        if refresh && self.menu.menu.top().is_some_and(Page::start) {
            let cache = self.args.cache.clone().unwrap_or_else(crate::cache::default_dir);
            #[allow(unused_mut)]
            let mut listed = self.menu.listing.lock().map(|l| l.clone()).unwrap_or_default();
            #[cfg(feature = "live")]
            listed.extend(self.menu.own.shows.lock().map(|b| deck_show::alone::listing(&b)).unwrap_or_default());
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
            Answer::Play(n) => match facts.play.get(n) {
                Some(p) if p.show => self.play_show(p.value.trim_start_matches("show:")),
                Some(p) => self.switch(&p.value.clone()),
                None => {}
            },
            Answer::Go => self.show_go(),
            // out of its own show, or out of the session: the idle scene without the outlet; the start menu again
            Answer::Leave => {
                self.menu.left = true;
                self.stop_show();
                self.switch("idle");
            }
            Answer::Yes | Answer::Started => {}
        }
        self.menu_sync(&facts);
        true
    }

    /// The published show `id` on this deck as its own conductor: the scenes its outlet plays are fetched first (a
    /// thread), then its first row fires (`own_frame`). One that cannot play says why on the switcher's line.
    fn play_show(&mut self, id: &str) {
        #[cfg(feature = "live")]
        {
            self.stop_show();
            let body = self.menu.own.shows.lock().map(|b| b.clone()).unwrap_or_default();
            let alone = match deck_show::alone::find(&body, id).and_then(deck_show::alone::Alone::new) {
                Ok(a) => a,
                Err(why) => {
                    tracing::warn!("show: {why}");
                    self.switcher.status = format!("NOT PLAYED: {why}");
                    return;
                }
            };
            tracing::info!("show: {id} on this deck as its own conductor, outlet '{}'", alone.outlet());
            let scenes: Vec<_> = alone.needs().iter().filter_map(|p| deck_space::published::SceneRef::parse(p)).filter_map(Result::ok).collect();
            let (api, cache) = (self.args.api.clone(), self.args.cache.clone().unwrap_or_else(crate::cache::default_dir));
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::Builder::new()
                .name("show-scenes".into())
                .spawn(move || {
                    for r in scenes {
                        if let Err(e) = deck_native::published::fetch(r, &api, &cache) {
                            tracing::warn!("show: {e:#}");
                        }
                    }
                    let _ = tx.send(());
                })
                .ok();
            self.menu.own.fetching = Some((alone, rx));
        }
        #[cfg(not(feature = "live"))]
        let _ = id;
    }

    /// GO on the show page: the show this deck conducts fires its next row
    fn show_go(&mut self) {
        #[cfg(feature = "live")]
        {
            let ms = self.menu.own.ms();
            if let Some(a) = self.menu.own.playing.as_mut() {
                let cells = a.go(ms);
                self.menu.own.cells.extend(cells);
            }
        }
    }

    /// the show this deck conducts itself ends: what played fades (the piece side forgets it)
    fn stop_show(&mut self) {
        #[cfg(feature = "live")]
        if self.menu.own.active() {
            self.menu.own = Own { shows: self.menu.own.shows.clone(), ..Own::default() };
            let _ = self.pieces.cell(&deck_show::alone::Alone::leave());
        }
    }

    /// Once a frame: the show's scenes fetched – its first row fires; time passes for the show playing.
    fn own_frame(&mut self) {
        #[cfg(feature = "live")]
        {
            let ms = self.menu.own.ms();
            let fetched = self.menu.own.fetching.as_ref().is_some_and(|(_, rx)| !matches!(rx.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)));
            if fetched {
                if let Some((mut a, _)) = self.menu.own.fetching.take() {
                    self.menu.own.cells.extend(a.go(ms));
                    self.menu.own.playing = Some(a);
                }
            }
            if let Some(a) = self.menu.own.playing.as_mut() {
                let cells = a.tick(ms);
                self.menu.own.cells.extend(cells);
            }
        }
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
        self.start_frame(refresh);
        self.own_frame();
        // a check run's keys, from the moment the start menu shows
        if self.menu.menu.top().is_some_and(Page::start) || self.menu.menu_key_at.is_some() {
            let due = *self.menu.menu_key_at.get_or_insert(time + 0.4);
            if time >= due && !self.menu.menu_keys.is_empty() {
                self.menu.menu_key_at = Some(time + 0.4);
                if let Some(i) = self.menu.menu_keys.pop_front() {
                    self.menu_input(el, i);
                }
            }
        }
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
        // a check run's keys, once the ballot shows
        if view.is_some() && !self.menu.keys.is_empty() && time >= *self.menu.key_at.get_or_insert(time + 0.4) {
            self.menu.key_at = Some(time + 0.4);
            if let Some(i) = self.menu.keys.pop_front() {
                self.menu_input(el, i);
            }
        }
    }
}
