//! Linux: VA-API through ChromeOS's stateless decoders (cros-codecs): they parse the bitstream (slice headers, reference
//! lists) and libva's driver (iHD / i965 on Intel, radeonsi on AMD) decodes. Packets go in as Annex B (H.264, HEVC) or as
//! they are (VP9, AV1); each picture is a GBM buffer the driver decodes into, read back with vaGetImage (the driver
//! untiles) into a tight NV12 `Picture` for deck-space's NV12 pass. `codecs()` is the capability query: the codecs
//! whose VA profile the driver decodes (vaQueryConfigProfiles with the VLD entry point).
use cros_codecs::decoder::stateless::av1::Av1;
use cros_codecs::decoder::stateless::h264::H264;
use cros_codecs::decoder::stateless::h265::H265;
use cros_codecs::decoder::stateless::vp9::Vp9;
use cros_codecs::decoder::stateless::{DecodeError, DynStatelessVideoDecoder, StatelessDecoder, StatelessVideoDecoder};
use cros_codecs::decoder::{BlockingMode, DecodedHandle, DecoderEvent, StreamInfo};
use cros_codecs::libva::{self, Display, Image, VAEntrypoint, VAProfile};
use cros_codecs::video_frame::frame_pool::{FramePool, PooledVideoFrame};
use cros_codecs::video_frame::gbm_video_frame::{GbmDevice, GbmUsage, GbmVideoFrame};
use cros_codecs::video_frame::VideoFrame;
use cros_codecs::Fourcc;
use deck_media::annexb::AnnexB;
use deck_media::backend::{Caps, DecodeBackend, Decoded, Packet, Stream};
use deck_media::yuv::Matrix;
use deck_media::{Codec, Error, Format, Picture};
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

type Frame = PooledVideoFrame<GbmVideoFrame>;

/// frames beyond the decoder's own minimum, so a picture waiting to be read back never stalls the next packet
const SPARE_FRAMES: usize = 4;

fn fail(what: &str, e: impl std::fmt::Display) -> Error {
    Error(format!("VA-API: {what}: {e}"))
}

/// a codec's VA profiles, any of which will do
fn profiles(c: &Codec) -> &'static [VAProfile::Type] {
    match c {
        Codec::H264 => &[VAProfile::VAProfileH264High, VAProfile::VAProfileH264Main, VAProfile::VAProfileH264ConstrainedBaseline],
        Codec::Hevc => &[VAProfile::VAProfileHEVCMain],
        Codec::Vp9 => &[VAProfile::VAProfileVP9Profile0],
        Codec::Av1 => &[VAProfile::VAProfileAV1Profile0],
        _ => &[],
    }
}

const CODECS: [Codec; 4] = [Codec::H264, Codec::Hevc, Codec::Vp9, Codec::Av1];

/// the codecs among `reported` profiles whose decode entry point the driver has (`vld`)
fn decodable(reported: &[VAProfile::Type], vld: impl Fn(VAProfile::Type) -> bool) -> Vec<Codec> {
    CODECS.into_iter().filter(|c| profiles(c).iter().any(|p| reported.contains(p) && vld(*p))).collect()
}

/// the first render node whose VA display opens
fn node() -> Result<(String, Rc<Display>), Error> {
    let mut why = "no /dev/dri/renderD* node".to_string();
    for n in 128..136 {
        let path = format!("/dev/dri/renderD{n}");
        if !std::path::Path::new(&path).exists() {
            continue;
        }
        match Display::open_drm_display(&path) {
            Ok(d) => return Ok((path, d)),
            Err(e) => why = format!("{path}: {e}"),
        }
    }
    Err(Error(format!("VA-API: {why}")))
}

/// What this machine's VA driver decodes (none without a driver or a render node); asked once.
pub fn codecs() -> Vec<Codec> {
    static CODECS: OnceLock<Vec<Codec>> = OnceLock::new();
    CODECS
        .get_or_init(|| {
            let Ok((_, display)) = node() else { return Vec::new() };
            let Ok(reported) = display.query_config_profiles() else { return Vec::new() };
            decodable(&reported, |p| display.query_config_entrypoints(p).is_ok_and(|e| e.contains(&VAEntrypoint::VAEntrypointVLD)))
        })
        .clone()
}

/// The planes of a mapped NV12 image (`offsets`, `pitches` per plane) as rows tight, `w` × `h`.
fn tight_nv12(data: &[u8], offsets: [usize; 2], pitches: [usize; 2], (w, h): (u32, u32)) -> Result<Vec<u8>, Error> {
    let (w, h) = (w as usize, h as usize);
    let (cw, ch) = (2 * w.div_ceil(2), h.div_ceil(2));
    let mut out = Vec::with_capacity(w * h + cw * ch);
    for (plane, rows, len) in [(0, h, w), (1, ch, cw)] {
        for r in 0..rows {
            let at = offsets[plane] + r * pitches[plane];
            let row = data.get(at..at + len).ok_or_else(|| Error("VA-API: the image is smaller than its planes".into()))?;
            out.extend_from_slice(row);
        }
    }
    Ok(out)
}

/// One stream's decoder, its display and the GBM frames it decodes into.
struct Live {
    display: Rc<Display>,
    gbm: Arc<GbmDevice>,
    decoder: DynStatelessVideoDecoder<Frame>,
    pool: FramePool<GbmVideoFrame>,
    nv12: libva::VAImageFormat,
    matrix: Matrix,
}

fn open(stream: &Stream) -> Result<Live, Error> {
    let (path, display) = node()?;
    let gbm = GbmDevice::open(&path).map_err(|e| fail("GBM", e))?;
    let nv12 = display
        .query_image_formats()
        .map_err(|e| fail("image formats", e))?
        .into_iter()
        .find(|f| f.fourcc == libva::VA_FOURCC_NV12)
        .ok_or_else(|| Error("VA-API: the driver reads no NV12 image".into()))?;
    let mode = BlockingMode::Blocking;
    let d = display.clone();
    let decoder = match &stream.codec {
        Codec::H264 => StatelessDecoder::<H264, _>::new_vaapi(d, mode).map_err(|e| fail("H.264", e))?.into_trait_object(),
        Codec::Hevc => StatelessDecoder::<H265, _>::new_vaapi(d, mode).map_err(|e| fail("HEVC", e))?.into_trait_object(),
        Codec::Vp9 => StatelessDecoder::<Vp9, _>::new_vaapi(d, mode).map_err(|e| fail("VP9", e))?.into_trait_object(),
        Codec::Av1 => StatelessDecoder::<Av1, _>::new_vaapi(d, mode).map_err(|e| fail("AV1", e))?.into_trait_object(),
        c => return Err(Error(format!("VA-API: no decoder for {c:?}"))),
    };
    let frames = gbm.clone();
    let pool = FramePool::new(move |info: &StreamInfo| {
        // `size` made one of these already: a failure here is the device going away mid-stream
        frames.clone().new_frame(Fourcc::from(b"NV12"), info.display_resolution, info.coded_resolution, GbmUsage::Decode).expect("a GBM frame")
    });
    let matrix = if stream.height >= 720 { Matrix::Bt709 } else { Matrix::Bt601 };
    Ok(Live { display, gbm, decoder, pool, nv12, matrix })
}

/// the pool for a (new) stream format; one frame is made first, so a device that cannot is an error, not a panic
fn size(live: &mut Live, info: &StreamInfo) -> Result<(), Error> {
    live.gbm.clone().new_frame(Fourcc::from(b"NV12"), info.display_resolution, info.coded_resolution, GbmUsage::Decode).map_err(|e| fail("a frame", e))?;
    live.pool.resize(&StreamInfo { min_num_frames: info.min_num_frames + SPARE_FRAMES, ..info.clone() });
    Ok(())
}

/// A decoded frame as a tight NV12 picture of the stream's size (within the decoded frame).
fn read(live: &Live, stream: &Stream, handle: &dyn DecodedHandle<Frame = Frame>) -> Result<Picture, Error> {
    handle.sync().map_err(|e| fail("a picture", e))?;
    let frame = handle.video_frame();
    let surface = frame.to_native_handle(&live.display).map_err(|e| fail("a picture's surface", e))?;
    let shown = handle.display_resolution();
    let (fw, fh) = surface.size();
    let (w, h) = if stream.width > 0 && stream.height > 0 { (stream.width.min(shown.width), stream.height.min(shown.height)) } else { (shown.width, shown.height) };
    let (w, h) = (w.min(fw), h.min(fh));
    let image = Image::create_from(&surface, live.nv12, (fw, fh), (w, h)).map_err(|e| fail("reading a picture", e))?;
    let va = image.image();
    let data = tight_nv12(image.as_ref(), [va.offsets[0] as usize, va.offsets[1] as usize], [va.pitches[0] as usize, va.pitches[1] as usize], (w, h))?;
    Ok(Picture::new(w, h, Format::Nv12 { matrix: live.matrix, full: false }, data))
}

/// VA-API's decoder for one stream.
#[derive(Default)]
pub struct Vaapi {
    live: Option<Live>,
    stream: Option<Stream>,
    annexb: Option<AnnexB>,
    ready: VecDeque<Decoded>,
    bytes: Vec<u8>,
}

// SAFETY: the display's `Rc` and every clone of it (the decoder's, its surfaces') live inside this one value and move
// with it; libva and GBM objects may be used from another thread as long as one thread uses them at a time (`&mut self`)
unsafe impl Send for Vaapi {}

impl Vaapi {
    pub fn new() -> Vaapi {
        Vaapi::default()
    }

    /// what the decoder has ready: pictures into `ready` (unless `keep` is false: a reset drops them), a new format
    /// sizes the pool
    fn events(&mut self, keep: bool) -> Result<(), Error> {
        let Vaapi { live: Some(live), stream: Some(stream), ready, .. } = self else { return Ok(()) };
        while let Some(event) = live.decoder.next_event() {
            match event {
                DecoderEvent::FrameReady(h) if keep => ready.push_back(Decoded { id: h.timestamp() as i64, picture: read(live, stream, &*h)? }),
                DecoderEvent::FrameReady(_) => {}
                DecoderEvent::FormatChanged => {
                    let info = live.decoder.stream_info().cloned().ok_or_else(|| Error("VA-API: a new format without its size".into()))?;
                    size(live, &info)?;
                }
            }
        }
        Ok(())
    }

    /// one packet's bytes: the decoder takes them a unit at a time and may first want its events handled
    fn feed(&mut self, id: i64, mut data: &[u8]) -> Result<(), Error> {
        let mut stuck = 0;
        while !data.is_empty() {
            let Some(live) = self.live.as_mut() else { return Err(Error("the decoder is not configured".into())) };
            let Live { decoder, pool, .. } = live;
            match decoder.decode(id as u64, data, &mut || pool.alloc()) {
                Ok(n) if n > 0 => {
                    data = &data[n.min(data.len())..];
                    stuck = 0;
                }
                Ok(_) | Err(DecodeError::CheckEvents | DecodeError::NotEnoughOutputBuffers(_)) => {
                    stuck += 1;
                    if stuck > 4 {
                        return Err(Error("VA-API: the decoder takes no packet and gives no picture".into()));
                    }
                }
                Err(e) => return Err(fail("a packet", e)),
            }
            self.events(true)?;
        }
        Ok(())
    }
}

impl DecodeBackend for Vaapi {
    fn caps(&self) -> Caps {
        Caps { codecs: codecs(), hardware: true }
    }

    fn configure(&mut self, stream: &Stream) -> Result<(), Error> {
        if !codecs().contains(&stream.codec) {
            return Err(Error(format!("VA-API: the driver does not decode {:?} here", stream.codec)));
        }
        self.annexb = AnnexB::new(&stream.codec, &stream.config)?;
        self.live = Some(open(stream)?);
        self.stream = Some(stream.clone());
        self.ready.clear();
        Ok(())
    }

    fn submit(&mut self, packet: Packet<'_>) -> Result<(), Error> {
        let mut bytes = std::mem::take(&mut self.bytes);
        let data = match &self.annexb {
            Some(a) => {
                a.convert(packet.data, packet.key, &mut bytes)?;
                &bytes[..]
            }
            None => packet.data,
        };
        let r = self.feed(packet.id, data);
        self.bytes = bytes;
        r
    }

    fn poll(&mut self) -> Option<Decoded> {
        self.ready.pop_front()
    }

    fn drain(&mut self) {
        if let Some(live) = &mut self.live {
            let _ = live.decoder.flush();
        }
        let _ = self.events(true);
    }

    fn reset(&mut self) {
        if let Some(live) = &mut self.live {
            let _ = live.decoder.flush();
        }
        let _ = self.events(false);
        self.ready.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// the X1's UHD 620 (iHD) reports H.264, HEVC and VP9 profiles for decoding, no AV1: exactly those three
    #[test]
    fn the_capability_is_the_reported_decode_profiles() {
        let uhd620 = [
            VAProfile::VAProfileMPEG2Main,
            VAProfile::VAProfileH264Main,
            VAProfile::VAProfileH264High,
            VAProfile::VAProfileH264ConstrainedBaseline,
            VAProfile::VAProfileHEVCMain,
            VAProfile::VAProfileHEVCMain10,
            VAProfile::VAProfileVP9Profile0,
            VAProfile::VAProfileJPEGBaseline,
        ];
        assert_eq!(decodable(&uhd620, |_| true), vec![Codec::H264, Codec::Hevc, Codec::Vp9]);
        // an encode-only profile (no VLD entry point) is not a decoder
        assert_eq!(decodable(&uhd620, |p| p != VAProfile::VAProfileHEVCMain), vec![Codec::H264, Codec::Vp9]);
        assert_eq!(decodable(&[VAProfile::VAProfileAV1Profile0], |_| true), vec![Codec::Av1]);
        assert!(decodable(&[], |_| true).is_empty());
    }

    /// pitched planes at their offsets become tight rows, the chroma pairs rounded up for an odd size
    #[test]
    fn a_pitched_image_becomes_tight_nv12() {
        let (w, h, pitch, uv_at) = (3u32, 3u32, 8usize, 32usize);
        let mut data = vec![0u8; 64];
        for r in 0..3 {
            for c in 0..3 {
                data[r * pitch + c] = (10 * r + c) as u8;
            }
        }
        for r in 0..2 {
            for c in 0..4 {
                data[uv_at + r * pitch + c] = (100 + 10 * r + c) as u8;
            }
        }
        let out = tight_nv12(&data, [0, uv_at], [pitch, pitch], (w, h)).unwrap();
        assert_eq!(out, [0, 1, 2, 10, 11, 12, 20, 21, 22, 100, 101, 102, 103, 110, 111, 112, 113]);
        assert!(tight_nv12(&data[..40], [0, uv_at], [pitch, pitch], (w, h)).is_err());
    }

    /// a clip decodes on this machine's VA driver (where it has one): pictures of the stream's size, NV12
    #[test]
    fn a_clip_decodes_on_the_driver() {
        let caps = codecs();
        eprintln!("VA-API decodes: {caps:?}");
        let Some(path) = std::env::var_os("DECK_HW_CLIP") else { return eprintln!("DECK_HW_CLIP unset: skipped") };
        let clip = deck_media::Clip::open(std::fs::read(path).unwrap().into()).unwrap();
        let track = clip.video.clone().unwrap();
        if !caps.contains(&track.codec) {
            return eprintln!("no VA decoder for {:?}: skipped", track.codec);
        }
        let mut p = deck_media::Player::new(&clip, Box::new(Vaapi::new())).unwrap();
        let mut q = deck_media::Frames::new(8);
        for t in [0.0, 0.5, 1.0] {
            let pic = p.frame_for(t, &mut q).unwrap().unwrap().picture.clone();
            assert_eq!((pic.width, pic.height), (track.width, track.height));
            assert!(matches!(pic.format, Format::Nv12 { .. }));
        }
    }
}
