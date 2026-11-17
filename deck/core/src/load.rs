//! The load meters (CPU, GPU): a share of busy time over wall time, measured over at least `SAMPLE_S`, smoothed over
//! about `TAU_S` with the peak held `HOLD_S`. The adapters feed raw times (the process's CPU time, the GPU's
//! timestamps); what the meters mean, how they move and how they read is here once.

/// a sample covers at least this long (s): shorter ones are noise from the scheduler's slices
pub const SAMPLE_S: f64 = 0.25;
/// the smoothing's time constant (s)
pub const TAU_S: f32 = 1.0;
/// a peak stays this long (s), then falls with `TAU_S`
pub const HOLD_S: f32 = 2.0;

/// what CPU means: the whole process (every thread: audio, frames, loaders) against all the machine's cores
pub const CPU_MEANS: &str = "this process on all cores, as the task manager counts it";
/// what GPU means: the frames' GPU work against the time between frames
pub const GPU_MEANS: &str = "the GPU's work on the frames, of the time between them";

/// Busy time and wall time summed until they span a sample.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Busy {
    busy: f64,
    wall: f64,
}

impl Busy {
    /// `busy` s of work in `wall` s on `lanes` lanes (cores; 1 for the GPU); the share (0..1) and the span it covers once
    /// they span a sample
    pub fn add(&mut self, busy: f64, wall: f64, lanes: usize) -> Option<(f32, f32)> {
        if !(busy.is_finite() && wall.is_finite()) || busy < 0.0 || wall <= 0.0 {
            return None;
        }
        self.busy += busy / lanes.max(1) as f64;
        self.wall += wall;
        if self.wall < SAMPLE_S {
            return None;
        }
        let share = (self.busy / self.wall).clamp(0.0, 1.0) as f32;
        let span = self.wall as f32;
        *self = Busy::default();
        Some((share, span))
    }
}

/// A meter's reading: the smoothed share and the held peak (0..1).
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Reading {
    pub value: f32,
    pub peak: f32,
}

impl Reading {
    /// "CPU 7 %, peak 12 %"
    pub fn words(&self, name: &str) -> String {
        format!("{name} {:.0} %, peak {:.0} %", self.value * 100.0, self.peak * 100.0)
    }

    /// as two words (f32 bits) for an atomic between threads
    pub fn to_bits(self) -> u64 {
        u64::from(self.value.to_bits()) << 32 | u64::from(self.peak.to_bits())
    }

    pub fn from_bits(bits: u64) -> Reading {
        Reading { value: f32::from_bits((bits >> 32) as u32), peak: f32::from_bits(bits as u32) }
    }
}

/// The smoothing and the peak hold: an exponential average over `TAU_S`, a peak held `HOLD_S` and then falling.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Meter {
    reading: Option<Reading>,
    held: f32,
}

impl Meter {
    /// a sample's share over `span` s
    pub fn push(&mut self, share: f32, span: f32) {
        if !share.is_finite() || !span.is_finite() || span <= 0.0 {
            return;
        }
        let share = share.clamp(0.0, 1.0);
        let Some(r) = self.reading.as_mut() else {
            self.reading = Some(Reading { value: share, peak: share });
            return;
        };
        let k = 1.0 - libm::expf(-span / TAU_S);
        r.value += (share - r.value) * k;
        if share >= r.peak {
            r.peak = share;
            self.held = 0.0;
        } else {
            self.held += span;
            if self.held > HOLD_S {
                r.peak = (r.peak + (r.value - r.peak) * k).max(r.value);
            }
        }
    }

    /// None before the first sample
    pub fn reading(&self) -> Option<Reading> {
        self.reading
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn busy_waits_for_a_whole_sample_and_divides_by_the_lanes() {
        let mut b = Busy::default();
        // 50 fps frames: 5 ms of work in 20 ms
        for _ in 0..12 {
            assert_eq!(b.add(0.005, 0.02, 1), None);
        }
        let (share, span) = b.add(0.005, 0.02, 1).unwrap();
        assert!((share - 0.25).abs() < 1e-3 && (span - 0.26).abs() < 1e-3, "{share} {span}");
        // a process busy 1 s of CPU time in 0.25 s on 8 cores: half the machine
        assert_eq!(b.add(1.0, 0.25, 8), Some((0.5, 0.25)));
        // more work than wall: clamped; nonsense: ignored
        assert_eq!(b.add(2.0, 0.25, 1).map(|s| s.0), Some(1.0));
        assert_eq!(b.add(f64::NAN, 0.25, 1), None);
        assert_eq!(b.add(0.1, 0.0, 1), None);
    }

    /// the owner's "jumps like crazy": samples that swing between 10 % and 50 % read near their mean, not either end
    #[test]
    fn a_jumping_load_reads_steady() {
        let mut m = Meter::default();
        assert_eq!(m.reading(), None);
        let mut values = Vec::new();
        for i in 0..80 {
            m.push(if i % 2 == 0 { 0.1 } else { 0.5 }, SAMPLE_S as f32);
            values.push(m.reading().unwrap().value);
        }
        let settled = &values[20..];
        let (lo, hi) = settled.iter().fold((1.0f32, 0.0f32), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
        assert!(lo > 0.25 && hi < 0.35 && hi - lo < 0.06, "settled between {lo} and {hi}");
    }

    #[test]
    fn a_step_settles_in_about_a_second() {
        let mut m = Meter::default();
        m.push(0.0, 0.25);
        m.push(1.0, 0.25);
        assert!(m.reading().unwrap().value < 0.3, "not at once");
        for _ in 0..3 {
            m.push(1.0, 0.25);
        }
        // 1 s = one time constant: 63 %
        assert!((m.reading().unwrap().value - (1.0 - (-1.0f32).exp())).abs() < 0.01, "{:?}", m.reading());
        for _ in 0..16 {
            m.push(1.0, 0.25);
        }
        assert!(m.reading().unwrap().value > 0.98);
    }

    #[test]
    fn a_peak_is_held_then_falls_to_the_value() {
        let mut m = Meter::default();
        m.push(0.1, 0.25);
        m.push(0.9, 0.25);
        assert_eq!(m.reading().unwrap().peak, 0.9);
        // held for HOLD_S
        for _ in 0..8 {
            m.push(0.1, 0.25);
        }
        assert_eq!(m.reading().unwrap().peak, 0.9);
        for _ in 0..40 {
            m.push(0.1, 0.25);
        }
        let r = m.reading().unwrap();
        assert!(r.peak >= r.value && r.peak - r.value < 0.01, "{r:?}");
    }

    #[test]
    fn a_reading_crosses_threads_and_reads_in_words() {
        let r = Reading { value: 0.071, peak: 0.124 };
        assert_eq!(Reading::from_bits(r.to_bits()), r);
        assert_eq!(r.words("CPU"), "CPU 7 %, peak 12 %");
    }
}
