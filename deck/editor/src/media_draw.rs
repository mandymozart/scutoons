//! The medium's Asset view: its picture (or waveform) fitted into the window under the bar – the asset's crop shaded,
//! its fade darkening it –, the timeline with the asset's span, the use's in and out points and the playhead, the asset's
//! edits as fields to drag, and what it is. Returns where the picture's card goes.
use crate::media::{Body, Field, Hit, Media, Sort, TRIM_UNIT};
use deck_core::units::{self, Unit};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_kit::editor::units::{self as unit_field, Prefs};

/// what the units are read with: the units chosen per kind of value, the tempo `(bpm, per_bar)` when known
pub struct Reading<'a> {
    pub prefs: &'a Prefs,
    pub tempo: Option<(f64, u32)>,
}

fn alpha(c: [f32; 4], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], c[3] * a]
}

/// `w` over `h` fitted into `r`, centred
pub fn fit(r: Rect, aspect: f32) -> Rect {
    let (mut w, mut h) = (r.w, r.w / aspect.max(0.01));
    if h > r.h {
        (w, h) = (r.h * aspect, r.h);
    }
    Rect { x: r.x + (r.w - w) * 0.5, y: r.y + (r.h - h) * 0.5, w, h }
}

pub fn draw(d: &mut DrawList, t: &Tokens, m: &mut Media, (w, h): (f32, f32), top: f32, reading: &Reading) -> Option<Rect> {
    m.hits.clear();
    m.line = None;
    let aspect = m.aspect();
    let o = m.open.as_ref()?;
    let (pad, font, small, row) = (t.size.pad * 2.0, t.size.font, t.size.small, t.size.row);
    let (x0, w0) = (pad, w - 2.0 * pad);
    let mut y = top + pad;
    d.text(x0, y, font, &format!("{} · {}", o.sort.word(), o.path), t.color.text);
    y += row * 0.9;
    let use_words = match &o.used {
        Some(u) => format!("a use: {} › {} – its trim cuts inside the asset's edits; the file stays whole", u.scene, u.node),
        None if matches!(o.sort, Sort::Video | Sort::Audio) => "the asset: its edits apply wherever it is used – the file stays whole".to_string(),
        None => String::new(),
    };
    let use_words = match &o.note {
        Some(n) => format!("{use_words} · {n}"),
        None => use_words,
    };
    d.text(x0, y, small, &controls::fit(&use_words, (w0 / d.advance(small)) as usize), t.color.muted);
    y += row;
    let timed = matches!(o.body, Body::Video(_) | Body::Audio(_));
    let fields: Vec<Field> = Field::ALL.into_iter().filter(|f| timed && (!f.picture() || matches!(o.body, Body::Video(_)))).collect();
    let field_rows = fields.len().div_ceil(4) as f32;
    let line_h = if timed { row * 3.0 + field_rows * (row + 4.0) + row * 0.5 } else { 0.0 };
    // the status bar along the bottom is about as tall as the bar on top
    let stage = Rect { x: x0, y, w: w0, h: (h - y - pad - line_h - top).max(row) };
    let mut card = None;
    match &o.body {
        Body::Loading => {
            d.text(stage.x, stage.y + stage.h * 0.5, font, "loading …", t.color.muted);
        }
        Body::Failed(why) => {
            d.text(stage.x, stage.y + stage.h * 0.5, font, &controls::fit(why, (stage.w / d.advance(font)) as usize), t.color.danger);
        }
        Body::Model => {
            d.text(stage.x, stage.y + stage.h * 0.5, font, "a 3D set: its preview on a stage of its own is not built yet", t.color.muted);
        }
        Body::Video(_) | Body::Image(_) => {
            card = aspect.map(|a| fit(stage, a));
            if let (Some(r), Some(p)) = (card, o.playback()) {
                shade(d, t, r, p.crop, 1.0 - p.level(o.clock));
            }
        }
        Body::Audio(wave) => {
            let n = wave.peaks.len().max(1);
            let (bw, mid) = (stage.w / n as f32, stage.y + stage.h * 0.5);
            let (a, b) = o.span().unwrap_or((0.0, wave.seconds));
            let gain = o.playback().map_or(1.0, |p| p.gain);
            for (i, p) in wave.peaks.iter().enumerate() {
                let s = (i as f64 + 0.5) / n as f64 * wave.seconds + o.offset;
                let c = if s >= a && s <= b { t.color.bright } else { alpha(t.color.muted, 0.5) };
                let hh = (p * gain * stage.h * 0.45).clamp(1.0, stage.h * 0.5);
                d.rect(stage.x + i as f32 * bw, mid - hh, bw.max(1.0), hh * 2.0, c);
            }
        }
    }
    if let (true, Some(length)) = (timed, o.source_length()) {
        let line = Rect { x: x0, y: stage.y + stage.h + row * 0.5, w: w0, h: row };
        timeline(d, t, m, line, length, reading);
        edits(d, t, m, &fields, Rect { x: x0, y: line.y + row * 2.5, w: w0, h: row }, reading);
    }
    card
}

/// what the crop cuts off, darkened over the picture `r`, and the fade's darkness `dark` (0..1) over what stays
fn shade(d: &mut DrawList, t: &Tokens, r: Rect, crop: deck_core::asset_edit::Crop, dark: f32) {
    let [u, v, w, h] = crop.rect();
    let kept = Rect { x: r.x + u * r.w, y: r.y + v * r.h, w: w * r.w, h: h * r.h };
    let cut = alpha(t.color.stage, 0.72);
    d.rect(r.x, r.y, r.w, kept.y - r.y, cut);
    d.rect(r.x, kept.y + kept.h, r.w, r.y + r.h - kept.y - kept.h, cut);
    d.rect(r.x, kept.y, kept.x - r.x, kept.h, cut);
    d.rect(kept.x + kept.w, kept.y, r.x + r.w - kept.x - kept.w, kept.h, cut);
    if !crop.none() {
        d.outline(kept.x, kept.y, kept.w, kept.h, (0.0, 1.0), alpha(t.color.bright, 0.6));
    }
    if dark > 0.0 {
        d.rect(kept.x, kept.y, kept.w, kept.h, alpha(t.color.stage, dark.clamp(0.0, 1.0)));
    }
}

/// the asset's edits as fields to drag, four to a row; one with a unit in the unit chosen for its kind, its switch at its end
fn edits(d: &mut DrawList, t: &Tokens, m: &mut Media, fields: &[Field], r: Rect, reading: &Reading) {
    let Some(o) = m.open.as_ref() else { return };
    let gap = t.size.pad;
    let w = (r.w - 3.0 * gap) / 4.0;
    let mut placed = Vec::new();
    for (i, &f) in fields.iter().enumerate() {
        let (col, line) = ((i % 4) as f32, (i / 4) as f32);
        let at = Rect { x: r.x + col * (w + gap), y: r.y + line * (r.h + 4.0), w, h: r.h };
        let (lo, hi, log) = f.range();
        let fraction = crate::fraction(f.get(&o.edits) as f32, lo as f32, hi as f32, log);
        let hot = m.drag == Some(Hit::Edit(f));
        let Some((kind, offered, first)) = f.unit() else {
            placed.push((controls::scrub(d, t, at, f.label(), &f.shown(&o.edits), fraction, hot), Hit::Edit(f)));
            continue;
        };
        let refs = o.refs(Some(f), reading.tempo);
        let offered: Vec<Unit> = offered.iter().copied().filter(|u| u.known(&refs)).collect();
        let unit = Some(reading.prefs.unit(kind, first, &refs)).filter(|u| offered.contains(u)).unwrap_or(first);
        let v = unit.from_base(f.get(&o.edits), &refs).unwrap_or(f.get(&o.edits));
        let (field, segs) = unit_field::scrub(d, t, at, f.label(), v, unit, &offered, fraction, hot);
        placed.push((field, Hit::Edit(f)));
        placed.extend(segs.into_iter().map(|(r, u)| (r, Hit::Unit(kind, u))));
    }
    m.hits.extend(placed);
}

fn timeline(d: &mut DrawList, t: &Tokens, m: &mut Media, r: Rect, length: f64, reading: &Reading) {
    let Some(o) = m.open.as_ref() else { return };
    let small = t.size.small;
    let (a, b) = o.span().unwrap_or((0.0, length));
    let (aa, ab) = o.edits.trim.span(length);
    // the stretch shown: kept while a handle is held, so the file does not move under the pointer
    if m.drag.is_none() || m.view.1 <= m.view.0 {
        m.view = crate::media::view((a, b), length);
    }
    let (lo, hi) = m.view;
    let x_of = |s: f64| r.x + ((s - lo) / (hi - lo).max(1e-6)).clamp(0.0, 1.0) as f32 * r.w;
    d.rect(r.x, r.y, r.w, r.h, t.color.control);
    // the whole file under it, the stretch shown marked
    let whole = |s: f64| r.x + (s / length.max(1e-6)) as f32 * r.w;
    d.rect(r.x, r.y + r.h + 2.0, r.w, 3.0, t.color.line);
    d.rect(whole(lo), r.y + r.h + 2.0, (whole(hi) - whole(lo)).max(2.0), 3.0, t.color.muted);
    // the asset's span under the use's: a use cuts only inside it
    if o.used.is_some() {
        d.rect(x_of(aa), r.y + r.h - 3.0, x_of(ab) - x_of(aa), 3.0, alpha(t.color.text, 0.5));
    }
    d.rect(x_of(a), r.y, x_of(b) - x_of(a), r.h, t.color.selection);
    let head = x_of(o.at());
    d.rect(head - 1.0, r.y - 3.0, 2.0, r.h + 6.0, t.color.bright);
    m.line = Some(r);
    m.hits.push((r, Hit::Seek));
    let hw = 6.0;
    for (s, hit) in [(a, Hit::Start), (b, Hit::End)] {
        let x = x_of(s);
        let held = m.drag == Some(hit);
        d.rect(x - hw * 0.5, r.y - 4.0, hw, r.h + 8.0, if held { t.color.bright } else { t.color.text });
        m.hits.push((Rect { x: x - hw * 2.0, y: r.y - 6.0, w: hw * 4.0, h: r.h + 12.0 }, hit));
    }
    let whose = if o.used.is_some() { "the use's" } else { "the asset's" };
    let lap = o.playback().map_or(b - a, |p| p.lap());
    // the trim's points and lengths in the unit chosen for it (s · frames · beats at the clock's tempo), its switch at the end
    let refs = o.refs(None, reading.tempo);
    let offered: Vec<Unit> = [Unit::Seconds, Unit::Frames, Unit::Beats].into_iter().filter(|u| u.known(&refs)).collect();
    let unit = reading.prefs.unit(TRIM_UNIT, Unit::Seconds, &refs);
    let unit = if offered.contains(&unit) { unit } else { Unit::Seconds };
    let said = |s: f64| units::show(units::convert(s, Unit::Seconds, unit, &refs).unwrap_or(s), unit);
    let end = if (b - length).abs() < 1e-3 { format!("{} (the end)", said(b)) } else { said(b) };
    let words = format!("{whose} in {} · out {end} · plays {} of {} · at {}", said(a), said(lap), said(length), said(o.at()));
    let h = t.size.tree_row - 6.0;
    let segs = unit_field::switch(d, t, (r.x + r.w, r.y + r.h + 7.0, h), &offered, unit);
    let room = segs.first().map_or(r.w, |(s, _)| s.x - r.x - t.size.pad);
    d.text(r.x, r.y + r.h + 9.0, small, &controls::fit(&words, (room / d.advance(small)).max(0.0) as usize), t.color.muted);
    m.hits.extend(segs.into_iter().map(|(s, u)| (s, Hit::Unit(TRIM_UNIT, u))));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_fits_its_place_centred() {
        let r = Rect { x: 0.0, y: 0.0, w: 400.0, h: 100.0 };
        assert_eq!(fit(r, 2.0), Rect { x: 100.0, y: 0.0, w: 200.0, h: 100.0 });
        assert_eq!(fit(r, 8.0), Rect { x: 0.0, y: 25.0, w: 400.0, h: 50.0 });
    }
}
