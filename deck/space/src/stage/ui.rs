//! The deck's own UI on the stage (#485, RULES H8): the kit's screen on egui (deck-kit `base::screen`) with the deck's
//! menu (#386) and its panel (#481) as layers, drawn by egui-wgpu inside the stage's pass – over the splash, under the
//! build label – on the native deck and in the browser (WebGPU and WebGL2 alike: egui-wgpu's shader is WGSL, naga
//! turns it into GLSL as it does the stage's). It replaces the two CPU pictures (menu, panel) the stage uploaded before.
//! The layers change when their owners say (deck-ui `Menu::paint`, deck-show `Face::paint`); egui runs then, and on a
//! new size or input – between those the meshes it made are drawn again.
use deck_kit::base::input::Input;
use deck_kit::base::screen::{Layer, Screen};
use deck_kit::egui::epaint::{ClippedPrimitive, TextureId};
use deck_kit::live::{LAYERS, MENU, PANEL};
use egui_wgpu::{Renderer, RendererOptions, ScreenDescriptor};

pub(super) struct Ui {
    screen: Screen,
    renderer: Renderer,
    /// what egui made last, and its screen
    jobs: Vec<ClippedPrimitive>,
    size: [u32; 2],
    /// the textures egui gave up with the last frame: freed once the next one is made
    free: Vec<TextureId>,
}

impl Ui {
    /// `format`: the surface's (a non-sRGB one: egui-wgpu writes display values, as the stage's shaders do)
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Ui {
        Ui {
            screen: Screen::new(deck_outlet::text::mono_ttf(), LAYERS),
            // no MSAA, no depth, no dithering (the CPU pictures it replaced had none)
            renderer: Renderer::new(device, format, RendererOptions { msaa_samples: 1, depth_stencil_format: None, dithering: false, predictable_texture_filtering: false }),
            jobs: Vec::new(),
            size: [0, 0],
            free: Vec::new(),
        }
    }

    pub(super) fn set_menu(&mut self, menu: Option<Layer>) {
        self.screen.set(MENU, menu);
    }

    pub(super) fn set_panel(&mut self, panel: Option<Layer>) {
        self.screen.set(PANEL, panel);
    }

    /// one input for egui while a layer shows (a pointer moving over the scene alone runs no frame)
    pub(super) fn input(&mut self, i: Input) {
        if self.shows() {
            self.screen.input(i);
        }
    }

    /// whether a layer shows
    pub(super) fn shows(&self) -> bool {
        self.screen.layer(MENU).is_some() || self.screen.layer(PANEL).is_some()
    }

    /// Before the pass: a new frame of egui's when something changed – its textures to the GPU, its meshes into the
    /// renderer's buffers.
    pub(super) fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, size: (u32, u32)) {
        let Some(painted) = self.screen.frame(size) else { return };
        for id in self.free.drain(..) {
            self.renderer.free_texture(&id);
        }
        for (id, delta) in &painted.set {
            self.renderer.update_texture(device, queue, *id, delta);
        }
        self.free = painted.free;
        self.size = [painted.size.0, painted.size.1];
        self.jobs = painted.primitives;
        let desc = ScreenDescriptor { size_in_pixels: self.size, pixels_per_point: 1.0 };
        // no paint callbacks: nothing comes back to submit
        let _ = self.renderer.update_buffers(device, queue, enc, &self.jobs, &desc);
    }

    /// In the pass: the meshes over what is drawn; the scissor egui set is undone for what comes after.
    pub(super) fn draw(&self, rp: &mut wgpu::RenderPass<'static>) {
        if self.jobs.is_empty() || self.size[0] == 0 || self.size[1] == 0 {
            return;
        }
        let desc = ScreenDescriptor { size_in_pixels: self.size, pixels_per_point: 1.0 };
        self.renderer.render(rp, &self.jobs, &desc);
        rp.set_scissor_rect(0, 0, self.size[0], self.size[1]);
    }
}
