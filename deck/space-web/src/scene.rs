//! A scene in a browser canvas (#463, #465): the renderer every deck draws with (deck-space `stage`) on the page's GPU, the
//! scene as the native deck plays it, the splash and the build label over it; the HUD is off. No scene given = the deck's
//! idle scene, as on the native deck. The control domain is deck-engine's, the clock the scene's own tempo. The page
//! only fetches and decodes what is asked for: the space's set and portraits, a chapter and its slices, an outlet's media.
//! An `outlet` node joins the relay once the page names the outlet (`join_outlet`, outlet.rs). Not yet: the sound.
use crate::outlet::Outlet;
use crate::{err, frame_for, open_canvas, portrait_asks, set_from_page, CanvasGpu, SPLASH_AT_MOST};
use deck_engine::control::Controls;
use deck_space::chapter::{finish_slice, Chapter, Dialog, Source};
use deck_space::graph::{Graph, Kind};
use deck_space::quality::MAX_SIDE;
use deck_space::scene::{builtin, Scene, IDLE_SCENE};
use deck_space::space::portrait::Portraits;
use deck_space::space::Space;
use deck_space::splash::Fade;
use deck_space::stage::{ChapterFeed, Over, Prepared, Stage, Tick};
use deck_space::tempo;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wasm_bindgen::prelude::*;

/// a chapter that loads nothing is asked again after this (s)
const RETRY_S: f32 = 10.0;

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

#[wasm_bindgen]
pub struct SceneView {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    canvas: web_sys::HtmlCanvasElement,
    graph: Graph,
    stage: Stage,
    controls: Controls,
    space: Option<Space>,
    room: String,
    pumps: Vec<Pump>,
    beats: f64,
    bpm: f64,
    per_bar: u32,
    beat: tempo::Now,
    now: f32,
    age: f32,
    fade: Fade,
    backend: String,
    /// #465: the Live outlet of an `outlet` node, once joined
    outlet: Option<Outlet>,
    /// a set the space should show next (the page fetches it)
    set_wanted: Option<String>,
    /// #470: the pairing card's code ("" = no card)
    pairing_code: String,
}

#[wasm_bindgen]
impl SceneView {
    /// `scene` on `canvas` (its size in device pixels is the canvas' width / height): a built-in's name, a scene document,
    /// or "" = the idle scene. `prefer` as `SpaceView::create`.
    pub async fn create(canvas: web_sys::HtmlCanvasElement, scene: String, prefer: String) -> Result<SceneView, JsValue> {
        let text = if scene.is_empty() { IDLE_SCENE } else { builtin(&scene).unwrap_or(&scene) };
        let scene = Scene::parse(text).map_err(err)?;
        let mut graph = Graph::build(&scene).map_err(err)?;
        let tempo = deck_engine::tempo_config(&graph).map_err(err)?;
        // the page has no relay session of a scene's own
        let space_node = graph.first(Kind::Space);
        if let Some(s) = space_node {
            graph.nodes[s].params.remove("join");
        }
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
        };
        surface.configure(&device, &config);
        let scale = scene.stage.render_scale;
        let mut stage = Stage::new(&device, &queue, format, (w, h), scale, scale, false).await.map_err(err)?;
        let (mut feeds, mut pumps) = (Vec::new(), Vec::new());
        for (i, node) in graph.nodes.iter().enumerate().filter(|(_, n)| n.kind == Kind::Chapter) {
            let (tx, rx) = sync_channel(1);
            let (status, ready) = (Arc::new(Mutex::new("waiting".to_string())), Arc::new(AtomicBool::new(false)));
            feeds.push(ChapterFeed { node: i, rx, status: status.clone(), ready: ready.clone() });
            let source = Source::of(node);
            let next = source.start;
            pumps.push(Pump { source, tx, status, ready, strip: Vec::new(), next, flying: false, held: None, failed: 0, wait_until: 0.0 });
        }
        stage.load(&device, format, &graph, feeds).await.map_err(err)?;
        let mut room = String::new();
        let space = space_node.map(|n| {
            room = graph.nodes[n].str("set").unwrap_or_default().to_string();
            let mut s = Space::new(&graph, n, &scene.being, std::path::Path::new(""), Portraits::manual());
            s.set_view(w as f32, h as f32);
            frame_for(&mut s, w, h);
            s
        });
        let controls = Controls::new(&graph, None);
        let bpm = tempo.bpm;
        let per_bar = tempo.per_bar;
        Ok(SceneView { surface, device, queue, config, canvas: element, graph, stage, controls, space, room, pumps, beats: 0.0, bpm, per_bar, beat: tempo::Now::default(), now: 0.0, age: 0.0, fade: Fade { alpha: 1.0 }, backend: backend.into(), outlet: None, set_wanted: None, pairing_code: String::new() })
    }

    /// #465: join `relay` ("" = the hosted show) as output of `outlet`, its text in `locale`, show media's signed links from
    /// `api` (#470) - when the scene has an `outlet` node; false: it has none
    pub fn join_outlet(&mut self, relay: &str, outlet: &str, locale: &str, api: &str) -> Result<bool, JsValue> {
        if self.graph.first(Kind::Outlet).is_none() {
            return Ok(false);
        }
        let (tx, rx) = sync_channel(2);
        let o = Outlet::new(relay, outlet, if locale.is_empty() { "de" } else { locale }, api, (self.config.width, self.config.height), tx).map_err(err)?;
        self.stage.set_outlet(rx);
        self.outlet = Some(o);
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
    /// m: a medium's URL - never with the token, s: a show medium's signed link from the API - with the deck's token, #470)
    pub fn outlet_asks(&mut self) -> js_sys::Array {
        let out = js_sys::Array::new();
        for (k, path) in self.outlet.as_mut().map(|o| o.asks()).unwrap_or_default() {
            out.push(&JsValue::from_str(&k));
            out.push(&JsValue::from_str(&path));
        }
        out
    }

    pub fn outlet_text(&mut self, key: &str, text: &str) {
        if let Some(o) = self.outlet.as_mut() {
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

    /// #470: the API's answer to an `s:` ask (its status and body) - the signed link is asked next as `m:`
    pub fn outlet_signed(&mut self, key: &str, status: u16, body: &str) {
        if let (Some(o), Some(id)) = (self.outlet.as_mut(), key.strip_prefix("s:")) {
            o.signed(id, status, body);
        }
    }

    /// #470: the deck's link (DeckLink) - why it has no token ("" = it has one) and the code to pair with ("" = none: the
    /// pairing card hides); `app` = the viewer the member pairs on
    pub fn deck_link(&mut self, why: &str, code: &str, app: &str) {
        if let Some(o) = self.outlet.as_mut() {
            if o.unpaired != why {
                o.unpaired = why.to_owned();
            }
        }
        if self.pairing_code != code {
            self.pairing_code = code.to_owned();
            let shown = (!code.is_empty()).then(|| deck_link::pairing::Shown::Pending { code: code.to_owned() });
            let card = deck_space::pairing::Card::of(shown.as_ref(), app).map(|c| c.raster());
            self.stage.set_pairing(&self.queue, card.as_deref());
        }
    }

    pub fn outlet_failed(&mut self, key: &str, why: String) {
        if let Some(o) = self.outlet.as_mut() {
            o.shelf().failed(key, why.clone());
            // a medium (m:) or its signed link (s:, #470) – either way the medium is missing
            o.medium(key.strip_prefix("m:").or_else(|| key.strip_prefix("s:")).unwrap_or(""), Some(why));
        }
    }

    /// the set a cell asks the space to show ("" = none new) – the page fetches it and hands it over (`give_set`)
    pub fn set_request(&mut self) -> String {
        self.set_wanted.take().unwrap_or_default()
    }

    /// "webgpu" or "webgl2"
    pub fn backend(&self) -> String {
        self.backend.clone()
    }

    /// the set the space node floats in ("" = no space, or none)
    pub fn room(&self) -> String {
        self.room.clone()
    }

    /// the set's .glb and its images, decoded by the browser (`SpaceView.encoded_images`)
    pub fn give_set(&mut self, glb_bytes: &[u8], images: js_sys::Array) -> Result<(), JsValue> {
        match self.space.as_mut() {
            Some(s) => set_from_page(s, glb_bytes, images),
            None => Ok(()),
        }
    }

    /// the sprites the heads ask for: slot, character, mood, …
    pub fn portrait_requests(&mut self) -> js_sys::Array {
        self.space.as_mut().map(portrait_asks).unwrap_or_default()
    }

    pub fn give_portrait(&mut self, slot: u32, width: u32, height: u32, rgba: Vec<u8>) {
        let img = image::RgbaImage::from_raw(width, height, rgba).ok_or_else(|| "a sprite of the wrong size".to_string());
        if let Some(s) = self.space.as_mut() {
            s.portraits.deliver(slot as usize, img);
        }
    }

    pub fn no_portrait(&mut self, slot: u32, why: String) {
        if let Some(s) = self.space.as_mut() {
            s.portraits.deliver(slot as usize, Err(why));
        }
    }

    /// the chapters to fetch, per chapter node: their paths under the library API
    pub fn chapters(&self) -> Vec<String> {
        self.pumps.iter().map(|p| p.source.api_path()).collect()
    }

    /// chapter node `i`'s chapter as the API answered (JSON)
    pub fn give_chapter(&mut self, i: usize, text: &str) -> Result<(), JsValue> {
        let ch: Chapter = serde_json::from_str(text).map_err(err)?;
        let p = &mut self.pumps[i];
        p.strip = ch.strip(&p.source.quality);
        let line = if p.strip.is_empty() { format!("'{}' has no slices", ch.title) } else { format!("'{}': {} slices", ch.title, p.strip.len()) };
        p.say(line);
        Ok(())
    }

    /// the slices to fetch now: chapter node, its CDN path, …
    pub fn slice_requests(&mut self) -> js_sys::Array {
        let out = js_sys::Array::new();
        let now = self.now;
        for (i, p) in self.pumps.iter_mut().enumerate() {
            if p.strip.is_empty() || p.flying || p.held.is_some() || now < p.wait_until {
                continue;
            }
            p.flying = true;
            out.push(&JsValue::from(i as u32));
            out.push(&JsValue::from_str(&format!("{}/{}", p.source.toon, p.strip[p.at()].0)));
        }
        out
    }

    /// chapter node `i`'s slice, decoded by the browser
    pub fn give_slice(&mut self, i: usize, width: u32, height: u32, rgba: Vec<u8>) -> Result<(), JsValue> {
        let img = image::RgbaImage::from_raw(width, height, rgba).ok_or_else(|| err("a slice of the wrong size"))?;
        let p = &mut self.pumps[i];
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
        let p = &mut self.pumps[i];
        p.next += 1;
        p.flying = false;
        p.failed += 1;
        if p.failed >= p.strip.len() {
            p.failed = 0;
            p.wait_until = now + RETRY_S;
            p.say(format!("no slice loads ({why}) – again in {RETRY_S} s"));
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
        self.stage.resize(&self.device, (w, h));
        if let Some(s) = self.space.as_mut() {
            s.set_view(w as f32, h as f32);
            frame_for(s, w, h);
        }
    }

    /// One frame: `dt` since the last (s), `time` the page's clock (s).
    pub fn frame(&mut self, dt: f32, time: f32) -> Result<(), JsValue> {
        self.now = time;
        let dt = dt.clamp(0.0, 0.1);
        for p in &mut self.pumps {
            if let Some(ready) = p.held.take() {
                if let Err(std::sync::mpsc::TrySendError::Full(back)) = p.tx.try_send(ready) {
                    p.held = Some(back);
                }
            }
        }
        if let Some(o) = self.outlet.as_mut() {
            o.frame(Duration::from_secs_f32(time.max(0.0)), (self.config.width, self.config.height));
            if let (Some(t), Some(s)) = (o.space.take(), self.space.as_mut()) {
                s.show(None, t.spot.as_deref());
                let set = t.set.map(|p| p.to_string_lossy().into_owned()).filter(|set| *set != self.room);
                if let Some(set) = set {
                    self.room = set.clone();
                    self.set_wanted = Some(set);
                }
            }
        }
        if self.graph.first(Kind::Tempo).is_some() {
            self.beats += dt as f64 * self.bpm / 60.0;
            self.beat = tempo::Now { bpm: self.bpm, pos: tempo::position(self.beats, self.per_bar), per_bar: self.per_bar, running: true };
        }
        self.controls.update(dt, 0, &self.beat);
        if let Some(s) = self.space.as_mut() {
            s.update(dt, time, &mut self.controls, &self.beat, &|_, _| 0.0);
        }
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(e) => return Err(err(e)),
        };
        let (graph, controls) = (&self.graph, &self.controls);
        let input = |n: usize, p: &str, d: f32| controls.input(n, p, d);
        let mut enc = self.device.create_command_encoder(&Default::default());
        let view = frame.texture.create_view(&Default::default());
        // the splash waits for a chapter's first slice, as on the native deck, but not for ever
        self.age += dt;
        self.fade.step(!self.stage.slides_ready() && self.age < SPLASH_AT_MOST, dt);
        self.stage.frame(&self.device, &self.queue, &mut enc, &view, &Tick { graph, input: &input, tempo: &self.beat, time, dt }, &Over { hud: None, splash: (self.fade.alpha, false), label_centred: true }, self.space.as_mut());
        self.queue.submit([enc.finish()]);
        frame.present();
        Ok(())
    }
}
