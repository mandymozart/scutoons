//! Speaker layouts as data, and the standard down-mix tables (plan 2.6, #318). A layout is a name and a list of speakers
//! (`az`imuth in degrees, 0 = front and positive clockwise = to the right; `el`evation up; `dist`ance; `lfe` = a
//! low-frequency channel no panner feeds). The built-ins are a table here; a deck configuration supplies more (a venue's
//! array) and a node may carry its own `az` / `el` / `dist` lists – panners and the fold read them, plain routing never
//! does.
//!
//! The tables are the spec's: the Web Audio API's "up-mixing and down-mixing" matrices for mono, stereo, quad and 5.1, whose
//! 5.1 → stereo row is ITU-R BS.775-3's (Lo = L + C/√2 + Ls/√2, the LFE dropped). Channel orders: mono M · stereo L R ·
//! quad L R Ls Rs · 5.1 L R C LFE Ls Rs · 7.1 L R C LFE Lb Rb Ls Rs. A matrix is `rows[to][from]`.
use serde::{Deserialize, Serialize};
use std::f32::consts::FRAC_1_SQRT_2 as H;

/// One speaker: where it stands around the listener.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Speaker {
    /// degrees clockwise from the front (right is positive)
    pub az: f32,
    /// degrees above the horizon
    pub el: f32,
    /// distance from the listener, in units of the layout's radius
    #[serde(default = "one")]
    pub dist: f32,
    /// a low-frequency channel: panners give it nothing
    #[serde(default)]
    pub lfe: bool,
}

fn one() -> f32 {
    1.0
}

const fn sp(az: f32) -> Speaker {
    Speaker { az, el: 0.0, dist: 1.0, lfe: false }
}
const LFE: Speaker = Speaker { az: 0.0, el: -30.0, dist: 1.0, lfe: true };

/// A named set of speakers in channel order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    pub name: String,
    pub speakers: Vec<Speaker>,
}

impl Layout {
    pub fn new(name: &str, speakers: &[Speaker]) -> Layout {
        Layout { name: name.to_string(), speakers: speakers.to_vec() }
    }

    pub fn channels(&self) -> usize {
        self.speakers.len()
    }
}

/// the built-in layouts, by name
pub const BUILTIN: [&str; 5] = ["mono", "stereo", "quad", "5.1", "7.1"];

/// a built-in layout by name
pub fn builtin(name: &str) -> Option<Layout> {
    let s: &[Speaker] = match name {
        "mono" => &[sp(0.0)],
        "stereo" => &[sp(-30.0), sp(30.0)],
        "quad" => &[sp(-45.0), sp(45.0), sp(-135.0), sp(135.0)],
        "5.1" => &[sp(-30.0), sp(30.0), sp(0.0), LFE, sp(-110.0), sp(110.0)],
        "7.1" => &[sp(-30.0), sp(30.0), sp(0.0), LFE, sp(-150.0), sp(150.0), sp(-90.0), sp(90.0)],
        _ => return None,
    };
    Some(Layout::new(name, s))
}

/// the built-in layout with `n` channels, if there is one
pub fn name_for(n: usize) -> Option<&'static str> {
    Some(match n {
        1 => "mono",
        2 => "stereo",
        4 => "quad",
        6 => "5.1",
        8 => "7.1",
        _ => return None,
    })
}

/// the gain matrix that mixes layout `from` down (or up) to `to` – the spec's tables, `rows[to channel][from channel]`;
/// None where the spec has none (7.1 and anything custom: give a `matrix`)
pub fn matrix(from: &str, to: &str) -> Option<Vec<Vec<f32>>> {
    let rows: &[&[f32]] = match (from, to) {
        (a, b) if a == b && BUILTIN.contains(&a) => return Some(identity(builtin(a)?.channels())),
        ("mono", "stereo") => &[&[1.0], &[1.0]],
        ("mono", "quad") => &[&[1.0], &[1.0], &[0.0], &[0.0]],
        ("mono", "5.1") => &[&[0.0], &[0.0], &[1.0], &[0.0], &[0.0], &[0.0]],
        ("stereo", "mono") => &[&[0.5, 0.5]],
        ("stereo", "quad") => &[&[1.0, 0.0], &[0.0, 1.0], &[0.0, 0.0], &[0.0, 0.0]],
        ("stereo", "5.1") => &[&[1.0, 0.0], &[0.0, 1.0], &[0.0, 0.0], &[0.0, 0.0], &[0.0, 0.0], &[0.0, 0.0]],
        ("quad", "mono") => &[&[0.25, 0.25, 0.25, 0.25]],
        ("quad", "stereo") => &[&[0.5, 0.0, 0.5, 0.0], &[0.0, 0.5, 0.0, 0.5]],
        ("quad", "5.1") => &[
            &[1.0, 0.0, 0.0, 0.0],
            &[0.0, 1.0, 0.0, 0.0],
            &[0.0, 0.0, 0.0, 0.0],
            &[0.0, 0.0, 0.0, 0.0],
            &[0.0, 0.0, 1.0, 0.0],
            &[0.0, 0.0, 0.0, 1.0],
        ],
        ("5.1", "mono") => &[&[H, H, 1.0, 0.0, 0.5, 0.5]],
        ("5.1", "stereo") => &[&[1.0, 0.0, H, 0.0, H, 0.0], &[0.0, 1.0, H, 0.0, 0.0, H]],
        ("5.1", "quad") => &[
            &[1.0, 0.0, H, 0.0, 0.0, 0.0],
            &[0.0, 1.0, H, 0.0, 0.0, 0.0],
            &[0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            &[0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
        ],
        _ => return None,
    };
    Some(rows.iter().map(|r| r.to_vec()).collect())
}

/// `n` × `n` identity
pub fn identity(n: usize) -> Vec<Vec<f32>> {
    (0..n).map(|r| (0..n).map(|c| if r == c { 1.0 } else { 0.0 }).collect()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// the tables against the spec, number by number
    #[test]
    fn the_down_mix_tables_are_the_specs() {
        // ITU-R BS.775-3 and Web Audio: Lo = L + 0.7071 C + 0.7071 Ls, Ro = R + 0.7071 C + 0.7071 Rs, the LFE dropped
        let m = matrix("5.1", "stereo").unwrap();
        assert_eq!(m[0], [1.0, 0.0, 0.707_106_77, 0.0, 0.707_106_77, 0.0]);
        assert_eq!(m[1], [0.0, 1.0, 0.707_106_77, 0.0, 0.0, 0.707_106_77]);
        // 5.1 → mono: 0.7071 (L + R) + C + 0.5 (Ls + Rs)
        assert_eq!(matrix("5.1", "mono").unwrap(), vec![vec![0.707_106_77, 0.707_106_77, 1.0, 0.0, 0.5, 0.5]]);
        // 5.1 → quad: L + 0.7071 C, R + 0.7071 C, Ls, Rs
        let q = matrix("5.1", "quad").unwrap();
        assert_eq!(q[0], [1.0, 0.0, 0.707_106_77, 0.0, 0.0, 0.0]);
        assert_eq!(q[3], [0.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
        // quad → stereo: 0.5 (L + Ls), 0.5 (R + Rs); quad → mono: a quarter of each
        assert_eq!(matrix("quad", "stereo").unwrap(), vec![vec![0.5, 0.0, 0.5, 0.0], vec![0.0, 0.5, 0.0, 0.5]]);
        assert_eq!(matrix("quad", "mono").unwrap(), vec![vec![0.25; 4]]);
        // stereo → mono: 0.5 (L + R); the up-mixes copy (mono → stereo twice, mono → 5.1 on the centre)
        assert_eq!(matrix("stereo", "mono").unwrap(), vec![vec![0.5, 0.5]]);
        assert_eq!(matrix("mono", "stereo").unwrap(), vec![vec![1.0], vec![1.0]]);
        assert_eq!(matrix("mono", "5.1").unwrap()[2], [1.0]);
        // every row has the width of its `from` layout and there is a row per `to` channel
        for a in BUILTIN {
            for b in BUILTIN {
                if let Some(m) = matrix(a, b) {
                    assert_eq!(m.len(), builtin(b).unwrap().channels(), "{a} → {b}");
                    assert!(m.iter().all(|r| r.len() == builtin(a).unwrap().channels()), "{a} → {b}");
                }
            }
        }
        assert!(matrix("7.1", "stereo").is_none(), "the spec has no 7.1 table: a custom matrix");
    }

    #[test]
    fn the_built_in_layouts_have_their_speakers() {
        assert_eq!(builtin("stereo").unwrap().speakers.iter().map(|s| s.az).collect::<Vec<_>>(), [-30.0, 30.0]);
        let l = builtin("5.1").unwrap();
        assert_eq!((l.channels(), l.speakers[3].lfe, l.speakers.iter().filter(|s| s.lfe).count()), (6, true, 1));
        for n in [1, 2, 4, 6, 8] {
            assert_eq!(builtin(name_for(n).unwrap()).unwrap().channels(), n);
        }
    }
}
