//! ABI v2's memory model (research/ABI-V2.md §1.1): a module's memory layout is **derived from its descriptor**, not a
//! fixed `Io`. One pure function, [`Layout::of`], places a frozen header and the regions the descriptor declares
//! (params, control, audio, vars, sensors, assets, events) back to back: one contiguous `inp` block the host writes and
//! one `out` block right after it that the host reads, per call. The derive evaluates it as a `const` (the module's
//! static buffer is exactly `size` bytes); the host evaluates the same function at load from the parsed descriptor
//! ([`Shape::of`], feature `host`). The module exports [`Layout::hash`] as `dm_layout()` and the host refuses a mismatch.
//!
//! The input to the layout is a [`Shape`] – the few numbers of a descriptor that place bytes – so the layout never
//! depends on the graph (an `any` port reserves its maximum; the header byte `ch_in[p]` / `ch_out[p]` says how many
//! channels the cables carry) and can be hashed and tested.
use crate::ev::Ev;
use crate::hdr::{AssetInfo, InHdr, OutHdr};
use core::mem::size_of;

/// the ABI version a v2 module speaks (`dm_abi()`, the descriptor's `abi`)
pub const ABI_VERSION: u32 = 2;
/// frames in one call at most: the engine's quantum (#317); a smaller `frames` is for tests
pub const QUANTUM: usize = 128;
/// channels of one audio port at most (an `any` port reserves this many)
pub const MAX_CHANNELS: usize = 32;
/// audio (and cv) ports per direction
pub const MAX_AUDIO_PORTS: usize = 8;
/// event ports per direction
pub const MAX_EVENT_PORTS: usize = 8;
/// control ports per direction (besides the parameters) – a sanity bound
pub const MAX_CONTROL: usize = 256;
/// parameters – a sanity bound, not a design cap (§2)
pub const MAX_PARAMS: usize = 1024;
/// declared variable slots (`reads`, §3)
pub const MAX_VARS: usize = 256;
/// asset slots (§1.5)
pub const MAX_ASSETS: usize = 16;
/// records per block an event port may declare (deck-catalog's `MAX_EVENT_CAPACITY`)
pub const MAX_EVENT_CAPACITY: usize = 512;
/// every region starts on this many bytes
pub const ALIGN: u32 = 16;

/// The sensor vocabulary of ABI v2 (§4.1), closed: a name in `needs` / `wants` outside it is refused. Each is a group
/// of `f32` channels, block-rate, read from `Ctx`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Sensor {
    /// accel x y z (m/s², gravity included), rotation rate x y z (rad/s), orientation quaternion x y z w
    Motion = 0,
    /// lat °, lon °, accuracy m, heading °, speed m/s – coarse (≈ 100 m)
    Position = 1,
    /// as `Position`, not rounded (the audience sees the name)
    PositionPrecise = 2,
    /// 52 blendshapes 0…1, then head yaw, pitch, roll
    CameraFace = 3,
    /// motion energy 0…1, centroid x y, mean luma
    CameraMotion = 4,
}

/// how many sensors a module can declare (each name at most once)
pub const MAX_SENSORS: usize = Sensor::ALL.len();

impl Sensor {
    pub const ALL: [Sensor; 5] = [Sensor::Motion, Sensor::Position, Sensor::PositionPrecise, Sensor::CameraFace, Sensor::CameraMotion];

    /// the name in a descriptor's `needs` / `wants`
    pub const fn name(self) -> &'static str {
        match self {
            Sensor::Motion => "motion",
            Sensor::Position => "position",
            Sensor::PositionPrecise => "position.precise",
            Sensor::CameraFace => "camera.face",
            Sensor::CameraMotion => "camera.motion",
        }
    }

    /// the channels after the status word
    pub const fn channels(self) -> u32 {
        match self {
            Sensor::Motion => 10,
            Sensor::Position | Sensor::PositionPrecise => 5,
            Sensor::CameraFace => 55,
            Sensor::CameraMotion => 4,
        }
    }

    /// the vocabulary's entry for `name` – a `const fn`, so the derive's constant shape refuses an unknown name
    pub const fn from_name(name: &str) -> Option<Sensor> {
        let mut i = 0;
        while i < Sensor::ALL.len() {
            let (a, b) = (Sensor::ALL[i].name().as_bytes(), name.as_bytes());
            if a.len() == b.len() {
                let mut j = 0;
                while j < a.len() && a[j] == b[j] {
                    j += 1;
                }
                if j == a.len() {
                    return Some(Sensor::ALL[i]);
                }
            }
            i += 1;
        }
        None
    }
}

/// The numbers of a descriptor that place bytes. Ports are in declaration order per direction; an array entry past
/// the last port is 0 (a port has at least 1 channel and a capacity of at least 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shape {
    /// `#[param]`s
    pub params: u16,
    /// control input ports (the parameters are inputs too, through the params table)
    pub control_in: u16,
    pub control_out: u16,
    /// declared variable slots (`reads`)
    pub vars: u16,
    /// per audio / cv input port: the channels the region reserves (`any` = `MAX_CHANNELS`, cv = 2)
    pub audio_in: [u8; MAX_AUDIO_PORTS],
    pub audio_out: [u8; MAX_AUDIO_PORTS],
    /// per event input port: its capacity (records per block)
    pub events_in: [u16; MAX_EVENT_PORTS],
    pub events_out: [u16; MAX_EVENT_PORTS],
    /// the declared sensors, `needs` then `wants`, in order
    pub sensors: [Option<Sensor>; MAX_SENSORS],
    /// asset slots
    pub assets: u8,
}

const fn count_u8(a: &[u8]) -> usize {
    let mut i = 0;
    while i < a.len() && a[i] != 0 {
        i += 1;
    }
    i
}

const fn count_u16(a: &[u16]) -> usize {
    let mut i = 0;
    while i < a.len() && a[i] != 0 {
        i += 1;
    }
    i
}

impl Shape {
    /// no ports, no parameters
    pub const EMPTY: Shape = Shape {
        params: 0,
        control_in: 0,
        control_out: 0,
        vars: 0,
        audio_in: [0; MAX_AUDIO_PORTS],
        audio_out: [0; MAX_AUDIO_PORTS],
        events_in: [0; MAX_EVENT_PORTS],
        events_out: [0; MAX_EVENT_PORTS],
        sensors: [None; MAX_SENSORS],
        assets: 0,
    };

    pub const fn audio_in_ports(&self) -> usize {
        count_u8(&self.audio_in)
    }
    pub const fn audio_out_ports(&self) -> usize {
        count_u8(&self.audio_out)
    }
    pub const fn event_in_ports(&self) -> usize {
        count_u16(&self.events_in)
    }
    pub const fn event_out_ports(&self) -> usize {
        count_u16(&self.events_out)
    }
    pub const fn sensor_count(&self) -> usize {
        let mut i = 0;
        while i < MAX_SENSORS && self.sensors[i].is_some() {
            i += 1;
        }
        i
    }

    /// the shape's limits; `Err` names the first one broken (the derive and `Shape::of` refuse before this)
    pub const fn check(&self) -> Result<(), &'static str> {
        if self.params as usize > MAX_PARAMS {
            return Err("more than MAX_PARAMS parameters");
        }
        if self.control_in as usize > MAX_CONTROL || self.control_out as usize > MAX_CONTROL {
            return Err("more than MAX_CONTROL control ports in a direction");
        }
        if self.vars as usize > MAX_VARS {
            return Err("more than MAX_VARS variable slots");
        }
        if self.assets as usize > MAX_ASSETS {
            return Err("more than MAX_ASSETS asset slots");
        }
        let mut i = 0;
        while i < MAX_AUDIO_PORTS {
            if self.audio_in[i] as usize > MAX_CHANNELS || self.audio_out[i] as usize > MAX_CHANNELS {
                return Err("an audio port with more than MAX_CHANNELS channels");
            }
            if (i >= self.audio_in_ports() && self.audio_in[i] != 0) || (i >= self.audio_out_ports() && self.audio_out[i] != 0) {
                return Err("an audio port after a port of 0 channels");
            }
            i += 1;
        }
        let mut i = 0;
        while i < MAX_EVENT_PORTS {
            if self.events_in[i] as usize > MAX_EVENT_CAPACITY || self.events_out[i] as usize > MAX_EVENT_CAPACITY {
                return Err("an event port above MAX_EVENT_CAPACITY");
            }
            if (i >= self.event_in_ports() && self.events_in[i] != 0) || (i >= self.event_out_ports() && self.events_out[i] != 0) {
                return Err("an event port after a port of capacity 0");
            }
            i += 1;
        }
        let n = self.sensor_count();
        let mut i = 0;
        while i < MAX_SENSORS {
            if i >= n && self.sensors[i].is_some() {
                return Err("a sensor after an empty sensor slot");
            }
            let mut j = 0;
            while j < i {
                if let (Some(a), Some(b)) = (self.sensors[i], self.sensors[j]) {
                    if a as u8 == b as u8 {
                        return Err("a sensor declared twice");
                    }
                }
                j += 1;
            }
            i += 1;
        }
        Ok(())
    }
}

/// Where every region lies, in bytes from the address `dm_io()` returns. `inp` is `[0, inp_size)`, starting with the
/// `InHdr`; `out` is `[out, size)`, starting with the `OutHdr`. Entries of the per-port arrays past the last port are 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub shape: Shape,
    /// `params f32[P]`
    pub params: u32,
    /// `dirty u32[⌈P/32⌉]`: bit i = parameter i changed since the previous call
    pub dirty: u32,
    /// `control_in f32[Ci]`
    pub control_in: u32,
    /// per port: `f32[ch][QUANTUM]`, planar (channel c's frames start at `c · QUANTUM`)
    pub audio_in: [u32; MAX_AUDIO_PORTS],
    /// `vars f32[V]`, NaN = not set
    pub vars: u32,
    /// per declared sensor: `status u32`, then `f32[channels]`
    pub sensors: [u32; MAX_SENSORS],
    /// `AssetInfo[A]`
    pub assets: u32,
    /// per port: `Ev[capacity]`, the first `InHdr.ev_in_len[p]` used, sorted by frame
    pub events_in: [u32; MAX_EVENT_PORTS],
    /// bytes the host writes: `[0, inp_size)`
    pub inp_size: u32,
    /// the `OutHdr`; also the size of `inp` rounded to `ALIGN`
    pub out: u32,
    /// `control_out f32[Co]`
    pub control_out: u32,
    /// `written u32[⌈Co/32⌉]`: bit i = control output i was written in this call (unwritten = the consumer holds)
    pub written: u32,
    pub audio_out: [u32; MAX_AUDIO_PORTS],
    /// per port: `Ev[capacity]`, the first `OutHdr.ev_out_len[p]` used
    pub events_out: [u32; MAX_EVENT_PORTS],
    /// all bytes, `inp` and `out`
    pub size: u32,
    /// `dm_layout()`: FNV-1a over the ABI version, the record sizes, the shape and every offset
    pub hash: u32,
}

const fn up(at: u32) -> u32 {
    at.div_ceil(ALIGN) * ALIGN
}

const fn words(bits: u32) -> u32 {
    bits.div_ceil(32)
}

const fn fnv(mut h: u32, v: u32) -> u32 {
    let b = v.to_le_bytes();
    let mut i = 0;
    while i < 4 {
        h ^= b[i] as u32;
        h = h.wrapping_mul(0x0100_0193);
        i += 1;
    }
    h
}

const fn fnv_all(mut h: u32, vs: &[u32]) -> u32 {
    let mut i = 0;
    while i < vs.len() {
        h = fnv(h, vs[i]);
        i += 1;
    }
    h
}

/// the sizes of the frozen records: a change to one of them is a new ABI
pub const IN_HDR_SIZE: u32 = size_of::<InHdr>() as u32;
pub const OUT_HDR_SIZE: u32 = size_of::<OutHdr>() as u32;
pub const EV_SIZE: u32 = size_of::<Ev>() as u32;
pub const ASSET_INFO_SIZE: u32 = size_of::<AssetInfo>() as u32;

impl Layout {
    /// The one layout function (SDK and host). Panics on a shape that breaks [`Shape::check`] – in a `const`, that is
    /// a compile error at the derive.
    pub const fn of(shape: &Shape) -> Layout {
        if let Err(e) = shape.check() {
            panic!("{}", e);
        }
        let s = *shape;
        let q = QUANTUM as u32;
        let mut at = up(IN_HDR_SIZE);
        let params = at;
        at = up(at + 4 * s.params as u32);
        let dirty = at;
        at = up(at + 4 * words(s.params as u32));
        let control_in = at;
        at = up(at + 4 * s.control_in as u32);
        let mut audio_in = [0u32; MAX_AUDIO_PORTS];
        let mut i = 0;
        while i < s.audio_in_ports() {
            audio_in[i] = at;
            at = up(at + 4 * q * s.audio_in[i] as u32);
            i += 1;
        }
        let vars = at;
        at = up(at + 4 * s.vars as u32);
        let mut sensors = [0u32; MAX_SENSORS];
        let mut i = 0;
        while i < s.sensor_count() {
            sensors[i] = at;
            let ch = match s.sensors[i] {
                Some(x) => x.channels(),
                None => 0,
            };
            at = up(at + 4 + 4 * ch);
            i += 1;
        }
        let assets = at;
        at = up(at + ASSET_INFO_SIZE * s.assets as u32);
        let mut events_in = [0u32; MAX_EVENT_PORTS];
        let mut i = 0;
        while i < s.event_in_ports() {
            events_in[i] = at;
            at = up(at + EV_SIZE * s.events_in[i] as u32);
            i += 1;
        }
        let inp_size = at;
        let out = at;
        at = up(at + OUT_HDR_SIZE);
        let control_out = at;
        at = up(at + 4 * s.control_out as u32);
        let written = at;
        at = up(at + 4 * words(s.control_out as u32));
        let mut audio_out = [0u32; MAX_AUDIO_PORTS];
        let mut i = 0;
        while i < s.audio_out_ports() {
            audio_out[i] = at;
            at = up(at + 4 * q * s.audio_out[i] as u32);
            i += 1;
        }
        let mut events_out = [0u32; MAX_EVENT_PORTS];
        let mut i = 0;
        while i < s.event_out_ports() {
            events_out[i] = at;
            at = up(at + EV_SIZE * s.events_out[i] as u32);
            i += 1;
        }
        let size = at;

        let mut h = fnv_all(0x811c_9dc5, &[ABI_VERSION, IN_HDR_SIZE, OUT_HDR_SIZE, EV_SIZE, ASSET_INFO_SIZE, q, ALIGN]);
        h = fnv_all(h, &[s.params as u32, s.control_in as u32, s.control_out as u32, s.vars as u32, s.assets as u32]);
        let mut i = 0;
        while i < MAX_AUDIO_PORTS {
            h = fnv_all(h, &[s.audio_in[i] as u32, s.audio_out[i] as u32, audio_in[i], audio_out[i]]);
            i += 1;
        }
        let mut i = 0;
        while i < MAX_EVENT_PORTS {
            h = fnv_all(h, &[s.events_in[i] as u32, s.events_out[i] as u32, events_in[i], events_out[i]]);
            i += 1;
        }
        let mut i = 0;
        while i < MAX_SENSORS {
            let id = match s.sensors[i] {
                Some(x) => x as u32 + 1,
                None => 0,
            };
            h = fnv_all(h, &[id, sensors[i]]);
            i += 1;
        }
        h = fnv_all(h, &[params, dirty, control_in, vars, assets, inp_size, out, control_out, written, size]);

        Layout {
            shape: s,
            params,
            dirty,
            control_in,
            audio_in,
            vars,
            sensors,
            assets,
            events_in,
            inp_size,
            out,
            control_out,
            written,
            audio_out,
            events_out,
            size,
            hash: h,
        }
    }

    /// the slot of a declared sensor (`None` = the module did not declare it)
    pub fn sensor_slot(&self, sensor: Sensor) -> Option<usize> {
        self.shape.sensors[..self.shape.sensor_count()].iter().position(|s| *s == Some(sensor))
    }
}

#[cfg(feature = "host")]
mod from_descriptor {
    use super::*;
    use deck_catalog::{Channels, Descriptor, Dir, PortType};

    impl Shape {
        /// The shape a descriptor declares, with the limits of ABI v2 – the host's side of the derive's constant.
        /// `video` ports take no region (the host refuses them with its own message until #185).
        pub fn of(d: &Descriptor) -> Result<Shape, String> {
            let me = &d.name;
            let mut s = Shape::EMPTY;
            if d.params.len() > MAX_PARAMS {
                return Err(format!("module '{me}' has {} parameters, at most {MAX_PARAMS}", d.params.len()));
            }
            s.params = d.params.len() as u16;
            let (mut ai, mut ao, mut ei, mut eo, mut ci, mut co) = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
            for p in d.ports.iter() {
                let at = format!("module '{me}': port '{}'", p.name);
                match p.ty {
                    PortType::Audio | PortType::Cv => {
                        let ch = match (p.ty, p.channels) {
                            (PortType::Cv, _) => 2,
                            (_, None) => 2,
                            (_, Some(Channels::Any(_))) => MAX_CHANNELS,
                            (_, Some(Channels::Count(n))) => n as usize,
                        };
                        if !(1..=MAX_CHANNELS).contains(&ch) {
                            return Err(format!("{at}: {ch} channels, 1 … {MAX_CHANNELS}"));
                        }
                        let (n, list) = if p.dir == Dir::In { (&mut ai, &mut s.audio_in) } else { (&mut ao, &mut s.audio_out) };
                        if *n >= MAX_AUDIO_PORTS {
                            return Err(format!("{at}: more than {MAX_AUDIO_PORTS} audio ports {:?}", p.dir));
                        }
                        list[*n] = ch as u8;
                        *n += 1;
                    }
                    PortType::Event => {
                        let (n, list) = if p.dir == Dir::In { (&mut ei, &mut s.events_in) } else { (&mut eo, &mut s.events_out) };
                        if *n >= MAX_EVENT_PORTS {
                            return Err(format!("{at}: more than {MAX_EVENT_PORTS} event ports {:?}", p.dir));
                        }
                        if !(1..=MAX_EVENT_CAPACITY).contains(&(p.capacity as usize)) {
                            return Err(format!("{at}: capacity {}, 1 … {MAX_EVENT_CAPACITY}", p.capacity));
                        }
                        list[*n] = p.capacity;
                        *n += 1;
                    }
                    PortType::Control => {
                        let n = if p.dir == Dir::In { &mut ci } else { &mut co };
                        if *n >= MAX_CONTROL {
                            return Err(format!("{at}: more than {MAX_CONTROL} control ports {:?}", p.dir));
                        }
                        *n += 1;
                    }
                    PortType::Video => {}
                }
            }
            (s.control_in, s.control_out) = (ci as u16, co as u16);
            if d.reads.len() > MAX_VARS {
                return Err(format!("module '{me}' reads {} variables, at most {MAX_VARS}", d.reads.len()));
            }
            s.vars = d.reads.len() as u16;
            for (n, name) in d.needs.iter().chain(d.wants.iter()).enumerate() {
                let sensor = Sensor::from_name(name).ok_or_else(|| {
                    let known: Vec<&str> = Sensor::ALL.iter().map(|s| s.name()).collect();
                    format!("module '{me}' declares the capability '{name}', which ABI {ABI_VERSION} does not know ({})", known.join(", "))
                })?;
                if s.sensors[..n].contains(&Some(sensor)) {
                    return Err(format!("module '{me}' declares '{name}' twice"));
                }
                if n >= MAX_SENSORS {
                    return Err(format!("module '{me}' declares more than {MAX_SENSORS} capabilities"));
                }
                s.sensors[n] = Some(sensor);
            }
            if d.assets.len() > MAX_ASSETS {
                return Err(format!("module '{me}' has {} asset slots, at most {MAX_ASSETS}", d.assets.len()));
            }
            s.assets = d.assets.len() as u8;
            s.check().map_err(|e| format!("module '{me}': {e}"))?;
            Ok(s)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frozen_records_have_their_sizes() {
        assert_eq!((IN_HDR_SIZE, OUT_HDR_SIZE, EV_SIZE, ASSET_INFO_SIZE), (256, 64, 40, 24));
    }

    #[test]
    fn an_empty_module_is_two_headers() {
        let l = Layout::of(&Shape::EMPTY);
        assert_eq!((l.inp_size, l.out, l.size), (256, 256, 320));
    }

    #[test]
    fn regions_are_aligned_ordered_and_sized_by_the_shape() {
        let mut s = Shape::EMPTY;
        (s.params, s.control_in, s.control_out, s.vars, s.assets) = (3, 1, 33, 2, 1);
        s.audio_in[..2].copy_from_slice(&[2, 5]);
        s.audio_out[0] = 1;
        s.events_in[0] = 64;
        s.events_out[..2].copy_from_slice(&[3, 512]);
        s.sensors[..2].copy_from_slice(&[Some(Sensor::Motion), Some(Sensor::CameraMotion)]);
        let l = Layout::of(&s);
        let starts = [
            l.params,
            l.dirty,
            l.control_in,
            l.audio_in[0],
            l.audio_in[1],
            l.vars,
            l.sensors[0],
            l.sensors[1],
            l.assets,
            l.events_in[0],
            l.out,
            l.control_out,
            l.written,
            l.audio_out[0],
            l.events_out[0],
            l.events_out[1],
            l.size,
        ];
        assert!(starts.iter().all(|a| a % ALIGN == 0), "{starts:?}");
        assert!(starts.windows(2).all(|w| w[0] < w[1]), "{starts:?}");
        assert_eq!(l.audio_in[1] - l.audio_in[0], 2 * 128 * 4);
        assert_eq!(l.vars - l.audio_in[1], 5 * 128 * 4);
        assert_eq!(l.sensors[1] - l.sensors[0], up(4 + 10 * 4));
        assert_eq!(l.written - l.control_out, up(33 * 4));
        assert_eq!(l.size - l.events_out[1], 512 * 40);
        assert_eq!(l.inp_size, l.out);
        assert_eq!((l.sensor_slot(Sensor::CameraMotion), l.sensor_slot(Sensor::Position)), (Some(1), None));
    }

    #[test]
    fn a_stereo_synth_is_a_few_kilobytes_each_way() {
        // §1.1: "A stereo synth is ≈ 5 KB each way (v1 ≈ 72 KB)"
        let mut s = Shape::EMPTY;
        (s.params, s.control_in) = (91, 1);
        s.events_in[0] = 64;
        s.audio_out[0] = 2;
        let l = Layout::of(&s);
        assert!(l.inp_size < 4096 && l.size - l.out < 2048, "{l:?}");
    }

    /// The layout function is frozen with ABI 2: these numbers change only together with `ABI_VERSION` (a v3 and a
    /// recompile of every stored module, §7). A failure here means the layout moved – a module built before would be
    /// refused by `dm_layout()`.
    #[test]
    fn the_layout_hash_is_frozen() {
        let mut synth = Shape::EMPTY;
        (synth.params, synth.control_in) = (91, 1);
        synth.events_in[0] = 64;
        synth.audio_out[0] = 2;
        let mut wide = Shape::EMPTY;
        (wide.params, wide.control_in, wide.control_out, wide.vars, wide.assets) = (3, 1, 33, 2, 1);
        wide.audio_in[..2].copy_from_slice(&[2, 5]);
        wide.audio_out[0] = 32;
        wide.events_out[..2].copy_from_slice(&[3, 512]);
        wide.sensors[..2].copy_from_slice(&[Some(Sensor::Motion), Some(Sensor::CameraFace)]);
        let got = [Shape::EMPTY, synth, wide].map(|s| Layout::of(&s)).map(|l| (l.size, l.hash));
        assert_eq!(got, FROZEN, "the layout moved: {got:x?}");
    }
    const FROZEN: [(u32, u32); 3] = [(0x140, 0xe48e_706c), (0x10d0, 0xd5a8_be91), (0xa1d0, 0x44ef_66ca)];

    #[test]
    fn the_hash_moves_with_any_number_of_the_shape() {
        let base = Layout::of(&Shape::EMPTY).hash;
        let mut seen = vec![base];
        let edits: [fn(&mut Shape); 11] = [
            |s| s.params = 1,
            |s| s.control_in = 1,
            |s| s.control_out = 1,
            |s| s.vars = 1,
            |s| s.assets = 1,
            |s| s.audio_in[0] = 1,
            |s| s.audio_out[0] = 1,
            |s| s.events_in[0] = 1,
            |s| s.events_out[0] = 1,
            |s| s.sensors[0] = Some(Sensor::Position),
            |s| s.sensors[0] = Some(Sensor::PositionPrecise),
        ];
        for edit in edits {
            let mut s = Shape::EMPTY;
            edit(&mut s);
            let h = Layout::of(&s).hash;
            assert!(!seen.contains(&h), "{s:?}");
            seen.push(h);
        }
    }

    #[test]
    fn shapes_outside_the_limits_are_refused() {
        let mut s = Shape::EMPTY;
        s.audio_in[1] = 2;
        assert!(s.check().unwrap_err().contains("after a port of 0"));
        let mut s = Shape::EMPTY;
        s.audio_out[0] = 33;
        assert!(s.check().is_err());
        let mut s = Shape::EMPTY;
        s.sensors[..2].copy_from_slice(&[Some(Sensor::Motion), Some(Sensor::Motion)]);
        assert!(s.check().unwrap_err().contains("twice"));
        let mut s = Shape::EMPTY;
        s.params = 1025;
        assert!(s.check().is_err());
    }

    #[test]
    fn the_sensor_vocabulary_is_closed() {
        for s in Sensor::ALL {
            assert_eq!(Sensor::from_name(s.name()), Some(s));
        }
        assert_eq!(Sensor::from_name("camera"), None);
    }
}
