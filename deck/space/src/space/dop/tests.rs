use super::*;
use crate::scene_camera::{Ease, Length, MoveDef, Repeat, Track, TrackPoint, Turn};

fn shot(id: &str, target: [f32; 3], yaw: f32, pitch: f32, distance: f32, lens: Lens) -> ShotDef {
    ShotDef { id: id.into(), label: Default::default(), target, yaw, pitch, distance, lens, aim: None, focus_on: None, phones: None }
}

fn camera() -> SceneCamera {
    let track = Track {
        points: vec![
            TrackPoint::at([-4.0, 1.6, 3.0]),
            TrackPoint::at([0.0, 1.2, 0.5]),
            TrackPoint { handle: Some([1.0, 0.0, -1.0]), ..TrackPoint::at([4.0, 2.0, 2.0]) },
        ],
    };
    SceneCamera {
        shots: vec![
            shot("wide", [0.0, 1.0, 0.0], 20.0, 8.0, 6.0, Lens::focal(30.0)),
            shot("close", [0.2, 1.5, 0.0], -35.0, 4.0, 1.6, Lens::focal(70.0)),
            shot("iso", [0.0, 0.5, 0.0], 45.0, 35.0, 12.0, Lens::orthographic(5.0)),
        ],
        track: Some(track),
        ..SceneCamera::default()
    }
}

fn idle(secs: f64) -> Clock {
    Clock { secs, beat: 0.0, bpm: 120.0, per_bar: 4, running: false }
}

fn at_beat(beat: f64) -> Clock {
    Clock { secs: beat * 0.5, beat, bpm: 120.0, per_bar: 4, running: true }
}

fn start() -> View {
    View { target: Vec3::new(1.0, 1.2, -0.5), yaw: 2.0, pitch: 0.3, distance: 3.0, lens: Lens::focal(50.0) }
}

fn no_one(_: &str) -> Option<Vec3> {
    None
}

/// every frame of a move at 60 fps from `from`; the last is where it ended
fn run(dop: &mut Dop, spec: MoveSpec, to: &str, from: View) -> Vec<View> {
    dop.play(vec![dop.cue(spec, to).unwrap()]);
    let mut now = from;
    let mut out = Vec::new();
    for f in 0..=2000 {
        let Some(v) = dop.step(&idle(f64::from(f) / 60.0), now, &no_one) else { break };
        out.push(v);
        now = v;
        if !dop.director.active() {
            break;
        }
    }
    out
}

/// how high the view is at its aim (m)
fn view_height(v: &View) -> f32 {
    if v.lens.ortho() {
        v.lens.size
    } else {
        2.0 * v.distance * (v.lens.fov_y() * 0.5).tan()
    }
}

/// where fixed points around the stage land on the screen through `v` (NDC)
fn on_screen(v: &View) -> Vec<glam::Vec2> {
    let mut o = Orbit::new(Vec3::ZERO, 0.0, 0.0, 1.0);
    v.apply(&mut o);
    let vp = o.view_proj(16.0 / 9.0);
    [[0.0, 1.0, 0.0], [0.6, 0.5, 0.3], [-0.5, 1.4, -0.4], [0.2, 0.0, 0.6]].iter().map(|p| vp.project_point3(Vec3::from_array(*p)).truncate()).collect()
}

/// the most any of those points moves on the screen from one frame to the next
fn picture_jump(frames: &[View]) -> f32 {
    frames.windows(2).map(|w| on_screen(&w[0]).iter().zip(on_screen(&w[1])).map(|(a, b)| a.distance(b)).fold(0.0, f32::max)).fold(0.0, f32::max)
}

fn spec(kind: MoveKind) -> MoveSpec {
    let mut s = MoveSpec::of(kind);
    s.length = if kind == MoveKind::Cut { Length::Seconds(0.0) } else { Length::Seconds(2.0) };
    if kind == MoveKind::Follow {
        s.follow = Some("pope".into());
    }
    s
}

#[test]
fn every_move_ends_exactly_on_its_shot_and_starts_where_the_camera_is() {
    let mut dop = Dop::new(None, camera());
    // a rack keeps its frame (a_rack_focus_keeps_the_frame_and_shifts_the_focus)
    for kind in MoveKind::ALL.into_iter().filter(|k| *k != MoveKind::Rack) {
        for to in ["close", "iso", "wide"] {
            let frames = run(&mut dop, spec(kind), to, start());
            let end = dop.view(to).unwrap();
            assert_eq!(*frames.last().unwrap(), end, "{kind:?} → {to}: exactly the shot");
            if kind == MoveKind::Cut {
                assert_eq!(frames.len(), 1, "a cut is there at once");
                continue;
            }
            assert!(frames[0].gap(&start()) < 1e-3, "{kind:?} → {to}: no jump at the start ({})", frames[0].gap(&start()));
            assert!(frames.len() > 100, "{kind:?}: it takes its two seconds");
            // no jump on the way: every frame near the one before (a whip is fast, a fly-through long); into the orthographic
            // shot the eye backs far off on purpose – there the picture is what must not jump
            if to == "iso" {
                let jump = frames.windows(2).map(|w| w[0].target.distance(w[1].target) + (view_height(&w[1]) / view_height(&w[0]) - 1.0).abs()).fold(0.0, f32::max);
                assert!(jump < 2.5, "{kind:?} → iso: the picture jumps {jump}");
                continue;
            }
            let worst = frames.windows(2).map(|w| w[0].eye().distance(w[1].eye()) + w[0].target.distance(w[1].target)).fold(0.0, f32::max);
            assert!(worst < 2.5, "{kind:?} → {to}: a frame jumps {worst} m");
        }
    }
}

#[test]
fn a_hand_interrupts_a_move_where_it_is_and_the_next_starts_from_there() {
    let mut dop = Dop::new(None, camera());
    dop.play(vec![dop.cue(spec(MoveKind::Spiral), "close").unwrap()]);
    let mut orbit = Orbit::new(start().target, start().yaw, start().pitch, start().distance);
    let mut last = None;
    for f in 0..60 {
        if let Some(v) = dop.step(&idle(f64::from(f) / 60.0), View::of(&orbit), &no_one) {
            v.apply(&mut orbit);
            last = Some(v);
        }
    }
    let mid = last.unwrap();
    let eye = orbit.eye();
    // a drag: the director lets go, the camera goes on from the view it had – no jump
    orbit.drag(0.0, 0.0);
    assert!(orbit.take_touched());
    dop.director.interrupt();
    assert_eq!(dop.step(&idle(1.01), View::of(&orbit), &no_one), None, "the camera is the hand's");
    orbit.step(1.0 / 60.0);
    assert!(orbit.eye().distance(eye) < 1e-4, "{:?} vs {:?}", orbit.eye(), eye);
    assert!(View::of(&orbit).gap(&mid) < 1e-3);
    // a new move from there starts on the view the hand left
    dop.play(vec![dop.cue(spec(MoveKind::Glide), "wide").unwrap()]);
    let first = dop.step(&idle(1.02), View::of(&orbit), &no_one).unwrap();
    assert!(first.gap(&View::of(&orbit)) < 0.05, "{}", first.gap(&View::of(&orbit)));
}

#[test]
fn beat_lengths_land_on_the_bar_line() {
    let mut dop = Dop::new(None, camera());
    let mut s = MoveSpec::of(MoveKind::Glide);
    (s.length, s.start) = (Length::Bars(2.0), Start::Bar);
    dop.play(vec![dop.cue(s, "close").unwrap()]);
    let end = dop.view("close").unwrap();
    let mut beat = 1.3;
    let mut started = None;
    let mut arrived = None;
    while beat < 20.0 {
        match dop.step(&at_beat(beat), start(), &no_one) {
            None => assert!(started.is_none() || arrived.is_some(), "waits only before it starts"),
            Some(v) => {
                started.get_or_insert(beat);
                if v == end && arrived.is_none() {
                    arrived = Some(beat);
                }
            }
        }
        beat += 0.01;
    }
    let (started, arrived) = (started.unwrap(), arrived.unwrap());
    assert!((4.0..4.011).contains(&started), "queued to the next bar line: {started}");
    assert!((11.999..12.011).contains(&arrived), "two bars later, on the bar line: {arrived}");
    // started now (not on the bar), a whole number of bars from the start
    let mut s = MoveSpec::of(MoveKind::Orbit);
    s.length = Length::Beats(6.0);
    dop.play(vec![dop.cue(s, "wide").unwrap()]);
    assert!(dop.step(&at_beat(5.5), start(), &no_one).is_some());
    assert_ne!(dop.step(&at_beat(11.49), start(), &no_one), dop.view("wide"));
    assert_eq!(dop.step(&at_beat(11.5), start(), &no_one), dop.view("wide"));
}

#[test]
fn a_chain_plays_one_move_after_the_other_from_where_the_last_ended() {
    let mut cam = camera();
    let mut a = MoveSpec::of(MoveKind::Push);
    a.length = Length::Seconds(1.0);
    let mut b = MoveSpec::of(MoveKind::Orbit);
    (b.length, b.rounds, b.turn) = (Length::Seconds(2.0), 1.0, Turn::Right);
    cam.moves.push(MoveDef { id: "intro".into(), to: "close".into(), spec: a, then: Some("round".into()) });
    cam.moves.push(MoveDef { id: "round".into(), to: "wide".into(), spec: b, then: None });
    let mut dop = Dop::new(Some("scene:nave".into()), cam);
    assert_eq!(dop.word("scene:nave", "move:intro"), Some(Ok("camera: move:intro".into())));
    let close = dop.view("close").unwrap();
    let mut now = start();
    let mut seen_close = false;
    let mut last = now;
    for f in 0..=200 {
        let secs = f64::from(f) / 60.0;
        let Some(v) = dop.step(&idle(secs), now, &no_one) else { break };
        if (secs - 1.0).abs() < 1e-9 {
            seen_close = v.gap(&close) < 1e-3;
        }
        assert!(v.eye().distance(last.eye()) < 0.6 || f == 0, "frame {f}");
        (now, last) = (v, v);
    }
    assert!(seen_close, "the first ends on its shot at 1 s");
    assert_eq!(now, dop.view("wide").unwrap(), "the chain ends on the last shot at 3 s");
    assert!(!dop.director.active());
}

#[test]
fn a_follow_keeps_its_subject_in_the_shot_until_a_hand() {
    let mut dop = Dop::new(None, camera());
    let mut s = spec(MoveKind::Follow);
    s.length = Length::Seconds(0.5);
    dop.play(vec![dop.cue(s, "close").unwrap()]);
    let walk = |secs: f64| Vec3::new(secs as f32 * 0.5, 1.0, 0.0);
    let mut v = start();
    for f in 0..120 {
        let secs = f64::from(f) / 60.0;
        let subject = move |h: &str| (h == "pope").then(|| walk(secs));
        v = dop.step(&idle(secs), v, &subject).unwrap();
    }
    assert!(v.target.distance(walk(119.0 / 60.0)) < 1e-5, "aimed at where the subject is now");
    let close = dop.view("close").unwrap();
    assert_eq!((v.yaw, v.pitch, v.distance, v.lens), (close.yaw, close.pitch, close.distance, close.lens), "with the shot's angle and lens");
    dop.director.interrupt();
    assert_eq!(dop.step(&idle(3.0), v, &|_| Some(Vec3::ZERO)), None);
}

#[test]
fn the_shapes_of_the_moves() {
    let mut dop = Dop::new(None, camera());
    // a fly-through passes the track's middle point
    let frames = run(&mut dop, spec(MoveKind::Fly), "close", start());
    let mid = Vec3::new(0.0, 1.2, 0.5);
    let near = frames.iter().map(|v| v.eye().distance(mid)).fold(f32::MAX, f32::min);
    assert!(near < 0.6, "the fly-through runs along the track: {near}");
    // an orbit goes all the way round: the eye is behind the aim at some point
    let mut o = spec(MoveKind::Orbit);
    o.rounds = 1.0;
    let wide = dop.view("wide").unwrap();
    let frames = run(&mut dop, o, "wide", wide);
    let far = frames.iter().map(|v| v.eye().distance(wide.eye())).fold(0.0, f32::max);
    assert!(far > wide.distance * 1.8, "round to the other side: {far}");
    // a crane down rises above both ends first
    let frames = run(&mut dop, spec(MoveKind::CraneDown), "close", start());
    let top = frames.iter().map(|v| v.eye().y).fold(f32::MIN, f32::max);
    assert!(top > start().eye().y.max(dop.view("close").unwrap().eye().y) + 1.0, "{top}");
    // a dolly zoom keeps the subject's size while the distance changes
    let mut same = dop.view("close").unwrap();
    same.distance = 3.2;
    same.lens.focal = 140.0;
    dop.camera.shots.push(View { target: dop.view("close").unwrap().target, ..same }.shot("far"));
    let close = dop.view("close").unwrap();
    let frames = run(&mut dop, spec(MoveKind::DollyZoom), "far", close);
    let width = |v: &View| 2.0 * v.distance * (v.lens.fov_y() * 0.5).tan();
    let w0 = width(&frames[0]);
    assert!(frames.iter().all(|v| (width(v) / w0 - 1.0).abs() < 0.01), "the subject keeps its size");
    // a shake is unrest on the way, none at the ends
    let mut sh = spec(MoveKind::Shake);
    sh.ease = Ease::Linear;
    let calm = run(&mut dop, MoveSpec { kind: MoveKind::Glide, ..sh.clone() }, "wide", start());
    let shaken = run(&mut dop, sh, "wide", start());
    let diff = calm.iter().zip(&shaken).map(|(a, b)| (a.yaw - b.yaw).abs()).fold(0.0, f32::max);
    assert!(diff > 0.005, "it shakes: {diff}");
}

#[test]
fn camera_words_cut_and_play_only_for_their_scene() {
    let mut dop = Dop::new(Some("scene:nave".into()), camera());
    assert_eq!(dop.word("scene:other", "shot:wide"), None, "another scene's");
    assert_eq!(dop.word("scene:nave", "join:character:catpope"), None, "no camera word");
    assert!(matches!(dop.word("scene:nave", "shot:nowhere"), Some(Err(_))));
    assert!(matches!(dop.word("scene:nave", "move:nothing"), Some(Err(_))));
    assert_eq!(dop.word("scene:*", "shot:iso"), Some(Ok("camera: shot:iso".into())));
    assert_eq!(dop.current.as_deref(), Some("iso"));
    let v = dop.step(&idle(0.0), start(), &no_one).unwrap();
    assert_eq!(v, dop.view("iso").unwrap(), "a shot is a cut");
    let mut orbit = Orbit::new(Vec3::ZERO, 0.0, 0.0, 4.0);
    v.apply(&mut orbit);
    assert_eq!(orbit.ortho, Some(5.0), "the iso shot looks orthographic");
    let p = orbit.proj(1.0);
    assert!((p.w_axis.w - 1.0).abs() < 1e-6, "an orthographic projection keeps w");
    assert!(View::of(&orbit).gap(&v) < 1e-3, "the orbit's view is the shot's");
}

/// The parallax layers fit the primary camera (the opening shot, its lens): a full layer covers that camera's whole view
/// wherever the hand has turned the stage
#[test]
fn a_layer_covers_its_part_of_the_primary_cameras_view() {
    use crate::scene_document::SceneDocument;
    use crate::scene_resolve::{core_rigs, no_sets, resolve};
    use crate::space::portrait::Portraits;
    use deck_catalog::layer::Anchor;
    let mut doc = SceneDocument::new("duet", "Duet", vec!["catpope".into()]);
    doc.recast(&["character:catpope".into()]);
    doc.camera = Some(camera());
    doc.stage.layers = vec![crate::Layer::empty(5.0, Anchor::Full)];
    let scene = resolve(&doc, &Default::default(), true, &core_rigs, &no_sets).unwrap().scene;
    let g = crate::graph::Graph::build(&scene).unwrap();
    let mut s = crate::space::Space::new(&g, g.first(crate::graph::Kind::Space).unwrap(), &scene.being, std::path::Path::new("/nowhere"), Portraits::manual());
    s.camera.free = true;
    s.set_scene(&scene);
    s.set_view(1600.0, 900.0);
    s.camera.drag(300.0, -120.0);
    let mut planes = Vec::new();
    s.layer_planes(true, &mut planes);
    let p = planes.first().expect("the layer");
    let mut primary = Orbit::new(Vec3::ZERO, 0.0, 0.0, 1.0);
    View::of_shot(&camera().shots[0]).apply(&mut primary);
    let vp = primary.view_proj(1600.0 / 900.0);
    for (c, want) in p.corners().into_iter().zip([(-1.0, 1.0), (1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)]) {
        let q = vp * c.extend(1.0);
        let (x, y) = (q.x / q.w, q.y / q.w);
        assert!((x - want.0).abs() < 1e-3 && (y - want.1).abs() < 1e-3, "a corner on the view's corner: ({x}, {y}) vs {want:?}");
    }
}

/// A scene without a camera – new, or cast but never given one – opens head-on: yaw 0, level at an adult's eye height,
/// on the stage's centre line, the same view every time; a scene that sets its camera's height stands there
#[test]
fn a_scene_without_a_camera_opens_head_on_at_the_centre() {
    use crate::scene_document::SceneDocument;
    use crate::scene_resolve::{core_rigs, no_sets, resolve};
    use crate::space::portrait::Portraits;
    use crate::space::values::Table;
    let open = |doc: &SceneDocument, turn: f32, free: bool| -> (View, Vec3, f32) {
        let scene = resolve(doc, &Default::default(), true, &core_rigs, &no_sets).unwrap().scene;
        let g = crate::graph::Graph::build(&scene).unwrap();
        let mut s = crate::space::Space::new(&g, g.first(crate::graph::Kind::Space).unwrap(), &scene.being, std::path::Path::new("/nowhere"), Portraits::manual());
        s.camera.free = free;
        s.camera.drag(turn * 400.0, turn * 90.0);
        s.set_scene(&scene);
        s.set_view(1280.0, 720.0);
        if free {
            s.open_on_scene_camera();
        }
        let mut ctl = Table::new(&g);
        for k in 0..90 {
            s.update(1.0 / 60.0, k as f32 / 60.0, &mut ctl, &tempo::Now::default(), &|_, _| 0.0);
        }
        (View::of(&s.camera), s.stage_centre(), s.floor_point().y)
    };
    let new = SceneDocument::new("fresh", "Fresh", vec!["catpope".into()]);
    let mut cast = SceneDocument::new("duet", "Duet", vec!["catpope".into()]);
    cast.recast(&["character:catpope".into(), "item:incense_burner".into()]);
    for doc in [&new, &cast] {
        for free in [true, false] {
            let (a, centre, floor) = open(doc, 0.0, free);
            let (b, ..) = open(doc, 1.0, free);
            assert!(a.gap(&b) < 1e-4, "{}: the same view every time ({free})", doc.id);
            assert!(wrap_pi(a.yaw).abs() < 1e-5 && a.pitch.abs() < 1e-5, "{}: head-on and level: {a:?}", doc.id);
            assert!((a.target.x - centre.x).abs() < 1e-4 && (a.target.z - centre.z).abs() < 1e-4, "{}: on the centre line", doc.id);
            assert!((a.eye().y - floor - crate::scene_camera::DEFAULT_HEIGHT).abs() < 1e-3, "{}: an adult's eye", doc.id);
        }
    }
    let mut kid = cast.clone();
    kid.camera = Some(SceneCamera { height: crate::scene_camera::EYE_HEIGHTS[2].1, ..SceneCamera::default() });
    let (v, _, floor) = open(&kid, 0.0, true);
    assert!((v.eye().y - floor - 1.1).abs() < 1e-3, "a kid's eye");
}

/// Build opens a scene on its camera: the same view however the stage was looked at before, a shot exactly
#[test]
fn a_scene_opens_on_its_camera_the_same_however_the_stage_was_looked_at() {
    use crate::scene_document::SceneDocument;
    use crate::scene_resolve::{core_rigs, no_sets, resolve};
    use crate::space::portrait::Portraits;
    use crate::space::values::Table;
    let mut doc = SceneDocument::new("duet", "Duet", vec!["catpope".into()]);
    doc.recast(&["character:catpope".into(), "item:incense_burner".into()]);
    let open = |doc: &SceneDocument, turn: f32| -> View {
        let scene = resolve(doc, &Default::default(), true, &core_rigs, &no_sets).unwrap().scene;
        let g = crate::graph::Graph::build(&scene).unwrap();
        let mut s = crate::space::Space::new(&g, g.first(crate::graph::Kind::Space).unwrap(), &scene.being, std::path::Path::new("/nowhere"), Portraits::manual());
        s.camera.free = true;
        s.set_camera(Some("scene:duet"), scene.camera.clone());
        s.set_view(1280.0, 720.0);
        s.camera.drag(turn * 300.0, turn * 80.0);
        s.camera.pan(turn * 40.0, 0.0, 720.0);
        s.camera.zoom(turn * 3.0);
        s.open_on_scene_camera();
        let mut ctl = Table::new(&g);
        for k in 0..30 {
            s.update(1.0 / 60.0, k as f32 / 60.0, &mut ctl, &tempo::Now::default(), &|_, _| 0.0);
        }
        View::of(&s.camera)
    };
    let (a, b) = (open(&doc, 0.0), open(&doc, 1.0));
    {
        let mut d2 = doc.clone();
        d2.camera = Some(camera());
        let scene = resolve(&d2, &Default::default(), true, &core_rigs, &no_sets).unwrap().scene;
        let g = crate::graph::Graph::build(&scene).unwrap();
        let mut s = crate::space::Space::new(&g, g.first(crate::graph::Kind::Space).unwrap(), &scene.being, std::path::Path::new("/nowhere"), Portraits::manual());
        s.camera.free = true;
        s.set_camera(Some("scene:duet"), scene.camera.clone());
        s.set_view(1280.0, 720.0);
        s.open_on_scene_camera();
        s.look_through_shot("close");
        let mut ctl = Table::new(&g);
        for k in 0..120 {
            s.update(1.0 / 60.0, k as f32 / 60.0, &mut ctl, &tempo::Now::default(), &|_, _| 0.0);
        }
        let close = View::of_shot(&camera().shots[1]);
        assert!(View::of(&s.camera).gap(&close) < 1e-3, "glided into the shot: {:?} vs {close:?}", View::of(&s.camera));
        assert_eq!(s.viewing(), &Viewing::Shot("close".into()));
        s.look_free();
        for k in 0..120 {
            s.update(1.0 / 60.0, 2.0 + k as f32 / 60.0, &mut ctl, &tempo::Now::default(), &|_, _| 0.0);
        }
        assert!(View::of(&s.camera).gap(&View::of_shot(&camera().shots[0])) < 1e-3, "back to the free view it left (the opening shot): {:?} {:?}", View::of(&s.camera), s.camera);
        assert!(!s.camera.looking());
    }
    assert!(a.gap(&b) < 1e-4, "{a:?} vs {b:?}");
    doc.camera = Some(camera());
    let v = open(&doc, 1.0);
    let wide = View::of_shot(&camera().shots[0]);
    assert!(v.gap(&wide) < 1e-3, "on the opening shot: {v:?}");
}

/// Between a perspective and an orthographic shot the projection blends as a dolly zoom – a move and the stage's glide
/// alike: the picture never jumps (the orthographic end too), half way it is a long lens far back, the end is the shot
/// exactly; a hand mid-blend brings the eye in as the lens opens, the picture held
#[test]
fn perspective_and_orthographic_blend_without_a_jump() {
    let mut dop = Dop::new(None, camera());
    let (wide, iso) = (dop.view("wide").unwrap(), dop.view("iso").unwrap());
    let mut glide = spec(MoveKind::Glide);
    glide.ease = Ease::Linear;
    for (from, to, id) in [(wide, iso, "iso"), (iso, wide, "wide")] {
        let frames = run(&mut dop, glide.clone(), id, from);
        assert_eq!(*frames.last().unwrap(), to);
        assert!(picture_jump(&frames) < 0.05, "→ {id}: the picture jumps {}", picture_jump(&frames));
        let mid = frames[frames.len() / 2];
        assert!(!mid.lens.ortho() && mid.lens.focal > 55.0 && mid.distance > from.distance.max(to.distance), "half way a long lens far back: {mid:?}");
        let half = (view_height(&from) + view_height(&to)) * 0.5;
        assert!((view_height(&mid) / half - 1.0).abs() < 0.03, "the view's height glides: {} vs {half}", view_height(&mid));
    }
    let shot = |v: &View| crate::space::camera::Shot { target: v.target, yaw: v.yaw, pitch: v.pitch, distance: v.distance, fov_y: v.lens.fov_y(), ortho: v.lens.ortho().then_some(v.lens.size) };
    let mut o = Orbit::new(wide.target, wide.yaw, wide.pitch, wide.distance);
    wide.apply(&mut o);
    o.look_through(shot(&iso));
    let mut frames = vec![View::of(&o)];
    for _ in 0..80 {
        o.step(1.0 / 60.0);
        frames.push(View::of(&o));
    }
    assert!(View::of(&o).gap(&iso) < 1e-3 && o.ortho == Some(5.0) && !o.far_back, "arrived orthographic");
    assert!(picture_jump(&frames) < 0.05, "the glide into it: {}", picture_jump(&frames));
    o.go_free();
    let mut frames = vec![View::of(&o)];
    for _ in 0..25 {
        o.step(1.0 / 60.0);
        frames.push(View::of(&o));
    }
    assert!(o.far_back && o.ortho.is_none(), "half way back: perspective, far back");
    o.drag(0.0, 0.0);
    for _ in 0..240 {
        o.step(1.0 / 60.0);
        frames.push(View::of(&o));
    }
    assert!(picture_jump(&frames) < 0.05, "back and a hand on the way: {}", picture_jump(&frames));
    assert!(!o.far_back && (o.fov_y - wide.lens.fov_y()).abs() < 1e-3 && o.distance < 30.0, "the hand's lens again, the eye near: {o:?}");
}

/// Every camera shows its view: a pyramid from the eye to the frame at its aim (the frame on the view's corners), the
/// orthographic one a box as high as its view; a longer lens a narrower pyramid
#[test]
fn a_camera_shows_its_view_as_a_pyramid_or_a_box() {
    let dop = Dop::new(None, camera());
    let wide = dop.view("wide").unwrap();
    let aspect = 16.0 / 9.0;
    let f = wide.frustum(aspect);
    assert_eq!((f.lines().len(), f.near, f.aim, f.eye), (8, None, wide.target, wide.eye()));
    let mut o = Orbit::new(Vec3::ZERO, 0.0, 0.0, 1.0);
    wide.apply(&mut o);
    let vp = o.view_proj(aspect);
    for (c, want) in f.far.iter().zip([(-1.0, 1.0), (1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)]) {
        let q = vp.project_point3(*c);
        assert!((q.x - want.0).abs() < 1e-3 && (q.y - want.1).abs() < 1e-3, "a corner of the frame on the view's: {q} vs {want:?}");
    }
    let long = View { lens: Lens::focal(150.0), ..wide }.frustum(aspect);
    assert!(long.far[0].distance(long.far[1]) < f.far[0].distance(f.far[1]) * 0.25, "a long lens: narrow");
    let iso = dop.view("iso").unwrap().frustum(aspect);
    assert_eq!(iso.lines().len(), 12);
    let near = iso.near.expect("a box");
    assert!((iso.far[0].distance(iso.far[3]) - 5.0).abs() < 1e-4 && (near[0].distance(near[3]) - 5.0).abs() < 1e-4, "as high as its view");
}

/// a cast stage (catpope and the incense burner) on `cam`, a deck's (not an editor's free view)
fn cast_stage(cam: SceneCamera) -> crate::space::Space {
    use crate::scene_document::SceneDocument;
    use crate::scene_resolve::{core_rigs, no_sets, resolve};
    use crate::space::portrait::Portraits;
    let mut doc = SceneDocument::new("duet", "Duet", vec!["catpope".into()]);
    doc.recast(&["character:catpope".into(), "item:incense_burner".into()]);
    doc.camera = Some(cam);
    let scene = resolve(&doc, &Default::default(), true, &core_rigs, &no_sets).unwrap().scene;
    let g = crate::graph::Graph::build(&scene).unwrap();
    let mut s = crate::space::Space::new(&g, g.first(crate::graph::Kind::Space).unwrap(), &scene.being, std::path::Path::new("/nowhere"), Portraits::manual());
    s.set_view(1280.0, 720.0);
    s.set_camera(Some("scene:duet"), scene.camera.clone());
    s
}

/// A shot aims at an address – a filler, a holder – wherever it stands, with its own angles; what cannot be aimed at is
/// refused by the core; the editor's view gets every camera's frustum, the one looked through lit
#[test]
fn a_shot_aims_at_an_address_and_every_camera_shows_itself() {
    let mut cam = camera();
    cam.shots[1].aim = Some("item:incense_burner".into());
    let mut s = cast_stage(cam.clone());
    let item = s.cast.iter().find(|c| c.who.place() == "item:incense_burner").expect("the item stands");
    let (at, holder) = (s.member_sphere(item).0, item.holder.clone());
    let v = s.shot_view("close").unwrap();
    assert!(v.target.distance(at) < 1e-5, "aimed at the item");
    assert_eq!((v.yaw, v.pitch, v.distance), (cam.shots[1].yaw.to_radians(), cam.shots[1].pitch.to_radians(), cam.shots[1].distance));
    assert_eq!(s.aim_point(&holder), Some(at), "by its holder too");
    assert!(s.aim_point("character:catpope").is_some() && s.aim_point("spot:nowhere").is_none());
    let holders = [holder.as_str()];
    for (aim, ok) in [("item:incense_burner", true), (holder.as_str(), true), ("spot:altar", true), ("nobody", false), ("scene:duet", false), ("character:catpope.ghost", false)] {
        let mut c = cam.clone();
        c.shots[1].aim = Some(aim.into());
        let mut out = Vec::new();
        c.problems(&holders, &mut out);
        assert_eq!(out.is_empty(), ok, "{aim}: {out:?}");
    }
    s.look_through_shot("close");
    let frusta = s.camera_frusta();
    assert_eq!(frusta.iter().map(|(id, _, lit)| (id.clone().unwrap_or_default(), *lit)).collect::<Vec<_>>(), [("wide".into(), false), ("close".into(), true), ("iso".into(), false)]);
    assert!(frusta[1].1.aim.distance(at) < 1e-5);
    let bare = cast_stage(SceneCamera::default());
    let f = bare.camera_frusta();
    assert!(f.len() == 1 && f[0].0.is_none() && f[0].2, "no shots: the primary camera, looked through");
    let (eye, ahead) = (s.camera.eye(), s.camera.target);
    assert!(s.segment_to_pixels(eye + (eye - ahead), ahead).is_some(), "a segment half behind the eye is cut where it passes it");
    assert!(s.segment_to_pixels(eye + (eye - ahead), eye + (eye - ahead) * 2.0).is_none());
}

/// Upright (a phone in portrait) the free view widens; a shot's own lens wins and is never widened
#[test]
fn a_shots_lens_wins_over_the_portrait_widening() {
    let mut s = cast_stage(camera());
    s.look_through_shot("close");
    let close = camera().shots[1].lens.fov_y();
    s.fit_screen(390.0, 844.0);
    assert_eq!(s.camera.through.map(|t| t.fov_y), Some(close), "the shot's lens");
    let wide = crate::space::camera::screen_fov(390.0 / 844.0);
    assert!(wide > crate::space::camera::FOV_Y && s.camera.free_fov() == wide);
    let mut free = cast_stage(SceneCamera::default());
    free.fit_screen(390.0, 844.0);
    assert_eq!(free.camera.fov_y, wide, "a scene without a lens of its own: the free view's, widened");
    free.fit_screen(1280.0, 720.0);
    assert_eq!(free.camera.fov_y, crate::space::camera::FOV_Y);
}

/// A camera word for a scene that is not on the stage yet (a show row starts the scene and picks its shot) waits for it
#[test]
fn a_camera_word_before_its_scene_waits_for_it() {
    let mut s = cast_stage(camera());
    assert!(s.camera_word("scene:next", "shot:iso"), "a camera word, kept");
    assert_eq!(s.viewing(), &Viewing::Primary);
    s.set_camera(Some("scene:next"), Some(camera()));
    assert_eq!(s.viewing(), &Viewing::Shot("iso".into()), "taken as the scene arrived");
    s.set_camera(Some("scene:later"), Some(camera()));
    assert_eq!(s.viewing(), &Viewing::Primary, "once only");
}

/// a linear glide of `len` from the bar line at beat 0 to "close", repeating as `repeat`
fn repeating(repeat: Repeat, len: Length) -> Dop {
    let mut dop = Dop::new(Some("scene:nave".into()), camera());
    let s = MoveSpec { length: len, ease: Ease::Linear, start: Start::Bar, repeat, ..MoveSpec::of(MoveKind::Glide) };
    dop.play(vec![dop.cue(s, "close").unwrap()]);
    assert!(dop.step(&at_beat(0.0), start(), &no_one).is_some(), "starts on the bar line at 0");
    dop
}

fn dop_view(id: &str) -> View {
    Dop::new(None, camera()).view(id).unwrap()
}

#[test]
fn once_loop_and_ping_pong_on_the_beat_grid() {
    let close = dop_view("close");
    let at = |d: &mut Dop, beat: f64| d.step(&at_beat(beat), start(), &no_one).expect("a view");
    // once: there after its bar, and stays
    let mut once = repeating(Repeat::Once, Length::Bars(1.0));
    assert!(at(&mut once, 4.0).gap(&close) < 1e-3);
    assert!(!once.director.active(), "once: done");

    // loop: from its start again on every bar line
    let mut lp = repeating(Repeat::Loop, Length::Bars(1.0));
    let mid = at(&mut lp, 2.0);
    for bar in 1..6 {
        let b = f64::from(bar) * 4.0;
        assert!(at(&mut lp, b + 0.001).gap(&start()) < 0.05, "bar {bar}: back at its start");
        assert!(at(&mut lp, b + 2.0).gap(&mid) < 1e-3, "bar {bar}: the same way again");
        assert!(at(&mut lp, b + 3.999).gap(&close) < 0.05, "bar {bar}: there at its end");
        assert!(lp.director.active(), "a loop runs on");
    }

    // ping-pong: there in the first bar, back in the second, there again in the third
    let mut pp = repeating(Repeat::PingPong, Length::Bars(1.0));
    assert!(at(&mut pp, 3.999).gap(&close) < 0.05);
    assert!(at(&mut pp, 6.0).gap(&mid) < 1e-3, "halfway back");
    assert!(at(&mut pp, 7.999).gap(&start()) < 0.05, "back at its start at bar 2's end");
    assert!(at(&mut pp, 11.999).gap(&close) < 0.05, "there again");

    // 3 beats in 4/4: each round waits for the bar line, holding where it arrived
    let mut odd = repeating(Repeat::Loop, Length::Beats(3.0));
    assert!(at(&mut odd, 3.5).gap(&close) < 1e-3, "holds the shot until the bar line");
    assert!(at(&mut odd, 4.001).gap(&start()) < 0.05, "the repeat lands on the bar line");
    assert!(at(&mut odd, 7.5).gap(&close) < 1e-3);
    assert!(at(&mut odd, 8.001).gap(&start()) < 0.05);

    // seconds: no bar line, the rounds follow each other at once
    let mut secs = Dop::new(None, camera());
    let s = MoveSpec { length: Length::Seconds(1.5), ease: Ease::Linear, repeat: Repeat::Loop, ..MoveSpec::of(MoveKind::Glide) };
    secs.play(vec![secs.cue(s, "close").unwrap()]);
    let idle_at = |d: &mut Dop, t: f64| d.step(&idle(t), start(), &no_one).expect("a view");
    let first = idle_at(&mut secs, 0.0);
    assert!(idle_at(&mut secs, 1.5001).gap(&first) < 0.05, "round two right after round one");

    // another move ends a loop; a hand ends a ping-pong
    lp.play(vec![lp.cut("wide").unwrap()]);
    assert!(at(&mut lp, 30.0).gap(&dop_view("wide")) < 1e-3);
    assert!(!lp.director.active());
    pp.director.interrupt();
    assert!(!pp.director.active(), "a hand stops a ping-pong");
}

#[test]
fn a_fly_through_reaches_each_timed_point_on_its_bar() {
    // the start view's eye on the track's start, the shot's eye on its end: the eye runs exactly on the track
    let pts = [[-4.0, 1.6, 3.0], [0.0, 1.2, 0.5], [2.0, 1.4, 1.0], [4.0, 2.0, 2.0]];
    let aim = Vec3::new(0.0, 1.0, 0.0);
    let mut cam = camera();
    let mut track = Track { points: pts.iter().map(|p| TrackPoint::at(*p)).collect() };
    track.points[1].time = Some(Length::Bars(1.0));
    track.points[2].time = Some(Length::Bars(3.0));
    track.points[2].ease = Some(Ease::InOut);
    track.points[3].time = Some(Length::Bars(4.0));
    cam.track = Some(track);
    cam.shots.push(View::from_eye(Vec3::from_array(pts[3]), aim, Lens::default()).shot("end"));
    let mut dop = Dop::new(None, cam);
    let from = View::from_eye(Vec3::from_array(pts[0]), aim, Lens::default());
    let mut s = MoveSpec::of(MoveKind::Fly);
    (s.length, s.start) = (Length::Bars(16.0), Start::Bar);
    dop.play(vec![dop.cue(s, "end").unwrap()]);
    assert_eq!(dop.step(&at_beat(2.5), from, &no_one), None, "waits for the bar line");
    for (bar, p) in [(1.0, 0), (2.0, 1), (4.0, 2), (5.0, 3)] {
        let v = dop.step(&at_beat(bar * 4.0), from, &no_one).unwrap();
        assert!(v.eye().distance(Vec3::from_array(pts[p])) < 0.05, "point {} on bar {bar}: {:?}", p + 1, v.eye());
    }
    assert!(!dop.director.active(), "the end's time is its length, not the move's 16 bars");
}

#[test]
fn a_camera_word_on_a_beat_waits_for_it() {
    let mut dop = Dop::new(Some("scene:nave".into()), camera());
    let wide = dop.view("wide").unwrap();
    assert_eq!(dop.step(&at_beat(5.0), start(), &no_one), None);
    assert!(matches!(dop.word_on("scene:nave", "shot:wide", Some(8.0)), Some(Ok(_))));
    assert_eq!(dop.step(&at_beat(7.9), start(), &no_one), None, "nothing before its bar");
    assert_eq!(dop.step(&at_beat(8.0), start(), &no_one), Some(wide), "the cut on its beat");
    // a move playing goes on until the beat, then the cut replaces it
    let mut dop = repeating(Repeat::Loop, Length::Bars(1.0));
    dop.word_on("scene:nave", "shot:wide", Some(12.0)).unwrap().unwrap();
    assert_ne!(dop.step(&at_beat(10.0), start(), &no_one), Some(wide), "the loop still runs");
    assert_eq!(dop.step(&at_beat(12.0), start(), &no_one), Some(wide));
}

#[test]
fn a_rack_focus_keeps_the_frame_and_shifts_the_focus() {
    let mut cam = camera();
    cam.shots[1].focus_on = Some("pope".into());
    let mut dop = Dop::new(None, cam);
    let from = View { lens: Lens { focus: Some(2.0), ..start().lens }, ..start() };
    let pope = Vec3::new(0.4, 1.1, 0.3);
    dop.play(vec![dop.cue(spec(MoveKind::Rack), "close").unwrap()]);
    let mut frames = Vec::new();
    let mut now = from;
    for f in 0..=200 {
        let Some(v) = dop.step(&idle(f64::from(f) / 60.0), now, &|h: &str| (h == "pope").then_some(pope)) else { break };
        frames.push(v);
        now = v;
        if !dop.director.active() {
            break;
        }
    }
    let end = *frames.last().unwrap();
    for v in &frames {
        assert_eq!((v.target, v.yaw, v.pitch, v.distance, v.lens.focal), (from.target, from.yaw, from.pitch, from.distance, from.lens.focal), "the frame stays");
    }
    let want = from.eye().distance(pope);
    assert!((end.focus() - want).abs() < 1e-4, "on the subject, measured from this eye: {} vs {want}", end.focus());
    let sign = (want - 2.0).signum();
    assert!(frames.windows(2).all(|w| (w[1].focus() - w[0].focus()) * sign >= -1e-5), "the focus travels one way");
    assert_eq!((&dop.viewing, dop.focus_shot.as_deref()), (&Viewing::Free, Some("close")), "the frame is not the shot's, its focus holds");
    assert_eq!(dop.focus_holder(), None, "free: the focus is the view's");
}

#[test]
fn a_shot_s_subject_keeps_the_focus_as_it_moves() {
    let mut cam = camera();
    cam.shots[1].focus_on = Some("pope".into());
    let mut dop = Dop::new(None, cam);
    let mut s = spec(MoveKind::Follow);
    s.length = Length::Seconds(0.5);
    dop.play(vec![dop.cue(s, "close").unwrap()]);
    let walk = |secs: f64| Vec3::new(secs as f32 * 0.5, 1.0, 0.0);
    let mut v = start();
    for f in 0..120 {
        let secs = f64::from(f) / 60.0;
        v = dop.step(&idle(secs), v, &move |h: &str| (h == "pope").then(|| walk(secs))).unwrap();
        if secs > 0.6 {
            assert!((v.focus() - v.eye().distance(walk(secs))).abs() < 1e-4, "the focus follows: {}", v.focus());
        }
    }
    assert_eq!(dop.focus_holder().map(|s| s.id.as_str()), Some("close"));
}

#[test]
fn a_move_s_focus_is_the_lens_s_else_the_aim_s() {
    let mut cam = camera();
    cam.shots[0].lens.focus = Some(9.0);
    let mut dop = Dop::new(None, cam);
    let wide = run(&mut dop, spec(MoveKind::Glide), "wide", start());
    assert!((wide.last().unwrap().focus() - 9.0).abs() < 1e-4);
    let close = run(&mut dop, spec(MoveKind::Glide), "close", *wide.last().unwrap());
    let end = close.last().unwrap();
    assert_eq!(end.lens.focus, None, "none: at the aim");
    assert!(close.iter().all(|v| v.focus() <= 9.0 + 1e-4 && v.focus() >= end.distance - 1e-4), "it travels between them");
    assert!(close.windows(2).all(|w| (w[1].focus() - w[0].focus()).abs() < 0.5), "no jump");
}

/// LIVE-CAMERAS.md §6: a phone holding a shot of this deck's scene steers the picture from that shot's view; another
/// scene's camera is not its own; a move played meanwhile plays over the phone, then the phone goes on from where it ended
/// ([delegated]); free again, the deck's own camera
#[test]
fn a_held_camera_steers_the_deck_and_a_move_plays_over_it() {
    use crate::live_cameras::Held;
    use crate::scene_document::SceneDocument;
    use crate::scene_resolve::{core_rigs, no_sets, resolve};
    use crate::space::portrait::Portraits;
    use crate::space::values::Table;
    use protocol::HandPose;
    let mut cam = camera();
    cam.moves.push(MoveDef { id: "push".into(), to: "wide".into(), spec: MoveSpec { length: Length::Seconds(0.5), ..MoveSpec::of(MoveKind::Glide) }, then: None });
    let mut doc = SceneDocument::new("duet", "Duet", vec!["catpope".into()]);
    doc.recast(&["character:catpope".into()]);
    doc.camera = Some(cam);
    let scene = resolve(&doc, &Default::default(), true, &core_rigs, &no_sets).unwrap().scene;
    let g = crate::graph::Graph::build(&scene).unwrap();
    let mut s = crate::space::Space::new(&g, g.first(crate::graph::Kind::Space).unwrap(), &scene.being, std::path::Path::new("/nowhere"), Portraits::manual());
    s.set_view(1280.0, 720.0);
    s.set_camera(Some("scene:duet"), scene.camera.clone());
    let mut ctl = Table::new(&g);
    let mut t = 0.0;
    let mut run = |s: &mut crate::space::Space, frames: usize| {
        for _ in 0..frames {
            t += 1.0 / 60.0;
            s.update(1.0 / 60.0, t, &mut ctl, &crate::tempo::Now::default(), &|_, _| 0.0);
        }
    };
    run(&mut s, 5);
    let pose = |t: f64, p: [f64; 3]| HandPose { t, p: p.to_vec(), q: vec![0.0, 0.0, 0.0, 1.0] };
    s.held(Held::Pose { camera: "scene:crypt".into(), pose: pose(1.0, [0.0, 0.0, 0.0]) });
    assert_eq!(s.held_camera(), None, "another scene's camera");
    s.held(Held::Holder { camera: "scene:duet > shot:close".into(), holder: Some("c4".into()) });
    s.held(Held::Pose { camera: "scene:duet > shot:close".into(), pose: pose(1.0, [0.0, 0.0, 0.0]) });
    run(&mut s, 30);
    let close = View::of_shot(&camera().shots[1]);
    assert!(s.camera.eye().distance(close.eye()) < 1e-3, "from the shot's view: {} vs {}", s.camera.eye(), close.eye());
    s.held(Held::Pose { camera: "scene:duet".into(), pose: pose(2.0, [5.0, 0.0, 0.0]) });
    run(&mut s, 30);
    assert!(s.camera.eye().distance(close.eye()) < 1e-3, "one held camera at a time");
    s.held(Held::Pose { camera: "scene:duet > shot:close".into(), pose: pose(2.0, [1.0, 0.0, 0.0]) });
    run(&mut s, 30);
    assert!((s.camera.eye().distance(close.eye()) - 1.0).abs() < 1e-3, "a metre stepped is a metre moved: {}", s.camera.eye());
    assert!(s.camera_word("scene:duet", "move:push"));
    run(&mut s, 60);
    let wide = View::of_shot(&camera().shots[0]);
    assert!(s.camera.eye().distance(wide.eye()) < 1e-2, "the move played over the phone to its end: {} vs {}", s.camera.eye(), wide.eye());
    s.held(Held::Pose { camera: "scene:duet > shot:close".into(), pose: pose(3.0, [1.0, 0.0, 0.0]) });
    run(&mut s, 30);
    assert!(s.camera.eye().distance(wide.eye()) < 1e-2, "handed back: the phone goes on from there, nothing jumps: {}", s.camera.eye());
    s.held(Held::Holder { camera: "scene:duet > shot:close".into(), holder: None });
    assert!(s.held.is_none() && s.held_camera().is_none(), "free: the deck's own camera");
}

/// a phone's take (take.rs): it starts where the camera is, follows its keys on their times – in beats on the clock, so a
/// faster tempo plays it faster – and ends exactly on its own end shot
#[test]
fn a_recorded_take_plays_its_keys_on_their_times_and_ends_on_its_shot() {
    use crate::scene_camera::{Take, TakeKey};
    let key = |x: f32, yaw: f32, t: f32| TakeKey { at: [x, 1.6, 4.0], yaw, pitch: 0.0, time: Length::Beats(t) };
    let take = Take { keys: vec![key(-2.0, 0.0, 0.0), key(0.0, 10.0, 4.0), key(2.0, 30.0, 8.0)] };
    let mut cam = camera();
    let (s30, c30) = 30f32.to_radians().sin_cos();
    cam.shots.push(shot("take1_end", [2.0 - 4.0 * s30, 1.6, 4.0 - 4.0 * c30], 30.0, 0.0, 4.0, Lens::focal(50.0)));
    let spec = MoveSpec { length: Length::Beats(8.0), take: Some(take), ..MoveSpec::of(MoveKind::Take) };
    let mut problems = Vec::new();
    spec.problems("move take1", &mut problems);
    assert!(problems.is_empty(), "{problems:?}");
    let mut dop = Dop::new(None, cam);
    let frames = run(&mut dop, spec.clone(), "take1_end", start());
    let end = dop.view("take1_end").unwrap();
    assert_eq!(*frames.last().unwrap(), end, "exactly its end shot");
    assert!((frames.len() as f32 - 4.0 * 60.0).abs() <= 2.0, "8 beats at 120 bpm: 4 s, {} frames", frames.len());
    assert!(frames[0].gap(&start()) < 0.05, "starts where the camera was");
    let mid = frames[120];
    assert!(mid.eye().distance(Vec3::new(0.0, 1.6, 4.0)) < 0.05 && (mid.yaw.to_degrees() - 10.0).abs() < 1.0, "on its middle key half way: {:?}", mid.eye());
    let step = frames.windows(2).map(|w| w[0].gap(&w[1])).fold(0.0, f32::max);
    assert!(step < 0.8, "nothing jumps: the most a frame moves is {step}");
    // the clock twice as fast: half the time
    dop.play(vec![dop.cue(spec, "take1_end").unwrap()]);
    let fast = |f: u32| Clock { secs: f64::from(f) / 60.0, beat: f64::from(f) / 15.0, bpm: 240.0, per_bar: 4, running: true };
    let mut now = start();
    let mut n = 0;
    while let Some(v) = dop.step(&fast(n), now, &no_one) {
        now = v;
        n += 1;
        if !dop.director.active() || n > 1000 {
            break;
        }
    }
    assert!((n as f32 - 120.0).abs() <= 2.0, "8 beats at 240 bpm: {n} frames");
}
