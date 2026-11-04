use super::*;
use crate::wire::encode_send;

const OUT: &str = "[[node]]\nid = \"out\"\nkind = \"audio_out\"\n";

/// an LFO (still) and an FM voice into the output
fn scene() -> String {
    format!(
        "[[node]]\nid = \"wob\"\nkind = \"lfo\"\nrate = 0.0\ndepth = 1.0\nphase = 0.25\n\
         [[node]]\nid = \"voice\"\nkind = \"fm\"\nfreq = 110.0\nratio = 2.0\nindex = 0.0\ngain = 0.0\n{OUT}\
         [[link]]\nfrom = \"voice\"\nto = \"out.in\"\n"
    )
}

fn table(c: &Core) -> serde_json::Value {
    serde_json::from_str(&c.table_json()).unwrap()
}

/// the value of `node`'s port `name` in an analysis frame (the table says where)
fn value(c: &Core, frame: &[f32], node: &str, name: &str) -> f32 {
    let t = table(c);
    let row = t["nodes"].as_array().unwrap().iter().find(|n| n["id"] == node).unwrap();
    let ports: Vec<&str> = row["ports"].as_array().unwrap().iter().map(|p| p.as_str().unwrap()).collect();
    let outs: Vec<&str> = row["outs"].as_array().unwrap().iter().map(|p| p.as_str().unwrap()).collect();
    let k = ports.iter().position(|p| *p == name).or_else(|| outs.iter().position(|p| *p == name).map(|k| ports.len() + k)).unwrap();
    frame[VALUES_AT + row["at"].as_u64().unwrap() as usize + k]
}

fn play(c: &mut Core, blocks: usize) -> Vec<f32> {
    let mut all = Vec::new();
    for _ in 0..blocks {
        all.extend_from_slice(c.render(128));
    }
    all
}

#[test]
fn the_table_names_nodes_ports_outputs_and_where_their_values_are() {
    let c = Core::load(&scene(), 48_000.0, 2).unwrap();
    let t = table(&c);
    assert_eq!(t["channels"], 2);
    assert_eq!(t["nodes"][0], serde_json::json!({"id": "wob", "kind": "lfo", "ports": ["rate", "depth"], "outs": ["out"], "at": 0}));
    assert_eq!(t["nodes"][1]["ports"], serde_json::json!(["cv", "freq", "ratio", "index", "gain"]));
    assert_eq!(t["nodes"][1]["at"], 3, "after the LFO's two inputs and one output");
    let odd = Core::load(&format!("[[node]]\nid = \"a\\\"b\\\\c\"\nkind = \"osc\"\n{OUT}"), 48_000.0, 2).unwrap();
    assert_eq!(table(&odd)["nodes"][0]["id"], "a\"b\\c");
}

#[test]
fn sent_vectors_reach_the_sound_and_the_analysis() {
    let mut c = Core::load(&scene(), 48_000.0, 2).unwrap();
    assert!(c.render(128).iter().all(|x| *x == 0.0), "gain 0: silence");
    // a vector onto the voice from freq: 1500 Hz, ratio 1, no FM, gain 0.5 – plus a component it has no input for
    let mut b = vec![];
    encode_send(&mut b, Mode::Force, 1, 1, 1.0, &[1500.0, 1.0, 0.0, 0.5, 9.0]);
    assert_eq!(c.apply(&b), Ok(1));
    play(&mut c, 40);
    for _ in 0..8 {
        c.analyse(1.0 / 60.0, 0, &Now::default()); // the analyzer rises over a few frames
    }
    let f = c.analyse(1.0 / 60.0, 0, &Now::default()).to_vec();
    assert_eq!(f.len(), FRAME + f[VALUE_COUNT] as usize);
    assert!(f[0] > 0.5, "level {}", f[0]);
    assert!(f[2] > 0.8 && f[2] > f[1] + 0.3, "1500 Hz is mid: {:?}", &f[..4]);
    // the strongest bin: 1500 Hz / 46.875 Hz per bin = 32, give or take the window's leakage into a neighbour
    let peak = (4..4 + SPECTRUM).max_by(|&i, &j| f[i].total_cmp(&f[j])).unwrap() - 4;
    assert!((31..=33).contains(&peak), "1500 Hz is bin 32: {peak}");
    assert_eq!(value(&c, &f, "voice", "freq"), 1500.0, "the heard block's input");
    // release: the gain back to its own 0 → it fades, then silence
    b.clear();
    encode_send(&mut b, Mode::Release, 1, 4, 0.0, &[0.0]);
    c.apply(&b).unwrap();
    assert!(play(&mut c, 2).iter().any(|x| x.abs() > 0.05), "it fades, no click");
    play(&mut c, 200);
    assert!(c.render(128).iter().all(|x| x.abs() < 1e-4), "then silence");
}

/// #188: the output is limited (never over 1, the gain reduction in the analysis frame); a base record turns a linked
/// input under its link
#[test]
fn overload_is_limited_and_bases_turn() {
    let scene = format!(
        "[[node]]\nid = \"a\"\nkind = \"osc\"\nfreq = 110.0\ngain = 1.0\n[[node]]\nid = \"b\"\nkind = \"osc\"\nfreq = 165.0\ngain = 1.0\n\
         [[node]]\nid = \"l\"\nkind = \"lfo\"\nrate = 0.0\ndepth = 1.0\nphase = 0.25\n[[node]]\nid = \"m\"\nkind = \"mul\"\nb = 1.0\n\
         [[node]]\nid = \"mix\"\nkind = \"mixer\"\n{OUT}\
         [[link]]\nfrom = \"l\"\nto = \"m.a\"\nscale = 2.0\n[[link]]\nfrom = \"a\"\nto = \"mix.in1\"\n[[link]]\nfrom = \"b\"\nto = \"mix.in2\"\n\
         [[link]]\nfrom = \"mix\"\nto = \"out.in\"\n"
    );
    let mut c = Core::load(&scene, 48_000.0, 2).unwrap();
    let worst = play(&mut c, 100).iter().fold(0.0f32, |w, x| w.max(x.abs()));
    assert!(worst < 1.0 && worst > 0.8, "two sines at full gain, held under 1: {worst}");
    let f = c.analyse(1.0 / 60.0, 0, &Now::default()).to_vec();
    assert!(f[REDUCTION] > 3.0, "the gain reduction in dB: {}", f[REDUCTION]);
    // the unipolar LFO at its top (phase 0.25 of a still sine = 1) · 2, then a base of 0.5 under the link
    let near = |x: f32, want: f32| assert!((x - want).abs() < 1e-5, "{x} ≠ {want}");
    near(value(&c, &f, "m", "a"), 2.0);
    let mut b = vec![];
    encode_send(&mut b, Mode::Base, 3, 0, 0.0, &[0.5]);
    assert_eq!(c.apply(&b), Ok(1));
    play(&mut c, 2);
    let f = c.analyse(1.0 / 60.0, 0, &Now::default()).to_vec();
    near(value(&c, &f, "m", "a"), 2.5); // the LFO's 1 · 2 on top of the base 0.5
    near(value(&c, &f, "m", "out"), 2.5); // a control output: the heard block's value
}

/// #282: the frame carries the channel count and one peak per channel (since the last frame, then reset)
#[test]
fn the_frame_reports_a_peak_per_channel() {
    let mut c = Core::load(&scene(), 48_000.0, 2).unwrap();
    c.render(128);
    let f = c.analyse(1.0 / 60.0, 0, &Now::default()).to_vec();
    assert_eq!(f[OUTPUT_COUNT], 2.0);
    assert!(f[PEAKS..PEAKS + MAX_OUTPUTS].iter().all(|p| *p == 0.0), "silence: nothing on any channel");
    let mut b = vec![];
    encode_send(&mut b, Mode::Force, 1, 1, 1.0, &[1500.0, 1.0, 0.0, 0.5]);
    c.apply(&b).unwrap();
    play(&mut c, 40);
    let f = c.analyse(1.0 / 60.0, 0, &Now::default()).to_vec();
    let peaks = &f[PEAKS..PEAKS + MAX_OUTPUTS];
    assert!(peaks[0] > 0.3 && peaks[0] <= 1.0, "gain 0.5 on a sine: {peaks:?}");
    assert_eq!(peaks[0], peaks[1], "the stereo main: the same on both");
    assert!(peaks[2..].iter().all(|p| *p == 0.0), "no peaks past the count");
    // #373: the frame carries the place on the dB scale and the zone beside the level; the limiter's bar
    let (level, place, zone) = (f[PEAKS], f[POSITIONS], f[ZONES]);
    let want = deck_core::meters::reading(level);
    assert_eq!((place, zone), (want.position, want.zone as u8 as f32));
    assert!(place > 0.5 && place < 1.0, "a half-scale sine sits high on the scale: {place}");
    assert_eq!(f[LIMITER_BAR], deck_core::meters::limiter_bar(f[REDUCTION]));
    assert!(f[POSITIONS + 2..POSITIONS + MAX_OUTPUTS].iter().chain(&f[ZONES + 2..ZONES + MAX_OUTPUTS]).all(|p| *p == 0.0));
    // nothing rendered since: the held peak falls (×1/e per FALL_SECONDS), it does not jump to 0
    let f = c.analyse(deck_core::meters::FALL_SECONDS, 0, &Now::default()).to_vec();
    let fell = level * std::f32::consts::E.recip();
    assert!((f[PEAKS] - fell).abs() < 1e-5 && (f[PEAKS + 1] - fell).abs() < 1e-5, "held, then falling: {} → {}", level, f[PEAKS]);
    assert!(f[POSITIONS] < place);
}

/// #282: a scene's [outputs] run on the audio clock and their messages leave as signal records
const OUTPUTS_SCENE: &str = "[[node]]
id = \"wob\"
kind = \"lfo\"
rate = 0.0
depth = 1.0
phase = 0.25
[[node]]
id = \"knob\"
kind = \"add\"
a = 0.0
b = 0.0
[outputs]
dmx = \"auto\"
midi = \"x\"
[[outputs.fixture]]
id = \"strobe\"
profile = \"strobe\"
address = 1
[[outputs.map]]
from = \"knob.a\"
to = [{ dmx = 20 }, { dmx = 21 }, { cc = 74, channel = 1 }, { cc = 20, port = \"conductor\" }]
";

/// the records of `bytes`: (kind, flags, ch, at, payload)
fn records(bytes: &[u8]) -> Vec<(u8, u8, u16, u32, Vec<u8>)> {
    let (mut at, mut out) = (0, vec![]);
    while at < bytes.len() {
        let n = u16::from_le_bytes([bytes[at + 2], bytes[at + 3]]) as usize;
        let ch = u16::from_le_bytes([bytes[at + 4], bytes[at + 5]]);
        let ms = u32::from_le_bytes(bytes[at + 8..at + 12].try_into().unwrap());
        out.push((bytes[at], bytes[at + 1], ch, ms, bytes[at + 12..at + 12 + n].to_vec()));
        at += 12 + n.div_ceil(4) * 4;
    }
    out
}

#[test]
fn a_scenes_outputs_leave_as_signal_records() {
    let mut c = Core::load(OUTPUTS_SCENE, 48_000.0, 2).unwrap();
    // a knob forced to 1: node 1 = "knob", port 0 = "a"
    let mut b = vec![];
    encode_send(&mut b, Mode::Force, 1, 0, 1.0, &[1.0]);
    c.apply(&b).unwrap();
    let mut got = vec![];
    for _ in 0..400 {
        c.render(128);
        got.extend_from_slice(c.take_signals());
    }
    assert!(c.take_signals().is_empty(), "taken once");
    let rec = records(&got);
    let range = rec.iter().rev().find(|r| r.0 == 2 && r.2 == 20).expect("DMX 20…21 as one range");
    assert_eq!(range.4, vec![255, 255]);
    let cc = |conductor: u8| rec.iter().rev().find(|r| r.0 == 1 && r.1 == conductor).map(|r| r.4.clone());
    assert_eq!(cc(0), Some(vec![0xB0, 74, 127]), "the deck's MIDI out");
    assert_eq!(cc(1), Some(vec![0xB0, 20, 127]), "the conductor");
    // 400 blocks = 1.07 s: the stamps run on the audio clock, none past it
    assert!(rec.iter().all(|r| r.3 <= 1070), "{:?}", rec.iter().map(|r| r.3).max());
    assert!(rec.windows(2).all(|w| w[0].3 <= w[1].3), "in time order");
    let mut b = vec![];
    encode_send(&mut b, Mode::Force, 1, 0, 1.0, &[0.0]);
    c.apply(&b).unwrap();
    let mut got = vec![];
    for _ in 0..200 {
        c.render(128);
        got.extend_from_slice(c.take_signals());
    }
    assert_eq!(records(&got).iter().rev().find(|r| r.0 == 2 && r.2 == 20).unwrap().4, vec![0, 0]);
    let mut quiet = Core::load(&scene(), 48_000.0, 2).unwrap();
    play(&mut quiet, 50);
    assert!(quiet.take_signals().is_empty(), "a scene without [outputs] sends nothing");
}

#[test]
fn outputs_name_real_paths_or_the_load_fails_listing_them() {
    let bad = OUTPUTS_SCENE.replace("knob.a", "knob.nope");
    let e = Core::load(&bad, 48_000.0, 2).err().unwrap();
    assert!(e.starts_with("the scene's [outputs]:") && e.contains("knob.nope"), "{e}");
    let profile = OUTPUTS_SCENE.replace("profile = \"strobe\"", "profile = \"fixtures/mine.toml\"");
    assert!(Core::load(&profile, 48_000.0, 2).err().unwrap().contains("no profile files"));
}

#[test]
fn bad_records_and_kinds_the_browser_lacks_are_errors() {
    let mut c = Core::load(&scene(), 48_000.0, 2).unwrap();
    let mut b = vec![];
    encode_send(&mut b, Mode::Blend, 7, 0, 0.5, &[1.0]);
    assert_eq!(c.apply(&b), Err("wire: no node 7".into()));
    assert_eq!(c.apply(&[1, 0, 0]), Err("wire: a truncated header at byte 0".into()));
    let music = Core::load("[[node]]\nid = \"m\"\nkind = \"chapter_music\"\n", 48_000.0, 2).err().unwrap();
    assert!(music.contains("does not carry 'chapter_music'"), "{music}");
}

/// #236: a walk plays its voice through the voice's cv – the same seed, the same samples; its knobs are its inputs
const WALK: &str = "[[node]]\nid = \"voice\"\nkind = \"fm\"\nratio = 1.5\nindex = 2.0\ngain = 0.2\n\
    [[node]]\nid = \"melody\"\nkind = \"walk\"\nseed = 42\nbase = 147.0\nspread = 1.0\nrate = 4.0\n\
    [[node]]\nid = \"out\"\nkind = \"audio_out\"\n\
    [[link]]\nfrom = \"melody.cv\"\nto = \"voice.cv\"\n[[link]]\nfrom = \"voice\"\nto = \"out.in\"\n";

#[test]
fn a_walk_plays_its_voice_and_its_knobs_are_inputs() {
    let mut a = Core::load(WALK, 48_000.0, 2).unwrap();
    assert_eq!(table(&a)["nodes"][1]["ports"], serde_json::json!(["spread", "rate", "glide"]));
    let one = play(&mut a, 750);
    assert!(one.iter().any(|x| x.abs() > 0.01), "it sounds");
    assert_eq!(one, play(&mut Core::load(WALK, 48_000.0, 2).unwrap(), 750), "one seed, the same samples");
    // the spread to 0 through a wire record (node 1 = the walk, port 0 = spread): the base held, another sound
    let mut held = Core::load(WALK, 48_000.0, 2).unwrap();
    let mut b = vec![];
    encode_send(&mut b, Mode::Base, 1, 0, 0.0, &[0.0]);
    assert_eq!(held.apply(&b), Ok(1));
    assert_ne!(play(&mut held, 750), one, "spread 0 holds the base");
    // the clock: a scene without a `tempo` node leaves it as it is (clock.rs), one with a node configures it
    assert!(a.tempo_node().is_none());
    let own = Core::load(&format!("{WALK}[[node]]\nid = \"t\"\nkind = \"tempo\"\nbpm = 100.0\nlead = true\n"), 48_000.0, 2).unwrap();
    let c = own.tempo_node().expect("its own clock");
    assert!((c.bpm, c.lead) == (100.0, true));
}

/// #236: an `events` host – an arp on a walk – plays its voice; its knobs a–d are its inputs (NaN = the patch's own)
const EVENTS: &str = "[[node]]\nid = \"voice\"\nkind = \"fm\"\nratio = 2.0\nindex = 1.0\ngain = 0.2\n\
    [[node]]\nid = \"arp\"\nkind = \"events\"\nout = \"arp.notes\"\nknobs = [\"walk.spread\", \"arp.rate\"]\n\
    generator = [{ id = \"walk\", kind = \"random_walk\", seed = 5, rate = 0.5, spread = 0.6 }, \
    { id = \"arp\", kind = \"arp\", ratios = [1.0, 1.2, 1.5], octaves = 2, mode = \"updown\", rate = 4.0 }]\n\
    edge = [{ from = \"walk.notes\", to = \"arp.notes\" }]\n\
    [[node]]\nid = \"out\"\nkind = \"audio_out\"\n\
    [[link]]\nfrom = \"arp.cv\"\nto = \"voice.cv\"\n[[link]]\nfrom = \"voice\"\nto = \"out.in\"\n";

#[test]
fn an_events_host_plays_its_patch_and_its_knobs_are_inputs() {
    let mut a = Core::load(EVENTS, 48_000.0, 2).unwrap();
    assert_eq!(table(&a)["nodes"][1]["ports"], serde_json::json!(["a", "b", "c", "d"]));
    let one = play(&mut a, 750);
    assert!(one.iter().any(|x| x.abs() > 0.01), "it sounds");
    assert_eq!(one, play(&mut Core::load(EVENTS, 48_000.0, 2).unwrap(), 750), "one seed, the same samples");
    // knob b (the arp's rate) through a record; NaN on a (unset) leaves the patch's spread
    let mut fast = Core::load(EVENTS, 48_000.0, 2).unwrap();
    let mut b = vec![];
    encode_send(&mut b, Mode::Base, 1, 0, 0.0, &[f32::NAN, 8.0]);
    assert_eq!(fast.apply(&b), Ok(1));
    let fast = play(&mut fast, 750);
    assert_ne!(fast, one, "knob b: eight steps a beat");
    // a starting value in the node: the same as the record
    let started = EVENTS.replace("kind = \"events\"\n", "kind = \"events\"\nb = 8.0\n");
    assert_eq!(play(&mut Core::load(&started, 48_000.0, 2).unwrap(), 750), fast);
    assert!(Core::load(&EVENTS.replace("updown", "sideways"), 48_000.0, 2).err().is_some_and(|e| e.contains("unknown mode")));
}
