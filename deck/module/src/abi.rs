//! The ABI between a module (wasm32-unknown-unknown) and its host, version `ABI_VERSION`.
//!
//! A module owns one `Io` in its linear memory and exports its address (`dm_io`). The host writes the call's inputs
//! (`Io::inp`) there, calls `dm_process` / `dm_control`, and reads the outputs (`Io::out`) back. Plain `repr(C)` numbers
//! and byte arrays only – no pointers – so the layout is the same on wasm32 and on the host, which computes the offsets
//! from this very type (`host::Offsets`). Nothing here allocates; a call is at most `MAX_BLOCK` frames (the host splits
//! a longer block).

pub const ABI_VERSION: u32 = 1;
/// frames in one call
pub const MAX_BLOCK: usize = 1024;
/// audio ports a module has per direction; each is a stereo pair (the graph's `Stereo` block)
pub const MAX_AUDIO_PORTS: usize = 4;
/// control inputs / outputs besides the parameters – one f32 each, read / written once a call
pub const MAX_CONTROL_IN: usize = 8;
pub const MAX_CONTROL_OUT: usize = 8;
/// parameters (`#[param]` fields)
pub const MAX_PARAMS: usize = 128;
/// scene and world variables (key → number) the host hands over
pub const MAX_VARS: usize = 32;
/// events a call can emit (the rest is counted in `Out::events_dropped`)
pub const MAX_EVENTS: usize = 64;
/// props on one event
pub const MAX_PROPS: usize = 4;
/// bytes: a character's id / persona / mood (NUL-padded)
pub const STR_LEN: usize = 32;
/// bytes: a variable's key, an event's name, an event prop's key (NUL-padded)
pub const VAR_KEY_LEN: usize = 24;
pub const EVENT_NAME_LEN: usize = 16;
pub const PROP_KEY_LEN: usize = 8;

/// One event a module emits (deck-events' model, flat): a name, a start and length in beats, up to `MAX_PROPS` props.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct EventRec {
    /// absolute beats on the tempo; NaN = now (the call's first frame)
    pub start: f64,
    /// beats; 0 = a moment
    pub len: f64,
    /// the declared event port (counted among the event outputs)
    pub port: u32,
    pub nprops: u32,
    pub name: [u8; EVENT_NAME_LEN],
    pub keys: [[u8; PROP_KEY_LEN]; MAX_PROPS],
    pub values: [f32; MAX_PROPS],
}

/// host → module
#[repr(C)]
pub struct In {
    /// the tempo map's anchor (deck-core's `TempoMap`): frame `tempo_frame` is beat `tempo_beat`, from there `bpm`
    pub tempo_frame: f64,
    pub tempo_beat: f64,
    pub bpm: f64,
    /// frames per second
    pub rate: f64,
    /// the audio frame (the tempo map's axis) of this call's first frame
    pub frame: f64,
    /// the module's seed, set before `dm_new` and kept: two decks with the same seed play the same
    pub seed: u64,
    /// frames in this call (≤ MAX_BLOCK)
    pub frames: u32,
    /// 0 = the tempo stands still
    pub running: u32,
    pub per_bar: u32,
    /// used entries of `var_keys` / `var_values`
    pub vars_len: u32,
    /// the control inputs (declared `#[port(input, control)]`, in order)
    pub control: [f32; MAX_CONTROL_IN],
    pub character_id: [u8; STR_LEN],
    pub character_persona: [u8; STR_LEN],
    pub character_mood: [u8; STR_LEN],
    pub var_keys: [[u8; VAR_KEY_LEN]; MAX_VARS],
    pub var_values: [f32; MAX_VARS],
    /// the audio inputs: port, channel (left, right), frame
    pub audio: [[[f32; MAX_BLOCK]; 2]; MAX_AUDIO_PORTS],
}

/// module → host
#[repr(C)]
pub struct Out {
    /// the audio outputs: port, channel, frame (cleared at the start of each `dm_process`)
    pub audio: [[[f32; MAX_BLOCK]; 2]; MAX_AUDIO_PORTS],
    /// the control outputs (declared `#[port(output, control)]`, in order): the last value written
    pub control: [f32; MAX_CONTROL_OUT],
    pub events_len: u32,
    pub events_dropped: u32,
    pub events: [EventRec; MAX_EVENTS],
}

#[repr(C)]
pub struct Io {
    pub inp: In,
    pub out: Out,
}

impl Io {
    /// all zero (a static's initial state; the host sets `inp` before the first call)
    pub const ZERO: Io = unsafe { core::mem::zeroed() };

    /// a zeroed `Io` on the heap (~72 KB: never through the stack)
    pub fn boxed() -> Box<Io> {
        let layout = std::alloc::Layout::new::<Io>();
        unsafe {
            let p = std::alloc::alloc_zeroed(layout) as *mut Io;
            assert!(!p.is_null(), "out of memory");
            Box::from_raw(p)
        }
    }
}

/// write `s` into a NUL-padded byte field (cut at a char boundary if too long)
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
