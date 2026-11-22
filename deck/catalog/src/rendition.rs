//! The one pick rule for a medium's files (RULES I3, agents/ASSETS.md "Grouping", "Audio quality levels"): a medium is a
//! default file and one file per quality level (the content's `{ src, variants }`), or a family of files whose version
//! markers say their level (`.96.mp3` … `.320.mp3`, `.480p` … `.1080p`, `.2x` / `.3x`, `.low` / `.medium` / `.high`).
//! A level turns into a file here, on every deck and – through the wasm – in every page: the level's file, else the
//! nearest lower one, else the default (a reader on low never gets a bigger file because a variant is missing). What a
//! deck has on disk narrows it (`pick_available`): the nearest file it holds, the higher ones last.
use crate::library::family::{main_path, version_name, LEAD};
use crate::library::Kind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The media quality levels, lowest first (the reader's setting, a deck's rung).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    Low,
    Medium,
    High,
}

impl Quality {
    pub const ALL: [Quality; 3] = [Quality::Low, Quality::Medium, Quality::High];

    pub fn name(self) -> &'static str {
        match self {
            Quality::Low => "low",
            Quality::Medium => "medium",
            Quality::High => "high",
        }
    }

    pub fn parse(s: &str) -> Option<Quality> {
        Quality::ALL.into_iter().find(|q| q.name().eq_ignore_ascii_case(s.trim()))
    }

    /// a level a node or a setting names, else medium (the default file's level)
    pub fn or_medium(s: &str) -> Quality {
        Quality::parse(s).unwrap_or(Quality::Medium)
    }
}

/// One medium: its default file and a file per level (paths as the content writes them).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Media {
    pub src: String,
    pub variants: BTreeMap<Quality, String>,
}

impl Media {
    pub fn new(src: &str) -> Media {
        Media { src: src.to_string(), variants: BTreeMap::new() }
    }

    pub fn with(mut self, q: Quality, path: &str) -> Media {
        if !path.is_empty() {
            self.variants.insert(q, path.to_string());
        }
        self
    }

    /// the content's `{ src, variants }`: keys that are no level and empty paths are left out
    pub fn of_variants<'a>(src: &str, variants: impl IntoIterator<Item = (&'a str, &'a str)>) -> Media {
        variants.into_iter().fold(Media::new(src), |m, (k, v)| match Quality::parse(k) {
            Some(q) => m.with(q, v),
            None => m,
        })
    }

    /// A family's files (one asset's renditions): each file's level by its marker (`level_of`), an image's unmarked
    /// file the low level beside densities; the default is the file a slot takes (the Library's `main`).
    pub fn of_files(paths: &[&str]) -> Media {
        let names: Vec<_> = paths.iter().map(|p| version_name(p)).collect();
        let files: Vec<(&str, &str, Kind, &str)> = names.iter().zip(paths).map(|(n, p)| (n.label.as_str(), n.ext.as_str(), n.kind, *p)).collect();
        let kind = LEAD.into_iter().find(|k| files.iter().any(|f| f.2 == *k)).unwrap_or(Kind::File);
        let src = main_path(kind, &files).unwrap_or_default();
        let mut m = Media::new(src);
        for &(_, _, _, p) in files.iter().filter(|f| f.2 == kind) {
            if let Some(q) = level_of(p) {
                m.variants.entry(q).or_insert_with(|| p.to_string());
            }
        }
        let dense = files.iter().any(|f| f.0 == "2x" || f.0 == "3x");
        if kind == Kind::Image && dense && level_of(src).is_none() {
            m.variants.entry(Quality::Low).or_insert_with(|| src.to_string());
        }
        m
    }

    /// the file for `q`: its level's, else the nearest lower level's, else the default
    pub fn pick(&self, q: Quality) -> &str {
        Quality::ALL[..=q as usize].iter().rev().find_map(|l| self.variants.get(l)).unwrap_or(&self.src)
    }

    /// every file in the order a deck tries them for `q`: the level's, the lower ones down, the default, then the higher
    /// ones up – each once
    pub fn candidates(&self, q: Quality) -> Vec<&str> {
        let lower = Quality::ALL[..=q as usize].iter().rev().filter_map(|l| self.variants.get(l));
        let higher = Quality::ALL[q as usize + 1..].iter().filter_map(|l| self.variants.get(l));
        let mut out: Vec<&str> = Vec::new();
        for p in lower.chain(std::iter::once(&self.src)).chain(higher).map(String::as_str).filter(|p| !p.is_empty()) {
            if !out.contains(&p) {
                out.push(p);
            }
        }
        out
    }

    /// the nearest file `has` holds for `q` (the deck's cache offline), None when it holds none
    pub fn pick_available(&self, q: Quality, has: impl Fn(&str) -> bool) -> Option<&str> {
        self.candidates(q).into_iter().find(|p| has(p))
    }
}

/// The level a file stands for by its version marker: audio ≤ 96 kbit/s low, ≤ 160 medium, above high (the registry's
/// grouping, AssetPaths); video ≤ 480p low, ≤ 720p medium, above high; density `.2x` medium, `.3x` high; a level word
/// in the name (`cathedral.low.v1.glb`). None: the file says no level (a source, a width, a poster).
pub fn level_of(path: &str) -> Option<Quality> {
    let v = version_name(path);
    let num = |unit: char| v.label.strip_suffix(unit).and_then(|n| n.parse::<u32>().ok());
    if let Some(k) = num('k').filter(|_| v.kind == Kind::Audio) {
        return Some(match k {
            0..=96 => Quality::Low,
            97..=160 => Quality::Medium,
            _ => Quality::High,
        });
    }
    if let Some(p) = num('p') {
        return Some(match p {
            0..=480 => Quality::Low,
            481..=720 => Quality::Medium,
            _ => Quality::High,
        });
    }
    match v.label.as_str() {
        "2x" => return Some(Quality::Medium),
        "3x" => return Some(Quality::High),
        _ => {}
    }
    let name = path.rsplit('/').next().unwrap_or(path);
    name.split('.').skip(1).find_map(Quality::parse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Quality::*;

    fn audio() -> Media {
        Media::of_variants("s/c.128.mp3", [("low", "s/c.96.mp3"), ("medium", "s/c.128.mp3"), ("high", "s/c.320.mp3")])
    }

    #[test]
    fn a_level_turns_into_its_file_else_the_nearest_lower_else_the_default() {
        let a = audio();
        assert_eq!((a.pick(Low), a.pick(Medium), a.pick(High)), ("s/c.96.mp3", "s/c.128.mp3", "s/c.320.mp3"));
        // high missing: medium; low missing: the default (never a bigger file than the author's)
        let m = Media::of_variants("v/x.720p.mp4", [("medium", "v/x.720p.mp4")]);
        assert_eq!((m.pick(High), m.pick(Low)), ("v/x.720p.mp4", "v/x.720p.mp4"));
        let only_low = Media::new("a.webp").with(Low, "a.webp").with(High, "a.3x.webp");
        assert_eq!(only_low.pick(Medium), "a.webp");
        assert_eq!(Media::of_variants("x", [("ultra", "y"), ("low", "")]), Media::new("x"), "no level, no empty path");
    }

    #[test]
    fn the_candidates_go_down_then_the_default_then_up() {
        let a = audio();
        assert_eq!(a.candidates(Medium), ["s/c.128.mp3", "s/c.96.mp3", "s/c.320.mp3"]);
        assert_eq!(a.candidates(Low), ["s/c.96.mp3", "s/c.128.mp3", "s/c.320.mp3"]);
        assert_eq!(a.candidates(High), ["s/c.320.mp3", "s/c.128.mp3", "s/c.96.mp3"]);
    }

    #[test]
    fn offline_the_nearest_file_the_deck_holds() {
        let a = audio();
        assert_eq!(a.pick_available(High, |p| p.ends_with(".96.mp3")), Some("s/c.96.mp3"));
        assert_eq!(a.pick_available(Low, |p| p.ends_with(".320.mp3")), Some("s/c.320.mp3"), "a higher one only when no lower is there");
        assert_eq!(a.pick_available(High, |p| p != "s/c.320.mp3"), Some("s/c.128.mp3"));
        assert_eq!(a.pick_available(Medium, |_| false), None);
    }

    #[test]
    fn every_media_kind_s_markers_name_a_level() {
        let cases = [
            ("t/x.96.mp3", Some(Low)),
            ("t/x.128.mp3", Some(Medium)),
            ("t/x.160.mp3", Some(Medium)),
            ("t/x.192.mp3", Some(High)),
            ("t/x.320.mp3", Some(High)),
            ("t/x.480p.mp4", Some(Low)),
            ("t/x.720p.mp4", Some(Medium)),
            ("t/x.1080p.mp4", Some(High)),
            ("t/x.2x.webp", Some(Medium)),
            ("t/x.3x.webp", Some(High)),
            ("t/x.webp", None),
            ("t/x.1280w.mp4", None),
            ("t/x.poster.webp", None),
            ("map/sets/cathedral.low.v1.glb", Some(Low)),
            ("map/sets/cathedral.high.glb", Some(High)),
            ("t/high/x.wav", None),
        ];
        for (path, want) in cases {
            assert_eq!(level_of(path), want, "{path}");
        }
    }

    #[test]
    fn a_family_s_files_become_a_medium() {
        let a = Media::of_files(&["c/s.96.mp3", "c/s.320.mp3", "c/s.128.mp3"]);
        assert_eq!(a, audio_at("c/s"));
        let v = Media::of_files(&["v/x.480p.mp4", "v/x.1080p.mp4", "v/x.720p.mp4", "v/x.poster.webp"]);
        assert_eq!((v.src.as_str(), v.pick(Low), v.pick(High)), ("v/x.720p.mp4", "v/x.480p.mp4", "v/x.1080p.mp4"));
        let i = Media::of_files(&["s1/c1/a.webp", "s1/c1/a.2x.webp", "s1/c1/a.3x.webp"]);
        assert_eq!((i.src.as_str(), i.pick(Low), i.pick(Medium), i.pick(High)), ("s1/c1/a.webp", "s1/c1/a.webp", "s1/c1/a.2x.webp", "s1/c1/a.3x.webp"));
        let g = Media::of_files(&["sets/n.low.v1.glb", "sets/n.high.v1.glb"]);
        assert_eq!((g.pick(Low), g.pick(Medium), g.pick(High)), ("sets/n.low.v1.glb", "sets/n.low.v1.glb", "sets/n.high.v1.glb"));
        let lone = Media::of_files(&["irs/plate.wav"]);
        assert_eq!((lone.pick(Low), lone.pick(High)), ("irs/plate.wav", "irs/plate.wav"));
    }

    fn audio_at(base: &str) -> Media {
        Media::new(&format!("{base}.128.mp3")).with(Low, &format!("{base}.96.mp3")).with(Medium, &format!("{base}.128.mp3")).with(High, &format!("{base}.320.mp3"))
    }

    #[test]
    fn a_setting_names_a_level() {
        assert_eq!((Quality::parse(" High "), Quality::parse("ultra"), Quality::or_medium("auto")), (Some(High), None, Medium));
    }
}
