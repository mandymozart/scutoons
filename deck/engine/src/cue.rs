//! The cue (decisions/deck.md 2026-11-18 [workspaces]): while a show is live it owns the outlets, and what else plays here –
//! a tab's preview, a pre-listened cell – is a second, small engine folded to stereo (`DeckConfig::folded`) and summed onto
//! the device's `cue` pair, never onto a channel another group plays on. The cue mix blends the show (this device's main
//! pair) with the preview, as a DJ mixer's cue / master: 0 = the show, 1 = the preview. A device without a cue renders the
//! preview for its meters and plays it nowhere.
use super::{DeckConfig, Engine, QUANTUM};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// what a preview is summed with by default: the preview alone
pub const MIX: f32 = 1.0;

/// The frame thread's side: hand the audio thread a preview (or none), set the mix.
pub struct Cue {
    slot: rtrb::Producer<Option<Box<Engine>>>,
    back: rtrb::Consumer<Box<Engine>>,
    mix: Arc<AtomicU32>,
    heard: bool,
}

impl Cue {
    /// `engine` (a `DeckConfig::folded` build) plays on the cue from the next quantum on, in place of the one before; None
    /// stops it. False: the hand-over is full (the audio thread is not running) – nothing changed.
    pub fn play(&mut self, engine: Option<Engine>) -> bool {
        while self.back.pop().is_ok() {}
        self.slot.push(engine.map(Box::new)).is_ok()
    }

    /// the cue mix: 0 = the show only, 1 = the preview only
    pub fn set_mix(&self, mix: f32) {
        self.mix.store(mix.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub fn mix(&self) -> f32 {
        f32::from_bits(self.mix.load(Ordering::Relaxed))
    }

    /// whether the device has a cue of its own: else a preview is silent (its meters still read)
    pub fn heard(&self) -> bool {
        self.heard
    }
}

/// The audio thread's side, inside the engine whose device it shares.
pub(crate) struct Side {
    slot: rtrb::Consumer<Option<Box<Engine>>>,
    back: rtrb::Producer<Box<Engine>>,
    engine: Option<Box<Engine>>,
    mix: Arc<AtomicU32>,
    last: f32,
    /// the cue's device channels (one or two) and the show's pair beside them
    on: Vec<usize>,
    show: Vec<usize>,
}

/// The cue's device channels on `deck`: its pair as mapped, none when the cue shares a channel with another group (it
/// would reach the audience) or is not mapped.
pub fn channels(deck: &DeckConfig) -> Vec<usize> {
    let of = |g: &str, w: usize| deck.channels_of(g, w).into_iter().flatten().collect::<Vec<usize>>();
    let cue = of("cue", 2);
    let mut others = of("main", 2);
    for o in deck.outlets.iter().filter(|o| o.group != "cue") {
        others.extend(of(&o.group, o.channels.len()));
    }
    if cue.iter().any(|c| others.contains(c)) {
        return Vec::new();
    }
    cue
}

impl Side {
    pub(crate) fn new(deck: &DeckConfig) -> (Side, Cue) {
        let (tx, rx) = rtrb::RingBuffer::new(4);
        let (back_tx, back_rx) = rtrb::RingBuffer::new(8);
        let mix = Arc::new(AtomicU32::new(MIX.to_bits()));
        let on = channels(deck);
        let show = deck.channels_of("main", 2).into_iter().flatten().collect();
        let heard = !on.is_empty();
        (Side { slot: rx, back: back_tx, engine: None, mix: mix.clone(), last: MIX, on, show }, Cue { slot: tx, back: back_rx, mix, heard })
    }

    /// the preview's next quantum, blended onto the cue channels of `dev` (planar, `n` frames); never allocates (an engine
    /// handed back while the frame thread's ring is full is dropped here – it does not happen while it reaps)
    pub(crate) fn quantum(&mut self, dev: &mut super::Buf, n: usize) {
        while let Ok(next) = self.slot.pop() {
            if let Some(old) = std::mem::replace(&mut self.engine, next) {
                let _ = self.back.push(old);
            }
        }
        let Some(p) = self.engine.as_deref_mut() else { return };
        p.quantum();
        let (from, to) = (self.last, f32::from_bits(self.mix.load(Ordering::Relaxed)).clamp(0.0, 1.0));
        self.last = to;
        if self.on.is_empty() {
            return;
        }
        let pv = &p.dev;
        let wide = pv.channels();
        let (data, stride) = dev.planar_mut();
        for f in 0..n.min(QUANTUM) {
            let m = from + (to - from) * (f + 1) as f32 / n as f32;
            let (l, r) = (pv.ch(0)[f], pv.ch(1.min(wide - 1))[f]);
            let show: [f32; 2] = [0, 1].map(|k| self.show.get(k).or(self.show.first()).map_or(0.0, |&c| data[c * stride + f]));
            if self.on.len() == 1 {
                data[self.on[0] * stride + f] += (1.0 - m) * 0.5 * (show[0] + show[1]) + m * 0.5 * (l + r);
            } else {
                for (k, &c) in self.on.iter().take(2).enumerate() {
                    data[c * stride + f] += (1.0 - m) * show[k] + m * [l, r][k];
                }
            }
        }
    }

    /// the cue mix as set now: 0 = the show, 1 = the preview
    pub(crate) fn mix(&self) -> f32 {
        f32::from_bits(self.mix.load(Ordering::Relaxed)).clamp(0.0, 1.0)
    }

    /// the preview's clock follows its host's: its frontier as the host's (they render in step), heard at `heard_ns`
    pub(crate) fn mark(&mut self, pending: usize, heard_ns: u64) {
        if let Some(p) = self.engine.as_deref_mut() {
            p.pending = pending;
            p.mark(heard_ns);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Build, Seams, Sequencing};
    use deck_space::graph::Graph;
    use deck_space::scene::Scene;

    fn osc(id: &str, hz: f32, group: &str, pin: &str) -> String {
        format!("[[node]]\nid=\"{id}\"\nkind=\"osc\"\nfreq={hz}\ngain=0.4\n[[node]]\nid=\"{id}_o\"\nkind=\"out\"\ngroup=\"{group}\"\n{pin}[[link]]\nfrom=\"{id}\"\nto=\"{id}_o.in\"\n")
    }

    fn build(text: &str, deck: DeckConfig) -> (Engine, crate::Handles) {
        let g = Graph::build(&Scene::parse(text).unwrap()).unwrap();
        Engine::build_seams(&g, 48_000.0, 1.0, Build::detect(), Sequencing::InBlock, &Seams::with_deck(deck)).unwrap()
    }

    fn render(e: &mut Engine) -> Vec<f32> {
        let ch = e.device_channels();
        let mut buf = vec![0.0f32; 4800 * ch];
        e.render(&mut buf, ch, |x| x);
        buf
    }

    fn peaks(e: &mut Engine) -> Vec<f32> {
        let ch = e.device_channels();
        let buf = render(e);
        (0..ch).map(|c| buf.iter().skip(c).step_by(ch).fold(0.0f32, |m, x| m.max(x.abs()))).collect()
    }

    fn preview(text: &str) -> Engine {
        let g = Graph::build(&Scene::parse(text).unwrap()).unwrap();
        build(text, DeckConfig::folded(&crate::outlet_groups(&g))).0
    }

    /// a scene of any width and any groups, pins too, comes out of the fold as one stereo pair
    #[test]
    fn the_fold_plays_every_group_on_one_pair() {
        let scene = osc("a", 220.0, "main", "") + &osc("b", 330.0, "stem", "") + &osc("c", 440.0, "cue", "first=3\n");
        let p = peaks(&mut preview(&scene));
        assert_eq!(p.len(), 2);
        assert!(p.iter().all(|&x| x > 0.3), "{p:?}");
    }

    /// the rack (main 1/2, cue 3/4): the preview reaches the cue only, the mix blends the show's pair with it, and main is
    /// what it was
    #[test]
    fn a_preview_sums_onto_the_cue_by_the_mix_and_never_onto_main() {
        let rack = DeckConfig::device(4).with_outlet("main", &[1, 2]).with_outlet("cue", &[3, 4]);
        let show = osc("s", 220.0, "main", "");
        let alone = peaks(&mut build(&show, rack.clone()).0);
        assert!(alone[0] > 0.3 && alone[2] == 0.0, "{alone:?}");
        let (mut e, mut h) = build(&show, rack.clone());
        let mut cue = h.cue.take().unwrap();
        assert!(cue.heard());
        assert!(cue.play(Some(preview(&osc("p", 330.0, "main", "")))));
        peaks(&mut e);
        let p = peaks(&mut e);
        assert!((p[0] - alone[0]).abs() < 1e-6 && (p[1] - alone[1]).abs() < 1e-6, "main untouched: {p:?}");
        assert!(p[2] > 0.3 && p[3] > 0.3, "the preview on the cue: {p:?}");
        // the mix at the show: the cue is main's pair
        cue.set_mix(0.0);
        peaks(&mut e);
        let buf = render(&mut e);
        assert!(buf.chunks(4).all(|f| (f[2] - f[0]).abs() < 1e-6 && (f[3] - f[1]).abs() < 1e-6), "the show on the cue");
        // stopped: the cue is silent again
        assert!(cue.play(None));
        peaks(&mut e);
        assert_eq!(peaks(&mut e)[2], 0.0);
    }

    /// a device without a cue, or a cue on main's channels: the preview is heard nowhere, main plays as it did
    #[test]
    fn without_a_cue_of_its_own_the_preview_is_silent() {
        let show = osc("s", 220.0, "main", "");
        for deck in [DeckConfig::device(2), DeckConfig::device(4).with_outlet("cue", &[2, 3])] {
            assert!(channels(&deck).is_empty(), "{deck:?}");
            let (mut e, mut h) = build(&show, deck.clone());
            let alone = render(&mut build(&show, deck).0);
            let mut cue = h.cue.take().unwrap();
            assert!(!cue.heard());
            cue.play(Some(preview(&osc("p", 330.0, "main", ""))));
            assert_eq!(render(&mut e), alone);
        }
        let rack = DeckConfig::device(8).with_outlet("main", &[1, 2]).with_outlet("cue", &[3, 4]).with_outlet("stem", &[5, 6]);
        assert_eq!(channels(&rack), [2, 3]);
    }
}
