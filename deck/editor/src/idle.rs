//! Home's stage: the deck's own idle scene (deck-space `IDLE_SCENE`, fireflies on black, no sound) drawn by the deck's
//! own scene renderer (deck-space `stage::Stage`) into the editor's stage texture – the app at rest looks like a deck at
//! rest. Made at the first Home frame, kept for the next.
use anyhow::Result;
use deck_space::graph::Graph;
use deck_space::scene::{Scene, IDLE_SCENE};
use deck_space::stage::{Over, Stage, Tick};

pub struct Idle {
    stage: Stage,
    graph: Graph,
    size: (u32, u32),
    started: std::time::Instant,
}

impl Idle {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, size: (u32, u32)) -> Result<Idle> {
        let graph = Graph::build(&Scene::parse(IDLE_SCENE)?)?;
        let mut stage = pollster::block_on(Stage::new(device, queue, crate::stage::FORMAT, size, 1.0, 1.0, true))?;
        pollster::block_on(stage.load(device, crate::stage::FORMAT, &graph, Vec::new()))?;
        Ok(Idle { stage, graph, size, started: std::time::Instant::now() })
    }

    /// the idle scene over `view` (the stage's texture, `size` pixels)
    pub fn frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, enc: &mut wgpu::CommandEncoder, (view, size): (&wgpu::TextureView, (u32, u32)), dt: f32, tempo: &deck_core::tempo::Now) {
        if size != self.size {
            self.stage.resize(device, size);
            self.size = size;
        }
        let input = |_: usize, _: &str, d: f32| d;
        let time = self.started.elapsed().as_secs_f32();
        let tick = Tick { graph: &self.graph, input: &input, tempo, time, dt };
        self.stage.frame(device, queue, enc, view, &tick, &Over { hud: None, splash: (0.0, false), label_centred: false, label_hidden: true, hud_at: None }, None);
    }
}
