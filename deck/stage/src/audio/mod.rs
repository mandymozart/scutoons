//! The audio graph (CYBERDECKS.md §8): the graph's audio nodes as blocks in topological order, run by the device
//! callback in blocks of at most MAX_BLOCK frames. Everything is allocated at build time – the callback only reads
//! atomics (Param), pops lock-free rings (rtrb) and writes preallocated buffers (tests/no_alloc.rs proves it).
//! The frame counter and the per-block stamps are the stage's time base (§8 "Two clocks", clock.rs): each backend calls
//! `mark` with the moment the block it is about to render will be heard.
pub mod backend;
mod blocks;

use crate::clock::Stamp;
use crate::graph::{Domain, Graph, Kind, PortType};
use anyhow::Result;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub const MAX_BLOCK: usize = 1024;
/// the most input ports any audio kind has (the mixer: 4 audio + 4 gains)
const MAX_PORTS: usize = 8;

/// A control value shared with the audio thread: an f32 in an atomic, written by the frame thread, read per block.
#[derive(Clone, Debug)]
pub struct Param(Arc<AtomicU32>);

impl Param {
    pub fn new(v: f32) -> Param {
        Param(Arc::new(AtomicU32::new(v.to_bits())))
    }
    pub fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
    pub fn set(&self, v: f32) {
        self.0.store(v.to_bits(), Ordering::Relaxed)
    }
}

/// A decoded track at its own sample rate (the music block resamples while it plays).
pub struct Track {
    pub rate: f32,
    pub l: Vec<f32>,
    pub r: Vec<f32>,
}

impl Track {
    pub fn seconds(&self) -> f32 {
        self.l.len() as f32 / self.rate
    }
}

/// One stereo block buffer (MAX_BLOCK frames, a block uses the first `n`).
pub struct Stereo {
    pub l: Box<[f32]>,
    pub r: Box<[f32]>,
}

impl Stereo {
    fn new() -> Stereo {
        Stereo { l: vec![0.0; MAX_BLOCK].into(), r: vec![0.0; MAX_BLOCK].into() }
    }
}

/// An audio node's DSP: reads its inputs (by input port index; None = not linked or a control port), writes its outputs.
pub trait Block: Send {
    fn process(&mut self, ins: &[Option<&Stereo>], outs: &mut [Stereo], n: usize);
}

struct Slot {
    block: Box<dyn Block>,
    out: Vec<Stereo>,
    /// per input port: the (slot, output port) it reads
    inputs: Vec<Option<(usize, usize)>>,
}

/// The stage's clock: frames rendered for the device (or by the null backend's timer) and the stamp of the newest
/// block – when its first frame is heard (clock.rs turns the stamps into the heard position per video frame).
#[derive(Clone)]
pub struct Clock {
    pub frames: Arc<AtomicU64>,
    pub rate: f32,
    pub stamp: Arc<Stamp>,
}

impl Clock {
    /// frames rendered so far (ahead of what is heard by the output latency + the block)
    pub fn rendered(&self) -> u64 {
        self.frames.load(Ordering::Relaxed)
    }
}

pub struct Engine {
    slots: Vec<Slot>,
    out_slot: Option<usize>,
    master: Param,
    /// the master gain the last block ended on – a change ramps over one block (no click on a mute)
    gain: f32,
    frames: Arc<AtomicU64>,
    stamp: Arc<Stamp>,
}

/// What the frame thread keeps of a built engine.
pub struct Handles {
    /// the control inputs of audio nodes: (graph node, input port, param)
    pub params: Vec<(usize, usize, Param)>,
    /// chapter_music nodes: where their loader sends the decoded track
    pub music: Vec<MusicFeed>,
    /// fft nodes: their sample ring
    pub taps: Vec<(usize, rtrb::Consumer<f32>)>,
    pub clock: Clock,
    pub master: Param,
}

pub struct MusicFeed {
    pub node: usize,
    pub tracks: rtrb::Producer<Arc<Track>>,
    /// what the HUD shows ("loading", "playing <track>", the error)
    pub status: Arc<Mutex<String>>,
}

impl Engine {
    /// Build the graph's audio nodes for `rate`. `volume` is the master gain on top of the audio_out node's own.
    pub fn build(graph: &Graph, rate: f32, volume: f32) -> Result<(Engine, Handles)> {
        let mut slot_of = vec![None; graph.nodes.len()];
        let mut slots = Vec::new();
        let mut handles = Handles {
            params: Vec::new(),
            music: Vec::new(),
            taps: Vec::new(),
            clock: Clock { frames: Arc::new(AtomicU64::new(0)), rate, stamp: Arc::new(Stamp::default()) },
            master: Param::new(volume),
        };
        for &i in &graph.order {
            let node = &graph.nodes[i];
            if node.kind.domain() != Domain::Audio {
                continue;
            }
            let ports = node.kind.inputs();
            debug_assert!(ports.len() <= MAX_PORTS);
            let params: Vec<Param> = ports
                .iter()
                .enumerate()
                .map(|(k, port)| {
                    let p = Param::new(node.f32(port.name, blocks::default_for(node.kind, port.name)));
                    if port.ty == PortType::Control && graph.link_into(i, k).is_some() {
                        handles.params.push((i, k, p.clone()));
                    }
                    p
                })
                .collect();
            let inputs = (0..ports.len())
                .map(|k| match graph.link_into(i, k) {
                    Some(l) if l.ty == PortType::Audio => slot_of[l.from.0].map(|s| (s, l.from.1)),
                    _ => None,
                })
                .collect();
            let block = blocks::make(node, i, rate, &params, &mut handles)?;
            slot_of[i] = Some(slots.len());
            // the audio_out sink has no output port but keeps its block in one buffer – render() reads it
            let outs = if node.kind == Kind::AudioOut { 1 } else { node.kind.outputs().len() };
            slots.push(Slot { block, out: (0..outs).map(|_| Stereo::new()).collect(), inputs });
        }
        let out_slot = graph.first(Kind::AudioOut).and_then(|i| slot_of[i]);
        let engine = Engine {
            slots,
            out_slot,
            master: handles.master.clone(),
            gain: volume,
            frames: handles.clock.frames.clone(),
            stamp: handles.clock.stamp.clone(),
        };
        Ok((engine, handles))
    }

    /// The block about to be rendered is heard at `heard_ns` (clock::now_ns's clock) – the backends call this right
    /// before `render`. Never allocates, never waits.
    pub fn mark(&self, heard_ns: u64) {
        self.stamp.write([self.frames.load(Ordering::Relaxed), heard_ns]);
    }

    /// Fill an interleaved device buffer (`channels` per frame; L, R, the rest silent). Never allocates.
    pub fn render<T: Copy>(&mut self, out: &mut [T], channels: usize, conv: impl Fn(f32) -> T) {
        let channels = channels.max(1);
        let total = out.len() / channels;
        let mut done = 0;
        while done < total {
            let n = (total - done).min(MAX_BLOCK);
            self.run(n);
            let (from, to) = (self.gain, self.master.get());
            self.gain = to;
            let frames = &mut out[done * channels..(done + n) * channels];
            match self.out_slot.map(|s| &self.slots[s].out[0]) {
                Some(o) => {
                    for (f, frame) in frames.chunks_exact_mut(channels).enumerate() {
                        let master = from + (to - from) * (f + 1) as f32 / n as f32;
                        let (l, r) = (safe(o.l[f] * master), safe(o.r[f] * master));
                        if channels == 1 {
                            frame[0] = conv(0.5 * (l + r));
                            continue;
                        }
                        frame[0] = conv(l);
                        frame[1] = conv(r);
                        for x in &mut frame[2..] {
                            *x = conv(0.0);
                        }
                    }
                }
                None => frames.iter_mut().for_each(|x| *x = conv(0.0)),
            }
            done += n;
            self.frames.fetch_add(n as u64, Ordering::Relaxed);
        }
    }

    fn run(&mut self, n: usize) {
        for i in 0..self.slots.len() {
            let (before, rest) = self.slots.split_at_mut(i);
            let slot = &mut rest[0];
            let mut ins: [Option<&Stereo>; MAX_PORTS] = [None; MAX_PORTS];
            for (k, src) in slot.inputs.iter().enumerate() {
                if let Some((s, p)) = *src {
                    ins[k] = Some(&before[s].out[p]);
                }
            }
            slot.block.process(&ins[..slot.inputs.len()], &mut slot.out, n);
        }
    }
}

/// a hard limit before the device (a broken patch must never blast the speakers) and no NaN / inf
fn safe(x: f32) -> f32 {
    if x.is_finite() {
        x.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Scene;

    fn engine(text: &str) -> (Engine, Handles) {
        let g = Graph::build(&Scene::parse(text).unwrap()).unwrap();
        Engine::build(&g, 48_000.0, 1.0).unwrap()
    }

    const OSC_OUT: &str = "[[node]]\nid=\"o\"\nkind=\"osc\"\nfreq=440\ngain=0.5\n\
                           [[node]]\nid=\"out\"\nkind=\"audio_out\"\nvolume=1\n[[link]]\nfrom=\"o\"\nto=\"out.in\"\n";

    #[test]
    fn an_oscillator_reaches_the_output() {
        let (mut e, h) = engine(OSC_OUT);
        let mut buf = vec![0.0f32; 4800 * 2];
        e.render(&mut buf, 2, |x| x);
        let peak = buf.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(peak > 0.3 && peak <= 0.5 + 1e-3, "peak {peak}");
        assert_eq!(h.clock.frames.load(Ordering::Relaxed), 4800);
    }

    #[test]
    fn the_master_param_scales_and_extra_channels_are_silent() {
        let (mut e, h) = engine(OSC_OUT);
        h.master.set(0.0);
        let mut buf = vec![1.0f32; 512 * 4];
        e.render(&mut buf, 4, |x| x); // the ramp down
        assert_eq!(buf[511 * 4], 0.0, "the ramp ends at the new gain");
        e.render(&mut buf, 4, |x| x);
        assert!(buf.iter().all(|x| *x == 0.0));
    }

    #[test]
    fn no_output_node_is_silence() {
        let (mut e, _) = engine("[[node]]\nid=\"o\"\nkind=\"osc\"\n");
        let mut buf = vec![1.0f32; 256];
        e.render(&mut buf, 2, |x| x);
        assert!(buf.iter().all(|x| *x == 0.0));
    }

    #[test]
    fn linked_control_inputs_become_params() {
        let (_, h) = engine(
            "[[node]]\nid=\"l\"\nkind=\"lfo\"\n[[node]]\nid=\"o\"\nkind=\"osc\"\n[[link]]\nfrom=\"l\"\nto=\"o.freq\"\n",
        );
        assert_eq!(h.params.len(), 1);
        assert_eq!(h.params[0].1, Kind::Osc.input("freq").unwrap());
    }
}
