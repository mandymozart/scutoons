//! `stress` – the #306 benchmark's hostile workloads (agents/research/MODULE-BENCH.md). `mode` 0 plays silence;
//! 1 is a runaway: `loop {}` with nothing inside (the cheapest loop a gas counter has to catch); 2 grows the linear
//! memory by 16 pages (1 MB) a call with `memory.grow` – after the host froze the memory – and writes the result
//! (the old size in pages, or -1 refused) to the `grown` control output.
#![allow(clippy::duplicated_attributes)]
use deck_module::{Block, Ctx, Module};

#[derive(Module)]
#[module(
    name = "stress",
    doc = "a runaway loop or 1 MB of memory growth a call (the #306 benchmark)"
)]
#[port(output, audio, name = "out", channels = 2)]
#[port(output, control, name = "grown")]
pub struct Stress {
    #[param(choices = "play|runaway|grow", default = 0, doc = "what the call does")]
    mode: f32,
}

#[cfg(target_arch = "wasm32")]
fn grow_1mb() -> f32 {
    core::arch::wasm32::memory_grow(0, 16) as isize as f32
}

#[cfg(not(target_arch = "wasm32"))]
fn grow_1mb() -> f32 {
    -1.0
}

impl Module for Stress {
    #[allow(clippy::empty_loop)]
    fn process(&mut self, block: &mut Block, _: &Ctx) {
        match self.mode as u32 {
            1 => loop {},
            2 => {
                let r = grow_1mb();
                block.set(Self::GROWN, r);
            }
            _ => {}
        }
        let mut out = block.audio_out(Self::OUT);
        for c in 0..out.ch() {
            out.chan(c).fill(0.0);
        }
    }
}
