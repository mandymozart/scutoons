//! The Module workspace laid out: the catalog's modules on the left, the stage with the Problems band under it, the open
//! module on the right – its working copy, Open in editor · Build · Save draft, what happened last, its Parameters.
use super::{fraction, problems, shown, Hit, Module};
use crate::stage::Stage;
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};

pub struct Layout {
    pub left: Rect,
    pub stage: Rect,
    pub band: Rect,
    pub right: Rect,
}

pub fn layout(w: f32, h: f32, t: &Tokens) -> Layout {
    let top = t.size.top;
    let left_w = t.size.tree.min(w * 0.22);
    let right_w = (w * 0.26).clamp(260.0, 440.0);
    let cw = (w - left_w - right_w).max(1.0);
    let body = (h - top).max(1.0);
    let band_h = (body * 0.3).clamp(110.0, 280.0);
    Layout {
        left: Rect { x: 0.0, y: top, w: left_w, h: body },
        stage: Rect { x: left_w, y: top, w: cw, h: (body - band_h).max(1.0) },
        band: Rect { x: left_w, y: h - band_h, w: cw, h: band_h },
        right: Rect { x: w - right_w, y: top, w: right_w, h: body },
    }
}

fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

pub fn draw(d: &mut DrawList, t: &Tokens, m: &mut Module, stage: &Stage, w: f32, h: f32, pointer: (f32, f32)) {
    let l = layout(w, h, t);
    let (pad, font, small, row) = (t.size.pad, t.size.font, t.size.small, t.size.row);
    m.hits.clear();
    for (r, line_x) in [(l.left, l.left.x + l.left.w - 1.0), (l.right, l.right.x)] {
        d.rect(r.x, r.y, r.w, r.h, t.color.surface);
        d.rect(line_x, r.y, 1.0, r.h, t.color.line);
    }
    d.rect(l.band.x, l.band.y, l.band.w, l.band.h, t.color.surface_head);
    d.rect(l.band.x, l.band.y, l.band.w, 1.0, t.color.line);

    // left: the catalog's modules
    let mut y = l.left.y + pad;
    d.text(pad, y + 4.0, small, "MODULES", t.color.muted);
    y += t.size.tree_row;
    let open_id = m.open.as_ref().map(|o| o.id.clone());
    let rows: Vec<controls::TreeRow> = m
        .list
        .iter()
        .map(|e| controls::TreeRow { depth: 0, label: e.id.clone(), detail: if e.factory { "factory".into() } else if e.draft { "draft".into() } else { String::new() }, icon: "sliders", open: None, selected: open_id.as_deref() == Some(e.id.as_str()) })
        .collect();
    for (i, r) in controls::tree(d, t, 0.0, y, l.left.w - 1.0, &rows, pointer).into_iter().enumerate() {
        m.hits.push((r, Hit::Open(i)));
    }

    // the band: the last build's problems, a click opens the code there
    let bx = l.band.x + pad;
    let mut by = l.band.y + pad * 0.5;
    d.text(bx, by + 4.0, small, &format!("PROBLEMS · {}", problems::summary(&m.problems)), t.color.muted);
    by += t.size.tree_row;
    let line_h = t.size.tree_row;
    let shown_rows = ((l.band.y + l.band.h - by) / line_h).max(0.0) as usize;
    for (i, p) in m.problems.iter().enumerate().take(shown_rows) {
        let r = Rect { x: l.band.x, y: by, w: l.band.w, h: line_h };
        if r.has(pointer.0, pointer.1) {
            d.rect(r.x, r.y, r.w, r.h, t.color.hover);
        }
        let ty = by + (line_h - font * 1.25) * 0.5;
        d.disc(bx + 4.0, ty + font * 0.62, 3.5, if p.error { t.color.danger } else { t.color.warn });
        let place_w = d.text(bx + 14.0, ty, small, &p.place, t.color.muted) + 10.0;
        let text = fit(d, &p.text, font, l.band.w - place_w - 3.0 * pad);
        d.text(bx + 14.0 + place_w, ty, font, &text, t.color.text);
        m.hits.push((r, Hit::Problem(i)));
        by += line_h;
    }

    // right: the module open
    let (x0, pw) = (l.right.x + pad, l.right.w - 2.0 * pad);
    let mut y = l.right.y + pad;
    let Some(open) = m.open.clone() else {
        d.text(x0, y + 4.0, font, "Pick a module on the left.", t.color.muted);
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
        if y + row > l.right.y + l.right.h {
            break;
        }
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
