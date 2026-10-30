//! deck-core in a browser Web Worker (#182): the control graph + synth + FFT analysis and the relay protocol codec,
//! exported with wasm-bindgen. Each workload has two entry points: a per-call one (what a worker would really do every
//! frame / audio block – it crosses the JS ↔ wasm boundary with a Float32Array) and a `bench_*` one that loops inside
//! wasm (the pure compute cost). deck/lab/www runs both in a worker next to the TypeScript versions (deck/lab/ts).
use deck_core::analysis::{Bands, SIZE};
use deck_core::batch::BatchGraph;
use deck_core::{ControlGraph, Registry, SceneDef};
use wasm_bindgen::prelude::*;

pub mod formats;

const RATE: f32 = 48_000.0;

#[wasm_bindgen]
pub struct Graph {
    g: ControlGraph,
}

#[wasm_bindgen]
impl Graph {
    /// The bench scene: `n` LFO → smooth → mul chains (3n control nodes) and `voices` FM voices; `wide` = the chains
    /// independent (3 levels) instead of feeding each other (3n levels).
    #[wasm_bindgen(constructor)]
    pub fn new(n: usize, voices: usize, wide: bool) -> Result<Graph, JsError> {
        let scene = SceneDef::parse(&scene_text(n, voices, wide)).map_err(|e| JsError::new(&e))?;
        let g = ControlGraph::build(&scene, &Registry::compiled(), RATE).map_err(|e| JsError::new(&e))?;
        Ok(Graph { g })
    }

    pub fn step(&mut self, dt: f32) {
        self.g.step(dt);
    }

    /// One audio block into `out` (wasm-bindgen copies the Float32Array in and back).
    pub fn render(&mut self, out: &mut [f32]) {
        self.g.render(out);
    }

    pub fn bench_steps(&mut self, iters: u32) {
        for _ in 0..iters {
            self.g.step(1.0 / 60.0);
        }
    }

    /// `blocks` audio blocks of `frames` (128 = an AudioWorklet quantum); returns a checksum so nothing is optimised away.
    pub fn bench_render(&mut self, blocks: u32, frames: usize) -> f32 {
        let mut buf = vec![0.0; frames];
        let mut sum = 0.0;
        for _ in 0..blocks {
            self.g.render(&mut buf);
            sum += buf[0];
        }
        sum
    }
}

fn scene_text(n: usize, voices: usize, wide: bool) -> String {
    if wide {
        deck_core::graph::bench_scene_wide(n, voices)
    } else {
        deck_core::graph::bench_scene(n, voices)
    }
}

/// The same scene through deck-core's data-oriented evaluator (batch.rs).
#[wasm_bindgen]
pub struct Batch {
    g: BatchGraph,
}

#[wasm_bindgen]
impl Batch {
    #[wasm_bindgen(constructor)]
    pub fn new(n: usize, voices: usize, wide: bool) -> Result<Batch, JsError> {
        let scene = SceneDef::parse(&scene_text(n, voices, wide)).map_err(|e| JsError::new(&e))?;
        Ok(Batch { g: BatchGraph::build(&scene, RATE).map_err(|e| JsError::new(&e))? })
    }

    pub fn step(&mut self, dt: f32) {
        self.g.step(dt);
    }

    pub fn render(&mut self, out: &mut [f32]) {
        self.g.render(out);
    }

    pub fn bench_render(&mut self, blocks: u32, frames: usize) -> f32 {
        let mut buf = vec![0.0; frames];
        let mut sum = 0.0;
        for _ in 0..blocks {
            self.g.render(&mut buf);
            sum += buf[0];
        }
        sum
    }
}

#[wasm_bindgen]
pub struct Analyzer {
    b: Bands,
    window: Vec<f32>,
}

#[wasm_bindgen]
impl Analyzer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Analyzer {
        let window = (0..SIZE).map(|i| 0.5 * (std::f32::consts::TAU * 440.0 * i as f32 / RATE).sin()).collect();
        Analyzer { b: Bands::new(RATE), window }
    }

    /// The four values (level, bass, mid, high) of a 1024-sample window.
    pub fn analyse(&mut self, window: &[f32]) -> Vec<f32> {
        self.b.analyse(window).to_vec()
    }

    pub fn bench(&mut self, iters: u32) -> f32 {
        let mut sum = 0.0;
        for _ in 0..iters {
            sum += self.b.analyse(&self.window)[0];
        }
        sum
    }
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Decode one relay envelope and encode it again (the protocol crate, serde) – returns the encoded text.
#[wasm_bindgen]
pub fn codec_roundtrip(text: &str) -> Result<String, JsError> {
    let e: protocol::Envelope = serde_json::from_str(text).map_err(|e| JsError::new(&e.to_string()))?;
    serde_json::to_string(&e).map_err(|e| JsError::new(&e.to_string()))
}

/// `iters` decode + encode round trips inside wasm; returns the total encoded length.
#[wasm_bindgen]
pub fn bench_codec(text: &str, iters: u32) -> usize {
    let mut n = 0;
    for _ in 0..iters {
        let e: protocol::Envelope = serde_json::from_str(text).expect("a valid envelope");
        n += serde_json::to_string(&e).expect("serializes").len();
    }
    n
}
