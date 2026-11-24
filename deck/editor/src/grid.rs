//! Build's grid & snap: its button in the tools pill (tools.rs) opens its popover (the grid off, on the floor or on a wall,
//! its step, snap and what it snaps to, the move and rotate steps); the grid is drawn over the stage in the editor only;
//! a scene member dragged on the stage snaps (deck-space snap.rs, space/place.rs) and its offset goes into the scene.
//! Kept per view (a rig, a scene) in the editor's cache.
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_core::units::{self, Refs, Unit};
use deck_kit::editor::controls::{self, Rect};
use deck_kit::editor::units as unit_field;
use deck_space::scene_edit::Part;
use deck_space::snap::{Guide, Plane, Precision, Settings, Target, SNAP_PX, TARGETS};
use deck_space::space::place::{Grab, ScreenLine};
use deck_space::space::Space;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// how far the grid reaches around where the camera looks (m)
const REACH: f32 = 12.0;
/// a press that moved less than this (logical px) is a click: it selects the member
const CLICK: f32 = 4.0;
/// a scrubbed number doubles every this many pixels
const DOUBLING_PX: f32 = 80.0;
const POPOVER_W: f32 = 300.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Field {
    Grid(Option<Plane>),
    Step,
    Snap,
    To(Target),
    Move,
    Rotate,
    /// the steps read in this unit (m, cm, ft)
    Unit(Unit),
}

impl Field {
    fn precision(self) -> Option<Precision> {
        match self {
            Field::Step => Some(Precision::Grid),
            Field::Move => Some(Precision::Move),
            Field::Rotate => Some(Precision::Rotate),
            _ => None,
        }
    }
}

/// a member in the hand on the stage
struct Held {
    grab: Grab,
    from: (f32, f32),
    moved: bool,
    at: Option<[f32; 3]>,
    guides: Vec<Guide>,
}

pub struct GridSnap {
    views: BTreeMap<String, Settings>,
    file: Option<PathBuf>,
    key: String,
    open: bool,
    pill: Option<Rect>,
    fields: Vec<(Rect, Field)>,
    /// a number being scrubbed: which, the pointer's x and the value when it started
    scrub: Option<(Field, f32, f32)>,
    held: Option<Held>,
    /// the unit a press on a step's switch chose, for the app to keep
    picked: Option<Unit>,
}

impl GridSnap {
    /// the views' settings kept in `file` (none yet: every view starts with the defaults)
    pub fn load(file: Option<PathBuf>) -> GridSnap {
        let views = file.as_deref().and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        GridSnap { views, file, key: String::new(), open: false, pill: None, fields: Vec::new(), scrub: None, held: None, picked: None }
    }

    pub fn settings(&self) -> Settings {
        self.views.get(&self.key).cloned().unwrap_or_default()
    }

    fn change(&mut self, f: impl FnOnce(&mut Settings)) {
        let mut s = self.settings();
        f(&mut s);
        self.views.insert(self.key.clone(), s);
        self.keep();
    }

    fn keep(&self) {
        let Some(file) = &self.file else { return };
        if let Ok(text) = serde_json::to_string_pretty(&self.views) {
            if let Err(e) = std::fs::write(file, text) {
                tracing::warn!("grid: {}: {e}", file.display());
            }
        }
    }

    /// The view shown (`rig:<subject>`, `scene:<id>`): a view seen the first time lays its grid's origin on the stage's
    /// floor. true = another view than before.
    pub fn view(&mut self, key: &str, space: &Space) -> bool {
        if self.key == key {
            return false;
        }
        self.key = key.to_string();
        if !self.views.contains_key(key) {
            let origin = space.floor_point().to_array();
            self.views.insert(key.to_string(), Settings { origin, ..Settings::default() });
        }
        true
    }

    /// a check run's `--grid floor|wall|off[,snap][,open]`
    pub fn script(&mut self, spec: &str) {
        for word in spec.split(',') {
            match word {
                "floor" => self.change(|s| (s.grid, s.plane) = (true, Plane::Floor)),
                "wall" => self.change(|s| (s.grid, s.plane) = (true, Plane::Wall)),
                "off" => self.change(|s| s.grid = false),
                "snap" => self.change(|s| s.snap = true),
                "open" => self.open = true,
                _ => tracing::warn!("--grid: '{word}' – floor, wall, off, snap, open"),
            }
        }
    }

    /// The grid over the stage (`area`, logical pixels; the space draws at `scale` pixels per logical one), and the
    /// lines a member's drag snapped to.
    pub fn draw_stage(&self, ui: &mut DrawList, t: &Tokens, space: &Space, area: Rect, scale: f32) {
        let s = self.settings();
        let to_area = |l: &ScreenLine| clip(area, (area.x + l.0 .0 / scale, area.y + l.0 .1 / scale), (area.x + l.1 .0 / scale, area.y + l.1 .1 / scale));
        let [r, g, b, _] = t.color.bright;
        for l in space.grid_pixels(&s, REACH) {
            if let Some((a, z)) = to_area(&l) {
                ui.segment(a, z, 1.0, if l.2 { [r, g, b, 0.5] } else { [r, g, b, 0.2] });
            }
        }
        if let Some(h) = &self.held {
            for l in space.guide_pixels(h.grab.member, &h.guides, REACH) {
                if let Some((a, b)) = to_area(&l) {
                    ui.segment(a, b, 1.5, t.color.danger);
                }
            }
        }
    }

    /// where its button is this frame (none: not shown, the popover closes)
    pub fn anchor(&mut self, at: Option<Rect>) {
        self.pill = at;
        if at.is_none() {
            self.open = false;
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// The popover from its button, while open, over everything else: under it, or over it when the window (`room`) has
    /// no place below.
    pub fn popover(&mut self, ui: &mut DrawList, t: &Tokens, room: (f32, f32), distance: Unit) {
        self.fields.clear();
        let Some(pill) = self.pill.filter(|_| self.open) else { return };
        let s = self.settings();
        let (pad, row) = (t.size.pad, t.size.row);
        let w = POPOVER_W;
        let x = (pill.x + pill.w - w).min(room.0 - w - pad).max(pad);
        let rows = 8.0;
        let below = pill.y + pill.h + pad * 0.5;
        let whole = rows * row + 2.0 * pad;
        let top = if below + whole <= room.1 { below } else { (pill.y - pad * 0.5 - whole).max(pad) };
        ui.round_rect(x, top, w, rows * row + 2.0 * pad, t.size.radius, t.color.surface_head);
        ui.outline(x, top, w, rows * row + 2.0 * pad, (t.size.radius, 1.0), t.color.line);
        let (ix, iw) = (x + pad, w - 2.0 * pad);
        let mut y = top + pad;
        let label = |ui: &mut DrawList, y: f32, text: &str| ui.text(ix, y + (row - t.size.font * 1.25) * 0.5, t.size.font, text, t.color.muted);
        label(ui, y, "Grid");
        let lx = ix + iw * 0.42;
        let grid = [(None, "off"), (Some(Plane::Floor), "floor"), (Some(Plane::Wall), "wall")];
        let on: Vec<bool> = grid.iter().map(|(p, _)| if s.grid { *p == Some(s.plane) } else { p.is_none() }).collect();
        let (rects, _) = controls::chips(ui, t, (lx, y + 3.0, iw * 0.58), &grid.map(|(_, n)| n), &on);
        self.fields.extend(rects.into_iter().zip(grid).map(|(r, (p, _))| (r, Field::Grid(p))));
        y += row;
        let hot = self.scrub.map(|(f, ..)| f);
        let number = |ui: &mut DrawList, y: f32, label: &str, field: Field, value: f32, unit: &str| {
            let (min, max) = field.precision().map_or((1.0, 2.0), Precision::limits);
            let fraction = (value / min).ln() / (max / min).ln();
            (controls::scrub(ui, t, Rect { x: ix, y, w: iw, h: row }, label, &format!("{}{unit}", crate::number(value)), fraction, hot == Some(field)), field)
        };
        let offered = Unit::Metres.dimension().offered(&Refs::default());
        let metres = |ui: &mut DrawList, y: f32, label: &str, field: Field, value: f32| {
            let (min, max) = field.precision().map_or((1.0, 2.0), Precision::limits);
            let fraction = (value / min).ln() / (max / min).ln();
            let shown = units::convert(f64::from(value), Unit::Metres, distance, &Refs::default()).unwrap_or(f64::from(value));
            let (r, segs) = unit_field::scrub(ui, t, Rect { x: ix, y, w: iw, h: row }, label, shown, distance, &offered, fraction, hot == Some(field));
            (r, field, segs)
        };
        let (r, f, segs) = metres(ui, y, "Step", Field::Step, s.step);
        self.fields.push((r, f));
        self.fields.extend(segs.into_iter().map(|(r, u)| (r, Field::Unit(u))));
        y += row;
        label(ui, y, "Snap");
        let (rects, _) = controls::chips(ui, t, (lx, y + 3.0, iw * 0.58), &["off", "on"], &[!s.snap, s.snap]);
        self.fields.extend(rects.into_iter().enumerate().filter(|(i, _)| (*i == 1) != s.snap).map(|(_, r)| (r, Field::Snap)));
        y += row;
        label(ui, y, "Snap to");
        let on: Vec<bool> = TARGETS.iter().map(|t| s.to.contains(t)).collect();
        let names = ["grid", "axes", "objects", "outlines"];
        let (rects, used) = controls::chips(ui, t, (lx, y + 3.0, iw * 0.58), &names, &on);
        self.fields.extend(rects.into_iter().zip(TARGETS).map(|(r, target)| (r, Field::To(target))));
        y += used.max(row) + 4.0;
        let (r, f, segs) = metres(ui, y, "Move step", Field::Move, s.move_step);
        self.fields.push((r, f));
        self.fields.extend(segs.into_iter().map(|(r, u)| (r, Field::Unit(u))));
        y += row;
        self.fields.push(number(ui, y, "Rotate", Field::Rotate, s.rotate, "°"));
        y += row;
        ui.text(ix, y + (row - t.size.small * 1.25) * 0.5, t.size.small, "Shift / Ctrl: snap the other way", t.color.muted);
    }

    /// A press at (`x`, `y`) while the popover is open: a field of it is set or starts a scrub; a press elsewhere closes
    /// it. true = the press was the grid's.
    pub fn press(&mut self, x: f32, y: f32) -> bool {
        if !self.open {
            return false;
        }
        let Some(&(_, field)) = self.fields.iter().find(|(r, _)| r.has(x, y)) else {
            self.open = false;
            return false;
        };
        let s = self.settings();
        match field {
            Field::Grid(None) => self.change(|s| s.grid = false),
            Field::Grid(Some(p)) => self.change(|s| (s.grid, s.plane) = (true, p)),
            Field::Snap => self.change(|s| s.snap = !s.snap),
            Field::To(target) => self.change(|s| s.toggle(target)),
            Field::Step => self.scrub = Some((field, x, s.step)),
            Field::Move => self.scrub = Some((field, x, s.move_step)),
            Field::Rotate => self.scrub = Some((field, x, s.rotate)),
            Field::Unit(u) => self.picked = Some(u),
        }
        true
    }

    /// the pointer at `x` while a number is scrubbed: true = it was
    pub fn scrub_to(&mut self, x: f32) -> bool {
        let Some((field, x0, v0)) = self.scrub else { return false };
        let Some(p) = field.precision() else { return false };
        let v = p.clamp(v0 * ((x - x0) / DOUBLING_PX).exp2());
        let v = if field == Field::Rotate { v.round() } else { (v * 100.0).round() / 100.0 };
        let v = p.clamp(v);
        self.change(|s| match field {
            Field::Step => s.step = v,
            Field::Move => s.move_step = v,
            _ => s.rotate = v,
        });
        true
    }

    /// the unit a press on a step's switch chose since last asked
    pub fn take_unit(&mut self) -> Option<Unit> {
        self.picked.take()
    }

    /// the button let go: a scrub ends; true = there was one
    pub fn scrub_end(&mut self) -> bool {
        self.scrub.take().is_some()
    }

    /// A press on the stage at view pixel (`x`, `y`) (logical `at`): a member under it goes into the hand. true = it did.
    pub fn grab(&mut self, space: &Space, x: f32, y: f32, at: (f32, f32)) -> bool {
        self.held = space.grab_member(x, y).map(|grab| Held { grab, from: at, moved: false, at: None, guides: Vec::new() });
        self.held.is_some()
    }

    /// The member in the hand to view pixel (`x`, `y`) (logical `at`), snapped unless `flip` (Shift / Ctrl) turns it
    /// around; `scale` = view pixels per logical one.
    pub fn drag(&mut self, space: &mut Space, x: f32, y: f32, at: (f32, f32), flip: bool, scale: f32) {
        let s = self.settings();
        let Some(h) = self.held.as_mut() else { return };
        h.moved |= (at.0 - h.from.0).abs() + (at.1 - h.from.1).abs() > CLICK;
        if !h.moved {
            return;
        }
        if let Some((moved_to, guides)) = space.drag_member(h.grab, x, y, &s, s.active(flip), SNAP_PX * scale) {
            (h.at, h.guides) = (Some(moved_to), guides);
        }
    }

    /// The button let go: the member in the hand, and its new offset from its spot if it moved.
    pub fn drop(&mut self) -> Option<(usize, Option<[f32; 3]>)> {
        self.held.take().map(|h| (h.grab.member, h.at.filter(|_| h.moved)))
    }
}

/// the segment `a`–`b` inside `r` (Liang–Barsky), None = all of it outside
fn clip(r: Rect, a: (f32, f32), b: (f32, f32)) -> Option<((f32, f32), (f32, f32))> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for (p, q) in [(-dx, a.0 - r.x), (dx, r.x + r.w - a.0), (-dy, a.1 - r.y), (dy, r.y + r.h - a.1)] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    (t0 <= t1).then_some(((a.0 + t0 * dx, a.1 + t0 * dy), (a.0 + t1 * dx, a.1 + t1 * dy)))
}

/// the editor's cache file of the views' grid settings
pub fn cache_file(cache: &Path) -> PathBuf {
    cache.join("editor-grid.json")
}

impl GridSnap {
    /// the view whose grid is shown: the scene open, else the rig; a check run's `script` on each view it shows
    pub fn follow(&mut self, scene: Option<&crate::scene::SceneView>, stage: &crate::stage::Stage, script: Option<&str>) {
        let key = scene.map_or_else(|| format!("rig:{}", stage.bundle.rig.subject), |v| v.edit.key());
        if self.view(&key, &stage.space) {
            if let Some(spec) = script {
                self.script(spec);
            }
        }
    }
}

impl crate::App {
    /// view pixels of the stage at window point (`x`, `y`): the stage fills the window
    pub(crate) fn stage_pixel(&self, x: f32, y: f32) -> (f32, f32) {
        let s = self.scale();
        (x * s, y * s)
    }

    /// A press on the stage of a scene: a member under it goes into the hand. true = it did.
    pub(crate) fn grid_grab(&mut self, x: f32, y: f32) -> bool {
        if self.scene.as_ref().is_none_or(|v| v.inside.is_some()) {
            return false;
        }
        let (sx, sy) = self.stage_pixel(x, y);
        self.grid.grab(&self.stage.space, sx, sy, (x, y))
    }

    pub(crate) fn grid_drag(&mut self, x: f32, y: f32) {
        let (sx, sy) = self.stage_pixel(x, y);
        let flip = self.mods.shift_key() || self.mods.control_key();
        let scale = self.scale();
        self.grid.drag(&mut self.stage.space, sx, sy, (x, y), flip, scale);
    }

    /// The member let go: moved, its offset goes into the scene and the stage plays it; not moved, it is selected.
    pub(crate) fn grid_drop(&mut self) {
        let Some((member, at)) = self.grid.drop() else { return };
        let Some(holder) = self.stage.space.cast.get(member).map(|c| c.holder.clone()) else { return };
        let Some(view) = self.scene.as_mut() else { return };
        match at {
            Some(at) => {
                if let Err(why) = view.edit.move_holder(&holder, at) {
                    let (x, y) = self.cursor;
                    self.refuse(why, x, y);
                }
                self.replay_scene();
            }
            None => {
                if let Some(m) = view.edit.members.iter().position(|m| m.holder == holder) {
                    view.sel = Part::Member(m);
                }
                self.stage.space.pick(Some(member));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_is_cut_to_the_stage() {
        let r = Rect { x: 0.0, y: 0.0, w: 100.0, h: 50.0 };
        assert_eq!(clip(r, (-50.0, 25.0), (150.0, 25.0)), Some(((0.0, 25.0), (100.0, 25.0))));
        assert_eq!(clip(r, (10.0, 10.0), (20.0, 20.0)), Some(((10.0, 10.0), (20.0, 20.0))));
        assert_eq!(clip(r, (-10.0, -10.0), (-5.0, 60.0)), None);
    }

    #[test]
    fn the_popover_sets_the_views_settings_and_keeps_them() {
        let file = std::env::temp_dir().join(format!("deck-editor-grid-{}.json", std::process::id()));
        let mut g = GridSnap::load(Some(file.clone()));
        g.key = "scene:duet".into();
        g.fields = vec![
            (Rect { x: 0.0, y: 0.0, w: 10.0, h: 10.0 }, Field::Grid(Some(Plane::Wall))),
            (Rect { x: 20.0, y: 0.0, w: 10.0, h: 10.0 }, Field::Snap),
            (Rect { x: 40.0, y: 0.0, w: 10.0, h: 10.0 }, Field::To(Target::Objects)),
            (Rect { x: 60.0, y: 0.0, w: 10.0, h: 10.0 }, Field::Step),
        ];
        g.open = true;
        assert!(g.press(5.0, 5.0) && g.press(25.0, 5.0) && g.press(45.0, 5.0));
        let s = g.settings();
        assert_eq!((s.plane, s.snap), (Plane::Wall, true));
        assert!(!s.to.contains(&Target::Objects));
        // the step scrubbed: one doubling to the right
        assert!(g.press(65.0, 5.0));
        assert!(g.scrub_to(65.0 + DOUBLING_PX));
        assert!(g.scrub_end());
        assert_eq!(g.settings().step, 1.0);
        // another view keeps its own; the file brings them back
        g.key = "rig:character.catpope".into();
        assert_eq!(g.settings(), Settings::default());
        let back = GridSnap::load(Some(file.clone()));
        assert_eq!(back.views.get("scene:duet").map(|s| (s.plane, s.step)), Some((Plane::Wall, 1.0)));
        // a press elsewhere closes it and is not the grid's
        assert!(!g.press(500.0, 500.0));
        assert!(!g.open);
        let _ = std::fs::remove_file(file);
    }
}
