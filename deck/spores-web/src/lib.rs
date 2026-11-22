//! The Spores widget in the browser (#763): one weave (the spore contract's JSON) drawn by deck-spores into a canvas –
//! the head, the canvas laid out and painted, the press, wheel, drag and keys handled by the widget's own input – and
//! the gestures the creator carries out handed back as JSON from each frame. The page drives it: `create` on a canvas,
//! `resize` in device pixels, the pointer in CSS pixels, `frame` on every animation frame.
#![cfg(target_arch = "wasm32")]

use deck_kit::base::draw::DrawList;
use deck_kit::base::fonts::{Faces, Widths};
use deck_kit::base::graph::Graph;
use deck_kit::base::screen::{Face, Layer, Pictures, Screen};
use deck_kit::base::tokens::{Theme, Tokens};
use deck_kit::editor::controls::Rect;
use deck_kit::egui::epaint::{ClippedPrimitive, TextureId};
use deck_spores::input::{Event, Gesture, Key};
use deck_spores::{view, Hit, State};
use egui_wgpu::{Renderer, RendererOptions, ScreenDescriptor};
use protocol::spore_network::{Canvas, Weave};
use std::collections::HashMap;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

/// the creator's base: 1rem of an editor page, CSS px (shared/tokens/editor.css)
const BASE: f32 = 14.0;
/// an icon's side in the pictures, px; the pictures' width
const ICON: u32 = 48;
const PICTURES_W: u32 = 512;
/// the deck's mono face
const MONO: &[u8] = include_bytes!("../../outlet/assets/IBMPlexMono-Bold-latin.ttf");

fn err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&format!("{e:#}"))
}

/// Feather's icons, the widget's only pictures
fn pictures() -> Pictures {
    let h = deck_kit::editor::icons::raster_height(PICTURES_W, ICON);
    let mut rgba = vec![255u8; (PICTURES_W * h * 4) as usize];
    rgba.chunks_exact_mut(4).for_each(|p| p[3] = 0);
    let icons = deck_kit::editor::icons::raster(&mut rgba, (PICTURES_W, h), ICON);
    Pictures { size: [PICTURES_W as usize, h as usize], rgba, icons, frames: Vec::new(), thumbs: HashMap::new() }
}

#[wasm_bindgen]
pub struct Spores {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    canvas: web_sys::HtmlCanvasElement,
    clear: wgpu::Color,
    screen: Screen,
    renderer: Renderer,
    widths: Arc<Widths>,
    jobs: Vec<ClippedPrimitive>,
    free: Vec<TextureId>,
    tokens: Tokens,
    /// device pixels per CSS pixel
    scale: f32,
    state: State,
    weave: Option<(Graph, Vec<(usize, Vec<usize>)>, Canvas)>,
    hits: Vec<(Rect, Hit)>,
    cursor: (f32, f32),
    gestures: Vec<serde_json::Value>,
    backend: &'static str,
}

#[wasm_bindgen]
impl Spores {
    /// The widget on `canvas`; `prefer`: "auto" (WebGPU if the browser has an adapter, else WebGL2) or "webgl2".
    pub async fn create(canvas: web_sys::HtmlCanvasElement, prefer: String) -> Result<Spores, JsValue> {
        let backends = if prefer == "webgl2" { wgpu::Backends::GL } else { wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL };
        let instance = wgpu::util::new_instance_with_webgpu_detection(wgpu::InstanceDescriptor { backends, ..wgpu::InstanceDescriptor::new_without_display_handle() }).await;
        let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone())).map_err(err)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions { power_preference: wgpu::PowerPreference::LowPower, force_fallback_adapter: false, compatible_surface: Some(&surface), apply_limit_buckets: false })
            .await
            .map_err(err)?;
        let backend = match adapter.get_info().backend {
            wgpu::Backend::BrowserWebGpu => "webgpu",
            wgpu::Backend::Gl => "webgl2",
            _ => "other",
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("spores-web"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            })
            .await
            .map_err(err)?;
        let caps = surface.get_capabilities(&adapter);
        let first = *caps.formats.first().ok_or_else(|| err("the canvas offers no format"))?;
        // egui-wgpu writes display values: a non-sRGB surface
        let format = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(first);
        // see-through where the browser lets it (the panel's ground shows), else the surface's colour
        let see_through = caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::PreMultiplied);
        let alpha_mode = if see_through { wgpu::CompositeAlphaMode::PreMultiplied } else { caps.alpha_modes[0] };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: canvas.width().max(1),
            height: canvas.height().max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: vec![],
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&device, &config);
        let faces = Faces::shipped(MONO);
        let widths = Arc::new(faces.widths());
        let mut screen = Screen::with_egui_text(faces, 1);
        screen.set_pictures(pictures());
        let renderer = Renderer::new(&device, format, RendererOptions { msaa_samples: 1, depth_stencil_format: None, dithering: false, predictable_texture_filtering: false });
        let tokens = Tokens::new(Theme::Night, BASE);
        let mut out = Spores {
            surface,
            device,
            queue,
            config,
            canvas,
            clear: wgpu::Color::TRANSPARENT,
            screen,
            renderer,
            widths,
            jobs: Vec::new(),
            free: Vec::new(),
            tokens,
            scale: 1.0,
            state: State::default(),
            weave: None,
            hits: Vec::new(),
            cursor: (0.0, 0.0),
            gestures: Vec::new(),
            backend,
        };
        out.set_clear(see_through);
        Ok(out)
    }

    /// "webgpu" or "webgl2"
    pub fn backend(&self) -> String {
        self.backend.to_string()
    }

    fn set_clear(&mut self, see_through: bool) {
        let c = self.tokens.color.surface;
        self.clear = if see_through { wgpu::Color::TRANSPARENT } else { wgpu::Color { r: c[0].into(), g: c[1].into(), b: c[2].into(), a: 1.0 } };
    }

    /// night (the default) or day
    pub fn set_theme(&mut self, night: bool) {
        self.tokens = Tokens::new(if night { Theme::Night } else { Theme::Day }, BASE);
        let see_through = self.clear.a == 0.0;
        self.set_clear(see_through);
        self.screen.again();
    }

    /// the canvas's size in device pixels, `scale` device pixels per CSS pixel
    pub fn resize(&mut self, w: u32, h: u32, scale: f32) {
        let (w, h) = (w.max(1), h.max(1));
        self.scale = scale.max(0.5);
        if (w, h) == (self.config.width, self.config.height) {
            return;
        }
        self.canvas.set_width(w);
        self.canvas.set_height(h);
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.screen.again();
    }

    /// The weave to show (the spore contract's JSON); a new level lets the selection go.
    pub fn set_weave(&mut self, json: &str) -> Result<(), JsValue> {
        let w: Weave = serde_json::from_str(json).map_err(err)?;
        let (g, frames) = deck_spores::weave::graph(&w);
        let canvas = w.canvas.clone().unwrap_or_default();
        if self.weave.as_ref().is_none_or(|(old, _, _)| old.nodes.iter().map(|n| &n.id).ne(g.nodes.iter().map(|n| &n.id))) {
            self.state.selected.clear();
            self.state.focus = None;
        }
        self.weave = Some((g, frames, canvas));
        Ok(())
    }

    /// The pointer moved to (x, y), CSS pixels in the canvas.
    pub fn pointer_move(&mut self, x: f32, y: f32) {
        self.cursor = (x, y);
        self.handle(Event::Drag { at: (x, y) });
    }

    /// A button pressed: 0 the left, 1 the middle (pans; the left pans while space is held).
    pub fn pointer_down(&mut self, x: f32, y: f32, button: u8, shift: bool) {
        self.cursor = (x, y);
        self.handle(Event::Shift(shift));
        if button == 1 || self.state.space {
            self.handle(Event::Grab { at: (x, y) });
            return;
        }
        let hit = self.hits.iter().rev().find(|(r, _)| r.has(x, y)).map(|(_, h)| *h);
        if let Some(hit) = hit {
            let hit = match hit {
                Hit::Node(i) if shift => Hit::NodeAlso(i),
                other => other,
            };
            self.handle(Event::Press { hit, at: (x, y) });
        }
    }

    pub fn pointer_up(&mut self) {
        self.handle(Event::Release);
    }

    /// the wheel at (x, y): `pinch` a trackpad's pinch (zooms), a trackpad's scroll pans (`fingers`), a mouse's notch zooms
    pub fn wheel(&mut self, x: f32, y: f32, dx: f32, dy: f32, fingers: bool, pinch: bool) -> bool {
        self.handle(Event::Wheel { at: (x, y), delta: (dx, dy), fingers, pinch })
    }

    /// A key ("Tab", "Escape", "f", "g" with ctrl, " " for space down / up). True = the widget took it.
    pub fn key(&mut self, key: &str, down: bool, shift: bool, ctrl: bool) -> bool {
        self.handle(Event::Shift(shift));
        if key == " " {
            return self.handle(Event::Space(down && self.state.over(self.cursor.0, self.cursor.1)));
        }
        if !down {
            return false;
        }
        let key = match key {
            "Tab" => Key::Tab,
            "Escape" => Key::Escape,
            "f" | "F" => Key::Frame,
            "g" | "G" if ctrl => Key::Group,
            _ => return false,
        };
        self.handle(Event::Key { key, at: self.cursor })
    }

    /// what is selected, the focused spore first when there is one (JSON: ids)
    pub fn selection(&self) -> String {
        let mut ids: Vec<&String> = self.state.focus.iter().collect();
        ids.extend(self.state.selected.iter().filter(|s| Some(*s) != self.state.focus.as_ref()));
        serde_json::to_string(&ids).unwrap_or_default()
    }

    fn handle(&mut self, e: Event) -> bool {
        let (took, g) = self.state.handle(e);
        if let Some(g) = g {
            self.gestures.push(gesture_json(&g));
        }
        took
    }

    /// One frame at `secs` (the page's clock): the widget drawn; the gestures since the last frame (JSON array).
    pub fn frame(&mut self, secs: f64) -> Result<String, JsValue> {
        self.state.secs = secs as f32;
        let (w, h) = (self.config.width, self.config.height);
        let (lw, lh) = (w as f32 / self.scale, h as f32 / self.scale);
        let t = &self.tokens;
        let mut d = DrawList::with_widths(self.widths.clone());
        let area = Rect { x: 0.0, y: 0.0, w: lw, h: lh };
        let empty = Graph::default();
        let (g, frames, doc) = match &self.weave {
            Some((g, f, c)) => (g, f.as_slice(), c),
            None => (&empty, &[][..], &Canvas::default()),
        };
        let mut hits = view::head(&mut d, t, &self.state, g, area);
        let canvas = Rect { x: 0.0, y: t.size.row, w: lw, h: (lh - t.size.row).max(0.0) };
        let laid = view::lay(t, &mut self.state, (g, frames, doc), self.cursor, canvas);
        let none = |_: &str| None;
        let lent = view::Lent { flow: Some(secs as f32), lifted: None, marked: &[], editing: &none };
        hits.extend(view::paint(&mut d, t, &mut self.state, (g, &laid, doc), lent, self.cursor));
        d.clip(None);
        self.hits = hits;
        self.screen.set(0, Some(Layer { list: d, scale: self.scale, grid: self.scale, face: Face::Mono }));
        let painted = self.screen.frame((w, h));
        self.screen.take(0);
        if let Some(p) = painted {
            for id in self.free.drain(..) {
                self.renderer.free_texture(&id);
            }
            for (id, delta) in &p.set {
                self.renderer.update_texture(&self.device, &self.queue, *id, delta);
            }
            self.free = p.free;
            self.jobs = p.primitives;
        }
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                self.screen.again();
                return Ok("[]".into());
            }
            other => return Err(err(format!("the canvas's surface: {other:?}"))),
        };
        let view = frame.texture.create_view(&Default::default());
        let mut enc = self.device.create_command_encoder(&Default::default());
        let desc = ScreenDescriptor { size_in_pixels: [w, h], pixels_per_point: 1.0 };
        let _ = self.renderer.update_buffers(&self.device, &self.queue, &mut enc, &self.jobs, &desc);
        {
            let mut rp = enc
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    multiview_mask: None,
                    label: Some("spores"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(self.clear), store: wgpu::StoreOp::Store },
                    })],
                    ..Default::default()
                })
                .forget_lifetime();
            self.renderer.render(&mut rp, &self.jobs, &desc);
        }
        self.queue.submit([enc.finish()]);
        self.queue.present(frame);
        let out = serde_json::Value::Array(std::mem::take(&mut self.gestures));
        Ok(out.to_string())
    }
}

/// a gesture as the page reads it: `{ "kind": …, … }`
fn gesture_json(g: &Gesture) -> serde_json::Value {
    use serde_json::json;
    match g {
        Gesture::Fold(id) => json!({ "kind": "fold", "spore": id }),
        Gesture::Group(ids) => json!({ "kind": "group", "spores": ids }),
        Gesture::Note(near) => json!({ "kind": "note", "near": near }),
        Gesture::RenameGroup(k) => json!({ "kind": "renameGroup", "group": k }),
        Gesture::RecolourGroup(k) => json!({ "kind": "recolourGroup", "group": k }),
        Gesture::FoldGroup(k) => json!({ "kind": "foldGroup", "group": k }),
        Gesture::EditNote(k) => json!({ "kind": "editNote", "note": k }),
        Gesture::TakePin((spore, out, port)) => json!({ "kind": "takePort", "spore": spore, "outlet": out, "port": port }),
        Gesture::TakeSpore { id, from } => json!({ "kind": "takeSpore", "spore": id, "at": [from.0, from.1] }),
    }
}
