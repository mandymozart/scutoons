//! Band-limited oscillators: saw, pulse (with pulse width), triangle and sine; a square sub-oscillator one octave down;
//! hard sync. PolyBLEP (two samples) takes the aliasing out of the saw's and pulse's steps and, as PolyBLAMP, out of the
//! triangle's corners (ENGINE-REFACTOR-PLAN §3.1: PolyBLEP is cheap and plain; the wavetable is cleaner, see
//! ENGINE-ALIGNMENT §2a (d)).
//!
//! A step of height Δ at time x = 0 (x in samples, negative before it) is corrected by Δ/2 · B(x), B(x) = (1 + x)² for
//! x in [−1, 0) and −(1 − x)² for x in [0, 1). A kink whose slope changes by s a sample, by s · L(x) with L(x) =
//! (1 + x)³/6 before and (1 − x)³/6 after. A hard sync is a step that cannot be seen a sample ahead, so every
//! oscillator here **delays its output by one sample**: the half of the correction that belongs to the sample before the
//! event is added when that sample leaves.
use super::math::sin_turns;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wave {
    Saw,
    Pulse,
    Tri,
    Sine,
}

impl Wave {
    /// from a parameter value: 0 saw, 1 pulse, 2 triangle, 3 sine
    pub fn from_param(v: f32) -> Wave {
        match v.round() as i32 {
            0 => Wave::Saw,
            1 => Wave::Pulse,
            2 => Wave::Tri,
            _ => Wave::Sine,
        }
    }
}

/// the aliasing-free waveform at phase `t` (cycles): saw rises from −1, pulse is +1 until `pw`, triangle and sine start at
/// 0 going up
fn naive(wave: Wave, t: f64, pw: f64) -> f64 {
    match wave {
        Wave::Saw => 2.0 * t - 1.0,
        Wave::Pulse => {
            if t < pw {
                1.0
            } else {
                -1.0
            }
        }
        Wave::Tri => {
            let u = t + 0.25;
            1.0 - 4.0 * (u - u.floor() - 0.5).abs()
        }
        Wave::Sine => sin_turns(t),
    }
}

/// the signed distance in cycles from the event at phase `c` to phase `p`, in (−0.5, 0.5]
fn dist(p: f64, c: f64) -> f64 {
    let d = p - c;
    d - (d + 0.5).floor()
}

fn blep(x: f64) -> f64 {
    if (-1.0..0.0).contains(&x) {
        (1.0 + x) * (1.0 + x)
    } else if (0.0..1.0).contains(&x) {
        -(1.0 - x) * (1.0 - x)
    } else {
        0.0
    }
}

fn blamp(x: f64) -> f64 {
    if (-1.0..0.0).contains(&x) {
        (1.0 + x) * (1.0 + x) * (1.0 + x) / 6.0
    } else if (0.0..1.0).contains(&x) {
        (1.0 - x) * (1.0 - x) * (1.0 - x) / 6.0
    } else {
        0.0
    }
}

/// the correction for the waveform's own steps and kinks at phase `p`; `skip_wrap` leaves out the step at phase 0 (a
/// sync reset replaces it)
fn correction(wave: Wave, p: f64, dt: f64, pw: f64, skip_wrap: bool) -> f64 {
    let mut c = 0.0;
    match wave {
        Wave::Saw => {
            if !skip_wrap {
                c -= blep(dist(p, 0.0) / dt); // −2 / 2
            }
        }
        Wave::Pulse => {
            if !skip_wrap {
                c += blep(dist(p, 0.0) / dt); // +2 / 2
            }
            c -= blep(dist(p, pw) / dt);
        }
        Wave::Tri => {
            // slope 4 → −4 at a quarter, −4 → 4 at three quarters (per cycle): ±8 · dt a sample
            c -= 8.0 * dt * blamp(dist(p, 0.25) / dt);
            c += 8.0 * dt * blamp(dist(p, 0.75) / dt);
        }
        Wave::Sine => {}
    }
    c
}

/// What one sample of an oscillator gives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tick {
    /// the output, one sample late
    pub value: f32,
    /// the phase wrapped (or was reset by a sync) during this sample, this fraction of a sample ago
    pub wrap: Option<f64>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Osc {
    phase: f64,
    prev: f32,
}

impl Osc {
    pub fn reset(&mut self) {
        *self = Osc::default();
    }

    /// One sample. `dt` = frequency / rate (cycles a sample), `pw` the pulse width, `reset` = a master oscillator
    /// wrapped this fraction of a sample ago and restarts this one (hard sync).
    pub fn tick(&mut self, wave: Wave, dt: f64, pw: f64, reset: Option<f64>) -> Tick {
        let dt = dt.clamp(1e-7, 0.45);
        let pw = pw.clamp(0.02, 0.98);
        let mut p = self.phase + dt;
        let mut wrap = None;
        let (mut jump, mut at) = (0.0, 0.0);
        if let Some(f) = reset {
            let before = p - dt * f; // the phase at the instant of the reset
            let before = before - before.floor();
            jump = naive(wave, 0.0, pw) - naive(wave, before, pw);
            at = f;
            p = dt * f;
            wrap = Some(f);
        } else if p >= 1.0 {
            p -= 1.0;
            wrap = Some(p / dt);
        }
        self.phase = p;
        let mut cur = naive(wave, p, pw) + correction(wave, p, dt, pw, reset.is_some());
        let mut pre = 0.0;
        if reset.is_some() {
            cur += jump * 0.5 * blep(at); // after the step
            pre = jump * 0.5 * blep(at - 1.0); // before it: belongs to the sample that has gone
        }
        let value = self.prev + pre as f32;
        self.prev = cur as f32;
        Tick { value, wrap }
    }
}

/// A square one octave below its parent: a flip-flop that toggles on every wrap of the parent's phase (a sync reset
/// counts), so it is an exact octave and band-limited with the same PolyBLEP.
#[derive(Debug, Clone, Copy)]
pub struct Sub {
    level: f64,
    prev: f32,
}

impl Default for Sub {
    fn default() -> Self {
        Sub { level: 1.0, prev: 1.0 }
    }
}

impl Sub {
    pub fn reset(&mut self) {
        *self = Sub::default();
    }

    /// One sample; `wrap` is the parent's `Tick::wrap`. The output is one sample late like the parent's.
    pub fn tick(&mut self, wrap: Option<f64>) -> f32 {
        let (mut cur, mut pre) = (self.level, 0.0);
        if let Some(f) = wrap {
            self.level = -self.level;
            let jump = 2.0 * self.level;
            cur = self.level + jump * 0.5 * blep(f);
            pre = jump * 0.5 * blep(f - 1.0);
        }
        let value = self.prev + pre as f32;
        self.prev = cur as f32;
        value
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    const RATE: f64 = 48_000.0;

    /// the magnitude spectrum of a power-of-two block (radix-2 FFT, no window: use it on coherent signals)
    pub fn spectrum(x: &[f32]) -> Vec<f64> {
        let n = x.len();
        let mut re: Vec<f64> = x.iter().map(|&v| f64::from(v)).collect();
        let mut im = vec![0.0f64; n];
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
        (0..n / 2).map(|i| (re[i] * re[i] + im[i] * im[i]).sqrt() * 2.0 / n as f64).collect()
    }

    fn render(wave: Wave, dt: f64, pw: f64, n: usize) -> Vec<f32> {
        let mut o = Osc::default();
        (0..n).map(|_| o.tick(wave, dt, pw, None).value).collect()
    }

    /// harmonic amplitudes against 2/(π m), and the share of energy off the harmonics, for a saw whose period fits the
    /// block exactly (bin `k` of `n`)
    fn saw_purity(k: usize, n: usize, band_limited: bool) -> (Vec<f64>, f64) {
        let dt = k as f64 / n as f64;
        let x: Vec<f32> = if band_limited {
            render(Wave::Saw, dt, 0.5, n + 1)[1..].to_vec() // skip the one sample of delay: the block starts at the wrap
        } else {
            (0..n).map(|i| (2.0 * ((i as f64 * dt) % 1.0) - 1.0) as f32).collect()
        };
        let s = spectrum(&x);
        let harm: f64 = (1..).take_while(|m| m * k < n / 2).map(|m| s[m * k] * s[m * k]).sum();
        let off: f64 = (1..n / 2).filter(|b| b % k != 0).map(|b| s[b] * s[b]).sum();
        let amps = (1..=10).take_while(|m| m * k < n / 2).map(|m| s[m * k]).collect();
        (amps, 10.0 * (off / harm).log10())
    }

    #[test]
    fn polyblep_saw_keeps_its_harmonics_and_loses_most_of_the_aliasing() {
        // 211 bins of 16384 at 48 kHz = 618 Hz; every alias lands off a harmonic bin (k is odd)
        let (amps, off_db) = saw_purity(211, 16384, true);
        let (_, naive_db) = saw_purity(211, 16384, false);
        for (i, a) in amps.iter().enumerate() {
            let m = (i + 1) as f64;
            let want = 2.0 / (core::f64::consts::PI * m);
            assert!((20.0 * (a / want).log10()).abs() < 0.6, "harmonic {m}: {a} against {want}");
        }
        assert!(off_db < -30.0, "off-harmonic energy {off_db} dB");
        assert!(off_db < naive_db - 15.0, "PolyBLEP {off_db} dB against the naive saw's {naive_db} dB");
    }

    #[test]
    fn polyblep_stays_pure_high_up() {
        // 4.4 kHz: harmonics are few, the aliases of the 2nd and 3rd would sit right between them
        let (_, off_db) = saw_purity(1501, 16384, true);
        let (_, naive_db) = saw_purity(1501, 16384, false);
        assert!(off_db < -20.0 && off_db < naive_db - 8.0, "{off_db} dB against naive {naive_db} dB");
    }

    #[test]
    fn pulse_width_sets_the_duty_cycle_and_the_dc() {
        for pw in [0.1, 0.25, 0.5, 0.8] {
            let n = 48_000;
            let x = render(Wave::Pulse, 100.0 / RATE, pw, n);
            let mean = x.iter().map(|&v| f64::from(v)).sum::<f64>() / n as f64;
            assert!((mean - (2.0 * pw - 1.0)).abs() < 0.002, "pw {pw}: mean {mean}");
            let high = x.iter().filter(|&&v| v > 0.0).count() as f64 / n as f64;
            assert!((high - pw).abs() < 0.002, "pw {pw}: duty {high}");
        }
    }

    #[test]
    fn triangle_and_sine_have_unit_amplitude_and_the_right_period() {
        for wave in [Wave::Tri, Wave::Sine] {
            let x = render(wave, 1.0 / 128.0, 0.5, 1024);
            let peak = x.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(peak > 0.98 && peak < 1.01, "{wave:?} peak {peak}");
            for i in 128..1024 - 128 {
                assert!((x[i] - x[i + 128]).abs() < 1e-5, "{wave:?} is not periodic at {i}");
            }
        }
    }

    #[test]
    fn triangle_is_clean_where_the_naive_one_is_not() {
        // a triangle's harmonics fall as 1/m²: the odd ones have amplitude 8 / (π m)², the even ones none
        let (k, n) = (211usize, 16384usize);
        let x = render(Wave::Tri, k as f64 / n as f64, 0.5, n + 1)[1..].to_vec();
        let s = spectrum(&x);
        for m in [1usize, 3, 5, 7] {
            let want = 8.0 / (core::f64::consts::PI * m as f64).powi(2);
            assert!((20.0 * (s[m * k] / want).log10()).abs() < 0.3, "harmonic {m}: {} against {want}", s[m * k]);
        }
        let harm: f64 = (1..).take_while(|m| m * k < n / 2).map(|m| s[m * k] * s[m * k]).sum();
        let off: f64 = (1..n / 2).filter(|b| b % k != 0).map(|b| s[b] * s[b]).sum();
        assert!(10.0 * (off / harm).log10() < -60.0);
    }

    #[test]
    fn sync_locks_osc2_to_the_period_of_osc1() {
        let (dt1, dt2) = (440.0 / RATE, 440.0 * 2.37 / RATE);
        let n = 48_000;
        let (mut o1, mut o2) = (Osc::default(), Osc::default());
        let (mut wraps, mut synced, mut free) = (vec![], vec![], vec![]);
        let mut o3 = Osc::default();
        for i in 0..n {
            let t1 = o1.tick(Wave::Saw, dt1, 0.5, None);
            if let Some(f) = t1.wrap {
                wraps.push(i as f64 - f);
            }
            synced.push(o2.tick(Wave::Saw, dt2, 0.5, t1.wrap).value);
            free.push(o3.tick(Wave::Saw, dt2, 0.5, None).value);
        }
        // and the pattern repeats with the master's period (48000 / 440 = 109.09… samples → every 6000 samples = 55 cycles)
        for i in 100..n - 6000 - 2 {
            assert!((synced[i] - synced[i + 6000]).abs() < 1e-3, "the synced saw is not periodic with the master at {i}");
        }
        // unsynced it is not: its own pitch runs against the master's
        let drift = (100..n - 6000 - 2).filter(|&i| (free[i] - free[i + 6000]).abs() > 0.1).count();
        assert!(drift > 10_000, "the free-running saw repeats too: {drift}");
        // the mean period of the master's wraps is its pitch
        let mean = (wraps[wraps.len() - 1] - wraps[0]) / (wraps.len() - 1) as f64;
        assert!((mean - RATE / 440.0).abs() < 0.01, "{mean}");
    }

    #[test]
    fn sync_does_not_alias_much_more_than_a_plain_saw() {
        // a synced saw at 2.37× the master has steps of its own and the master's resets: all band-limited
        let (k1, n) = (97usize, 16384usize);
        let (dt1, dt2) = (k1 as f64 / n as f64, k1 as f64 * 2.37 / n as f64);
        let (mut o1, mut o2) = (Osc::default(), Osc::default());
        let x: Vec<f32> = (0..n + 1).map(|_| {
            let t = o1.tick(Wave::Saw, dt1, 0.5, None);
            o2.tick(Wave::Saw, dt2, 0.5, t.wrap).value
        }).collect();
        let s = spectrum(&x[1..]);
        let total: f64 = s.iter().map(|v| v * v).sum();
        // the signal repeats every master period, so its energy is on multiples of k1; what is off them is aliasing
        let off: f64 = (1..n / 2).filter(|b| b % k1 != 0).map(|b| s[b] * s[b]).sum();
        assert!(10.0 * (off / total).log10() < -28.0, "{} dB", 10.0 * (off / total).log10());
    }

    #[test]
    fn the_sub_is_an_exact_octave_down_and_square() {
        let dt = 1000.0 / RATE;
        let (mut o, mut s) = (Osc::default(), Sub::default());
        let x: Vec<f32> = (0..48_000).map(|_| {
            let t = o.tick(Wave::Saw, dt, 0.5, None);
            s.tick(t.wrap)
        }).collect();
        // 500 Hz = 96 samples a period; sample the settled middle of each half
        let high = x.iter().filter(|&&v| v > 0.99).count() as f64;
        let low = x.iter().filter(|&&v| v < -0.99).count() as f64;
        assert!((high - low).abs() < 100.0 && high + low > 40_000.0, "square: {high} high {low} low");
        let crossings = (1..x.len()).filter(|&i| x[i - 1] < 0.0 && x[i] >= 0.0).count();
        assert!((499..=500).contains(&crossings), "500 Hz in a second = an octave under 1 kHz: {crossings} rising crossings");
    }

    #[test]
    fn the_sub_follows_a_sync_reset() {
        // restarted by the master, the slave's wraps (reset or its own) toggle its sub: the sub still halves them
        let (mut o1, mut o2, mut s2) = (Osc::default(), Osc::default(), Sub::default());
        let mut wraps = 0;
        let x: Vec<f32> = (0..48_000).map(|_| {
            let t1 = o1.tick(Wave::Saw, 300.0 / RATE, 0.5, None);
            let t2 = o2.tick(Wave::Saw, 910.0 / RATE, 0.5, t1.wrap);
            wraps += usize::from(t2.wrap.is_some());
            s2.tick(t2.wrap)
        }).collect();
        let crossings = (1..x.len()).filter(|&i| x[i - 1] < 0.0 && x[i] >= 0.0).count();
        assert!(crossings.abs_diff(wraps / 2) <= 1, "{crossings} rising edges for {wraps} wraps");
    }
}
