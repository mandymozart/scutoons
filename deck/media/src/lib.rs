//! The deck's own media playback, in the core on every deck (plan §2.9): demux (MP4 / MOV, WebM / Matroska), video
//! decoding behind the `DecodeBackend` trait (the software backend: AV1 by rav1d, HAP's Snappy + DXT frames), audio by
//! symphonia, the player that decodes ahead and hands out the frame for the heard time, and the choice of an asset's
//! derivative for this device. The host brings the bytes; nothing here opens a file, a socket or a device.
//!
//! AV1 builds wherever rav1d does: natively and on wasm32-wasip1. On wasm32-unknown-unknown (the browser) rav1d does not
//! compile – its C-shaped API imports `libc::ptrdiff_t` / `off_t` / errno values, which the `libc` crate has no
//! definitions for on that target – so there the software backend plays HAP only. TODO(#326): rav1d for the browser.
pub mod audio;
pub mod backend;
pub mod choose;
pub mod demux;
pub mod dxt;
pub mod hap;
pub mod player;
pub mod yuv;

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub mod av1;

pub use backend::{Caps, DecodeBackend, Decoded, Packet, Software};
pub use demux::{Clip, Codec, Hap, Sample, Track};
pub use player::{Frame, Frames, Player};

/// whether this build decodes AV1 in software
pub const AV1: bool = cfg!(not(all(target_arch = "wasm32", target_os = "unknown")));

/// What went wrong, in words a person reads in the HUD or a log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

pub(crate) fn err<T>(why: impl Into<String>) -> Result<T, Error> {
    Err(Error(why.into()))
}

/// How a picture's bytes are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// 8-bit RGBA, rows of `width` pixels
    Rgba8,
    /// DXT1 / BC1 blocks (HAP), straight to a GPU that samples BC
    Bc1,
    /// DXT5 / BC3 blocks (HAP Alpha)
    Bc3,
    /// HAP Q: scaled YCoCg in DXT5 blocks – a shader (or `to_rgba`) turns it into RGB
    Bc3YCoCg,
    /// 8-bit YUV 4:2:0 (AV1): the luma plane `width` × `height`, then U, then V at half size rounded up, rows tight –
    /// the GPU gets the planes as they are and a shader turns them into RGB (or `to_rgba`)
    Yuv420 { matrix: yuv::Matrix, full: bool },
}

/// One decoded picture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub format: Format,
    pub data: Vec<u8>,
}

impl Picture {
    /// the same picture as RGBA8 (the software path for a GPU without BC, and the goldens)
    pub fn to_rgba(&self) -> Picture {
        let data = match self.format {
            Format::Rgba8 => return self.clone(),
            Format::Bc1 => dxt::decode(&self.data, self.width, self.height, dxt::Block::Bc1),
            Format::Bc3 => dxt::decode(&self.data, self.width, self.height, dxt::Block::Bc3),
            Format::Bc3YCoCg => {
                let mut px = dxt::decode(&self.data, self.width, self.height, dxt::Block::Bc3);
                dxt::ycocg_to_rgb(&mut px);
                px
            }
            Format::Yuv420 { matrix, full } => {
                let [y, u, v] = self.planes().expect("a 4:2:0 picture");
                let planes = yuv::Planes { y: (y.0, y.1 as usize), u: (u.0, u.1 as usize), v: (v.0, v.1 as usize) };
                yuv::to_rgba(&planes, self.width as usize, self.height as usize, matrix, full)
            }
        };
        Picture { width: self.width, height: self.height, format: Format::Rgba8, data }
    }

    /// whether the GPU gets it as blocks
    pub fn compressed(&self) -> bool {
        matches!(self.format, Format::Bc1 | Format::Bc3 | Format::Bc3YCoCg)
    }

    /// A 4:2:0 picture's planes Y, U, V: (bytes, width = row length, height). None for any other format.
    pub fn planes(&self) -> Option<[(&[u8], u32, u32); 3]> {
        let Format::Yuv420 { .. } = self.format else { return None };
        let (w, h) = (self.width, self.height);
        let (cw, ch) = (w.div_ceil(2), h.div_ceil(2));
        let (luma, chroma) = ((w * h) as usize, (cw * ch) as usize);
        let (y, rest) = self.data.split_at(luma);
        let (u, v) = rest.split_at(chroma);
        Some([(y, w, h), (u, cw, ch), (&v[..chroma], cw, ch)])
    }
}

/// FNV-1a over bytes: the goldens' fingerprint (the same on every target)
pub fn fingerprint(bytes: &[u8]) -> u64 {
    fold(0xcbf2_9ce4_8422_2325, bytes)
}

fn fold(h: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(h, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3))
}

/// A clip decoded whole, as the goldens compare it on every target: each video frame as RGBA in order, the sound's
/// samples' bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Golden {
    pub frames: u32,
    pub video: u64,
    pub samples: u32,
    pub audio: u64,
}

impl Golden {
    /// `"<frames>:<video hash> <samples>:<audio hash>"`
    pub fn line(&self) -> String {
        format!("{}:{:016x} {}:{:016x}", self.frames, self.video, self.samples, self.audio)
    }
}

/// The golden of the clip in `bytes`: every frame through the software backend at its own time, the first audio track.
pub fn golden(bytes: &[u8]) -> Result<Golden, Error> {
    let clip = Clip::open(bytes.into())?;
    let (mut frames, mut video) = (0, fingerprint(&[]));
    if let Some(track) = clip.video.clone() {
        let mut player = Player::new(&clip, Box::new(Software::default()))?;
        player.looping = false;
        let mut queue = Frames::new(4);
        for s in &track.samples {
            let f = player.frame_for(track.seconds(s.pts), &mut queue)?.ok_or_else(|| Error("a frame is missing".into()))?;
            video = fold(video, &f.picture.to_rgba().data);
            frames += 1;
        }
    }
    let (mut samples, mut audio) = (0, fingerprint(&[]));
    if clip.audio.is_some() {
        if let Some(pcm) = audio::decode(bytes.to_vec())? {
            samples = pcm.samples.len() as u32;
            for s in &pcm.samples {
                audio = fold(audio, &s.to_bits().to_le_bytes());
            }
        }
    }
    Ok(Golden { frames, video, samples, audio })
}
