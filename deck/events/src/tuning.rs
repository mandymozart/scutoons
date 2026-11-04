//! The deck's tuning (research/ABI-V2.md §1.4, owner 2026-11-03: "i need both in my final system"): a note carries its
//! pitch both ways, Hz and the MIDI key (fractional, 60.0 = middle C), and the core's event host fills whichever the source
//! lacked – here, once, so no module converts. Twelve-tone equal temperament on an A4 reference (a deck setting); a
//! tuning table for other temperaments is a later field. libm, so native and wasm give the same bits (plan 3.4).

/// the MIDI key of A4
pub const A4_KEY: f32 = 69.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// the reference pitch of A4 in Hz (440 unless the deck says otherwise)
    pub a4: f32,
}

impl Default for Tuning {
    fn default() -> Tuning {
        Tuning { a4: 440.0 }
    }
}

impl Tuning {
    /// the pitch of `key` (fractional) in Hz
    pub fn hz(&self, key: f32) -> f32 {
        self.a4 * libm::exp2f((key - A4_KEY) / 12.0)
    }

    /// the key (fractional) of `hz`; NaN for a pitch ≤ 0
    pub fn key(&self, hz: f32) -> f32 {
        if hz > 0.0 {
            A4_KEY + 12.0 * libm::log2f(hz / self.a4)
        } else {
            f32::NAN
        }
    }

    /// `(hz, key)` with the absent one (NaN) filled from the other; both absent stay absent
    pub fn complete(&self, hz: f32, key: f32) -> (f32, f32) {
        match (hz.is_nan(), key.is_nan()) {
            (true, false) => (self.hz(key), key),
            (false, true) => (hz, self.key(hz)),
            _ => (hz, key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_and_hz_meet_on_the_reference() {
        let t = Tuning::default();
        assert_eq!(t.hz(69.0), 440.0);
        assert!((t.hz(60.0) - 261.6256).abs() < 1e-3, "middle C");
        assert_eq!(t.key(440.0), 69.0);
        assert!((t.key(t.hz(60.5)) - 60.5).abs() < 1e-4, "a quarter tone survives the round trip");
        assert!((Tuning { a4: 432.0 }.hz(69.0) - 432.0).abs() < 1e-6);
        assert!(t.key(0.0).is_nan());
        assert_eq!(t.complete(f32::NAN, 69.0), (440.0, 69.0));
        assert_eq!(t.complete(880.0, f32::NAN), (880.0, 81.0));
        let (a, b) = t.complete(f32::NAN, f32::NAN);
        assert!(a.is_nan() && b.is_nan());
        assert_eq!(t.complete(100.0, 3.0), (100.0, 3.0), "both given: the source's truth stays");
    }
}
