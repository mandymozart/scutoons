//! The browser's video decoders (WebCodecs `VideoDecoder`), offered to deck-media twice: its hardware ones as the
//! platform's hardware, its software ones as the platform's software (`backend::install_software`): the
//! core's probes asked of `isConfigSupported`, packets in, each picture out as its `VideoFrame` – it stays on the GPU
//! (`Format::Held`) and is copied there into the stage's texture (`copy_held`), then closed. This decides nothing: the
//! codec strings, the packet order, the pacing and the fallback are deck-media's (`rfc6381`, `Player`, `backend::Auto`).
//! WebCodecs is bound here by hand: web-sys has it only behind `web_sys_unstable_apis`.
use deck_media::backend::Stream;
use deck_media::{rfc6381, Caps, Codec, DecodeBackend, Decoded, Error, Format, Frames, Hardware, Held, Packet, Picture, Player};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = VideoDecoder)]
    type JsDecoder;
    #[wasm_bindgen(constructor, js_class = "VideoDecoder", catch)]
    fn new(init: &js_sys::Object) -> Result<JsDecoder, JsValue>;
    #[wasm_bindgen(method, js_class = "VideoDecoder", catch)]
    fn configure(this: &JsDecoder, config: &js_sys::Object) -> Result<(), JsValue>;
    #[wasm_bindgen(method, js_class = "VideoDecoder", catch)]
    fn decode(this: &JsDecoder, chunk: &Chunk) -> Result<(), JsValue>;
    #[wasm_bindgen(method, js_class = "VideoDecoder", catch)]
    fn flush(this: &JsDecoder) -> Result<js_sys::Promise, JsValue>;
    #[wasm_bindgen(method, js_class = "VideoDecoder", catch)]
    fn close(this: &JsDecoder) -> Result<(), JsValue>;
    #[wasm_bindgen(static_method_of = JsDecoder, js_class = "VideoDecoder", js_name = isConfigSupported, catch)]
    fn is_config_supported(config: &js_sys::Object) -> Result<js_sys::Promise, JsValue>;

    #[wasm_bindgen(js_name = EncodedVideoChunk)]
    type Chunk;
    #[wasm_bindgen(constructor, js_class = "EncodedVideoChunk", catch)]
    fn new(init: &js_sys::Object) -> Result<Chunk, JsValue>;
}

fn object(fields: &[(&str, JsValue)]) -> js_sys::Object {
    let o = js_sys::Object::new();
    for (k, v) in fields {
        let _ = js_sys::Reflect::set(&o, &JsValue::from_str(k), v);
    }
    o
}

fn said(e: JsValue) -> Error {
    let why = js_sys::Reflect::get(&e, &"message".into()).ok().and_then(|m| m.as_string()).or_else(|| e.as_string()).unwrap_or_else(|| format!("{e:?}"));
    Error(format!("WebCodecs: {why}"))
}

/// The core's probes asked of the browser once for its hardware decoders and once for its software ones
/// (`isConfigSupported`, "prefer-hardware" / "prefer-software") – and the decoders installed for each; once, before a
/// scene opens a video. The core orders them (`backend::Auto`: hardware, the browser's software, deck-media's own).
/// Returns the probes the hardware answered yes to; none: the browser's software and deck-media's decode.
#[wasm_bindgen]
pub async fn video_install() -> Vec<String> {
    let hardware = supported("prefer-hardware").await;
    let software = supported("prefer-software").await;
    let hardware_yes: Vec<String> = hardware.iter().map(|(_, s)| s.to_string()).collect();
    web_sys::console::log_1(&JsValue::from_str(&format!("video: the browser decodes in hardware: {:?}, in software: {:?}", names(&hardware), names(&software))));
    let (hw, sw): (Vec<Codec>, Vec<Codec>) = (hardware.into_iter().map(|(c, _)| c).collect(), software.into_iter().map(|(c, _)| c).collect());
    let any = !hw.is_empty() || !sw.is_empty();
    if !hw.is_empty() {
        deck_media::backend::install(Box::new(Browser { codecs: hw, prefer: "prefer-hardware" }));
    }
    if !sw.is_empty() {
        deck_media::backend::install_software(Box::new(Browser { codecs: sw, prefer: "prefer-software" }));
    }
    if any {
        deck_space::video::install_held(copy_held);
    }
    hardware_yes
}

fn names(v: &[(Codec, &str)]) -> Vec<String> {
    v.iter().map(|(c, _)| format!("{c:?}")).collect()
}

/// the core's probes that the browser decodes with `prefer`
async fn supported(prefer: &str) -> Vec<(Codec, &'static str)> {
    let mut yes = Vec::new();
    if js_sys::Reflect::has(&js_sys::global(), &"VideoDecoder".into()).unwrap_or(false) {
        for (codec, string) in rfc6381::probes() {
            let ask = object(&[("codec", string.into()), ("hardwareAcceleration", prefer.into())]);
            let Ok(p) = JsDecoder::is_config_supported(&ask) else { continue };
            let support = JsFuture::from(p).await.ok().and_then(|r| js_sys::Reflect::get(&r, &"supported".into()).ok());
            if support.and_then(|s| s.as_bool()) == Some(true) {
                yes.push((codec, string));
            }
        }
    }
    yes
}

/// A held picture's `VideoFrame`, closed when the core lets the picture go.
struct Frame(web_sys::VideoFrame);
// SAFETY: this crate builds for wasm32-unknown-unknown without threads: a JS handle never leaves its thread.
unsafe impl Send for Frame {}
// SAFETY: as above.
unsafe impl Sync for Frame {}

impl Drop for Frame {
    fn drop(&mut self) {
        self.0.close();
    }
}

/// a held picture into `texture` (RGBA, its size), on the GPU: WebGPU's and WebGL2's external image copy
fn copy_held(held: &Held, queue: &wgpu::Queue, texture: &wgpu::Texture) {
    let Some(f) = held.0.downcast_ref::<Frame>() else { return };
    queue.copy_external_image_to_texture(
        &wgpu::CopyExternalImageSourceInfo { source: wgpu::ExternalImageSource::VideoFrame(Clone::clone(&f.0)), origin: wgpu::Origin2d::ZERO, flip_y: false },
        wgpu::CopyExternalImageDestInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
            color_space: wgpu::PredefinedColorSpace::Srgb,
            premultiplied_alpha: false,
        },
        texture.size(),
    );
}

struct Browser {
    codecs: Vec<Codec>,
    prefer: &'static str,
}

impl Hardware for Browser {
    fn codecs(&self) -> Vec<Codec> {
        self.codecs.clone()
    }

    fn backend(&self) -> Box<dyn DecodeBackend> {
        Box::new(WebCodecs { prefer: self.prefer, config: None, live: None, retired: None })
    }
}

/// What one `VideoDecoder`'s callbacks leave: its pictures, its failure, its counts. A decoder made again (a seek)
/// leaves the old one's `live` false: what it still gives is closed.
#[derive(Default)]
struct Shared {
    live: bool,
    ready: VecDeque<web_sys::VideoFrame>,
    failed: Option<String>,
    sent: usize,
    given: usize,
}

struct Live {
    dec: JsDecoder,
    shared: Rc<RefCell<Shared>>,
    _output: Closure<dyn FnMut(web_sys::VideoFrame)>,
    _error: Closure<dyn FnMut(JsValue)>,
}

impl Live {
    fn retire(self) -> Live {
        self.shared.borrow_mut().live = false;
        let _ = self.dec.close();
        for f in self.shared.borrow_mut().ready.drain(..) {
            f.close();
        }
        self
    }
}

struct WebCodecs {
    prefer: &'static str,
    config: Option<js_sys::Object>,
    live: Option<Live>,
    /// the decoder before the last seek, kept while a late callback of it may still come
    retired: Option<Live>,
}

// SAFETY: this crate builds for wasm32-unknown-unknown without threads: the decoder never leaves its thread.
unsafe impl Send for WebCodecs {}

impl WebCodecs {
    fn open(&mut self) -> Result<(), Error> {
        if let Some(old) = self.live.take() {
            self.retired = Some(old.retire());
        }
        let Some(config) = &self.config else { return Err(Error("the decoder is not configured".into())) };
        let shared = Rc::new(RefCell::new(Shared { live: true, ..Default::default() }));
        let s = shared.clone();
        let output = Closure::<dyn FnMut(web_sys::VideoFrame)>::new(move |f: web_sys::VideoFrame| {
            let mut s = s.borrow_mut();
            if !s.live {
                return f.close();
            }
            s.given += 1;
            s.ready.push_back(f);
        });
        let s = shared.clone();
        let error = Closure::<dyn FnMut(JsValue)>::new(move |e: JsValue| {
            let mut s = s.borrow_mut();
            if s.live {
                s.failed = Some(said(e).0);
            }
        });
        let dec = JsDecoder::new(&object(&[("output", output.as_ref().clone()), ("error", error.as_ref().clone())])).map_err(said)?;
        dec.configure(config).map_err(said)?;
        self.live = Some(Live { dec, shared, _output: output, _error: error });
        Ok(())
    }
}

impl DecodeBackend for WebCodecs {
    fn caps(&self) -> Caps {
        let hardware = self.prefer == "prefer-hardware";
        let codecs = if hardware { deck_media::backend::hardware_codecs() } else { deck_media::backend::platform_software_codecs() };
        Caps { codecs, hardware }
    }

    fn configure(&mut self, stream: &Stream) -> Result<(), Error> {
        let codec = rfc6381::codec_string(stream).ok_or_else(|| Error(format!("WebCodecs: no codec string for {:?}", stream.codec)))?;
        let mut fields = vec![("codec", codec.into()), ("codedWidth", stream.width.into()), ("codedHeight", stream.height.into()), ("hardwareAcceleration", self.prefer.into())];
        if let Some(d) = rfc6381::description(stream) {
            fields.push(("description", js_sys::Uint8Array::from(d).into()));
        }
        self.config = Some(object(&fields));
        self.open()
    }

    fn submit(&mut self, packet: Packet<'_>) -> Result<(), Error> {
        let live = self.live.as_ref().ok_or_else(|| Error("the decoder is not configured".into()))?;
        if let Some(why) = live.shared.borrow_mut().failed.take() {
            return Err(Error(why));
        }
        let kind = if packet.key { "key" } else { "delta" };
        let chunk = Chunk::new(&object(&[("type", kind.into()), ("timestamp", (packet.id as f64).into()), ("data", js_sys::Uint8Array::from(packet.data).into())])).map_err(said)?;
        live.dec.decode(&chunk).map_err(said)?;
        live.shared.borrow_mut().sent += 1;
        Ok(())
    }

    fn poll(&mut self) -> Option<Decoded> {
        let f = self.live.as_ref()?.shared.borrow_mut().ready.pop_front()?;
        let (id, width, height) = (f.timestamp() as i64, f.display_width(), f.display_height());
        let held = Held(Arc::new(Frame(f)));
        Some(Decoded { id, picture: Picture { width, height, format: Format::Held, data: Vec::new(), held: Some(held) } })
    }

    fn drain(&mut self) {
        if let Some(Ok(p)) = self.live.as_ref().map(|l| l.dec.flush()) {
            wasm_bindgen_futures::spawn_local(async move {
                let _ = JsFuture::from(p).await;
            });
        }
    }

    fn reset(&mut self) {
        if let Err(e) = self.open() {
            if let Some(l) = &self.live {
                l.shared.borrow_mut().failed = Some(e.0);
            }
        }
    }

    fn check(&mut self) -> Result<(), Error> {
        match self.live.as_ref().and_then(|l| l.shared.borrow_mut().failed.take()) {
            Some(why) => Err(Error(why)),
            None => Ok(()),
        }
    }

    fn in_flight(&self) -> usize {
        self.live.as_ref().map_or(0, |l| {
            let s = l.shared.borrow();
            s.sent - s.given
        })
    }
}

impl Drop for WebCodecs {
    fn drop(&mut self) {
        // a late callback of a closed decoder must still find its closure
        for l in [self.live.take(), self.retired.take()].into_iter().flatten() {
            let l = l.retire();
            l._output.forget();
            l._error.forget();
        }
    }
}

/// A clip decoded as fast as the deck's backend goes (`Player::open`: WebCodecs where the core picks it): the page steps
/// it, yielding to its event loop between steps, and times it.
#[wasm_bindgen]
pub struct VideoBench {
    player: Player,
    frames: Frames,
    decoded: u32,
}

#[wasm_bindgen]
impl VideoBench {
    #[wasm_bindgen(constructor)]
    pub fn new(bytes: Vec<u8>) -> Result<VideoBench, JsValue> {
        let clip = deck_media::Clip::open(bytes.into()).map_err(crate::err)?;
        let mut player = Player::open(&clip).map_err(crate::err)?;
        player.looping = false;
        player.ahead = f64::MAX;
        Ok(VideoBench { player, frames: Frames::new(32), decoded: 0 })
    }

    /// the clip's frames
    pub fn count(&self) -> u32 {
        self.player.track().samples.len() as u32
    }

    /// frames per second of the clip
    pub fn fps(&self) -> f64 {
        f64::from(self.count()) / self.player.length()
    }

    /// what the clip is and where it decodes: "<codec> <w>×<h> on <backend>"
    pub fn about(&self) -> String {
        let t = self.player.track();
        let on = if deck_media::backend::hardware_codecs().contains(&t.codec) {
            "WebCodecs hardware"
        } else if deck_media::backend::platform_software_codecs().contains(&t.codec) {
            "WebCodecs software"
        } else {
            "deck-media software"
        };
        format!("{:?} {}×{} on {on}", t.codec, t.width, t.height)
    }

    /// decode on from the last frame that came back; returns how many frames came back in all
    pub fn step(&mut self) -> Result<u32, JsValue> {
        let t = self.player.track();
        let at = (self.decoded as usize).min(t.samples.len() - 1);
        let heard = t.seconds(t.samples[at].pts);
        self.decoded += self.player.fill(heard, &mut self.frames).map_err(crate::err)? as u32;
        Ok(self.decoded)
    }
}
