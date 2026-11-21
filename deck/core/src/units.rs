//! The units a value is read and typed in (decisions/deck.md [design]): the one catalogue and its conversions, which
//! every field with a unit switch uses – deck-kit's (the editor, the deck) directly, the Svelte kit's through space-web.
//! A value is stored in its own unit (a crop as parts of the picture, a fade in seconds, a gain in dB); a unit is only
//! how it is shown and typed. Each dimension has a base the conversions go through:
//!
//! - **time:** seconds; ms; frames (at the source's frame rate); beats and bars (at a tempo and its beats a bar)
//! - **length:** parts of a reference (0..1); % of it; px (the reference's pixels, whole);
//!   on a web reference (a slice at 1x) pt (4/3 px) and rem (16 px); on a font size, em (the reference itself) in place of %
//! - **distance:** metres in the world (the stage, the scene); cm; ft
//! - **angle:** degrees of a vertical field of view; mm (the focal length that sees it on a full-frame sensor)
//! - **level:** dB; % (the linear gain, 100 % = 0 dB); × (the linear gain itself, 1 = 0 dB)
//! - **tempo:** BPM
//!
//! A unit whose reference is unknown (frames without a frame rate, px without a picture) is not offered.

/// the sensor a focal length is read on: full frame, 24 mm high
pub const SENSOR_H_MM: f64 = 24.0;
/// the level a gain of 0 reads as, dB
pub const SILENT_DB: f64 = -120.0;
/// CSS px in a point and in a rem
pub const CSS_PX_PER_PT: f64 = 4.0 / 3.0;
pub const CSS_PX_PER_REM: f64 = 16.0;
/// metres in an international foot
pub const M_PER_FOOT: f64 = 0.3048;
/// the beats a bar when the tempo's own is not known
pub const PER_BAR: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dimension {
    Time,
    Length,
    /// real-world distances (the stage, a camera's height): base metres, needs no reference
    Distance,
    Angle,
    Level,
    Tempo,
}

impl Dimension {
    pub const ALL: [Dimension; 6] = [Dimension::Time, Dimension::Length, Dimension::Distance, Dimension::Angle, Dimension::Level, Dimension::Tempo];

    pub fn id(self) -> &'static str {
        match self {
            Dimension::Time => "time",
            Dimension::Length => "length",
            Dimension::Distance => "distance",
            Dimension::Angle => "angle",
            Dimension::Level => "level",
            Dimension::Tempo => "tempo",
        }
    }

    pub fn of_id(id: &str) -> Option<Dimension> {
        Dimension::ALL.into_iter().find(|d| d.id() == id)
    }

    /// its units, in the order a switch shows them
    pub fn units(self) -> &'static [Unit] {
        match self {
            Dimension::Time => &[Unit::Seconds, Unit::Milliseconds, Unit::Frames, Unit::Beats, Unit::Bars],
            Dimension::Length => &[Unit::Pixels, Unit::Percent, Unit::Em, Unit::Points, Unit::Rem],
            Dimension::Distance => &[Unit::Metres, Unit::Centimetres, Unit::Feet],
            Dimension::Angle => &[Unit::Millimetres, Unit::Degrees],
            Dimension::Level => &[Unit::Decibels, Unit::Gain, Unit::Linear],
            Dimension::Tempo => &[Unit::Bpm],
        }
    }

    /// the units it can be read in with what is known
    pub fn offered(self, refs: &Refs) -> Vec<Unit> {
        self.units().iter().copied().filter(|u| u.known(refs)).collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Unit {
    Seconds,
    Milliseconds,
    Frames,
    Beats,
    Bars,
    Pixels,
    Percent,
    /// parts of a font size (the reference is one)
    Em,
    /// CSS points: 4/3 px
    Points,
    /// CSS rem: 16 px
    Rem,
    /// a distance in the world, the distance's base
    Metres,
    Centimetres,
    /// international feet: 0.3048 m
    Feet,
    Degrees,
    Millimetres,
    Decibels,
    /// a level as its linear gain, %
    Gain,
    /// a level as its linear gain, 1 = 0 dB
    Linear,
    Bpm,
}

/// What a conversion needs to know besides the value: the source's frame rate, the tempo and its beats a bar, the
/// reference a length is a part of in pixels. None = not known: the units that need it are not offered.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Refs {
    pub fps: Option<f64>,
    pub bpm: Option<f64>,
    pub per_bar: Option<u32>,
    pub pixels: Option<f64>,
    /// the pixels are a web reference at 1x: pt and rem are offered
    pub web: bool,
    /// the reference is a font size: em is offered, % is not
    pub em: bool,
}

fn positive(v: Option<f64>) -> Option<f64> {
    v.filter(|v| v.is_finite() && *v > 0.0)
}

impl Refs {
    /// From its wire form `[fps, bpm, per_bar, pixels, unused, web, em]` (the web kit's, through space-web): NaN, ≤ 0 or left
    /// out = not known.
    pub fn from_wire(w: &[f64]) -> Refs {
        let at = |i: usize| positive(w.get(i).copied());
        Refs { fps: at(0), bpm: at(1), per_bar: at(2).map(|b| b.round() as u32), pixels: at(3), web: at(5).is_some(), em: at(6).is_some() }
    }

    fn fps(&self) -> Option<f64> {
        positive(self.fps)
    }
    fn bpm(&self) -> Option<f64> {
        positive(self.bpm)
    }
    fn per_bar(&self) -> f64 {
        f64::from(self.per_bar.filter(|b| *b > 0).unwrap_or(PER_BAR))
    }
    fn pixels(&self) -> Option<f64> {
        positive(self.pixels)
    }
}

impl Unit {
    pub const ALL: [Unit; 19] = [
        Unit::Seconds,
        Unit::Milliseconds,
        Unit::Frames,
        Unit::Beats,
        Unit::Bars,
        Unit::Pixels,
        Unit::Percent,
        Unit::Em,
        Unit::Points,
        Unit::Rem,
        Unit::Metres,
        Unit::Centimetres,
        Unit::Feet,
        Unit::Degrees,
        Unit::Millimetres,
        Unit::Decibels,
        Unit::Gain,
        Unit::Linear,
        Unit::Bpm,
    ];

    pub fn dimension(self) -> Dimension {
        match self {
            Unit::Seconds | Unit::Milliseconds | Unit::Frames | Unit::Beats | Unit::Bars => Dimension::Time,
            Unit::Pixels | Unit::Percent | Unit::Em | Unit::Points | Unit::Rem => Dimension::Length,
            Unit::Metres | Unit::Centimetres | Unit::Feet => Dimension::Distance,
            Unit::Degrees | Unit::Millimetres => Dimension::Angle,
            Unit::Decibels | Unit::Gain | Unit::Linear => Dimension::Level,
            Unit::Bpm => Dimension::Tempo,
        }
    }

    /// its stable name (a remembered choice, the wasm API)
    pub fn id(self) -> &'static str {
        match self {
            Unit::Seconds => "s",
            Unit::Milliseconds => "ms",
            Unit::Frames => "frames",
            Unit::Beats => "beats",
            Unit::Bars => "bars",
            Unit::Pixels => "px",
            Unit::Percent => "percent",
            Unit::Em => "em",
            Unit::Points => "pt",
            Unit::Rem => "rem",
            Unit::Metres => "m",
            Unit::Centimetres => "cm",
            Unit::Feet => "ft",
            Unit::Degrees => "deg",
            Unit::Millimetres => "mm",
            Unit::Decibels => "db",
            Unit::Gain => "gain",
            Unit::Linear => "linear",
            Unit::Bpm => "bpm",
        }
    }

    pub fn of_id(id: &str) -> Option<Unit> {
        Unit::ALL.into_iter().find(|u| u.id() == id)
    }

    /// as a person reads it after a number
    pub fn word(self) -> &'static str {
        match self {
            Unit::Seconds => "s",
            Unit::Milliseconds => "ms",
            Unit::Frames => "frames",
            Unit::Beats => "beats",
            Unit::Bars => "bars",
            Unit::Pixels => "px",
            Unit::Percent | Unit::Gain => "%",
            Unit::Em => "em",
            Unit::Points => "pt",
            Unit::Rem => "rem",
            Unit::Metres => "m",
            Unit::Centimetres => "cm",
            Unit::Feet => "ft",
            Unit::Degrees => "°",
            Unit::Millimetres => "mm",
            Unit::Decibels => "dB",
            Unit::Linear => "×",
            Unit::Bpm => "BPM",
        }
    }

    /// counted in whole steps (a source's pixels, its frames)
    pub fn whole(self) -> bool {
        matches!(self, Unit::Pixels | Unit::Frames | Unit::Milliseconds)
    }

    /// the decimals it is shown with
    pub fn decimals(self) -> usize {
        match self {
            Unit::Pixels | Unit::Frames | Unit::Milliseconds | Unit::Millimetres | Unit::Degrees | Unit::Centimetres => 0,
            Unit::Percent | Unit::Gain | Unit::Decibels | Unit::Bpm => 1,
            Unit::Seconds | Unit::Beats | Unit::Bars | Unit::Metres | Unit::Feet | Unit::Em | Unit::Points | Unit::Linear => 2,
            Unit::Rem => 3,
        }
    }

    /// one step of a field in it (an arrow key): its last decimal; type units step coarser
    pub fn step(self) -> f64 {
        match self {
            Unit::Points => 0.5,
            Unit::Rem => 0.05,
            _ => libm::pow(10.0, -(self.decimals() as f64)),
        }
    }

    /// what it needs is known
    pub fn known(self, refs: &Refs) -> bool {
        match self {
            Unit::Frames => refs.fps().is_some(),
            Unit::Beats | Unit::Bars => refs.bpm().is_some(),
            Unit::Pixels => refs.pixels().is_some(),
            Unit::Points | Unit::Rem => refs.web && refs.pixels().is_some(),
            Unit::Percent => !refs.em,
            Unit::Em => refs.em,
            _ => true,
        }
    }

    /// `v` in this unit as its dimension's base (none: its reference is not known)
    pub fn to_base(self, v: f64, refs: &Refs) -> Option<f64> {
        Some(match self {
            Unit::Seconds | Unit::Degrees | Unit::Decibels | Unit::Bpm => v,
            Unit::Milliseconds => v / 1000.0,
            Unit::Frames => v / refs.fps()?,
            Unit::Beats => v * 60.0 / refs.bpm()?,
            Unit::Bars => v * refs.per_bar() * 60.0 / refs.bpm()?,
            Unit::Pixels => v / refs.pixels()?,
            Unit::Percent => v / 100.0,
            Unit::Em => v,
            Unit::Points => v * CSS_PX_PER_PT / refs.pixels()?,
            Unit::Rem => v * CSS_PX_PER_REM / refs.pixels()?,
            Unit::Metres => v,
            Unit::Centimetres => v / 100.0,
            Unit::Feet => v * M_PER_FOOT,
            Unit::Millimetres => fov_deg(v),
            Unit::Gain if v <= 0.0 => SILENT_DB,
            Unit::Gain => (20.0 * libm::log10(v / 100.0)).max(SILENT_DB),
            Unit::Linear if v <= 0.0 => SILENT_DB,
            Unit::Linear => (20.0 * libm::log10(v)).max(SILENT_DB),
        })
    }

    /// the dimension's base `b` in this unit
    pub fn from_base(self, b: f64, refs: &Refs) -> Option<f64> {
        Some(match self {
            Unit::Seconds | Unit::Degrees | Unit::Decibels | Unit::Bpm => b,
            Unit::Milliseconds => b * 1000.0,
            Unit::Frames => b * refs.fps()?,
            Unit::Beats => b * refs.bpm()? / 60.0,
            Unit::Bars => b * refs.bpm()? / 60.0 / refs.per_bar(),
            Unit::Pixels => b * refs.pixels()?,
            Unit::Percent => b * 100.0,
            Unit::Em => b,
            Unit::Points => b * refs.pixels()? / CSS_PX_PER_PT,
            Unit::Rem => b * refs.pixels()? / CSS_PX_PER_REM,
            Unit::Metres => b,
            Unit::Centimetres => b * 100.0,
            Unit::Feet => b / M_PER_FOOT,
            Unit::Millimetres => focal_mm(b),
            Unit::Gain if b <= SILENT_DB => 0.0,
            Unit::Gain => 100.0 * libm::pow(10.0, b / 20.0),
            Unit::Linear if b <= SILENT_DB => 0.0,
            Unit::Linear => libm::pow(10.0, b / 20.0),
        })
    }

    /// `v` on this unit's steps: whole pixels and frames, else as shown
    pub fn round(self, v: f64) -> f64 {
        let k = libm::pow(10.0, self.decimals() as f64);
        libm::round(v * k) / k
    }
}

/// the vertical field of view (degrees) a focal length `mm` sees on the full-frame sensor
pub fn fov_deg(mm: f64) -> f64 {
    (2.0 * libm::atan(SENSOR_H_MM / (2.0 * mm.max(1e-3)))).to_degrees()
}

/// the focal length (mm) that sees a vertical field of view of `deg` degrees on the full-frame sensor
pub fn focal_mm(deg: f64) -> f64 {
    SENSOR_H_MM / (2.0 * libm::tan(deg.to_radians() * 0.5).max(1e-6))
}

/// `v` from one unit into another of the same dimension (none: another dimension, or a reference not known)
pub fn convert(v: f64, from: Unit, to: Unit, refs: &Refs) -> Option<f64> {
    if from == to {
        return Some(v);
    }
    if from.dimension() != to.dimension() {
        return None;
    }
    to.from_base(from.to_base(v, refs)?, refs)
}

/// The unit a value is shown in: the one chosen for it, if it can be offered, else `fallback` if it can, else the
/// first its dimension offers.
pub fn pick(chosen: Option<Unit>, fallback: Unit, refs: &Refs) -> Unit {
    let dim = fallback.dimension();
    let fits = |u: &Unit| u.dimension() == dim && u.known(refs);
    chosen.filter(fits).or(Some(fallback).filter(fits)).or_else(|| dim.offered(refs).first().copied()).unwrap_or(fallback)
}

/// `v` (in `unit`) as its number on the unit's steps, a minus sign, no word
pub fn number(v: f64, unit: Unit) -> String {
    let v = unit.round(v);
    let v = if v == 0.0 { 0.0 } else { v };
    format!("{v:.*}", unit.decimals()).replace('-', "−")
}

/// `v` (in `unit`) as a person reads it: "12 px", "0.50 s", "−3.0 dB", "27°"
pub fn show(v: f64, unit: Unit) -> String {
    match unit {
        Unit::Degrees => format!("{}°", number(v, unit)),
        Unit::Decibels if unit.round(v) > 0.0 => format!("+{} dB", number(v, unit)),
        _ => format!("{} {}", number(v, unit), unit.word()),
    }
}

/// A typed value: a number (a comma for the point, a minus either way), optionally followed by a unit of `unit`'s
/// dimension that it is then in; else it is in `unit`.
pub fn parse(text: &str, unit: Unit) -> Option<(f64, Unit)> {
    let t = text.trim().replace('−', "-").replace(',', ".");
    let split = t.find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | '+'))).unwrap_or(t.len());
    let (num, word) = t.split_at(split);
    let v: f64 = num.trim().parse().ok().filter(|v: &f64| v.is_finite())?;
    let word = word.trim();
    if word.is_empty() {
        return Some((v, unit));
    }
    // "%" is both a length's and a level's word: within one dimension it names one unit
    unit.dimension().units().iter().copied().find(|u| u.word().eq_ignore_ascii_case(word) || u.id().eq_ignore_ascii_case(word)).map(|u| (v, u))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9 * a.abs().max(b.abs()).max(1.0)
    }

    fn every() -> Refs {
        Refs { fps: Some(25.0), bpm: Some(120.0), per_bar: Some(4), pixels: Some(1920.0), web: true, em: false }
    }

    #[test]
    fn every_unit_goes_there_and_back() {
        let refs = every();
        for dim in Dimension::ALL {
            for &a in dim.units() {
                for &b in dim.units() {
                    for v in [0.5, 1.0, 17.0, 120.0] {
                        let there = convert(v, a, b, &refs).unwrap();
                        let back = convert(there, b, a, &refs).unwrap();
                        assert!(close(back, v), "{v} {} → {there} {} → {back}", a.id(), b.id());
                    }
                }
            }
        }
    }

    #[test]
    fn pixels_and_percent_are_parts_of_the_source() {
        let refs = Refs { pixels: Some(1920.0), ..Refs::default() };
        assert_eq!(convert(10.0, Unit::Percent, Unit::Pixels, &refs), Some(192.0));
        assert_eq!(Unit::Pixels.from_base(0.1, &refs), Some(192.0), "a crop of a tenth of 1920 px");
        assert_eq!(Unit::Pixels.to_base(480.0, &refs), Some(0.25));
        assert_eq!(Unit::Pixels.round(Unit::Pixels.from_base(0.123, &refs).unwrap()), 236.0, "whole source pixels");
        assert_eq!(convert(1.0, Unit::Pixels, Unit::Percent, &Refs::default()), None, "no picture: no pixels");
        assert_eq!(Dimension::Length.offered(&Refs::default()), [Unit::Percent]);
    }

    #[test]
    fn type_units_are_css_pixels_of_a_web_reference() {
        let slice = Refs { pixels: Some(390.0), web: true, ..Refs::default() };
        assert_eq!(Dimension::Length.offered(&slice), [Unit::Pixels, Unit::Percent, Unit::Points, Unit::Rem]);
        assert_eq!(convert(10.0, Unit::Percent, Unit::Pixels, &slice), Some(39.0));
        assert!(close(convert(39.0, Unit::Pixels, Unit::Rem, &slice).unwrap(), 2.4375));
        assert!(close(convert(12.0, Unit::Points, Unit::Pixels, &slice).unwrap(), 16.0));
        let font = Refs { pixels: Some(16.0), web: true, em: true, ..Refs::default() };
        assert_eq!(Dimension::Length.offered(&font), [Unit::Pixels, Unit::Em, Unit::Points, Unit::Rem]);
        assert!(close(convert(0.5, Unit::Em, Unit::Pixels, &font).unwrap(), 8.0));
        assert_eq!(Dimension::Length.offered(&Refs { pixels: Some(1920.0), ..Refs::default() }), [Unit::Pixels, Unit::Percent], "a picture has no pt / rem");
    }

    #[test]
    fn a_distance_is_metres_or_centimetres_or_feet_and_needs_no_reference() {
        let none = Refs::default();
        assert_eq!(Dimension::Distance.offered(&none), [Unit::Metres, Unit::Centimetres, Unit::Feet]);
        assert_eq!(convert(1.8, Unit::Metres, Unit::Centimetres, &none).map(|v| Unit::Centimetres.round(v)), Some(180.0));
        assert!(close(convert(3.0, Unit::Feet, Unit::Metres, &none).unwrap(), 0.9144));
        assert_eq!(number(convert(1.8, Unit::Metres, Unit::Feet, &none).unwrap(), Unit::Feet), "5.91");
        assert_eq!(convert(1.0, Unit::Metres, Unit::Percent, &none), None, "a world distance is no part of a picture");
        assert_eq!(parse("180 cm", Unit::Metres), Some((180.0, Unit::Centimetres)));
        assert_eq!(parse("6ft", Unit::Metres), Some((6.0, Unit::Feet)));
        assert_eq!(show(1.8, Unit::Metres), "1.80 m");
    }

    #[test]
    fn beats_and_bars_are_seconds_at_a_tempo() {
        let refs = Refs { bpm: Some(120.0), per_bar: Some(4), ..Refs::default() };
        assert_eq!(convert(4.0, Unit::Beats, Unit::Seconds, &refs), Some(2.0));
        assert_eq!(convert(2.0, Unit::Bars, Unit::Seconds, &refs), Some(4.0));
        assert_eq!(convert(6.0, Unit::Beats, Unit::Bars, &Refs { per_bar: Some(3), ..refs }), Some(2.0), "three beats a bar");
        assert_eq!(convert(1.0, Unit::Bars, Unit::Beats, &Refs { bpm: Some(90.0), ..Refs::default() }), Some(4.0), "four a bar when not known");
        assert_eq!(convert(1.0, Unit::Beats, Unit::Seconds, &Refs::default()), None, "no tempo: no beats");
        assert_eq!(convert(2.0, Unit::Seconds, Unit::Frames, &Refs { fps: Some(24.0), ..Refs::default() }), Some(48.0));
        assert_eq!(convert(250.0, Unit::Milliseconds, Unit::Seconds, &Refs::default()), Some(0.25));
    }

    #[test]
    fn a_focal_length_is_the_angle_it_sees() {
        assert!((fov_deg(24.0) - 53.130_102).abs() < 1e-5, "24 mm on a 24 mm sensor: 2·atan(½)");
        assert!(close(focal_mm(fov_deg(50.0)), 50.0));
        let deg = convert(50.0, Unit::Millimetres, Unit::Degrees, &Refs::default()).unwrap();
        assert_eq!(number(deg, Unit::Degrees), "27");
    }

    #[test]
    fn a_level_is_db_or_its_gain() {
        let refs = Refs::default();
        assert!(close(convert(0.0, Unit::Decibels, Unit::Gain, &refs).unwrap(), 100.0));
        assert!((convert(-6.0, Unit::Decibels, Unit::Gain, &refs).unwrap() - 50.118_723).abs() < 1e-5);
        assert_eq!(convert(0.0, Unit::Gain, Unit::Decibels, &refs), Some(SILENT_DB), "silence is the floor, not −∞");
        assert!(close(convert(0.5, Unit::Linear, Unit::Gain, &refs).unwrap(), 50.0), "a sample's volume 0..1");
        assert_eq!(convert(0.0, Unit::Linear, Unit::Decibels, &refs), Some(SILENT_DB));
    }

    #[test]
    fn a_unit_not_known_falls_back() {
        let none = Refs::default();
        assert_eq!(pick(Some(Unit::Pixels), Unit::Percent, &none), Unit::Percent);
        assert_eq!(pick(Some(Unit::Pixels), Unit::Percent, &every()), Unit::Pixels);
        assert_eq!(pick(Some(Unit::Decibels), Unit::Percent, &every()), Unit::Percent, "another dimension's choice is no choice");
        assert_eq!(pick(None, Unit::Beats, &none), Unit::Seconds, "no tempo: the first time unit");
    }

    #[test]
    fn a_value_reads_and_types_in_its_unit() {
        assert_eq!(show(192.4, Unit::Pixels), "192 px");
        assert_eq!(show(-3.0, Unit::Decibels), "−3.0 dB");
        assert_eq!(show(3.0, Unit::Decibels), "+3.0 dB");
        assert_eq!(show(-0.001, Unit::Percent), "0.0 %");
        assert_eq!(show(27.0, Unit::Degrees), "27°");
        assert_eq!(parse("12,5", Unit::Percent), Some((12.5, Unit::Percent)));
        assert_eq!(parse("300 px", Unit::Percent), Some((300.0, Unit::Pixels)));
        assert_eq!(parse("−6 dB", Unit::Gain), Some((-6.0, Unit::Decibels)));
        assert_eq!(parse("2 bars", Unit::Seconds), Some((2.0, Unit::Bars)));
        assert_eq!(parse("40°", Unit::Millimetres), Some((40.0, Unit::Degrees)));
        assert_eq!(parse("3 dB", Unit::Seconds), None, "a unit of another dimension");
        assert_eq!(parse("fast", Unit::Seconds), None);
    }

    #[test]
    fn the_wire_form_leaves_out_what_is_not_known() {
        assert_eq!(Refs::from_wire(&[25.0, 120.0, 3.0, 1920.0, f64::NAN]), Refs { fps: Some(25.0), bpm: Some(120.0), per_bar: Some(3), pixels: Some(1920.0), web: false, em: false });
        assert_eq!(Refs::from_wire(&[f64::NAN, f64::NAN, f64::NAN, 390.0, f64::NAN, 1.0, 1.0]), Refs { pixels: Some(390.0), web: true, em: true, ..Refs::default() });
        assert_eq!(Refs::from_wire(&[0.0, -1.0]), Refs::default());
    }

    #[test]
    fn ids_name_one_unit_each() {
        for u in Unit::ALL {
            assert_eq!(Unit::of_id(u.id()), Some(u));
            assert!(u.dimension().units().contains(&u));
        }
        for d in Dimension::ALL {
            assert_eq!(Dimension::of_id(d.id()), Some(d));
        }
    }
}
