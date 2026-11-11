//! The renderer every deck draws with (CYBERDECKS.md §8 "Render", #463: out of deck-native, so the browser is the same
//! deck): one fullscreen pass per video node in graph order, each into its own texture, the screen node's blit, and over
//! it the HUD or its bar, the splash, the deck's own UI (#386 the menu, #481 the panel: deck-kit's egui screen, `ui.rs`, #485)
//! and the build label. It owns no window: the host (deck-native's
//! window, space-web's canvas) brings the device and the frame's target. A shader that does not compile falls back to `pass`.
//! S3: a pass reads up to three textures (its `in`, a chapter's current / previous slice, an overlay's `top`); the
//! chapter nodes' slices arrive from their loaders (`ChapterFeed`) and are uploaded when the node cuts.
//! #186: an `outlet` node cross-fades the composed planes of its cells over its `in` (shaders/stage/outlet.wgsl).
//! #188: a `space` node is not a fullscreen pass – space/gpu.rs draws the set and the being into its texture with a
//! depth buffer.
use crate::graph::{Graph, Kind, Node};
use crate::hud::{self, Hud};
use crate::quality::{Rung, MAX_SIDE};
use crate::space::gpu::SpaceGpu;
use crate::space::Space;
use anyhow::{bail, Context, Result};
use deck_core::tempo::{Now, Quantize};
use deck_outlet::frame::{Crossfade, Frame};
use image::RgbaImage;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

mod ui;

const COMMON: &str = include_str!("shaders/stage/common.wgsl");
const OFFSCREEN: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// how long a chapter's cut cross-fades (s)
const FADE: f32 = 0.35;

fn builtin(name: &str) -> Option<&'static str> {
    Some(match name {
        "test_pattern" => include_str!("shaders/stage/test_pattern.wgsl"),
        "kaleido" => include_str!("shaders/stage/kaleido.wgsl"),
        "rgb_shift" => include_str!("shaders/stage/rgb_shift.wgsl"),
        "invert" => include_str!("shaders/stage/invert.wgsl"),
        "pass" => include_str!("shaders/stage/pass.wgsl"),
        "fireflies" => include_str!("shaders/stage/fireflies.wgsl"),
        "flash" => include_str!("shaders/stage/flash.wgsl"),
        "chapter" => include_str!("shaders/stage/chapter.wgsl"),
        "overlay" => include_str!("shaders/stage/overlay.wgsl"),
        "outlet" => include_str!("shaders/stage/outlet.wgsl"),
        "splash" => include_str!("shaders/stage/splash.wgsl"),
        _ => return None,
    })
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    time: f32,
    dt: f32,
    res: [f32; 2],
    p: [f32; 8],
}

/// One slice of a `chapter` node, ready to show.
pub struct Prepared {
    pub index: usize,
    pub count: usize,
    pub slice: RgbaImage,
    pub caption: Option<RgbaImage>,
}

/// A `chapter` node's slices on their way to the renderer; the loader is the host's.
pub struct ChapterFeed {
    pub node: usize,
    pub rx: Receiver<Prepared>,
    pub status: Arc<Mutex<String>>,
    /// the first slice is decoded (the splash may go)
    pub ready: Arc<AtomicBool>,
}

/// What a frame is drawn from.
pub struct Tick<'a> {
    pub graph: &'a Graph,
    /// a node's control input: the linked output mapped by the link, else the node's own param, else the default
    pub input: &'a dyn Fn(usize, &str, f32) -> f32,
    pub tempo: &'a Now,
    pub time: f32,
    pub dt: f32,
}

/// What goes over the scene this frame.
pub struct Over<'a> {
    pub hud: Option<&'a Hud>,
    /// the splash's alpha (0 = none) and whether its loading line runs
    pub splash: (f32, bool),
    /// the build label centred at the top (a page keeps its own buttons in the corners), else top right
    pub label_centred: bool,
    /// #478: where the full HUD is drawn (`hud::page_rect`: the browser's Live page); None = top left (`hud::rect`)
    pub hud_at: Option<[f32; 4]>,
}

struct Target {
    view: wgpu::TextureView,
}

/// one of the fullscreen passes drawn into a viewport over the scene
struct Overlay {
    pipeline: wgpu::RenderPipeline,
    ubuf: wgpu::Buffer,
    bind: wgpu::BindGroup,
}

/// a texture a pass reads: a node's output port (a pass's target = port 0, a chapter's caption = its port `text`), or
/// a chapter's current / previous slice
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Src {
    Out(usize, usize),
    Current(usize),
    Previous(usize),
}

struct Pass {
    node: usize,
    kind: Kind,
    /// the fullscreen pass (None for a space node – its own pipelines)
    pipeline: Option<wgpu::RenderPipeline>,
    ubuf: wgpu::Buffer,
    /// what bindings 1, 3 and 4 read
    inputs: [Option<Src>; 3],
    target: Option<Target>,
    bind: Option<wgpu::BindGroup>,
}

/// an uploaded image: its view and aspect (w / h)
struct Image {
    view: wgpu::TextureView,
    aspect: f32,
}

/// a chapter node's slides: the loader's channel, what is shown, when it cut
struct Slides {
    node: usize,
    feed: ChapterFeed,
    current: Option<Image>,
    previous: Option<Image>,
    caption: Option<Image>,
    /// (the slice's index, the chapter's slice count)
    index: (usize, usize),
    cut_at: f32,
    last_next: f32,
    /// `quantize`: a due cut waits for the next beat / bar of the tempo clock (#181)
    quantize: Quantize,
    pending: bool,
    last_beat: f64,
}

/// the outlet node's planes (#186, live/): the composer's channel, the current and previous plane, their crossfade
struct OutletPlanes {
    node: usize,
    rx: Option<Receiver<Frame>>,
    current: Option<Image>,
    previous: Option<Image>,
    fade: Crossfade,
}

pub struct Stage {
    layout: wgpu::BindGroupLayout,
    linear: wgpu::Sampler,
    nearest: wgpu::Sampler,
    dummy: wgpu::TextureView,
    passes: Vec<Pass>,
    screen: Option<Pass>,
    /// the surface's size in pixels
    size: (u32, u32),
    render_scale: f32,
    /// #220: the quality rung's knobs (quality.rs – the host's governor sets them) and the scene's render_scale under them
    quality: Rung,
    scene_scale: f32,
    slides: Vec<Slides>,
    /// the outlet node's planes (#186)
    outlet: Option<OutletPlanes>,
    /// the space node's GPU side, made at its first frame
    space: Option<SpaceGpu>,
    /// the host sized the surface to the quality rung already (the browser's governor): the render size is the surface's
    host_sized: bool,
    hud_pass: Overlay,
    hud_tex: wgpu::Texture,
    /// #209: the collapsed stats bar – its texture as wide as the screen in HUD pixels (`hud::bar_width`)
    bar_pass: Overlay,
    bar_tex: wgpu::Texture,
    bar_w: u32,
    /// #485: the deck's own UI – the menu (#386, deck-ui `menu`) and the panel (#481, deck-show `face`) as layers of
    /// deck-kit's egui screen, drawn by egui-wgpu in the pass
    ui: ui::Ui,
    /// #423: the build label (build_label.rs) – rastered once at start, drawn top right over everything, always
    label_pass: Overlay,
    label_size: (u32, u32),
    splash: Overlay,
    /// the splash is a black veil without the logo (#250)
    pub veil_only: bool,
    logo: Image,
}

impl Stage {
    /// The renderer's own GPU objects, no scene yet (`load`) – the splash can show before the scene's shaders are
    /// compiled. `format`: the surface's; `render_scale`: the render size × the surface's now; `scene_scale`: the scene's
    /// own, the quality rung multiplies it later; `veil` = the splash is black without the logo (#250).
    pub async fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat, size: (u32, u32), render_scale: f32, scene_scale: f32, veil: bool) -> Result<Stage> {
        let tex_entry = |binding, ty| wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::FRAGMENT, ty, count: None };
        let tex = wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pass"),
            entries: &[
                tex_entry(0, wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }),
                tex_entry(1, tex),
                tex_entry(2, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)),
                tex_entry(3, tex),
                tex_entry(4, tex),
            ],
        });
        let sampler = |filter| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            })
        };
        let (linear, nearest) = (sampler(wgpu::FilterMode::Linear), sampler(wgpu::FilterMode::Nearest));
        let dummy = texture(device, 1, 1, OFFSCREEN, wgpu::TextureUsages::TEXTURE_BINDING).create_view(&Default::default());
        let tex_usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
        let hud_tex = texture(device, hud::W, hud::H, OFFSCREEN, tex_usage);
        let bar_w = hud::bar_width(size.0 as f32, size.1 as f32);
        let bar_tex = bar_texture(device, bar_w);
        let (lw, lh, px) = crate::splash::logo((size.0 / 2).clamp(64, 2048))?;
        let logo = upload(device, queue, lw, lh, &px);
        tracing::info!("deck: build {}", crate::BUILD);
        let (lw, lh, lpx) = crate::build_label::raster(crate::BUILD);
        let label_tex = texture(device, lw, lh, OFFSCREEN, tex_usage);
        write(queue, &label_tex, lw, lh, &lpx);
        let pass = builtin("pass").context("the pass shader")?;
        let overlay = async |label: &str, body: &str, view: wgpu::TextureView, sampler: &wgpu::Sampler| -> Result<Overlay> {
            let pipeline = pipeline(device, &layout, label, body, format, true).await?;
            let ubuf = uniform_buffer(device);
            let bind = bind_group(device, &layout, &ubuf, [&view, &dummy, &dummy], sampler);
            Ok(Overlay { pipeline, ubuf, bind })
        };
        let of = |t: &wgpu::Texture| t.create_view(&Default::default());
        let splash = overlay("splash", builtin("splash").context("the splash shader")?, logo.view.clone(), &linear).await?;
        let hud_pass = overlay("hud", pass, of(&hud_tex), &nearest).await?;
        let bar_pass = overlay("hud bar", pass, of(&bar_tex), &nearest).await?;
        let label_pass = overlay("build label", pass, of(&label_tex), &nearest).await?;
        Ok(Stage {
            layout,
            linear,
            nearest,
            dummy,
            passes: Vec::new(),
            screen: None,
            size,
            render_scale: render_scale.clamp(0.25, 1.0),
            quality: Rung::default(),
            scene_scale: scene_scale.clamp(0.25, 1.0),
            slides: Vec::new(),
            outlet: None,
            space: None,
            host_sized: false,
            hud_pass,
            hud_tex,
            bar_pass,
            bar_tex,
            bar_w,
            ui: ui::Ui::new(device, format),
            label_pass,
            label_size: (lw, lh),
            splash,
            veil_only: veil,
            logo,
        })
    }

    /// The scene's passes (`format`: the surface's), its chapters' feeds, the targets at the render size.
    pub async fn load(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat, graph: &Graph, feeds: Vec<ChapterFeed>) -> Result<()> {
        for &i in &graph.order {
            let node = &graph.nodes[i];
            let body = match node.kind {
                Kind::TestPattern => builtin("test_pattern").unwrap().to_string(),
                Kind::Shader => shader_body(node)?,
                Kind::Screen => builtin("pass").unwrap().to_string(),
                Kind::Chapter => builtin("chapter").unwrap().to_string(),
                Kind::Overlay => builtin("overlay").unwrap().to_string(),
                Kind::Outlet => {
                    self.outlet = Some(OutletPlanes { node: i, rx: None, current: None, previous: None, fade: Crossfade::default() });
                    builtin("outlet").unwrap().to_string()
                }
                Kind::Space => {
                    self.passes.push(Pass { node: i, kind: Kind::Space, pipeline: None, ubuf: uniform_buffer(device), inputs: [None; 3], target: None, bind: None });
                    continue;
                }
                _ => continue,
            };
            let fmt = if node.kind == Kind::Screen { format } else { OFFSCREEN };
            let pipe = match pipeline(device, &self.layout, &node.id, &body, fmt, false).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("node '{}': the shader does not compile – showing its input instead\n{e:#}", node.id);
                    pipeline(device, &self.layout, &node.id, builtin("pass").unwrap(), fmt, false).await?
                }
            };
            let link = |port| node.kind.input(port).and_then(|k| graph.link_into(i, k)).map(|l| Src::Out(l.from.0, l.from.1));
            let inputs = match node.kind {
                Kind::Chapter | Kind::Outlet => [link("in"), Some(Src::Current(i)), Some(Src::Previous(i))],
                Kind::Overlay => [link("in"), link("top"), None],
                _ => [link("in"), None, None],
            };
            let pass = Pass { node: i, kind: node.kind, pipeline: Some(pipe), ubuf: uniform_buffer(device), inputs, target: None, bind: None };
            if node.kind == Kind::Screen {
                self.screen = Some(pass);
            } else {
                self.passes.push(pass);
            }
        }
        for feed in feeds {
            let node = feed.node;
            let q = graph.nodes[node].str("quantize").unwrap_or("off");
            let Some(quantize) = Quantize::parse(q) else {
                bail!("node '{}': unknown quantize '{q}' (off, beat, bar)", graph.nodes[node].id)
            };
            self.slides.push(Slides { node, feed, current: None, previous: None, caption: None, index: (0, 0), cut_at: 0.0, last_next: 0.0, quantize, pending: false, last_beat: 0.0 });
        }
        // a graph without a screen node (the browser's being): its last pass goes on the screen
        if self.screen.is_none() {
            if let Some(last) = self.passes.last().map(|p| p.node) {
                let pipe = pipeline(device, &self.layout, "screen", builtin("pass").unwrap(), format, false).await?;
                self.screen = Some(Pass { node: usize::MAX, kind: Kind::Screen, pipeline: Some(pipe), ubuf: uniform_buffer(device), inputs: [Some(Src::Out(last, 0)), None, None], target: None, bind: None });
            }
        }
        self.rebuild(device);
        Ok(())
    }

    /// The being was built anew (the browser): the space pass is node `node` now.
    pub fn set_space_node(&mut self, device: &wgpu::Device, node: usize) {
        let Some(p) = self.passes.iter_mut().find(|p| p.kind == Kind::Space) else { return };
        let old = Src::Out(p.node, 0);
        p.node = node;
        for input in self.screen.iter_mut().flat_map(|s| s.inputs.iter_mut()) {
            if *input == Some(old) {
                *input = Some(Src::Out(node, 0));
            }
        }
        self.rebind(device);
    }

    /// #220: the browser's governor sized the surface to the rung itself: only the space's lights and post passes follow
    pub fn set_host_quality(&mut self, rung: Rung) {
        self.host_sized = true;
        self.render_scale = 1.0;
        self.quality = rung;
        if let Some(g) = self.space.as_mut() {
            g.set_quality(rung);
        }
    }

    /// the space node's GPU side, once it drew (the dev mode's reload of the post stack)
    pub fn space_gpu(&mut self) -> Option<&mut SpaceGpu> {
        self.space.as_mut()
    }

    /// the composer's channel for the scene's outlet node (#186); without an outlet node it is dropped
    pub fn set_outlet(&mut self, rx: Receiver<Frame>) {
        if let Some(o) = self.outlet.as_mut() {
            o.rx = Some(rx);
        }
    }

    /// the outlet's crossfade at `time` (the HUD: still fading)
    pub fn outlet_fading(&self, time: f32) -> bool {
        self.outlet.as_ref().is_some_and(|o| o.fade.fading(time))
    }

    /// every chapter node shows a slice (the splash may go)
    pub fn slides_ready(&self) -> bool {
        self.slides.iter().all(|s| s.current.is_some())
    }

    /// per chapter node: "<slice> / <count>" or the loader's status
    pub fn slides_status(&self) -> Vec<String> {
        self.slides
            .iter()
            .map(|s| match s.current {
                Some(_) => format!("{} / {} {}", s.index.0 + 1, s.index.1, s.feed.status.lock().unwrap()),
                None => s.feed.status.lock().unwrap().clone(),
            })
            .collect()
    }

    /// The dev mode's hot reload: recompile the shader nodes `nodes` from their WGSL files. A file that doesn't read or
    /// compile keeps the running pipeline; the errors come back for the HUD.
    pub async fn reload_shaders(&mut self, device: &wgpu::Device, graph: &Graph, nodes: &[usize]) -> Vec<String> {
        let mut errors = Vec::new();
        for p in self.passes.iter_mut().filter(|p| nodes.contains(&p.node) && p.pipeline.is_some()) {
            let node = &graph.nodes[p.node];
            let compiled = match shader_body(node) {
                Ok(body) => pipeline(device, &self.layout, &node.id, &body, OFFSCREEN, false).await,
                Err(e) => Err(e),
            };
            match compiled {
                Ok(pl) => {
                    p.pipeline = Some(pl);
                    tracing::info!("node '{}': shader reloaded", node.id);
                }
                Err(e) => {
                    tracing::error!("node '{}': the shader does not compile – the last one runs on\n{e:#}", node.id);
                    errors.push(format!("node '{}': {e:#}", node.id));
                }
            }
        }
        errors
    }

    /// #220: a quality rung – the 3D's render size (the scene's `render_scale` × the rung's size: a scale of the window,
    /// or a height in pixels) and the space's lights and post passes. The offscreen targets are made anew only when the
    /// size changes (a step of the governor, never per frame).
    pub fn set_quality(&mut self, device: &wgpu::Device, rung: Rung, scene_scale: f32) {
        self.quality = rung;
        self.scene_scale = scene_scale;
        if let Some(g) = self.space.as_mut() {
            g.set_quality(rung);
        }
        let s = self.quality_scale();
        if (s - self.render_scale).abs() > 1e-3 {
            self.render_scale = s;
            self.rebuild(device);
        }
    }

    /// the scene's render_scale × the rung's size on this surface
    fn quality_scale(&self) -> f32 {
        if self.host_sized {
            return 1.0;
        }
        (self.scene_scale * self.quality.size.scale_for(self.size.1)).clamp(0.1, 4.0)
    }

    /// the render size × the screen's
    pub fn render_scale(&self) -> f32 {
        self.render_scale
    }

    fn render_size(&self) -> (u32, u32) {
        let s = self.render_scale;
        let side = |v: u32| ((v as f32 * s) as u32).clamp(1, MAX_SIDE);
        (side(self.size.0), side(self.size.1))
    }

    /// the surface's new size
    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        self.size = size;
        let bar_w = hud::bar_width(size.0 as f32, size.1 as f32);
        if bar_w != self.bar_w {
            self.bar_w = bar_w;
            self.bar_tex = bar_texture(device, bar_w);
            let view = self.bar_tex.create_view(&Default::default());
            self.bar_pass.bind = bind_group(device, &self.layout, &self.bar_pass.ubuf, [&view, &self.dummy, &self.dummy], &self.nearest);
        }
        // a rung with a height in pixels: its scale follows the window
        self.render_scale = self.quality_scale();
        self.rebuild(device);
    }

    /// the offscreen targets at the render size, then every bind group (an input's view changes with its target)
    fn rebuild(&mut self, device: &wgpu::Device) {
        let (w, h) = self.render_size();
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        for p in &mut self.passes {
            p.target = Some(Target { view: texture(device, w, h, OFFSCREEN, usage).create_view(&Default::default()) });
        }
        if let Some(s) = self.space.as_mut() {
            s.resize(device, (w, h));
        }
        self.rebind(device);
    }

    /// every bind group from the current views (after a resize, or a chapter's cut)
    fn rebind(&mut self, device: &wgpu::Device) {
        let text = text_port();
        let mut views: HashMap<Src, &wgpu::TextureView> = HashMap::new();
        for p in &self.passes {
            if let Some(t) = &p.target {
                views.insert(Src::Out(p.node, 0), &t.view);
            }
        }
        for s in &self.slides {
            for (src, img) in [(Src::Current(s.node), &s.current), (Src::Previous(s.node), &s.previous), (Src::Out(s.node, text), &s.caption)] {
                if let Some(i) = img {
                    views.insert(src, &i.view);
                }
            }
        }
        if let Some(o) = &self.outlet {
            for (src, img) in [(Src::Current(o.node), &o.current), (Src::Previous(o.node), &o.previous)] {
                if let Some(i) = img {
                    views.insert(src, &i.view);
                }
            }
        }
        let find = |s: Option<Src>| s.and_then(|s| views.get(&s).copied()).unwrap_or(&self.dummy);
        let bind = |p: &Pass| bind_group(device, &self.layout, &p.ubuf, p.inputs.map(find), &self.linear);
        let binds: Vec<wgpu::BindGroup> = self.passes.iter().map(bind).collect();
        let screen_bind = self.screen.as_ref().map(bind);
        for (p, b) in self.passes.iter_mut().zip(binds) {
            p.bind = Some(b);
        }
        if let Some(s) = self.screen.as_mut() {
            s.bind = screen_bind;
        }
    }

    /// The chapter nodes: a cut when the hold is over, or on `next` rising past `threshold` after `min_hold` – with
    /// `quantize` on the next beat / bar after that (a stopped clock doesn't hold it) – if the loader has the next slice
    /// ready. Returns whether a texture changed (the bind groups follow).
    fn advance(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, t: &Tick) -> bool {
        let mut changed = false;
        // the outlet: a composed cell → its plane on the GPU, the crossfade from the last one (#186)
        if let Some(o) = self.outlet.as_mut() {
            if let Some(f) = o.rx.as_ref().and_then(|rx| rx.try_recv().ok()) {
                let img = upload(device, queue, f.plane.width(), f.plane.height(), f.plane.as_raw());
                o.previous = o.current.replace(img);
                o.fade.cut(t.time, f.backdrop, f.enter_ms, f.exit_ms);
                changed = true;
            }
        }
        for s in &mut self.slides {
            let node = &t.graph.nodes[s.node];
            let hold = (t.input)(s.node, "hold", 6.0).max(0.5);
            let next = (t.input)(s.node, "next", 0.0);
            let (threshold, shortest) = (node.f32("threshold", 0.6), node.f32("min_hold", 1.5));
            let age = t.time - s.cut_at;
            let kick = next > threshold && s.last_next <= threshold && age >= shortest;
            s.last_next = next;
            let beat = t.tempo.pos.beat;
            let on_grid = !t.tempo.running || s.quantize.crossed(s.last_beat, beat, t.tempo.per_bar);
            s.last_beat = beat;
            if s.current.is_some() {
                s.pending |= age >= hold || kick;
                if !(s.pending && on_grid) {
                    continue;
                }
            }
            let Ok(Prepared { index, count, slice, caption }) = s.feed.rx.try_recv() else { continue };
            s.pending = false;
            let img = upload(device, queue, slice.width(), slice.height(), slice.as_raw());
            s.previous = s.current.replace(img);
            s.caption = caption.map(|c| upload(device, queue, c.width(), c.height(), c.as_raw()));
            s.index = (index, count);
            s.cut_at = t.time;
            changed = true;
        }
        changed
    }

    /// what a pass's control params can't say: a chapter's cut (fade, input, aspects, age), an overlay's top
    fn extra(&self, p: &Pass, time: f32, out: &mut [f32; 8]) {
        match p.kind {
            Kind::Chapter => {
                let Some(s) = self.slides.iter().find(|s| s.node == p.node) else { return };
                let age = time - s.cut_at;
                out[2] = if s.previous.is_some() { (age / FADE).clamp(0.0, 1.0) } else { 1.0 };
                out[3] = if p.inputs[0].is_some() { 1.0 } else { 0.0 };
                out[4] = s.current.as_ref().map_or(1.0, |c| c.aspect);
                out[5] = s.previous.as_ref().map_or(1.0, |c| c.aspect);
                out[6] = age;
            }
            Kind::Outlet => {
                let Some(o) = &self.outlet else { return };
                let [a, gone, cur_bg, prev_bg] = o.fade.mix(time);
                (out[1], out[2], out[3], out[4], out[5]) = (a, gone, cur_bg, prev_bg, if p.inputs[0].is_some() { 1.0 } else { 0.0 });
            }
            Kind::Overlay => {
                let top = p.inputs[1];
                if let Some(s) = self.slides.iter().find(|s| top == Some(Src::Out(s.node, text_port()))) {
                    if let Some(c) = &s.caption {
                        (out[4], out[5], out[6]) = (c.aspect, time - s.cut_at, 1.0);
                    }
                } else if top.is_some() {
                    let (w, h) = self.render_size();
                    (out[4], out[5], out[6]) = (w as f32 / h as f32, 1e3, 1.0);
                }
            }
            _ => {}
        }
    }

    /// #209: the stats bar's width in HUD pixels (what the host rasters it at)
    pub fn bar_width(&self) -> u32 {
        self.bar_w
    }

    /// The deck's menu (#386): Some = its page (deck-ui `Menu::paint`, a layer of deck-kit's screen) over the whole
    /// screen, over the splash and under the panel; None = gone. Called when it changes, not every frame.
    pub fn set_menu(&mut self, menu: Option<deck_kit::base::screen::Layer>) {
        self.ui.set_menu(menu);
    }

    /// The deck's panel (#481): Some = its layer (deck-show `Face::paint`) along the screen's bottom, over the scene, the
    /// HUD and the menu; None = gone. Called when it changes.
    pub fn set_panel(&mut self, panel: Option<deck_kit::base::screen::Layer>) {
        self.ui.set_panel(panel);
    }

    /// #485: an input of the person at the deck (the kit's, in device pixels) for egui's input state while the deck's
    /// own UI shows – what it means is the menu's and the panel's owners' (they set new layers).
    pub fn ui_input(&mut self, i: deck_kit::base::input::Input) {
        self.ui.input(i);
    }

    /// one frame of the splash alone (the first, before the scene's passes exist)
    pub fn show_splash(&self, device: &wgpu::Device, queue: &wgpu::Queue, view: &wgpu::TextureView) {
        let u = Uniforms { time: 0.0, dt: 0.0, res: [self.size.0 as f32, self.size.1 as f32], p: [1.0, self.logo.aspect, 1.0, if self.veil_only { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0, 0.0] };
        queue.write_buffer(&self.splash.ubuf, 0, bytemuck::bytes_of(&u));
        let mut enc = device.create_command_encoder(&Default::default());
        draw(&mut begin(&mut enc, view), &self.splash);
        queue.submit([enc.finish()]);
    }

    /// One frame into `enc`, ending on the surface's `view`. `space` = the scene's space node's state (its set's 3D arrives
    /// through it once).
    #[allow(clippy::too_many_arguments)]
    pub fn frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, t: &Tick, over: &Over, space: Option<&mut Space>) {
        self.scene(device, queue, enc, t, space);
        let (sw, sh) = (self.size.0 as f32, self.size.1 as f32);
        let splash = over.splash;
        if splash.0 > 0.0 {
            let p = [splash.0, self.logo.aspect, if splash.1 { 1.0 } else { 0.0 }, if self.veil_only { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0, 0.0];
            let u = Uniforms { time: t.time, dt: t.dt, res: [sw, sh], p };
            queue.write_buffer(&self.splash.ubuf, 0, bytemuck::bytes_of(&u));
        }
        // #485: egui's frame of the deck's own UI, when it changed – before the pass (its buffers are written)
        self.ui.prepare(device, queue, enc, self.size);
        // egui-wgpu draws into a pass that does not borrow the encoder's lifetime
        let mut rp = begin(enc, view).forget_lifetime();
        if let Some(s) = &self.screen {
            rp.set_pipeline(s.pipeline.as_ref().expect("the screen's pipeline"));
            rp.set_bind_group(0, s.bind.as_ref(), &[]);
            rp.draw(0..3, 0..1);
        }
        // #209: the collapsed bar across the top edge, at the HUD's scale (its raster is as wide as the screen)
        if let Some(hud) = over.hud.filter(|h| h.mode == hud::Mode::Bar && h.bar_w == self.bar_w && !h.bar.is_empty()) {
            write(queue, &self.bar_tex, self.bar_w, hud::BAR_H, &hud.bar);
            let s = hud::scale(sh);
            rp.set_viewport(0.0, 0.0, (self.bar_w as f32 * s).min(sw), (hud::BAR_H as f32 * s).min(sh), 0.0, 1.0);
            draw(&mut rp, &self.bar_pass);
        }
        if let Some(hud) = over.hud.filter(|h| h.mode == hud::Mode::Full) {
            write(queue, &self.hud_tex, hud::W, hud::H, &hud.pixels);
            // rem-like: an integer scale by the screen height (2 at 1080p), a 1rem margin at that scale
            let [x, y, hw, hh] = over.hud_at.unwrap_or_else(|| hud::rect(sw, sh));
            rp.set_viewport(x, y, hw, hh, 0.0, 1.0);
            draw(&mut rp, &self.hud_pass);
        }
        if splash.0 > 0.0 {
            rp.set_viewport(0.0, 0.0, sw, sh, 0.0, 1.0);
            draw(&mut rp, &self.splash);
        }
        // #386 #481: the deck's menu and its panel (deck-kit's egui screen), over the splash, under the build label
        self.ui.draw(&mut rp);
        // #423: the build label in the top right corner – over everything, in every scene and HUD mode (deck-ui places it)
        let [x, y, w, h] = deck_ui::label_rect(sw, sh, hud::label_scale(sw, sh), self.label_size.0, self.label_size.1);
        let x = if over.label_centred { (sw - w) / 2.0 } else { x };
        rp.set_viewport(x, y, w, h, 0.0, 1.0);
        draw(&mut rp, &self.label_pass);
    }

    /// the frame's offscreen passes: the chapters' cuts, the uniforms, every node's pass
    fn scene(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, t: &Tick, space: Option<&mut Space>) {
        if self.advance(device, queue, t) {
            self.rebind(device);
        }
        let (w, h) = self.render_size();
        let space: Option<&Space> = match space {
            Some(s) => {
                let quality = self.quality;
                let gpu = self.space.get_or_insert_with(|| {
                    let mut g = SpaceGpu::new(device, queue, s, OFFSCREEN, (w, h));
                    g.set_quality(quality);
                    g
                });
                gpu.sync(device, queue, s);
                Some(&*s)
            }
            None => None,
        };
        for p in self.passes.iter().chain(self.screen.as_ref()) {
            let mut params = params(t, p);
            self.extra(p, t.time, &mut params);
            let u = Uniforms { time: t.time, dt: t.dt, res: [w as f32, h as f32], p: params };
            queue.write_buffer(&p.ubuf, 0, bytemuck::bytes_of(&u));
        }
        for p in &self.passes {
            let target = &p.target.as_ref().unwrap().view;
            let Some(pipeline) = &p.pipeline else {
                if let (Some(gpu), Some(s)) = (self.space.as_mut(), space) {
                    gpu.draw(queue, enc, target, (w, h), s, t.time);
                }
                continue;
            };
            let mut rp = begin(enc, target);
            rp.set_pipeline(pipeline);
            rp.set_bind_group(0, p.bind.as_ref(), &[]);
            rp.draw(0..3, 0..1);
        }
    }
}

fn draw(rp: &mut wgpu::RenderPass<'_>, o: &Overlay) {
    rp.set_pipeline(&o.pipeline);
    rp.set_bind_group(0, &o.bind, &[]);
    rp.draw(0..3, 0..1);
}

/// RGBA pixels into a `w` × `h` texture
fn write(queue: &wgpu::Queue, tex: &wgpu::Texture, w: u32, h: u32, px: &[u8]) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo { texture: tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        px,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: None },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
}

/// #209: the stats bar's texture, `w` HUD pixels wide
fn bar_texture(device: &wgpu::Device, w: u32) -> wgpu::Texture {
    texture(device, w, hud::BAR_H, OFFSCREEN, wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST)
}

fn bind_group(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, ubuf: &wgpu::Buffer, v: [&wgpu::TextureView; 3], sampler: &wgpu::Sampler) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: ubuf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(v[0]) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(v[1]) },
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(v[2]) },
        ],
    })
}

/// a pass's eight params: its control inputs (linked or the node's own) in port order, defaults per kind; the chapter's
/// hold rides in the last slot (the renderer fills slots 2–6, `extra`)
fn params(t: &Tick, p: &Pass) -> [f32; 8] {
    let mut out = [0.0; 8];
    let names: &[&str] = match p.kind {
        Kind::TestPattern => &["speed", "zoom", "hue", "pulse"],
        Kind::Shader => &["a", "b", "c", "d"],
        Kind::Chapter => &["zoom", "pulse"],
        Kind::Overlay => &["opacity", "pulse", "shake"],
        Kind::Outlet => &["opacity"],
        _ => &[],
    };
    let input = |name: &str| (t.input)(p.node, name, crate::params::default(p.kind, name));
    for (i, name) in names.iter().enumerate() {
        out[i] = input(name);
    }
    if p.kind == Kind::Chapter {
        out[7] = input("hold");
    }
    out
}

/// the chapter kind's caption port
fn text_port() -> usize {
    Kind::Chapter.output("text").expect("chapter.text")
}

/// an RGBA8 image as a sampled texture
fn upload(device: &wgpu::Device, queue: &wgpu::Queue, w: u32, h: u32, rgba: &[u8]) -> Image {
    let tex = texture(device, w, h, OFFSCREEN, wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST);
    write(queue, &tex, w, h, rgba);
    Image { view: tex.create_view(&Default::default()), aspect: w as f32 / h.max(1) as f32 }
}

/// a shader node's body: `effect` = a built-in name, or `wgsl` = a file defining `fn effect(uv) -> vec4<f32>`
fn shader_body(node: &Node) -> Result<String> {
    if let Some(path) = node.str("wgsl") {
        return std::fs::read_to_string(path).with_context(|| format!("node '{}': reading {path}", node.id));
    }
    let name = node.str("effect").unwrap_or("pass");
    builtin(name).map(str::to_string).with_context(|| format!("node '{}': unknown effect '{name}' (kaleido, rgb_shift, invert, pass, fireflies – or wgsl = a file)", node.id))
}

/// A fullscreen pass: common.wgsl + `body`. Async because the shader's verdict is (a native host blocks on it).
async fn pipeline(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, label: &str, body: &str, format: wgpu::TextureFormat, blend: bool) -> Result<wgpu::RenderPipeline> {
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some(label), source: wgpu::ShaderSource::Wgsl(format!("{COMMON}\n{body}").into()) });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some(label), bind_group_layouts: &[Some(layout)], immediate_size: 0 });
    let p = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&pl),
        vertex: wgpu::VertexState { module: &module, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &[] },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState { format, blend: blend.then_some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })],
        }),
        multiview_mask: None,
        cache: None,
    });
    if let Some(e) = scope.pop().await {
        bail!("{e}");
    }
    Ok(p)
}

pub fn begin<'a>(enc: &'a mut wgpu::CommandEncoder, view: &'a wgpu::TextureView) -> wgpu::RenderPass<'a> {
    enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        multiview_mask: None,
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    })
}

fn texture(device: &wgpu::Device, w: u32, h: u32, format: wgpu::TextureFormat, usage: wgpu::TextureUsages) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

fn uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor { label: None, size: std::mem::size_of::<Uniforms>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false })
}
