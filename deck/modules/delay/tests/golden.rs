//! The golden render: the script through the delay, hashed. A change of the hash is a change of the sound. And what
//! the delay does, measured: the echoes' sample positions on the beat grid, ping-pong, the chorus, the top of the
//! feedback.
mod common;
use common::*;
use deck_module::dsp::arith::sin_turns;

const GOLDEN: u64 = 0xc572_8f46_19a8_def6;

/// `len` samples of both output channels, the input `x(i)` on both inputs, in calls of `block` frames
fn render(b: &mut Bench<Delay>, len: usize, block: usize, x: impl Fn(usize) -> f32) -> (Vec<f32>, Vec<f32>) {
    let (mut l, mut r) = (vec![], vec![]);
    while l.len() < len {
        let at = l.len();
        let input: Vec<f32> = (at..at + block).map(&x).collect();
        b.rig.set_audio_in(Delay::IN, 0, &input);
        b.rig.set_audio_in(Delay::IN, 1, &input);
        b.run(block);
        let out = b.rig.audio_out(Delay::OUT);
        l.extend_from_slice(out.chan(0));
        r.extend_from_slice(out.chan(1));
    }
    (l[..len].to_vec(), r[..len].to_vec())
}

fn click(i: usize) -> f32 {
    if i == 0 {
        1.0
    } else {
        0.0
    }
}

/// the samples that are not silent, (position, value)
fn heard(x: &[f32]) -> Vec<(usize, f32)> {
    x.iter().enumerate().filter(|(_, v)| v.abs() > 1e-6).map(|(i, v)| (i, *v)).collect()
}

/// the frequency between each two upward zero crossings (interpolated), from sample `from` on
fn frequencies(x: &[f32], from: usize) -> Vec<f64> {
    let ups: Vec<f64> = (from + 1..x.len()).filter(|&i| x[i - 1] < 0.0 && x[i] >= 0.0).map(|i| (i - 1) as f64 + f64::from(x[i - 1] / (x[i - 1] - x[i]))).collect();
    ups.windows(2).map(|w| RATE / (w[1] - w[0])).collect()
}

#[test]
fn the_render_is_the_golden_one() {
    let mut b = bench(1);
    let mut h = Hash::new();
    let peak = (0..900).map(|k| step(&mut b, k, &mut h)).fold(0.0f32, f32::max);
    assert!(peak > 0.1 && peak < 4.0, "{peak}");
    assert_eq!(h.0, GOLDEN, "the render changed: {:#018x}", h.0);
}

#[test]
fn a_click_echoes_on_dotted_eighths_at_160_bpm() {
    let mut b = bench(1);
    b.rig.set_tempo(0.0, 0.0, 160.0, 4, true);
    b.set("sync", 8.0).set("feedback", 50.0).set("damp", 20_000.0).set("mix", 100.0);
    // a dotted eighth at 160 BPM: 0.75 × 60 / 160 s = 281.25 ms = 13 500 samples at 48 kHz
    let (l, r) = render(&mut b, 13_500 * 5 + 100, 128, click);
    let want: Vec<(usize, f32)> = (1..=5).map(|k| (13_500 * k, 0.5f32.powi(k as i32 - 1))).collect();
    assert_eq!(heard(&l), want, "the echoes, each half the last, and silence between them");
    assert_eq!(heard(&r), want);

    // ping-pong: the first echo left, the second right, the third left again
    let mut b = bench(1);
    b.rig.set_tempo(0.0, 0.0, 160.0, 4, true);
    b.set("sync", 8.0).set("feedback", 50.0).set("damp", 20_000.0).set("mix", 100.0).set("pingpong", 100.0);
    let (l, r) = render(&mut b, 13_500 * 4 + 100, 128, click);
    assert_eq!(heard(&l), [(13_500, 1.0), (40_500, 0.25)]);
    assert_eq!(heard(&r), [(27_000, 0.5), (54_000, 0.125)]);

    // free: 100 ms, an octave longer by the control input
    let mut b = bench(1);
    b.set("time", 100.0).set("feedback", 0.0).set("mix", 100.0);
    b.rig.set_control_in(Delay::TIME_MOD, 1.0);
    let (l, _) = render(&mut b, 12_000, 128, click);
    assert_eq!(heard(&l), [(9600, 1.0)]);
}

#[test]
fn modulated_at_7_ms_it_choruses() {
    let sine = |i: usize| sin_turns(440.0 * i as f64 / RATE) as f32;
    let chorus = |depth: f32, mix: f32| {
        let mut b = bench(1);
        b.set("time", 7.0).set("feedback", 0.0).set("depth", depth).set("rate", 1.0).set("mix", mix);
        render(&mut b, 2 * 48_000 + 4800, 128, sine).0
    };
    // the echo alone: its pitch swings by 440 Hz × the time's speed, 2 ms × 2π × 1 Hz – ± 5.53 Hz (± 22 cents)
    let f = frequencies(&chorus(2.0, 100.0), 4800);
    let (lo, hi) = f.iter().fold((f64::MAX, f64::MIN), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    let want = 440.0 * 0.002 * core::f64::consts::TAU;
    assert!(((hi - lo) / 2.0 - want).abs() < 0.1 * want, "the swing: {lo:.2} … {hi:.2} Hz, want ± {want:.2}");
    assert!(((hi + lo) / 2.0 - 440.0).abs() < 0.3, "around the note: {lo:.2} … {hi:.2}");
    let f = frequencies(&chorus(0.0, 100.0), 4800);
    assert!(f.iter().all(|v| (v - 440.0).abs() < 0.05), "unmodulated it stays on the note");
    // with the dry signal the moving comb comes and goes: the level of the sum swings from near cancellation to double
    let mixed = chorus(2.0, 50.0);
    let levels: Vec<f32> = mixed[4800..].chunks(480).map(|c| c.iter().fold(0.0f32, |m, v| m.max(v.abs()))).collect();
    let (lo, hi) = levels.iter().fold((f32::MAX, 0.0f32), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
    assert!(lo < 0.2 && hi > 0.9, "the sum's level swings: {lo:.3} … {hi:.3}");
}

#[test]
fn at_the_top_of_the_feedback_it_stays_bounded_and_the_tail_ends() {
    for pingpong in [0.0, 100.0] {
        let mut b = bench(1);
        b.set("time", 30.0).set("feedback", 100.0).set("damp", 20_000.0).set("mix", 100.0).set("pingpong", pingpong).set("depth", 15.0).set("rate", 7.0);
        let mut peak = 0.0f32;
        let mut tail = vec![];
        for call in 0..(20 * 48_000 / 128) {
            let level = if call < 10 * 48_000 / 128 { 0.5 } else { 0.0 };
            b.feed(Delay::IN, 0, 128, level);
            b.feed(Delay::IN, 1, 128, level);
            b.run(128);
            let out = b.rig.audio_out(Delay::OUT);
            for v in out.chan(0).iter().chain(out.chan(1)) {
                assert!(v.is_finite(), "pingpong {pingpong}: call {call}");
                peak = peak.max(v.abs());
            }
            if level == 0.0 {
                tail.push(out.chan(0).iter().map(|v| v * v).sum::<f32>());
            }
        }
        assert!(peak > 0.5 && peak < 3.0, "pingpong {pingpong}: the loop saturates instead of growing: {peak}");
        let (first, last) = (tail[..375].iter().sum::<f32>(), tail[tail.len() - 375..].iter().sum::<f32>());
        assert!(last < first, "pingpong {pingpong}: the tail fades: {first} then {last}");
        // the feedback down: after the line's length the output is exactly silent
        b.set("feedback", 0.0);
        let (l, r) = render(&mut b, 5 * 48_000, 128, |_| 0.0);
        assert!(l[4 * 48_000..].iter().chain(&r[4 * 48_000..]).all(|v| *v == 0.0), "pingpong {pingpong}: silent in the end");
    }
}

#[test]
fn the_cut_of_the_blocks_changes_nothing() {
    let run = |block: usize| {
        let mut b = bench(1);
        b.rig.set_tempo(0.0, 0.0, 132.0, 4, true);
        b.set("sync", 6.0).set("feedback", 70.0).set("damp", 3000.0).set("pingpong", 60.0).set("spread", 9.0).set("depth", 3.0).set("rate", 2.0);
        render(&mut b, 48_000, block, |i| if i < 4800 { sin_turns(i as f64 * 0.0137) as f32 * 0.5 } else { 0.0 })
    };
    assert!(run(128) == run(37), "the same samples in calls of 128 and of 37 frames");
}
