//! The Show workspace's node view: the selected row as deck-show's graph (nodes.rs there), laid out left → right or top →
//! bottom (the workspace's setting) by deck-kit's graph, its pins and wires typed by deck-kit's signal table. Zoom and
//! pan as the stage: the wheel zooms round the pointer, a middle drag or space + drag pans, F frames the selection (or
//! everything). A click selects a node (shift adds), Tab moves the focus as a browser does; selected nodes group
//! (ctrl+G or Group), a note joins the focused node; groups, notes and folds are the row's (deck-show canvas.rs). A drag
//! on the empty canvas selects by a box, a click on a wire pins its value (node_pick.rs). Pins are wired by dragging,
//! wires cut, a node dropped on a wire (node_wire.rs). The painting is node_paint.rs.
use super::node_card;
use super::node_drop;
use super::node_paint::{self, Cam, Frame, Reveal};
use super::node_pick::{self, Boxing};
use super::node_wire::{self, Hand};
use super::roles::Typing;
use super::{Hit, Host};
use deck_kit::base::canvas::View;
use deck_kit::base::draw::DrawList;
use deck_kit::base::graph::{self, Direction, Graph, Metrics};
use deck_kit::base::signal::Signal;
use deck_kit::base::text::Field;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_show::canvas::{self, Edit, GROUP};
use std::collections::BTreeMap;
use winit::keyboard::{Key, NamedKey};

/// the direction, the canvas's size and the graph's extent a view was framed for
type Framed = (Direction, (f32, f32), (f32, f32));

/// a group's frame in the graph: its index in the row's canvas, its nodes
pub type Frames = [(usize, Vec<usize>)];

/// the settings' switches the view follows (settings.rs; "reduce motion" turns the first three off)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motion {
    /// what runs on the wires: pulses, streams, idle wires faint
    pub activity: bool,
    /// the latent grid under what the pointer is on
    pub reveal: bool,
    /// the border flash and shimmer of a focused node
    pub flash: bool,
    /// a node's picture of the piece it plays
    pub previews: bool,
}

impl Default for Motion {
    fn default() -> Motion {
        Motion { activity: true, reveal: true, flash: true, previews: false }
    }
}

/// a press in the node view: the Spores widget's
pub use deck_spores::Hit as NodeHit;

/// the view's state: its zoom and pan (kept while the editor runs), the direction, the drag in the hand, what is
/// selected and focused, the reveals fading
#[derive(Debug, Clone, Default)]
pub struct NodeView {
    pub view: View,
    pub direction: Direction,
    /// framed for this direction, canvas size and graph extent: a new direction frames again, so does a new size or
    /// graph (another row) until the view is zoomed or panned by hand
    framed: Option<Framed>,
    touched: bool,
    /// the canvas drawn last, on screen; whether it was drawn this frame
    pub(super) canvas: Rect,
    pub(super) shown: bool,
    bounds: Rect,
    grab: Option<(f32, f32)>,
    /// space is held: a left drag pans
    pub space: bool,
    /// shift is held (Tab goes back)
    pub shift: bool,
    /// a check run's zoom, round the canvas's middle whenever it frames itself
    pub zoom_once: Option<f32>,
    /// the selected nodes and the focused one, by id
    pub selected: Vec<String>,
    pub focus: Option<String>,
    /// the node that flashed, and when (secs)
    pub(super) flash: Option<(String, f32)>,
    /// what the pointer is on (`node:<id>`, `group:<id>`, `wire:<n>`) and the reveals fading in and out
    pub hover: Option<String>,
    reveals: Vec<Reveal>,
    /// the pointer as the latent grid follows it, slowly (canvas)
    pub(super) blob: Option<(f32, f32)>,
    /// the editor's clock, secs, and the settings' switches (main.rs sets them each frame)
    pub secs: f32,
    pub motion: Motion,
    /// the pictures of the pieces the nodes play, by key, and the keys the last frame drew
    pub pictures: BTreeMap<String, u64>,
    pub wanted: Vec<String>,
    /// the nodes drawn last: id and place on the canvas, in the graph's order; Tab's order through them; their tones
    pub(super) drawn: Vec<(String, Rect)>,
    pub(super) order: Vec<usize>,
    pub(super) tones: Vec<f32>,
    /// the wires drawn last, by link: the key a pin keeps (None: no value to pin) and the middle on screen
    pub(super) wire_keys: Vec<Option<String>>,
    pub(super) wire_mids: Vec<Option<(f32, f32)>>,
    /// where the pinned values' tags were drawn last: the link and the tag's rect on screen
    pub(super) pin_tags: Vec<(usize, Rect)>,
    /// the wires whose value stays shown, by key
    pub pinned: Vec<String>,
    /// the box being drawn
    pub boxing: Option<Boxing>,
    /// the tone asked for (Hz), until main.rs takes it
    pub(super) tone: Option<f32>,
    /// the graph drawn last, where its open nodes' pins are pressed and its wires hang, on screen
    pub(super) graph: Graph,
    pub(super) pins: Vec<(deck_show::wire_ops::At, Rect)>,
    pub(super) wires: Vec<(usize, deck_kit::base::wire::Wire)>,
    /// a wire, a cut or a node in the hand (node_wire.rs)
    pub hand: Option<Hand>,
}

/// how long a reveal fades in and out (the widget's), the flash and its shimmer, secs
pub use deck_spores::{FADE_IN, FADE_OUT};
pub const FLASH: f32 = 0.6;

impl deck_spores::Seen for NodeView {
    fn selected(&self, id: &str) -> bool {
        self.selected.iter().any(|s| s == id)
    }
    fn flashing(&self, id: &str) -> Option<f32> {
        NodeView::flashing(self, id)
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

/// a row's canvas as the Spores widget paints it (the spore contract's shape; deck-show keeps its own on the row)
fn spore_canvas(c: &canvas::Canvas) -> protocol::spore_network::Canvas {
    use protocol::spore_network as sp;
    sp::Canvas {
        groups: Some(c.groups.iter().map(|g| sp::Group { id: g.id.clone(), label: g.label.clone(), colour: Some(g.colour.clone()), spores: g.nodes.clone(), folded: Some(g.folded) }).collect()),
        notes: Some(c.notes.iter().map(|n| sp::Note { id: n.id.clone(), text: n.text.clone(), author: Some(n.author.clone()), near: n.node.clone() }).collect()),
        folded: Some(c.folded.iter().map(|(k, v)| (k.clone(), *v)).collect()),
        placed: None,
    }
}

impl NodeView {
    fn frame(&mut self) {
        self.frame_on(self.bounds);
    }

    fn frame_on(&mut self, r: Rect) {
        self.view = View::framing(r, (self.canvas.w, self.canvas.h), self.canvas.w.min(self.canvas.h) * 0.06);
    }

    fn over(&self, x: f32, y: f32) -> bool {
        self.canvas.w > 0.0 && self.canvas.has(x, y)
    }

    fn local(&self, x: f32, y: f32) -> (f32, f32) {
        (x - self.canvas.x, y - self.canvas.y)
    }

    fn id(&self, i: usize) -> Option<String> {
        self.drawn.get(i).map(|(id, _)| id.clone())
    }

    /// `id` alone selected and focused, flashing
    fn focus_on(&mut self, id: String) {
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
}

impl Host {
    /// the wheel over the canvas: a notch (or a pinch) zooms round the pointer, a touchpad's two fingers pan
    pub fn nodes_wheel(&mut self, (x, y): (f32, f32), (dx, dy): (f32, f32), fingers: bool, pinch: bool) -> bool {
        let n = &mut self.nodes;
        if !self.two_d || self.sheet.is_some() || !n.over(x, y) {
            return false;
        }
        n.touched = true;
        match fingers && !pinch {
            true => n.view.pan_by((dx * 40.0, dy * 40.0)),
            false => n.view.zoom_at(n.local(x, y), dy),
        }
        true
    }

    /// a press that pans (the middle button, or the left with space held) taken on the canvas
    pub fn nodes_grab(&mut self, x: f32, y: f32) -> bool {
        let n = &mut self.nodes;
        if !self.two_d || self.sheet.is_some() || !n.over(x, y) {
            return false;
        }
        n.grab = Some((x, y));
        n.touched = true;
        true
    }

    pub(super) fn nodes_drag(&mut self, x: f32, y: f32) {
        if let Some((gx, gy)) = self.nodes.grab {
            self.nodes.view.pan_by((x - gx, y - gy));
            self.nodes.grab = Some((x, y));
        }
        self.nodes.box_move((x, y));
        self.hand_move((x, y));
    }

    /// space over the canvas (nothing typed): a left drag pans while it is held
    pub fn nodes_space(&mut self, down: bool) -> bool {
        let (x, y) = self.cursor;
        if !down {
            return std::mem::take(&mut self.nodes.space);
        }
        let free = self.typing.is_none() && self.context.is_none() && self.flashing.is_none() && self.asking.is_none();
        self.nodes.space = free && self.two_d && self.sheet.is_none() && self.nodes.over(x, y);
        self.nodes.space
    }

    pub(super) fn nodes_release(&mut self) {
        self.nodes.grab = None;
        self.nodes.box_end();
        self.hand_release();
    }

    /// F over the canvas frames the selected nodes, or everything when none is
    pub(super) fn nodes_frame(&mut self) -> bool {
        let (x, y) = self.cursor;
        if !self.two_d || self.sheet.is_some() || !self.nodes.over(x, y) {
            return false;
        }
        let n = &mut self.nodes;
        let picked: Vec<Rect> = n.drawn.iter().filter(|(id, _)| n.selected.contains(id)).map(|(_, r)| *r).collect();
        match picked.is_empty() {
            true => n.frame(),
            false => n.frame_on(graph::bounds_of(picked)),
        }
        n.touched = true;
        true
    }

    /// The view's keys while it is drawn and nothing is typed: Tab (shift: back) moves the focus through the nodes,
    /// Escape lets the selection go; over the canvas ctrl+G groups the selection. True = taken.
    pub(super) fn nodes_key(&mut self, key: &Key, ctrl: bool) -> bool {
        if !self.nodes.shown || !self.two_d || self.sheet.is_some() || self.typing.is_some() || self.context.is_some() {
            return false;
        }
        let (x, y) = self.cursor;
        let n = &mut self.nodes;
        match key {
            Key::Named(NamedKey::Tab) if !n.order.is_empty() => {
                let len = n.order.len();
                let at = n.focus.as_ref().and_then(|f| n.order.iter().position(|&i| n.drawn.get(i).is_some_and(|(id, _)| id == f)));
                let next = match (at, n.shift) {
                    (None, false) => 0,
                    (None, true) => len - 1,
                    (Some(k), false) => (k + 1) % len,
                    (Some(k), true) => (k + len - 1) % len,
                };
                if let Some(id) = n.id(n.order[next]) {
                    n.focus_on(id);
                }
                true
            }
            Key::Named(NamedKey::Escape) if !n.selected.is_empty() => {
                n.selected.clear();
                n.focus = None;
                true
            }
            Key::Character(c) if ctrl && c.eq_ignore_ascii_case("g") && !n.selected.is_empty() && n.over(x, y) => {
                self.nodes_hit(NodeHit::Group);
                true
            }
            _ => false,
        }
    }

    pub(super) fn nodes_press(&mut self, hit: Hit) {
        match hit {
            Hit::NodeFlow(d) => self.nodes.direction = d,
            Hit::NodeFrame => self.nodes.frame(),
            Hit::Nodes(h) => self.nodes_hit(h),
            _ => {}
        }
    }

    fn nodes_hit(&mut self, h: NodeHit) {
        let c = self.shown().rows.get(self.row).map(|r| r.nodes.clone()).unwrap_or_default();
        let cursor = self.cursor;
        let n = &mut self.nodes;
        match h {
            NodeHit::Canvas => n.box_start(cursor),
            NodeHit::Node(i) => {
                if let Some(id) = n.id(i) {
                    n.hand = Some(Hand::Node { node: id.clone(), from: cursor });
                    n.focus_on(id);
                }
            }
            NodeHit::Pin(i, out, k) => self.pin_press((i, out, k)),
            NodeHit::NodeAlso(i) => {
                let Some(id) = n.id(i) else { return };
                match n.selected.iter().position(|s| *s == id) {
                    Some(k) => {
                        n.selected.remove(k);
                    }
                    None => {
                        n.selected.push(id.clone());
                        n.flash = Some((id.clone(), n.secs));
                        n.ask_tone(&id);
                        n.focus = Some(id);
                    }
                }
            }
            NodeHit::Fold(i) => {
                if let Some(node) = n.id(i) {
                    self.edit_canvas(&Edit::Fold { node });
                }
            }
            NodeHit::Group => {
                let nodes: Vec<String> = n.selected.iter().filter(|s| !s.starts_with(GROUP)).cloned().collect();
                self.edit_canvas(&Edit::Group { nodes });
            }
            NodeHit::Note => {
                let node = n.focus.clone().or_else(|| n.selected.first().cloned()).filter(|s| !s.starts_with(GROUP));
                let author = self.api.who();
                if self.edit_canvas(&Edit::Note { node, author }) {
                    let id = self.shown().rows.get(self.row).and_then(|r| r.nodes.notes.last()).map(|x| x.id.clone());
                    self.typing = id.map(|id| (Typing::Note(id), Field::with("")));
                }
            }
            NodeHit::GroupName(k) => {
                self.typing = c.groups.get(k).map(|g| (Typing::NodeGroup(g.id.clone()), Field::with(&g.label)));
            }
            NodeHit::GroupColour(k) => {
                if let Some(g) = c.groups.get(k) {
                    self.edit_canvas(&Edit::GroupColour { group: g.id.clone() });
                }
            }
            NodeHit::GroupFold(k) => {
                if let Some(g) = c.groups.get(k) {
                    self.edit_canvas(&Edit::Fold { node: format!("{GROUP}{}", g.id) });
                }
            }
            NodeHit::NoteText(k) => {
                self.typing = c.notes.get(k).map(|x| (Typing::Note(x.id.clone()), Field::with(&x.text)));
            }
        }
    }

    /// a check run: `ids` selected as shift + clicks would, then F over the canvas frames them
    pub(super) fn script_nodes(&mut self, ids: &[String], frame: bool) {
        for id in ids {
            if let Some(i) = self.nodes.drawn.iter().position(|(x, _)| x == id) {
                self.press(Hit::Nodes(NodeHit::NodeAlso(i)));
            }
        }
        if frame {
            let c = self.nodes.canvas;
            self.cursor = (c.x + c.w * 0.5, c.y + c.h * 0.5);
            self.nodes_frame();
        }
    }

    /// an edit of the row's canvas (groups, notes, folds): through the core, refused in its words. True = done.
    pub(super) fn edit_canvas(&mut self, e: &Edit) -> bool {
        let Some(row) = self.shown().rows.get(self.row).map(|r| r.id.clone()) else { return false };
        match canvas::edit(self.shown(), &row, e) {
            Ok(done) => {
                self.status = done.said;
                self.changed(done.show);
                true
            }
            Err(why) => {
                self.refuse(why);
                false
            }
        }
    }
}

/// The view in `a` (the panel's content): the head with the direction, framing, the zoom, Group and Note and the legend;
/// the canvas under it with the row's groups, wires, nodes and notes.
pub fn draw(d: &mut DrawList, t: &Tokens, host: &mut Host, (g, frames): (&Graph, &Frames), a: Rect) {
    let (pad, small) = (t.size.pad, t.size.small);
    let head = t.size.row;
    let dir = host.nodes.direction;
    let names = [Direction::Lr.word(), Direction::Tb.word()];
    let pills = controls::pills(d, t, a.x, a.y + 2.0, head - 8.0, &names, if dir == Direction::Lr { 0 } else { 1 });
    for (r, dir) in pills.iter().zip([Direction::Lr, Direction::Tb]) {
        host.hits.push((*r, Hit::NodeFlow(dir)));
    }
    let mut x = pills.last().map_or(a.x, |r| r.x + r.w) + pad;
    let mut buttons: Vec<(&str, Hit)> = vec![("Fit", Hit::NodeFrame)];
    if !host.nodes.selected.is_empty() {
        buttons.push(("Group", Hit::Nodes(NodeHit::Group)));
    }
    buttons.push(("Note", Hit::Nodes(NodeHit::Note)));
    for (word, hit) in buttons {
        let r = controls::pills(d, t, x, a.y + 2.0, head - 8.0, &[word], usize::MAX)[0];
        host.hits.push((r, hit));
        x = r.x + r.w + pad * 0.5;
    }
    x += pad * 0.5;
    x += d.text(x, a.y + (head - small * 1.25) * 0.5 - 2.0, small, &format!("{:.0} %", host.nodes.view.zoom * 100.0), t.color.muted) + 2.0 * pad;
    let mut used: Vec<Signal> = g.nodes.iter().flat_map(|n| n.ins.iter().chain(&n.outs)).map(|p| p.signal).collect();
    used.sort();
    used.dedup();
    let r = small * 0.45;
    for s in used {
        let w = d.advance(small) * s.word().chars().count() as f32;
        if x + 3.0 * r + w > a.x + a.w {
            break;
        }
        d.pin((x + r, a.y + head * 0.5 - 2.0), r, (s.shape(), (1.0, 0.0)), t.color.signals.of(s), t.color.surface);
        x += 2.0 * r + pad * 0.5;
        x += d.text(x, a.y + (head - small * 1.25) * 0.5 - 2.0, small, s.word(), t.color.muted) + pad * 1.5;
    }

    let canvas = Rect { x: a.x, y: a.y + head, w: a.w, h: (a.h - head).max(0.0) };
    let unit = t.size.rem(1.0);
    let previews = host.nodes.motion.previews;
    let metrics = Metrics { preview: if previews { 5.0 * unit } else { 0.0 }, ..Metrics::of(unit) };
    let placed = graph::layout(g, dir, &metrics);
    let doc = host.shown().rows.get(host.row).map(|r| r.nodes.clone()).unwrap_or_default();
    let spores = spore_canvas(&doc);
    let typing = host.typing.clone();
    let cursor = host.cursor;
    let n = &mut host.nodes;
    n.shown = true;
    n.canvas = canvas;
    n.bounds = graph::bounds(&placed);
    n.drawn = g.nodes.iter().zip(&placed).map(|(x, p)| (x.id.clone(), p.rect)).collect();
    n.order = graph::order(&placed, dir);
    n.tones = g.nodes.iter().map(deck_show::focus::tone).collect();
    n.wanted = g.nodes.iter().filter_map(|x| x.picture.clone()).collect();
    let size = (canvas.w, canvas.h);
    let extent = (n.bounds.w, n.bounds.h);
    let new_dir = n.framed.map(|f| f.0) != Some(dir);
    if new_dir || (n.framed != Some((dir, size, extent)) && !n.touched) {
        n.frame();
        n.framed = Some((dir, size, extent));
        n.touched &= !new_dir;
        if let Some(z) = n.zoom_once {
            n.view.zoom_to((canvas.w * 0.5, canvas.h * 0.5), z);
        }
    }
    d.rect(canvas.x, canvas.y, canvas.w, 1.0, t.color.line);
    host.hits.push((canvas, Hit::Nodes(NodeHit::Canvas)));
    let cam = Cam { at: (canvas.x, canvas.y), view: n.view, unit, dir };
    let frames: Vec<Frame> = frames.iter().filter_map(|(k, members)| node_paint::frame_of(&spores, *k, members, g, &placed, &cam)).collect();
    let wires = node_paint::wires(g, &placed, &cam);
    n.graph = g.clone();
    n.wires.clone_from(&wires);
    n.pins = node_wire::pins(g, &placed, &cam, canvas, (t.size.rem(0.64) * cam.view.zoom).max(8.0));
    n.wire_keys = (0..g.links.len()).map(|i| node_pick::wire_key(g, i)).collect();
    n.wire_mids = vec![None; g.links.len()];
    for (i, w) in &wires {
        n.wire_mids[*i] = Some(w.at(0.5));
    }
    // what the pointer is on: a node, else a wire, else a group's frame
    let over = n.over(cursor.0, cursor.1);
    let hover = over
        .then(|| {
            let node = placed.iter().zip(&g.nodes).rev().find(|(p, _)| cam.rect(p.rect).has(cursor.0, cursor.1)).map(|(_, x)| (format!("node:{}", x.id), node_paint::main_colour(t, x)));
            let wire = || wires.iter().find(|(_, w)| deck_kit::base::canvas::to_line(&w.points(), cursor) < 5.0).map(|(i, _)| (format!("wire:{i}"), t.color.signals.of(g.links[*i].signal)));
            let group = || frames.iter().find(|f| f.rect.has(cursor.0, cursor.1)).map(|f| (format!("group:{}", f.id), f.colour));
            node.or_else(wire).or_else(group)
        })
        .flatten();
    n.hover = hover.as_ref().map(|(k, _)| k.clone());
    let world = cam.view.to_world((cursor.0 - canvas.x, cursor.1 - canvas.y));
    n.blob = Some(n.blob.map_or(world, |b| (b.0 + (world.0 - b.0) * 0.12, b.1 + (world.1 - b.1) * 0.12)));
    let (on, now) = (n.motion.reveal, n.secs);    node_paint::follow(&mut n.reveals, hover.filter(|_| on), now);
    let n = host.nodes.clone();
    let (lifted, marked) = node_wire::marks(host);

    d.clip(Some(canvas));
    for f in &frames {
        node_paint::ground(d, t, f, &cam, canvas);
    }
    for r in &n.reveals {
        node_paint::reveal(d, r, (&n, &cam, canvas), (&frames, &wires, g, &placed));
    }
    let pinned = |i: usize| n.wire_keys.get(i).cloned().flatten().is_some_and(|k| n.pinned.contains(&k));
    for (i, w) in wires.iter().filter(|(i, _)| Some(*i) != lifted) {
        let l = &g.links[*i];
        node_paint::wire(d, t, (l, w), (&cam, n.motion.activity, host.flow), n.hover.as_deref() == Some(&format!("wire:{i}")) || pinned(*i) || marked.contains(i));
    }
    for (i, (node, p)) in g.nodes.iter().zip(&placed).enumerate() {
        let tint = node.id.strip_prefix(GROUP).and_then(|id| doc.group(id)).and_then(|grp| deck_kit::editor::colour::rgb(&grp.colour));
        let r = node_card::card(d, t, (node, p, tint), &cam, &n);
        host.hits.push((r.body, Hit::Nodes(NodeHit::Node(i))));
        if let Some(f) = r.fold {
            host.hits.push((f, Hit::Nodes(NodeHit::Fold(i))));
        }
    }
    for ((i, out, k), r) in &n.pins {
        host.hits.push((*r, Hit::Nodes(NodeHit::Pin(*i, *out, *k))));
    }
    for f in &frames {
        for (r, hit) in node_paint::label(d, t, f, &cam) {
            host.hits.push((r, Hit::Nodes(hit)));
        }
    }
    let editing = |id: &str| match &typing {
        Some((Typing::Note(x), field)) if x == id => Some(field.text.clone()),
        _ => None,
    };
    let notes = node_paint::notes(d, t, (&spores, &n.drawn, n.bounds), &cam, &editing);
    for (k, r) in notes {
        host.hits.push((r, Hit::Nodes(NodeHit::NoteText(k))));
    }
    let cards: Vec<Rect> = placed.iter().map(|p| cam.rect(p.rect)).collect();
    let mut tags = Vec::new();
    for (i, w) in wires.iter().filter(|(i, _)| pinned(*i)) {
        let l = &g.links[*i];
        let size = node_paint::tag_size(d, t, l.value.as_deref().unwrap_or(""));
        let r = graph::tag_spot(w, size, &cards, dir, t.size.pad * 0.5);
        node_paint::pin_tag(d, t, l, r);
        tags.push((*i, r));
    }
    host.nodes.pin_tags = tags;
    if let Some(i) = n.hover.as_deref().and_then(|h| h.strip_prefix("wire:")).and_then(|i| i.parse::<usize>().ok()).filter(|&i| !pinned(i)) {
        if let Some(l) = g.links.get(i) {
            node_paint::value_label(d, t, l, cursor);
        }
    }
    if let Some(b) = &n.boxing {
        node_pick::draw_box(d, t, b);
    }
    node_wire::draw(d, t, host, &cam, canvas);
    node_drop::draw(d, t, host, canvas);
    d.clip(None);
    if let Some((Typing::NodeGroup(id), field)) = &typing {
        if let Some(f) = frames.iter().find(|f| f.id == *id) {
            node_paint::typed_label(d, t, f, &field.text);
        }
    }
    if g.nodes.is_empty() {
        d.text(canvas.x + pad, canvas.y + pad, small, "No row selected: pick one in the rows band.", t.color.muted);
    }
}

#[cfg(test)]
mod tests {
    use super::super::author;
    use super::super::rows::tests::{host_with, H, W};
    use super::*;
    use deck_kit::base::draw::Prim;
    use deck_kit::base::tokens::Theme;

    const TOY: &str = include_str!("../../../show/fixtures/top-toy.show.json");

    fn night() -> Tokens {
        Tokens::new(Theme::Night, 16.0)
    }

    fn frame(host: &mut Host) -> DrawList {
        let mut d = DrawList::new(0.6);
        host.hits.clear();
        author::draw(&mut d, &night(), host, W, H, (0.0, 0.0));
        d
    }

    fn toy() -> Host {
        let mut host = host_with(TOY);
        host.row = host.shown().rows.iter().position(|r| r.id == "yellowmouse-enters").unwrap();
        host
    }

    /// the screen rect of the drawn node `id`
    fn node_at(host: &Host, id: &str) -> Rect {
        let i = host.nodes.drawn.iter().position(|(x, _)| x == id).unwrap();
        host.hits.iter().find(|(_, h)| *h == Hit::Nodes(NodeHit::Node(i))).map(|(r, _)| *r).unwrap()
    }

    #[test]
    fn the_rows_nodes_show_with_typed_pins_and_wires() {
        let mut host = toy();
        host.nodes.motion.activity = false;
        let d = frame(&mut host);
        let texts = d.texts();
        for want in ["reader", "clock", "on event", "LR", "TB", "event", "address", "Note"] {
            assert!(texts.contains(&want), "{want}: {texts:?}");
        }
        assert!(texts.iter().any(|x| x.starts_with("this row")), "what the row sends");
        let event = night().color.signals.event;
        assert!(d.prims().iter().any(|p| matches!(p, Prim::Poly { n: 3, color, .. } if *color == event)), "an event's triangle pin");
        let wire = d.prims().iter().filter(|p| matches!(p, Prim::Segment { color, .. } if color[..3] == event[..3])).count();
        assert!(wire >= deck_kit::base::wire::SAMPLES, "event wires drawn: {wire}");
    }

    #[test]
    fn a_pinned_value_never_covers_a_node_left_to_right_or_top_to_bottom() {
        let mut host = toy();
        frame(&mut host);
        host.nodes.pinned = host.nodes.wire_keys.iter().flatten().cloned().collect();
        assert!(!host.nodes.pinned.is_empty(), "wires with values");
        for dir in [Direction::Lr, Direction::Tb] {
            host.press(Hit::NodeFlow(dir));
            frame(&mut host);
            assert_eq!(host.nodes.direction, dir);
            assert_eq!(host.nodes.pin_tags.len(), host.nodes.pinned.len(), "{dir:?}: every pin drawn");
            for (_, tag) in &host.nodes.pin_tags {
                for id in host.nodes.drawn.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>() {
                    let card = node_at(&host, &id);
                    let over = tag.x < card.x + card.w && card.x < tag.x + tag.w && tag.y < card.y + card.h && card.y < tag.y + tag.h;
                    assert!(!over, "{dir:?}: a pin on {id}: {tag:?} {card:?}");
                }
            }
        }
    }

    #[test]
    fn the_direction_switches_and_frames_again() {
        let mut host = toy();
        frame(&mut host);
        let lr = host.nodes.bounds;
        assert!(host.hits.iter().any(|(r, h)| *h == Hit::NodeFlow(Direction::Tb) && r.w > 0.0), "the TB pill");
        host.press(Hit::NodeFlow(Direction::Tb));
        frame(&mut host);
        assert_eq!(host.nodes.direction, Direction::Tb);
        assert_ne!(host.nodes.bounds, lr, "laid out again");
    }

    #[test]
    fn the_wheel_zooms_round_the_pointer_and_a_drag_pans() {
        let mut host = toy();
        frame(&mut host);
        let c = host.nodes.canvas;
        let at = (c.x + c.w * 0.5, c.y + c.h * 0.5);
        let (z0, under) = (host.nodes.view.zoom, host.nodes.view.to_world(host.nodes.local(at.0, at.1)));
        assert!(host.nodes_wheel(at, (0.0, 2.0), false, false));
        assert!(host.nodes.view.zoom > z0);
        let back = host.nodes.view.to_screen(under);
        assert!((back.0 - (at.0 - c.x)).abs() < 1e-2 && (back.1 - (at.1 - c.y)).abs() < 1e-2, "the point stays under the pointer");
        assert!(!host.nodes_wheel((c.x - 50.0, c.y), (0.0, 1.0), false, false), "beside the canvas the wheel is not the view's");
        let pan = host.nodes.view.pan;
        assert!(host.nodes_grab(at.0, at.1));
        host.pointer(at.0 + 30.0, at.1 - 10.0);
        host.release();
        host.pointer(at.0 + 90.0, at.1);
        assert_eq!(host.nodes.view.pan, (pan.0 + 30.0, pan.1 - 10.0), "panned while held, not after");
        host.cursor = at;
        assert!(host.nodes_frame());
        assert!(host.nodes.view.zoom <= deck_kit::base::canvas::FRAME_MAX);
    }

    #[test]
    fn a_click_selects_shift_adds_tab_moves_the_focus_and_f_frames_the_selection() {
        let mut host = toy();
        frame(&mut host);
        let r = node_at(&host, "reader");
        host.press(Hit::Nodes(NodeHit::Node(host.nodes.drawn.iter().position(|(x, _)| x == "reader").unwrap())));
        assert_eq!(host.nodes.selected, ["reader"]);
        assert!(host.nodes.flashing("reader").is_some(), "the focus flashes");
        let clock = host.nodes.drawn.iter().position(|(x, _)| x == "clock").unwrap();
        host.press(Hit::Nodes(NodeHit::Node(clock)).with_shift());
        assert_eq!(host.nodes.selected, ["reader", "clock"]);
        host.cursor = (r.x + 2.0, r.y + 2.0);
        let all = host.nodes.view;
        assert!(host.nodes_frame());
        assert!(host.nodes.view.zoom >= all.zoom, "framed on the two: closer");
        // Tab goes on from the focus in reading order, shift + Tab back
        let order: Vec<String> = host.nodes.order.iter().map(|&i| host.nodes.drawn[i].0.clone()).collect();
        let at = order.iter().position(|x| x == "clock").unwrap();
        assert!(host.key(&Key::Named(NamedKey::Tab), false));
        assert_eq!(host.nodes.focus.as_deref(), Some(order[(at + 1) % order.len()].as_str()));
        assert_eq!(host.nodes.selected.len(), 1);
        host.nodes.shift = true;
        host.key(&Key::Named(NamedKey::Tab), false);
        assert_eq!(host.nodes.focus.as_deref(), Some("clock"));
        assert!(host.key(&Key::Named(NamedKey::Escape), false));
        assert!(host.nodes.selected.is_empty());
    }

    #[test]
    fn a_node_folds_by_hand_and_the_row_keeps_it() {
        let mut host = toy();
        frame(&mut host);
        let i = host.nodes.drawn.iter().position(|(x, _)| x == "reader").unwrap();
        let before = host.nodes.drawn[i].1;
        assert!(host.hits.iter().any(|(_, h)| *h == Hit::Nodes(NodeHit::Fold(i))), "a fold on the node");
        host.press(Hit::Nodes(NodeHit::Fold(i)));
        assert_eq!(host.shown().rows[host.row].nodes.folded.get("reader"), Some(&true));
        frame(&mut host);
        let after = host.nodes.drawn.iter().find(|(x, _)| x == "reader").unwrap().1;
        assert!(after.h < before.h, "a pill now");
    }

    #[test]
    fn selected_nodes_group_into_a_dotted_frame_that_folds_to_a_pill() {
        let mut host = toy();
        frame(&mut host);
        for id in ["reader", "clock"] {
            let i = host.nodes.drawn.iter().position(|(x, _)| x == id).unwrap();
            host.press(Hit::Nodes(NodeHit::Node(i)).with_shift());
        }
        let n = host.nodes.drawn.len();
        host.cursor = (host.nodes.canvas.x + 5.0, host.nodes.canvas.y + 5.0);
        assert!(host.key(&Key::Character("g".into()), true), "ctrl+G over the canvas groups the selection");
        let grp = host.shown().rows[host.row].nodes.groups[0].clone();
        assert_eq!(grp.nodes, ["reader", "clock"]);
        let d = frame(&mut host);
        let colour = deck_kit::editor::colour::rgba(deck_kit::editor::colour::rgb(&grp.colour).unwrap(), 1.0);
        let dots = d.prims().iter().filter(|p| matches!(p, Prim::Rect { color, w, .. } if color[..3] == colour[..3] && *w < 4.0)).count();
        assert!(dots > 20, "the group's ground: dots in its colour, {dots}");
        assert!(d.texts().contains(&grp.label.as_str()));
        let fold = host.hits.iter().find(|(_, h)| *h == Hit::Nodes(NodeHit::GroupFold(0))).map(|(_, h)| *h).expect("the group's fold");
        host.press(fold);
        frame(&mut host);
        assert_eq!(host.nodes.drawn.len(), n - 1, "two nodes, one pill");
        assert!(host.nodes.drawn.iter().any(|(x, _)| *x == format!("{GROUP}{}", grp.id)));
    }

    #[test]
    fn a_note_is_written_beside_the_focused_node_with_its_author() {
        let mut host = toy();
        frame(&mut host);
        let i = host.nodes.drawn.iter().position(|(x, _)| x == "row").unwrap();
        host.press(Hit::Nodes(NodeHit::Node(i)));
        host.press(Hit::Nodes(NodeHit::Note));
        assert!(matches!(&host.typing, Some((Typing::Note(_), _))), "the note opens for typing");
        for c in ["l", "o", "u", "d"] {
            host.key(&Key::Character(c.into()), false);
        }
        host.key(&Key::Named(NamedKey::Enter), false);
        let note = host.shown().rows[host.row].nodes.notes[0].clone();
        assert_eq!((note.text.as_str(), note.node.as_deref()), ("loud", Some("row")));
        let d = frame(&mut host);
        assert!(d.texts().contains(&"loud"));
        assert!(host.hits.iter().any(|(_, h)| *h == Hit::Nodes(NodeHit::NoteText(0))));
    }

    #[test]
    fn hovering_a_node_reveals_the_grid_in_its_colour() {
        let mut host = toy();
        let d = frame(&mut host);
        let event = night().color.signals.event;
        let small = |d: &DrawList| d.prims().iter().filter(|p| matches!(p, Prim::Rect { color, w, .. } if color[..3] == event[..3] && *w < 4.0)).count();
        let before = small(&d);
        let r = node_at(&host, "row");
        host.cursor = (r.x + r.w * 0.5, r.y + r.h * 0.5);
        host.nodes.secs = 10.0;
        frame(&mut host);
        assert_eq!(host.nodes.hover.as_deref(), Some("node:row"));
        host.nodes.secs = 10.0 + FADE_IN;
        let dots = small(&frame(&mut host));
        assert!(dots > before + 10, "the latent grid round the row: {dots}");
        host.nodes.motion.reveal = false;
        frame(&mut host);
        host.nodes.secs += FADE_OUT * 2.0;
        assert_eq!(small(&frame(&mut host)), before, "switched off: it fades and no reveal stays");
    }
}
