//! The meters (#373, BRIDGE-AUDIT.md §2): what a level meter reads – ONE model for the browser's strip, the creator's strip
//! and the native HUD. A peak is held and falls (`Meters`), a linear level sits on the dB scale (`level_db`, `position`),
//! its zone is green / amber / red (`Zone`), the limiter's gain reduction fills a bar (`limiter_bar`). The scale and the
//! zone edges are constants here and nowhere else: the pages draw what the analysis frame (live-core's wire.rs) carries and
//! get the marks from the generated shared/live-core/src/wire.ts.

/// a held peak loses 63 % per this many seconds (a meter, not a scope)
pub const FALL_SECONDS: f32 = 0.25;
/// the scale: dBFS from the floor (silence) to the top (a little headroom above the 0.0 dB mark, so a clip shows)
pub const FLOOR_DB: f32 = -48.0;
pub const TOP_DB: f32 = 3.0;
/// the zones' edges (dB): below `OK_DB` green, up to `HOT_DB` amber, above red (digital full scale is 0.0)
pub const OK_DB: f32 = -12.0;
pub const HOT_DB: f32 = -3.0;
/// the limiter's bar is full at this gain reduction (dB)
pub const LIMITER_FULL_DB: f32 = 12.0;

fn unit(x: f32) -> f32 {
    if x.is_finite() {
        x.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// A linear level (0..1, 1 = full scale) in dB, floored at the scale's bottom (silence = the floor).
pub fn level_db(linear: f32) -> f32 {
    if !linear.is_finite() || linear <= 0.0 {
        return FLOOR_DB;
    }
    (20.0 * libm::log10f(linear)).max(FLOOR_DB)
}

/// Where a dB value sits on the meter, 0..1 from the bottom.
pub fn position(db: f32) -> f32 {
    unit((db - FLOOR_DB) / (TOP_DB - FLOOR_DB))
}

/// The limiter's gain reduction (dB, 0 = untouched; the sign does not matter) as a 0..1 bar.
pub fn limiter_bar(db: f32) -> f32 {
    unit(db.abs() / LIMITER_FULL_DB)
}

/// A meter's zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Ok = 0,
    Hot = 1,
    Clip = 2,
}

impl Zone {
    /// in code order: a zone's code is its index (the analysis frame carries the code)
    pub const ALL: [Zone; 3] = [Zone::Ok, Zone::Hot, Zone::Clip];

    pub fn of_db(db: f32) -> Zone {
        if db >= HOT_DB {
            Zone::Clip
        } else if db >= OK_DB {
            Zone::Hot
        } else {
            Zone::Ok
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Zone::Ok => "ok",
            Zone::Hot => "hot",
            Zone::Clip => "clip",
        }
    }
}

/// One output's reading: the held level, its place on the scale and its zone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    pub level: f32,
    pub position: f32,
    pub zone: Zone,
}

/// The held peaks of a device's outputs. Each analysis frame brings the largest sample per output since the one before;
/// a held peak falls (`FALL_SECONDS`) until a higher one comes. No allocation after `new`.
#[derive(Debug, Clone, PartialEq)]
pub struct Meters {
    held: Vec<f32>,
}

impl Meters {
    pub fn new(outputs: usize) -> Meters {
        Meters { held: vec![0.0; outputs] }
    }

    /// One frame's peaks (0..1 per output) `dt` seconds after the one before; more peaks than outputs are ignored.
    pub fn push(&mut self, peaks: &[f32], dt: f32) {
        let keep = libm::expf(-dt.max(0.0) / FALL_SECONDS);
        for (h, &p) in self.held.iter_mut().zip(peaks) {
            *h = unit(p).max(*h * keep);
        }
    }

    /// the held peaks, 0..1 per output
    pub fn held(&self) -> &[f32] {
        &self.held
    }

    /// output `i`'s reading (silence past the outputs)
    pub fn reading(&self, i: usize) -> Reading {
        let level = self.held.get(i).copied().unwrap_or(0.0);
        reading(level)
    }

    pub fn reset(&mut self) {
        self.held.fill(0.0);
    }
}

/// A linear level's reading.
pub fn reading(level: f32) -> Reading {
    let level = unit(level);
    let db = level_db(level);
    Reading { level, position: position(db), zone: Zone::of_db(db) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scale_reads_levels_in_db() {
        assert_eq!(level_db(1.0), 0.0);
        assert!((level_db(0.5) + 6.02).abs() < 0.01);
        assert_eq!(level_db(0.0), FLOOR_DB, "silence sits on the floor");
        assert_eq!(level_db(1e-9), FLOOR_DB);
        assert_eq!(level_db(f32::NAN), FLOOR_DB);
        assert_eq!(position(FLOOR_DB), 0.0);
        assert_eq!(position(TOP_DB), 1.0);
        let (zero, ok, hot) = (position(0.0), position(OK_DB), position(HOT_DB));
        assert!(zero > 0.9 && zero < 1.0, "0 dB is near the top, with headroom above");
        assert!(ok < hot && hot < zero);
    }

    #[test]
    fn zones_change_at_their_edges() {
        let zones = [-30.0, -12.0, -6.0, -3.0, 0.0].map(|db| Zone::of_db(db).name());
        assert_eq!(zones, ["ok", "hot", "hot", "clip", "clip"]);
        assert_eq!(Zone::ALL.map(|z| z as usize), [0, 1, 2]);
    }

    #[test]
    fn the_limiter_bar_is_full_at_twelve_db() {
        assert_eq!(limiter_bar(0.0), 0.0);
        assert_eq!(limiter_bar(-6.0), 0.5);
        assert_eq!(limiter_bar(6.0), 0.5);
        assert_eq!(limiter_bar(-40.0), 1.0);
        assert_eq!(limiter_bar(f32::NAN), 0.0);
    }

    #[test]
    fn a_peak_is_held_and_falls_until_a_higher_one() {
        let mut m = Meters::new(2);
        m.push(&[1.0, 0.5], 1.0 / 60.0);
        assert_eq!(m.held(), &[1.0, 0.5]);
        m.push(&[0.0, 0.0], FALL_SECONDS);
        let e = 1.0 / std::f32::consts::E;
        assert!((m.held()[0] - e).abs() < 1e-6, "×1/e per FALL_SECONDS: {:?}", m.held());
        m.push(&[0.9, 0.0], 0.0);
        assert_eq!(m.held()[0], 0.9, "a higher peak replaces it at once");
        m.push(&[0.1, 0.0], 0.0);
        assert_eq!(m.held()[0], 0.9, "a lower one does not");
        m.push(&[0.0, 0.0], -3.0);
        assert_eq!(m.held()[0], 0.9, "a clock that went back holds");
        m.push(&[f32::NAN, 7.0], 0.0);
        assert_eq!(m.held()[1], 1.0, "a peak is made safe to draw (clamped, NaN = 0)");
        m.reset();
        assert_eq!(m.held(), &[0.0, 0.0]);
    }

    #[test]
    fn a_reading_carries_level_position_and_zone() {
        let r = reading(0.5);
        assert_eq!((r.level, r.zone), (0.5, Zone::Hot));
        assert!(r.position > reading(0.01).position);
        assert_eq!(reading(0.0).zone, Zone::Ok);
        assert_eq!(Meters::new(1).reading(5), reading(0.0), "past the outputs: silence");
    }
}
