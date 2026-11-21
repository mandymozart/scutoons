//! The script both tests play: stereo noise through every mode, the cutoff swept by the knob, the control input and
//! key-tracked notes, the resonance and the drive turned.
#![allow(dead_code)]
pub use deck_module::golden::{Bench, Hash};
use deck_module::Ev;
pub use filter_module::Filter;

pub const RATE: f64 = 48_000.0;

/// a stereo cable in and out
pub fn bench(seed: u64) -> Bench<Filter> {
    let mut b = Bench::new(RATE, seed);
    b.rig.set_channels(true, Filter::IN, 2);
    b.rig.set_channels(false, Filter::OUT, 2);
    b
}

/// step `k` of the script; the call's peak
pub fn step(b: &mut Bench<Filter>, k: usize, h: &mut Hash) -> f32 {
    let n = 16 + (k % 8) * 16;
    b.set("mode", (k / 50 % 5) as f32).set("cutoff", 100.0 * (1 << (k % 7)) as f32).set("res", (k % 10) as f32 * 10.0);
    b.set("drive", (k / 20 % 4) as f32 * 30.0).set("key", (k / 35 % 3) as f32 * 50.0);
    b.feed(Filter::IN, 0, n, 0.5);
    b.feed(Filter::IN, 1, n, 0.5);
    b.rig.set_control_in(Filter::CUTOFF_MOD, ((k % 9) as f32 - 4.0) * 0.25);
    let note = [Ev::note_on(n as u32 / 3, k as u32, 110.0 * (1 + k % 5) as f32, f32::NAN, 1.0)];
    b.rig.set_events_in(Filter::NOTES, if k % 16 == 0 { &note } else { &[] });
    b.run(n);
    let out = b.rig.audio_out(Filter::OUT);
    h.f32s(out.chan(0));
    h.f32s(out.chan(1));
    out.chan(0).iter().chain(out.chan(1)).fold(0.0, |m, v| m.max(v.abs()))
}
