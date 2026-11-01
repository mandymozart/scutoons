//! Fixture profiles (#211): a small TOML per device – its channel names → numbers (1-based from the fixture's start
//! address, 8 bit or a coarse / fine pair for 16 bit) and, for a strobe, which channel flashes and what the others
//! rest at. A scene places a profile at an address; mappings then name channels (`strobe.dimmer`), never numbers.
//! The generic strobe (`fixtures/strobe.toml`) and the Cameo Thunder Wash 600 RGBW (7CH_1) are built in.

use std::collections::BTreeMap;

use serde::Deserialize;

/// The built-in generic strobe: dimmer 1, rate 2, duration 3.
pub const STROBE: &str = include_str!("../fixtures/strobe.toml");

/// The Cameo Thunder Wash 600 RGBW in DMX mode 4CH_1: red 1, green 2, blue 3, white 4 (no strobe channel).
pub const THUNDERWASH_600_RGBW: &str = include_str!("../fixtures/thunderwash-600-rgbw.toml");

/// The built-in profiles by name.
pub fn builtin(name: &str) -> Option<&'static str> {
    match name {
        "strobe" => Some(STROBE),
        "thunderwash-600-rgbw" => Some(THUNDERWASH_600_RGBW),
        _ => None,
    }
}

/// A channel of a profile: 8 bit, or 16 bit with its fine channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Channel {
    /// 1-based offset from the fixture's address
    pub coarse: u16,
    pub fine: Option<u16>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StrobeSpec {
    /// the channel a flash opens
    pub flash: String,
    /// the flash's full value (0..255)
    pub level: u8,
    /// the other channels' values while the deck drives the flashes (the fixture's own oscillator off)
    pub rest: BTreeMap<String, u8>,
    /// an optional cap: flashes per second (off by default – owner 2026-10-31)
    pub max_hz: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub name: String,
    pub channels: BTreeMap<String, Channel>,
    pub strobe: Option<StrobeSpec>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    name: String,
    channels: BTreeMap<String, RawChannel>,
    strobe: Option<RawStrobe>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawChannel {
    Eight(i64),
    Sixteen { coarse: i64, fine: i64 },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStrobe {
    flash: String,
    #[serde(default = "full")]
    level: i64,
    #[serde(default)]
    rest: BTreeMap<String, i64>,
    max_hz: Option<f64>,
}

fn full() -> i64 {
    255
}

fn offset(name: &str, n: i64) -> Result<u16, String> {
    u16::try_from(n).ok().filter(|n| (1..=512).contains(n)).ok_or_else(|| format!("channel '{name}' = {n}: a channel is 1…512"))
}

impl Profile {
    /// Parse and check a profile; the error says what is wrong and where.
    pub fn parse(text: &str) -> Result<Profile, String> {
        let raw: Raw = toml::from_str(text).map_err(|e| format!("not a fixture profile: {}", e.message()))?;
        if raw.channels.is_empty() {
            return Err(format!("profile '{}' has no channels", raw.name));
        }
        let mut channels = BTreeMap::new();
        let mut used: BTreeMap<u16, String> = BTreeMap::new();
        for (name, c) in raw.channels {
            let ch = match c {
                RawChannel::Eight(n) => Channel { coarse: offset(&name, n)?, fine: None },
                RawChannel::Sixteen { coarse, fine } => Channel { coarse: offset(&name, coarse)?, fine: Some(offset(&name, fine)?) },
            };
            for n in [Some(ch.coarse), ch.fine].into_iter().flatten() {
                if let Some(other) = used.insert(n, name.clone()) {
                    return Err(format!("profile '{}': channel {n} is both '{other}' and '{name}'", raw.name));
                }
            }
            channels.insert(name, ch);
        }
        let names = |c: &BTreeMap<String, Channel>| c.keys().cloned().collect::<Vec<_>>().join(", ");
        let strobe = match raw.strobe {
            None => None,
            Some(s) => {
                if !channels.contains_key(&s.flash) {
                    return Err(format!("profile '{}': strobe.flash '{}' is not one of its channels ({})", raw.name, s.flash, names(&channels)));
                }
                let level = u8::try_from(s.level).map_err(|_| format!("profile '{}': strobe.level {} is not 0…255", raw.name, s.level))?;
                let mut rest = BTreeMap::new();
                for (c, v) in s.rest {
                    if !channels.contains_key(&c) {
                        return Err(format!("profile '{}': strobe.rest '{c}' is not one of its channels ({})", raw.name, names(&channels)));
                    }
                    if c == s.flash {
                        return Err(format!("profile '{}': strobe.rest names the flash channel '{c}'", raw.name));
                    }
                    let v = u8::try_from(v).map_err(|_| format!("profile '{}': strobe.rest {c} = {v} is not 0…255", raw.name))?;
                    rest.insert(c, v);
                }
                if let Some(hz) = s.max_hz {
                    if !(hz.is_finite() && hz > 0.0) {
                        return Err(format!("profile '{}': strobe.max_hz {hz} must be above 0 (leave it out for no cap)", raw.name));
                    }
                }
                Some(StrobeSpec { flash: s.flash, level, rest, max_hz: s.max_hz })
            }
        };
        Ok(Profile { name: raw.name, channels, strobe })
    }

    /// the highest offset any channel uses (the fixture's footprint)
    pub fn footprint(&self) -> u16 {
        self.channels.values().flat_map(|c| [Some(c.coarse), c.fine]).flatten().max().unwrap_or(0)
    }

    pub fn channel_names(&self) -> String {
        self.channels.keys().cloned().collect::<Vec<_>>().join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_generic_strobe() {
        let p = Profile::parse(STROBE).unwrap();
        assert_eq!(p.name, "strobe");
        assert_eq!(p.channels["dimmer"], Channel { coarse: 1, fine: None });
        assert_eq!(p.channels["rate"].coarse, 2);
        assert_eq!(p.channels["duration"].coarse, 3);
        assert_eq!(p.footprint(), 3);
        let s = p.strobe.unwrap();
        assert_eq!((s.flash.as_str(), s.level, s.max_hz), ("dimmer", 255, None), "no cap by default");
        assert_eq!(s.rest["rate"], 0);
        assert!(builtin("strobe").is_some() && builtin("moving-head").is_none());
    }

    #[test]
    fn the_thunderwash_in_4ch_1() {
        let p = Profile::parse(THUNDERWASH_600_RGBW).unwrap();
        let at = |n: &str| p.channels[n].coarse;
        assert_eq!([at("red"), at("green"), at("blue"), at("white")], [1, 2, 3, 4]);
        assert_eq!(p.footprint(), 4, "four channels, none of them a strobe or a mode channel");
        assert!(p.strobe.is_none());
        assert!(builtin("thunderwash-600-rgbw").is_some());
    }

    #[test]
    fn sixteen_bit_channels() {
        let p = Profile::parse("name = \"head\"\n[channels]\npan = { coarse = 1, fine = 2 }\ndimmer = 5\n").unwrap();
        assert_eq!(p.channels["pan"], Channel { coarse: 1, fine: Some(2) });
        assert_eq!(p.footprint(), 5);
        assert!(p.strobe.is_none());
    }

    #[test]
    fn errors_name_the_problem() {
        let err = |t: &str| Profile::parse(t).unwrap_err();
        assert!(err("name = \"x\"\n[channels]\na = 0\n").contains("1…512"));
        assert!(err("name = \"x\"\n[channels]\na = 1\nb = 1\n").contains("channel 1 is both 'a' and 'b'"));
        assert!(err("name = \"x\"\n[channels]\npan = { coarse = 1, fine = 1 }\n").contains("channel 1 is both"));
        let e = err("name = \"x\"\n[channels]\ndimmer = 1\n[strobe]\nflash = \"dimer\"\n");
        assert!(e.contains("'dimer'") && e.contains("dimmer"), "{e}");
        assert!(err("name = \"x\"\n[channels]\nd = 1\n[strobe]\nflash = \"d\"\nlevel = 300\n").contains("0…255"));
        assert!(err("name = \"x\"\n[channels]\nd = 1\n[strobe]\nflash = \"d\"\nrest = { d = 0 }\n").contains("flash channel"));
        assert!(err("name = \"x\"\n[channels]\nd = 1\n[strobe]\nflash = \"d\"\nmax_hz = 0\n").contains("above 0"));
        assert!(err("name = \"x\"\n[channels]\nd = 1\nspeed = 2\n[strobe]\nflash = \"d\"\nbogus = 1\n").contains("not a fixture profile"));
        assert!(err("name = \"x\"\n[channels]\n").contains("no channels"));
    }
}
