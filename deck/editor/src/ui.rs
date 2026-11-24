//! The kit's adapter in the editor (#456, #485 slice 2): a frame's draw list is a layer of deck-kit's egui screen (base
//! `screen.rs`: egui's text in the deck's mono face, its shapes, the input stays the editor's own), drawn by egui-wgpu in
//! one pass over the cleared frame. The stage is a texture of the renderer's, drawn under the list; Feather's icons and
//! the creator's logo are rastered once into the screen's pictures. The editor's own GPU pass (shaders/ui.wgsl, a glyph
//! atlas of its own) is gone: the editor and the decks draw their UI through the same base.
use anyhow::Result;
use deck_kit::base::draw::DrawList;
use deck_kit::base::fonts::{Faces, Widths};
use deck_kit::base::screen::{Layer, Pictures, Screen, Face};
use deck_kit::egui::epaint::{ClippedPrimitive, Color32, Mesh, Primitive, TextureId};
use deck_kit::egui::{pos2, Rect};
use egui_wgpu::{Renderer, RendererOptions, ScreenDescriptor};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// an icon's side in the pictures (pixels)
const ICON: u32 = 48;
/// the pictures' width once previews join them
const THUMBS_W: usize = 2048;

/// Feather's icons (white, their coverage in alpha) and the logo's frames (their own colours) in one image.
fn pictures() -> Pictures {
    let logo = crate::logo::frames();
    let side = crate::logo::SIDE;
    let w = 10 * side;
    let icons_h = deck_kit::editor::icons::raster_height(w, ICON);
    let logo_cols = w / side;
    let h = icons_h + (crate::logo::FRAMES as u32).div_ceil(logo_cols) * side;
    let mut pixels = vec![255u8; (w * h * 4) as usize];
    pixels.chunks_exact_mut(4).for_each(|p| p[3] = 0);
    let (wf, hf) = (w as f32, h as f32);
    let inset = |x: u32, y: u32, s: u32| Rect::from_min_max(pos2((x as f32 + 0.5) / wf, (y as f32 + 0.5) / hf), pos2(((x + s) as f32 - 0.5) / wf, ((y + s) as f32 - 0.5) / hf));
    // the icons: the kit's raster of Feather's
    let icons = deck_kit::editor::icons::raster(&mut pixels, (w, h), ICON);
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
    Pictures { size: [w as usize, h as usize], rgba: pixels, icons, frames, thumbs: HashMap::new() }
}

/// the base pictures with the previews' pictures under them, a row of them at a time
fn with_thumbs(base: &Pictures, thumbs: &[(u64, image::RgbaImage)]) -> Pictures {
    let [bw, bh] = base.size;
    let (tw, th) = (crate::library::STILL.0 as usize, crate::library::STILL.1 as usize);
    let w = bw.max(THUMBS_W);
    let cols = (w / tw).max(1);
    let rows = thumbs.len().div_ceil(cols);
    let h = bh + rows * th;
    let mut rgba = vec![0u8; w * h * 4];
    for y in 0..bh {
        rgba[y * w * 4..(y * w + bw) * 4].copy_from_slice(&base.rgba[y * bw * 4..(y + 1) * bw * 4]);
    }
    let (wf, hf) = (w as f32, h as f32);
    let (sx, sy) = (bw as f32 / wf, bh as f32 / hf);
    let rescale = |r: &Rect| Rect::from_min_max(pos2(r.min.x * sx, r.min.y * sy), pos2(r.max.x * sx, r.max.y * sy));
    let mut out = Pictures { size: [w, h], rgba: Vec::new(), icons: base.icons.iter().map(|(k, r)| (*k, rescale(r))).collect(), frames: base.frames.iter().map(rescale).collect(), thumbs: HashMap::new() };
    for (i, (id, img)) in thumbs.iter().enumerate() {
        let (x0, y0) = (i % cols * tw, bh + i / cols * th);
        for (y, line) in img.rows().take(th).enumerate() {
            for (x, p) in line.take(tw).enumerate() {
                let at = ((y0 + y) * w + x0 + x) * 4;
                rgba[at..at + 4].copy_from_slice(&p.0);
            }
        }
        out.thumbs.insert(*id, Rect::from_min_max(pos2((x0 as f32 + 0.5) / wf, (y0 as f32 + 0.5) / hf), pos2(((x0 + tw) as f32 - 0.5) / wf, ((y0 + th) as f32 - 0.5) / hf)));
    }
    out.rgba = rgba;
    out
}

/// The pass: the kit's screen, egui-wgpu's renderer, the stage's texture, this frame's list.
pub struct Ui {
    screen: Screen,
    renderer: Renderer,
    /// the stage's texture in the renderer, and where it goes this frame (logical pixels)
    stage_id: Option<TextureId>,
    stage_rect: Option<[f32; 4]>,
    /// the video cards' live frames (posters.rs) in the renderer by node id, and where each goes this frame
    cards: HashMap<String, TextureId>,
    card_rects: Vec<([f32; 4], TextureId)>,
    /// what the kit draws this frame, and the list that takes turns with it
    list: DrawList,
    spare: Option<DrawList>,
    /// what the lists lay text out with: the faces the screen sets
    widths: Arc<Widths>,
    /// what egui made last
    jobs: Vec<ClippedPrimitive>,
    free: Vec<TextureId>,
    /// the icons and the logo, before any preview joined them
    base: Pictures,
    /// the overlay level's still (deck-kit `still`): its texture, and whether it is drawn this frame in the stage's place
    backdrop: Option<(wgpu::Texture, TextureId)>,
    backdrop_on: bool,
}

impl Ui {
    pub fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat, _font_px: f32) -> Result<Ui> {
        // the editor keeps egui's text (the decks set the face on the kit's own atlas); ABC Areal when installed
        let faces = Faces::installed(deck_outlet::text::mono_ttf());
        tracing::info!(from_folder = ?faces.from_folder, folder = ?deck_kit::base::fonts::folder(), "fonts");
        let widths = Arc::new(faces.widths());
        let mut screen = Screen::with_egui_text(faces, 1);
        let base = pictures();
        screen.set_pictures(base.clone());
        Ok(Ui {
            screen,
            // no MSAA, no depth, no dithering (the editor's own pass had none)
            renderer: Renderer::new(device, format, RendererOptions { msaa_samples: 1, depth_stencil_format: None, dithering: false, predictable_texture_filtering: false }),
            stage_id: None,
            stage_rect: None,
            cards: HashMap::new(),
            card_rects: Vec::new(),
            list: DrawList::with_widths(widths.clone()),
            spare: None,
            widths,
            jobs: Vec::new(),
            free: Vec::new(),
            base,
            backdrop: None,
            backdrop_on: false,
        })
    }

    /// the overlay's still, from now on until it is let go
    pub fn set_backdrop(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, still: &deck_kit::base::still::Still) {
        self.drop_backdrop();
        if still.w == 0 {
            return;
        }
        let size = wgpu::Extent3d { width: still.w, height: still.h, depth_or_array_layers: 1 };
        let tex = device.create_texture(&wgpu::TextureDescriptor { label: Some("overlay still"), size, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8UnormSrgb, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[] });
        queue.write_texture(tex.as_image_copy(), &still.rgba, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * still.w), rows_per_image: Some(still.h) }, size);
        let id = self.renderer.register_native_texture(device, &tex.create_view(&Default::default()), wgpu::FilterMode::Linear);
        self.backdrop = Some((tex, id));
    }

    pub fn has_backdrop(&self) -> bool {
        self.backdrop.is_some()
    }

    /// the still let go (the overlay closed)
    pub fn drop_backdrop(&mut self) {
        if let Some((_, id)) = self.backdrop.take() {
            self.renderer.free_texture(&id);
        }
    }

    /// this frame shows the still over the whole window instead of the stage and the cards
    pub fn backdrop(&mut self) {
        self.backdrop_on = self.backdrop.is_some();
    }

    /// the previews' pictures, from the next frame on
    pub fn set_thumbs(&mut self, thumbs: &[(u64, image::RgbaImage)]) {
        self.screen.set_pictures(with_thumbs(&self.base, thumbs));
    }

    /// the stage's texture (made again when the stage's texture changes)
    pub fn set_stage(&mut self, device: &wgpu::Device, view: &wgpu::TextureView) {
        match self.stage_id {
            Some(id) => self.renderer.update_egui_texture_from_wgpu_texture(device, view, wgpu::FilterMode::Linear, id),
            None => self.stage_id = Some(self.renderer.register_native_texture(device, view, wgpu::FilterMode::Linear)),
        }
    }

    /// a video card's frame texture (made again when `fresh`: its texture changed)
    pub fn set_card(&mut self, device: &wgpu::Device, id: &str, view: &wgpu::TextureView, fresh: bool) {
        match self.cards.get(id) {
            Some(&t) if fresh => self.renderer.update_egui_texture_from_wgpu_texture(device, view, wgpu::FilterMode::Linear, t),
            Some(_) => {}
            None => {
                let t = self.renderer.register_native_texture(device, view, wgpu::FilterMode::Linear);
                self.cards.insert(id.to_string(), t);
            }
        }
    }

    /// the cards' textures of nodes not in `keep` freed
    pub fn keep_cards(&mut self, keep: &[&str]) {
        let renderer = &mut self.renderer;
        self.cards.retain(|id, t| keep.contains(&id.as_str()) || {
            renderer.free_texture(t);
            false
        });
    }

    /// card `id`'s frame into this rectangle, under the list (its lines stay over it)
    pub fn card(&mut self, id: &str, x: f32, y: f32, w: f32, h: f32) {
        if let Some(&t) = self.cards.get(id) {
            self.card_rects.push(([x, y, w, h], t));
        }
    }

    pub fn begin(&mut self) {
        self.list.clear();
        self.stage_rect = None;
        self.card_rects.clear();
        self.backdrop_on = false;
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
        let spare = self.spare.take().unwrap_or_else(|| DrawList::with_widths(self.widths.clone()));
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
        let cards = self.card_rects.iter().map(|&([x, y, w, h], id)| {
            let mut mesh = Mesh::with_texture(id);
            mesh.add_rect_with_uv(Rect::from_min_size(pos2(x * scale, y * scale), deck_kit::egui::vec2(w * scale, h * scale)), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            ClippedPrimitive { clip_rect: full, primitive: Primitive::Mesh(mesh) }
        });
        let backdrop = self.backdrop.as_ref().filter(|_| self.backdrop_on).map(|(_, id)| {
            let mut mesh = Mesh::with_texture(*id);
            mesh.add_rect_with_uv(full, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
            ClippedPrimitive { clip_rect: full, primitive: Primitive::Mesh(mesh) }
        });
        let under: Vec<ClippedPrimitive> = match backdrop {
            Some(b) => vec![b],
            None => stage.into_iter().chain(cards).collect(),
        };
        let jobs: Vec<ClippedPrimitive> = under.into_iter().chain(self.jobs.iter().cloned()).collect();
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
