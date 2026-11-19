//! Where packets become pictures. The core keeps demux, the choice, decode-ahead, seeking and timing; a backend only
//! decodes. `Software` is always there – the reference and the fallback; a platform's hardware decode-only backends
//! (Media Foundation, VA-API, WebCodecs …) come from its adapter (`Hardware`, installed once by the host), and `Auto`
//! decides per stream: hardware where the adapter decodes the codec and the deck setting allows it, else software; a
//! hardware error goes on in software from the next key frame.
use crate::demux::{Codec, Hap};
use crate::{err, Error, Picture};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

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
    /// no more packets follow: give out the pictures still held (a decoder that reorders keeps some back)
    fn drain(&mut self) {}
    /// forget what was submitted (a seek, a loop)
    fn reset(&mut self);
    /// a decoder that answers later (the browser's) says here what failed since it took its packets
    fn check(&mut self) -> Result<(), Error> {
        Ok(())
    }
    /// packets taken whose pictures did not come out yet, for a decoder that answers later (one that answers at once: 0)
    fn in_flight(&self) -> usize {
        0
    }
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
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    got: Vec<(i64, Picture)>,
}

impl Default for Software {
    fn default() -> Self {
        Software {
            inner: Inner::None,
            stream: None,
            ready: Default::default(),
            #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
            got: Vec::new(),
        }
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
        self.stream = None;
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

/// A platform's hardware decoders, as its adapter offers them: what they decode (asked once), and one for a stream.
pub trait Hardware: Send + Sync {
    fn codecs(&self) -> Vec<Codec>;
    fn backend(&self) -> Box<dyn DecodeBackend>;
}

static HARDWARE: OnceLock<Box<dyn Hardware>> = OnceLock::new();
static HARDWARE_OFF: AtomicBool = AtomicBool::new(false);

/// The host's hardware decoders (its adapter's), once at start; a second call is ignored.
pub fn install(hardware: Box<dyn Hardware>) {
    let _ = HARDWARE.set(hardware);
}

/// The deck setting "hardware decoding": auto (true, the default) or off. Streams opened after it follow it.
pub fn set_hardware(on: bool) {
    HARDWARE_OFF.store(!on, Ordering::Relaxed);
}

fn hardware() -> Option<&'static dyn Hardware> {
    if HARDWARE_OFF.load(Ordering::Relaxed) {
        return None;
    }
    HARDWARE.get().map(|h| &**h)
}

/// The codecs this deck decodes in hardware (none without an adapter, or with the setting off): the capability the
/// derivative choice asks (`choose::Device::here`).
pub fn hardware_codecs() -> Vec<Codec> {
    static CODECS: OnceLock<Vec<Codec>> = OnceLock::new();
    match hardware() {
        Some(h) => CODECS.get_or_init(|| h.codecs()).clone(),
        None => Vec::new(),
    }
}

/// The backend the core picks per stream: the hardware decoder where it takes the codec, else software; when the
/// hardware fails mid-stream, software from the next key frame (where software decodes the codec – else the error).
#[derive(Default)]
pub struct Auto {
    hardware: Option<Box<dyn DecodeBackend>>,
    software: Software,
    /// the hardware failed: packets wait for a key frame before software takes them
    wait_key: bool,
}

impl Auto {
    /// whether the stream decodes in hardware
    pub fn on_hardware(&self) -> bool {
        self.hardware.is_some()
    }
}

impl DecodeBackend for Auto {
    fn caps(&self) -> Caps {
        let hw = hardware_codecs();
        let mut codecs = Software::codecs();
        for c in &hw {
            if !codecs.contains(c) {
                codecs.push(c.clone());
            }
        }
        Caps { codecs, hardware: !hw.is_empty() }
    }

    fn configure(&mut self, stream: &Stream) -> Result<(), Error> {
        self.hardware = None;
        self.wait_key = false;
        if let Some(h) = hardware().filter(|_| hardware_codecs().contains(&stream.codec)) {
            let mut b = h.backend();
            match b.configure(stream) {
                Ok(()) => self.hardware = Some(b),
                Err(e) => tracing::warn!("video: hardware decoding of {:?} did not start, software instead: {e}", stream.codec),
            }
        }
        match self.software.configure(stream) {
            Err(e) if self.hardware.is_none() => Err(e),
            _ => Ok(()),
        }
    }

    fn submit(&mut self, packet: Packet<'_>) -> Result<(), Error> {
        if let Some(h) = &mut self.hardware {
            match h.submit(packet) {
                Ok(()) => return Ok(()),
                Err(e) => self.hardware_failed(e)?,
            }
        }
        if self.wait_key && !packet.key {
            return Ok(());
        }
        self.wait_key = false;
        self.software.submit(packet)
    }

    fn poll(&mut self) -> Option<Decoded> {
        match &mut self.hardware {
            Some(h) => h.poll(),
            None => self.software.poll(),
        }
    }

    fn drain(&mut self) {
        match &mut self.hardware {
            Some(h) => h.drain(),
            None => self.software.drain(),
        }
    }

    fn reset(&mut self) {
        match &mut self.hardware {
            Some(h) => h.reset(),
            None => self.software.reset(),
        }
    }

    fn check(&mut self) -> Result<(), Error> {
        match self.hardware.as_mut().map(|h| h.check()) {
            Some(Err(e)) => self.hardware_failed(e),
            _ => Ok(()),
        }
    }

    fn in_flight(&self) -> usize {
        self.hardware.as_ref().map_or(0, |h| h.in_flight())
    }
}

impl Auto {
    /// the hardware failed: software from the next key frame where software decodes the codec, else the error
    fn hardware_failed(&mut self, e: Error) -> Result<(), Error> {
        if self.software.stream.is_none() {
            return Err(e);
        }
        tracing::warn!("video: the hardware decoder failed, software from the next key frame: {e}");
        self.hardware = None;
        self.wait_key = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// a "hardware" decoder for HAP that fails on its third packet
    struct Flaky(Software, u32);

    impl DecodeBackend for Flaky {
        fn caps(&self) -> Caps {
            Caps { codecs: vec![Codec::Hap(Hap::Rgb)], hardware: true }
        }
        fn configure(&mut self, stream: &Stream) -> Result<(), Error> {
            self.0.configure(stream)
        }
        fn submit(&mut self, packet: Packet<'_>) -> Result<(), Error> {
            self.1 += 1;
            if self.1 == 3 {
                return err("the GPU went away");
            }
            self.0.submit(packet)
        }
        fn poll(&mut self) -> Option<Decoded> {
            self.0.poll()
        }
        fn reset(&mut self) {
            self.0.reset()
        }
    }

    struct Gpu;

    impl Hardware for Gpu {
        fn codecs(&self) -> Vec<Codec> {
            vec![Codec::Hap(Hap::Rgb)]
        }
        fn backend(&self) -> Box<dyn DecodeBackend> {
            Box::new(Flaky(Software::default(), 0))
        }
    }

    #[test]
    fn hardware_first_then_software_from_the_next_key_frame_and_off_by_the_setting() {
        install(Box::new(Gpu));
        assert_eq!(hardware_codecs(), [Codec::Hap(Hap::Rgb)]);
        let stream = Stream { codec: Codec::Hap(Hap::Rgb), width: 4, height: 4, config: vec![] };
        let mut a = Auto::default();
        a.configure(&stream).unwrap();
        assert!(a.on_hardware());
        // one 4 × 4 DXT1 frame: a HAP section header (8 bytes, stored DXT1) and its one block
        let frame = [8, 0, 0, 0xab, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut got = vec![];
        for (id, key) in [(0, true), (1, false), (2, false), (3, false), (4, true), (5, false)] {
            a.submit(Packet { data: &frame, id, key }).unwrap();
            got.extend(std::iter::from_fn(|| a.poll()).map(|d| d.id));
        }
        assert!(!a.on_hardware(), "it failed on packet 2");
        assert_eq!(got, [0, 1, 4, 5], "2 and 3 wait for the key frame 4");
        set_hardware(false);
        assert!(hardware_codecs().is_empty());
        a.configure(&stream).unwrap();
        assert!(!a.on_hardware(), "off: software");
        set_hardware(true);
    }
}
