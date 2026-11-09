//! The mixer band under the stage (#456; owner 2026-11-09: "Yes, band under stage"): a strip per track – its level on the
//! core's meter scale, a fader, mute – and what goes out. Folds to its head. The scale, the zones and the slots are the core's.
use crate::stage::Stage;
use deck_core::meters::{self, Zone};
use deck_kit::controls::{self, Rect};
use deck_kit::draw::DrawList;
use deck_kit::tokens::Tokens;

/// the band's height when open and folded
pub const OPEN: f32 = 132.0;
pub const FOLDED: f32 = 24.0;
/// the top of a fader's travel
pub const GAIN_MAX: f32 = 2.0;
const STRIP: f32 = 86.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Part {
    Head,
    Fader(usize, Rect),
    Mute(usize),
    Pan(usize),
}

fn meter(d: &mut DrawList, t: &Tokens, r: Rect, peak: f32) {
    let db = meters::level_db(peak);
    let color = match Zone::of_db(db) {
        Zone::Ok => t.color.go,
        Zone::Hot => t.color.warn,
        Zone::Clip => t.color.stop,
    };
    controls::level(d, t, r, meters::position(db), color);
}

/// the band in `area`; where its parts landed
pub fn draw(d: &mut DrawList, t: &Tokens, area: Rect, open: bool, stage: &Stage, hot: Option<usize>, panning: Option<(usize, f32)>) -> Vec<(Rect, Part)> {
    let mut out = Vec::new();
    d.rect(area.x, area.y, area.w, area.h, t.color.surface);
    d.rect(area.x, area.y, area.w, 1.0, t.color.line);
    let head = Rect { x: area.x, y: area.y, w: area.w, h: FOLDED };
    d.icon(area.x + t.size.pad, area.y + (FOLDED - 12.0) * 0.5, 12.0, if open { "down" } else { "up" }, t.color.muted);
    d.text(area.x + t.size.pad + 16.0, area.y + (FOLDED - t.size.small * 1.25) * 0.5, t.size.small, "MIXER", t.color.muted);
    out.push((head, Part::Head));
    if !open {
        return out;
    }
    let top = area.y + FOLDED;
    let (h, small) = (area.h - FOLDED - 8.0, t.size.small);
    let inner = h - small * 1.5;
    for (i, s) in stage.strips.iter().enumerate() {
        let x = area.x + t.size.pad + i as f32 * STRIP;
        d.text(x, top, small, &s.track, t.color.text);
        let y = top + small * 1.5;
        meter(d, t, Rect { x, y, w: 8.0, h: inner }, s.peak);
        let muted = stage.bundle.rig.tracks.iter().find(|x| x.id == s.track).is_some_and(|x| x.mute);
        let gain = stage.bundle.rig.tracks.iter().find(|x| x.id == s.track).map_or(1.0, |x| x.gain);
        let value = if hot == Some(i) { stage.strip_gain(s) } else { gain };
        let f = Rect { x: x + 14.0, y, w: 16.0, h: inner };
        controls::fader(d, t, f, value / GAIN_MAX, hot == Some(i));
        if !s.owned {
            out.push((f, Part::Fader(i, f)));
        }
        let m = controls::toggle(d, t, Rect { x: x + 38.0, y, w: 24.0, h: 22.0 }, "M", muted);
        out.push((m, Part::Mute(i)));
        d.text(x + 38.0, y + 28.0, small, &crate::number(value), t.color.muted);
        let stored = stage.bundle.rig.tracks.iter().find(|x| x.id == s.track).map_or(0.0, |x| x.pan);
        let pan = panning.filter(|(j, _)| *j == i).map_or(stored, |(_, v)| v);
        let p = Rect { x: x + 38.0, y: y + 46.0, w: 40.0, h: 12.0 };
        d.round_rect(p.x, p.y, p.w, p.h, 3.0, t.color.control);
        d.round_rect(p.x + p.w * 0.5 + pan * (p.w * 0.5 - 3.0) - 2.0, p.y + 1.0, 4.0, p.h - 2.0, 2.0, t.color.text);
        out.push((p, Part::Pan(i)));
    }
    let x = area.x + area.w - 40.0;
    d.text(x - 6.0, top, small, "OUT", t.color.muted);
    meter(d, t, Rect { x, y: top + small * 1.5, w: 10.0, h: inner }, stage.master_peak);
    out
}
