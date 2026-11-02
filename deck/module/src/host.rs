//! The loader's side of the ABI (feature `host`): the descriptor a module declares, found in the wasm without running
//! it, validated; and where the fields of `Io` lie in the module's memory. Used by deck-native's wasmtime host, the
//! compile service (#281) and its tests. The browser reads the same JSON from `WebAssembly.Module.customSections`.
use crate::abi::*;
use core::mem::{offset_of, size_of};
use serde::{Deserialize, Serialize};

/// the custom section the derive writes the descriptor into
pub const SECTION: &str = "deck.module";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Dir {
    In,
    Out,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PortKind {
    Audio,
    Control,
    Video,
    /// events out (deck-events' model): a module's notes and triggers, not a graph cable
    Event,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamDesc {
    pub name: String,
    pub min: f32,
    pub max: f32,
    pub default: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortDesc {
    pub name: String,
    pub dir: Dir,
    pub kind: PortKind,
}

/// What a module declares: read before anything runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Descriptor {
    pub abi: u32,
    pub name: String,
    pub version: String,
    pub params: Vec<ParamDesc>,
    pub ports: Vec<PortDesc>,
}

fn ident(s: &str) -> bool {
    !s.is_empty() && s.len() <= 24 && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') && !s.as_bytes()[0].is_ascii_digit()
}

impl Descriptor {
    /// Parses and checks: the ABI version, the names, the ranges, the counts the ABI has room for.
    pub fn parse(json: &str) -> Result<Descriptor, String> {
        let d: Descriptor = serde_json::from_str(json).map_err(|e| format!("descriptor: {e}"))?;
        d.validate()?;
        Ok(d)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.abi != ABI_VERSION {
            return Err(format!("module '{}' speaks ABI {}, this host speaks {ABI_VERSION}", self.name, self.abi));
        }
        if !ident(&self.name) {
            return Err(format!("module name '{}' is not a lowercase identifier (a-z, 0-9, _; at most 24)", self.name));
        }
        if self.params.len() > MAX_PARAMS {
            return Err(format!("module '{}' has {} parameters, at most {MAX_PARAMS}", self.name, self.params.len()));
        }
        let mut names: Vec<&str> = Vec::new();
        for p in &self.params {
            if !ident(&p.name) {
                return Err(format!("parameter name '{}' is not a lowercase identifier", p.name));
            }
            if !(p.min.is_finite() && p.max.is_finite() && p.min < p.max && (p.min..=p.max).contains(&p.default)) {
                return Err(format!("parameter '{}': needs min < max and a default inside", p.name));
            }
            names.push(&p.name);
        }
        for p in &self.ports {
            if !ident(&p.name) {
                return Err(format!("port name '{}' is not a lowercase identifier", p.name));
            }
            names.push(&p.name);
        }
        names.sort_unstable();
        if let Some(w) = names.windows(2).find(|w| w[0] == w[1]) {
            return Err(format!("module '{}': the name '{}' is used twice (parameters and ports share one namespace)", self.name, w[0]));
        }
        for (dir, kind, max) in [
            (Dir::In, PortKind::Audio, MAX_AUDIO_PORTS),
            (Dir::Out, PortKind::Audio, MAX_AUDIO_PORTS),
            (Dir::In, PortKind::Control, MAX_CONTROL_IN),
            (Dir::Out, PortKind::Control, MAX_CONTROL_OUT),
        ] {
            let n = self.ports_of(dir, kind).count();
            if n > max {
                return Err(format!("module '{}' has {n} {kind:?} ports {dir:?}, at most {max}", self.name));
            }
        }
        Ok(())
    }

    /// the ports of one direction and kind, in declaration order – the n-th is `block.audio_in(n)` and so on
    pub fn ports_of(&self, dir: Dir, kind: PortKind) -> impl Iterator<Item = &PortDesc> {
        self.ports.iter().filter(move |p| p.dir == dir && p.kind == kind)
    }

    pub fn param_index(&self, name: &str) -> Option<usize> {
        self.params.iter().position(|p| p.name == name)
    }
}

/// The `deck.module` custom section of a wasm binary (the descriptor's JSON), without instantiating it.
pub fn read_descriptor(wasm: &[u8]) -> Option<&str> {
    fn leb(b: &[u8], at: &mut usize) -> Option<usize> {
        let (mut v, mut shift) = (0usize, 0);
        loop {
            let byte = *b.get(*at)?;
            *at += 1;
            v |= usize::from(byte & 0x7f).checked_shl(shift)?;
            if byte & 0x80 == 0 {
                return Some(v);
            }
            shift += 7;
            if shift > 28 {
                return None;
            }
        }
    }
    if wasm.get(..4)? != b"\0asm" {
        return None;
    }
    let mut at = 8;
    while at < wasm.len() {
        let id = wasm[at];
        at += 1;
        let size = leb(wasm, &mut at)?;
        let end = at.checked_add(size)?;
        if id == 0 {
            let mut p = at;
            let n = leb(wasm, &mut p)?;
            let name = wasm.get(p..p.checked_add(n)?)?;
            if name == SECTION.as_bytes() {
                return core::str::from_utf8(wasm.get(p + n..end)?).ok();
            }
        }
        at = end;
    }
    None
}

/// Where the fields of `Io` lie, in bytes from the address `dm_io` returns.
#[derive(Debug, Clone, Copy)]
pub struct Offsets {
    pub size: usize,
    pub event_size: usize,
    pub tempo_frame: usize,
    pub tempo_beat: usize,
    pub bpm: usize,
    pub rate: usize,
    pub frame: usize,
    pub seed: usize,
    pub frames: usize,
    pub running: usize,
    pub per_bar: usize,
    pub vars_len: usize,
    pub control_in: usize,
    pub character_id: usize,
    pub character_persona: usize,
    pub character_mood: usize,
    pub var_keys: usize,
    pub var_values: usize,
    pub audio_in: usize,
    pub audio_out: usize,
    pub control_out: usize,
    pub events_len: usize,
    pub events_dropped: usize,
    pub events: usize,
}

impl Offsets {
    pub const fn get() -> Offsets {
        let i = offset_of!(Io, inp);
        let o = offset_of!(Io, out);
        Offsets {
            size: size_of::<Io>(),
            event_size: size_of::<EventRec>(),
            tempo_frame: i + offset_of!(In, tempo_frame),
            tempo_beat: i + offset_of!(In, tempo_beat),
            bpm: i + offset_of!(In, bpm),
            rate: i + offset_of!(In, rate),
            frame: i + offset_of!(In, frame),
            seed: i + offset_of!(In, seed),
            frames: i + offset_of!(In, frames),
            running: i + offset_of!(In, running),
            per_bar: i + offset_of!(In, per_bar),
            vars_len: i + offset_of!(In, vars_len),
            control_in: i + offset_of!(In, control),
            character_id: i + offset_of!(In, character_id),
            character_persona: i + offset_of!(In, character_persona),
            character_mood: i + offset_of!(In, character_mood),
            var_keys: i + offset_of!(In, var_keys),
            var_values: i + offset_of!(In, var_values),
            audio_in: i + offset_of!(In, audio),
            audio_out: o + offset_of!(Out, audio),
            control_out: o + offset_of!(Out, control),
            events_len: o + offset_of!(Out, events_len),
            events_dropped: o + offset_of!(Out, events_dropped),
            events: o + offset_of!(Out, events),
        }
    }
}

/// Reads the events of the last call from a copy of `Io::out`'s event region.
pub fn event_at(events: &[u8], index: usize) -> Option<EventRec> {
    let size = size_of::<EventRec>();
    let b = events.get(index * size..(index + 1) * size)?;
    // SAFETY: EventRec is plain numbers and bytes, any bit pattern is valid; `b` is exactly one record, read unaligned
    Some(unsafe { core::ptr::read_unaligned(b.as_ptr() as *const EventRec) })
}
