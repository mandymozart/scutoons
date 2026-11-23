//! The Show workspace's node view: the selected row as deck-show's graph (nodes.rs there), laid out left → right or top →
//! bottom (the workspace's setting) by deck-kit's graph, its pins and wires typed by deck-kit's signal table. Zoom and
//! pan as the stage: the wheel zooms round the pointer, a middle drag or space + drag pans, F frames the selection (or
//! everything). A click selects a node (shift adds), Tab moves the focus as a browser does; selected nodes group
//! (ctrl+G or Group), a note joins the focused node; groups, notes and folds are the row's (deck-show canvas.rs). A drag
//! on the empty canvas selects by a box, a click on a wire pins its value (node_pick.rs). Pins are wired by dragging,
//! wires cut, a node dropped on a wire (node_wire.rs). The painting is node_paint.rs.
use super::node_drop;
use super::node_paint;
use super::node_wire::{self, Hand};
use super::roles::Typing;
use super::{Hit, Host};
use deck_kit::base::draw::DrawList;
use deck_kit::base::graph::Graph;
use deck_kit::base::text::Field;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::Rect;
use deck_show::canvas::{self, Edit, GROUP};
use deck_spores::input::{Event, Gesture, Key as SporeKey};
use winit::keyboard::{Key, NamedKey};

/// the settings' switches, the frames of the groups: the Spores widget's
pub use deck_spores::state::{Frames, Motion};

/// a press in the node view: the Spores widget's
pub use deck_spores::Hit as NodeHit;

/// the view's state: the Spores widget's (zoom and pan, selection, focus, what was drawn), and what the show's hand
/// holds – a wire, a cut or a node (node_wire.rs)
#[derive(Debug, Clone, Default)]
pub struct NodeView {
    pub state: deck_spores::State,
    pub hand: Option<Hand>,
}

impl std::ops::Deref for NodeView {
    type Target = deck_spores::State;
    fn deref(&self) -> &deck_spores::State {
        &self.state
    }
}

impl std::ops::DerefMut for NodeView {
    fn deref_mut(&mut self) -> &mut deck_spores::State {
        &mut self.state
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


impl Host {
    /// the view may take input: drawn in 2D, no sheet over it
    fn nodes_free(&self) -> bool {
        self.two_d && self.sheet.is_none()
    }

    /// the wheel over the canvas: a notch (or a pinch) zooms round the pointer, a touchpad's two fingers pan
    pub fn nodes_wheel(&mut self, at: (f32, f32), delta: (f32, f32), fingers: bool, pinch: bool) -> bool {
        self.nodes_free() && self.nodes.handle(Event::Wheel { at, delta, fingers, pinch }).0
    }

    /// a press that pans (the middle button, or the left with space held) taken on the canvas
    pub fn nodes_grab(&mut self, x: f32, y: f32) -> bool {
        self.nodes_free() && self.nodes.handle(Event::Grab { at: (x, y) }).0
    }

    pub(super) fn nodes_drag(&mut self, x: f32, y: f32) {
        self.nodes.handle(Event::Drag { at: (x, y) });
        self.hand_move((x, y));
    }

    /// space over the canvas (nothing typed): a left drag pans while it is held
    pub fn nodes_space(&mut self, down: bool) -> bool {
        let (x, y) = self.cursor;
        if !down {
            return std::mem::take(&mut self.nodes.space);
        }
        let free = self.typing.is_none() && self.context.is_none() && self.flashing.is_none() && self.asking.is_none();
        let take = free && self.nodes_free() && self.nodes.over(x, y);
        self.nodes.handle(Event::Space(take)).0
    }

    pub(super) fn nodes_release(&mut self) {
        self.nodes.handle(Event::Release);
        self.hand_release();
    }

    /// F over the canvas frames the selected nodes, or everything when none is
    pub(super) fn nodes_frame(&mut self) -> bool {
        self.nodes_free() && self.nodes.handle(Event::Key { key: SporeKey::Frame, at: self.cursor }).0
    }

    /// The view's keys while it is drawn and nothing is typed: Tab (shift: back) moves the focus through the nodes,
    /// Escape lets the selection go; over the canvas ctrl+G groups the selection. True = taken.
    pub(super) fn nodes_key(&mut self, key: &Key, ctrl: bool) -> bool {
        if !self.nodes.shown || !self.nodes_free() || self.typing.is_some() || self.context.is_some() {
            return false;
        }
        let key = match key {
            Key::Named(NamedKey::Tab) => SporeKey::Tab,
            Key::Named(NamedKey::Escape) => SporeKey::Escape,
            Key::Character(c) if ctrl && c.eq_ignore_ascii_case("g") => SporeKey::Group,
            _ => return false,
        };
        let (took, gesture) = self.nodes.handle(Event::Key { key, at: self.cursor });
        if let Some(g) = gesture {
            self.nodes_gesture(g);
        }
        took
    }

    pub(super) fn nodes_press(&mut self, hit: Hit) {
        if let Hit::Nodes(h) = hit {
            self.nodes_hit(h);
        }
    }

    fn nodes_hit(&mut self, h: NodeHit) {
        let (_, gesture) = self.nodes.handle(Event::Press { hit: h, at: self.cursor });
        if let Some(g) = gesture {
            self.nodes_gesture(g);
        }
    }

    /// what the widget hands back, done to the row: its canvas through the core, the hand, the typing
    fn nodes_gesture(&mut self, g: Gesture) {
        let c = self.shown().rows.get(self.row).map(|r| r.nodes.clone()).unwrap_or_default();
        match g {
            Gesture::TakeSpore { id, from } => self.nodes.hand = Some(Hand::Node { node: id, from }),
            Gesture::TakePin(at) => self.pin_press(at),
            Gesture::Fold(node) => {
                self.edit_canvas(&Edit::Fold { node });
            }
            Gesture::Group(nodes) => {
                self.edit_canvas(&Edit::Group { nodes });
            }
            Gesture::Note(node) => {
                let author = self.api.who();
                if self.edit_canvas(&Edit::Note { node, author }) {
                    let id = self.shown().rows.get(self.row).and_then(|r| r.nodes.notes.last()).map(|x| x.id.clone());
                    self.typing = id.map(|id| (Typing::Note(id), Field::with("")));
                }
            }
            Gesture::RenameGroup(k) => {
                self.typing = c.groups.get(k).map(|g| (Typing::NodeGroup(g.id.clone()), Field::with(&g.label)));
            }
            Gesture::RecolourGroup(k) => {
                if let Some(g) = c.groups.get(k) {
                    self.edit_canvas(&Edit::GroupColour { group: g.id.clone() });
                }
            }
            Gesture::FoldGroup(k) => {
                if let Some(g) = c.groups.get(k) {
                    self.edit_canvas(&Edit::Fold { node: format!("{GROUP}{}", g.id) });
                }
            }
            Gesture::EditNote(k) => {
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

/// The view in `a` (the panel's content): the Spores widget's head and canvas (deck-spores view.rs) over the row's
/// graph and canvas, the show's hand and drops over it, a group's label while it is typed.
pub fn draw(d: &mut DrawList, t: &Tokens, host: &mut Host, (g, frames): (&Graph, &Frames), a: Rect) {
    let (pad, small) = (t.size.pad, t.size.small);
    let head = t.size.row;
    for (r, hit) in deck_spores::view::head(d, t, &host.nodes, g, a) {
        host.hits.push((r, Hit::Nodes(hit)));
    }
    let canvas = Rect { x: a.x, y: a.y + head, w: a.w, h: (a.h - head).max(0.0) };
    let doc = host.shown().rows.get(host.row).map(|r| r.nodes.clone()).unwrap_or_default();
    let spores = spore_canvas(&doc);
    let (typing, cursor, flow) = (host.typing.clone(), host.cursor, host.flow);
    let laid = deck_spores::view::lay(t, &mut host.nodes, (g, frames, &spores), cursor, canvas);
    let (lifted, marked) = node_wire::marks(host);
    let editing = |id: &str| match &typing {
        Some((Typing::Note(x), field)) if x == id => Some(field.text.clone()),
        _ => None,
    };
    let lent = deck_spores::view::Lent { flow, lifted, marked: &marked, editing: &editing };
    let hits = deck_spores::view::paint(d, t, &mut host.nodes, (g, &laid, &spores), lent, cursor);
    host.hits.extend(hits.into_iter().map(|(r, h)| (r, Hit::Nodes(h))));
    node_wire::draw(d, t, host, &laid.cam, canvas);
    node_drop::draw(d, t, host, canvas);
    d.clip(None);
    if let Some((Typing::NodeGroup(id), field)) = &typing {
        if let Some(f) = laid.frames.iter().find(|f| f.id == *id) {
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
    use deck_kit::base::graph::Direction;
    use deck_spores::{FADE_IN, FADE_OUT};
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
            host.press(Hit::Nodes(NodeHit::Flow(dir)));
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
        assert!(host.hits.iter().any(|(r, h)| *h == Hit::Nodes(NodeHit::Flow(Direction::Tb)) && r.w > 0.0), "the TB pill");
        host.press(Hit::Nodes(NodeHit::Flow(Direction::Tb)));
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
