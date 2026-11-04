//! `Ctx` v2 (ABI-V2 §3): what a module sees of the world, read-only, written once per call by the host into the
//! `InHdr` and the module's declared regions – the show's three times, the tempo, the seed, the character with its
//! senses, the declared variable slots, the sensors, the assets, and the call's control and event inputs. A module
//! writes back only through its outputs.
use super::ev::Ev;
use super::hdr::{AssetInfo, AssetKind};
use super::io::Inp;
use super::layout::Sensor;
use super::rt::AssetBuf;
use crate::abi::get_str;
use crate::ctx::Tempo;

/// a declared variable slot (`#[reads("story.act")]` → the derive's `Self::STORY_ACT`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Var(pub u16);

/// a declared asset slot (`#[asset(name = "kick", …)]` → `Self::KICK`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetSlot(pub u8);

/// The character the module plays for (empty strings = none) and the being's senses, 0 … 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Character<'a> {
    pub id: &'a str,
    pub persona: &'a str,
    pub mood: &'a str,
    /// the mood's position in the scene's `moods` registry (cheaper to switch on than the string)
    pub mood_index: u32,
    pub energy: f32,
    pub touch: f32,
    pub strain: f32,
}

/// a sensor's state (§4.2); its values are zero unless `Live` or `Replay`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Status {
    /// the platform lacks it – or the module did not declare it
    Unavailable = 0,
    Asking = 1,
    Denied = 2,
    Live = 3,
    /// a recorded sensor log drives it (tests, conformance)
    Replay = 4,
}

impl Status {
    pub fn from_u32(v: u32) -> Status {
        [Status::Unavailable, Status::Asking, Status::Denied, Status::Live, Status::Replay].get(v as usize).copied().unwrap_or(Status::Unavailable)
    }

    /// values are there (live or replayed)
    pub fn live(self) -> bool {
        matches!(self, Status::Live | Status::Replay)
    }
}

/// `motion`: accel x y z (m/s², gravity included), rotation rate x y z (rad/s), orientation quaternion x y z w
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motion {
    pub status: Status,
    pub accel: [f32; 3],
    pub rotation: [f32; 3],
    pub orientation: [f32; 4],
}

/// `position` / `position.precise`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub status: Status,
    /// degrees
    pub lat: f32,
    pub lon: f32,
    /// m
    pub accuracy: f32,
    /// degrees
    pub heading: f32,
    /// m/s
    pub speed: f32,
}

/// `camera.face`: 52 blendshapes 0 … 1 and the head's yaw, pitch, roll
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Face<'a> {
    pub status: Status,
    pub shapes: &'a [f32],
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

/// `camera.motion`: motion energy 0 … 1, its centroid x y, the mean luma
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraMotion {
    pub status: Status,
    pub energy: f32,
    pub centroid: [f32; 2],
    pub luma: f32,
}

macro_rules! status_fn {
    ($($t:ty),*) => {$(
        impl $t {
            pub fn status(&self) -> Status {
                self.status
            }
        }
    )*};
}
status_fn!(Motion, Position, Face<'_>, CameraMotion);

/// A filled asset slot (§1.5): read-only, immutable for the instance's life.
#[derive(Debug, Clone, Copy)]
pub struct Asset<'a> {
    pub kind: AssetKind,
    pub bytes: &'a [u8],
    /// pcm: channels · image: 4
    pub channels: u32,
    /// pcm: frames per channel · image: the width
    pub frames: u32,
    /// pcm: the sample rate
    pub rate: f32,
    /// image: the height
    pub height: u32,
}

impl<'a> Asset<'a> {
    /// pcm: channel `c`'s frames (planar f32); empty for another kind or a channel past the asset's
    pub fn pcm(&self, c: usize) -> &'a [f32] {
        let (ch, n) = (self.channels as usize, self.frames as usize);
        if self.kind != AssetKind::Pcm || c >= ch || (c + 1) * n * 4 > self.bytes.len() {
            return &[];
        }
        // SAFETY: the buffer is 16-aligned (`AssetBuf`) and holds at least `(c + 1) · n` f32s (checked above)
        unsafe { core::slice::from_raw_parts((self.bytes.as_ptr() as *const f32).add(c * n), n) }
    }
}

/// One call's context. `Copy`: hand it around freely.
#[derive(Clone, Copy)]
pub struct Ctx<'a> {
    inp: Inp<'a>,
    assets: &'a [Option<AssetBuf>],
}

impl<'a> Ctx<'a> {
    pub fn new(inp: Inp<'a>, assets: &'a [Option<AssetBuf>]) -> Ctx<'a> {
        Ctx { inp, assets }
    }

    // --- time (§2.8 of the plan: three times, no wall clock)

    /// the audio frame of this call's first frame (the tempo map's axis)
    pub fn frame(&self) -> f64 {
        self.inp.hdr().frame
    }

    /// frames in this call (≤ `QUANTUM`)
    pub fn frames(&self) -> usize {
        self.inp.frames()
    }

    /// frames per second
    pub fn rate(&self) -> f64 {
        self.inp.hdr().rate
    }

    /// the show time at this call's first frame (ms, the relay's show clock); `None` without a show clock
    pub fn show_ms(&self) -> Option<f64> {
        let h = self.inp.hdr();
        (h.show_valid != 0).then_some(h.show_ms)
    }

    pub fn tempo(&self) -> Tempo {
        let h = self.inp.hdr();
        Tempo { bpm: h.bpm, rate: h.rate, running: h.running != 0, per_bar: h.per_bar.max(1), anchor_frame: h.tempo_frame, anchor_beat: h.tempo_beat }
    }

    /// the beat at this call's first frame, and at the frame after its last
    pub fn beats(&self) -> (f64, f64) {
        self.tempo().span(self.frame(), self.frames())
    }

    /// 0 … 1 within the beat at the call's first frame
    pub fn phase(&self) -> f64 {
        self.tempo().phase_at(self.frame())
    }

    /// the bar at the call's first frame, fractional (bar 0 starts at beat 0)
    pub fn bar(&self) -> f64 {
        let t = self.tempo();
        t.beat_at(self.frame()) / t.per_bar as f64
    }

    /// the node's seed (1 unless the node sets it) – the source of anything random
    pub fn seed(&self) -> u64 {
        self.inp.hdr().seed
    }

    pub fn character(&self) -> Character<'a> {
        let h = self.inp.hdr();
        Character {
            id: get_str(&h.character_id),
            persona: get_str(&h.character_persona),
            mood: get_str(&h.character_mood),
            mood_index: h.mood_index,
            energy: h.energy,
            touch: h.touch,
            strain: h.strain,
        }
    }

    /// a declared variable (`None`: not set, or not declared)
    pub fn var(&self, v: Var) -> Option<f32> {
        self.inp.vars().get(v.0 as usize).copied().filter(|x| !x.is_nan())
    }

    // --- the call's inputs (also on `Block`, which delegates here)

    /// control input `i`'s value this call (0 for an undeclared one)
    pub fn control_in(&self, i: usize) -> f32 {
        self.inp.control_in().get(i).copied().unwrap_or(0.0)
    }

    /// event input `p`'s records this call, sorted by frame
    pub fn events_in(&self, p: usize) -> &'a [Ev] {
        self.inp.events_in(p)
    }

    // --- sensors (§4)

    /// a sensor's status and channels; `Unavailable` and no channels when the module did not declare it
    pub fn sensor(&self, s: Sensor) -> (Status, &'a [f32]) {
        match self.inp.layout().sensor_slot(s).and_then(|slot| self.inp.sensor(slot)) {
            Some((status, values)) => (Status::from_u32(status), values),
            None => (Status::Unavailable, &[]),
        }
    }

    fn channels<const N: usize>(&self, s: Sensor) -> (Status, [f32; N]) {
        let (status, v) = self.sensor(s);
        let mut out = [0.0; N];
        let n = v.len().min(N);
        out[..n].copy_from_slice(&v[..n]);
        (status, out)
    }

    pub fn motion(&self) -> Motion {
        let (status, v) = self.channels::<10>(Sensor::Motion);
        Motion { status, accel: [v[0], v[1], v[2]], rotation: [v[3], v[4], v[5]], orientation: [v[6], v[7], v[8], v[9]] }
    }

    /// `position.precise` when declared, else `position`
    pub fn position(&self) -> Position {
        let which = if self.inp.layout().sensor_slot(Sensor::PositionPrecise).is_some() { Sensor::PositionPrecise } else { Sensor::Position };
        let (status, v) = self.channels::<5>(which);
        Position { status, lat: v[0], lon: v[1], accuracy: v[2], heading: v[3], speed: v[4] }
    }

    pub fn face(&self) -> Face<'a> {
        let (status, v) = self.sensor(Sensor::CameraFace);
        if v.len() < 55 {
            return Face { status, shapes: &[], yaw: 0.0, pitch: 0.0, roll: 0.0 };
        }
        Face { status, shapes: &v[..52], yaw: v[52], pitch: v[53], roll: v[54] }
    }

    pub fn camera_motion(&self) -> CameraMotion {
        let (status, v) = self.channels::<4>(Sensor::CameraMotion);
        CameraMotion { status, energy: v[0], centroid: [v[1], v[2]], luma: v[3] }
    }

    // --- assets (§1.5)

    /// a filled asset slot; `None` = not ready (play silence, never wait)
    pub fn asset(&self, slot: AssetSlot) -> Option<Asset<'a>> {
        let info: &AssetInfo = self.inp.asset(slot.0 as usize)?;
        let buf = self.assets.get(slot.0 as usize)?.as_ref()?;
        if info.ready == 0 || info.len as usize > buf.len() {
            return None;
        }
        Some(Asset {
            kind: AssetKind::from_u8(info.kind)?,
            bytes: &buf.bytes()[..info.len as usize],
            channels: info.channels as u32,
            frames: info.frames,
            rate: info.rate,
            height: info.height,
        })
    }
}
