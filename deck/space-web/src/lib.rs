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
//! - **Rigs** (#191): `rig_play` – a character's rig played for a persona and a mood, as the page's scene and knobs;
//!   `core_rig` (the image's) and `scene_rig` (scene-content's, #271) hand it the bundle.
//! - **Controls (#209):** a tap selects a knob / the being / nothing (`take_tap`, `select` – a white ring), the knobs
//!   face the camera (deck-space). `DeckControls` – deck-space's `controls` without a GPU (it works when the 3D can't):
//!   what the selection gives the two corner sticks, the sticks' relative moves, MIDI learn and the mappings' text.
//! - **GPU:** WebGPU when the browser has an adapter (wgpu's detection asks for one – a `navigator.gpu` without an
//!   adapter, like the S22's Chrome, counts as none), else WebGL2; `prefer` = "webgl2" skips WebGPU.
//! - **Quality (#220, deck-space quality.rs):** the page gives the display size (`resize`) and every drawn frame's
//!   interval (`govern`; `due` = the preset's frame rate cap – an early frame is skipped); the governor steps the rung,
//!   and the view sets the canvas' own size to the rung's render size – the browser stretches it, the page's UI stays
//!   sharp, pointer positions are read in canvas pixels. `quality` for the panel, `quality_report` = the
//!   `deck.quality` the page sends, `set_quality` the panel's level / Auto, `presets` / `set_presets` the settings
//!   sheet's table (the same presets as the stage's quality file – TOML or JSON), `pixel_ratio` their pixel cap.
//! - **Monitor (#364):** `DeckMonitor` – deck-core's `monitor` on the page's frame thread, beside the governor (no GPU:
//!   it works when the 3D can't, and before the sound): the page feeds each animation frame (drawn or skipped), the
//!   worker's audio window (its words) and the dropout count; `report` is the stats bar's and the sound report's numbers,
//!   judgement and words.
//! - **The deck's state (#367, #368):** `DeckState` – deck-space's `deck::Deck` without a GPU: the mood's energy ladder (the
//!   native being's walk) from the level, the clock and a hand, and what plays (its own scene or a rig, replayed on every
//!   mood); the page applies its turns.
//! - **Scenes (#463, #465):** `SceneView` – a scene drawn by the renderer every deck uses (deck-space `stage`), its control
//!   domain deck-engine's, its set, portraits and chapters fetched by the page; the idle scene when none is given.
#![cfg(target_arch = "wasm32")]

mod link;
mod outlet;
mod scene;

use deck_space::controls::{Controls, Midi, Selection};
use deck_space::graph::{Graph, Kind};
use deck_space::quality::{Governor, Gpu, Level, Presets};
use deck_space::scene::{Scene, CATHEDRAL_SCENE};
use deck_space::space::being::Look;
use deck_space::space::glb;
use deck_space::space::portrait::Portraits;
use deck_space::space::values::{self, Knobs, Mirror, Path as ValuePath, Values};
use deck_space::space::{wire, Cursor, Space, Touch};
use deck_space::splash::Fade;
use deck_space::stage::{Over, Stage, Tick};
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
        // a linked input's knob turns its base (#188): the page sways it on top (its LFO on the cutoff)
        let ok = if self.mirror.linked(node, port) { self.mirror.set_base(node, port, value) } else { self.mirror.set_input(node, port, value) };
        if ok {
            self.touched(ValuePath::Input(node, port));
        }
        ok
    }
    fn base(&self, node: usize, port: usize) -> Option<f32> {
        self.mirror.base(node, port)
    }
    fn set_lfo_shape(&mut self, node: usize, shape: usize) -> bool {
        let ok = self.mirror.set_shape(node, shape);
        if ok {
            self.touched(ValuePath::Shape(node));
        }
        ok
    }
}

pub(crate) fn err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

fn json(v: &impl serde::Serialize) -> String {
    serde_json::to_string(v).unwrap_or_default()
}

/// The deck's being from its wire graph, in the cathedral: the space node takes the cathedral's (where the being
/// floats, the camera, the set – not its `join`, the page streams), the look the cathedral's (its drift, breath …)
/// with the wire's character, moods, size and exposed parameters; the page's starting values become the nodes' params
/// (what a double click eases back to) – a linked input's its link's offset, its base (#188). No feelings: the page's mood is the head's.
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
            // a linked input's start value is its base: the link's offset (#188 – what its knob turns, a double click
            // eases back to)
            if let Some(l) = graph.links.iter_mut().find(|l| l.to == (n, k)) {
                l.offset = v as f32;
                continue;
            }
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
    // a linked input's base until the page writes its own: the link's offset
    for l in &graph.links {
        m.set_base(l.to.0, l.to.1, l.offset);
    }
    m
}

/// The native stage frames the being for a landscape screen (55° high); a phone held upright sees as much across as the
/// stage sees from top to bottom – the field of view widens with the height (at most 75°), the camera stays.
pub(crate) fn frame_for(space: &mut Space, w: u32, h: u32) {
    let aspect = w as f32 / h.max(1) as f32;
    let native = 55f32.to_radians();
    space.camera.fov_y = if aspect >= 1.0 { native } else { (2.0 * ((native * 0.5).tan() / aspect).atan()).min(75f32.to_radians()) };
}

/// A character's rig played for a persona and a mood (#191, deck-space `rig::Bundle`): `bundle` = the rig with its
/// instruments and presets as JSON (scene-content's answer), `persona` / `mood` empty = none. Returns JSON: the
/// flattened scene (`scene` TOML, `nodes`, `links`), `character`, `knobs` (path, label, min, max, value, log) and
/// `tracks` (which preset each plays). The page calls it again on a mood change and applies what differs.
#[wasm_bindgen]
pub fn rig_play(bundle: &str, persona: &str, mood: &str) -> Result<String, JsValue> {
    deck_space::rig::Bundle::play_json(bundle, persona, mood).map_err(|e| JsValue::from_str(&format!("{e:#}")))
}

/// The Cathedral (#360: the X1's old boot scene, embedded in this build) played as `rig_play` answers – a TEST scene since
/// #425: no deck plays it when its character has no rig (that is silent: `DeckState.now`); its `engine.scene` is what
/// live-core loads.
#[wasm_bindgen]
pub fn deck_scene() -> Result<String, JsValue> {
    deck_space::rig::Played::deck_scene_json().map_err(|e| JsValue::from_str(&format!("{e:#}")))
}

/// The core rig of `subject` (`character.<id>` / `item.<id>`) the deck carries without an account (#191): the bundle as
/// JSON for `rig_play`, empty when there is none.
#[wasm_bindgen]
pub fn core_rig(subject: &str) -> Result<String, JsValue> {
    deck_space::rig::Bundle::core_json(subject).map_err(|e| JsValue::from_str(&format!("{e:#}")))
}

/// The catalog's rig of one holder from scene-content's answer (#271): `answer` = the answer's JSON, `group` =
/// `characters` / `items`, `id` = the id as asked – the bundle as JSON for `rig_play`, empty when it has no rig.
#[wasm_bindgen]
pub fn scene_rig(answer: &str, group: &str, id: &str) -> Result<String, JsValue> {
    deck_space::rig::content::scene_rig_json(answer, group, id).map_err(|e| JsValue::from_str(&format!("{e:#}")))
}

/// An instrument / preset / rig document checked with what it names (#313, deck-space `rig::validate`) – the same check
/// the API asks the build service for before a catalog write, here as a hint on a draft: `request` = `{ kind, id,
/// document, instruments?, presets? }` as JSON, the answer `{ problems: [..] }` (none = valid).
#[wasm_bindgen]
pub fn rig_problems(request: &str) -> Result<String, JsValue> {
    deck_space::rig::validate::problems_json(request).map_err(|e| JsValue::from_str(&format!("{e:#}")))
}

/// #283: every character's default rig – plays, drafted or missing – and its personas' own (deck-space
/// `rig::assign::coverage`): `request` = `{ characters: [{ id, personas }], rigs: [{ id, live, draft, removed }] }` as
/// JSON (the registry and the catalog's rig list), the answer `[{ character, subject, default, personas?, missing }]`.
#[wasm_bindgen]
pub fn rig_coverage(request: &str) -> Result<String, JsValue> {
    deck_space::rig::assign::coverage_json(request).map_err(|e| JsValue::from_str(&format!("{e:#}")))
}

/// #283: a rig document with what it names from the catalog (`{ rig, instruments, presets }` as JSON, every entry's
/// document) – the bundle as JSON for `rig_play`, so a draft is heard before it is published.
#[wasm_bindgen]
pub fn rig_catalog_bundle(request: &str) -> Result<String, JsValue> {
    deck_space::rig::assign::catalog_bundle_json(request).map_err(|e| JsValue::from_str(&format!("{e:#}")))
}

/// #412: a bundle's face (deck-space `Rig::describe`): its tracks and what a code rig's policy owns of them, as JSON – the
/// Assignment tab marks the rows from it.
#[wasm_bindgen]
pub fn rig_face(bundle: &str) -> Result<String, JsValue> {
    deck_space::rig::face::face_json(bundle).map_err(|e| JsValue::from_str(&format!("{e:#}")))
}

/// #412: a rig's policy template (deck-space `rig_policy_template`): `request` = `{ module, rig, instruments, presets }` as
/// JSON (the rig document and the catalog's documents), the answer `{ "lib.rs": … }` – the first files of `module:<module>`.
#[wasm_bindgen]
pub fn rig_policy_template(request: &str) -> Result<String, JsValue> {
    deck_space::rig::template::template_json(request).map_err(|e| JsValue::from_str(&format!("{e:#}")))
}

/// #425: the instrument that wraps one module (`{ module: <the catalog's module document> }` as JSON → the instrument
/// document as JSON) – the rig page's "start from existing modules" adds it to the catalog and a track of it to the rig;
/// throws why a module cannot be one (no descriptor yet, no audio output).
#[wasm_bindgen]
pub fn rig_module_instrument(request: &str) -> Result<String, JsValue> {
    deck_space::rig::assign::module_instrument_json(request).map_err(|e| JsValue::from_str(&format!("{e:#}")))
}

/// a set's .glb and its images as the browser decoded them (`encoded_images`) into the space
pub(crate) fn set_from_page(space: &mut Space, glb_bytes: &[u8], images: js_sys::Array) -> Result<(), JsValue> {
    let mut decoded: HashMap<usize, image::RgbaImage> = HashMap::new();
    let items: Vec<JsValue> = images.iter().collect();
    for c in items.chunks_exact(4) {
        let (Some(i), Some(w), Some(h)) = (c[0].as_f64(), c[1].as_f64(), c[2].as_f64()) else { continue };
        let px = js_sys::Uint8Array::new(&c[3]).to_vec();
        if let Some(img) = image::RgbaImage::from_raw(w as u32, h as u32, px) {
            decoded.insert(i as usize, img);
        }
    }
    let (max, hide) = space.set_options();
    let hide = hide.to_vec();
    let model = glb::parse_with(glb_bytes, max, &hide, &mut |i, _| decoded.remove(&i).ok_or_else(|| anyhow::anyhow!("not decoded by the browser")));
    let ok = model.is_ok();
    space.set_arrived(model);
    if ok {
        Ok(())
    } else {
        Err(err(&space.set_status))
    }
}

/// the sprites the heads ask for: slot, character, mood, …
pub(crate) fn portrait_asks(space: &mut Space) -> js_sys::Array {
    let out = js_sys::Array::new();
    for (slot, character, mood) in space.portraits.requests() {
        out.push(&JsValue::from(slot as u32));
        out.push(&JsValue::from_str(&character));
        out.push(&JsValue::from_str(&mood));
    }
    out
}

/// a set that does not come does not hold the splash longer than this (s)
pub(crate) const SPLASH_AT_MOST: f32 = 5.0;

/// A canvas' GPU, as `SpaceView` and `SceneView` draw on it.
struct CanvasGpu {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    alpha: wgpu::CompositeAlphaMode,
    backend: &'static str,
    /// the adapter's name (a browser's WebGPU often gives none: privacy)
    name: String,
}

/// `prefer`: "auto" (WebGPU if the browser has an adapter, else WebGL2) or "webgl2".
async fn open_canvas(canvas: web_sys::HtmlCanvasElement, prefer: &str) -> Result<CanvasGpu, JsValue> {
    let backends = if prefer == "webgl2" { wgpu::Backends::GL } else { wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL };
    let instance = wgpu::util::new_instance_with_webgpu_detection(wgpu::InstanceDescriptor { backends, ..wgpu::InstanceDescriptor::new_without_display_handle() }).await;
    let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas)).map_err(err)?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
            apply_limit_buckets: false,
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
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
        })
        .await
        .map_err(err)?;
    let caps = surface.get_capabilities(&adapter);
    let first = *caps.formats.first().ok_or_else(|| err("the canvas offers no format"))?;
    // the space's shaders write display values (the stage's offscreen target is Rgba8Unorm): a non-sRGB surface –
    // BGRA8Unorm on Chrome's WebGPU, RGBA8Unorm on WebGL2; the post stack's textures take the same format (#277)
    let format = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(first);
    Ok(CanvasGpu { surface, device, queue, format, alpha: caps.alpha_modes[0], backend, name: info.name })
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
    /// the renderer every deck draws with (deck-space `stage`, #463)
    stage: Stage,
    /// the splash until the set is there, at most `SPLASH_AT_MOST` seconds
    fade: Fade,
    cover: bool,
    age: f32,
    backend: String,
    adapter: String,
    /// #220: the canvas (its size = the render size), the display size the page gives, the governor
    canvas: web_sys::HtmlCanvasElement,
    display: (u32, u32),
    governor: Governor,
    presets: Presets,
    gpu_class: Gpu,
}

#[wasm_bindgen]
impl SpaceView {
    /// The view on `canvas` (its size in device pixels is the canvas' width / height): the being of `deck_graph` (a
    /// `deck.graph` as JSON) with the page's starting `values` (JSON: path → value). `prefer`: "auto" (WebGPU if the
    /// browser has an adapter, else WebGL2) or "webgl2". A canvas that once had a WebGPU context can't give WebGL2 –
    /// the page retries on a fresh canvas with "webgl2" if this fails.
    pub async fn create(canvas: web_sys::HtmlCanvasElement, deck_graph: String, values: String, prefer: String) -> Result<SpaceView, JsValue> {
        let display = (canvas.width().max(1), canvas.height().max(1));
        let element = canvas.clone();
        let CanvasGpu { surface, device, queue, format, alpha, backend, name } = open_canvas(canvas, &prefer).await?;
        let gpu = if backend == "webgpu" { Gpu::WebGpu } else { Gpu::WebGl2 };
        let presets = Presets::default();
        let mut governor = Governor::with(&presets, gpu);
        governor.set_display(display.0, display.1);
        let (w, h) = governor.render_size();
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
        let (graph, look) = being_of(&deck_graph, &values).map_err(err)?;
        let node = graph.first(Kind::Space).ok_or_else(|| err("the deck's graph has no space node"))?;
        let mut stage = Stage::new(&device, &queue, format, (w, h), 1.0, 1.0, false).await.map_err(err)?;
        stage.set_host_quality(*governor.rung());
        stage.load(&device, format, &graph, Vec::new()).await.map_err(err)?;
        let mut space = Space::new(&graph, node, &look, std::path::Path::new(""), Portraits::manual());
        space.set_view(w as f32, h as f32);
        frame_for(&mut space, w, h);
        let page = Page { mirror: mirror_of(&graph), turned: Vec::new() };
        let meters = graph.nodes.iter().map(|n| vec![0.0; n.kind.outputs().len()]).collect();
        // a browser's WebGPU adapter often has no name (privacy): the backend alone then
        let adapter = if name.is_empty() { backend.to_string() } else { format!("{name} ({backend})") };
        Ok(SpaceView { space, graph, page, meters, beat: tempo::Now::default(), surface, device, queue, config, stage, fade: Fade { alpha: 1.0 }, cover: true, age: 0.0, backend: backend.into(), adapter, canvas: element, display, governor, presets, gpu_class: gpu })
    }

    /// no set is coming: the splash goes
    pub fn uncover(&mut self) {
        self.cover = false;
    }

    /// "webgpu" or "webgl2"
    pub fn backend(&self) -> String {
        self.backend.clone()
    }

    /// the adapter's name and backend (the page's status line)
    pub fn adapter(&self) -> String {
        self.adapter.clone()
    }

    /// the display's new size in device pixels (the canvas' box × the page's pixel ratio); the canvas itself gets the
    /// quality rung's render size
    pub fn resize(&mut self, w: u32, h: u32) {
        self.display = (w.max(1), h.max(1));
        self.fit();
    }

    /// the canvas, the surface, the camera and the GPU's targets at the render size (only when it changed)
    fn fit(&mut self) {
        self.governor.set_display(self.display.0, self.display.1);
        let (w, h) = self.governor.render_size();
        self.stage.set_host_quality(*self.governor.rung());
        if (w, h) == (self.config.width, self.config.height) {
            return;
        }
        self.canvas.set_width(w);
        self.canvas.set_height(h);
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.space.set_view(w as f32, h as f32);
        frame_for(&mut self.space, w, h);
        self.stage.resize(&self.device, (w, h));
    }

    /// Another being (the character switcher): a new `deck.graph` and the page's values now; the set and the camera
    /// stay. Its portraits are asked for anew (`portrait_requests`).
    pub fn set_being(&mut self, deck_graph: String, values: String) -> Result<(), JsValue> {
        let (graph, look) = being_of(&deck_graph, &values).map_err(err)?;
        self.space.replace_being(&graph, &look);
        if let Some(node) = graph.first(Kind::Space) {
            self.stage.set_space_node(&self.device, node);
        }
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

    /// A linked input's base – the page's own value under its modulation (#188: the knob turns it, `changes` hands it
    /// back); false = the being has no such linked input.
    pub fn set_base(&mut self, path: &str, value: f32) -> bool {
        match values::resolve(&self.graph, path) {
            Some(ValuePath::Input(n, k)) => self.page.mirror.set_base(n, k, value),
            _ => false,
        }
    }

    /// The output limiter's gain reduction (dB, the deepest since the last call – live-core's analysis frame): the
    /// head's red flush (#188). Call it once a frame before `frame`.
    pub fn set_strain(&mut self, reduction_db: f32, dt: f32) {
        self.space.being.strain(dt.clamp(0.0, 0.1), reduction_db);
    }

    /// The head's mood (the deck's – `DeckState`'s ladder); an unknown one leaves the head as it is.
    pub fn set_mood(&mut self, mood: &str) {
        if let Some(i) = self.space.being.moods.iter().position(|m| m == mood) {
            self.space.being.head.fixed = Some(i);
        }
    }

    /// the core's tempo clock as its last frame left it – bpm, the beat (counted from the start) heard at the page's
    /// `heard_ms`, beats per bar, running – drawn at the page's `now_ms` (the beat between two clock frames, #387)
    pub fn set_beat(&mut self, bpm: f64, beat: f64, heard_ms: f64, now_ms: f64, per_bar: u32, running: bool) {
        let per_bar = per_bar.max(1);
        let beat = deck_core::tempo::beat_after(beat, bpm, running, (now_ms - heard_ms) * 1e-3);
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
                ValuePath::Input(n, k) => self.page.mirror.base(n, k).unwrap_or_else(|| self.page.mirror.raw_input(n, k)),
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
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            other => return Err(err(format!("the canvas's surface: {other:?}"))),
        };
        let view = frame.texture.create_view(&Default::default());
        let mut enc = self.device.create_command_encoder(&Default::default());
        self.age += dt;
        self.fade.step(self.cover && self.age < SPLASH_AT_MOST, dt);
        let tick = Tick { graph: &self.graph, input: &|_, _, d| d, tempo: &self.beat, time, dt };
        self.stage.frame(&self.device, &self.queue, &mut enc, &view, &tick, &Over { hud: None, splash: (self.fade.alpha, false), label_centred: true, hud_at: None }, Some(&mut self.space));
        self.queue.submit([enc.finish()]);
        self.queue.present(frame);
        Ok(())
    }

    /// #220: one frame's interval for the governor – `now` the page's monotonic clock (s), `dt_ms` since the last
    /// frame (unclamped: a long stall is a hitch, not load), `glitches` the audio's running dropout count. A step
    /// resizes the canvas; returns whether the quality changed.
    pub fn govern(&mut self, now: f64, dt_ms: f32, glitches: u32) -> bool {
        let changed = self.governor.frame(now, dt_ms, glitches as u64);
        if changed {
            self.fit();
        }
        changed
    }

    /// #220: the quality now, for the panel – JSON { level, ceiling, auto, scale, frame, fps, width, height }
    pub fn quality(&self) -> String {
        let r = self.governor.report();
        serde_json::json!({
            "level": r.level.name(),
            "ceiling": r.ceiling.name(),
            "auto": r.auto,
            "scale": r.scale,
            "frame": (r.frame_ms * 10.0).round() / 10.0,
            "fps": self.governor.rung().fps,
            "width": self.config.width,
            "height": self.config.height,
        })
        .to_string()
    }

    /// #220: the preset's frame rate cap – draw now, `since_ms` after the last drawn frame? (false: skip this one)
    pub fn due(&self, since_ms: f32) -> bool {
        self.governor.due(since_ms)
    }

    /// #220: the presets as the stage's quality file (TOML) – the sheet's "Copy as text"
    pub fn presets_text(&self) -> String {
        self.presets.to_text()
    }

    /// #220: the presets as JSON (the settings sheet's table)
    pub fn presets_json(&self) -> String {
        serde_json::to_string(&self.presets).unwrap_or_default()
    }

    /// #375: the option lists the settings offer – levels (id, label), fps caps, scales, heights, pixel ratios, the
    /// lights and size limits – JSON, deck-space's `quality::options_json`; the page lists exactly these
    pub fn options_json(&self) -> String {
        deck_space::quality::options_json()
    }

    /// #220: presets from TOML (the stage's file, or only what it changes) – "" or why not (they stay as they were)
    pub fn set_presets(&mut self, text: &str) -> String {
        match Presets::parse(text) {
            Ok(p) => self.use_presets(p),
            Err(e) => e,
        }
    }

    /// #220: presets from the sheet's JSON – "" or why not
    pub fn set_presets_json(&mut self, json: &str) -> String {
        match serde_json::from_str::<Presets>(json) {
            // through the text: the same clamping as the file
            Ok(p) => self.set_presets(&p.to_text()),
            Err(e) => e.to_string(),
        }
    }

    fn use_presets(&mut self, p: Presets) -> String {
        self.presets = p;
        self.governor.set_presets(&self.presets, self.gpu_class);
        self.fit();
        String::new()
    }

    /// #220: the presets' device pixels per CSS pixel at most (0 = none: the page's own cap)
    pub fn pixel_ratio(&self) -> f32 {
        self.presets.pixel_ratio.unwrap_or(0.0)
    }

    /// #220: the `deck.quality` to send now (JSON), or "" – at once on a change, else at most every 2 s
    pub fn quality_report(&mut self, now: f64) -> String {
        self.governor.report_due(now).map(|r| serde_json::to_string(&r.wire()).unwrap_or_default()).unwrap_or_default()
    }

    /// #220: the panel's choice – a level ("low" … "ultra"; another word keeps the level) and whether the governor
    /// steps below it
    pub fn set_quality(&mut self, level: &str, auto: bool) {
        if let Some(l) = Level::parse(level) {
            self.presets.level = Some(l);
            if l != self.governor.ceiling() {
                self.governor.set_level(l);
            }
        }
        self.presets.auto = auto;
        if auto != self.governor.auto() {
            self.governor.set_auto(auto);
        }
        self.fit();
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
        self.cover = false;
        set_from_page(&mut self.space, glb_bytes, images)
    }

    /// The portraits the heads ask for: [slot, character, mood, …] – the page fetches and decodes each and answers
    /// with `give_portrait` (or `no_portrait`).
    pub fn portrait_requests(&mut self) -> js_sys::Array {
        portrait_asks(&mut self.space)
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

    /// a finger (the page's pointer event with its `pointerId`): `phase` 0 = down, 1 = moved, 2 = up – the first drives
    /// the pointer with a bigger reach than the mouse, two pinch the camera (deck-space gesture.rs)
    pub fn pointers(&mut self, id: u32, phase: u32, x: f32, y: f32) {
        self.space.pointers(id as u64, Touch::from_code(phase), x, y, &mut self.page);
    }

    /// zoom in (+) or out (−) by wheel steps (a wheel notch is 1)
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

    /// the stage HUD's line: the hovered button (its value – a linked one's base – and source), else the set's state
    pub fn hud_line(&self) -> String {
        self.space.hud_line(&self.graph)
    }

    /// the head's mood now
    pub fn mood(&self) -> String {
        self.space.being.moods.get(self.space.being.head.mood).cloned().unwrap_or_default()
    }

    /// #209: the selection a tap made since the last call – `""` (empty space: deselected), `being`, `knob:node.port` –
    /// or undefined (no tap)
    pub fn take_tap(&mut self) -> Option<String> {
        self.space.take_tap().map(|_| self.space.selection_name())
    }

    /// #209: select by name (`""`, `being`, `knob:node.port` – the page's knob tapped): the being rings it
    pub fn select(&mut self, name: &str) {
        self.space.select_named(name);
    }
}

/// #209: the deck's controls – what the hands drive (deck-space `controls`: the selection → the two sticks, MIDI learn,
/// the mappings as text for a preset; #369: the knobs' value ↔ 0..1, a drag's and a key's turn, the gamepad, the panel's
/// line). No GPU: the page has it even when the 3D can't start.
#[wasm_bindgen]
pub struct DeckControls {
    c: Controls,
    /// the last MIDI message's 0..1 and whether it learned
    value: f32,
    learned: bool,
}

#[wasm_bindgen]
impl DeckControls {
    /// no knobs yet: `set_deck` hands them in
    #[wasm_bindgen(constructor)]
    pub fn new() -> DeckControls {
        DeckControls { c: Controls::new::<&str>(&[]), value: 0.0, learned: false }
    }

    /// Another deck's knobs – `DeckState.sliders` as it answered (JSON: `[{path, min, max, value, log, label}]`: a rig's, or
    /// the deck's own scene's again) – and the being's stick pairs on them (#191): `sticks` = x, y of the left stick then x,
    /// y of the right ("" = no axis); fewer than four = the defaults. Throws on sliders that are no such JSON.
    pub fn set_deck(&mut self, sliders: &str, sticks: Vec<String>) -> Result<(), JsValue> {
        let knobs: Vec<deck_space::deck::Slider> = serde_json::from_str(sliders).map_err(|e| JsValue::from_str(&format!("the sliders: {e}")))?;
        let s = |i: usize| sticks.get(i).map_or("", String::as_str);
        let pairs = (sticks.len() >= 4).then(|| [[s(0), s(1)], [s(2), s(3)]]);
        self.c.set_knobs(&knobs, pairs);
        Ok(())
    }

    /// #369: where `value` sits on knob `path`'s range, 0..1 on its curve (NaN: the knob's default; no such knob: 0)
    pub fn norm(&self, path: &str, value: f32) -> f32 {
        self.c.norm(path, value)
    }

    /// #369: knob `path`'s value at `t` (0..1); undefined: no such knob
    pub fn value_at(&self, path: &str, t: f32) -> Option<f32> {
        self.c.value(path, t)
    }

    /// #369: a knob dragged by `dy` (screen y: up = more) from its value `start` – `travel` turns it end to end, `fine` a
    /// tenth as fast: its value (undefined: no such knob)
    pub fn turn(&self, path: &str, start: f32, dy: f32, travel: f32, fine: bool) -> Option<f32> {
        self.c.turn(path, start, dy, travel, fine)
    }

    /// #369: a key (the DOM's `key`) on a knob at `value`: its new value (undefined: no such knob, or a key that doesn't turn it)
    pub fn step(&self, path: &str, value: f32, key: &str, fine: bool) -> Option<f32> {
        self.c.step(path, value, key, fine)
    }

    /// #228, #369: one poll of the first connected gamepad (the standard mapping's axes, its buttons pressed = non-zero;
    /// empty = no pad) – what the page does, four numbers an event: `[kind, side, dx, dy]`, kind 0 a stick down · 1 its
    /// deflection (y up) · 2 up · 3 the selection moved on (already: `selection`)
    pub fn pad(&mut self, axes: Vec<f32>, buttons: Vec<u8>) -> Vec<f32> {
        let mut pressed = [false; 16];
        for (p, b) in pressed.iter_mut().zip(&buttons) {
            *p = *b != 0;
        }
        self.c.pad(&axes, &pressed[..buttons.len().min(16)]).iter().flat_map(|e| e.code()).collect()
    }

    /// #209, #369: what the sticks drive in one line – `being` / `master` in the page's language
    pub fn selection_line(&self, being: &str, master: &str) -> String {
        self.c.selection_line(being, master)
    }

    /// #369: stick `side`'s [x, y] labels ("–" for an axis that drives nothing)
    pub fn stick_labels(&self, side: usize) -> Vec<String> {
        self.c.stick_labels(side).map(String::from).to_vec()
    }

    /// the knob that owns the sticks ("" none)
    pub fn knob(&self) -> String {
        self.c.knob().to_string()
    }

    /// `""` (the master section), `being`, `knob:<path>`, `object:<id>`
    pub fn select(&mut self, name: &str) {
        self.c.select(Selection::parse(name));
    }

    pub fn selection(&self) -> String {
        self.c.selection().name()
    }

    /// stick `side` (0 left, 1 right): its [x, y] paths now ("" = none)
    pub fn stick_paths(&self, side: usize) -> Vec<String> {
        self.c.stick_paths(side).map(String::from).to_vec()
    }

    /// a thumb lands on stick `side`; `x`, `y` = its paths' 0..1 now
    pub fn stick_down(&mut self, side: usize, x: f32, y: f32) {
        self.c.stick_down(side, x, y);
    }

    /// the thumb's deflection (−1..1, y up) → [x, y] 0..1 for `stick_driving`'s paths
    pub fn stick_move(&self, side: usize, dx: f32, dy: f32) -> Vec<f32> {
        self.c.stick_move(side, dx, dy).to_vec()
    }

    /// the [x, y] paths the stick drives while the thumb is down
    pub fn stick_driving(&self, side: usize) -> Vec<String> {
        self.c.stick_driving(side).map(String::from).to_vec()
    }

    pub fn stick_up(&mut self, side: usize) {
        self.c.stick_up(side);
    }

    /// assign stick `side`'s axis (0 x, 1 y) to `path` for the being ("" = its default)
    pub fn assign(&mut self, side: usize, axis: usize, path: &str) {
        self.c.assign(side, axis, path);
    }

    /// arm MIDI learn for `path` ("" disarms)
    pub fn learn(&mut self, path: &str) {
        self.c.learn(path);
    }

    /// the path MIDI learn waits for ("" none)
    pub fn learning(&self) -> String {
        self.c.learning().unwrap_or_default().to_string()
    }

    /// One MIDI message: the path it moved or learned ("" = ignored); then `value` (its 0..1) and `learned`.
    pub fn midi(&mut self, msg: &[u8]) -> String {
        let (path, value, learned) = match self.c.midi(msg) {
            Midi::Ignored => (String::new(), 0.0, false),
            Midi::Learned(p) => (p.to_string(), 0.0, true),
            Midi::Set(p, v) => (p.to_string(), v, false),
        };
        (self.value, self.learned) = (value, learned);
        path
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn learned(&self) -> bool {
        self.learned
    }

    /// what drives `path` ("CC 7 · 1", "" none)
    pub fn mapped(&self, path: &str) -> String {
        self.c.mapped(path).unwrap_or_default()
    }

    pub fn unmap(&mut self, path: &str) {
        self.c.unmap(path);
    }

    /// the mappings as text (a preset's – #191; the page keeps it per deck in localStorage)
    pub fn to_text(&self) -> String {
        self.c.to_text()
    }

    /// load `to_text`'s lines; the lines it couldn't use come back (the page says so)
    pub fn load_text(&mut self, text: &str) -> Vec<String> {
        self.c.load_text(text)
    }
}

impl Default for DeckControls {
    fn default() -> Self {
        DeckControls::new()
    }
}

/// #364: deck-core's monitor – samples in, the report out (JSON: deck-core `monitor::Report`). No GPU.
#[wasm_bindgen]
pub struct DeckMonitor {
    m: deck_core::monitor::Monitor,
    /// #373: the scene's device messages as a monitor reads them – their words and the lamps' activity (deck-core `devices`)
    devices: deck_core::devices::DeviceMonitor,
}

#[wasm_bindgen]
impl DeckMonitor {
    #[wasm_bindgen(constructor)]
    pub fn new() -> DeckMonitor {
        DeckMonitor { m: deck_core::monitor::Monitor::new(), devices: deck_core::devices::DeviceMonitor::new() }
    }

    /// #373: a device message the scene's outputs resolved to, at `now_ms`, as the page decoded it (JSON, deck-space
    /// `devices::push_json`): the core words it and lights the protocol's lamp
    pub fn signal(&mut self, now_ms: f64, json: &str) {
        if let Ok(v) = serde_json::from_str(json) {
            deck_space::devices::push_json(&mut self.devices, &v, now_ms);
        }
    }

    /// #373: a row per protocol that carried a message (JSON: deck-core `devices::Row`) as of `now_ms`
    pub fn devices(&self, now_ms: f64) -> String {
        serde_json::to_string(&self.devices.rows(now_ms)).unwrap_or_default()
    }

    /// #373: a new sound starts – the monitor of the device messages starts again
    pub fn reset_devices(&mut self) {
        self.devices.reset();
    }

    /// the frame target: the quality rung's fps cap (0 = none)
    pub fn target(&mut self, fps: f32) {
        self.m.target(fps);
    }

    /// an animation frame at `now_ms` with `work_ms` of the page's work; `drawn` = the 3D drew (not skipped by the cap)
    pub fn frame(&mut self, now_ms: f64, work_ms: f32, drawn: bool) {
        self.m.frame(now_ms, work_ms, drawn);
    }

    /// the worker's audio window (live-core `lc_monitor_take`'s words) at `now_ms`
    pub fn audio(&mut self, now_ms: f64, words: &[f64]) {
        if let Some(w) = deck_core::monitor::AudioWindow::from_words(words) {
            self.m.audio(now_ms, w);
        }
    }

    /// the worklet's dropouts so far (underruns – the browser sees no other cause)
    pub fn underruns(&mut self, now_ms: f64, total: f64) {
        self.m.glitches(now_ms, [0, 0, total as u64]);
    }

    /// the sound stopped
    pub fn silent(&mut self) {
        self.m.silent();
    }

    /// everything now, as JSON
    pub fn report(&mut self, now_ms: f64) -> String {
        serde_json::to_string(&self.m.report(now_ms)).unwrap_or_default()
    }

    /// the first seconds of the sound as the report's table, one line each
    pub fn series(&self) -> Vec<String> {
        self.m.series_table()
    }
}

impl Default for DeckMonitor {
    fn default() -> Self {
        DeckMonitor::new()
    }
}

/// #371: the deck's answer to the conductor's push (deck-space push.rs `Run`, the machine the native deck's sync worker
/// drives too): the page fetches what it is told and hands the replies in; the items, the plan at the deck's quality,
/// each file's check, the answer (`deck.synced`) and the PUSH line are the core's.
#[wasm_bindgen]
pub struct DeckPush {
    api: String,
    cdn: String,
    run: Option<deck_space::push::Run>,
}

#[wasm_bindgen]
impl DeckPush {
    /// the library API (scene-content) and the CDN root
    #[wasm_bindgen(constructor)]
    pub fn new(api: String, cdn: String) -> DeckPush {
        DeckPush { api, cdn, run: None }
    }

    /// a push (`scene.sync`, JSON) at the deck's 3D `quality` ('' = the core's default) – the older one is dropped; the
    /// URL to ask
    pub fn start(&mut self, sync: &str, quality: &str) -> Result<String, JsValue> {
        let sync: protocol::SceneSync = serde_json::from_str(sync).map_err(err)?;
        let (run, url) = deck_space::push::Run::new(&self.api, &self.cdn, quality, sync);
        self.run = Some(run);
        Ok(url)
    }

    /// the API's reply: its HTTP status (0: the network is gone) and body
    pub fn asked(&mut self, status: u16, body: &str) {
        if let Some(r) = &mut self.run {
            r.asked(status, body);
        }
    }

    /// the next file to fetch, `[number, url]` (JSON); undefined: none left
    pub fn next_fetch(&mut self) -> Option<String> {
        self.run.as_mut()?.next_fetch().map(|f| json(&f))
    }

    /// file `n` came: its HTTP status (0: the network is gone), its length and first bytes (`push::HEAD` of them)
    pub fn received(&mut self, n: u32, status: u16, len: f64, head: &[u8]) {
        if let Some(r) = &mut self.run {
            r.received(n as usize, status, len as u64, head);
        }
    }

    /// the answer now (`deck.synced`, JSON); '' before a push
    pub fn answer(&self) -> String {
        self.run.as_ref().map(|r| json(&r.answer())).unwrap_or_default()
    }

    /// the answer's PUSH line; '' before a push
    pub fn line(&self) -> String {
        self.run.as_ref().map(deck_space::push::Run::line).unwrap_or_default()
    }
}

/// #367, #368: the deck's state without a GPU (deck-space `deck::Deck`) – the mood's energy ladder (mood.rs, the walk the
/// native being's head runs) and what plays: the deck's own scene or a rig, replayed on every mood (the lifecycle, the
/// facts: streamed paths, knob targets, the diff). The page passes the level, the clock and a hand each frame and
/// applies the turns it answers (JSON: deck-space `deck::Turn` / `Step`).
#[wasm_bindgen]
pub struct DeckState {
    d: deck_space::deck::Deck,
}

#[wasm_bindgen]
impl DeckState {
    #[wasm_bindgen(constructor)]
    pub fn new() -> DeckState {
        DeckState { d: deck_space::deck::Deck::new() }
    }

    /// #425: the rig editor's deck – the same, on an empty rigging stage (`room` answers "": no set behind the being)
    pub fn rigging() -> DeckState {
        DeckState { d: deck_space::deck::Deck::rigging() }
    }

    /// #425: the set the being floats in (the page loads its model); "" = none, the empty rigging stage
    pub fn room(&self) -> String {
        self.d.room().to_string()
    }

    /// the character's portraits (their moods): the ladder anew
    pub fn set_portraits(&mut self, moods: Vec<String>) {
        self.d.ladder.set_portraits(&moods);
    }

    /// hold the mood on `mood`; "" = the energy walks it again
    pub fn pin(&mut self, mood: &str) {
        self.d.ladder.pin(Some(mood));
    }

    /// One animation frame: the analysis level (0..1), the clock's bpm and whether it runs, a hand (0..1), `dt` s since
    /// the last, and a code rig's preset outputs as heard (#410: the played scene's `presets` paths in order – what the
    /// engine streamed as `policy.<out>`, NaN = not heard; empty without a code rig) – when the mood changed or the
    /// policy chose another preset its `Step` as JSON (the mood, the rig's turn for it or why not), else ""
    /// (deck-space `Deck::frame`).
    pub fn step(&mut self, level: f32, bpm: f32, running: bool, touch: f32, dt: f32, heard: Vec<f32>) -> String {
        self.d.frame(level, bpm, running, touch, dt, &heard).map(|s| json(&s)).unwrap_or_default()
    }

    /// #410: the paths of a code rig's preset outputs (`policy.<out>`) the page hands to `step` as heard, in order (none
    /// without a code rig)
    pub fn preset_paths(&self) -> Vec<String> {
        self.d.preset_paths().map(String::from).collect()
    }

    /// #368: what plays now, as a `Turn` to load whole (JSON)
    pub fn now(&self) -> String {
        json(&self.d.now())
    }

    /// #368: play a rig bundle (JSON: rig + instruments + presets) for `persona` and the mood now ("" = the deck's own
    /// scene) – the `Turn` (JSON); throws on a bad bundle, what played stays.
    pub fn play(&mut self, bundle: &str, persona: &str) -> Result<String, JsValue> {
        self.d.play_json(bundle, persona).map(|t| json(&t)).map_err(|e| JsValue::from_str(&format!("{e:#}")))
    }

    /// #368: the core rig of a character (`catpope`) or a subject – the deck's own scene when there is none or it fails
    /// (the `Turn`'s note says which)
    pub fn play_core(&mut self, character: &str, persona: &str) -> String {
        json(&self.d.play_core(character, persona))
    }

    /// #368: the knobs as sliders labelled in `locale` (JSON: `[{path, min, max, value, log, label}]`)
    pub fn sliders(&self, locale: &str) -> String {
        json(&self.d.sliders(locale))
    }

    /// #369: where `value` sits on knob `path`'s range, 0..1 on its curve (NaN: its default; no such knob: 0)
    pub fn norm(&self, path: &str, value: f32) -> f32 {
        self.d.norm(path, value)
    }

    /// #369: knob `path`'s value at `t` (0..1); undefined: no such knob
    pub fn value_at(&self, path: &str, t: f32) -> Option<f32> {
        self.d.value_at(path, t)
    }

    /// #368: the being on the wire (a `deck.graph`, JSON) – the deck's own scene wears `character`
    pub fn graph(&self, character: &str) -> String {
        json(&self.d.graph(character))
    }

    /// #282: the played scene's wiring (JSON: deck-space `rig::wiring::Wiring` – nodes with their ports and depth, links
    /// with the paths of their values) for the rig designer's Nodes tab
    pub fn wiring(&self) -> String {
        json(&self.d.wiring())
    }

    /// #368: where a value of `path` goes in the engine ("" = nowhere: a node the engine does not play)
    pub fn target(&self, path: &str) -> String {
        self.d.target(path).unwrap_or_default().to_string()
    }

    /// the mood now
    pub fn mood(&self) -> String {
        self.d.mood().to_string()
    }

    /// the energy now (0..1)
    pub fn energy(&self) -> f32 {
        self.d.ladder.energy()
    }

    /// the moods on the wire (the being's palette): the ladder, then the other portraits
    pub fn moods(&self) -> Vec<String> {
        self.d.ladder.wire()
    }

    /// the moods a picker offers
    pub fn choices(&self) -> Vec<String> {
        self.d.ladder.choices()
    }

    /// #370: the clock picker's choices (deck-core `ClockMode::ALL`), in the order the clock frame's `mode` index counts
    pub fn clock_modes(&self) -> Vec<String> {
        deck_core::tempo::ClockMode::ALL.iter().map(|m| m.name().to_string()).collect()
    }

    /// the portrait `mood` shows ("" = none)
    pub fn face(&self, mood: &str) -> String {
        self.d.ladder.face(mood).unwrap_or_default().to_string()
    }
}

impl Default for DeckState {
    fn default() -> Self {
        DeckState::new()
    }
}
