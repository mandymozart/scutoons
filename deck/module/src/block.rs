//! `Block`: one call's audio and control I/O, and the events going out.
use crate::abi::{put_str, EventRec, In, Out, MAX_AUDIO_PORTS, MAX_BLOCK, MAX_CONTROL_OUT, MAX_EVENTS, MAX_PROPS};

/// An event a module emits (deck-events' model): a name (`music.note`), a start and length in beats on the tempo, up
/// to `MAX_PROPS` numeric props. Names are cut at 16 bytes, prop keys at 8.
#[derive(Clone, Copy)]
pub struct Event {
    pub(crate) rec: EventRec,
}

impl Event {
    /// starting at an absolute `beat`
    pub fn at_beat(name: &str, beat: f64) -> Event {
        let mut rec = EventRec { start: beat, len: 0.0, port: 0, nprops: 0, name: [0; 16], keys: [[0; 8]; MAX_PROPS], values: [0.0; MAX_PROPS] };
        put_str(&mut rec.name, name);
        Event { rec }
    }

    /// starting with the call (a trigger)
    pub fn now(name: &str) -> Event {
        Event::at_beat(name, f64::NAN)
    }

    /// lasting `beats` (0 = a moment)
    pub fn lasting(mut self, beats: f64) -> Event {
        self.rec.len = beats;
        self
    }

    /// a prop (ignored when all slots are taken)
    pub fn with(mut self, key: &str, value: f32) -> Event {
        let i = self.rec.nprops as usize;
        if i < MAX_PROPS {
            put_str(&mut self.rec.keys[i], key);
            self.rec.values[i] = value;
            self.rec.nprops += 1;
        }
        self
    }

    /// on the n-th declared event output (default 0)
    pub fn on(mut self, port: usize) -> Event {
        self.rec.port = port as u32;
        self
    }
}

/// What `control` can write: the control outputs and the events.
pub struct Outputs<'a> {
    out: &'a mut Out,
}

impl<'a> Outputs<'a> {
    pub fn new(out: &'a mut Out) -> Outputs<'a> {
        Outputs { out }
    }

    /// the n-th declared control output (ignored past the ports the ABI has)
    pub fn set_control(&mut self, port: usize, value: f32) {
        if port < MAX_CONTROL_OUT {
            self.out.control[port] = value;
        }
    }

    /// emit an event; false when the call's buffer is full (counted – the host logs it)
    pub fn emit(&mut self, e: Event) -> bool {
        let n = self.out.events_len as usize;
        if n >= MAX_EVENTS {
            self.out.events_dropped = self.out.events_dropped.saturating_add(1);
            return false;
        }
        self.out.events[n] = e.rec;
        self.out.events_len += 1;
        true
    }

    /// the events emitted so far in this call
    pub fn events(&self) -> &[EventRec] {
        &self.out.events[..(self.out.events_len as usize).min(MAX_EVENTS)]
    }
}

/// One audio call: `frames` frames, the declared audio inputs and outputs (stereo each), the control inputs, and the
/// `Outputs`. Audio outputs start at silence.
pub struct Block<'a> {
    frames: usize,
    inp: &'a In,
    outputs: Outputs<'a>,
}

impl<'a> Block<'a> {
    pub fn new(inp: &'a In, out: &'a mut Out) -> Block<'a> {
        Block { frames: (inp.frames as usize).min(MAX_BLOCK), inp, outputs: Outputs { out } }
    }

    pub fn frames(&self) -> usize {
        self.frames
    }

    /// the n-th declared audio input (left, right); silence when the host connected nothing
    pub fn audio_in(&self, port: usize) -> (&[f32], &[f32]) {
        let p = port.min(MAX_AUDIO_PORTS - 1);
        (&self.inp.audio[p][0][..self.frames], &self.inp.audio[p][1][..self.frames])
    }

    /// the n-th declared audio output (left, right)
    pub fn audio_out(&mut self, port: usize) -> (&mut [f32], &mut [f32]) {
        let p = port.min(MAX_AUDIO_PORTS - 1);
        let [l, r] = &mut self.outputs.out.audio[p];
        (&mut l[..self.frames], &mut r[..self.frames])
    }

    /// the n-th declared audio output together with the events emitted so far in this call – what a module needs to
    /// render its notes into audio (`cv::GateCv::render`)
    pub fn audio_out_with_events(&mut self, port: usize) -> (&mut [f32], &mut [f32], &[EventRec]) {
        let p = port.min(MAX_AUDIO_PORTS - 1);
        let out = &mut *self.outputs.out;
        let n = (out.events_len as usize).min(MAX_EVENTS);
        let [l, r] = &mut out.audio[p];
        (&mut l[..self.frames], &mut r[..self.frames], &out.events[..n])
    }

    /// the n-th declared control input's value for this call
    pub fn control_in(&self, port: usize) -> f32 {
        self.inp.control.get(port).copied().unwrap_or(0.0)
    }

    pub fn outputs(&mut self) -> &mut Outputs<'a> {
        &mut self.outputs
    }

    /// shorthand for `outputs().emit`
    pub fn emit(&mut self, e: Event) -> bool {
        self.outputs.emit(e)
    }

    /// shorthand for `outputs().set_control`
    pub fn set_control(&mut self, port: usize, value: f32) {
        self.outputs.set_control(port, value)
    }
}
