//! The native editor's proof (#444, research/NATIVE-EDITOR-GUI.md §6; owner 2026-11-09: our own UI, not GPUI): one window
//! on deck-native's pieces (#443 `host::window`, `gpu::Gpu`), the stage on the left – catpope's core rig played and drawn
//! as the deck does it (stage.rs) –, a panel on the right: a draggable divider, three tabs, the rig's knobs as
//! scrub-numbers (drag turns them in the engine), one text field (umlauts, dead keys, the IME, selection, copy and
//! paste), a stress tab of 200 controls. The design tokens come from a watched file (tokens.rs). Everything is drawn in
//! one 2D pass (ui.rs). The gates are measured and printed at exit: `--frames N` quits after N frames with the numbers
//! (the UI's build time per frame, the frame interval, the audio glitches); `--screenshot FILE` saves the last frame.
//!
//! deck-editor [--rig catpope] [--frames N] [--screenshot FILE] [--tokens FILE] [--stress] [--volume V]
mod bar;
mod stage;
mod text;
mod tokens;
mod ui;

use anyhow::{Context, Result};
use deck_native::gpu::Gpu;
use deck_native::host;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, Ime, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowId};

struct Args {
    rig: String,
    frames: Option<u64>,
    screenshot: Option<PathBuf>,
    tokens: Option<PathBuf>,
    stress: bool,
    volume: f32,
    /// the window's logical size (the gate is measured at 1920,1080)
    size: Option<(u32, u32)>,
}

fn args() -> Result<Args> {
    let mut a = Args { rig: "catpope".into(), frames: None, screenshot: None, tokens: None, stress: false, volume: 0.5, size: None };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut val = || it.next().with_context(|| format!("{k} needs a value"));
        match k.as_str() {
            "--rig" => a.rig = val()?,
            "--frames" => a.frames = Some(val()?.parse()?),
            "--screenshot" => a.screenshot = Some(PathBuf::from(val()?)),
            "--tokens" => a.tokens = Some(PathBuf::from(val()?)),
            "--volume" => a.volume = val()?.parse()?,
            "--stress" => a.stress = true,
            "--size" => {
                let v = val()?;
                let (w, h) = v.split_once([',', 'x']).with_context(|| format!("--size takes W,H, not {v}"))?;
                a.size = Some((w.trim().parse()?, h.trim().parse()?));
            }
            "--help" | "-h" => {
                println!("deck-editor [--rig catpope] [--frames N] [--screenshot FILE] [--tokens FILE] [--stress] [--size W,H] [--volume V]");
                std::process::exit(0);
            }
            other => anyhow::bail!("unknown argument {other}"),
        }
    }
    Ok(a)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Rig,
    Text,
    Stress,
}

const TABS: [(Tab, &str); 3] = [(Tab::Rig, "Rig"), (Tab::Text, "Text"), (Tab::Stress, "Stress")];
/// the stress tab's controls (the gate: 200 controls at 1080p)
const STRESS: usize = 200;
/// a scrub-number's travel for the knob's whole range (pixels)
const SCRUB_PX: f32 = 240.0;

/// what the pointer is over, from the last layout
#[derive(Clone, Copy, PartialEq)]
enum Hit {
    Nothing,
    Stage,
    Divider,
    Tab(Tab),
    Knob(usize),
    Field(f32),
    Stress(usize),
}

/// what is being dragged
#[derive(Clone, Copy, PartialEq)]
enum Drag {
    Divider,
    Knob(usize, f32, f32),
    Stress(usize, f32, f32),
    Select,
}

struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Rect {
    fn has(&self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

/// the panel's state and this frame's hit rectangles
struct Panel {
    width: f32,
    tab: Tab,
    field: text::Field,
    stress: Vec<f32>,
    drag: Option<Drag>,
    hits: Vec<(Rect, Hit)>,
    field_rect: Rect,
    advance: f32,
}

struct App {
    args: Args,
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    ui: Option<ui::Ui>,
    stage: stage::Stage,
    tokens: tokens::Watched,
    panel: Panel,
    mods: ModifiersState,
    cursor: (f32, f32),
    last: Option<Instant>,
    started: Instant,
    frames: u64,
    /// per frame: the UI's build and encode time (ms), the frame interval (ms)
    ui_ms: Vec<f32>,
    frame_ms: Vec<f32>,
    exit: i32,
    clipboard: Option<arboard::Clipboard>,
    token_changes: u32,
}

fn main() {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,wgpu_core=warn,wgpu_hal=error,naga=warn".into())).init();
    match run() {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            tracing::error!("{e:#}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<i32> {
    let args = args()?;
    let stage = stage::Stage::new(&args.rig, args.volume)?;
    let tokens = tokens::Watched::new(args.tokens.clone().unwrap_or_else(tokens::default_path));
    let event_loop = EventLoop::new().context("no event loop (no display?)")?;
    let width = tokens.tokens.size.panel;
    let mut app = App {
        panel: Panel { width, tab: if args.stress { Tab::Stress } else { Tab::Rig }, field: text::Field::with("Grüße an die Bühne – type here"), stress: (0..STRESS).map(|i| (i % 17) as f32 / 16.0).collect(), drag: None, hits: Vec::new(), field_rect: Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }, advance: 8.0 },
        args,
        window: None,
        gpu: None,
        ui: None,
        stage,
        tokens,
        mods: ModifiersState::empty(),
        cursor: (0.0, 0.0),
        last: None,
        started: Instant::now(),
        frames: 0,
        ui_ms: Vec::new(),
        frame_ms: Vec::new(),
        exit: 0,
        clipboard: arboard::Clipboard::new().map_err(|e| tracing::warn!("no clipboard: {e}")).ok(),
        token_changes: 0,
    };
    event_loop.run_app(&mut app).context("the event loop")?;
    let summary = app.summary();
    tracing::info!("{summary}");
    println!("{summary}");
    Ok(app.exit)
}

impl App {
    fn summary(&self) -> String {
        let stat = |v: &[f32]| -> (f32, f32) {
            if v.is_empty() {
                return (0.0, 0.0);
            }
            let mut s = v.to_vec();
            s.sort_by(f32::total_cmp);
            (s.iter().sum::<f32>() / s.len() as f32, s[s.len() * 99 / 100])
        };
        let (ui_avg, ui_p99) = stat(&self.ui_ms);
        let (f_avg, f_p99) = stat(&self.frame_ms);
        let size = self.gpu.as_ref().map(|g| g.size()).unwrap_or((0, 0));
        format!(
            "deck-editor: frames={} size={}x{} controls={} ui_ms avg={ui_avg:.2} p99={ui_p99:.2} frame_ms avg={f_avg:.2} p99={f_p99:.2} glitches={} audio={} tokens_reloads={} adapter={}",
            self.frames,
            size.0,
            size.1,
            if self.panel.tab == Tab::Stress { STRESS } else { self.stage.knobs.len() },
            self.stage.glitches(),
            self.stage.audio,
            self.token_changes,
            self.gpu.as_ref().map(|g| g.adapter.name.clone()).unwrap_or_default()
        )
    }

    fn scale(&self) -> f32 {
        self.window.as_ref().map_or(1.0, |w| w.scale_factor() as f32)
    }

    /// what is under the pointer, from the last frame's layout
    fn hit(&self, x: f32, y: f32) -> Hit {
        self.panel.hits.iter().find(|(r, _)| r.has(x, y)).map_or(Hit::Nothing, |(_, h)| *h)
    }

    fn knob_at(&mut self, i: usize, x: f32) {
        let Some(Drag::Knob(_, x0, v0)) = self.panel.drag else { return };
        let k = &self.stage.knobs[i];
        let v = scrub(v0, x - x0, k.min, k.max, k.log);
        let (node, port, min, max) = (k.node, k.port_index, k.min, k.max);
        self.stage.set(node, port, v, min, max);
    }

    fn pointer(&mut self, x: f32, y: f32) {
        self.cursor = (x, y);
        match self.panel.drag {
            Some(Drag::Divider) => {
                let w = self.gpu.as_ref().map_or(1280.0, |g| g.size().0 as f32);
                self.panel.width = (w - x).clamp(200.0, w - 200.0);
            }
            Some(Drag::Knob(i, _, _)) => self.knob_at(i, x),
            Some(Drag::Stress(i, x0, v0)) => self.panel.stress[i] = (v0 + (x - x0) / SCRUB_PX).clamp(0.0, 1.0),
            Some(Drag::Select) => {
                let col = ((x - self.panel.field_rect.x) / self.panel.advance).round().max(0.0) as usize;
                let to = self.panel.field.byte_at_char(col);
                self.panel.field.go(to, true);
            }
            None => {
                if let Hit::Stage = self.hit(x, y) {
                    let s = self.scale();
                    let top = self.tokens.tokens.size.bar;
                    self.stage.pointer(x * s, (y - top) * s);
                }
            }
        }
    }

    fn press(&mut self) {
        let (x, y) = self.cursor;
        match self.hit(x, y) {
            Hit::Divider => self.panel.drag = Some(Drag::Divider),
            Hit::Tab(t) => {
                self.panel.tab = t;
                self.focus(false);
            }
            Hit::Knob(i) => {
                let k = &self.stage.knobs[i];
                let v = self.stage.value(k);
                self.panel.drag = Some(Drag::Knob(i, x, v));
                self.focus(false);
            }
            Hit::Stress(i) => self.panel.drag = Some(Drag::Stress(i, x, self.panel.stress[i])),
            Hit::Field(_) => {
                let col = ((x - self.panel.field_rect.x) / self.panel.advance).round().max(0.0) as usize;
                let to = self.panel.field.byte_at_char(col);
                self.panel.field.go(to, self.mods.shift_key());
                self.panel.drag = Some(Drag::Select);
                self.focus(true);
            }
            Hit::Stage | Hit::Nothing => self.focus(false),
        }
    }

    fn focus(&mut self, on: bool) {
        if self.panel.field.focused != on {
            self.panel.field.focused = on;
            if let Some(w) = &self.window {
                w.set_ime_allowed(on);
            }
        }
    }

    /// a key while the field has the focus
    fn key(&mut self, k: &KeyEvent) {
        if k.state != ElementState::Pressed {
            return;
        }
        let (ctrl, shift) = (self.mods.control_key() || self.mods.super_key(), self.mods.shift_key());
        let f = &mut self.panel.field;
        match k.logical_key.as_ref() {
            Key::Named(NamedKey::Backspace) => f.backspace(),
            Key::Named(NamedKey::Delete) => f.delete(),
            Key::Named(NamedKey::ArrowLeft) => f.left(ctrl, shift),
            Key::Named(NamedKey::ArrowRight) => f.right(ctrl, shift),
            Key::Named(NamedKey::Home) => f.home(shift),
            Key::Named(NamedKey::End) => f.end(shift),
            Key::Named(NamedKey::Escape) => self.focus(false),
            Key::Character(c) if ctrl => match c.to_ascii_lowercase().as_str() {
                "a" => f.select_all(),
                "c" | "x" => {
                    let s = f.selected().to_string();
                    if !s.is_empty() {
                        if let Some(cb) = self.clipboard.as_mut() {
                            let _ = cb.set_text(s);
                        }
                        if c.eq_ignore_ascii_case("x") {
                            f.backspace();
                        }
                    }
                }
                "v" => {
                    if let Some(t) = self.clipboard.as_mut().and_then(|cb| cb.get_text().ok()) {
                        f.insert(t.lines().next().unwrap_or(""));
                    }
                }
                _ => {}
            },
            _ => {
                // the text the key produces: umlauts and dead-key compositions arrive here, composed
                if !ctrl && !self.mods.alt_key() {
                    if let Some(t) = k.text.as_ref().filter(|t| !t.chars().any(char::is_control)) {
                        f.insert(t);
                    }
                }
            }
        }
    }

    fn redraw(&mut self, el: &ActiveEventLoop) {
        let (Some(window), Some(gpu), Some(ui)) = (self.window.clone(), self.gpu.as_mut(), self.ui.as_mut()) else { return };
        if self.tokens.poll() {
            self.token_changes += 1;
        }
        let now = Instant::now();
        let dt = self.last.map(|l| (now - l).as_secs_f32()).unwrap_or(1.0 / 60.0).min(0.25);
        if let Some(l) = self.last {
            self.frame_ms.push((now - l).as_secs_f32() * 1000.0);
        }
        self.last = Some(now);
        let Some(frame) = gpu.acquire().unwrap_or_else(|e| {
            tracing::error!("render: {e:#}");
            self.exit = 1;
            el.exit();
            None
        }) else {
            window.request_redraw();
            return;
        };
        let t = &self.tokens.tokens;
        let scale = window.scale_factor() as f32;
        let (w, h) = gpu.size();
        let (wf, hf) = (w as f32 / scale, h as f32 / scale);
        let panel_w = self.panel.width.min(wf - 100.0);
        let divider = t.size.divider;
        let stage_w = (wf - panel_w - divider).max(1.0);
        // the regions start under the top bar (bar.rs: the title, the status, the version)
        let top = t.size.bar;
        let body = (hf - top).max(1.0);
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        // the stage at its viewport's size in pixels
        let (stage_view, fresh) = self.stage.frame(&gpu.device, &gpu.queue, &mut enc, ((stage_w * scale) as u32, (body * scale) as u32), dt);
        if fresh {
            ui.set_stage(&gpu.device, stage_view);
        }
        // the panel: everything in logical pixels, the pass scales
        let t0 = Instant::now();
        ui.begin();
        ui.stage(0.0, top, stage_w, body);
        let px = panel_w;
        let x0 = wf - panel_w;
        ui.rect(x0 - divider, top, divider, body, t.color.divider);
        ui.rect(x0, top, px, body, t.color.panel);
        self.panel.hits.clear();
        self.panel.hits.push((Rect { x: x0 - divider - 3.0, y: top, w: divider + 6.0, h: body }, Hit::Divider));
        let (pad, font, row) = (t.size.pad, t.size.font, t.size.row);
        let mut y = top + pad;
        // the tabs
        let mut tx = x0 + pad;
        for (tab, name) in TABS {
            let tw = ui.atlas.advance(font) * name.chars().count() as f32 + pad;
            let active = tab == self.panel.tab;
            ui.rect(tx, y, tw, row - 4.0, if active { t.color.tab_active } else { t.color.tab });
            if active {
                ui.rect(tx, y + row - 6.0, tw, 2.0, t.color.accent);
            }
            ui.text(tx + pad / 2.0, y + 4.0, font, name, if active { t.color.text } else { t.color.muted });
            self.panel.hits.push((Rect { x: tx, y, w: tw, h: row }, Hit::Tab(tab)));
            tx += tw + 4.0;
        }
        y += row + pad / 2.0;
        match self.panel.tab {
            Tab::Rig => {
                ui.text(x0 + pad, y, font * 0.85, "drag a number to turn the knob in the engine", t.color.muted);
                y += row * 0.8;
                let inner = px - 2.0 * pad;
                for i in 0..self.stage.knobs.len() {
                    if y + row > hf - pad {
                        break;
                    }
                    let k = &self.stage.knobs[i];
                    let v = self.stage.value(k);
                    let frac = fraction(v, k.min, k.max, k.log);
                    ui.text(x0 + pad, y + 5.0, font * 0.9, &k.label, t.color.text);
                    let bx = x0 + pad + inner * 0.5;
                    ui.rect(bx, y + 2.0, inner * 0.5, row - 6.0, t.color.field);
                    ui.rect(bx, y + 2.0, inner * 0.5 * frac, row - 6.0, [t.color.accent[0], t.color.accent[1], t.color.accent[2], 0.35]);
                    let dragging = matches!(self.panel.drag, Some(Drag::Knob(j, _, _)) if j == i);
                    ui.text(bx + 6.0, y + 5.0, font * 0.9, &number(v), if dragging { t.color.accent } else { t.color.text });
                    self.panel.hits.push((Rect { x: bx, y, w: inner * 0.5, h: row }, Hit::Knob(i)));
                    y += row;
                }
            }
            Tab::Text => {
                ui.text(x0 + pad, y, font * 0.85, "umlauts, dead keys, IME · shift+arrows · ctrl+a/c/x/v", t.color.muted);
                y += row * 0.8;
                let fx = x0 + pad;
                let fw = px - 2.0 * pad;
                let focused = self.panel.field.focused;
                ui.rect(fx, y, fw, row, t.color.field);
                if focused {
                    ui.rect(fx, y + row - 2.0, fw, 2.0, t.color.accent);
                }
                let adv = ui.atlas.advance(font);
                self.panel.advance = adv;
                self.panel.field_rect = Rect { x: fx + 6.0, y, w: fw - 6.0, h: row };
                let (shown, cursor_col) = self.panel.field.shown();
                if let Some((a, b)) = self.panel.field.selection() {
                    let (ca, cb) = (self.panel.field.chars_before(a) as f32, self.panel.field.chars_before(b) as f32);
                    ui.rect(fx + 6.0 + ca * adv, y + 4.0, (cb - ca) * adv, row - 8.0, t.color.selection);
                }
                ui.text(fx + 6.0, y + 6.0, font, &shown, t.color.text);
                if let Some((p, _)) = &self.panel.field.preedit {
                    let c0 = self.panel.field.chars_before(self.panel.field.cursor) as f32;
                    ui.rect(fx + 6.0 + c0 * adv, y + row - 5.0, p.chars().count() as f32 * adv, 1.5, t.color.accent);
                }
                if focused && (self.started.elapsed().as_millis() / 500) % 2 == 0 {
                    ui.rect(fx + 6.0 + cursor_col as f32 * adv, y + 5.0, 1.5, row - 10.0, t.color.text);
                }
                self.panel.hits.push((Rect { x: fx, y, w: fw, h: row }, Hit::Field(fx + 6.0)));
                if focused {
                    let cx = (fx + 6.0 + cursor_col as f32 * adv) * scale;
                    window.set_ime_cursor_area(winit::dpi::PhysicalPosition::new(cx as i32, (y * scale) as i32), winit::dpi::PhysicalSize::new(2u32, (row * scale) as u32));
                }
                y += row + pad;
                ui.text(x0 + pad, y, font * 0.85, &format!("{} chars · cursor {} · selected {:?}", self.panel.field.text.chars().count(), cursor_col, self.panel.field.selected()), t.color.muted);
            }
            Tab::Stress => {
                ui.text(x0 + pad, y, font * 0.85, &format!("{STRESS} scrub-numbers (the gate: UI ≤ 2 ms at 1080p)"), t.color.muted);
                y += row * 0.8;
                let cols = 4;
                let cw = (px - 2.0 * pad) / cols as f32;
                let rh = ((hf - y - pad) / (STRESS / cols) as f32).clamp(8.0, row);
                for i in 0..STRESS {
                    let (c, r) = ((i % cols) as f32, (i / cols) as f32);
                    let (cx, cy) = (x0 + pad + c * cw, y + r * rh);
                    let v = self.panel.stress[i];
                    ui.rect(cx + 1.0, cy + 1.0, cw - 2.0, rh - 2.0, t.color.field);
                    ui.rect(cx + 1.0, cy + 1.0, (cw - 2.0) * v, rh - 2.0, [t.color.accent[0], t.color.accent[1], t.color.accent[2], 0.35]);
                    if rh >= 12.0 {
                        ui.text(cx + 4.0, cy + (rh - font * 0.7) * 0.5, font * 0.7, &format!("{:.2}", v), t.color.text);
                    }
                    self.panel.hits.push((Rect { x: cx, y: cy, w: cw, h: rh }, Hit::Stress(i)));
                }
            }
        }
        // the status: the numbers of the gates, live (a broken tokens file first: the bar cuts from the end)
        let recent = &self.frame_ms[self.frame_ms.len().saturating_sub(30)..];
        let mean = if recent.is_empty() { 16.7 } else { recent.iter().sum::<f32>() / recent.len() as f32 };
        let status = format!(
            "{}{:.0} fps · ui {:.2} ms · {} quads · {} · {}",
            self.tokens.problem.as_ref().map(|p| format!("{p} – ")).unwrap_or_default(),
            1000.0 / mean.max(0.1),
            self.ui_ms.last().copied().unwrap_or(0.0),
            ui.quads(),
            self.stage.glitch_text(),
            self.stage.audio
        );
        bar::draw(ui, t, wf, &status);
        self.panel.hits.push((Rect { x: 0.0, y: top, w: stage_w, h: body }, Hit::Stage));
        // the pass: logical pixels → the surface's, through the screen uniform
        let view = frame.texture.create_view(&Default::default());
        ui.flush(&gpu.device, &gpu.queue, &mut enc, &view, (wf, hf), t.color.background);
        self.ui_ms.push(t0.elapsed().as_secs_f32() * 1000.0);
        let shot = self.args.screenshot.clone().filter(|_| self.args.frames.is_some_and(|n| self.frames + 1 >= n) && gpu.can_capture);
        let readback = shot.as_ref().map(|_| copy_out(&gpu.device, &mut enc, &frame.texture, (w, h)));
        gpu.queue.submit([enc.finish()]);
        frame.present();
        if let (Some(path), Some((buf, stride))) = (shot, readback) {
            if let Err(e) = save(&gpu.device, &buf, stride, (w, h), gpu.format(), &path) {
                tracing::error!("screenshot: {e:#}");
            } else {
                tracing::info!("screenshot: {}", path.display());
            }
        }
        self.frames += 1;
        if self.args.frames.is_some_and(|n| self.frames >= n) {
            el.exit();
            return;
        }
        window.request_redraw();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = match host::window(el, "SCUTOONS editor – proof", true) {
            Ok(w) => w,
            Err(e) => {
                tracing::error!("no window: {e}");
                self.exit = 1;
                el.exit();
                return;
            }
        };
        match Gpu::new(window.clone()).and_then(|gpu| ui::Ui::new(&gpu.device, &gpu.queue, gpu.format(), self.tokens.tokens.size.font).map(|ui| (gpu, ui))) {
            Ok((gpu, ui)) => {
                self.gpu = Some(gpu);
                self.ui = Some(ui);
            }
            Err(e) => {
                tracing::error!("render: {e:#}");
                self.exit = 1;
                el.exit();
                return;
            }
        }
        window.set_cursor_visible(true);
        if let Some((w, h)) = self.args.size {
            let _ = window.request_inner_size(winit::dpi::LogicalSize::new(w, h));
        }
        self.window = Some(window.clone());
        window.request_redraw();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) => {
                if let Some(g) = self.gpu.as_mut() {
                    g.resize(s.width, s.height);
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {}
            WindowEvent::ModifiersChanged(m) => self.mods = m.state(),
            WindowEvent::CursorMoved { position, .. } => {
                let s = self.scale();
                self.pointer(position.x as f32 / s, position.y as f32 / s);
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => self.press(),
            WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Left, .. } => self.panel.drag = None,
            WindowEvent::MouseWheel { delta, .. } => {
                let steps = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                if let Hit::Knob(i) = self.hit(self.cursor.0, self.cursor.1) {
                    let k = &self.stage.knobs[i];
                    let v = self.stage.value(k);
                    let v = scrub(v, steps * 12.0, k.min, k.max, k.log);
                    let (node, port, min, max) = (k.node, k.port_index, k.min, k.max);
                    self.stage.set(node, port, v, min, max);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed && event.logical_key == Key::Named(NamedKey::Escape) && !self.panel.field.focused {
                    el.exit();
                } else if self.panel.field.focused {
                    self.key(&event);
                } else if event.state == ElementState::Pressed && event.logical_key == Key::Named(NamedKey::Tab) {
                    let i = TABS.iter().position(|(t, _)| *t == self.panel.tab).unwrap_or(0);
                    self.panel.tab = TABS[(i + 1) % TABS.len()].0;
                }
            }
            WindowEvent::Ime(ime) => match ime {
                Ime::Preedit(s, cursor) => self.panel.field.preedit(s, cursor),
                Ime::Commit(s) => self.panel.field.insert(&s),
                Ime::Enabled | Ime::Disabled => {}
            },
            WindowEvent::RedrawRequested => self.redraw(el),
            _ => {}
        }
    }
}

/// a knob's value under the pointer's travel `dx` from `v0` (the whole range over SCRUB_PX; a log knob in its exponent)
fn scrub(v0: f32, dx: f32, min: f32, max: f32, log: bool) -> f32 {
    let f0 = fraction(v0, min, max, log);
    let f = (f0 + dx / SCRUB_PX).clamp(0.0, 1.0);
    if log && min > 0.0 && max > 0.0 {
        min * (max / min).powf(f)
    } else {
        min + (max - min) * f
    }
}

fn fraction(v: f32, min: f32, max: f32, log: bool) -> f32 {
    if log && min > 0.0 && max > 0.0 && v > 0.0 {
        ((v / min).ln() / (max / min).ln()).clamp(0.0, 1.0)
    } else if (max - min).abs() > f32::EPSILON {
        ((v - min) / (max - min)).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn number(v: f32) -> String {
    if v.abs() >= 100.0 {
        format!("{v:.0}")
    } else if v.abs() >= 10.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.3}")
    }
}

/// the surface's texture into a buffer, for `save` (rows padded to 256 bytes)
fn copy_out(device: &wgpu::Device, enc: &mut wgpu::CommandEncoder, tex: &wgpu::Texture, (w, h): (u32, u32)) -> (wgpu::Buffer, u32) {
    let stride = (w * 4).div_ceil(256) * 256;
    let buf = device.create_buffer(&wgpu::BufferDescriptor { label: Some("screenshot"), size: u64::from(stride) * u64::from(h), usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture: tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(stride), rows_per_image: None } },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    (buf, stride)
}

fn save(device: &wgpu::Device, buf: &wgpu::Buffer, stride: u32, (w, h): (u32, u32), format: wgpu::TextureFormat, path: &std::path::Path) -> Result<()> {
    let slice = buf.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::PollType::Wait).context("waiting for the screenshot")?;
    rx.recv().context("the map's answer")?.context("mapping the screenshot")?;
    let data = slice.get_mapped_range();
    let bgra = matches!(format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for row in 0..h {
        let line = &data[(row * stride) as usize..(row * stride + w * 4) as usize];
        for p in line.chunks_exact(4) {
            if bgra {
                px.extend_from_slice(&[p[2], p[1], p[0], 255]);
            } else {
                px.extend_from_slice(&[p[0], p[1], p[2], 255]);
            }
        }
    }
    drop(data);
    buf.unmap();
    image::RgbaImage::from_raw(w, h, px).context("the screenshot's pixels")?.save(path).with_context(|| format!("writing {}", path.display()))
}
