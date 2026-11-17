//! Build's list of the modules related to the rig's character – the ones made for it (the core's `assign::related`) –
//! beside its stage while it is at the rig's level. A line is carried by the kit's one drag (`Carry`, the scene list's
//! too) and dropped into the rig's 3D space, the stage, as a track (`Choice::Module`, the same as Add's). Every other
//! module still plugs in through Add.
// TODO(#580): the library browser takes this list over as its "related" filter on the modules source
use crate::adding::{self, Choice};
use deck_catalog::Descriptor;
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::carry::Carry;
use deck_kit::editor::controls::{self, Rect};
use deck_space::rig::{assign, Bundle, Instrument};

/// a line of the list: the module's address, its name, what its track sends
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub address: String,
    pub label: String,
    pub detail: &'static str,
}

/// the character whose related modules show beside `b` (`yellowmouse`, a persona `yellowmouse.kid`), none for an item's rig
pub fn character(b: &Bundle) -> Option<&str> {
    crate::toon::character(&b.rig.subject)
}

/// the lines for `character` out of the catalog's modules that can be a track
pub fn lines(character: &str, modules: &[(Instrument, Descriptor)]) -> Vec<Line> {
    assign::related(character, modules)
        .into_iter()
        .map(|(i, d)| Line { address: format!("module:{}", d.name), label: d.name.to_string(), detail: if i.out.is_some() { "sound" } else { "events" } })
        .collect()
}

/// what the list says when no module names the character
pub fn none_yet(character: &str) -> String {
    format!("No module is made for {character} yet. Every module still plugs in through Add.")
}

/// The module carried in, dropped into the rig's 3D space: the rig with its track, as Add would make it.
pub fn dropped(b: &Bundle, carried: &Carry, modules: &[(Instrument, Descriptor)]) -> Result<(Bundle, String), String> {
    let m = modules.iter().find(|(_, d)| format!("module:{}", d.name) == carried.address).ok_or_else(|| format!("{} is not in the catalog any more", carried.address))?;
    adding::apply(b, &Choice::Module(Box::new(m.clone())))
}

/// at most this many lines show; the rest are counted
const SHOWN: usize = 8;

/// The list from the top of `area`: its heading, the lines (a press takes one in the hand) or why there is none; where
/// the lines landed and the height it took.
pub fn draw(d: &mut DrawList, t: &Tokens, area: Rect, character: &str, lines: &[Line], carried: Option<&str>, pointer: (f32, f32)) -> (Vec<(Rect, usize)>, f32) {
    let mut y = area.y;
    y += controls::section(d, t, area.x, y, area.w, &format!("MADE FOR {}", character.to_uppercase()), true);
    let (font, small, row) = (t.size.font, t.size.small, t.size.tree_row);
    let mut out = Vec::new();
    if lines.is_empty() {
        let room = (area.w / d.advance(small)).max(8.0) as usize;
        for part in deck_kit::editor::info::wrap(&none_yet(character), room) {
            d.text(area.x, y, small, &part, t.color.muted);
            y += small * 1.5;
        }
    }
    for (i, l) in lines.iter().enumerate() {
        if i == SHOWN || y + row > area.y + area.h {
            d.text(area.x, y + 2.0, small, &format!("{} more – Add finds them", lines.len() - i), t.color.muted);
            y += small * 1.5;
            break;
        }
        let r = Rect { x: area.x, y, w: area.w, h: row };
        if carried == Some(l.address.as_str()) {
            d.rect(r.x, r.y, r.w, r.h, t.color.selected);
        } else if r.has(pointer.0, pointer.1) {
            d.rect(r.x, r.y, r.w, r.h, t.color.hover);
        }
        let ty = y + (row - font * 1.25) * 0.5;
        d.text(area.x + 4.0, ty, font, &controls::fit(&l.label, ((area.w * 0.7) / d.advance(font)) as usize), t.color.text);
        let w = d.advance(small) * l.detail.chars().count() as f32;
        d.text(area.x + area.w - w - 4.0, ty + 1.0, small, l.detail, t.color.muted);
        out.push((r, i));
        y += row;
    }
    (out, y - area.y + t.size.row * 0.5)
}

/// the list for the rig open from the top of `area` (an item's rig: nothing); where its lines landed, the height it took
pub fn panel(d: &mut DrawList, t: &Tokens, area: Rect, b: &Bundle, rigs: &crate::rigs::Rigs, carried: Option<&str>, pointer: (f32, f32)) -> (Vec<(Rect, usize)>, f32) {
    let Some(c) = character(b) else { return (Vec::new(), 0.0) };
    draw(d, t, area, c, &lines(c, &rigs.modules().0), carried, pointer)
}

impl crate::App {
    /// the related lines of the rig open (none for an item's rig)
    pub(crate) fn related(&self) -> Vec<Line> {
        character(&self.stage.bundle).map(|c| lines(c, &self.rigs.modules().0)).unwrap_or_default()
    }

    /// a line pressed: its module goes into the hand
    pub(crate) fn related_press(&mut self, i: usize) {
        if let Some(l) = self.related().into_iter().nth(i) {
            self.carry = Some(Carry::new(&l.address, &l.label));
        }
    }

    /// The button let go while carrying a module: over the stage (no panel there) it becomes a track; elsewhere nothing.
    /// True = a module was carried.
    pub(crate) fn related_drop(&mut self, x: f32, y: f32) -> bool {
        let Some(carried) = self.carry.take() else { return false };
        if self.panels().floats.covers(x, y) {
            return true;
        }
        let modules = self.rigs.modules().0;
        let done = dropped(&self.stage.bundle, &carried, &modules);
        match done {
            Ok((next, words)) => self.change(Ok(next), words, x, y),
            Err(why) => self.say(why, x, y, true),
        }
        true
    }

    /// a check run's drag (`--related NAME[,drop]`): the line of NAME pressed, carried to the stage's middle, let go there
    pub(crate) fn script_related(&mut self) {
        const START: u64 = 60;
        let Some((name, drop)) = self.args.related.clone() else { return };
        match self.frames {
            f if f == START => {
                let line = self.related().iter().position(|l| l.label == name);
                match line.and_then(|i| self.panel.hits.iter().find(|(_, h)| *h == crate::Hit::Related(i))) {
                    Some((r, _)) => {
                        let (x, y) = (r.x + r.w * 0.3, r.y + r.h * 0.5);
                        self.pointer(x, y);
                        self.press();
                    }
                    None => tracing::error!("--related: no line {name} beside the rig"),
                }
            }
            f if f > START && f <= START + 20 => {
                let (w, h) = self.window_size();
                let k = (f - START) as f32 / 20.0;
                let (x0, y0) = self.cursor;
                self.pointer(x0 + (w * 0.42 - x0) * k, y0 + (h * 0.45 - y0) * k);
            }
            f if f == START + 30 && drop => self.release(),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn module(name: &str, characters: &[&str], out: &str) -> (Instrument, Descriptor) {
        assign::module_track(&json!({ "id": name, "descriptor": { "name": name, "characters": characters, "ports": [{ "name": "out", "dir": "out", "type": out }] } })).expect("it wraps")
    }

    fn catalog() -> Vec<(Instrument, Descriptor)> {
        vec![module("wing", &["yellowmouse"], "audio"), module("torso", &["catpope"], "audio"), module("steps", &["yellowmouse"], "event"), module("drone", &[], "audio")]
    }

    /// the panel model: the modules made for the rig's character, by name, what each sends; none = what to do instead
    #[test]
    fn the_list_shows_the_modules_made_for_the_character() {
        let b = Bundle::empty("character.yellowmouse");
        let c = character(&b).expect("a character's rig");
        let l = lines(c, &catalog());
        assert_eq!(l.iter().map(|l| (l.address.as_str(), l.detail)).collect::<Vec<_>>(), [("module:steps", "events"), ("module:wing", "sound")]);
        assert!(lines("boy", &catalog()).is_empty());
        assert_eq!(none_yet("boy"), "No module is made for boy yet. Every module still plugs in through Add.");
        assert_eq!(character(&Bundle::empty("item.lamp")), None);
    }

    /// a module carried in and dropped on the stage is a track of the rig, as Add makes it; the list stays
    #[test]
    fn a_module_dropped_into_the_rig_is_its_track() {
        let b = Bundle::empty("character.yellowmouse");
        let mut carried = Carry::new("module:wing", "wing");
        carried.to(900.0, 500.0);
        let (next, words) = dropped(&b, &carried, &catalog()).expect("it plugs in");
        assert_eq!(words, "track wing added, playing module:wing");
        assert_eq!(next.rig.tracks.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), ["wing"]);
        assert_eq!(next.stands_as(), None, "its being is its graph now");
        assert_eq!(lines("yellowmouse", &catalog()).len(), 2, "the list stays while plugging");
        assert!(dropped(&b, &Carry::new("module:gone", "gone"), &catalog()).unwrap_err().contains("not in the catalog"));
    }
}
