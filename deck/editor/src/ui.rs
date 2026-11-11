//! The kit's adapter in the editor (#456, #485 slice 2): a frame's draw list is a layer of deck-kit's egui screen (base
//! `screen.rs`: egui's text in the deck's mono face, its shapes, the input stays the editor's own), drawn by egui-wgpu in
//! one pass over the cleared frame. The stage is a texture of the renderer's, drawn under the list; Feather's icons and
//! the creator's logo are rastered once into the screen's pictures. The editor's own GPU pass (shaders/ui.wgsl, a glyph
//! atlas of its own) is gone: the editor and the decks draw their UI through the same base.
use anyhow::Result;
use deck_kit::base::draw::DrawList;
use deck_kit::base::screen::{Layer, Pictures, Screen, Face};
use deck_kit::egui::epaint::{ClippedPrimitive, Color32, Mesh, Primitive, TextureId};
use deck_kit::egui::{pos2, Rect};
use egui_wgpu::{Renderer, RendererOptions, ScreenDescriptor};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

/// an icon's side in the pictures (pixels)
const ICON: u32 = 36;

/// Feather's icons (white, their coverage in alpha) and the logo's frames (their own colours) in one image.
fn pictures() -> Pictures {
    let logo = crate::logo::frames();
    let side = crate::logo::SIDE;
    let w = 10 * side;
    let icon_cols = w / ICON;
    let icons_h = (deck_kit::editor::icons::ICONS.len() as u32).div_ceil(icon_cols) * ICON;
    let logo_cols = w / side;
    let h = icons_h + (crate::logo::FRAMES as u32).div_ceil(logo_cols) * side;
    let mut pixels = vec![255u8; (w * h * 4) as usize];
    pixels.chunks_exact_mut(4).for_each(|p| p[3] = 0);
    let (wf, hf) = (w as f32, h as f32);
    let inset = |x: u32, y: u32, s: u32| Rect::from_min_max(pos2((x as f32 + 0.5) / wf, (y as f32 + 0.5) / hf), pos2(((x + s) as f32 - 0.5) / wf, ((y + s) as f32 - 0.5) / hf));
    // the icons: Feather's stroke 2.5 and square caps
    let mut icons = HashMap::new();
    for (i, (name, body)) in deck_kit::editor::icons::ICONS.iter().enumerate() {
        let (x0, y0) = (i as u32 % icon_cols * ICON, i as u32 / icon_cols * ICON);
        let svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"#fff\" stroke-width=\"2.5\" stroke-linecap=\"square\" stroke-linejoin=\"miter\">{body}</svg>");
        let scale = (ICON - 4) as f32 / 24.0;
        let mut map = resvg::tiny_skia::Pixmap::new(ICON, ICON).expect("an icon's pixmap");
        if let Ok(tree) = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()) {
            resvg::render(&tree, resvg::tiny_skia::Transform::from_translate(2.0, 2.0).pre_scale(scale, scale), &mut map.as_mut());
        }
        for (k, p) in map.pixels().iter().enumerate() {
            let (px, py) = (x0 + k as u32 % ICON, y0 + k as u32 / ICON);
            pixels[((py * w + px) * 4 + 3) as usize] = p.alpha();
        }
        icons.insert(*name, inset(x0, y0, ICON));
    }
    // the logo's frames
    let mut frames = Vec::with_capacity(logo.len());
    for (i, f) in logo.iter().enumerate() {
        let (x0, y0) = (i as u32 % logo_cols * side, icons_h + i as u32 / logo_cols * side);
        for (k, p) in f.chunks_exact(4).enumerate() {
            let (px, py) = (x0 + k as u32 % side, y0 + k as u32 / side);
            pixels[((py * w + px) * 4) as usize..((py * w + px) * 4 + 4) as usize].copy_from_slice(p);
        }
        frames.push(inset(x0, y0, side));
    }
    Pictures { size: [w as usize, h as usize], rgba: pixels, icons, frames }
}

/// The pass: the kit's screen, egui-wgpu's renderer, the stage's texture, this frame's list.
pub struct Ui {
    screen: Screen,
    renderer: Renderer,
    /// the stage's texture in the renderer, and where it goes this frame (logical pixels)
    stage_id: Option<TextureId>,
    stage_rect: Option<[f32; 4]>,
    /// what the kit draws this frame, and the list that takes turns with it
    list: DrawList,
    spare: Option<DrawList>,
    /// what egui made last
    jobs: Vec<ClippedPrimitive>,
    free: Vec<TextureId>,
}

impl Ui {
    pub fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat, _font_px: f32) -> Result<Ui> {
        // #485 slice 3: the editor keeps egui's text (the decks set the face on the kit's own atlas)
        let mut screen = Screen::with_egui_text(deck_outlet::text::mono_ttf(), 1);
        screen.set_pictures(pictures());
        Ok(Ui {
            screen,
            // no MSAA, no depth, no dithering (the editor's own pass had none)
            renderer: Renderer::new(device, format, RendererOptions { msaa_samples: 1, depth_stencil_format: None, dithering: false, predictable_texture_filtering: false }),
            stage_id: None,
            stage_rect: None,
            list: DrawList::new(deck_outlet::text::mono_per_px()),
            spare: None,
            jobs: Vec::new(),
            free: Vec::new(),
        })
    }

    /// the stage's texture (made again when the stage's texture changes)
    pub fn set_stage(&mut self, device: &wgpu::Device, view: &wgpu::TextureView) {
        match self.stage_id {
            Some(id) => self.renderer.update_egui_texture_from_wgpu_texture(device, view, wgpu::FilterMode::Linear, id),
            None => self.stage_id = Some(self.renderer.register_native_texture(device, view, wgpu::FilterMode::Linear)),
        }
    }

    pub fn begin(&mut self) {
        self.list.clear();
        self.stage_rect = None;
    }

    /// the stage into this rectangle
    pub fn stage(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.stage_rect = Some([x, y, w, h]);
    }

    /// draw this frame over `view` (cleared to `clear`); `logical` = the window's size in the pixels the list was laid
    /// out in (the surface may be larger on a HiDPI screen: the layer's scale)
    pub fn flush(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, logical: (f32, f32), clear: [f32; 4]) {
        let size = view.texture().size();
        let scale = (size.width as f32 / logical.0.max(1.0)).max(f32::MIN_POSITIVE);
        // the list goes to the screen and comes back after the frame (two lists take turns: their memory is kept)
        let spare = self.spare.take().unwrap_or_else(|| DrawList::new(deck_outlet::text::mono_per_px()));
        let list = std::mem::replace(&mut self.list, spare);
        self.screen.set(0, Some(Layer { list, scale, grid: scale, face: Face::Mono }));
        let painted = self.screen.frame((size.width, size.height));
        if let Some(layer) = self.screen.take(0) {
            self.spare = Some(std::mem::replace(&mut self.list, layer.list));
        }
        if let Some(painted) = painted {
            for id in self.free.drain(..) {
                self.renderer.free_texture(&id);
            }
            for (id, delta) in &painted.set {
                self.renderer.update_texture(device, queue, *id, delta);
            }
            self.free = painted.free;
            self.jobs = painted.primitives;
        }
        // the stage under everything, its own texture
        let full = Rect::from_min_size(pos2(0.0, 0.0), deck_kit::egui::vec2(size.width as f32, size.height as f32));
        let stage = match (self.stage_rect, self.stage_id) {
            (Some([x, y, w, h]), Some(id)) => {
                let mut mesh = Mesh::with_texture(id);
                mesh.add_rect_with_uv(Rect::from_min_size(pos2(x * scale, y * scale), deck_kit::egui::vec2(w * scale, h * scale)), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                Some(ClippedPrimitive { clip_rect: full, primitive: Primitive::Mesh(mesh) })
            }
            _ => None,
        };
        let jobs: Vec<ClippedPrimitive> = stage.into_iter().chain(self.jobs.iter().cloned()).collect();
        let desc = ScreenDescriptor { size_in_pixels: [size.width, size.height], pixels_per_point: 1.0 };
        // no paint callbacks: nothing comes back to submit
        let _ = self.renderer.update_buffers(device, queue, enc, &jobs, &desc);
        let mut rp = enc
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                multiview_mask: None,
                label: Some("ui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: f64::from(clear[0]), g: f64::from(clear[1]), b: f64::from(clear[2]), a: 1.0 }), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            })
            .forget_lifetime();
        self.renderer.render(&mut rp, &jobs, &desc);
    }
}

impl Deref for Ui {
    type Target = DrawList;
    fn deref(&self) -> &DrawList {
        &self.list
    }
}

impl DerefMut for Ui {
    fn deref_mut(&mut self) -> &mut DrawList {
        &mut self.list
    }
}
