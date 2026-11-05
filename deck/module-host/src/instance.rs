//! A module instance (#341, research/ABI-V2.md §1.1, §1.5, §6): the module's io is laid out from its descriptor
//! (`deck_module::Layout`, the SDK's one function) and crosses the boundary in **one copy in and one copy out per
//! call** – the host stages `inp` in its own buffer (through the SDK's `InpMut`, the code `testing::Rig` uses), writes it
//! with one `Backend::write`, calls `dm_process` / `dm_control`, and reads `out` back with one `Backend::read`.
//!
//! - **Load / build:** `dm_abi() == 2`, `dm_layout()` equal to the layout the host computes (else refused), the io
//!   inside the memory; assets bound before `dm_new` (`dm_asset_buf`, the bytes, the slot record `ready`), so `init`
//!   sees them; then the memory limit is frozen at its size (`Backend::freeze_memory`): nothing grows while playing.
//! - **Two modes (§6, owner 2026-11-03):** [`Mode::Strict`] (tests, the creator's Play): an allocation in a call
//!   traps (the header's `strict`), an overrun of 0.75 × the quantum traps, and a trap *silences* the instance for good.
//!   [`Mode::Show`]: allocations and overruns are counted and accepted; only a runaway past 4 × the quantum is stopped;
//!   a trap **restarts** the instance – "dont cut the instrument": the outputs fade out over the block, the instance is
//!   rebuilt **off the audio thread** ([`Instance::take_rebuild`] → [`Rebuild::run`] on the platform's loader thread →
//!   [`Instance::offer`]) from the cached compile with the current parameters and context, and swapped in at the next
//!   block with a one-block fade-in. A module that keeps trapping waits longer each time (one block, 0.1 s, 1 s, 10 s).
//! - **`needs`:** a block in which a needed sensor is not live is not run (silent, counted, [`Instance::unavailable`]).
//!
//! Nothing on the call path allocates (a trap's message aside); the restart's snapshot buffer is allocated with the
//! instance. The host writes the header and regions; the module reads them; the host clears the dirty bits and the event
//! inputs after each call.
use crate::{fault_text, Backend, Export, Fault, Loaded, ModuleStats, Preset, MEMORY_LIMIT};
use anyhow::{anyhow, bail, Result};
use deck_catalog::AssetKind as CatalogAssetKind;
use deck_core::tempo::TempoMap;
use deck_module::io::{Inp, Out};
use deck_module::layout::MAX_AUDIO_PORTS;
use deck_module::{params, AssetInfo, AssetKind, Ev, InpMut, Layout as IoLayout, Planar, Sensor, Status, ABI_VERSION, QUANTUM};
use std::sync::Arc;

/// how an instance treats its faults (§6)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// tests and the creator's Play: allocations and overruns trap, a trap silences – a creator must see the bug
    Strict,
    /// a show: allocations and overruns are counted, a trap restarts the instance (never cut the instrument)
    Show,
}

/// An asset bound to a slot at instantiation (§1.5): the decoded bytes (deck-media's), read-only, kept for a restart.
#[derive(Debug, Clone)]
pub struct AssetData {
    pub slot: usize,
    pub kind: AssetKind,
    pub bytes: Arc<[u8]>,
    /// pcm: channels · image: 4
    pub channels: u8,
    /// pcm: frames per channel · image: the width
    pub frames: u32,
    /// pcm: the sample rate
    pub rate: f32,
}

impl AssetData {
    /// planar f32 PCM: `channels[c]` are channel c's frames (all as long as the first)
    pub fn pcm(slot: usize, channels: &[&[f32]], rate: f32) -> AssetData {
        let frames = channels.first().map_or(0, |c| c.len());
        let bytes: Vec<u8> = channels.iter().flat_map(|c| c[..frames].iter().flat_map(|v| v.to_le_bytes())).collect();
        AssetData { slot, kind: AssetKind::Pcm, bytes: bytes.into(), channels: channels.len() as u8, frames: frames as u32, rate }
    }

    /// raw bytes (a MIDI file, a sequence)
    pub fn bytes(slot: usize, bytes: &[u8]) -> AssetData {
        AssetData { slot, kind: AssetKind::Bytes, bytes: bytes.into(), channels: 0, frames: 0, rate: 0.0 }
    }
}

/// how an instance is built
#[derive(Debug, Clone)]
pub struct Setup {
    pub rate: f64,
    /// the node's seed (1 unless the node sets it)
    pub seed: u64,
    pub mode: Mode,
    pub assets: Vec<AssetData>,
    /// whether a call has a wall-clock deadline (`deadline_ms`); false only for [`Setup::unbudgeted`] (tests)
    wall_budget: bool,
}

impl Setup {
    /// strict, no assets
    pub fn new(rate: f64, seed: u64) -> Setup {
        Setup { rate, seed, mode: Mode::Strict, assets: Vec::new(), wall_budget: true }
    }

    /// For tests that are not ABOUT time (#382): no wall-clock deadline worth the name ([`UNBUDGETED_MS`], so a real
    /// runaway still ends) – a loaded machine cannot trap a call. The mode's other rules stand (strict still traps an
    /// allocation, a show still restarts on a trap) and the stats still count against the quantum. A deck never calls this.
    pub fn unbudgeted(mut self) -> Setup {
        self.wall_budget = false;
        self
    }

    pub fn mode(mut self, mode: Mode) -> Setup {
        self.mode = mode;
        self
    }

    pub fn asset(mut self, a: AssetData) -> Setup {
        self.assets.push(a);
        self
    }
}

/// the epoch deadline of an [`Setup::unbudgeted`] instance (ms)
pub const UNBUDGETED_MS: u64 = 60_000;

/// the host's copy of a module's io (`inp` then `out`, `Layout::size` bytes, 16-aligned)
type IoBuf = Box<[u128]>;

fn io_buf(layout: &IoLayout) -> IoBuf {
    vec![0u128; (layout.size as usize).div_ceil(16)].into_boxed_slice()
}

fn bytes(io: &IoBuf) -> &[u8] {
    bytemuck::cast_slice(io)
}

fn bytes_mut(io: &mut IoBuf) -> &mut [u8] {
    bytemuck::cast_slice_mut(io)
}

fn inp_mut<'a>(io: &'a mut IoBuf, layout: &'a IoLayout) -> InpMut<'a> {
    // SAFETY: the buffer is 16-aligned and `layout.size` bytes, laid out by `layout`; borrowed mutably for 'a
    unsafe { InpMut::new(io.as_mut_ptr() as *mut u8, layout) }
}

fn out_view<'a>(io: &'a IoBuf, layout: &'a IoLayout) -> Out<'a> {
    // SAFETY: as `inp_mut`; only read through the shared borrow (the view's writers are not called here)
    unsafe { Out::new(io.as_ptr() as *mut u8, layout) }
}

/// every parameter dirty (the module loads the whole table at its next call)
fn all_dirty(io: &mut IoBuf, layout: &IoLayout, n: usize) {
    params::mark_all(inp_mut(io, layout).table().1, n);
}

/// Instantiates, checks and builds one backend instance from a staged `inp` (the parameters all dirty): `dm_abi`,
/// `dm_io`, `dm_layout`, the io range, the assets (their records written into `io`), one copy of `inp`, `dm_new`,
/// the memory frozen. Returns the backend and where its io lies. Off the audio thread (instantiating allocates).
fn boot<B: Backend>(loaded: &Loaded<B>, io: &mut IoBuf, assets: &[AssetData], wall_budget: bool) -> Result<(B, usize)> {
    let name = &loaded.desc.name;
    let layout = &loaded.io;
    let mut b = B::instantiate(&loaded.compiled, name)?;
    b.set_budget(if wall_budget { 1000 } else { UNBUDGETED_MS });
    let abi = b.call(Export::Abi, &[]).map_err(|f| anyhow!("{name}: dm_abi: {}", fault_text(&f)))?;
    if abi != ABI_VERSION {
        bail!("module '{name}' speaks ABI {abi}, its descriptor says {ABI_VERSION}");
    }
    let at = b.call(Export::Io, &[]).map_err(|f| anyhow!("{name}: dm_io: {}", fault_text(&f)))? as usize;
    let hash = b.call(Export::Layout, &[]).map_err(|f| anyhow!("{name}: dm_layout: {}", fault_text(&f)))?;
    if hash != layout.hash {
        bail!("module '{name}': its layout ({hash:08x}) is not the one its descriptor gives ({:08x}) – built against another SDK?", layout.hash);
    }
    if at % 16 != 0 || at.checked_add(layout.size as usize).map_or(true, |end| end > b.memory_size()) {
        bail!("module '{name}': its io does not lie in its memory");
    }
    // the assets: the module's own buffer per slot, the bytes, the record – before `dm_new`, so `init` sees them
    for a in assets {
        let ptr = b
            .call(Export::AssetBuf, &[crate::Arg::U32(a.slot as u32), crate::Arg::U32(a.bytes.len() as u32)])
            .map_err(|f| anyhow!("{name}: dm_asset_buf: {}", fault_text(&f)))? as usize;
        if ptr == 0 || ptr.checked_add(a.bytes.len()).map_or(true, |end| end > b.memory_size()) {
            bail!("module '{name}': asset slot {} – the module gave no buffer for it", a.slot);
        }
        b.write(ptr, &a.bytes);
        let info = AssetInfo::filled(ptr as u32, a.kind, a.bytes.len() as u32, a.channels, a.frames, a.rate);
        if let Some(slot) = inp_mut(io, layout).asset(a.slot) {
            *slot = info;
        }
    }
    b.write(at, &bytes(io)[..layout.inp_size as usize]);
    // building the module runs its `init`: generous, but bounded
    b.set_budget(if wall_budget { 50 } else { UNBUDGETED_MS });
    b.call(Export::New, &[]).map_err(|f| anyhow!("{name}: dm_new: {}", fault_text(&f)))?;
    b.freeze_memory();
    Ok((b, at))
}

/// what fades the next output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fade {
    None,
    /// the first block after a restart: 0 → 1
    In,
}

/// a restart under way (a show)
struct Down<B> {
    why: String,
    /// frames since the fault
    waited: u64,
    /// frames to wait before the rebuild may start
    pause: u64,
    /// the trapped instance, dropped by the rebuild (off the audio thread)
    dead: Option<B>,
    /// the rebuild is out on the loader thread
    building: bool,
    /// a rebuilt instance and where its io lies, swapped in at the next call
    fresh: Option<(B, usize)>,
}

/// A rebuild of a trapped instance, to run on the platform's loader thread (never the audio thread): it drops the
/// dead instance, builds a fresh one from the cached compile with the snapshot of the parameters and context.
pub struct Rebuild<B: Backend> {
    loaded: Arc<Loaded<B>>,
    snapshot: IoBuf,
    assets: Arc<[AssetData]>,
    dead: Option<B>,
    wall_budget: bool,
}

/// what a rebuild gives back to [`Instance::offer`]
pub struct Fresh<B: Backend> {
    result: Result<(B, usize), String>,
    snapshot: IoBuf,
}

impl<B: Backend> Rebuild<B> {
    pub fn run(self) -> Fresh<B> {
        let Rebuild { loaded, mut snapshot, assets, dead, wall_budget } = self;
        drop(dead);
        let result = boot(&loaded, &mut snapshot, &assets, wall_budget).map_err(|e| format!("{e:#}"));
        Fresh { result, snapshot }
    }
}

/// a module that keeps trapping waits longer each time: one block, 0.1 s, 1 s, then 10 s
fn pause_frames(streak: u32, rate: f64) -> u64 {
    match streak {
        0 | 1 => QUANTUM as u64,
        2 => (0.1 * rate) as u64,
        3 => rate as u64,
        _ => (10.0 * rate) as u64,
    }
}

/// a trap streak ends after this long without one
const CLEAN_SECONDS: f64 = 30.0;

/// One running module: what the engine drives through its `ModuleInstance` seam (deck-engine), and everything else
/// of it (events, control outputs, sensors, assets, presets, stats, a show's restart).
pub struct Instance<B: Backend> {
    loaded: Arc<Loaded<B>>,
    /// `None` while a restart is under way
    backend: Option<B>,
    /// where the io lies in the module's memory (`dm_io`)
    at: usize,
    layout: IoLayout,
    /// the host's copy of the io: `inp` is staged here, `out` read back here
    io: IoBuf,
    /// a second buffer of the same size for a restart's snapshot (allocated once, here)
    spare: Option<IoBuf>,
    rate: f64,
    mode: Mode,
    wall_budget: bool,
    assets: Arc<[AssetData]>,
    silenced: Option<String>,
    unavailable: Option<Sensor>,
    down: Option<Down<B>>,
    streak: u32,
    clean: u64,
    fade: Fade,
    stats: ModuleStats,
}

impl<B: Backend> Instance<B> {
    /// Instantiates `loaded` with every parameter at its default, 120 bpm from frame 0, no character,
    /// no variable set, the setup's assets bound; then `dm_new` and the memory frozen.
    pub fn new(loaded: &Arc<Loaded<B>>, setup: Setup) -> Result<Instance<B>> {
        let name = &loaded.desc.name;
        let layout = loaded.io;
        for a in &setup.assets {
            let Some(d) = loaded.desc.assets.get(a.slot) else { bail!("module '{name}' has no asset slot {}", a.slot) };
            let kind = match d.kind {
                CatalogAssetKind::Pcm => AssetKind::Pcm,
                CatalogAssetKind::Bytes => AssetKind::Bytes,
                CatalogAssetKind::Image => AssetKind::Image,
            };
            if a.kind != kind {
                bail!("module '{name}': asset '{}' takes {:?}, given {:?}", d.name, d.kind, a.kind);
            }
            if a.bytes.len() > d.max_bytes as usize {
                bail!("module '{name}': asset '{}' is {} bytes, the slot takes at most {}", d.name, a.bytes.len(), d.max_bytes);
            }
        }
        let mut io = io_buf(&layout);
        {
            let mut inp = inp_mut(&mut io, &layout);
            let h = inp.hdr();
            let m = TempoMap::new(120.0, 4, setup.rate);
            (h.rate, h.seed, h.strict) = (setup.rate, setup.seed, u32::from(setup.mode == Mode::Strict));
            (h.tempo_frame, h.tempo_beat, h.bpm, h.per_bar, h.running) = (m.frame, m.beat, m.bpm, m.per_bar, u32::from(m.running));
            inp.vars().fill(f32::NAN);
            let (t, d) = inp.table();
            params::defaults(t, d, &loaded.params);
        }
        let (backend, at) = boot(loaded, &mut io, &setup.assets, setup.wall_budget)?;
        inp_mut(&mut io, &layout).after_call();
        let mut me = Instance {
            loaded: loaded.clone(),
            backend: Some(backend),
            at,
            layout,
            spare: Some(io_buf(&layout)),
            io,
            rate: setup.rate,
            mode: setup.mode,
            wall_budget: setup.wall_budget,
            assets: setup.assets.into(),
            silenced: None,
            unavailable: None,
            down: None,
            streak: 0,
            clean: 0,
            fade: Fade::None,
            stats: ModuleStats::default(),
        };
        let size = me.backend.as_ref().map_or(0, |b| b.memory_size());
        me.stats.memory(size);
        Ok(me)
    }

    fn inp(&mut self) -> InpMut<'_> {
        inp_mut(&mut self.io, &self.layout)
    }

    fn out(&self) -> Out<'_> {
        out_view(&self.io, &self.layout)
    }

    pub fn loaded(&self) -> &Arc<Loaded<B>> {
        &self.loaded
    }

    pub fn io_layout(&self) -> &IoLayout {
        &self.layout
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// the backend (tests, meters); `None` while a restart is under way
    pub fn backend(&self) -> Option<&B> {
        self.backend.as_ref()
    }

    /// why this instance is silent for good (strict mode: a trap or an overrun)
    pub fn silenced(&self) -> Option<&str> {
        self.silenced.as_deref()
    }

    /// why a restart is under way (a show), if one is
    pub fn restarting(&self) -> Option<&str> {
        self.down.as_ref().map(|d| d.why.as_str())
    }

    /// the next call swaps the rebuilt instance in (its inputs are the fresh instance's first)
    pub fn swapping(&self) -> bool {
        self.down.as_ref().is_some_and(|d| d.fresh.is_some())
    }

    /// the needed sensor that kept the last block from running, if one did
    pub fn unavailable(&self) -> Option<&'static str> {
        self.unavailable.map(Sensor::name)
    }

    pub fn stats(&self) -> &ModuleStats {
        &self.stats
    }

    // --- the context (written once into `inp`, crossing with every call)

    pub fn set_tempo(&mut self, m: &TempoMap) {
        let mut inp = self.inp();
        let h = inp.hdr();
        (h.tempo_frame, h.tempo_beat, h.bpm, h.per_bar, h.running) = (m.frame, m.beat, m.bpm, m.per_bar, u32::from(m.running));
    }

    /// the show time at the next call's first frame (ms), `None` without a show clock
    pub fn set_show_ms(&mut self, ms: Option<f64>) {
        let mut inp = self.inp();
        let h = inp.hdr();
        (h.show_ms, h.show_valid) = (ms.unwrap_or(0.0), u32::from(ms.is_some()));
    }

    /// the character the module plays for (empty = none)
    pub fn set_character(&mut self, id: &str, persona: &str, mood: &str) {
        self.inp().set_character(id, persona, mood);
    }

    /// the mood's place in the module's own `moods` list + 1, 0 = not in it (A1, #409: deck-space `context::mood_index`)
    pub fn set_mood_index(&mut self, i: u32) {
        self.inp().hdr().mood_index = i;
    }

    /// the being's senses, 0 … 1
    pub fn set_senses(&mut self, energy: f32, touch: f32, strain: f32) {
        let mut inp = self.inp();
        let h = inp.hdr();
        (h.energy, h.touch, h.strain) = (energy, touch, strain);
    }

    /// The variables, key → number: each slot the module declared (`reads`) takes its key's value, NaN (not set) when
    /// the list lacks it. No allocation (a scan per declared key).
    pub fn set_vars(&mut self, vars: &[(String, f32)]) {
        let loaded = self.loaded.clone();
        let mut inp = self.inp();
        for (slot, key) in inp.vars().iter_mut().zip(loaded.desc.reads.iter()) {
            *slot = vars.iter().find(|(k, _)| k == key).map_or(f32::NAN, |(_, v)| *v);
        }
    }

    /// declared variable slot `i` (`Loaded::var_slot`); `None` = not set
    pub fn set_var(&mut self, i: usize, value: Option<f32>) {
        if let Some(v) = self.inp().vars().get_mut(i) {
            *v = value.unwrap_or(f32::NAN);
        }
    }

    /// a declared sensor's status and channels for the next call; false when the module does not declare it
    pub fn set_sensor(&mut self, sensor: Sensor, status: Status, values: &[f32]) -> bool {
        let Some(slot) = self.layout.sensor_slot(sensor) else { return false };
        self.inp().set_sensor(slot, status as u32, values)
    }

    // --- parameters (§2): the host clamps, the module smooths

    /// parameter `index` for the next call: clamped to its range, NaN keeps the last good value, dirty when it changed
    pub fn set_param(&mut self, index: usize, value: f32) {
        let loaded = self.loaded.clone();
        let mut inp = self.inp();
        let (t, d) = inp.table();
        params::write(t, d, &loaded.params, index, value);
    }

    /// parameter `index` as the table holds it
    pub fn param(&mut self, index: usize) -> Option<f32> {
        self.inp().table().0.get(index).copied()
    }

    /// `Preset::apply`: every parameter to its default, then the preset's values (no allocation)
    pub fn apply_preset(&mut self, preset: &Preset) {
        let loaded = self.loaded.clone();
        let mut inp = self.inp();
        let (t, d) = inp.table();
        params::apply_preset(t, d, &loaded.params, preset.values());
    }

    /// the descriptor's factory preset (false: the module has none)
    pub fn apply_factory_preset(&mut self) -> bool {
        let loaded = self.loaded.clone();
        match loaded.factory.as_ref() {
            Some(p) => {
                self.apply_preset(p);
                true
            }
            None => false,
        }
    }

    // --- inputs

    pub fn set_control_in(&mut self, i: usize, value: f32) {
        if let Some(v) = self.inp().control_in().get_mut(i) {
            *v = value;
        }
    }

    /// audio input `p`'s channel `c` for the next call, `QUANTUM` frames to write (empty: no such channel)
    pub fn audio_in_mut(&mut self, p: usize, c: usize) -> &mut [f32] {
        let layout = &self.layout;
        inp_mut(&mut self.io, layout).into_audio_in(p, c)
    }

    /// the channels the cables carry into input `p` / out of output `p` (0 = the declared count)
    pub fn set_channels(&mut self, input: bool, p: usize, ch: u8) {
        let mut inp = self.inp();
        let h = inp.hdr();
        let list = if input { &mut h.ch_in } else { &mut h.ch_out };
        if let Some(c) = list.get_mut(p) {
            *c = ch;
        }
    }

    /// nothing is linked to audio input `p` (the module may early-out; the host zero-fills it)
    pub fn set_in_silent(&mut self, p: usize, silent: bool) {
        if p >= MAX_AUDIO_PORTS {
            return;
        }
        if silent {
            let reserved = self.layout.shape.audio_in.get(p).copied().unwrap_or(0) as usize;
            for c in 0..reserved {
                self.audio_in_mut(p, c).fill(0.0);
            }
        }
        let mut inp = self.inp();
        let h = inp.hdr();
        h.in_silent = if silent { h.in_silent | (1 << p) } else { h.in_silent & !(1 << p) };
    }

    /// A stereo pair into audio input `p` (tests): channel 0 = `l`, 1 = `r` (a mono port takes `l`); the port carries
    /// two channels (or its one).
    pub fn set_audio_in(&mut self, p: usize, l: &[f32], r: &[f32]) {
        let reserved = self.layout.shape.audio_in.get(p).copied().unwrap_or(0);
        if reserved == 0 {
            return;
        }
        for (c, src) in [l, r].into_iter().enumerate().take(reserved as usize) {
            let dst = self.audio_in_mut(p, c);
            let n = src.len().min(dst.len());
            dst[..n].copy_from_slice(&src[..n]);
        }
        self.set_channels(true, p, reserved.min(2));
        self.set_in_silent(p, false);
    }

    /// Event input `p`'s records for the next call, sorted by frame (stably); past the capacity they are dropped and
    /// counted. Returns how many were taken.
    pub fn set_events_in(&mut self, p: usize, events: &[Ev]) -> usize {
        let n = self.inp().set_events_in(p, events);
        self.stats.events_in += n as u64;
        self.stats.dropped_in += (events.len() - n) as u64;
        n
    }

    /// `n` of the next call's input events came late (their time had passed; the event host clamped them to frame 0)
    pub fn add_late(&mut self, n: u32) {
        let mut inp = self.inp();
        let h = inp.hdr();
        h.late = h.late.saturating_add(n);
        self.stats.late += u64::from(n);
    }

    // --- what the last call wrote (the host's copy of `out`)

    /// audio output `p` of the last call (silence while the instance does not run)
    pub fn audio_out(&self, p: usize) -> Planar<'_> {
        let s = &self.layout.shape;
        let max = s.audio_out.get(p).copied().unwrap_or(0) as usize;
        // SAFETY: our own io, laid out by `layout`, read through the shared borrow
        let inp = unsafe { Inp::new(self.io.as_ptr() as *const u8, &self.layout) };
        let (ch_out, frames) = (inp.hdr().ch_out, inp.frames());
        let ch = match ch_out.get(p).copied().unwrap_or(0) as usize {
            0 => max,
            n => n.min(max),
        };
        self.out().into_audio_out(p, ch, frames.min(QUANTUM))
    }

    /// audio output `p`'s first two channels into `l` / `r` (a mono port into both; tests)
    pub fn read_audio_out(&mut self, p: usize, l: &mut [f32], r: &mut [f32]) {
        let out = self.audio_out(p);
        let (a, b) = out.stereo();
        for (dst, src) in [(l, a), (r, b)] {
            let n = dst.len().min(src.len());
            dst[..n].copy_from_slice(&src[..n]);
            dst[n..].fill(0.0);
        }
    }

    /// control output `i`: its value and whether the last call wrote it (unwritten = the consumer holds)
    pub fn control_out(&self, i: usize) -> (f32, bool) {
        let out = self.out();
        let v = out.control_out().get(i).copied().unwrap_or(0.0);
        let w = out.written().get(i / 32).is_some_and(|w| w & (1 << (i % 32)) != 0);
        (v, w)
    }

    /// event output `p`'s records of the last call
    pub fn events_out(&self, p: usize) -> &[Ev] {
        // the view borrows `self.io`; its slice lives as long
        let out = self.out();
        let s = out.events_out(p);
        // SAFETY: `s` points into `self.io`, which outlives the returned borrow of `self`
        unsafe { core::slice::from_raw_parts(s.as_ptr(), s.len()) }
    }

    /// the `OutHdr` of the last call (allocations, silent flags, event counts)
    pub fn out_hdr(&self) -> deck_module::OutHdr {
        *self.out().hdr()
    }

    // --- the call

    fn budget_us(&self) -> u32 {
        (0.75 * QUANTUM as f64 / self.rate * 1e6) as u32
    }

    /// the epoch deadline: the budget when strict, the runaway ceiling (4 × the quantum) in a show
    fn deadline_ms(&self) -> u64 {
        if !self.wall_budget {
            return UNBUDGETED_MS;
        }
        let quantum_ms = QUANTUM as f64 / self.rate * 1000.0;
        match self.mode {
            Mode::Strict => ((0.75 * quantum_ms).ceil() as u64).clamp(2, 50),
            Mode::Show => ((4.0 * quantum_ms).ceil() as u64).clamp(2, 200),
        }
    }

    /// One call of `frames` (≤ `QUANTUM`) frames starting at audio frame `frame`: `dm_process` or `dm_control`, the
    /// module's domain. False when the module did not run (silenced, restarting, a needed sensor missing, a fault).
    pub fn run(&mut self, frame: f64, frames: u32) -> bool {
        if self.silenced.is_some() {
            return false;
        }
        let frames = frames.min(QUANTUM as u32);
        {
            let mut inp = self.inp();
            let h = inp.hdr();
            (h.frame, h.frames) = (frame, frames);
        }
        if let Some(down) = &mut self.down {
            match down.fresh.take() {
                Some((fresh, at)) => {
                    // the swap: the fresh instance takes the current table (all dirty) and context with this call
                    self.backend = Some(fresh);
                    self.at = at;
                    self.down = None;
                    self.fade = Fade::In;
                    self.stats.restarts += 1;
                    all_dirty(&mut self.io, &self.layout, self.layout.shape.params as usize);
                }
                None => {
                    down.waited += u64::from(frames);
                    self.quiet(frames, false);
                    return false;
                }
            }
        }
        if let Some(s) = self.missing_need() {
            self.unavailable = Some(s);
            self.stats.unavailable += 1;
            self.quiet(frames, false);
            return false;
        }
        self.unavailable = None;
        let export = match self.loaded.desc.domain {
            deck_catalog::Domain::Control => Export::Control,
            _ => Export::Process,
        };
        let (deadline, budget_us) = (self.deadline_ms(), self.budget_us());
        let Some(b) = self.backend.as_mut() else { return false };
        let inp_size = self.layout.inp_size as usize;
        b.write(self.at, &bytes(&self.io)[..inp_size]);
        b.set_budget(deadline);
        let t = b.clock_us();
        let r = b.call(export, &[]);
        let us = b.clock_us().saturating_sub(t).min(u64::from(u32::MAX)) as u32;
        match r {
            Ok(_) => {
                let (out, size) = (self.layout.out as usize, self.layout.size as usize);
                b.read(self.at + out, &mut bytes_mut(&mut self.io)[out..size]);
                let mem = b.memory_size();
                self.inp().after_call();
                self.stats.record(us, budget_us);
                self.stats.memory(mem);
                let h = self.out_hdr();
                self.stats.allocs += u64::from(h.allocs);
                self.stats.events_out += h.ev_out_len.iter().map(|&n| u64::from(n)).sum::<u64>();
                self.stats.dropped_out += h.dropped.iter().map(|&n| u64::from(n)).sum::<u64>();
                self.stats.written += self.out().written().iter().map(|w| u64::from(w.count_ones())).sum::<u64>();
                if self.fade == Fade::In {
                    self.fade = Fade::None;
                    self.ramp(frames, true);
                }
                self.clean += u64::from(frames);
                if self.clean as f64 >= CLEAN_SECONDS * self.rate {
                    self.streak = 0;
                }
                true
            }
            Err(f) => {
                self.fault(f, export, frames);
                false
            }
        }
    }

    /// `run` under the engine's name (`ModuleInstance::process`: the module's one call, either domain)
    pub fn process(&mut self, frame: f64, frames: u32) -> bool {
        self.run(frame, frames)
    }

    // --- the engine's seam (#342): the planar cable as channel slices

    /// the module's descriptor
    pub fn descriptor(&self) -> &deck_catalog::Descriptor {
        &self.loaded.desc
    }

    /// how its ports map onto the graph
    pub fn graph(&self) -> &crate::Layout {
        &self.loaded.layout
    }

    /// Audio input `port` for the next call as channel slices (the engine's planar cable, any width; empty = nothing
    /// linked): the port takes `chans.len()` channels (at most what it reserves; the rest of its region is zero) and
    /// says so in its header.
    pub fn set_audio_chans(&mut self, port: usize, chans: &[&[f32]]) {
        let reserved = self.layout.shape.audio_in.get(port).copied().unwrap_or(0) as usize;
        if reserved == 0 {
            return;
        }
        if chans.is_empty() {
            self.set_in_silent(port, true);
            self.set_channels(true, port, 0);
            return;
        }
        for c in 0..reserved {
            let dst = self.audio_in_mut(port, c);
            match chans.get(c) {
                Some(src) => {
                    let n = src.len().min(dst.len());
                    dst[..n].copy_from_slice(&src[..n]);
                    dst[n..].fill(0.0);
                }
                None => dst.fill(0.0),
            }
        }
        self.set_channels(true, port, chans.len().min(reserved) as u8);
        self.set_in_silent(port, false);
    }

    /// Channel `c` of audio output `port` of the last call into `dst` (its length = the frames asked); a channel the
    /// output lacks is silent.
    pub fn read_audio_chan(&mut self, port: usize, c: usize, dst: &mut [f32]) {
        let src = self.audio_out(port);
        if c < src.ch() {
            let s = src.chan(c);
            let n = dst.len().min(s.len());
            dst[..n].copy_from_slice(&s[..n]);
            dst[n..].fill(0.0);
        } else {
            dst.fill(0.0);
        }
    }

    /// the channels the engine's cable takes out of audio output `port` (an `any` output follows it; 0 = declared)
    pub fn set_out_channels(&mut self, port: usize, ch: usize) {
        self.set_channels(false, port, ch.min(u8::MAX as usize) as u8);
    }

    /// the first `needs` sensor whose status is not live / replay
    fn missing_need(&mut self) -> Option<Sensor> {
        let n = self.loaded.desc.needs.len();
        let layout = self.layout;
        let mut inp = self.inp();
        (0..n).find_map(|slot| {
            let sensor = layout.shape.sensors[slot]?;
            let (word, _) = inp.sensor(slot)?;
            (!Status::from_u32(*word).live()).then_some(sensor)
        })
    }

    fn fault(&mut self, f: Fault, export: Export, frames: u32) {
        self.stats.traps += 1;
        let name = &self.loaded.desc.name;
        match self.mode {
            Mode::Strict => {
                let msg = match f {
                    Fault::Budget => format!("{} ran past its time budget", export.name()),
                    Fault::Trap(why) => format!("{}: {why}", export.name()),
                };
                tracing::error!("module '{name}' silenced – {msg}");
                self.silenced = Some(msg);
                self.quiet(frames, false);
            }
            Mode::Show => {
                let msg = match f {
                    Fault::Budget => format!("{} ran past 4 × the quantum (a runaway)", export.name()),
                    Fault::Trap(why) => format!("{}: {why}", export.name()),
                };
                self.streak += 1;
                self.clean = 0;
                let pause = pause_frames(self.streak, self.rate);
                tracing::warn!("module '{name}' restarts in {:.2} s – {msg}", pause as f64 / self.rate);
                // the last good block fades out over this one; then silence until the swap
                self.quiet(frames, true);
                self.down = Some(Down { why: msg, waited: 0, pause, dead: self.backend.take(), building: false, fresh: None });
            }
        }
    }

    /// The outputs while the module does not run: no events, no control output written (consumers hold), audio
    /// silent – or, with `fade_out`, the last block's audio fading to zero over `frames`.
    fn quiet(&mut self, frames: u32, fade_out: bool) {
        let layout = self.layout;
        // SAFETY: the out header and regions of our own io (laid out by `layout`), written through one view
        let mut out = unsafe { Out::new(self.io.as_mut_ptr() as *mut u8, &layout) };
        out.hdr_mut().ev_out_len = [0; 8];
        out.hdr_mut().dropped = [0; 8];
        out.hdr_mut().allocs = 0;
        out.written_mut().fill(0);
        // silent outputs say so (the engine may skip their sum); a fading one is not silent yet
        out.hdr_mut().out_silent = if fade_out { 0 } else { (1u32 << layout.shape.audio_out_ports()) - 1 };
        if fade_out {
            self.ramp(frames, false);
        } else {
            for p in 0..layout.shape.audio_out_ports() {
                let ch = layout.shape.audio_out[p] as usize;
                let mut o = out.audio_out(p, ch, QUANTUM);
                for c in o.chans() {
                    c.fill(0.0);
                }
            }
        }
    }

    /// every audio output's first `frames` frames times a ramp: up 0 → 1 (`up`) or down 1 → 0
    fn ramp(&mut self, frames: u32, up: bool) {
        let layout = self.layout;
        let n = (frames as usize).clamp(1, QUANTUM);
        // SAFETY: as `quiet`
        let mut out = unsafe { Out::new(self.io.as_mut_ptr() as *mut u8, &layout) };
        for p in 0..layout.shape.audio_out_ports() {
            let ch = layout.shape.audio_out[p] as usize;
            let mut o = out.audio_out(p, ch, n);
            for c in o.chans() {
                for (i, s) in c.iter_mut().enumerate() {
                    let g = (i + 1) as f32 / n as f32;
                    *s *= if up { g } else { 1.0 - g };
                }
            }
        }
    }

    // --- the restart (a show), driven by the platform's loader thread

    /// The rebuild of a trapped instance once its pause is over (a show; `None` otherwise or while one is out). Cheap
    /// and allocation-free (the snapshot buffer was allocated with the instance): run [`Rebuild::run`] on a loader
    /// thread and hand the result to [`Instance::offer`].
    pub fn take_rebuild(&mut self) -> Option<Rebuild<B>> {
        let down = self.down.as_mut()?;
        if down.building || down.fresh.is_some() || down.waited < down.pause {
            return None;
        }
        let mut snapshot = self.spare.take()?;
        snapshot.copy_from_slice(&self.io);
        // the fresh `init` sees no input events
        inp_mut(&mut snapshot, &self.layout).after_call();
        all_dirty(&mut snapshot, &self.layout, self.layout.shape.params as usize);
        down.building = true;
        Some(Rebuild { loaded: self.loaded.clone(), snapshot, assets: self.assets.clone(), dead: down.dead.take(), wall_budget: self.wall_budget })
    }

    /// A rebuild's result: swapped in at the next call with a fade-in; a failed one counts as another fault (the pause
    /// grows) and a new rebuild follows.
    pub fn offer(&mut self, fresh: Fresh<B>) {
        let Fresh { result, snapshot } = fresh;
        if result.is_ok() {
            // the fresh instance's asset records (its own buffers) go into the staged `inp`
            let a = self.layout.assets as usize;
            let len = self.layout.shape.assets as usize * core::mem::size_of::<AssetInfo>();
            bytes_mut(&mut self.io)[a..a + len].copy_from_slice(&bytes(&snapshot)[a..a + len]);
        }
        self.spare = Some(snapshot);
        let Some(down) = self.down.as_mut() else { return };
        down.building = false;
        match result {
            Ok(b) => down.fresh = Some(b),
            Err(e) => {
                self.stats.traps += 1;
                self.streak += 1;
                down.pause = pause_frames(self.streak, self.rate);
                down.waited = 0;
                tracing::warn!("module '{}': the restart failed, again in {:.2} s – {e}", self.loaded.desc.name, down.pause as f64 / self.rate);
                down.why = e;
            }
        }
    }

    /// µs of the last call and the slowest so far
    pub fn cpu_us(&self) -> (u32, u32) {
        (self.stats.us_last, self.stats.us_max)
    }

    /// the module's memory now (bytes; 0 while restarting)
    pub fn memory_size(&self) -> usize {
        self.backend.as_ref().map_or(0, |b| b.memory_size())
    }
}

/// the host refuses a module whose assets cannot fit beside its io in [`MEMORY_LIMIT`]
pub(crate) fn assets_fit(desc: &deck_catalog::Descriptor, io: &IoLayout) -> Result<()> {
    let total: u64 = desc.assets.iter().map(|a| u64::from(a.max_bytes)).sum::<u64>() + u64::from(io.size);
    if total > MEMORY_LIMIT as u64 {
        bail!("module '{}': its asset slots ({} bytes with the io) cannot fit in the {} MiB a module has", desc.name, total, MEMORY_LIMIT >> 20);
    }
    Ok(())
}
