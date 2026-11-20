//! Where packets become pictures. The core keeps demux, the choice, decode-ahead, seeking and timing; a backend only
//! decodes. `Software` is always there – the reference and the floor; a platform's decode-only backends (Media
//! Foundation, VA-API, WebCodecs …) come from its adapter (`Hardware`, installed once by the host, and the platform's
//! own software decoders, `install_software`), and `Auto` decides per stream, in this order: hardware where the
//! adapter decodes the codec and the deck setting allows it, then the platform's own software decoders (the
//! browser's), then `Software`; a failure goes on in the next one from the next key frame.
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
static PLATFORM_SOFTWARE: OnceLock<Box<dyn Hardware>> = OnceLock::new();
static HARDWARE_OFF: AtomicBool = AtomicBool::new(false);

/// The host's hardware decoders (its adapter's), once at start; a second call is ignored.
pub fn install(hardware: Box<dyn Hardware>) {
    let _ = HARDWARE.set(hardware);
}

/// The host's own software decoders (the browser's WebCodecs without hardware): tried after the hardware ones, before
/// `Software`; once at start, a second call is ignored. Their `Hardware::backend` decodes in software.
pub fn install_software(decoders: Box<dyn Hardware>) {
    let _ = PLATFORM_SOFTWARE.set(decoders);
}

/// The codecs the host's own software decoders take (none without an adapter): the capability the derivative choice asks.
pub fn platform_software_codecs() -> Vec<Codec> {
    static CODECS: OnceLock<Vec<Codec>> = OnceLock::new();
    match PLATFORM_SOFTWARE.get() {
        Some(h) => CODECS.get_or_init(|| h.codecs()).clone(),
        None => Vec::new(),
    }
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

/// which decoder a stream plays on, in the order the core tries them
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Hardware,
    /// the host's own software decoders (the browser's)
    Platform,
    /// deck-media's `Software`
    Core,
}

/// The backend the core picks per stream: the hardware decoder where it takes the codec, then the host's software
/// decoders, then `Software`; when one fails mid-stream, the next from the next key frame (where one decodes the
/// codec – else the error).
pub struct Auto {
    stream: Option<Stream>,
    /// the adapter's decoder in use (tier `Hardware` or `Platform`); None: `software`
    adapter: Option<Box<dyn DecodeBackend>>,
    tier: Tier,
    software: Software,
    /// a decoder failed: packets wait for a key frame before the next one takes them
    wait_key: bool,
}

impl Default for Auto {
    fn default() -> Self {
        Auto { stream: None, adapter: None, tier: Tier::Core, software: Software::default(), wait_key: false }
    }
}

impl Auto {
    /// whether the stream decodes in hardware
    pub fn on_hardware(&self) -> bool {
        self.adapter.is_some() && self.tier == Tier::Hardware
    }

    /// where the stream decodes now
    pub fn tier(&self) -> Tier {
        self.tier
    }

    /// the first adapter decoder from tier `from` on that takes the stream
    fn adapter_from(&mut self, from: Tier) -> Option<(Tier, Box<dyn DecodeBackend>)> {
        let stream = self.stream.clone()?;
        let tiers = [
            (Tier::Hardware, hardware().filter(|_| hardware_codecs().contains(&stream.codec))),
            (Tier::Platform, PLATFORM_SOFTWARE.get().map(|h| &**h).filter(|_| platform_software_codecs().contains(&stream.codec))),
        ];
        for (tier, h) in tiers {
            let Some(h) = h.filter(|_| tier >= from) else { continue };
            let mut b = h.backend();
            match b.configure(&stream) {
                Ok(()) => return Some((tier, b)),
                Err(e) => tracing::warn!("video: {tier:?} decoding of {:?} did not start, the next decoder instead: {e}", stream.codec),
            }
        }
        None
    }
}

impl DecodeBackend for Auto {
    fn caps(&self) -> Caps {
        let hw = hardware_codecs();
        let mut codecs = Software::codecs();
        for c in hw.iter().chain(&platform_software_codecs()) {
            if !codecs.contains(c) {
                codecs.push(c.clone());
            }
        }
        Caps { codecs, hardware: !hw.is_empty() }
    }

    fn configure(&mut self, stream: &Stream) -> Result<(), Error> {
        self.wait_key = false;
        self.stream = Some(stream.clone());
        (self.tier, self.adapter) = match self.adapter_from(Tier::Hardware) {
            Some((t, b)) => (t, Some(b)),
            None => (Tier::Core, None),
        };
        match self.software.configure(stream) {
            Err(e) if self.adapter.is_none() => Err(e),
            _ => Ok(()),
        }
    }

    fn submit(&mut self, packet: Packet<'_>) -> Result<(), Error> {
        if let Some(h) = &mut self.adapter {
            match h.submit(packet) {
                Ok(()) => return Ok(()),
                Err(e) => self.failed(e)?,
            }
        }
        if self.wait_key {
            if !packet.key {
                return Ok(());
            }
            self.wait_key = false;
            // the host's software decoders are set up only now, clean, at the key frame
            if self.tier == Tier::Platform {
                match self.adapter_from(Tier::Platform) {
                    Some((t, b)) => (self.tier, self.adapter) = (t, Some(b)),
                    None => self.tier = Tier::Core,
                }
            }
        }
        match &mut self.adapter {
            Some(h) => h.submit(packet),
            None => self.software.submit(packet),
        }
    }

    fn poll(&mut self) -> Option<Decoded> {
        match &mut self.adapter {
            Some(h) => h.poll(),
            None => self.software.poll(),
        }
    }

    fn drain(&mut self) {
        match &mut self.adapter {
            Some(h) => h.drain(),
            None => self.software.drain(),
        }
    }

    fn reset(&mut self) {
        match &mut self.adapter {
            Some(h) => h.reset(),
            None => self.software.reset(),
        }
    }

    fn check(&mut self) -> Result<(), Error> {
        match self.adapter.as_mut().map(|h| h.check()) {
            Some(Err(e)) => self.failed(e),
            _ => Ok(()),
        }
    }

    fn in_flight(&self) -> usize {
        self.adapter.as_ref().map_or(0, |h| h.in_flight())
    }
}

impl Auto {
    /// the decoder in use failed: the next one from the next key frame where one decodes the codec, else the error
    fn failed(&mut self, e: Error) -> Result<(), Error> {
        let next = match self.tier {
            Tier::Hardware if self.stream.as_ref().is_some_and(|s| platform_software_codecs().contains(&s.codec)) => Tier::Platform,
            _ if self.software.stream.is_some() => Tier::Core,
            _ => return Err(e),
        };
        tracing::warn!("video: the {:?} decoder failed, {next:?} from the next key frame: {e}", self.tier);
        self.adapter = None;
        self.tier = next;
        self.wait_key = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// a decoder for HAP that fails on its `n`th packet (0: never)
    struct Flaky(Software, u32, u32);

    impl DecodeBackend for Flaky {
        fn caps(&self) -> Caps {
            Caps { codecs: vec![Codec::Hap(Hap::Rgb)], hardware: true }
        }
        fn configure(&mut self, stream: &Stream) -> Result<(), Error> {
            self.0.configure(stream)
        }
        fn submit(&mut self, packet: Packet<'_>) -> Result<(), Error> {
            self.1 += 1;
            if self.1 == self.2 {
                return err("the decoder went away");
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

    /// a platform's decoders for HAP, failing on the given packet of each stream
    struct Gpu(u32);

    impl Hardware for Gpu {
        fn codecs(&self) -> Vec<Codec> {
            vec![Codec::Hap(Hap::Rgb)]
        }
        fn backend(&self) -> Box<dyn DecodeBackend> {
            Box::new(Flaky(Software::default(), 0, self.0))
        }
    }

    /// one 4 × 4 DXT1 frame: a HAP section header (8 bytes, stored DXT1) and its one block
    const FRAME: [u8; 12] = [8, 0, 0, 0xab, 0, 0, 0, 0, 0, 0, 0, 0];

    #[test]
    fn hardware_first_then_the_platforms_software_then_the_core_each_from_the_next_key_frame_and_hardware_off_by_the_setting() {
        install(Box::new(Gpu(3)));
        install_software(Box::new(Gpu(2)));
        assert_eq!(hardware_codecs(), [Codec::Hap(Hap::Rgb)]);
        assert_eq!(platform_software_codecs(), [Codec::Hap(Hap::Rgb)]);
        let stream = Stream { codec: Codec::Hap(Hap::Rgb), width: 4, height: 4, config: vec![] };
        let mut a = Auto::default();
        a.configure(&stream).unwrap();
        assert_eq!(a.tier(), Tier::Hardware);
        assert!(a.on_hardware());
        // hardware fails on packet 2; 2 and 3 wait for the key frame 4; the platform's software takes 4 and fails on its
        // second packet (5); 5 and 6 wait for the key frame 7, where the core's software takes over
        let keys = [(0, true), (1, false), (2, false), (3, false), (4, true), (5, false), (6, false), (7, true), (8, false)];
        let (mut got, mut tiers) = (vec![], vec![]);
        for (id, key) in keys {
            a.submit(Packet { data: &FRAME, id, key }).unwrap();
            got.extend(std::iter::from_fn(|| a.poll()).map(|d| d.id));
            tiers.push(a.tier());
        }
        assert_eq!(got, [0, 1, 4, 7, 8], "hardware 0 1, platform 4, core 7 8");
        use Tier::*;
        assert_eq!(tiers, [Hardware, Hardware, Platform, Platform, Platform, Core, Core, Core, Core]);
        assert!(!a.on_hardware());
        // a fresh stream starts at the top again
        a.configure(&stream).unwrap();
        assert_eq!(a.tier(), Tier::Hardware);
        // hardware off: the platform's software first
        set_hardware(false);
        assert!(hardware_codecs().is_empty());
        a.configure(&stream).unwrap();
        assert_eq!(a.tier(), Tier::Platform);
        assert!(!a.on_hardware());
        set_hardware(true);
    }
}
