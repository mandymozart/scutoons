//! A scene in a browser canvas (#463, #465): the renderer every deck draws with (deck-space `stage`) on the page's GPU, the
//! scene as the native deck plays it, the splash and the build label over it; the HUD is off. No scene given = the deck's
//! idle scene, as on the native deck. The control domain is deck-engine's, the clock the scene's own tempo. The page
//! only fetches and decodes what is asked for: the space's set and portraits, a chapter and its slices, an outlet's media.
//! An `outlet` node joins the relay once the page names the outlet (`join_outlet`, outlet.rs). Not yet: the sound.
//!
//! A show's piece (#91, #473) plays here as on the native deck: the core decides (deck-space piece.rs `Side`, veil.rs),
//! this brings the three things a platform has – **find** (a built-in scene by its name), **switch** (the scene's text
//! is built into a new `Show` behind the black veil, the canvas and its GPU stay), **set** (a gliding value into the
//! control domain). What the page fetched for the show before carries its number (`TURN`) and is dropped.
//!
//! A published scene (#246, plan 2b): `scene:<id>[@v]` is loaded by the core's loader through deck-space's fetched.rs
//! (published.rs: the document, scene-content, the resolve step) – the page fetches what it asks (`scene_asks`,
//! `scene_answer`); the idle scene plays until it is here, then it is the deck's own scene. One that does not play
//! gives the test pattern, and the core's reason goes to the console. The browser keeps no copy (the HTTP cache).
//!
//! It listens (#520, research/SCENE-EVENTS.md §4 E2c): a published scene in a session (`join_scene`, the page names the
//! relay) has a link of its own – deck-space published.rs `CastLink`, a deck of the session, its socket the page's
//! (`SceneLink`). The core decides: the cast words of an `address` and the story events the scene's listeners hear, the
//! story asked for only while it listens. This only moves what came and **switches**: the scene the cast changed to is
//! the deck's own scene from then on and is changed to through black, as a piece is (the browser deck plays no sound, so
//! there is no in-place recast to make – live-core is not in this path). #538: a deck with an outlet joins the same way,
//! and a published scene a show's cell launched listens like a `?scene=`: the link's cast is the scene that plays (the
//! core's piece.rs `Side::playing`), its changed cast switched to beside the outlet (`Side::recast`).
use crate::outlet::Outlet;
use crate::{err, figure_asks, figure_from_page, open_canvas, portrait_asks, set_from_page, CanvasGpu, SPLASH_AT_MOST};
use deck_engine::control::Controls;
use deck_show::face::Face;
use deck_space::chapter::{finish_slice, Chapter, Dialog, Source};
use deck_space::fetched::Fetched;
use deck_space::graph::{Graph, Kind};
use deck_space::hud::{Alert, Hud};
use deck_space::quality::MAX_SIDE;
use deck_space::piece::{Found, Side, Switch};
use deck_space::published::{CastLink, SceneRef};
use deck_space::rig::address::Place;
use deck_space::scene::{named, Named, Scene, IDLE_SCENE, TEST_PATTERN_SCENE};
use deck_space::space::portrait::Portraits;
use deck_space::space::Space;
use deck_space::splash::Fade;
use deck_space::stage::{ChapterFeed, Over, Prepared, Stage, Tick};
use deck_space::tempo;
use deck_space::veil::Change;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wasm_bindgen::prelude::*;

/// #386: the deck's menu (deck-ui `menu.rs`) – keys, the pointer, the facts, the picture
mod menu;

/// a chapter that loads nothing is asked again after this (s)
const RETRY_S: f32 = 10.0;
/// what the page fetches for a show (a chapter, a slice, a sprite) is numbered `show × TURN + index`: an answer for a
/// show that is gone is dropped
const TURN: usize = 4096;

/// A `chapter` node's slices on their way: the page fetches, this keeps the order and hands them to the stage.
struct Pump {
    source: Source,
    tx: SyncSender<Prepared>,
    status: Arc<Mutex<String>>,
    ready: Arc<AtomicBool>,
    strip: Vec<(String, Vec<Dialog>)>,
    next: usize,
    flying: bool,
    /// ready, waiting for room in the channel
    held: Option<Prepared>,
    failed: usize,
    wait_until: f32,
}

impl Pump {
    fn say(&self, s: String) {
        *self.status.lock().unwrap() = s;
    }

    fn at(&self) -> usize {
        self.next % self.strip.len().max(1)
    }
}

/// A scene as it plays: all that a scene load replaces (the canvas, its GPU and the outlet stay).
struct Show {
    graph: Graph,
    stage: Stage,
    controls: Controls,
    space: Option<Space>,
    room: String,
    pumps: Vec<Pump>,
    bpm: f64,
    per_bar: u32,
    size: (u32, u32),
    /// the video nodes' files were asked for
    videos_asked: bool,
}

impl Show {
    /// the scene `text` (TOML) for a surface of `format` and `size`
    async fn build(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat, size: (u32, u32), text: &str) -> Result<Show, String> {
        let why = |e: anyhow::Error| format!("{e:#}");
        let scene = Scene::parse(text).map_err(why)?;
        let mut graph = Graph::build(&scene).map_err(why)?;
        let tempo = deck_engine::tempo_config(&graph).map_err(why)?;
        // the page has no relay session of a scene's own
        let space_node = graph.first(Kind::Space);
        if let Some(s) = space_node {
            graph.nodes[s].params.remove("join");
        }
        let scale = scene.stage.render_scale;
        let mut stage = Stage::new(device, queue, format, size, scale, scale, false).await.map_err(why)?;
        let (mut feeds, mut pumps) = (Vec::new(), Vec::new());
        for (i, node) in graph.nodes.iter().enumerate().filter(|(_, n)| n.kind == Kind::Chapter) {
            let (tx, rx) = sync_channel(1);
            let (status, ready) = (Arc::new(Mutex::new("waiting".to_string())), Arc::new(AtomicBool::new(false)));
            feeds.push(ChapterFeed { node: i, rx, status: status.clone(), ready: ready.clone() });
            let source = Source::of(node);
            let next = source.start;
            pumps.push(Pump { source, tx, status, ready, strip: Vec::new(), next, flying: false, held: None, failed: 0, wait_until: 0.0 });
        }
        stage.load(device, format, &graph, feeds).await.map_err(why)?;
        let mut room = String::new();
        let space = space_node.map(|n| {
            room = graph.nodes[n].str("set").unwrap_or_default().to_string();
            let mut s = Space::new(&graph, n, &scene.being, std::path::Path::new(""), Portraits::manual());
            s.set_scene(&scene);
            s.fit_screen(size.0 as f32, size.1 as f32);
            s
        });
        let controls = Controls::new(&graph, None);
        Ok(Show { graph, stage, controls, space, room, pumps, bpm: tempo.bpm, per_bar: tempo.per_bar, size, videos_asked: false })
    }

    fn resize(&mut self, device: &wgpu::Device, (w, h): (u32, u32)) {
        self.size = (w, h);
        self.stage.resize(device, (w, h));
        if let Some(s) = self.space.as_mut() {
            s.fit_screen(w as f32, h as f32);
        }
    }
}

/// #478: what the Live page asked, to the relay; #556: the ballot's cast, into the vote open now
fn live_do(o: &mut Outlet, did: Option<deck_ui::live::Do>) {
    let _ = match did {
        Some(deck_ui::live::Do::Take(role)) => o.relay.take_role(role.as_deref()),
        Some(deck_ui::live::Do::Cast(option)) => o.relay.vote_cast(option as i64),
        Some(deck_ui::live::Do::Name(name)) => {
            // kept on this device as the reader's own choice (`live_kept`)
            o.named = Some(name.clone()).filter(|n| !n.is_empty());
            o.relay.set_name(&name)
        }
        None => false,
    };
}

#[wasm_bindgen]
pub struct SceneView {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    canvas: web_sys::HtmlCanvasElement,
    show: Show,
    /// the shows so far (the first is 0): what the page fetches is numbered with it (`TURN`)
    turn: usize,
    /// a scene loaded in place: the page fetches its chapters (`reloaded`)
    reloaded: bool,
    beats: f64,
    beat: tempo::Now,
    now: f32,
    age: f32,
    fade: Fade,
    backend: String,
    /// #465: the Live outlet of an `outlet` node, once joined
    outlet: Option<Outlet>,
    /// a set the space should show next (the page fetches it)
    set_wanted: Option<String>,
    /// #470 #386: the pairing's code ("" = none) and what the menu's "Pair this deck" says
    pairing_code: String,
    pair: deck_ui::menu::Pair,
    /// #386: the deck's menu – on its splash until START, unless the deck is told its outlet
    menu: deck_ui::menu::Menu,
    /// the deck's own scene (TOML): a piece plays beside its outlet, black comes back to it
    deck: String,
    /// #473: the piece side, once the outlet is joined
    side: Option<Side>,
    /// #481: the deck's dialogs for the person at it, as a picture and its presses (deck-show `face.rs`)
    face: Face,
    /// #556: the ballot of a vote this deck may cast in, as a picture and its keys and presses (deck-ui `ballot.rs`)
    ballot: deck_ui::ballot::Page,
    /// #479: a cell whose piece is being fetched – compared again when the answer is here; a newer cell replaces it
    held: Option<(protocol::Cell, Place)>,
    /// the change through black to the scene (TOML) the piece side asks for, the show being built behind it, the built one
    change: Change,
    building: bool,
    built: Rc<RefCell<Option<Result<Show, String>>>>,
    /// #478: the Live page (the native deck's F6, deck-ui live.rs) and the HUD it is drawn on
    live: deck_ui::live::Page,
    hud: Hud,
    /// #478: the page's place – the screen pixels per CSS pixel and the host's line across the top (`live_place`)
    live_at: (f32, f32),
    /// #246: the published scene asked for, until it is here; the core's loader asks through `scenes`
    published: Option<SceneRef>,
    scenes: Fetched,
    /// #520: the published scene's link to the session (`join_scene`); its kept copy, once here (the cast plays from it)
    cast: Option<Rc<RefCell<CastLink>>>,
    kept: Option<String>,
}

#[wasm_bindgen]
impl SceneView {
    /// `scene` on `canvas` (its size in device pixels is the canvas' width / height): a system scene's name, a sandbox's
    /// in a dev build (#427: the feature `sandbox`, `npm run build:wasm -w shared/live-deck -- --sandbox`), a scene
    /// document, a published scene `scene:<id>[@v]` (#246: idle until it is here), or "" = the idle scene. `prefer` as
    /// `SpaceView::create`.
    pub async fn create(canvas: web_sys::HtmlCanvasElement, scene: String, prefer: String) -> Result<SceneView, JsValue> {
        let published = SceneRef::parse(&scene).transpose().map_err(err)?;
        let deck = match named(&scene) {
            _ if scene.is_empty() || published.is_some() => IDLE_SCENE,
            Named::System(text) => text,
            #[cfg(feature = "sandbox")]
            Named::Sandbox => deck_space::scene::sandbox(&scene).unwrap_or_default(),
            #[cfg(not(feature = "sandbox"))]
            Named::Sandbox => return Err(err(deck_space::scene::sandbox_refused(&scene))),
            Named::Other => scene.as_str(),
        }
        .to_owned();
        Graph::build(&Scene::parse(&deck).map_err(err)?).map_err(err)?;
        let element = canvas.clone();
        let (w, h) = (canvas.width().clamp(1, MAX_SIDE), canvas.height().clamp(1, MAX_SIDE));
        let CanvasGpu { surface, device, queue, format, alpha, backend, .. } = open_canvas(canvas, &prefer).await?;
        element.set_width(w);
        element.set_height(h);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: w,
            height: h,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: alpha,
            view_formats: vec![],
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&device, &config);
        let show = Show::build(&device, &queue, format, (w, h), &deck).await.map_err(err)?;
        Ok(SceneView {
            surface,
            device,
            queue,
            config,
            canvas: element,
            show,
            turn: 0,
            reloaded: false,
            beats: 0.0,
            beat: tempo::Now::default(),
            now: 0.0,
            age: 0.0,
            fade: Fade { alpha: 1.0 },
            backend: backend.into(),
            outlet: None,
            set_wanted: None,
            pairing_code: String::new(),
            pair: deck_ui::menu::Pair::Off,
            menu: deck_ui::menu::Menu::new(false),
            deck,
            side: None,
            held: None,
            face: Face::default(),
            ballot: Default::default(),
            change: Change::default(),
            building: false,
            built: Rc::default(),
            live: Default::default(),
            hud: Hud::default(),
            live_at: (1.0, 0.0),
            published,
            scenes: Fetched::default(),
            cast: None,
            kept: None,
        })
    }

    /// #520: join `relay` (ws:// or wss://…/live/<code>) as a deck of its session for the published scene this view plays,
    /// named `nick` ("" = the core's name) – its socket is the page's (client.ts `RelaySocket` over the returned link).
    /// None: no relay named, or neither a published scene nor an outlet whose cells may launch one (#538).
    pub fn join_scene(&mut self, relay: &str, nick: &str) -> Result<Option<SceneLink>, JsValue> {
        if self.published.is_none() && self.kept.is_none() && self.side.is_none() {
            return Ok(None);
        }
        let nick = deck_ui::live::preset_name(None, Some(nick), "browser");
        let Some(link) = CastLink::join(relay, &nick, "browser").transpose().map_err(err)? else { return Ok(None) };
        let link = Rc::new(RefCell::new(link));
        self.cast = Some(link.clone());
        self.cast_play();
        Ok(Some(SceneLink { link }))
    }

    /// #520: the published scene's kept copy to its link – the cast it plays from. #538: on a deck with an outlet the scene
    /// that plays is the core's answer (piece.rs `Side::playing`: a cell's `scene:<id>`), its copy the one the outlet's
    /// store loaded (a deck with an outlet plays no `?scene=` of its own: the idle scene has no outlet).
    fn cast_play(&self) {
        let Some(link) = self.cast.as_ref() else { return };
        if let Err(e) = link.borrow_mut().play(self.kept_now()) {
            web_sys::console::warn_1(&JsValue::from_str(&format!("scene: no cast words or story while it plays: {e}")));
        }
    }

    /// the kept copy of the published scene that plays: the deck's own, or (#538) a cell's beside the outlet
    fn kept_now(&self) -> Option<&str> {
        match (self.side.as_ref(), self.outlet.as_ref()) {
            (Some(side), Some(o)) => side.playing(None).and_then(|p| SceneRef::parse(p)?.ok()).and_then(|r| o.fetched.kept(&r)),
            _ => self.kept.as_deref(),
        }
    }

    /// #520: what came for the scene through its link went to the core; the scene its cast changed to is switched to
    /// through black – the deck's own scene from then on, or (#538) a cell's scene beside the outlet
    fn cast_turn(&mut self) {
        let Some(link) = self.cast.as_ref() else { return };
        let (said, changed, held) = {
            let mut l = link.borrow_mut();
            (l.said(), l.changed(), l.held())
        };
        if let Some(s) = self.show.space.as_mut() {
            held.into_iter().for_each(|h| s.held(h));
        }
        for line in said {
            web_sys::console::log_1(&JsValue::from_str(&line));
        }
        match changed {
            Some(Ok(text)) => match self.side.as_ref() {
                // #538: a cell's scene – switched to beside the outlet, as its Load (the core's `Side::recast`)
                Some(side) => match side.recast(text) {
                    Ok(scene) => self.change.ask(&scene),
                    Err(e) => web_sys::console::warn_1(&JsValue::from_str(&format!("scene: the changed cast is not played: {e}"))),
                },
                None => {
                    self.deck = text;
                    self.change.ask(&self.deck);
                }
            },
            Some(Err(e)) => web_sys::console::warn_1(&JsValue::from_str(&format!("scene: the changed cast is not played: {e}"))),
            None => {}
        }
    }

    /// #246: what the page fetches for the published scene: key (`scene:<id>`) and path after the API's base, in pairs
    pub fn scene_asks(&mut self) -> js_sys::Array {
        let out = js_sys::Array::new();
        for (place, path) in self.scenes.asks() {
            out.push(&JsValue::from_str(&place.to_string()));
            out.push(&JsValue::from_str(&path));
        }
        out
    }

    /// #246: the answer to a `scene_asks` pair: its HTTP status (0 = the server can't be reached) and body
    pub fn scene_answer(&mut self, key: &str, status: u16, body: &str) {
        if let Ok(place) = Place::parse(key) {
            self.scenes.answered(&place, status, body);
        }
    }

    /// #246: the published scene, once the core's loader has it, becomes the deck's own scene (through black); one that
    /// does not play gives the test pattern and its reason
    fn published(&mut self) {
        let Some(r) = self.published.as_ref() else { return };
        let text = match self.scenes.scene(r) {
            None => return,
            Some(Found::Scene(text)) => text,
            Some(found) => {
                let why = match found {
                    Found::Missing(why) => why,
                    _ => format!("{r} is no scene"),
                };
                web_sys::console::warn_1(&JsValue::from_str(&format!("the scene can't be played – the test pattern instead: {why}")));
                TEST_PATTERN_SCENE.to_string()
            }
        };
        self.kept = self.scenes.kept(r).map(String::from);
        self.published = None;
        self.deck = text.clone();
        self.change.ask(&text);
        self.cast_play();
    }

    /// #478: where the page goes – `density` canvas pixels per CSS pixel, `top` canvas pixels covered by the page's line
    /// across the top (the stats line): the page is centred below it at its readable size (deck-space `hud::page_rect`)
    pub fn live_place(&mut self, density: f32, top: f32) {
        self.live_at = (density, top);
    }

    /// the page's rectangle and scale on the canvas now
    fn live_rect(&self) -> ([f32; 4], f32) {
        deck_space::hud::page_rect(self.config.width as f32, self.config.height as f32, self.live_at.0, self.live_at.1)
    }


    /// #478: the HUD's line for the stats bar – `LIVE TENOR: ANNA`, or the relay's line before the deck is in its session
    pub fn live_line(&self) -> String {
        let Some(o) = self.outlet.as_ref() else { return String::new() };
        let s = o.relay.live();
        if s.joined { deck_ui::live::hud_line(&s) } else { s.link }
    }

    /// #478: what this device keeps of the page's choice (the text `join_outlet` takes back next time)
    pub fn live_kept(&self) -> String {
        let Some(o) = self.outlet.as_ref() else { return String::new() };
        let s = o.relay.live();
        let kept = crate::outlet::Kept { role: Some(s.role.unwrap_or_default()), name: o.named.clone() };
        serde_json::to_string(&kept).unwrap_or_default()
    }

    /// #465: join `relay` ("" = the hosted show) as output of `outlet`, its text in `locale`, show media's signed links from
    /// `api` (#470) - when the scene has an `outlet` node; false: it has none
    /// #478: `kept` = what the Live page chose on this device last (`live_kept`; "" = nothing); `reader` = the signed-in
    /// reader's name ("" = nobody) – the deck's name when none is kept
    pub fn join_outlet(&mut self, relay: &str, outlet: &str, kept: &str, reader: &str, locale: &str, api: &str) -> Result<bool, JsValue> {
        if self.show.graph.first(Kind::Outlet).is_none() {
            return Ok(false);
        }
        let (tx, rx) = sync_channel(2);
        let o = Outlet::new(relay, outlet, kept, reader, if locale.is_empty() { "de" } else { locale }, api, (self.config.width, self.config.height), tx).map_err(err)?;
        self.show.stage.set_outlet(rx);
        self.outlet = Some(o);
        self.side = Some(Side::new(&self.deck));
        // #386: a deck told its outlet is in a session: its splash passes by itself once loaded, no START
        if self.menu.on_splash() {
            self.menu.close();
        }
        Ok(true)
    }

    // the outlet's relay client, driven by the page's socket (client.ts `RelaySocket` – the names are relay-web's)
    pub fn start(&mut self, now: f64) {
        self.relay(|c| c.start(now));
    }

    pub fn opened(&mut self, now: f64) {
        self.relay(|c| c.opened(now));
    }

    pub fn closed(&mut self, now: f64, why: &str) {
        self.relay(|c| c.closed(now, why));
    }

    pub fn text(&mut self, now: f64, data: &str) {
        self.relay(|c| c.text(now, data));
    }

    /// #438: the WebTransport session and its stream are up – the page's quic.ts drives the outlet's relay client as
    /// relay-web's (a hosted relay offers QUIC; without these the outlet's first dial threw and it never joined: found in
    /// #481's live test in production)
    pub fn quic_opened(&mut self, now: f64) {
        self.relay(|c| c.quic_opened(now));
    }

    pub fn quic_closed(&mut self, now: f64, why: &str) {
        self.relay(|c| c.quic_closed(now, why));
    }

    pub fn quic_stream(&mut self, now: f64, bytes: &[u8]) {
        self.relay(|c| c.quic_stream(now, bytes));
    }

    pub fn quic_datagram(&mut self, now: f64, bytes: &[u8]) {
        self.relay(|c| c.quic_datagram(now, bytes));
    }

    pub fn tick(&mut self, now: f64) {
        self.relay(|c| c.tick(now));
    }

    pub fn stop(&mut self) {
        self.relay(|c| c.stop());
    }

    pub fn next_wake(&self) -> Option<f64> {
        self.outlet.as_ref().and_then(|o| o.relay.next_wake())
    }

    /// what the socket should do (JSON, as relay-web's `take`); the messages stay here (the outlet routes them)
    pub fn take(&mut self) -> String {
        let now = Duration::from_secs_f32(self.now.max(0.0));
        let out = self.outlet.as_mut().map(|o| o.take(now)).unwrap_or_default();
        serde_json::to_string(&out).unwrap_or_else(|_| "[]".into())
    }

    pub fn line(&self) -> String {
        self.outlet.as_ref().map(|o| o.relay.line()).unwrap_or_default()
    }

    fn relay(&mut self, f: impl FnOnce(&mut deck_space::relay::Client)) {
        if let Some(o) = self.outlet.as_mut() {
            f(&mut o.relay);
        }
    }

    /// what the outlet asks the page to fetch: key, path, … (key c: a chapter under the API, p: a slice under the CDN,
    /// m: a medium's URL - never with the token, s: a show medium's signed link from the API - with the deck's token, #470;
    /// r: the deck's proof from the API - with the reader's sign-in, #478)
    pub fn outlet_asks(&mut self) -> js_sys::Array {
        let out = js_sys::Array::new();
        for (k, path) in self.outlet.as_mut().map(|o| o.asks()).unwrap_or_default() {
            out.push(&JsValue::from_str(&k));
            out.push(&JsValue::from_str(&path));
        }
        out
    }

    pub fn outlet_text(&mut self, key: &str, text: &str) {
        let Some(o) = self.outlet.as_mut() else { return };
        if !o.answered(key, Ok((200, text))) {
            o.shelf().give_text(key, text);
            o.medium("", None);
        }
    }

    /// a picture the outlet asked for, decoded by the browser
    pub fn outlet_image(&mut self, key: &str, width: u32, height: u32, rgba: Vec<u8>) {
        let Some(o) = self.outlet.as_mut() else { return };
        match image::RgbaImage::from_raw(width, height, rgba) {
            Some(img) => {
                o.shelf().give_image(key, img);
                o.medium(key.strip_prefix("m:").unwrap_or(""), None);
            }
            None => self.outlet_failed(key, "a picture of the wrong size".into()),
        }
    }

    /// a medium that is no picture (a video, a sound) is in the browser's cache
    pub fn outlet_fetched(&mut self, key: &str) {
        if let Some(o) = self.outlet.as_mut() {
            o.shelf().failed(key, "not a picture - the browser plays no video yet".into());
            o.medium(key.strip_prefix("m:").unwrap_or(""), None);
        }
    }

    /// #470: the API's answer to an `s:` ask (its status and body) - the signed link is asked next as `m:`; #475: a
    /// flashed show's (`sf:`)
    pub fn outlet_signed(&mut self, key: &str, status: u16, body: &str) {
        let Some(o) = self.outlet.as_mut() else { return };
        if o.answered(key, Ok((status, body))) {
            return;
        }
        if let Some(id) = key.strip_prefix("s:") {
            o.signed(id, status, body);
        }
    }

    /// #478: the API's answer to the `r:` ask - the deck's proof of facts for its session (status 0: no answer)
    pub fn outlet_proof(&mut self, status: u16, body: &str) {
        let now = Duration::from_secs_f32(self.now.max(0.0));
        if let Some(o) = self.outlet.as_mut() {
            o.proof(status, body, now);
        }
    }

    /// #470: the deck's link (DeckLink) - why it has no token ("" = it has one) and the code to pair with ("" = none);
    /// `app` = the viewer the member pairs on; #484 `signed` = a member is signed in (the deck is its user). #386: the
    /// menu's "Pair this deck" says it (the corner card is gone).
    pub fn deck_link(&mut self, why: &str, code: &str, app: &str, signed: bool) {
        if let Some(o) = self.outlet.as_mut() {
            if o.unpaired != why {
                o.unpaired = why.to_owned();
            }
        }
        self.set_pair(why, code, app, signed);
    }

    pub fn outlet_failed(&mut self, key: &str, why: String) {
        if key.starts_with("r:") {
            return self.outlet_proof(0, "");
        }
        let Some(o) = self.outlet.as_mut() else { return };
        if !o.answered(key, Err(why.clone())) {
            o.shelf().failed(key, why.clone());
            // a medium (m:) or its signed link (s:, #470) – either way the medium is missing
            o.medium(key.strip_prefix("m:").or_else(|| key.strip_prefix("s:")).unwrap_or(""), Some(why));
        }
    }

    /// the set a cell asks the space to show ("" = none new) – the page fetches it and hands it over (`give_set`)
    pub fn set_request(&mut self) -> String {
        self.set_wanted.take().unwrap_or_default()
    }

    /// a scene was loaded in place since the last call: the number its chapters are given with (`chapters()`' index is
    /// added), else -1
    pub fn reloaded(&mut self) -> i32 {
        if std::mem::take(&mut self.reloaded) {
            (self.turn * TURN) as i32
        } else {
            -1
        }
    }

    /// `index` of this show, as the page hears it
    fn numbered(&self, index: usize) -> u32 {
        (self.turn * TURN + index) as u32
    }

    /// the index an answer of the page is for; None = for a show that is gone
    fn mine(&self, numbered: usize) -> Option<usize> {
        (numbered / TURN == self.turn).then_some(numbered % TURN)
    }

    /// "webgpu" or "webgl2"
    pub fn backend(&self) -> String {
        self.backend.clone()
    }

    /// the set the space node floats in ("" = no space, or none)
    pub fn room(&self) -> String {
        self.show.room.clone()
    }

    /// the set's .glb and its images, decoded by the browser (`SpaceView.encoded_images`)
    pub fn give_set(&mut self, glb_bytes: &[u8], images: js_sys::Array) -> Result<(), JsValue> {
        match self.show.space.as_mut() {
            Some(s) => set_from_page(s, glb_bytes, images),
            None => Ok(()),
        }
    }

    /// the pictures the stage asks for (lib.rs `portrait_asks`: slot, kind, id, field, mood, …)
    pub fn portrait_requests(&mut self) -> js_sys::Array {
        let asks = self.show.space.as_mut().map(portrait_asks).unwrap_or_default();
        for i in (0..asks.length()).step_by(crate::ASK) {
            asks.set(i, JsValue::from(self.numbered(asks.get(i).as_f64().unwrap_or(0.0) as usize)));
        }
        asks
    }

    /// the cast's 3D models the stage asks for (lib.rs `figure_asks`: slot, kind, id, field, idle field, …) – the page fetches each
    /// file and hands it in (`give_figure`)
    pub fn figure_requests(&mut self) -> js_sys::Array {
        let asks = self.show.space.as_mut().map(figure_asks).unwrap_or_default();
        for i in (0..asks.length()).step_by(crate::FIGURE_ASK) {
            asks.set(i, JsValue::from(self.numbered(asks.get(i).as_f64().unwrap_or(0.0) as usize)));
        }
        asks
    }

    /// a figure's .glb and its images, decoded by the browser (`SpaceView.encoded_images`), and its idle's .glb (#817)
    pub fn give_figure(&mut self, slot: u32, glb_bytes: &[u8], images: js_sys::Array, idle: Option<Vec<u8>>) {
        if let (Some(slot), Some(s)) = (self.mine(slot as usize), self.show.space.as_mut()) {
            figure_from_page(s, slot, glb_bytes, images, idle.as_deref());
        }
    }

    pub fn no_figure(&mut self, slot: u32, why: String) {
        if let (Some(slot), Some(s)) = (self.mine(slot as usize), self.show.space.as_mut()) {
            s.figures.deliver(slot, Err(why));
        }
    }

    pub fn give_portrait(&mut self, slot: u32, width: u32, height: u32, rgba: Vec<u8>) {
        let img = image::RgbaImage::from_raw(width, height, rgba).ok_or_else(|| "a sprite of the wrong size".to_string());
        if let (Some(slot), Some(s)) = (self.mine(slot as usize), self.show.space.as_mut()) {
            s.portraits.deliver(slot, img);
        }
    }

    pub fn no_portrait(&mut self, slot: u32, why: String) {
        if let (Some(slot), Some(s)) = (self.mine(slot as usize), self.show.space.as_mut()) {
            s.portraits.deliver(slot, Err(why));
        }
    }

    /// the chapters to fetch, per chapter node: their paths under the library API
    pub fn chapters(&self) -> Vec<String> {
        self.show.pumps.iter().map(|p| p.source.api_path()).collect()
    }

    /// chapter node `i`'s chapter as the API answered (JSON)
    pub fn give_chapter(&mut self, i: usize, text: &str) -> Result<(), JsValue> {
        let Some(p) = self.mine(i).and_then(|i| self.show.pumps.get_mut(i)) else { return Ok(()) };
        let ch: Chapter = serde_json::from_str(text).map_err(err)?;
        p.strip = ch.strip(&p.source.quality);
        let line = if p.strip.is_empty() { format!("'{}' has no slices", ch.title) } else { format!("'{}': {} slices", ch.title, p.strip.len()) };
        p.say(line);
        Ok(())
    }

    /// the slices to fetch now: chapter node, its CDN path, …
    pub fn slice_requests(&mut self) -> js_sys::Array {
        let out = js_sys::Array::new();
        let (now, turn) = (self.now, self.turn * TURN);
        for (i, p) in self.show.pumps.iter_mut().enumerate() {
            if p.strip.is_empty() || p.flying || p.held.is_some() || now < p.wait_until {
                continue;
            }
            p.flying = true;
            out.push(&JsValue::from((turn + i) as u32));
            out.push(&JsValue::from_str(&format!("{}/{}", p.source.toon, p.strip[p.at()].0)));
        }
        out
    }

    /// chapter node `i`'s slice, decoded by the browser
    pub fn give_slice(&mut self, i: usize, width: u32, height: u32, rgba: Vec<u8>) -> Result<(), JsValue> {
        let Some(p) = self.mine(i).and_then(|i| self.show.pumps.get_mut(i)) else { return Ok(()) };
        let img = image::RgbaImage::from_raw(width, height, rgba).ok_or_else(|| err("a slice of the wrong size"))?;
        let (index, count) = (p.at(), p.strip.len());
        let (slice, caption) = finish_slice(img, &p.strip[index].1, p.source.bake);
        p.held = Some(Prepared { index, count, slice, caption });
        p.next += 1;
        p.flying = false;
        p.failed = 0;
        p.ready.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// chapter node `i`'s slice could not be fetched: the next one; when none loads, again later
    pub fn no_slice(&mut self, i: usize, why: String) {
        let now = self.now;
        let Some(p) = self.mine(i).and_then(|i| self.show.pumps.get_mut(i)) else { return };
        p.next += 1;
        p.flying = false;
        p.failed += 1;
        if p.failed >= p.strip.len() {
            p.failed = 0;
            p.wait_until = now + RETRY_S;
            p.say(format!("no slice loads ({why}) – again in {RETRY_S} s"));
        }
    }

    /// the video nodes' files to fetch, once per scene: video node, its media path, …
    pub fn video_requests(&mut self) -> js_sys::Array {
        let out = js_sys::Array::new();
        if std::mem::replace(&mut self.show.videos_asked, true) {
            return out;
        }
        let asks: Vec<(usize, String)> = self.show.stage.video_sources(&self.show.graph).into_iter().filter_map(|(n, src)| Some((n, src?.to_string()))).collect();
        for (node, src) in asks {
            out.push(&JsValue::from(self.numbered(node)));
            out.push(&JsValue::from_str(&src));
        }
        out
    }

    /// video node `node`'s file, as fetched
    pub fn give_video(&mut self, node: usize, bytes: Vec<u8>) {
        let Some(node) = self.mine(node) else { return };
        match deck_space::video::Movie::open(bytes) {
            Ok(movie) => self.show.stage.set_video(node, movie),
            Err(e) => self.no_video(self.numbered(node) as usize, e),
        }
    }

    /// video node `node`'s file did not come or does not play: the node stays dark
    pub fn no_video(&mut self, node: usize, why: String) {
        if let Some(node) = self.mine(node) {
            web_sys::console::warn_1(&JsValue::from_str(&format!("video {}: {why}", self.show.graph.nodes[node].id)));
        }
    }

    /// the display's new size in device pixels
    pub fn resize(&mut self, w: u32, h: u32) {
        let (w, h) = (w.clamp(1, MAX_SIDE), h.clamp(1, MAX_SIDE));
        if (w, h) == (self.config.width, self.config.height) {
            return;
        }
        self.canvas.set_width(w);
        self.canvas.set_height(h);
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.show.resize(&self.device, (w, h));
    }

    /// switch, the load: the scene `text` is built behind the veil (the page's GPU compiles between frames); `frame` takes it
    fn load(&mut self, text: String) {
        self.building = true;
        let (device, queue, format, size, built) = (self.device.clone(), self.queue.clone(), self.config.format, (self.config.width, self.config.height), self.built.clone());
        wasm_bindgen_futures::spawn_local(async move {
            let show = Show::build(&device, &queue, format, size, &text).await;
            *built.borrow_mut() = Some(show);
        });
    }

    /// switch, the change: the built show plays – the outlet's planes go to its stage and the cell on is composed for it,
    /// the page fetches its set and chapters, the menu is painted again
    fn play(&mut self, mut show: Show) {
        let size = (self.config.width, self.config.height);
        if show.size != size {
            show.resize(&self.device, size);
        }
        self.show = show;
        // what its cast is drawn by: scene-content's answer in the scene's kept copy (each look's form and model)
        let content = self.kept_now().and_then(deck_space::published::content_of);
        if let (Some(c), Some(s)) = (content, self.show.space.as_mut()) {
            s.set_looks(&c);
        }
        self.face.again();
        self.ballot.again();
        self.turn += 1;
        self.reloaded = true;
        self.age = 0.0;
        self.set_wanted = Some(self.show.room.clone()).filter(|r| !r.is_empty());
        if let Some(o) = self.outlet.as_mut() {
            let (tx, rx) = sync_channel(2);
            self.show.stage.set_outlet(rx);
            o.rewire(tx);
        }
        self.menu.again();
    }

    /// The piece side's frame (#473): the outlet's cells to the core, what it asks to switch to, the gliding values into
    /// the playing scene. A scene asked for while a change runs waits for it (the newest wins).
    fn pieces(&mut self, time: f32) {
        let (Some(o), Some(side)) = (self.outlet.as_mut(), self.side.as_mut()) else { return };
        let now = time as f64 * 1000.0;
        let mut cells = std::mem::take(&mut o.pieces);
        if cells.is_empty() && self.held.as_ref().is_some_and(|(_, place)| !o.fetched.waits(place)) {
            cells.extend(self.held.take().map(|(cell, _)| cell));
        }
        let mut switched = false;
        for cell in cells {
            let mut waits = None;
            let said = side.cell(&cell, now, |p| {
                let found = crate::outlet::find(&mut o.fetched, p);
                waits = (found.is_none() && o.fetched.waits(p)).then(|| p.clone());
                found
            });
            // the piece is asked for (deck-space fetched.rs): what plays stays, the cell waits for the answer
            self.held = waits.map(|place| (cell, place));
            if let Some((_, place)) = &self.held {
                o.status.set_piece(&format!("fetching {place}"), &[]);
                continue;
            }
            o.status.set_piece(&said.did, &said.problems);
            switched |= said.switch.is_some();
            match said.switch {
                Some(Switch::Scene(text)) => self.change.ask(&text),
                Some(Switch::Deck) => self.change.ask(&self.deck),
                None => {}
            }
        }
        for (path, v) in side.values_at(now) {
            // a value whose node the scene lacks was said when the cell came (piece.rs `resolve`)
            self.show.controls.set_path(&self.show.graph, &path, v);
        }
        // #538: another scene plays – the link's cast follows it (a cell's published scene listens as a `?scene=` does)
        if switched {
            self.cast_play();
        }
    }

    /// The deck's panel (#481): what the flash side says to the person at the deck, drawn by the core (deck-show
    /// `face.rs`: the component library's parts, a layer the kit paints through egui, #485) and handed to the stage –
    /// only when it changed.
    fn panel(&mut self) {
        let shown = self.outlet.as_ref().and_then(|o| o.shown());
        let size = (self.config.width as f32, self.config.height as f32);
        if let Some(layer) = self.face.paint(shown.as_ref(), size, deck_outlet::text::mono_per_px()) {
            self.show.stage.set_panel(layer);
        }
    }

    /// The ballot (#556): the vote this deck may cast in, drawn by the core (deck-ui `ballot.rs`: the panel's parts, a
    /// layer under the menu and the panel) over the scene that plays on – handed to the stage only when it changed.
    fn ballot_paint(&mut self) {
        let view = self.outlet.as_ref().and_then(|o| o.ballot());
        let size = (self.config.width as f32, self.config.height as f32);
        if let Some(layer) = self.ballot.paint(view.as_ref(), size, deck_outlet::text::mono_per_px()) {
            self.show.stage.set_vote(layer);
        }
    }

    /// #556: an input the menu left (a key, the pointer's press – deck-kit's `Input`), for the ballot – unless the Live
    /// page is open (its keys are its own); whether the ballot took it. Its cast goes to the relay client.
    pub(super) fn ballot_input(&mut self, i: deck_ui::menu::Input) -> bool {
        if self.live.open {
            return false;
        }
        let Some(o) = self.outlet.as_mut() else { return false };
        let (took, did) = self.ballot.input(i, o.ballot().as_ref());
        live_do(o, did);
        took
    }

    /// #481: a dialog of the flash side shows (its keys are its own, also over the open Live page)
    fn dialog(&self) -> bool {
        self.outlet.as_ref().is_some_and(|o| matches!(o.shown(), Some(deck_show::dialog::Shown::Dialog(_))))
    }

    /// #481: a tap or a click at (x, y) of the canvas, in its pixels: what the deck's UI has there is pressed (a
    /// button, a switch, a field, a toast); #496: else the stage's – the cast member under it is picked, or the pick
    /// dropped (deck-space pick.rs). true = it was the deck's.
    pub fn panel_tap(&mut self, x: f32, y: f32) -> bool {
        let face = &mut self.face;
        let ui = self.outlet.as_mut().is_some_and(|o| {
            let (flash, fetched) = o.flash_parts();
            face.tap(x, y, flash, fetched)
        });
        ui || self.show.space.as_mut().is_some_and(|s| s.tap(x, y))
    }

    /// #481: a key of the page, by its `KeyboardEvent.key` (`shift` held). true = it was the deck's UI's (a dialog's
    /// key, `f` for "please don't flash") – #496: or the stage's pick's (Tab, Escape).
    pub fn panel_key(&mut self, key: &str, shift: bool) -> bool {
        // the open Live page (#478) has the keys – a name is typed there – unless a dialog shows over it
        if self.live.open && !self.dialog() {
            return false;
        }
        let face = &mut self.face;
        let ui = self.outlet.as_mut().is_some_and(|o| {
            let (flash, fetched) = o.flash_parts();
            face.key(key, flash, fetched)
        });
        ui || self.show.space.as_mut().is_some_and(|s| s.key(key, shift))
    }

    /// #481: the reader signed in or out: the deck asks for its proof again at once (who may reject a flash is in it)
    pub fn proof_again(&mut self) {
        if let Some(o) = self.outlet.as_mut() {
            o.proof_again();
        }
    }

    /// The change through black (veil.rs), one frame: Some = the veil's opacity.
    fn through_black(&mut self, dt: f32) -> Option<f32> {
        if let Some(text) = self.change.due() {
            self.change.start(&text);
        }
        let built = self.built.borrow_mut().take();
        if let Some(built) = built {
            self.building = false;
            match built {
                Ok(show) => self.play(show),
                // what played stays, and the HUD's piece line says why
                Err(e) => {
                    if let Some(o) = self.outlet.as_mut() {
                        o.status.set_piece("not loaded", &[e]);
                    }
                }
            }
        }
        let ready = !self.building && (self.show.stage.slides_ready() || self.age >= SPLASH_AT_MOST);
        let (load, veil) = self.change.step(dt, ready);
        if let Some(text) = load {
            self.load(text);
        }
        veil
    }

    /// One frame: `dt` since the last (s), `time` the page's clock (s).
    pub fn frame(&mut self, dt: f32, time: f32) -> Result<(), JsValue> {
        self.now = time;
        let dt = dt.clamp(0.0, 0.1);
        self.published();
        self.pieces(time);
        self.cast_turn();
        self.age += dt;
        let veil = self.through_black(dt);
        for p in &mut self.show.pumps {
            if let Some(ready) = p.held.take() {
                if let Err(std::sync::mpsc::TrySendError::Full(back)) = p.tx.try_send(ready) {
                    p.held = Some(back);
                }
            }
        }
        if let Some(o) = self.outlet.as_mut() {
            o.frame(Duration::from_secs_f32(time.max(0.0)), (self.config.width, self.config.height));
            if let Some(s) = self.show.space.as_mut() {
                o.held.drain(..).for_each(|h| s.held(h));
            }
            if let (Some(t), Some(s)) = (o.space.take(), self.show.space.as_mut()) {
                s.show(None, t.spot.as_deref());
                let set = t.set.map(|p| p.to_string_lossy().into_owned()).filter(|set| *set != self.show.room);
                if let Some(set) = set {
                    self.show.room = set.clone();
                    self.set_wanted = Some(set);
                }
            }
        }
        self.panel();
        self.ballot_paint();
        self.menu_paint();
        let show = &mut self.show;
        if show.graph.first(Kind::Tempo).is_some() {
            self.beats += dt as f64 * show.bpm / 60.0;
            self.beat = tempo::Now { bpm: show.bpm, pos: tempo::position(self.beats, show.per_bar), per_bar: show.per_bar, running: true };
        }
        show.controls.update(dt, 0, &self.beat);
        if let Some(s) = show.space.as_mut() {
            s.update(dt, time, &mut show.controls, &self.beat, &|_, _| 0.0);
        }
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            other => return Err(err(format!("the canvas's surface: {other:?}"))),
        };
        let (graph, controls) = (&show.graph, &show.controls);
        let input = |n: usize, p: &str, d: f32| controls.input(n, p, d);
        let mut enc = self.device.create_command_encoder(&Default::default());
        let view = frame.texture.create_view(&Default::default());
        // the splash waits for a chapter's first slice, as on the native deck, but not for ever; a scene change is a black
        // veil without the logo (#250)
        // #386: a deck in no session keeps its splash until START (the menu's splash)
        self.fade.step(veil.is_none() && (!show.stage.slides_ready() && self.age < SPLASH_AT_MOST || self.menu.on_splash()), dt);
        show.stage.veil_only = veil.is_some();
        let splash = (veil.unwrap_or(self.fade.alpha), false);
        // #478: the Live page on the HUD, the native deck's raster, its cursor's line lit
        if let (true, Some(o)) = (self.live.open, self.outlet.as_ref()) {
            let (lines, cursor) = self.live.lines(&o.relay.live());
            self.hud.set_lines(lines);
            self.hud.set_alert(Some(Alert { row: cursor, lit: true }));
            self.hud.raster();
        }
        let hud = self.live.open.then_some(&self.hud);
        let hud_at = Some(deck_space::hud::page_rect(self.config.width as f32, self.config.height as f32, self.live_at.0, self.live_at.1).0);
        show.stage.frame(&self.device, &self.queue, &mut enc, &view, &Tick { graph, input: &input, tempo: &self.beat, time, dt }, &Over { hud, splash, label_centred: true, label_hidden: false, hud_at }, show.space.as_mut());
        self.queue.submit([enc.finish()]);
        self.queue.present(frame);
        Ok(())
    }
}

/// #520: a published scene's link to its session (deck-space published.rs `CastLink`), driven by the page's socket
/// (client.ts `RelaySocket` – the names are relay-web's); what comes for the scene the view takes (`SceneView::frame`).
#[wasm_bindgen]
pub struct SceneLink {
    link: Rc<RefCell<CastLink>>,
}

#[wasm_bindgen]
impl SceneLink {
    pub fn start(&self, now: f64) {
        self.link.borrow_mut().relay_at(now).start(now);
    }

    pub fn opened(&self, now: f64) {
        self.link.borrow_mut().relay_at(now).opened(now);
    }

    pub fn closed(&self, now: f64, why: &str) {
        self.link.borrow_mut().relay_at(now).closed(now, why);
    }

    pub fn text(&self, now: f64, data: &str) {
        self.link.borrow_mut().relay_at(now).text(now, data);
    }

    pub fn quic_opened(&self, now: f64) {
        self.link.borrow_mut().relay_at(now).quic_opened(now);
    }

    pub fn quic_closed(&self, now: f64, why: &str) {
        self.link.borrow_mut().relay_at(now).quic_closed(now, why);
    }

    pub fn quic_stream(&self, now: f64, bytes: &[u8]) {
        self.link.borrow_mut().relay_at(now).quic_stream(now, bytes);
    }

    pub fn quic_datagram(&self, now: f64, bytes: &[u8]) {
        self.link.borrow_mut().relay_at(now).quic_datagram(now, bytes);
    }

    pub fn tick(&self, now: f64) {
        self.link.borrow_mut().relay_at(now).tick(now);
    }

    pub fn stop(&self) {
        self.link.borrow_mut().relay().stop();
    }

    pub fn next_wake(&self) -> Option<f64> {
        self.link.borrow().client().next_wake()
    }

    /// what the socket should do (JSON, as relay-web's `take`); what came for the scene stays here
    pub fn take(&self) -> String {
        serde_json::to_string(&self.link.borrow_mut().take()).unwrap_or_else(|_| "[]".into())
    }

    pub fn line(&self) -> String {
        self.link.borrow().client().line()
    }
}
