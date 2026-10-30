//! The click (#181, the owner's "ein richtiger Klick"): a short sine tick on every beat of the tempo map, higher on
//! the bar's first beat, sample-accurate – the tick starts on the frame where the beat begins. Sounds only while the
//! clock runs and the click is on (the tempo node's `click`, the K key). Params: `freq` (1000 Hz), `accent` (1500 Hz),
//! `decay` (0.03 s); the `gain` input.
use super::{Block, Param, Stereo};
use crate::graph::Node;
use crate::tempo::BeatReader;

pub struct Click {
    beats: BeatReader,
    gain: Param,
    rate: f32,
    freq: f32,
    accent: f32,
    decay: f32,
    level: f32,
    phase: f32,
    inc: f32,
    /// the last beat that ticked (or that was passed while silent); None before the first block
    last: Option<i64>,
}

impl Click {
    pub fn new(node: &Node, rate: f32, gain: Param, beats: BeatReader) -> Click {
        let decay = (-1.0 / (node.f32("decay", 0.03).max(1e-3) * rate)).exp();
        Click {
            beats,
            gain,
            rate,
            freq: node.f32("freq", 1000.0),
            accent: node.f32("accent", 1500.0),
            decay,
            level: 0.0,
            phase: 0.0,
            inc: 0.0,
            last: None,
        }
    }
}

impl Block for Click {
    fn process(&mut self, _: &[Option<&Stereo>], outs: &mut [Stereo], n: usize) {
        let out = &mut outs[0];
        let Some((map, on, start)) = self.beats.block() else {
            out.l[..n].fill(0.0);
            out.r[..n].fill(0.0);
            return;
        };
        let gain = self.gain.get();
        for i in 0..n {
            let b = map.beat_at(start + i as f64);
            let k = b.floor() as i64;
            // the first frame ever: a beat that starts right here ticks
            let last = *self.last.get_or_insert(if b - (k as f64) < 1e-9 { k - 1 } else { k });
            if k > last {
                if map.running && on {
                    let top = k.rem_euclid(map.per_bar.max(1) as i64) == 0;
                    self.inc = if top { self.accent } else { self.freq } / self.rate;
                    self.level = 1.0;
                    self.phase = 0.0;
                }
                self.last = Some(k);
            } else if k < last {
                self.last = Some(k); // the beat went back (a phase set): no tick, go on from here
            }
            let v = if self.level > 1e-4 {
                let v = (self.phase * std::f32::consts::TAU).sin() * self.level * gain;
                self.phase = (self.phase + self.inc).fract();
                self.level *= self.decay;
                v
            } else {
                0.0
            };
            out.l[i] = v;
            out.r[i] = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::audio::Engine;
    use crate::graph::Graph;
    use crate::scene::Scene;

    const SCENE: &str = "[[node]]\nid=\"t\"\nkind=\"tempo\"\nbpm=120\nbeats=4\nclick=true\n\
                         [[node]]\nid=\"c\"\nkind=\"click\"\ngain=0.5\n\
                         [[node]]\nid=\"out\"\nkind=\"audio_out\"\nvolume=1\n[[link]]\nfrom=\"c\"\nto=\"out.in\"\n";

    /// render `frames` in 480-frame blocks (mono = the left channel)
    fn render(e: &mut Engine, frames: usize) -> Vec<f32> {
        let mut all = Vec::new();
        let mut buf = vec![0.0f32; 480 * 2];
        for _ in 0..frames / 480 {
            e.render(&mut buf, 2, |x| x);
            all.extend(buf.chunks_exact(2).map(|f| f[0]));
        }
        all
    }

    /// where each tick begins (a sample after at least 10 ms of silence)
    fn onsets(v: &[f32]) -> Vec<usize> {
        let mut quiet = 480;
        let mut out = Vec::new();
        for (i, x) in v.iter().enumerate() {
            if x.abs() > 1e-6 {
                if quiet >= 480 {
                    out.push(i);
                }
                quiet = 0;
            } else {
                quiet += 1;
            }
        }
        out
    }

    /// zero crossings in the 10 ms after `at` (1000 Hz ≈ 20, 1500 Hz ≈ 30)
    fn crossings(v: &[f32], at: usize) -> usize {
        v[at..at + 480].windows(2).filter(|w| (w[0] <= 0.0) != (w[1] <= 0.0)).count()
    }

    #[test]
    fn ticks_land_on_the_beat_sample_accurately() {
        let (mut e, _) = Engine::build(&Graph::build(&Scene::parse(SCENE).unwrap()).unwrap(), 48_000.0, 1.0).unwrap();
        let v = render(&mut e, 48_000 * 5 / 2);
        // 120 BPM at 48 kHz: a beat every 24 000 frames; the sine starts at 0, so its first non-zero sample is the next
        let at: Vec<usize> = onsets(&v).iter().map(|&i| i - 1).collect();
        assert_eq!(at, vec![0, 24_000, 48_000, 72_000, 96_000]);
        assert!(crossings(&v, 0) > 26 && crossings(&v, 96_000) > 26, "the bar's first beat is the accent");
        assert!(crossings(&v, 24_000) < 23);
    }

    #[test]
    fn a_tempo_change_moves_the_ticks_and_stop_silences_them() {
        let (mut e, h) = Engine::build(&Graph::build(&Scene::parse(SCENE).unwrap()).unwrap(), 48_000.0, 1.0).unwrap();
        let v = render(&mut e, 48_000);
        assert_eq!(onsets(&v).len(), 2);
        // the frame thread: 60 BPM from frame 48 000 (beat 2) on
        let (m, click) = h.tempo.map().unwrap();
        h.tempo.publish(&m.with_bpm(48_000.0, 60.0), click);
        let v = render(&mut e, 96_000);
        let at: Vec<usize> = onsets(&v).iter().map(|&i| i - 1 + 48_000).collect();
        assert_eq!(at, vec![48_000, 96_000]);
        // stopped at frame 144 000: silence; the click off: silence
        let (m, _) = h.tempo.map().unwrap();
        h.tempo.publish(&m.with_running(144_000.0, false), true);
        assert!(render(&mut e, 96_000).iter().all(|x| *x == 0.0));
        h.tempo.publish(&m, false);
        assert!(render(&mut e, 96_000).iter().all(|x| *x == 0.0));
    }
}
