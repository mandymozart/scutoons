//! The native window around the renderer every deck draws with (deck-space `stage`, #463): the window's device and surface
//! (gpu.rs, vsync, Fifo) and `capture` – `--screenshot` saves the last frame of a check run as a PNG file. A software
//! adapter (llvmpipe / lavapipe / WARP) is the chosen fallback when no hardware adapter answers, at half the render size.
use crate::chapter::ChapterFeed;
use crate::control::Controls;
use crate::graph::Graph;
use crate::gpu::Gpu;
use crate::hud::Hud;
use anyhow::{Context, Result};
use deck_outlet::frame::Frame;
pub use deck_space::space::Space;
use deck_space::quality::Rung;
use deck_space::stage::{Over, Stage, Tick};
use std::path::PathBuf;
use std::sync::Arc;
use winit::window::Window;

pub struct Renderer {
    /// the window's device and surface (gpu.rs, #443)
    pub gpu: Gpu,
    stage: Stage,
    /// save the next frame here (PNG), then forget it
    pub capture: Option<PathBuf>,
}

impl Renderer {
    /// `veil` = the splash's first frame is black without the logo (#250: a scene switch fades through black)
    pub fn new(window: Arc<Window>, graph: &Graph, render_scale: f32, feeds: Vec<ChapterFeed>, veil: bool) -> Result<Renderer> {
        // the device and the surface (gpu.rs, #443): the backends' fallbacks, the format, the size
        let gpu = Gpu::new(window)?;
        let format = gpu.format();
        let scale = if gpu.software() { render_scale.min(0.5) } else { render_scale };
        let mut stage = pollster::block_on(Stage::new(&gpu.device, &gpu.queue, format, gpu.size(), scale, render_scale, veil))?;
        // the splash first – one frame of it at once, the scene's shaders compile behind it
        if let Ok(frame) = gpu.surface.get_current_texture() {
            stage.show_splash(&gpu.device, &gpu.queue, &frame.texture.create_view(&Default::default()));
            frame.present();
        }
        pollster::block_on(stage.load(&gpu.device, format, graph, feeds))?;
        Ok(Renderer { gpu, stage, capture: None })
    }

    /// the splash is a black veil without the logo (#250)
    pub fn set_veil(&mut self, veil: bool) {
        self.stage.veil_only = veil;
    }

    /// the composer's channel for the scene's outlet node (live/, #186); without an outlet node it is dropped
    pub fn set_outlet(&mut self, rx: std::sync::mpsc::Receiver<Frame>) {
        self.stage.set_outlet(rx);
    }

    /// the outlet's crossfade at `time` (the HUD: still fading)
    pub fn outlet_fading(&self, time: f32) -> bool {
        self.stage.outlet_fading(time)
    }

    /// every chapter node shows a slice (the splash may go)
    pub fn slides_ready(&self) -> bool {
        self.stage.slides_ready()
    }

    /// per chapter node: "<slice> / <count>" or the loader's status
    pub fn slides_status(&self) -> Vec<String> {
        self.stage.slides_status()
    }

    /// The dev mode's hot reload: recompile the shader nodes `nodes` from their WGSL files. A file that doesn't read or
    /// compile keeps the running pipeline; the errors come back for the HUD.
    pub fn reload_shaders(&mut self, graph: &Graph, nodes: &[usize]) -> Vec<String> {
        pollster::block_on(self.stage.reload_shaders(&self.gpu.device, graph, nodes))
    }

    /// The dev mode's hot reload of the post stack (#204): the passes `effects` from their WGSL files (deck-space
    /// shaders/post/, with the prelude common.wgsl from there too). A pass that doesn't read or compile keeps the
    /// running one; the errors come back for the HUD. Before the space's first frame there is nothing to reload.
    #[cfg(feature = "space")]
    pub fn reload_post(&mut self, effects: &[crate::space::post::Effect]) -> Vec<String> {
        use crate::space::post;
        let mut errors = Vec::new();
        let Some(gpu) = self.stage.space_gpu() else { return errors };
        let read = |file: &str| std::fs::read_to_string(post::source_path(file)).with_context(|| format!("post: reading {file}"));
        for &e in effects {
            let src = match read("common.wgsl").and_then(|c| Ok((c, read(e.file())?))) {
                Ok(s) => s,
                Err(err) => {
                    errors.push(format!("{err:#}"));
                    continue;
                }
            };
            self.gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
            let pipes = gpu.compile_post(&self.gpu.device, e, &src.0, &src.1);
            match pollster::block_on(self.gpu.device.pop_error_scope()) {
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
    /// or a height in pixels; the HUD stays native) and the space's lights and post passes.
    pub fn set_quality(&mut self, rung: Rung, scene_scale: f32) {
        self.stage.set_quality(&self.gpu.device, rung, scene_scale);
    }

    /// the render size × the screen's
    pub fn render_scale(&self) -> f32 {
        self.stage.render_scale()
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if self.gpu.resize(w, h) {
            self.stage.resize(&self.gpu.device, (w, h));
        }
    }

    /// #209: the stats bar's width in HUD pixels (what main rasters it at)
    pub fn bar_width(&self) -> u32 {
        self.stage.bar_width()
    }

    /// The deck's menu (#386, deck-ui `Menu::paint`): Some = its picture over the whole screen; None = gone. Called when
    /// it changes, not every frame.
    pub fn set_menu(&mut self, menu: Option<(u32, u32, &[u8])>) {
        self.stage.set_menu(&self.gpu.device, &self.gpu.queue, menu);
    }

    /// the window's size in pixels (what the menu is painted for)
    pub fn size(&self) -> (f32, f32) {
        (self.gpu.config.width as f32, self.gpu.config.height as f32)
    }

    /// The deck's UI for the person at it (#481, deck-show `face.rs`): Some = its pixels, a strip along the screen's
    /// bottom; None = gone. Called when it changes, not every frame.
    pub fn set_panel(&mut self, panel: Option<(u32, u32, &[u8])>) {
        self.stage.set_panel(&self.gpu.device, &self.gpu.queue, panel);
    }

    /// One frame; returns the CPU time spent after the surface handed out its texture (the vsync wait excluded), in ms.
    /// `splash` = the splash's alpha (0 = none) and whether its loading line runs.
    /// `space` = the scene's space node's state (its set's 3D arrives through it once).
    #[allow(clippy::too_many_arguments)]
    pub fn frame(&mut self, graph: &Graph, ctl: &Controls, time: f32, dt: f32, hud: Option<&Hud>, splash: (f32, bool), space: Option<&mut Space>) -> Result<f32> {
        let Some(frame) = self.gpu.acquire()? else { return Ok(0.0) };
        let work = std::time::Instant::now();
        let mut enc = self.gpu.device.create_command_encoder(&Default::default());
        let input = |n: usize, p: &str, d: f32| ctl.input(n, p, d);
        let view = frame.texture.create_view(&Default::default());
        self.stage.frame(&self.gpu.device, &self.gpu.queue, &mut enc, &view, &Tick { graph, input: &input, tempo: &ctl.tempo, time, dt }, &Over { hud, splash, label_centred: false, hud_at: None }, space);
        let shot = self.capture.take().filter(|_| self.gpu.can_capture).map(|path| {
            let (w, h) = (self.gpu.config.width, self.gpu.config.height);
            let row = (w * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
            let buf = self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
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
        self.gpu.queue.submit([enc.finish()]);
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
        self.gpu.device.poll(wgpu::PollType::Wait)?;
        let data = slice.get_mapped_range();
        let bgra = matches!(self.gpu.config.format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
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

#[cfg(test)]
mod backend_tests {
    use crate::gpu::{backends_fallback, default_backends};

    #[test]
    fn linux_starts_on_vulkan_alone_and_falls_back_once() {
        assert_eq!(default_backends(true), wgpu::Backends::VULKAN);
        assert_eq!(default_backends(false), wgpu::Backends::all());
        assert_eq!(backends_fallback(wgpu::Backends::VULKAN), Some(wgpu::Backends::all()));
        assert_eq!(backends_fallback(wgpu::Backends::all()), None);
    }
}
