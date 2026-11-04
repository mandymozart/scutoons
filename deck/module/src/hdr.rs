//! The two frozen headers at the start of `inp` and `out` (ABI-V2 §1.1, §3), and an asset slot's record (§1.5). Plain
//! `repr(C)` numbers and byte arrays, little-endian, the same on wasm32 and the host. A field added here is a new ABI
//! (v3), which is why the reserved bytes exist: a later field takes them only together with a version step.
use crate::layout::{MAX_AUDIO_PORTS, MAX_EVENT_PORTS};

/// bytes: a character's id / persona / mood (NUL-padded, cut on a char boundary)
pub const STR_LEN: usize = 32;

/// host → module, written once per call at the start of `inp` (256 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InHdr {
    /// the audio frame of this call's first frame, on the tempo map's axis
    pub frame: f64,
    /// frames per second
    pub rate: f64,
    /// the show time at the call's first frame (ms, relay-mapped, #321); valid when `show_valid` ≠ 0
    pub show_ms: f64,
    /// the tempo map's anchor (deck-core's `TempoMap`): frame `tempo_frame` is beat `tempo_beat`, from there `bpm`
    pub tempo_frame: f64,
    pub tempo_beat: f64,
    pub bpm: f64,
    /// the node's seed (1 unless the node sets it, §3)
    pub seed: u64,
    /// frames in this call, ≤ `QUANTUM`
    pub frames: u32,
    pub per_bar: u32,
    /// 0 = the tempo stands still
    pub running: u32,
    pub show_valid: u32,
    /// ≠ 0: an allocation inside `dm_process` / `dm_control` traps (tests and Play); 0: it is only counted (a show, §6)
    pub strict: u32,
    /// the mood's position in the scene's `moods` registry
    pub mood_index: u32,
    /// bit p: audio input p has nothing linked (zero-filled) – a module may early-out
    pub in_silent: u32,
    /// events the host clamped to frame 0 because their time had passed (counted, §1.4)
    pub late: u32,
    /// the being's senses, 0 … 1 (deck-space's `Senses`)
    pub energy: f32,
    pub touch: f32,
    pub strain: f32,
    pub _pad: u32,
    pub character_id: [u8; STR_LEN],
    pub character_persona: [u8; STR_LEN],
    pub character_mood: [u8; STR_LEN],
    /// per audio input / output port: the channels the cables carry now (≤ the port's reserved maximum)
    pub ch_in: [u8; MAX_AUDIO_PORTS],
    pub ch_out: [u8; MAX_AUDIO_PORTS],
    /// per event input port: the records in its region this call
    pub ev_in_len: [u16; MAX_EVENT_PORTS],
    pub _reserved: [u8; 24],
}

/// module → host, at the start of `out` (64 bytes). The SDK resets it at the start of each call.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutHdr {
    /// allocations made inside this call (the counting allocator, §6)
    pub allocs: u32,
    /// bit p: the module left audio output p silent (the engine may skip the sum)
    pub out_silent: u32,
    /// per event output port: the records written this call
    pub ev_out_len: [u16; MAX_EVENT_PORTS],
    /// per event output port: records past the capacity, dropped this call
    pub dropped: [u32; MAX_EVENT_PORTS],
    pub _reserved: [u8; 8],
}

impl OutHdr {
    pub const ZERO: OutHdr = OutHdr { allocs: 0, out_silent: 0, ev_out_len: [0; MAX_EVENT_PORTS], dropped: [0; MAX_EVENT_PORTS], _reserved: [0; 8] };
}

/// An asset slot as the host filled it at instantiation (§1.5; 24 bytes). `ready` = 0: not (yet) there – the module
/// plays silence, never waits.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssetInfo {
    /// the address `dm_asset_buf` returned (in the module's memory)
    pub ptr: u32,
    /// bytes
    pub len: u32,
    /// pcm: frames per channel · image: the width in pixels
    pub frames: u32,
    /// pcm: the asset's sample rate (resampling is the module's)
    pub rate: f32,
    /// `AssetKind as u8`: 0 pcm (planar f32), 1 bytes, 2 image (RGBA8)
    pub kind: u8,
    /// pcm: channels · image: 4
    pub channels: u8,
    pub ready: u8,
    pub _pad: u8,
    /// image: the height in pixels
    pub height: u32,
}

impl AssetInfo {
    /// the record of a slot the host has filled (`ready`): `ptr` from `dm_asset_buf`, `len` bytes; pcm: `channels` ×
    /// `frames` at `rate`; image: `frames` is the width, the height follows from the length (RGBA8)
    pub fn filled(ptr: u32, kind: AssetKind, len: u32, channels: u8, frames: u32, rate: f32) -> AssetInfo {
        let height = if kind == AssetKind::Image && frames > 0 { len / (4 * frames) } else { 0 };
        AssetInfo { ptr, len, frames, rate, kind: kind as u8, channels, ready: 1, _pad: 0, height }
    }
}

/// what an asset slot holds (deck-catalog's `AssetKind`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AssetKind {
    Pcm = 0,
    Bytes = 1,
    Image = 2,
}

impl AssetKind {
    pub fn from_u8(k: u8) -> Option<AssetKind> {
        [AssetKind::Pcm, AssetKind::Bytes, AssetKind::Image].get(k as usize).copied()
    }
}

impl InHdr {
    /// all zero
    pub const ZERO: InHdr = InHdr {
        frame: 0.0,
        rate: 0.0,
        show_ms: 0.0,
        tempo_frame: 0.0,
        tempo_beat: 0.0,
        bpm: 0.0,
        seed: 0,
        frames: 0,
        per_bar: 0,
        running: 0,
        show_valid: 0,
        strict: 0,
        mood_index: 0,
        in_silent: 0,
        late: 0,
        energy: 0.0,
        touch: 0.0,
        strain: 0.0,
        _pad: 0,
        character_id: [0; STR_LEN],
        character_persona: [0; STR_LEN],
        character_mood: [0; STR_LEN],
        ch_in: [0; MAX_AUDIO_PORTS],
        ch_out: [0; MAX_AUDIO_PORTS],
        ev_in_len: [0; MAX_EVENT_PORTS],
        _reserved: [0; 24],
    };
}

/// write `s` into a NUL-padded byte field, cut on a char boundary
pub fn put_str(field: &mut [u8], s: &str) {
    let mut n = s.len().min(field.len());
    while !s.is_char_boundary(n) {
        n -= 1;
    }
    field[..n].copy_from_slice(&s.as_bytes()[..n]);
    field[n..].fill(0);
}

/// read a NUL-padded byte field (up to the first NUL; invalid UTF-8 reads as empty)
pub fn get_str(field: &[u8]) -> &str {
    let n = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    core::str::from_utf8(&field[..n]).unwrap_or("")
}

const _: () = assert!(core::mem::size_of::<InHdr>() == 256);
const _: () = assert!(core::mem::size_of::<OutHdr>() == 64);
const _: () = assert!(core::mem::size_of::<AssetInfo>() == 24);
