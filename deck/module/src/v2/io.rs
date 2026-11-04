//! Typed views on a module's io bytes through its `Layout` – the only place the SDK turns offsets into references.
//! `Inp` reads the block the host wrote; `Out` writes the block the host reads. Both are a pointer and the layout:
//! creating one costs nothing and allocates nothing. `Planar` / `PlanarMut` are an audio port's channels (§1.3).
use super::ev::Ev;
use super::hdr::{AssetInfo, InHdr, OutHdr};
use super::layout::{Layout, MAX_ASSETS, MAX_AUDIO_PORTS, MAX_EVENT_PORTS, MAX_SENSORS, QUANTUM};
use core::marker::PhantomData;
use core::mem::size_of;
use core::slice;

/// what a channel or region the module did not declare reads as
static ZEROS: [f32; QUANTUM] = [0.0; QUANTUM];

/// An audio port's channels for this call, planar: channel `c` is `chan(c)`, `frames()` long.
#[derive(Clone, Copy)]
pub struct Planar<'a> {
    data: &'a [f32],
    ch: usize,
    frames: usize,
}

impl<'a> Planar<'a> {
    /// no channels (an undeclared port)
    pub const EMPTY: Planar<'static> = Planar { data: &[], ch: 0, frames: 0 };

    /// the channels the cable carries now
    pub fn ch(&self) -> usize {
        self.ch
    }

    pub fn frames(&self) -> usize {
        self.frames
    }

    /// channel `c`; a channel the port does not carry reads as silence
    pub fn chan(&self, c: usize) -> &'a [f32] {
        if c < self.ch {
            &self.data[c * QUANTUM..c * QUANTUM + self.frames]
        } else {
            &ZEROS[..self.frames]
        }
    }

    /// the first two channels; a mono port gives its one channel twice
    pub fn stereo(&self) -> (&'a [f32], &'a [f32]) {
        (self.chan(0), if self.ch == 1 { self.chan(0) } else { self.chan(1) })
    }
}

/// An audio output's channels for this call (silence at the start of the call).
pub struct PlanarMut<'a> {
    data: &'a mut [f32],
    ch: usize,
    frames: usize,
}

impl<'a> PlanarMut<'a> {
    pub fn ch(&self) -> usize {
        self.ch
    }

    pub fn frames(&self) -> usize {
        self.frames
    }

    /// channel `c`; a channel the port does not carry now is empty (writes through an iterator go nowhere)
    pub fn chan(&mut self, c: usize) -> &mut [f32] {
        if c < self.ch {
            &mut self.data[c * QUANTUM..c * QUANTUM + self.frames]
        } else {
            &mut []
        }
    }

    /// the first two channels; a mono port gives its channel and an empty second
    pub fn stereo(&mut self) -> (&mut [f32], &mut [f32]) {
        let n = self.frames;
        if self.ch >= 2 {
            let (a, b) = self.data.split_at_mut(QUANTUM);
            (&mut a[..n], &mut b[..n])
        } else if self.ch == 1 {
            (&mut self.data[..n], &mut [])
        } else {
            (&mut [], &mut [])
        }
    }

    /// every channel the port carries now, in order
    pub fn chans(&mut self) -> impl Iterator<Item = &mut [f32]> + '_ {
        let n = self.frames;
        self.data.chunks_exact_mut(QUANTUM).take(self.ch).map(move |c| &mut c[..n])
    }
}

/// The block the host wrote (read-only).
#[derive(Clone, Copy)]
pub struct Inp<'a> {
    base: *const u8,
    layout: &'a Layout,
    _io: PhantomData<&'a [u8]>,
}

impl<'a> Inp<'a> {
    /// # Safety
    /// `base` is 16-aligned and points at `layout.size` bytes laid out by `layout`, not written while `'a` lasts
    pub unsafe fn new(base: *const u8, layout: &'a Layout) -> Inp<'a> {
        Inp { base, layout, _io: PhantomData }
    }

    pub fn layout(&self) -> &'a Layout {
        self.layout
    }

    fn f32s(&self, at: u32, n: usize) -> &'a [f32] {
        // SAFETY: `at` comes from the layout (16-aligned, inside `inp`), `n` from the same shape
        unsafe { slice::from_raw_parts(self.base.add(at as usize) as *const f32, n) }
    }

    pub fn hdr(&self) -> &'a InHdr {
        // SAFETY: the header is at offset 0 and `base` is 16-aligned
        unsafe { &*(self.base as *const InHdr) }
    }

    /// frames in this call, ≤ `QUANTUM`
    pub fn frames(&self) -> usize {
        (self.hdr().frames as usize).min(QUANTUM)
    }

    pub fn params(&self) -> &'a [f32] {
        self.f32s(self.layout.params, self.layout.shape.params as usize)
    }

    pub fn dirty(&self) -> &'a [u32] {
        let n = (self.layout.shape.params as usize).div_ceil(32);
        // SAFETY: as `f32s`
        unsafe { slice::from_raw_parts(self.base.add(self.layout.dirty as usize) as *const u32, n) }
    }

    pub fn control_in(&self) -> &'a [f32] {
        self.f32s(self.layout.control_in, self.layout.shape.control_in as usize)
    }

    /// audio input `p` (an undeclared port: no channels). The channel count is the header's `ch_in[p]`, at most the
    /// port's reserved count; 0 there means the reserved count (a fixed port).
    pub fn audio_in(&self, p: usize) -> Planar<'a> {
        let s = &self.layout.shape;
        if p >= s.audio_in_ports() {
            return Planar { data: &[], ch: 0, frames: self.frames() };
        }
        let max = s.audio_in[p] as usize;
        let ch = match self.hdr().ch_in[p] as usize {
            0 => max,
            n => n.min(max),
        };
        Planar { data: self.f32s(self.layout.audio_in[p], max * QUANTUM), ch, frames: self.frames() }
    }

    /// variable slot `i` (NaN = not set)
    pub fn vars(&self) -> &'a [f32] {
        self.f32s(self.layout.vars, self.layout.shape.vars as usize)
    }

    /// a declared sensor's status word and channels
    pub fn sensor(&self, slot: usize) -> Option<(u32, &'a [f32])> {
        let s = &self.layout.shape;
        let sensor = (slot < MAX_SENSORS).then(|| s.sensors[slot]).flatten()?;
        let at = self.layout.sensors[slot];
        // SAFETY: as `f32s`; the status word is the region's first 4 bytes
        let status = unsafe { *(self.base.add(at as usize) as *const u32) };
        Some((status, self.f32s(at + 4, sensor.channels() as usize)))
    }

    pub fn asset(&self, slot: usize) -> Option<&'a AssetInfo> {
        if slot >= (self.layout.shape.assets as usize).min(MAX_ASSETS) {
            return None;
        }
        // SAFETY: inside the asset region, AssetInfo is 4-aligned and the region 16-aligned
        Some(unsafe { &*(self.base.add(self.layout.assets as usize + slot * size_of::<AssetInfo>()) as *const AssetInfo) })
    }

    /// event input `p`'s records this call, sorted by frame (an undeclared port: none)
    pub fn events_in(&self, p: usize) -> &'a [Ev] {
        let s = &self.layout.shape;
        if p >= s.event_in_ports() {
            return &[];
        }
        let n = (self.hdr().ev_in_len[p] as usize).min(s.events_in[p] as usize);
        // SAFETY: as `f32s`; Ev is 4-aligned
        unsafe { slice::from_raw_parts(self.base.add(self.layout.events_in[p] as usize) as *const Ev, n) }
    }
}

/// The block the host writes, from the host's side: `testing::Rig` and the module host (#341) stage `inp` through it
/// (one implementation of where the host's numbers go), then copy it into the module in one piece.
pub struct InpMut<'a> {
    base: *mut u8,
    layout: &'a Layout,
    _io: PhantomData<&'a mut [u8]>,
}

impl<'a> InpMut<'a> {
    /// # Safety
    /// `base` is 16-aligned and points at `layout.size` bytes laid out by `layout`; nothing else touches them while
    /// `'a` lasts
    pub unsafe fn new(base: *mut u8, layout: &'a Layout) -> InpMut<'a> {
        InpMut { base, layout, _io: PhantomData }
    }

    /// the same bytes, read-only
    pub fn as_inp(&self) -> Inp<'_> {
        // SAFETY: the same io, shared for the borrow
        unsafe { Inp::new(self.base, self.layout) }
    }

    fn f32s(&mut self, at: u32, n: usize) -> &mut [f32] {
        // SAFETY: `at` comes from the layout (16-aligned, inside `inp`), `n` from the same shape; `&mut self` is the
        // only access
        unsafe { slice::from_raw_parts_mut(self.base.add(at as usize) as *mut f32, n) }
    }

    pub fn hdr(&mut self) -> &mut InHdr {
        // SAFETY: the header is at offset 0 and `base` is 16-aligned
        unsafe { &mut *(self.base as *mut InHdr) }
    }

    /// the parameter table and its dirty bitmap (what `params::write` / `defaults` / `apply_preset` take)
    pub fn table(&mut self) -> (&mut [f32], &mut [u32]) {
        let (p, d, n) = (self.layout.params, self.layout.dirty, self.layout.shape.params as usize);
        // SAFETY: two disjoint regions of `inp`
        unsafe {
            (
                slice::from_raw_parts_mut(self.base.add(p as usize) as *mut f32, n),
                slice::from_raw_parts_mut(self.base.add(d as usize) as *mut u32, n.div_ceil(32)),
            )
        }
    }

    pub fn control_in(&mut self) -> &mut [f32] {
        let (at, n) = (self.layout.control_in, self.layout.shape.control_in as usize);
        self.f32s(at, n)
    }

    /// audio input `p`'s channel `c` (`QUANTUM` frames); empty for a port or channel the module did not declare
    pub fn audio_in(&mut self, p: usize, c: usize) -> &mut [f32] {
        InpMut { base: self.base, layout: self.layout, _io: PhantomData }.into_audio_in(p, c)
    }

    /// as `audio_in`, for the view's whole life (a host hands the channel out to be filled)
    pub fn into_audio_in(self, p: usize, c: usize) -> &'a mut [f32] {
        let s = &self.layout.shape;
        if p >= s.audio_in_ports() || c >= s.audio_in[p] as usize {
            return &mut [];
        }
        let at = self.layout.audio_in[p] + (c * QUANTUM * 4) as u32;
        // SAFETY: as `f32s`; the view's exclusive borrow ('a) moves into the slice
        unsafe { slice::from_raw_parts_mut(self.base.add(at as usize) as *mut f32, QUANTUM) }
    }

    /// the declared variable slots (NaN = not set)
    pub fn vars(&mut self) -> &mut [f32] {
        let (at, n) = (self.layout.vars, self.layout.shape.vars as usize);
        self.f32s(at, n)
    }

    /// a declared sensor's status word and channels
    pub fn sensor(&mut self, slot: usize) -> Option<(&mut u32, &mut [f32])> {
        let sensor = (slot < MAX_SENSORS).then(|| self.layout.shape.sensors[slot]).flatten()?;
        let at = self.layout.sensors[slot] as usize;
        // SAFETY: the region is the status word, then `channels` f32s
        unsafe {
            let status = &mut *(self.base.add(at) as *mut u32);
            let values = slice::from_raw_parts_mut(self.base.add(at + 4) as *mut f32, sensor.channels() as usize);
            Some((status, values))
        }
    }

    /// asset slot `slot`'s record
    pub fn asset(&mut self, slot: usize) -> Option<&mut AssetInfo> {
        if slot >= (self.layout.shape.assets as usize).min(MAX_ASSETS) {
            return None;
        }
        // SAFETY: inside the asset region (4-aligned records in a 16-aligned region)
        Some(unsafe { &mut *(self.base.add(self.layout.assets as usize + slot * size_of::<AssetInfo>()) as *mut AssetInfo) })
    }

    /// Event input `p`'s records for the next call: copied up to the port's capacity, `port` set, sorted by frame
    /// (stably). Returns how many were taken (the rest is dropped – the caller counts it); 0 for an undeclared port.
    pub fn set_events_in(&mut self, p: usize, events: &[Ev]) -> usize {
        if p >= self.layout.shape.event_in_ports() {
            return 0;
        }
        let cap = self.layout.shape.events_in[p] as usize;
        let n = events.len().min(cap);
        // SAFETY: the port's region holds `cap` records; Ev is 4-aligned
        let region = unsafe { slice::from_raw_parts_mut(self.base.add(self.layout.events_in[p] as usize) as *mut Ev, cap) };
        region[..n].copy_from_slice(&events[..n]);
        for e in region[..n].iter_mut() {
            e.port = p as u8;
        }
        // stable and in place (no scratch buffer on the audio thread); events mostly come sorted, so it is linear
        for i in 1..n {
            let mut j = i;
            while j > 0 && region[j - 1].frame > region[j].frame {
                region.swap(j - 1, j);
                j -= 1;
            }
        }
        self.hdr().ev_in_len[p] = n as u16;
        n
    }

    /// After a call, what the host clears (§9a): the dirty bits, the event inputs, the late count.
    pub fn after_call(&mut self) {
        self.table().1.fill(0);
        let h = self.hdr();
        h.ev_in_len = [0; MAX_EVENT_PORTS];
        h.late = 0;
    }
}

/// The block the module writes and the host reads.
pub struct Out<'a> {
    base: *mut u8,
    layout: &'a Layout,
    _io: PhantomData<&'a mut [u8]>,
}

impl<'a> Out<'a> {
    /// # Safety
    /// `base` is the io's base (16-aligned, `layout.size` bytes laid out by `layout`); nothing else touches the `out`
    /// part while `'a` lasts
    pub unsafe fn new(base: *mut u8, layout: &'a Layout) -> Out<'a> {
        Out { base, layout, _io: PhantomData }
    }

    pub fn layout(&self) -> &'a Layout {
        self.layout
    }

    fn at<T>(&self, at: u32) -> *mut T {
        // SAFETY: offsets come from the layout and stay inside the io
        unsafe { self.base.add(at as usize) as *mut T }
    }

    pub fn hdr(&self) -> &OutHdr {
        // SAFETY: the header starts `out`, 16-aligned
        unsafe { &*self.at::<OutHdr>(self.layout.out) }
    }

    pub fn hdr_mut(&mut self) -> &mut OutHdr {
        // SAFETY: as `hdr`, and `&mut self` is the only access
        unsafe { &mut *self.at::<OutHdr>(self.layout.out) }
    }

    pub fn control_out(&self) -> &[f32] {
        // SAFETY: the region holds `control_out` f32s
        unsafe { slice::from_raw_parts(self.at(self.layout.control_out), self.layout.shape.control_out as usize) }
    }

    pub fn control_out_mut(&mut self) -> &mut [f32] {
        // SAFETY: as `control_out`
        unsafe { slice::from_raw_parts_mut(self.at(self.layout.control_out), self.layout.shape.control_out as usize) }
    }

    pub fn written(&self) -> &[u32] {
        // SAFETY: the bitmap's words
        unsafe { slice::from_raw_parts(self.at(self.layout.written), (self.layout.shape.control_out as usize).div_ceil(32)) }
    }

    pub fn written_mut(&mut self) -> &mut [u32] {
        // SAFETY: as `written`
        unsafe { slice::from_raw_parts_mut(self.at(self.layout.written), (self.layout.shape.control_out as usize).div_ceil(32)) }
    }

    /// the whole reserved region of audio output `p` (`ch_max · QUANTUM`)
    fn audio_region(&self, p: usize) -> (*mut f32, usize) {
        let s = &self.layout.shape;
        if p >= s.audio_out_ports() {
            return (core::ptr::NonNull::dangling().as_ptr(), 0);
        }
        (self.at(self.layout.audio_out[p]), s.audio_out[p] as usize * QUANTUM)
    }

    /// audio output `p` with `ch` channels (≤ reserved) and `frames` frames
    pub fn audio_out(&mut self, p: usize, ch: usize, frames: usize) -> PlanarMut<'_> {
        let (ptr, len) = self.audio_region(p);
        // SAFETY: the port's reserved region; `&mut self` is the only access
        let data = unsafe { slice::from_raw_parts_mut(ptr, len) };
        PlanarMut { ch: ch.min(len / QUANTUM), frames: frames.min(QUANTUM), data }
    }

    /// audio output `p` read back (the host's side, `testing::Rig`); consumes the view, so nothing writes meanwhile
    pub fn into_audio_out(self, p: usize, ch: usize, frames: usize) -> Planar<'a> {
        let (ptr, len) = self.audio_region(p);
        // SAFETY: as `audio_out`, shared
        let data = unsafe { slice::from_raw_parts(ptr as *const f32, len) };
        Planar { ch: ch.min(len / QUANTUM), frames: frames.min(QUANTUM), data }
    }

    fn events_region(&self, p: usize) -> (*mut Ev, usize) {
        let s = &self.layout.shape;
        if p >= s.event_out_ports() || p >= MAX_EVENT_PORTS {
            return (core::ptr::NonNull::dangling().as_ptr(), 0);
        }
        (self.at(self.layout.events_out[p]), s.events_out[p] as usize)
    }

    /// event output `p`'s whole region (its capacity)
    pub fn events_out_mut(&mut self, p: usize) -> &mut [Ev] {
        let (ptr, n) = self.events_region(p);
        // SAFETY: the port's region; `&mut self` is the only access
        unsafe { slice::from_raw_parts_mut(ptr, n) }
    }

    /// event output `p`'s records written this call
    pub fn events_out(&self, p: usize) -> &[Ev] {
        let (ptr, cap) = self.events_region(p);
        let n = if p < MAX_EVENT_PORTS { (self.hdr().ev_out_len[p] as usize).min(cap) } else { 0 };
        // SAFETY: as `events_out_mut`, shared
        unsafe { slice::from_raw_parts(ptr as *const Ev, n) }
    }

    /// audio output `ap` and the records event output `ep` holds so far – disjoint regions, borrowed together (a module
    /// rendering its own notes into cv, `cv::GateCv::render_notes`)
    pub fn audio_and_events(&mut self, ap: usize, ch: usize, frames: usize, ep: usize) -> (PlanarMut<'_>, &[Ev]) {
        let (aptr, alen) = self.audio_region(ap);
        let (eptr, cap) = self.events_region(ep);
        let n = if ep < MAX_EVENT_PORTS { (self.hdr().ev_out_len[ep] as usize).min(cap) } else { 0 };
        // SAFETY: two different regions of `out` (the layout never overlaps them)
        let (data, events) = unsafe { (slice::from_raw_parts_mut(aptr, alen), slice::from_raw_parts(eptr as *const Ev, n)) };
        (PlanarMut { ch: ch.min(alen / QUANTUM), frames: frames.min(QUANTUM), data }, events)
    }

    /// at the start of a call: the header zeroed, no control output written, no event out, the audio outputs silent
    /// for `frames` frames
    pub fn reset(&mut self, frames: usize) {
        *self.hdr_mut() = OutHdr::ZERO;
        self.written_mut().fill(0);
        for p in 0..self.layout.shape.audio_out_ports().min(MAX_AUDIO_PORTS) {
            let (ptr, len) = self.audio_region(p);
            // SAFETY: the port's region
            let data = unsafe { slice::from_raw_parts_mut(ptr, len) };
            for c in data.chunks_exact_mut(QUANTUM) {
                c[..frames.min(QUANTUM)].fill(0.0);
            }
        }
    }
}
