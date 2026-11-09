//! The mixer band under the stage (#456; owner 2026-11-09: "Yes, band under stage", "more compact and it needs to show 0 db and
//! perhapse last clip value. copy a bit like abletons channel strip", "when entering a node, the mixer should show the internal
//! gains down there. so it is relativ."): the level of the tree it shows, a tab per level, so a parent's levels stay in
//! reach. A strip: its level on the core's meter scale beside a fader on the same scale with the 0 dB mark, the highest
//! peak until you click it; a track's has pan and mute. Folds to its head. The scale, the zones and the slots are the core's.
use crate::stage::Stage;
use deck_core::meters::{self, Zone};
use deck_kit::controls::{self, Rect};
use deck_kit::draw::DrawList;
use deck_kit::tokens::Tokens;

/// the band's height when open and folded
pub const OPEN: f32 = 112.0;
pub const FOLDED: f32 = 22.0;
/// the mixer's gain range (catalog `mixer`)
pub const GAIN_MAX: f32 = 1.5;
const STRIP: f32 = 66.0;

/// a strip: a track's on the rig's sum, or a gain inside a track
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    Track(usize),
    Inner(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Part {
    Head,
    /// a level of the tree: 0 = the rig
    Tab(usize),
    Fader(Target, Rect),
    Mute(usize),
    Pan(usize),
    /// a highest-peak readout: a strip's, or the output's
    Peak(Option<Target>),
}

/// a gain on the meter's scale, 0..1 from the bottom
pub fn gain_position(gain: f32) -> f32 {
    meters::position(meters::level_db(gain))
}

/// the gain at a place on the meter's scale; the very bottom is silence
pub fn position_gain(position: f32) -> f32 {
    if position <= 0.0 {
        return 0.0;
    }
    let db = meters::FLOOR_DB + position.min(1.0) * (meters::TOP_DB - meters::FLOOR_DB);
    10f32.powf(db / 20.0).min(GAIN_MAX)
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

/// the 0 dB mark across a strip's meter and fader
fn zero(d: &mut DrawList, t: &Tokens, x: f32, y: f32, w: f32, h: f32) {
    let at = y + h * (1.0 - meters::position(0.0));
    d.rect(x, at, w, 1.0, t.color.text);
}

fn db_text(linear: f32) -> String {
    if linear <= 0.0 || meters::level_db(linear) <= meters::FLOOR_DB {
        return "-inf".to_string();
    }
    format!("{:.1}", meters::level_db(linear))
}

/// the highest peak, red once it has clipped; its place
fn peak_readout(d: &mut DrawList, t: &Tokens, x: f32, y: f32, w: f32, max: f32) -> Rect {
    let clipped = meters::level_db(max) > 0.0;
    let r = Rect { x, y, w, h: t.size.small + 3.0 };
    d.round_rect(r.x, r.y, r.w, r.h, 2.0, if clipped { t.color.stop } else { t.color.surface_head });
    let text = db_text(max);
    let tw = d.advance(t.size.small) * text.chars().count() as f32;
    d.text(r.x + r.w - tw - 2.0, r.y + 1.0, t.size.small, &text, if clipped { t.color.text } else { t.color.muted });
    r
}

/// one strip to draw
struct Row {
    target: Target,
    name: String,
    peak: f32,
    max: f32,
    value: f32,
    /// no hand on the fader: a policy owns it, or a cable feeds it
    fixed: bool,
    /// a track's: its pan and whether it is muted
    track: Option<(f32, bool)>,
}

/// what the band draws at this level
pub struct View {
    /// the tree's levels: "rig", then the tracks entered
    pub tabs: Vec<String>,
    pub tab: usize,
    /// the fader in the hand
    pub hot: Option<Target>,
    pub panning: Option<(usize, f32)>,
}

fn rows(stage: &Stage, v: &View) -> Vec<Row> {
    let hot = v.hot;
    if v.tab == 0 {
        return stage
            .strips
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let track = stage.bundle.rig.tracks.iter().find(|x| x.id == s.track);
                let target = Target::Track(i);
                let stored = track.map_or(1.0, |x| x.gain);
                let pan = v.panning.filter(|(j, _)| *j == i).map(|(_, p)| p).or(track.map(|x| x.pan)).unwrap_or(0.0);
                Row { target, name: s.track.clone(), peak: s.peak, max: s.max, value: if hot == Some(target) { stage.strip_gain(s) } else { stored }, fixed: s.owned, track: Some((pan, track.is_some_and(|x| x.mute))) }
            })
            .collect();
    }
    let track = v.tabs.get(v.tab).map(String::as_str).unwrap_or("");
    stage
        .inner
        .iter()
        .enumerate()
        .filter(|(_, s)| s.track == track)
        .map(|(i, s)| Row { target: Target::Inner(i), name: s.label.clone(), peak: s.peak, max: s.max, value: stage.inner_gain(s), fixed: s.linked, track: None })
        .collect()
}

/// the band in `area`; where its parts landed
pub fn draw(d: &mut DrawList, t: &Tokens, area: Rect, open: bool, stage: &Stage, v: &View) -> Vec<(Rect, Part)> {
    let mut out = Vec::new();
    d.rect(area.x, area.y, area.w, area.h, t.color.surface);
    d.rect(area.x, area.y, area.w, 1.0, t.color.line);
    let head = Rect { x: area.x, y: area.y, w: area.w, h: FOLDED };
    d.icon(area.x + t.size.pad, area.y + (FOLDED - 12.0) * 0.5, 12.0, if open { "down" } else { "up" }, t.color.muted);
    let label_w = d.text(area.x + t.size.pad + 16.0, area.y + (FOLDED - t.size.small * 1.25) * 0.5, t.size.small, "MIXER", t.color.muted);
    let names: Vec<&str> = v.tabs.iter().map(String::as_str).collect();
    for (depth, r) in controls::pills(d, t, area.x + t.size.pad + 16.0 + label_w + t.size.pad, area.y + 2.0, FOLDED - 4.0, &names, v.tab).into_iter().enumerate() {
        out.push((r, Part::Tab(depth)));
    }
    out.push((head, Part::Head));
    if !open {
        return out;
    }
    let small = t.size.small;
    let y0 = area.y + FOLDED;
    for (n, row) in rows(stage, v).into_iter().enumerate() {
        let x = area.x + t.size.pad + n as f32 * STRIP;
        let name: String = row.name.chars().take(7).collect();
        d.text(x, y0, small, &name, t.color.text);
        let pan_y = y0 + small + 4.0;
        if let (Some((pan, _)), Target::Track(i)) = (row.track, row.target) {
            let p = Rect { x, y: pan_y, w: STRIP - 10.0, h: 7.0 };
            d.round_rect(p.x, p.y, p.w, p.h, 3.0, t.color.control);
            d.round_rect(p.x + p.w * 0.5 + pan * (p.w * 0.5 - 2.0) - 1.5, p.y, 3.0, p.h, 1.5, t.color.text);
            out.push((p, Part::Pan(i)));
        }
        // meter and fader on one scale, the 0 dB mark across both
        let y = pan_y + 11.0;
        let inner = area.y + area.h - y - 5.0;
        meter(d, t, Rect { x, y, w: 6.0, h: inner }, row.peak);
        let f = Rect { x: x + 8.0, y, w: 11.0, h: inner };
        controls::fader(d, t, f, gain_position(row.value), v.hot == Some(row.target));
        zero(d, t, x, y, 19.0, inner);
        if !row.fixed {
            out.push((f, Part::Fader(row.target, f)));
        }
        let col = x + 23.0;
        let w = STRIP - 33.0;
        let r = peak_readout(d, t, col, y, w, row.max);
        out.push((r, Part::Peak(Some(row.target))));
        d.text(col, y + small + 7.0, small, &db_text(row.value), t.color.muted);
        if let (Some((_, muted)), Target::Track(i)) = (row.track, row.target) {
            let m = controls::toggle(d, t, Rect { x: col, y: y + inner - 16.0, w, h: 16.0 }, "M", muted);
            out.push((m, Part::Mute(i)));
        }
    }
    // what goes out
    let x = area.x + area.w - 54.0;
    d.text(x, y0, small, "OUT", t.color.muted);
    let y = y0 + small + 15.0;
    let inner = area.y + area.h - y - 5.0;
    meter(d, t, Rect { x, y, w: 8.0, h: inner }, stage.master_peak);
    zero(d, t, x, y, 8.0, inner);
    let r = peak_readout(d, t, x + 12.0, y, 34.0, stage.master_max);
    out.push((r, Part::Peak(None)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_gain_sits_on_the_zero_mark_and_comes_back() {
        assert_eq!(gain_position(1.0), meters::position(0.0));
        assert!((position_gain(gain_position(1.0)) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn the_bottom_of_the_fader_is_silence_and_its_top_the_mixers_limit() {
        assert_eq!(position_gain(0.0), 0.0);
        assert!(position_gain(1.0) <= GAIN_MAX);
    }

    #[test]
    fn silence_reads_minus_infinity() {
        assert_eq!(db_text(0.0), "-inf");
        assert_eq!(db_text(1.0), "0.0");
    }
}
