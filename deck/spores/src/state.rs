//! The widget's state: its zoom and pan, the direction, what is selected and focused, the reveals fading, what was
//! drawn last (where spores, ports and threads are on screen); framing, focus and the box select (pick.rs).
use crate::paint::Reveal;
use crate::pick::{meets, Boxing};
use crate::Seen;
use deck_kit::base::canvas::View;
use deck_kit::base::graph::{Direction, Graph};
use deck_kit::base::wire::Wire;
use deck_kit::editor::controls::Rect;
use std::collections::BTreeMap;

/// a port of a spore drawn: the spore, whether an outlet, which of them
pub type At = (usize, bool, usize);

/// the direction, the canvas's size and the graph's extent a view was framed for
pub type Framed = (Direction, (f32, f32), (f32, f32));

/// a group's frame in the graph: its index in the canvas, its spores
pub type Frames = [(usize, Vec<usize>)];

/// how long a focused spore's flash and shimmer run, secs
pub const FLASH: f32 = 0.6;

/// the switches the widget follows (the host's settings; "reduce motion" turns the first three off)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motion {
    /// what runs on the threads: pulses, streams, idle threads faint
    pub activity: bool,
    /// the latent grid under what the pointer is on
    pub reveal: bool,
    /// the border flash and shimmer of a focused spore
    pub flash: bool,
    /// a spore's picture of the piece it plays
    pub previews: bool,
}

impl Default for Motion {
    fn default() -> Motion {
        Motion { activity: true, reveal: true, flash: true, previews: false }
    }
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub view: View,
    pub direction: Direction,
    /// framed for this direction, canvas size and graph extent: a new direction frames again, so does a new size or
    /// graph (another level) until the view is zoomed or panned by hand
    pub framed: Option<Framed>,
    pub touched: bool,
    /// the canvas drawn last, on screen; whether it was drawn this frame
    pub canvas: Rect,
    pub shown: bool,
    pub bounds: Rect,
    pub grab: Option<(f32, f32)>,
    /// space is held: a left drag pans
    pub space: bool,
    /// shift is held (Tab goes back)
    pub shift: bool,
    /// a check run's zoom, round the canvas's middle whenever it frames itself
    pub zoom_once: Option<f32>,
    /// the selected spores and the focused one, by id
    pub selected: Vec<String>,
    pub focus: Option<String>,
    /// the spore that flashed, and when (secs)
    pub flash: Option<(String, f32)>,
    /// what the pointer is on (`node:<id>`, `group:<id>`, `wire:<n>`) and the reveals fading in and out
    pub hover: Option<String>,
    pub reveals: Vec<Reveal>,
    /// the pointer as the latent grid follows it, slowly (canvas)
    pub blob: Option<(f32, f32)>,
    /// the host's clock, secs, and the settings' switches (the host sets them each frame)
    pub secs: f32,
    pub motion: Motion,
    /// the pictures of the pieces the spores play, by key, and the keys the last frame drew
    pub pictures: BTreeMap<String, u64>,
    pub wanted: Vec<String>,
    /// the spores drawn last: id and place on the canvas, in the graph's order; Tab's order through them; their tones
    pub drawn: Vec<(String, Rect)>,
    pub order: Vec<usize>,
    pub tones: Vec<f32>,
    /// the threads drawn last, by link: the key a pin keeps (None: no value to pin) and the middle on screen
    pub wire_keys: Vec<Option<String>>,
    pub wire_mids: Vec<Option<(f32, f32)>>,
    /// where the pinned values' tags were drawn last: the link and the tag's rect on screen
    pub pin_tags: Vec<(usize, Rect)>,
    /// the threads whose value stays shown, by key
    pub pinned: Vec<String>,
    /// the box being drawn
    pub boxing: Option<Boxing>,
    /// the tone asked for (Hz), until the host takes it
    pub tone: Option<f32>,
    /// the graph drawn last, where its open spores' ports are pressed and its threads hang, on screen
    pub graph: Graph,
    pub pins: Vec<(At, Rect)>,
    pub wires: Vec<(usize, Wire)>,
}

impl State {
    pub fn frame(&mut self) {
        self.frame_on(self.bounds);
    }

    pub fn frame_on(&mut self, r: Rect) {
        self.view = View::framing(r, (self.canvas.w, self.canvas.h), self.canvas.w.min(self.canvas.h) * 0.06);
    }

    pub fn over(&self, x: f32, y: f32) -> bool {
        self.canvas.w > 0.0 && self.canvas.has(x, y)
    }

    pub fn local(&self, x: f32, y: f32) -> (f32, f32) {
        (x - self.canvas.x, y - self.canvas.y)
    }

    pub fn id(&self, i: usize) -> Option<String> {
        self.drawn.get(i).map(|(id, _)| id.clone())
    }

    /// `id` alone selected and focused, flashing
    pub fn focus_on(&mut self, id: String) {
        self.selected = vec![id.clone()];
        self.flash = Some((id.clone(), self.secs));
        self.ask_tone(&id);
        self.focus = Some(id);
    }

    /// the flash's age, while it runs
    pub fn flashing(&self, id: &str) -> Option<f32> {
        let (f, at) = self.flash.as_ref()?;
        let age = self.secs - at;
        (self.motion.flash && f == id && (0.0..FLASH).contains(&age)).then_some(age / FLASH)
    }

    /// a press on the empty canvas at `at`: a box starts (a click without a drag is decided on release)
    pub fn box_start(&mut self, at: (f32, f32)) {
        let wire = self.hover.as_deref().and_then(|h| h.strip_prefix("wire:")).and_then(|i| i.parse::<usize>().ok()).and_then(|i| self.wire_keys.get(i).cloned().flatten());
        self.boxing = Some(Boxing { from: at, to: at, add: self.shift, before: self.selected.clone(), wire });
    }

    /// the pointer moved with the box in the hand: what it touches is selected as it grows
    pub fn box_move(&mut self, at: (f32, f32)) {
        let Some(b) = self.boxing.as_mut() else { return };
        b.to = at;
        if !b.dragged() {
            return;
        }
        let (r, add, before) = (b.rect(), b.add, b.before.clone());
        let inside: Vec<String> = self.drawn.iter().filter(|(_, n)| meets(self.on_screen(*n), r)).map(|(id, _)| id.clone()).collect();
        let mut picked = if add { before } else { Vec::new() };
        for id in inside {
            if !picked.contains(&id) {
                picked.push(id);
            }
        }
        self.selected = picked;
    }

    /// the button let go: a drag keeps what the box selected and focuses the first new spore in reading order; a click
    /// pins or unpins the thread under it, else lets the selection go (unless shift)
    pub fn box_end(&mut self) {
        let Some(b) = self.boxing.take() else { return };
        if b.dragged() {
            let new = self.order.iter().filter_map(|&i| self.drawn.get(i)).map(|(id, _)| id).find(|id| self.selected.contains(id) && !b.before.contains(id)).cloned();
            if let Some(id) = new {
                self.flash = Some((id.clone(), self.secs));
                self.ask_tone(&id);
                self.focus = Some(id);
            }
            return;
        }
        match b.wire {
            Some(key) => match self.pinned.iter().position(|k| *k == key) {
                Some(k) => {
                    self.pinned.remove(k);
                }
                None => self.pinned.push(key),
            },
            None if !b.add => {
                self.selected.clear();
                self.focus = None;
            }
            None => {}
        }
    }

    /// a spore's place on the canvas, on screen
    pub fn on_screen(&self, r: Rect) -> Rect {
        let (o, e) = (self.view.to_screen((r.x, r.y)), self.view.to_screen((r.x + r.w, r.y + r.h)));
        Rect { x: self.canvas.x + o.0, y: self.canvas.y + o.1, w: e.0 - o.0, h: e.1 - o.1 }
    }

    /// the spore `id` was focused or selected: its tone is wanted
    pub fn ask_tone(&mut self, id: &str) {
        if let Some(hz) = self.drawn.iter().position(|(x, _)| x == id).and_then(|i| self.tones.get(i).copied()) {
            self.tone = Some(hz);
        }
    }

    /// the tone asked for since the last call, Hz
    pub fn take_tone(&mut self) -> Option<f32> {
        self.tone.take()
    }
}

impl Seen for State {
    fn selected(&self, id: &str) -> bool {
        self.selected.iter().any(|s| s == id)
    }
    fn flashing(&self, id: &str) -> Option<f32> {
        State::flashing(self, id)
    }
    fn previews(&self) -> bool {
        self.motion.previews
    }
    fn picture(&self, key: &str) -> Option<u64> {
        self.pictures.get(key).copied()
    }
    fn secs(&self) -> f32 {
        self.secs
    }
    fn blob(&self) -> Option<(f32, f32)> {
        self.blob
    }
}
