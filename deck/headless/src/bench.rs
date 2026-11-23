//! bench.rs (#834, research/RENDER-AUDIO-BUDGET.md §4.2): the stage's CPU side of a frame without a GPU – deck-space's
//! `Space::update` and its CPU writes (`write` + `write_labels`: the spheres, tubes and labels a deck uploads) over N frames
//! of a scene on the virtual clock, the camera close up (the knobs fold – the spread's case) or at the stage's view. The
//! clock is the caller's (the runner's `Instant`; the library stays platform-free), so the same numbers come from the PC
//! and the X1: `deck-scenarios --bench crows-rave --frames 1200` (deck/headless README).
use deck_space::graph::{Graph, Kind, PortType};
use deck_space::scene::Scene;
use deck_space::space::being::{Sphere, TubeVertex};
use deck_space::space::label::Label;
use deck_space::space::portrait::Portraits;
use deck_space::space::values::Table;
use deck_space::space::Space;
use deck_space::tempo;

const VIEW: (f32, f32) = (1920.0, 1080.0);
const DT: f32 = 1.0 / 60.0;
/// frames before the timed ones (the camera settles, the buffers grow once)
const WARM: usize = 120;

/// What a bench run measured: the scene's size, and per frame the update and the writes (ms: mean, p99).
#[derive(Debug, Clone, serde::Serialize)]
pub struct Bench {
    pub frames: usize,
    pub beings: usize,
    pub knobs: usize,
    pub folded: usize,
    pub spheres: usize,
    pub tube_vertices: usize,
    pub update_ms: (f64, f64),
    pub write_ms: (f64, f64),
}

impl Bench {
    /// one line for a terminal
    pub fn line(&self) -> String {
        format!(
            "bench: {} frames · {} beings, {} knobs ({} folded), {} spheres, {} tube vertices · update {:.3} ms (p99 {:.3}) · write {:.3} ms (p99 {:.3})",
            self.frames, self.beings, self.knobs, self.folded, self.spheres, self.tube_vertices, self.update_ms.0, self.update_ms.1, self.write_ms.0, self.write_ms.1
        )
    }
}

/// Run `scene` for `frames` timed frames; `close` = the camera zoomed in to the portrait (the knobs fold), `every_knob` =
/// every control input of the scene's own nodes a knob (as before the 4-a-node cap, #816); `now` = a clock in ms.
pub fn bench(scene: &Scene, frames: usize, close: bool, every_knob: bool, now: &mut dyn FnMut() -> f64) -> Result<Bench, String> {
    let graph = Graph::build(scene).map_err(|e| format!("{e:#}"))?;
    let node = graph.first(Kind::Space).ok_or("the scene has no space node – nothing on the stage")?;
    let mut look = scene.being.clone();
    if every_knob && look.expose.is_empty() {
        look.expose = graph.nodes.iter().flat_map(|n| n.kind.inputs().iter().filter(|i| i.ty == PortType::Control).map(move |i| format!("{}.{}", n.id, i.name))).collect();
    }
    let mut space = Space::new(&graph, node, &look, std::path::Path::new(""), Portraits::manual());
    space.set_view(VIEW.0, VIEW.1);
    if close {
        space.camera.zoom(100.0);
    }
    let mut table = Table::new(&graph);
    let (cap_s, cap_t) = space.capacity();
    let (mut spheres, mut tubes, mut labels) = (Vec::<Sphere>::with_capacity(cap_s), Vec::<TubeVertex>::with_capacity(cap_t), Vec::<Label>::new());
    let (mut update, mut write) = (Vec::with_capacity(frames), Vec::with_capacity(frames));
    let mut time = 0.0;
    for k in 0..WARM + frames {
        time += DT;
        let t0 = now();
        space.update(DT, time, &mut table, &tempo::Now::default(), &|_, _| 0.0);
        let t1 = now();
        space.write(&mut spheres, &mut tubes);
        labels.clear();
        space.write_labels(&mut labels);
        let t2 = now();
        if k >= WARM {
            update.push(t1 - t0);
            write.push(t2 - t1);
        }
    }
    let beings: Vec<_> = space.beings().collect();
    Ok(Bench {
        frames,
        beings: beings.len(),
        knobs: beings.iter().map(|b| b.buttons.len()).sum(),
        folded: beings.iter().flat_map(|b| &b.buttons).filter(|k| !k.hidden && k.pos != k.free).count(),
        spheres: spheres.len(),
        tube_vertices: tubes.len(),
        update_ms: stats(&mut update),
        write_ms: stats(&mut write),
    })
}

/// mean and p99 of `v` (ms)
fn stats(v: &mut [f64]) -> (f64, f64) {
    if v.is_empty() {
        return (0.0, 0.0);
    }
    v.sort_by(f64::total_cmp);
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    (mean, v[((v.len() - 1) as f64 * 0.99).round() as usize])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// the bench runs a scene with a being, counts what the stage draws and times both halves on the clock it is given
    #[test]
    fn the_bench_counts_the_stage_and_times_both_halves() {
        let scene = Scene::parse(include_str!("../../native/scenes/sandbox/cathedral.toml")).unwrap();
        let mut t = 0.0;
        let mut clock = || {
            t += 0.5;
            t
        };
        let b = bench(&scene, 10, true, false, &mut clock).unwrap();
        assert_eq!(b.frames, 10);
        assert!(b.beings >= 1 && b.knobs > 0 && b.spheres > 0 && b.tube_vertices > 0, "{b:?}");
        assert_eq!((b.update_ms, b.write_ms), ((0.5, 0.5), (0.5, 0.5)), "each half one tick of the clock");
        let all = bench(&scene, 2, true, true, &mut clock).unwrap();
        assert!(all.knobs >= b.knobs, "every control a knob: {} ≥ {}", all.knobs, b.knobs);
    }
}
