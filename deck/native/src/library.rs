//! scutoons.com for the stage (CYBERDECKS.md §8): the public library API read-only and the CDN files, mirrored into a
//! disk cache so a venue runs offline – `<cache>/api/toons/<toon>/chapters/<n>.json` (refreshed when online, the cached
//! copy when not) and `<cache>/cdn/<toon>/<path>` (CDN paths are immutable – a changed file gets a new name – so a
//! cached file is never fetched again). Decoding: symphonia (mp3, wav).
use crate::audio::{MusicFeed, Track};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

pub use crate::cache::{API, CDN};

#[derive(Clone)]
pub struct Library {
    pub api: String,
    pub cdn: String,
    pub cache: PathBuf,
}

/// The part of a chapter (shared/content/src/content.ts `Chapter`, audio.ts `AudioConfiguration`) the stage reads.
#[derive(Debug, Deserialize)]
pub struct Chapter {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub audio: Option<Audio>,
    #[serde(default)]
    pub groups: Vec<Group>,
}

#[derive(Debug, Deserialize)]
pub struct Group {
    #[serde(default)]
    pub slices: Vec<Slice>,
}

/// One slice of the strip (content.ts `Slice`): the image (CDN path, per quality), its size, the dialog over it.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Slice {
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub image_variants: HashMap<String, String>,
    #[serde(default)]
    pub dialog: Vec<deck_outlet::text::Dialog>,
}

impl Slice {
    pub fn image_path(&self, quality: &str) -> Option<&str> {
        self.image_variants.get(quality).or(self.image.as_ref()).map(String::as_str)
    }
}

#[derive(Debug, Deserialize)]
pub struct Audio {
    #[serde(default)]
    pub music: Option<Music>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Music {
    #[serde(default)]
    pub default_track: Option<String>,
    #[serde(default)]
    pub tracks: Vec<MusicTrack>,
}

#[derive(Debug, Deserialize)]
pub struct MusicTrack {
    pub id: String,
    pub path: String,
    /// media quality → path (low / medium / high)
    #[serde(default)]
    pub variants: HashMap<String, String>,
}

impl Chapter {
    /// the slices with an image, in reading order
    pub fn slices(&self) -> impl Iterator<Item = &Slice> {
        self.groups.iter().flat_map(|g| &g.slices).filter(|s| s.image.is_some() || !s.image_variants.is_empty())
    }

    /// the track `id` (else the chapter's default track, else the first) at `quality` (else its plain path)
    pub fn music_path(&self, id: Option<&str>, quality: &str) -> Option<(&str, &str)> {
        let m = self.audio.as_ref()?.music.as_ref()?;
        let want = id.or(m.default_track.as_deref());
        let t = want.and_then(|w| m.tracks.iter().find(|t| t.id == w)).or(m.tracks.first())?;
        Some((t.id.as_str(), t.variants.get(quality).unwrap_or(&t.path).as_str()))
    }
}

impl Library {
    fn agent() -> ureq::Agent {
        ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(5)).timeout(Duration::from_secs(60)).build()
    }

    pub fn chapter_cache_path(&self, toon: &str, n: u32) -> Result<PathBuf> {
        Ok(self.cache.join("api").join("toons").join(safe_segment(toon)?).join("chapters").join(format!("{n}.json")))
    }

    pub fn file_cache_path(&self, toon: &str, path: &str) -> Result<PathBuf> {
        let mut p = self.cache.join("cdn").join(safe_segment(toon)?);
        for seg in path.split('/') {
            p.push(safe_segment(seg)?);
        }
        Ok(p)
    }

    /// the chapter from the API (written to the cache), or the cached copy when the API cannot be reached
    pub fn chapter(&self, toon: &str, n: u32) -> Result<(Chapter, &'static str)> {
        let cached = self.chapter_cache_path(toon, n)?;
        let url = format!("{}/library/toons/{toon}/chapters/{n}", self.api.trim_end_matches('/'));
        let mut req = Self::agent().get(&url);
        // the deck token once a member approved the deck (#180, deck-link's run directory) – public chapters load without
        if let Some(t) = deck_link::TokenSource::from_env().get() {
            req = req.set("Authorization", &format!("Bearer {t}"));
        }
        match req.call() {
            Ok(r) => {
                let text = r.into_string()?;
                let ch: Chapter = serde_json::from_str(&text).with_context(|| format!("{url}: not a chapter"))?;
                write_atomic(&cached, text.as_bytes())?;
                Ok((ch, "online"))
            }
            Err(e) => {
                let text = std::fs::read_to_string(&cached)
                    .with_context(|| format!("{url}: {e} – and no cached copy at {}", cached.display()))?;
                tracing::warn!("library: {url}: {e} – using the cached chapter");
                Ok((serde_json::from_str(&text)?, "cached"))
            }
        }
    }

    /// a CDN file of the toon's folder, downloaded once into the cache
    pub fn file(&self, toon: &str, path: &str) -> Result<PathBuf> {
        let local = self.file_cache_path(toon, path)?;
        if local.is_file() {
            return Ok(local);
        }
        let url = format!("{}/{toon}/{path}", self.cdn.trim_end_matches('/'));
        let r = Self::agent().get(&url).call().with_context(|| format!("downloading {url}"))?;
        let mut bytes = Vec::new();
        r.into_reader().take(512 << 20).read_to_end(&mut bytes)?;
        write_atomic(&local, &bytes)?;
        Ok(local)
    }
}

/// one path segment from the API / a scene: no separators, no `..`, nothing empty
fn safe_segment(s: &str) -> Result<&str> {
    if s.is_empty() || s == "." || s == ".." || s.contains(['/', '\\', ':']) {
        bail!("'{s}' is not a safe path segment");
    }
    Ok(s)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// decode a whole file (mp3 / wav) into a stereo track at its own rate; mono is doubled, more channels keep L + R
pub fn decode(path: &Path) -> Result<Track> {
    use symphonia::core::{audio::SampleBuffer, codecs::DecoderOptions, errors::Error, formats::FormatOptions,
                          io::MediaSourceStream, meta::MetadataOptions, probe::Hint};
    let file = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe().format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())?;
    let mut format = probed.format;
    let Some(track) = format.default_track() else { bail!("{}: no audio track", path.display()) };
    let id = track.id;
    let rate = track.codec_params.sample_rate.unwrap_or(44_100) as f32;
    let mut dec = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;
    let (mut l, mut r) = (Vec::new(), Vec::new());
    let mut buf: Option<SampleBuffer<f32>> = None;
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        };
        if packet.track_id() != id {
            continue;
        }
        let audio = match dec.decode(&packet) {
            Ok(a) => a,
            Err(Error::DecodeError(e)) => {
                tracing::debug!("decode: skipping a bad packet ({e})");
                continue;
            }
            Err(e) => return Err(e.into()),
        };
        let spec = *audio.spec();
        let ch = spec.channels.count().max(1);
        let sb = buf.get_or_insert_with(|| SampleBuffer::new(audio.capacity() as u64, spec));
        if sb.capacity() < audio.capacity() * ch {
            *sb = SampleBuffer::new(audio.capacity() as u64, spec);
        }
        sb.copy_interleaved_ref(audio);
        for f in sb.samples().chunks_exact(ch) {
            l.push(f[0]);
            r.push(if ch > 1 { f[1] } else { f[0] });
        }
    }
    if l.is_empty() {
        bail!("{}: no samples decoded", path.display());
    }
    Ok(Track { rate, l, r })
}

/// A chapter_music node's loader (a plain thread, off the frame and audio threads): chapter → track path → cached
/// file → decoded track → the music block. The HUD reads `status`.
pub fn spawn_music_loader(lib: Library, node: &crate::graph::Node, feed: MusicFeed) {
    let toon = node.str("toon").unwrap_or("catpope").to_string();
    let n = node.f32("chapter", 1.0) as u32;
    let track = node.str("track").map(str::to_string);
    let quality = node.str("quality").unwrap_or("medium").to_string();
    let MusicFeed { mut tracks, status, .. } = feed;
    let label = format!("{toon} {n}");
    let set = move |s: String| {
        tracing::info!("music {label}: {s}");
        *status.lock().unwrap() = s;
    };
    std::thread::spawn(move || {
        set("loading the chapter".into());
        let t0 = std::time::Instant::now();
        let r = (|| -> Result<String> {
            let (ch, source) = lib.chapter(&toon, n)?;
            let Some((id, path)) = ch.music_path(track.as_deref(), &quality) else { bail!("the chapter has no music") };
            let (id, path) = (id.to_string(), path.to_string());
            set(format!("fetching '{id}'"));
            let file = lib.file(&toon, &path)?;
            set(format!("decoding '{id}'"));
            let t = decode(&file)?;
            let secs = t.seconds();
            if tracks.push(Arc::new(t)).is_err() {
                bail!("the music block is not taking tracks");
            }
            Ok(format!("'{id}' {secs:.0} s ({source}, {:.1} s to load) – {}", t0.elapsed().as_secs_f32(), ch.title))
        })();
        set(match r {
            Ok(s) => s,
            Err(e) => format!("failed: {e:#}"),
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHAPTER: &str = r#"{"title":"Leugnen","audio":{"music":{"defaultTrack":"calmness","tracks":[
        {"id":"tension","path":"s1/c2/t.128.mp3"},
        {"id":"calmness","path":"s1/c1/c.128.mp3","variants":{"low":"s1/c1/c.96.mp3","high":"s1/c1/c.320.mp3"}}]}},
        "groups":[{"slices":[{"image":"s1/c1/a.webp","imageVariants":{"medium":"s1/c1/a.2x.webp"},
          "dialog":[{"rect":{"x":1,"y":2,"width":50},"text":"Leugnen","font":"lettering"}]},{"width":3}]}]}"#;

    #[test]
    fn the_music_path_follows_track_default_and_quality() {
        let ch: Chapter = serde_json::from_str(CHAPTER).unwrap();
        assert_eq!(ch.music_path(None, "high"), Some(("calmness", "s1/c1/c.320.mp3")));
        assert_eq!(ch.music_path(None, "medium"), Some(("calmness", "s1/c1/c.128.mp3"))); // no variant: the path
        assert_eq!(ch.music_path(Some("tension"), "low"), Some(("tension", "s1/c2/t.128.mp3")));
        let slices: Vec<&Slice> = ch.slices().collect();
        assert_eq!(slices.len(), 1, "a slice without an image is skipped");
        assert_eq!((slices[0].image_path("medium"), slices[0].image_path("high")), (Some("s1/c1/a.2x.webp"), Some("s1/c1/a.webp")));
        assert_eq!(slices[0].dialog[0].text(), "Leugnen");
        let none: Chapter = serde_json::from_str(r#"{"title":"x"}"#).unwrap();
        assert_eq!(none.music_path(None, "low"), None);
    }

    #[test]
    fn cache_paths_mirror_the_cdn_and_refuse_escapes() {
        let lib = Library { api: API.into(), cdn: CDN.into(), cache: PathBuf::from("c") };
        assert_eq!(lib.file_cache_path("catpope", "s1/c1/a.mp3").unwrap(), Path::new("c/cdn/catpope/s1/c1/a.mp3"));
        assert!(lib.file_cache_path("catpope", "s1/../../etc/passwd").is_err());
        assert!(lib.chapter_cache_path("../x", 1).is_err());
    }

    #[test]
    fn the_cached_chapter_is_used_offline() {
        let dir = std::env::temp_dir().join(format!("deck-native-test-{}", std::process::id()));
        let lib = Library { api: "http://127.0.0.1:9".into(), cdn: CDN.into(), cache: dir.clone() };
        write_atomic(&lib.chapter_cache_path("catpope", 1).unwrap(), CHAPTER.as_bytes()).unwrap();
        let (ch, source) = lib.chapter("catpope", 1).unwrap();
        assert_eq!((ch.title.as_str(), source), ("Leugnen", "cached"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_wav_decodes_to_stereo() {
        let dir = std::env::temp_dir().join(format!("deck-native-wav-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.wav");
        std::fs::write(&path, wav_mono_16(22_050, &[0, 16_384, -16_384, 0])).unwrap();
        let t = decode(&path).unwrap();
        assert_eq!(t.rate, 22_050.0);
        assert_eq!(t.l.len(), 4);
        assert_eq!(t.l, t.r);
        assert!((t.l[1] - 0.5).abs() < 1e-3);
        let _ = std::fs::remove_dir_all(dir);
    }

    fn wav_mono_16(rate: u32, samples: &[i16]) -> Vec<u8> {
        let data = samples.len() as u32 * 2;
        let mut v = Vec::new();
        v.extend(b"RIFF");
        v.extend((36 + data).to_le_bytes());
        v.extend(b"WAVEfmt ");
        v.extend(16u32.to_le_bytes());
        v.extend(1u16.to_le_bytes()); // PCM
        v.extend(1u16.to_le_bytes()); // mono
        v.extend(rate.to_le_bytes());
        v.extend((rate * 2).to_le_bytes());
        v.extend(2u16.to_le_bytes());
        v.extend(16u16.to_le_bytes());
        v.extend(b"data");
        v.extend(data.to_le_bytes());
        for s in samples {
            v.extend(s.to_le_bytes());
        }
        v
    }
}
