//! `testing::Rig` v2: a module run natively exactly as its wasm exports run it (`rt::Runtime` on an io buffer laid out
//! by `M::LAYOUT`), with the host's side played by the SDK's own host functions (`params::write`, `apply_preset`,
//! `io::InpMut` – the module host stages `inp` through the same code) – so a module's tests, and the proof that it
//! equals a built-in, are plain `#[test]`s. The build service runs the same tests as wasm under wasmtime (#344).
//!
//! Strict by default, as the build service's tests: a call that allocated panics after it returns – counted only
//! when the test binary installs `deck_module::v2::alloc::Counting` as its `#[global_allocator]`.
use super::ev::Ev;
use super::hdr::{AssetKind, InHdr, OutHdr};
use super::io::{Inp, InpMut, Out, Planar};
use super::layout::{Layout, Sensor, MAX_AUDIO_PORTS, QUANTUM};
use super::params::{self, Dirty};
use super::rt::Runtime;
use super::{AssetInfo, AssetSlot, Module, Var};

/// A module, its io and its host's view of both.
pub struct Rig<M: Module> {
    io: Box<[u128]>,
    layout: Layout,
    rt: Runtime<M>,
}

impl<M: Module> Rig<M> {
    /// 120 bpm, 4 beats a bar, the tempo running from frame 0 = beat 0; every parameter at its default; no character,
    /// no variable set; strict; then `dm_new`
    pub fn new(rate: f64, seed: u64) -> Rig<M> {
        let layout = M::LAYOUT;
        let mut rig = Rig { io: vec![0u128; (layout.size as usize).div_ceil(16)].into_boxed_slice(), layout, rt: Runtime::new() };
        {
            let h = rig.hdr_mut();
            (h.rate, h.seed, h.bpm, h.per_bar, h.running, h.strict) = (rate, seed, 120.0, 4, 1, 1);
        }
        let mut inp = rig.inp_mut();
        inp.vars().fill(f32::NAN);
        let (table, dirty) = inp.table();
        params::defaults(table, dirty, M::PARAMS);
        // SAFETY: the io is ours, laid out by M::LAYOUT
        let base = rig.base();
        unsafe { rig.rt.create(base) };
        rig.inp_mut().table().1.fill(0);
        rig
    }

    fn base(&mut self) -> *mut u8 {
        self.io.as_mut_ptr() as *mut u8
    }

    /// the host's view of `inp`
    fn inp_mut(&mut self) -> InpMut<'_> {
        // SAFETY: the io laid out by `layout`, borrowed mutably
        unsafe { InpMut::new(self.io.as_mut_ptr() as *mut u8, &self.layout) }
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    /// the module (its fields: what the derive copied from the table)
    pub fn module(&self) -> &M {
        self.rt.module().expect("built in new")
    }

    pub fn module_mut(&mut self) -> &mut M {
        self.rt.module_mut().expect("built in new")
    }

    /// the input header, as the host writes it
    pub fn hdr_mut(&mut self) -> &mut InHdr {
        // SAFETY: at offset 0, aligned
        unsafe { &mut *(self.io.as_mut_ptr() as *mut InHdr) }
    }

    pub fn inp(&self) -> Inp<'_> {
        // SAFETY: the io laid out by `layout`, not written while borrowed
        unsafe { Inp::new(self.io.as_ptr() as *const u8, &self.layout) }
    }

    fn out(&self) -> Out<'_> {
        // SAFETY: as `inp`; only read through `&self`
        unsafe { Out::new(self.io.as_ptr() as *mut u8, &self.layout) }
    }

    // --- the host's writes

    pub fn set_tempo(&mut self, anchor_frame: f64, anchor_beat: f64, bpm: f64, per_bar: u32, running: bool) {
        let h = self.hdr_mut();
        (h.tempo_frame, h.tempo_beat, h.bpm, h.per_bar, h.running) = (anchor_frame, anchor_beat, bpm, per_bar, u32::from(running));
    }

    pub fn set_show_ms(&mut self, ms: Option<f64>) {
        let h = self.hdr_mut();
        (h.show_ms, h.show_valid) = (ms.unwrap_or(0.0), u32::from(ms.is_some()));
    }

    pub fn set_character(&mut self, id: &str, persona: &str, mood: &str, mood_index: u32) {
        self.inp_mut().set_character(id, persona, mood);
        self.hdr_mut().mood_index = mood_index;
    }

    pub fn set_senses(&mut self, energy: f32, touch: f32, strain: f32) {
        let h = self.hdr_mut();
        (h.energy, h.touch, h.strain) = (energy, touch, strain);
    }

    /// a declared variable (`None` = not set)
    pub fn set_var(&mut self, v: Var, value: Option<f32>) {
        if let Some(slot) = self.inp_mut().vars().get_mut(v.0 as usize) {
            *slot = value.unwrap_or(f32::NAN);
        }
    }

    /// strict (the default: the build service, Play) or counting only (a show)
    pub fn set_strict(&mut self, strict: bool) {
        self.hdr_mut().strict = u32::from(strict);
    }

    /// parameter `index` as the host writes it: clamped, NaN ignored, dirty when it changed
    pub fn set_param(&mut self, index: usize, value: f32) {
        let mut inp = self.inp_mut();
        let (t, d) = inp.table();
        params::write(t, d, M::PARAMS, index, value);
    }

    /// the factory preset (`#[preset]`): every parameter to its default, then its values
    pub fn apply_preset(&mut self) {
        let mut inp = self.inp_mut();
        let (t, d) = inp.table();
        params::apply_preset(t, d, M::PARAMS, M::PRESET);
    }

    /// the parameter table as the host holds it
    pub fn params(&self) -> &[f32] {
        self.inp().params()
    }

    pub fn set_control_in(&mut self, i: usize, value: f32) {
        if let Some(slot) = self.inp_mut().control_in().get_mut(i) {
            *slot = value;
        }
    }

    /// audio input `p`, channel `c` = `samples` (≤ `QUANTUM`); the rest of the channel stays as it was
    pub fn set_audio_in(&mut self, p: usize, c: usize, samples: &[f32]) {
        let mut inp = self.inp_mut();
        let chan = inp.audio_in(p, c);
        assert!(!chan.is_empty(), "no audio input {p} channel {c}");
        chan[..samples.len()].copy_from_slice(samples);
    }

    /// the channels the cables carry into input `p` / out of output `p` (0 = the declared count)
    pub fn set_channels(&mut self, input: bool, p: usize, ch: u8) {
        let h = self.hdr_mut();
        if p < MAX_AUDIO_PORTS {
            if input {
                h.ch_in[p] = ch;
            } else {
                h.ch_out[p] = ch;
            }
        }
    }

    pub fn set_in_silent(&mut self, p: usize, silent: bool) {
        let h = self.hdr_mut();
        h.in_silent = if silent { h.in_silent | (1 << p) } else { h.in_silent & !(1 << p) };
    }

    /// event input `p`'s records for the next call (sorted by frame here, stably; past the capacity: dropped and false)
    pub fn set_events_in(&mut self, p: usize, events: &[Ev]) -> bool {
        assert!(p < self.layout.shape.event_in_ports(), "no event input {p}");
        self.inp_mut().set_events_in(p, events) == events.len()
    }

    /// a declared sensor's status (`Status as u32`) and channels
    pub fn set_sensor(&mut self, s: Sensor, status: super::Status, values: &[f32]) {
        let slot = self.layout.sensor_slot(s).unwrap_or_else(|| panic!("the module does not declare {}", s.name()));
        self.inp_mut().set_sensor(slot, status as u32, values);
    }

    /// Fill asset `slot` as the host does at instantiation: `dm_asset_buf`, the bytes, the slot's record (ready).
    /// PCM: `channels` planar channels of f32 at `rate`.
    pub fn load_asset(&mut self, slot: AssetSlot, kind: AssetKind, bytes: &[u8], channels: u8, frames: u32, rate: f32) {
        let ptr = self.rt.asset_buf(slot.0 as usize, bytes.len());
        assert!(!ptr.is_null(), "asset slot {} refused", slot.0);
        // SAFETY: the buffer holds bytes.len() bytes
        unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len()) };
        let info = AssetInfo::filled(ptr as usize as u32, kind, bytes.len() as u32, channels, frames, rate);
        *self.inp_mut().asset(slot.0 as usize).expect("a declared asset slot") = info;
    }

    // --- calls

    /// one call (`process` or `control`, the module's domain) of `frames` frames starting at audio frame `frame`;
    /// afterwards the dirty bits and the event inputs are cleared, as the host does
    pub fn run(&mut self, frame: f64, frames: u32) {
        let h = self.hdr_mut();
        (h.frame, h.frames) = (frame, frames.min(QUANTUM as u32));
        // SAFETY: the io is ours
        let base = self.base();
        unsafe { self.rt.call(base) };
        self.inp_mut().after_call();
        let (allocs, strict) = (self.out_hdr().allocs, self.hdr_mut().strict != 0);
        assert!(!(strict && allocs > 0), "module '{}' allocated {allocs} time(s) inside a call (strict)", M::NAME);
    }

    // --- what the module wrote

    pub fn out_hdr(&self) -> OutHdr {
        *self.out().hdr()
    }

    /// audio output `p` of the last call, with the channels the header says
    pub fn audio_out(&self, p: usize) -> Planar<'_> {
        let s = &self.layout.shape;
        let max = if p < MAX_AUDIO_PORTS { s.audio_out[p] as usize } else { 0 };
        let ch = match self.inp().hdr().ch_out.get(p).copied().unwrap_or(0) as usize {
            0 => max,
            n => n.min(max),
        };
        self.out().into_audio_out(p, ch, self.inp().frames())
    }

    /// control output `i`'s value and whether the last call wrote it
    pub fn control_out(&self, i: usize) -> (f32, bool) {
        let out = self.out();
        let v = out.control_out().get(i).copied().unwrap_or(0.0);
        let w = out.written().get(i / 32).is_some_and(|w| w & (1 << (i % 32)) != 0);
        (v, w)
    }

    /// event output `p`'s records of the last call
    pub fn events_out(&self, p: usize) -> Vec<Ev> {
        self.out().events_out(p).to_vec()
    }

    /// the dirty bits the next call will see
    pub fn dirty(&self) -> Vec<usize> {
        Dirty(self.inp().dirty()).iter().collect()
    }
}
