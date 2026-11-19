//! Windows: Media Foundation's video decoder (an MFT) with a D3D11 device manager, so the GPU's fixed-function decoder
//! (DXVA – NVDEC, Quick Sync, VCN) does the work. Packets go in as Annex B (deck-media's `annexb`); each picture comes
//! out as a D3D11 NV12 texture and is copied back into an NV12 `Picture` (rows tight, cropped to the stream's size)
//! through a ring of staging textures, which deck-space's video pass turns into RGB. `codecs()` is the capability query: what the GPU's decoder profiles
//! and the installed decoders both take (HEVC needs Microsoft's HEVC Video Extensions, AV1 its AV1 Video Extension).
//! `software` = the same decoder without the GPU, for comparison only.
use deck_media::annexb::AnnexB;
use deck_media::backend::{Caps, DecodeBackend, Decoded, Packet, Stream};
use deck_media::yuv::Matrix;
use deck_media::{Codec, Error, Format, Picture};
use std::collections::VecDeque;
use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;
use windows::core::{Interface, GUID};
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1};
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, COINIT_MULTITHREADED};

fn fail(what: &str, e: windows::core::Error) -> Error {
    Error(format!("Media Foundation: {what}: {e}"))
}

/// a codec's Media Foundation subtype and its D3D11 decoder profile
fn kind(c: &Codec) -> Option<(GUID, GUID)> {
    Some(match c {
        Codec::H264 => (MFVideoFormat_H264, D3D11_DECODER_PROFILE_H264_VLD_NOFGT),
        Codec::Hevc => (MFVideoFormat_HEVC, D3D11_DECODER_PROFILE_HEVC_VLD_MAIN),
        Codec::Av1 => (MFVideoFormat_AV1, D3D11_DECODER_PROFILE_AV1_VLD_PROFILE0),
        Codec::Vp9 => (MFVideoFormat_VP90, D3D11_DECODER_PROFILE_VP9_VLD_PROFILE0),
        _ => return None,
    })
}

const CODECS: [Codec; 4] = [Codec::H264, Codec::Hevc, Codec::Av1, Codec::Vp9];

thread_local! {
    static COM: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// COM on this thread (multithreaded; a thread that is single-threaded already stays so) and Media Foundation, once
fn start() -> Result<(), Error> {
    COM.with(|on| {
        if !on.get() {
            // SAFETY: plain COM initialisation; a failure (another mode) leaves the thread as it was
            let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            on.set(true);
        }
    });
    static MF: OnceLock<Result<(), String>> = OnceLock::new();
    // SAFETY: MFStartup once per process; it is never shut down (the deck decodes until it exits)
    MF.get_or_init(|| unsafe { MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET) }.map_err(|e| e.to_string())).clone().map_err(|e| Error(format!("Media Foundation did not start: {e}")))
}

/// the D3D11 device the decoder works on, behind Media Foundation's device manager
struct Gpu {
    device: ID3D11Device,
    manager: IMFDXGIDeviceManager,
}

fn gpu() -> Result<Gpu, Error> {
    let mut device = None;
    // SAFETY: out slots on the stack; the device is multithread-protected before anyone shares it
    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_VIDEO_SUPPORT | D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            Some(&[D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0]),
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            None,
        )
        .map_err(|e| fail("no D3D11 video device", e))?;
        let device: ID3D11Device = device.ok_or_else(|| Error("Media Foundation: no D3D11 device".into()))?;
        let _ = device.cast::<ID3D11Multithread>().map_err(|e| fail("multithreading", e))?.SetMultithreadProtected(true);
        let (mut token, mut manager) = (0u32, None);
        MFCreateDXGIDeviceManager(&mut token, &mut manager).map_err(|e| fail("device manager", e))?;
        let manager: IMFDXGIDeviceManager = manager.ok_or_else(|| Error("Media Foundation: no device manager".into()))?;
        manager.ResetDevice(&device, token).map_err(|e| fail("device manager", e))?;
        Ok(Gpu { device, manager })
    }
}

/// The decoders Media Foundation has for `subtype`, best first (synchronous ones; a store extension's included).
fn decoders(subtype: GUID) -> Vec<IMFActivate> {
    let input = MFT_REGISTER_TYPE_INFO { guidMajorType: MFMediaType_Video, guidSubtype: subtype };
    let (mut list, mut n) = (std::ptr::null_mut(), 0u32);
    let flags = MFT_ENUM_FLAG_SYNCMFT | MFT_ENUM_FLAG_LOCALMFT | MFT_ENUM_FLAG_SORTANDFILTER;
    // SAFETY: MFTEnumEx hands out an array of `n` activation objects allocated with CoTaskMemAlloc; each is taken
    // (released when dropped) and the array freed
    unsafe {
        if MFTEnumEx(MFT_CATEGORY_VIDEO_DECODER, flags, Some(&input), None, &mut list, &mut n).is_err() || list.is_null() {
            return Vec::new();
        }
        let out = std::slice::from_raw_parts_mut(list, n as usize).iter_mut().filter_map(Option::take).collect();
        CoTaskMemFree(Some(list as _));
        out
    }
}

/// What this machine decodes on its GPU through Media Foundation: the D3D11 decoder profiles the GPU reports, for codecs
/// a decoder is installed for. Asked once.
pub fn codecs() -> Vec<Codec> {
    static CAPS: OnceLock<Vec<Codec>> = OnceLock::new();
    CAPS.get_or_init(|| {
        if start().is_err() {
            return Vec::new();
        }
        let Ok(gpu) = gpu() else { return Vec::new() };
        let Ok(video) = gpu.device.cast::<ID3D11VideoDevice>() else { return Vec::new() };
        // SAFETY: plain queries on a live device
        let profiles: Vec<GUID> = unsafe { (0..video.GetVideoDecoderProfileCount()).filter_map(|i| video.GetVideoDecoderProfile(i).ok()).collect() };
        CODECS.iter().filter(|c| kind(c).is_some_and(|(sub, profile)| profiles.contains(&profile) && !decoders(sub).is_empty())).cloned().collect()
    })
    .clone()
}

/// Time spent copying pictures back from the GPU, and how many: what a zero-copy path would save.
pub fn copied() -> (u64, std::time::Duration) {
    (COPIES.load(Ordering::Relaxed), std::time::Duration::from_nanos(COPY_NS.load(Ordering::Relaxed)))
}

static COPIES: AtomicU64 = AtomicU64::new(0);
static COPY_NS: AtomicU64 = AtomicU64::new(0);

/// a configured decoder
struct Live {
    mft: IMFTransform,
    /// kept alive while the decoder uses it
    _gpu: Option<Gpu>,
    /// the decoder hands out its own samples (it does with a D3D manager)
    provides: bool,
    /// the bytes of an output sample when it does not
    size: u32,
    /// the decoded frame's size (the coded one, ≥ the picture's)
    frame: (u32, u32),
    matrix: Matrix,
    full: bool,
    /// GPU pictures on their way back
    ring: Ring,
}

/// how many GPU pictures can be on their way back at once
const RING: usize = 4;

/// GPU pictures on their way back: each copied into a staging texture of the ring and read once the GPU is done with
/// it, so the decoder runs on meanwhile – reading each one at once waits for its decode every frame.
#[derive(Default)]
struct Ring {
    /// the decoder texture's size (its height padded)
    size: (u32, u32),
    slots: Vec<ID3D11Texture2D>,
    /// (the picture's id, its slot), oldest first
    waiting: VecDeque<(i64, usize)>,
    next: usize,
}

/// Media Foundation's decoder for one stream: on the GPU (`new`) or, for comparison, on the CPU (`software`).
pub struct MediaFoundation {
    gpu: bool,
    live: Option<Live>,
    stream: Option<Stream>,
    annexb: Option<AnnexB>,
    ready: VecDeque<Decoded>,
    bytes: Vec<u8>,
}

// SAFETY: Media Foundation's decoders and the device manager are free-threaded; the D3D11 device is multithread-protected,
// and the backend is used from one thread at a time (`&mut self`)
unsafe impl Send for MediaFoundation {}

impl Default for MediaFoundation {
    fn default() -> Self {
        MediaFoundation::new()
    }
}

impl MediaFoundation {
    pub fn new() -> MediaFoundation {
        MediaFoundation { gpu: true, live: None, stream: None, annexb: None, ready: VecDeque::new(), bytes: Vec::new() }
    }

    /// the same decoder on the CPU, to measure against
    pub fn software() -> MediaFoundation {
        MediaFoundation { gpu: false, ..MediaFoundation::new() }
    }

    fn open(&self, stream: &Stream) -> Result<Live, Error> {
        start()?;
        let Some((subtype, _)) = kind(&stream.codec) else { return Err(Error(format!("Media Foundation: no decoder for {:?}", stream.codec))) };
        if self.gpu && !codecs().contains(&stream.codec) {
            return Err(Error(format!("Media Foundation: the GPU does not decode {:?} here", stream.codec)));
        }
        let gpu = if self.gpu { Some(gpu()?) } else { None };
        // SAFETY: COM calls on live objects; the media types are made and set before streaming starts
        unsafe {
            let mut mft = None;
            for a in decoders(subtype) {
                let Ok(m) = a.ActivateObject::<IMFTransform>() else { continue };
                let aware = m.GetAttributes().and_then(|at| at.GetUINT32(&MF_SA_D3D11_AWARE)).unwrap_or(0) == 1;
                if gpu.is_none() || aware {
                    mft = Some(m);
                    break;
                }
            }
            let mft = mft.ok_or_else(|| Error(format!("Media Foundation: no decoder for {:?} installed", stream.codec)))?;
            if let Some(g) = &gpu {
                mft.ProcessMessage(MFT_MESSAGE_SET_D3D_MANAGER, g.manager.as_raw() as usize).map_err(|e| fail("the GPU's decoder", e))?;
            }
            let input = MFCreateMediaType().map_err(|e| fail("media type", e))?;
            input.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).map_err(|e| fail("media type", e))?;
            input.SetGUID(&MF_MT_SUBTYPE, &subtype).map_err(|e| fail("media type", e))?;
            if stream.width > 0 && stream.height > 0 {
                input.SetUINT64(&MF_MT_FRAME_SIZE, (u64::from(stream.width) << 32) | u64::from(stream.height)).map_err(|e| fail("media type", e))?;
            }
            mft.SetInputType(0, &input, 0).map_err(|e| fail("the stream's type", e))?;
            let mut live = Live { mft, _gpu: gpu, provides: false, size: 0, frame: (stream.width, stream.height), matrix: Matrix::Bt709, full: false, ring: Ring::default() };
            output(&mut live, stream)?;
            live.mft.ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0).map_err(|e| fail("start", e))?;
            live.mft.ProcessMessage(MFT_MESSAGE_NOTIFY_START_OF_STREAM, 0).map_err(|e| fail("start", e))?;
            Ok(live)
        }
    }

    /// pictures the decoder has ready, into `ready`; Ok(how many)
    fn pull(&mut self) -> Result<usize, Error> {
        let (Some(live), Some(stream)) = (self.live.as_mut(), self.stream.as_ref()) else { return Ok(0) };
        let mut n = 0;
        loop {
            // SAFETY: an output buffer the decoder fills (its own sample, or one of `size` bytes made here); what it hands
            // back is taken out of the ManuallyDrop slots exactly once
            let (r, sample) = unsafe {
                let own = if live.provides {
                    None
                } else {
                    let s = MFCreateSample().map_err(|e| fail("sample", e))?;
                    s.AddBuffer(&MFCreateMemoryBuffer(live.size).map_err(|e| fail("buffer", e))?).map_err(|e| fail("buffer", e))?;
                    Some(s)
                };
                let mut out = [MFT_OUTPUT_DATA_BUFFER { dwStreamID: 0, pSample: ManuallyDrop::new(own), dwStatus: 0, pEvents: ManuallyDrop::new(None) }];
                let mut status = 0u32;
                let r = live.mft.ProcessOutput(0, &mut out, &mut status);
                drop(ManuallyDrop::take(&mut out[0].pEvents));
                (r, ManuallyDrop::take(&mut out[0].pSample))
            };
            match r {
                Ok(()) => {
                    if let Some(s) = sample {
                        // SAFETY: a live sample
                        let id = unsafe { s.GetSampleTime() }.map_err(|e| fail("a picture's time", e))?;
                        if !copy_in(live, stream, &s, id, &mut self.ready)? {
                            let picture = read(live, stream, &s)?;
                            self.ready.push_back(Decoded { id, picture });
                        }
                        n += 1;
                    }
                }
                Err(e) if e.code() == MF_E_TRANSFORM_NEED_MORE_INPUT => {
                    take_back(live, stream, false, &mut self.ready)?;
                    return Ok(n);
                }
                Err(e) if e.code() == MF_E_TRANSFORM_STREAM_CHANGE => output(live, stream)?,
                Err(e) => return Err(fail("decoding", e)),
            }
        }
    }
}

/// the decoder's NV12 output type, chosen (again after a stream change)
fn output(live: &mut Live, stream: &Stream) -> Result<(), Error> {
    // SAFETY: COM calls on a live decoder
    unsafe {
        let mut i = 0;
        let t = loop {
            let t = live.mft.GetOutputAvailableType(0, i).map_err(|e| fail("the decoder gives no NV12", e))?;
            if t.GetGUID(&MF_MT_SUBTYPE).is_ok_and(|g| g == MFVideoFormat_NV12) {
                break t;
            }
            i += 1;
        };
        live.mft.SetOutputType(0, &t, 0).map_err(|e| fail("NV12 out", e))?;
        if let Ok(size) = t.GetUINT64(&MF_MT_FRAME_SIZE) {
            live.frame = ((size >> 32) as u32, size as u32);
        }
        // what the stream says, else HD is BT.709 and SD BT.601; video range unless it says full
        live.matrix = match t.GetUINT32(&MF_MT_YUV_MATRIX).map(|m| m as i32) {
            Ok(m) if m == MFVideoTransferMatrix_BT601.0 => Matrix::Bt601,
            Ok(m) if m == MFVideoTransferMatrix_BT709.0 => Matrix::Bt709,
            _ if stream.height.max(live.frame.1) >= 720 => Matrix::Bt709,
            _ => Matrix::Bt601,
        };
        live.full = t.GetUINT32(&MF_MT_VIDEO_NOMINAL_RANGE).is_ok_and(|r| r as i32 == MFNominalRange_0_255.0);
        let info = live.mft.GetOutputStreamInfo(0).map_err(|e| fail("output", e))?;
        live.provides = info.dwFlags & (MFT_OUTPUT_STREAM_PROVIDES_SAMPLES.0 as u32) != 0;
        live.size = info.cbSize;
    }
    Ok(())
}

/// rows of `w` luma bytes and `cw` U, V pairs out of an NV12 image with `pitch` and `rows` luma rows, into `data`
///
/// SAFETY: `scan` points at `pitch * rows * 3 / 2` readable bytes
unsafe fn tight(scan: *const u8, pitch: usize, rows: usize, (w, h): (u32, u32), data: &mut Vec<u8>) {
    let (cw, ch) = (w.div_ceil(2) as usize, h.div_ceil(2) as usize);
    // SAFETY: the caller's
    let bytes = unsafe { std::slice::from_raw_parts(scan, pitch * rows * 3 / 2) };
    for r in 0..h as usize {
        data.extend_from_slice(&bytes[r * pitch..][..w as usize]);
    }
    let uv = &bytes[pitch * rows..];
    for r in 0..ch {
        data.extend_from_slice(&uv[r * pitch..][..2 * cw]);
    }
}

/// A GPU picture into the ring (false: not a GPU picture). A full ring first gives its oldest back, waiting for it.
fn copy_in(live: &mut Live, stream: &Stream, sample: &IMFSample, id: i64, ready: &mut VecDeque<Decoded>) -> Result<bool, Error> {
    let Some(device) = live._gpu.as_ref().map(|g| g.device.clone()) else { return Ok(false) };
    // SAFETY: COM calls on live objects; the decoder's texture slice is copied on the device's own context
    unsafe {
        let buffer = sample.GetBufferByIndex(0).map_err(|e| fail("a picture's buffer", e))?;
        let Ok(dxgi) = buffer.cast::<IMFDXGIBuffer>() else { return Ok(false) };
        let mut raw = std::ptr::null_mut();
        dxgi.GetResource(&ID3D11Texture2D::IID, &mut raw).map_err(|e| fail("a picture's texture", e))?;
        let tex = ID3D11Texture2D::from_raw(raw);
        let index = dxgi.GetSubresourceIndex().map_err(|e| fail("a picture's texture", e))?;
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        tex.GetDesc(&mut desc);
        if live.ring.size != (desc.Width, desc.Height) || live.ring.slots.is_empty() {
            take_back(live, stream, true, ready)?;
            let mine = D3D11_TEXTURE2D_DESC {
                Width: desc.Width,
                Height: desc.Height,
                MipLevels: 1,
                ArraySize: 1,
                Format: desc.Format,
                SampleDesc: windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: 0,
            };
            let mut slots = Vec::new();
            for _ in 0..RING {
                let mut t = None;
                device.CreateTexture2D(&mine, None, Some(&mut t)).map_err(|e| fail("a staging texture", e))?;
                slots.extend(t);
            }
            live.ring = Ring { size: (desc.Width, desc.Height), slots, ..Ring::default() };
        }
        if live.ring.waiting.len() >= RING {
            take_one(live, stream, true, ready)?;
        }
        let slot = live.ring.next;
        live.ring.next = (slot + 1) % RING;
        let ctx = device.GetImmediateContext().map_err(|e| fail("the device's context", e))?;
        ctx.CopySubresourceRegion(&live.ring.slots[slot], 0, 0, 0, 0, &tex, index, None);
        ctx.Flush();
        live.ring.waiting.push_back((id, slot));
    }
    Ok(true)
}

/// the ring's pictures the GPU is done with, oldest first, into `ready` (`wait`: all of them, waiting)
fn take_back(live: &mut Live, stream: &Stream, wait: bool, ready: &mut VecDeque<Decoded>) -> Result<(), Error> {
    while !live.ring.waiting.is_empty() {
        if !take_one(live, stream, wait, ready)? {
            break;
        }
    }
    Ok(())
}

/// the ring's oldest picture into `ready`; false: the GPU is not done with it yet (only without `wait`)
fn take_one(live: &mut Live, stream: &Stream, wait: bool, ready: &mut VecDeque<Decoded>) -> Result<bool, Error> {
    let Some(&(id, slot)) = live.ring.waiting.front() else { return Ok(false) };
    let Some(device) = live._gpu.as_ref().map(|g| g.device.clone()) else { return Ok(false) };
    let t = Instant::now();
    let (w, h) = shown(live, stream);
    let mut data = Vec::with_capacity(w as usize * h as usize + 2 * w.div_ceil(2) as usize * h.div_ceil(2) as usize);
    // SAFETY: the staging texture is mapped while its rows are read, within its row pitch and height
    unsafe {
        let ctx = device.GetImmediateContext().map_err(|e| fail("the device's context", e))?;
        let tex = &live.ring.slots[slot];
        let mut map = D3D11_MAPPED_SUBRESOURCE::default();
        let flags = if wait { 0 } else { D3D11_MAP_FLAG_DO_NOT_WAIT.0 as u32 };
        match ctx.Map(tex, 0, D3D11_MAP_READ, flags, Some(&mut map)) {
            Ok(()) => {}
            Err(e) if e.code() == windows::Win32::Graphics::Dxgi::DXGI_ERROR_WAS_STILL_DRAWING => return Ok(false),
            Err(e) => return Err(fail("reading a picture", e)),
        }
        tight(map.pData as *const u8, map.RowPitch as usize, live.ring.size.1 as usize, (w, h), &mut data);
        ctx.Unmap(tex, 0);
    }
    live.ring.waiting.pop_front();
    COPIES.fetch_add(1, Ordering::Relaxed);
    COPY_NS.fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
    ready.push_back(Decoded { id, picture: Picture { width: w, height: h, format: Format::Nv12 { matrix: live.matrix, full: live.full }, data } });
    Ok(true)
}

/// the picture's size: the stream's, within the decoded frame
fn shown(live: &Live, stream: &Stream) -> (u32, u32) {
    let (fw, fh) = live.frame;
    if stream.width > 0 && stream.height > 0 {
        (stream.width.min(fw), stream.height.min(fh))
    } else {
        (fw, fh)
    }
}

/// A decoded sample in system memory (the decoder on the CPU) as a tight NV12 picture of the stream's size.
fn read(live: &Live, stream: &Stream, sample: &IMFSample) -> Result<Picture, Error> {
    let t = Instant::now();
    let (fw, fh) = live.frame;
    let (w, h) = shown(live, stream);
    let mut data = Vec::with_capacity(w as usize * h as usize + 2 * w.div_ceil(2) as usize * h.div_ceil(2) as usize);
    // SAFETY: the buffer is locked while its rows are read, within the pitch and the locked length
    unsafe {
        let buffer = sample.GetBufferByIndex(0).map_err(|e| fail("a picture's buffer", e))?;
        let (mut scan, mut len) = (std::ptr::null_mut(), 0u32);
        buffer.Lock(&mut scan, None, Some(&mut len)).map_err(|e| fail("reading a picture", e))?;
        let pitch = fw.max(1) as usize;
        tight(scan, pitch, (len as usize / pitch * 2 / 3).min(fh as usize).max(h as usize), (w, h), &mut data);
        let _ = buffer.Unlock();
    }
    COPIES.fetch_add(1, Ordering::Relaxed);
    COPY_NS.fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
    Ok(Picture { width: w, height: h, format: Format::Nv12 { matrix: live.matrix, full: live.full }, data })
}

impl DecodeBackend for MediaFoundation {
    fn caps(&self) -> Caps {
        Caps { codecs: if self.gpu { codecs() } else { CODECS.to_vec() }, hardware: self.gpu }
    }

    fn configure(&mut self, stream: &Stream) -> Result<(), Error> {
        self.annexb = AnnexB::new(&stream.codec, &stream.config)?;
        self.live = Some(self.open(stream)?);
        self.stream = Some(stream.clone());
        self.ready.clear();
        Ok(())
    }

    fn submit(&mut self, packet: Packet<'_>) -> Result<(), Error> {
        let data = match &self.annexb {
            Some(a) => {
                a.convert(packet.data, packet.key, &mut self.bytes)?;
                &self.bytes[..]
            }
            None => packet.data,
        };
        let Some(mft) = self.live.as_ref().map(|l| l.mft.clone()) else { return Err(Error("the decoder is not configured".into())) };
        // SAFETY: a sample with one memory buffer of the packet's bytes, written while locked
        let sample = unsafe {
            let buffer = MFCreateMemoryBuffer(data.len() as u32).map_err(|e| fail("buffer", e))?;
            let mut p = std::ptr::null_mut();
            buffer.Lock(&mut p, None, None).map_err(|e| fail("buffer", e))?;
            std::ptr::copy_nonoverlapping(data.as_ptr(), p, data.len());
            buffer.Unlock().map_err(|e| fail("buffer", e))?;
            buffer.SetCurrentLength(data.len() as u32).map_err(|e| fail("buffer", e))?;
            let s = MFCreateSample().map_err(|e| fail("sample", e))?;
            s.AddBuffer(&buffer).map_err(|e| fail("sample", e))?;
            s.SetSampleTime(packet.id).map_err(|e| fail("sample", e))?;
            if packet.key {
                s.SetUINT32(&MFSampleExtension_CleanPoint, 1).map_err(|e| fail("sample", e))?;
            }
            s
        };
        loop {
            // SAFETY: a live decoder and sample
            match unsafe { mft.ProcessInput(0, &sample, 0) } {
                Ok(()) => break,
                Err(e) if e.code() == MF_E_NOTACCEPTING => {
                    if self.pull()? == 0 {
                        return Err(Error("Media Foundation: the decoder takes no packet and gives no picture".into()));
                    }
                }
                Err(e) => return Err(fail("a packet", e)),
            }
        }
        self.pull()?;
        Ok(())
    }

    fn poll(&mut self) -> Option<Decoded> {
        if self.ready.is_empty() {
            if let (Some(live), Some(stream)) = (self.live.as_mut(), self.stream.as_ref()) {
                let _ = take_back(live, stream, false, &mut self.ready);
            }
        }
        self.ready.pop_front()
    }

    fn drain(&mut self) {
        if let Some(live) = &self.live {
            // SAFETY: a live decoder
            let _ = unsafe { live.mft.ProcessMessage(MFT_MESSAGE_COMMAND_DRAIN, 0) };
            let _ = self.pull();
        }
        if let (Some(live), Some(stream)) = (self.live.as_mut(), self.stream.as_ref()) {
            let _ = take_back(live, stream, true, &mut self.ready);
        }
    }

    fn reset(&mut self) {
        self.ready.clear();
        if let Some(live) = &mut self.live {
            live.ring.waiting.clear();
            // SAFETY: a live decoder
            let _ = unsafe { live.mft.ProcessMessage(MFT_MESSAGE_COMMAND_FLUSH, 0) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// the H.264 picture of a GPU decoder (where this machine has one) matches the software decoder's within rounding
    #[test]
    fn the_gpu_and_the_cpu_decode_the_same_picture() {
        let caps = codecs();
        eprintln!("Media Foundation on the GPU: {caps:?}");
        let Some(path) = std::env::var_os("DECK_HW_CLIP") else { return eprintln!("DECK_HW_CLIP unset: skipped") };
        let clip = deck_media::Clip::open(std::fs::read(path).unwrap().into()).unwrap();
        let codec = clip.video.as_ref().unwrap().codec.clone();
        if !caps.contains(&codec) {
            return eprintln!("no GPU decoder for {codec:?}: skipped");
        }
        let frames = |b: MediaFoundation| {
            let mut p = deck_media::Player::new(&clip, Box::new(b)).unwrap();
            let mut q = deck_media::Frames::new(8);
            [0.0, 0.5, 1.0].map(|t| p.frame_for(t, &mut q).unwrap().unwrap().picture.clone())
        };
        for (a, b) in frames(MediaFoundation::new()).iter().zip(frames(MediaFoundation::software()).iter()) {
            assert_eq!((a.width, a.height, a.format), (b.width, b.height, b.format));
            let worst = a.data.iter().zip(&b.data).map(|(x, y)| x.abs_diff(*y)).max().unwrap();
            assert!(worst <= 2, "worst {worst}");
        }
    }
}
