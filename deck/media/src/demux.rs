//! A clip's tracks and their samples: ISO BMFF (MP4 / MOV, by re_mp4) and Matroska / WebM (by matroska-demuxer). A
//! sample is a byte range of the clip's data with its time; WebM's frames are copied once into one arena so both
//! containers index alike.
use crate::{err, Error};
use std::io::Cursor;
use std::sync::Arc;

/// HAP's flavours (their QuickTime fourccs)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Hap {
    /// `Hap1`: RGB in DXT1
    Rgb,
    /// `Hap5`: RGBA in DXT5
    Rgba,
    /// `HapY`: HAP Q, scaled YCoCg in DXT5
    Q,
    /// `HapM`, `HapA`, `Hap7`: HAP Q Alpha, HAP Alpha-only, HAP R – not decoded yet
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Codec {
    Av1,
    Hap(Hap),
    H264,
    Vp9,
    Aac,
    Vorbis,
    Opus,
    Flac,
    /// a codec the deck does not know, by its container's name for it
    Other(String),
}

impl Codec {
    fn of_fourcc(f: &str) -> Codec {
        match f {
            "av01" => Codec::Av1,
            "Hap1" => Codec::Hap(Hap::Rgb),
            "Hap5" => Codec::Hap(Hap::Rgba),
            "HapY" => Codec::Hap(Hap::Q),
            "HapM" | "HapA" | "Hap7" => Codec::Hap(Hap::Other),
            "avc1" | "avc3" => Codec::H264,
            "vp09" => Codec::Vp9,
            "mp4a" => Codec::Aac,
            "fLaC" => Codec::Flac,
            "Opus" => Codec::Opus,
            other => Codec::Other(other.to_string()),
        }
    }

    fn of_matroska(id: &str) -> Codec {
        match id {
            "V_AV1" => Codec::Av1,
            "V_VP9" => Codec::Vp9,
            "V_MPEG4/ISO/AVC" => Codec::H264,
            "A_VORBIS" => Codec::Vorbis,
            "A_OPUS" => Codec::Opus,
            "A_FLAC" => Codec::Flac,
            "A_AAC" => Codec::Aac,
            other => Codec::Other(other.to_string()),
        }
    }

    pub fn is_video(&self) -> bool {
        matches!(self, Codec::Av1 | Codec::Hap(_) | Codec::H264 | Codec::Vp9)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    /// presentation time in the track's ticks
    pub pts: i64,
    pub key: bool,
    /// its bytes in the clip's `data`
    pub at: usize,
    pub len: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    pub codec: Codec,
    /// pixels (video); 0 for audio
    pub width: u32,
    pub height: u32,
    /// ticks per second
    pub timescale: u32,
    /// the track's length in ticks
    pub duration: i64,
    /// the decoder's configuration record (AV1's `av1C`), when the container has one
    pub config: Vec<u8>,
    /// in presentation order
    pub samples: Vec<Sample>,
}

impl Track {
    pub fn seconds(&self, ticks: i64) -> f64 {
        ticks as f64 / f64::from(self.timescale.max(1))
    }

    pub fn length(&self) -> f64 {
        self.seconds(self.duration)
    }

    /// the last sample shown at `ticks` (the first one before the start)
    pub fn at(&self, ticks: i64) -> usize {
        self.samples.partition_point(|s| s.pts <= ticks).saturating_sub(1)
    }

    /// the nearest key sample at or before sample `i`
    pub fn key_before(&self, i: usize) -> usize {
        (0..=i.min(self.samples.len().saturating_sub(1))).rev().find(|&k| self.samples[k].key).unwrap_or(0)
    }
}

/// A demuxed clip: its first video track, its first audio track, and the bytes the samples point into.
#[derive(Debug, Clone)]
pub struct Clip {
    pub video: Option<Track>,
    pub audio: Option<Track>,
    pub data: Arc<[u8]>,
    pub container: Container,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    Mp4,
    Matroska,
}

impl Clip {
    /// The clip in `bytes` (a whole file). Err: not a container the deck reads, or a broken one.
    pub fn open(bytes: Arc<[u8]>) -> Result<Clip, Error> {
        if bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) {
            matroska(bytes)
        } else if bytes.len() >= 8 {
            mp4(bytes)
        } else {
            err("too short for a clip")
        }
    }

    pub fn sample(&self, s: &Sample) -> &[u8] {
        &self.data[s.at..s.at + s.len]
    }
}

fn mp4(bytes: Arc<[u8]>) -> Result<Clip, Error> {
    let file = re_mp4::Mp4::read_bytes(&bytes).map_err(|e| Error(format!("MP4: {e}")))?;
    let (mut video, mut audio) = (None, None);
    for t in file.tracks().values() {
        let stsd = &t.trak(&file).mdia.minf.stbl.stsd.contents;
        let fourcc = match stsd {
            re_mp4::StsdBoxContent::Unknown(f) => f.to_string(),
            re_mp4::StsdBoxContent::Mp4a(_) => "mp4a".into(),
            other => other.codec_string().unwrap_or_default().split('.').next().unwrap_or_default().to_string(),
        };
        let codec = Codec::of_fourcc(&fourcc);
        let mut samples: Vec<Sample> = t
            .samples
            .iter()
            .map(|s| Sample { pts: s.composition_timestamp, key: s.is_sync, at: s.offset as usize, len: s.size as usize })
            .collect();
        if samples.iter().any(|s| s.at + s.len > bytes.len()) {
            return err("MP4: a sample lies past the end of the file");
        }
        samples.sort_by_key(|s| s.pts);
        let track = Track {
            width: u32::from(t.width),
            height: u32::from(t.height),
            timescale: u32::try_from(t.timescale).map_err(|_| Error("MP4: timescale out of range".into()))?,
            duration: t.duration as i64,
            config: t.raw_codec_config(&file).unwrap_or_default(),
            samples,
            codec,
        };
        let slot = if track.codec.is_video() || matches!(t.kind, Some(re_mp4::TrackKind::Video)) { &mut video } else { &mut audio };
        if slot.is_none() && (track.codec.is_video() || !matches!(t.kind, Some(re_mp4::TrackKind::Subtitle))) {
            *slot = Some(track);
        }
    }
    Ok(Clip { video, audio, data: bytes, container: Container::Mp4 })
}

/// a Matroska track: its number, codec id, codec private data, pixel size
type Picked = (u64, String, Option<Vec<u8>>, Option<(u64, u64)>);

fn matroska(bytes: Arc<[u8]>) -> Result<Clip, Error> {
    let mut mkv = matroska_demuxer::MatroskaFile::open(Cursor::new(&*bytes)).map_err(|e| Error(format!("WebM: {e}")))?;
    // ticks of `timestamp_scale` ns
    let scale = mkv.info().timestamp_scale().get();
    let timescale = u32::try_from(1_000_000_000 / scale.max(1)).unwrap_or(1000).max(1);
    let duration = mkv.info().duration().unwrap_or(0.0) as i64;
    let pick = |want: matroska_demuxer::TrackType| mkv.tracks().iter().find(|t| t.track_type() == want).map(|t| (t.track_number().get(), t.codec_id().to_string(), t.codec_private().map(<[u8]>::to_vec), t.video().map(|v| (v.pixel_width().get(), v.pixel_height().get()))));
    let v = pick(matroska_demuxer::TrackType::Video);
    let a = pick(matroska_demuxer::TrackType::Audio);
    let mut arena = Vec::new();
    let (mut vs, mut as_) = (Vec::new(), Vec::new());
    let mut frame = matroska_demuxer::Frame::default();
    while mkv.next_frame(&mut frame).map_err(|e| Error(format!("WebM: {e}")))? {
        let into = if v.as_ref().is_some_and(|v| v.0 == frame.track) {
            &mut vs
        } else if a.as_ref().is_some_and(|a| a.0 == frame.track) {
            &mut as_
        } else {
            continue;
        };
        into.push(Sample { pts: frame.timestamp as i64, key: frame.is_keyframe.unwrap_or(true), at: arena.len(), len: frame.data.len() });
        arena.extend_from_slice(&frame.data);
    }
    // a track starts at 0 (a muxer's offset goes) and lasts to its last sample's end
    let track = |t: Option<Picked>, mut samples: Vec<Sample>| {
        t.map(|(_, id, private, size)| {
            samples.sort_by_key(|s| s.pts);
            let first = samples.first().map_or(0, |s| s.pts);
            samples.iter_mut().for_each(|s| s.pts -= first);
            let last = samples.last().map_or(0, |s| s.pts);
            // the mean spacing: millisecond ticks round each frame's time
            let n = samples.len();
            let (w, h) = size.unwrap_or((0, 0));
            let duration = if n > 1 { last + (last as f64 / (n - 1) as f64).round() as i64 } else { duration };
            Track { codec: Codec::of_matroska(&id), width: w as u32, height: h as u32, timescale, duration, config: private.unwrap_or_default(), samples }
        })
    };
    Ok(Clip { video: track(v, vs), audio: track(a, as_), data: arena.into(), container: Container::Matroska })
}
