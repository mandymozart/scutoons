//! The DSP units, one copy for modules and the engine. Every unit is plain data: built in `init` (the ones with a buffer
//! allocate there), then nothing allocates, reads a clock or calls the platform's maths – a render is the same bits
//! native and wasm.
//!
//! | unit | what |
//! |---|---|
//! | `osc` | PolyBLEP saw / pulse / triangle / sine, a sub, hard sync |
//! | `wavetable` | mip-mapped saw / square / triangle |
//! | `noise` | seeded white and pink |
//! | `svf` | state-variable filter: LP / BP / HP / notch, drive |
//! | `onepole` | 6 dB lowpass / highpass |
//! | `adsr` | the envelope |
//! | `line` | the envelope with straight segments |
//! | `follower` | envelope follower: attack, release |
//! | `level` | decibels as a factor, silent at the bottom |
//! | `delay` | delay line (whole and cubic fractional reads), allpass, damped comb |
//! | `fft` | radix-2 FFT of real input, and back |
//! | `arith` | f64 maths from arithmetic only |
pub mod adsr;
pub mod arith;
pub mod delay;
pub mod fft;
pub mod follower;
pub mod level;
pub mod line;
pub mod noise;
pub mod onepole;
pub mod osc;
pub mod svf;
pub mod wavetable;

/// A feedback state this small becomes 0: wasm has no flush-to-zero, and a decaying state would stay in the denormals.
#[inline(always)]
pub fn flush(x: f32) -> f32 {
    if x.abs() < 1e-20 {
        0.0
    } else {
        x
    }
}

/// the tests' measuring instrument: an f64 FFT, apart from the unit under test
#[cfg(test)]
pub(crate) mod probe {
    /// in place, a power-of-two length
    pub fn fft(re: &mut [f64], im: &mut [f64]) {
        let n = re.len();
        assert!(n.is_power_of_two() && im.len() == n);
        let mut j = 0;
        for i in 1..n {
            let mut bit = n >> 1;
            while j & bit != 0 {
                j ^= bit;
                bit >>= 1;
            }
            j ^= bit;
            if i < j {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        let mut len = 2;
        while len <= n {
            let ang = -core::f64::consts::TAU / len as f64;
            for start in (0..n).step_by(len) {
                for k in 0..len / 2 {
                    let (wr, wi) = ((ang * k as f64).cos(), (ang * k as f64).sin());
                    let (a, b) = (start + k, start + k + len / 2);
                    let (tr, ti) = (re[b] * wr - im[b] * wi, re[b] * wi + im[b] * wr);
                    (re[b], im[b]) = (re[a] - tr, im[a] - ti);
                    (re[a], im[a]) = (re[a] + tr, im[a] + ti);
                }
            }
            len <<= 1;
        }
    }

    /// the magnitude spectrum (bins 0..n/2), scaled so a full-scale sine's bin reads its amplitude; no window: use it
    /// on coherent signals
    pub fn spectrum(x: &[f32]) -> Vec<f64> {
        let n = x.len();
        let mut re: Vec<f64> = x.iter().map(|&v| f64::from(v)).collect();
        let mut im = vec![0.0f64; n];
        fft(&mut re, &mut im);
        (0..n / 2).map(|i| (re[i] * re[i] + im[i] * im[i]).sqrt() * 2.0 / n as f64).collect()
    }

    pub fn db(x: f64) -> f64 {
        20.0 * x.max(1e-30).log10()
    }
}

#[cfg(test)]
mod tests {
    use super::flush;

    #[test]
    fn flush_zeroes_only_what_nobody_hears() {
        assert_eq!(flush(1e-21), 0.0);
        assert_eq!(flush(-1e-21), 0.0);
        assert_eq!(flush(f32::MIN_POSITIVE / 2.0), 0.0);
        assert_eq!(flush(1e-19), 1e-19);
        assert_eq!(flush(-0.5), -0.5);
    }
}
