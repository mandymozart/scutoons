//! The colour picker over the editor (deck-kit's popover) and its eyedropper: which background a pick sets, this machine's
//! recent colours, the stage's pixel under the pointer. A pick goes to its owner through `Do`; the value is checked there.
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::colour::{self, Out, Part, Picker, Placed, Recent, Rgb};
use deck_kit::editor::controls::Rect;
use std::path::{Path, PathBuf};

/// whose background the picker sets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Rig,
    Scene,
    Show,
}

/// what the editor does with a press, a drag or a key in the picker
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Do {
    Nothing,
    /// the stage shows it while the area or the hue moves
    Preview(Target, Rgb),
    Set(Target, Rgb),
    /// type the value in its row
    Type(Target),
}

struct Open {
    target: Target,
    anchor: Rect,
    picker: Picker,
    opened_with: Option<Rgb>,
    set: Option<Rgb>,
}

#[derive(Default)]
pub struct Colour {
    open: Option<Open>,
    placed: Placed,
    /// the eyedropper is armed for this target
    pub dropper: Option<Target>,
    /// the stage's pixel under the pointer, read after the last frame
    pub under: Option<Rgb>,
    pub recent: Recent,
    file: Option<PathBuf>,
}

/// the editor cache's file of the recent colours
pub fn file(cache: &Path) -> PathBuf {
    cache.join("editor-colours.json")
}

impl Colour {
    /// the recent colours kept in `file` (none: a check run's, kept for the run only)
    pub fn load(file: Option<PathBuf>) -> Colour {
        let list: Vec<String> = file.as_deref().and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        let mut recent = Recent(list.iter().filter_map(|h| deck_catalog::background::rgb(h)).map(colour::bytes).collect());
        recent.0.truncate(Recent::MAX);
        Colour { recent, file, ..Colour::default() }
    }

    pub fn open(&mut self, target: Target, anchor: Rect, value: Option<Rgb>) {
        self.close();
        self.dropper = None;
        self.open = Some(Open { target, anchor, picker: Picker::new(value), opened_with: value, set: None });
    }

    pub fn target(&self) -> Option<Target> {
        self.open.as_ref().map(|o| o.target).or(self.dropper)
    }

    /// the popover or the eyedropper has the pointer
    pub fn taking(&self) -> bool {
        self.open.is_some() || self.dropper.is_some()
    }

    pub fn dragging(&self) -> bool {
        self.open.as_ref().is_some_and(|o| o.picker.dragging())
    }

    /// a part's place in the popover drawn last (check runs)
    pub fn part(&self, part: impl Fn(&Part) -> bool) -> Option<Rect> {
        self.placed.parts.iter().find(|(_, p)| part(p)).map(|(r, _)| *r)
    }

    /// closed: a colour it set that differs from the one it opened with is a recent one
    pub fn close(&mut self) {
        if let Some(c) = self.open.take().and_then(|o| o.set.filter(|c| Some(*c) != o.opened_with)) {
            self.remember(c);
        }
        self.placed = Placed::default();
    }

    fn remember(&mut self, c: Rgb) {
        self.recent.remember(c);
        if let Some(f) = &self.file {
            let list: Vec<String> = self.recent.0.iter().map(|c| colour::hex(*c)).collect();
            if let Ok(json) = serde_json::to_string(&list) {
                let _ = std::fs::write(f, json);
            }
        }
    }

    /// The popover, or the eyedropper at the pointer; `value` is the target's colour now.
    pub fn draw(&mut self, d: &mut DrawList, t: &Tokens, room: (f32, f32), value: Option<Rgb>, pointer: (f32, f32)) {
        if self.dropper.is_some() {
            colour::dropper(d, t, pointer, room, self.under);
        }
        let Some(o) = self.open.as_mut() else { return };
        o.picker.follow(value);
        self.placed = colour::popover(d, t, (o.anchor, room), &o.picker, &self.recent.0, value, pointer);
    }

    pub fn press(&mut self, x: f32, y: f32) -> Do {
        if let Some(target) = self.dropper.take() {
            return match self.under {
                Some(c) => {
                    self.remember(c);
                    Do::Set(target, c)
                }
                None => Do::Nothing,
            };
        }
        let Some(o) = self.open.as_mut() else { return Do::Nothing };
        if !self.placed.whole.has(x, y) {
            self.close();
            return Do::Nothing;
        }
        let target = o.target;
        match o.picker.press(&self.placed, x, y) {
            Some(Out::Pick(c)) => {
                o.set = Some(c);
                Do::Set(target, c)
            }
            Some(Out::Moving(c)) => Do::Preview(target, c),
            Some(Out::Hex) => {
                self.close();
                Do::Type(target)
            }
            Some(Out::Dropper) => {
                self.close();
                self.dropper = Some(target);
                self.under = None;
                Do::Nothing
            }
            None => Do::Nothing,
        }
    }

    pub fn drag_to(&mut self, x: f32, y: f32) -> Do {
        let Some(o) = self.open.as_mut() else { return Do::Nothing };
        match o.picker.drag_to(&self.placed, x, y) {
            Some(Out::Moving(c)) => Do::Preview(o.target, c),
            _ => Do::Nothing,
        }
    }

    pub fn release(&mut self) -> Do {
        let Some(o) = self.open.as_mut() else { return Do::Nothing };
        match o.picker.release() {
            Some(Out::Pick(c)) => {
                o.set = Some(c);
                Do::Set(o.target, c)
            }
            _ => Do::Nothing,
        }
    }

    /// Esc: the eyedropper or the popover goes; whether there was one
    pub fn escape(&mut self) -> bool {
        let was = self.taking();
        self.dropper = None;
        self.close();
        was
    }
}

/// the colour the target's background has now; none: unset, an image or a video
pub fn value(target: Target, stage: &crate::stage::Stage, scene: Option<&crate::scene::SceneView>, show: &crate::show::Host) -> Option<Rgb> {
    let b = match target {
        Target::Rig => stage.bundle.rig.background.as_ref(),
        Target::Scene => scene.and_then(|v| v.edit.doc.stage.background.as_ref()),
        Target::Show => show.shown().background.as_ref(),
    };
    b.and_then(deck_space::Background::rgb).map(colour::bytes)
}

impl crate::App {
    /// the picker under the swatch the pointer is on
    pub(crate) fn colour_open(&mut self, target: Target) {
        let (x, y) = self.cursor;
        let anchor = match target {
            Target::Rig => self.panel.hits.iter().find(|(r, h)| *h == crate::Hit::BackgroundColour && r.has(x, y)).map(|(r, _)| *r),
            Target::Scene => self.panel.hits.iter().find(|(r, h)| *h == crate::Hit::SceneField(crate::scene::Field::BackgroundColour) && r.has(x, y)).map(|(r, _)| *r),
            Target::Show => self.show.hits.iter().find(|(r, h)| *h == crate::show::Hit::BackgroundColour && r.has(x, y)).map(|(r, _)| *r),
        };
        let v = value(target, &self.stage, self.scene.as_ref(), &self.show);
        self.colour.open(target, anchor.unwrap_or(Rect { x, y, w: 0.0, h: 0.0 }), v);
    }

    pub(crate) fn colour_do(&mut self, d: Do) {
        match d {
            Do::Nothing => {}
            Do::Preview(_, c) => self.stage.space.set_background(Some(deck_space::Background::colour(&colour::hex(c)))),
            Do::Set(target, c) => {
                let text = colour::hex(c);
                match target {
                    Target::Rig => self.rig_background(&text),
                    Target::Scene => self.scene_background(&text),
                    Target::Show => self.show.set_background(&text),
                }
            }
            Do::Type(Target::Rig) => {
                let now = self.stage.bundle.rig.background.as_ref().map_or("", |b| b.text()).to_string();
                self.begin_edit(crate::Editing::Background, crate::Hit::Background, now);
            }
            Do::Type(Target::Scene) => {
                let now = self.scene.as_ref().and_then(|v| v.edit.doc.stage.background.as_ref()).map_or("", |b| b.text()).to_string();
                self.begin_edit(crate::Editing::SceneBackground, crate::Hit::SceneField(crate::scene::Field::Background), now);
            }
            Do::Type(Target::Show) => self.show.press(crate::show::Hit::Background),
        }
    }

    /// A check run's picker on the background shown (`--colour`): `open` opens it; `pick` takes a palette colour, closes
    /// (a recent one), opens again and drags the area; `drop:X,Y` holds the eyedropper at X,Y; `take:X,Y` takes it there.
    pub(crate) fn script_colour(&mut self) {
        let Some(spec) = self.args.colour.clone() else { return };
        let (what, at) = match spec.split_once(':') {
            Some((w, a)) => (w, a.split_once(',').and_then(|(x, y)| Some((x.parse::<f32>().ok()?, y.parse::<f32>().ok()?)))),
            None => (spec.as_str(), None),
        };
        let press_at = |app: &mut crate::App, r: Rect, (fx, fy): (f32, f32)| {
            app.pointer(r.x + r.w * fx, r.y + r.h * fy);
            app.press();
        };
        let open = |app: &mut crate::App| {
            let swatch = app.panel.hits.iter().find(|(_, h)| matches!(h, crate::Hit::BackgroundColour | crate::Hit::SceneField(crate::scene::Field::BackgroundColour))).map(|(r, _)| *r);
            let swatch = swatch.or_else(|| app.show.hits.iter().find(|(_, h)| *h == crate::show::Hit::BackgroundColour).map(|(r, _)| *r));
            match swatch {
                Some(r) => press_at(app, r, (0.5, 0.5)),
                None => tracing::error!("--colour: no background swatch shown"),
            }
        };
        let f = self.frames;
        match (what, f) {
            (_, 60) => open(self),
            ("pick", 70) => {
                if let Some(r) = self.colour.part(|p| *p == Part::Swatch(colour::PALETTE[1].1)) {
                    press_at(self, r, (0.5, 0.5));
                }
            }
            ("pick", 74) => {
                self.colour.escape();
            }
            ("pick", 78) => open(self),
            ("pick", 84) => {
                if let Some(r) = self.colour.part(|p| *p == Part::Area) {
                    press_at(self, r, (0.7, 0.3));
                }
            }
            ("pick", 88) => {
                if let Some(r) = self.colour.part(|p| *p == Part::Area) {
                    self.pointer(r.x + r.w * 0.8, r.y + r.h * 0.25);
                    self.release();
                }
            }
            ("drop" | "take", 70) => {
                if let Some(r) = self.colour.part(|p| *p == Part::Dropper) {
                    press_at(self, r, (0.5, 0.5));
                }
            }
            ("drop" | "take", f) if f > 70 && self.colour.dropper.is_some() => {
                if let Some((x, y)) = at {
                    self.pointer(x, y);
                }
                if what == "take" && f >= 95 {
                    let (target, under) = (self.colour.dropper, self.colour.under);
                    self.press();
                    let now = target.and_then(|t| value(t, &self.stage, self.scene.as_ref(), &self.show));
                    tracing::info!("colour: took {:?} from the stage at {:?} for {target:?}; its background now {:?}", under.map(colour::hex), at, now.map(colour::hex));
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drawn(c: &mut Colour, value: Option<Rgb>) {
        c.draw(&mut DrawList::new(0.6), &Tokens::default(), (1920.0, 1080.0), value, (0.0, 0.0));
    }

    fn press_part(c: &mut Colour, part: impl Fn(&Part) -> bool) -> Do {
        let r = c.part(part).expect("drawn");
        c.press(r.x + 1.0, r.y + 1.0)
    }

    #[test]
    fn a_swatch_sets_and_a_changed_colour_is_remembered_when_it_closes() {
        let mut c = Colour::default();
        c.open(Target::Scene, Rect { x: 10.0, y: 10.0, w: 20.0, h: 20.0 }, Some([0, 0, 0]));
        drawn(&mut c, Some([0, 0, 0]));
        let green = colour::PALETTE[0].1;
        assert_eq!(press_part(&mut c, |p| *p == Part::Swatch(green)), Do::Set(Target::Scene, green));
        assert!(c.recent.0.is_empty(), "not while it is open");
        assert_eq!(c.press(1900.0, 1070.0), Do::Nothing, "a press outside closes it");
        assert!(!c.taking());
        assert_eq!(c.recent.0, [green]);
    }

    #[test]
    fn the_area_previews_and_sets_when_let_go() {
        let mut c = Colour::default();
        c.open(Target::Rig, Rect { x: 10.0, y: 10.0, w: 20.0, h: 20.0 }, Some([255, 0, 0]));
        drawn(&mut c, Some([255, 0, 0]));
        let area = c.part(|p| *p == Part::Area).unwrap();
        assert!(matches!(c.press(area.x + 1.0, area.y + 1.0), Do::Preview(Target::Rig, _)));
        assert!(matches!(c.drag_to(area.x + area.w, area.y + area.h), Do::Preview(Target::Rig, [0, 0, 0])));
        assert_eq!(c.release(), Do::Set(Target::Rig, [0, 0, 0]));
        assert_eq!(c.release(), Do::Nothing);
    }

    #[test]
    fn the_eyedropper_takes_the_stage_pixel_and_esc_cancels_it() {
        let mut c = Colour::default();
        c.open(Target::Show, Rect { x: 10.0, y: 10.0, w: 20.0, h: 20.0 }, None);
        drawn(&mut c, None);
        assert_eq!(press_part(&mut c, |p| *p == Part::Dropper), Do::Nothing);
        assert_eq!((c.dropper, c.target()), (Some(Target::Show), Some(Target::Show)));
        assert_eq!(c.press(500.0, 500.0), Do::Nothing, "nothing read yet");
        c.dropper = Some(Target::Show);
        c.under = Some([0x33, 0x66, 0x99]);
        assert_eq!(c.press(500.0, 500.0), Do::Set(Target::Show, [0x33, 0x66, 0x99]));
        assert_eq!(c.recent.0, [[0x33, 0x66, 0x99]], "a stage pick is a recent one at once");
        c.dropper = Some(Target::Show);
        assert!(c.escape() && c.dropper.is_none());
        assert!(!c.escape());
    }

    #[test]
    fn the_hex_types_in_the_row_and_recent_colours_last_on_disk() {
        let dir = std::env::temp_dir().join(format!("deck-editor-colours-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = file(&dir);
        let mut c = Colour::load(Some(f.clone()));
        c.open(Target::Rig, Rect::default(), None);
        drawn(&mut c, None);
        assert_eq!(press_part(&mut c, |p| *p == Part::Hex), Do::Type(Target::Rig));
        assert!(!c.taking());
        c.remember([1, 2, 3]);
        assert_eq!(Colour::load(Some(f)).recent.0, [[1, 2, 3]]);
        let _ = std::fs::remove_dir_all(dir);
    }
}
