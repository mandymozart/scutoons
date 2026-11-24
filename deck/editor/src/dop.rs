//! Build's camera tools (the DOP panel): the scene's primary camera – its source, then the sections Lens · Focus · Height
//! · Shots · Move · Track · Phones, each one line (a line icon, its name, what it is set to) folded until it is opened,
//! the open ones remembered per tab (`Sel::open`). The camera, its checks, the moves and their groups are the core's
//! (deck-space scene_camera.rs, scene_dop.rs, space/dop); the icons and the folding sections are the kit's
//! (editor/icons.rs, editor/fold.rs); here the panel is laid out and its presses go to the core.
use crate::scene::SceneView;
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_core::units::{self, Refs, Unit};
use deck_kit::editor::units as unit_field;
use deck_kit::editor::{fold, icons};
use deck_space::scene_camera::{fov_of, height_preset, Ease, EYE_HEIGHTS, Length, Lens, MoveKind, MoveSpec, Projection, Repeat, SceneCamera, Side, Source, Start, Turn, FOCAL_PRESETS, FOCUS_RANGE, MAX_ROUNDS, SIZE_RANGE};
use deck_space::space::dop::View;
use deck_ui::filter::Condition;

pub const WIDTH: f32 = 384.0;
/// how near (logical px) the focus marker is let go to a subject to focus on it
const SUBJECT_REACH: f32 = 40.0;

/// what the panel has selected and the move it is making
#[derive(Debug, Clone, PartialEq)]
pub struct Sel {
    pub shot: Option<usize>,
    pub point: Option<usize>,
    pub kept: Option<usize>,
    pub draft: MoveSpec,
    /// the stage's floor (m): eye heights stand on it
    pub floor: f32,
    /// what a shot can aim at on the stage (Space `aim_choices`)
    pub aims: Vec<String>,
    /// a phone's take being recorded (take.rs)
    pub rec: Option<crate::take::Rec>,
    /// the sections open (`Part::bit`); none at first
    pub open: u8,
}

impl Default for Sel {
    fn default() -> Sel {
        Sel { shot: None, point: None, kept: None, draft: MoveSpec::of(MoveKind::Glide), floor: 0.0, aims: Vec::new(), rec: None, open: 0 }
    }
}

/// a number dragged as a scrub bar
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Num {
    Size,
    Length,
    Rounds,
    Height,
    Amount,
    /// the selected track point's arrival time
    PointTime,
    /// the lens's focus distance
    Focus,
    /// how long a phone holding the camera may go without a pose (s; 0 = no limit)
    Release,
}

impl Num {
    /// min, max, log
    fn range(self) -> (f32, f32, bool) {
        match self {
            Num::Size => (SIZE_RANGE.0, 50.0, true),
            Num::Length => (0.25, 32.0, true),
            Num::Rounds => (0.0, MAX_ROUNDS, false),
            Num::Height => (0.0, 10.0, false),
            Num::Amount => (0.0, 10.0, false),
            Num::PointTime => (0.25, 64.0, true),
            Num::Focus => (0.1, 100.0, true),
            Num::Release => (0.0, 120.0, false),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Field {
    Source(Source),
    Projection(Projection),
    Preset(usize),
    /// the focal length typed
    Focal,
    /// the focal length read in this unit (mm, or the degrees it sees)
    FocalUnit(Unit),
    /// the distances read in this unit (m, cm, ft)
    DistanceUnit(Unit),
    /// an eye height preset, or typed
    Height(usize),
    HeightTyped,
    Scrub(Num),
    Shot(usize),
    AddShot,
    SetShot,
    RemoveShot,
    Kind(usize),
    Unit(usize),
    Ease(usize),
    Start(Start),
    Repeat(Repeat),
    Turn(Turn),
    Side(Side),
    Follow(usize),
    /// what the selected shot aims at: none = its own point, else `Sel::aims`'s
    Aim(Option<usize>),
    /// the focus at the aim (no distance, no subject)
    FocusAim,
    /// the selected shot's focus on `Sel::aims`'s subject
    FocusOn(usize),
    Preview,
    Keep,
    Kept(usize),
    PlayKept,
    RemoveKept,
    AddTrack,
    AddPoint,
    RemovePoint,
    RemoveTrack,
    /// a track point picked from the list
    Point(usize),
    /// the selected point's time: by the track's length (none), or counted in `Length::UNITS[i]`
    PointUnit(Option<usize>),
    /// the ease of the way to the selected point: even (none), or `POINT_EASES[i]`
    PointEase(Option<usize>),
    /// phones may take the selected shot's camera (none selected: the primary camera), or no longer
    Phones(bool),
    /// a chip of who may take it (the one condition control's fact `i`)
    PhonesFact(usize),
    /// record the selected camera's phone as a take; pressed while recording: keep it
    Record,
    Discard,
    /// a section's head: open or close it; with Alt, all of them
    Fold(Part),
    /// the camera's slot `i` (a module drops on it), its module taken off
    Slot(usize),
    Detach(usize),
}

/// the kind of value a focal length is (its unit is chosen once: the editor's settings)
pub const FOCAL_UNIT: &str = "focal";
/// the kind of value a distance in the world is: a view height, a focus distance, a crane's height, an eye height
pub const DISTANCE_UNIT: &str = "distance";

/// the units the panel reads its values in (the editor's settings)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Units {
    pub focal: Unit,
    pub distance: Unit,
}

impl Default for Units {
    fn default() -> Units {
        Units { focal: Unit::Millimetres, distance: Unit::Metres }
    }
}

impl Units {
    pub fn of(prefs: &unit_field::Prefs) -> Units {
        let none = Refs::default();
        Units { focal: prefs.unit(FOCAL_UNIT, Unit::Millimetres, &none), distance: prefs.unit(DISTANCE_UNIT, Unit::Metres, &none) }
    }
}

/// a distance (stored in metres) as a scrub in the unit chosen for distances, with its unit switch
#[allow(clippy::too_many_arguments)]
fn metres_scrub(d: &mut DrawList, t: &Tokens, a: Rect, y: &mut f32, n: Num, label: &str, metres: f32, unit: Unit, hot: Option<Num>, out: &mut Vec<(Rect, Field)>) {
    let (min, max, log) = n.range();
    let none = Refs::default();
    let shown = units::convert(f64::from(metres), Unit::Metres, unit, &none).unwrap_or(f64::from(metres));
    let row = Rect { x: a.x, y: *y, w: a.w, h: t.size.row };
    let (r, segs) = unit_field::scrub(d, t, row, label, shown, unit, &Unit::Metres.dimension().offered(&none), crate::fraction(metres, min, max, log), hot == Some(n));
    out.push((r, Field::Scrub(n)));
    out.extend(segs.into_iter().map(|(r, u)| (r, Field::DistanceUnit(u))));
    *y += t.size.row;
}
/// the eases a way to a track point may take beside an even speed
const POINT_EASES: [Ease; 4] = [Ease::In, Ease::Out, Ease::InOut, Ease::Whip];

fn unit_of(l: Length) -> usize {
    match l {
        Length::Beats(_) => 0,
        Length::Bars(_) => 1,
        Length::Seconds(_) => 2,
    }
}

/// a track point in a few words: its name, when it arrives, the way's ease
pub fn point_words(track: &deck_space::scene_camera::Track, i: usize) -> String {
    let n = track.points.len();
    let name = if i == 0 { "start".to_string() } else if i + 1 == n { "end".to_string() } else { format!("point {}", i + 1) };
    let p = &track.points[i];
    let time = match (i, p.time) {
        (0, _) => "0".to_string(),
        (_, Some(t)) => format!("{} {}", crate::number(t.value()), t.unit()),
        (_, None) if i + 1 == n => "the move's length".to_string(),
        (_, None) => "by length".to_string(),
    };
    let ease = p.ease.map(|e| format!(" · {}", e.word())).unwrap_or_default();
    format!("{name} · {time}{ease}")
}

/// a focal length as a person reads it: whole millimetres, a tenth when it has one
fn mm(f: f32) -> String {
    let r = (f * 10.0).round() / 10.0;
    if r.fract() == 0.0 {
        format!("{r:.0}")
    } else {
        format!("{r:.1}")
    }
}

/// a shot in a few words: its angles, distance and lens (and its aim when it has one)
pub fn shot_words(s: &deck_space::scene_camera::ShotDef) -> String {
    let lens = if s.lens.ortho() { format!("ortho {} m", crate::number(s.lens.size)) } else { format!("{} mm", mm(s.lens.focal)) };
    let aim = s.aim.as_deref().map(|a| format!(" · at {a}")).unwrap_or_default();
    format!("{:.0}° {:.0}° {} m · {lens}{aim}", s.yaw, s.pitch, crate::number(s.distance))
}

/// what plays on the stage: the move's kind word, the shot it ends on, how far it is (0..1)
pub type Playing = Option<(String, String, f32)>;

/// The panel's sections, in their order: each one line (its icon, its name, what it is set to) that opens onto its body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Lens,
    Focus,
    Height,
    Shots,
    Move,
    Track,
    Phones,
}

impl Part {
    pub const ALL: [Part; 7] = [Part::Lens, Part::Focus, Part::Height, Part::Shots, Part::Move, Part::Track, Part::Phones];

    pub fn word(self) -> &'static str {
        match self {
            Part::Lens => "lens",
            Part::Focus => "focus",
            Part::Height => "height",
            Part::Shots => "shots",
            Part::Move => "move",
            Part::Track => "track",
            Part::Phones => "phones",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Part::Lens => "dop-lens",
            Part::Focus => "dop-focus",
            Part::Height => "dop-height",
            Part::Shots => "dop-shots",
            Part::Move => "dop-move",
            Part::Track => "dop-track",
            Part::Phones => "dop-phones",
        }
    }

    pub fn bit(self) -> u8 {
        1 << Part::ALL.iter().position(|p| *p == self).unwrap_or(0)
    }

    /// every section open
    pub const fn all() -> u8 {
        (1 << Part::ALL.len()) - 1
    }
}

/// a number in a summary: whole when it is, else two places
fn short(v: f32) -> String {
    if (v - v.round()).abs() < 1e-3 {
        format!("{v:.0}")
    } else {
        format!("{v:.2}")
    }
}

#[allow(clippy::too_many_arguments)]
fn pills_row(d: &mut DrawList, t: &Tokens, a: Rect, y: &mut f32, names: &[&str], on: usize, fields: &[Field], out: &mut Vec<(Rect, Field)>) {
    for (r, f) in controls::pills(d, t, a.x, *y, t.size.row - 4.0, names, on).into_iter().zip(fields) {
        out.push((r, *f));
    }
    *y += t.size.row;
}

fn caption(d: &mut DrawList, t: &Tokens, a: Rect, y: &mut f32, text: &str) {
    d.text(a.x, *y + 2.0, t.size.small, &controls::fit(text, (a.w / d.advance(t.size.small)) as usize), t.color.muted);
    *y += t.size.small * 1.6;
}

/// a group's small uppercase label
fn label(d: &mut DrawList, t: &Tokens, a: Rect, y: &mut f32, text: &str) {
    *y += fold::caption(d, t, a.x, *y, text);
}

#[allow(clippy::too_many_arguments)]
fn scrub(d: &mut DrawList, t: &Tokens, a: Rect, y: &mut f32, n: Num, label: &str, v: f32, shown: &str, hot: Option<Num>, out: &mut Vec<(Rect, Field)>) {
    let (min, max, log) = n.range();
    let r = controls::scrub(d, t, Rect { x: a.x, y: *y, w: a.w, h: t.size.row }, label, shown, crate::fraction(v, min, max, log), hot == Some(n));
    out.push((r, Field::Scrub(n)));
    *y += t.size.row;
}

#[allow(clippy::too_many_arguments)]
fn chips_row(d: &mut DrawList, t: &Tokens, a: Rect, y: &mut f32, names: &[&str], lit: &[bool], fields: impl IntoIterator<Item = Field>, out: &mut Vec<(Rect, Field)>) {
    let (rects, used) = controls::chips(d, t, (a.x, *y, a.w), names, lit);
    out.extend(rects.into_iter().zip(fields));
    *y += used + 6.0;
}

/// an exact value's field, a row high; its place
fn exact(d: &mut DrawList, t: &Tokens, a: Rect, y: &mut f32, value: &str, note: &str, hot: bool) -> Rect {
    let r = fold::value(d, t, Rect { x: a.x, y: *y, w: a.w, h: t.size.row - 4.0 }, value, note, hot);
    *y += t.size.row;
    r
}

/// a group label naming whose lens or height it is: the selected shot's, else the camera's
fn whose_label(cam: &SceneCamera, sel: &Sel, what: &str) -> String {
    sel.shot.and_then(|i| cam.shots.get(i)).map_or(what.to_string(), |s| format!("{what} · {}", s.id))
}

/// the lens the panel edits: the selected shot's, else the camera's
pub fn lens_of(cam: &SceneCamera, sel: &Sel) -> Lens {
    sel.shot.and_then(|i| cam.shots.get(i)).map_or(cam.lens, |s| s.lens)
}

/// the eye height the panel shows: the selected shot's, else the camera's
fn height_of(cam: &SceneCamera, sel: &Sel) -> f32 {
    sel.shot.and_then(|i| cam.shots.get(i)).map_or(cam.height, |s| s.height(sel.floor))
}

/// a number of the draft or the lens, as a scrub starts from it
pub fn value(view: &SceneView, n: Num) -> f32 {
    let s = &view.dop.draft;
    match n {
        Num::Size => lens_of(&view.edit.camera(), &view.dop).size,
        Num::Length => s.length.value(),
        Num::Rounds => s.rounds,
        Num::Height => s.height,
        Num::Amount => s.amount,
        Num::PointTime => view.dop.point.and_then(|i| view.edit.camera().track?.points.get(i)?.time).map_or(1.0, |t| t.value()),
        Num::Focus => focus_of(&view.edit.camera(), &view.dop),
        Num::Release => release_of(&view.edit.camera(), &view.dop).map_or(0.0, |ms| ms as f32 / 1000.0),
    }
}

/// the focus distance the panel shows (m): the lens's, else the selected shot's aim's distance
fn focus_of(cam: &SceneCamera, sel: &Sel) -> f32 {
    let shot = sel.shot.and_then(|i| cam.shots.get(i));
    lens_of(cam, sel).focus.unwrap_or_else(|| shot.map_or(3.0, |s| s.distance))
}

/// how long the selected camera's phone may go without a pose (ms; none: no limit)
fn release_of(cam: &SceneCamera, sel: &Sel) -> Option<u64> {
    match sel.shot {
        Some(i) => cam.shots.get(i)?.phones.as_ref()?.release_ms,
        None => cam.phones.as_ref()?.release_ms,
    }
}

/// who may take the selected shot's camera (none selected: the primary camera) on a phone, and its condition's chips
pub fn phones_condition(cam: &SceneCamera, sel: &Sel) -> Option<Condition> {
    let phones = match sel.shot {
        Some(i) => cam.shots.get(i)?.phones.as_ref(),
        None => cam.phones.as_ref(),
    }?;
    Some(Condition::admits(phones.admits.as_ref(), &[]))
}

/// A section's line: what it is set to now.
pub fn summary(part: Part, view: &SceneView) -> String {
    let (cam, sel) = (&view.edit.camera(), &view.dop);
    let shot = sel.shot.and_then(|i| cam.shots.get(i));
    match part {
        Part::Lens => {
            let lens = lens_of(cam, sel);
            if lens.ortho() {
                format!("orthographic · {} m high", short(lens.size))
            } else {
                format!("{} mm · perspective · {:.0}° high", mm(lens.focal), fov_of(lens.focal).to_degrees())
            }
        }
        Part::Focus => match (shot.and_then(|s| s.focus_on.as_deref()), lens_of(cam, sel).focus) {
            (Some(on), _) => format!("on {on}"),
            (None, Some(f)) => format!("{} m", short(f)),
            (None, None) => format!("at the aim · {} m", short(focus_of(cam, sel))),
        },
        Part::Height => {
            let h = height_of(cam, sel);
            match height_preset(h) {
                Some(i) => format!("{} · {} m", EYE_HEIGHTS[i].0, short(h)),
                None => format!("{} m", short(h)),
            }
        }
        Part::Shots => match (cam.shots.len(), shot) {
            (0, _) => "none yet".into(),
            (n, Some(s)) => format!("{n} shot{} · {} picked", if n == 1 { "" } else { "s" }, s.id),
            (n, None) => format!("{n} shot{}", if n == 1 { "" } else { "s" }),
        },
        Part::Move => {
            let s = &sel.draft;
            let to = shot.map_or(" · pick a shot".to_string(), |x| format!(" → {}", x.id));
            let len = if s.kind == MoveKind::Cut { String::new() } else { format!(" · {} {}", short(s.length.value()), s.length.unit()) };
            format!("{}{to}{len}", s.kind.word())
        }
        Part::Track => cam.track.as_ref().map_or("none".into(), |t| {
            let len = deck_space::space::dop::track::Path::of(t).map_or(0.0, |p| p.len());
            format!("{} points · {} m", t.points.len(), short(len))
        }),
        Part::Phones => match phones_condition(cam, sel) {
            None => "no phone".into(),
            Some(c) => format!("{} may take {}", c.words, shot.map_or("the camera", |s| s.id.as_str())),
        },
    }
}

/// Lens: the projection, the focal lengths as the cones they see (or an orthographic view's height), the exact value.
#[allow(clippy::too_many_arguments)]
fn lens_body(d: &mut DrawList, t: &Tokens, a: Rect, y: &mut f32, view: &SceneView, us: Units, hot: Option<Num>, out: &mut Vec<(Rect, Field)>) {
    let (cam, sel) = (&view.edit.camera(), &view.dop);
    let lens = lens_of(cam, sel);
    label(d, t, a, y, &whose_label(cam, sel, "projection"));
    pills_row(d, t, a, y, &["Perspective", "Orthographic"], usize::from(lens.ortho()), &[Field::Projection(Projection::Perspective), Field::Projection(Projection::Orthographic)], out);
    if lens.ortho() {
        return metres_scrub(d, t, a, y, Num::Size, "view height", lens.size, us.distance, hot, out);
    }
    label(d, t, a, y, "focal length");
    let gap = t.size.pad * 0.6;
    let (cells, used) = fold::grid(a.x, *y, a.w, FOCAL_PRESETS.len(), FOCAL_PRESETS.len(), t.size.rem(4.4), gap);
    for (i, (r, f)) in cells.into_iter().zip(FOCAL_PRESETS).enumerate() {
        let (room, ink) = fold::tile(d, t, r, &format!("{f}"), lens.preset() == Some(i));
        fold::cone(d, room, fov_of(f) * 0.5, ink);
        out.push((r, Field::Preset(i)));
    }
    *y += used + gap;
    label(d, t, a, y, "exact");
    let (refs, focal) = (Refs::default(), us.focal);
    let shown = units::convert(f64::from(lens.focal), Unit::Millimetres, focal, &refs).unwrap_or(f64::from(lens.focal));
    let note = match focal {
        Unit::Degrees => format!("{} mm on full frame", mm(lens.focal)),
        _ => format!("{:.0}° high on full frame", fov_of(lens.focal).to_degrees()),
    };
    let row = Rect { x: a.x, y: *y, w: a.w, h: t.size.row - 4.0 };
    let (r, segs) = unit_field::exact(d, t, row, shown, focal, &note, &Unit::Millimetres.dimension().offered(&refs), lens.preset().is_none());
    *y += t.size.row;
    out.push((r, Field::Focal));
    out.extend(segs.into_iter().map(|(r, u)| (r, Field::FocalUnit(u))));
}

/// Focus: at the aim, a distance (scrubbed) or – a shot's – a subject on the stage it follows.
#[allow(clippy::too_many_arguments)]
fn focus_body(d: &mut DrawList, t: &Tokens, a: Rect, y: &mut f32, view: &SceneView, us: Units, hot: Option<Num>, out: &mut Vec<(Rect, Field)>) {
    let (cam, sel) = (&view.edit.camera(), &view.dop);
    let lens = lens_of(cam, sel);
    let shot = sel.shot.and_then(|i| cam.shots.get(i));
    let on = shot.and_then(|s| s.focus_on.as_deref());
    label(d, t, a, y, "focus on");
    let mut names = vec!["at the aim".to_string()];
    let mut lit = vec![lens.focus.is_none() && on.is_none()];
    let mut fields = vec![Field::FocusAim];
    if shot.is_some() {
        for (k, s) in sel.aims.iter().enumerate().filter(|(_, s)| !s.starts_with("spot:")) {
            names.push(s.clone());
            lit.push(on == Some(s.as_str()));
            fields.push(Field::FocusOn(k));
        }
    }
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    chips_row(d, t, a, y, &names, &lit, fields, out);
    match on {
        Some(s) => caption(d, t, a, y, &format!("follows {s}: drag the marker off it for a distance")),
        None => {
            let v = focus_of(cam, sel);
            metres_scrub(d, t, a, y, Num::Focus, if lens.focus.is_some() { "focus distance" } else { "focus · the aim" }, v, us.distance, hot, out);
        }
    }
}

/// Height: the eye heights as figures on a ground line, the exact value (typed).
fn height_body(d: &mut DrawList, t: &Tokens, a: Rect, y: &mut f32, view: &SceneView, us: Units, out: &mut Vec<(Rect, Field)>) {
    let (cam, sel) = (&view.edit.camera(), &view.dop);
    let height = height_of(cam, sel);
    let preset = height_preset(height);
    label(d, t, a, y, &whose_label(cam, sel, "eye height"));
    let gap = t.size.pad * 0.6;
    let (cells, used) = fold::grid(a.x, *y, a.w, EYE_HEIGHTS.len(), EYE_HEIGHTS.len(), t.size.rem(4.4), gap);
    for (i, (r, (name, _))) in cells.into_iter().zip(EYE_HEIGHTS).enumerate() {
        let (room, ink) = fold::tile(d, t, r, name, preset == Some(i));
        if let Some(icon) = icons::named(&format!("height-{name}")) {
            fold::picture(d, room, 32.0, icon, ink);
        }
        out.push((r, Field::Height(i)));
    }
    *y += used + gap;
    label(d, t, a, y, "exact");
    let none = Refs::default();
    let shown = units::convert(f64::from(height), Unit::Metres, us.distance, &none).unwrap_or(f64::from(height));
    let row = Rect { x: a.x, y: *y, w: a.w, h: t.size.row - 4.0 };
    let (r, segs) = unit_field::exact(d, t, row, shown, us.distance, "click to type", &Unit::Metres.dimension().offered(&none), preset.is_none());
    *y += t.size.row;
    out.push((r, Field::HeightTyped));
    out.extend(segs.into_iter().map(|(r, u)| (r, Field::DistanceUnit(u))));
}

/// Shots: each with a small picture of its view, taking the view as a shot, what the selected one aims at.
fn shots_body(d: &mut DrawList, t: &Tokens, a: Rect, y: &mut f32, view: &SceneView, out: &mut Vec<(Rect, Field)>) {
    let (cam, sel) = (&view.edit.camera(), &view.dop);
    if cam.shots.is_empty() {
        caption(d, t, a, y, "none yet: frame the stage and take the view as a shot");
    }
    let (tw, th) = (t.size.rem(3.1), t.size.rem(1.85));
    let h = th + t.size.pad;
    let px = t.size.small;
    for (i, s) in cam.shots.iter().enumerate() {
        let r = Rect { x: a.x - t.size.pad * 0.5, y: *y, w: a.w + t.size.pad * 0.5, h };
        let picked = sel.shot == Some(i);
        if picked {
            d.round_rect(r.x, r.y, r.w, r.h, t.size.radius * 1.4, fold::picked_ground(t));
        }
        fold::view_thumb(d, t, Rect { x: a.x, y: r.y + (h - th) * 0.5, w: tw, h: th }, s.pitch, s.lens.ortho(), if picked { t.color.text } else { t.color.muted });
        let ty = r.y + (h - px * 1.25) * 0.5;
        let id_x = a.x + tw + t.size.pad;
        d.sans(id_x, ty, px, &s.id, if picked { t.color.bright } else { t.color.text });
        let words_x = id_x + t.size.rem(3.4);
        let first = if i == 0 { "opens · " } else { "" };
        let line = format!("{first}{}", shot_words(s));
        d.sans(words_x, ty, px, &controls::fit_sans(d, px, &line, a.x + a.w - words_x), t.color.muted);
        out.push((r, Field::Shot(i)));
        *y += h + 2.0;
    }
    *y += 4.0;
    let mut names = vec!["+ from view"];
    let mut fields = vec![Field::AddShot];
    if sel.shot.is_some() {
        names.extend(["set from view", "remove"]);
        fields.extend([Field::SetShot, Field::RemoveShot]);
    }
    pills_row(d, t, a, y, &names, usize::MAX, &fields, out);
    if let Some(s) = sel.shot.and_then(|i| cam.shots.get(i)) {
        label(d, t, a, y, &format!("{} aims at", s.id));
        let mut names = vec!["its point"];
        names.extend(sel.aims.iter().map(String::as_str));
        let lit: Vec<bool> = std::iter::once(s.aim.is_none()).chain(sel.aims.iter().map(|x| s.aim.as_deref() == Some(x.as_str()))).collect();
        chips_row(d, t, a, y, &names, &lit, (0..names.len()).map(|i| Field::Aim(i.checked_sub(1))), out);
    }
}

/// Move: every kind a tile in its group, only the picked kind's settings under them, Preview / Keep, the kept moves.
#[allow(clippy::too_many_arguments)]
fn move_body(d: &mut DrawList, t: &Tokens, a: Rect, y: &mut f32, view: &SceneView, playing: &Playing, us: Units, hot: Option<Num>, out: &mut Vec<(Rect, Field)>) {
    let (cam, sel) = (&view.edit.camera(), &view.dop);
    let s = &sel.draft;
    let gap = t.size.pad * 0.6;
    for (name, gloss, kinds) in MoveKind::GROUPS {
        label(d, t, a, y, &if gloss.is_empty() { name.to_string() } else { format!("{name} · {gloss}") });
        let (cells, used) = fold::grid(a.x, *y, a.w, kinds.len(), 6, t.size.rem(4.6), gap);
        for (r, k) in cells.into_iter().zip(kinds.iter()) {
            let (room, ink) = fold::tile(d, t, r, k.word(), *k == s.kind);
            if let Some(icon) = icons::named(&format!("move-{}", k.word())) {
                fold::picture(d, room, 28.0, icon, ink);
            }
            if let Some(i) = MoveKind::ALL.iter().position(|x| x == k) {
                out.push((r, Field::Kind(i)));
            }
        }
        *y += used + gap;
    }
    label(d, t, a, y, &format!("{} settings", s.kind.word()));
    let to = sel.shot.and_then(|i| cam.shots.get(i)).map_or("pick a shot", |s| s.id.as_str());
    controls::row(d, t, a, y, "ends on", to);
    if s.kind != MoveKind::Cut {
        let (min, max, log) = Num::Length.range();
        let row = Rect { x: a.x, y: *y, w: a.w, h: t.size.row };
        let fraction = crate::fraction(s.length.value(), min, max, log);
        let (r, segs) = unit_field::scrub(d, t, row, "length", f64::from(s.length.value()), s.length.of_unit(), &Length::UNITS, fraction, hot == Some(Num::Length));
        out.push((r, Field::Scrub(Num::Length)));
        out.extend(segs.into_iter().filter_map(|(r, u)| Some((r, Field::Unit(Length::UNITS.iter().position(|x| *x == u)?)))));
        *y += t.size.row;
        let eases: Vec<&str> = Ease::ALL.iter().map(|e| e.word()).collect();
        let fields: Vec<Field> = (0..Ease::ALL.len()).map(Field::Ease).collect();
        pills_row(d, t, a, y, &eases, Ease::ALL.iter().position(|e| *e == s.ease).unwrap_or(0), &fields, out);
    }
    label(d, t, a, y, "starts");
    pills_row(d, t, a, y, &["now", "on the bar line"], usize::from(s.start == Start::Bar), &[Field::Start(Start::Now), Field::Start(Start::Bar)], out);
    if !matches!(s.kind, MoveKind::Cut | MoveKind::Follow) {
        let names: Vec<&str> = Repeat::ALL.iter().map(|r| r.word()).collect();
        let lit: Vec<bool> = Repeat::ALL.iter().map(|r| *r == s.repeat).collect();
        chips_row(d, t, a, y, &names, &lit, Repeat::ALL.map(Field::Repeat), out);
    }
    match s.kind {
        MoveKind::Orbit | MoveKind::Spiral => {
            scrub(d, t, a, y, Num::Rounds, "rounds", s.rounds, &crate::number(s.rounds), hot, out);
            pills_row(d, t, a, y, &["turn left", "turn right"], usize::from(s.turn == Turn::Right), &[Field::Turn(Turn::Left), Field::Turn(Turn::Right)], out);
        }
        MoveKind::In => pills_row(d, t, a, y, &["from the left", "from the right"], usize::from(s.side == Side::Right), &[Field::Side(Side::Left), Field::Side(Side::Right)], out),
        MoveKind::CraneDown | MoveKind::CraneUp => metres_scrub(d, t, a, y, Num::Height, "height", s.height, us.distance, hot, out),
        MoveKind::Shake => scrub(d, t, a, y, Num::Amount, "unrest", s.amount, &format!("{}°", crate::number(s.amount)), hot, out),
        MoveKind::Follow => {
            let holders: Vec<&str> = view.edit.doc.holders.iter().map(|h| h.id.as_str()).collect();
            if holders.is_empty() {
                caption(d, t, a, y, "the scene has no holder to follow");
            } else {
                let on = s.follow.as_deref().and_then(|f| holders.iter().position(|h| *h == f)).unwrap_or(usize::MAX);
                let fields: Vec<Field> = (0..holders.len()).map(Field::Follow).collect();
                pills_row(d, t, a, y, &holders, on, &fields, out);
            }
        }
        MoveKind::Fly if cam.track.is_none() => caption(d, t, a, y, "runs along the trolley track: add one under Track"),
        _ => {}
    }
    if let Some((kind, shot, p)) = playing {
        let bar = Rect { x: a.x, y: *y + 2.0, w: a.w, h: 3.0 };
        d.rect(bar.x, bar.y, bar.w, bar.h, t.color.control);
        d.rect(bar.x, bar.y, bar.w * p.clamp(0.0, 1.0), bar.h, t.color.text);
        *y += 8.0;
        caption(d, t, a, y, &format!("playing: {kind} to {shot} · {:.0} %", p * 100.0));
    }
    pills_row(d, t, a, y, &["Preview", "Keep as move"], usize::MAX, &[Field::Preview, Field::Keep], out);
    if cam.moves.is_empty() {
        return;
    }
    label(d, t, a, y, "kept moves");
    for (i, m) in cam.moves.iter().enumerate() {
        let r = Rect { x: a.x, y: *y, w: a.w, h: t.size.tree_row };
        if sel.kept == Some(i) {
            d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.selection);
        }
        let then = m.then.as_deref().map_or(String::new(), |n| format!(" → {n}"));
        let repeat = if m.spec.repeat == Repeat::Once { String::new() } else { format!(" · {}", m.spec.repeat.word()) };
        let line = format!("{} · {} to {} · {} {}{repeat}{then}", m.id, m.spec.kind.word(), m.to, crate::number(m.spec.length.value()), m.spec.length.unit());
        d.text(r.x + 6.0, r.y + (r.h - t.size.small * 1.25) * 0.5, t.size.small, &controls::fit(&line, ((r.w - 12.0) / d.advance(t.size.small)) as usize), t.color.text);
        out.push((r, Field::Kept(i)));
        *y += t.size.tree_row;
    }
    *y += 4.0;
    if sel.kept.is_some() {
        pills_row(d, t, a, y, &["Play", "Remove"], usize::MAX, &[Field::PlayKept, Field::RemoveKept], out);
    }
}

/// Track: the trolley path, its points with their times and eases.
fn track_body(d: &mut DrawList, t: &Tokens, a: Rect, y: &mut f32, view: &SceneView, hot: Option<Num>, out: &mut Vec<(Rect, Field)>) {
    let (cam, sel) = (&view.edit.camera(), &view.dop);
    let Some(track) = cam.track.as_ref() else {
        return pills_row(d, t, a, y, &["Add track"], usize::MAX, &[Field::AddTrack], out);
    };
    let len = deck_space::space::dop::track::Path::of(track).map_or(0.0, |p| p.len());
    let at = sel.point.map_or(String::new(), |p| format!(" · point {} selected", p + 1));
    caption(d, t, a, y, &format!("{} points · {} m{at} · drag them on the stage", track.points.len(), crate::number(len)));
    for i in 0..track.points.len() {
        let r = Rect { x: a.x, y: *y, w: a.w, h: t.size.tree_row };
        let picked = sel.point == Some(i);
        if picked {
            d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.selection);
        }
        let line = point_words(track, i);
        d.text(r.x + 6.0, r.y + (r.h - t.size.small * 1.25) * 0.5, t.size.small, &controls::fit(&line, ((r.w - 12.0) / d.advance(t.size.small)) as usize), if picked { t.color.bright } else { t.color.text });
        out.push((r, Field::Point(i)));
        *y += t.size.tree_row;
    }
    *y += 4.0;
    if let Some(p) = sel.point.filter(|p| *p > 0).and_then(|i| track.points.get(i)) {
        label(d, t, a, y, "arrives");
        let at = Field::PointUnit(Some(track.points.iter().find_map(|q| q.time).map_or(0, unit_of)));
        chips_row(d, t, a, y, &["by length", "at a time"], &[p.time.is_none(), p.time.is_some()], [Field::PointUnit(None), at], out);
        if let Some(time) = p.time {
            let (min, max, log) = Num::PointTime.range();
            let row = Rect { x: a.x, y: *y, w: a.w, h: t.size.row };
            let (r, segs) = unit_field::scrub(d, t, row, "at", f64::from(time.value()), time.of_unit(), &Length::UNITS, crate::fraction(time.value(), min, max, log), hot == Some(Num::PointTime));
            out.push((r, Field::Scrub(Num::PointTime)));
            out.extend(segs.into_iter().filter_map(|(r, u)| Some((r, Field::PointUnit(Some(Length::UNITS.iter().position(|x| *x == u)?))))));
            *y += t.size.row;
        }
        let mut names = vec!["even"];
        names.extend(POINT_EASES.iter().map(|e| e.word()));
        let lit: Vec<bool> = std::iter::once(p.ease.is_none()).chain(POINT_EASES.iter().map(|e| p.ease == Some(*e))).collect();
        chips_row(d, t, a, y, &names, &lit, (0..names.len()).map(|k| Field::PointEase(k.checked_sub(1))), out);
    }
    let mut names = vec!["Add point"];
    let mut fields = vec![Field::AddPoint];
    if sel.point.is_some_and(|p| p > 0 && p + 1 < track.points.len()) {
        names.push("Remove point");
        fields.push(Field::RemovePoint);
    }
    names.push("Remove track");
    fields.push(Field::RemoveTrack);
    pills_row(d, t, a, y, &names, usize::MAX, &fields, out);
}

/// Phones (LIVE-CAMERAS.md §4): whether a phone may take this camera in a live session, who (the one condition control,
/// any one fact), how long without a pose, its id the show and the QR name, a take recorded from it.
fn phones_body(d: &mut DrawList, t: &Tokens, a: Rect, y: &mut f32, view: &SceneView, hot: Option<Num>, out: &mut Vec<(Rect, Field)>) {
    let (cam, sel) = (&view.edit.camera(), &view.dop);
    let shot = sel.shot.and_then(|i| cam.shots.get(i));
    let c = phones_condition(cam, sel);
    label(d, t, a, y, &whose_label(cam, sel, "phones"));
    pills_row(d, t, a, y, &["No phone", "Phones may take it"], usize::from(c.is_some()), &[Field::Phones(false), Field::Phones(true)], out);
    let Some(c) = c else { return };
    label(d, t, a, y, "who");
    let (names, on) = c.chips();
    chips_row(d, t, a, y, &names, &on, (0..names.len()).map(Field::PhonesFact), out);
    let release = release_of(cam, sel).map_or(0.0, |ms| ms as f32 / 1000.0);
    let shown = if release > 0.0 { format!("{} s, then it returns", crate::number(release)) } else { "no limit – Leave frees it".into() };
    scrub(d, t, a, y, Num::Release, "no pose for", release, &shown, hot, out);
    label(d, t, a, y, "qr");
    let id = deck_space::live_cameras::camera_id(&view.edit.key(), shot.map(|s| s.id.as_str()));
    exact(d, t, a, y, &id, "Perform › Session", false);
    match sel.rec.as_ref() {
        Some(r) => {
            caption(d, t, a, y, &format!("● REC {:.1} s · {} poses · the stage looks through it", r.secs, r.poses));
            pills_row(d, t, a, y, &["Stop and keep", "Discard"], usize::MAX, &[Field::Record, Field::Discard], out);
        }
        None => pills_row(d, t, a, y, &["Record a take"], usize::MAX, &[Field::Record], out),
    }
}

/// The panel: the camera's source, then each section a line that opens onto its body; where its fields landed.
/// `us`: the units the values are read in
pub fn draw(d: &mut DrawList, t: &Tokens, a: Rect, view: &SceneView, playing: Playing, us: Units, hot: Option<Num>) -> Vec<(Rect, Field)> {
    let mut out = Vec::new();
    let cam = view.edit.camera();
    let sel = &view.dop;
    let mut y = a.y;
    let on = usize::from(cam.source == Source::Handheld);
    pills_row(d, t, a, &mut y, &["Scene camera", "Handheld"], on, &[Field::Source(Source::Scene), Field::Source(Source::Handheld)], &mut out);
    if cam.source == Source::Handheld {
        caption(d, t, a, &mut y, "a phone steers it – not built yet; the shots and moves wait");
    }
    y += t.size.pad * 0.5;
    d.rect(a.x, y - t.size.edge, a.w, t.size.edge, t.color.line);
    let indent = t.size.pad * 2.0 + fold::HEAD_ICON;
    let body = Rect { x: a.x + indent, y: 0.0, w: a.w - indent - t.size.pad, h: 0.0 };
    for part in Part::ALL {
        let open = sel.open & part.bit() != 0;
        let r = fold::head(d, t, Rect { x: a.x, y, w: a.w, h: fold::head_height(t) }, part.icon(), part.word(), &summary(part, view), open);
        out.push((r, Field::Fold(part)));
        y += r.h;
        if !open {
            continue;
        }
        y += t.size.pad * 0.3;
        match part {
            Part::Lens => lens_body(d, t, body, &mut y, view, us, hot, &mut out),
            Part::Focus => focus_body(d, t, body, &mut y, view, us, hot, &mut out),
            Part::Height => height_body(d, t, body, &mut y, view, us, &mut out),
            Part::Shots => shots_body(d, t, body, &mut y, view, &mut out),
            Part::Move => move_body(d, t, body, &mut y, view, &playing, us, hot, &mut out),
            Part::Track => track_body(d, t, body, &mut y, view, hot, &mut out),
            Part::Phones => phones_body(d, t, body, &mut y, view, hot, &mut out),
        }
        y += t.size.pad;
        d.rect(a.x, y - t.size.edge, a.w, t.size.edge, t.color.line);
    }
    y += t.size.pad * 0.6;
    caption(d, t, Rect { x: a.x + t.size.pad, ..a }, &mut y, "Alt-click a head: open / close all");
    y += t.size.pad;
    let rows = view.edit.slot_rows(deck_space::scene_modules::On::Camera);
    for (r, p) in crate::slots::section(d, t, a, &mut y, &rows, view.carry) {
        out.push((r, match p {
            crate::slots::Part::Slot(i) => Field::Slot(i),
            crate::slots::Part::Detach(i) => Field::Detach(i),
        }));
    }
    out
}

/// the height the panel takes
pub fn height(t: &Tokens, view: &SceneView) -> f32 {
    let mut d = DrawList::new(0.6);
    let fields = draw(&mut d, t, Rect { x: 0.0, y: 0.0, w: WIDTH, h: 4000.0 }, view, None, Units::default(), None);
    fields.iter().map(|(r, _)| r.y + r.h).fold(0.0, f32::max) + t.size.pad + t.size.row * 1.5
}

impl crate::App {
    pub(crate) fn dop_done(&mut self, done: Result<(), String>, said: String, x: f32, y: f32) {
        let ok = done.is_ok();
        self.scene_changed(done, said, x, y);
        if ok {
            self.sync_dop();
        }
    }

    /// the stage's camera takes the scene's camera as edited (what plays keeps playing)
    pub(crate) fn sync_dop(&mut self) {
        if let Some(view) = self.scene.as_ref() {
            self.stage.space.dop.edited(view.edit.camera());
        }
    }

    /// a field of the DOP panel pressed
    pub(crate) fn dop_field(&mut self, f: Field, x: f32, y: f32) {
        let now = View::of(&self.stage.space.camera);
        let tempo = self.tempo();
        let Some(view) = self.scene.as_mut() else { return };
        let cam = view.edit.camera();
        let sel = &mut view.dop;
        let lens = lens_of(&cam, sel);
        let shot = sel.shot;
        match f {
            Field::Fold(p) => {
                let open = sel.open & p.bit() != 0;
                sel.open = match (self.mods.alt_key(), open) {
                    (true, true) => 0,
                    (true, false) => Part::all(),
                    (false, _) => sel.open ^ p.bit(),
                };
            }
            Field::Source(s) => {
                let done = view.edit.set_source(s);
                self.dop_done(done, format!("camera: {}", if s == Source::Handheld { "handheld – nothing steers it yet" } else { "the scene's shots and moves" }), x, y);
            }
            Field::Projection(p) => {
                let done = view.edit.set_lens(shot, Lens { projection: p, ..lens });
                self.dop_done(done, format!("lens: {}", if p == Projection::Orthographic { "orthographic" } else { "perspective" }), x, y);
            }
            Field::Preset(i) => {
                let done = view.edit.set_lens(shot, Lens { focal: FOCAL_PRESETS[i], ..lens });
                self.dop_done(done, format!("lens: {} mm", FOCAL_PRESETS[i]), x, y);
            }
            Field::Focal => {
                let unit = self.settings.units.unit(FOCAL_UNIT, Unit::Millimetres, &Refs::default());
                let v = units::convert(f64::from(lens.focal), Unit::Millimetres, unit, &Refs::default()).unwrap_or(f64::from(lens.focal));
                self.begin_edit(crate::Editing::Focal, crate::Hit::Dop(Field::Focal), units::number(v, unit))
            }
            Field::FocalUnit(u) => self.change_settings(|s| s.units.set(FOCAL_UNIT, u)),
            Field::DistanceUnit(u) => self.change_settings(|s| s.units.set(DISTANCE_UNIT, u)),
            Field::Height(i) => {
                let (name, h) = EYE_HEIGHTS[i];
                let done = view.edit.set_height(shot, h, view.dop.floor);
                self.dop_done(done, format!("eye height: {name}, {} m", crate::number(h)), x, y);
                self.dop_relook(shot);
            }
            Field::HeightTyped => {
                let now = shot.and_then(|i| cam.shots.get(i)).map_or(cam.height, |s| s.height(view.dop.floor));
                let unit = self.settings.units.unit(DISTANCE_UNIT, Unit::Metres, &Refs::default());
                let v = units::convert(f64::from(now), Unit::Metres, unit, &Refs::default()).unwrap_or(f64::from(now));
                self.begin_edit(crate::Editing::Height, crate::Hit::Dop(Field::HeightTyped), units::number(v, unit));
            }
            Field::Scrub(_) => {}
            Field::Phones(open) => {
                let done = view.edit.set_phones(shot, open);
                let whose = shot.and_then(|i| cam.shots.get(i)).map_or("the camera".to_string(), |s| s.id.clone());
                self.dop_done(done, format!("{whose}: {}", if open { "phones may take it – anyone with the code" } else { "no phone takes it" }), x, y);
            }
            Field::PhonesFact(i) => {
                let Some(fact) = phones_condition(&cam, sel).and_then(|c| c.fact(i)) else { return };
                let done = view.edit.phones_fact(shot, &fact);
                let words = phones_condition(&view.edit.camera(), &view.dop).map(|c| c.words).unwrap_or_default();
                self.dop_done(done, format!("phones: {words}"), x, y);
            }
            Field::Record => self.take_toggle(x, y),
            Field::Discard => self.take_discard(),
            Field::Shot(i) => {
                sel.shot = Some(i);
                if let Some(id) = cam.shots.get(i).map(|s| s.id.clone()) {
                    self.stage.space.look_through_shot(&id);
                }
            }
            Field::AddShot => match view.edit.add_shot(now.shot("")) {
                Ok(i) => {
                    view.dop.shot = Some(i);
                    self.dop_done(Ok(()), "a shot from the view".into(), x, y);
                }
                Err(e) => self.refuse(e, x, y),
            },
            Field::SetShot => {
                let Some(i) = shot else { return };
                let done = view.edit.set_shot(i, now.shot(""));
                self.dop_done(done, "the shot is the view now".into(), x, y);
            }
            Field::RemoveShot => {
                let Some(i) = shot else { return };
                let done = view.edit.remove_shot(i);
                if done.is_ok() {
                    view.dop.shot = None;
                }
                self.dop_done(done, "the shot left the camera".into(), x, y);
            }
            Field::Kind(i) => {
                let kind = MoveKind::ALL[i];
                let follow = sel.draft.follow.clone().or_else(|| view.edit.doc.holders.first().map(|h| h.id.clone()));
                sel.draft = MoveSpec { follow: (kind == MoveKind::Follow).then_some(follow).flatten(), ..MoveSpec::of(kind) };
            }
            Field::Unit(u) => {
                // the same length counted in the other unit, at the clock's tempo
                let (bpm, per_bar) = tempo.unwrap_or((120.0, units::PER_BAR));
                let l = sel.draft.length;
                sel.draft.length = l.to_unit(Length::UNITS[u], per_bar, bpm).unwrap_or(l);
            }
            Field::Ease(i) => sel.draft.ease = Ease::ALL[i],
            Field::Start(s) => sel.draft.start = s,
            Field::Repeat(r) => sel.draft.repeat = r,
            Field::Turn(t) => sel.draft.turn = t,
            Field::Side(s) => sel.draft.side = s,
            Field::Follow(i) => sel.draft.follow = view.edit.doc.holders.get(i).map(|h| h.id.clone()),
            Field::Aim(k) => {
                let Some(i) = shot else { return };
                let aim = k.and_then(|k| sel.aims.get(k).cloned());
                let done = view.edit.set_aim(i, aim.as_deref());
                self.dop_done(done, format!("the shot aims at {}", aim.as_deref().unwrap_or("its own point")), x, y);
                self.dop_relook(shot);
            }
            Field::FocusAim => {
                if let Some(i) = shot.filter(|i| cam.shots.get(*i).is_some_and(|s| s.focus_on.is_some())) {
                    if let Err(e) = view.edit.set_focus_on(i, None) {
                        return self.refuse(e, x, y);
                    }
                }
                let done = view.edit.set_lens(shot, Lens { focus: None, ..lens });
                self.dop_done(done, "focus: at the aim".into(), x, y);
            }
            Field::FocusOn(k) => {
                let Some(i) = shot else { return };
                let on = sel.aims.get(k).cloned();
                let done = view.edit.set_focus_on(i, on.as_deref());
                self.dop_done(done, format!("focus on {}", on.as_deref().unwrap_or("nothing")), x, y);
            }
            Field::Preview => {
                let Some(to) = shot.and_then(|i| cam.shots.get(i)).map(|s| s.id.clone()) else { return self.refuse("pick the shot it ends on".into(), x, y) };
                let draft = sel.draft.clone();
                let mut problems = Vec::new();
                draft.problems("the move", &mut problems);
                if let Some(p) = problems.first() {
                    return self.refuse(p.clone(), x, y);
                }
                self.stage.space.dop.edited(cam);
                match self.stage.space.dop.cue(draft, &to) {
                    Ok(c) => self.stage.space.dop.play(vec![c]),
                    Err(e) => self.refuse(e, x, y),
                }
            }
            Field::Keep => {
                let Some(to) = shot.and_then(|i| cam.shots.get(i)).map(|s| s.id.clone()) else { return self.refuse("pick the shot it ends on".into(), x, y) };
                match view.edit.add_move(sel.draft.clone(), &to) {
                    Ok(i) => {
                        view.dop.kept = Some(i);
                        let id = view.edit.camera().moves[i].id.clone();
                        let place = view.edit.key();
                        self.dop_done(Ok(()), format!("kept as {place} > move:{id}"), x, y);
                    }
                    Err(e) => self.refuse(e, x, y),
                }
            }
            Field::Kept(i) => {
                sel.kept = Some(i);
                if let Some(m) = cam.moves.get(i) {
                    sel.draft = m.spec.clone();
                    sel.shot = cam.shots.iter().position(|s| s.id == m.to);
                }
            }
            Field::PlayKept => {
                let Some(m) = sel.kept.and_then(|i| cam.moves.get(i)) else { return };
                let id = m.id.clone();
                self.stage.space.dop.edited(cam);
                match self.stage.space.dop.chain(&id) {
                    Ok(c) => self.stage.space.dop.play(c),
                    Err(e) => self.refuse(e, x, y),
                }
            }
            Field::Slot(_) => self.note("drop a module from the library on the slot".into()),
            Field::Detach(i) => self.slot_detach(deck_space::scene_modules::On::Camera, i, x, y),
            Field::RemoveKept => {
                let Some(i) = sel.kept.take() else { return };
                let done = view.edit.remove_move(i);
                self.dop_done(done, "the move left the camera".into(), x, y);
            }
            Field::AddTrack => {
                let fwd = (now.target - now.eye()).normalize_or_zero();
                let right = fwd.cross(glam::Vec3::Y).normalize_or_zero() * now.distance.max(1.0);
                let (s, e) = (now.target - right + glam::Vec3::Y * 0.5, now.target + right + glam::Vec3::Y * 0.5);
                let done = view.edit.add_track(s.to_array(), e.to_array());
                view.dop.point = Some(0);
                self.dop_done(done, "a trolley track: drag its start and end on the stage".into(), x, y);
            }
            Field::AddPoint => {
                let n = cam.track.as_ref().map_or(0, |t| t.points.len());
                let after = sel.point.unwrap_or(0).min(n.saturating_sub(2));
                match view.edit.add_track_point(after) {
                    Ok(p) => {
                        view.dop.point = Some(p);
                        self.dop_done(Ok(()), "a point between".into(), x, y);
                    }
                    Err(e) => self.refuse(e, x, y),
                }
            }
            Field::RemovePoint => {
                let Some(p) = sel.point.take() else { return };
                let done = view.edit.remove_track_point(p);
                self.dop_done(done, "the point left the track".into(), x, y);
            }
            Field::Point(i) => sel.point = Some(i),
            Field::PointUnit(u) => {
                let Some(i) = sel.point.filter(|i| *i > 0) else { return };
                let Some(track) = cam.track.as_ref() else { return };
                let (bpm, per_bar) = tempo.unwrap_or((120.0, units::PER_BAR));
                let has_time = track.points.get(i).is_some_and(|p| p.time.is_some());
                let done = match u {
                    // every time of the track counted in the other unit, at the clock's tempo
                    Some(u) if has_time => view.edit.set_track_unit(Length::UNITS[u], per_bar, bpm),
                    // a new time: after the ones before it, in the unit the track's times are in
                    Some(u) => {
                        let v = track.points[..i].iter().filter_map(|p| p.time).map(|t| t.value()).fold(0.0, f32::max) + 1.0;
                        let unit = track.points.iter().find_map(|p| p.time).map_or(Length::UNITS[u], |t| t.of_unit());
                        view.edit.set_point_time(i, Length::in_unit(unit, v))
                    }
                    None => view.edit.set_point_time(i, None),
                };
                let time = view.edit.camera().track.and_then(|t| t.points.get(i).and_then(|p| p.time));
                let said = time.map_or("the point: by the track's length".into(), |t| format!("the point arrives at {} {}", crate::number(t.value()), t.unit()));
                self.dop_done(done, said, x, y);
            }
            Field::PointEase(k) => {
                let Some(i) = sel.point.filter(|i| *i > 0) else { return };
                let ease = k.map(|k| POINT_EASES[k]);
                let done = view.edit.set_point_ease(i, ease);
                self.dop_done(done, format!("the way to the point: {}", ease.map_or("even", |e| e.word())), x, y);
            }
            Field::RemoveTrack => {
                let done = view.edit.remove_track();
                if done.is_ok() {
                    view.dop.point = None;
                }
                self.dop_done(done, "the track left the scene".into(), x, y);
            }
        }
    }

    /// a scrub of the panel dragged from (`x0`, `v0`) to `x`
    pub(crate) fn dop_scrub_to(&mut self, n: Num, x0: f32, v0: f32, x: f32) {
        let Some(view) = self.scene.as_mut() else { return };
        let (min, max, log) = n.range();
        let v = crate::scrub(v0, x - x0, min, max, log);
        let v = if n == Num::Rounds { (v * 4.0).round() / 4.0 } else { (v * 100.0).round() / 100.0 };
        let s = &mut view.dop.draft;
        match n {
            Num::Length => s.length = s.length.with(v),
            Num::Rounds => s.rounds = v,
            Num::Height => s.height = v,
            Num::Amount => s.amount = v,
            Num::PointTime => {
                let Some(i) = view.dop.point else { return };
                let Some(t) = view.edit.camera().track.and_then(|t| t.points.get(i).and_then(|p| p.time)) else { return };
                let _ = view.edit.set_point_time(i, Some(t.with(v)));
                self.sync_dop();
            }
            Num::Size => {
                let lens = lens_of(&view.edit.camera(), &view.dop);
                let _ = view.edit.set_lens(view.dop.shot, Lens { size: v, ..lens });
                self.sync_dop();
            }
            Num::Focus => {
                let lens = lens_of(&view.edit.camera(), &view.dop);
                let _ = view.edit.set_lens(view.dop.shot, Lens { focus: Some(v), ..lens });
                self.sync_dop();
            }
            Num::Release => {
                let s = (v * 2.0).round() / 2.0;
                let _ = view.edit.set_release(view.dop.shot, (s > 0.0).then_some((s * 1000.0) as u64));
            }
        }
    }

    /// The focus marker of shot `i` dragged to (`x`, `y`): the focus distance is where its line of sight passes closest
    /// to the pointer – off a subject it was on.
    pub(crate) fn focus_to(&mut self, i: usize, x: f32, y: f32) {
        let (sx, sy) = self.stage_pixel(x, y);
        let Some(view) = self.scene.as_mut() else { return };
        let Some(s) = view.edit.camera().shots.get(i).cloned() else { return };
        let Some(v) = self.stage.space.shot_view(&s.id) else { return };
        let eye = v.eye();
        let Some(d) = self.stage.space.along_sight(sx, sy, eye, v.target - eye) else { return };
        let d = (d.clamp(FOCUS_RANGE.0, FOCUS_RANGE.1) * 100.0).round() / 100.0;
        if s.focus_on.is_some() {
            let _ = view.edit.set_focus_on(i, None);
        }
        let _ = view.edit.set_lens(Some(i), Lens { focus: Some(d), ..s.lens });
        self.sync_dop();
    }

    /// The focus marker let go: on a subject the focus follows it, elsewhere it keeps its distance.
    pub(crate) fn focus_settled(&mut self, i: usize) {
        let (x, y) = self.cursor;
        let (sx, sy) = self.stage_pixel(x, y);
        let on = self.stage.space.subject_at(sx, sy, SUBJECT_REACH * self.scale());
        let Some(view) = self.scene.as_mut() else { return };
        match on {
            Some(on) => {
                let done = view.edit.set_focus_on(i, Some(&on));
                self.dop_done(done, format!("focus on {on}"), x, y);
            }
            None => {
                let d = view.edit.camera().shots.get(i).and_then(|s| s.lens.focus);
                self.dop_done(Ok(()), d.map_or("focus: at the aim".into(), |d| format!("focus: {} m", crate::number(d))), x, y);
            }
        }
    }

    /// a scrub let go: a lens change plays
    pub(crate) fn dop_settled(&mut self, n: Num) {
        let (x, y) = self.cursor;
        match n {
            Num::Size => self.dop_done(Ok(()), "lens: view height".into(), x, y),
            Num::PointTime => self.dop_done(Ok(()), "the point's time".into(), x, y),
            Num::Focus => self.dop_done(Ok(()), "focus distance".into(), x, y),
            Num::Release => {
                let ms = self.scene.as_ref().and_then(|v| release_of(&v.edit.camera(), &v.dop));
                self.dop_done(Ok(()), ms.map_or("phones: no limit – only Leave or a take-over frees it".into(), |ms| format!("phones: back after {} s without a pose", crate::number(ms as f32 / 1000.0))), x, y);
            }
            _ => {}
        }
    }

    /// A check run's camera (`--dop shots+track+preview`; `times` + `to-seconds`): three shots around the view (the second a long lens, the third
    /// isometric), a track with a point between that is selected, a spiral previewed to the second shot; `phones` opens the
    /// camera and the second shot to phones.
    pub(crate) fn script_dop(&mut self) {
        let Some(spec) = self.args.dop.clone() else { return };
        let (x, y) = self.cursor;
        let now = View::of(&self.stage.space.camera);
        match self.frames {
            // `open-lens`, `open-move` …: those sections open; `open-all`
            76 if spec.contains("open-") => {
                let Some(view) = self.scene.as_mut() else { return };
                let all = spec.contains("open-all");
                view.dop.open = Part::ALL.iter().filter(|p| all || spec.contains(&format!("open-{}", p.word()))).fold(0, |m, p| m | p.bit());
            }
            78 if spec.contains("shots") => {
                let Some(view) = self.scene.as_mut() else { return };
                let close = View { distance: now.distance * 0.5, yaw: now.yaw - 0.7, lens: Lens::focal(70.0), ..now };
                let iso = View { yaw: 45f32.to_radians(), pitch: 35f32.to_radians(), distance: now.distance * 2.5, lens: Lens::orthographic(now.distance * 1.4), ..now };
                for v in [now, close, iso] {
                    if let Err(e) = view.edit.add_shot(v.shot("")) {
                        return self.refuse(e, x, y);
                    }
                }
                view.dop.shot = Some(1);
                self.dop_done(Ok(()), "three shots".into(), x, y);
            }
            80 if spec.contains("track") => {
                self.dop_field(Field::AddTrack, x, y);
                self.dop_field(Field::AddPoint, x, y);
                if let Some(view) = self.scene.as_mut() {
                    let p = view.edit.camera().track.map(|t| t.points[1].at).unwrap_or_default();
                    let _ = view.edit.move_track_point(1, [p[0], p[1] + 0.8, p[2] - 1.2]);
                    let _ = view.edit.set_handle(1, Some([0.9, 0.0, 0.4]));
                    // `times`: the point between on bar 2 eased out, the end on bar 4, a fly-through that ping-pongs
                    if spec.contains("times") {
                        let _ = view.edit.set_point_time(1, Some(Length::Bars(2.0)));
                        let _ = view.edit.set_point_ease(1, Some(Ease::Out));
                        let _ = view.edit.set_point_time(2, Some(Length::Bars(4.0)));
                        view.dop.point = Some(1);
                        view.dop.draft = MoveSpec { repeat: Repeat::PingPong, ..MoveSpec::of(MoveKind::Fly) };
                    }
                }
                self.dop_done(Ok(()), "a trolley track".into(), x, y);
            }
            // `to-seconds`: the selected point's time switched to seconds
            83 if spec.contains("to-seconds") => self.dop_field(Field::PointUnit(Some(2)), x, y),
            // `phones`: the primary camera open to anyone with the code, the second shot to members only, selected
            81 if spec.contains("phones") => {
                let Some(view) = self.scene.as_mut() else { return };
                let member = deck_ui::filter::Fact::Account(protocol::Account::Member);
                let guest = deck_ui::filter::Fact::Account(protocol::Account::Guest);
                let done = view.edit.set_phones(None, true).and(view.edit.set_phones(Some(1), true)).and(view.edit.phones_fact(Some(1), &member)).and(view.edit.phones_fact(Some(1), &guest));
                view.dop.shot = Some(1);
                self.dop_done(done, "phones: the camera, and shot2 for members".into(), x, y);
            }
            82 if spec.contains("wide") => {
                let c = &mut self.stage.space.camera;
                c.drag(-260.0, 160.0);
                c.zoom(-9.0);
            }
            84 if spec.contains("look") => self.look_to(crate::tools::Look::Shot(2)),
            // half way from the first shot (perspective) to the isometric one, as a move or the switcher's glide passes it
            84 if spec.contains("blend") => {
                let dop = &self.stage.space.dop;
                let ids: Vec<String> = dop.camera.shots.iter().map(|s| s.id.clone()).collect();
                let (Some(a), Some(b)) = (ids.first().and_then(|i| dop.view(i)), ids.get(2).and_then(|i| dop.view(i))) else { return };
                let glide = MoveSpec { ease: Ease::Linear, ..MoveSpec::of(MoveKind::Glide) };
                let v = deck_space::space::dop::moves::pose(&glide, &a, &b, 0.5, &deck_space::space::dop::moves::Ctx { track: None, secs: 0.0, knots: None });
                let c = &mut self.stage.space.camera;
                c.far_back = true;
                v.apply(c);
                if let Some(view) = self.scene.as_mut() {
                    view.dop.shot = Some(2);
                }
            }
            // the first shot focused 4.5 m out, the second on the stage's first subject
            86 if spec.contains("focus") => {
                let Some(view) = self.scene.as_mut() else { return };
                let on = view.dop.aims.iter().find(|a| !a.starts_with("spot:")).cloned();
                let lens = view.edit.camera().shots.first().map(|s| s.lens).unwrap_or_default();
                let _ = view.edit.set_lens(Some(0), Lens { focus: Some(4.5), ..lens });
                if let Some(on) = on.as_deref() {
                    let _ = view.edit.set_focus_on(1, Some(on));
                }
                // `distance`: the first shot selected, its focus a distance
                view.dop.shot = Some(if spec.contains("distance") { 0 } else { 1 });
                view.dop.draft = MoveSpec { repeat: Repeat::PingPong, ..MoveSpec::of(MoveKind::Rack) };
                self.dop_done(Ok(()), format!("focus on {}", on.as_deref().unwrap_or("nothing")), x, y);
            }
            // `take`: a phone holding shot2 recorded from frame 88 – 2 m to the right, rising a little and turning to look
            // back left, 30 poses a second – kept at frame 140, then played
            88 if spec.contains("take") => {
                let id = self.scene.as_ref().map(|v| deck_space::live_cameras::camera_id(&v.edit.key(), Some("shot2")));
                if let Some(id) = id {
                    self.take_begin(id);
                }
            }
            f @ 89..=139 if spec.contains("take") => {
                let s = (f - 89) as f64 / 30.0;
                let u = s / (50.0 / 30.0);
                let (sh, ch) = (u * 0.6 / 2.0).sin_cos();
                if let Some(r) = self.take.as_mut() {
                    r.pose(s * 1000.0, &[u * 2.0, (u * std::f64::consts::PI).sin() * 0.3, -u * 0.5], &[0.0, sh, 0.0, ch]);
                }
            }
            140 if spec.contains("take") => self.take_keep(x, y),
            142 if spec.contains("take") => self.dop_field(Field::PlayKept, x, y),
            84 if spec.contains("preview") => {
                if let Some(view) = self.scene.as_mut() {
                    view.dop.shot = Some(1);
                    view.dop.draft = MoveSpec { length: Length::Bars(8.0), ..MoveSpec::of(MoveKind::Spiral) };
                }
                self.dop_field(Field::Preview, x, y);
            }
            _ => {}
        }
    }

    /// an eye height typed, in the unit distances are read in or one it names
    pub(crate) fn dop_height(&mut self, text: &str) {
        let (x, y) = self.cursor;
        let unit = self.settings.units.unit(DISTANCE_UNIT, Unit::Metres, &Refs::default());
        let typed = units::parse(text, unit).and_then(|(v, u)| units::convert(v, u, Unit::Metres, &Refs::default()));
        let Some(h) = typed.map(|v| v as f32) else { return self.refuse(format!("'{text}': an eye height in m, cm or ft"), x, y) };
        let Some(view) = self.scene.as_mut() else { return };
        let shot = view.dop.shot;
        let done = view.edit.set_height(shot, h, view.dop.floor);
        self.dop_done(done, format!("eye height: {} m", crate::number(h)), x, y);
        self.dop_relook(shot);
    }

    /// the view follows a changed height: through the shot changed, or the primary camera
    fn dop_relook(&mut self, shot: Option<usize>) {
        let id = shot.and_then(|i| self.stage.space.dop.camera.shots.get(i)).map(|s| s.id.clone());
        match id {
            Some(id) => self.stage.space.look_through_shot(&id),
            None if self.stage.space.dop.camera.shots.is_empty() => self.stage.space.look_through_scene(),
            None => {}
        }
    }

    /// a focal length typed
    pub(crate) fn dop_focal(&mut self, text: &str) {
        let (x, y) = self.cursor;
        let unit = self.settings.units.unit(FOCAL_UNIT, Unit::Millimetres, &Refs::default());
        let typed = units::parse(text, unit).and_then(|(v, u)| units::convert(v, u, Unit::Millimetres, &Refs::default()));
        let Some(focal) = typed.map(|v| v as f32) else { return self.refuse(format!("'{text}': a focal length in mm, or the degrees it sees (40°)"), x, y) };
        let Some(view) = self.scene.as_mut() else { return };
        let lens = lens_of(&view.edit.camera(), &view.dop);
        let done = view.edit.set_lens(view.dop.shot, Lens { focal, projection: Projection::Perspective, ..lens });
        self.dop_done(done, format!("lens: {} mm", mm(focal)), x, y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::show::build::SceneOpen;
    use deck_space::scene_document::SceneDocument;

    /// the panel model: the camera's source with the handheld placeholder, the lens presets and the typed value, the
    /// shots, the move's settings by kind, the track once there is one
    #[test]
    fn the_dop_panel_shows_the_camera_and_offers_what_the_selection_allows() {
        let mut doc = SceneDocument::new("duet", "Duet", vec!["catpope".into()]);
        doc.recast(&["character:catpope".into()]);
        let (mut view, _, _) = SceneView::open(SceneOpen { doc, base: Some(1), kept: None, note: None, save: None }, None).expect("it plays");
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        let area = Rect { x: 0.0, y: 0.0, w: WIDTH, h: 4000.0 };
        let fields = |d: &mut DrawList, view: &SceneView| draw(d, &t, area, view, None, Units::default(), None).into_iter().map(|(_, f)| f).collect::<Vec<_>>();
        let f = fields(&mut d, &view);
        assert!(f.contains(&Field::Source(Source::Handheld)) && d.texts().contains(&"Handheld"));
        assert!(Part::ALL.iter().all(|p| f.contains(&Field::Fold(*p))) && !f.contains(&Field::Preset(0)), "every section folded at first");
        assert!(d.texts().iter().any(|s| s.contains("30 mm · perspective · 44° high")) && d.texts().iter().any(|s| s.contains("adult · 1.60 m")), "each line says what it is set to: {:?}", d.texts());
        view.dop.open = Part::all();
        let f = fields(&mut d, &view);
        assert!((0..6).all(|i| f.contains(&Field::Preset(i))) && f.contains(&Field::Focal));
        assert!((0..7).all(|i| f.contains(&Field::Height(i))) && f.contains(&Field::HeightTyped), "the eye heights beside the lens");
        assert!(d.texts().contains(&"giraffe"));
        assert!(!f.contains(&Field::SetShot) && f.contains(&Field::AddTrack));
        let now = View { target: glam::Vec3::new(0.0, 1.0, 0.0), yaw: 0.3, pitch: 0.1, distance: 3.0, lens: Lens::focal(85.0) };
        let i = view.edit.add_shot(now.shot("")).unwrap();
        view.dop.shot = Some(i);
        view.dop.draft = MoveSpec::of(MoveKind::Spiral);
        let f = fields(&mut d, &view);
        assert!(f.contains(&Field::SetShot) && f.contains(&Field::Scrub(Num::Rounds)) && f.contains(&Field::Turn(Turn::Right)));
        assert!(d.texts().iter().any(|s| s.contains("85 mm")), "a typed focal length shows on its own segment");
        // the access row: closed, then open – who may take it as the one condition control's chips
        assert!(f.contains(&Field::Phones(true)) && !f.contains(&Field::PhonesFact(0)));
        view.edit.set_phones(Some(i), true).unwrap();
        let f = fields(&mut d, &view);
        assert!(f.contains(&Field::PhonesFact(1)) && d.texts().iter().any(|s| s.contains("guest and up may take shot1")) && d.texts().iter().any(|s| s.contains("scene:duet > shot:shot1")));
        // how long without a pose, and Record; while recording: the take's line, Stop and keep, Discard
        assert!(f.contains(&Field::Scrub(Num::Release)) && d.texts().iter().any(|s| s.contains("no limit")) && f.contains(&Field::Record) && !f.contains(&Field::Discard));
        view.edit.set_release(Some(i), Some(4500)).unwrap();
        assert_eq!(value(&view, Num::Release), 4.5);
        view.dop.rec = Some(crate::take::Rec { camera: "scene:duet > shot:shot1".into(), secs: 3.24, poses: 97 });
        let f = fields(&mut d, &view);
        assert!(d.texts().iter().any(|s| s.contains("4.500 s, then it returns")), "{:?}", d.texts());
        assert!(f.contains(&Field::Discard) && d.texts().iter().any(|s| s.starts_with("● REC 3.2 s · 97 poses")), "{:?}", d.texts());
        view.dop.rec = None;
        view.edit.set_phones(Some(i), false).unwrap();
        view.dop.aims = vec!["character:catpope".into(), "spot:altar".into()];
        let f = fields(&mut d, &view);
        assert!(f.contains(&Field::FocusAim) && f.contains(&Field::FocusOn(0)) && !f.contains(&Field::FocusOn(1)), "a subject to focus on, no spot");
        assert!(f.contains(&Field::Scrub(Num::Focus)) && d.texts().iter().any(|s| s.contains("3.00 m")), "at the aim: its distance");
        view.edit.set_focus_on(i, Some("character:catpope")).unwrap();
        let f = fields(&mut d, &view);
        assert!(!f.contains(&Field::Scrub(Num::Focus)) && d.texts().iter().any(|s| s.contains("follows character:catpope")), "on a subject: no distance");
        view.dop.draft = MoveSpec::of(MoveKind::Rack);
        assert!(fields(&mut d, &view).contains(&Field::Repeat(Repeat::PingPong)), "a rack focus repeats");
        view.dop.draft = MoveSpec::of(MoveKind::Spiral);
        view.edit.set_lens(Some(i), Lens::orthographic(6.0)).unwrap();
        let f = fields(&mut d, &view);
        assert!(f.contains(&Field::Scrub(Num::Size)) && !f.contains(&Field::Preset(0)), "orthographic: a view height");
        view.edit.add_track([0.0, 1.0, 2.0], [2.0, 1.0, 2.0]).unwrap();
        view.dop.point = Some(0);
        let f = fields(&mut d, &view);
        assert!(f.contains(&Field::AddPoint) && !f.contains(&Field::RemovePoint), "the start stays");
        assert!(Repeat::ALL.iter().all(|r| f.contains(&Field::Repeat(*r))) && d.texts().contains(&"ping-pong"), "the repeat chip");
        assert!(f.contains(&Field::Point(0)) && f.contains(&Field::Point(1)) && !f.contains(&Field::PointUnit(None)), "the start takes no time");
        view.edit.add_track_point(0).unwrap();
        view.edit.set_point_time(1, Some(Length::Bars(2.0))).unwrap();
        view.edit.set_point_ease(1, Some(Ease::Out)).unwrap();
        view.dop.point = Some(1);
        let f = fields(&mut d, &view);
        let texts = d.texts();
        assert!(texts.iter().any(|s| s.contains("point 2 · 2.000 bars · out")) && texts.iter().any(|s| s.contains("end · the move's length")), "each point with its time: {texts:?}");
        assert!(f.contains(&Field::PointUnit(Some(1))) && f.contains(&Field::Scrub(Num::PointTime)) && f.contains(&Field::PointEase(None)));
        view.dop.draft = MoveSpec::of(MoveKind::Cut);
        assert!(!fields(&mut d, &view).contains(&Field::Repeat(Repeat::Loop)), "a cut does not repeat");
        assert!(height(&t, &view) > t.size.row * 10.0);
    }

    /// every distance reads in the unit chosen for distances, with its switch, and a point's time keeps to the clock's units
    #[test]
    fn distances_and_a_points_time_carry_a_unit_switch() {
        let doc = SceneDocument::new("duet", "Duet", vec!["catpope".into()]);
        let (mut view, _, _) = SceneView::open(SceneOpen { doc, base: Some(1), kept: None, note: None, save: None }, None).expect("it plays");
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        let area = Rect { x: 0.0, y: 0.0, w: WIDTH, h: 4000.0 };
        let feet = Units { distance: Unit::Feet, ..Units::default() };
        let fields = |d: &mut DrawList, view: &SceneView, us: Units| draw(d, &t, area, view, None, us, None).into_iter().map(|(_, f)| f).collect::<Vec<_>>();
        view.dop.open = Part::all();
        let f = fields(&mut d, &view, feet);
        assert!([Unit::Metres, Unit::Centimetres, Unit::Feet].iter().all(|u| f.contains(&Field::DistanceUnit(*u))), "the eye height's switch");
        assert!(d.texts().iter().any(|s| s.contains("ft")), "the eye height typed shows in feet: {:?}", d.texts());
        view.dop.draft = MoveSpec::of(MoveKind::CraneUp);
        let before = d.texts().len();
        fields(&mut d, &view, Units { distance: Unit::Centimetres, ..Units::default() });
        assert!(d.texts()[before..].iter().any(|s| s.ends_with(" cm")), "a crane's height in cm: {:?}", &d.texts()[before..]);
        view.edit.add_track([0.0, 1.0, 2.0], [2.0, 1.0, 2.0]).unwrap();
        view.edit.add_track_point(0).unwrap();
        view.edit.set_point_time(1, Some(Length::Bars(2.0))).unwrap();
        view.dop.point = Some(1);
        let f = fields(&mut d, &view, Units::default());
        assert!(Length::UNITS.iter().enumerate().all(|(k, _)| f.contains(&Field::PointUnit(Some(k)))), "a point's time switches between beats, bars and s");
        assert!(f.contains(&Field::PointUnit(None)), "or arrives by the track's length");
        let moved = Length::Bars(2.0).to_unit(Unit::Seconds, 4, 120.0).unwrap();
        assert_eq!((moved.value(), moved.of_unit()), (4.0, Unit::Seconds), "two bars at 120 BPM are four seconds");
    }

    /// the moves as tiles in the core's groups, each kind once; only what a section holds shows when it is open
    #[test]
    fn the_moves_are_tiles_in_their_groups_and_sections_open_one_by_one() {
        let doc = SceneDocument::new("duet", "Duet", vec!["catpope".into()]);
        let (mut view, _, _) = SceneView::open(SceneOpen { doc, base: Some(1), kept: None, note: None, save: None }, None).expect("it plays");
        let (mut d, t) = (DrawList::new(0.6), Tokens::default());
        let area = Rect { x: 0.0, y: 0.0, w: WIDTH, h: 4000.0 };
        view.dop.open = Part::Move.bit();
        let f: Vec<Field> = draw(&mut d, &t, area, &view, None, Units::default(), None).into_iter().map(|(_, f)| f).collect();
        assert!((0..MoveKind::ALL.len()).all(|i| f.contains(&Field::Kind(i))) && f.contains(&Field::Preview));
        assert!(!f.contains(&Field::Preset(0)) && !f.contains(&Field::AddTrack), "the other sections stay folded");
        let texts = d.texts();
        assert!(texts.contains(&"LOCOMOTION · THE CAMERA TRAVELS") && texts.contains(&"ACCENTS") && texts.contains(&"GLIDE SETTINGS"), "{texts:?}");
        assert!(MoveKind::ALL.iter().all(|k| icons::named(&format!("move-{}", k.word())).is_some()), "every kind has its line art");
        assert!(EYE_HEIGHTS.iter().all(|(n, _)| icons::named(&format!("height-{n}")).is_some()), "every eye height has its figure");
        assert_eq!(Part::all().count_ones() as usize, Part::ALL.len());
    }
}
