//! A random walk on a continuous pitch scale (the phone page's melody, owner 2026-10-31: "random, one parameter turns
//! it into a drone and back to more range … stay out of the 12 tone system"): on every step a pitch around `base_hz`,
//! `spread` 0 = the base held, 1 = ±`octaves`; `glide` rides along as a prop for the voice. Seeded: the same seed, the
//! same melody on every deck.
use super::{steps, unit};
use crate::event::{Event, Time};
use crate::generator::{Generator, Inbox, Outbox, Ports, Span};
use crate::names::Sym;

#[derive(Debug, Clone)]
pub struct RandomWalk {
    pub note: Sym,
    pub pitch: Sym,
    pub glide_key: Sym,
    pub seed: u64,
    pub base_hz: f32,
    /// 0..1
    pub spread: f32,
    pub octaves: f32,
    pub per_beat: f64,
    /// of a step: how long the note holds (0..1)
    pub length: f64,
    /// of a step: how long the voice glides into it (0..1)
    pub glide: f32,
}

impl RandomWalk {
    /// step k's pitch
    pub fn pitch_at(&self, k: i64) -> f32 {
        let r = unit(self.seed, k) as f32 * 2.0 - 1.0;
        self.base_hz * 2f32.powf(r * self.spread.clamp(0.0, 1.0) * self.octaves)
    }
}

impl Generator for RandomWalk {
    fn ports(&self) -> Ports {
        Ports { ins: &[], outs: &["notes"], params: &["spread", "rate", "glide"] }
    }

    fn set(&mut self, index: usize, value: f32) {
        match index {
            0 => self.spread = value.clamp(0.0, 1.0),
            1 => self.per_beat = f64::from(value.max(0.0)),
            2 => self.glide = value.clamp(0.0, 1.0),
            _ => {}
        }
    }

    fn query(&mut self, span: &Span, _: &Inbox, out: &mut Outbox) {
        let dur = self.length.clamp(0.0, 1.0) / self.per_beat.max(f64::MIN_POSITIVE);
        for (k, beat) in steps(span.from, span.to, self.per_beat) {
            let e = Event::new(self.note, Time::Beats(beat))
                .lasting(Time::Beats(dur))
                .with(self.pitch, self.pitch_at(k))
                .with(self.glide_key, self.glide);
            out.push(0, e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::names::Names;
    use deck_core::tempo::TempoMap;

    fn walk(n: &mut Names, seed: u64, spread: f32) -> RandomWalk {
        RandomWalk {
            note: n.event("music.note").unwrap(),
            pitch: n.key("pitch").unwrap(),
            glide_key: n.key("glide").unwrap(),
            seed,
            base_hz: 147.0,
            spread,
            octaves: 2.0,
            per_beat: 2.0,
            length: 0.9,
            glide: 0.5,
        }
    }

    fn notes(g: &mut RandomWalk, t: &TempoMap, cuts: &[f64]) -> Vec<(f64, f32)> {
        let mut all = Vec::new();
        for w in cuts.windows(2) {
            let span = Span::frames(t, w[0], w[1]);
            let mut out = Outbox::new(1, 1024);
            g.query(&span, &Inbox { ports: &[] }, &mut out);
            let p = g.pitch;
            all.extend(out.port(0).iter().map(|e| match e.start {
                Time::Beats(b) => (b, e.props.get(p).unwrap()),
                _ => unreachable!(),
            }));
        }
        all
    }

    #[test]
    fn the_same_seed_plays_the_same_melody_however_the_window_is_cut() {
        let mut n = Names::new();
        let t = TempoMap::new(84.0, 4, 48_000.0);
        let mut a = walk(&mut n, 42, 1.0);
        let mut b = walk(&mut n, 42, 1.0);
        let whole = notes(&mut a, &t, &[0.0, 200_000.0]);
        let blocks: Vec<f64> = (0..=781).map(|i| i as f64 * 256.0).chain([200_000.0]).collect();
        let cut = notes(&mut b, &t, &blocks);
        assert_eq!(whole, cut, "window independence: one window or 782 blocks");
        assert!(whole.len() >= 11, "84 bpm, 2 steps a beat, ~4.2 s: {}", whole.len());
        let other = notes(&mut walk(&mut n, 7, 1.0), &t, &[0.0, 200_000.0]);
        assert_ne!(whole, other, "another seed, another melody");
    }

    #[test]
    fn spread_zero_holds_the_base_and_one_stays_in_range() {
        let mut n = Names::new();
        let drone = walk(&mut n, 1, 0.0);
        assert!((0..100).all(|k| drone.pitch_at(k) == 147.0));
        let wide = walk(&mut n, 1, 1.0);
        assert!((0..1000).all(|k| (147.0 / 4.0..=147.0 * 4.0).contains(&wide.pitch_at(k))));
    }
}
