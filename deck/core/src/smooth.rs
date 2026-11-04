//! Control values into audio without zipper noise (#188, owner 2026-10-31: "no crackling … parameter changes must be
//! smoothed in the audio engine"). An audio block reads its control inputs once; held flat, every change would jump at
//! the block's edge (375 times a second at 128 frames) – a knob's drag, a feeling's nudge, an LFO's link all click.
//! `Ramp` is a one-pole smoother stepped once per block whose output is spread linearly across the block: sample `i`
//! of a block is `from + step · i`, and the next block starts where this one was heading – continuous at every block
//! edge, a value written once per frame glides in over a few time constants. Native (deck-native's voices and blocks)
//! and the browser (live-core, the same engine) use the same one. Never allocates.

/// how long a gain or a frequency glides (s): a step in either is what clicks loudest
pub const GLIDE: f32 = 0.025;
/// the other inputs (FM ratio and index, a filter's cutoff …) (s)
pub const QUICK: f32 = 0.012;

/// One control input's smoother: where it is, and its rate (1 / the time constant in samples).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ramp {
    cur: f32,
    per: f32,
}

impl Ramp {
    /// starting at `value` (no glide at the start), time constant `tau` s at `rate` Hz (`tau` 0 = no smoothing: the
    /// block still ramps linearly from the last value to the new one)
    pub fn new(value: f32, tau: f32, rate: f32) -> Ramp {
        let per = if tau > 0.0 && rate > 0.0 { 1.0 / (tau * rate) } else { f32::INFINITY };
        Ramp { cur: if value.is_finite() { value } else { 0.0 }, per }
    }

    /// The next block of `n` samples toward `target`: (the first sample's value, the step per sample). The block ends
    /// one step short of where the next begins, so the steps run on across the edge.
    #[inline]
    pub fn block(&mut self, target: f32, n: usize) -> (f32, f32) {
        let from = self.cur;
        if n == 0 || !target.is_finite() {
            return (from, 0.0);
        }
        let k = 1.0 - libm::expf(-(n as f32) * self.per);
        let mut to = from + (target - from) * k;
        if (to - target).abs() <= 1e-6 * (1.0 + target.abs()) || !to.is_finite() {
            to = target;
        }
        self.cur = to;
        (from, (to - from) / n as f32)
    }

    /// where it will start the next block
    pub fn value(&self) -> f32 {
        self.cur
    }

    /// there at once (a voice reset, a new scene)
    pub fn jump(&mut self, value: f32) {
        if value.is_finite() {
            self.cur = value;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// samples of a block from `Ramp::block`
    fn run(r: &mut Ramp, target: f32, n: usize, out: &mut Vec<f32>) {
        let (from, step) = r.block(target, n);
        out.extend((0..n).map(|i| from + step * i as f32));
    }

    /// owner 2026-10-31: a jump of a knob never jumps in the sound – no step between samples larger than the glide's
    #[test]
    fn a_jump_glides_without_a_step_at_the_block_edges() {
        let mut r = Ramp::new(0.0, GLIDE, 48_000.0);
        let mut x = Vec::new();
        for _ in 0..4 {
            run(&mut r, 0.0, 128, &mut x);
        }
        // the frame thread jumps the value from 0 to 1 (a gain from silence to full)
        for _ in 0..80 {
            run(&mut r, 1.0, 128, &mut x);
        }
        let worst = x.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f32, f32::max);
        // the steepest block: k = 1 − e^(−128 / 1200) ≈ 0.101 of the jump over 128 samples ≈ 7.9e-4 per sample
        assert!(worst < 1e-3, "the largest step between samples {worst}");
        assert!((x[x.len() - 1] - 1.0).abs() < 1e-3, "settled after 80 blocks (213 ms = 8.5 τ): {}", x[x.len() - 1]);
        // the steps run on across a block's edge: the last step of a block is the step into the next
        let edge = 4 * 128 + 128;
        assert!(((x[edge] - x[edge - 1]) - (x[edge - 1] - x[edge - 2])).abs() < 1e-3);
    }

    #[test]
    fn a_held_value_stays_put_and_odd_blocks_work() {
        let mut r = Ramp::new(0.25, QUICK, 48_000.0);
        assert_eq!(r.block(0.25, 128), (0.25, 0.0));
        assert_eq!(r.block(0.25, 0), (0.25, 0.0));
        let (_, s) = r.block(0.75, 37);
        assert!(s > 0.0 && r.value() < 0.75);
        // tau 0: the whole way in one block, linearly
        let mut z = Ramp::new(0.0, 0.0, 48_000.0);
        assert_eq!(z.block(1.0, 4), (0.0, 0.25));
        assert_eq!(z.value(), 1.0);
        // never NaN
        assert_eq!(z.block(f32::NAN, 4), (1.0, 0.0));
        z.jump(3.0);
        assert_eq!(z.value(), 3.0);
    }
}
