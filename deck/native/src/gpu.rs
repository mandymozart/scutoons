//! A window's GPU (#443): the wgpu instance, adapter, device, queue and surface a deck-native window draws on, out of
//! render.rs so the native editor (#444) draws the stage on the same device beside its own panels. The deck's rules stay
//! as they were in `Renderer::new`: #257 the backends the first instance gets (`WGPU_BACKEND`, else Vulkan alone on Linux –
//! a GL backend in the instance opens its own EGL display on the Wayland connection and segfaults when the compositor
//! dies), GL only after Vulkan found nothing, a software adapter (llvmpipe / lavapipe / WARP) as the last fallback; a
//! non-sRGB surface format (the shaders write display values); `can_capture` when the surface's texture may be copied
//! (`--screenshot`); vsync (`AutoVsync`), two frames of latency.
use anyhow::{bail, Context, Result};
use std::sync::Arc;
use winit::window::Window;

pub struct Gpu {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub adapter: wgpu::AdapterInfo,
    /// a screenshot copies the surface's texture – where the surface allows it
    pub can_capture: bool,
}

/// #257: the backends the first instance gets – `WGPU_BACKEND` if set, else Vulkan alone on Linux (all elsewhere)
fn backends_first() -> wgpu::Backends {
    wgpu::Backends::from_env().unwrap_or_else(|| default_backends(cfg!(target_os = "linux")))
}

pub(crate) fn default_backends(linux: bool) -> wgpu::Backends {
    if linux { wgpu::Backends::VULKAN } else { wgpu::Backends::all() }
}

/// where to go when `tried` had no adapter: GL, only after Vulkan alone found nothing (None = nothing left)
pub(crate) fn backends_fallback(tried: wgpu::Backends) -> Option<wgpu::Backends> {
    (tried == wgpu::Backends::VULKAN).then_some(wgpu::Backends::all())
}

impl Gpu {
    /// the device and the surface for `window`, configured at its size
    pub fn new(window: Arc<Window>) -> Result<Gpu> {
        Gpu::with_features(window, wgpu::Features::empty())
    }

    /// `new`, with these features where the adapter has them (the editor's GPU timing)
    pub fn with_features(window: Arc<Window>, wanted: wgpu::Features) -> Result<Gpu> {
        let mut backends = backends_first();
        let (surface, adapter) = loop {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { backends, ..wgpu::InstanceDescriptor::new_without_display_handle_from_env() });
            let surface = instance.create_surface(window.clone()).context("creating the window surface")?;
            let ask = |fallback| {
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    force_fallback_adapter: fallback,
                    compatible_surface: Some(&surface),
                    apply_limit_buckets: false,
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
        tracing::info!("render: {} ({:?}, {:?}, driver {})", info.name, info.backend, info.device_type, info.driver_info);
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("deck-native"),
            required_features: adapter.features() & wanted,
            required_limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
        }))?;
        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let Some(&first) = caps.formats.first() else { bail!("the surface offers no format") };
        // shaders write display values: a non-sRGB surface keeps them as written
        let format = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(first);
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
            // wgpu 30: the surface's colour space; Auto = what wgpu always chose (sRGB for our 8-bit formats)
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&device, &config);
        Ok(Gpu { surface, device, queue, config, adapter: info, can_capture })
    }

    /// a software adapter (the fallback): the deck renders at half size on it
    pub fn software(&self) -> bool {
        self.adapter.device_type == wgpu::DeviceType::Cpu
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// the surface's size in pixels
    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// the surface at the window's new size; false for an empty size (a minimised window), which changes nothing
    pub fn resize(&mut self, w: u32, h: u32) -> bool {
        if w == 0 || h == 0 {
            return false;
        }
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        true
    }

    /// the frame to draw on; None when the surface was outdated or lost (configured again – draw next time) or this
    /// frame is skipped; an error when the GPU is out of memory
    pub fn acquire(&mut self) -> Result<Option<wgpu::SurfaceTexture>> {
        // wgpu 30: the answer is an enum without an out-of-memory case (the device's errors report that)
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => Ok(Some(f)),
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                Ok(None)
            }
            other => {
                tracing::debug!("render: skipping a frame ({other:?})");
                Ok(None)
            }
        }
    }
}
