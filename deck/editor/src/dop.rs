//! Build's DOP panel: the scene's primary camera – its source, the lens (projection, the focal presets, a typed focal
//! length, an orthographic view's height, the focus: at the aim, a distance or a subject), its shots (taken from the view, looked through), a move to preview or keep,
//! the kept moves, the trolley track. The camera, its checks and the moves are the core's (deck-space scene_camera.rs,
//! scene_dop.rs, space/dop); here the panel is drawn and its presses go to the core.
use crate::scene::SceneView;
use deck_kit::base::draw::DrawList;
use deck_kit::base::tokens::Tokens;
use deck_kit::editor::controls::{self, Rect};
use deck_space::scene_camera::{fov_of, height_preset, Ease, EYE_HEIGHTS, Length, Lens, MoveKind, MoveSpec, Projection, Repeat, SceneCamera, Side, Source, Start, Turn, FOCAL_PRESETS, FOCUS_RANGE, MAX_ROUNDS, SIZE_RANGE};
use deck_space::space::dop::View;

pub const WIDTH: f32 = 340.0;
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
}

impl Default for Sel {
    fn default() -> Sel {
        Sel { shot: None, point: None, kept: None, draft: MoveSpec::of(MoveKind::Glide), floor: 0.0, aims: Vec::new() }
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
    /// the selected point's time: by the track's length (none), or in `UNITS[i]`
    PointUnit(Option<usize>),
    /// the ease of the way to the selected point: even (none), or `POINT_EASES[i]`
    PointEase(Option<usize>),
    /// the camera's slot `i` (a module drops on it), its module taken off
    Slot(usize),
    Detach(usize),
}

const UNITS: [&str; 3] = ["beats", "bars", "s"];
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

#[allow(clippy::too_many_arguments)]
fn scrub(d: &mut DrawList, t: &Tokens, a: Rect, y: &mut f32, n: Num, label: &str, v: f32, shown: &str, hot: Option<Num>, out: &mut Vec<(Rect, Field)>) {
    let (min, max, log) = n.range();
    let r = controls::scrub(d, t, Rect { x: a.x, y: *y, w: a.w, h: t.size.row }, label, shown, crate::fraction(v, min, max, log), hot == Some(n));
    out.push((r, Field::Scrub(n)));
    *y += t.size.row;
}

/// the lens the panel edits: the selected shot's, else the camera's
pub fn lens_of(cam: &SceneCamera, sel: &Sel) -> Lens {
    sel.shot.and_then(|i| cam.shots.get(i)).map_or(cam.lens, |s| s.lens)
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
    }
}

/// the focus distance the panel shows (m): the lens's, else the selected shot's aim's distance
fn focus_of(cam: &SceneCamera, sel: &Sel) -> f32 {
    let shot = sel.shot.and_then(|i| cam.shots.get(i));
    lens_of(cam, sel).focus.unwrap_or_else(|| shot.map_or(3.0, |s| s.distance))
}

/// The focus: at the aim, a distance (scrubbed) or – a shot's – a subject on the stage it follows.
fn focus_rows(d: &mut DrawList, t: &Tokens, a: Rect, y: &mut f32, view: &SceneView, hot: Option<Num>, out: &mut Vec<(Rect, Field)>) {
    let (cam, sel) = (&view.edit.camera(), &view.dop);
    let lens = lens_of(cam, sel);
    let shot = sel.shot.and_then(|i| cam.shots.get(i));
    let on = shot.and_then(|s| s.focus_on.as_deref());
    caption(d, t, a, y, "focus");
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
    let (rects, used) = controls::chips(d, t, (a.x, *y, a.w), &names, &lit);
    out.extend(rects.into_iter().zip(fields));
    *y += used + 6.0;
    match on {
        Some(s) => caption(d, t, a, y, &format!("follows {s}: drag the marker off it for a distance")),
        None => {
            let v = focus_of(cam, sel);
            let shown = if lens.focus.is_some() { format!("{} m", crate::number(v)) } else { format!("{} m · the aim", crate::number(v)) };
            scrub(d, t, a, y, Num::Focus, "focus distance", v, &shown, hot, out);
        }
    }
}

/// The panel; where its fields landed.
pub fn draw(d: &mut DrawList, t: &Tokens, a: Rect, view: &SceneView, playing: Playing, hot: Option<Num>) -> Vec<(Rect, Field)> {
    let mut out = Vec::new();
    let cam = view.edit.camera();
    let sel = &view.dop;
    let mut y = a.y;
    y += controls::section(d, t, a.x, y, a.w, "CAMERA", true);
    let on = usize::from(cam.source == Source::Handheld);
    pills_row(d, t, a, &mut y, &["Scene camera", "Handheld – later"], on, &[Field::Source(Source::Scene), Field::Source(Source::Handheld)], &mut out);
    if cam.source == Source::Handheld {
        caption(d, t, a, &mut y, "a phone steers it – not built yet; the shots and moves wait");
    }

    let lens = lens_of(&cam, sel);
    let whose = sel.shot.and_then(|i| cam.shots.get(i)).map_or("LENS · THE CAMERA".to_string(), |s| format!("LENS · {}", s.id.to_uppercase()));
    y += controls::section(d, t, a.x, y, a.w, &whose, false);
    pills_row(d, t, a, &mut y, &["Perspective", "Orthographic"], usize::from(lens.ortho()), &[Field::Projection(Projection::Perspective), Field::Projection(Projection::Orthographic)], &mut out);
    if lens.ortho() {
        scrub(d, t, a, &mut y, Num::Size, "view height", lens.size, &format!("{} m", crate::number(lens.size)), hot, &mut out);
    } else {
        let mut names: Vec<String> = FOCAL_PRESETS.iter().map(|f| format!("{f}")).collect();
        names.push(if lens.preset().is_none() { format!("{} mm", mm(lens.focal)) } else { "mm…".into() });
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let fields: Vec<Field> = (0..FOCAL_PRESETS.len()).map(Field::Preset).chain([Field::Focal]).collect();
        pills_row(d, t, a, &mut y, &names, lens.preset().unwrap_or(FOCAL_PRESETS.len()), &fields, &mut out);
        caption(d, t, a, &mut y, &format!("{} mm · {:.0}° high on full frame", mm(lens.focal), fov_of(lens.focal).to_degrees()));
    }
    focus_rows(d, t, a, &mut y, view, hot, &mut out);
    let height = sel.shot.and_then(|i| cam.shots.get(i)).map_or(cam.height, |s| s.height(sel.floor));
    let preset = height_preset(height);
    let mut names: Vec<String> = EYE_HEIGHTS.iter().map(|(n, _)| n.to_string()).collect();
    names.push(format!("{} m…", crate::number(height)));
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let lit: Vec<bool> = (0..=EYE_HEIGHTS.len()).map(|i| preset.map_or(i == EYE_HEIGHTS.len(), |p| p == i)).collect();
    let (rects, used) = controls::chips(d, t, (a.x, y, a.w), &names, &lit);
    out.extend(rects.into_iter().enumerate().map(|(i, r)| (r, if i < EYE_HEIGHTS.len() { Field::Height(i) } else { Field::HeightTyped })));
    y += used + 6.0;

    y += controls::section(d, t, a.x, y, a.w, "SHOTS", false);
    if cam.shots.is_empty() {
        caption(d, t, a, &mut y, "none yet: frame the stage and take the view as a shot");
    }
    for (i, s) in cam.shots.iter().enumerate() {
        let r = Rect { x: a.x, y, w: a.w, h: t.size.tree_row };
        let picked = sel.shot == Some(i);
        if picked {
            d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.selection);
        }
        let first = if i == 0 { " · opens" } else { "" };
        let line = format!("{}{first}   {}", s.id, shot_words(s));
        d.text(r.x + 6.0, r.y + (r.h - t.size.small * 1.25) * 0.5, t.size.small, &controls::fit(&line, ((r.w - 12.0) / d.advance(t.size.small)) as usize), if picked { t.color.bright } else { t.color.text });
        out.push((r, Field::Shot(i)));
        y += t.size.tree_row;
    }
    y += 4.0;
    let mut names = vec!["Add from view"];
    let mut fields = vec![Field::AddShot];
    if sel.shot.is_some() {
        names.extend(["Set from view", "Remove"]);
        fields.extend([Field::SetShot, Field::RemoveShot]);
    }
    pills_row(d, t, a, &mut y, &names, usize::MAX, &fields, &mut out);
    if let Some(s) = sel.shot.and_then(|i| cam.shots.get(i)) {
        caption(d, t, a, &mut y, "aims at");
        let mut names = vec!["its point"];
        names.extend(sel.aims.iter().map(String::as_str));
        let lit: Vec<bool> = std::iter::once(s.aim.is_none()).chain(sel.aims.iter().map(|x| s.aim.as_deref() == Some(x.as_str()))).collect();
        let (rects, used) = controls::chips(d, t, (a.x, y, a.w), &names, &lit);
        out.extend(rects.into_iter().enumerate().map(|(i, r)| (r, Field::Aim(i.checked_sub(1)))));
        y += used + 6.0;
    }

    y += controls::section(d, t, a.x, y, a.w, "MOVE", false);
    let s = &sel.draft;
    let kinds: Vec<&str> = MoveKind::ALL.iter().map(|k| k.word()).collect();
    let lit: Vec<bool> = MoveKind::ALL.iter().map(|k| *k == s.kind).collect();
    let (rects, used) = controls::chips(d, t, (a.x, y, a.w), &kinds, &lit);
    out.extend(rects.into_iter().enumerate().map(|(i, r)| (r, Field::Kind(i))));
    y += used + 6.0;
    let to = sel.shot.and_then(|i| cam.shots.get(i)).map_or("pick a shot above", |s| s.id.as_str());
    controls::row(d, t, a, &mut y, "ends on", to);
    if s.kind != MoveKind::Cut {
        let unit = unit_of(s.length);
        scrub(d, t, a, &mut y, Num::Length, "length", s.length.value(), &format!("{} {}", crate::number(s.length.value()), s.length.unit()), hot, &mut out);
        pills_row(d, t, a, &mut y, &UNITS, unit, &[Field::Unit(0), Field::Unit(1), Field::Unit(2)], &mut out);
        let eases: Vec<&str> = Ease::ALL.iter().map(|e| e.word()).collect();
        let fields: Vec<Field> = (0..Ease::ALL.len()).map(Field::Ease).collect();
        pills_row(d, t, a, &mut y, &eases, Ease::ALL.iter().position(|e| *e == s.ease).unwrap_or(0), &fields, &mut out);
    }
    pills_row(d, t, a, &mut y, &["now", "on the bar line"], usize::from(s.start == Start::Bar), &[Field::Start(Start::Now), Field::Start(Start::Bar)], &mut out);
    if !matches!(s.kind, MoveKind::Cut | MoveKind::Follow) {
        let names: Vec<&str> = Repeat::ALL.iter().map(|r| r.word()).collect();
        let lit: Vec<bool> = Repeat::ALL.iter().map(|r| *r == s.repeat).collect();
        let (rects, used) = controls::chips(d, t, (a.x, y, a.w), &names, &lit);
        out.extend(rects.into_iter().zip(Repeat::ALL).map(|(r, k)| (r, Field::Repeat(k))));
        y += used + 6.0;
    }
    match s.kind {
        MoveKind::Orbit | MoveKind::Spiral => {
            scrub(d, t, a, &mut y, Num::Rounds, "rounds", s.rounds, &crate::number(s.rounds), hot, &mut out);
            pills_row(d, t, a, &mut y, &["turn left", "turn right"], usize::from(s.turn == Turn::Right), &[Field::Turn(Turn::Left), Field::Turn(Turn::Right)], &mut out);
        }
        MoveKind::In => pills_row(d, t, a, &mut y, &["from the left", "from the right"], usize::from(s.side == Side::Right), &[Field::Side(Side::Left), Field::Side(Side::Right)], &mut out),
        MoveKind::CraneDown | MoveKind::CraneUp => scrub(d, t, a, &mut y, Num::Height, "height", s.height, &format!("{} m", crate::number(s.height)), hot, &mut out),
        MoveKind::Shake => scrub(d, t, a, &mut y, Num::Amount, "unrest", s.amount, &format!("{}°", crate::number(s.amount)), hot, &mut out),
        MoveKind::Follow => {
            let holders: Vec<&str> = view.edit.doc.holders.iter().map(|h| h.id.as_str()).collect();
            if holders.is_empty() {
                caption(d, t, a, &mut y, "the scene has no holder to follow");
            } else {
                let on = s.follow.as_deref().and_then(|f| holders.iter().position(|h| *h == f)).unwrap_or(usize::MAX);
                let fields: Vec<Field> = (0..holders.len()).map(Field::Follow).collect();
                pills_row(d, t, a, &mut y, &holders, on, &fields, &mut out);
            }
        }
        MoveKind::Fly if cam.track.is_none() => caption(d, t, a, &mut y, "runs along the trolley track: add one below"),
        _ => {}
    }
    if let Some((kind, shot, p)) = &playing {
        let bar = Rect { x: a.x, y: y + 2.0, w: a.w, h: 3.0 };
        d.rect(bar.x, bar.y, bar.w, bar.h, t.color.control);
        d.rect(bar.x, bar.y, bar.w * p.clamp(0.0, 1.0), bar.h, t.color.accent);
        y += 8.0;
        caption(d, t, a, &mut y, &format!("playing: {kind} to {shot} · {:.0} %", p * 100.0));
    }
    pills_row(d, t, a, &mut y, &["Preview", "Keep as move"], usize::MAX, &[Field::Preview, Field::Keep], &mut out);

    if !cam.moves.is_empty() {
        y += controls::section(d, t, a.x, y, a.w, "MOVES", false);
        for (i, m) in cam.moves.iter().enumerate() {
            let r = Rect { x: a.x, y, w: a.w, h: t.size.tree_row };
            if sel.kept == Some(i) {
                d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.selection);
            }
            let then = m.then.as_deref().map_or(String::new(), |n| format!(" → {n}"));
            let repeat = if m.spec.repeat == Repeat::Once { String::new() } else { format!(" · {}", m.spec.repeat.word()) };
            let line = format!("{} · {} to {} · {} {}{repeat}{then}", m.id, m.spec.kind.word(), m.to, crate::number(m.spec.length.value()), m.spec.length.unit());
            d.text(r.x + 6.0, r.y + (r.h - t.size.small * 1.25) * 0.5, t.size.small, &controls::fit(&line, ((r.w - 12.0) / d.advance(t.size.small)) as usize), t.color.text);
            out.push((r, Field::Kept(i)));
            y += t.size.tree_row;
        }
        y += 4.0;
        if sel.kept.is_some() {
            pills_row(d, t, a, &mut y, &["Play", "Remove"], usize::MAX, &[Field::PlayKept, Field::RemoveKept], &mut out);
        }
    }

    y += controls::section(d, t, a.x, y, a.w, "TROLLEY TRACK", false);
    match cam.track.as_ref() {
        None => pills_row(d, t, a, &mut y, &["Add track"], usize::MAX, &[Field::AddTrack], &mut out),
        Some(track) => {
            let len = deck_space::space::dop::track::Path::of(track).map_or(0.0, |p| p.len());
            let at = sel.point.map_or(String::new(), |p| format!(" · point {} selected", p + 1));
            caption(d, t, a, &mut y, &format!("{} points · {} m{at} · drag them on the stage", track.points.len(), crate::number(len)));
            for i in 0..track.points.len() {
                let r = Rect { x: a.x, y, w: a.w, h: t.size.tree_row };
                let picked = sel.point == Some(i);
                if picked {
                    d.round_rect(r.x, r.y, r.w, r.h, t.size.radius, t.color.selection);
                }
                let line = point_words(track, i);
                d.text(r.x + 6.0, r.y + (r.h - t.size.small * 1.25) * 0.5, t.size.small, &controls::fit(&line, ((r.w - 12.0) / d.advance(t.size.small)) as usize), if picked { t.color.bright } else { t.color.text });
                out.push((r, Field::Point(i)));
                y += t.size.tree_row;
            }
            y += 4.0;
            if let Some(p) = sel.point.filter(|p| *p > 0).and_then(|i| track.points.get(i)) {
                caption(d, t, a, &mut y, "arrives");
                let mut names = vec!["by length"];
                names.extend(UNITS);
                let lit: Vec<bool> = (0..=UNITS.len()).map(|k| p.time.map_or(k == 0, |t| k == unit_of(t) + 1)).collect();
                let (rects, used) = controls::chips(d, t, (a.x, y, a.w), &names, &lit);
                out.extend(rects.into_iter().enumerate().map(|(k, r)| (r, Field::PointUnit(k.checked_sub(1)))));
                y += used + 6.0;
                if let Some(time) = p.time {
                    scrub(d, t, a, &mut y, Num::PointTime, "at", time.value(), &format!("{} {}", crate::number(time.value()), time.unit()), hot, &mut out);
                }
                let mut names = vec!["even"];
                names.extend(POINT_EASES.iter().map(|e| e.word()));
                let lit: Vec<bool> = std::iter::once(p.ease.is_none()).chain(POINT_EASES.iter().map(|e| p.ease == Some(*e))).collect();
                let (rects, used) = controls::chips(d, t, (a.x, y, a.w), &names, &lit);
                out.extend(rects.into_iter().enumerate().map(|(k, r)| (r, Field::PointEase(k.checked_sub(1)))));
                y += used + 6.0;
            }
            let mut names = vec!["Add point"];
            let mut fields = vec![Field::AddPoint];
            if sel.point.is_some_and(|p| p > 0 && p + 1 < track.points.len()) {
                names.push("Remove point");
                fields.push(Field::RemovePoint);
            }
            names.push("Remove track");
            fields.push(Field::RemoveTrack);
            pills_row(d, t, a, &mut y, &names, usize::MAX, &fields, &mut out);
        }
    }
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
    let fields = draw(&mut d, t, Rect { x: 0.0, y: 0.0, w: WIDTH, h: 4000.0 }, view, None, None);
    fields.iter().map(|(r, _)| r.y + r.h).fold(0.0, f32::max) + t.size.pad + t.size.row * 1.5
}

impl crate::App {
    fn dop_done(&mut self, done: Result<(), String>, said: String, x: f32, y: f32) {
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
        let Some(view) = self.scene.as_mut() else { return };
        let cam = view.edit.camera();
        let sel = &mut view.dop;
        let lens = lens_of(&cam, sel);
        let shot = sel.shot;
        match f {
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
            Field::Focal => self.begin_edit(crate::Editing::Focal, crate::Hit::Dop(Field::Focal), mm(lens.focal)),
            Field::Height(i) => {
                let (name, h) = EYE_HEIGHTS[i];
                let done = view.edit.set_height(shot, h, view.dop.floor);
                self.dop_done(done, format!("eye height: {name}, {} m", crate::number(h)), x, y);
                self.dop_relook(shot);
            }
            Field::HeightTyped => {
                let now = shot.and_then(|i| cam.shots.get(i)).map_or(cam.height, |s| s.height(view.dop.floor));
                self.begin_edit(crate::Editing::Height, crate::Hit::Dop(Field::HeightTyped), crate::number(now));
            }
            Field::Scrub(_) => {}
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
                let v = sel.draft.length.value();
                sel.draft.length = [Length::Beats(v), Length::Bars(v), Length::Seconds(v)][u];
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
                let time = u.map(|u| {
                    let v = track.points.get(i).and_then(|p| p.time).map(|t| t.value()).unwrap_or_else(|| {
                        track.points[..i].iter().filter_map(|p| p.time).map(|t| t.value()).fold(0.0, f32::max) + 1.0
                    });
                    [Length::Beats(v), Length::Bars(v), Length::Seconds(v)][u]
                });
                let done = view.edit.set_point_time(i, time);
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
            _ => {}
        }
    }

    /// A check run's camera (`--dop shots+track+preview`): three shots around the view (the second a long lens, the third
    /// isometric), a track with a point between that is selected, a spiral previewed to the second shot.
    pub(crate) fn script_dop(&mut self) {
        let Some(spec) = self.args.dop.clone() else { return };
        let (x, y) = self.cursor;
        let now = View::of(&self.stage.space.camera);
        match self.frames {
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

    /// an eye height typed (m)
    pub(crate) fn dop_height(&mut self, text: &str) {
        let (x, y) = self.cursor;
        let Some(view) = self.scene.as_mut() else { return };
        let Some(h) = crate::typed_number(text.trim().trim_end_matches('m')) else { return self.refuse(format!("'{text}': an eye height in metres"), x, y) };
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
        let Some(view) = self.scene.as_mut() else { return };
        let Ok(focal) = text.trim().trim_end_matches("mm").trim().replace(',', ".").parse::<f32>() else { return self.refuse(format!("'{text}': a focal length in mm"), x, y) };
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
        let fields = |d: &mut DrawList, view: &SceneView| draw(d, &t, area, view, None, None).into_iter().map(|(_, f)| f).collect::<Vec<_>>();
        let f = fields(&mut d, &view);
        assert!(f.contains(&Field::Source(Source::Handheld)) && d.texts().iter().any(|s| s.contains("Handheld – later")));
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
        view.dop.aims = vec!["character:catpope".into(), "spot:altar".into()];
        let f = fields(&mut d, &view);
        assert!(f.contains(&Field::FocusAim) && f.contains(&Field::FocusOn(0)) && !f.contains(&Field::FocusOn(1)), "a subject to focus on, no spot");
        assert!(f.contains(&Field::Scrub(Num::Focus)) && d.texts().iter().any(|s| s.contains("3.000 m · the aim")), "at the aim: its distance");
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
}
