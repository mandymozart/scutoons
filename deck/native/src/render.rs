//! The renderer (CYBERDECKS.md §8 "Render"): wgpu (Vulkan on the deck; DX12 / Vulkan / Metal / GL elsewhere), one
//! fullscreen pass per video node in graph order, each into its own texture; the screen node blits its input to the
//! surface (vsync, Fifo), the HUD goes on top. A software adapter (llvmpipe / lavapipe / WARP) is the chosen fallback
//! when no hardware adapter answers, at half the render size. A shader that does not compile falls back to `pass`.
//! S3: a pass reads up to three textures (its `in`, a chapter's current / previous slice, an overlay's `top`); the
//! chapter nodes' slices arrive from their loaders (chapter.rs) and are uploaded when the node cuts; the splash (the
//! logo, splash.rs) goes over everything – its first frame is shown before the scene's shaders are compiled.
//! #186: an `outlet` node cross-fades the composed planes of its cells (live/) over its `in` (shaders/outlet.wgsl).
//! #188: a `space` node is not a fullscreen pass – space/gpu.rs draws the set and the being into its texture with a
//! depth buffer (the `space` module, #185). `capture` saves the next frame as a PNG (`--screenshot`). `reload_shaders`
//! recompiles the passes whose WGSL file changed (the dev mode's hot reload).
use crate::chapter::{ChapterFeed, Prepared};
use crate::control::Controls;
use crate::graph::{Graph, Kind};
use crate::hud::{self, Hud};
use crate::live::frame::{Crossfade, Frame};
#[cfg(feature = "space")]
use crate::space::gpu::SpaceGpu;
#[cfg(feature = "space")]
pub use crate::space::Space;
use crate::tempo::Quantize;
use deck_space::quality::{Rung, MAX_SIDE};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use winit::window::Window;

const COMMON: &str = include_str!("shaders/common.wgsl");
const OFFSCREEN: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// how long a chapter's cut cross-fades (s)
const FADE: f32 = 0.35;

fn builtin(name: &str) -> Option<&'static str> {
    Some(match name {
        "test_pattern" => include_str!("shaders/test_pattern.wgsl"),
        "kaleido" => include_str!("shaders/kaleido.wgsl"),
        "rgb_shift" => include_str!("shaders/rgb_shift.wgsl"),
        "invert" => include_str!("shaders/invert.wgsl"),
        "pass" => include_str!("shaders/pass.wgsl"),
        "fireflies" => include_str!("shaders/fireflies.wgsl"),
        "flash" => include_str!("shaders/flash.wgsl"),
        "chapter" => include_str!("shaders/chapter.wgsl"),
        "overlay" => include_str!("shaders/overlay.wgsl"),
        "outlet" => include_str!("shaders/outlet.wgsl"),
        "splash" => include_str!("shaders/splash.wgsl"),
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

struct Target {
    view: wgpu::TextureView,
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
    rx: Option<std::sync::mpsc::Receiver<Frame>>,
    current: Option<Image>,
    previous: Option<Image>,
    fade: Crossfade,
}

/// Without the `space` module no scene has a space node (modules.rs refuses it): nothing to draw.
#[cfg(not(feature = "space"))]
pub enum Space {}

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub adapter: wgpu::AdapterInfo,
    layout: wgpu::BindGroupLayout,
    linear: wgpu::Sampler,
    nearest: wgpu::Sampler,
    dummy: wgpu::TextureView,
    passes: Vec<Pass>,
    screen: Option<Pass>,
    hud_pass: Pass,
    hud_tex: wgpu::Texture,
    /// #209: the collapsed stats bar – its texture as wide as the screen in HUD pixels (`hud::bar_width`)
    bar_pass: Pass,
    bar_tex: wgpu::Texture,
    bar_w: u32,
    /// the pairing card (#180, pairing.rs): its texture, shown while `pairing` is set
    pair_pass: Pass,
    pair_tex: wgpu::Texture,
    pairing: bool,
    render_scale: f32,
    /// #220: the quality rung's knobs (deck-space quality.rs – the governor in main.rs sets them) and the scene's
    /// render_scale under them
    quality: Rung,
    scene_scale: f32,
    slides: Vec<Slides>,
    /// the outlet node's planes (#186)
    outlet: Option<OutletPlanes>,
    splash: Pass,
    /// the splash is a black veil without the logo (#250)
    pub veil_only: bool,
    logo: Image,
    /// the space node's GPU side, made at its first frame
    #[cfg(feature = "space")]
    space: Option<SpaceGpu>,
    /// save the next frame here (PNG), then forget it
    pub capture: Option<PathBuf>,
    can_capture: bool,
}

/// #257: the backends the first instance gets – `WGPU_BACKEND` if set, else Vulkan alone on Linux (all elsewhere)
fn backends_first() -> wgpu::Backends {
    wgpu::Backends::from_env().unwrap_or_else(|| default_backends(cfg!(target_os = "linux")))
}

fn default_backends(linux: bool) -> wgpu::Backends {
    if linux { wgpu::Backends::VULKAN } else { wgpu::Backends::all() }
}

/// where to go when `tried` had no adapter: GL, only after Vulkan alone found nothing (None = nothing left)
fn backends_fallback(tried: wgpu::Backends) -> Option<wgpu::Backends> {
    (tried == wgpu::Backends::VULKAN).then_some(wgpu::Backends::all())
}

impl Renderer {
    /// `veil` = the splash's first frame is black without the logo (#250: a scene switch fades through black)
    pub fn new(window: Arc<Window>, graph: &Graph, render_scale: f32, feeds: Vec<ChapterFeed>, veil: bool) -> Result<Renderer> {
        // #257: on Linux the instance starts with Vulkan alone – a GL backend in the instance opens its own EGL
        // display on the Wayland connection, and when the compositor dies that one segfaults (libEGL_mesa)
        let mut backends = backends_first();
        let (surface, adapter) = loop {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends, ..wgpu::InstanceDescriptor::from_env_or_default() });
            let surface = instance.create_surface(window.clone()).context("creating the window surface")?;
            let ask = |fallback| {
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    force_fallback_adapter: fallback,
                    compatible_surface: Some(&surface),
                }))
            };
            match ask(false) {
                Ok(a) => break (surface, a),
                Err(e) => match backends_fallback(backends) {
                    Some(next) => {
                        tracing::warn!("render: no adapter on {backends:?} ({e}) – trying {next:?}");
                        backends = next;
                    }
                    None => {
                        tracing::warn!("render: no hardware adapter ({e}) – the software fallback");
                        let a = ask(true).context("no GPU adapter at all, not even a software one")?;
                        break (surface, a);
                    }
                },
            }
        };
        let info = adapter.get_info();
        let software = info.device_type == wgpu::DeviceType::Cpu;
        tracing::info!("render: {} ({:?}, {:?}, driver {})", info.name, info.backend, info.device_type, info.driver_info);
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("deck-native"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))?;
        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let Some(&first) = caps.formats.first() else { bail!("the surface offers no format") };
        // shaders write display values: a non-sRGB surface keeps them as written
        let format = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(first);
        // a screenshot copies the surface's texture – where the surface allows it
        let can_capture = caps.usages.contains(wgpu::TextureUsages::COPY_SRC);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | if can_capture { wgpu::TextureUsages::COPY_SRC } else { wgpu::TextureUsages::empty() },
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let tex_entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty,
            count: None,
        };
        let tex = wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pass"),
            entries: &[
                tex_entry(0, wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                }),
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
        let dummy = texture(&device, 1, 1, OFFSCREEN, wgpu::TextureUsages::TEXTURE_BINDING)
            .create_view(&Default::default());
        let hud_tex = texture(&device, hud::W, hud::H, OFFSCREEN, wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST);
        let bar_w = hud::bar_width(config.width as f32, config.height as f32);
        let bar_tex = bar_texture(&device, bar_w);
        let pair_tex = texture(&device, crate::pairing::W, crate::pairing::H, OFFSCREEN, wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST);
        // the splash first – one frame of it at once, the scene's shaders compile behind it
        let (lw, lh, px) = crate::splash::logo((config.width / 2).clamp(64, 2048))?;
        let logo = upload(&device, &queue, lw, lh, &px);
        let overlay_pass = |label: &str, body: &str| -> Result<Pass> {
            Ok(Pass {
                node: usize::MAX,
                kind: Kind::Screen,
                pipeline: Some(pipeline(&device, &layout, label, body, format, true)?),
                ubuf: uniform_buffer(&device),
                inputs: [None; 3],
                target: None,
                bind: None,
            })
        };
        let splash = overlay_pass("splash", builtin("splash").unwrap())?;
        let hud_pass = overlay_pass("hud", builtin("pass").unwrap())?;
        let bar_pass = overlay_pass("hud bar", builtin("pass").unwrap())?;
        let pair_pass = overlay_pass("pairing", builtin("pass").unwrap())?;

        let mut r = Renderer {
            hud_pass,
            bar_pass,
            bar_tex,
            bar_w,
            pair_pass,
            pair_tex,
            pairing: false,
            splash,
            veil_only: veil,
            logo,
            surface,
            device,
            queue,
            config,
            adapter: info,
            layout,
            linear,
            nearest,
            dummy,
            passes: Vec::new(),
            screen: None,
            hud_tex,
            render_scale: if software { render_scale.min(0.5) } else { render_scale }.clamp(0.25, 1.0),
            quality: Rung::default(),
            scene_scale: render_scale.clamp(0.25, 1.0),
            slides: Vec::new(),
            outlet: None,
            #[cfg(feature = "space")]
            space: None,
            capture: None,
            can_capture,
        };
        r.rebind();
        r.show_splash();
        for &i in &graph.order {
            let node = &graph.nodes[i];
            let body = match node.kind {
                Kind::TestPattern => builtin("test_pattern").unwrap().to_string(),
                Kind::Shader => shader_body(node)?,
                Kind::Screen => builtin("pass").unwrap().to_string(),
                Kind::Chapter => builtin("chapter").unwrap().to_string(),
                Kind::Overlay => builtin("overlay").unwrap().to_string(),
                Kind::Outlet => {
                    r.outlet = Some(OutletPlanes { node: i, rx: None, current: None, previous: None, fade: Crossfade::default() });
                    builtin("outlet").unwrap().to_string()
                }
                Kind::Space => {
                    r.passes.push(Pass { node: i, kind: Kind::Space, pipeline: None, ubuf: uniform_buffer(&r.device), inputs: [None; 3], target: None, bind: None });
                    continue;
                }
                _ => continue,
            };
            let fmt = if node.kind == Kind::Screen { format } else { OFFSCREEN };
            let pipeline = match pipeline(&r.device, &r.layout, &node.id, &body, fmt, false) {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("node '{}': the shader does not compile – showing its input instead\n{e:#}", node.id);
                    pipeline(&r.device, &r.layout, &node.id, builtin("pass").unwrap(), fmt, false)?
                }
            };
            let link = |port| node.kind.input(port).and_then(|k| graph.link_into(i, k)).map(|l| Src::Out(l.from.0, l.from.1));
            let inputs = match node.kind {
                Kind::Chapter | Kind::Outlet => [link("in"), Some(Src::Current(i)), Some(Src::Previous(i))],
                Kind::Overlay => [link("in"), link("top"), None],
                _ => [link("in"), None, None],
            };
            let pass = Pass { node: i, kind: node.kind, pipeline: Some(pipeline), ubuf: uniform_buffer(&r.device), inputs, target: None, bind: None };
            if node.kind == Kind::Screen {
                r.screen = Some(pass);
            } else {
                r.passes.push(pass);
            }
        }
        for feed in feeds {
            let node = feed.node;
            let q = graph.nodes[node].str("quantize").unwrap_or("off");
            let Some(quantize) = Quantize::parse(q) else {
                bail!("node '{}': unknown quantize '{q}' (off, beat, bar)", graph.nodes[node].id)
            };
            r.slides.push(Slides {
                node,
                feed,
                current: None,
                previous: None,
                caption: None,
                index: (0, 0),
                cut_at: 0.0,
                last_next: 0.0,
                quantize,
                pending: false,
                last_beat: 0.0,
            });
        }
        r.rebuild();
        Ok(r)
    }

    /// the composer's channel for the scene's outlet node (live/, #186); without an outlet node it is dropped
    pub fn set_outlet(&mut self, rx: std::sync::mpsc::Receiver<Frame>) {
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
    pub fn reload_shaders(&mut self, graph: &Graph, nodes: &[usize]) -> Vec<String> {
        let mut errors = Vec::new();
        for p in self.passes.iter_mut().filter(|p| nodes.contains(&p.node) && p.pipeline.is_some()) {
            let node = &graph.nodes[p.node];
            match shader_body(node).and_then(|body| pipeline(&self.device, &self.layout, &node.id, &body, OFFSCREEN, false)) {
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

    /// The dev mode's hot reload of the post stack (#204): the passes `effects` from their WGSL files (deck-space
    /// shaders/post/, with the prelude common.wgsl from there too). A pass that doesn't read or compile keeps the
    /// running one; the errors come back for the HUD. Before the space's first frame there is nothing to reload.
    #[cfg(feature = "space")]
    pub fn reload_post(&mut self, effects: &[crate::space::post::Effect]) -> Vec<String> {
        use crate::space::post;
        let mut errors = Vec::new();
        let Some(gpu) = self.space.as_mut() else { return errors };
        let read = |file: &str| std::fs::read_to_string(post::source_path(file)).with_context(|| format!("post: reading {file}"));
        for &e in effects {
            let src = match read("common.wgsl").and_then(|c| Ok((c, read(e.file())?))) {
                Ok(s) => s,
                Err(err) => {
                    errors.push(format!("{err:#}"));
                    continue;
                }
            };
            self.device.push_error_scope(wgpu::ErrorFilter::Validation);
            let pipes = gpu.compile_post(&self.device, e, &src.0, &src.1);
            match pollster::block_on(self.device.pop_error_scope()) {
                Some(err) => {
                    tracing::error!("post '{}': the shader does not compile – the last one runs on\n{err}", e.name());
                    errors.push(format!("post '{}': {err}", e.name()));
                }
                None => {
                    if let Some(p) = pipes {
                        gpu.install_post(e, p);
                        tracing::info!("post '{}': reloaded", e.name());
                    }
                }
            }
        }
        errors
    }

    /// #220: a quality rung – the 3D's render size (the scene's `render_scale` × the rung's size: a scale of the window,
    /// or a height in pixels; the HUD stays native) and the space's lights and post passes. The offscreen targets are
    /// made anew only when the size changes (a step of the governor, never per frame).
    pub fn set_quality(&mut self, rung: Rung, scene_scale: f32) {
        self.quality = rung;
        self.scene_scale = scene_scale;
        #[cfg(feature = "space")]
        if let Some(g) = self.space.as_mut() {
            g.set_quality(rung);
        }
        let s = self.quality_scale();
        if (s - self.render_scale).abs() > 1e-3 {
            self.render_scale = s;
            self.rebuild();
        }
    }

    /// the scene's render_scale × the rung's size on this window
    fn quality_scale(&self) -> f32 {
        (self.scene_scale * self.quality.size.scale_for(self.config.height)).clamp(0.1, 4.0)
    }

    /// the render size × the screen's
    pub fn render_scale(&self) -> f32 {
        self.render_scale
    }

    fn render_size(&self) -> (u32, u32) {
        let s = self.render_scale;
        let side = |v: u32| ((v as f32 * s) as u32).clamp(1, MAX_SIDE);
        (side(self.config.width), side(self.config.height))
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        let bar_w = hud::bar_width(w as f32, h as f32);
        if bar_w != self.bar_w {
            self.bar_w = bar_w;
            self.bar_tex = bar_texture(&self.device, bar_w);
        }
        // a rung with a height in pixels: its scale follows the window
        self.render_scale = self.quality_scale();
        self.rebuild();
    }

    /// the offscreen targets at the render size, then every bind group (an input's view changes with its target)
    fn rebuild(&mut self) {
        let (w, h) = self.render_size();
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        for p in &mut self.passes {
            p.target = Some(Target { view: texture(&self.device, w, h, OFFSCREEN, usage).create_view(&Default::default()) });
        }
        #[cfg(feature = "space")]
        if let Some(s) = self.space.as_mut() {
            s.resize(&self.device, (w, h));
        }
        self.rebind();
    }

    /// every bind group from the current views (after a resize, or a chapter's cut)
    fn rebind(&mut self) {
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
        let bind = |p: &Pass, v: [&wgpu::TextureView; 3], sampler: &wgpu::Sampler| {
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: p.ubuf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(v[0]) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(v[1]) },
                    wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(v[2]) },
                ],
            })
        };
        let binds: Vec<wgpu::BindGroup> = self.passes.iter().map(|p| bind(p, p.inputs.map(find), &self.linear)).collect();
        let screen_bind = self.screen.as_ref().map(|s| bind(s, s.inputs.map(find), &self.linear));
        let hud_view = self.hud_tex.create_view(&Default::default());
        let hud_bind = bind(&self.hud_pass, [&hud_view, &self.dummy, &self.dummy], &self.nearest);
        let bar_view = self.bar_tex.create_view(&Default::default());
        let bar_bind = bind(&self.bar_pass, [&bar_view, &self.dummy, &self.dummy], &self.nearest);
        let pair_view = self.pair_tex.create_view(&Default::default());
        let pair_bind = bind(&self.pair_pass, [&pair_view, &self.dummy, &self.dummy], &self.nearest);
        let splash_bind = bind(&self.splash, [&self.logo.view, &self.dummy, &self.dummy], &self.linear);
        for (p, b) in self.passes.iter_mut().zip(binds) {
            p.bind = Some(b);
        }
        if let Some(s) = self.screen.as_mut() {
            s.bind = screen_bind;
        }
        self.hud_pass.bind = Some(hud_bind);
        self.bar_pass.bind = Some(bar_bind);
        self.pair_pass.bind = Some(pair_bind);
        self.splash.bind = Some(splash_bind);
    }

    /// #209: the stats bar's width in HUD pixels (what main rasters it at)
    pub fn bar_width(&self) -> u32 {
        self.bar_w
    }

    /// The pairing card (#180): Some = its pixels (pairing::W × pairing::H, from `pairing::Card::raster`), shown in the
    /// lower right corner over everything; None = gone. Called when it changes, not every frame.
    pub fn set_pairing(&mut self, card: Option<&[u8]>) {
        if let Some(px) = card {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo { texture: &self.pair_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                px,
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(crate::pairing::W * 4), rows_per_image: None },
                wgpu::Extent3d { width: crate::pairing::W, height: crate::pairing::H, depth_or_array_layers: 1 },
            );
        }
        self.pairing = card.is_some();
    }

    /// one frame of the splash alone (the window's first, before the scene's passes exist)
    fn show_splash(&mut self) {
        let Ok(frame) = self.surface.get_current_texture() else { return };
        let res = [self.config.width as f32, self.config.height as f32];
        let u = Uniforms { time: 0.0, dt: 0.0, res, p: [1.0, self.logo.aspect, 1.0, if self.veil_only { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0, 0.0] };
        self.queue.write_buffer(&self.splash.ubuf, 0, bytemuck::bytes_of(&u));
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let view = frame.texture.create_view(&Default::default());
            let mut rp = begin(&mut enc, &view);
            rp.set_pipeline(self.splash.pipeline.as_ref().expect("the splash's pipeline"));
            rp.set_bind_group(0, self.splash.bind.as_ref(), &[]);
            rp.draw(0..3, 0..1);
        }
        self.queue.submit([enc.finish()]);
        frame.present();
    }

    /// The chapter nodes: a cut when the hold is over, or on `next` rising past `threshold` after `min_hold` – with
    /// `quantize` on the next beat / bar after that (a stopped clock doesn't hold it) – if the loader has the next slice
    /// ready. Returns whether a texture changed (the bind groups follow).
    fn advance(&mut self, graph: &Graph, ctl: &Controls, time: f32) -> bool {
        let mut changed = false;
        // the outlet: a composed cell → its plane on the GPU, the crossfade from the last one (#186)
        if let Some(o) = self.outlet.as_mut() {
            if let Some(f) = o.rx.as_ref().and_then(|rx| rx.try_recv().ok()) {
                let img = upload(&self.device, &self.queue, f.plane.width(), f.plane.height(), f.plane.as_raw());
                o.previous = o.current.replace(img);
                o.fade.cut(time, f.backdrop, f.enter_ms, f.exit_ms);
                changed = true;
            }
        }
        for s in &mut self.slides {
            let node = &graph.nodes[s.node];
            let hold = ctl.input(s.node, "hold", 6.0).max(0.5);
            let next = ctl.input(s.node, "next", 0.0);
            let (threshold, shortest) = (node.f32("threshold", 0.6), node.f32("min_hold", 1.5));
            let age = time - s.cut_at;
            let kick = next > threshold && s.last_next <= threshold && age >= shortest;
            s.last_next = next;
            let beat = ctl.tempo.pos.beat;
            let on_grid = !ctl.tempo.running || s.quantize.crossed(s.last_beat, beat, ctl.tempo.per_bar);
            s.last_beat = beat;
            if s.current.is_some() {
                s.pending |= age >= hold || kick;
                if !(s.pending && on_grid) {
                    continue;
                }
            }
            let Ok(Prepared { index, count, slice, caption }) = s.feed.rx.try_recv() else { continue };
            s.pending = false;
            let img = upload(&self.device, &self.queue, slice.width(), slice.height(), slice.as_raw());
            s.previous = s.current.replace(img);
            s.caption = caption.map(|c| upload(&self.device, &self.queue, c.width(), c.height(), c.as_raw()));
            s.index = (index, count);
            s.cut_at = time;
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

    /// One frame; returns the CPU time spent after the surface handed out its texture (the vsync wait excluded), in ms.
    /// `splash` = the splash's alpha (0 = none) and whether its loading line runs.
    /// `space` = the scene's space node's state (its set's 3D arrives through it once).
    #[allow(clippy::too_many_arguments)]
    pub fn frame(&mut self, graph: &Graph, ctl: &Controls, time: f32, dt: f32, hud: Option<&Hud>, splash: (f32, bool), space: Option<&mut Space>) -> Result<f32> {
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(0.0);
            }
            Err(wgpu::SurfaceError::OutOfMemory) => bail!("the GPU is out of memory"),
            Err(e) => {
                tracing::debug!("render: skipping a frame ({e})");
                return Ok(0.0);
            }
        };
        let work = std::time::Instant::now();
        if self.advance(graph, ctl, time) {
            self.rebind();
        }
        let (w, h) = self.render_size();
        #[cfg(not(feature = "space"))]
        let _ = space;
        #[cfg(feature = "space")]
        let space: Option<&Space> = match space {
            Some(s) => {
                let quality = self.quality;
                let gpu = self.space.get_or_insert_with(|| {
                    let mut g = SpaceGpu::new(&self.device, &self.queue, s, OFFSCREEN, (w, h));
                    g.set_quality(quality);
                    g
                });
                gpu.sync(&self.device, &self.queue, s);
                Some(&*s)
            }
            None => None,
        };
        for p in self.passes.iter().chain(self.screen.as_ref()) {
            let mut params = params(ctl, p);
            self.extra(p, time, &mut params);
            let u = Uniforms { time, dt, res: [w as f32, h as f32], p: params };
            self.queue.write_buffer(&p.ubuf, 0, bytemuck::bytes_of(&u));
        }
        let (sw, sh) = (self.config.width as f32, self.config.height as f32);
        if splash.0 > 0.0 {
            let p = [splash.0, self.logo.aspect, if splash.1 { 1.0 } else { 0.0 }, if self.veil_only { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0, 0.0];
            let u = Uniforms { time, dt, res: [sw, sh], p };
            self.queue.write_buffer(&self.splash.ubuf, 0, bytemuck::bytes_of(&u));
        }
        let mut enc = self.device.create_command_encoder(&Default::default());
        for p in &self.passes {
            let target = &p.target.as_ref().unwrap().view;
            let Some(pipeline) = &p.pipeline else {
                #[cfg(feature = "space")]
                if let (Some(gpu), Some(s)) = (self.space.as_mut(), space) {
                    gpu.draw(&self.queue, &mut enc, target, (w, h), s, time);
                }
                continue;
            };
            let mut rp = begin(&mut enc, target);
            rp.set_pipeline(pipeline);
            rp.set_bind_group(0, p.bind.as_ref(), &[]);
            rp.draw(0..3, 0..1);
        }
        let view = frame.texture.create_view(&Default::default());
        {
            let mut rp = begin(&mut enc, &view);
            if let Some(s) = &self.screen {
                rp.set_pipeline(s.pipeline.as_ref().expect("the screen's pipeline"));
                rp.set_bind_group(0, s.bind.as_ref(), &[]);
                rp.draw(0..3, 0..1);
            }
            // #209: the collapsed bar across the top edge, at the HUD's scale (its raster is as wide as the screen)
            if let Some(hud) = hud.filter(|h| h.mode == hud::Mode::Bar && h.bar_w == self.bar_w && !h.bar.is_empty()) {
                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo { texture: &self.bar_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                    &hud.bar,
                    wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(self.bar_w * 4), rows_per_image: None },
                    wgpu::Extent3d { width: self.bar_w, height: hud::BAR_H, depth_or_array_layers: 1 },
                );
                let s = hud::scale(sh);
                rp.set_viewport(0.0, 0.0, (self.bar_w as f32 * s).min(sw), (hud::BAR_H as f32 * s).min(sh), 0.0, 1.0);
                rp.set_pipeline(self.bar_pass.pipeline.as_ref().expect("the bar's pipeline"));
                rp.set_bind_group(0, self.bar_pass.bind.as_ref(), &[]);
                rp.draw(0..3, 0..1);
            }
            if let Some(hud) = hud.filter(|h| h.mode == hud::Mode::Full) {
                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &self.hud_tex,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &hud.pixels,
                    wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(hud::W * 4), rows_per_image: None },
                    wgpu::Extent3d { width: hud::W, height: hud::H, depth_or_array_layers: 1 },
                );
                // rem-like: an integer scale by the screen height (2 at 1080p), a 1rem margin at that scale
                let s = hud::scale(sh);
                let (hw, hh) = ((hud::W as f32 * s).min(sw), (hud::H as f32 * s).min(sh));
                let m = (16.0 * s).min(sw - hw).max(0.0);
                rp.set_viewport(m, m.min(sh - hh).max(0.0), hw, hh, 0.0, 1.0);
                rp.set_pipeline(self.hud_pass.pipeline.as_ref().expect("the HUD's pipeline"));
                rp.set_bind_group(0, self.hud_pass.bind.as_ref(), &[]);
                rp.draw(0..3, 0..1);
            }
            if splash.0 > 0.0 {
                rp.set_viewport(0.0, 0.0, sw, sh, 0.0, 1.0);
                rp.set_pipeline(self.splash.pipeline.as_ref().expect("the splash's pipeline"));
                rp.set_bind_group(0, self.splash.bind.as_ref(), &[]);
                rp.draw(0..3, 0..1);
            }
            if self.pairing {
                // the HUD's scale (2 at 1080p), its 1rem margin, in the lower right corner
                let s = (sh / 540.0).round().max(1.0);
                let (cw, ch) = ((crate::pairing::W as f32 * s).min(sw), (crate::pairing::H as f32 * s).min(sh));
                let m = 16.0 * s;
                rp.set_viewport((sw - cw - m).max(0.0), (sh - ch - m).max(0.0), cw, ch, 0.0, 1.0);
                rp.set_pipeline(self.pair_pass.pipeline.as_ref().expect("the pairing card's pipeline"));
                rp.set_bind_group(0, self.pair_pass.bind.as_ref(), &[]);
                rp.draw(0..3, 0..1);
            }
        }
        let shot = self.capture.take().filter(|_| self.can_capture).map(|path| {
            let (w, h) = (self.config.width, self.config.height);
            let row = (w * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
            let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("screenshot"),
                size: (row * h) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            enc.copy_texture_to_buffer(
                frame.texture.as_image_copy(),
                wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: None } },
                wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            );
            (path, buf, row, w, h)
        });
        self.queue.submit([enc.finish()]);
        let ms = work.elapsed().as_secs_f32() * 1000.0;
        if let Some((path, buf, row, w, h)) = shot {
            if let Err(e) = self.save(&path, &buf, row, w, h) {
                tracing::error!("screenshot {}: {e:#}", path.display());
            }
        }
        frame.present();
        Ok(ms)
    }

    /// the copied frame → a PNG (BGRA surfaces swapped to RGBA)
    fn save(&self, path: &std::path::Path, buf: &wgpu::Buffer, row: u32, w: u32, h: u32) -> Result<()> {
        let slice = buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::PollType::Wait)?;
        let data = slice.get_mapped_range();
        let bgra = matches!(self.config.format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
        let mut px = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for p in data[(y * row) as usize..(y * row + w * 4) as usize].chunks_exact(4) {
                px.extend_from_slice(&if bgra { [p[2], p[1], p[0], 255] } else { [p[0], p[1], p[2], 255] });
            }
        }
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)?;
        }
        image::save_buffer(path, &px, w, h, image::ExtendedColorType::Rgba8)?;
        tracing::info!("screenshot: {}", path.display());
        Ok(())
    }
}

/// a pass's eight params: its control inputs (linked or the node's own) in port order, defaults per kind; the chapter's
/// hold rides in the last slot (the renderer fills slots 2–6, `extra`)
fn params(ctl: &Controls, p: &Pass) -> [f32; 8] {
    let mut out = [0.0; 8];
    let names: &[&str] = match p.kind {
        Kind::TestPattern => &["speed", "zoom", "hue", "pulse"],
        Kind::Shader => &["a", "b", "c", "d"],
        Kind::Chapter => &["zoom", "pulse"],
        Kind::Overlay => &["opacity", "pulse", "shake"],
        Kind::Outlet => &["opacity"],
        _ => &[],
    };
    let input = |name: &str| ctl.input(p.node, name, crate::params::default(p.kind, name));
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
    queue.write_texture(
        wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        rgba,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: None },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    Image { view: tex.create_view(&Default::default()), aspect: w as f32 / h.max(1) as f32 }
}

/// a shader node's body: `effect` = a built-in name, or `wgsl` = a file defining `fn effect(uv) -> vec4<f32>`
fn shader_body(node: &crate::graph::Node) -> Result<String> {
    if let Some(path) = node.str("wgsl") {
        return std::fs::read_to_string(path).with_context(|| format!("node '{}': reading {path}", node.id));
    }
    let name = node.str("effect").unwrap_or("pass");
    builtin(name).map(str::to_string).with_context(|| {
        format!("node '{}': unknown effect '{name}' (kaleido, rgb_shift, invert, pass, fireflies – or wgsl = a file)", node.id)
    })
}

fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    label: &str,
    body: &str,
    format: wgpu::TextureFormat,
    blend: bool,
) -> Result<wgpu::RenderPipeline> {
    device.push_error_scope(wgpu::ErrorFilter::Validation);
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(format!("{COMMON}\n{body}").into()),
    });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });
    let p = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&pl),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: blend.then_some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    });
    if let Some(e) = pollster::block_on(device.pop_error_scope()) {
        bail!("{e}");
    }
    Ok(p)
}

fn begin<'a>(enc: &'a mut wgpu::CommandEncoder, view: &'a wgpu::TextureView) -> wgpu::RenderPass<'a> {
    enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
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

/// #209: the stats bar's texture, `w` HUD pixels wide
fn bar_texture(device: &wgpu::Device, w: u32) -> wgpu::Texture {
    texture(device, w, hud::BAR_H, OFFSCREEN, wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST)
}

fn uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: std::mem::size_of::<Uniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

#[cfg(test)]
mod backend_tests {
    use super::*;

    #[test]
    fn linux_starts_on_vulkan_alone_and_falls_back_once() {
        assert_eq!(default_backends(true), wgpu::Backends::VULKAN);
        assert_eq!(default_backends(false), wgpu::Backends::all());
        assert_eq!(backends_fallback(wgpu::Backends::VULKAN), Some(wgpu::Backends::all()));
        assert_eq!(backends_fallback(wgpu::Backends::all()), None);
    }
}
