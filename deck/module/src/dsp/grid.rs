//! The beat grid a sequencer plays on: step `k` sits at beat `k / per_beat`, so its number alone says what plays and the
//! pattern is on the grid however the host cuts the blocks. A stopped tempo has no steps.
use crate::ctx::Tempo;

/// the steps of a grid of `per_beat` steps a beat whose beat falls in [from, to): (step, its beat)
pub fn steps(from: f64, to: f64, per_beat: f64) -> impl Iterator<Item = (i64, f64)> {
    let ok = per_beat > 0.0 && to > from;
    let first = if ok { libm::ceil(from * per_beat) as i64 } else { 0 };
    let end = if ok { libm::ceil(to * per_beat) as i64 } else { 0 };
    (first..end).map(move |k| (k, k as f64 / per_beat))
}

/// where `beat` lands in a call of `frames` frames from `frame`: its frame in the call, when its rounded frame on the
/// timeline is one of the call's. Every beat lands in exactly one call, whatever their sizes (a stopped tempo: none).
pub fn frame_in(tempo: &Tempo, frame: f64, frames: usize, beat: f64) -> Option<u32> {
    let at = libm::round(tempo.frame_of(beat)?);
    (at >= frame && at < frame + frames as f64).then_some((at - frame) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempo(bpm: f64, running: bool) -> Tempo {
        Tempo { bpm, rate: 48_000.0, running, per_bar: 4, anchor_frame: 0.0, anchor_beat: 0.0 }
    }

    #[test]
    fn steps_cover_a_span_once() {
        let a: Vec<i64> = steps(0.0, 1.0, 4.0).map(|(k, _)| k).collect();
        let b: Vec<i64> = steps(0.0, 0.3, 4.0).chain(steps(0.3, 1.0, 4.0)).map(|(k, _)| k).collect();
        assert_eq!(a, [0, 1, 2, 3]);
        assert_eq!(a, b, "cut anywhere, the same steps");
        assert_eq!(steps(1.0, 1.0, 4.0).count(), 0, "a stopped tempo has no steps");
    }

    #[test]
    fn a_beat_lands_in_one_call_whatever_the_cut() {
        let t = tempo(160.0, true);
        // 160 BPM at 48 kHz: 18 000 frames a beat; beat 1/3 is frame 6000, 1/7 is 2571.43 → 2571
        for beat in [0.0, 1.0 / 3.0, 1.0 / 7.0, 2.5] {
            for size in [1usize, 37, 128] {
                let hits: Vec<u64> = (0..50_000 / size)
                    .filter_map(|c| frame_in(&t, (c * size) as f64, size, beat).map(|f| (c * size) as u64 + u64::from(f)))
                    .collect();
                assert_eq!(hits, [libm::round(beat * 18_000.0) as u64], "beat {beat}, calls of {size}");
            }
        }
        assert_eq!(frame_in(&tempo(160.0, false), 0.0, 128, 0.0), None, "stopped: nowhere");
    }
}
