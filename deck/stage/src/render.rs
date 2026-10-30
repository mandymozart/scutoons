//! The renderer (CYBERDECKS.md §8 "Render"): wgpu (Vulkan on the deck; DX12 / Vulkan / Metal / GL elsewhere), one
//! fullscreen pass per video node in graph order, each into its own texture; the screen node blits its input to the
//! surface (vsync, Fifo), the HUD goes on top. A software adapter (llvmpipe / lavapipe / WARP) is the chosen fallback
//! when no hardware adapter answers, at half the render size. A shader that does not compile falls back to `pass`.
//! S3: a pass reads up to three textures (its `in`, a chapter's current / previous slice, an overlay's `top`); the
//! chapter nodes' slices arrive from their loaders (chapter.rs) and are uploaded when the node cuts; the splash (the
//! logo, splash.rs) goes over everything – its first frame is shown before the scene's shaders are compiled.
use crate::chapter::{ChapterFeed, Prepared};
use crate::control::Controls;
use crate::graph::{Graph, Kind};
use crate::hud::{self, Hud};
use crate::tempo::Quantize;
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
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
        "chapter" => include_str!("shaders/chapter.wgsl"),
        "overlay" => include_str!("shaders/overlay.wgsl"),
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
    pipeline: wgpu::RenderPipeline,
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
    render_scale: f32,
    slides: Vec<Slides>,
    splash: Pass,
    logo: Image,
}

impl Renderer {
    pub fn new(window: Arc<Window>, graph: &Graph, render_scale: f32, feeds: Vec<ChapterFeed>) -> Result<Renderer> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::from_env_or_default());
        let surface = instance.create_surface(window.clone()).context("creating the window surface")?;
        let ask = |fallback| {
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: fallback,
                compatible_surface: Some(&surface),
            }))
        };
        let adapter = match ask(false) {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!("render: no hardware adapter ({e}) – the software fallback");
                ask(true).context("no GPU adapter at all, not even a software one")?
            }
        };
        let info = adapter.get_info();
        let software = info.device_type == wgpu::DeviceType::Cpu;
        tracing::info!("render: {} ({:?}, {:?}, driver {})", info.name, info.backend, info.device_type, info.driver_info);
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("deck-stage"),
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
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
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
        // the splash first – one frame of it at once, the scene's shaders compile behind it
        let (lw, lh, px) = crate::splash::logo((config.width / 2).clamp(64, 2048))?;
        let logo = upload(&device, &queue, lw, lh, &px);
        let overlay_pass = |label: &str, body: &str| -> Result<Pass> {
            Ok(Pass {
                node: usize::MAX,
                kind: Kind::Screen,
                pipeline: pipeline(&device, &layout, label, body, format, true)?,
                ubuf: uniform_buffer(&device),
                inputs: [None; 3],
                target: None,
                bind: None,
            })
        };
        let splash = overlay_pass("splash", builtin("splash").unwrap())?;
        let hud_pass = overlay_pass("hud", builtin("pass").unwrap())?;

        let mut r = Renderer {
            hud_pass,
            splash,
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
            slides: Vec::new(),
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
                Kind::Chapter => [link("in"), Some(Src::Current(i)), Some(Src::Previous(i))],
                Kind::Overlay => [link("in"), link("top"), None],
                _ => [link("in"), None, None],
            };
            let pass = Pass { node: i, kind: node.kind, pipeline, ubuf: uniform_buffer(&r.device), inputs, target: None, bind: None };
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

    fn render_size(&self) -> (u32, u32) {
        let s = self.render_scale;
        (((self.config.width as f32 * s) as u32).max(1), ((self.config.height as f32 * s) as u32).max(1))
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.rebuild();
    }

    /// the offscreen targets at the render size, then every bind group (an input's view changes with its target)
    fn rebuild(&mut self) {
        let (w, h) = self.render_size();
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        for p in &mut self.passes {
            p.target = Some(Target { view: texture(&self.device, w, h, OFFSCREEN, usage).create_view(&Default::default()) });
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
        let splash_bind = bind(&self.splash, [&self.logo.view, &self.dummy, &self.dummy], &self.linear);
        for (p, b) in self.passes.iter_mut().zip(binds) {
            p.bind = Some(b);
        }
        if let Some(s) = self.screen.as_mut() {
            s.bind = screen_bind;
        }
        self.hud_pass.bind = Some(hud_bind);
        self.splash.bind = Some(splash_bind);
    }

    /// one frame of the splash alone (the window's first, before the scene's passes exist)
    fn show_splash(&mut self) {
        let Ok(frame) = self.surface.get_current_texture() else { return };
        let res = [self.config.width as f32, self.config.height as f32];
        let u = Uniforms { time: 0.0, dt: 0.0, res, p: [1.0, self.logo.aspect, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0] };
        self.queue.write_buffer(&self.splash.ubuf, 0, bytemuck::bytes_of(&u));
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let view = frame.texture.create_view(&Default::default());
            let mut rp = begin(&mut enc, &view);
            rp.set_pipeline(&self.splash.pipeline);
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
    pub fn frame(&mut self, graph: &Graph, ctl: &Controls, time: f32, dt: f32, hud: Option<&Hud>, splash: (f32, bool)) -> Result<f32> {
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
        for p in self.passes.iter().chain(self.screen.as_ref()) {
            let mut params = params(ctl, p);
            self.extra(p, time, &mut params);
            let u = Uniforms { time, dt, res: [w as f32, h as f32], p: params };
            self.queue.write_buffer(&p.ubuf, 0, bytemuck::bytes_of(&u));
        }
        let (sw, sh) = (self.config.width as f32, self.config.height as f32);
        if splash.0 > 0.0 {
            let p = [splash.0, self.logo.aspect, if splash.1 { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0, 0.0, 0.0];
            let u = Uniforms { time, dt, res: [sw, sh], p };
            self.queue.write_buffer(&self.splash.ubuf, 0, bytemuck::bytes_of(&u));
        }
        let mut enc = self.device.create_command_encoder(&Default::default());
        for p in &self.passes {
            let mut rp = begin(&mut enc, &p.target.as_ref().unwrap().view);
            rp.set_pipeline(&p.pipeline);
            rp.set_bind_group(0, p.bind.as_ref(), &[]);
            rp.draw(0..3, 0..1);
        }
        let view = frame.texture.create_view(&Default::default());
        {
            let mut rp = begin(&mut enc, &view);
            if let Some(s) = &self.screen {
                rp.set_pipeline(&s.pipeline);
                rp.set_bind_group(0, s.bind.as_ref(), &[]);
                rp.draw(0..3, 0..1);
            }
            if let Some(hud) = hud.filter(|h| h.visible) {
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
                let s = (sh / 540.0).round().max(1.0);
                let (hw, hh) = ((hud::W as f32 * s).min(sw), (hud::H as f32 * s).min(sh));
                let m = (16.0 * s).min(sw - hw).max(0.0);
                rp.set_viewport(m, m.min(sh - hh).max(0.0), hw, hh, 0.0, 1.0);
                rp.set_pipeline(&self.hud_pass.pipeline);
                rp.set_bind_group(0, self.hud_pass.bind.as_ref(), &[]);
                rp.draw(0..3, 0..1);
            }
            if splash.0 > 0.0 {
                rp.set_viewport(0.0, 0.0, sw, sh, 0.0, 1.0);
                rp.set_pipeline(&self.splash.pipeline);
                rp.set_bind_group(0, self.splash.bind.as_ref(), &[]);
                rp.draw(0..3, 0..1);
            }
        }
        self.queue.submit([enc.finish()]);
        let ms = work.elapsed().as_secs_f32() * 1000.0;
        frame.present();
        Ok(ms)
    }
}

/// a pass's eight params: its control inputs (linked or the node's own) in port order, defaults per kind; the chapter's
/// hold rides in the last slot (the renderer fills slots 2–6, `extra`)
fn params(ctl: &Controls, p: &Pass) -> [f32; 8] {
    let mut out = [0.0; 8];
    let names: &[(&str, f32)] = match p.kind {
        Kind::TestPattern => &[("speed", 1.0), ("zoom", 1.0), ("hue", 0.0), ("pulse", 0.0)],
        Kind::Shader => &[("a", 0.5), ("b", 0.5), ("c", 0.5), ("d", 0.0)],
        Kind::Chapter => &[("zoom", 1.0), ("pulse", 0.0)],
        Kind::Overlay => &[("opacity", 1.0), ("pulse", 0.0), ("shake", 0.0)],
        _ => &[],
    };
    for (i, (name, default)) in names.iter().enumerate() {
        out[i] = ctl.input(p.node, name, *default);
    }
    if p.kind == Kind::Chapter {
        out[7] = ctl.input(p.node, "hold", 6.0);
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
        format!("node '{}': unknown effect '{name}' (kaleido, rgb_shift, invert, pass – or wgsl = a file)", node.id)
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

fn uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: std::mem::size_of::<Uniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
