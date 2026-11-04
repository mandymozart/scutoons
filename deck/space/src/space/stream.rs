//! What a deck streams of its being (#188 phase 4): every value its body shows (values.rs paths – the buttons' inputs,
//! the hoses' sources), sampled at most RATE times a second, sent only when it moved more than its epsilon; the head's
//! mood when it changed. The relay client (relay.rs) folds what it sent, so a reconnect sends everything once again;
//! its `Pace` is this stream's schedule too. Platform-free: plain `(path, vector)` pairs.
use super::being::Being;
use super::values::{self, Path, Values};
use crate::graph::{Graph, PortType};
use crate::relay::Pace;

struct Entry {
    path: String,
    at: Path,
    eps: f32,
    last: f32,
}

pub struct Stream {
    entries: Vec<Entry>,
    /// when the next sample may be taken (the show's time)
    pace: Pace,
    mood: Option<usize>,
}

/// One sample's changes: the values that moved, the mood if it changed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Changes {
    pub values: Vec<(String, Vec<f32>)>,
    pub mood: Option<String>,
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        self.values.is_empty() && self.mood.is_none()
    }
}

/// a path's value now: an input (linked or not), an output (an audio one = its peak), an LFO's shape
pub fn read(graph: &Graph, vals: &dyn Values, p: Path) -> f32 {
    match p {
        Path::Input(n, k) => {
            let kind = graph.nodes[n].kind;
            let name = kind.inputs()[k].name;
            vals.input(n, name, crate::params::default(kind, name))
        }
        Path::Output(n, k) if graph.nodes[n].kind.outputs()[k].ty == PortType::Audio => vals.meter(n, k),
        Path::Output(n, k) => vals.output(n, k),
        Path::Shape(n) => vals.lfo_shape(n).unwrap_or(0) as f32,
    }
}

impl Stream {
    pub fn new(graph: &Graph, being: &Being) -> Stream {
        let entries = being
            .shown()
            .into_iter()
            .map(|at| Entry { path: values::path_name(graph, at), at, eps: values::epsilon(graph, at), last: f32::NAN })
            .collect();
        Stream { entries, pace: Pace::default(), mood: None }
    }

    /// how many values the stream carries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// At `time` (s): None between two samples (relay.rs `RATE`), else what moved since it was last sent (the first
    /// sample: all).
    pub fn sample(&mut self, time: f32, graph: &Graph, vals: &dyn Values, being: &Being) -> Option<Changes> {
        if !self.pace.due(f64::from(time)) {
            return None;
        }
        let mut c = Changes::default();
        for e in &mut self.entries {
            let v = read(graph, vals, e.at);
            if e.last.is_nan() || (v - e.last).abs() > e.eps {
                e.last = v;
                c.values.push((e.path.clone(), vec![v]));
            }
        }
        if self.mood != Some(being.head.mood) {
            self.mood = Some(being.head.mood);
            c.mood = being.moods.get(being.head.mood).cloned();
        }
        (!c.is_empty()).then_some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Scene, CATHEDRAL_SCENE};
    use crate::space::values::{Knobs, Local, Mirror, Table};
    use crate::tempo;

    /// a deck's stream, applied to a mirror of the same graph, makes the same being: what each button and hose shows
    #[test]
    fn a_mirror_fed_by_the_stream_shows_what_the_deck_shows() {
        let scene = Scene::parse(CATHEDRAL_SCENE).unwrap();
        let g = Graph::build(&scene).unwrap();
        let mut ctl = Table::new(&g);
        let mut deck = Being::new(&g, &scene.being);
        let mut far = Being::new(&g, &scene.being);
        let mut mirror = Mirror::new(&g);
        let mut stream = Stream::new(&g, &deck);
        assert!(stream.len() >= deck.buttons.len(), "every button + the hoses' sources");
        let meter = |n: usize, p: usize| ((n * 3 + p) % 4) as f32 * 0.25;
        let knob = deck.buttons.iter().find(|b| !b.linked && !b.shape).map(|b| (b.node, b.port)).unwrap();
        let (mut sent, mut samples) = (0, 0);
        for f in 0..120 {
            let t = f as f32 / 60.0;
            let beat = tempo::Now { bpm: 84.0, pos: tempo::position(t as f64 * 1.4, 4), per_bar: 4, running: true };
            ctl.step(1.0 / 60.0);
            ctl.set(knob.0, knob.1, 0.1 + f as f32 * 0.001);
            let local = Local { ctl: &ctl, meter: &meter };
            deck.update(1.0 / 60.0, t, &local, &beat);
            if let Some(c) = stream.sample(t, &g, &local, &deck) {
                samples += 1;
                sent += c.values.len();
                for (path, v) in &c.values {
                    assert!(mirror.apply(&g, path, v), "{path}");
                }
            }
            far.update(1.0 / 60.0, t, &mirror, &beat);
        }
        assert!((59..=61).contains(&samples), "30 a second: {samples}");
        assert!(sent < samples * stream.len(), "only what changed: {sent}");
        for (a, b) in deck.buttons.iter().zip(&far.buttons) {
            // the input as streamed (a linked knob's base stays with its deck – #188)
            assert!((a.live - b.live).abs() <= values::epsilon(&g, if a.shape { Path::Shape(a.node) } else { Path::Input(a.node, a.port) }) * 2.0 + 1e-6, "{} {} vs {}", a.name, a.live, b.live);
        }
        // what moved after the last sample goes out with the next; then a graph that stands still sends nothing
        let local = Local { ctl: &ctl, meter: &meter };
        let _ = stream.sample(10.0, &g, &local, &deck);
        assert_eq!(stream.sample(11.0, &g, &local, &deck), None);
    }
}
