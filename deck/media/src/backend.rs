//! Where packets become pictures. The core keeps demux, the choice, decode-ahead, seeking and timing; a backend only
//! decodes. `Software` is always there – the reference and the fallback; hardware decode-only backends (WebCodecs,
//! VideoToolbox, MediaCodec, VA-API) implement the same trait. TODO(#326): the hardware adapters (plan P13c).
use crate::demux::{Codec, Hap};
use crate::{err, Error, Picture};

/// what a backend decodes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caps {
    pub codecs: Vec<Codec>,
    pub hardware: bool,
}

/// the stream a backend is set up for
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stream {
    pub codec: Codec,
    pub width: u32,
    pub height: u32,
    pub config: Vec<u8>,
}

/// one compressed frame; `id` comes back with its picture
#[derive(Debug, Clone, Copy)]
pub struct Packet<'a> {
    pub data: &'a [u8],
    pub id: i64,
    pub key: bool,
}

/// a decoded picture and its packet's id
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    pub id: i64,
    pub picture: Picture,
}

pub trait DecodeBackend: Send {
    fn caps(&self) -> Caps;
    fn configure(&mut self, stream: &Stream) -> Result<(), Error>;
    fn submit(&mut self, packet: Packet<'_>) -> Result<(), Error>;
    /// the next picture that is ready, in presentation order
    fn poll(&mut self) -> Option<Decoded>;
    /// forget what was submitted (a seek, a loop)
    fn reset(&mut self);
}

enum Inner {
    None,
    Hap { width: u32, height: u32 },
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    Av1(crate::av1::Av1),
}

/// The software decoders: HAP everywhere, AV1 where rav1d builds (`crate::AV1`).
pub struct Software {
    inner: Inner,
    stream: Option<Stream>,
    ready: std::collections::VecDeque<Decoded>,
    got: Vec<(i64, Picture)>,
}

impl Default for Software {
    fn default() -> Self {
        Software { inner: Inner::None, stream: None, ready: Default::default(), got: Vec::new() }
    }
}

impl Software {
    pub fn codecs() -> Vec<Codec> {
        let mut c = vec![Codec::Hap(Hap::Rgb), Codec::Hap(Hap::Rgba), Codec::Hap(Hap::Q)];
        if crate::AV1 {
            c.push(Codec::Av1);
        }
        c
    }
}

impl DecodeBackend for Software {
    fn caps(&self) -> Caps {
        Caps { codecs: Software::codecs(), hardware: false }
    }

    fn configure(&mut self, stream: &Stream) -> Result<(), Error> {
        self.inner = match &stream.codec {
            Codec::Hap(Hap::Other) => return err("HAP: this flavour (Q Alpha, Alpha-only, R) is not decoded yet"),
            Codec::Hap(_) => Inner::Hap { width: stream.width, height: stream.height },
            #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
            Codec::Av1 => Inner::Av1(crate::av1::Av1::new()?),
            other => return err(format!("no software decoder for {other:?} on this deck")),
        };
        self.stream = Some(stream.clone());
        self.ready.clear();
        Ok(())
    }

    fn submit(&mut self, packet: Packet<'_>) -> Result<(), Error> {
        match &mut self.inner {
            Inner::None => return err("the decoder is not configured"),
            Inner::Hap { width, height } => {
                let picture = crate::hap::decode(packet.data, *width, *height)?;
                self.ready.push_back(Decoded { id: packet.id, picture });
            }
            #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
            Inner::Av1(d) => {
                d.send(packet.data, packet.id, &mut self.got)?;
                self.ready.extend(self.got.drain(..).map(|(id, picture)| Decoded { id, picture }));
            }
        }
        Ok(())
    }

    fn poll(&mut self) -> Option<Decoded> {
        self.ready.pop_front()
    }

    fn reset(&mut self) {
        self.ready.clear();
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        if let Inner::Av1(d) = &mut self.inner {
            d.flush();
        }
    }
}
