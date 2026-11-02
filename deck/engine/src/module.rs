//! A rig module as an audio node (#280): `module:<name>` in a scene. Its inputs are the module's audio inputs, control
//! inputs and parameters (every parameter is also a control input – a link into `w.spread` sets it, an unlinked one holds
//! the node's TOML value or the descriptor's default), its outputs the module's audio outputs.
//! Each block the tempo map and the inputs go into the module's memory, `dm_process` runs under a time budget, the
//! outputs come back. A trap or an overrun silences the node – zeros from then on, one report – the show goes on.
//! Node params besides the module's own: `seed` (the module's seed, default 1 like `walk`), `character`, `persona`,
//! `mood` (strings), `vars` (a table of key → number): what `Ctx` reads.
//! The module runtime is a seam: the platform hands the engine a `ModuleSource` (the native deck: wasmtime behind
//! deck-module-host's `Backend`, the browser: its own backend) and the engine drives the instances it opens through
//! `ModuleInstance` – blanket-implemented for deck-module-host's `Instance<B>`, so the host logic stays in that crate.
//! Nothing here allocates per block; the module's own code is the module's business (it may not allocate either – the SDK
//! says so; tests/no_alloc.rs covers the built-ins only).
use super::{Block, Param, Stereo, MAX_BLOCK};
use anyhow::{anyhow, Result};
use deck_core::tempo::{BeatReader, TempoMap};
use deck_module_host::{Backend, InBind};
use deck_space::graph::{Kind, Node};
use deck_space::modreg::ModuleKind;
use std::sync::Arc;

/// What the engine drives of one running module (deck-module-host's `Instance`, whatever its backend).
pub trait ModuleInstance: Send {
    /// set once a trap or an overrun silenced the instance (it then renders zeros)
    fn silenced(&self) -> bool;
    fn set_character(&mut self, id: &str, persona: &str, mood: &str);
    fn set_vars(&mut self, vars: &[(String, f32)]);
    fn set_tempo(&mut self, map: &TempoMap);
    fn set_param(&mut self, index: usize, value: f32);
    fn set_control_in(&mut self, port: usize, value: f32);
    fn set_audio_in(&mut self, port: usize, l: &[f32], r: &[f32]);
    fn process(&mut self, frame: f64, frames: u32);
    fn read_audio_out(&mut self, port: usize, l: &mut [f32], r: &mut [f32]);
}

impl<B: Backend + Send> ModuleInstance for deck_module_host::Instance<B> {
    fn silenced(&self) -> bool {
        deck_module_host::Instance::silenced(self).is_some()
    }
    fn set_character(&mut self, id: &str, persona: &str, mood: &str) {
        deck_module_host::Instance::set_character(self, id, persona, mood)
    }
    fn set_vars(&mut self, vars: &[(String, f32)]) {
        deck_module_host::Instance::set_vars(self, vars)
    }
    fn set_tempo(&mut self, map: &TempoMap) {
        deck_module_host::Instance::set_tempo(self, map)
    }
    fn set_param(&mut self, index: usize, value: f32) {
        deck_module_host::Instance::set_param(self, index, value)
    }
    fn set_control_in(&mut self, port: usize, value: f32) {
        deck_module_host::Instance::set_control_in(self, port, value)
    }
    fn set_audio_in(&mut self, port: usize, l: &[f32], r: &[f32]) {
        deck_module_host::Instance::set_audio_in(self, port, l, r)
    }
    fn process(&mut self, frame: f64, frames: u32) {
        deck_module_host::Instance::process(self, frame, frames);
    }
    fn read_audio_out(&mut self, port: usize, l: &mut [f32], r: &mut [f32]) {
        deck_module_host::Instance::read_audio_out(self, port, l, r)
    }
}

/// A module opened for a node: the instance and how the graph's ports map into it.
pub struct Opened {
    pub instance: Box<dyn ModuleInstance>,
    /// per graph input: where it goes in the module
    pub inputs: Vec<InBind>,
    /// per graph output: the module's audio output port
    pub outputs: Vec<usize>,
    /// how many parameters the module has
    pub params: usize,
}

/// Where the platform's loaded modules are (the native deck: its library of `.wasm` files, compiled once).
pub trait ModuleSource: Send + Sync {
    /// the module behind `kind`, instantiated for `rate` Hz with `seed`; Err = not loaded / failed to instantiate
    fn open(&self, kind: ModuleKind, rate: f64, seed: u64) -> Result<Opened>;
    /// an instance was silenced (the platform's HUD and tests can look); called once per instance, from the render
    fn silenced(&self) {}
}

pub(super) struct ModuleBlock {
    inst: Box<dyn ModuleInstance>,
    source: Arc<dyn ModuleSource>,
    /// per graph input: where it goes in the module, and its param
    inputs: Vec<(InBind, Param)>,
    /// the parameter values last handed over (bits, so NaN compares)
    sent: Vec<u32>,
    outs: Vec<usize>,
    beats: BeatReader,
    /// without a tempo clock: 120 bpm from frame 0, counted here
    own: TempoMap,
    frame: f64,
    silence: Box<[f32]>,
    reported: bool,
}

impl ModuleBlock {
    pub(super) fn new(node: &Node, rate: f32, params: &[Param], beats: BeatReader, source: &Arc<dyn ModuleSource>) -> Result<ModuleBlock> {
        let Kind::Module(kind) = node.kind else { unreachable!("a module kind") };
        let seed = match node.params.get("seed") {
            Some(toml::Value::Integer(i)) => *i as u64,
            Some(toml::Value::Float(f)) => *f as u64,
            _ => 1,
        };
        let Opened { instance: mut inst, inputs: binds, outputs, params: count } =
            source.open(kind, f64::from(rate), seed).map_err(|e| anyhow!("node '{}': {e:#}", node.id))?;
        inst.set_character(node.str("character").unwrap_or(""), node.str("persona").unwrap_or(""), node.str("mood").unwrap_or(""));
        if let Some(toml::Value::Table(t)) = node.params.get("vars") {
            let vars: Vec<(String, f32)> = t
                .iter()
                .filter_map(|(k, v)| v.as_float().map(|f| f as f32).or_else(|| v.as_integer().map(|i| i as f32)).map(|f| (k.clone(), f)))
                .collect();
            inst.set_vars(&vars);
        }
        let inputs: Vec<(InBind, Param)> = binds.into_iter().zip(params.iter().cloned()).collect();
        let mut sent = vec![f32::NAN.to_bits(); count];
        for (bind, p) in &inputs {
            if let InBind::Param(i) = bind {
                inst.set_param(*i, p.get());
                sent[*i] = p.get().to_bits();
            }
        }
        Ok(ModuleBlock {
            inst,
            source: source.clone(),
            inputs,
            sent,
            outs: outputs,
            beats,
            own: TempoMap::new(120.0, 4, f64::from(rate)),
            frame: 0.0,
            silence: vec![0.0; MAX_BLOCK].into(),
            reported: false,
        })
    }
}

impl Block for ModuleBlock {
    fn process(&mut self, ins: &[Option<&Stereo>], outs: &mut [Stereo], n: usize) {
        let (map, start) = match self.beats.block() {
            Some((m, _, start)) => (m, start),
            None => (self.own, self.frame),
        };
        self.frame = start + n as f64;
        if !self.inst.silenced() {
            self.inst.set_tempo(&map);
            for (k, (bind, param)) in self.inputs.iter().enumerate() {
                match *bind {
                    InBind::Audio(port) => match ins.get(k).copied().flatten() {
                        Some(b) => self.inst.set_audio_in(port, &b.l[..n], &b.r[..n]),
                        None => self.inst.set_audio_in(port, &self.silence[..n], &self.silence[..n]),
                    },
                    InBind::Control(port) => self.inst.set_control_in(port, param.get()),
                    InBind::Param(i) => {
                        let v = param.get();
                        if v.to_bits() != self.sent[i] {
                            self.sent[i] = v.to_bits();
                            self.inst.set_param(i, v);
                        }
                    }
                }
            }
            self.inst.process(start, n as u32);
        }
        if self.inst.silenced() {
            if !self.reported {
                self.reported = true;
                self.source.silenced();
            }
            for o in outs.iter_mut() {
                o.l[..n].fill(0.0);
                o.r[..n].fill(0.0);
            }
            return;
        }
        for (o, &port) in outs.iter_mut().zip(&self.outs) {
            self.inst.read_audio_out(port, &mut o.l[..n], &mut o.r[..n]);
        }
    }
}
