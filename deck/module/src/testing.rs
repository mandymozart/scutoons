//! Native test harness: a module run exactly as the wasm exports run it (`rt::*`), without wasm – so a module's own
//! tests, and the proof that a module equals a built-in, are plain `#[test]`s. (The build service runs a creator's
//! tests as wasm under wasmtime instead – RIG-EDITING.md §4.)
use crate::abi::{get_str, put_str, EventRec, Io, MAX_VARS};
use crate::{rt, Module};
use std::cell::RefCell;

thread_local! {
    static LOG: RefCell<Vec<(u32, String)>> = const { RefCell::new(Vec::new()) };
}

/// where `log!` goes natively
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn sink(level: u32, line: &str) {
    // the harness's own Vec is not the module's allocation (on wasm a log line allocates nothing)
    let armed = crate::v2::alloc::suspend();
    LOG.with(|l| l.borrow_mut().push((level, line.to_owned())));
    crate::v2::alloc::resume(armed);
}

/// the lines `log!` wrote on this thread since the last call: (level, text)
pub fn take_log() -> Vec<(u32, String)> {
    LOG.with(|l| std::mem::take(&mut *l.borrow_mut()))
}

/// One event as the host sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct EventView {
    pub name: String,
    pub start: f64,
    pub len: f64,
    pub port: u32,
    pub props: Vec<(String, f32)>,
}

impl EventView {
    pub fn from_rec(r: &EventRec) -> EventView {
        EventView {
            name: get_str(&r.name).to_owned(),
            start: r.start,
            len: r.len,
            port: r.port,
            props: (0..(r.nprops as usize).min(r.keys.len())).map(|i| (get_str(&r.keys[i]).to_owned(), r.values[i])).collect(),
        }
    }

    pub fn prop(&self, key: &str) -> Option<f32> {
        self.props.iter().find(|(k, _)| k == key).map(|&(_, v)| v)
    }
}

/// a module and its `Io`
pub struct Rig<M: Module> {
    pub io: Box<Io>,
    pub module: M,
}

impl<M: Module> Rig<M> {
    /// 120 bpm, 4 beats a bar, the tempo running from frame 0 = beat 0
    pub fn new(rate: f64, seed: u64) -> Rig<M> {
        let mut io = Io::boxed();
        io.inp.rate = rate;
        io.inp.seed = seed;
        io.inp.bpm = 120.0;
        io.inp.per_bar = 4;
        io.inp.running = 1;
        let module = rt::create::<M>(&mut io);
        Rig { io, module }
    }

    pub fn set_tempo(&mut self, anchor_frame: f64, anchor_beat: f64, bpm: f64, per_bar: u32, running: bool) {
        let i = &mut self.io.inp;
        (i.tempo_frame, i.tempo_beat, i.bpm, i.per_bar, i.running) = (anchor_frame, anchor_beat, bpm, per_bar, u32::from(running));
    }

    pub fn set_character(&mut self, id: &str, persona: &str, mood: &str) {
        put_str(&mut self.io.inp.character_id, id);
        put_str(&mut self.io.inp.character_persona, persona);
        put_str(&mut self.io.inp.character_mood, mood);
    }

    pub fn set_vars(&mut self, vars: &[(&str, f32)]) {
        let n = vars.len().min(MAX_VARS);
        for (i, (k, v)) in vars.iter().take(n).enumerate() {
            put_str(&mut self.io.inp.var_keys[i], k);
            self.io.inp.var_values[i] = *v;
        }
        self.io.inp.vars_len = n as u32;
    }

    pub fn set_param(&mut self, index: usize, value: f32) {
        self.module.set_param(index, value);
    }

    /// one audio call of `frames` frames starting at audio frame `frame`
    pub fn process(&mut self, frame: f64, frames: u32) {
        self.io.inp.frame = frame;
        rt::process(&mut self.module, &mut self.io, frames);
    }

    pub fn control(&mut self, frame: f64, frames: u32) {
        self.io.inp.frame = frame;
        rt::control(&mut self.module, &mut self.io, frames);
    }

    /// the events the last call emitted
    pub fn events(&self) -> Vec<EventView> {
        self.io.out.events[..self.io.out.events_len as usize].iter().map(EventView::from_rec).collect()
    }

    /// the last call's audio output `port`, left and right
    pub fn audio_out(&self, port: usize) -> (&[f32], &[f32]) {
        let n = self.io.inp.frames as usize;
        (&self.io.out.audio[port][0][..n], &self.io.out.audio[port][1][..n])
    }
}
