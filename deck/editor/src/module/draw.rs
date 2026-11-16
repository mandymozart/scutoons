//! The Module workspace, floating panels over the stage: the catalog's modules, the open module – its working copy, Open
//! in editor · Build · Save draft, what happened last, its Parameters –, the last build's Problems, the info.
use super::{fraction, problems, shown, Hit, Module};
use crate::stage::Stage;
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_kit::editor::info::Help;

fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

pub fn draw(d: &mut DrawList, t: &Tokens, m: &mut Module, stage: &Stage, (w, h): (f32, f32), pointer: (f32, f32), help: Option<&Help>) {
    let (pad, font, small) = (t.size.pad, t.size.font, t.size.small);
    m.hits.clear();
    let problems_h = t.size.tree_row * (1.0 + m.problems.len().max(1) as f32);
    let shown_panels = [("modules", t.size.tree, None), ("module", 360.0, None), ("problems", 640.0, Some(problems_h)), ("info", 260.0, Some(small * 1.4 * 6.0))];
    m.panels.layout(d, t, ((w, h), crate::bar::bottom(t)), &[], &shown_panels);

    // the catalog's modules
    if let Some(o) = m.panels.open(d, t, "modules", pointer, m.hits.len()) {
        let open_id = m.open.as_ref().map(|o| o.id.clone());
        let rows: Vec<controls::TreeRow> = m
            .list
            .iter()
            .map(|e| controls::TreeRow { depth: 0, label: e.id.clone(), detail: if e.factory { "factory".into() } else if e.draft { "draft".into() } else { String::new() }, icon: "sliders", open: None, selected: open_id.as_deref() == Some(e.id.as_str()) })
            .collect();
        if rows.is_empty() {
            d.text(o.area.x, o.area.y + 4.0, small, "no module listed yet", t.color.muted);
        }
        for (i, r) in controls::tree(d, t, o.area.x, o.area.y, o.area.w, &rows, pointer).into_iter().enumerate() {
            m.hits.push((r, Hit::Open(i)));
        }
        m.panels.close(d, t, o, &mut m.hits);
    }

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
                d.rect(r.x, r.y, r.w, r.h, t.color.hover);
            }
            let ty = by + (line_h - font * 1.25) * 0.5;
            d.disc(a.x + 4.0, ty + font * 0.62, 3.5, if p.error { t.color.danger } else { t.color.warn });
            let place_w = d.text(a.x + 14.0, ty, small, &p.place, t.color.muted) + 10.0;
            let text = fit(d, &p.text, font, a.w - place_w - 2.0 * pad);
            d.text(a.x + 14.0 + place_w, ty, font, &text, t.color.text);
            m.hits.push((r, Hit::Problem(i)));
            by += line_h;
        }
        m.panels.close(d, t, o, &mut m.hits);
    }

    // the module open
    if let Some(o) = m.panels.open(d, t, "module", pointer, m.hits.len()) {
        open_module(d, t, m, stage, o.area, pointer);
        m.panels.close(d, t, o, &mut m.hits);
    }

    if let Some(o) = m.panels.open(d, t, "info", pointer, m.hits.len()) {
        deck_kit::editor::info::body(d, t, (o.area.x, o.area.y, o.area.w), help);
        m.panels.close(d, t, o, &mut m.hits);
    }
}

fn open_module(d: &mut DrawList, t: &Tokens, m: &mut Module, stage: &Stage, a: Rect, _pointer: (f32, f32)) {
    let (pad, font, small, row) = (t.size.pad, t.size.font, t.size.small, t.size.row);
    let (x0, pw) = (a.x, a.w);
    let mut y = a.y;
    let Some(open) = m.open.clone() else {
        d.text(x0, y + 4.0, font, "Pick a module in Modules.", t.color.muted);
        status(d, t, m, x0, y + row, pw);
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
    let names = ["Open in editor", "Build", "Save draft"];
    for (r, hit) in controls::pills(d, t, x0, y, row - 4.0, &names, usize::MAX).into_iter().zip([Hit::Code, Hit::Build, Hit::Save]) {
        m.hits.push((r, hit));
    }
    y += row + pad * 0.5;
    y = status(d, t, m, x0, y, pw);

    // its Parameters, from the descriptor
    y += controls::section(d, t, x0, y, pw, if m.heard { "PARAMETERS · heard on the stage" } else { "PARAMETERS · defaults" }, false);
    let Some(desc) = m.descriptor.clone() else {
        d.text(x0, y, small, "No descriptor yet: build it.", t.color.muted);
        return;
    };
    let hot = m.drag.map(|(i, ..)| i);
    for (i, p) in desc.params.iter().enumerate() {
        let v = m.value(i, stage);
        let field = controls::scrub(d, t, Rect { x: x0, y, w: pw, h: row }, &p.name, &shown(p, v), fraction(p, v), hot == Some(i));
        m.hits.push((field, Hit::Param(i)));
        y += row;
    }
}

/// the last thing that happened, wrapped; the y under it
fn status(d: &mut DrawList, t: &Tokens, m: &Module, x: f32, mut y: f32, w: f32) -> f32 {
    let small = t.size.small;
    let cols = (w / d.advance(small)).max(8.0) as usize;
    for line in deck_kit::editor::info::wrap(&m.status, cols).into_iter().take(4) {
        d.text(x, y, small, &line, if m.bad { t.color.danger } else { t.color.text });
        y += small * 1.5;
    }
    y + t.size.pad
}
