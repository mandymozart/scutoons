//! The script both tests play – notes in every mode over a stereo sound at 44.1 kHz, the knobs turned – and the
//! helpers: a sound, the sampler with it in its slot, notes played through it.
#![allow(dead_code, unused_imports)]
pub use deck_module::golden::{Bench, Hash};
pub use sampler_module::Sampler;
use deck_module::dsp::noise::White;
use deck_module::{AssetKind, Ev};

pub const RATE: f64 = 48_000.0;

/// `channels` sides of `frames` of seeded noise falling away, a side each
pub fn sound(channels: usize, frames: usize) -> Vec<Vec<f32>> {
    (0..channels as u64)
        .map(|seed| {
            let mut w = White::new(seed + 11);
            (0..frames).map(|i| w.sample() * 0.8 * (1.0 - i as f32 / frames as f32)).collect()
        })
        .collect()
}

/// the sampler with `sound` (planar sides at `rate`) in its slot, a stereo cable out
pub fn bench_with(seed: u64, sound: &[Vec<f32>], rate: f32) -> Bench<Sampler> {
    let frames = sound[0].len();
    let bytes: Vec<u8> = sound.iter().flat_map(|c| c.iter().flat_map(|v| v.to_le_bytes())).collect();
    let mut b = Bench::with(RATE, seed, |rig| rig.load_asset(Sampler::SAMPLE, AssetKind::Pcm, &bytes, sound.len() as u8, frames as u32, rate));
    b.rig.set_channels(false, Sampler::OUT, 2);
    b
}

/// the sampler with nothing in its slot
pub fn bench_empty(seed: u64) -> Bench<Sampler> {
    let mut b = Bench::new(RATE, seed);
    b.rig.set_channels(false, Sampler::OUT, 2);
    b
}

/// `frames` of the sampler in calls of `block`, `events` (at absolute frames) given to the call each falls in; both sides
pub fn play(b: &mut Bench<Sampler>, frames: usize, block: usize, events: &[(usize, Ev)]) -> [Vec<f32>; 2] {
    let mut out = [Vec::with_capacity(frames), Vec::with_capacity(frames)];
    let mut at = 0;
    while at < frames {
        let n = block.min(frames - at);
        let now: Vec<Ev> = events.iter().filter(|(f, _)| (at..at + n).contains(f)).map(|(f, e)| Ev { frame: (f - at) as u32, ..*e }).collect();
        b.rig.set_events_in(Sampler::NOTES, &now);
        b.run(n);
        let o = b.rig.audio_out(Sampler::OUT);
        for (c, side) in out.iter_mut().enumerate() {
            side.extend_from_slice(o.chan(c));
        }
        at += n;
    }
    out
}

/// a note on at velocity 1
pub fn on(id: u32, key: f32) -> Ev {
    Ev::note_on(0, id, f32::NAN, key, 1.0)
}

pub fn off(id: u32) -> Ev {
    Ev::note_off(0, id)
}

/// step `k` of the script; the call's peak
pub fn step(b: &mut Bench<Sampler>, k: usize, h: &mut Hash) -> f32 {
    let n = 16 + (k % 8) * 16;
    b.set("mode", (k / 120 % 3) as f32).set("pitch", (k / 360 % 2) as f32).set("tune", (k / 50 % 5) as f32 * 3.0 - 6.0);
    b.set("gain", -12.0 + (k / 70 % 4) as f32 * 4.0).set("start", (k / 90 % 3) as f32 * 10.0).set("length", 40.0 + (k / 110 % 4) as f32 * 20.0);
    b.set("release", 0.01 + (k / 130 % 3) as f32 * 0.1).set("voices", 1.0 + (k / 80 % 8) as f32);
    let mut events = vec![];
    if k % 7 == 0 {
        events.push(Ev::note_on((k % 5) as u32, k as u32, f32::NAN, 48.0 + (k % 25) as f32, 0.3 + (k % 7) as f32 * 0.1));
    }
    if k % 7 == 4 && k >= 7 {
        events.push(Ev::note_off(3, (k - 4) as u32));
    }
    if k % 11 == 5 {
        events.push(Ev::note_expr(1, (k - k % 7) as u32, deck_module::Expr::Bend, (k % 5) as f32 - 2.0));
    }
    if k % 400 == 399 {
        events.push(Ev::reset(2));
    }
    b.rig.set_events_in(Sampler::NOTES, &events);
    b.run(n);
    let out = b.rig.audio_out(Sampler::OUT);
    h.f32s(out.chan(0));
    h.f32s(out.chan(1));
    out.chan(0).iter().chain(out.chan(1)).fold(0.0, |m, v| m.max(v.abs()))
}
