//! control.rs's tests: the block-rate control half against the patcher model, the API that was deck-core's `BatchGraph`
//! (#319: its tests, on the engine), the face over the engine (commands, the snapshot ring by heard frame).
use super::*;
use crate::{Engine, QUANTUM};
use deck_space::scene::Scene;

fn graph(text: &str) -> Graph {
    Graph::build(&Scene::parse(text).unwrap()).unwrap()
}

fn now() -> tempo::Now {
    tempo::Now::default()
}

/// the control half the engine built for `g`, and its handles (the audio Params)
fn engine(g: &Graph) -> (Engine, crate::Handles) {
    Engine::build(g, 48_000.0, 1.0).unwrap()
}

#[test]
fn links_map_lfos_into_video_and_audio_inputs() {
    let g = graph(
        r#"
[[node]]
id = "l"
kind = "lfo"
rate = 0.25
shape = "square"
[[node]]
id = "p"
kind = "test_pattern"
zoom = 3
[[node]]
id = "o"
kind = "osc"
[[link]]
from = "l"
to = "p.speed"
scale = 2
offset = 1
[[link]]
from = "l"
to = "o.freq"
scale = 100
"#,
    );
    let (mut e, h) = engine(&g);
    let freq = h.params[0].2.clone();
    let c = e.control_mut();
    c.step(0.1, &now());
    assert_eq!(c.input(1, "speed", 0.0), 3.0); // square high = 1 → 1 * 2 + 1
    assert_eq!(c.input(1, "zoom", 1.0), 3.0); // unlinked: the node's param
    assert_eq!(c.input(1, "hue", 0.5), 0.5); // neither: the default
    assert_eq!(freq.get(), 100.0, "the step wrote the audio thread's Param");
    c.step(2.0, &now()); // phase 0.025 + 0.5 → the low half
    assert_eq!(c.input(1, "speed", 0.0), 1.0);
}

#[test]
fn set_base_writes_unlinked_inputs_like_a_cc() {
    let g = graph(
        r#"
[[node]]
id = "l"
kind = "lfo"
rate = 0
shape = "square"
[[node]]
id = "p"
kind = "test_pattern"
[[node]]
id = "f"
kind = "fm"
[[link]]
from = "l"
to = "p.speed"
"#,
    );
    let (mut e, h) = engine(&g);
    let index = h.knobs.iter().find(|k| (k.0, k.1) == (2, 3)).expect("fm.index is a knob").2.clone();
    let c = e.control_mut();
    assert!(c.set_base(1, 2, 0.7), "p.hue: unlinked");
    assert_eq!(c.input(1, "hue", 0.0), 0.7);
    // #188: a linked input's knob turns its base; the link flows on top (the square LFO at 1 here once stepped)
    assert_eq!(c.base(1, 0), Some(0.0), "the link's offset");
    assert!(c.set_base(1, 0, 3.0), "p.speed: its base");
    assert_eq!((c.base(1, 0), c.base(1, 2)), (Some(3.0), None));
    assert!(c.set_base(2, 3, 4.5));
    c.step(0.0, &now());
    assert_eq!(index.get(), 4.5, "the audio thread's Param, at the next step");
    // the LFO's depth through its batch column: a square at phase 0 = depth
    assert!(c.set_base(0, 1, 0.25));
    c.step(0.01, &now());
    assert_eq!(c.input(1, "speed", 0.0), 3.25, "the depth 0.25 on top of the base 3");
    assert!(c.set_base(0, 0, 50.0), "the rate");
    c.step(0.011, &now()); // phase 0.55 → the low half
    assert_eq!(c.input(1, "speed", 1.0), 3.0);
    assert!(!c.set_base(9, 0, 1.0) && !c.set_base(1, 40, 1.0), "no such input");
}

/// #188: a knob on a linked audio input turns its base – the audio thread's Param gets the link on top of it
#[test]
fn a_linked_audio_input_turns_as_its_base() {
    let g = graph(
        r#"
[[node]]
id = "l"
kind = "lfo"
rate = 0
shape = "square"
[[node]]
id = "o"
kind = "osc"
[[link]]
from = "l"
to = "o.freq"
scale = 100
offset = 40
"#,
    );
    let (mut e, h) = engine(&g);
    let freq = h.params[0].2.clone();
    let c = e.control_mut();
    c.step(0.01, &now());
    assert_eq!(freq.get(), 140.0, "the square high: 1 · 100 + 40");
    let port = Kind::Osc.input("freq").unwrap();
    assert!(c.set_base(1, port, 300.0));
    c.step(0.01, &now());
    assert_eq!((freq.get(), c.input(1, "freq", 0.0), c.base(1, port)), (400.0, 400.0, Some(300.0)));
}

#[test]
fn an_lfo_changes_its_shape_and_keeps_its_phase() {
    let g = graph("[[node]]\nid=\"a\"\nkind=\"lfo\"\nrate=0.1\n[[node]]\nid=\"b\"\nkind=\"lfo\"\nrate=0.1\nshape=\"saw\"\n");
    let mut c = Control::detached(&g);
    assert_eq!((c.lfo_shape(0), c.lfo_shape(1)), (Some(0), Some(1)));
    assert_eq!(c.shape(), (1, 2));
    c.step(1.0, &now()); // phase 0.1
    assert!(c.set_lfo_shape(0, 2));
    assert_eq!(c.shape(), (1, 2), "a square batch and a saw batch");
    c.step(1.0, &now()); // phase 0.2: a square's high half
    assert_eq!(c.outputs(0)[0], 1.0);
    assert!(c.set_lfo_shape(0, 1));
    assert_eq!(c.shape(), (1, 1), "both saws: one batch");
    c.step(1.0, &now()); // phase 0.3 → saw −0.4 → 0.3
    assert!((c.outputs(0)[0] - c.outputs(1)[0]).abs() < 1e-6, "same phase, same rate, same shape");
    assert!(!c.set_lfo_shape(5, 0) && c.lfo_shape(5).is_none());
}

/// A shape change moves an LFO out of the middle of its batch: the one that takes its place keeps its own phase,
/// columns and output (the index bookkeeping of the swap)
#[test]
fn moving_an_lfo_keeps_its_neighbours() {
    let mut s = String::new();
    for i in 0..4 {
        s += &format!("[[node]]\nid=\"l{i}\"\nkind=\"lfo\"\nrate={}\ndepth={}\n", 0.1 + i as f32 * 0.07, 1.0 + i as f32);
    }
    let g = graph(&s);
    let (mut a, mut b) = (Control::detached(&g), Control::detached(&g));
    for f in 0..50 {
        if f == 10 {
            assert!(a.set_lfo_shape(1, 2) && b.set_lfo_shape(1, 2));
            assert!(a.set_lfo_shape(1, 0) && a.set_lfo_shape(0, 1) && a.set_lfo_shape(0, 0), "back and forth");
            assert!(a.set_lfo_shape(1, 2));
            assert!(a.set_base(3, 1, 9.0) && b.set_base(3, 1, 9.0), "the moved-in-place node's column follows it");
        }
        a.step(0.05, &now());
        b.step(0.05, &now());
        for n in 0..4 {
            assert_eq!(a.outputs(n), b.outputs(n), "frame {f} node {n}");
        }
    }
    assert_eq!(a.lfo_shape(1), Some(2));
}

#[test]
fn the_tempo_node_outputs_the_clock() {
    let g = graph("[[node]]\nid=\"t\"\nkind=\"tempo\"\n[[node]]\nid=\"p\"\nkind=\"test_pattern\"\n[[link]]\nfrom=\"t.pulse\"\nto=\"p.pulse\"\n");
    let mut c = Controls::new(&g, None);
    let now = tempo::Now { bpm: 96.0, pos: tempo::position(9.0, 4), per_bar: 4, running: true };
    c.update(0.016, 0, &now);
    assert_eq!(c.outputs(0), [96.0, 9.0, 2.0, 0.0, 0.25, 1.0]);
    assert_eq!(c.input(1, "pulse", 0.0), 1.0);
    c.update(0.016, 0, &tempo::Now { pos: tempo::position(9.5, 4), ..now });
    assert!(c.input(1, "pulse", 0.0) < 0.02);
}

/// Inside the engine the tempo node reads the clock at the block's first frame (sample time, plan §2.8) – no frame
/// thread passes it in
#[test]
fn the_engine_steps_the_clock_at_the_block() {
    let g = graph("[[node]]\nid=\"t\"\nkind=\"tempo\"\nbpm=120\n[[node]]\nid=\"o\"\nkind=\"osc\"\n[[node]]\nid=\"out\"\nkind=\"audio_out\"\n[[link]]\nfrom=\"o\"\nto=\"out.in\"\n");
    let (mut e, _) = engine(&g);
    let mut buf = vec![0.0f32; 2 * QUANTUM];
    for _ in 0..375 {
        e.render(&mut buf, 2, |x| x); // 375 quanta = 1 s; the last stepped at frame 374 · 128
    }
    let [bpm, beat, ..] = e.control().outputs(0) else { panic!() };
    assert_eq!(*bpm, 120.0);
    assert!((*beat as f64 - 374.0 * 128.0 / 48_000.0 * 2.0).abs() < 1e-4, "beat {beat}");
}

/// The control domain as it ran before #183 (the patcher model: per step, per node in topological order, each input
/// found by searching the links, the shape read from the params) – the reference the batches must match. With the
/// helpers of `BatchGraph` (#319).
struct Patcher {
    out: Vec<Vec<f32>>,
    state: Vec<f32>,
}

impl Patcher {
    fn new(g: &Graph) -> Patcher {
        Patcher { out: g.nodes.iter().map(|n| vec![0.0; n.kind.outputs().len()]).collect(), state: vec![0.0; g.nodes.len()] }
    }

    fn input(&self, g: &Graph, node: usize, port: &str, default: f32) -> f32 {
        let n = &g.nodes[node];
        let Some(k) = n.kind.input(port) else { return default };
        match g.link_into(node, k) {
            Some(l) if l.ty == PortType::Control => self.out[l.from.0][l.from.1] * l.scale + l.offset,
            _ => n.f32(port, default),
        }
    }

    fn update(&mut self, g: &Graph, dt: f32, tempo: &tempo::Now) {
        if let Some(i) = g.first(Kind::Tempo) {
            let p = &tempo.pos;
            let pulse = if tempo.running { (1.0 - p.phase).powi(6) } else { 0.0 };
            let v = [tempo.bpm, p.beat, p.bar as f64, p.phase, p.bar_phase, pulse];
            for (o, x) in self.out[i].iter_mut().zip(v) {
                *o = x as f32;
            }
        }
        for &i in &g.order {
            let d = |port: &str| default_for(g.nodes[i].kind, port);
            match g.nodes[i].kind {
                Kind::Lfo => {
                    let rate = self.input(g, i, "rate", d("rate"));
                    let depth = self.input(g, i, "depth", d("depth"));
                    self.state[i] = (self.state[i] + rate * dt).fract();
                    let wave = match g.nodes[i].str("shape").unwrap_or("sine") {
                        "saw" => self.state[i] * 2.0 - 1.0,
                        "square" => {
                            if self.state[i] < 0.5 {
                                1.0
                            } else {
                                -1.0
                            }
                        }
                        _ => (self.state[i] * std::f32::consts::TAU).sin(),
                    };
                    self.out[i][0] = (0.5 + 0.5 * wave) * depth;
                }
                Kind::Smooth => {
                    let (x, t) = (self.input(g, i, "in", d("in")), self.input(g, i, "time", d("time")));
                    self.state[i] += (x - self.state[i]) * if t <= 0.0 { 1.0 } else { 1.0 - (-dt / t).exp() };
                    self.out[i][0] = self.state[i];
                }
                Kind::Mul => self.out[i][0] = self.input(g, i, "a", d("a")) * self.input(g, i, "b", d("b")),
                Kind::Add => self.out[i][0] = self.input(g, i, "a", d("a")) + self.input(g, i, "b", d("b")),
                _ => {}
            }
        }
    }
}

/// The same scene through the patcher and the batches: every control output and every video input agree.
#[test]
fn the_batches_match_the_patcher() {
    // three levels of LFOs (rate and depth fed from the level below, the tempo's phase into one), all shapes,
    // negative rates, the helpers on top, a test pattern reading them
    let mut s = String::from("[[node]]\nid=\"t\"\nkind=\"tempo\"\n[[node]]\nid=\"p\"\nkind=\"test_pattern\"\n");
    for l in 0..3 {
        for w in 0..7 {
            let shape = ["sine", "saw", "square"][(l + w) % 3];
            s += &format!("[[node]]\nid=\"l{l}_{w}\"\nkind=\"lfo\"\nrate={}\nshape=\"{shape}\"\n", 0.3 * w as f32 - 0.8);
            if l > 0 {
                s += &format!("[[link]]\nfrom=\"l{}_{w}\"\nto=\"l{l}_{w}.rate\"\nscale=1.7\noffset=-0.4\n", l - 1);
                s += &format!("[[link]]\nfrom=\"l{}_{}\"\nto=\"l{l}_{w}.depth\"\n", l - 1, (w + 1) % 7);
            }
        }
    }
    s += "[[node]]\nid=\"m\"\nkind=\"mul\"\nb=3\n[[node]]\nid=\"a\"\nkind=\"add\"\n[[node]]\nid=\"s\"\nkind=\"smooth\"\ntime=0.3\n";
    s += "[[link]]\nfrom=\"l2_5\"\nto=\"m.a\"\n[[link]]\nfrom=\"m\"\nto=\"a.a\"\n[[link]]\nfrom=\"l0_2\"\nto=\"a.b\"\nscale=-1\n[[link]]\nfrom=\"a\"\nto=\"s.in\"\n";
    s += "[[link]]\nfrom=\"t.phase\"\nto=\"l0_3.depth\"\n[[link]]\nfrom=\"l2_4\"\nto=\"p.hue\"\nscale=2\n[[link]]\nfrom=\"l1_1\"\nto=\"p.zoom\"\n[[link]]\nfrom=\"s\"\nto=\"p.speed\"\n";
    let g = graph(&s);
    let mut b = Controls::new(&g, None);
    let Half::Own(own) = &b.half else { panic!("no engine: its own half") };
    assert_eq!(own.shape(), (6, 12), "3 LFO levels × 3 shapes, then mul, add, smooth one level each");
    let mut p = Patcher::new(&g);
    for f in 0..600 {
        let now = tempo::Now { bpm: 120.0, pos: tempo::position(f as f64 * 0.033, 4), per_bar: 4, running: true };
        let dt = 1.0 / 60.0 + (f % 5) as f32 * 0.003;
        b.update(dt, 0, &now);
        p.update(&g, dt, &now);
        for i in 0..g.nodes.len() {
            for (x, y) in b.outputs(i).iter().zip(&p.out[i]) {
                assert!((x - y).abs() < 1e-4, "frame {f} node {}: {x} vs {y}", g.nodes[i].id);
            }
        }
        for port in ["speed", "zoom", "hue", "pulse"] {
            let (x, y) = (b.input(1, port, 0.5), p.input(&g, 1, port, 0.5));
            assert!((x - y).abs() < 1e-4, "frame {f} p.{port}: {x} vs {y}");
        }
    }
}

/// `BatchGraph`'s `sent_values_force_blend_and_release`, on the engine (#319): a send forces, blends and lets go; on an
/// audio input it glides in (smooth.rs) – no jump
#[test]
fn sent_values_force_blend_and_release() {
    let g = graph(
        r#"
[[node]]
id = "l"
kind = "lfo"
rate = 0.0
depth = 1.0
phase = 0.25
[[node]]
id = "m"
kind = "mul"
b = 2.0
[[link]]
from = "l"
to = "m.a"
[[node]]
id = "o"
kind = "osc"
freq = 440.0
gain = 0.0
[[node]]
id = "out"
kind = "audio_out"
[[link]]
from = "o"
to = "out.in"
"#,
    );
    let (mut e, _) = engine(&g);
    let m = |e: &mut Engine| {
        e.control_mut().step(0.0, &now());
        e.control().outputs(1)[0]
    };
    assert!((m(&mut e) - 2.0).abs() < 1e-5, "the lfo holds 1 (phase 0.25), m = 1 · 2");
    assert!(e.control_mut().send(1, 0, 3.0, 1.0));
    assert!((m(&mut e) - 6.0).abs() < 1e-5, "force: a = 3");
    assert!(e.control_mut().send(1, 0, 3.0, 0.5));
    assert!((m(&mut e) - 4.0).abs() < 1e-5, "blend half: a = (1 + 3) / 2");
    assert!(e.control_mut().send(1, 1, 0.5, 1.0));
    assert!((m(&mut e) - 1.0).abs() < 1e-5, "a constant input: b = 0.5");
    e.control_mut().send(1, 0, 3.0, 0.0);
    e.control_mut().send(1, 1, 0.0, 0.0);
    assert!((m(&mut e) - 2.0).abs() < 1e-5, "amount 0 = as built");
    assert!((e.control().input_at(1, 0).unwrap() - 1.0).abs() < 1e-6, "a path `m.a` reads the input");
    // the audio node: gain 0 → silence, a forced gain → sound
    let mut out = vec![0.0f32; 2 * QUANTUM];
    e.render(&mut out, 2, |x| x);
    assert!(out.iter().all(|x| *x == 0.0));
    let gain = Kind::Osc.input("gain").unwrap();
    assert!(e.control_mut().send(2, gain, 0.5, 1.0));
    e.render(&mut out, 2, |x| x);
    assert!(out.iter().all(|x| x.abs() < 0.1), "the gain glides in (smooth.rs, #188) – no jump");
    for _ in 0..30 {
        e.render(&mut out, 2, |x| x);
    }
    let peak = out.iter().fold(0.0f32, |p, x| p.max(x.abs()));
    assert!(peak > 0.3, "there after a few blocks: {peak}");
    assert!(!e.control_mut().send(2, 9, 1.0, 1.0) && !e.control_mut().send(9, 0, 1.0, 1.0), "no such input");
    let table: Vec<_> = e.control().nodes().collect();
    assert_eq!((table[2].0, table[2].1, table[2].2[gain]), ("o", "osc", "gain"));
}

/// `BatchGraph`'s `a_linked_input_turns_as_its_base`, on the engine: the link's modulation keeps flowing on top of a
/// turned base; a send blends over both and lets go back to the turned base
#[test]
fn a_linked_input_turns_as_its_base() {
    let g = graph(
        r#"
[[node]]
id = "l"
kind = "lfo"
rate = 0.0
depth = 1.0
phase = 0.25
[[node]]
id = "m"
kind = "mul"
b = 1.0
[[link]]
from = "l"
to = "m.a"
scale = 2.0
offset = 0.5
"#,
    );
    let mut c = Control::detached(&g);
    let m = |c: &mut Control| {
        c.step(0.0, &now());
        c.outputs(1)[0]
    };
    assert!((m(&mut c) - 2.5).abs() < 1e-5, "1 · 2 + the offset 0.5");
    assert_eq!(c.base(1, 0), Some(0.5), "the link's offset is the base");
    assert!(c.set_base(1, 0, 3.0));
    assert!((m(&mut c) - 5.0).abs() < 1e-5, "1 · 2 + the new base 3");
    assert!(c.send(0, 1, 0.5, 1.0), "the LFO's depth halved");
    assert!((m(&mut c) - 4.0).abs() < 1e-5, "the modulation still on top: 0.5 · 2 + 3");
    assert!(c.set_base(1, 1, 2.0), "an unlinked input: the base is its value");
    assert!((m(&mut c) - 8.0).abs() < 1e-5);
    c.send(1, 0, 10.0, 1.0);
    assert!((m(&mut c) - 20.0).abs() < 1e-5, "forced");
    c.send(1, 0, 0.0, 0.0);
    assert!((m(&mut c) - 8.0).abs() < 1e-5, "released: back to the turned base, not the built one");
    assert!(!c.set_base(1, 5, 1.0) && c.base(9, 0).is_none());
}

/// `BatchGraph`'s `wide_scenes_batch_wide` and `refusals`: 50 independent lfo → mul pairs are two levels of one batch
/// each; a cycle of helpers is refused (by the graph)
#[test]
fn wide_scenes_batch_wide_and_cycles_are_refused() {
    let mut s = String::new();
    for i in 0..50 {
        s += &format!("[[node]]\nid = \"l{i}\"\nkind = \"lfo\"\nrate = 1.0\n[[node]]\nid = \"m{i}\"\nkind = \"mul\"\nb = 2.0\n[[link]]\nfrom = \"l{i}\"\nto = \"m{i}.a\"\n");
    }
    s += "[[node]]\nid = \"o\"\nkind = \"osc\"\nfreq = 440.0\ngain = 0.1\n";
    assert_eq!(Control::detached(&graph(&s)).shape(), (2, 2));
    let cycle = "[[node]]\nid = \"a\"\nkind = \"add\"\n[[node]]\nid = \"b\"\nkind = \"add\"\n[[link]]\nfrom = \"a\"\nto = \"b.a\"\n[[link]]\nfrom = \"b\"\nto = \"a.a\"\n";
    assert!(Graph::build(&Scene::parse(cycle).unwrap()).unwrap_err().to_string().contains("cycle"));
}

/// deck-core's `synth.rs` `a_jumped_input_glides_across_the_block` (#188, retired with the reference blocks in #324), on the
/// engine: a gain sent between two blocks glides in on the oscillator and on the FM voice – the largest step between two
/// samples stays near the tone's own (a 50 Hz sine at gain 0.5 steps by ≤ 0.0033); held flat the jump would be the sine's
/// value at once (≈ 0.37 here)
#[test]
fn a_jumped_gain_glides_across_the_block() {
    for (kind, params) in [("osc", ""), ("fm", "ratio = 1.0\nindex = 0.0\n")] {
        let g = graph(&format!(
            "[[node]]\nid = \"o\"\nkind = \"{kind}\"\nfreq = 50.0\ngain = 0.0\n{params}[[node]]\nid = \"out\"\nkind = \"audio_out\"\n[[link]]\nfrom = \"o\"\nto = \"out.in\"\n"
        ));
        let gain = Kind::parse(kind).unwrap().input("gain").unwrap();
        let (mut e, _) = engine(&g);
        let mut out = vec![0.0f32; 2 * QUANTUM];
        let mut left: Vec<f32> = Vec::new();
        e.render(&mut out, 2, |x| x);
        left.extend(out.iter().step_by(2));
        assert!(e.control_mut().send(0, gain, 0.5, 1.0));
        for _ in 0..40 {
            e.render(&mut out, 2, |x| x);
            left.extend(out.iter().step_by(2));
        }
        let worst = left.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
        assert!(worst < 0.0075, "{kind}: the largest step {worst}");
        let peak = left[left.len() - 1000..].iter().fold(0.0f32, |p, x| p.max(x.abs()));
        assert!(peak > 0.3, "{kind}: settled at the sent gain: {peak}");
    }
}

/// The smoother is the exact one-pole: after `t` seconds a step input is reached by 1 − e^(−t/τ), whatever the step size –
/// so it glides the same at block rate as it did per frame
#[test]
fn the_smoother_glides_the_same_at_any_step() {
    let g = graph("[[node]]\nid=\"s\"\nkind=\"smooth\"\nin=1.0\ntime=0.2\n");
    for dt in [128.0 / 48_000.0, 1.0 / 60.0, 0.05] {
        let mut c = Control::detached(&g);
        let steps = (0.4 / dt as f64).round() as usize;
        for _ in 0..steps {
            c.step(dt, &now());
        }
        let t = steps as f32 * dt;
        assert!((c.outputs(0)[0] - (1.0 - (-t / 0.2).exp())).abs() < 1e-4, "dt {dt}");
    }
}

/// The face over the engine: commands reach the control half at the next block, and the face reads the block that is
/// heard (`lead` frames behind the rendered frontier) from the snapshot ring, not the newest
#[test]
fn the_face_reads_the_heard_block_and_its_commands_arrive() {
    let g = graph(
        r#"
[[node]]
id = "l"
kind = "lfo"
rate = 4.0
shape = "saw"
[[node]]
id = "p"
kind = "test_pattern"
[[node]]
id = "o"
kind = "osc"
[[node]]
id = "out"
kind = "audio_out"
[[link]]
from = "l"
to = "p.speed"
[[link]]
from = "l"
to = "o.gain"
[[link]]
from = "o"
to = "out.in"
"#,
    );
    let (mut e, mut h) = engine(&g);
    let mut face = Controls::new(&g, Some(&mut h));
    assert!(h.control.is_none(), "the face took the link");
    let mut buf = vec![0.0f32; 2 * QUANTUM];
    face.update(0.0, 0, &now());
    assert_eq!(face.outputs(0), [0.0], "nothing rendered yet: nothing to show");
    // the saw (0..depth: its phase) rises by 4 · 128 / 48000 per block; block b (from frame b · 128) shows b + 1 steps
    let per = 4.0 * QUANTUM as f32 / 48_000.0;
    for _ in 0..20 {
        e.render(&mut buf, 2, |x| x);
    }
    face.update(0.0, 0, &now());
    let newest = face.outputs(0)[0];
    assert!((newest - 20.0 * per).abs() < 1e-5, "the newest block: phase 20 · per → {newest}");
    face.update(0.0, 5 * QUANTUM, &now());
    let heard = face.outputs(0)[0];
    assert!((heard - 16.0 * per).abs() < 1e-5, "five blocks back: block 15 → {heard}");
    face.update(0.0, 5 * QUANTUM + 60, &now());
    assert!((face.outputs(0)[0] - 15.0 * per).abs() < 1e-5, "between blocks: the block that started before (14)");
    face.update(0.0, 1_000 * QUANTUM, &now());
    assert!(face.outputs(0)[0] > 0.0, "older than the ring: the oldest kept block");
    // a base turned on the face reaches the engine at the next block; the face reads its own copy at once
    assert!(face.set_base(1, 0, 2.0));
    assert_eq!(face.base(1, 0), Some(2.0));
    e.render(&mut buf, 2, |x| x);
    assert_eq!(e.control().base(1, 0), Some(2.0), "the command arrived");
    face.update(0.0, 0, &now());
    assert_eq!(face.input(1, "speed", 0.0), e.control().input(1, "speed", 0.0), "the face reads what the engine computed");
    assert!(face.send(2, 1, 0.3, 1.0) && face.set_lfo_shape(0, 2));
    e.render(&mut buf, 2, |x| x);
    assert_eq!((e.control().input_at(2, 1), e.control().lfo_shape(0)), (Some(0.3), Some(2)));
    assert!(!face.set_lfo_shape(1, 0) && !face.send(7, 0, 0.0, 1.0));
}

#[test]
fn the_snapshot_ring_by_frame() {
    let r = Snapshots::new(2);
    let mut out = [9.0f32; 2];
    assert_eq!(r.read(0, &mut out), None);
    assert_eq!(out, [9.0, 9.0], "nothing written: out keeps its values");
    for b in 0..(DEPTH as u64 + 10) {
        r.write(b * 128, &[b as f32, -(b as f32)]);
    }
    let newest = DEPTH as u64 + 9;
    assert_eq!(r.read(u64::MAX, &mut out), Some(newest * 128));
    assert_eq!(out, [newest as f32, -(newest as f32)]);
    assert_eq!(r.read(40 * 128 + 127, &mut out), Some(40 * 128));
    assert_eq!(out[0], 40.0);
    assert_eq!(r.read(0, &mut out), Some(10 * 128), "the oldest kept block");
}
