//! The conformance fixture of the module ABI (#341, #343, research/ABI-V2.md §7): one module that touches every region the
//! layout has – audio of 2 and 5 channels each way, a control input, a control output with a `path` and a plain one,
//! an event input and output, a declared variable, a sensor it `wants`, an asset slot, a factory preset – and that
//! misbehaves on request (its `mode`). Not an instrument. What it plays:
//! - `out` (2 ch): `in` + 0.25 + `bias`, per channel and frame.
//! - `wide_out` (5 ch): channel c = `wide_in` channel c × (c + 1); channel 4 adds the `sample` asset's first channel
//!   (silence while the slot is not ready).
//! - `level` (path `fx:@gain`) = the control input `push` × 2, written every call; `tilt` = the motion sensor's accel x
//!   when it is live or replayed, else not written (the consumer holds); `info` = bpm / 100 + variable `fx.level` + the
//!   beat at the call's first frame / 1000.
//! - `echo`: every note of `notes` again, id + 1000 (capacity 4: more are dropped and counted).
//!
//! `mode`: 0 play · 1 panic · 2 a loop that never returns · 3 allocate in the call · 4 allocate 8 MiB in the call
//! (the memory is frozen: the allocation fails, a trap) · 5 log 50 lines a call · 6 spin `work` × 1000 steps first.
#![allow(clippy::duplicated_attributes)]
use deck_module::log;
use deck_module::{Block, Ctx, Ev, Kind, Module};

#[derive(Module)]
#[module(name = "fixture", wants = "motion", doc = "the module ABI conformance fixture")]
#[reads("fx.level")]
#[asset(name = "sample", kind = "pcm", max_bytes = 65536, doc = "played on wide_out channel 4")]
#[preset(name = "init", doc = "a little bias", values(bias = 0.5))]
#[port(input, audio, name = "in", channels = 2)]
#[port(input, audio, name = "wide_in", channels = 5)]
#[port(input, control, name = "push")]
#[port(input, event, name = "notes", names = "music.note", capacity = 8)]
#[port(output, audio, name = "out", channels = 2)]
#[port(output, audio, name = "wide_out", channels = 5)]
#[port(output, control, name = "level", path = "fx:@gain", mode = "set")]
#[port(output, control, name = "tilt")]
#[port(output, control, name = "info")]
#[port(output, event, name = "echo", names = "music.note", capacity = 4)]
pub struct Fixture {
    #[param(choices = "play|panic|loop|alloc|grow|chatty|work", default = 0, doc = "how the fixture behaves")]
    pub mode: f32,
    #[param(range = -1.0..1.0, default = 0.0, doc = "added to out")]
    pub bias: f32,
    #[param(range = 0.0..1000000.0, default = 0.0, doc = "mode 6: thousands of spin steps")]
    pub work: f32,
}

fn spin(steps: u64) -> u64 {
    let mut n = 0u64;
    for i in 0..steps {
        n = core::hint::black_box(n.wrapping_mul(6364136223846793005).wrapping_add(i));
    }
    n
}

impl Module for Fixture {
    /// a heap with room to spare before the memory is frozen: a stray small allocation in a show is then served and
    /// counted (mode 3); one that needs more memory fails – a trap, a restart (mode 4)
    fn init(&mut self, _: &Ctx) {
        drop(core::hint::black_box(Vec::<u8>::with_capacity(256 << 10)));
    }

    fn process(&mut self, block: &mut Block, ctx: &Ctx) {
        match self.mode as u32 {
            1 => panic!("boom {}", ctx.seed()),
            2 => loop {
                core::hint::black_box(spin(1 << 20));
            },
            3 => {
                core::hint::black_box(vec![1u8; 16]);
            }
            4 => {
                core::hint::black_box(vec![1u8; 8 << 20]);
            }
            5 => {
                for i in 0..50 {
                    log!("line {i}");
                }
            }
            6 => {
                core::hint::black_box(spin(self.work as u64 * 1000));
            }
            _ => {}
        }
        let add = 0.25 + self.bias;
        {
            let inp = block.audio_in(Self::IN);
            let mut out = block.audio_out(Self::OUT);
            for c in 0..out.ch() {
                for (o, i) in out.chan(c).iter_mut().zip(inp.chan(c)) {
                    *o = i + add;
                }
            }
        }
        {
            let wide = block.audio_in(Self::WIDE_IN);
            let sample = ctx.asset(Self::SAMPLE).map(|a| a.pcm(0)).unwrap_or(&[]);
            let mut out = block.audio_out(Self::WIDE_OUT);
            for c in 0..out.ch() {
                let gain = (c + 1) as f32;
                for (k, (o, i)) in out.chan(c).iter_mut().zip(wide.chan(c)).enumerate() {
                    *o = i * gain + if c == 4 { sample.get(k).copied().unwrap_or(0.0) } else { 0.0 };
                }
            }
        }
        block.set(Self::LEVEL, block.control_in(Self::PUSH) * 2.0);
        let m = ctx.motion();
        if m.status().live() {
            block.set(Self::TILT, m.accel[0]);
        }
        let (bpm, beat) = (ctx.tempo().bpm as f32, ctx.beats().0 as f32);
        block.set(Self::INFO, bpm / 100.0 + ctx.var(Self::FX_LEVEL).unwrap_or(0.0) + beat / 1000.0);
        for ev in block.events_in(Self::NOTES) {
            let echo = match ev.kind() {
                Some(Kind::NoteOn) => Ev::note_on(ev.frame, ev.id + 1000, ev.pitch_hz(), ev.key(), ev.velocity()),
                Some(Kind::NoteOff) => Ev::note_off(ev.frame, ev.id + 1000),
                _ => continue,
            };
            block.emit(Self::ECHO, echo);
        }
    }
}
