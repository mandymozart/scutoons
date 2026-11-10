//! The inspector (#456; owner 2026-11-09: "the right side is the inspector ... it should show what can be set on the current
//! node"): what the selection is and what can be set on it – the rig, a track or a node. It draws and says where its fields
//! landed; the values and their ranges are the core's.
use crate::stage::Stage;
use crate::wires::Port;
use deck_kit::controls::{self, Rect};
use deck_kit::draw::DrawList;
use deck_kit::tokens::Tokens;

/// what is selected
#[derive(Debug, Clone, PartialEq)]
pub enum Sel {
    Rig,
    Track(String),
    /// a node of the graph, by index
    Node(usize),
}

/// a field you can drag
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Field {
    /// one of the rig's knobs
    Knob(usize),
    /// an input of a node
    Param(usize, usize),
    /// a far world node's address (by its graph node): a click opens the address book (#477)
    Address(usize),
}

fn knobs(d: &mut DrawList, t: &Tokens, stage: &Stage, (area, y): (Rect, &mut f32), track: Option<&str>, hot: Option<Field>, out: &mut Vec<(Rect, Field)>) {
    let prefix = track.map(|t| format!("{t}_"));
    let shown: Vec<usize> = (0..stage.knobs.len()).filter(|&i| prefix.as_ref().is_none_or(|p| stage.knobs[i].path.starts_with(p.as_str()))).collect();
    if shown.is_empty() {
        return;
    }
    *y += controls::section(d, t, area.x, *y, area.w, "KNOBS", false);
    for i in shown {
        if *y + t.size.row > area.y + area.h {
            break;
        }
        let k = &stage.knobs[i];
        let v = stage.value(k);
        let f = crate::fraction(v, k.min, k.max, k.log);
        let bar = controls::scrub(d, t, Rect { x: area.x, y: *y, w: area.w, h: t.size.row }, &k.label, &crate::number(v), f, hot == Some(Field::Knob(i)));
        out.push((bar, Field::Knob(i)));
        *y += t.size.row;
    }
}

fn port_line(d: &mut DrawList, t: &Tokens, area: Rect, y: &mut f32, p: &Port) {
    let c = if p.input { t.color.text } else { t.color.accent };
    d.disc(area.x + 5.0, *y + t.size.tree_row * 0.5, 4.0, c);
    d.text(area.x + 16.0, *y + 5.0, t.size.font, &p.name, t.color.text);
    let other = p.cables.last().map_or(p.ty.to_string(), |c| format!("< {}", c.from));
    let w = d.advance(t.size.small) * other.chars().count() as f32;
    d.text(area.x + area.w - w, *y + 6.0, t.size.small, &other, t.color.muted);
    *y += t.size.tree_row;
}

/// draw it inside `area`; where its fields landed
pub fn draw(d: &mut DrawList, t: &Tokens, area: Rect, stage: &Stage, sel: &Sel, hot: Option<Field>) -> Vec<(Rect, Field)> {
    let mut out = Vec::new();
    let mut y = area.y;
    let row = t.size.row;
    match sel {
        Sel::Rig => {
            y += controls::section(d, t, area.x, y, area.w, "RIG", true);
            controls::row(d, t, area, &mut y, "subject", &stage.bundle.rig.subject);
            controls::row(d, t, area, &mut y, "tracks", &stage.bundle.rig.tracks.len().to_string());
            let moods = stage.bundle.rig.moods.keys().cloned().collect::<Vec<_>>().join(" ");
            if !moods.is_empty() {
                controls::row(d, t, area, &mut y, "moods", &moods);
            }
            knobs(d, t, stage, (area, &mut y), None, hot, &mut out);
        }
        Sel::Track(id) => {
            y += controls::section(d, t, area.x, y, area.w, "TRACK", true);
            if let Some(track) = stage.bundle.rig.tracks.iter().find(|x| &x.id == id) {
                controls::row(d, t, area, &mut y, "name", &track.id);
                controls::row(d, t, area, &mut y, "instrument", &track.instrument);
                controls::row(d, t, area, &mut y, "preset", track.preset.as_deref().unwrap_or("–"));
                controls::row(d, t, area, &mut y, "gain", &crate::number(track.gain));
                controls::row(d, t, area, &mut y, "pan", &crate::number(track.pan));
                controls::row(d, t, area, &mut y, "mute", if track.mute { "on" } else { "off" });
            }
            knobs(d, t, stage, (area, &mut y), Some(id), hot, &mut out);
        }
        Sel::Node(node) => {
            y += controls::section(d, t, area.x, y, area.w, "NODE", true);
            let id = stage.node_ids.get(*node).cloned().unwrap_or_default();
            controls::row(d, t, area, &mut y, "name", &id);
            controls::row(d, t, area, &mut y, "kind", stage.kind_name(*node));
            if let Some((_, _, address)) = stage.far(*node) {
                let top = y;
                let room = ((area.w - (area.w * 0.42).min(96.0)) / d.advance(t.size.font)).max(0.0) as usize;
                controls::row(d, t, area, &mut y, "address", &controls::fit(&address.to_string(), room));
                out.push((Rect { x: area.x, y: top, w: area.w, h: y - top }, Field::Address(*node)));
            }
            let params = stage.params(*node);
            if !params.is_empty() {
                y += controls::section(d, t, area.x, y, area.w, "PARAMETERS", false);
            }
            for p in params {
                if y + row > area.y + area.h {
                    break;
                }
                let value = match &p.linked {
                    Some(from) => format!("< {from}"),
                    None => crate::number(p.value),
                };
                let f = if p.linked.is_some() { 0.0 } else { p.spec.norm(p.value) };
                let field = Field::Param(*node, p.port);
                let bar = controls::scrub(d, t, Rect { x: area.x, y, w: area.w, h: row }, p.name, &value, f, hot == Some(field));
                if p.linked.is_none() {
                    out.push((bar, field));
                }
                y += row;
            }
            let ports: Vec<&Port> = stage.ports.iter().filter(|q| q.node == id).collect();
            if !ports.is_empty() {
                y += controls::section(d, t, area.x, y, area.w, "PORTS", false);
                for p in ports {
                    if y + t.size.tree_row > area.y + area.h {
                        break;
                    }
                    port_line(d, t, area, &mut y, p);
                }
            }
        }
    }
    out
}
