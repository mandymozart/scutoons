//! The Module workspace, floating panels over the stage: the open module – its working copy, Open in editor · Build, what
//! happened last, its Parameters –, the last build's Problems, the info; the catalog's modules are the library's (main.rs).
use super::{fraction, problems, shown, Hit, Module};
use crate::stage::Stage;
use deck_core::units::{self, Refs};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_kit::editor::info::Help;
use deck_kit::editor::units::{self as unit_field, Prefs};

fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

pub fn draw(d: &mut DrawList, t: &Tokens, m: &mut Module, (stage, prefs): (&Stage, &Prefs), (w, h): (f32, f32), pointer: (f32, f32), help: Option<&Help>) {
    let (pad, font, small) = (t.size.pad, t.size.font, t.size.small);
    m.hits.clear();
    let problems_h = t.size.tree_row * (1.0 + m.problems.len().max(1) as f32);
    let drives_h = t.size.row * 1.4 * m.lamps.traces.len().max(1) as f32 + t.size.row;
    let shown_panels = [(crate::library::PANEL, crate::library::WIDTH, None), ("module", 360.0, None), ("problems", 640.0, Some(problems_h)), ("info", 260.0, Some(small * 1.4 * 6.0)), ("drives", 420.0, Some(drives_h))];
    m.panels.layout(d, t, ((w, h), crate::bar::bottom(t)), &[], &shown_panels);

    // the last build's problems, a click opens the code there
    if let Some(o) = m.panels.open(d, t, "problems", pointer, m.hits.len()) {
        let a = o.area;
        let mut by = a.y;
        d.text(a.x, by + 4.0, small, &format!("PROBLEMS · {}", problems::summary(&m.problems)), t.color.muted);
        by += t.size.tree_row;
        let line_h = t.size.tree_row;
        for (i, p) in m.problems.iter().enumerate() {
            let r = Rect { x: a.x, y: by, w: a.w, h: line_h };
            if r.has(pointer.0, pointer.1) {
                controls::list_row(d, t, r, "hover");
            }
            let ty = by + (line_h - font * 1.25) * 0.5;
            d.disc(a.x + 4.0, ty + font * 0.62, 3.5, if p.error { t.color.danger } else { t.color.warn });
            let place_w = d.text(a.x + 14.0, ty, small, &p.place, t.color.muted) + 10.0;
            let text = fit(d, &p.text, font, a.w - place_w - 2.0 * pad);
            d.text(a.x + 14.0 + place_w, ty, font, &text, t.color.text);
            m.hits.push((r, Hit::Problem(i)));
            by += line_h;
        }
        m.panels.close(d, o, &mut m.hits);
    }

    // the module open
    if let Some(o) = m.panels.open(d, t, "module", pointer, m.hits.len()) {
        open_module(d, t, m, (stage, prefs), o.area, pointer);
        m.panels.close(d, o, &mut m.hits);
    }

    // what it drives: its control outputs as lamps, their values over time
    let node = m.heard.then(|| stage.node_ids.iter().position(|n| n == super::NODE)).flatten();
    m.lamps.read(stage, node);
    if let Some(o) = m.panels.open(d, t, "drives", pointer, m.hits.len()) {
        super::lamps::draw(d, t, &m.lamps, m.heard, o.area);
        m.panels.close(d, o, &mut m.hits);
    }

    if let Some(o) = m.panels.open(d, t, "info", pointer, m.hits.len()) {
        deck_kit::editor::info::body(d, t, (o.area.x, o.area.y, o.area.w), help);
        m.panels.close(d, o, &mut m.hits);
    }
}

/// what the module panel says before one is open
pub const PICK: &str = "Pick a module in the Library.";

fn open_module(d: &mut DrawList, t: &Tokens, m: &mut Module, (stage, prefs): (&Stage, &Prefs), a: Rect, _pointer: (f32, f32)) {
    let (pad, font, small, row) = (t.size.pad, t.size.font, t.size.small, t.size.row);
    let (x0, pw) = (a.x, a.w);
    let mut y = a.y;
    let Some(open) = m.open.clone() else {
        d.text(x0, y + 4.0, font, PICK, t.color.muted);
        return;
    };
    d.text(x0, y + 2.0, font, &open.key(), t.color.text);
    y += row * 0.9;
    d.text(x0, y, small, &fit(d, &open.dir.display().to_string(), small, pw), t.color.muted);
    y += row * 0.8;
    if open.factory {
        d.text(x0, y, small, &fit(d, &format!("factory, read-only: saves as module:{}", open.save_as), small, pw), t.color.muted);
        y += row * 0.8;
    }
    let names = ["Open in editor", "Build"];
    for (r, hit) in controls::pills(d, t, x0, y, row - 4.0, &names, usize::MAX).into_iter().zip([Hit::Code, Hit::Build]) {
        m.hits.push((r, hit));
    }
    y += row + pad * 0.5;
    crate::inspector::background(d, t, Rect { x: x0, y, w: pw, h: row }, &mut y, &super::background_row(m.descriptor.as_ref()));
    let slot = m.descriptor.as_ref().and_then(|d| d.slot.as_deref()).map_or_else(|| "none · a rig's track".to_string(), |s| format!("{s} · drop it on that slot"));
    controls::row(d, t, Rect { x: x0, y, w: pw, h: row }, &mut y, "slot", &slot);

    // its Parameters, from the descriptor
    y += controls::section(d, t, x0, y, pw, if m.heard { "PARAMETERS · heard on the stage" } else { "PARAMETERS · defaults" }, false);
    let Some(desc) = m.descriptor.clone() else {
        d.text(x0, y, small, "No descriptor yet: build it.", t.color.muted);
        return;
    };
    let hot = m.drag.map(|(i, ..)| i);
    let none = Refs::default();
    for (i, p) in desc.params.iter().enumerate() {
        let v = m.value(i, stage);
        let at = Rect { x: x0, y, w: pw, h: row };
        if let Some((kind, stored, offered)) = super::switchable(p) {
            let unit = prefs.unit(kind, stored, &none);
            let read = units::convert(f64::from(v), stored, unit, &none).unwrap_or(f64::from(v));
            let (field, segs) = unit_field::scrub(d, t, at, &p.name, read, unit, offered, fraction(p, v), hot == Some(i));
            m.hits.push((field, Hit::Param(i)));
            m.hits.extend(segs.into_iter().map(|(r, u)| (r, Hit::Unit(kind, u))));
        } else {
            let field = controls::scrub(d, t, at, &p.name, &shown(p, v), fraction(p, v), hot == Some(i));
            m.hits.push((field, Hit::Param(i)));
        }
        y += row;
    }
}

