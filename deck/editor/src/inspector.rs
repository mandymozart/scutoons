//! The inspector (#456; owner 2026-11-09: "the right side is the inspector ... it should show what can be set on the current
//! node"): what the selection is and what can be set on it – the rig, a track or a node. It draws and says where its fields
//! landed; the values and their ranges are the core's.
use crate::stage::{Param, Stage};
use crate::wires::Port;
use deck_space::modreg::PageSpec;
use deck_kit::editor::colour;
use deck_kit::editor::controls::{self, Rect};
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;

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
    /// an area tab of a node's sections (node, area): its first page, pressed again its next page
    Area(usize, usize),
    /// a far world node's address (by its graph node): a click opens the address book
    Address(usize),
    /// takes the selected track or node away
    Remove,
    /// the selected track armed or disarmed (it takes the hand, its monitoring on)
    Arm,
    /// the rig's stage background: a click on its value types it, on its swatch opens the colour picker
    Background,
    BackgroundColour,
    /// one of the rig's presets: loaded into its working state
    PresetLoad(usize),
    /// the working state saved as a new preset (a name is typed)
    PresetSaveAs,
    /// the loaded preset saved: your own overwritten, a factory one derived
    PresetSave,
}

/// The rig's preset line: its presets' names, the one loaded, whether the knobs left it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PresetLine {
    /// a rig on its own is open (not a scene, not a member inside one)
    pub shown: bool,
    pub names: Vec<String>,
    pub loaded: Option<usize>,
    pub dirty: bool,
    /// the loaded one is a factory preset: Save derives
    pub factory: bool,
}

impl PresetLine {
    /// the row's value: the loaded preset with a dot once a knob moved, or the working state
    pub fn value(&self) -> String {
        match self.loaded.and_then(|i| self.names.get(i)) {
            Some(n) => format!("{n}{}", if self.dirty { " •" } else { "" }),
            None => "none · the working state".into(),
        }
    }

    /// the buttons: Save as preset always, Save once a loaded one is dirty
    pub fn buttons(&self) -> Vec<(&'static str, Field)> {
        let mut out = vec![("Save as preset", Field::PresetSaveAs)];
        if self.loaded.is_some() && self.dirty {
            out.push((if self.factory { "Save (derives)" } else { "Save" }, Field::PresetSave));
        }
        out
    }
}

/// the PRESET section: the loaded one, the presets to load (as many as fit a row), the buttons
fn presets(d: &mut DrawList, t: &Tokens, area: Rect, y: &mut f32, line: &PresetLine, out: &mut Vec<(Rect, Field)>) {
    *y += controls::section(d, t, area.x, *y, area.w, "PRESET", false);
    controls::row(d, t, area, y, "preset", &line.value());
    let h = t.size.row - 4.0;
    if !line.names.is_empty() {
        let fit: Vec<&str> = line.names.iter().map(String::as_str).take(6).collect();
        for (i, r) in controls::pills(d, t, area.x, *y + 2.0, h, &fit, line.loaded.unwrap_or(usize::MAX)).into_iter().enumerate() {
            if r.x + r.w <= area.x + area.w {
                out.push((r, Field::PresetLoad(i)));
            }
        }
        *y += t.size.row;
    }
    let buttons = line.buttons();
    let names: Vec<&str> = buttons.iter().map(|b| b.0).collect();
    for (r, (_, f)) in controls::pills(d, t, area.x, *y + 2.0, h, &names, usize::MAX).into_iter().zip(buttons) {
        out.push((r, f));
    }
    *y += t.size.row;
}

/// Which page of a node's sections the inspector shows: an area (its tab) and a page of it. One button per area, the
/// Analog Four's way: a press on another area opens its first page, a press on the open one its next page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Paging {
    pub node: usize,
    pub area: usize,
    pub page: usize,
}

/// the areas of `pages` in their order, each with the indices of its pages
pub fn areas(pages: &[PageSpec]) -> Vec<(&'static str, Vec<usize>)> {
    let mut out: Vec<(&'static str, Vec<usize>)> = Vec::new();
    for (i, p) in pages.iter().enumerate() {
        match out.iter_mut().find(|(a, _)| *a == p.area) {
            Some((_, list)) => list.push(i),
            None => out.push((p.area, vec![i])),
        }
    }
    out
}

impl Paging {
    /// where `node` stands: this paging if it is that node's, else its first page
    pub fn of(self, node: usize) -> Paging {
        if self.node == node {
            self
        } else {
            Paging { node, area: 0, page: 0 }
        }
    }

    /// a press on area tab `area` of `node`, which has `count` pages
    pub fn press(self, node: usize, area: usize, count: usize) -> Paging {
        let now = self.of(node);
        let page = if now.area == area { (now.page + 1) % count.max(1) } else { 0 };
        Paging { node, area, page }
    }

    /// page `area/page` of `pages` opened on `node`
    pub fn at(node: usize, pages: &[PageSpec], key: &str) -> Option<Paging> {
        areas(pages).iter().enumerate().find_map(|(a, (_, list))| list.iter().position(|&i| pages[i].key == key).map(|page| Paging { node, area: a, page }))
    }
}

/// the knobs' grid: two rows of four
const COLUMNS: usize = 4;

/// A node's sections: the area tabs, the open area's page names (the shown one bright) and its page as two rows of four
/// knobs; a knob place without a parameter stays, so twin pages line up. The parameters it placed.
#[allow(clippy::too_many_arguments)]
fn sections(d: &mut DrawList, t: &Tokens, (area, y): (Rect, &mut f32), node: usize, pages: &[PageSpec], params: &[Param], paging: Paging, hot: Option<Field>, out: &mut Vec<(Rect, Field)>) {
    let list = areas(pages);
    let now = paging.of(node);
    let a = now.area.min(list.len().saturating_sub(1));
    let names: Vec<&str> = list.iter().map(|(n, _)| *n).collect();
    let h = t.size.row - 4.0;
    for (i, r) in controls::pills(d, t, area.x, *y, h, &names, a).into_iter().enumerate() {
        out.push((r, Field::Area(node, i)));
    }
    *y += t.size.row;
    let Some((_, in_area)) = list.get(a) else { return };
    let shown = in_area[now.page.min(in_area.len() - 1)];
    let small = t.size.small;
    let mut x = area.x;
    for &i in in_area {
        let name = pages[i].page;
        d.sans(x, *y, small, name, if i == shown { t.color.text } else { t.color.muted });
        x += d.sans_width(small, name) + t.size.pad;
    }
    *y += small * 1.8;
    let labels = knob_labels(&pages[shown].slots);
    let cell = area.w / COLUMNS as f32;
    let tall = (cell * 1.05).min(t.size.row * 3.2);
    for (k, slot) in pages[shown].slots.iter().enumerate() {
        let r = Rect { x: area.x + (k % COLUMNS) as f32 * cell, y: *y + (k / COLUMNS) as f32 * tall, w: cell, h: tall - t.size.pad * 0.5 };
        let Some(p) = slot.and_then(|name| params.iter().find(|p| p.name == name)) else {
            controls::knob(d, t, r, "", None, false);
            continue;
        };
        let field = Field::Param(node, p.port);
        let value = match &p.linked {
            Some(from) => format!("< {from}"),
            None => crate::number(p.value),
        };
        let f = if p.linked.is_some() { 0.0 } else { p.spec.norm(p.value) };
        controls::knob(d, t, r, labels[k], Some((&value, f)), hot == Some(field));
        if p.linked.is_none() {
            out.push((r, field));
        }
    }
    *y += tall * 2.0;
}

/// What a page's knobs say: their names without the prefix the page already names (`o1_wave` on OSC 1 is `wave`), the
/// whole names where that would leave two the same.
fn knob_labels(slots: &[Option<&'static str>]) -> Vec<&'static str> {
    let short: Vec<&'static str> = slots.iter().map(|s| s.map_or("", |n| n.split_once('_').map_or(n, |(_, rest)| rest))).collect();
    let unique = short.iter().enumerate().all(|(i, a)| a.is_empty() || !short[..i].contains(a));
    if unique {
        short
    } else {
        slots.iter().map(|s| s.unwrap_or("")).collect()
    }
}

/// The stage background's row as a rig, a scene, a show or a module shows it: its own, else what it falls back to.
#[derive(Debug, Clone, PartialEq)]
pub struct BackgroundRow {
    pub value: String,
    pub swatch: Option<[f32; 3]>,
    /// said under the row: a video that does not play yet, how a module sets it
    pub note: Option<String>,
}

// TODO(#326): a video background plays once the deck decodes video
pub const VIDEO_NOTE: &str = "a video does not play on the stage yet: black";

pub fn background_row(own: Option<&deck_space::Background>, unset: &str) -> BackgroundRow {
    let Some(b) = own else { return BackgroundRow { value: format!("none · {unset}"), swatch: None, note: None } };
    let note = b.video.is_some().then(|| VIDEO_NOTE.to_string());
    BackgroundRow { value: b.text().to_string(), swatch: b.rgb(), note }
}

/// The row drawn, its note under it; the swatch's place (the colour picker) and the value's (a click types it).
pub fn background(d: &mut DrawList, t: &Tokens, area: Rect, y: &mut f32, row: &BackgroundRow) -> (Rect, Rect) {
    let top = *y;
    let (swatch, _) = colour::swatch_field(d, t, area, y, "background", &row.value, row.swatch.map(colour::bytes));
    let at = Rect { x: area.x, y: top, w: area.w, h: *y - top };
    if let Some(note) = &row.note {
        let small = t.size.small;
        for line in deck_kit::editor::info::wrap(note, (area.w / d.advance(small)).max(8.0) as usize).into_iter().take(3) {
            d.text(area.x, *y, small, &line, t.color.muted);
            *y += small * 1.5;
        }
        *y += t.size.pad * 0.5;
    }
    (swatch, at)
}

fn remove(d: &mut DrawList, t: &Tokens, area: Rect, y: &mut f32, out: &mut Vec<(Rect, Field)>) {
    if let Some(r) = controls::pills(d, t, area.x, *y, t.size.row - 4.0, &["Remove"], usize::MAX).into_iter().next() {
        out.push((r, Field::Remove));
    }
    *y += t.size.row;
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
        let r = Rect { x: area.x, y: *y, w: area.w, h: t.size.row };
        // the live layer over it: the bar plays the live value, the mark and the first number are yours (what is saved)
        let (text, hand) = match stage.live(k) {
            Some(h) => (format!("{} · live {}", crate::number(h.hand), crate::number(h.live)), Some(crate::fraction(h.hand, k.min, k.max, k.log))),
            None => (crate::number(v), None),
        };
        let bar = controls::scrub(d, t, r, &k.label, &text, f, hot == Some(Field::Knob(i)));
        if let Some(h) = hand {
            d.rect(r.x + (r.w - 2.0) * h.clamp(0.0, 1.0), r.y + 2.0, 2.0, r.h - 4.0, t.color.text);
        }
        out.push((bar, Field::Knob(i)));
        *y += t.size.row;
    }
}

fn port_line(d: &mut DrawList, t: &Tokens, area: Rect, y: &mut f32, p: &Port) {
    let c = if p.input { t.color.text } else { t.color.muted };
    d.disc(area.x + 5.0, *y + t.size.tree_row * 0.5, 4.0, c);
    d.text(area.x + 16.0, *y + 5.0, t.size.font, &p.name, t.color.text);
    let other = p.cables.last().map_or(p.ty.to_string(), |c| format!("< {}", c.from));
    let w = d.advance(t.size.small) * other.chars().count() as f32;
    d.text(area.x + area.w - w, *y + 6.0, t.size.small, &other, t.color.muted);
    *y += t.size.tree_row;
}

/// draw it inside `area`; `edits` false (Play) leaves out the rows whose edits do not last: presets, Remove
pub fn draw(d: &mut DrawList, t: &Tokens, area: Rect, (stage, line): (&Stage, &PresetLine), sel: &Sel, (paging, edits): (Paging, bool), hot: Option<Field>) -> Vec<(Rect, Field)> {
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
            let row = background_row(stage.bundle.rig.background.as_ref(), "its modules', else black");
            let (swatch, at) = background(d, t, area, &mut y, &row);
            // the first rect that has the pointer takes the press: the swatch before its row
            out.push((swatch, Field::BackgroundColour));
            out.push((at, Field::Background));
            if edits && line.shown && !stage.knobs.is_empty() {
                presets(d, t, area, &mut y, line, &mut out);
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
                let armed = stage.monitor.is_armed(id);
                let a = controls::toggle(d, t, Rect { x: area.x, y: y + 2.0, w: d.advance(t.size.font) * 6.0, h: t.size.row - 4.0 }, "Arm", armed);
                out.push((a, Field::Arm));
                if armed {
                    d.text(a.x + a.w + t.size.pad, y + 5.0, t.size.small, "takes your hand · monitored", t.color.muted);
                }
                y += t.size.row;
                if let Some(why) = stage.silent.get(id) {
                    let small = t.size.small;
                    for line in deck_kit::editor::info::wrap(why, (area.w / d.advance(small)).max(8.0) as usize).into_iter().take(4) {
                        d.text(area.x, y, small, &line, t.color.danger);
                        y += small * 1.5;
                    }
                    y += t.size.pad;
                }
                if edits {
                    remove(d, t, area, &mut y, &mut out);
                }
            }
            knobs(d, t, stage, (area, &mut y), Some(id), hot, &mut out);
        }
        Sel::Node(node) => {
            y += controls::section(d, t, area.x, y, area.w, "NODE", true);
            let id = stage.node_ids.get(*node).cloned().unwrap_or_default();
            controls::row(d, t, area, &mut y, "name", &id);
            controls::row(d, t, area, &mut y, "kind", stage.kind_name(*node));
            if edits && stage.ports.iter().any(|q| q.node == id && !q.track.is_empty()) {
                remove(d, t, area, &mut y, &mut out);
            }
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
            let pages = stage.kinds.get(*node).map_or(&[][..], |k| k.pages());
            if !pages.is_empty() {
                sections(d, t, (area, &mut y), *node, pages, &params, paging, hot, &mut out);
            }
            let on_a_page = |name: &str| pages.iter().any(|p| p.slots.contains(&Some(name)));
            for p in params.iter().filter(|p| !on_a_page(p.name)) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use deck_space::Background;

    /// the panel model on every holder: none says what plays instead, a colour carries its swatch, a video says it is black
    #[test]
    fn the_background_row_says_what_is_set() {
        assert_eq!(background_row(None, "its rigs', else black"), BackgroundRow { value: "none · its rigs', else black".into(), swatch: None, note: None });
        let colour = background_row(Some(&Background::colour("#000")), "x");
        assert_eq!((colour.value.as_str(), colour.swatch, colour.note), ("#000", Some([0.0; 3]), None));
        let image = background_row(Some(&Background::image("backgrounds/nave.webp")), "x");
        assert_eq!((image.value.as_str(), image.swatch), ("backgrounds/nave.webp", None));
        let video = background_row(Background::typed("loops/sky.webm").unwrap().as_ref(), "x");
        assert_eq!(video.note.as_deref(), Some(VIDEO_NOTE));
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        let area = Rect { x: 10.0, y: 20.0, w: 300.0, h: 400.0 };
        let mut y = area.y;
        let (swatch, at) = background(&mut d, &t, area, &mut y, &video);
        assert_eq!((at.x, at.y, at.w), (10.0, 20.0, 300.0));
        assert!(at.has(swatch.x + 1.0, swatch.y + 1.0) && swatch.w < at.w, "the swatch sits in its row");
        assert!((at.h - t.size.tree_row).abs() < 1e-3, "the row is what a click types into");
        assert!(y > at.y + at.h, "its note under it");
    }

    #[test]
    fn the_preset_line_says_what_is_loaded_and_offers_save_once_dirty() {
        let mut l = PresetLine { names: vec!["soft".into(), "dirty".into()], ..Default::default() };
        assert_eq!((l.value().as_str(), l.buttons().len()), ("none · the working state", 1));
        l.loaded = Some(1);
        assert_eq!((l.value().as_str(), l.buttons().len()), ("dirty", 1), "clean: nothing to save");
        l.dirty = true;
        assert_eq!(l.value(), "dirty •");
        assert_eq!(l.buttons()[1], ("Save", Field::PresetSave));
        l.factory = true;
        assert_eq!(l.buttons()[1].0, "Save (derives)");
    }
}
