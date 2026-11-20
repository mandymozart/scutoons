//! A sound asset's preview voice on the audio thread (deck-core `sound_preview`, decisions/deck.md [media]): the frame
//! thread asks Play / Stop / PANIC, the engine plays the clip on top of what goes out, under the master gain – on the
//! cue while a show is live (scaled by the cue mix, as a preview is), else on main.
use super::{DeckConfig, QUANTUM};
use deck_core::asset_edit::Playback;
use deck_core::sound_preview::{self, Clip, Voice};
use std::sync::Arc;

enum Ask {
    Play { clip: Arc<Clip>, from: f64, edit: Playback, live: bool },
    Stop,
    Panic,
}

/// The frame thread's side.
pub struct Sounds {
    asks: rtrb::Producer<Ask>,
    back: rtrb::Consumer<Arc<Clip>>,
}

impl Sounds {
    fn ask(&mut self, a: Ask) -> bool {
        while self.back.pop().is_ok() {}
        self.asks.push(a).is_ok()
    }

    /// `clip` from `from` s on, looped inside the playback's span at its speed, gain and fades; `live`: a show is live, it
    /// goes to the cue. False: the ring is full (the audio thread is not running).
    pub fn play(&mut self, clip: Arc<Clip>, from: f64, edit: Playback, live: bool) -> bool {
        self.ask(Ask::Play { clip, from, edit, live })
    }

    /// it fades out and ends
    pub fn stop(&mut self) -> bool {
        self.ask(Ask::Stop)
    }

    /// silent from the next quantum
    pub fn panic(&mut self) -> bool {
        self.ask(Ask::Panic)
    }
}

/// The audio thread's side.
pub(crate) struct Side {
    asks: rtrb::Consumer<Ask>,
    back: rtrb::Producer<Arc<Clip>>,
    voice: Voice,
    live: bool,
    main: Vec<usize>,
    cue: Vec<usize>,
}

impl Side {
    pub(crate) fn new(deck: &DeckConfig, rate: f32) -> (Side, Sounds) {
        let (tx, rx) = rtrb::RingBuffer::new(16);
        let (back_tx, back_rx) = rtrb::RingBuffer::new(16);
        let main = deck.channels_of("main", 2).into_iter().flatten().collect();
        let cue = super::cue::channels(deck);
        (Side { asks: rx, back: back_tx, voice: Voice::new(rate), live: false, main, cue }, Sounds { asks: tx, back: back_rx })
    }

    fn give_back(&mut self, clip: Option<Arc<Clip>>) {
        if let Some(c) = clip {
            let _ = self.back.push(c);
        }
    }

    /// the voice's next quantum onto `dev`, times the master gain (and the cue mix while live)
    pub(crate) fn quantum(&mut self, dev: &mut super::Buf, n: usize, master: f32, mix: f32) {
        while let Ok(a) = self.asks.pop() {
            let old = match a {
                Ask::Play { clip, from, edit, live } => {
                    self.live = live;
                    self.voice.play(clip, from, edit)
                }
                Ask::Stop => {
                    self.voice.stop();
                    None
                }
                Ask::Panic => self.voice.cut(),
            };
            self.give_back(old);
        }
        if !self.voice.sounding() {
            return;
        }
        let on = sound_preview::channels(self.live, &self.main, &self.cue);
        let scale = if self.live { master * mix } else { master };
        let mut chans: [&mut [f32]; 2] = [&mut [], &mut []];
        let mut wide = 0;
        let (data, stride) = dev.planar_mut();
        for (k, ch) in data.chunks_mut(stride).enumerate() {
            if let Some(at) = on.iter().take(2).position(|&c| c == k) {
                chans[at] = &mut ch[..n.min(QUANTUM)];
                wide = wide.max(at + 1);
            }
        }
        let ended = self.voice.add(&mut chans[..wide], n, scale);
        self.give_back(ended);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Build, Engine, Seams, Sequencing};
    use deck_space::graph::Graph;
    use deck_space::scene::Scene;

    fn engine(deck: DeckConfig) -> (Engine, crate::Handles) {
        let g = Graph::build(&Scene::parse("").unwrap()).unwrap();
        Engine::build_seams(&g, 48_000.0, 1.0, Build::detect(), Sequencing::InBlock, &Seams::with_deck(deck)).unwrap()
    }

    fn peaks(e: &mut Engine) -> Vec<f32> {
        let ch = e.device_channels();
        let mut buf = vec![0.0f32; 4800 * ch];
        e.render(&mut buf, ch, |x| x);
        (0..ch).map(|c| buf.iter().skip(c).step_by(ch).fold(0.0f32, |m, x| m.max(x.abs()))).collect()
    }

    /// one second of a 0.5 square: the first half +, the second −
    fn clip() -> Arc<Clip> {
        let s: Vec<f32> = (0..48_000).map(|k| if k < 24_000 { 0.5 } else { -0.5 }).collect();
        Arc::new(Clip::from_interleaved(48_000, 1, &s))
    }

    fn rack() -> DeckConfig {
        DeckConfig::device(4).with_outlet("main", &[1, 2]).with_outlet("cue", &[3, 4])
    }

    /// Play renders the span from the playhead on main; Stop and PANIC silence it
    #[test]
    fn the_voice_plays_its_span_on_main_and_stops_on_stop_and_panic() {
        let (mut e, mut h) = engine(rack());
        let mut s = h.sound.take().unwrap();
        assert_eq!(peaks(&mut e), [0.0; 4], "silent until Play");
        // the span is the second half only: every sample played is −0.5
        assert!(s.play(clip(), 0.6, Playback { start: 0.5, end: 1.0, ..Playback::whole(1.0) }, false));
        let ch = e.device_channels();
        let mut buf = vec![0.0f32; 48_000 * ch];
        e.render(&mut buf, ch, |x| x);
        let left: Vec<f32> = buf.iter().step_by(ch).copied().skip(480).collect();
        assert!(left.iter().all(|&x| (x + 0.5).abs() < 1e-5), "inside the span only, looped");
        assert!(buf.chunks(4).all(|f| f[2] == 0.0 && f[3] == 0.0), "not on the cue");
        assert!(s.stop());
        peaks(&mut e);
        assert_eq!(peaks(&mut e), [0.0; 4], "Stop: it fades and ends");
        assert!(s.play(clip(), 0.0, Playback { start: 0.0, end: 1.0, ..Playback::whole(1.0) }, false));
        assert!(peaks(&mut e)[0] > 0.4);
        assert!(s.panic());
        assert_eq!(peaks(&mut e), [0.0; 4], "PANIC: silent from the next quantum");
    }

    /// while a show is live it goes to the cue (by the cue mix), never to main; without a cue it is heard nowhere
    #[test]
    fn while_live_it_goes_to_the_cue() {
        let (mut e, mut h) = engine(rack());
        let mut s = h.sound.take().unwrap();
        s.play(clip(), 0.0, Playback { start: 0.0, end: 1.0, ..Playback::whole(1.0) }, true);
        let p = peaks(&mut e);
        assert!(p[0] == 0.0 && p[1] == 0.0 && p[2] > 0.4 && p[3] > 0.4, "{p:?}");
        h.cue.as_ref().unwrap().set_mix(0.0);
        peaks(&mut e);
        assert_eq!(peaks(&mut e), [0.0; 4], "the cue mix at the show: the preview is out of it");
        let (mut e, mut h) = engine(DeckConfig::device(2));
        h.sound.take().unwrap().play(clip(), 0.0, Playback { start: 0.0, end: 1.0, ..Playback::whole(1.0) }, true);
        assert_eq!(peaks(&mut e), [0.0; 2], "no cue of its own: nowhere");
    }
}
