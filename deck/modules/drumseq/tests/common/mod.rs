//! The script both tests play: the groove through calls of every size while the cells, the swing, the rate, the length
//! and the tempo move, the transport stopping and starting and jumping.
#![allow(dead_code)]
pub use deck_module::golden::{Bench, Hash};
pub use drumseq_module::{DrumSeq, LANES};
use deck_module::Ev;

pub const RATE: f64 = 48_000.0;

pub fn bench(seed: u64) -> Bench<DrumSeq> {
    Bench::new(RATE, seed)
}

/// what the module wrote, into the hash (an absent value as that, not as its NaN's bits)
pub fn hash(h: &mut Hash, lane: usize, events: &[Ev]) {
    for e in events {
        h.u32s(&[lane as u32, e.frame, u32::from(e.kind), e.id]);
        h.f32s(&[e.len]);
        for v in e.v {
            h.u32s(&[u32::from(v.is_nan())]);
            h.f32s(&[if v.is_nan() { 0.0 } else { v }]);
        }
    }
}

/// step `k` of the script; how many notes left the module
pub fn step(b: &mut Bench<DrumSeq>, k: usize, h: &mut Hash) -> f32 {
    let n = 16 + (k % 8) * 16;
    if k % 9 == 0 {
        b.set(&format!("l{}s{}", 1 + k / 9 % LANES, 1 + k / 3 % 16), (k / 9 % 3) as f32);
    }
    b.set("swing", [0.0, 50.0, 100.0][k / 120 % 3]).set("rate", [4.0, 3.0, 2.0, 6.0][k / 200 % 4]);
    b.set("steps", [16.0, 12.0, 7.0][k / 150 % 3]).set("accent", [100.0, 90.0][k / 70 % 2]);
    // 160 BPM running, a jump to beat 32 at call 500, a stop at 700 and a start at 760, then 96 BPM
    let frame = b.frame as f64;
    match k {
        0 => b.rig.set_tempo(0.0, 0.0, 160.0, 4, true),
        500 => b.rig.set_tempo(frame, 32.0, 160.0, 4, true),
        700 => b.rig.set_tempo(frame, 40.25, 160.0, 4, false),
        760 => b.rig.set_tempo(frame, 40.25, 96.0, 4, true),
        _ => {}
    }
    b.run(n);
    let mut count = 0;
    for lane in 0..LANES {
        let out = b.rig.events_out(DrumSeq::LANE1 + lane);
        hash(h, lane, &out);
        count += out.len();
    }
    h.f32s(&[b.rig.control_out(DrumSeq::STEP).0]);
    count as f32
}
