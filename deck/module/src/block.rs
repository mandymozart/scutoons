//! `Block` (an audio call's I/O) and `Outputs` (what a call may write: control outputs, events) – ABI-V2 §1.3, §1.4.
use crate::ev::Ev;
use crate::io::{Inp, Out, Planar, PlanarMut};
use crate::layout::{MAX_AUDIO_PORTS, MAX_EVENT_PORTS};

/// What a call writes besides audio: control outputs (a value and a *written* bit – unwritten = the consumer holds the
/// last value) and events on the declared event outputs.
pub struct Outputs<'a> {
    out: Out<'a>,
}

impl<'a> Outputs<'a> {
    pub fn new(out: Out<'a>) -> Outputs<'a> {
        Outputs { out }
    }

    /// control output `i` = `value` this call (ignored past the declared outputs). A `path` output acts on its target
    /// one block later (§1.1).
    pub fn set(&mut self, i: usize, value: f32) {
        if let Some(slot) = self.out.control_out_mut().get_mut(i) {
            *slot = value;
            self.out.written_mut()[i / 32] |= 1 << (i % 32);
        }
    }

    /// control output `i`'s value (the last written, held across calls)
    pub fn get(&self, i: usize) -> f32 {
        self.out.control_out().get(i).copied().unwrap_or(0.0)
    }

    /// Emit `ev` on event output `port` (its `port` field is set here). False when the port's capacity is full this
    /// call (counted in `OutHdr.dropped`) or the port is not declared. Emit in frame order; the host sorts stably.
    pub fn emit(&mut self, port: usize, mut ev: Ev) -> bool {
        if port >= MAX_EVENT_PORTS || port >= self.out.layout().shape.event_out_ports() {
            return false;
        }
        let n = self.out.hdr().ev_out_len[port] as usize;
        let region = self.out.events_out_mut(port);
        if n >= region.len() {
            let d = &mut self.out.hdr_mut().dropped[port];
            *d = d.saturating_add(1);
            return false;
        }
        ev.port = port as u8;
        region[n] = ev;
        self.out.hdr_mut().ev_out_len[port] += 1;
        true
    }

    /// the records emitted on `port` so far this call
    pub fn events(&self, port: usize) -> &[Ev] {
        self.out.events_out(port)
    }
}

/// One audio call: `frames()` frames (≤ `QUANTUM`), the declared audio inputs and outputs (planar, outputs start
/// silent), the control and event inputs, and the `Outputs`.
pub struct Block<'a> {
    inp: Inp<'a>,
    outputs: Outputs<'a>,
}

impl<'a> Block<'a> {
    pub fn new(inp: Inp<'a>, out: Out<'a>) -> Block<'a> {
        Block { inp, outputs: Outputs { out } }
    }

    pub fn frames(&self) -> usize {
        self.inp.frames()
    }

    /// audio input `p` (silence when nothing is linked; `in_silent(p)` says so up front)
    pub fn audio_in(&self, p: usize) -> Planar<'a> {
        self.inp.audio_in(p)
    }

    /// true when the host linked nothing to audio input `p` this call (its channels are zero)
    pub fn in_silent(&self, p: usize) -> bool {
        p < MAX_AUDIO_PORTS && self.inp.hdr().in_silent & (1 << p) != 0
    }

    fn ch_out(&self, p: usize) -> usize {
        let s = &self.inp.layout().shape;
        let max = if p < MAX_AUDIO_PORTS { s.audio_out[p] as usize } else { 0 };
        match self.inp.hdr().ch_out.get(p).copied().unwrap_or(0) as usize {
            0 => max,
            n => n.min(max),
        }
    }

    /// audio output `p` with the channels the engine linked (`ch_out[p]`; a fixed port: its declared count)
    pub fn audio_out(&mut self, p: usize) -> PlanarMut<'_> {
        let (ch, n) = (self.ch_out(p), self.frames());
        self.outputs.out.audio_out(p, ch, n)
    }

    /// audio output `ap` together with the events emitted on `ep` so far – what a module needs to render its own notes
    /// into audio (`cv::GateCv::render_notes`)
    pub fn audio_out_with_events(&mut self, ap: usize, ep: usize) -> (PlanarMut<'_>, &[Ev]) {
        let (ch, n) = (self.ch_out(ap), self.frames());
        self.outputs.out.audio_and_events(ap, ch, n, ep)
    }

    /// mark audio output `p` silent this call (the engine may skip its sum); the module leaves it zero
    pub fn set_silent(&mut self, p: usize) {
        if p < MAX_AUDIO_PORTS {
            self.outputs.out.hdr_mut().out_silent |= 1 << p;
        }
    }

    /// control input `i` this call
    pub fn control_in(&self, i: usize) -> f32 {
        self.inp.control_in().get(i).copied().unwrap_or(0.0)
    }

    /// event input `p`'s records this call, sorted by frame
    pub fn events_in(&self, p: usize) -> &'a [Ev] {
        self.inp.events_in(p)
    }

    pub fn outputs(&mut self) -> &mut Outputs<'a> {
        &mut self.outputs
    }

    /// shorthand for `outputs().set`
    pub fn set(&mut self, i: usize, value: f32) {
        self.outputs.set(i, value)
    }

    /// shorthand for `outputs().emit`
    pub fn emit(&mut self, port: usize, ev: Ev) -> bool {
        self.outputs.emit(port, ev)
    }
}
