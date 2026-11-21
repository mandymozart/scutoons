//! The golden render: the script's envelopes – audio and control – hashed. A change of the hash is a change of every
//! sound an envelope shapes.
mod common;
use common::*;
use deck_module::Ev;

const GOLDEN: u64 = 0x6f18_9084_a911_0268;

/// `frames` frames in calls of 128, every sample of `out`; the gate rises at frame 0 and falls at `off` (if any)
fn play(b: &mut Bench<Env>, frames: usize, off: Option<usize>) -> Vec<f32> {
    let mut all = vec![];
    for at in (0..frames).step_by(128) {
        let mut events = vec![];
        if at == 0 {
            events.push(Ev::note_on(0, 1, 220.0, 57.0, 0.5));
        }
        if let Some(off) = off.filter(|o| (at..at + 128).contains(o)) {
            events.push(Ev::note_off((off - at) as u32, 1));
        }
        b.rig.set_events_in(Env::GATE, &events);
        b.run(128);
        let out = b.rig.audio_out(Env::OUT).chan(0).to_vec();
        assert_eq!(b.rig.control_out(Env::ENV), (out[127], true), "the control output is the call's last sample");
        all.extend(out);
    }
    all
}

#[test]
fn the_render_is_the_golden_one() {
    let mut b = bench(1);
    let mut h = Hash::new();
    let peak = (0..800).map(|k| step(&mut b, k, &mut h)).fold(0.0f32, f32::max);
    assert!(peak > 0.9 && peak <= 1.0, "{peak}");
    assert_eq!(h.0, GOLDEN, "the render changed: {:#018x}", h.0);
}

#[test]
fn adsr_holds_its_sustain_and_releases_at_the_gates_end() {
    for curve in [0.0, 1.0] {
        let mut b = bench(1);
        b.set("curve", curve).set("a", 0.01).set("d", 0.02).set("s", 50.0).set("r", 0.04);
        let x = play(&mut b, 9600, Some(4800));
        assert!(x[479] == 1.0 && x[..480].windows(2).all(|w| w[1] > w[0]), "the attack is a line to the top");
        assert_eq!((x[480 + 960], x[4799]), (0.5, 0.5), "curve {curve}: the sustain");
        assert!(x[4800] < 0.5 && x[4800 + 1920 - 2] > 0.0 && x[4800 + 1920] == 0.0, "curve {curve}: the release takes its time");
        // half-way down the decay: a line is half-way, the exponential nearly there
        let mid = x[480 + 480 - 1];
        assert!(if curve == 0.0 { (mid - 0.75).abs() < 0.002 } else { mid < 0.53 }, "curve {curve}: {mid}");
    }
}

#[test]
fn ad_is_a_one_shot_and_ar_holds_the_top() {
    let mut b = bench(1);
    b.rig.apply_preset();
    b.set("velocity", 0.0);
    let x = play(&mut b, 24_000, Some(100));
    assert_eq!(x[47], 1.0, "the gate fell during the decay: it plays on");
    assert!(x[6000] > 0.0 && x[48 + 12_000] == 0.0 && x[23_999] == 0.0, "down in its decay time, then nothing");
    // it is over: another mode does not bring it back up without a gate
    b.set("mode", 0.0);
    b.run(128);
    assert_eq!(b.rig.control_out(Env::ENV).0, 0.0);
    // a held adsr at a sustain of zero is not over: it follows the knob
    let mut b = bench(1);
    b.set("s", 0.0).set("a", 0.001).set("d", 0.01);
    assert_eq!(play(&mut b, 4800, None)[4799], 0.0);
    b.set("s", 50.0);
    b.run(128);
    assert_eq!(b.rig.control_out(Env::ENV).0, 0.5);
    let mut b = bench(1);
    b.set("mode", 2.0).set("a", 0.01).set("r", 0.01).set("s", 20.0);
    let x = play(&mut b, 9600, Some(4800));
    assert!(x[480..4800].iter().all(|v| *v == 1.0) && x[4800 + 480] == 0.0);
}

#[test]
fn velocity_scales_it_and_a_second_note_keeps_the_gate() {
    let mut b = bench(1);
    b.set("velocity", 100.0).set("a", 0.001).set("s", 100.0);
    let x = play(&mut b, 4800, None);
    assert_eq!(x[4799], 0.5, "velocity 0.5 at 100 %");
    // a second note takes the gate: the first one's off does not release
    b.rig.set_events_in(Env::GATE, &[Ev::note_on(0, 2, 220.0, 57.0, 1.0), Ev::note_off(10, 1)]);
    for _ in 0..40 {
        b.run(128);
    }
    assert_eq!(b.rig.control_out(Env::ENV).0, 1.0);
    b.rig.set_events_in(Env::GATE, &[Ev::reset(0)]);
    for _ in 0..200 {
        b.run(128);
    }
    assert_eq!(b.rig.control_out(Env::ENV).0, 0.0, "a reset lowers the gate");
}
