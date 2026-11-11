//! The Show workspace's sheets over the stage (#91 item 6): the cue sheet (every row in order, its trigger, what each
//! column does) and the problems list (the document's errors, then what the decks lack after the last flash; a click
//! selects the row, cell or seat). The words and what a problem points at are deck-show's view.rs; this file lays them out.
use super::{Hit, Host, Sheet};
use deck_kit::controls::{self, Rect};
use deck_kit::draw::DrawList;
use deck_kit::tokens::Tokens;
use deck_show::view;

fn fit(d: &DrawList, text: &str, px: f32, w: f32) -> String {
    controls::fit(text, (w / d.advance(px)).max(0.0) as usize)
}

pub fn draw(d: &mut DrawList, t: &Tokens, host: &mut Host, area: Rect, pointer: (f32, f32)) {
    d.rect(area.x, area.y, area.w, area.h, t.color.surface_head);
    match host.sheet {
        Some(Sheet::Cues) => cues(d, t, host, area, pointer),
        Some(Sheet::Problems) => problems(d, t, host, area, pointer),
        Some(Sheet::Flash) => super::flash::draw(d, t, host, area, pointer),
        Some(Sheet::Who) => super::roles::who(d, t, host, area, pointer),
        None => {}
    }
}

fn cues(d: &mut DrawList, t: &Tokens, host: &mut Host, a: Rect, pointer: (f32, f32)) {
    let (pad, font, small) = (t.size.pad, t.size.font, t.size.small);
    let show = host.shown().clone();
    let sheet = view::cue_sheet(&show);
    let lh = font * 1.5;
    let (xn, xr) = (a.x + pad, a.x + pad + d.advance(font) * 3.0);
    let xt = xr + (a.w * 0.18).max(100.0);
    let xw = xt + (a.w * 0.36).max(140.0);
    let mut y = a.y + pad;
    d.text(xn, y, small, "#", t.color.muted);
    d.text(xr, y, small, "ROW", t.color.muted);
    d.text(xt, y, small, "TRIGGER", t.color.muted);
    d.text(xw, y, small, "WHAT HAPPENS", t.color.muted);
    y += small * 1.8;
    d.rect(a.x, y - 4.0, a.w, 1.0, t.color.line);
    for (i, c) in sheet.iter().enumerate() {
        let lines: Vec<String> = c.does.iter().map(|(col, w)| format!("{col}: {w}")).chain(c.sends.iter().map(|s| format!("sends {s}"))).collect();
        let h = lines.len().max(1) as f32 * lh + pad * 0.5;
        if y + h > a.y + a.h {
            d.text(xr, y + 2.0, small, &format!("{} more rows …", sheet.len() - i), t.color.muted);
            break;
        }
        let whole = Rect { x: a.x, y, w: a.w, h };
        if i == host.row {
            d.rect(whole.x, whole.y, whole.w, whole.h, t.color.selected);
        } else if whole.has(pointer.0, pointer.1) {
            d.rect(whole.x, whole.y, whole.w, whole.h, t.color.hover);
        }
        d.text(xn, y + 4.0, font, &c.number.to_string(), t.color.muted);
        d.text(xr, y + 4.0, font, &fit(d, &c.row, font, xt - xr - pad), t.color.text);
        d.text(xt, y + 4.0, font, &fit(d, &c.trigger, font, xw - xt - pad), t.color.text);
        if lines.is_empty() {
            d.text(xw, y + 4.0, font, "nothing changes", t.color.muted);
        }
        for (k, l) in lines.iter().enumerate() {
            let color = if l.starts_with("sends ") { t.color.accent } else { t.color.text };
            d.text(xw, y + 4.0 + k as f32 * lh, font, &fit(d, l, font, a.x + a.w - xw - pad), color);
        }
        d.rect(a.x, y + h - 1.0, a.w, 1.0, t.color.line);
        host.hits.push((whole, Hit::Row(i)));
        y += h;
    }
}

fn problems(d: &mut DrawList, t: &Tokens, host: &mut Host, a: Rect, pointer: (f32, f32)) {
    let (pad, font, small) = (t.size.pad, t.size.font, t.size.small);
    let show = host.shown().clone();
    // the flash reports belong to the conductor this editor runs: the top
    let list = view::problems(&show, host.path.is_empty().then_some(&host.sub));
    let mut y = a.y + pad;
    let (errors, warnings) = (list.iter().filter(|p| p.error).count(), list.iter().filter(|p| !p.error).count());
    d.text(a.x + pad, y, small, &format!("{errors} ERRORS · {warnings} WARNINGS – A WARNING NEVER BLOCKS GO"), t.color.muted);
    y += small * 1.8;
    d.rect(a.x, y - 4.0, a.w, 1.0, t.color.line);
    if list.is_empty() {
        d.text(a.x + pad, y + 4.0, font, "No problems: the document is valid and every deck that answered is ready.", t.color.muted);
        return;
    }
    let lh = font * 1.7;
    for (i, p) in list.iter().enumerate() {
        if y + lh > a.y + a.h {
            d.text(a.x + pad, y + 2.0, small, &format!("{} more …", list.len() - i), t.color.muted);
            break;
        }
        let whole = Rect { x: a.x, y, w: a.w, h: lh };
        if whole.has(pointer.0, pointer.1) {
            d.rect(whole.x, whole.y, whole.w, whole.h, t.color.hover);
        }
        d.disc(a.x + pad + 4.0, y + lh * 0.5, 4.0, if p.error { t.color.danger } else { t.color.warn });
        d.text(a.x + pad + 16.0, y + (lh - font * 1.25) * 0.5, font, &fit(d, &p.text, font, a.w - 2.0 * pad - 16.0), t.color.text);
        host.hits.push((whole, Hit::Problem(i)));
        y += lh;
    }
}
