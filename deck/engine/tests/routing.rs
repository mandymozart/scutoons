//! Channels and routing (plan 2.6, #318): sends with channel maps, folders, buses, the spec's down-mix tables, pan laws,
//! the feedback loop's one block, cycle rejection, outlets and the deck configuration, the limiter per group, the meters.
//! Every comparison is exact: the sources are rendered alone as references and the routing graph must give their samples
//! summed, scaled, moved and mixed by the routing's own arithmetic, bit for bit. Levels stay under the limiter's ceiling, which
//! makes it a pure delay of `LOOK` frames (the same on every channel) – so references and routed renders line up.
use deck_engine::layout::{Layout, Speaker};
use deck_engine::pan::{Law, Panner};
use deck_engine::{Build, DeckConfig, Engine, Sequencing, Seams};
use deck_space::graph::Graph;
use deck_space::scene::Scene;

const RATE: f32 = 48_000.0;
const FRAMES: usize = 128 * 40;

fn engine(toml: &str, deck: DeckConfig) -> (Engine, deck_engine::Handles) {
    let graph = Graph::build(&Scene::parse(toml).unwrap_or_else(|e| panic!("{e}\n{toml}"))).unwrap_or_else(|e| panic!("{e}\n{toml}"));
    Engine::build_seams(&graph, RATE, 1.0, Build::detect(), Sequencing::InBlock, &Seams::with_deck(deck)).unwrap_or_else(|e| panic!("{e:#}\n{toml}"))
}

/// `FRAMES` frames of every device channel
fn render(toml: &str, deck: DeckConfig) -> Vec<Vec<f32>> {
    let d = deck.channels;
    let (mut e, _) = engine(toml, deck);
    let mut out = vec![Vec::with_capacity(FRAMES); d];
    let mut buf = vec![0.0f32; 128 * d];
    for _ in 0..FRAMES / 128 {
        e.render(&mut buf, d, |x| x);
        for frame in buf.chunks_exact(d) {
            for (c, x) in frame.iter().enumerate() {
                out[c].push(*x);
            }
        }
    }
    out
}

const STEREO: fn() -> DeckConfig = DeckConfig::stereo;

fn osc(id: &str, freq: f32, gain: f32) -> String {
    format!("[[node]]\nid=\"{id}\"\nkind=\"osc\"\nfreq={freq}\ngain={gain}\n")
}
fn node(id: &str, kind: &str, params: &str) -> String {
    format!("[[node]]\nid=\"{id}\"\nkind=\"{kind}\"\n{params}\n")
}
fn link(from: &str, to: &str, extra: &str) -> String {
    format!("[[link]]\nfrom=\"{from}\"\nto=\"{to}\"\n{extra}\n")
}

/// a source rendered alone onto a stereo `audio_out`: the reference (both channels, after the limiter's delay)
fn alone(source: &str) -> Vec<Vec<f32>> {
    render(&(source.to_string() + &node("out", "audio_out", "volume=1") + &link("a", "out.in", "")), STEREO())
}

fn peak(x: &[f32]) -> f32 {
    x.iter().fold(0.0, |m, v| m.max(v.abs()))
}

fn exact(a: &[f32], b: &[f32], what: &str) {
    assert_eq!(a.len(), b.len(), "{what}");
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        assert!(x.to_bits() == y.to_bits(), "{what}: sample {i}: {x} vs {y}");
    }
}

/// the same samples, a zero of either sign alike (a pan gain of -0.0 and a ramp that starts from it)
fn equal(a: &[f32], b: &[f32], what: &str) {
    assert_eq!(a.len(), b.len(), "{what}");
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        assert!(x == y, "{what}: sample {i}: {x} vs {y}");
    }
}

/// a send from a bus's pre-fader tap with the channel map 1/2 → 3/4 at half level, a folder summing two buses, an `out` on
/// device channels 3 and 4: the P0 probe's routing graph (plan 3.8), now in the engine
#[test]
fn a_send_with_a_channel_map_a_folder_and_a_cue_out() {
    let a = osc("a", 220.0, 0.2);
    let b = osc("b", 330.0, 0.15);
    let toml = a.clone()
        + &b
        + &node("ta", "bus", "")
        + &node("tb", "bus", "gain=0.5")
        + &node("folder", "bus", "")
        + &node("sendbus", "bus", "channels=4")
        + &node("out", "out", "group=\"main\"")
        + &node("cue", "out", "group=\"cue\"")
        + &link("a", "ta.in", "")
        + &link("b", "tb.in", "")
        + &link("ta.out", "folder.in", "")
        + &link("tb.out", "folder.in", "")
        // the send: ta's pre-fader sum at half level, channels 1/2 → 3/4 of a four-channel bus
        + &link("ta.pre", "sendbus.in", "scale=0.5\nmap=[[1, 3], [2, 4]]")
        + &link("folder.out", "out.in", "")
        // the cue out reads the send bus's channels 3/4
        + &link("sendbus.out", "cue.in", "map=[[3, 1], [4, 2]]");
    let deck = DeckConfig::device(4).with_outlet("main", &[1, 2]).with_outlet("cue", &[3, 4]);
    let got = render(&toml, deck);
    let ra = alone(&osc("a", 220.0, 0.2));
    let rb = alone(&osc("a", 330.0, 0.15));
    for c in 0..2 {
        // the folder: ta (unity) + tb (half): (0 + a) + b/2, the fader gliding from 1 to .5 only at the start – check after it settled
        let main: Vec<f32> = ra[c].iter().zip(&rb[c]).map(|(a, b)| a + b * 0.5).collect();
        let skip = 128 * 8; // the fader's glide (deck-core smooth GLIDE) has ended
        exact(&got[c][skip..], &main[skip..], &format!("main {c}"));
        // the cue: ta's sound at half level on channels 3 and 4
        let cue: Vec<f32> = ra[c].iter().map(|a| a * 0.5).collect();
        exact(&got[2 + c], &cue, &format!("cue {}", 3 + c));
    }
}

/// one source may go to several targets (a copy), several sources into one target sum, a map may swap
#[test]
fn channel_maps_copy_swap_and_sum() {
    // the sequencer's cv has pitch on the left and the gate on the right: two different channels
    let seq = node("a", "seq", "bpm=240\ndivision=4\ngate=0.5\nsteps=[45, 0, 52, 57]");
    let k = "scale=0.0009765625"; // 2^-10: the pitch (≤ 220 Hz) under the limiter's ceiling, exactly
    let reference = render(&(seq.clone() + &node("out", "audio_out", "volume=1") + &link("a.cv", "out.in", k)), STEREO());
    let (pitch, gate) = (&reference[0], &reference[1]);
    assert!(peak(pitch) > 0.05 && peak(gate) > 0.0009, "the reference plays");
    let through = |map: &str, group_first: usize| {
        let toml = seq.clone() + &node("b", "bus", "") + &node("o", "out", &format!("first={group_first}")) + &link("a.cv", "b.in", &format!("{k}\nmap={map}")) + &link("b.out", "o.in", "");
        render(&toml, DeckConfig::device(4))
    };
    // swap: 1 → 2, 2 → 1
    let s = through("[[1, 2], [2, 1]]", 1);
    exact(&s[0], gate, "swap: channel 1 is the gate");
    exact(&s[1], pitch, "swap: channel 2 is the pitch");
    // copy: the pitch to channels 1 and 2, nothing else
    let c = through("[[1, 1], [1, 2]]", 1);
    exact(&c[0], pitch, "copy 1");
    exact(&c[1], pitch, "copy 2");
    // sum: both into channel 1 (a mono bus)
    let toml = seq.clone() + &node("b", "bus", "channels=1") + &node("o", "out", "first=3") + &link("a.cv", "b.in", &format!("{k}\nmap=[[1, 1], [2, 1]]")) + &link("b.out", "o.in", "");
    let m = render(&toml, DeckConfig::device(4));
    let sum: Vec<f32> = pitch.iter().zip(gate).map(|(p, g)| (0.0 + p) + g).collect();
    exact(&m[2], &sum, "sum: pitch + gate on channel 3");
    assert_eq!(peak(&m[0]), 0.0);
    assert_eq!(peak(&m[3]), 0.0, "a mono out writes one channel");
    // channels of the map beyond the bus are dropped, and an identity link into a wider bus lands on 1..n
    let wide = through("[[1, 3]]", 1);
    assert!(peak(&wide[0]) == 0.0 && peak(&wide[1]) == 0.0 && peak(&wide[2]) > 0.05, "a map to channel 3 makes the bus three wide");
}

/// several links into one input add in the scene's order; a lone unscaled link is read in place (a zero-copy path that
/// gives the same samples)
#[test]
fn inputs_sum_in_the_links_order() {
    let a = osc("a", 220.0, 0.2) + &osc("b", 330.0, 0.15) + &osc("c", 110.0, 0.1);
    let sum = |order: [&str; 3]| {
        let mut toml = a.clone() + &node("bus", "bus", "") + &node("out", "audio_out", "volume=1");
        for s in order {
            toml += &link(s, "bus.in", "");
        }
        toml + &link("bus.out", "out.in", "")
    };
    let got = render(&sum(["a", "b", "c"]), STEREO());
    let (ra, rb, rc) = (alone(&osc("a", 220.0, 0.2)), alone(&osc("a", 330.0, 0.15)), alone(&osc("a", 110.0, 0.1)));
    let want: Vec<f32> = (0..FRAMES).map(|i| ((0.0 + ra[0][i]) + rb[0][i]) + rc[0][i]).collect();
    exact(&got[0], &want, "(a + b) + c");
    let other = render(&sum(["c", "b", "a"]), STEREO());
    let want: Vec<f32> = (0..FRAMES).map(|i| ((0.0 + rc[0][i]) + rb[0][i]) + ra[0][i]).collect();
    exact(&other[0], &want, "(c + b) + a – the links' order is the sum's order");
}

#[test]
fn mute_solo_and_the_fader() {
    let a = osc("a", 220.0, 0.2);
    let two = |params_a: &str, params_b: &str| {
        a.clone()
            + &osc("b", 330.0, 0.2)
            + &node("ta", "bus", params_a)
            + &node("tb", "bus", params_b)
            + &node("main", "bus", "")
            + &node("out", "audio_out", "volume=1")
            + &link("a", "ta.in", "")
            + &link("b", "tb.in", "")
            + &link("ta.out", "main.in", "")
            + &link("tb.out", "main.in", "")
            + &link("main.out", "out.in", "")
    };
    let ra = alone(&a);
    // a muted bus is silent at its out: the sum is the other bus alone
    let muted = render(&two("mute=1", ""), STEREO());
    let rb = alone(&osc("a", 330.0, 0.2));
    exact(&muted[0], &rb[0].iter().map(|b| 0.0 + b).collect::<Vec<_>>(), "mute");
    // a soloed bus silences the others
    let solo = render(&two("solo=1", ""), STEREO());
    exact(&solo[0], &ra[0].iter().map(|a| 0.0 + a).collect::<Vec<_>>(), "solo");
    // a muted bus still feeds its pre-fader send (a muted kick still ducks through its send)
    let send = a.clone() + &node("t", "bus", "mute=1") + &node("s", "bus", "") + &node("out", "audio_out", "volume=1") + &link("a", "t.in", "") + &link("t.pre", "s.in", "") + &link("s.out", "out.in", "");
    let got = render(&send, STEREO());
    exact(&got[0], &ra[0].iter().map(|a| 0.0 + a).collect::<Vec<_>>(), "pre keeps playing under a mute");
    // the fader scales: gain 0.5 is exactly half
    let half = render(&(a.clone() + &node("t", "bus", "gain=0.5") + &node("out", "audio_out", "volume=1") + &link("a", "t.in", "") + &link("t.out", "out.in", "")), STEREO());
    exact(&half[0][128 * 8..], &ra[0][128 * 8..].iter().map(|a| a * 0.5).collect::<Vec<_>>(), "fader");
}

/// Soloing a folder bus plays the folder – its children stay audible into it (Reaper's folder solo); a bus outside it is
/// silent. It used to gate the children, so the soloed folder was silence.
#[test]
fn a_soloed_folder_plays_its_children() {
    let scene = osc("a", 220.0, 0.2) + &osc("b", 330.0, 0.15) + &osc("c", 110.0, 0.2)
        + &node("ta", "bus", "")
        + &node("tb", "bus", "")
        + &node("tc", "bus", "")
        + &node("folder", "bus", "solo=1")
        + &node("main", "bus", "")
        + &node("out", "audio_out", "volume=1")
        + &link("a", "ta.in", "")
        + &link("b", "tb.in", "")
        + &link("c", "tc.in", "")
        + &link("ta.out", "folder.in", "")
        + &link("tb.out", "folder.in", "")
        + &link("folder.out", "main.in", "")
        + &link("tc.out", "main.in", "")
        + &link("main.out", "out.in", "");
    let got = render(&scene, STEREO());
    let (ra, rb) = (alone(&osc("a", 220.0, 0.2)), alone(&osc("a", 330.0, 0.15)));
    // main = folder + tc (gated to zero), folder = (0 + a) + b
    let want: Vec<f32> = ra[0].iter().zip(&rb[0]).map(|(a, b)| ((0.0 + a) + b) + 0.0).collect();
    exact(&got[0], &want, "the folder's children, and nothing of tc");
}

/// the loop closes only through `feedback`, which plays its input one block (128 frames) late
#[test]
fn feedback_is_one_block_late_and_other_cycles_are_errors() {
    let a = osc("a", 220.0, 0.2);
    let toml = a.clone() + &node("fb", "feedback", "") + &node("out", "audio_out", "volume=1") + &link("a", "fb.in", "") + &link("fb.out", "out.in", "");
    let got = render(&toml, STEREO());
    let r = alone(&a);
    assert!(got[0][..128].iter().all(|x| *x == 0.0), "nothing came in before the first block");
    exact(&got[0][128..], &r[0][..FRAMES - 128], "the same sound, 128 frames later");
    // a loop through a bus: y = x + y(one block ago) / 2, summed in the links' order
    let looped = osc("a", 220.0, 0.05) + &node("b", "bus", "") + &node("fb", "feedback", "") + &node("out", "audio_out", "volume=1")
        + &link("a", "b.in", "")
        + &link("fb.out", "b.in", "scale=0.5")
        + &link("b.out", "fb.in", "")
        + &link("b.out", "out.in", "");
    let got = render(&looped, STEREO());
    let x = alone(&osc("a", 220.0, 0.05));
    let mut y = vec![0.0f32; FRAMES];
    for i in 0..FRAMES {
        let late = if i >= 128 { y[i - 128] * 0.5 } else { 0.0 };
        y[i] = (0.0 + x[0][i]) + late;
    }
    // x[0][i] and y share the limiter's delay – the recursion is on the delayed signal's own samples
    exact(&got[0][128..], &y[128..], "the loop");
    // every other cycle is refused, and the error names it
    let cyc = node("p", "bus", "") + &node("q", "bus", "") + &link("p.out", "q.in", "") + &link("q.out", "p.in", "");
    let e = Graph::build(&Scene::parse(&cyc).unwrap()).unwrap_err().to_string();
    assert!(e.contains("cycle") && e.contains('p') && e.contains('q') && e.contains("feedback"), "{e}");
}

/// every `feedback` node is one block late, also when it reads another one: two in a row are two blocks; a node fed only by
/// its own output is a loop of silence (it used to read and write its one buffer in the same step – a panic)
#[test]
fn feedback_nodes_in_a_row_each_add_a_block() {
    let a = osc("a", 220.0, 0.2);
    let r = alone(&a);
    let toml = a.clone() + &node("f1", "feedback", "") + &node("f2", "feedback", "") + &node("out", "audio_out", "volume=1")
        + &link("a", "f1.in", "")
        + &link("f1.out", "f2.in", "")
        + &link("f2.out", "out.in", "");
    let got = render(&toml, STEREO());
    assert!(got[0][..256].iter().all(|x| *x == 0.0), "nothing before two blocks");
    exact(&got[0][256..], &r[0][..FRAMES - 256], "two feedback nodes = 256 frames");
    // the same with the second one declared first (the closing order follows the node order)
    let swapped = a + &node("f2", "feedback", "") + &node("f1", "feedback", "") + &node("out", "audio_out", "volume=1")
        + &link("a", "f1.in", "")
        + &link("f1.out", "f2.in", "")
        + &link("f2.out", "out.in", "");
    exact(&render(&swapped, STEREO())[0], &got[0], "the declaration order does not matter");
    // a node that only hears itself
    let own = osc("a", 220.0, 0.2) + &node("f", "feedback", "") + &node("out", "audio_out", "volume=1") + &link("f.out", "f.in", "") + &link("f.out", "out.in", "");
    assert_eq!(peak(&render(&own, STEREO())[0]), 0.0);
}

/// 5.1 down-mixed to stereo by the spec's table, by a `downmix` node and by the deck's fold of an outlet onto two channels
#[test]
fn a_surround_scene_on_six_channels_and_folded_on_two() {
    let freqs = [220.0, 261.6, 329.6, 110.0, 392.0, 440.0]; // L R C LFE Ls Rs
    let mut srcs = String::new();
    let mut sixbus = node("bed", "bus", "channels=6");
    for (k, f) in freqs.iter().enumerate() {
        srcs += &osc(&format!("s{k}"), *f, 0.1);
        sixbus += &link(&format!("s{k}"), "bed.in", &format!("map=[[1, {}]]", k + 1));
    }
    let scene = srcs.clone() + &sixbus + &node("o", "out", "layout=\"5.1\"") + &link("bed.out", "o.in", "");
    // on a six-channel device each channel plays itself
    let six = render(&scene, DeckConfig::device(6));
    let refs: Vec<Vec<Vec<f32>>> = freqs.iter().map(|f| alone(&osc("a", *f, 0.1))).collect();
    for (k, r) in refs.iter().enumerate() {
        exact(&six[k], &r[0], &format!("channel {}", k + 1));
    }
    // on a stereo device the outlet is folded by BS.775: Lo = L + C/√2 + Ls/√2, Ro = R + C/√2 + Rs/√2, the LFE dropped
    let two = render(&scene, DeckConfig::stereo());
    let h = std::f32::consts::FRAC_1_SQRT_2;
    let lo: Vec<f32> = (0..FRAMES).map(|i| (six[0][i] * 1.0 + six[2][i] * h) + six[4][i] * h).collect();
    let ro: Vec<f32> = (0..FRAMES).map(|i| (six[1][i] * 1.0 + six[2][i] * h) + six[5][i] * h).collect();
    exact(&two[0], &lo, "folded left");
    exact(&two[1], &ro, "folded right");
    // the same down-mix as a node in the graph (the artistic choice): identical samples
    let node_mix = srcs + &sixbus_with_downmix(&freqs);
    let viad = render(&node_mix, DeckConfig::stereo());
    exact(&viad[0], &lo, "downmix node, left");
    exact(&viad[1], &ro, "downmix node, right");
    // a fold off: the first channels play
    let mut drop = DeckConfig::stereo();
    drop.fold = deck_engine::Fold::Drop;
    let dropped = render(&scene, drop);
    exact(&dropped[0], &six[0], "dropped left is L");
    exact(&dropped[1], &six[1], "dropped right is R");
}

fn sixbus_with_downmix(freqs: &[f32; 6]) -> String {
    let mut s = node("bed", "bus", "channels=6");
    for k in 0..freqs.len() {
        s += &link(&format!("s{k}"), "bed.in", &format!("map=[[1, {}]]", k + 1));
    }
    s + &node("dm", "downmix", "from=\"5.1\"\nto=\"stereo\"") + &node("o", "out", "") + &link("bed.out", "dm.in", "") + &link("dm.out", "o.in", "")
}

/// a custom gain matrix, any n → m
#[test]
fn a_custom_matrix_mixes_any_n_to_m() {
    let scene = osc("x", 220.0, 0.2) + &osc("y", 330.0, 0.2) + &node("b", "bus", "channels=3") + &link("x", "b.in", "map=[[1, 1]]") + &link("y", "b.in", "map=[[1, 3]]")
        // two output rows over three input channels
        + &node("m", "downmix", "inputs=3\nmatrix=[0.5, 0.0, 0.25, 0.0, 1.0, 0.5]")
        + &node("o", "out", "")
        + &link("b.out", "m.in", "")
        + &link("m.out", "o.in", "");
    let got = render(&scene, STEREO());
    let (rx, ry) = (alone(&osc("a", 220.0, 0.2)), alone(&osc("a", 330.0, 0.2)));
    let l: Vec<f32> = (0..FRAMES).map(|i| (0.0 + 0.5 * rx[0][i]) + 0.25 * ry[0][i]).collect();
    let r: Vec<f32> = (0..FRAMES).map(|i| 0.0 + 0.5 * ry[0][i]).collect();
    exact(&got[0], &l, "row 1: .5 x + .25 y");
    exact(&got[1], &r, "row 2: .5 y (the middle input is silent)");
}

/// a mono source panned over four speakers: each channel is the source times the VBAP gain of the library's panner
#[test]
fn vbap_over_four_speakers_in_the_engine() {
    let ring = "az=[45.0, 135.0, 225.0, 315.0]\nmode=\"vbap\"";
    let toml = |az: f32| osc("a", 220.0, 0.2) + &node("p", "pan", &format!("{ring}\nazimuth={az}")) + &node("o", "out", "") + &link("a", "p.in", "") + &link("p.out", "o.in", "");
    let r = alone(&osc("a", 220.0, 0.2));
    let speakers: Vec<Speaker> = [45.0, 135.0, 225.0, 315.0].iter().map(|&az| Speaker { az, el: 0.0, dist: 1.0, lfe: false }).collect();
    let panner = Panner::new(&Layout::new("ring", &speakers), Law::Vbap, 6.0, 0.1).unwrap();
    for az in [45.0f32, 90.0, 100.0, 200.0, 330.0] {
        let got = render(&toml(az), DeckConfig::device(4));
        let mut g = [0.0f32; 4];
        panner.gains(az, 0.0, 1.0, &mut g);
        for c in 0..4 {
            let want: Vec<f32> = r[0].iter().map(|x| x * g[c]).collect();
            equal(&got[c], &want, &format!("azimuth {az}, speaker {c}"));
        }
        let power: f32 = (0..4).map(|c| got[c][4000] * got[c][4000]).sum();
        assert!((power - r[0][4000] * r[0][4000]).abs() < 1e-6, "power is kept at azimuth {az}");
    }
    // equal power on a stereo `pan`: the middle plays 1/√2 on both channels
    let mid = render(&(osc("a", 220.0, 0.2) + &node("p", "pan", "") + &node("o", "out", "") + &link("a", "p.in", "") + &link("p.out", "o.in", "")), STEREO());
    let g = std::f32::consts::FRAC_1_SQRT_2;
    for (c, m) in mid.iter().enumerate() {
        assert!(m.iter().zip(&r[0]).all(|(m, x)| (m - x * g).abs() < 1e-6), "channel {c}");
    }
}

/// outlets: the same scene plays its `main` on 1/2 and its `cue` on 3/4 of an eight-channel interface; on a phone the cue
/// has nowhere to play and is silent, the main is untouched
#[test]
fn outlets_follow_the_deck_configuration() {
    let scene = osc("a", 220.0, 0.2) + &osc("b", 330.0, 0.2) + &node("m", "out", "group=\"main\"") + &node("c", "out", "group=\"cue\"") + &link("a", "m.in", "") + &link("b", "c.in", "");
    let rack = DeckConfig::device(8).with_outlet("main", &[1, 2]).with_outlet("cue", &[3, 4]);
    let got = render(&scene, rack);
    let (ra, rb) = (alone(&osc("a", 220.0, 0.2)), alone(&osc("a", 330.0, 0.2)));
    exact(&got[0], &ra[0], "main 1");
    exact(&got[1], &ra[1], "main 2");
    exact(&got[2], &rb[0], "cue 3");
    exact(&got[3], &rb[1], "cue 4");
    for (c, ch) in got.iter().enumerate().skip(4) {
        assert_eq!(peak(ch), 0.0, "channel {} is silent", c + 1);
    }
    // explicit device channels on the node win over the configuration
    let pinned = osc("a", 220.0, 0.2) + &node("m", "out", "channels=[7, 8]") + &link("a", "m.in", "");
    let got = render(&pinned, DeckConfig::device(8));
    exact(&got[6], &ra[0], "pinned 7");
    assert_eq!(peak(&got[0]), 0.0);
    // on a phone the cue has no channel: silent, and the build says so
    let (_, h) = engine(&scene, DeckConfig::stereo());
    assert!(h.notes.iter().any(|n| n.contains("cue") && n.contains("silent")), "{:?}", h.notes);
    let phone = render(&scene, DeckConfig::stereo());
    exact(&phone[0], &ra[0], "the main is the same on a phone");
}

/// A channel list naming a device channel that is missing drops that channel and keeps the others where the list puts them;
/// it used to close the gap, so the next channel played where the missing one should have.
#[test]
fn a_missing_device_channel_is_dropped_not_closed_up() {
    let two = |out_params: &str| {
        osc("x", 220.0, 0.2) + &osc("y", 330.0, 0.15) + &node("b", "bus", "channels=2") + &node("o", "out", out_params)
            + &link("x", "b.in", "map=[[1, 1]]")
            + &link("y", "b.in", "map=[[1, 2]]")
            + &link("b.out", "o.in", "")
    };
    let (rx, ry) = (alone(&osc("a", 220.0, 0.2)), alone(&osc("a", 330.0, 0.15)));
    let (x, y): (Vec<f32>, Vec<f32>) = (rx[0].iter().map(|v| 0.0 + v).collect(), ry[0].iter().map(|v| 0.0 + v).collect());
    let mut drop = DeckConfig::device(4);
    drop.fold = deck_engine::Fold::Drop;
    // L on channel 5 (missing), R on channel 2
    let got = render(&two("channels=[5, 2]"), drop.clone());
    exact(&got[1], &y, "R stays on 2");
    assert!([0, 2, 3].iter().all(|&c| peak(&got[c]) == 0.0), "L has no channel: silent");
    // the same through the deck configuration's group
    let got = render(&two("group=\"cue\""), drop.with_outlet("cue", &[5, 2]));
    exact(&got[1], &y, "the group's R stays on 2");
    assert_eq!(peak(&got[0]), 0.0);
    // three channels (no standard layout, so no table to fold by): the middle one's channel is missing
    let three = osc("x", 220.0, 0.2) + &osc("y", 330.0, 0.15) + &node("b", "bus", "channels=3") + &node("o", "out", "channels=[1, 9, 2]")
        + &link("x", "b.in", "map=[[1, 1]]")
        + &link("y", "b.in", "map=[[1, 3]]")
        + &link("b.out", "o.in", "");
    let got = render(&three, STEREO());
    exact(&got[0], &x, "1 → 1");
    exact(&got[1], &y, "3 → 2");
}

/// the limiter works per group: a loud cue is held under the ceiling, the main next to it does not move
#[test]
fn the_limiter_links_a_group_and_leaves_the_others_alone() {
    let scene = osc("a", 220.0, 0.2) + &osc("b", 330.0, 0.95) + &osc("c", 331.0, 0.95) + &node("sum", "bus", "")
        + &node("m", "out", "group=\"main\"")
        + &node("c1", "out", "group=\"cue\"")
        + &link("a", "m.in", "")
        + &link("b", "sum.in", "")
        + &link("c", "sum.in", "")
        + &link("sum.out", "c1.in", "");
    let got = render(&scene, DeckConfig::device(4).with_outlet("cue", &[3, 4]));
    let r = alone(&osc("a", 220.0, 0.2));
    exact(&got[0], &r[0], "the quiet main is untouched");
    assert!(peak(&got[2]) < 1.0 && peak(&got[2]) > 0.8, "the cue is limited, peak {}", peak(&got[2]));
    // linked: a loud left and a quiet right – the right, far under the ceiling alone, is lowered with the left (the
    // pair keeps its balance); a limiter per channel would leave it at 0.2
    let lr = osc("b", 330.0, 0.95) + &osc("c", 331.0, 0.95) + &osc("q", 220.0, 0.2) + &node("sum", "bus", "channels=2")
        + &node("c1", "out", "group=\"cue\"")
        + &link("b", "sum.in", "map=[[1, 1]]")
        + &link("c", "sum.in", "map=[[1, 1]]")
        + &link("q", "sum.in", "map=[[1, 2]]")
        + &link("sum.out", "c1.in", "");
    let got = render(&lr, DeckConfig::device(4).with_outlet("cue", &[3, 4]));
    let (loud, quiet) = (peak(&got[2][FRAMES / 2..]), peak(&got[3][FRAMES / 2..]));
    assert!(loud < 1.0 && quiet < 0.2 * 0.6, "the quiet right follows the loud left's gain: {loud} / {quiet}");
    assert!(quiet > 0.2 * 0.3, "but it still plays: {quiet}");
}

/// the device meters follow the speakers: the peak of each device channel
#[test]
fn the_device_has_a_meter_per_channel() {
    let scene = osc("a", 220.0, 0.25) + &node("o", "out", "first=2") + &link("a", "o.in", "");
    let (mut e, h) = engine(&scene, DeckConfig::device(3));
    let mut buf = vec![0.0f32; 128 * 3 * 8];
    e.render(&mut buf, 3, |x| x);
    assert_eq!(h.device.channels(), 3);
    assert_eq!(h.device.peak(0), 0.0);
    assert!(h.device.peak(1) > 0.2 && h.device.peak(1) < 0.26 && h.device.peak(2) > 0.2, "{} {}", h.device.peak(1), h.device.peak(2));
}

#[test]
fn bad_routing_is_refused_with_a_reason() {
    let build = |toml: &str| Engine::build_seams(&Graph::build(&Scene::parse(toml).unwrap()).unwrap(), RATE, 1.0, Build::detect(), Sequencing::InBlock, &Seams::default()).err().map(|e| format!("{e:#}"));
    assert!(build(&(node("p", "pan", "layout=\"nowhere\""))).unwrap().contains("unknown layout"));
    assert!(build(&node("p", "pan", "layout=\"quad\"\nmode=\"equal_power\"")).unwrap().contains("two speakers"));
    assert!(build(&node("d", "downmix", "from=\"7.1\"\nto=\"stereo\"")).unwrap().contains("matrix"));
    assert!(Graph::build(&Scene::parse(&(osc("a", 1.0, 1.0) + &node("b", "bus", "") + &link("a", "b.in", "map=[[0, 1]]"))).unwrap()).is_err(), "channels count from 1");
    assert!(Graph::build(&Scene::parse(&(node("l", "lfo", "") + &node("m", "lfo", "") + &link("l", "m.rate", "map=[[1, 1]]"))).unwrap()).is_err(), "only audio has a map");
}
