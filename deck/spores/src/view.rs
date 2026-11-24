//! The widget on screen: its head (the direction, Fit, Group and Note, the zoom, the legend of the signals shown), then
//! the canvas in two phases – `lay` (the layout, the framing, what the pointer is on, where spores, ports and threads
//! are) and `paint` (frames, reveals, threads, cards, notes, values, the box), which returns what can be pressed. A host
//! draws its own overlays (a wire in the hand, a drop) between `paint` and clearing the clip.
use crate::focus;
use crate::paint::{self, Cam, Frame};
use crate::pick;
use crate::state::{At, Frames};
use crate::{Hit, State};
use deck_kit::base::canvas::to_line;
use deck_kit::base::draw::DrawList;
use deck_kit::base::graph::{self, Direction, Graph, Metrics, Placed};
use deck_kit::base::signal::Signal;
use deck_kit::base::tokens::Tokens;
use deck_kit::base::wire::Wire;
use deck_kit::editor::colour;
use deck_kit::editor::controls::{self, Rect};
use protocol::spore_network::Canvas;

/// The head in `a` (its height is the row's): the direction, Fit, Group (with a selection) and Note, the zoom and the
/// signals the graph shows. What can be pressed.
pub fn head(d: &mut DrawList, t: &Tokens, s: &State, g: &Graph, a: Rect) -> Vec<(Rect, Hit)> {
    let (pad, small) = (t.size.pad, t.size.small);
    let head = t.size.row;
    let mut hits = Vec::new();
    let dir = s.direction;
    let names = [Direction::Lr.word(), Direction::Tb.word()];
    let pills = controls::pills(d, t, a.x, a.y + 2.0, head - 8.0, &names, if dir == Direction::Lr { 0 } else { 1 });
    for (r, dir) in pills.iter().zip([Direction::Lr, Direction::Tb]) {
        hits.push((*r, Hit::Flow(dir)));
    }
    let mut x = pills.last().map_or(a.x, |r| r.x + r.w) + pad;
    let mut buttons: Vec<(&str, Hit)> = vec![("Fit", Hit::Fit)];
    if !s.selected.is_empty() {
        buttons.push(("Group", Hit::Group));
    }
    buttons.push(("Note", Hit::Note));
    for (word, hit) in buttons {
        let r = controls::pills(d, t, x, a.y + 2.0, head - 8.0, &[word], usize::MAX)[0];
        hits.push((r, hit));
        x = r.x + r.w + pad * 0.5;
    }
    x += pad * 0.5;
    x += d.text(x, a.y + (head - small * 1.25) * 0.5 - 2.0, small, &format!("{:.0} %", s.view.zoom * 100.0), t.color.muted) + 2.0 * pad;
    let mut used: Vec<Signal> = g.nodes.iter().flat_map(|n| n.ins.iter().chain(&n.outs)).map(|p| p.signal).collect();
    used.sort();
    used.dedup();
    let r = small * 0.45;
    for sig in used {
        let w = d.advance(small) * sig.word().chars().count() as f32;
        if x + 3.0 * r + w > a.x + a.w {
            break;
        }
        d.pin((x + r, a.y + head * 0.5 - 2.0), r, (sig.shape(), (1.0, 0.0)), t.color.signals.of(sig), t.color.surface);
        x += 2.0 * r + pad * 0.5;
        x += d.text(x, a.y + (head - small * 1.25) * 0.5 - 2.0, small, sig.word(), t.color.muted) + pad * 1.5;
    }
    hits
}

/// where the ports of the open spores are pressed on screen (`reach` round each), those on the canvas only
pub fn pins(g: &Graph, placed: &[Placed], cam: &Cam, canvas: Rect, reach: f32) -> Vec<(At, Rect)> {
    let mut out = Vec::new();
    for (i, (_, p)) in g.nodes.iter().zip(placed).enumerate().filter(|(_, (n, _))| !n.collapsed) {
        for (spots, outlet) in [(&p.ins, false), (&p.outs, true)] {
            for (k, q) in spots.iter().enumerate() {
                let q = cam.s(*q);
                if canvas.has(q.0, q.1) {
                    out.push(((i, outlet, k), Rect { x: q.0 - reach, y: q.1 - reach, w: 2.0 * reach, h: 2.0 * reach }));
                }
            }
        }
    }
    out
}

/// what `lay` worked out for `paint` (and a host's overlays): the canvas on screen, where the spores are, the camera, the
/// open groups' frames, the threads on screen
pub struct Laid {
    pub canvas: Rect,
    pub placed: Vec<Placed>,
    pub cam: Cam,
    pub frames: Vec<Frame>,
    pub wires: Vec<(usize, Wire)>,
}

/// The canvas in `canvas` laid out: the graph's layout in the view's direction, framed while it was not zoomed or panned
/// by hand, the reveals following what the pointer (`cursor`) is on, and the state's record of what is where.
pub fn lay(t: &Tokens, s: &mut State, (g, frames, doc): (&Graph, &Frames, &Canvas), cursor: (f32, f32), canvas: Rect) -> Laid {
    let unit = t.size.rem(1.0);
    let dir = s.direction;
    let metrics = Metrics { preview: if s.motion.previews { 5.0 * unit } else { 0.0 }, ..Metrics::of(unit) };
    let placed = graph::layout(g, dir, &metrics);
    s.shown = true;
    s.canvas = canvas;
    s.bounds = graph::bounds(&placed);
    s.drawn = g.nodes.iter().zip(&placed).map(|(x, p)| (x.id.clone(), p.rect)).collect();
    s.order = graph::order(&placed, dir);
    s.tones = g.nodes.iter().map(focus::tone).collect();
    s.wanted = g.nodes.iter().filter_map(|x| x.picture.clone()).collect();
    let size = (canvas.w, canvas.h);
    let extent = (s.bounds.w, s.bounds.h);
    let new_dir = s.framed.map(|f| f.0) != Some(dir);
    if new_dir || (s.framed != Some((dir, size, extent)) && !s.touched) {
        s.frame();
        s.framed = Some((dir, size, extent));
        s.touched &= !new_dir;
        if let Some(z) = s.zoom_once {
            s.view.zoom_to((canvas.w * 0.5, canvas.h * 0.5), z);
        }
    }
    let cam = Cam { at: (canvas.x, canvas.y), view: s.view, unit, dir };
    let frames: Vec<Frame> = frames.iter().filter_map(|(k, members)| paint::frame_of(doc, *k, members, g, &placed, &cam)).collect();
    let wires = paint::wires(g, &placed, &cam);
    s.graph = g.clone();
    s.wires.clone_from(&wires);
    s.pins = pins(g, &placed, &cam, canvas, (t.size.rem(0.64) * cam.view.zoom).max(8.0));
    s.wire_keys = (0..g.links.len()).map(|i| pick::wire_key(g, i)).collect();
    s.wire_mids = vec![None; g.links.len()];
    for (i, w) in &wires {
        s.wire_mids[*i] = Some(w.at(0.5));
    }
    // what the pointer is on: a spore, else a thread, else a group's frame
    let hover = s
        .over(cursor.0, cursor.1)
        .then(|| {
            let node = placed.iter().zip(&g.nodes).rev().find(|(p, _)| cam.rect(p.rect).has(cursor.0, cursor.1)).map(|(_, x)| (format!("node:{}", x.id), paint::main_colour(t, x)));
            let wire = || wires.iter().find(|(_, w)| to_line(&w.points(), cursor) < 5.0).map(|(i, _)| (format!("wire:{i}"), t.color.signals.of(g.links[*i].signal)));
            let group = || frames.iter().find(|f| f.rect.has(cursor.0, cursor.1)).map(|f| (format!("group:{}", f.id), f.colour));
            node.or_else(wire).or_else(group)
        })
        .flatten();
    s.hover = hover.as_ref().map(|(k, _)| k.clone());
    let world = cam.view.to_world((cursor.0 - canvas.x, cursor.1 - canvas.y));
    s.blob = Some(s.blob.map_or(world, |b| (b.0 + (world.0 - b.0) * 0.12, b.1 + (world.1 - b.1) * 0.12)));
    let (on, now) = (s.motion.reveal, s.secs);
    paint::follow(&mut s.reveals, hover.filter(|_| on), now);
    Laid { canvas, placed, cam, frames, wires }
}

/// what the host lends a frame of `paint`: the flowing beads' clock (none: still), the thread its hand lifted (not
/// drawn) and those it marks (lit), the text of a note being typed
pub struct Lent<'a> {
    pub flow: Option<f32>,
    pub lifted: Option<usize>,
    pub marked: &'a [usize],
    pub editing: &'a dyn Fn(&str) -> Option<String>,
}

/// The canvas painted as `lay` left it: the line under the head, the groups' grounds, the reveals, the threads, the
/// cards, the groups' labels, the notes, the pinned values, a hovered thread's value, the box. The clip stays on the
/// canvas for the host's overlays (it clears it). What can be pressed, the canvas first.
pub fn paint(d: &mut DrawList, t: &Tokens, s: &mut State, (g, laid, doc): (&Graph, &Laid, &Canvas), lent: Lent<'_>, cursor: (f32, f32)) -> Vec<(Rect, Hit)> {
    let Laid { canvas, placed, cam, frames, wires } = laid;
    let canvas = *canvas;
    let mut hits = vec![(canvas, Hit::Canvas)];
    d.rect(canvas.x, canvas.y, canvas.w, 1.0, t.color.line);
    d.clip(Some(canvas));
    paint::field(d, t, cam, canvas);
    for f in frames {
        paint::ground(d, t, f, cam, canvas, s.hover.as_deref().and_then(|h| h.strip_prefix("group:")) == Some(f.id.as_str()));
    }
    for r in &s.reveals {
        paint::reveal(d, r, (&*s, cam, canvas), (frames, wires, g, placed));
    }
    let pinned = |i: usize| s.wire_keys.get(i).cloned().flatten().is_some_and(|k| s.pinned.contains(&k));
    for (i, w) in wires.iter().filter(|(i, _)| Some(*i) != lent.lifted) {
        let l = &g.links[*i];
        let lit = s.hover.as_deref() == Some(&format!("wire:{i}")) || pinned(*i) || lent.marked.contains(i);
        paint::wire(d, t, (l, w), (cam, s.motion.activity, lent.flow), lit);
    }
    let groups = doc.groups.as_deref().unwrap_or_default();
    for (i, (node, p)) in g.nodes.iter().zip(placed).enumerate() {
        let tint = node.id.strip_prefix(graph::PILL).and_then(|id| groups.iter().find(|grp| grp.id == id)).and_then(|grp| grp.colour.as_deref().and_then(colour::rgb));
        let r = crate::card::card(d, t, (node, p, tint), cam, &*s);
        hits.push((r.body, Hit::Node(i)));
        if let Some(f) = r.fold {
            hits.push((f, Hit::Fold(i)));
        }
    }
    for ((i, out, k), r) in &s.pins {
        hits.push((*r, Hit::Pin(*i, *out, *k)));
    }
    for f in frames {
        hits.extend(paint::label(d, t, f, cam));
    }
    let notes = paint::notes(d, t, (doc, &s.drawn, s.bounds), cam, lent.editing);
    hits.extend(notes.into_iter().map(|(k, r)| (r, Hit::NoteText(k))));
    let cards: Vec<Rect> = placed.iter().map(|p| cam.rect(p.rect)).collect();
    let mut tags = Vec::new();
    for (i, w) in wires.iter().filter(|(i, _)| pinned(*i)) {
        let l = &g.links[*i];
        let size = paint::tag_size(d, t, l.value.as_deref().unwrap_or(""));
        let r = graph::tag_spot(w, size, &cards, cam.dir, t.size.pad * 0.5);
        paint::pin_tag(d, t, l, r);
        tags.push((*i, r));
    }
    if let Some(i) = s.hover.as_deref().and_then(|h| h.strip_prefix("wire:")).and_then(|i| i.parse::<usize>().ok()).filter(|&i| !pinned(i)) {
        if let Some(l) = g.links.get(i) {
            paint::value_label(d, t, l, cursor);
        }
    }
    if let Some(b) = &s.boxing {
        pick::draw_box(d, t, b);
    }
    s.pin_tags = tags;
    hits
}
