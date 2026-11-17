use super::*;
use crate::scene_camera::{Ease, Length, MoveDef, Track, TrackPoint, Turn};

fn shot(id: &str, target: [f32; 3], yaw: f32, pitch: f32, distance: f32, lens: Lens) -> ShotDef {
    ShotDef { id: id.into(), label: Default::default(), target, yaw, pitch, distance, lens }
}

fn camera() -> SceneCamera {
    let track = Track {
        points: vec![
            TrackPoint { at: [-4.0, 1.6, 3.0], handle: None },
            TrackPoint { at: [0.0, 1.2, 0.5], handle: None },
            TrackPoint { at: [4.0, 2.0, 2.0], handle: Some([1.0, 0.0, -1.0]) },
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
    for kind in MoveKind::ALL {
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
            // no jump on the way: every frame near the one before (a whip is fast, a fly-through long)
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
    assert!(a.gap(&b) < 1e-4, "{a:?} vs {b:?}");
    doc.camera = Some(camera());
    let v = open(&doc, 1.0);
    let wide = View::of_shot(&camera().shots[0]);
    assert!(v.gap(&wide) < 1e-3, "on the opening shot: {v:?}");
}
