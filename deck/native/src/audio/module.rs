//! A rig module as an audio node (#280, module_host.rs): `module:<name>` in a scene. Its inputs are the module's audio
//! inputs, control inputs and parameters (every parameter is also a control input – a link into `w.spread` sets it, an
//! unlinked one holds the node's TOML value or the descriptor's default), its outputs the module's audio outputs.
//! Each block the tempo map and the inputs go into the module's memory, `dm_process` runs under a time budget, the
//! outputs come back. A trap or an overrun silences the node – zeros from then on, one log line – the show goes on.
//! Node params besides the module's own: `seed` (the module's seed, default 1 like `walk`), `character`, `persona`,
//! `mood` (strings), `vars` (a table of key → number): what `Ctx` reads.
//! Nothing here allocates per block; the module's own code is the module's business (it may not allocate either – the SDK
//! says so; tests/no_alloc.rs covers the built-ins only).
use super::{Block, Param, Stereo, MAX_BLOCK};
use crate::graph::{Kind, Node};
use crate::module_host::{InBind, Instance, Library};
use crate::tempo::BeatReader;
use anyhow::{anyhow, Result};
use deck_core::tempo::TempoMap;

pub(super) struct ModuleBlock {
    inst: Instance,
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
    pub(super) fn new(node: &Node, rate: f32, params: &[Param], beats: BeatReader) -> Result<ModuleBlock> {
        let Kind::Module(kind) = node.kind else { unreachable!("a module kind") };
        let loaded = Library::global()
            .of_kind(kind)
            .ok_or_else(|| anyhow!("node '{}': the module '{}' is not loaded (DECK_MODULES)", node.id, kind.spec().kind))?;
        let seed = match node.params.get("seed") {
            Some(toml::Value::Integer(i)) => *i as u64,
            Some(toml::Value::Float(f)) => *f as u64,
            _ => 1,
        };
        let mut inst = Instance::new(&loaded, f64::from(rate), seed).map_err(|e| anyhow!("node '{}': {e:#}", node.id))?;
        inst.set_character(node.str("character").unwrap_or(""), node.str("persona").unwrap_or(""), node.str("mood").unwrap_or(""));
        if let Some(toml::Value::Table(t)) = node.params.get("vars") {
            let vars: Vec<(String, f32)> = t
                .iter()
                .filter_map(|(k, v)| v.as_float().map(|f| f as f32).or_else(|| v.as_integer().map(|i| i as f32)).map(|f| (k.clone(), f)))
                .collect();
            inst.set_vars(&vars);
        }
        let inputs: Vec<(InBind, Param)> = loaded.layout.inputs.iter().copied().zip(params.iter().cloned()).collect();
        let mut sent = vec![f32::NAN.to_bits(); loaded.desc.params.len()];
        for (bind, p) in &inputs {
            if let InBind::Param(i) = bind {
                inst.set_param(*i, p.get());
                sent[*i] = p.get().to_bits();
            }
        }
        Ok(ModuleBlock {
            inst,
            inputs,
            sent,
            outs: loaded.layout.outputs.clone(),
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
        if self.inst.silenced().is_none() {
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
        if self.inst.silenced().is_some() {
            if !self.reported {
                self.reported = true;
                crate::module_host::note_silenced();
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
