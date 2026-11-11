//! The kit on egui (#485; RULES H8, owner 2026-11-11 [ui-kit]: "internal rust UI library based on egui"): a screen's
//! layers of draw lists painted through egui – its `Context`, epaint's meshes and text, its fonts, the input fed from
//! the kit's `Input` – into meshes in device pixels an adapter draws (deck-space's stage, through egui-wgpu, in its
//! pass). egui's widgets and theme are not used: a layer is our components' draw list, painted shape by shape.
//!
//! One point is one device pixel; a layer brings its own scale (the deck's menu and its panel are painted at different
//! ones). The faces: the deck's mono face (an outline font egui lays out; the platform hands its bytes in) and the pixel
//! face (`pixel.rs`, a bitmap atlas egui samples nearest – egui takes no bitmap fonts). A frame is run only when a
//! layer, the screen's size or the input changed; in between the adapter draws what it has.
use crate::base::draw::{DrawList, Prim};
use crate::base::input::Input;
use egui::epaint::{ClippedPrimitive, ImageDelta, Color32, CornerRadius, Mesh, Shape, Stroke, StrokeKind, TextureId};
use egui::{pos2, vec2, Context, Event, FontData, FontDefinitions, FontFamily, FontId, Key, LayerId, Modifiers, PointerButton, Pos2, RawInput, Rect, ViewportId};
use std::collections::HashMap;
use std::sync::Arc;

/// a line's height per pixel of font size: a text sits in the middle of it (the reader's 1.35, as `panel.rs` lays out)
pub const LINE: f32 = 1.35;
/// the largest texture egui may ask for (WebGL2 guarantees 2048)
const MAX_TEXTURE: usize = 2048;

/// What paints a layer's text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// the deck's mono face (IBM Plex Mono Bold), laid out by egui
    Mono,
    /// the deck's pixel face (font8x8), the HUD's arcade look
    #[cfg(feature = "pixel")]
    Pixel,
}

/// One layer of a screen: a draw list in logical pixels, `scale` device pixels each, its edges on a grid of `grid` pixels
/// per logical one (`grid` = `scale`: the screen's own pixels; 1: a logical pixel grows whole, as a picture painted at
/// one pixel a cell and grown nearest did), its text in `face`.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub list: DrawList,
    pub scale: f32,
    pub grid: f32,
    pub face: Face,
}

/// What a run painted: egui's meshes in device pixels (a point is a pixel), clipped, in painter's order, and the textures
/// they need: `set` new or patched (each texture's deltas in their order) before the meshes are drawn, `free` after.
pub struct Painted {
    pub primitives: Vec<ClippedPrimitive>,
    pub set: Vec<(TextureId, ImageDelta)>,
    pub free: Vec<TextureId>,
    pub size: (u32, u32),
}

/// The pictures a layer's icons and logo frames come from (the editor's: its adapter rasters Feather's icons and the
/// creator's logo into one straight RGBA image): where each one lies in it, in uv.
pub struct Pictures {
    pub size: [usize; 2],
    pub rgba: Vec<u8>,
    pub icons: HashMap<&'static str, Rect>,
    pub frames: Vec<Rect>,
}

/// A screen of layers (bottom up) on one egui context.
pub struct Screen {
    ctx: Context,
    mono: &'static [u8],
    layers: Vec<Option<Layer>>,
    events: Vec<Event>,
    at: Pos2,
    size: (u32, u32),
    dirty: bool,
    #[cfg(feature = "pixel")]
    pixel: Option<egui::TextureHandle>,
    /// the icons' and the logo's image, and its texture once sent
    pictures: Option<(Arc<Pictures>, Option<egui::TextureHandle>)>,
}

impl Screen {
    /// `layers` empty layers; `mono` = the deck's mono face (a TrueType file)
    pub fn new(mono: &'static [u8], layers: usize) -> Screen {
        Screen {
            ctx: context(mono),
            mono,
            layers: vec![None; layers],
            events: Vec::new(),
            at: Pos2::ZERO,
            size: (0, 0),
            dirty: true,
            #[cfg(feature = "pixel")]
            pixel: None,
            pictures: None,
        }
    }

    /// Layer `i` shows `layer` from the next frame on (none: nothing).
    pub fn set(&mut self, i: usize, layer: Option<Layer>) {
        if let Some(slot) = self.layers.get_mut(i) {
            *slot = layer;
            self.dirty = true;
        }
    }

    /// Layer `i` back from the screen (it shows nothing from the next frame on): a caller that draws every frame keeps
    /// its list's memory.
    pub fn take(&mut self, i: usize) -> Option<Layer> {
        self.dirty = true;
        self.layers.get_mut(i).and_then(Option::take)
    }

    /// The pictures the layers' icons and logo frames come from (sent to the GPU with the next frame).
    pub fn set_pictures(&mut self, pictures: Pictures) {
        self.pictures = Some((Arc::new(pictures), None));
        self.dirty = true;
    }

    pub fn layer(&self, i: usize) -> Option<&Layer> {
        self.layers.get(i).and_then(Option::as_ref)
    }

    /// One input, in device pixels, for egui's input state (hover, presses, keys); what it means to the kit's
    /// components stays with their owner (deck-ui `menu.rs`, deck-show `face.rs`), who sets a new layer.
    pub fn input(&mut self, i: Input) {
        let modifiers = Modifiers::NONE;
        let key = |key| Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers };
        let press = |pos, button| [true, false].map(|pressed| Event::PointerButton { pos, button, pressed, modifiers });
        match i {
            Input::Up => self.events.push(key(Key::ArrowUp)),
            Input::Down => self.events.push(key(Key::ArrowDown)),
            Input::Left => self.events.push(key(Key::ArrowLeft)),
            Input::Right => self.events.push(key(Key::ArrowRight)),
            Input::Enter => self.events.push(key(Key::Enter)),
            Input::Back => self.events.push(key(Key::Escape)),
            // the menu's own key (Esc with nothing open, M, a pad's Start): no key of egui's
            Input::Open => {}
            Input::Point(x, y) => {
                self.at = pos2(x, y);
                self.events.push(Event::PointerMoved(self.at));
            }
            Input::Press(x, y) => {
                self.at = pos2(x, y);
                self.events.push(Event::PointerMoved(self.at));
                self.events.extend(press(self.at, PointerButton::Primary));
            }
            Input::Secondary => self.events.extend(press(self.at, PointerButton::Secondary)),
        }
        // a screen nobody repaints keeps only the latest input
        let n = self.events.len();
        self.events.drain(..n.saturating_sub(64));
    }

    /// The stage was made anew (a new GPU side holds none of egui's textures): a new context, everything sent again.
    pub fn again(&mut self) {
        self.ctx = context(self.mono);
        #[cfg(feature = "pixel")]
        {
            self.pixel = None;
        }
        if let Some((_, sent)) = self.pictures.as_mut() {
            *sent = None;
        }
        self.dirty = true;
    }

    /// egui's context (its input state, its fonts)
    pub fn context(&self) -> &Context {
        &self.ctx
    }

    /// A frame for a screen `size` device pixels large: None = nothing changed since the last one (draw what you have).
    pub fn frame(&mut self, size: (u32, u32)) -> Option<Painted> {
        if size != self.size {
            (self.size, self.dirty) = (size, true);
        }
        if !self.dirty && self.events.is_empty() {
            return None;
        }
        self.dirty = false;
        let mut raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(size.0 as f32, size.1 as f32))),
            max_texture_side: Some(MAX_TEXTURE),
            events: std::mem::take(&mut self.events),
            ..RawInput::default()
        };
        raw.viewports.entry(ViewportId::ROOT).or_default().native_pixels_per_point = Some(1.0);
        #[cfg(feature = "pixel")]
        let pixel = Some(self.pixel_atlas());
        #[cfg(not(feature = "pixel"))]
        let pixel: Option<TextureId> = None;
        let pictures = self.pictures_texture();
        let pictures = pictures.as_ref().map(|(id, p)| (*id, p.as_ref()));
        let layers = &self.layers;
        // a pass without egui's root Ui: only our shapes on the background layer
        let out = self.ctx.run_pass(raw, |ctx| {
            let painter = ctx.layer_painter(LayerId::background());
            let mut shapes = Vec::new();
            for layer in layers.iter().flatten() {
                ctx.fonts_mut(|fonts| paint(fonts, layer, (pixel, pictures), &mut shapes));
            }
            painter.extend(shapes);
        });
        let primitives = self.ctx.tessellate(out.shapes, out.pixels_per_point);
        // egui 0.37's delta asserts it was handled when dropped: taken apart here, the adapter gets plain lists
        let mut delta = out.textures_delta;
        let set = delta.set.drain().flat_map(|(id, deltas)| deltas.into_iter().map(move |d| (id, d))).collect();
        let free = std::mem::take(&mut delta.free).into_iter().collect();
        Some(Painted { primitives, set, free, size })
    }

    /// the pixel face's atlas as egui's texture (made once a context)
    #[cfg(feature = "pixel")]
    fn pixel_atlas(&mut self) -> TextureId {
        let ctx = &self.ctx;
        self.pixel
            .get_or_insert_with(|| {
                let side = crate::base::pixel::SIDE;
                let image = egui::ColorImage::from_rgba_unmultiplied([side, side], &crate::base::pixel::atlas());
                ctx.load_texture("deck-kit pixel face", image, egui::TextureOptions::NEAREST)
            })
            .id()
    }

    /// the pictures as egui's texture (sent once a context), sampled linearly
    fn pictures_texture(&mut self) -> Option<(TextureId, Arc<Pictures>)> {
        let ctx = &self.ctx;
        let (p, sent) = self.pictures.as_mut()?;
        let handle = sent.get_or_insert_with(|| ctx.load_texture("deck-kit pictures", egui::ColorImage::from_rgba_unmultiplied(p.size, &p.rgba), egui::TextureOptions::LINEAR));
        Some((handle.id(), p.clone()))
    }
}

/// a context whose only fonts are the deck's mono face, at one pixel a point
fn context(mono: &'static [u8]) -> Context {
    let ctx = Context::default();
    let mut fonts = FontDefinitions::empty();
    fonts.font_data.insert("mono".into(), Arc::new(FontData::from_static(mono)));
    for family in [FontFamily::Monospace, FontFamily::Proportional] {
        fonts.families.insert(family, vec!["mono".into()]);
    }
    ctx.set_fonts(fonts);
    ctx
}

fn color(c: [f32; 4]) -> Color32 {
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(b(c[0]), b(c[1]), b(c[2]), b(c[3]))
}

/// One layer's draw list as epaint's shapes, in painter's order: the rectangles as quads of one mesh (whole pixels, no
/// soft edge, as the CPU painter had them), the text as egui's galleys (mono) or the atlas's quads (pixel), the icons and
/// the logo's frames as quads on the pictures (when a screen has them).
fn paint(fonts: &mut egui::epaint::FontsView<'_>, layer: &Layer, (pixel, pictures): (Option<TextureId>, Option<(TextureId, &Pictures)>), out: &mut Vec<Shape>) {
    let (s, g) = (layer.scale.max(f32::MIN_POSITIVE), layer.grid.max(f32::MIN_POSITIVE));
    // a logical place on the layer's grid, in device pixels
    let snap = |v: f32| (v * g).round() * (s / g);
    let (texture, white) = match (layer.face, pixel) {
        #[cfg(feature = "pixel")]
        (Face::Pixel, Some(id)) => (id, pos2(4.0 / crate::base::pixel::SIDE as f32, 4.0 / crate::base::pixel::SIDE as f32)),
        _ => (TextureId::default(), egui::epaint::WHITE_UV),
    };
    let mut mesh = Mesh::with_texture(texture);
    let solid = Rect::from_min_max(white, white);
    let fill = |mesh: &mut Mesh, (x, y, w, h): (f32, f32, f32, f32), c: Color32| {
        let r = Rect::from_min_max(pos2(snap(x), snap(y)), pos2(snap(x + w), snap(y + h)));
        if r.width() > 0.0 && r.height() > 0.0 {
            mesh.add_rect_with_uv(r, solid, c);
        }
    };
    let flush = |mesh: &mut Mesh, out: &mut Vec<Shape>| {
        if !mesh.is_empty() {
            out.push(Shape::mesh(std::mem::replace(mesh, Mesh::with_texture(texture))));
        }
    };
    // the icons and logo frames in a row, on the pictures' texture
    let pics_texture = pictures.map_or(TextureId::default(), |(id, _)| id);
    let mut pics = Mesh::with_texture(pics_texture);
    let picture = |pics: &mut Mesh, mesh: &mut Mesh, out: &mut Vec<Shape>, (x, y, size): (f32, f32, f32), uv: Option<&Rect>, c: Color32| {
        if let Some(uv) = uv {
            flush(mesh, out);
            pics.add_rect_with_uv(Rect::from_min_size(pos2(x * s, y * s), vec2(size * s, size * s)), *uv, c);
        }
    };
    for p in layer.list.prims() {
        if !matches!(p, Prim::Icon { .. } | Prim::Logo { .. }) && !pics.is_empty() {
            out.push(Shape::mesh(std::mem::replace(&mut pics, Mesh::with_texture(pics_texture))));
        }
        match *p {
            Prim::Rect { x, y, w, h, r, color: c } if r > 0.0 => {
                flush(&mut mesh, out);
                let rect = Rect::from_min_size(pos2(x * s, y * s), vec2(w * s, h * s));
                out.push(Shape::rect_filled(rect, CornerRadius::same((r * s).round().min(255.0) as u8), color(c)));
            }
            Prim::Rect { x, y, w, h, color: c, .. } => fill(&mut mesh, (x, y, w, h), color(c)),
            Prim::Outline { x, y, w, h, r, thick, color: c } if r > 0.0 => {
                flush(&mut mesh, out);
                let rect = Rect::from_min_size(pos2(x * s, y * s), vec2(w * s, h * s));
                out.push(Shape::rect_stroke(rect, CornerRadius::same((r * s).round().min(255.0) as u8), Stroke::new(thick * s, color(c)), StrokeKind::Inside));
            }
            Prim::Outline { x, y, w, h, thick, color: c, .. } => {
                let (t, c) = (thick.max(1.0 / s), color(c));
                fill(&mut mesh, (x, y, w, t), c);
                fill(&mut mesh, (x, y + h - t, w, t), c);
                fill(&mut mesh, (x, y + t, t, h - 2.0 * t), c);
                fill(&mut mesh, (x + w - t, y + t, t, h - 2.0 * t), c);
            }
            Prim::Text { x, y, px, start, end, color: c } => {
                let text = layer.list.text_of(start, end);
                if text.is_empty() {
                    continue;
                }
                match layer.face {
                    #[cfg(feature = "pixel")]
                    Face::Pixel => glyphs(&mut mesh, text, (x, y, px), (s, g), color(c)),
                    Face::Mono => {
                        flush(&mut mesh, out);
                        let font = FontId::monospace(px * s);
                        let top = y * s + (px * s * LINE - fonts.row_height(&font)) / 2.0;
                        // the mono face carries no arrows: the core's sentences have them
                        let text = match fonts.has_glyphs(&font, text) {
                            true => text.to_owned(),
                            false => text.replace('→', ">").replace('←', "<"),
                        };
                        let galley = fonts.layout_no_wrap(text, font, color(c));
                        out.push(Shape::galley(pos2(snap(x), top), galley, color(c)));
                    }
                }
            }
            Prim::Icon { x, y, size, name, color: c } => {
                picture(&mut pics, &mut mesh, out, (x, y, size), pictures.and_then(|(_, p)| p.icons.get(name)), color(c));
            }
            Prim::Logo { x, y, size, frame } => {
                picture(&mut pics, &mut mesh, out, (x, y, size), pictures.and_then(|(_, p)| p.frames.get(frame)), Color32::WHITE);
            }
        }
    }
    flush(&mut mesh, out);
    if !pics.is_empty() {
        out.push(Shape::mesh(pics));
    }
}

/// `text` in the pixel face from (x, y) at size `px`: a quad per glyph from the atlas, its 8 cells whole pixels of the
/// layer's grid, in the middle of a line `px × LINE`
#[cfg(feature = "pixel")]
fn glyphs(mesh: &mut Mesh, text: &str, (x, y, px): (f32, f32, f32), (s, g): (f32, f32), c: Color32) {
    use crate::base::pixel::{cell, origin, SIDE};
    let k = s / g;
    let top = ((y + (px * LINE - px) / 2.0) * g).round() * k;
    let side = px * s;
    let uv = 8.0 / SIDE as f32;
    for (i, ch) in text.chars().enumerate() {
        if ch == ' ' {
            continue;
        }
        let left = ((x + i as f32 * px) * g).round() * k;
        let (ox, oy) = origin(cell(ch));
        let from = pos2(ox as f32 / SIDE as f32, oy as f32 / SIDE as f32);
        mesh.add_rect_with_uv(Rect::from_min_size(pos2(left, top), vec2(side, side)), Rect::from_min_size(from, vec2(uv, uv)), c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::epaint::Primitive;

    /// the deck's mono face (deck-outlet's assets)
    const MONO: &[u8] = include_bytes!("../../../outlet/assets/IBMPlexMono-Bold-latin.ttf");

    fn meshes(p: &Painted) -> Vec<&Mesh> {
        p.primitives.iter().filter_map(|c| if let Primitive::Mesh(m) = &c.primitive { Some(m) } else { None }).collect()
    }

    /// the colours of a painted frame's vertices
    #[cfg(feature = "live")]
    fn colors(p: &Painted) -> Vec<Color32> {
        meshes(p).iter().flat_map(|m| m.vertices.iter().map(|v| v.color)).collect()
    }

    #[test]
    fn a_layer_is_painted_once_in_device_pixels_and_again_only_when_it_changes() {
        let mut screen = Screen::new(MONO, 2);
        assert!(screen.frame((640, 480)).is_some(), "the first frame");
        assert!(screen.frame((640, 480)).is_none(), "nothing changed");
        let mut d = DrawList::new(0.6);
        d.rect(10.0, 20.0, 30.0, 5.0, [1.0, 0.0, 0.0, 1.0]);
        screen.set(0, Some(Layer { list: d, scale: 2.0, grid: 2.0, face: Face::Mono }));
        let p = screen.frame((640, 480)).expect("a new layer");
        let red = meshes(&p).into_iter().flat_map(|m| m.vertices.iter()).filter(|v| v.color == Color32::RED).map(|v| v.pos).collect::<Vec<_>>();
        assert_eq!(red, [pos2(20.0, 40.0), pos2(80.0, 40.0), pos2(20.0, 50.0), pos2(80.0, 50.0)], "the rectangle at twice its size, whole pixels");
        assert!(screen.frame((800, 600)).is_some(), "a new size paints again");
    }

    #[test]
    fn the_mono_face_is_laid_out_by_egui_and_its_atlas_is_sent() {
        let mut screen = Screen::new(MONO, 1);
        let mut d = DrawList::new(0.6);
        d.text(0.0, 0.0, 14.0, "Annehmen", [1.0; 4]);
        screen.set(0, Some(Layer { list: d, scale: 2.0, grid: 2.0, face: Face::Mono }));
        let p = screen.frame((320, 240)).expect("a frame");
        assert!(p.set.iter().any(|(id, _)| *id == TextureId::default()), "egui's font atlas goes to the GPU");
        let glyphs = meshes(&p).iter().map(|m| m.vertices.len()).sum::<usize>() / 4;
        assert_eq!(glyphs, 8, "a quad a letter");
        // the mono face is 0.6 em wide in egui too: what the panel lays out with
        let w = screen.context().fonts_mut(|f| f.layout_no_wrap("MMMMMMMMMM".into(), FontId::monospace(28.0), Color32::WHITE).size().x);
        assert!((w - 10.0 * 0.6 * 28.0).abs() < 2.0, "{w}");
    }

    #[cfg(feature = "pixel")]
    #[test]
    fn the_pixel_face_is_one_mesh_on_its_atlas_whole_pixels_of_its_grid() {
        let mut screen = Screen::new(MONO, 1);
        let mut d = DrawList::new(1.0);
        d.rect(0.0, 0.0, 16.0, 10.0, [0.0, 0.0, 0.0, 1.0]);
        d.text(0.0, 1.0, 8.0, "AB", [1.0; 4]);
        // a picture of one pixel a cell, grown to two: the glyph's top on the logical grid (round(1 + 1.4) = 2 → 4)
        screen.set(0, Some(Layer { list: d, scale: 2.0, grid: 1.0, face: Face::Pixel }));
        let p = screen.frame((320, 240)).expect("a frame");
        let m = meshes(&p);
        assert_eq!(m.len(), 1, "the ground and the glyphs on the atlas: one mesh");
        assert_ne!(m[0].texture_id, TextureId::default(), "the pixel face's atlas, not egui's fonts");
        assert!(p.set.iter().any(|(id, _)| *id == m[0].texture_id), "the atlas goes to the GPU");
        let tops: Vec<f32> = m[0].vertices.chunks(4).map(|q| q[0].pos.y).collect();
        assert_eq!(tops, [0.0, 4.0, 4.0], "the ground, then A and B");
        let lefts: Vec<f32> = m[0].vertices.chunks(4).map(|q| q[0].pos.x).collect();
        assert_eq!(lefts, [0.0, 0.0, 16.0], "a glyph 16 device pixels wide");
    }

    #[cfg(feature = "live")]
    #[test]
    fn a_panel_keeps_its_ground_edge_buttons_and_type_through_egui() {
        use crate::live::panel::{self, Part};
        let t = crate::live::tokens::panel();
        let mut d = DrawList::new(0.6);
        let parts = [Part::Caption("Flash · Vorschlag", "von ana"), Part::Title("top · Version 3"), Part::Buttons(&["Annehmen", "Ablehnen"], None, None)];
        panel::draw(&mut d, &t, (320.0, 240.0), &parts, false);
        let mut screen = Screen::new(MONO, 2);
        screen.set(1, Some(panel::layer(d, 2.0)));
        let p = screen.frame((640, 480)).expect("a frame");
        let c = colors(&p);
        assert!(c.contains(&Color32::from_rgb(0, 209, 255)), "the accent on the top edge");
        assert!(c.contains(&Color32::BLACK), "the ground");
        assert!(c.iter().filter(|&&c| c == Color32::WHITE).count() > 40, "the primary button and the white type");
    }

    #[test]
    fn icons_and_logo_frames_are_quads_on_the_pictures_in_painters_order() {
        let mut screen = Screen::new(MONO, 1);
        let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(0.5, 0.5));
        let rgba = vec![255; 4 * 4 * 4];
        screen.set_pictures(Pictures { size: [4, 4], rgba, icons: HashMap::from([("play", uv)]), frames: vec![uv] });
        let mut d = DrawList::new(0.6);
        d.rect(0.0, 0.0, 10.0, 10.0, [0.0, 0.0, 0.0, 1.0]);
        d.icon(1.0, 2.0, 8.0, "play", [1.0, 0.0, 0.0, 1.0]);
        d.icon(1.0, 2.0, 8.0, "no such icon", [1.0; 4]);
        d.logo(10.0, 0.0, 4.0, 0);
        d.rect(0.0, 20.0, 10.0, 10.0, [0.0, 0.0, 0.0, 1.0]);
        screen.set(0, Some(Layer { list: d, scale: 2.0, grid: 2.0, face: Face::Mono }));
        let p = screen.frame((320, 240)).expect("a frame");
        let m = meshes(&p);
        assert_eq!(m.iter().map(|m| m.vertices.len() / 4).collect::<Vec<_>>(), [1, 2, 1], "ground, the icon and the logo, ground");
        assert!(p.set.iter().any(|(id, _)| *id == m[1].texture_id), "the pictures go to the GPU");
        assert_eq!((m[1].vertices[0].pos, m[1].vertices[0].color), (pos2(2.0, 4.0), Color32::RED), "the icon in its colour");
        assert_eq!(m[1].vertices[4].color, Color32::WHITE, "the logo in its own colours");
    }

    #[test]
    fn input_reaches_egui_and_a_new_context_sends_everything_again() {
        let mut screen = Screen::new(MONO, 1);
        screen.frame((640, 480));
        screen.input(Input::Point(10.0, 20.0));
        assert!(screen.frame((640, 480)).is_some(), "input runs a frame");
        assert_eq!(screen.context().input(|i| i.pointer.hover_pos()), Some(pos2(10.0, 20.0)));
        screen.input(Input::Press(30.0, 40.0));
        screen.frame((640, 480));
        assert_eq!(screen.context().input(|i| i.pointer.interact_pos()), Some(pos2(30.0, 40.0)));
        screen.again();
        let p = screen.frame((640, 480)).expect("everything again");
        assert!(p.set.iter().any(|(id, d)| *id == TextureId::default() && d.pos.is_none()), "egui's font atlas as a whole");
    }
}
