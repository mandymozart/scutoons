//! The deck's general-purpose loader (decisions/deck.md [media]): a medium → the file for the deck's
//! quality (deck-catalog's one pick rule) → its bytes from the deck's cache or the host's fetch → decoded for the
//! consumer (a module's asset slot first), always off the audio thread. Offline the nearest file the cache holds plays.
//! Where the bytes live is the host's (`Files`: deck-native's disk cache and CDN, a page's fetch).
use crate::{err, Error};
use deck_catalog::rendition::{Media, Quality};

/// Where a host keeps and gets a medium's files (paths as the content writes them).
pub trait Files {
    /// the file's bytes when the deck holds it
    fn cached(&self, path: &str) -> Option<Vec<u8>>;
    /// the file's bytes from where the deck fetches (the host keeps them); Err offline or when it is not there
    fn fetch(&self, path: &str) -> Result<Vec<u8>, String>;
}

/// Where a file came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Cache,
    Fetched,
}

/// The file a medium loaded as.
#[derive(Debug, Clone, PartialEq)]
pub struct Got {
    pub path: String,
    pub bytes: Vec<u8>,
    pub from: Source,
    /// it is not the file the quality picks (that one could not be had)
    pub fallback: bool,
}

/// The file for `q`: the picked one, cached or fetched; else the nearest one the cache holds (offline); else the others
/// fetched in the rule's order. Err names the picked file and why it could not be had.
pub fn load(media: &Media, q: Quality, files: &dyn Files) -> Result<Got, Error> {
    let want = media.pick(q);
    if want.is_empty() {
        return err("the medium names no file");
    }
    let got = |path: &str, bytes, from| Got { path: path.to_string(), bytes, from, fallback: path != want };
    if let Some(b) = files.cached(want) {
        return Ok(got(want, b, Source::Cache));
    }
    let why = match files.fetch(want) {
        Ok(b) => return Ok(got(want, b, Source::Fetched)),
        Err(e) => e,
    };
    let others: Vec<&str> = media.candidates(q).into_iter().filter(|p| *p != want).collect();
    if let Some((p, b)) = others.iter().find_map(|p| Some((*p, files.cached(p)?))) {
        return Ok(got(p, b, Source::Cache));
    }
    if let Some((p, b)) = others.iter().find_map(|p| Some((*p, files.fetch(p).ok()?))) {
        return Ok(got(p, b, Source::Fetched));
    }
    err(format!("{want}: {why}"))
}

/// A sound as a module's `pcm` slot takes it: planar f32, a channel per entry, at the file's own rate.
#[derive(Debug, Clone, PartialEq)]
pub struct Planar {
    pub rate: f32,
    pub channels: Vec<Vec<f32>>,
}

/// a whole sound file decoded and split into its channels (resampling is the consumer's)
pub fn planar(bytes: Vec<u8>) -> Result<Planar, Error> {
    let Some(pcm) = crate::audio::decode(bytes)? else { return err("the file has no sound") };
    let n = usize::from(pcm.channels.max(1));
    if pcm.samples.is_empty() {
        return err("no samples decoded");
    }
    let channels = (0..n).map(|c| pcm.samples.iter().skip(c).step_by(n).copied().collect()).collect();
    Ok(Planar { rate: pcm.rate as f32, channels })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    struct Disk {
        cache: HashMap<String, Vec<u8>>,
        cdn: Option<HashMap<String, Vec<u8>>>,
        fetched: RefCell<Vec<String>>,
    }

    impl Files for Disk {
        fn cached(&self, path: &str) -> Option<Vec<u8>> {
            self.cache.get(path).cloned()
        }
        fn fetch(&self, path: &str) -> Result<Vec<u8>, String> {
            self.fetched.borrow_mut().push(path.to_string());
            let Some(cdn) = &self.cdn else { return Err("offline".into()) };
            cdn.get(path).cloned().ok_or_else(|| "404".into())
        }
    }

    fn disk(cache: &[&str], cdn: Option<&[&str]>) -> Disk {
        let map = |l: &[&str]| l.iter().map(|p| (p.to_string(), p.as_bytes().to_vec())).collect();
        Disk { cache: map(cache), cdn: cdn.map(map), fetched: RefCell::new(Vec::new()) }
    }

    fn ir() -> Media {
        Media::of_files(&["irs/plate.96.mp3", "irs/plate.128.mp3", "irs/plate.320.mp3"])
    }

    #[test]
    fn low_and_high_load_different_renditions() {
        let d = disk(&[], Some(&["irs/plate.96.mp3", "irs/plate.128.mp3", "irs/plate.320.mp3"]));
        let (low, high) = (load(&ir(), Quality::Low, &d).unwrap(), load(&ir(), Quality::High, &d).unwrap());
        assert_eq!((low.path.as_str(), high.path.as_str()), ("irs/plate.96.mp3", "irs/plate.320.mp3"));
        assert_eq!((low.from, low.fallback), (Source::Fetched, false));
    }

    #[test]
    fn a_cached_pick_is_not_fetched_again() {
        let d = disk(&["irs/plate.320.mp3"], Some(&[]));
        let g = load(&ir(), Quality::High, &d).unwrap();
        assert_eq!((g.from, d.fetched.borrow().len()), (Source::Cache, 0));
    }

    #[test]
    fn a_missing_level_falls_back_to_the_nearest_there() {
        let d = disk(&[], Some(&["irs/plate.96.mp3", "irs/plate.320.mp3"]));
        let g = load(&ir(), Quality::Medium, &d).unwrap();
        assert_eq!((g.path.as_str(), g.fallback), ("irs/plate.96.mp3", true));
    }

    #[test]
    fn offline_the_best_cached_rendition_plays() {
        let d = disk(&["irs/plate.128.mp3", "irs/plate.320.mp3"], None);
        let g = load(&ir(), Quality::High, &d).unwrap();
        assert_eq!((g.path.as_str(), g.from), ("irs/plate.320.mp3", Source::Cache));
        let g = load(&ir(), Quality::Low, &d).unwrap();
        assert_eq!((g.path.as_str(), g.from, g.fallback), ("irs/plate.128.mp3", Source::Cache, true), "low not cached: the nearest the deck holds");
        let none = load(&ir(), Quality::Low, &disk(&[], None)).unwrap_err();
        assert_eq!(none.0, "irs/plate.96.mp3: offline");
    }

    #[test]
    fn a_wav_splits_into_its_channels() {
        let p = planar(wav_16(22_050, 2, &[0, 16_384, -16_384, 8_192])).unwrap();
        assert_eq!((p.rate, p.channels.len(), p.channels[0].len()), (22_050.0, 2, 2));
        assert!((p.channels[1][0] - 0.5).abs() < 1e-3 && (p.channels[0][1] + 0.5).abs() < 1e-3);
        assert!(planar(b"not a sound".to_vec()).is_err());
    }

    fn wav_16(rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
        let data = samples.len() as u32 * 2;
        let mut v = Vec::new();
        v.extend(b"RIFF");
        v.extend((36 + data).to_le_bytes());
        v.extend(b"WAVEfmt ");
        v.extend(16u32.to_le_bytes());
        v.extend(1u16.to_le_bytes());
        v.extend(channels.to_le_bytes());
        v.extend(rate.to_le_bytes());
        v.extend((rate * 2 * u32::from(channels)).to_le_bytes());
        v.extend((2 * channels).to_le_bytes());
        v.extend(16u16.to_le_bytes());
        v.extend(b"data");
        v.extend(data.to_le_bytes());
        for s in samples {
            v.extend(s.to_le_bytes());
        }
        v
    }
}
