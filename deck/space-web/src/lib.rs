//! space-web (#198): deck-space's being and set in a browser canvas – the same code as the deck's stage, compiled to
//! wasm32. The page (shared/space-web's `SpaceView` wrapper, the viewer's /live/deck) drives it:
//!
//! - **The being** comes from the deck's `deck.graph` (what the page streams to the relay, wire.rs `from_wire`), in the
//!   cathedral's space (its `at`, camera, light – scenes/cathedral.toml) with the cathedral's look; the page's values
//!   seed its buttons.
//! - **One state:** every frame the page writes its values (`set` – the paths it streams, inputs and outputs) and its
//!   mood; the being's buttons turn the same values (a drag, a double click's ease back) and the page reads them back
//!   (`changes`) – into its sliders, its sound and its stream.
//! - **Assets:** the page fetches the set's .glb and the portraits (scene-content + the CDN) and lets the browser
//!   decode the images (`encoded_images` → `give_set`, `portrait_requests` → `give_portrait`); the glb is parsed here.
//! - **Input:** pointer and touch events go to the stage's own picking, drag and double click (`Space`).
//! - **GPU:** WebGPU when the browser has an adapter (wgpu's detection asks for one – a `navigator.gpu` without an
//!   adapter, like the S22's Chrome, counts as none), else WebGL2; `prefer` = "webgl2" skips WebGPU.
#![cfg(target_arch = "wasm32")]

use deck_space::graph::{Graph, Kind};
use deck_space::scene::{Scene, CATHEDRAL_SCENE};
use deck_space::space::being::Look;
use deck_space::space::glb;
use deck_space::space::gpu::SpaceGpu;
use deck_space::space::portrait::Portraits;
use deck_space::space::values::{self, Knobs, Mirror, Path as ValuePath, Values};
use deck_space::space::{wire, Cursor, Space, Touch};
use deck_space::tempo;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// The page's values as the being sees them: a mirror of the deck's graph the page writes every frame, and what the
/// being's hand turned since the page last asked (`changes`).
struct Page {
    mirror: Mirror,
    turned: Vec<ValuePath>,
}

impl Page {
    fn touched(&mut self, p: ValuePath) {
        if !self.turned.contains(&p) {
            self.turned.push(p);
        }
    }
}

impl Knobs for Page {
    fn input(&self, node: usize, port: &str, default: f32) -> f32 {
        Values::input(&self.mirror, node, port, default)
    }
    fn outputs(&self, node: usize) -> &[f32] {
        self.mirror.outputs_of(node)
    }
    fn lfo_shape(&self, node: usize) -> Option<usize> {
        Values::lfo_shape(&self.mirror, node)
    }
    fn set(&mut self, node: usize, port: usize, value: f32) -> bool {
        let ok = self.mirror.set_input(node, port, value);
        if ok {
            self.touched(ValuePath::Input(node, port));
        }
        ok
    }
    fn set_lfo_shape(&mut self, node: usize, shape: usize) -> bool {
        let ok = self.mirror.set_shape(node, shape);
        if ok {
            self.touched(ValuePath::Shape(node));
        }
        ok
    }
}

fn err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

/// The deck's being from its wire graph, in the cathedral: the space node takes the cathedral's (where the being
/// floats, the camera, the set – not its `join`, the page streams), the look the cathedral's (its drift, breath …)
/// with the wire's character, moods, size and exposed parameters; the page's starting values become the nodes' params
/// (what a double click eases back to). No feelings: the page's mood is the head's.
fn being_of(deck_graph: &str, values_json: &str) -> anyhow::Result<(Graph, Look)> {
    let g: protocol::DeckGraph = serde_json::from_str(deck_graph)?;
    let (mut graph, wired) = wire::from_wire(&g)?;
    let cathedral = Scene::parse(CATHEDRAL_SCENE)?;
    if let (Some(s), Some(c)) = (graph.first(Kind::Space), cathedral.nodes.iter().find(|n| n.kind == "space")) {
        let mut params = c.params.clone();
        params.remove("join");
        graph.nodes[s].params = params;
    }
    let start: HashMap<String, f64> = if values_json.is_empty() { HashMap::new() } else { serde_json::from_str(values_json)? };
    for (path, v) in start {
        if let Some(ValuePath::Input(n, k)) = values::resolve(&graph, &path) {
            let name = graph.nodes[n].kind.inputs()[k].name;
            graph.nodes[n].params.insert(name.into(), toml::Value::Float(v));
        }
    }
    let look = Look { size: wired.size, character: wired.character, moods: wired.moods, expose: wired.expose, feel: Vec::new(), react: Vec::new(), ..cathedral.being };
    Ok((graph, look))
}

/// a mirror holding the graph's params (the page's starting values) until the page writes its own
fn mirror_of(graph: &Graph) -> Mirror {
    let mut m = Mirror::new(graph);
    for (n, node) in graph.nodes.iter().enumerate() {
        for (k, port) in node.kind.inputs().iter().enumerate() {
            if node.params.contains_key(port.name) {
                m.set_input(n, k, node.f32(port.name, 0.0));
            }
        }
    }
    m
}

/// The native stage frames the being for a landscape screen (55° high); a phone held upright sees as much across as the
/// stage sees from top to bottom – the field of view widens with the height (at most 75°), the camera stays.
fn frame_for(space: &mut Space, w: u32, h: u32) {
    let aspect = w as f32 / h.max(1) as f32;
    let native = 55f32.to_radians();
    space.camera.fov_y = if aspect >= 1.0 { native } else { (2.0 * ((native * 0.5).tan() / aspect).atan()).min(75f32.to_radians()) };
}

#[wasm_bindgen]
pub struct SpaceView {
    space: Space,
    graph: Graph,
    page: Page,
    /// the audio outputs' peaks the page writes (the hoses' bubbles)
    meters: Vec<Vec<f32>>,
    beat: tempo::Now,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    gpu: Option<SpaceGpu>,
    backend: String,
    adapter: String,
}

#[wasm_bindgen]
impl SpaceView {
    /// The view on `canvas` (its size in device pixels is the canvas' width / height): the being of `deck_graph` (a
    /// `deck.graph` as JSON) with the page's starting `values` (JSON: path → value). `prefer`: "auto" (WebGPU if the
    /// browser has an adapter, else WebGL2) or "webgl2". A canvas that once had a WebGPU context can't give WebGL2 –
    /// the page retries on a fresh canvas with "webgl2" if this fails.
    pub async fn create(canvas: web_sys::HtmlCanvasElement, deck_graph: String, values: String, prefer: String) -> Result<SpaceView, JsValue> {
        let backends = if prefer == "webgl2" { wgpu::Backends::GL } else { wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL };
        let instance = wgpu::util::new_instance_with_webgpu_detection(&wgpu::InstanceDescriptor { backends, ..Default::default() }).await;
        let (w, h) = (canvas.width().max(1), canvas.height().max(1));
        let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas)).map_err(err)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .map_err(err)?;
        let info = adapter.get_info();
        let backend = match info.backend {
            wgpu::Backend::BrowserWebGpu => "webgpu",
            wgpu::Backend::Gl => "webgl2",
            _ => "other",
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("space-web"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(err)?;
        let caps = surface.get_capabilities(&adapter);
        let first = *caps.formats.first().ok_or_else(|| err("the canvas offers no format"))?;
        // the space's shaders write display values (the stage's offscreen target is Rgba8Unorm): a non-sRGB surface
        let format = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(first);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: w,
            height: h,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);
        let (graph, look) = being_of(&deck_graph, &values).map_err(err)?;
        let node = graph.first(Kind::Space).ok_or_else(|| err("the deck's graph has no space node"))?;
        let mut space = Space::new(&graph, node, &look, std::path::Path::new(""), Portraits::manual());
        space.set_view(w as f32, h as f32);
        frame_for(&mut space, w, h);
        let page = Page { mirror: mirror_of(&graph), turned: Vec::new() };
        let meters = graph.nodes.iter().map(|n| vec![0.0; n.kind.outputs().len()]).collect();
        // a browser's WebGPU adapter often has no name (privacy): the backend alone then
        let adapter = if info.name.is_empty() { backend.to_string() } else { format!("{} ({backend})", info.name) };
        Ok(SpaceView { space, graph, page, meters, beat: tempo::Now::default(), surface, device, queue, config, gpu: None, backend: backend.into(), adapter })
    }

    /// "webgpu" or "webgl2"
    pub fn backend(&self) -> String {
        self.backend.clone()
    }

    /// the adapter's name and backend (the page's status line)
    pub fn adapter(&self) -> String {
        self.adapter.clone()
    }

    /// the canvas' new size in device pixels
    pub fn resize(&mut self, w: u32, h: u32) {
        let (w, h) = (w.max(1), h.max(1));
        if (w, h) == (self.config.width, self.config.height) {
            return;
        }
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.space.set_view(w as f32, h as f32);
        frame_for(&mut self.space, w, h);
        if let Some(g) = self.gpu.as_mut() {
            g.resize(&self.device, (w, h));
        }
    }

    /// Another being (the character switcher): a new `deck.graph` and the page's values now; the set and the camera
    /// stay. Its portraits are asked for anew (`portrait_requests`).
    pub fn set_being(&mut self, deck_graph: String, values: String) -> Result<(), JsValue> {
        let (graph, look) = being_of(&deck_graph, &values).map_err(err)?;
        self.space.replace_being(&graph, &look);
        self.page = Page { mirror: mirror_of(&graph), turned: Vec::new() };
        self.meters = graph.nodes.iter().map(|n| vec![0.0; n.kind.outputs().len()]).collect();
        self.graph = graph;
        Ok(())
    }

    /// One of the page's values (a `node.port` path it streams: an input, a control output, an audio output's peak,
    /// `node.shape`); false = the being has no such path.
    pub fn set(&mut self, path: &str, value: f32) -> bool {
        match values::resolve(&self.graph, path) {
            Some(ValuePath::Output(n, k)) if self.graph.nodes[n].kind.outputs()[k].ty == deck_space::graph::PortType::Audio => {
                self.meters[n][k] = value;
                true
            }
            Some(_) => self.page.mirror.apply(&self.graph, path, &[value]),
            None => false,
        }
    }

    /// The head's mood (the page's – its energy ladder); an unknown one leaves the head as it is.
    pub fn set_mood(&mut self, mood: &str) {
        if let Some(i) = self.space.being.moods.iter().position(|m| m == mood) {
            self.space.being.head.fixed = Some(i);
        }
    }

    /// the page's tempo: bpm, the beat (counted from the start), beats per bar, running
    pub fn set_beat(&mut self, bpm: f64, beat: f64, per_bar: u32, running: bool) {
        let per_bar = per_bar.max(1);
        self.beat = tempo::Now { bpm, pos: tempo::position(beat, per_bar), per_bar, running };
    }

    /// What the being's hand turned since the last call (a drag, a double click's ease back): JSON, path → value.
    pub fn changes(&mut self) -> String {
        if self.page.turned.is_empty() {
            return String::new();
        }
        let mut out = serde_json::Map::new();
        for p in std::mem::take(&mut self.page.turned) {
            let v = match p {
                ValuePath::Input(n, k) => self.page.mirror.raw_input(n, k),
                ValuePath::Shape(n) => Values::lfo_shape(&self.page.mirror, n).unwrap_or(0) as f32,
                ValuePath::Output(..) => continue,
            };
            out.insert(values::path_name(&self.graph, p), serde_json::Value::from(v as f64));
        }
        serde_json::Value::Object(out).to_string()
    }

    /// One frame: `dt` since the last (s), `time` the page's clock (s). Updates the being and draws.
    pub fn frame(&mut self, dt: f32, time: f32) -> Result<(), JsValue> {
        let meters = &self.meters;
        let meter = |n: usize, p: usize| meters.get(n).and_then(|m| m.get(p)).copied().unwrap_or(0.0);
        self.space.update(dt.clamp(0.0, 0.1), time, &mut self.page, &self.beat, &meter);
        let size = (self.config.width, self.config.height);
        let gpu = self.gpu.get_or_insert_with(|| SpaceGpu::new(&self.device, &self.queue, &self.space, self.config.format, size));
        gpu.sync(&self.device, &self.queue, &mut self.space);
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(e) => return Err(err(e)),
        };
        let view = frame.texture.create_view(&Default::default());
        let mut enc = self.device.create_command_encoder(&Default::default());
        gpu.draw(&self.queue, &mut enc, &view, size, &self.space, time);
        self.queue.submit([enc.finish()]);
        frame.present();
        Ok(())
    }

    /// The set's .glb images still encoded, for the browser to decode: [index, bytes, index, bytes …]
    pub fn encoded_images(glb_bytes: &[u8]) -> Result<js_sys::Array, JsValue> {
        let out = js_sys::Array::new();
        for (i, bytes) in glb::encoded_images(glb_bytes).map_err(err)? {
            out.push(&JsValue::from(i as u32));
            out.push(&js_sys::Uint8Array::from(bytes.as_slice()));
        }
        Ok(out)
    }

    /// The set has arrived: its .glb and its images decoded by the browser – `images` = [index, width, height, RGBA
    /// bytes, …] (an image the browser couldn't decode is left out: it draws untextured grey).
    pub fn give_set(&mut self, glb_bytes: &[u8], images: js_sys::Array) -> Result<(), JsValue> {
        let mut decoded: HashMap<usize, image::RgbaImage> = HashMap::new();
        let items: Vec<JsValue> = images.iter().collect();
        for c in items.chunks_exact(4) {
            let (Some(i), Some(w), Some(h)) = (c[0].as_f64(), c[1].as_f64(), c[2].as_f64()) else { continue };
            let px = js_sys::Uint8Array::new(&c[3]).to_vec();
            if let Some(img) = image::RgbaImage::from_raw(w as u32, h as u32, px) {
                decoded.insert(i as usize, img);
            }
        }
        let (max, hide) = self.space.set_options();
        let hide = hide.to_vec();
        let model = glb::parse_with(glb_bytes, max, &hide, &mut |i, _| decoded.remove(&i).ok_or_else(|| anyhow::anyhow!("not decoded by the browser")));
        let ok = model.is_ok();
        self.space.set_arrived(model);
        if ok {
            Ok(())
        } else {
            Err(err(&self.space.set_status))
        }
    }

    /// The portraits the heads ask for: [slot, character, mood, …] – the page fetches and decodes each and answers
    /// with `give_portrait` (or `no_portrait`).
    pub fn portrait_requests(&mut self) -> js_sys::Array {
        let out = js_sys::Array::new();
        for (slot, character, mood) in self.space.portraits.requests() {
            out.push(&JsValue::from(slot as u32));
            out.push(&JsValue::from_str(&character));
            out.push(&JsValue::from_str(&mood));
        }
        out
    }

    /// a portrait sprite decoded by the browser (RGBA8)
    pub fn give_portrait(&mut self, slot: u32, width: u32, height: u32, rgba: Vec<u8>) {
        let img = image::RgbaImage::from_raw(width, height, rgba).ok_or_else(|| "a sprite of the wrong size".to_string());
        self.space.portraits.deliver(slot as usize, img);
    }

    /// a portrait the page couldn't get (the head shows the nearest mood it has)
    pub fn no_portrait(&mut self, slot: u32, why: String) {
        self.space.portraits.deliver(slot as usize, Err(why));
    }

    // ── input: the stage's own picking, drag, double click ──

    /// the pointer at device pixel x, y (a mouse or a pen)
    pub fn pointer_move(&mut self, x: f32, y: f32) {
        self.space.pointer_moved(x, y, &mut self.page);
    }

    pub fn pointer_down(&mut self) {
        self.space.press();
    }

    pub fn pointer_up(&mut self) {
        self.space.release();
    }

    pub fn pointer_leave(&mut self) {
        self.space.pointer_left();
    }

    /// a finger: `phase` 0 = down, 1 = moved, 2 = up (a bigger reach than the mouse)
    pub fn touch(&mut self, id: u32, phase: u32, x: f32, y: f32) {
        let phase = match phase {
            0 => Touch::Started,
            1 => Touch::Moved,
            _ => Touch::Ended,
        };
        self.space.touch(id as u64, phase, x, y, &mut self.page);
    }

    /// zoom in (+) or out (−) by wheel steps (a pinch)
    pub fn zoom(&mut self, steps: f32) {
        self.space.camera.zoom(steps);
    }

    /// the camera back home
    pub fn home(&mut self) {
        self.space.camera.reset();
    }

    /// the CSS cursor for where the pointer is
    pub fn cursor(&self) -> String {
        match self.space.cursor() {
            Cursor::Default => "default",
            Cursor::Grab => "grab",
            Cursor::Grabbing => "grabbing",
            Cursor::Locked => "not-allowed",
        }
        .into()
    }

    /// the stage HUD's line: the hovered button (its value, range or source), else the set's state
    pub fn hud_line(&self) -> String {
        self.space.hud_line(&self.graph)
    }

    /// the head's mood now
    pub fn mood(&self) -> String {
        self.space.being.moods.get(self.space.being.head.mood).cloned().unwrap_or_default()
    }
}
