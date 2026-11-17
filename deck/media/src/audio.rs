//! A clip's sound, decoded whole by symphonia (AAC, Vorbis, FLAC, PCM in MP4 / MOV, WebM / Matroska or their own files)
//! into interleaved f32 – the sample pool's bytes, decoded before they play, never on the audio thread.
use crate::{err, Error};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

#[derive(Debug, Clone, PartialEq)]
pub struct Pcm {
    pub rate: u32,
    pub channels: u16,
    /// interleaved, −1…1
    pub samples: Vec<f32>,
}

impl Pcm {
    pub fn seconds(&self) -> f64 {
        self.samples.len() as f64 / f64::from(self.channels.max(1)) / f64::from(self.rate.max(1))
    }
}

/// The first audio track of a whole file. Ok(None): the file has no sound.
pub fn decode(bytes: Vec<u8>) -> Result<Option<Pcm>, Error> {
    let mss = MediaSourceStream::new(Box::new(std::io::Cursor::new(bytes)), Default::default());
    let probed = symphonia::default::get_probe()
        .format(&Hint::new(), mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| Error(format!("audio: {e}")))?;
    let mut format = probed.format;
    let Some(track) = format.tracks().iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL && t.codec_params.sample_rate.is_some()) else {
        return Ok(None);
    };
    let (id, params) = (track.id, track.codec_params.clone());
    let mut dec = match symphonia::default::get_codecs().make(&params, &DecoderOptions::default()) {
        Ok(d) => d,
        Err(SymError::Unsupported(what)) => return err(format!("audio: {what} is not decoded on the deck")),
        Err(e) => return err(format!("audio: {e}")),
    };
    let rate = params.sample_rate.unwrap_or(48_000);
    let mut channels = params.channels.map_or(0, |c| c.count() as u16);
    let mut samples = Vec::new();
    let mut buf: Option<SampleBuffer<f32>> = None;
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(SymError::ResetRequired) => break,
            Err(e) => return err(format!("audio: {e}")),
        };
        if packet.track_id() != id {
            continue;
        }
        let decoded = match dec.decode(&packet) {
            Ok(d) => d,
            Err(SymError::DecodeError(_)) => continue,
            Err(e) => return err(format!("audio: {e}")),
        };
        let spec = *decoded.spec();
        channels = spec.channels.count() as u16;
        let b = buf.get_or_insert_with(|| SampleBuffer::new(decoded.capacity() as u64, spec));
        if b.capacity() < decoded.capacity() * spec.channels.count() {
            *b = SampleBuffer::new(decoded.capacity() as u64, spec);
        }
        b.copy_interleaved_ref(decoded);
        samples.extend_from_slice(b.samples());
    }
    Ok(Some(Pcm { rate, channels, samples }))
}
